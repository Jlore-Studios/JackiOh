// #4026: a suggestion survey runs on a spare lane, on a subscription inside its `start_headroom`, a
// strong seat's first; and several run at once, one an area of `suggestions.areas`, sharing the
// room under `suggestions.max_open` among them.
package tests

import (
	"fmt"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/waiting"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// surveyClaim plans once on run `runID`, which GitHub then lists as going, as the status loop's
// start does: what the plan claimed.
func surveyClaim(ctx *hctx.Context, runID string) *pyjson.Object {
	ctx.Cfg.RunID = runID
	planned := plan.Make(ctx, false, nil, "auto", nil)
	ctx.GH.(*fakes.FakeGitHub).Runs.Set(runID, pyjson.New("status", "in_progress"))
	return planned
}

// surveyUsage puts subscription `id`'s 5-hour usage at `utilization`, read just now.
func surveyUsage(ctx *hctx.Context, id string, utilization float64) {
	flowUpdate(ctx, func(s *pyjson.Object) {
		s.Obj("providers").Set(id, pyjson.New("usage", pyjson.New("five_hour",
			pyjson.New("utilization", utilization, "resets_at",
				clock.ISO(ctx.Now().Add(90*time.Minute))))))
	}, "seed")
}

// claude-7 is 2 points under its 70% cap: it may still make calls, so a survey once started there
// and was cut off at the cap; inside its `start_headroom` (5%) its lanes are not spare, and no
// survey goes there now. claude-3 has no cap, but 90% of its 5-hour allowance is spent, so the
// load balancer puts it after the accounts with room. The survey goes to claude-8. Forced, it
// takes the pool's first, claude-3, as every survey did before.
func TestSurveys_ASurveyRunsOnASpareSubscriptionWithRoom(t *testing.T) {
	ctx := spareNight(20, 6)
	surveyUsage(ctx, "claude-7", 0.68)
	surveyUsage(ctx, "claude-3", 0.9)
	assertNotIn(t, "claude-7", strings.Join(spareOf(ctx, pyjson.New()), "; "))
	state := ctx.Store.Load()
	lanes := waiting.LanesOf(ctx, state, pyjson.New())
	assertEqual(t, plan.SurveyProvider(ctx, state, lanes, true, "", nil).ID, "claude-3")
	idleNow(ctx)
	planned := surveyClaim(ctx, "101")
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"suggest", "claude-8"})
	// No spare lane anywhere: no survey, though claude-3 could make the calls.
	assertTrue(t, plan.SurveyProvider(ctx, state, lanes, false, "", nil) == nil, "no survey")
}

// A strong seat's spare lane takes the survey before a medium one's, whatever the pool's order.
func TestSurveys_AStrongSeatsSpareLaneGoesFirst(t *testing.T) {
	ctx := spareNight(20, 6, "gpt")
	state := ctx.Store.Load()
	lanes := waiting.LanesOf(ctx, state, pyjson.New())
	pool := ctx.Cfg.Pool
	spare := []plan.SpareLanes{{Provider: pool.Get("gpt"), Lanes: 1},
		{Provider: pool.Get("claude-2"), Lanes: 1, Hard: true}}
	assertEqual(t, plan.SurveyProvider(ctx, state, lanes, false, "", spare).ID, "claude-2")
	assertEqual(t, plan.SurveyProvider(ctx, state, lanes, false, "", spare[:1]).ID, "gpt")
}

// idleNow is a tick having seen the lanes spare just now (`suggestions.idle_minutes` is 0).
func idleNow(ctx *hctx.Context) {
	flowUpdate(ctx, func(s *pyjson.Object) {
		s.Obj("suggest").Set("idle_since", clock.ISO(ctx.Now()))
	}, "seed")
}

// Every account's 15 lanes spare: a survey starts in each of the five areas, in turn, the ten
// suggestions' room shared out so the first does not take it all (2 each), each holding a lane of
// its own, spread over the accounts by the load balancer (the fewest runs first); a sixth waits for
// an area. The status says which area each surveys.
func TestSurveys_SeveralRunAtOnceEachOnItsOwnArea(t *testing.T) {
	ctx := spareNight(20, 6)
	assertEqual(t, len(ctx.Cfg.SuggestionAreas), 5)
	idleNow(ctx)
	var got []string
	for i := range 6 {
		planned := surveyClaim(ctx, strconv.Itoa(101+i))
		if planned.Get("action") != "suggest" {
			got = append(got, fmt.Sprint(planned.Get("action")))
			continue
		}
		got = append(got, fmt.Sprint(planned.Get("slot"), " ", planned.Get("area"), " ",
			planned.Get("count"), " ", planned.Get("provider")))
	}
	assertEqual(t, got, []string{"1 rules 2 claude-3", "2 ai 2 claude-7", "3 server 2 claude-8",
		"4 client 2 claude-1", "5 tests 2 claude-4", "none"})
	state := ctx.Store.Load()
	lanes := plan.ReadLanes(ctx, state)
	assertEqual(t, []string{lanes.Held[-1], lanes.Held[-5]}, []string{"claude-3", "claude-4"})
	assertIn(t, "a survey with `claude-7`", lanes.Describe())
	assertEqual(t, status.Record(state, -3).Str("area"), "server")
	held, live := status.LanesNow(ctx, state)
	assertEqual(t, held.Len(), 5)
	assertIn(t, "a suggestion survey of `client`", strings.Join(status.RunningLines(ctx, state, live), "\n"))
}

