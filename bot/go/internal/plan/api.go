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
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
)

// MODES = ("auto", "build", "revise", "review", "suggest")
var Modes = []string{"auto", "build", "revise", "review", "suggest"}

// RUN_REQUEST_TTL = timedelta(hours=12)
var RunRequestTtl any // unported

// CI_LOG_JOBS = 4
var CILogJobs int = 4

// LIVE = ("queued", "in_progress", "waiting", "pending", "requested")
var Live = []string{"queued", "in_progress", "waiting", "pending", "requested"}

// ROLE_OF = {"build": "build", "revise": "revise", "review": "review", "plan": "plan"}
var RoleOf = map[string]string{"build": "build", "revise": "revise", "review": "review", "plan": "plan"}

// ANY_QUIET = "*"
var AnyQuiet string = "*"

// QUIET_STEP = "Wait until the subscription is quiet"
var QuietStep string = "Wait until the subscription is quiet"

// ` ([run](url))` for a run in Actions, and nothing for one started by hand.

func RunLink(cfg any) string {
	panic("unported: plan.run_link")
}

// nothing is plan.nothing.

func Nothing(reason string) *pyjson.Object {
	panic("unported: plan.nothing")
}

// PENDING = "pending:"
var Pending string = "pending:"

// PENDING_GRACE = timedelta(minutes=15)
var PendingGrace any // unported

// The status of the run a pending claim names: the bot-night run whose title carries its
// reference (bot-night.yml's `run-name`). Until that run shows up it is alive for
// `PENDING_GRACE` after the claim (the reference holds the claim's time), and dead after.

func PendingRun(ctx *hctx.Context, runID string) string {
	panic("unported: plan.pending_run")
}

// `alive`, `dead`, or `unknown` when GitHub could not say. Runs now overlap, so a run that
// cannot be read is treated as alive: its lane stays held and its item is not requeued.

func RunStatus(ctx *hctx.Context, runID any) string {
	panic("unported: plan.run_status")
}

// LOOP_TITLE = re.compile(r"^bot-night ([0-9a-f]{8}-[0-9]{9,11}) on ")
var LoopTitle any // unported

// DELIVER_WITHIN = timedelta(minutes=30)
var DeliverWithin any // unported

// DELIVER_TRIES = 3
var DeliverTries int = 3

// DELIVERIES = "loop_deliveries"
var Deliveries string = "loop_deliveries"

// The plan reference of a night run the status loop started, or "" for any other run.

func LoopRef(run *pyjson.Object) string {
	panic("unported: plan.loop_ref")
}

// Whether `run`, a night run that has ended, waits for the status loop to deliver it: the
// loop started it, it ended less than `DELIVER_WITHIN` ago, and the loop has neither delivered
// it nor given up on it (`DELIVERIES`).

func AwaitingDelivery(ctx *hctx.Context, run *pyjson.Object) bool {
	panic("unported: plan.awaiting_delivery")
}

// True unless the run is known to have ended.

func RunAlive(ctx *hctx.Context, runID any) bool {
	panic("unported: plan.run_alive")
}

// LOOP_STALE = timedelta(minutes=15)
var LoopStale any // unported

// With `controller` on (#1455), whether the status loop is going, the one place that plans and
// starts night runs while it is: then bot-night's own gate defers to it and nothing else starts a
// run. It is going when its run is in progress and its sweep ticked within `LOOP_STALE`. A run
// list or a state file GitHub cannot read says no, so the night runs fall back to planning
// themselves.

func LoopAlive(ctx *hctx.Context) bool {
	panic("unported: plan.loop_alive")
}

// LOCK_TTL = timedelta(minutes=3)
var LockTtl any // unported

// Take the planning lock on the state file, or say False when another planner holds it. The
// plan job's concurrency group serializes the night runs' plans; the status loop plans too
// (#1455), so both take this lock, which the state file's compare-and-set makes one holder's.

func TakePlanningLock(ctx *hctx.Context, owner string) bool {
	panic("unported: plan.take_planning_lock")
}

// drop_planning_lock is plan.drop_planning_lock.

func DropPlanningLock(ctx *hctx.Context, owner string) {
	panic("unported: plan.drop_planning_lock")
}

