// Package cigate is the runner gate (bot/README.md "The runner gate"): GitHub's free plan runs 20
// hosted jobs at once for the whole organization, and every CI run asks for all of its jobs as soon
// as it can, so when many runs go at once each of them waits behind all the others at every stage
// and none finishes soon. So a pull request's CI and a merge-queue build wait before they start, in
// the `ci-runners` environment, whose one required reviewer is the night bot's account: ci.yml's
// `changes` job, which every other job of the run needs, names it (through the repository variable
// CI_GATE_ENVIRONMENT, which infra/github.tf sets). A job waiting for a reviewer holds no runner.
// This package is that reviewer: it approves a waiting run once GitHub's runners have room for it.
//
// `main`'s own CI, a promotion's (a pull request into `production`) and a dispatch never wait:
// ci.yml names the environment only for a pull request into `main` and a merge-queue build.
//
// The order: merge-queue builds first, the oldest first (the queue lands them in that order, and
// each holds up every pull request behind it), and each as long as fewer than `QueueLimit` of
// GitHub's runners' jobs are queued for one; then pull requests' runs, the oldest first, one a
// tick (`PullsPerTick`), each while fewer than `PullLimit` are queued. A run counts for `RunJobs`
// jobs from its approval on, within the tick, since it asks for most of its jobs only once its
// first has run. Any run that has waited `MaxWaitQueue` (a merge-queue build) or `MaxWaitPull` (a
// pull request's) starts whatever is queued, so the gate only orders and delays CI, never starves
// it; and when the jobs queued cannot be read, every waiting run starts. The status loop runs a
// tick every `--gate-every` seconds, and every sweep runs one too (bot-commands' schedule among
// them), so a stopped loop slows the gate to the sweep's pace rather than stopping CI.
package cigate

import (
	"fmt"
	"sort"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Environment is the environment ci.yml's `changes` job waits in (infra/github.tf
// `ci_runners`, through the CI_GATE_ENVIRONMENT variable).
var Environment = "ci-runners"

// Reviewer is the environment's one required reviewer: only a process with its token approves.
var Reviewer = "jgoetzmann-bot"

// Workflow is the workflow whose runs wait.
var Workflow = "ci.yml"

// Gated is the events whose runs wait (ci.yml names the environment for these alone).
var Gated = []string{"merge_group", "pull_request"}

// QueueLimit: a merge-queue build starts while fewer of GitHub's runners' jobs than this are
// queued for one.
var QueueLimit = 16

// PullLimit: a pull request's run starts while fewer than this are queued.
var PullLimit = 6

// PullsPerTick is how many pull requests' runs start in one tick at most.
var PullsPerTick = 1

// RunJobs is how many queued jobs a run counts for from its approval on, within the tick.
var RunJobs = 12

// MaxWaitQueue and MaxWaitPull: a run that has waited this long starts whatever is queued.
var (
	MaxWaitQueue = 10 * time.Minute
	MaxWaitPull  = 30 * time.Minute
)

// Tick approves the waiting runs that may start now (the package's comment). A note of what it did,
// or "" when nothing waits. A variable so the tests can replace it.
var Tick = tick

func tick(ctx *hctx.Context) string {
	if !strings.EqualFold(ctx.Cfg.BotLogin, Reviewer) {
		return "" // another bot's process (Squishy's): not the environment's reviewer
	}
	var listed []*pyjson.Object
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		listed = ctx.GH.ListRuns(Workflow, "waiting", 100, false)
	}); failed {
		return "could not read the runs waiting (" + py.SliceTo(exc.Error(), 200) + ")"
	}
	var waiting []*pyjson.Object
	for _, run := range listed {
		if pyjson.Equal(run.Get("status"), "waiting") && py.In(run.Get("event"), Gated...) {
			waiting = append(waiting, run)
		}
	}
	if len(waiting) == 0 {
		return ""
	}
	sort.SliceStable(waiting, func(i, j int) bool {
		qi, qj := isQueue(waiting[i]), isQueue(waiting[j])
		if qi != qj {
			return qi
		}
		return py.OrStr(waiting[i].Get("created_at"), "") < py.OrStr(waiting[j].Get("created_at"), "")
	})
	queued, read := Queued(ctx)
	now := ctx.Now()
	pulls := 0
	var started, why []string
	for _, run := range waiting {
		queue := isQueue(run)
		waited := now.Sub(clock.ParseISOValue(run.Get("created_at")))
		limit, most := PullLimit, MaxWaitPull
		if queue {
			limit, most = QueueLimit, MaxWaitQueue
		}
		overdue := waited >= most
		room := queued < limit && (queue || pulls < PullsPerTick)
		if read && !room && !overdue {
			continue
		}
		reason := fmt.Sprintf("%d job(s) queued for a runner", queued)
		switch {
		case !read:
			reason = "the jobs queued could not be read"
		case !room:
			reason = fmt.Sprintf("it waited %d min", int(waited.Minutes()))
		}
		if note := approve(ctx, run, reason); note != "" {
			why = append(why, note)
			continue
		}
		started = append(started, describe(run))
		queued += RunJobs
		if !queue {
			pulls++
		}
	}
	held := len(waiting) - len(started) - len(why)
	note := fmt.Sprintf("started %d of %d waiting run(s)", len(started), len(waiting))
	if len(started) > 0 {
		note += " (" + strings.Join(started, ", ") + ")"
	}
	if held > 0 {
		note += fmt.Sprintf("; %d wait for room", held)
	}
	if len(why) > 0 {
		note += "; " + strings.Join(why, "; ")
	}
	return note
}

