// The runner gate (bot/go/internal/cigate, bot/README.md "The runner gate"): a pull request's CI, a
// push to `main`'s and the super run wait in `ci-runners`, holding no runner, until the night bot's
// account, its one required reviewer, approves them once a run of their rung ends: only a few go at
// once, counted in pull requests, not runners. The merge queue's builds never wait.
package tests

import (
	"errors"
	"fmt"
	"regexp"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cardsets"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cigate"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cli"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/sweep"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

type cigateWorld struct {
	gh  *fakes.FakeGitHub
	ctx *hctx.Context
}

func newCigateWorld(t *testing.T) *cigateWorld {
	t.Helper()
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day})
	return &cigateWorld{gh: gh, ctx: ctx}
}

// waiting is a CI run of `event` that has waited `minutes` in `ci-runners`, which this account may
// approve.
func (w *cigateWorld) waiting(id int, event string, minutes int) {
	w.waitingIn(id, "ci.yml", event, minutes)
}

// waitingIn is `waiting` for a run of `workflow`.
func (w *cigateWorld) waitingIn(id int, workflow, event string, minutes int) {
	key := fmt.Sprint(id)
	w.gh.Runs.Set(key, pyjson.New("id", id, "name", workflow, "workflow", workflow, "status", "waiting",
		"event", event, "head_branch", fmt.Sprintf("branch-%d", id),
		"created_at", clock.ISO(testsupport.Day.Add(-time.Duration(minutes)*time.Minute))))
	w.gh.Pending[key] = []*pyjson.Object{pyjson.New("environment", pyjson.New("id", 77, "name", "ci-runners"),
		"current_user_can_approve", true)}
}

// running is runs `ids` of `workflow` and `event` going (approved, in progress).
func (w *cigateWorld) running(workflow, event string, ids ...int) {
	for _, id := range ids {
		w.gh.Runs.Set(fmt.Sprint(id), pyjson.New("id", id, "workflow", workflow, "status", "in_progress",
			"event", event, "head_branch", fmt.Sprintf("branch-%d", id)))
	}
}

func (w *cigateWorld) approved() []string {
	out := []string{}
	for _, a := range w.gh.Approvals {
		out = append(out, a.Run)
	}
	return out
}

// With a slot free, a pull request's run starts at once: approved in `ci-runners`, and no longer
// waiting.
func TestCigate_AWaitingRunStartsAtOnceWhenItsRungHasASlot(t *testing.T) {
	w := newCigateWorld(t)
	w.waiting(1, "pull_request", 0)
	note := cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"1"})
	assertEqual(t, w.gh.Approvals[0].Environments, []any{77})
	assertIn(t, "0 of its rung's 3 run(s) going", w.gh.Approvals[0].Comment)
	assertIn(t, "started 1 of 1 waiting run(s) (run 1 on branch-1)", note)
	assertEqual(t, w.gh.Runs.Obj("1").Get("status"), "queued")
	assertEqual(t, cigate.Tick(w.ctx), "") // nothing waits now
}

// It counts pull requests, not runners: `PullRuns` of `main`'s CI and pull requests' go at once,
// however many jobs are queued, and the next starts once one of them ends; jobs on other workflows'
// runs, and runs that never waited (a merge-queue build, a dispatch), take no slot.
func TestCigate_OnlyAFewPullRequestsRunsGoAtOnce(t *testing.T) {
	w := newCigateWorld(t)
	w.running("ci.yml", "pull_request", 1, 2)
	w.running("ci.yml", "push", 3)
	w.running("ci.yml", "merge_group", 4, 5, 6)
	w.running("ci.yml", "workflow_dispatch", 7)
	w.running("bot-night.yml", "workflow_dispatch", 8)
	w.waiting(10, "pull_request", 2)
	w.waiting(11, "pull_request", 1)
	assertEqual(t, cigate.PullRuns, 3)
	assertIn(t, "started 0 of 2 waiting run(s); 2 wait for a run of theirs to end", cigate.Tick(w.ctx))
	w.gh.Runs.Obj("1").Set("status", "completed")
	cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"10"})
	assertIn(t, "2 of its rung's 3 run(s) going", w.gh.Approvals[0].Comment)
	going, read := cigate.Going(w.ctx)
	assertEqual(t, going, [2]int{0, 3})
	assertEqual(t, read, true)
	w.gh.Runs.Obj("2").Set("status", "completed")
	w.gh.Runs.Obj("3").Set("status", "completed")
	cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"10", "11"})
}

