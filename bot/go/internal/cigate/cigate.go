// Package cigate is the runner gate (bot/README.md "The runner gate"): GitHub runs a fixed number of
// hosted jobs at once for the whole organization (the Team plan's 60, #3740; the Free plan's 20
// before), and every CI run asks for all of its jobs as soon as it can, so when many runs go at once
// each of them waits behind all the others at every stage and none finishes soon. GitHub gives no
// job a priority over another: its runners take queued jobs in the order they were queued. So the
// runs that ask for many runners wait before they start, in the `ci-runners` environment, whose one
// required reviewer is the night bot's account, and start in the owner's order (2026-11-13): the
// merge queue, then staging and production, then `main`'s CI and pull requests. A job waiting for a
// reviewer holds no runner. ci.yml's `changes` job, which every other job of the run needs, names
// the environment (through the repository variable CI_GATE_ENVIRONMENT, which infra/github.tf sets)
// for every run but a dispatch; super.yml's first jobs name it for every run but its schedule's.
// This package is that reviewer: it approves a waiting run once GitHub's runners have room for it.
//
// The order, each rung the oldest first: merge-queue builds (the queue lands them in that order,
// and each holds up every pull request behind it), each while fewer than `QueueLimit` of GitHub's
// runners' jobs are queued for one; then staging's and production's runs (super.yml, production's
// gate, and CI on a promotion's pull request into `_production`), each while fewer than
// `ReleaseLimit` are queued and more than `ReleaseFree` of the `Runners` are free; then `main`'s CI,
// a ready pull request's and a draft's (a ready one's CI is all that keeps it out of the merge
// queue, while a draft's runs beside its own review run), one a tick (`PullsPerTick`), each while
// fewer than `PullLimit` are queued and more than `PullFree` are free. Each rung's limits are
// tighter than the one above's, so a run never starts while one above it waits for room, and the
// runners kept free are what lets a merge-queue build start at once rather than in line behind the
// jobs of the rungs below it. A CI run asks for most of its jobs only once its first, `FirstJob`, has
// run, about a minute after it starts, so until then it counts for `RunJobs` queued jobs: from its
// approval on, and on every later tick until that job has finished (`Queued`), so runs approved a
// tick apart are not all let in before the first of them asks for its runners. Any run that has
// waited its rung's longest wait (`MaxWaitQueue`, `MaxWaitRelease`, `MaxWaitPull`) starts whatever is
// queued, so the gate only orders and delays CI, never starves it; and when the jobs queued cannot
// be read, every waiting run starts. A dispatch of ci.yml, a person's, never waits. The status loop
// runs a tick every `--gate-every` seconds, and every sweep runs one too (bot-commands' schedule
// among them), so a stopped loop slows the gate to the sweep's pace rather than stopping CI.
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

// Workflow is CI's workflow, whose runs wait.
var Workflow = "ci.yml"

// Super is the daily super run's workflow (production's gate), whose runs wait too.
var Super = "super.yml"

// Production is the branch a promotion's pull request goes into.
var Production = "_production"

// Gated is the events whose runs wait, by workflow (each names the environment for these alone).
var Gated = map[string][]string{
	Workflow: {"merge_group", "pull_request", "push"},
	Super:    {"pull_request", "workflow_dispatch"},
}

// Runners is how many of GitHub's runners the organization may hold at once (the Team plan's 60,
// #3740; dashboard.HostedRunners reads it).
var Runners = 60

// QueueLimit: a merge-queue build starts while fewer of GitHub's runners' jobs than this are
// queued for one.
var QueueLimit = 16

// ReleaseLimit and ReleaseFree: a run of staging's or production's starts while fewer than
// `ReleaseLimit` are queued and more than `ReleaseFree` of the `Runners` are free.
var (
	ReleaseLimit = 10
	ReleaseFree  = 10
)

// PullLimit and PullFree: `main`'s CI and a pull request's run start while fewer than `PullLimit`
// are queued and more than `PullFree` are free: as many as one merge-queue build asks for at once
// (`RunJobs`), so one can start then without waiting in line behind theirs.
var (
	PullLimit = 6
	PullFree  = 20
)

// PullsPerTick is how many runs of the last rung (`main`'s CI and pull requests') start in one tick
// at most.
var PullsPerTick = 1

// RunJobs is how many queued jobs a CI run counts for from its approval until its `FirstJob` has
// finished: the jobs a full run asks for at once when it does, its builds and its Rust test groups
// (20 of a merge-queue build's on 2026-11-06). A super run counts for as many on its approval's tick.
var RunJobs = 20

// FirstJob is the name of ci.yml's `changes` job, the one every other job needs: until it has
// finished, a CI run has asked for none of its other jobs.
var FirstJob = "what the change touches"

// MaxWaitQueue, MaxWaitRelease and MaxWaitPull: a run of the merge queue, of staging or production,
// or of the last rung that has waited this long starts whatever is queued.
var (
	MaxWaitQueue   = 10 * time.Minute
	MaxWaitRelease = 15 * time.Minute
	MaxWaitPull    = 30 * time.Minute
)

