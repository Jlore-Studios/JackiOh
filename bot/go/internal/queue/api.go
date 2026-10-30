// Package queue is bot/harness/queue.py.
//
// The work queue, kept as labels on issues and pull requests plus a record in the state file.
//
// `bot:build` on an issue and `bot:revise` on a pull request mean queued; `bot:cross-review` on a
// bot pull request means it waits for a review run; `bot:working` means a run holds it;
// `bot:blocked` means it waits for a person. Only one `bot:` state label is on a thread at a time,
// apart from `bot:pr` and `bot:pr-open`, which say what a thread is.
//
// The prefix is the running bot's (`config.LABEL_PREFIX`): Squishy's queue is `squishy:build` and the
// rest (#60). Beside its queue label, a mode label (`squishy:oneshot`, `squishy:split`,
// `squishy:split-bot`, and either bot's `fullsend`, #505) says how a queued issue is built
// (`mode_of`). A thread that is another bot's
// (`owner`: its labels on it, or assigned to it) is left alone, so the two never take the same issue.
package queue

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/asks"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// STATE_LABELS = (LABEL_BUILD, LABEL_REVISE, LABEL_CROSS, LABEL_WORKING, LABEL_BLOCKED)
var StateLabels any // unported

// label_names is queue.label_names.

func LabelNames(thread *pyjson.Object) map[string]bool {
	panic("unported: queue.label_names")
}

// Leave exactly `wanted` (or none) of the queue's state labels on the thread. A pull request
// back in the queue is no longer ready for a person to merge, so `ready for merge` comes off.

func SetStateLabel(ctx *hctx.Context, number int, current map[string]bool, wanted string) {
	panic("unported: queue.set_state_label")
}

// branch_for_issue is queue.branch_for_issue.

func BranchForIssue(number int) string {
	panic("unported: queue.branch_for_issue")
}

// Where a revision of pull request `number` that was cut off keeps its unfinished work
// (#317 part 3): never the pull request's own branch, which would start CI on it.

func WipBranch(number int) string {
	panic("unported: queue.wip_branch")
}

// open_pull_for_branch is queue.open_pull_for_branch.

func OpenPullForBranch(ctx *hctx.Context, branch string) *pyjson.Object {
	panic("unported: queue.open_pull_for_branch")
}

// Labelled `human` (whatever its case): people do it, and the bot leaves it alone. So is one
// labelled `human required` (#793): the bot found it needs what only a person can do and
// handed it to people (`deliver.Deliverer._needs_person`), or a person said so.

func IsHuman(names map[string]bool) bool {
	panic("unported: queue.is_human")
}

// OWNING = ("build", "revise", "cross-review", "working", "blocked", "pr-open", "pr", "needs-plan", "oneshot", "split", "split-bot", "fullsend", "tree")
var Owning = []string{"build", "revise", "cross-review", "working", "blocked", "pr-open", "pr", "needs-plan", "oneshot", "split", "split-bot", "fullsend", "tree"}

// The other bot whose thread this is (`config.OTHERS`), or None: one of its owning labels is
// on it, or it is assigned to that bot's account.

func Owner(thread *pyjson.Object) any {
	panic("unported: queue.owner")
}

// owner_reply is queue.owner_reply.

func OwnerReply(number int, other any) string {
	panic("unported: queue.owner_reply")
}

// How a queued issue is built (#60): the first of the bot's modes whose label is on it
// (`oneshot`, `split`, `split-bot`, `fullsend`), or "" for a plain build.

func ModeOf(names map[string]bool) string {
	panic("unported: queue.mode_of")
}

// Leave exactly `mode`'s label (or none, for "") of the mode labels on the issue; the labels
// it has after.

func SetMode(ctx *hctx.Context, number int, names map[string]bool, mode string) map[string]bool {
	panic("unported: queue.set_mode")
}

// MODE_WORDS = {"": "build it", "oneshot": "build it in one run with fullsend", "split": "split it into sub-issues I build", "split-bot": "split it into sub-issue
var ModeWords = map[string]string{"": "build it", "oneshot": "build it in one run with fullsend", "split": "split it into sub-issues I build", "split-bot": "split it into sub-issues the night bot builds", "fullsend": "split it into parts that land on one branch, then reconcile them into one pull request"}

