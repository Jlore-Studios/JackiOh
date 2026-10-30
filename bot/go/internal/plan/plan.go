// Package plan is bot/harness/plan.py.
//
// The first job of a night run: decide what this run does and which subscription does it.
//
// It spends nothing. In order: the two kill switches, housekeeping (items a dead run left
// `bot:working`, bot PRs that conflict with `main`), the lanes (at most `max_parallel` runs on
// GitHub's runners and, apart from those, `machine_parallel` on the machine hold an item at once,
// and a subscription holds as many as its `lanes`), then one item and the subscription that
// takes it (`pairs`), or a suggestion survey when nothing is queued and one is due. Having claimed
// an item, it starts another run when a lane and more work are still free, so the lanes fill up.
//
// Who does what (`pairs`, `assign`): items go in order of urgency (forced, then the priority tier,
// reviews, revisions, then builds, the harder first and then the oldest), so work already begun is
// finished before new work starts. An item's difficulty (`difficulty:easy`,
// `medium` or `hard`; medium without a label) sets the weakest tier that may build it (`MIN_TIER`).
// Among the seats (`providers.Seat`: one model on one subscription) whose subscription is free and
// available (`providers.availability`: switched on, its secret set, inside its hours unless the item
// is forced, under its limits):
//
//   - **build, revise, fix**: the first free seat in the lane's tier order (#695: Muse before
//     agy before GPT before Sonnet), on the weakest seat of a subscription that meets the
//     tier; ties break by the load balancer (`balance`: the most room under its caps, then the
//     fewest runs held) and then the usage order (`priority`), with `build_last` ones such as
//     claude-2 after all of them. The subscription that planned the item builds only when no other
//     lane seat is free, so planner and builder differ (and the run may switch: `work._switch`). A
//     builder above the item's tier (no seat of that tier free, or one comes later in the tier
//     order) is said in the run's log.
//   - **plan**: a build that has no plan yet gets a planning session first, on the lane's weakest
//     seat that meets its plan floor: a medium seat plans an easy or medium item, a strong one a
//     hard item. When the builder's own subscription has a seat of that tier, the plan and the
//     build share one run; otherwise the planning is a run of its own, and the item goes back to
//     the queue to build from the plan. Such a planning run is short and starts before any long
//     run, so a planner plans a weak builder's next item before it builds one of its own.
//   - **review** in the run: the run's own strongest seat of at least medium, in a fresh
//     adversarial session; a weak builder's own model checks itself instead (findings only, never
//     an approval), and the change goes to its final audit.
//   - **a review run** (`bot:cross-review`): the lane's final audit. A medium or hard item waits
//     for a strong seat; an easy item takes the weakest seat that ships it (medium first). No weak
//     review runs go out. What counts is the review rule (`review_rule.py`): the final audit's
//     approval (strong, or medium for an easy item).
//   - **a conflict on a cleared change** (a bot PR whose commit met the rule, then conflicted with
//     `main`): a builder with its own reviewer in the run first, since that review carries the
//     clearance to the resolution (`deliver._carry`) and no review run follows.
//
// An item labelled `human` is never queued (`candidates`), nor a build that waits for another
// issue (`queue.waits_for`: a "Blocked by #n" line, GitHub's own blocked-by link, or an earlier
// part of the same patch still open). `claude-1` is shared with its owner, so
// an unforced run uses it only after the gate saw it quiet (`quiet_ok`).
package plan

