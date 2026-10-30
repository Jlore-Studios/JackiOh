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
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
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
	eventsEqual(t, suspendVerbs("/harness suspend claude-3 using it myself"),
		[][2]string{{"suspend", "claude-3 using it myself"}})
	eventsEqual(t, suspendVerbs("@jgoetzmann-bot suspend claude-3"), [][2]string{{"suspend", "claude-3"}})
	eventsEqual(t, suspendVerbs("@jgoetzmann-bot suspend: claude-3 using it myself"),
		[][2]string{{"suspend", "claude-3 using it myself"}})
	eventsEqual(t, suspendVerbs("@jgoetzmann-bot suspend the old animation")[0][0], "request")
	eventsEqual(t, commands.Parse("/harness suspend gpt", suspendBotName)[0].Level(), 3)
}

func TestSuspend_ResumeWithASubscriptionIsStartWithItsName(t *testing.T) {
	eventsEqual(t, suspendVerbs("/harness resume claude-3"), [][2]string{{"start", "claude-3"}})
	eventsEqual(t, suspendVerbs("@jgoetzmann-bot resume claude-3"), [][2]string{{"start", "claude-3"}})
	eventsEqual(t, suspendVerbs("@jgoetzmann-bot start with option A")[0][0], "request")
}

func TestSuspend_HelpExplainsSuspend(t *testing.T) {
	text := commands.HelpText(suspendBotName, "suspend")
	eventsIn(t, "`suspend <subscription> [reason]`", text)
	eventsIn(t, "For example: `@jgoetzmann-bot suspend: claude-3", text)
	eventsIn(t, "`suspend <subscription> [reason]`", commands.HelpText(suspendBotName, ""))
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
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night, TrustText: eventsStr(suspendTrust)})
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
	return eventsLast(w.t, w.gh.BotComments(5))
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
	eventsEqual(t, w.why("claude-1"), "") // None
	w.send("/harness suspend claude-1 using it myself")
	eventsIn(t, "Suspended `claude-1`", w.reply())
	held := eventsDig(t, w.ctx.Store.Load(), "providers", "claude-1", "suspended")
	eventsEqual(t, []any{eventsDig(t, held, "by"), eventsDig(t, held, "reason")}, []any{"jgoetzmann", "using it myself"})
	eventsEqual(t, w.why("claude-1"), "suspended by @jgoetzmann (using it myself); "+
		"`/harness resume claude-1` lifts it")
	eventsIn(t, "suspended", w.why("claude-1", true))
	eventsIn(t, "suspended by @jgoetzmann", status.Report(w.ctx))
	eventsFalse(t, w.ctx.Store.Load().Get("halted"))
	w.send("/harness resume claude-1")
	eventsIn(t, "Resumed `claude-1`", w.reply())
	eventsNotIn(t, "suspended", eventsDig(t, w.ctx.Store.Load(), "providers", "claude-1"))
	eventsEqual(t, w.why("claude-1"), "") // None
}

func TestSuspend_TheMentionFormSuspendsWithAReasonAfterAColon(t *testing.T) {
	w := newSuspendCommands(t)
	w.send("@jgoetzmann-bot suspend: claude-2 for the demo")
	eventsEqual(t, eventsDig(t, w.suspended("claude-2"), "reason"), "for the demo")
	w.send("@jgoetzmann-bot resume claude-2")
	eventsNone(t, w.suspended("claude-2"))
}

func TestSuspend_SuspendNeedsTheOperator(t *testing.T) {
	w := newSuspendCommands(t)
	w.send("/harness suspend claude-1", suspendHelper, "COLLABORATOR")
	eventsIn(t, "operator level", w.reply())
	eventsNone(t, w.suspended("claude-1"))
}

func TestSuspend_AnUnknownOrMissingSubscriptionChangesNothing(t *testing.T) {
	w := newSuspendCommands(t)
	w.send("/harness suspend claude-9")
	eventsIn(t, "`claude-9` is not a subscription", w.reply())
	eventsIn(t, "`claude-1`", w.reply())
	w.send("/harness suspend")
	eventsIn(t, "needs a subscription", w.reply())
	eventsEqual(t, eventsDig(t, w.ctx.Store.Load(), "providers"), pyjson.New())
}

