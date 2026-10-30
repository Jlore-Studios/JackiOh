// quiet_test.go is bot/tests/test_quiet.py: starting only when the subscription is quiet, and never
// mistaking the partner bot for a person.
package tests

import (
	"slices"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/quiet"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// T0
var quietT0 = time.Date(2026, 9, 30, 3, 0, 0, 0, time.UTC)

// SETTINGS
var quietSettings = &config.Quiet{Enabled: true, IntervalMinutes: 10, MaxWaitMinutes: 40, PingModel: "haiku",
	Partners: []*config.Partner{}}

// PARTNER
var quietPartner = &config.Partner{Repo: "jgoetzmann/bright-bots-harness", Workflows: pyjson.New(
	"implement.yml", []any{"Run planned items"}, "discover.yml", []any{"Discover and propose"})}

// quietUsage is usage(five, seven=0.5, resets="2026-09-30T05:00:00Z", status="allowed").
func quietUsage(five float64, opts ...string) *pyjson.Object {
	return quietUsageOf(five, 0.5, opts...)
}

// quietUsageOf is usage() with `seven` given; `opts` are resets then status.
func quietUsageOf(five, seven float64, opts ...string) *pyjson.Object {
	resets, status := "2026-09-30T05:00:00Z", "allowed"
	if len(opts) > 0 && opts[0] != "" {
		resets = opts[0]
	}
	if len(opts) > 1 {
		status = opts[1]
	}
	return pyjson.New("five_hour", pyjson.New("utilization", five, "resets_at", resets),
		"seven_day", pyjson.New("utilization", seven, "resets_at", "2026-10-02T00:00:00Z"),
		"status", status)
}

// quietWorld is World: a fake clock that `sleep` advances, and a queue of usage readings (nil is
// None).
type quietWorld struct {
	at       time.Time
	readings []*pyjson.Object
	pings    int
	sleeps   []float64
}

func newQuietWorld(readings []*pyjson.Object) *quietWorld {
	return &quietWorld{at: quietT0, readings: slices.Clone(readings), sleeps: []float64{}}
}

func (w *quietWorld) now() time.Time { return w.at }

func (w *quietWorld) sleep(seconds float64) {
	w.sleeps = append(w.sleeps, seconds)
	w.at = w.at.Add(time.Duration(seconds * float64(time.Second)))
}

func (w *quietWorld) ping() *pyjson.Object {
	w.pings++
	if len(w.readings) == 0 {
		return nil
	}
	first := w.readings[0]
	w.readings = w.readings[1:]
	return first
}

// quietVerdict is verdict(readings, partner=lambda t1, t2: None).
func quietVerdict(readings []*pyjson.Object, partner func(t1, t2 time.Time) string) (*quiet.Verdict, *quietWorld) {
	if partner == nil {
		partner = func(t1, t2 time.Time) string { return "" }
	}
	world := newQuietWorld(readings)
	return quiet.WaitForQuiet(quietSettings, world.ping, partner, world.now, world.sleep), world
}

// ------------------------------------------------------------------ CompareTests

// quietS is CompareTests.s(five, seven=0.5, resets="r1", status="allowed").
func quietS(five, seven float64, resets, status string) *quiet.Sample {
	return &quiet.Sample{At: quietT0, FiveHour: &five, SevenDay: &seven, FiveResets: resets, SevenResets: "w",
		Status: status}
}

func TestQuiet_Cases(t *testing.T) {
	assertEqual(t, quiet.Compare(quietS(0.10, 0.5, "r1", "allowed"), quietS(0.10, 0.5, "r1", "allowed")), "quiet")
	assertEqual(t, quiet.Compare(quietS(0.10, 0.5, "r1", "allowed"), quietS(0.11, 0.5, "r1", "allowed")), "rose")
	assertEqual(t, quiet.Compare(quietS(0.10, 0.50, "r1", "allowed"), quietS(0.10, 0.51, "r1", "allowed")), "rose")
	assertEqual(t, quiet.Compare(quietS(0.40, 0.5, "r1", "allowed"), quietS(0.02, 0.5, "r2", "allowed")), "inconclusive")
	assertEqual(t, quiet.Compare(quietS(0.10, 0.5, "r1", "allowed"), quietS(0.10, 0.5, "r1", "rejected")), "refused")
}

// ------------------------------------------------------------------ WaitTests

func TestQuiet_QuietAfterOneInterval(t *testing.T) {
	result, world := quietVerdict([]*pyjson.Object{quietUsage(0.10), quietUsage(0.10)}, nil)
	assertTrue(t, result.Quiet)
	assertEqual(t, world.pings, 2)
	assertEqual(t, world.sleeps, []float64{600.0})
}

func TestQuiet_ARiseWaitsAndLooksAgain(t *testing.T) {
	result, world := quietVerdict([]*pyjson.Object{quietUsage(0.10), quietUsage(0.13), quietUsage(0.13)}, nil)
	assertTrue(t, result.Quiet)
	assertEqual(t, world.pings, 3)
	assertEqual(t, len(result.Samples), 3)
}

func TestQuiet_ARiseWhileThePartnerSpendsIsExcused(t *testing.T) {
	seen := [][2]time.Time{}
	partner := func(t1, t2 time.Time) string {
		seen = append(seen, [2]time.Time{t1, t2})
		return "jgoetzmann/bright-bots-harness implement.yml: Run planned items"
	}
	result, _ := quietVerdict([]*pyjson.Object{quietUsage(0.10), quietUsage(0.20)}, partner)
	assertTrue(t, result.Quiet)
	assertIn(t, "Run planned items", result.ExcusedBy)
	assertEqual(t, len(seen), 1)
	assertTrue(t, seen[0][0].Equal(quietT0) && seen[0][1].Equal(quietT0.Add(10*time.Minute)), seen)
}

func TestQuiet_APersonWhoKeepsWorkingHoldsTheRunBack(t *testing.T) {
	readings := []*pyjson.Object{}
	for i := range 10 {
		readings = append(readings, quietUsage(0.10+0.02*float64(i)))
	}
	result, world := quietVerdict(readings, nil)
	assertFalse(t, result.Quiet)
	assertIn(t, "40 minutes", result.Reason)
	assertEqual(t, world.pings, 5) // one reading, then one every ten minutes for forty
}

func TestQuiet_AWindowThatTurnedOverIsReadAgain(t *testing.T) {
	result, world := quietVerdict([]*pyjson.Object{quietUsage(0.60, "a"), quietUsage(0.01, "b"),
		quietUsage(0.01, "b")}, nil)
	assertTrue(t, result.Quiet)
	assertEqual(t, world.pings, 3)
}

func TestQuiet_RefusedAndUnreadable(t *testing.T) {
	result, _ := quietVerdict([]*pyjson.Object{quietUsage(0.10), quietUsage(0.10, "", "rejected")}, nil)
	assertFalse(t, result.Quiet)
	assertIn(t, "refused", result.Reason)
	result, world := quietVerdict([]*pyjson.Object{nil}, nil)
	assertTrue(t, result.Quiet) // no reading at all: going ahead, as the partner does
	assertIn(t, "going ahead", result.Reason)
	assertEqual(t, world.sleeps, []float64{})
	result, _ = quietVerdict([]*pyjson.Object{quietUsage(0.10), nil, quietUsage(0.10)}, nil)
	assertTrue(t, result.Quiet) // a reading missing later is skipped, not a verdict
}

// quietReader is FakeReader.
type quietReader struct {
	runs map[string][]*pyjson.Object
	jobs map[any][]*pyjson.Object
	fail bool
}

func (r *quietReader) ListRuns(workflow string, limit int) []*pyjson.Object {
	if r.fail {
		herr.Raise(&herr.GitHubError{Msg: "boom", Status: 500})
	}
	if runs, ok := r.runs[workflow]; ok {
		return runs
	}
	return []*pyjson.Object{}
}

func (r *quietReader) ListJobs(runID any) []*pyjson.Object {
	if jobs, ok := r.jobs[runID]; ok {
		return jobs
	}
	return []*pyjson.Object{}
}

// quietISO is the step's `iso`: the zero time is None.
func quietISO(d time.Time) any {
	if d.IsZero() {
		return nil
	}
	return d.Format("2006-01-02T15:04:05Z")
}

// quietStep is step(name, started, ended=None); the zero time is None.
func quietStep(name string, started time.Time, ended ...time.Time) *pyjson.Object {
	var end time.Time
	if len(ended) > 0 {
		end = ended[0]
	}
	return pyjson.New("name", name, "started_at", quietISO(started), "completed_at", quietISO(end))
}

// ------------------------------------------------------------------ PartnerTests

// PartnerTests.t1, t2
var quietT1, quietT2 = quietT0, quietT0.Add(10 * time.Minute)

// quietSpending is PartnerTests.spending(steps, status="in_progress", updated=None).
func quietSpending(steps []*pyjson.Object, status string, updated time.Time) string {
	if status == "" {
		status = "in_progress"
	}
	if updated.IsZero() {
		updated = quietT2
	}
	runs := map[string][]*pyjson.Object{"implement.yml": {pyjson.New("id", 1, "status", status,
		"updated_at", updated.Format("2006-01-02T15:04:05Z"))}}
	jobs := map[any][]*pyjson.Object{1: {pyjson.New("name", "implement", "steps", pyjson.ListOf(steps))}}
	return quiet.PartnerSpending([]*config.Partner{quietPartner},
		func(repo string) quiet.RunReader { return &quietReader{runs: runs, jobs: jobs} }, quietT1, quietT2)
}

func TestQuiet_ASpendingStepInsideTheInterval(t *testing.T) {
	found := quietSpending([]*pyjson.Object{quietStep("Run planned items (harness run --item)", quietT1.Add(-time.Hour))},
		"", time.Time{})
	assertIn(t, "Run planned items", found)
}

func TestQuiet_AStepGithubSkippedSpentNothing(t *testing.T) {
	skipped := quietStep("Run planned items", quietT1)
	skipped.Set("conclusion", "skipped")
	assertEqual(t, quietSpending([]*pyjson.Object{skipped}, "", time.Time{}), "") // None
}

func TestQuiet_AStepThatEndedBeforeOrNeverStarted(t *testing.T) {
	before := quietStep("Run planned items", quietT1.Add(-2*time.Hour), quietT1.Add(-time.Minute))
	assertEqual(t, quietSpending([]*pyjson.Object{before}, "", time.Time{}), "")
	assertEqual(t, quietSpending([]*pyjson.Object{quietStep("Run planned items", time.Time{})}, "", time.Time{}), "")
}

func TestQuiet_WaitingAtThePartnersOwnGateIsNotSpending(t *testing.T) {
	gate := quietStep("Wait until the subscription is quiet (harness quiet)", quietT1)
	assertEqual(t, quietSpending([]*pyjson.Object{gate}, "", time.Time{}), "")
}

func TestQuiet_AnOldFinishedRunIsNotReadAndErrorsExcuseNothing(t *testing.T) {
	old := quietT1.Add(-3 * time.Hour)
	assertEqual(t, quietSpending([]*pyjson.Object{quietStep("Run planned items", old.Add(-time.Hour))},
		"completed", old), "")
	failing := quiet.PartnerSpending([]*config.Partner{quietPartner},
		func(repo string) quiet.RunReader {
			return &quietReader{runs: map[string][]*pyjson.Object{}, jobs: map[any][]*pyjson.Object{}, fail: true}
		}, quietT1, quietT2)
	assertEqual(t, failing, "")
}

// quietClient is the reader's Client: a token refuses, no token answers.
type quietClient struct{ token string }

func (c *quietClient) ListRuns(workflow string, status string, limit int, cached bool) []*pyjson.Object {
	if c.token != "" {
		herr.Raise(&herr.GitHubError{Msg: "Resource not accessible by integration", Status: 403})
	}
	return []*pyjson.Object{pyjson.New("id", 7)}
}

func (c *quietClient) ListJobs(runID any, cached bool) []*pyjson.Object {
	panic(&fakes.PyError{Type: "AttributeError", Msg: "'Client' object has no attribute 'list_jobs'"})
}

func TestQuiet_TheReaderFallsBackToNoToken(t *testing.T) {
	made := []string{}
	factory := func(repo, token string) quiet.ActionsReader {
		made = append(made, token)
		return &quietClient{token: token}
	}
	reader := quiet.NewPartnerReader("x/y", "actions-token", factory)
	assertEqual(t, reader.ListRuns("implement.yml", 10), []*pyjson.Object{pyjson.New("id", 7)})
	assertEqual(t, made, []string{"actions-token", ""})
}

// ------------------------------------------------------------------ PingTests

func TestQuiet_ThePingIsOneTurnOfTheSmallModelInAnEmptyDirectory(t *testing.T) {
	var argv []string
	var cwd string
	var env map[string]string
	run := func(a []string, args runner.RunArgs) (*runner.Completed, error) {
		argv, cwd, env = a, args.Cwd, args.Env
		return &runner.Completed{Stdout: strings.Join([]string{
			pyjson.Dumps(pyjson.New("type", "rate_limit_event", "rate_limit_info", pyjson.New(
				"status", "allowed", "unifiedWindows", pyjson.New(
					"five_hour", pyjson.New("utilization", 0.12, "resetsAt", 1790710800))))),
			pyjson.Dumps(pyjson.New("type", "result", "result", "ok")),
		}, "\n")}, nil
	}
	found := runner.PingUsage("claude", "haiku", run, "", "")
	assertAlmostEqual(t, found.Obj("five_hour").Float("utilization"), 0.12)
	assertIn(t, "--max-turns", argv)
	assertEqual(t, argv[slices.Index(argv, "--model")+1], "haiku")
	assertIn(t, "bot-ping-", cwd)
	assertNotIn(t, "BOT_GITHUB_TOKEN", env)
}

// ------------------------------------------------------------------ PeekTests
//
// `peek` reads what `make` would do, without doing it.

// quietLabels is json.dumps({n: t["labels"] for n, t in gh.threads.items()}, sort_keys=True).
func quietLabels(gh *fakes.FakeGitHub) string {
	all := pyjson.New()
	for n, thread := range gh.Threads.All() {
		all.Set(strconv.Itoa(n), thread.Get("labels"))
	}
	return pyjson.DumpsWith(all, pyjson.Options{SortKeys: true})
}

// quietAgree is PeekTests.agree: peek and make agree, and peek changes no label.
func quietAgree(t *testing.T, gh *fakes.FakeGitHub, at time.Time) (bool, string, bool) {
	t.Helper()
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: at})
	before := quietLabels(gh)
	look := plan.Peek_(ctx, false, nil, "auto")
	after := quietLabels(gh)
	assertEqual(t, before, after, "peek changed labels")
	planned := plan.Make(ctx, false, nil, "auto", nil)
	assertEqual(t, look.Work, planned.Str("action") != "none", look.Reason, " ", pyjson.Dumps(planned))
	return look.Work, look.Reason, look.Forced
}

