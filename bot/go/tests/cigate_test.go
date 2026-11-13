// The runner gate (bot/go/internal/cigate, bot/README.md "The runner gate"): only a few CI and CD
// runs go at once, counted in runs, not runners, and the slots go in the owner's order: the merge
// queue's builds and the deploys never wait and take theirs first, then staging's and production's
// checks, then `main`'s CI, then pull requests'. A run waits for its slot in `ci-runners`, holding no
// runner, until the night bot's account, its one required reviewer, approves it.
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
func TestCigate_AWaitingRunStartsAtOnceWhenASlotIsFree(t *testing.T) {
	w := newCigateWorld(t)
	w.waiting(1, "pull_request", 0)
	note := cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"1"})
	assertEqual(t, w.gh.Approvals[0].Environments, []any{77})
	assertIn(t, "0 of 6 slot(s) taken", w.gh.Approvals[0].Comment)
	assertIn(t, "started 1 of 1 waiting run(s) (run 1 on branch-1)", note)
	assertEqual(t, w.gh.Runs.Obj("1").Get("status"), "queued")
	assertEqual(t, cigate.Tick(w.ctx), "") // nothing waits now
}

// It counts runs, not runners: `Slots` CI and CD runs go at once, every kind together, whatever
// their jobs. The merge queue's builds, the deploys and promotions and a person's dispatch of CI
// never wait but take their slots; other workflows' runs take none. The next run starts once one
// ends.
func TestCigate_OnlyAFewRunsGoAtOnceCountedInRuns(t *testing.T) {
	w := newCigateWorld(t)
	assertEqual(t, cigate.Slots, 6)
	w.running("ci.yml", "merge_group", 1, 2, 3)
	w.running("deploy-staging.yml", "push", 4)
	w.running("ci.yml", "workflow_dispatch", 5)
	w.running("ci.yml", "pull_request", 6)
	w.running("bot-night.yml", "workflow_dispatch", 7)
	w.running("selfplay.yml", "schedule", 8)
	w.waiting(10, "pull_request", 2)
	assertIn(t, "started 0 of 1 waiting run(s); 1 wait for a slot", cigate.Tick(w.ctx))
	going, read := cigate.Read(w.ctx)
	assertEqual(t, going.All, 6)
	assertEqual(t, going.Rung, [3]int{0, 0, 1})
	assertEqual(t, read, true)
	w.gh.Runs.Obj("4").Set("status", "completed") // the deploy ends
	cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"10"})
	assertIn(t, "5 of 6 slot(s) taken", w.gh.Approvals[0].Comment)
	// The promotions count as the deploy does.
	w2 := newCigateWorld(t)
	w2.running("promote-staging.yml", "workflow_run", 1)
	w2.running("promote-production.yml", "workflow_run", 2)
	w2.running("super.yml", "schedule", 3)
	going, _ = cigate.Read(w2.ctx)
	assertEqual(t, going.All, 3)
}

// The owner's order: staging's and production's checks (a super run, CI on a promotion's pull
// request into `_production`), at most `ReleaseRuns` of them, then `main`'s CI, then pull
// requests', which always have `PullMin` slot(s) however full the pool.
func TestCigate_StagingAndProductionThenMainThenPullRequests(t *testing.T) {
	w := newCigateWorld(t)
	w.running("ci.yml", "merge_group", 1, 2, 3)
	w.waiting(10, "pull_request", 9) // a draft, the oldest
	w.gh.CreatePull("Patch: ten", "branch-10", "main", "", true)
	w.waiting(11, "pull_request", 8)
	w.waiting(12, "push", 1)
	w.waitingIn(13, "super.yml", "workflow_dispatch", 3)
	w.waiting(14, "pull_request", 2)
	w.gh.Runs.Obj("14").Set("pull_requests", []any{pyjson.New("base", pyjson.New("ref", "_production"))})
	w.waitingIn(15, "super.yml", "pull_request", 1)
	w.gh.Runs.Obj("15").Set("head_branch", "promote/2026-11-13-abc")
	assertEqual(t, cigate.ReleaseRuns, 2)
	assertEqual(t, cigate.PullMin, 1)
	cigate.Tick(w.ctx)
	// Two of staging's and production's (the oldest), main's, and one pull request's past a full pool.
	assertEqual(t, w.approved(), []string{"13", "14", "12", "11"})
	// The pool holds seven now (the pull request's past it): the draft waits, and once the pool has a
	// slot, the third check goes before it.
	w.gh.Runs.Obj("13").Set("status", "completed")
	cigate.Tick(w.ctx)
	assertEqual(t, len(w.approved()), 4)
	w.gh.Runs.Obj("1").Set("status", "completed")
	cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"13", "14", "12", "11", "15"})
	w.gh.Runs.Obj("11").Set("status", "completed")
	cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"13", "14", "12", "11", "15", "10"})
}