func TestSuspend_ResumeOfASubscriptionLeavesAHaltAlone(t *testing.T) {
	w := newSuspendCommands(t)
	w.send("/harness halt away")
	w.send("/harness suspend claude-1")
	w.send("/harness resume claude-1")
	eventsIn(t, "still halted", w.reply())
	eventsTrue(t, eventsDig(t, w.ctx.Store.Load(), "halted"))
	eventsNone(t, w.suspended("claude-1"))
	w.send("/harness resume over")
	eventsIn(t, "`over` is not a subscription", w.reply())
	eventsTrue(t, eventsDig(t, w.ctx.Store.Load(), "halted"))
	// A bare start, or one with a note that names no subscription, lifts the halt as it always
	// did, and never a suspension.
	w.send("/harness suspend gpt")
	w.send("/harness start now, back from vacation")
	eventsIn(t, "Started.", w.reply())
	eventsFalse(t, eventsDig(t, w.ctx.Store.Load(), "halted"))
	eventsNotNone(t, w.suspended("gpt"))
	w.send("/harness halt again")
	w.send("/harness start")
	eventsFalse(t, eventsDig(t, w.ctx.Store.Load(), "halted"))
	eventsNotNone(t, w.suspended("gpt"))
}

func TestSuspend_ResumeOfOneNotSuspendedSaysSo(t *testing.T) {
	w := newSuspendCommands(t)
	w.send("/harness resume claude-1")
	eventsIn(t, "`claude-1` is not suspended", w.reply())
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
	eventsEqual(t, eventsDig(t, planned, "action"), "none")
	eventsIn(t, "`claude-1` suspended by @jgoetzmann", eventsDig(t, planned, "reason"))
	ctx.Store.Update(func(s *pyjson.Object) any {
		if _, ok := providers.Record(s, "claude-1").Pop("suspended"); !ok {
			t.Fatal(`KeyError: 'suspended'`)
		}
		return nil
	}, "state")
	eventsEqual(t, eventsDig(t, plan.Make(ctx, false, nil, "auto", nil), "action"), "build")
}

func TestSuspend_TheProbeStopsARunOnASuspendedSubscription(t *testing.T) {
	gh, ctx := newSuspendScheduling()
	gh.AddIssue(5)
	pool := ctx.Cfg.Pool
	eventsNone(t, suspendProbe(ctx, 5, pool.Get("claude-1")))
	suspendSuspend(ctx, "claude-1")
	eventsEqual(t, suspendProbe(ctx, 5, pool.Get("claude-1")),
		[2]string{"`claude-1` was suspended by @jgoetzmann", "suspend"})
	eventsNone(t, suspendProbe(ctx, 5, pool.Get("claude-2")))
}

func TestSuspend_TheStatusIssueShowsASuspendedAccountAsOff(t *testing.T) {
	_, ctx := newSuspendScheduling()
	suspendSuspend(ctx, "claude-1")
	style, text := dashboard.Account(ctx, ctx.Store.Load(), ctx.Cfg.Pool.Get("claude-1"))
	eventsEqual(t, [2]string{style, text}, [2]string{"off", "⚫ suspended"})
}

// ---------------------------------------------------------------------------- RunTests

// suspendRunWithProbe is RunTests.run_with_probe.
func suspendRunWithProbe(t *testing.T, h *ratingHarness, fake *runner.FakeRunner) (*pyjson.Object, *pyjson.Object) {
	t.Helper()
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	eventsEqual(t, eventsDig(t, planned, "provider"), "claude-1")
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
	h := newRatingHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	building := func(request *runner.RunRequest) *runner.RunResult {
		suspendSuspend(h.ctx, "claude-1")
		return ratingBuilder(map[string]string{"src/game.txt": "v2\n"})(request)
	}
	fake := ratingRunner(map[string]runner.Handler{"build": building, "review": ratingReviewer(ratingApprove)})
	_, result := suspendRunWithProbe(t, h, fake)
	eventsEqual(t, []any{eventsDig(t, result, "status"), eventsDig(t, result, "interrupt")}, []any{"interrupted", "suspend"})
	roles := []string{}
	for _, c := range fake.Calls {
		roles = append(roles, c.Role)
	}
	eventsEqual(t, roles, []string{"plan", "build"}) // no review
	eventsIn(t, config.LabelBuild, h.gh.LabelNames(12))
	eventsIn(t, "Paused: `claude-1` was suspended by @jgoetzmann", eventsLast(t, h.gh.BotComments(12)))
	record := state.Item(h.ctx.Store.Load(), 12)
	eventsEqual(t, suspendCount(t, record, "interruptions"), 0)
	eventsEqual(t, suspendCount(t, record, "failures"), 0)
	eventsNotNone(t, h.originSHA("bot/issue-12")) // the work so far is kept
}

func TestSuspend_AFinishedChangeIsStillHandedOn(t *testing.T) {
	h := newRatingHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	reviewing := func(request *runner.RunRequest) *runner.RunResult {
		suspendSuspend(h.ctx, "claude-1")
		return ratingReviewer(ratingApprove)(request)
	}
	fake := ratingRunner(map[string]runner.Handler{"build": ratingBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": reviewing})
	_, result := suspendRunWithProbe(t, h, fake)
	eventsEqual(t, eventsDig(t, result, "status"), "approved")
	eventsEqual(t, len(h.gh.ListPulls("open", "bot/issue-12", 200)), 1)
}
