// The runner gate (bot/go/internal/cigate, bot/README.md "The runner gate"): a pull request's CI and
// a merge-queue build wait in `ci-runners`, holding no runner, until the night bot's account, its
// one required reviewer, approves them, once GitHub's runners have room: merge-queue builds first,
// each the oldest first, and any that waited too long whatever is queued.
package tests

import (
	"errors"
	"fmt"
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
	key := fmt.Sprint(id)
	w.gh.Runs.Set(key, pyjson.New("id", id, "name", "CI", "workflow", "ci.yml", "status", "waiting",
		"event", event, "head_branch", fmt.Sprintf("branch-%d", id),
		"created_at", clock.ISO(testsupport.Day.Add(-time.Duration(minutes)*time.Minute))))
	w.gh.Pending[key] = []*pyjson.Object{pyjson.New("environment", pyjson.New("id", 77, "name", "ci-runners"),
		"current_user_can_approve", true)}
}

// busy is a run going with `hosted` jobs queued for GitHub's runners and `boxed` for the bot's own.
func (w *cigateWorld) busy(hosted, boxed int) {
	var jobs []*pyjson.Object
	for i := 0; i < hosted; i++ {
		jobs = append(jobs, pyjson.New("id", 900+i, "status", "queued", "labels", []any{"ubuntu-latest"}))
	}
	for i := 0; i < boxed; i++ {
		jobs = append(jobs, pyjson.New("id", 950+i, "status", "queued", "labels", []any{"night-vm-gpt"}))
	}
	jobs = append(jobs, pyjson.New("id", 999, "status", "in_progress", "labels", []any{"ubuntu-latest"}))
	w.gh.Runs.Set("500", pyjson.New("id", 500, "name", "CI", "status", "in_progress", "event", "push"))
	w.gh.Jobs["500"] = jobs
}

// started is: run `id` has run its first job (`cigate.FirstJob`) and asked for the rest, `queued`
// of which wait for one of GitHub's runners.
func (w *cigateWorld) started(id int, queued int) {
	jobs := []*pyjson.Object{pyjson.New("id", id*100, "name", cigate.FirstJob, "status", "completed",
		"labels", []any{"ubuntu-latest"})}
	for i := 0; i < queued; i++ {
		jobs = append(jobs, pyjson.New("id", id*100+1+i, "name", fmt.Sprintf("job %d", i), "status", "queued",
			"labels", []any{"ubuntu-latest"}))
	}
	w.gh.Jobs[fmt.Sprint(id)] = jobs
}

func (w *cigateWorld) approved() []string {
	out := []string{}
	for _, a := range w.gh.Approvals {
		out = append(out, a.Run)
	}
	return out
}

// With room, a pull request's run starts at once: approved in `ci-runners`, and no longer waiting.
func TestCigate_AWaitingRunStartsAtOnceWhenGitHubsRunnersHaveRoom(t *testing.T) {
	w := newCigateWorld(t)
	w.waiting(1, "pull_request", 0)
	w.busy(2, 0)
	note := cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"1"})
	assertEqual(t, w.gh.Approvals[0].Environments, []any{77})
	assertIn(t, "2 job(s) queued for a runner", w.gh.Approvals[0].Comment)
	assertIn(t, "started 1 of 1 waiting run(s)", note)
	assertEqual(t, w.gh.Runs.Obj("1").Get("status"), "queued")
	assertEqual(t, cigate.Tick(w.ctx), "") // nothing waits now
}

// Busy, a pull request's run waits; jobs queued for the bot's own boxes do not count.
func TestCigate_APullRequestsRunWaitsWhileGitHubsRunnersAreBusy(t *testing.T) {
	w := newCigateWorld(t)
	w.waiting(1, "pull_request", 5)
	w.busy(cigate.PullLimit, 0)
	assertIn(t, "started 0 of 1 waiting run(s); 1 wait for room", cigate.Tick(w.ctx))
	assertEqual(t, w.approved(), []string{})
	w.busy(cigate.PullLimit-1, 10)
	cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"1"})
}

// Merge-queue builds go first, and start on a busier queue than a pull request's run; pull
// requests' runs start one a tick, the oldest first, and a run approved counts for its jobs.
func TestCigate_MergeQueueBuildsGoFirstThenPullRequestsOldestFirst(t *testing.T) {
	w := newCigateWorld(t)
	w.waiting(1, "pull_request", 3)
	w.waiting(2, "pull_request", 9) // older
	w.waiting(3, "merge_group", 1)
	w.busy(cigate.PullLimit, 0) // too busy for a pull request's run, not for a queue build
	cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"3"})
	w.busy(0, 0)
	w.started(3, 0)
	cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"3", "2"})
	w.started(2, 0)
	cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"3", "2", "1"})
	// A second queue build starts in the same tick only while the first's jobs leave room for it.
	w2 := newCigateWorld(t)
	w2.waiting(4, "merge_group", 2)
	w2.waiting(5, "merge_group", 1)
	w2.waiting(6, "pull_request", 30-1)
	patch(t, &cigate.RunJobs, cigate.QueueLimit/2)
	cigate.Tick(w2.ctx)
	assertEqual(t, w2.approved(), []string{"4", "5"})
}

