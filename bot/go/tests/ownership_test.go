// Ownership (#2748, section 1): an item stays with the subscription that owns it, through a hold
// bounded by `hold_minutes` while only its owner's 5-hour window keeps it; `/harness handoff`
// releases it; a freed lane takes its own work first.
package tests

import (
	"fmt"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/commands"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/sweep"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/waiting"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// ownershipAt is NIGHT (22:00 on the bot's clock) and `minutes` more.
func ownershipAt(minutes int) time.Time {
	return testsupport.Night.Add(time.Duration(minutes) * time.Minute)
}

// ownershipRecord is item #`number`'s record.
func ownershipRecord(ctx *hctx.Context, number int) *pyjson.Object {
	return ctx.Store.Load().Obj("items").Obj(strconv.Itoa(number))
}

// ownershipHoldComments is the bot's comments on #`number` that are its hold comment.
func ownershipHoldComments(gh *fakes.FakeGitHub, number int) []string {
	var found []string
	for _, body := range gh.BotComments(number) {
		if strings.Contains(body, "**Held for") || strings.Contains(body, "**No longer held") ||
			strings.Contains(body, "**Released") || strings.Contains(body, "**Handed off") {
			found = append(found, body)
		}
	}
	return found
}

// ownershipCutOff is #12, owned by claude-1, whose build there is cut off by a refusal on its
// 5-hour window that resets `resets` after NIGHT; Muse is free beside it. Returns the harness.
func ownershipCutOff(t *testing.T, resets time.Duration) *flowHarness {
	t.Helper()
	h := newFlowHarness(t, flowHarnessOpts{Machine: []string{"muse"}})
	h.gh.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelBuild)})
	flowUpdateItem(h.ctx, 12, pyjson.New("owner", "claude-1", "owner_why", "it planned it"))
	planned, result := h.night(workFake(map[string]any{"build": workBuilder(map[string]any{"src/game.txt": "half\n"}),
		"review": flowLimited(clock.ISO(testsupport.Night.Add(resets)))}))
	// Its owner first, though Muse comes first in the medium tier's order (one lane, one owner).
	assertEqual(t, []any{planned.Get("provider"), result.Get("status")}, []any{"claude-1", "interrupted"})
	assertEqual(t, dig(t, ownershipRecord(h.ctx, 12), "paused", "provider"), "claude-1")
	return h
}

// ownershipHold is how long an item waits for its owner on the night bot: the committed
// `hold_minutes` (120 since #2903), which every hold below is measured against.
func ownershipHold() int { return testsupport.MakeConfig().HoldMinutes }

// ownershipWorld is a fake GitHub on the night bot's config with `env` and `machine`, at NIGHT.
func ownershipWorld(env map[string]string, machine []string, overrides ...*pyjson.Object) (*fakes.FakeGitHub, *hctx.Context) {
	gh := fakes.NewFakeGitHub()
	o := testsupport.Config{Env: env, Machine: machine}
	if len(overrides) > 0 {
		o.Overrides = overrides[0]
	}
	return gh, testsupport.MakeCtx(gh, testsupport.Ctx{Cfg: testsupport.MakeConfig(o)})
}

// ownershipQueued is #`number`, planned and queued to build, owned by `owner`.
func ownershipQueued(gh *fakes.FakeGitHub, ctx *hctx.Context, number int, owner string) {
	gh.AddIssue(number, fakes.Issue{Labels: flowLabels(config.LabelBuild)})
	flowUpdateItem(ctx, number, pyjson.New("planned_at", clock.ISO(testsupport.Day), "planned_tier", "strong",
		"owner", owner, "owner_why", "it planned it"))
}

// ownershipReading is `provider`'s last reading, taken five minutes before NIGHT: its session at
// `session` resetting `resets` minutes after NIGHT, and its week at `week` resetting `weekResets`
// minutes after it.
func ownershipReading(ctx *hctx.Context, provider string, session float64, resets int, week float64, weekResets int) {
	flowUpdate(ctx, func(s *pyjson.Object) {
		providers.Record(s, provider).Set("usage", pyjson.New(
			"five_hour", pyjson.New("utilization", session, "resets_at", clock.ISO(ownershipAt(resets))),
			"seven_day", pyjson.New("utilization", week, "resets_at", clock.ISO(ownershipAt(weekResets))),
			"observed_at", clock.ISO(ownershipAt(-5))))
	})
}