// Where a provider's refreshed login is kept, encrypted, on the state branch (`vault.py`).

func VaultPath(providerID string) string {
	panic("unported: plan.vault_path")
}

// Why no run may start now, whatever is queued, or None. `make` and `peek` share it.

func Stops(ctx *hctx.Context, state *pyjson.Object, force bool) string {
	panic("unported: plan.stops")
}

// The runs that hold work now: item number (0 for a suggestion survey) -> provider id.
// `planning` are the items whose run is a planning run: those hold the planning lane
// (`Pool.plan_lanes`), not a lane of their provider's. GitHub's runners and the bot's machine
// have lanes of their own, so neither waits on the other: `limit` is GitHub's
// (`Pool.max_parallel`), `machine_limit` the machine's (`Pool.machine_parallel`), and `machine`
// names the subscriptions that run on the machine. Every run holds a runner where it runs, a
// planning run too, so `limit` bounds the hosted runners the bot's model jobs take (#1455) and
// `machine_limit` the machine's slots. `limit` is a pool the bots share, first come first
// served: `shared` is how many of its slots the other bots' runs hold now (`others_on_github`).
// `filling` are the items whose run fills another subscription's lane (`Provider.fill_in`):
// those hold no lane of their provider's own.

type Lanes struct {
	Limit        int
	Held         *pyjson.Object  // default: field(default_factory=dict)
	Planning     map[int]bool    // default: field(default_factory=set)
	PlanLimit    int             // default: 0
	MachineLimit int             // default: 0
	Machine      map[string]bool // default: frozenset()
	Shared       int             // default: 0
	Filling      map[int]bool    // default: field(default_factory=set)
}

func (x *Lanes) Busy() map[string]bool {
	panic("unported: plan.Lanes.busy")
}

// How many runs this provider holds now, its planning runs aside.

func (x *Lanes) Count(providerID string) int {
	panic("unported: plan.Lanes.count")
}

// How many planning runs this provider holds now.

func (x *Lanes) PlanningBy(providerID string) int {
	panic("unported: plan.Lanes.planning_by")
}

// How many runs this provider holds in other subscriptions' lanes (`fill_in`).

func (x *Lanes) FillingBy(providerID string) int {
	panic("unported: plan.Lanes.filling_by")
}

// Every run this provider holds now, its planning runs and fill-ins too.

func (x *Lanes) AllBy(providerID string) int {
	panic("unported: plan.Lanes.all_by")
}

// How many of its own `lanes` it holds now: its runs, those in other subscriptions'
// lanes (`fill_in`) aside. A subscription the quiet check guards, or one with usage caps,
// counts its planning run too: the quiet check cannot tell a second run of the bot's from
// its owner, and two runs deciding from one reading go past a cap together (claude-2
// planned and revised at once from 0% and was refused at 100% 15 minutes later).

func (x *Lanes) Used(provider *providers.Provider) int {
	panic("unported: plan.Lanes.used")
}

// It holds as many runs as its `lanes` (`used`).

func (x *Lanes) Full(provider *providers.Provider) bool {
	panic("unported: plan.Lanes.full")
}

// How many of the held runs are on the bot's machine.

func (x *Lanes) OnMachine(pool *providers.Pool) int {
	panic("unported: plan.Lanes.on_machine")
}

// Lanes free on GitHub's runners. A planning run there takes one too (#1455): the
// organization runs 20 hosted jobs at once, CI among them, so `max_parallel` caps every
// model job the bots run on them, not only builds, and the other bots' runs take theirs
// out of the same pool (`shared`).

func (x *Lanes) Free() int {
	panic("unported: plan.Lanes.free")
}

// Slots free on the machine; a planning run there takes one too (`machine_full`).

func (x *Lanes) MachineFree() int {
	panic("unported: plan.Lanes.machine_free")
}

// A lane is free where `provider_id` runs: the machine, or GitHub's runners.

func (x *Lanes) RoomFor(providerID string) bool {
	panic("unported: plan.Lanes.room_for")
}

func (x *Lanes) AnyRoom() bool {
	panic("unported: plan.Lanes.any_room")
}

func (x *Lanes) PlanFree() int {
	panic("unported: plan.Lanes.plan_free")
}