// A run approved counts for `RunJobs` until its first job has run, on later ticks too: it asks for
// the rest of its jobs only then, so a second run is not let in before the first has asked for its
// runners.
func TestCigate_ARunApprovedCountsForItsJobsUntilItsFirstJobHasRun(t *testing.T) {
	w := newCigateWorld(t)
	w.waiting(1, "merge_group", 2)
	w.waiting(2, "merge_group", 1)
	cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"1"}) // RunJobs (20) leaves no room under QueueLimit (16)
	// Its first job is going: it has asked for nothing else yet, so it still counts.
	w.gh.Jobs["1"] = []*pyjson.Object{pyjson.New("id", 100, "name", cigate.FirstJob, "status", "in_progress",
		"labels", []any{"ubuntu-latest"})}
	assertIn(t, "started 0 of 1 waiting run(s); 1 wait for room", cigate.Tick(w.ctx))
	queued, read := cigate.Queued(w.ctx)
	assertEqual(t, queued, cigate.RunJobs)
	assertEqual(t, read, true)
	// Its first job has run and its jobs have asked for runners: only those queued count now.
	w.started(1, 5)
	cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"1", "2"})
	assertIn(t, "5 job(s) queued for a runner", w.gh.Approvals[1].Comment)
	// Another workflow's run counts only for its jobs queued.
	w2 := newCigateWorld(t)
	w2.gh.Runs.Set("7", pyjson.New("id", 7, "name", "bot-night", "path", ".github/workflows/bot-night.yml",
		"status", "in_progress", "event", "workflow_dispatch"))
	queued, _ = cigate.Queued(w2.ctx)
	assertEqual(t, queued, 0)
	w2.gh.Runs.Set("8", pyjson.New("id", 8, "name", "CI", "path", ".github/workflows/ci.yml",
		"status", "queued", "event", "push"))
	queued, _ = cigate.Queued(w2.ctx)
	assertEqual(t, queued, cigate.RunJobs) // main's CI is never gated, but it takes runners as well
}

// A ready pull request's run starts before a draft's, which runs beside its own review run; each
// the oldest first, and every pull request counts as ready when the open ones cannot be read.
func TestCigate_AReadyPullRequestsRunStartsBeforeADrafts(t *testing.T) {
	w := newCigateWorld(t)
	w.waiting(1, "pull_request", 9) // the oldest, a draft
	w.waiting(2, "pull_request", 5)
	w.waiting(3, "pull_request", 3) // a draft
	w.gh.CreatePull("Patch: one", "branch-1", "main", "", true)
	w.gh.CreatePull("Patch: two", "branch-2", "main", "", false)
	w.gh.CreatePull("Patch: three", "branch-3", "main", "", true)
	cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"2"})
	w.started(2, 0)
	cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"2", "1"})
	w2 := newCigateWorld(t)
	w2.waiting(1, "pull_request", 9)
	w2.waiting(2, "pull_request", 5)
	w2.gh.CreatePull("Patch: one", "branch-1", "main", "", true)
	w2.gh.Patch.ListPulls = func(string, string, int) []*pyjson.Object {
		panic(&herr.GitHubError{Msg: "rate limited", Status: 403})
	}
	cigate.Tick(w2.ctx)
	assertEqual(t, w2.approved(), []string{"1"})
}

// A run that has waited too long starts whatever is queued, so the gate never starves CI; so does
// every waiting run when the jobs queued cannot be read.
func TestCigate_ARunThatWaitedTooLongOrAnUnreadableQueueStartsAnyway(t *testing.T) {
	w := newCigateWorld(t)
	w.waiting(1, "pull_request", int(cigate.MaxWaitPull/time.Minute))
	w.waiting(2, "merge_group", int(cigate.MaxWaitQueue/time.Minute))
	w.waiting(3, "pull_request", 1)
	w.busy(50, 0)
	cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{"2", "1"})
	assertIn(t, "it waited 30 min", w.gh.Approvals[1].Comment)
	w2 := newCigateWorld(t)
	w2.waiting(1, "pull_request", 1)
	w2.waiting(2, "pull_request", 2)
	w2.gh.Patch.ListRepoRuns = func(string, int, bool) []*pyjson.Object {
		panic(&herr.GitHubError{Msg: "rate limited", Status: 403})
	}
	cigate.Tick(w2.ctx)
	assertEqual(t, w2.approved(), []string{"2", "1"})
	assertIn(t, "the jobs queued could not be read", w2.gh.Approvals[0].Comment)
}