// The night bot holds items for two hours (#2903; an hour before it), `hold_minutes` in its
// `.harness/config.json`, never for Muse; Squishy holds none (its cases).
func TestOwnership_TheHoldIsTheNightBotsTwoHoursFromItsConfig(t *testing.T) {
	cfg := testsupport.MakeConfig()
	committed, _ := pyjson.AsInt(testsupport.RawConfig().Get("hold_minutes"))
	assertEqual(t, []any{cfg.HoldMinutes, committed, cfg.HoldExempt}, []any{120, 120, []string{"muse"}})
	assertTrue(t, cfg.HoldsFor("claude-1"))
	assertFalse(t, cfg.HoldsFor("muse"))
	off := testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("hold_minutes", 0)})
	assertFalse(t, off.Holds())
	assertFalse(t, off.HoldsFor("claude-1"))
}

func TestOwnership_ACutOffItemWaitsForItsOwnerWhoseResetIsNear(t *testing.T) {
	h := ownershipCutOff(t, 40*time.Minute)
	planned := flowMake(h.ctx)
	// Muse is free, but the item waits for its owner, whose 5-hour refusal ends in 40 minutes.
	assertEqual(t, planned.Get("action"), "none")
	assertIn(t, "waiting for their owners: #12 held for `claude-1` until 22:40 (it refused a call)",
		planned.Str("reason"))
	assertIn(t, config.LabelWaiting, h.gh.LabelNames(12))
	assertIn(t, config.LabelBuild, h.gh.LabelNames(12))
	hold := crossObj(t, ownershipRecord(h.ctx, 12), "hold")
	held := ownershipHold()
	assertEqual(t, []any{hold.Get("owner"), hold.Get("since"), hold.Get("until"), hold.Get("ends")},
		[]any{"claude-1", clock.ISO(ownershipAt(0)), clock.ISO(ownershipAt(40)), clock.ISO(ownershipAt(held))})
	// One comment says for whom, until when and why, and how to release it.
	said := ownershipHoldComments(h.gh, 12)
	assertEqual(t, len(said), 1)
	for _, part := range []string{"**Held for `claude-1`**", "until about 22:40 (in 40m)", "it refused a call",
		fmt.Sprintf("by %s (`hold_minutes`: %d)", plan.Clock(h.ctx, ownershipAt(held)), held), "`/harness handoff`:", "`/harness handoff medium` or `/harness handoff hard`",
		"`/harness handoff human`"} {
		assertIn(t, part, said[0])
	}
	assertTrue(t, py64(dig(t, ownershipRecord(h.ctx, 12), "waiting_comment")) != 0)
	// The queue, the status issue and `status` say so too.
	explained := waiting.Explain(h.ctx, h.ctx.Store.Load(), pyjson.New(), nil, "")
	assertEqual(t, explained.Rows[0].Waiting, "⏸️ held for `claude-1` until 22:40 (it refused a call)")
	assertIn(t, "- Waiting for their owners (`bot:waiting`; `/harness handoff` releases one): #12 held for "+
		"`claude-1` until 22:40 (it refused a call).", status.Report(h.ctx))
	// The sweep keeps it as it is: no second comment, no new hold.
	assertEqual(t, len(plan.SyncHolds(h.ctx, h.ctx.Store.Load(), plan.ReadLanes(h.ctx, h.ctx.Store.Load()),
		false, plan.AnyQuiet)), 1)
	assertEqual(t, len(ownershipHoldComments(h.gh, 12)), 1)
	assertEqual(t, dig(t, ownershipRecord(h.ctx, 12), "hold", "since"), clock.ISO(ownershipAt(0)))
}

// py64 is a JSON number as an int (0 for anything else).
func py64(v any) int {
	n, _ := pyjson.AsInt(v)
	return n
}

func TestOwnership_ACutOffItemIsHandedOffWhenItsOwnersResetIsFar(t *testing.T) {
	far := time.Duration(ownershipHold()+60) * time.Minute
	h := ownershipCutOff(t, far)
	planned := flowMake(h.ctx)
	// An hour more than an item waits for its owner: Muse takes it at once, and owns it.
	assertEqual(t, []any{planned.Get("action"), planned.Get("number"), planned.Get("provider")},
		[]any{"build", 12, "muse"})
	assertNotIn(t, config.LabelWaiting, h.gh.LabelNames(12))
	assertEqual(t, len(ownershipHoldComments(h.gh, 12)), 0)
	record := ownershipRecord(h.ctx, 12)
	assertEqual(t, record.Get("owner"), "muse")
	assertIn(t, "handed off from `claude-1`: `claude-1` it refused a call", record.Str("owner_why"))
	assertIn(t, fmt.Sprintf("it can start again only in %s, more than the %d minutes an item waits for its owner",
		clock.HumanDelta(far), ownershipHold()), record.Str("owner_why"))
	assertIn(t, "#12: handed off from its owner `claude-1`", strings.Join(stringsOf(planned.List("routing")), "\n"))
	assertFalse(t, record.Has("hold"))
}