// These lanes with one more run held (`filling`: in another subscription's lane).

func (x *Lanes) WithRun(number int, providerID string, action string, filling bool /* = False */) *Lanes {
	panic("unported: plan.Lanes.with_run")
}

func (x *Lanes) Describe() string {
	panic("unported: plan.Lanes.describe")
}

// SHARED_LIVE = ("queued", "in_progress")
var SharedLive = []string{"queued", "in_progress"}

// How many runs of the other bots (`config.OTHERS`, by their `workflow`) hold a slot of the
// GitHub runners the bots share (#1455). `max_parallel` is one pool, first come first served:
// a slot Squishy holds is one the night bot waits for, and the other way round. A night-bot run
// started with its plan names its runner in its title (bot-night.yml's `run-name`), and one on
// the machine holds no slot; any other live run counts as one. A list GitHub cannot read counts
// as none, so a failed read never stops every lane.

func OthersOnGithub(ctx *hctx.Context) int {
	panic("unported: plan.others_on_github")
}

// Every thread labelled `bot:working`, closed ones too: a run goes on after someone closes
// its issue, and holds its lane until it ends (deliver then drops its work).

func WorkingThreads(ctx *hctx.Context) []*pyjson.Object {
	panic("unported: plan.working_threads")
}

// The lanes held by runs that are still going. A run that ended without delivering holds
// nothing: housekeeping requeues its item.

func ReadLanes(ctx *hctx.Context, state *pyjson.Object) *Lanes {
	panic("unported: plan.read_lanes")
}

// An item's run is in another subscription's lane: it went to its provider's fill-in
// runners (`Provider.fill_in`), which `claim` records as its `runs_on`.

func FillsIn(pool *providers.Pool, record *pyjson.Object) bool {
	panic("unported: plan.fills_in")
}

// `provider` may start a run in the lanes it fills (`Provider.fill_in`): one is free, and
// not kept for a subscription it fills in for that may start work itself now. Muse takes gpt's
// lane on the night box only while gpt is capped, off, outside its hours or backing off.

func FillInFree(ctx *hctx.Context, state *pyjson.Object, provider *providers.Provider, lanes *Lanes) bool {
	panic("unported: plan.fill_in_free")
}

// `provider`'s lanes are shared with a subscription that fills them (`Provider.fill_in`),
// and runs hold every one of the shared lanes: it waits for one, even its own while a fill-in
// holds it, so its box never runs more at once than those lanes.

func FillInFull(pool *providers.Pool, provider *providers.Provider, lanes *Lanes) bool {
	panic("unported: plan.fill_in_full")
}

// The runner label `provider`'s next run takes: its own (`runs_on`) while one of its lanes
// is free, else its fill-in runners' (`fill_in`) while it may fill a lane (`fill_in_free`; a
// suggestion survey never does), else "".

func RunnerFor(ctx *hctx.Context, state *pyjson.Object, provider *providers.Provider, lanes *Lanes, role string) string {
	panic("unported: plan.runner_for")
}

// _usable is plan._usable.

func Usable(ctx *hctx.Context, state *pyjson.Object, provider *providers.Provider, lanes *Lanes, role string, forced bool, quietOK string) bool {
	panic("unported: plan._usable")
}

// One run's work: its subscription, and the seat each role runs on there. `action` is
// `plan` (a planning run of its own), `build`, `revise` or `review` (a review run).

type Assignment struct {
	Provider   *providers.Provider
	Action     string
	Difficulty string
	Plan       *providers.Seat // default: None
	Build      *providers.Seat // default: None
	Review     *providers.Seat // default: None
	Notes      []string        // default: field(default_factory=list)
	RunsOn     string          // default: ""
}

func (x *Assignment) SelfCheck() bool {
	panic("unported: plan.Assignment.self_check")
}

// It runs in another subscription's lane, on its fill-in runners (`Provider.fill_in`).

func (x *Assignment) FillsIn() bool {
	panic("unported: plan.Assignment.fills_in")
}

func (x *Assignment) Describe() string {
	panic("unported: plan.Assignment.describe")
}

// What the model job runs each role on (`work.py`).

func (x *Assignment) Seats() *pyjson.Object {
	panic("unported: plan.Assignment.seats")
}

// free_providers is plan.free_providers.