// Queued is how many jobs are queued for one of GitHub's own runners now (a self-hosted runner's
// queue is its box's), from the runs going and queued, and whether they could be read.
func Queued(ctx *hctx.Context) (int, bool) {
	var runs []*pyjson.Object
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		runs = append(ctx.GH.ListRepoRuns("in_progress", 100, true), ctx.GH.ListRepoRuns("queued", 100, true)...)
	}); failed {
		return 0, false
	}
	count := 0
	for _, run := range runs {
		var jobs []*pyjson.Object
		if _, failed := herr.Catch[*herr.GitHubError](func() { jobs = ctx.GH.ListJobs(run.Get("id"), true) }); failed {
			return 0, false
		}
		for _, job := range jobs {
			if job.Str("status") == "queued" && hosted(job) {
				count++
			}
		}
	}
	return count, true
}

// approve approves a run's wait in `Environment`, saying why in the approval; "" when it did,
// else why not.
func approve(ctx *hctx.Context, run *pyjson.Object, reason string) string {
	id := run.Get("id")
	var ids []any
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		for _, pending := range ctx.GH.PendingDeployments(id) {
			env := pending.Obj("environment")
			if pyjson.Equal(env.Get("name"), Environment) && pending.Truthy("current_user_can_approve") {
				ids = append(ids, env.Get("id"))
			}
		}
	}); failed {
		return fmt.Sprintf("%s: could not read what it waits on (%s)", describe(run), py.SliceTo(exc.Error(), 200))
	}
	if len(ids) == 0 {
		return describe(run) + ": it waits on nothing this account may approve"
	}
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		ctx.GH.ApproveDeployments(id, ids, "The runner gate (cigate): "+reason+".")
	}); failed {
		return fmt.Sprintf("%s: could not approve it (%s)", describe(run), py.SliceTo(exc.Error(), 200))
	}
	return ""
}

func isQueue(run *pyjson.Object) bool { return pyjson.Equal(run.Get("event"), "merge_group") }

// describe names a run: the merge queue's build of a pull request, or a pull request's branch.
func describe(run *pyjson.Object) string {
	branch := py.OrStr(run.Get("head_branch"), "")
	if isQueue(run) {
		return "queue build " + py.Str(run.Get("id"))
	}
	return fmt.Sprintf("run %s on %s", py.Str(run.Get("id")), branch)
}

// hosted is whether a job asks for one of GitHub's own runners.
func hosted(job *pyjson.Object) bool {
	for _, label := range pyjson.List(job.Get("labels")) {
		if providers.Hosted(py.Str(label)) {
			return true
		}
	}
	return false
}