// A fullsend issue (#505): it is queued in that mode (`mode_of`), or its record says its
// tree was split for fullsend. Its own branch is the one its parts land on.

func IsFullsend(mode string, record *pyjson.Object) bool {
	panic("unported: queue.is_fullsend")
}

// A fullsend part (#505): its record says it lands on its tree's branch, as deliver wrote
// when it opened the part, or `adopt` when it took in an issue someone had opened.

func IsPart(record *pyjson.Object) bool {
	panic("unported: queue.is_part")
}

// The sub-issues of issue `number` still open: while a fullsend tree has any, its parts are
// still landing on its branch, and nothing builds the issue itself (#505).

func OpenParts(ctx *hctx.Context, number int) []int {
	panic("unported: queue.open_parts")
}

// parts_reply is queue.parts_reply.

func PartsReply(number int, parts []int) string {
	panic("unported: queue.parts_reply")
}

// The reply to a request for a thread `is_human` keeps from the bot.

func HumanReply(number int, names map[string]bool /* = frozenset() */) string {
	panic("unported: queue.human_reply")
}

// The thread's difficulty: the hardest `difficulty:*` label on it (whatever its case) or
// `carried` (what its issue had, kept on a bot PR's record, or a floor the bot raised it to),
// and `medium` with neither.

func DifficultyOf(names map[string]bool, carried string /* = "" */) string {
	panic("unported: queue.difficulty_of")
}

// The hardest `difficulty:*` label on the thread, or "" for none.

func LabelledDifficulty(names map[string]bool) string {
	panic("unported: queue.labelled_difficulty")
}

// What the state file carries for a thread's difficulty: a bot pull request's issue's
// (`difficulty`), and a floor the bot raised it to (`difficulty_floor`: the easy rule broken,
// or three failures under a person's label, #317 part 8), the harder of the two.

func CarriedDifficulty(record *pyjson.Object) string {
	panic("unported: queue.carried_difficulty")
}

// Who set the item's difficulty label: `bot` when its record's rating (`difficulty_by`) names
// the label it carries, `person` for any other label, "" for none (#317 part 8). The bot never
// replaces a person's label.

func RatingSource(names map[string]bool, record *pyjson.Object) string {
	panic("unported: queue.rating_source")
}

// Someone has said how hard it is: a `difficulty:*` label, the bot's own rating
// (`difficulty_by`), or a difficulty its record carries. An unrated item counts as medium for
// routing, and a medium or strong model rates it when it plans it (#317 part 8).

func Rated(names map[string]bool, record *pyjson.Object) bool {
	panic("unported: queue.rated")
}

// Queue an issue for building. Returns the reply line. `ask` is the comment that asked, kept
// on the record that ends up queued so later stages can react to it (`asks`). `mode` is how to
// build it (#60): one of the bot's modes, "" for a plain build (its mode labels come off), or
// None to leave its mode labels as they are. A fullsend issue whose parts are still open is not
// queued at all (#505): the sweep queues its reconcile once they have all closed.

func QueueBuild(ctx *hctx.Context, number int, by string, force bool /* = False */, labelPresent bool /* = False */, ask *asks.Ask /* = None */, mode string /* = None */) string {
	panic("unported: queue.queue_build")
}

// `fullsend` on an issue whose sub-issues someone opened already, a tracker split by hand:
// they are its parts, with no split (#505). Its branch is made at `main`'s head; each open
// sub-issue the bot may build gets the record that sends its build there (`is_part`), and is
// queued if it was not; and the issue becomes a tree, whose reconcile the sweep queues once
// every sub-issue has closed (`sweep._trees`). One labelled `human` or the other bot's, one a
// run holds, or one with a pull request of mine open finishes on `main` as it is, and the
// reconcile waits for it to close too.

func Adopt(ctx *hctx.Context, number int, names map[string]bool, children []*pyjson.Object, by string) string {
	panic("unported: queue.adopt")
}

// The squash commit's title: the pull request's, as GitHub writes it for a pull request of
// several commits. Without it a one-commit pull request lands on `main` under the bot's own
// commit message ("bot: build pass 1 for #85").