func FreeProviders(ctx *hctx.Context, state *pyjson.Object, lanes *Lanes, role string, forced bool, quietOK string) []*providers.Provider {
	panic("unported: plan.free_providers")
}

// Balance = Mapping[str, tuple[int, int]]
var Balance any // unported

// LEVEL = (0, 0)
var Level = []int{0, 0}

// The load balancer (`balance` in providers.json): each subscription's key, so that among
// seats the router would otherwise order by the usage order alone, the one with the most room
// under its caps goes first (`providers.room`, in steps of `BALANCE_STEP`), then the one
// holding the fewest runs, and only then the usage order. Work spreads over a tier's
// subscriptions instead of spending the first in `priority` to its cap (claude-3 took 168
// runs in a week to claude-5's 13) while the rest sit idle. Empty when it is off.

func Balance_(ctx *hctx.Context, state *pyjson.Object, lanes *Lanes) any {
	panic("unported: plan.balance")
}

// `seats` in the order the router tries them: the load balancer's (`spread`), then the
// usage order (`priority`), then the tier's own preference order.

func Ranked(pool *providers.Pool, seats []*providers.Seat, spread any /* = None */) []*providers.Seat {
	panic("unported: plan.ranked")
}

// Who builds an item of `difficulty`: the first free seat in the lane's tier order
// (#695: Muse before agy before GPT before Sonnet), on its weakest seat that meets
// the tier; with a note when that seat is above the tier, saying why (none of that tier is
// free, or the tier order puts this one first). Among subscriptions whose seat is the same
// model (the Claude accounts' Sonnet or Opus), the load balancer (`spread`) goes before the
// usage order. One lane, one owner (#725): a medium or
// hard item stays with the subscription that planned it (`avoid`) when that one can build
// it, so planner and builder are the same; an easy item still goes weak-first, since its
// planner (medium) must not spend its lane building what a weak seat builds. A revision
// that answers a final audit (`audit_tier`) stays at the auditor's tier or above, so the
// auditor fixes and pushes it.
//
// A fullsend `part` (#505), whose plan is on record by now, goes first to the subscriptions
// whose strongest seat is weakest, and one with no seat of its tier may build it on a seat of
// `PART_FLOOR`: Muse builds a hard part from Opus's plan, and the strong models keep to the
// planning and to the reconcile that checks and reviews every part.

func BuilderSeat(pool *providers.Pool, providers []*providers.Provider, difficulty string, part bool /* = False */, avoid string /* = "" */, auditTier string /* = "" */, spread any /* = None */) (*providers.Seat, string) {
	panic("unported: plan.builder_seat")
}

// The planning lane's planners: the strongest seat of each subscription that may plan now
// (at least medium), the strong ones first, and within a tier the one holding the fewest plans
// first, then the usage order. A planning run takes none of its subscription's lanes, so a
// subscription plans while it builds, as many items at once as it has lanes, but it takes a
// runner: a slot on the machine (`machine_full`), or one of GitHub's lanes (`Lanes.free`,
// #1455). Planning comes first, so the strong
// models write the plans the medium ones build from before they build themselves. One the quiet
// check guards, or one with caps, plans only when it holds nothing else. Which item a planner
// may take is the item's plan floor (`queue.plan_floor`): a medium one plans only easy or
// unrated items. A plan is no short call (an Opus planner spent 20 minutes on #37), so like a
// build it starts only `start_headroom` under each cap.

func LanePlanners(ctx *hctx.Context, state *pyjson.Object, lanes *Lanes, forced bool, quietOK string) []*providers.Seat {
	panic("unported: plan.lane_planners")
}

// A build or revise run's own adversarial reviewer: its strongest seat of at least medium,
// or None, and the change goes to a review run. (`difficulty` does not change it: a hard item's
// builder is strong, so its own reviewer is too.)

func RunReviewer(pool *providers.Pool, provider *providers.Provider, difficulty string) *providers.Seat {
	panic("unported: plan.run_reviewer")
}

