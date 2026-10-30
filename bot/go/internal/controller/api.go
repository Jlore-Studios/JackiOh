// Package controller is bot/harness/controller.py.
//
// The status loop's planner (#1455): with `controller` on, `bot-status`'s loop plans the night
// bot's work and starts each run with its plan made, so a run no longer waits in GitHub's queue for
// a runner for its gate and its plan before its model job (6 to 25 minutes each on 2026-10-20, for
// jobs that ran under a minute).
//
// The loop already asked the gate's question every tick (`sweep._night_run`, `plan.peek`); now, when
// there is work, it plans itself, under the planning lock (`plan.take_planning_lock`) that the plan
// job takes too, and for each run:
//
//   - claims the item as a plan job does (`plan.make`), with `pending:<ref>` for the run id, since the
//     run it starts has no id yet; `plan.run_status` reads such a claim by the run whose title names
//     the reference, and gives it `plan.PENDING_GRACE` to show up;
//   - writes the plan to the state branch, `plans/<ref>.json`, where only the bots' tokens write, so the
//     model job reads it and cannot change it (bot-night.yml's `plan_ref`);
//   - starts bot-night with `plan_ref` and the plan's runner, CLI, secret name and shared flag, which
//     bot-night's gate and plan jobs skip on, and whose model job checks every one of them against
//     `providers.json` before any step is handed a secret (`__main__.cmd_plan_fetch`);
//   - finds the run by its title (`bot-night <ref> on <runner>`) and puts its id on the claim.
//
// It plans again until nothing more can start, as each plan job's `fill_lanes` used to start one
// more run, and prunes plan files older than `KEEP_PLANS`. bot-night's own gate and plan still run
// for a run started any other way (its schedule, a command, a person), and plan only while no loop
// is going (`plan.loop_alive`): rule 13 keeps the schedule.
//
// The loop also delivers the runs it started (#1537), so they have no `deliver` job either, which
// waited 7 to 25 minutes for a hosted runner on 2026-10-20 to run for under a minute. Each tick, before
// it plans, `deliver` takes every such run that has ended (`plan.awaiting_delivery`): it reads the
// run's plan from the state branch, downloads the model job's `work` upload through the Actions API
// and unzips it, clones the default branch afresh into the runner's scratch directory (the loop's
// own checkout is shallow and hours old, and the deliverer checks that the bundle descends from the
// commit the work started at), and runs the same `Deliverer` deliver ran, as the run. Each try is
// recorded in the state file first (`plan.DELIVERIES`), so one that fails, or ends the loop's
// process, is tried again on the next tick, at most `plan.DELIVER_TRIES` times. Until then
// housekeeping leaves the item alone; after them, or `plan.DELIVER_WITHIN` after the run ended, it
// is housekeeping's, as a dead run's item is. A delivery that fails never stops the tick: git runs
// through `subprocess` with its timeouts, and every failure is a note.
package controller

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"time"
)

// PLANS = "plans"
var Plans string = "plans"

// REF = re.compile(r"^[0-9a-f]{8}-[0-9]{9,11}$")
var Ref any // unported

// KEEP_PLANS = timedelta(days=7)
var KeepPlans any // unported

// FIND_TRIES = 4
var FindTries int = 4

// FIND_WAIT = 3.0
var FindWait float64 = 3.0

// TITLE = "bot-night {ref} on {runs_on}"
var Title string = "bot-night {ref} on {runs_on}"

// title_prefix is controller.title_prefix.

func TitlePrefix(ref string) string {
	panic("unported: controller.title_prefix")
}

// new_ref is controller.new_ref.

func NewRef(now time.Time) string {
	panic("unported: controller.new_ref")
}

// Plan and start night runs until nothing more can start. Each line says what happened.

func Start(ctx *hctx.Context, sleep func(float64) /* = None */) []string {
	panic("unported: controller.start")
}

// _what is controller._what.

func What(planned *pyjson.Object) string {
	panic("unported: controller._what")
}

// The id of the night run whose title names `ref`, or "" when GitHub does not list it yet.

func Find(ctx *hctx.Context, ref string, sleep func(float64)) string {
	panic("unported: controller.find")
}

// Put its run's id on every claim still pending, once GitHub lists the run.

func Resolve(ctx *hctx.Context) []string {
	panic("unported: controller.resolve")
}

// Delete the plan files older than `KEEP_PLANS` (their reference holds their time).

func Prune(ctx *hctx.Context) []string {
	panic("unported: controller.prune")
}

// The plan the loop wrote for `ref`, from the state branch. Raises ValueError when the
// reference is malformed or the file is missing or unreadable.

func Fetch(ctx *hctx.Context, ref string) *pyjson.Object {
	panic("unported: controller.fetch")
}

// What is wrong with a dispatch's `given` runner, CLI, secret name and shared flag against the
// plan and `providers.json`: empty when all agree. The model job runs only on an empty list, so a
// dispatch cannot hand a model job a secret, a CLI or a runner its subscription does not have.

func Check(ctx *hctx.Context, planned *pyjson.Object, given map[string]string) []string {
	panic("unported: controller.check")
}

// WORK_ARTIFACT = "work"
var WorkArtifact string = "work"

// CLONE_TIMEOUT = 600
var CloneTimeout int = 600

// KEEP_DELIVERIES = 100
var KeepDeliveries int = 100

// Deliver every night run the loop started that has ended and still waits for it
// (`plan.awaiting_delivery`). Each line says what happened; nothing here raises.

func Deliver(ctx *hctx.Context, workspace string /* = None */) []string {
	panic("unported: controller.deliver")
}

// _record is controller._record.

func Record(ctx *hctx.Context, runID string, fields *pyjson.Object) {
	panic("unported: controller._record")
}