// The room the running surveys claimed is theirs: with 6 suggestions open and 2 surveys running
// for 2 each, the next may open the last 2, and then no more start. One running survey and a
// single spare lane give the next all that is left.
func TestSurveys_TheClaimsNeverPassMaxOpen(t *testing.T) {
	ctx := spareNight(20, 6)
	for n := 20; n < 26; n++ {
		ctx.GH.(*fakes.FakeGitHub).AddIssue(n, fakes.Issue{Labels: []string{config.LabelSuggestion}})
	}
	idleNow(ctx)
	counts := []any{}
	for i := range 4 {
		planned := surveyClaim(ctx, strconv.Itoa(201+i))
		counts = append(counts, planned.Get("count"))
	}
	assertEqual(t, counts, []any{1, 1, 1, 1})
	assertEqual(t, surveyClaim(ctx, "205").Get("action"), "none")
	assertEqual(t, plan.SuggestionsDue(ctx, ctx.Store.Load(), false, 1), 0)
	flowUpdate(ctx, func(s *pyjson.Object) {
		s.Obj("suggest").Set("running", pyjson.New("1", pyjson.New("provider", "claude-3", "run_id", "201",
			"area", "rules", "count", 1)))
	}, "seed")
	assertEqual(t, plan.SuggestionsDue(ctx, ctx.Store.Load(), false, 1), 3)
}

// A survey whose run ended gives back its slot, its area and its share, and the asks it held wait
// for the next survey: the next claim takes its slot (and those asks) and the next area in turn,
// and the area it gave back comes round again once the others have theirs.
func TestSurveys_AnEndedSurveyGivesBackItsSlotAndArea(t *testing.T) {
	ctx := spareNight(20, 6)
	idleNow(ctx)
	for i := range 3 {
		surveyClaim(ctx, strconv.Itoa(301+i))
	}
	flowUpdate(ctx, func(s *pyjson.Object) {
		s.Obj("suggest").Obj("running").Obj("2").Set("taken_asks", []any{"c:7"})
	}, "seed")
	gh := ctx.GH.(*fakes.FakeGitHub)
	gh.Runs.Set("302", pyjson.New("status", "completed"))
	assertIn(t, "a survey's run ended", plan.HousekeepingDue(ctx, ctx.Store.Load()))
	var got []any
	for i := range 3 {
		planned := surveyClaim(ctx, strconv.Itoa(304+i))
		got = append(got, planned.Get("slot"), planned.Get("area"))
	}
	assertEqual(t, got, []any{2, "client", 4, "tests", 5, "ai"})
	assertEqual(t, dig(t, ctx.Store.Load(), "suggest", "running", "2", "taken_asks"), []any{"c:7"})
}

// A survey claimed before #4026 (`suggest.provider`, lane 0) is read as running until its run ends,
// holds its lane, and, given all the room when it started, keeps every other survey off; once its
// run ends, housekeeping clears it and the surveys start.
func TestSurveys_AClaimFromBeforeIsReadAndCleared(t *testing.T) {
	ctx := spareNight(20, 6)
	idleNow(ctx)
	flowUpdate(ctx, func(s *pyjson.Object) {
		s.Obj("suggest").Update(pyjson.New("provider", "claude-3", "run_id", "400"))
	}, "seed")
	gh := ctx.GH.(*fakes.FakeGitHub)
	gh.Runs.Set("400", pyjson.New("status", "in_progress"))
	assertEqual(t, plan.ReadLanes(ctx, ctx.Store.Load()).Held[0], "claude-3")
	assertEqual(t, surveyClaim(ctx, "401").Get("action"), "none")
	gh.Runs.Set("400", pyjson.New("status", "completed"))
	planned := surveyClaim(ctx, "402")
	assertEqual(t, []any{planned.Get("action"), planned.Get("slot")}, []any{"suggest", 1})
	assertEqual(t, dig(t, ctx.Store.Load(), "suggest", "provider"), nil)
}

// The prompt keeps a survey to its area, and a plan with none (from before #4026, or no areas)
// surveys the whole repository.
func TestSurveys_ThePromptNamesTheArea(t *testing.T) {
	text := work.SurveyArea(pyjson.New("area", "client", "area_focus", "the web client: `apps/web`"))
	assertIn(t, "**`client`**: the web client: `apps/web`.", text)
	assertIn(t, "Propose only improvements inside this area", text)
	assertEqual(t, work.SurveyArea(pyjson.New()), "The whole repository.")
}