// It approves only `ci-runners`, and only what this account may approve; it leaves alone a run
// that is not waiting, a push's or a dispatch's, and it never runs for another bot (Squishy), which
// is not the environment's reviewer.
func TestCigate_ItApprovesOnlyItsOwnEnvironmentForTheGatedEvents(t *testing.T) {
	w := newCigateWorld(t)
	w.waiting(1, "pull_request", 1)
	w.gh.Pending["1"] = []*pyjson.Object{pyjson.New("environment", pyjson.New("id", 5, "name", "staging"),
		"current_user_can_approve", true)}
	w.waiting(2, "pull_request", 2)
	w.gh.Pending["2"][0].Set("current_user_can_approve", false)
	w.waiting(3, "push", 3)
	w.waiting(4, "workflow_dispatch", 4)
	w.gh.Runs.Set("5", pyjson.New("id", 5, "workflow", "ci.yml", "status", "queued", "event", "pull_request"))
	w.started(5, 0)
	note := cigate.Tick(w.ctx)
	assertEqual(t, w.approved(), []string{})
	assertIn(t, "run 1 on branch-1: it waits on nothing this account may approve", note)
	assertIn(t, "run 2 on branch-2: it waits on nothing this account may approve", note)
	assertIn(t, "of 2 waiting run(s)", note)
	squishy := newCigateWorld(t)
	squishy.ctx.Cfg.BotLogin = "squishy-squooby"
	squishy.waiting(1, "pull_request", 1)
	assertEqual(t, cigate.Tick(squishy.ctx), "")
	assertEqual(t, squishy.approved(), []string{})
}

// `--gate-every 30`, as bot-status runs it: the gate ticks every 30 seconds between the rewrites,
// the runners keep their two minutes, and a failed tick is only a warning.
func TestCigate_TheStatusLoopTicksTheGateBetweenRewrites(t *testing.T) {
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
	out, _ := capture(t, &cli.Stdout)
	cli.CmdDashboard(testsupport.MakeConfig(), &cli.Args{Every: 300, RunnersEvery: 120, GateEvery: 30, ForSeconds: 300})
	assertEqual(t, ticks, []string{"rewrite 0", "gate 30", "gate 60", "gate 90", "gate 120", "runners 120",
		"gate 150", "gate 180", "gate 210", "gate 240", "runners 240", "gate 270", "rewrite 300"})
	assertIn(t, "::warning::the runner gate did not tick: network down", out.String())
	assertIn(t, "runner gate: started 1 of 1 waiting run(s)", out.String())
}

// Every sweep ticks the gate too (bot-commands' scheduled one among them), so CI still starts while
// the status loop is down.
func TestCigate_EverySweepTicksTheGate(t *testing.T) {
	w := newCigateWorld(t)
	eventsUpdateState(w.ctx, "last_sweep", pyjson.New("since", clock.ISO(testsupport.Day.Add(-time.Hour))))
	w.waiting(1, "merge_group", 0)
	assertIn(t, "runner gate: started 1 of 1 waiting run(s) (queue build 1)", sweep.Sweep(w.ctx))
	assertEqual(t, w.approved(), []string{"1"})
}

// ci.yml's `changes` job, which every other job of the run needs, waits in the environment the
// repository variable names, for a pull request into `main` and a merge-queue build only: never a
// push to `main`, a promotion's pull request (into `_production`) or a dispatch, and nothing waits
// with the variable unset. The status loop ticks the gate every 30 seconds.
func TestWorkflows_OnlyAPullRequestIntoMainAndAMergeQueueBuildWaitForTheRunnerGate(t *testing.T) {
	text := workflowsRead(t, "ci.yml")
	changes := workflowsJob(t, text, "changes")
	assertIn(t, "    environment: ${{ ((github.event_name == 'pull_request' && github.base_ref == 'main') || "+
		"github.event_name == 'merge_group') && vars.CI_GATE_ENVIRONMENT || '' }}\n", changes)
	assertEqual(t, strings.Count(text, "environment:"), 1, "only `changes` names an environment")
	supersede := workflowsJob(t, text, "queue-supersede")
	assertNotIn(t, "needs:", supersede) // it cancels older builds while this one waits
	status := workflowsRead(t, "bot-status.yml")
	assertIn(t, `GATE_EVERY_SECONDS: "30"`, status)
	assertIn(t, `--gate-every "${GATE_EVERY_SECONDS}"`, status)
}