// stringsOf is a JSON list of strings.
func stringsOf(list []any) []string {
	out := make([]string, 0, len(list))
	for _, v := range list {
		out = append(out, pyjson.Str(v))
	}
	return out
}

func TestOwnership_ItsSessionsLastHourIsWhenItCanStart(t *testing.T) {
	env := map[string]string{"HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN_4"}
	gh, ctx := ownershipWorld(env, []string{"muse"})
	ownershipQueued(gh, ctx, 12, "claude-4")
	// claude-4 is at its 70% cap; its session resets 40 minutes after the hold would end, and its
	// last hour, which lifts the cap (`last_hour`), starts 20 minutes before it: that is when it can
	// start, within the hold.
	resets := ownershipHold() + 40
	ownershipReading(ctx, "claude-4", 0.70, resets, 0.2, 3000)
	planned := flowMake(ctx)
	assertEqual(t, planned.Get("action"), "none")
	assertEqual(t, dig(t, ownershipRecord(ctx, 12), "hold", "until"), clock.ISO(ownershipAt(resets-60)))
	assertIn(t, fmt.Sprintf("#12 held for `claude-4` until %s (5-hour usage is 70%%", plan.Clock(ctx, ownershipAt(resets-60))),
		planned.Str("reason"))
	// Read past its cap, somebody else spent the session: no last hour, so it starts only at its
	// reset, past the hold, and Muse takes the item.
	gh2, ctx2 := ownershipWorld(env, []string{"muse"})
	ownershipQueued(gh2, ctx2, 12, "claude-4")
	ownershipReading(ctx2, "claude-4", 0.75, resets, 0.2, 3000)
	planned = flowMake(ctx2)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"build", "muse"})
	assertIn(t, "it can start again only in "+clock.HumanDelta(time.Duration(resets)*time.Minute),
		ownershipRecord(ctx2, 12).Str("owner_why"))
}

func TestOwnership_AWeeklyCapNeverHolds(t *testing.T) {
	env := map[string]string{"HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN_5"}
	// claude-5's week is over its 60% cap and resets in 30 minutes: it is never waited for.
	gh, ctx := ownershipWorld(env, []string{"muse"})
	ownershipQueued(gh, ctx, 12, "claude-5")
	ownershipReading(ctx, "claude-5", 0.1, 120, 0.62, 30)
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"build", "muse"})
	assertIn(t, "7-day usage is 62%", ownershipRecord(ctx, 12).Str("owner_why"))
	assertNotIn(t, config.LabelWaiting, gh.LabelNames(12))
	// Nor a refusal that ends with its week, though its session would hold it.
	gh, ctx = ownershipWorld(env, []string{"muse"})
	ownershipQueued(gh, ctx, 12, "claude-5")
	ownershipReading(ctx, "claude-5", 0.1, 120, 0.5, 30)
	flowUpdate(ctx, func(s *pyjson.Object) {
		providers.Record(s, "claude-5").Set("refused_until", clock.ISO(ownershipAt(30)))
	})
	planned = flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"build", "muse"})
	// One that ends with its session does hold it.
	gh, ctx = ownershipWorld(env, []string{"muse"})
	ownershipQueued(gh, ctx, 12, "claude-5")
	ownershipReading(ctx, "claude-5", 0.3, 30, 0.5, 3000)
	flowUpdate(ctx, func(s *pyjson.Object) {
		providers.Record(s, "claude-5").Set("refused_until", clock.ISO(ownershipAt(30)))
	})
	planned = flowMake(ctx)
	assertEqual(t, planned.Get("action"), "none")
	assertIn(t, config.LabelWaiting, gh.LabelNames(12))
	state := ctx.Store.Load()
	assertTrue(t, providers.WeeklyRefusal(ctx.Cfg.Pool.Get("claude-5"),
		providers.PeekRecord(state, "claude-5").Set("refused_until", clock.ISO(ownershipAt(3000))), ownershipAt(0)))
}

// ownershipMuseRefused is Muse refused on its 5-hour window until 20 minutes after NIGHT.
func ownershipMuseRefused(ctx *hctx.Context) {
	flowUpdate(ctx, func(s *pyjson.Object) {
		providers.Record(s, "muse").Set("refused_until", clock.ISO(ownershipAt(20)))
	})
}