func TestQuiet_AQueuedIssue(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	work, _, _ := quietAgree(t, gh, testsupport.Night)
	assertEqual(t, work, true)
}

func TestQuiet_NothingByDay(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	work, _, _ := quietAgree(t, gh, testsupport.Day)
	assertEqual(t, work, false)
}

func TestQuiet_AForcedItemSkipsTheQuietCheck(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day})
	ctx.Store.Update(func(s *pyjson.Object) any { state.Item(s, 3).Set("forced", true); return nil }, "state")
	look := plan.Peek_(ctx, false, nil, "auto")
	assertEqual(t, []any{look.Work, look.Reason, look.Forced, look.QuietProvider},
		[]any{true, "#3 (medium) is queued to build, for `claude-1`: plan on " +
			"`claude-1` (claude, `opus`, strong); build on `claude-1` (claude, " +
			"`sonnet`, medium); review on `claude-1` (claude, `opus`, strong)",
			true, ""})
	idle := testsupport.MakeCtx(fakes.NewFakeGitHub(), testsupport.Ctx{At: testsupport.Day})
	testsupport.IdleFor(idle)
	look = plan.Peek_(idle, true, nil, "auto")
	assertEqual(t, []bool{look.Work, look.Forced}, []bool{true, true})
}

func TestQuiet_ASurveyWhenIdle(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	assertFalse(t, plan.Peek_(testsupport.MakeCtx(gh), false, nil, "auto").Work) // idle, but not for 30 minutes yet
	assertTrue(t, plan.Peek_(testsupport.MakeCtx(gh), false, nil, "auto").Idle)
	ctx := testsupport.MakeCtx(gh)
	testsupport.IdleFor(ctx)
	look := plan.Peek_(ctx, false, nil, "auto")
	assertTrue(t, look.Work)
	assertIn(t, "survey", look.Reason)
}