import (
	"fmt"
	"regexp"
	"slices"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Modes is MODES.
var Modes = []string{"auto", "build", "revise", "review", "suggest"}

// RunRequestTtl is RUN_REQUEST_TTL: a run asked for and never taken up is dropped after this
// long.
var RunRequestTtl = 12 * time.Hour

// CILogJobs is CI_LOG_JOBS.
var CILogJobs = 4

// Live is LIVE: a run in one of these states is still going.
var Live = []string{"queued", "in_progress", "waiting", "pending", "requested"}

// RoleOf is ROLE_OF: the role a provider must allow for each kind of item.
var RoleOf = map[string]string{"build": "build", "revise": "revise", "review": "review", "plan": "plan"}

// AnyQuiet is ANY_QUIET: `quiet_ok` that lets every provider through, for questions that start
// nothing.
var AnyQuiet = "*"

// QuietStep is QUIET_STEP: the gate's step that waits for the shared subscription to be quiet
// (`bot-night.yml`).
var QuietStep = "Wait until the subscription is quiet"

// OTHERS is config.OTHERS, imported by name in the Python and patched there by the tests: a
// variable so the Go tests can replace it.
var Others = config.Others

// RunLink is ` ([run](url))` for a run in Actions, and nothing for one started by hand.
func RunLink(cfg *config.Config) string {
	if url := cfg.RunURL(); url != "" {
		return " ([run](" + url + "))"
	}
	return ""
}

// Nothing is plan.nothing.
func Nothing(reason string) *pyjson.Object {
	return pyjson.New("action", "none", "reason", reason)
}

// Pending is PENDING: the run id a claim carries while the run the status loop started for it is
// not yet found (`controller.py`): `pending:<item>-<unix seconds>`, which names the run's title.
var Pending = "pending:"

// PendingGrace is PENDING_GRACE: how long a pending claim's run may take to show up in the run
// list before it counts as dead.
var PendingGrace = 15 * time.Minute

// The unix seconds `datetime.fromtimestamp` takes: years 1 to 9999. Outside them it raises.
const (
	minTimestamp = -62135596800
	maxTimestamp = 253402300799
)

// PendingRun is the status of the run a pending claim names: the bot-night run whose title
// carries its reference (bot-night.yml's `run-name`). Until that run shows up it is alive for
// `PENDING_GRACE` after the claim (the reference holds the claim's time), and dead after.
func PendingRun(ctx *hctx.Context, runID string) string {
	ref := py.SliceFrom(runID, py.Len(Pending))
	var runs []*pyjson.Object
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		runs = ctx.GH.ListRuns(config.NightWorkflow, "", 50, false)
	}); failed {
		return "unknown"
	}
	for _, run := range runs {
		if strings.HasPrefix(py.OrStr(run.Get("display_title"), ""), "bot-night "+ref+" ") {
			if py.In(run.Get("status"), Live...) || AwaitingDelivery(ctx, run) {
				return "alive"
			}
			return "dead"
		}
	}
	// int(ref.rsplit("-", 1)[-1]), and the datetime it names.
	stamp, err := py.Int(ref[strings.LastIndex(ref, "-")+1:])
	if err != nil || stamp < minTimestamp || stamp > maxTimestamp {
		return "dead"
	}
	made := time.Unix(int64(stamp), 0).UTC()
	if ctx.Now().Sub(made) < PendingGrace {
		return "alive"
	}
	return "dead"
}

// RunStatus is `alive`, `dead`, or `unknown` when GitHub could not say. Runs now overlap, so a
// run that cannot be read is treated as alive: its lane stays held and its item is not requeued.
func RunStatus(ctx *hctx.Context, runID any) string {
	if !pyjson.Truthy(runID) {
		return "dead"
	}
	if strings.HasPrefix(py.Str(runID), Pending) {
		return PendingRun(ctx, py.Str(runID))
	}
	status := ""
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		run := ctx.GH.GetRun(runID)
		if py.In(run.Get("status"), Live...) || AwaitingDelivery(ctx, run) {
			status = "alive"
		} else {
			status = "dead"
		}
	}); failed {
		if exc.Status == 404 {
			return "dead"
		}
		return "unknown"
	}
	return status
}

// LoopTitle is LOOP_TITLE: a night run the status loop started (`controller.py`): bot-night.yml's
// `run-name` names its plan.
var LoopTitle = regexp.MustCompile(`^bot-night ([0-9a-f]{8}-[0-9]{9,11}) on `)

// DeliverWithin is DELIVER_WITHIN: how long after a run the loop started has ended the loop may
// still deliver it (#1537). Such a run has no `deliver` job of its own, so until then it counts
// as alive: its lane stays held and housekeeping leaves its item alone. After it, or once the
// loop has given up on it, the item is housekeeping's, as a dead run's is.
var DeliverWithin = 30 * time.Minute

// DeliverTries is DELIVER_TRIES: the deliveries the loop tries for one run, a tick apart, before
// it gives up on it.
var DeliverTries = 3

// Deliveries is DELIVERIES: the state file's record of the loop's deliveries, by run id
// (`controller.deliver`).
var Deliveries = "loop_deliveries"

// LoopRef is the plan reference of a night run the status loop started, or "" for any other run.
func LoopRef(run *pyjson.Object) string {
	if found := LoopTitle.FindStringSubmatch(py.OrStr(run.Get("display_title"), "")); found != nil {
		return found[1]
	}
	return ""
}

// AwaitingDelivery is whether `run`, a night run that has ended, waits for the status loop to
// deliver it: the loop started it, it ended less than `DELIVER_WITHIN` ago, and the loop has
// neither delivered it nor given up on it (`DELIVERIES`).
func AwaitingDelivery(ctx *hctx.Context, run *pyjson.Object) bool {
	if py.In(run.Get("status"), Live...) || LoopRef(run) == "" {
		return false
	}
	ended := clock.ParseISOValue(run.Get("updated_at"))
	if ended.IsZero() || ctx.Now().Sub(ended) >= DeliverWithin {
		return false
	}
	record := ctx.Store.Load().Obj(Deliveries).Get(py.Str(run.Get("id")))
	entry, isDict := record.(*pyjson.Object)
	if !isDict || entry == nil {
		return true
	}
	return !(entry.Truthy("done") || py.MustInt(py.Or(entry.Get("tries"), 0)) >= DeliverTries)
}

