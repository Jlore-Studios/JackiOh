// #3659: the excess compute: the lanes spare on subscriptions that may start a build or a plan now
// (`plan.Spare`), the suggestion survey that waits for `suggestions.spare_lanes` of them, and the
// status issue's section that says how many more issues would fill them.
package tests

import (
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	hstate "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/waiting"
)

// spareOf is each subscription's spare lanes, by id in the pool's order, with "+hard" on those that
// may take a hard issue: what `plan.Spare` gives for the lanes `live` holds (item number -> id).
func spareOf(ctx *hctx.Context, live *pyjson.Object) []string {
	state := ctx.Store.Load()
	var found []string
	for _, s := range plan.Spare(ctx, state, waiting.LanesOf(ctx, state, live), false) {
		text := s.Provider.ID + " " + strconv.Itoa(s.Lanes)
		if s.Hard {
			text += " +hard"
		}
		found = append(found, text)
	}
	return found
}

// spareNight is the night with every account's secret set, and room for `maxParallel` runs on
// GitHub's runners and `machineParallel` on the AWS boxes, the subscriptions on `machine` switched
// on there.
func spareNight(maxParallel, machineParallel int, machine ...string) *hctx.Context {
	cfg := testsupport.MakeConfig(testsupport.Config{Env: crossAll, Machine: machine})
	return testsupport.MakeCtx(fakes.NewFakeGitHub(), testsupport.Ctx{At: testsupport.Night,
		Cfg: testsupport.WithLanes(cfg, maxParallel, machineParallel)})
}

// GitHub's runners have room for 8 of the bots' runs, so the 15 lanes the Claude accounts have at
// night count 8, in the pool's order; with room for all, each counts its lanes in force
// (`lanes_by_cap`: claude-5's 40% cap gives it one), and a strong seat (Opus) takes hard work.
func TestSpare_TheRunnersRoomCapsTheSpareLanes(t *testing.T) {
	assertEqual(t, spareOf(spareNight(8, 6), pyjson.New()),
		[]string{"claude-3 2 +hard", "claude-7 2 +hard", "claude-8 2 +hard", "claude-1 2 +hard"})
	ctx := spareNight(20, 6)
	assertEqual(t, spareOf(ctx, pyjson.New()),
		[]string{"claude-3 2 +hard", "claude-7 2 +hard", "claude-8 2 +hard", "claude-1 2 +hard",
			"claude-4 2 +hard", "claude-6 2 +hard", "claude-5 1 +hard", "claude-2 2 +hard"})
	// A run holds a lane of its account's, and a runner.
	assertEqual(t, spareOf(ctx, pyjson.New("37", "claude-8")),
		[]string{"claude-3 2 +hard", "claude-7 2 +hard", "claude-8 1 +hard", "claude-1 2 +hard",
			"claude-4 2 +hard", "claude-6 2 +hard", "claude-5 1 +hard", "claude-2 2 +hard"})
}

// The 5% line: an account under its 40% cap (claude-5's) but inside its `start_headroom` of it
// starts no build or plan, so its lane is not spare; at 30% it is. Nor are the lanes of one backing off after
// a run that could not work, or the accounts' outside their hours.
func TestSpare_AnAccountInsideItsStartHeadroomCountsNone(t *testing.T) {
	ctx := spareNight(20, 6)
	five := func(id string, utilization float64) {
		flowUpdate(ctx, func(s *pyjson.Object) {
			s.Obj("providers").Set(id, pyjson.New("usage", pyjson.New("five_hour",
				pyjson.New("utilization", utilization, "resets_at",
					clock.ISO(testsupport.Night.Add(90*time.Minute))))))
		}, "seed")
	}
	cap := ctx.Cfg.Pool.Get("claude-5").HeldCaps(ctx.Now(), ctx.Cfg.Timezone, nil).Get("five_hour")
	assertEqual(t, cap, 0.4)
	five("claude-5", 0.36)
	assertNotIn(t, "claude-5", strings.Join(spareOf(ctx, pyjson.New()), "; "))
	five("claude-5", 0.30)
	assertIn(t, "claude-5 1 +hard", strings.Join(spareOf(ctx, pyjson.New()), "; "))
	flowUpdate(ctx, func(s *pyjson.Object) {
		s.Obj("providers").Set("claude-3", pyjson.New("infra", pyjson.New(
			"at", clock.ISO(testsupport.Night.Add(-10*time.Minute)), "reason", "401", "streak", 2)))
	}, "seed")
	assertNotIn(t, "claude-3", strings.Join(spareOf(ctx, pyjson.New()), "; "))
	day := testsupport.MakeCtx(fakes.NewFakeGitHub(), testsupport.Ctx{At: testsupport.Day, Cfg: ctx.Cfg})
	assertEqual(t, len(spareOf(day, pyjson.New())), 0)
}