// In the rung, `main`'s CI first, then a ready pull request's, then a draft's, each the oldest first;
// a run that waited `MaxWaitPull` goes first of all, so a draft never starves behind ready ones.
func TestCigate_MainFirstThenReadyPullRequestsThenDrafts(t *testing.T) {
	w := newCigateWorld(t)
	w.running("ci.yml", "pull_request", 1, 2)
	w.waiting(10, "pull_request", 9) // a draft, the oldest
	w.waiting(11, "pull_request", 3)
	w.waiting(12, "pull_request", 5)
	w.waiting(13, "push", 1)
	w.gh.CreatePull("Patch: ten", "branch-10", "main", "", true)
	w.gh.CreatePull("Patch: eleven", "branch-11", "main", "", false)
	w.gh.CreatePull("Patch: twelve", "branch-12", "main", "", false)
	for _, want := range [][]string{{"13"}, {"13", "12"}, {"13", "12", "11"}, {"13", "12", "11", "10"}} {
		cigate.Tick(w.ctx)
		assertEqual(t, w.approved(), want)
		w.gh.Runs.Obj(want[len(want)-1]).Set("status", "completed") // its run ends
	}
	// A draft that waited too long goes ahead of a ready pull request's run.
	w2 := newCigateWorld(t)
	w2.running("ci.yml", "pull_request", 1, 2)
	w2.waiting(10, "pull_request", int(cigate.MaxWaitPull/time.Minute))
	w2.waiting(11, "pull_request", 1)
	w2.waiting(12, "push", 0)
	w2.gh.CreatePull("Patch: ten", "branch-10", "main", "", true)
	cigate.Tick(w2.ctx)
	assertEqual(t, w2.approved(), []string{"10"})
	// Every pull request counts as ready when the open ones cannot be read.
	w3 := newCigateWorld(t)
	w3.running("ci.yml", "pull_request", 1, 2)
	w3.waiting(10, "pull_request", 9)
	w3.waiting(11, "pull_request", 5)
	w3.gh.CreatePull("Patch: ten", "branch-10", "main", "", true)
	w3.gh.Patch.ListPulls = func(string, string, int) []*pyjson.Object {
		panic(&herr.GitHubError{Msg: "rate limited", Status: 403})
	}
	cigate.Tick(w3.ctx)
	assertEqual(t, w3.approved(), []string{"10"})
}

// Staging's and production's runs (a super run, CI on a promotion's pull request into
// `_production`) have `ReleaseRuns` slots of their own: a full rung of pull requests holds none of
// them up, nor they a pull request's.
func TestCigate_StagingAndProductionHaveSlotsOfTheirOwn(t *testing.T) {
	w := newCigateWorld(t)
	w.running("ci.yml", "pull_request", 1, 2, 3)
	w.waiting(10, "pull_request", 9)
	w.waitingIn(11, "super.yml", "workflow_dispatch", 4)
	w.waiting(12, "pull_request", 3)
	w.gh.Runs.Obj("12").Set("pull_requests", []any{pyjson.New("base", pyjson.New("ref", "_production"))})
	w.waiting(13, "pull_request", 2)
	w.gh.Runs.Obj("13").Set("head_branch", "promote/2026-11-13-abc")
	assertEqual(t, cigate.ReleaseRuns, 2)
	assertIn(t, "started 2 of 4 waiting run(s) (super run 11 on branch-11, run 12 on branch-12); 2 wait",
		cigate.Tick(w.ctx))
	w.gh.Runs.Obj("11").Set("status", "completed")
	cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"11", "12", "13"})
	assertEqual(t, w.gh.Runs.Obj("10").Get("status"), "waiting")
	w.gh.Runs.Obj("1").Set("status", "completed")
	cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"11", "12", "13", "10"})
}