func TestOwnership_MuseIsNeverWaitedFor(t *testing.T) {
	// Muse is refused until its session resets in 20 minutes; it is exempt (`hold_exempt`), so agy
	// takes its item at once.
	gh, ctx := ownershipWorld(nil, []string{"muse", "agy"})
	ownershipQueued(gh, ctx, 12, "muse")
	ownershipMuseRefused(ctx)
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"build", "agy"})
	assertNotIn(t, config.LabelWaiting, gh.LabelNames(12))
	// The exemption is the config's: without it, the item waits for Muse.
	gh, ctx = ownershipWorld(nil, []string{"muse", "agy"}, pyjson.New("hold_exempt", []any{}))
	ownershipQueued(gh, ctx, 12, "muse")
	ownershipMuseRefused(ctx)
	planned = flowMake(ctx)
	assertEqual(t, planned.Get("action"), "none")
	assertEqual(t, dig(t, ownershipRecord(ctx, 12), "hold", "owner"), "muse")
}

func TestOwnership_TheHoldLiftsWhenItsOwnerCanStart(t *testing.T) {
	h := ownershipCutOff(t, 40*time.Minute)
	assertEqual(t, flowMake(h.ctx).Get("action"), "none")
	// The refusal is over: the owner takes it at once, Muse free or not.
	testsupport.ClockOf(h.ctx).At = ownershipAt(41)
	planned := flowMake(h.ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("number"), planned.Get("provider")},
		[]any{"build", 12, "claude-1"})
	assertEqual(t, h.gh.LabelNames(12), setOf(config.LabelBuilding, config.LabelPlanned, "difficulty:medium"))
	said := ownershipHoldComments(h.gh, 12)
	assertEqual(t, len(said), 1)
	assertIn(t, "**No longer held**: `claude-1`, which owns this, can start again", said[0])
	record := ownershipRecord(h.ctx, 12)
	assertEqual(t, record.Get("owner"), "claude-1")
	assertFalse(t, record.Has("hold"))
}

func TestOwnership_AHoldThatRunsOutGoesToAnyone(t *testing.T) {
	h := ownershipCutOff(t, 40*time.Minute)
	assertEqual(t, flowMake(h.ctx).Get("action"), "none")
	// Ten minutes past its reset, claude-1 is refused again until an hour after the hold ends:
	// past the time the item waits, so it goes to Muse, which owns it from then on.
	held := ownershipHold()
	testsupport.ClockOf(h.ctx).At = ownershipAt(50)
	flowUpdate(h.ctx, func(s *pyjson.Object) {
		providers.Record(s, "claude-1").Set("refused_until", clock.ISO(ownershipAt(held+60)))
	})
	planned := flowMake(h.ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"build", "muse"})
	assertNotIn(t, config.LabelWaiting, h.gh.LabelNames(12))
	said := ownershipHoldComments(h.gh, 12)
	assertEqual(t, len(said), 1)
	assertIn(t, "**Released to anyone**: `claude-1` it refused a call", said[0])
	assertIn(t, fmt.Sprintf("the %d minutes this waited for it end at %s, before it can start again (in %s)",
		held, plan.Clock(h.ctx, ownershipAt(held)), clock.HumanDelta(time.Duration(held+10)*time.Minute)), said[0])
	record := ownershipRecord(h.ctx, 12)
	assertEqual(t, record.Get("owner"), "muse")
	assertIn(t, "it took it up; released from its hold: `claude-1` it refused a call", record.Str("owner_why"))
}

func TestOwnership_AHoldEndsAtOnceWhenItsOwnerCannotBeWaitedFor(t *testing.T) {
	h := ownershipCutOff(t, 40*time.Minute)
	assertEqual(t, flowMake(h.ctx).Get("action"), "none")
	// A person suspends claude-1: no reset lifts that, so the item goes to anyone now.
	flowUpdate(h.ctx, func(s *pyjson.Object) {
		providers.Record(s, "claude-1").Set("suspended", pyjson.New("by", "jgoetzmann", "at", clock.ISO(ownershipAt(0)),
			"reason", "using it myself"))
	})
	planned := flowMake(h.ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"build", "muse"})
	assertIn(t, "**Released to anyone**: `claude-1` suspended by @jgoetzmann", ownershipHoldComments(h.gh, 12)[0])
}