func TestQuiet_ADeadRunsItemCountsAsWork(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelWorking}})
	gh.Runs.Set("5", pyjson.New("status", "completed"))
	ctx := testsupport.MakeCtx(gh)
	ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, 3).Set("run_id", "5")
		s.Set("suggest", pyjson.New("last_run", "2026-09-30T02:59:00Z", "requested", false))
		return nil
	}, "state")
	look := plan.Peek_(ctx, false, nil, "auto")
	assertTrue(t, look.Work)
	assertIn(t, "left working", look.Reason)
}

func TestQuiet_AConflictedBotPRCountsAsWork(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddPull(9, "bot/issue-2", fakes.Pull{Labels: []string{config.LabelPR}}).Set("mergeable_state", "dirty")
	ctx := testsupport.MakeCtx(gh)
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.Set("suggest", pyjson.New("last_run", "2026-09-30T02:59:00Z", "requested", false))
		return nil
	}, "state")
	assertIn(t, "conflicts with main", plan.Peek_(ctx, false, nil, "auto").Reason)
}

func TestQuiet_TheCommittedConfig(t *testing.T) {
	cfg := testsupport.MakeConfig()
	assertTrue(t, cfg.Quiet.Enabled)
	assertEqual(t, [2]int{cfg.Quiet.IntervalMinutes, cfg.Quiet.MaxWaitMinutes}, [2]int{10, 120})
	partner := cfg.Quiet.Partners[0]
	assertEqual(t, partner.Repo, "jgoetzmann/bright-bots-harness")
	steps := partner.Workflows.Get("implement.yml")
	if s, ok := steps.([]string); ok {
		steps = pyjson.ListOf(s)
	}
	assertIn(t, "Run planned items", steps)
}