// The AWS boxes have room of their own. Their models are medium ones, which take no hard issue.
// Muse fills gpt's and agy's lanes on the night box while they cannot start work; a lane it fills
// counts once: with its fill-in run in gpt's, gpt has none spare and agy its one, and with gpt and
// agy switched off Muse counts the two it may fill beside its own four. The tests' weak seat takes
// no work, so `lite` counts none.
func TestSpare_AFillInsLanesCountOnceAndTheBoxesTakeMediumWork(t *testing.T) {
	ctx := spareNight(0, 20, testsupport.AllMachine...)
	flowUpdate(ctx, func(s *pyjson.Object) {
		hstate.Item(s, 50).Update(pyjson.New("provider", "muse", "kind", "build", "run_id", "50",
			"runs_on", "night-vm-muse-fill"))
	}, "state")
	assertEqual(t, spareOf(ctx, pyjson.New("50", "muse")), []string{"muse 4", "agy 1"})
	assertEqual(t, spareOf(spareNight(0, 20, "muse"), pyjson.New()), []string{"muse 6"})
	assertEqual(t, spareOf(spareNight(0, 3, "muse"), pyjson.New()), []string{"muse 3"})
}

// A survey waits for `suggestions.spare_lanes` (3) spare lanes, no longer for a lane idle 30
// minutes: with claude-1's two alone it never starts, however long they stand; with every account's
// it starts once a tick has seen them (`suggestions.idle_minutes` is 0).
func TestSpare_ASurveyWaitsForThreeSpareLanes(t *testing.T) {
	cfg := testsupport.MakeConfig()
	assertEqual(t, []int{cfg.SuggestionsSpareLanes, cfg.SuggestionsIdleMinutes, cfg.SuggestionsMaxOpen},
		[]int{3, 0, 10})
	one := testsupport.MakeCtx(fakes.NewFakeGitHub(), testsupport.Ctx{At: testsupport.Night})
	testsupport.IdleFor(one, 120)
	look := plan.Peek_(one, false, nil, "auto")
	assertEqual(t, []any{look.Work, look.Idle, look.Spare}, []any{false, false, 2})
	assertEqual(t, plan.SuggestionsDue(one, one.Store.Load(), false, 2), 0)
	assertEqual(t, plan.SuggestionsDue(one, one.Store.Load(), false, 3), 10)
	// Asked for (`/harness suggest`), it runs whatever is spare.
	assertEqual(t, plan.SuggestionsDue(one, one.Store.Load(), true, 0), 10)
	every := spareNight(8, 6)
	look = plan.Peek_(every, false, nil, "auto")
	assertEqual(t, []any{look.Work, look.Idle, look.Spare}, []any{false, true, 8})
	testsupport.IdleFor(every, 0)
	look = plan.Peek_(every, false, nil, "auto")
	assertTrue(t, look.Work)
	assertIn(t, "a suggestion survey is due", look.Reason)
	// Queued work a spare lane can start comes first, and stops the clock.
	every.GH.(*fakes.FakeGitHub).AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	look = plan.Peek_(every, false, nil, "auto")
	assertEqual(t, []any{look.Idle, look.Spare}, []any{false, 0})
	assertIn(t, "#3 (medium) is queued to build", look.Reason)
}

