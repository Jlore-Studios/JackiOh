// #294: `/harness suspend <subscription>` starts no new work on one subscription until
// `/harness resume <subscription>`, `--force` included; a halt and a suspension never lift each
// other.
//
// bot/tests/test_suspend.py.
package tests

import (
	"path/filepath"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cli"
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
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

const (
	suspendTrust   = "jgoetzmann 3 id:95732896\nhelper 2\n"
	suspendBotName = "jgoetzmann-bot"
)

var suspendHelper = pyjson.New("login", "helper", "id", 7)

// suspendSuspend is suspend(ctx, provider_id, reason="").
func suspendSuspend(ctx *hctx.Context, providerID string, reason ...string) {
	why := ""
	if len(reason) > 0 {
		why = reason[0]
	}
	held := pyjson.New("by", "jgoetzmann", "at", clock.ISO(ctx.Now()), "reason", why)
	ctx.Store.Update(func(s *pyjson.Object) any {
		providers.Record(s, providerID).Update(pyjson.New("suspended", held))
		return nil
	}, "state")
}

// suspendVerbs is verbs(body): each command's (verb, args).
func suspendVerbs(body string) [][2]string {
	out := [][2]string{}
	for _, c := range commands.Parse(body, suspendBotName) {
		out = append(out, [2]string{c.Verb, c.Args})
	}
	return out
}

// ---------------------------------------------------------------------------- ParseTests

func TestSuspend_SuspendIsAnOperatorCommandAfterEitherPrefix(t *testing.T) {
	assertEqual(t, suspendVerbs("/harness suspend claude-3 using it myself"),
		[][2]string{{"suspend", "claude-3 using it myself"}})
	assertEqual(t, suspendVerbs("@jgoetzmann-bot suspend claude-3"), [][2]string{{"suspend", "claude-3"}})
	assertEqual(t, suspendVerbs("@jgoetzmann-bot suspend: claude-3 using it myself"),
		[][2]string{{"suspend", "claude-3 using it myself"}})
	assertEqual(t, suspendVerbs("@jgoetzmann-bot suspend the old animation")[0][0], "request")
	assertEqual(t, commands.Parse("/harness suspend gpt", suspendBotName)[0].Level(), 3)
}

func TestSuspend_ResumeWithASubscriptionIsStartWithItsName(t *testing.T) {
	assertEqual(t, suspendVerbs("/harness resume claude-3"), [][2]string{{"start", "claude-3"}})
	assertEqual(t, suspendVerbs("@jgoetzmann-bot resume claude-3"), [][2]string{{"start", "claude-3"}})
	assertEqual(t, suspendVerbs("@jgoetzmann-bot start with option A")[0][0], "request")
}

func TestSuspend_HelpExplainsSuspend(t *testing.T) {
	text := commands.HelpText(suspendBotName, "suspend")
	assertIn(t, "`suspend <subscription> [reason]`", text)
	assertIn(t, "For example: `@jgoetzmann-bot suspend: claude-3", text)
	assertIn(t, "`suspend <subscription> [reason]`", commands.HelpText(suspendBotName, ""))
}

// ---------------------------------------------------------------------------- CommandTests

// suspendCommands is CommandTests' world.
type suspendCommands struct {
	t   *testing.T
	gh  *fakes.FakeGitHub
	ctx *hctx.Context
	cid int
}

func newSuspendCommands(t *testing.T) *suspendCommands {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5)
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night, TrustText: ptr(suspendTrust)})
	return &suspendCommands{t: t, gh: gh, ctx: ctx, cid: 600}
}

// send is CommandTests.send(body, user=OPERATOR, association="OWNER").
func (w *suspendCommands) send(body string, as ...any) {
	user, association := fakes.Operator, "OWNER"
	if len(as) == 2 {
		user, association = as[0].(*pyjson.Object), as[1].(string)
	}
	// A fresh comment id each time: a line already answered on one id never runs again.
	w.cid++
	events.Handle(w.ctx, "issue_comment", pyjson.New(
		"action", "created", "sender", user, "issue", pyjson.New("number", 5, "title", "t"),
		"comment", pyjson.New("id", w.cid, "body", body, "user", user,
			"author_association", association)))
}

func (w *suspendCommands) reply() string {
	w.t.Helper()
	return lastOf(w.t, w.gh.BotComments(5))
}

// why is CommandTests.why(provider_id, forced=False): "" is None.
func (w *suspendCommands) why(providerID string, forced ...bool) string {
	cfg := w.ctx.Cfg
	return providers.Availability(cfg.Pool.Get(providerID), w.ctx.Store.Load(), testsupport.Night,
		cfg.Timezone, cfg.Secrets, len(forced) > 0 && forced[0], false)
}