func TestOwnership_TheWaitingCommentIsWrittenOnceAndUpdated(t *testing.T) {
	h := ownershipCutOff(t, 40*time.Minute)
	flowMake(h.ctx)
	first := py64(ownershipRecord(h.ctx, 12).Get("waiting_comment"))
	// Lifted at its reset; claude-1 takes it, and is cut off again for half an hour.
	testsupport.ClockOf(h.ctx).At = ownershipAt(41)
	planned, result := h.night(workFake(map[string]any{"build": workBuilder(map[string]any{"src/game.txt": "more\n"}),
		"review": flowLimited(clock.ISO(ownershipAt(71)))}))
	assertEqual(t, []any{planned.Get("provider"), result.Get("status")}, []any{"claude-1", "interrupted"})
	assertEqual(t, flowMake(h.ctx).Get("action"), "none")
	// Held again: the same comment says so, and there is still one.
	said := ownershipHoldComments(h.gh, 12)
	assertEqual(t, len(said), 1)
	assertIn(t, "**Held for `claude-1`**, the subscription that owns this, until about 23:11", said[0])
	assertEqual(t, py64(ownershipRecord(h.ctx, 12).Get("waiting_comment")), first)
}

// ownershipHeld is #12 held for claude-1, whose session is at 97% until 22:40; Muse is free.
func ownershipHeld(t *testing.T) (*fakes.FakeGitHub, *hctx.Context) {
	t.Helper()
	gh, ctx := ownershipWorld(nil, []string{"muse"})
	ownershipQueued(gh, ctx, 12, "claude-1")
	ownershipReading(ctx, "claude-1", 0.97, 40, 0.2, 3000)
	assertEqual(t, flowMake(ctx).Get("action"), "none")
	assertIn(t, config.LabelWaiting, gh.LabelNames(12))
	return gh, ctx
}

// ownershipAsk is `body` on #`number` from the operator, at `level`.
func ownershipAsk(gh *fakes.FakeGitHub, ctx *hctx.Context, number int, body string, level int) string {
	return events.RunCommands(ctx, commands.Parse(body, ctx.Cfg.BotLogin), gh.GetIssue(number), fakes.Operator,
		level, nil)[0]
}

func TestOwnership_HandoffReleasesItToAnyone(t *testing.T) {
	gh, ctx := ownershipHeld(t)
	said := ownershipAsk(gh, ctx, 12, "/harness handoff", 2)
	assertIn(t, "Released from `claude-1`: the next subscription that may build it at difficulty:medium "+
		"takes it, and owns it from then on.", said)
	assertNotIn(t, config.LabelWaiting, gh.LabelNames(12))
	record := ownershipRecord(ctx, 12)
	assertEqual(t, []any{record.Get("owner"), record.Get("owner_why")}, []any{"", "released by @jgoetzmann"})
	assertFalse(t, record.Has("hold"))
	assertIn(t, "**Handed off** by @jgoetzmann: Released from `claude-1`", ownershipHoldComments(gh, 12)[0])
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"build", "muse"})
	assertEqual(t, ownershipRecord(ctx, 12).Get("owner"), "muse")
}

func TestOwnership_HandoffHardSetsThePersonsDifficulty(t *testing.T) {
	gh, ctx := ownershipHeld(t)
	thread := gh.Threads.Item(12)
	thread.Set("labels", append(thread.List("labels"), pyjson.New("name", "difficulty:medium")))
	flowUpdateItem(ctx, 12, pyjson.New("difficulty_by", pyjson.New("difficulty", "medium", "provider", "claude-1")))
	said := ownershipAsk(gh, ctx, 12, "@jgoetzmann-bot handoff hard", 2)
	assertIn(t, "Released from `claude-1`; it is `difficulty:hard` now, your label, which I never change", said)
	labels := gh.LabelNames(12)
	assertIn(t, "difficulty:hard", labels)
	assertNotIn(t, "difficulty:medium", labels)
	assertNotIn(t, config.LabelWaiting, labels)
	record := ownershipRecord(ctx, 12)
	assertFalse(t, record.Has("difficulty_by"))
	assertEqual(t, queue.RatingSource(labels, record), "person")
	assertEqual(t, record.Get("owner"), "")
	// `medium` works the same way, back down.
	assertIn(t, "No subscription owned it; it is `difficulty:medium` now", ownershipAsk(gh, ctx, 12, "/harness handoff medium", 2))
	assertEqual(t, queue.LabelledDifficulty(gh.LabelNames(12)), "medium")
}

func TestOwnership_HandoffHumanHandsItToPeople(t *testing.T) {
	gh, ctx := ownershipHeld(t)
	gh.Threads.Item(12).Set("assignees", []any{fakes.Bot.Copy()})
	said := ownershipAsk(gh, ctx, 12, "/harness handoff human", 2)
	assertIn(t, "Released from `claude-1` and handed to people: labelled `human`", said)
	labels := gh.LabelNames(12)
	assertIn(t, config.LabelHuman, labels)
	for _, gone := range []string{config.LabelBuild, config.LabelWaiting} {
		assertNotIn(t, gone, labels)
	}
	var assigned []string
	for _, a := range gh.Threads.Item(12).List("assignees") {
		assigned = append(assigned, pyjson.Obj(a).Str("login"))
	}
	assertEqual(t, sortedOf(assigned), []string{"MaxGoetzmann", "jgoetzmann"})
	assertEqual(t, ownershipRecord(ctx, 12).Get("owner"), "")
	assertEqual(t, flowMake(ctx).Get("action"), "none") // people's now
}