// A review run's reviewer: the lane's final audit (#725). A medium or hard item waits for
// a strong seat (Opus); an easy item takes the weakest free seat that ships it (medium, then
// strong). No weak review runs go out: weak builders review themselves in fresh sessions, and
// a weak approval never ships. Within a tier, a family that has not approved the head yet
// goes before one that has (the same model may review it twice), then the load balancer's
// order (`spread`, `ranked`). A stand-in seat (`takes_over`) reviews nothing.

func ReviewSeat(pool *providers.Pool, providers []*providers.Provider, candidate *queue.Candidate, spread any /* = None */) *providers.Seat {
	panic("unported: plan.review_seat")
}

// Who takes `candidate` now, and on which seats, or None when nobody can.

func Assign(ctx *hctx.Context, state *pyjson.Object, candidate *queue.Candidate, lanes *Lanes, force bool, quietOK string) *Assignment {
	panic("unported: plan.assign")
}

// Every queued item that a subscription can take now, with who takes it on which seats,
// most urgent first. (`skipped` is the queue's: the items labelled `human`.)

func Pairs(ctx *hctx.Context, state *pyjson.Object, queue []*queue.Candidate, lanes *Lanes, force bool, quietOK string, skipped []string /* = None */) [][]any {
	panic("unported: plan.pairs")
}

// survey_provider is plan.survey_provider.

func SurveyProvider(ctx *hctx.Context, state *pyjson.Object, lanes *Lanes, force bool, quietOK string) *providers.Provider {
	panic("unported: plan.survey_provider")
}

// `provider` runs on the bot's machine (the night box), and its `machine_parallel` slots are
// all held. GitHub's runners have `max_parallel` of their own. (The AI's training lanes
// run on a box of their own, outside the harness, so no slot is kept for them.)

func MachineFull(pool *providers.Pool, provider *providers.Provider, lanes *Lanes) bool {
	panic("unported: plan.machine_full")
}

// Why each subscription cannot take work now, for the run's summary and `status`.

func WhyNone(ctx *hctx.Context, state *pyjson.Object, lanes *Lanes) string {
	panic("unported: plan.why_none")
}

// True when another `bot-night` run is waiting for the shared subscription to be quiet, so
// this one should not wait too (each waiting gate reads the usage every ten minutes).

func QuietWaiting(ctx *hctx.Context) bool {
	panic("unported: plan.quiet_waiting")
}

// What the gate learns: whether a run would find work, and whether it must first wait for
// the shared subscription to be quiet (`quiet_provider`), with `fallback` when other work
// could go ahead without it.

type Peek struct {
	Work          bool
	Reason        string
	Forced        bool     // default: False
	Provider      string   // default: ""
	QuietProvider string   // default: ""
	QuietSecret   string   // default: ""
	Fallback      bool     // default: False
	Held          int      // default: 0
	Skipped       []string // default: field(default_factory=list)
	Idle          bool     // default: False
}

// Whether a run would find work, why, and with which subscription. Changes nothing.
//
// The gate asks this before it spends anything on reading the subscription's usage.

func Peek_(ctx *hctx.Context, force bool /* = False */, item *int /* = None */, mode string /* = "auto" */) *Peek {
	panic("unported: plan.peek")
}

// A `/harness run` (or `start --force`) not yet taken up, if one is fresh enough.

func RunRequest(ctx *hctx.Context, state *pyjson.Object) *pyjson.Object {
	panic("unported: plan.run_request")
}

// On the day a subscription's `off_from` comes, open one issue asking a person what it
// should do now (the bot cannot change `.harness/`), and remember that it did.

func AnnounceSwitchedOff(ctx *hctx.Context, state *pyjson.Object) []string {
	panic("unported: plan.announce_switched_off")
}

// What `housekeeping` would requeue, read without changing anything, or None.

func HousekeepingDue(ctx *hctx.Context, state *pyjson.Object) string {
	panic("unported: plan.housekeeping_due")
}

// Decide what this run does and with which subscription. A subscription outside its hours
// works only on forced work: the `force` of this dispatch, or an item an operator queued with
// `--force`, which outlives the run it started. `quiet_ok` is the gate's verdict: the shared
// subscription it saw quiet, or "" for none; the workflow always passes one, and None (a run
// by hand) restricts nothing.

func Make(ctx *hctx.Context, force bool /* = False */, item *int /* = None */, mode string /* = "auto" */, quietOK string /* = None */) *pyjson.Object {
	panic("unported: plan.make")
}