func (w *suspendCommands) suspended(providerID string) *pyjson.Object {
	return providers.Suspension(w.ctx.Store.Load(), providerID)
}

func TestSuspend_SuspendAndResumeOneSubscription(t *testing.T) {
	w := newSuspendCommands(t)
	assertEqual(t, w.why("claude-1"), "") // None
	w.send("/harness suspend claude-1 using it myself")
	assertIn(t, "Suspended `claude-1`", w.reply())
	held := dig(t, w.ctx.Store.Load(), "providers", "claude-1", "suspended")
	assertEqual(t, []any{dig(t, held, "by"), dig(t, held, "reason")}, []any{"jgoetzmann", "using it myself"})
	assertEqual(t, w.why("claude-1"), "suspended by @jgoetzmann (using it myself); "+
		"`/harness resume claude-1` lifts it")
	assertIn(t, "suspended", w.why("claude-1", true))
	assertIn(t, "suspended by @jgoetzmann", status.Report(w.ctx))
	assertFalse(t, w.ctx.Store.Load().Get("halted"))
	w.send("/harness resume claude-1")
	assertIn(t, "Resumed `claude-1`", w.reply())
	assertNotIn(t, "suspended", dig(t, w.ctx.Store.Load(), "providers", "claude-1"))
	assertEqual(t, w.why("claude-1"), "") // None
}

func TestSuspend_TheMentionFormSuspendsWithAReasonAfterAColon(t *testing.T) {
	w := newSuspendCommands(t)
	w.send("@jgoetzmann-bot suspend: claude-2 for the demo")
	assertEqual(t, dig(t, w.suspended("claude-2"), "reason"), "for the demo")
	w.send("@jgoetzmann-bot resume claude-2")
	assertIsNone(t, w.suspended("claude-2"))
}

func TestSuspend_SuspendNeedsTheOperator(t *testing.T) {
	w := newSuspendCommands(t)
	w.send("/harness suspend claude-1", suspendHelper, "COLLABORATOR")
	assertIn(t, "operator level", w.reply())
	assertIsNone(t, w.suspended("claude-1"))
}

func TestSuspend_AnUnknownOrMissingSubscriptionChangesNothing(t *testing.T) {
	w := newSuspendCommands(t)
	w.send("/harness suspend claude-9")
	assertIn(t, "`claude-9` is not a subscription", w.reply())
	assertIn(t, "`claude-1`", w.reply())
	w.send("/harness suspend")
	assertIn(t, "needs a subscription", w.reply())
	assertEqual(t, dig(t, w.ctx.Store.Load(), "providers"), pyjson.New())
}

func TestSuspend_ResumeOfASubscriptionLeavesAHaltAlone(t *testing.T) {
	w := newSuspendCommands(t)
	w.send("/harness halt away")
	w.send("/harness suspend claude-1")
	w.send("/harness resume claude-1")
	assertIn(t, "still halted", w.reply())
	assertTrue(t, dig(t, w.ctx.Store.Load(), "halted"))
	assertIsNone(t, w.suspended("claude-1"))
	w.send("/harness resume over")
	assertIn(t, "`over` is not a subscription", w.reply())
	assertTrue(t, dig(t, w.ctx.Store.Load(), "halted"))
	// A bare start, or one with a note that names no subscription, lifts the halt as it always
	// did, and never a suspension.
	w.send("/harness suspend gpt")
	w.send("/harness start now, back from vacation")
	assertIn(t, "Started.", w.reply())
	assertFalse(t, dig(t, w.ctx.Store.Load(), "halted"))
	assertIsNotNone(t, w.suspended("gpt"))
	w.send("/harness halt again")
	w.send("/harness start")
	assertFalse(t, dig(t, w.ctx.Store.Load(), "halted"))
	assertIsNotNone(t, w.suspended("gpt"))
}

func TestSuspend_ResumeOfOneNotSuspendedSaysSo(t *testing.T) {
	w := newSuspendCommands(t)
	w.send("/harness resume claude-1")
	assertIn(t, "`claude-1` is not suspended", w.reply())
}

// ---------------------------------------------------------------------------- SchedulingTests

func newSuspendScheduling() (*fakes.FakeGitHub, *hctx.Context) {
	gh := fakes.NewFakeGitHub()
	return gh, testsupport.MakeCtx(gh)
}

// suspendProbe is make_probe(ctx, number, provider)(None): the reason and kind, or nil (None).
func suspendProbe(ctx *hctx.Context, number int, provider *providers.Provider) any {
	reason, kind, ok := cli.MakeProbe(ctx, &number, provider, nil)(nil)
	if !ok {
		return nil
	}
	return [2]string{reason, kind}
}