func TestOwnership_HandoffNeedsTheMaintainerLevel(t *testing.T) {
	gh, ctx := ownershipHeld(t)
	said := ownershipAsk(gh, ctx, 12, "/harness handoff", 1)
	assertEqual(t, said, "`handoff` needs the maintainer level; you have asker. Nothing was done.")
	assertIn(t, config.LabelWaiting, gh.LabelNames(12))
	assertEqual(t, ownershipRecord(ctx, 12).Get("owner"), "claude-1")
	assertIn(t, "takes `medium`, `hard` or `human`, or nothing", ownershipAsk(gh, ctx, 12, "/harness handoff claude-3", 2))
}

func TestOwnership_HandoffOnABotPullRequestReleasesItsIssue(t *testing.T) {
	gh, ctx := ownershipWorld(nil, []string{"muse"})
	gh.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelPROpen, "difficulty:hard")})
	gh.AddPull(20, "bot/issue-12", fakes.Pull{Labels: flowLabels(config.LabelPR, config.LabelRevise, "difficulty:hard")})
	flowUpdateItem(ctx, 12, pyjson.New("owner", "claude-1"))
	flowUpdateItem(ctx, 20, pyjson.New("issue", 12, "kind", "revise", "difficulty", "hard"))
	said := ownershipAsk(gh, ctx, 20, "/harness handoff medium", 2)
	assertIn(t, "Released from `claude-1`; it is `difficulty:medium` now", said)
	assertEqual(t, ownershipRecord(ctx, 12).Get("owner"), "")
	assertEqual(t, queue.LabelledDifficulty(gh.LabelNames(20)), "medium")
	assertEqual(t, queue.LabelledDifficulty(gh.LabelNames(12)), "medium")
	assertEqual(t, ownershipRecord(ctx, 20).Get("difficulty"), "medium")
	// So Muse may revise it now, and its revision makes Muse the item's owner.
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("number"), planned.Get("provider")},
		[]any{"revise", 20, "muse"})
	assertEqual(t, ownershipRecord(ctx, 12).Get("owner"), "muse")
}

func TestOwnership_AHandoffDuringARunFollowsItsDelivery(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Machine: []string{"muse"}})
	h.gh.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelBuild)})
	flowUpdateItem(h.ctx, 12, pyjson.New("owner", "claude-1"))
	planned := flowMake(h.ctx)
	assertEqual(t, planned.Get("provider"), "claude-1")
	said := ownershipAsk(h.gh, h.ctx, 12, "/harness handoff human", 2)
	assertEqual(t, said, "A run holds #12 now (`bot:building`), so it keeps it until it delivers; then I hand "+
		"it to people (`human`).")
	assertIn(t, config.LabelBuilding, h.gh.LabelNames(12))
	assertEqual(t, dig(t, ownershipRecord(h.ctx, 12), "handoff_requested", "to"), "human")
	// The run is cut off, delivers its work and goes back to the queue; then the handoff applies.
	out := filepath.Join(h.root, "out-run")
	r := workFake(map[string]any{"build": workBuilder(map[string]any{"src/game.txt": "half\n"}),
		"review": flowLimited(clock.ISO(ownershipAt(30)))})
	work.NewWorker(h.cfg, planned, r, h.clone, filepath.Join(h.root, "work"), out, nil, nil, nil, nil, nil, nil).Run()
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	labels := h.gh.LabelNames(12)
	assertIn(t, config.LabelHuman, labels)
	assertNotIn(t, config.LabelBuild, labels)
	record := ownershipRecord(h.ctx, 12)
	assertEqual(t, record.Get("owner"), "")
	assertFalse(t, record.Has("handoff_requested"))
	assertIn(t, "@jgoetzmann The run that held this has delivered, so: Released from `claude-1` and handed "+
		"to people", lastOf(t, h.gh.BotComments(12)))
}