// When the runs going cannot be read, every waiting run starts: the gate delays CI, never stops it.
func TestCigate_EveryWaitingRunStartsWhenTheRunsGoingCannotBeRead(t *testing.T) {
	w := newCigateWorld(t)
	w.running("ci.yml", "pull_request", 1, 2, 3)
	w.waiting(10, "pull_request", 1)
	w.waiting(11, "pull_request", 2)
	w.gh.Patch.ListRepoRuns = func(status string, limit int, cached bool) []*pyjson.Object {
		if status == "waiting" {
			return w.gh.Own().ListRepoRuns(status, limit, cached)
		}
		panic(&herr.GitHubError{Msg: "rate limited", Status: 403})
	}
	cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"11", "10"})
	assertIn(t, "the runs going could not be read", w.gh.Approvals[0].Comment)
}

// It approves only `ci-runners`, and only what this account may approve; it leaves alone a run
// that is not waiting, a dispatch of CI's (a person's), a merge-queue build's (which never waits)
// and another workflow's, and it never runs for another bot (Squishy), which is not the
// environment's reviewer.
func TestCigate_ItApprovesOnlyItsOwnEnvironmentForTheGatedEvents(t *testing.T) {
	w := newCigateWorld(t)
	w.waiting(1, "pull_request", 1)
	w.gh.Pending["1"] = []*pyjson.Object{pyjson.New("environment", pyjson.New("id", 5, "name", "staging"),
		"current_user_can_approve", true)}
	w.waiting(2, "pull_request", 2)
	w.gh.Pending["2"][0].Set("current_user_can_approve", false)
	w.waiting(4, "workflow_dispatch", 4)
	w.waiting(5, "merge_group", 5)
	w.waitingIn(6, "deploy-staging.yml", "push", 6)
	note := cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{})
	assertIn(t, "run 1 on branch-1: it waits on nothing this account may approve", note)
	assertIn(t, "run 2 on branch-2: it waits on nothing this account may approve", note)
	assertIn(t, "of 2 waiting run(s)", note)
	// main's CI waits, and a super run, whatever started it.
	w.waiting(3, "push", 3)
	w.waitingIn(7, "super.yml", "workflow_dispatch", 1)
	w.waitingIn(8, "super.yml", "pull_request", 0)
	assertIn(t, "started 3 of 5 waiting run(s) (super run 7 on branch-7, super run 8 on branch-8, run 3 on branch-3)",
		cigate.Tick(w.ctx))
	squishy := newCigateWorld(t)
	squishy.ctx.Cfg.BotLogin = "squishy-squooby"
	squishy.waiting(1, "pull_request", 1)
	assertEqual(t, cigate.Tick(squishy.ctx), "")
	assertEqual(t, squishy.approved(), []string{})
}