func TestSuspend_ASuspendedSubscriptionTakesNoWorkEvenForced(t *testing.T) {
	gh, ctx := newSuspendScheduling()
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	suspendSuspend(ctx, "claude-1")
	planned := plan.Make(ctx, true, nil, "auto", nil)
	assertEqual(t, dig(t, planned, "action"), "none")
	assertIn(t, "`claude-1` suspended by @jgoetzmann", dig(t, planned, "reason"))
	ctx.Store.Update(func(s *pyjson.Object) any {
		if _, ok := providers.Record(s, "claude-1").Pop("suspended"); !ok {
			t.Fatal(`KeyError: 'suspended'`)
		}
		return nil
	}, "state")
	assertEqual(t, dig(t, plan.Make(ctx, false, nil, "auto", nil), "action"), "build")
}

func TestSuspend_TheProbeStopsARunOnASuspendedSubscription(t *testing.T) {
	gh, ctx := newSuspendScheduling()
	gh.AddIssue(5)
	pool := ctx.Cfg.Pool
	assertIsNone(t, suspendProbe(ctx, 5, pool.Get("claude-1")))
	suspendSuspend(ctx, "claude-1")
	assertEqual(t, suspendProbe(ctx, 5, pool.Get("claude-1")),
		[2]string{"`claude-1` was suspended by @jgoetzmann", "suspend"})
	assertIsNone(t, suspendProbe(ctx, 5, pool.Get("claude-2")))
}

func TestSuspend_TheStatusIssueShowsASuspendedAccountAsOff(t *testing.T) {
	_, ctx := newSuspendScheduling()
	suspendSuspend(ctx, "claude-1")
	assertEqual(t, dashboardHeldBy(ctx, ctx.Store.Load(), ctx.Cfg.Pool.Get("claude-1")),
		[2]string{"off", "⚫ suspended"})
}

// ---------------------------------------------------------------------------- RunTests

// suspendRunWithProbe is RunTests.run_with_probe.
func suspendRunWithProbe(t *testing.T, h *flowHarness, fake *runner.FakeRunner) (*pyjson.Object, *pyjson.Object) {
	t.Helper()
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	assertEqual(t, dig(t, planned, "provider"), "claude-1")
	out := filepath.Join(h.root, "out-suspend")
	number := 12
	result := work.NewWorker(h.cfg, planned, fake, h.clone, filepath.Join(h.root, "work"), out,
		cli.MakeProbe(h.ctx, &number, h.cfg.Pool.Get("claude-1"), nil), nil, nil, nil, nil, nil).Run()
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	return planned, result
}

// suspendCount is int(record.get(key) or 0).
func suspendCount(t *testing.T, record *pyjson.Object, key string) int {
	t.Helper()
	if !record.Truthy(key) {
		return 0
	}
	n, ok := pyjson.AsInt(record.Get(key))
	if !ok {
		t.Fatalf("%s is %v", key, record.Get(key))
	}
	return n
}

func TestSuspend_ARunGoingWhenItsSubscriptionIsSuspendedPausesUncharged(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	building := func(request *runner.RunRequest) *runner.RunResult {
		suspendSuspend(h.ctx, "claude-1")
		return workBuilder(map[string]string{"src/game.txt": "v2\n"})(request)
	}
	fake := workFake(map[string]runner.Handler{"build": building, "review": workReviewer(workApprove)})
	_, result := suspendRunWithProbe(t, h, fake)
	assertEqual(t, []any{dig(t, result, "status"), dig(t, result, "interrupt")}, []any{"interrupted", "suspend"})
	roles := []string{}
	for _, c := range fake.Calls {
		roles = append(roles, c.Role)
	}
	assertEqual(t, roles, []string{"plan", "build"}) // no review
	assertIn(t, config.LabelBuild, h.gh.LabelNames(12))
	assertIn(t, "Paused: `claude-1` was suspended by @jgoetzmann", lastOf(t, h.gh.BotComments(12)))
	record := state.Item(h.ctx.Store.Load(), 12)
	assertEqual(t, suspendCount(t, record, "interruptions"), 0)
	assertEqual(t, suspendCount(t, record, "failures"), 0)
	assertIsNotNone(t, h.originSHA("bot/issue-12")) // the work so far is kept
}

func TestSuspend_AFinishedChangeIsStillHandedOn(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	reviewing := func(request *runner.RunRequest) *runner.RunResult {
		suspendSuspend(h.ctx, "claude-1")
		return workReviewer(workApprove)(request)
	}
	fake := workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": reviewing})
	_, result := suspendRunWithProbe(t, h, fake)
	assertEqual(t, dig(t, result, "status"), "approved")
	assertEqual(t, len(h.gh.ListPulls("open", "bot/issue-12", 200)), 1)
}