func MergeTitle(pull *pyjson.Object, method string) string {
	panic("unported: queue.merge_title")
}

// Keep a pull request from merging: auto-merge off, and out of `main`'s merge queue. A queued
// pull request has no auto-merge left to turn off (REST `auto_merge` reads null once GitHub put
// it in the queue), so only the dequeue stops it; a push to its branch takes it out as well.

func HoldMerge(ctx *hctx.Context, pull *pyjson.Object) {
	panic("unported: queue.hold_merge")
}

// Put a bot pull request back in `main`'s merge queue when the queue took its cleared head
// out and no failed build of it says why, so `events.on_queue_ci` never heard: it conflicted
// with a pull request ahead of it that then failed, or its checks ran past the queue's timeout.
// Only when the queue's removal is the last thing that touched its auto-merge and no person
// did it, and once for each commit `main` is at, so one that conflicts again waits for `main`
// to move. The housekeeping pass calls it. A note of what it did, or "".

func RequeueCleared(ctx *hctx.Context, pull *pyjson.Object, record *pyjson.Object) string {
	panic("unported: queue.requeue_cleared")
}

// Queue a pull request for a revision. Returns the reply line.

func QueueRevise(ctx *hctx.Context, number int, by string, force bool /* = False */, source string /* = "request" */, labelPresent bool /* = False */, extra *pyjson.Object /* = None */, ask *asks.Ask /* = None */) string {
	panic("unported: queue.queue_revise")
}

// Queue a review run of a bot pull request's head, and no revision (`/harness review`,
// #317 part 10): with `floor` (`strong` or `medium`) it waits for a reviewer of that tier or
// stronger. Returns the reply line.

func QueueReview(ctx *hctx.Context, number int, by string, floor string /* = "" */, notes string /* = "" */, ask *asks.Ask /* = None */) string {
	panic("unported: queue.queue_review")
}

// Take a thread out of the queue. A run that holds it keeps `bot:working` until it has
// stopped, so a request made in the meantime waits for that run instead of racing it.

func Stop(ctx *hctx.Context, number int, by string) string {
	panic("unported: queue.stop")
}

// PRIORITY_TIERS = {LABEL_PRIORITY_HIGH: 0, LABEL_PRIORITY_MEDIUM: 1, LABEL_PRIORITY_LOW: 3}
var PriorityTiers any // unported

// NO_PRIORITY = 2
var NoPriority int = 2

// PRIORITY_NAMES = {0: "high", 1: "medium", NO_PRIORITY: "none", 3: "low"}
var PriorityNames any // unported

// The thread's pickup tier, the lowest picked first: `priority:high` 0, `priority:medium`
// 1, none 2, `priority:low` 3. The highest label wins; names match whatever their case, and any
// other `priority:*` label counts as none.

func PriorityTier(names map[string]bool) int {
	panic("unported: queue.priority_tier")
}

// Candidate is queue.Candidate.

type Candidate struct {
	Number        int
	Kind          string
	Title         string
	Forced        bool
	QueuedAt      string
	Difficulty    string   // default: DEFAULT_DIFFICULTY
	Builder       string   // default: ""
	Priority      int      // default: NO_PRIORITY
	Planned       bool     // default: False
	PlanTier      string   // default: ""
	PlannedBy     string   // default: ""
	Approved      []string // default: ()
	ApprovalTiers []string // default: ()
	BotPR         bool     // default: False
	Carries       bool     // default: False
	Rated         bool     // default: True
	Conflict      bool     // default: False
	ReviewFloor   string   // default: ""
	Mode          string   // default: ""
	Part          bool     // default: False
	AuditTier     string   // default: ""
	RoundsUsed    int      // default: 0
	RoundTier     string   // default: ""
}

// The last commit of a bot pull request that met the review rule, as deliver recorded it
// (`{"sha", "at", "by", ...}`), or `{}`. It is keyed by commit: a head that moved since is not
// cleared, whatever the record says.

func Cleared(record *pyjson.Object) *pyjson.Object {
	panic("unported: queue.cleared")
}

// A revision queued because `main` left a cleared change with conflicts.