// In the pull requests' rung, a ready one's before a draft's, each the oldest first, but a run that
// waited `MaxWaitPull` goes first, so a draft never starves behind ready ones; every pull request
// counts as ready when the open ones cannot be read.
func TestCigate_AReadyPullRequestsRunGoesBeforeADraftsUnlessTheDraftWaitedTooLong(t *testing.T) {
	w := newCigateWorld(t)
	w.running("ci.yml", "merge_group", 1, 2, 3, 4, 5, 6) // a full pool: one pull request's at a time
	w.waiting(10, "pull_request", 9)                     // a draft, the oldest
	w.waiting(11, "pull_request", 3)
	w.waiting(12, "pull_request", 5)
	w.gh.CreatePull("Patch: ten", "branch-10", "main", "", true)
	w.gh.CreatePull("Patch: eleven", "branch-11", "main", "", false)
	w.gh.CreatePull("Patch: twelve", "branch-12", "main", "", false)
	for _, want := range [][]string{{"12"}, {"12", "11"}, {"12", "11", "10"}} {
		cigate.Tick(w.ctx)
		assertEqual(t, w.approved(), want)
		w.gh.Runs.Obj(want[len(want)-1]).Set("status", "completed") // its run ends
	}
	w2 := newCigateWorld(t)
	w2.running("ci.yml", "merge_group", 1, 2, 3, 4, 5, 6)
	w2.waiting(10, "pull_request", int(cigate.MaxWaitPull/time.Minute))
	w2.waiting(11, "pull_request", 1) // ready
	w2.gh.CreatePull("Patch: ten", "branch-10", "main", "", true)
	cigate.Tick(w2.ctx)
	assertEqual(t, w2.approved(), []string{"10"})
	w3 := newCigateWorld(t)
	w3.running("ci.yml", "merge_group", 1, 2, 3, 4, 5, 6)
	w3.waiting(10, "pull_request", 9)
	w3.waiting(11, "pull_request", 5)
	w3.gh.CreatePull("Patch: ten", "branch-10", "main", "", true)
	w3.gh.Patch.ListPulls = func(string, string, int) []*pyjson.Object {
		panic(&herr.GitHubError{Msg: "rate limited", Status: 403})
	}
	cigate.Tick(w3.ctx)
	assertEqual(t, w3.approved(), []string{"10"})
}

// When the runs going cannot be read, every waiting run starts: the gate delays CI, never stops it.
func TestCigate_EveryWaitingRunStartsWhenTheRunsGoingCannotBeRead(t *testing.T) {
	w := newCigateWorld(t)
	w.running("ci.yml", "merge_group", 1, 2, 3, 4, 5, 6)
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
	// The deploys and promotions the gate counts, and lets start at once, are workflows that exist.
	for _, name := range cigate.Deploys {
		assertNotIn(t, "vars.CI_GATE_ENVIRONMENT", workflowsRead(t, name))
	}
	supersede := workflowsJob(t, text, "queue-supersede")
	assertNotIn(t, "needs:", supersede) // it cancels older builds at once, beside `changes`
	status := workflowsRead(t, "bot-status.yml")
	assertIn(t, `GATE_EVERY_SECONDS: "30"`, status)
	assertIn(t, `--gate-every "${GATE_EVERY_SECONDS}"`, status)
}