// RunAlive is true unless the run is known to have ended.
func RunAlive(ctx *hctx.Context, runID any) bool {
	return RunStatus(ctx, runID) != "dead"
}

// LoopStale is LOOP_STALE: how long since the status loop's last tick (`loop_tick`) it still
// counts as going: two ticks missed, so a loop that hangs while its run still shows as in
// progress stops nothing for long.
var LoopStale = 15 * time.Minute

// LoopAlive is, with `controller` on (#1455), whether the status loop is going, the one place
// that plans and starts night runs while it is: then bot-night's own gate defers to it and
// nothing else starts a run. It is going when its run is in progress and its sweep ticked within
// `LOOP_STALE`. A run list or a state file GitHub cannot read says no, so the night runs fall
// back to planning themselves.
func LoopAlive(ctx *hctx.Context) bool {
	if !ctx.Cfg.Controller {
		return false
	}
	var tick time.Time
	going := false
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		runs := ctx.GH.ListRuns(config.StatusWorkflow, "in_progress", 5, false)
		going = slices.ContainsFunc(runs, func(run *pyjson.Object) bool {
			return eqStr(run.Get("status"), "in_progress") &&
				strings.HasSuffix(py.OrStr(run.Get("path"), ""), "/"+config.StatusWorkflow)
		})
		if !going {
			return
		}
		tick = clock.ParseISOValue(ctx.Store.Load().Get("loop_tick"))
	}); failed || !going {
		return false
	}
	return !tick.IsZero() && ctx.Now().Sub(tick) < LoopStale
}

// LockTtl is LOCK_TTL: how long a planner may hold the planning lock (`planning_lock`) before
// another may take it: a plan takes under a minute, so a lock this old was left by a planner
// that died.
var LockTtl = 3 * time.Minute

// TakePlanningLock is plan.take_planning_lock; a variable so the tests can replace it (the
// status loop and the command line call it through here).
var TakePlanningLock = takePlanningLock

// takePlanningLock takes the planning lock on the state file, or says false when another planner
// holds it. The plan job's concurrency group serializes the night runs' plans; the status loop
// plans too (#1455), so both take this lock, which the state file's compare-and-set makes one
// holder's.
func takePlanningLock(ctx *hctx.Context, owner string) bool {
	now := ctx.Now()
	var got []bool
	ctx.Store.Update(func(state *pyjson.Object) any {
		held := state.Obj("planning")
		var since time.Time
		if held.Truthy("at") {
			since = clock.ParseISOValue(held.Get("at"))
		}
		holder := held.Get("owner")
		if holder != nil && !pyjson.Equal(holder, owner) && !since.IsZero() && now.Sub(since) < LockTtl {
			got = []bool{false}
			return nil
		}
		got = []bool{true}
		state.Set("planning", pyjson.New("owner", owner, "at", clock.ISO(now)))
		return nil
	}, "planning lock")
	return len(got) > 0 && got[0]
}

// DropPlanningLock is plan.drop_planning_lock.
func DropPlanningLock(ctx *hctx.Context, owner string) {
	ctx.Store.Update(func(state *pyjson.Object) any {
		held := state.Obj("planning")
		if pyjson.Equal(held.Get("owner"), owner) {
			state.Set("planning", nil)
		}
		return nil
	}, "planning lock released")
}

// VaultPath is where a provider's refreshed login is kept, encrypted, on the state branch
// (`vault.py`).
func VaultPath(providerID string) string {
	return "vault/" + providerID + ".enc"
}

// Stops is why no run may start now, whatever is queued, or "" (None). `make` and `peek` share
// it.
func Stops(ctx *hctx.Context, state *pyjson.Object, force bool) string {
	cfg := ctx.Cfg
	if ctx.RepoHalted() {
		return fmt.Sprintf("halted by %s on main", config.HaltPath)
	}
	if state.Truthy("halted") {
		return fmt.Sprintf("halted by %s halt", config.Slash)
	}
	if cfg.Secrets.Known && !slices.ContainsFunc(cfg.Pool.Ordered(), func(p *providers.Provider) bool {
		if !p.Enabled {
			return false
		}
		if p.Login == "machine" {
			return true
		}
		has := cfg.Secrets.Has(p.Secret)
		return has != nil && *has
	}) {
		return "no subscription has its secret set, so no model can run"
	}
	// A run that could not work backs off its own subscription only (providers.INFRA_BACKOFFS).
	return ""
}