func Carries(record *pyjson.Object) bool {
	panic("unported: queue.carries")
}

// Whether a build is planned, and by what tier: the bot's own record of its planning run,
// or else a Plan section someone put in the issue's description (`issueplan.START` … `END`),
// which counts as a strong plan: a person, or a session they ran, wrote it on purpose, and the
// bot must not plan over it. A mode (#60) plans for itself: a split is the planning, and a
// one-shot writes its own spec first, so neither waits for a plan.

func PlanOf(record *pyjson.Object, thread *pyjson.Object, kind string, mode string /* = "" */) *pyjson.Object {
	panic("unported: queue.plan_of")
}

// The weakest tier whose plan this item builds from (`config.PLAN_FLOOR`): medium for an
// easy or medium item, strong for a hard one, and medium to rate and plan an item nobody has
// rated or planned yet. One planned but never rated (a plan from before ratings, or a planner
// that gave none) still needs a plan from its difficulty's floor.

func PlanFloor(candidate *Candidate) string {
	panic("unported: queue.plan_floor")
}

// Its plan was written by a model of at least its plan floor (#317 part 6). A plan from
// before planners' tiers were recorded, or one a person put in the description, counts as
// strong.

func PlanMeets(candidate *Candidate) bool {
	panic("unported: queue.plan_meets")
}

// The Needs plan stage: a build with no plan yet, or one whose plan came from a model
// weaker than its difficulty's plan floor (a medium plan for a hard item). A weak builder,
// which cannot plan, builds only from a plan that meets its floor.

func NeedsPlan(candidate *Candidate) bool {
	panic("unported: queue.needs_plan")
}

// KIND_ORDER = {"review": 0, "revise": 1, "build": 2}
var KindOrder any // unported

// The issues a description says must close first, its Plan section left out (a planner's
// notes are not the task's).

func NamedBlockers(body string) map[int]bool {
	panic("unported: queue.named_blockers")
}

// `((the patch's name, m), n)` for a part's title, or None. Several patches can share a
// name such as "Patch v0.2.X", so the parts of one patch are those with its name and its m.

func PartOf(title string) ([1]string, int, int) {
	panic("unported: queue.part_of")
}

// The issue a thread is a sub-issue of, from GitHub's own link, or 0.

func ParentOf(thread *pyjson.Object) int {
	panic("unported: queue.parent_of")
}

// `((the patch, m), n)` for a part, or None (#1370): the patch is the parent it is a sub-issue
// of, which every part has, so two patches' parts never mix when their titles both open
// `Patch (part 2 of 4)`. A part with no parent link falls back to its title's name.

func PatchOf(thread *pyjson.Object) ([1]any, int, int) {
	panic("unported: queue.patch_of")
}

// The issues among `threads` that wait for another to close first, with what they wait for:
// the issues a "Blocked by #n" line names, GitHub's own blocked-by links, and the earlier parts
// of the same patch, the sub-issues of the same parent (`patch_of`; a part builds only once
// every part before it has closed). Only open ones
// count. GitHub is asked about open issues only when some thread names one or is a part.

func WaitsFor(ctx *hctx.Context, threads []*pyjson.Object) *pyjson.Object {
	panic("unported: queue.waits_for")
}

// Queued threads: forced requests first, then by priority tier, then second reviews,
// revisions, and the oldest builds.
//
// Read by label, so no number of open threads hides one, and a label changed since the last
// plan counts at this one. Either queue label queues either kind of thread: an issue builds
// and a pull request revises. A queue label is the request, so a thread that failed before and
// was labelled again is taken again. A thread labelled `human` or `human required` is left
// out, whatever model would take it, and so are a fullsend issue whose parts are still open
// (`open_parts`, #505),
// forced or not, and a build that waits for another issue (`waits_for`) unless it was forced,
// each with a line in `skipped` when the caller keeps one, and in `held_back` (number -> why)
// when the caller keeps that (the status issue's queue).

func Candidates(ctx *hctx.Context, state *pyjson.Object, skipped []string /* = None */, heldBack *pyjson.Object /* = None */) []*Candidate {
	panic("unported: queue.candidates")
}
