// Package cigate is the runner gate (bot/README.md "The runner gate"): GitHub runs a fixed number of
// hosted jobs at once for the whole organization (the Team plan's 60, #3740; the Free plan's 20
// before), gives no job a priority over another (its runners take queued jobs in the order they
// were queued), and every CI run asks for all of its jobs as soon as it can, so when many runs go at
// once each shares the runners with all the others and none finishes soon. So only `Slots` CI and
// CD runs go at once, counted in runs (a pull request's CI is one), not runners, and the slots go in
// the owner's order (2026-11-13): the merge queue, then staging and production, then `main`'s CI,
// then pull requests. A run waits for its slot before it starts, in the `ci-runners` environment,
// whose one required reviewer is the night bot's account; a job waiting for a reviewer holds no
// runner. This package is that reviewer.
//
// The order:
//   - the merge queue's builds never wait, and take their slots first (the queue builds
//     `max_entries_to_build`, 3, at once, infra/github.tf);
//   - staging's and production's deploys and promotions (`Deploys`) never wait either, and take
//     theirs: a job or two each, the deploy itself, which nothing should hold up;
//   - then staging's and production's checks: super.yml (production's gate: the daily run on
//     `_staging`, a promotion's pull request) and CI on a promotion's pull request into
//     `_production` (a promotion is two runs, its CI and its super run), at most `ReleaseRuns` of
//     them at once, so a long super run never takes the whole pool;
//   - then `main`'s CI (the promotions wait on it);
//   - then pull requests': a ready one's (its CI is all that keeps it out of the merge queue), then
//     a draft's (it runs beside its own review run), each the oldest first. They always have
//     `PullMin` slot(s), the pool full or not, so pull requests, which feed the merge queue, never
//     stop.
//
// A run that has waited `MaxWaitRelease` or `MaxWaitPull` goes first of its rung, so a draft never
// starves behind a stream of ready pull requests; every run it waits on ends, so the gate orders and
// delays CI and never stops it. When the runs going cannot be read, every waiting run starts.
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

// Deploys is staging's and production's deploys and promotions: they never wait, and take their
// slots.
var Deploys = []string{"promote-staging.yml", "deploy-staging.yml", "promote-production.yml"}

// Production is the branch a promotion's pull request goes into.
var Production = "_production"

// Gated is the events whose runs wait, by workflow (each names the environment for these alone).
var Gated = map[string][]string{
	Workflow: {"pull_request", "push"},
	Super:    {"pull_request", "workflow_dispatch"},
}

// Slots is how many CI and CD runs go at once (approved and not yet ended), every kind together:
// the merge queue's three builds and three more.
var Slots = 6

// ReleaseRuns is how many of staging's and production's checks (super runs, CI on a promotion's
// pull request) go at once at most.
var ReleaseRuns = 2

// PullMin is how many pull requests' runs may go at once whatever the others hold.
var PullMin = 1

// MaxWaitRelease and MaxWaitPull: a run of staging's or production's, or of `main`'s or a pull
// request's, that has waited this long goes first of its rung.
var (
	MaxWaitRelease = 15 * time.Minute
	MaxWaitPull    = 30 * time.Minute
)

// The rungs a waiting run starts in, in order (the package's comment).
const (
	rungRelease = iota // staging's and production's checks
	rungMain           // `main`'s CI
	rungPulls          // pull requests'
	rungs
)

// Going is the runs going now (approved, and queued or in progress).
type Going struct {
	All  int        // of CI and CD: ci.yml's, super.yml's and the `Deploys`, whatever their event
	Rung [rungs]int // of those that waited here, by rung
}

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
		return waited(run) >= [rungs]time.Duration{MaxWaitRelease, MaxWaitPull, MaxWaitPull}[rung(run)]
	}
	drafts := Drafts(ctx, waiting)
	// Each rung in turn; in it, a run that waited too long first, then a ready pull request's before
	// a draft's, each the oldest first.
	sort.SliceStable(waiting, func(i, j int) bool {
		x, y := waiting[i], waiting[j]
		if rx, ry := rung(x), rung(y); rx != ry {
			return rx < ry
		}
		if ox, oy := overdue(x), overdue(y); ox != oy {
			return ox
		}
		dx, dy := drafts[py.OrStr(x.Get("head_branch"), "")], drafts[py.OrStr(y.Get("head_branch"), "")]
		if dx != dy {
			return dy
		}
		return waited(x) > waited(y)
	})
	going, read := Read(ctx)
	var started, why []string
	for _, run := range waiting {
		g := rung(run)
		slot := going.All < Slots
		switch g {
		case rungRelease:
			slot = slot && going.Rung[g] < ReleaseRuns
		case rungPulls:
			slot = slot || going.Rung[g] < PullMin
		}
		if read && !slot {
			continue
		}
		reason := fmt.Sprintf("%d of %d slot(s) taken", going.All, Slots)
		if !read {
			reason = "the runs going could not be read"
		}
		if note := approve(ctx, run, reason); note != "" {
			why = append(why, note)
			continue
		}
		started = append(started, describe(run))
		going.All++
		going.Rung[g]++
	}
	held := len(waiting) - len(started) - len(why)
	note := fmt.Sprintf("started %d of %d waiting run(s)", len(started), len(waiting))
	if len(started) > 0 {
		note += " (" + strings.Join(started, ", ") + ")"
	}
	if held > 0 {
		note += fmt.Sprintf("; %d wait for a slot", held)
	}
	if len(why) > 0 {
		note += "; " + strings.Join(why, "; ")
	}
	return note
}

// Read is the runs going now, and whether they could be read.
func Read(ctx *hctx.Context) (Going, bool) {
	var going Going
	var runs []*pyjson.Object
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		runs = append(ctx.GH.ListRepoRuns("in_progress", 100, true), ctx.GH.ListRepoRuns("queued", 100, true)...)
	}); failed {
		return going, false
	}
	for _, run := range runs {
		for _, workflow := range append([]string{Workflow, Super}, Deploys...) {
			if isWorkflow(run, workflow) {
				going.All++
				break
			}
		}
		if gated(run) {
			going.Rung[rung(run)]++
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

// rung is the rung a waiting run starts in (the package's comment).
func rung(run *pyjson.Object) int {
	switch {
	case release(run):
		return rungRelease
	case pyjson.Equal(run.Get("event"), "push"):
		return rungMain
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