func TestOwnership_ATimeBudgetPauseGoesOnOnTheSameSubscription(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Machine: []string{"muse"}})
	h.gh.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelBuild)})
	flowUpdateItem(h.ctx, 12, pyjson.New("planned_at", clock.ISO(testsupport.Day), "planned_tier", "strong",
		"owner", "claude-1", "owner_why", "it took it up"))
	planned := flowMake(h.ctx)
	assertEqual(t, planned.Get("provider"), "claude-1")
	// The work job stops itself at its time budget (`job_budget_minutes`) after the first build call.
	r := workFake(map[string]any{"build": workBuilder(map[string]any{"src/game.txt": "half\n"})})
	start := time.Now()
	now := func() time.Time {
		if len(r.Calls) > 0 {
			return start.Add(time.Duration(h.cfg.JobBudgetMinutes) * time.Minute)
		}
		return start
	}
	out := filepath.Join(h.root, "out-budget")
	work.NewWorker(h.cfg, planned, r, h.clone, filepath.Join(h.root, "work"), out, nil, nil, now, nil, nil, nil).Run()
	result := loadJSON(t, filepath.Join(out, "result.json"))
	assertEqual(t, []any{result.Get("status"), result.Get("interrupt")}, []any{"interrupted", "budget"})
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	assertEqual(t, dig(t, ownershipRecord(h.ctx, 12), "paused", "interrupt"), "budget")
	// Its lane is free again, and the next run goes on there at once, though Muse is free too and
	// comes first in the medium tier's order.
	assertEqual(t, len(h.gh.Dispatches), 1)
	next := flowMake(h.ctx)
	assertEqual(t, []any{next.Get("action"), next.Get("number"), next.Get("provider")},
		[]any{"build", 12, "claude-1"})
	assertIn(t, "its owner `claude-1` keeps it: one lane, one owner", strings.Join(stringsOf(next.List("routing")), "\n"))
}

// ownershipRevision is #12, owned by `owner`, and its pull request #20 waiting for a revision;
// #13 is a new build of higher priority.
func ownershipRevision(gh *fakes.FakeGitHub, ctx *hctx.Context, owner string) {
	gh.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelPROpen)})
	gh.AddPull(20, "bot/issue-12", fakes.Pull{Labels: flowLabels(config.LabelPR, config.LabelRevise)})
	flowUpdateItem(ctx, 12, pyjson.New("owner", owner, "pr", 20))
	flowUpdateItem(ctx, 20, pyjson.New("issue", 12, "kind", "revise", "source", "request"))
	ownershipQueued(gh, ctx, 13, "")
	thread := gh.Threads.Item(13)
	thread.Set("labels", append(thread.List("labels"), pyjson.New("name", config.LabelPriorityHigh)))
}

func TestOwnership_AFreedLaneTakesItsOwnRevisionBeforeANewBuild(t *testing.T) {
	gh, ctx := ownershipWorld(nil, nil)
	ownershipRevision(gh, ctx, "claude-1")
	planned := flowMake(ctx)
	// claude-1's own pull request goes before the new build, whatever its priority.
	assertEqual(t, []any{planned.Get("action"), planned.Get("number"), planned.Get("provider")},
		[]any{"revise", 20, "claude-1"})
	// One it does not own waits its turn: the higher priority first.
	gh, ctx = ownershipWorld(nil, nil)
	ownershipRevision(gh, ctx, "")
	planned = flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("number")}, []any{"build", 13})
}

func TestOwnership_AnOldRecordsOwnerIsItsPlanner(t *testing.T) {
	gh, ctx := ownershipWorld(nil, []string{"muse"})
	gh.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelPROpen)})
	gh.AddPull(20, "bot/issue-12", fakes.Pull{Labels: flowLabels(config.LabelPR, config.LabelRevise)})
	// A record from before owners were kept: no `owner`, the planner's `planned_by_id`.
	flowUpdateItem(ctx, 12, pyjson.New("planned_at", clock.ISO(testsupport.Day), "planned_by", "claude",
		"planned_by_id", "claude-1", "planned_tier", "strong", "pr", 20))
	flowUpdateItem(ctx, 20, pyjson.New("issue", 12, "kind", "revise", "source", "request"))
	assertEqual(t, queue.OwnerOf(ctx.Store.Load(), 20), "claude-1")
	assertEqual(t, queue.OwnerOf(ctx.Store.Load(), 12), "claude-1")
	candidates := queue.Candidates(ctx, ctx.Store.Load(), nil, nil)
	assertEqual(t, []any{candidates[0].Number, candidates[0].Owner}, []any{20, "claude-1"})
	// Its revision goes to its planner first, though Muse comes first in the medium tier's order,
	// and the claim writes the owner down.
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("number"), planned.Get("provider")},
		[]any{"revise", 20, "claude-1"})
	record := ownershipRecord(ctx, 12)
	assertEqual(t, []any{record.Get("owner"), record.Get("owner_why")}, []any{"claude-1", "it planned it"})
	// A released one (`owner` "") has none, whatever its planner.
	flowUpdate(ctx, func(s *pyjson.Object) { state.Item(s, 12).Set("owner", "") })
	assertEqual(t, queue.OwnerOf(ctx.Store.Load(), 20), "")
}

