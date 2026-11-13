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
// jobs of the rungs below it. And each rung below the merge queue has only a few runs going at once
// (`ReleaseRuns`, `PullRuns`), so the runners finish a few runs soon rather than many late. A CI run asks for most of its jobs only once its first, `FirstJob`, has
// run, about a minute after it starts, so until then it counts for `RunJobs` queued jobs: from its
// approval on, and on every later tick until that job has finished (`Queued`), so runs approved a
// tick apart are not all let in before the first of them asks for its runners. Any run that has
// waited its rung's longest wait (`MaxWaitQueue`, `MaxWaitRelease`, `MaxWaitPull`) starts whatever is
// queued, first of its rung once the rung has a run free (the gate started the runs it waits on, and
// each ends), so the gate only orders and delays CI, never starves it; and when what the runners
// hold cannot be read, every waiting run starts. A dispatch of ci.yml, a person's, never waits. The status loop
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
// or of the last rung that has waited this long starts whatever is queued, first of its rung, once
// the rung has a run free of those it may have going (`ReleaseRuns`, `PullRuns`).
var (
	MaxWaitQueue   = 10 * time.Minute
	MaxWaitRelease = 15 * time.Minute
	MaxWaitPull    = 30 * time.Minute
)

// ReleaseRuns and PullRuns: at most this many runs of staging's and production's, and of the last
// rung (`main`'s CI and pull requests'), go at once: approved and not yet ended. GitHub's runners
// take queued jobs in the order they came, so runs that go side by side share them and each ends
// late; a few at a time each end sooner, and a pull request reaches the merge queue sooner (the
// owner's ask of 2026-11-13). The merge queue has no such cap here: it builds
// `max_entries_to_build` (3) at once already (infra/github.tf).
var (
	ReleaseRuns = 2
	PullRuns    = 3
)

// The rungs, in the order runs start (the package's comment).
const (
	rungQueue   = iota // merge-queue builds
	rungRelease        // staging's and production's
	rungLast           // `main`'s CI and pull requests'
	rungs
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
	now := ctx.Now()
	waited := func(run *pyjson.Object) time.Duration {
		return now.Sub(clock.ParseISOValue(run.Get("created_at")))
	}
	overdue := func(run *pyjson.Object) bool {
		return waited(run) >= [rungs]time.Duration{MaxWaitQueue, MaxWaitRelease, MaxWaitPull}[rung(run)]
	}
	// Within the last rung: `main`'s CI first (the promotions wait on it, and the merge queue ran
	// most of it already), then a ready pull request's, then a draft's.
	drafts := Drafts(ctx, waiting)
	place := func(run *pyjson.Object) int {
		switch {
		case pyjson.Equal(run.Get("event"), "push"):
			return 0
		case drafts[py.OrStr(run.Get("head_branch"), "")]:
			return 2
		}
		return 1
	}
	// Each rung in turn; in it, a run that waited too long first, then by place, each the oldest
	// first.
	sort.SliceStable(waiting, func(i, j int) bool {
		x, y := waiting[i], waiting[j]
		if rx, ry := rung(x), rung(y); rx != ry {
			return rx < ry
		}
		if ox, oy := overdue(x), overdue(y); ox != oy {
			return ox
		}
		if pa, pb := place(x), place(y); pa != pb {
			return pa < pb
		}
		return waited(x) > waited(y)
	})
	load, read := Read(ctx)
	pulls := 0
	var started, why []string
	for _, run := range waiting {
		g := rung(run)
		limit, free, most := QueueLimit, 0, 0
		switch g {
		case rungRelease:
			limit, free, most = ReleaseLimit, ReleaseFree, ReleaseRuns
		case rungLast:
			limit, free, most = PullLimit, PullFree, PullRuns
		}
		// A slot among the runs its rung may have going: a run that waited too long waits for one
		// too, since the gate itself started those runs and each ends.
		slot := most == 0 || load.Runs[g] < most
		room := load.Queued < limit && (free == 0 || Runners-load.InUse > free) && (g != rungLast || pulls < PullsPerTick)
		if read && !(slot && (room || overdue(run))) {
			continue
		}
		reason := fmt.Sprintf("%d job(s) queued for a runner, %d of GitHub's %d runners in use", load.Queued,
			load.InUse, Runners)
		switch {
		case !read:
			reason = "the jobs queued could not be read"
		case !room:
			reason = fmt.Sprintf("it waited %d min", int(waited(run).Minutes()))
		}
		if note := approve(ctx, run, reason); note != "" {
			why = append(why, note)
			continue
		}
		started = append(started, describe(run))
		load.Queued += RunJobs
		load.InUse += RunJobs
		load.Runs[g]++
		if g == rungLast {
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

// Load is what GitHub's runners hold now (`Read`).
type Load struct {
	// Queued is how many jobs are queued for one of GitHub's own runners (a self-hosted runner's
	// queue is its box's).
	Queued int
	// InUse is how many of GitHub's runners are in use: the jobs queued and the jobs going.
	InUse int
	// Runs is how many runs of each rung that waited here are going (approved, not ended).
	Runs [rungs]int
}

// Read is the load on GitHub's runners now, from the runs going and queued, and whether it could be
// read. A CI run whose `FirstJob` has not finished counts for `RunJobs` more jobs, queued and in
// use: it has not asked for the rest of its jobs yet, and will within a minute or so.
func Read(ctx *hctx.Context) (Load, bool) {
	var load Load
	var runs []*pyjson.Object
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		runs = append(ctx.GH.ListRepoRuns("in_progress", 100, true), ctx.GH.ListRepoRuns("queued", 100, true)...)
	}); failed {
		return Load{}, false
	}
	for _, run := range runs {
		var jobs []*pyjson.Object
		if _, failed := herr.Catch[*herr.GitHubError](func() { jobs = ctx.GH.ListJobs(run.Get("id"), true) }); failed {
			return Load{}, false
		}
		first := false // its FirstJob has finished
		for _, job := range jobs {
			if hosted(job) {
				switch job.Str("status") {
				case "queued":
					load.Queued++
					load.InUse++
				case "in_progress":
					load.InUse++
				}
			}
			if job.Str("name") == FirstJob && job.Str("status") == "completed" {
				first = true
			}
		}
		if isWorkflow(run, Workflow) && !first {
			load.Queued += RunJobs
			load.InUse += RunJobs
		}
		if gated(run) {
			load.Runs[rung(run)]++
		}
	}
	return load, true
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

// rung is the rung a run starts in (the package's comment).
func rung(run *pyjson.Object) int {
	switch {
	case isQueue(run):
		return rungQueue
	case release(run):
		return rungRelease
	}
	return rungLast
}

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