// `--gate-every 30`, as bot-status runs it: the gate ticks every 30 seconds on a clock of its own
// (#3675), a rewrite's moment among them, the runners keep their two minutes, and a failed tick is
// only a warning.
func TestCigate_TheStatusLoopTicksTheGateOnAClockOfItsOwn(t *testing.T) {
	now := 0.0
	var ticks []string
	update := func(*hctx.Context, []string, bool, func() []string, bool, cardsets.Fetch) string {
		ticks = append(ticks, fmt.Sprintf("rewrite %g", now))
		return "rewrote #148"
	}
	refresh := func(*hctx.Context) string {
		ticks = append(ticks, fmt.Sprintf("runners %g", now))
		return "redrew #148's runners"
	}
	gate := func(*hctx.Context) string {
		ticks = append(ticks, fmt.Sprintf("gate %g", now))
		if now == 60 {
			panic(errors.New("network down"))
		}
		return "started 1 of 1 waiting run(s)"
	}
	patch[dashboardUpdateFunc](t, &dashboard.Update, update)
	patch(t, &dashboard.RefreshRunners, refresh)
	patch(t, &cigate.Tick, gate)
	dashboardOffline(t, nil)
	patch(t, &cli.Monotonic, func() float64 { return now })
	patch(t, &cli.Sleep, func(s float64) { now += s })
	patch(t, &cli.Background, inPlace)
	out, _ := capture(t, &cli.Stdout)
	cli.CmdDashboard(testsupport.MakeConfig(), &cli.Args{Every: 300, RunnersEvery: 120, GateEvery: 30, ForSeconds: 300})
	assertEqual(t, ticks, []string{"rewrite 0", "gate 30", "gate 60", "gate 90", "gate 120", "runners 120",
		"gate 150", "gate 180", "gate 210", "gate 240", "runners 240", "gate 270", "gate 300", "rewrite 300"})
	assertIn(t, "::warning::the runner gate did not tick: network down", out.String())
	assertIn(t, "runner gate: started 1 of 1 waiting run(s)", out.String())
}

// Every sweep ticks the gate too (bot-commands' scheduled one among them), so CI still starts while
// the status loop is down.
func TestCigate_EverySweepTicksTheGate(t *testing.T) {
	w := newCigateWorld(t)
	eventsUpdateState(w.ctx, "last_sweep", pyjson.New("since", clock.ISO(testsupport.Day.Add(-time.Hour))))
	w.waiting(1, "pull_request", 0)
	assertIn(t, "runner gate: started 1 of 1 waiting run(s) (run 1 on branch-1)", sweep.Sweep(w.ctx))
	assertEqual(t, w.approved(), []string{"1"})
}

// ci.yml's `changes` job, which every other job of the run needs, waits in the environment the
// repository variable names for a pull request's run and a push to `main`, the events cigate.Gated
// names (never a merge-queue build or a dispatch); super.yml's every first job (one that needs none) waits there too, but its schedule's,
// which only starts the daily run; and nothing waits with the variable unset. The status loop ticks
// the gate every 30 seconds.
func TestWorkflows_CIAndTheSuperRunWaitForTheRunnerGate(t *testing.T) {
	text := workflowsRead(t, "ci.yml")
	changes := workflowsJob(t, text, "changes")
	assertIn(t, "    environment: ${{ (github.event_name == 'pull_request' || github.event_name == 'push') && "+
		"vars.CI_GATE_ENVIRONMENT || '' }}\n", changes)
	assertEqual(t, strings.Count(text, "environment:"), 1, "only `changes` names an environment")
	assertEqual(t, cigate.Gated["ci.yml"], []string{"pull_request", "push"})
	super := workflowsRead(t, "super.yml")
	firsts := 0
	for _, m := range regexp.MustCompile(`(?m)^  ([a-z0-9-]+):\n`).FindAllStringSubmatch(super[strings.Index(super, "\njobs:\n"):], -1) {
		job := workflowsJob(t, super, m[1])
		if m[1] == "daily" || strings.Contains(job, "    needs:") {
			assertNotIn(t, "environment:", job)
			continue
		}
		assertIn(t, "    environment: ${{ vars.CI_GATE_ENVIRONMENT || '' }}\n", job)
		firsts++
	}
	assertEqual(t, firsts, 6) // rust-build, wasm, daily-tests, coverage, notebooks, db
	assertEqual(t, cigate.Gated["super.yml"], []string{"pull_request", "workflow_dispatch"})
	supersede := workflowsJob(t, text, "queue-supersede")
	assertNotIn(t, "needs:", supersede) // it cancels older builds at once, beside `changes`
	status := workflowsRead(t, "bot-status.yml")
	assertIn(t, `GATE_EVERY_SECONDS: "30"`, status)
	assertIn(t, `--gate-every "${GATE_EVERY_SECONDS}"`, status)
}
