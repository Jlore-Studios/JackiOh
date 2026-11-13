// Package cigate is the runner gate (bot/README.md "The runner gate"): GitHub runs a fixed number of
// hosted jobs at once for the whole organization (the Team plan's 60, #3740; the Free plan's 20
// before), gives no job a priority over another (its runners take queued jobs in the order they
// were queued), and every CI run asks for all of its jobs as soon as it can, so when many runs go at
// once each shares the runners with all the others and none finishes soon. So the gate lets only a
// few go at once, counted in pull requests, not runners (the owner's ask of 2026-11-13): a run waits
// before it starts, in the `ci-runners` environment, whose one required reviewer is the night bot's
// account, until a run of its rung ends. A job waiting for a reviewer holds no runner. This package
// is that reviewer.
//
// The rungs, in the owner's order: the merge queue's builds never wait (ci.yml leaves them out, and
// the queue builds `max_entries_to_build`, 3, at once already, infra/github.tf); staging's and
// production's runs (super.yml, production's gate, and CI on a promotion's pull request into
// `_production`) go `ReleaseRuns` at once; and `main`'s CI and pull requests' go `PullRuns` at once,
// `main`'s first (the promotions wait on it, and the merge queue ran most of it already), then a
// ready pull request's (its CI is all that keeps it out of the merge queue), then a draft's (it runs
// beside its own review run), each the oldest first. A pull request's CI is one run (a newer push
// replaces its running one), so these count pull requests; a promotion's pull request is two, its
// CI and its super run. A run that has waited `MaxWaitRelease` or `MaxWaitPull` goes first of its
// rung, so a draft never starves behind a stream of ready pull requests; every run it waits on is
// one the gate started, and each ends, so the gate orders and delays CI and never stops it. When the
// runs going cannot be read, every waiting run starts.
//
// ci.yml's `changes` job, which every other job of the run needs, names the environment (through
// the repository variable CI_GATE_ENVIRONMENT, which infra/github.tf sets) for a pull request's run
// and a push to `main`; super.yml's first jobs name it for every run but its schedule's. A dispatch
// of ci.yml, a person's, never waits. The status loop runs a tick every `--gate-every` seconds, and
// every sweep runs one too (bot-commands' schedule among them), so a stopped loop slows the gate to
// the sweep's pace rather than stopping CI.
package cigate

import (
	"fmt"
	"sort"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Environment is the environment the runs wait in (infra/github.tf `ci_runners`, through the
// CI_GATE_ENVIRONMENT variable).
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
	Workflow: {"pull_request", "push"},
	Super:    {"pull_request", "workflow_dispatch"},
}

// ReleaseRuns and PullRuns: at most this many runs of staging's and production's, and of `main`'s CI
// and pull requests' together, go at once (approved and not yet ended).
var (
	ReleaseRuns = 2
	PullRuns    = 3
)

// MaxWaitRelease and MaxWaitPull: a run of staging's or production's, or of `main`'s or a pull
// request's, that has waited this long goes first of its rung.
var (
	MaxWaitRelease = 15 * time.Minute
	MaxWaitPull    = 30 * time.Minute
)

// The rungs, in the order runs start (the package's comment).
const (
	rungRelease = iota // staging's and production's
	rungPulls          // `main`'s CI and pull requests'
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
		return waited(run) >= [rungs]time.Duration{MaxWaitRelease, MaxWaitPull}[rung(run)]
	}
	drafts := Drafts(ctx, waiting)
	place := func(run *pyjson.Object) int {
		switch {
		case pyjson.Equal(run.Get("event"), "push"):
			return 0 // `main`'s
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
	going, read := Going(ctx)
	most := [rungs]int{ReleaseRuns, PullRuns}
	var started, why []string
	for _, run := range waiting {
		g := rung(run)
		if read && going[g] >= most[g] {
			continue
		}
		reason := fmt.Sprintf("%d of its rung's %d run(s) going", going[g], most[g])
		if !read {
			reason = "the runs going could not be read"
		}
		if note := approve(ctx, run, reason); note != "" {
			why = append(why, note)
			continue
		}
		started = append(started, describe(run))
		going[g]++
	}
	held := len(waiting) - len(started) - len(why)
	note := fmt.Sprintf("started %d of %d waiting run(s)", len(started), len(waiting))
	if len(started) > 0 {
		note += " (" + strings.Join(started, ", ") + ")"
	}
	if held > 0 {
		note += fmt.Sprintf("; %d wait for a run of theirs to end", held)
	}
	if len(why) > 0 {
		note += "; " + strings.Join(why, "; ")
	}
	return note
}

// Going is how many runs of each rung that wait here are going now (approved, and queued or in
// progress), and whether they could be read.
func Going(ctx *hctx.Context) ([rungs]int, bool) {
	var going [rungs]int
	var runs []*pyjson.Object
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		runs = append(ctx.GH.ListRepoRuns("in_progress", 100, true), ctx.GH.ListRepoRuns("queued", 100, true)...)
	}); failed {
		return going, false
	}
	for _, run := range runs {
		if gated(run) {
			going[rung(run)]++
		}
	}
	return going, true
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

// rung is the rung a run starts in (the package's comment).
func rung(run *pyjson.Object) int {
	if release(run) {
		return rungRelease
	}
	return rungPulls
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

// describe names a run: a run on its branch, a super run named so.
func describe(run *pyjson.Object) string {
	what := "run"
	if isWorkflow(run, Super) {
		what = "super run"
	}
	return fmt.Sprintf("%s %s on %s", what, py.Str(run.Get("id")), py.OrStr(run.Get("head_branch"), ""))
}