// The status issue opens with the excess compute (#3659), under the headline and above the
// decisions: how many lanes are spare and how many more issues of which difficulty would fill them,
// then a table by subscription. GitHub's runners are shared with Squishy, whose run there takes
// one of their room.
func TestSpare_TheStatusIssueOpensWithTheExcessCompute(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	cfg := testsupport.MakeConfig(testsupport.Config{Env: providersSecrets(providers.SecretNames...),
		Machine: []string{"muse"}})
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night, Cfg: cfg})
	for _, r := range []struct {
		number   int
		provider string
	}{{37, "claude-8"}, {38, "claude-8"}, {49, "muse"}} {
		gh.AddIssue(r.number, fakes.Issue{Labels: []string{config.LabelWorking}})
		gh.Runs.Set(strconv.Itoa(r.number), pyjson.New("status", "in_progress"))
		flowUpdate(ctx, func(s *pyjson.Object) {
			hstate.Item(s, r.number).Update(pyjson.New("provider", r.provider, "kind", "build",
				"run_id", strconv.Itoa(r.number)))
		}, "state")
	}
	gh.AddIssue(60, fakes.Issue{Labels: []string{config.LabelSuggestion}})
	squishy := pyjson.New("id", "claude-squishy", "bot", "Squishy", "claude", true, "github", true,
		"where", "GitHub's runners", "lanes", 2, "used", 1, "runs", 1,
		"secret", "CLAUDE_CODE_OAUTH_TOKEN_SQUISHY", "startable", true)
	part := dashboard.SectionStart + "\n" + dashboard.LanesMark + pyjson.Dumps([]any{squishy}) + " -->\n" +
		"## 🫧 Squishy\n" + dashboard.SectionEnd
	body := dashboardRender(ctx, []string{part}, "")
	excess := strIndex(t, body, dashboard.ExcessHeading)
	assertTrue(t, strIndex(t, body, "# 🤖 "+dashboard.Title) < excess, "under the title")
	assertTrue(t, excess < strIndex(t, body, dashboard.DecisionsHeading), "above the decisions")
	section := body[excess:strIndex(t, body, dashboard.DecisionsHeading)]
	// 8 on GitHub's runners less claude-8's two runs (both its lanes) and Squishy's one: 5,
	// claude-3's two, claude-7's two and claude-1's one; Muse's three on the night box and the Muse
	// box (its fourth runs #49) and the two it fills for gpt and agy, which are off.
	assertIn(t, "> 🟢 **10 lanes spare**: free on subscriptions that may start work now, and nothing "+
		"queued is for them. **Queue 10 more issues** to fill them: **5 hard or medium**, for "+
		"`claude-3`, `claude-7`, `claude-1` (a strong model takes either), and **5 medium**, for "+
		"`muse` (medium models only).", section)
	assertIn(t, "| Subscription | Runs on | Spare lanes | Takes |\n|---|---|---|---|\n"+
		"| `claude-3` | GitHub's runners | 2 | 🔴 hard or 🟡 medium |\n"+
		"| `claude-7` | GitHub's runners | 2 | 🔴 hard or 🟡 medium |\n"+
		"| `claude-1` | GitHub's runners | 1 | 🔴 hard or 🟡 medium |\n"+
		"| `muse` | the night box and the Muse box | 5 | 🟡 medium |\n"+
		"| **All** | | **10** | **5 hard or medium, 5 medium** |", section)
	assertIn(t, "a runner where it runs has room for it (GitHub's runners have 5 free, the AWS boxes "+
		"5, once the planner has handed out every queued item it can start)", section)
	assertIn(t, "(`start_headroom`, 5% under it as `.harness/providers.json` sets it)", section)
	assertIn(t, "With 3 or more spare, the bot runs a suggestion survey (`suggestions.spare_lanes`), "+
		"up to 10 open suggestions (`bot:suggestion`: 1 open now).", section)

	// By day the accounts are outside their hours, and with the box off nothing may start.
	day := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day,
		Cfg: testsupport.MakeConfig(testsupport.Config{Env: crossAll})})
	section = strings.Join(dashboard.ExcessSection(day, day.Store.Load(),
		waiting.Explain(day, day.Store.Load(), pyjson.New(), nil, ""), nil, nil), "\n")
	assertIn(t, "> ⚪ **No excess compute**: every lane that may start work now has queued work for "+
		"it, or none may start a build or a plan", section)
	assertNotIn(t, "| Subscription |", section)
	// Halted, nothing is spare.
	section = strings.Join(dashboard.ExcessSection(ctx, ctx.Store.Load(),
		waiting.Explain(ctx, ctx.Store.Load(), pyjson.New(), nil, "HALT is on main"), nil, nil), "\n")
	assertIn(t, "> ⚪ **No excess compute**: the bot is halted.", section)
}