// The rungs, in the order runs start (the package's comment): from rankMain on, the last rung's.
const (
	rankQueue   = iota // a merge-queue build
	rankRelease        // staging's or production's
	rankMain           // `main`'s own CI
	rankReady          // a ready pull request's
	rankDraft          // a draft's
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
		listed = ctx.GH.ListRepoRuns("waiting", 100, false)
	}); failed {
		return "could not read the runs waiting (" + py.SliceTo(exc.Error(), 200) + ")"
	}
	var waiting []*pyjson.Object
	for _, run := range listed {
		if pyjson.Equal(run.Get("status"), "waiting") && gated(run) {
			waiting = append(waiting, run)
		}
	}
	if len(waiting) == 0 {
		return ""
	}
	drafts := Drafts(ctx, waiting)
	rank := func(run *pyjson.Object) int {
		switch {
		case isQueue(run):
			return rankQueue
		case release(run):
			return rankRelease
		case pyjson.Equal(run.Get("event"), "push"):
			return rankMain
		case drafts[py.OrStr(run.Get("head_branch"), "")]:
			return rankDraft
		}
		return rankReady
	}
	sort.SliceStable(waiting, func(i, j int) bool {
		ri, rj := rank(waiting[i]), rank(waiting[j])
		if ri != rj {
			return ri < rj
		}
		return py.OrStr(waiting[i].Get("created_at"), "") < py.OrStr(waiting[j].Get("created_at"), "")
	})
	queued, inUse, read := Queued(ctx)
	now := ctx.Now()
	pulls := 0
	var started, why []string
	for _, run := range waiting {
		r := rank(run)
		waited := now.Sub(clock.ParseISOValue(run.Get("created_at")))
		limit, free, most := QueueLimit, 0, MaxWaitQueue
		switch {
		case r == rankRelease:
			limit, free, most = ReleaseLimit, ReleaseFree, MaxWaitRelease
		case r >= rankMain:
			limit, free, most = PullLimit, PullFree, MaxWaitPull
		}
		overdue := waited >= most
		room := queued < limit && (free == 0 || Runners-inUse > free) && (r < rankMain || pulls < PullsPerTick)
		if read && !room && !overdue {
			continue
		}
		reason := fmt.Sprintf("%d job(s) queued for a runner, %d of GitHub's %d runners in use", queued, inUse, Runners)
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
		inUse += RunJobs
		if r >= rankMain {
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
// queue is its box's), how many of those runners are in use (the jobs queued and the jobs going),
// from the runs going and queued, and whether they could be read. A CI run whose `FirstJob` has not
// finished counts for `RunJobs` more in both: it has not asked for the rest of its jobs yet, and
// will within a minute or so.
func Queued(ctx *hctx.Context) (int, int, bool) {
	var runs []*pyjson.Object
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		runs = append(ctx.GH.ListRepoRuns("in_progress", 100, true), ctx.GH.ListRepoRuns("queued", 100, true)...)
	}); failed {
		return 0, 0, false
	}
	queued, inUse := 0, 0
	for _, run := range runs {
		var jobs []*pyjson.Object
		if _, failed := herr.Catch[*herr.GitHubError](func() { jobs = ctx.GH.ListJobs(run.Get("id"), true) }); failed {
			return 0, 0, false
		}
		first := false // its FirstJob has finished
		for _, job := range jobs {
			if hosted(job) {
				switch job.Str("status") {
				case "queued":
					queued++
					inUse++
				case "in_progress":
					inUse++
				}
			}
			if job.Str("name") == FirstJob && job.Str("status") == "completed" {
				first = true
			}
		}
		if isWorkflow(run, Workflow) && !first {
			queued += RunJobs
			inUse += RunJobs
		}
	}
	return queued, inUse, true
}

// Drafts is the head branches of the open pull requests that are drafts, read only when two or more
// of the `waiting` runs are pull requests' (it orders those alone); empty when they could not be
// read, so every pull request then counts as ready.
func Drafts(ctx *hctx.Context, waiting []*pyjson.Object) map[string]bool {
	found := map[string]bool{}
	pulls := 0
	for _, run := range waiting {
		if pyjson.Equal(run.Get("event"), "pull_request") {
			pulls++
		}
	}
	if pulls < 2 {
		return found
	}
	var open []*pyjson.Object
	if _, failed := herr.Catch[*herr.GitHubError](func() { open = ctx.GH.ListPulls("open", "", 100) }); failed {
		return found
	}
	for _, pull := range open {
		if pull.Truthy("draft") {
			if ref := py.OrStr(pull.Obj("head").Get("ref"), ""); ref != "" {
				found[ref] = true
			}
		}
	}
	return found
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

// isWorkflow is whether a run is one of `workflow`'s: the API names its file in `path`
// (`.github/workflows/ci.yml`).
func isWorkflow(run *pyjson.Object, workflow string) bool {
	return strings.HasSuffix(py.OrStr(run.Get("path"), ""), "/"+workflow) ||
		pyjson.Equal(run.Get("workflow"), workflow)
}

// gated is whether a run is of a workflow and an event that wait (`Gated`).
func gated(run *pyjson.Object) bool {
	for workflow, events := range Gated {
		if isWorkflow(run, workflow) && py.In(run.Get("event"), events...) {
			return true
		}
	}
	return false
}

// release is whether a run is staging's or production's: super.yml's, production's gate (the daily
// run on `_staging`, or a promotion's pull request), or CI on a pull request into `Production`
// (its branch is `promote/…`, scripts/promote-production.sh).
func release(run *pyjson.Object) bool {
	if isWorkflow(run, Super) || strings.HasPrefix(py.OrStr(run.Get("head_branch"), ""), "promote/") {
		return true
	}
	for _, pull := range pyjson.Objs(run.Get("pull_requests")) {
		if pyjson.Equal(pull.Obj("base").Get("ref"), Production) {
			return true
		}
	}
	return false
}

// describe names a run: the merge queue's build of a pull request, or a run on its branch (a super
// run named so).
func describe(run *pyjson.Object) string {
	branch := py.OrStr(run.Get("head_branch"), "")
	switch {
	case isQueue(run):
		return "queue build " + py.Str(run.Get("id"))
	case isWorkflow(run, Super):
		return fmt.Sprintf("super run %s on %s", py.Str(run.Get("id")), branch)
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