func TestOwnership_TheHandoffCommandIsReadAndListed(t *testing.T) {
	bot := "jgoetzmann-bot"
	var read [][3]any
	for _, c := range commands.Parse("/harness handoff\n/harness handoff hard\n@jgoetzmann-bot handoff human\n"+
		"@jgoetzmann-bot handoff to claude-3\n/harness handof", bot) {
		read = append(read, [3]any{c.Verb, c.Args, c.Level()})
	}
	assertEqual(t, read, [][3]any{{"handoff", "", 2}, {"handoff", "hard", 2}, {"handoff", "human", 2},
		{"request", "handoff to claude-3", 2}, {"typo", "handof", 1}})
	assertIn(t, "| `handoff [medium\\|hard\\|human]` | release this item from the subscription that owns it",
		commands.HelpText(bot, ""))
	help := commands.HelpText(bot, "handoff")
	assertIn(t, "**`handoff [medium|hard|human]`** (level 2, maintainer)", help)
	assertIn(t, "(`bot:waiting`)", help)
	assertIn(t, "For example: `/harness handoff hard`.", help)
	text := readText(t, filepath.Join(testsupport.Root, "bot", "README.md"))
	assertIn(t, "| `handoff [medium\\|hard\\|human]` |", text)
}

// A revision that answers a final audit goes to its owner, the auditor (#2748, section 4), on its
// seat of the audit's tier. An owner without that tier (a record from before the auditor owned its
// fix, when the builder did) cannot take it, so it is handed off at the item's own tier.
func TestOwnership_ARevisionAnsweringAnAuditIsItsAuditorsAndAHandoffIsAtTheItemsTier(t *testing.T) {
	pool := testsupport.MakeConfig().Pool
	claude1, claude3, muse := pool.Get("claude-1"), pool.Get("claude-3"), pool.Get("muse")
	// claude-3 comes first in the usage order, but claude-1 audited the item and owns its fix.
	seat, note := plan.BuilderSeat(pool, []*providers.Provider{muse, claude3, claude1}, "medium", false,
		"claude-1", "strong", nil)
	assertEqual(t, lanesSeatID(t, seat), []any{"claude-1", "strong"})
	assertIn(t, "its owner `claude-1` keeps it: one lane, one owner", note)
	seat, _ = plan.BuilderSeat(pool, []*providers.Provider{muse, claude3}, "medium", false, "muse", "strong", nil)
	assertEqual(t, lanesSeatID(t, seat), []any{"claude-3", "medium"})
}

// An item queued to start now (`--force`) waits for nobody: it goes to whoever can take it.
func TestOwnership_AForcedItemIsNeverHeld(t *testing.T) {
	gh, ctx := ownershipWorld(nil, []string{"muse"})
	ownershipQueued(gh, ctx, 12, "claude-1")
	flowUpdateItem(ctx, 12, pyjson.New("forced", true))
	ownershipReading(ctx, "claude-1", 0.97, 40, 0.2, 3000)
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"build", "muse"})
	assertNotIn(t, config.LabelWaiting, gh.LabelNames(12))
}

// Between plans, `bot:waiting` keeps the item for its owner in every reader that gives work out
// (the gate's question too), and the sweep releases it once its owner can no longer be waited for,
// so the next run takes it.
func TestOwnership_TheSweepKeepsTheHolds(t *testing.T) {
	gh, ctx := ownershipHeld(t)
	flowUpdate(ctx, func(s *pyjson.Object) {
		providers.Record(s, "claude-1").Set("suspended", pyjson.New("by", "jgoetzmann", "at", clock.ISO(ownershipAt(0))))
	})
	look := plan.Peek_(ctx, false, nil, "auto")
	assertFalse(t, look.Work, look.Reason)
	sweep.Sweep(ctx) // the first sweep only starts its clock
	notes := sweep.Sweep(ctx)
	assertIn(t, "#12 is released to anyone: `claude-1` suspended by @jgoetzmann", strings.Join(notes, "\n"))
	assertNotIn(t, config.LabelWaiting, gh.LabelNames(12))
	assertEqual(t, len(gh.Dispatches), 1) // and a run is started for it
	look = plan.Peek_(ctx, false, nil, "auto")
	assertEqual(t, []any{look.Work, look.Provider}, []any{true, "muse"})
}