// Start one more run when, with this item claimed, a lane (or the planning lane) and more
// work are still free. That run does the same, so the lanes fill one run at a time.

func FillLanes(ctx *hctx.Context, lanes *Lanes, taken *queue.Candidate, assignment *Assignment, planned *pyjson.Object) {
	panic("unported: plan.fill_lanes")
}

// Keep `bot:needs-plan` on exactly the queued items in the Needs plan stage, and on the ones
// being planned now; and `bot:planned` on the queued ones a strong model has planned (it stays
// after they leave the queue, and comes off only if the item needs a plan again).

func SyncNeedsPlan(ctx *hctx.Context, state *pyjson.Object) []string {
	panic("unported: plan.sync_needs_plan")
}

// Record which provider secrets this run's workflow has, for the status loop: its own list
// is fixed when its long run is created, so a secret added since shows there once a plan job
// has seen it (`providers.newer_secrets`).

func NoteSecrets(ctx *hctx.Context, state *pyjson.Object) {
	panic("unported: plan.note_secrets")
}

// KEEP_CLOSED = timedelta(days=7)
var KeepClosed any // unported

// PRUNE_EVERY = timedelta(hours=24)
var PruneEvery any // unported

// SHED_WHEN_CLOSED = ("handoff", "last_findings", "self_check_findings", "strike_log", "votes", "asks", "taken_asks", "question")
var ShedWhenClosed = []string{"handoff", "last_findings", "self_check_findings", "strike_log", "votes", "asks", "taken_asks", "question"}

// Once a day: drop the record of every item whose thread closed more than `KEEP_CLOSED`
// ago, and the bulky parts of the others that are closed. Open threads keep everything.

func Prune(ctx *hctx.Context, state *pyjson.Object) []string {
	panic("unported: plan.prune")
}

// Requeue items a dead run left working; queue a revision for conflicted bot PRs; put back
// in the merge queue a cleared one it took out with no failed build (`queue.requeue_cleared`).

func Housekeeping(ctx *hctx.Context, state *pyjson.Object) []string {
	panic("unported: plan.housekeeping")
}

// Build the plan for one queued thread on one subscription, and mark it `bot:working`.

func Claim(ctx *hctx.Context, candidate *queue.Candidate, assignment *Assignment) *pyjson.Object {
	panic("unported: plan.claim")
}

// Who builds a split's sub-issues (#60): this bot for `split` and `fullsend`, the other bot
// for `split-bot`.

func SplitBuilders(mode string) string {
	panic("unported: plan.split_builders")
}

// The sub-issues an issue already has, for a split that closes its tree out: what was done
// and what is still open. Fenced as data; "" when it has none.

func TreeText(ctx *hctx.Context, number int) string {
	panic("unported: plan.tree_text")
}

// The tails of the failed jobs of one CI run, fenced as data.

func CILogs(ctx *hctx.Context, runID any) string {
	panic("unported: plan.ci_logs")
}

// open_suggestion_count is plan.open_suggestion_count.

func OpenSuggestionCount(ctx *hctx.Context) int {
	panic("unported: plan.open_suggestion_count")
}

// How many suggestions a survey may open now: 0 when none is due or the cap is full. One is
// due when asked for, or once a free lane has had nothing to start for
// `suggestions.idle_minutes` (#797: `suggest.idle_since`, kept by `note_idle`).

func SuggestionsDue(ctx *hctx.Context, state *pyjson.Object, force bool /* = False */) int {
	panic("unported: plan.suggestions_due")
}

// Keep the clock a survey waits on (#797): `suggest.idle_since` is when a lane last became
// free with nothing in the queue it could start (`Peek.idle`), and goes when there is work for
// it again. The sweep calls this every tick, writing only when it changes; a survey claimed
// restarts it (`suggestion_plan`). Bookkeeping, so it reports nothing.

func NoteIdle(ctx *hctx.Context, idle bool) {
	panic("unported: plan.note_idle")
}

// suggestion_plan is plan.suggestion_plan.

func SuggestionPlan(ctx *hctx.Context, lanes *Lanes, force bool, quietOK string /* = "" */) *pyjson.Object {
	panic("unported: plan.suggestion_plan")
}
