// Package deliver is bot/harness/deliver.py.
//
// The last job of a night run: check the model job's output and publish it.
//
// This job holds the bot's token and runs no model. It does not trust the model job's files: the
// item comes from the plan job's outputs and the branch is worked out again here, and the bundle
// must hold the head the result names, descend from the commit the work started at, change no
// forbidden path, and fast-forward the branch on GitHub.
//
// The review rule (`review_rule.py`): a commit ships when its lane's final audit approved it
// (one strong model for a medium or hard item, one medium model for an easy item), and no
// model's rejection of it stands. A weak approval never ships anything. Votes are
// kept per commit, one per review, with the tier of the seat that cast them (`_vote`), from the
// plan's seats, never from the model job. A pull request whose commit has that gets auto-merge,
// which merges it once the required CI checks pass; one still short of it waits in
// `bot:cross-review` for a review run.
//
// A commit that meets the rule is recorded as cleared (`_clear`). When `main` then leaves it with
// conflicts, the revision that resolves them carries the clearance (`_carry`) if it started from
// that commit, its own run's reviewer approved it, and git shows it changed nothing beyond the files
// the merge left conflicted; it then ships on that one approval, with no review run.
package deliver

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// REPORT_CHARS = 30_000
var ReportChars int = 30000

// HANDOFF_NOTES = 8000
var HandoffNotes int = 8000

// HANDOFF_TRAIL = 6000
var HandoffTrail int = 6000

// DIED_LIMIT = 2
var DiedLimit int = 2

// VAULT_ATTEMPTS = 4
var VaultAttempts int = 4

// NO_RESULT = ("the model job left no result: it failed before the model started (the doctor " "step, the install or the CLI setup), or it was cancelled")
var NoResult string = "the model job left no result: it failed before the model started (the doctor step, the install or the CLI setup), or it was cancelled"

// load_result is deliver.load_result.

func LoadResult(outDir string) *pyjson.Object {
	panic("unported: deliver.load_result")
}

// Deliverer is deliver.Deliverer.

type Deliverer struct {
	Ctx        any       // ctx
	Cfg        any       // ctx.cfg
	GH         gh.Client // ctx.gh
	Plan       any       // dict(plan)
	Provider   any       // pool.get(self.plan.get("provider")) or pool.get( providers_mod.LEGACY_PROVIDER) or pool.ordered()[0]
	BuildSeat  any       // seat("build") or main
	ReviewSeat any       // seat("review") if seats else main
	OutDir     any       // Path(out_dir)
	Repo       any       // Git(repo_dir)
	Result     any       // load_result(out_dir)
	Log        []string  // []
	Chain      any       // True
}

// NewDeliverer is Deliverer(...).

func NewDeliverer(ctx *hctx.Context, plan *pyjson.Object, outDir string, repoDir string, action string /* = None */, number *int /* = None */, provider string /* = None */) *Deliverer {
	panic("unported: deliver.Deliverer.__init__")
}

func (x *Deliverer) Run() *pyjson.Object {
	panic("unported: deliver.Deliverer.run")
}

// Keep a login the CLI refreshed during the run (`vault.py`) for this provider's next
// run. The model job sealed it; this job cannot open it and only checks its shape.

func (x *Deliverer) storeVault() {
	panic("unported: deliver.Deliverer._store_vault")
}

// A job on the bot's machine read its disk: keep the newest reading, and open, rewrite
// or close the issue about a disk filling up (`disk.py`).

func (x *Deliverer) machineDisk() {
	panic("unported: deliver.Deliverer._machine_disk")
}

// A job on the bot's machine read its memory: keep its summary for the status issue's
// line on the machine's memory (`memory.py`, #312).

func (x *Deliverer) machineMemory() {
	panic("unported: deliver.Deliverer._machine_memory")
}

// A model call went through on this run's subscription: any streak of runs that could
// not work there is over (`providers.clear_infra`), and the issue asking a person about it,
// if one was opened, is closed.

func (x *Deliverer) loginWorks() {
	panic("unported: deliver.Deliverer._login_works")
}

// After `providers.INFRA_ASK_AFTER` runs in a row that could not work on this
// subscription, open one issue asking a person to fix it (the bot cannot: a login is a
// secret, and `.harness/` is not the bot's to change).

func (x *Deliverer) askAboutInfra(streak int, reason string) {
	panic("unported: deliver.Deliverer._ask_about_infra")
}

// Work the branch out again from GitHub, whatever the plan file says.

func (x *Deliverer) rederive() {
	panic("unported: deliver.Deliverer._rederive")
}

func (x *Deliverer) openPull() *pyjson.Object {
	panic("unported: deliver.Deliverer._open_pull")
}

// Turn auto-merge off on the branch's open PR (or on PR `number`), and take it out of the
// merge queue, before unapproved work lands on it.

func (x *Deliverer) holdAutoMerge(number *int /* = None */) {
	panic("unported: deliver.Deliverer._hold_auto_merge")
}

// Push the bundle's branch. `(pushed, problem)`; no bundle is `(False, "")`. A bundle
// whose head holds a conflict marker in a file it changed is refused (#317 part 2).

func (x *Deliverer) publish(approved bool) (bool, string) {
	panic("unported: deliver.Deliverer._publish")
}

// A stop or a halt that came after the model job's last checkpoint still counts.

func (x *Deliverer) lateStop(number int, status string) string {
	panic("unported: deliver.Deliverer._late_stop")
}

func (x *Deliverer) Item() {
	panic("unported: deliver.Deliverer._item")
}

func (x *Deliverer) deliverItem(number int) {
	panic("unported: deliver.Deliverer._deliver_item")
}

// A planning run's end: its plan is the item's handoff (`_keep_handoff`), and the item
// goes back to the queue to build from it on the cheapest model its difficulty allows.

func (x *Deliverer) planned(number int, status string) {
	panic("unported: deliver.Deliverer._planned")
}

// Label the item with the difficulty its planner rated (#317 part 8), and say so: only
// when no person set one (`queue.rating_source`), never lower than the bot's own earlier
// rating, and easy only when the plan's **Files to touch** table holds to the easy rule
// (`easy.plan_breach`, borderline included) and the rating's `why` admits no doubt
// (`easy.uncertain_why`, #695). Records `difficulty_by`. Returns the sentence for the
// comment.

func (x *Deliverer) ApplyRating(number int) string {
	panic("unported: deliver.Deliverer._apply_rating")
}

// Leave exactly `difficulty:<difficulty>` on the thread, of the difficulty labels.

func (x *Deliverer) setDifficulty(number int, difficulty string, labels map[string]bool) {
	panic("unported: deliver.Deliverer._set_difficulty")
}

// Who wrote this run's plan, and that model's tier.

func (x *Deliverer) planner() (string, string) {
	panic("unported: deliver.Deliverer._planner")
}

// Put this run's plan into the issue's description, in place of an earlier one, and
// take the item out of the Needs plan stage. False when the description could not take it.

func (x *Deliverer) planIntoIssue(number int, who string) bool {
	panic("unported: deliver.Deliverer._plan_into_issue")
}

// Someone closed the issue or pull request while the run worked on it: its work is not
// pushed, no pull request opens, and nothing queues it again.

func (x *Deliverer) closed(number int, thread *pyjson.Object) {
	panic("unported: deliver.Deliverer._closed")
}

// Settle the asks this run took (`asks`): 🎉 when it answered them, 😕 when it ended
// without an answer, or back to waiting when the item is queued again.

func (x *Deliverer) closeAsks(number int) {
	panic("unported: deliver.Deliverer._close_asks")
}

// Keep what a run that did not finish the item leaves for the next agent: its notes
// and the end of its session (`work.py`). A finished item's handoff is dropped.
//
// A planning session cut off before its plan was done leaves its draft (`work.
// PLAN_DRAFT_FILE`) and the end of its session: that handoff is a `draft`, which the next
// planner goes on from and no builder reads. Before this a cut-off plan kept nothing (#37).

func (x *Deliverer) keepHandoff(number int) {
	panic("unported: deliver.Deliverer._keep_handoff")
}

// Append this run to the item's journal on `bot-journal` (#342): what it was, each step
// the work job took and why, how it ended, and the builder's notes. A record, never a gate:
// a failure is logged and the delivery stands.

func (x *Deliverer) journal(number int) {
	panic("unported: deliver.Deliverer._journal")
}

func (x *Deliverer) journalLink(number int) string {
	panic("unported: deliver.Deliverer._journal_link")
}

func (x *Deliverer) Labels(number int) map[string]bool {
	panic("unported: deliver.Deliverer._labels")
}

func (x *Deliverer) Record(number int) *pyjson.Object {
	panic("unported: deliver.Deliverer._record")
}

func (x *Deliverer) remember(number int, fields *pyjson.Object) {
	panic("unported: deliver.Deliverer._remember")
}

func (x *Deliverer) link() string {
	panic("unported: deliver.Deliverer._link")
}

// The builder's seat, and every move the run made between its lane's models (each
// Claude account switches between Opus and Sonnet: `work.Worker._switch`).

func (x *Deliverer) builtOn() string {
	panic("unported: deliver.Deliverer._built_on")
}

func (x *Deliverer) requeueLabel(number int, kind string) {
	panic("unported: deliver.Deliverer._requeue_label")
}

// A run that could not work. Its subscription is left alone for a while
// (`providers.INFRA_BACKOFFS`), so the item goes to another one. When the model job reported
// why (its login, its CLI, an install on untouched main), that is all. When the job left no
// result at all (its runner died or timed out), it may be this item too: it goes to the back
// of the queue, and two in a row block it.

func (x *Deliverer) infra(number int, kind string, requeue string /* = None */) {
	panic("unported: deliver.Deliverer._infra")
}

// Keep what a run that did not finish made: a build's on its own branch, a revision's
// on `bot/wip/<pr>` (#317 part 3), which the next revision starts from. Before this a
// revision cut off by a usage cap threw its work away: #143 lost 16 Opus runs so.

func (x *Deliverer) keepWork(number int, kind string) (bool, string) {
	panic("unported: deliver.Deliverer._keep_work")
}

// Push an unfinished revision's bundle to `bot/wip/<pr>`, never to the pull request's
// branch. The same checks as `_publish`, conflict markers aside (a resolution may stop
// half-way): the head the result names, descending from where the work started, the pull
// request's branch not moved meanwhile, and no forbidden path. `(pushed, problem)`.

func (x *Deliverer) publishWip(number int) (bool, string) {
	panic("unported: deliver.Deliverer._publish_wip")
}

// A revision delivered, or the pull request closed: its unfinished work is done with.

func (x *Deliverer) dropWip(number int) {
	panic("unported: deliver.Deliverer._drop_wip")
}

// Whether a cut-off run's kept work moved on: a commit of its own, not a merge of
// `main`, past `before` (the head the last cut-off run kept, `bot/wip/<pr>`), or past where
// this run started when it did not build on that. A revision that resumes from its wip
// pushes it again every time, so "pushed" alone is no progress.

func (x *Deliverer) progressed(number int, kind string, before string) bool {
	panic("unported: deliver.Deliverer._progressed")
}

func (x *Deliverer) Interrupted(number int, kind string) {
	panic("unported: deliver.Deliverer._interrupted")
}

// Some builder session of the run worked: it ended well, or it changed something.

func (x *Deliverer) builderWorked() bool {
	panic("unported: deliver.Deliverer._builder_worked")
}

// A weak builder's change on an easy item that breaks the easy rule (`easy.change_breach`:
// too many files or lines, or an off-limits path) is not pushed (#317 part 8): its work is
// kept for a stronger builder (a build's on its own branch, with no pull request; a
// revision's on `bot/wip/<pr>`), the item becomes medium (its label, or a floor under a
// person's label), and it is queued again. True when it did so.

func (x *Deliverer) easyBreach(number int, kind string) bool {
	panic("unported: deliver.Deliverer._easy_breach")
}

// Lines added plus removed per path of the bundle's head against `main`, or, for a
// fullsend part (#505), against its tree's branch: the parts before it are not its change.

func (x *Deliverer) bundleCounts() *pyjson.Object {
	panic("unported: deliver.Deliverer._bundle_counts")
}

// One failure of the item's own (`stepup.strike`): True when it was its third, and the
// item was stepped up (or blocked at hard) instead of ending the way the caller would.

func (x *Deliverer) strike(number int, why string) bool {
	panic("unported: deliver.Deliverer._strike")
}

// A run that produced nothing usable: requeue it, or block it after too many.

func (x *Deliverer) Fail(number int, kind string) {
	panic("unported: deliver.Deliverer._fail")
}

// Only a person can do it (#793, `human.py`): the bot hands the thread over, as a
// person's `human` would (`triage.hand_to_people`). Its own labels come off
// (`human.LETS_GO`), it unassigns itself and assigns the people, `human required` goes on,
// and a comment names them and says what they must do. The queue leaves it alone from then
// on (`queue.is_human`).

func (x *Deliverer) NeedsPerson(number int, question string, what string /* = "build this" */, kept string /* = "" */, again string /* = "" */) {
	panic("unported: deliver.Deliverer._needs_person")
}

// One of my suggestions, approved, that I could not finish (#797): logged, so no survey
// offers it again (`declined`). A thread that was never a suggestion is left out.

func (x *Deliverer) cameToNothing(number int, why string) {
	panic("unported: deliver.Deliverer._came_to_nothing")
}

// After a finished run: clear the one-run flags, and go round again for a comment that
// arrived while the run held the thread (after a stop, one that came after it).

func (x *Deliverer) settle(number int) {
	panic("unported: deliver.Deliverer._settle")
}

func (x *Deliverer) build(number int, status string) {
	panic("unported: deliver.Deliverer._build")
}

// A split's end: open its sub-issues, or close a tree whose close-out found it done.
//
// The tree is checked again here (`split.from_dict`), not trusted from the model job. The
// sub-issues are opened in build order, each with its plan as the **Plan** section of its
// description (which both bots build from without planning again), its type labels as far
// as the repository has them, the parent's priority, its difficulty, and the queue label of
// whoever builds it: this bot's for `split` and `fullsend`, the other bot's for `split-bot`.
// Then they become GitHub sub-issues of the parent, linked by "blocked by", and the parent
// becomes a tree (`squishy:tree`) with a checklist.
//
// A fullsend split (#505) first makes the parent's own branch from `main`'s head, the branch
// its parts land on (`_land_part`), and each part's record says so (`onto`, `part_of`).

func (x *Deliverer) SplitOut(number int) {
	panic("unported: deliver.Deliverer._split_out")
}

// The branch a fullsend tree's parts land on (#505): the parent's own, made from `main`'s
// head when GitHub does not have it yet. `(branch, problem)`.

func (x *Deliverer) integrationBranch(number int) (string, string) {
	panic("unported: deliver.Deliverer._integration_branch")
}

// A fullsend part's end (#505): its change lands on its tree's branch (`onto`), never on
// `main`, with no pull request. It fast-forwards the branch, or joins the parts that landed
// while it was built in a merge commit; the branch is never pushed with force. A part that
// conflicts with them, or loses the race to push twice, is kept on its own branch instead
// (`parked` on the parent's record), for the reconcile to merge. The bundle is refused as
// `_publish` refuses one: the head the result names, descending from where the work started,
// no forbidden path, no conflict marker. Either way the part closes as done: the reconcile
// checks and reviews every part at once.

func (x *Deliverer) LandPart(number int) {
	panic("unported: deliver.Deliverer._land_part")
}

// Push the part's bundle onto `onto`: `(sha landed, problem)`, and `("", "")` when it
// conflicts with what is there, or the push lost the race twice.

func (x *Deliverer) land(onto string, number int) (string, string) {
	panic("unported: deliver.Deliverer._land")
}

// A merge commit of `local` onto `tip`, made without a worktree, or "" when the two
// conflict.

func (x *Deliverer) mergeCommit(tip string, local string, message string) string {
	panic("unported: deliver.Deliverer._merge_commit")
}

// Keep a part that could not land on its own branch, for the reconcile: "" when it is
// there, or the problem.

func (x *Deliverer) park(own string) string {
	panic("unported: deliver.Deliverer._park")
}

// In-run review verdict rounds for this item's difficulty (#725).

func (x *Deliverer) CyclesCap() int {
	panic("unported: deliver.Deliverer._cycles_cap")
}

// Verdict rounds this item used with its current owner, and that owner's tier (#725),
// for the record: usage pauses and infra never reach a verdict, so they never spend one.

func (x *Deliverer) rounds() (int, string) {
	panic("unported: deliver.Deliverer._rounds")
}

// The tier of the seat that just reviewed this pull request (#725): a revision that
// answers its findings stays at that tier, so the auditor fixes and pushes it.

func (x *Deliverer) auditTier() string {
	panic("unported: deliver.Deliverer._audit_tier")
}

// The first words of a comment on a build or revision that was not approved: the real
// round count, and the real reason when it stopped before its last round.

func (x *Deliverer) notApprovedHead() string {
	panic("unported: deliver.Deliverer._not_approved_head")
}

func (x *Deliverer) whyMd() string {
	panic("unported: deliver.Deliverer._why_md")
}

// Label a run that used every round `bot:stuck`, for a person to review; the sentence
// that says so, or "" for one that stopped sooner.

func (x *Deliverer) markStuck(numbers ...int) string {
	panic("unported: deliver.Deliverer._mark_stuck")
}

// An approval clears `bot:stuck`.

func (x *Deliverer) unstick(numbers ...int) {
	panic("unported: deliver.Deliverer._unstick")
}

// What a change built without a shipping review in its run went through.

func (x *Deliverer) selfCheckSaid() string {
	panic("unported: deliver.Deliverer._self_check_said")
}

func (x *Deliverer) createPull(title string, branch string, body string, draft bool) *pyjson.Object {
	panic("unported: deliver.Deliverer._create_pull")
}

func (x *Deliverer) Revise(number int, status string) {
	panic("unported: deliver.Deliverer._revise")
}

// The item's difficulty: its labels, and what its record carries (a bot PR keeps its
// issue's).

func (x *Deliverer) Difficulty(number int) string {
	panic("unported: deliver.Deliverer._difficulty")
}

// The head's approvals as (family, tier), one per review.

func (x *Deliverer) approvals(votes *pyjson.Object) [][2]string {
	panic("unported: deliver.Deliverer._approvals")
}

// A commit ships when its approvals meet the review rule for its difficulty
// (`review_rule.met`) and no model's rejection of it stands (a model that rejected it and
// later approved it has withdrawn it).

func (x *Deliverer) RuleMet(votes *pyjson.Object, difficulty string /* = "medium" */) bool {
	panic("unported: deliver.Deliverer._rule_met")
}

// Record this run's reviewer's verdict on `head`, with its tier: an approval adds one
// review (once per run and family), a rejection withdraws that family's approvals of the
// head. A new
// head starts afresh: its builder is this run's model when this run built it, and unknown
// otherwise. `carried` is the clearance a conflict resolution carries to it (`_carry`).

func (x *Deliverer) Vote(pr int, head string, approve bool, builder bool, carried *pyjson.Object /* = None */) *pyjson.Object {
	panic("unported: deliver.Deliverer._vote")
}

// Record this run's reviewer's approval of `head` on the PR, then turn on auto-merge
// when the rule is met (recording `head` as cleared); otherwise turn it off and queue a
// review run. Returns the line for the comment.

func (x *Deliverer) approved(pr int, head string, merge bool /* = True */, vote bool /* = True */, builder bool /* = True */, carried *pyjson.Object /* = None */) string {
	panic("unported: deliver.Deliverer._approved")
}

// Record `head` as cleared: its approvals met the review rule. A conflict with `main`
// later needs only the resolving run's own review (`_carry`). A clearance carried across
// a resolution keeps who cleared the change, and adds who reviewed the resolution.

func (x *Deliverer) clear_(pr int, head string, votes *pyjson.Object) {
	panic("unported: deliver.Deliverer._clear")
}

// Whether a revision carries its change's clearance to `head` (`review_rule`): it was
// queued because `main` left the change with conflicts, it started from the commit the
// rule cleared, a medium or strong model reviewed it in the run, and it changed nothing but
// the files the merge left conflicted. That last is worked out here from git, never from
// the model job: `head` against the merge git makes by itself of the cleared commit and the
// `main` that `head` took in. Returns the carry, or None and why not (empty when the
// revision was not a conflict on a cleared change).

func (x *Deliverer) Carry(record *pyjson.Object, head string) (*pyjson.Object, string) {
	panic("unported: deliver.Deliverer._carry")
}

// What a conflict revision of a cleared change did with the clearance, for the comment
// ("" for any other revision).

func (x *Deliverer) carryLine(record *pyjson.Object, carry *pyjson.Object, whyNot string) string {
	panic("unported: deliver.Deliverer._carry_line")
}

// A change no model in its run could review: its commit has no vote yet, and it waits
// for a review run.

func (x *Deliverer) awaitReview(pr int, head string, queue bool /* = True */) string {
	panic("unported: deliver.Deliverer._await_review")
}

func (x *Deliverer) waitForReview(pr int, votes *pyjson.Object, labels map[string]bool, difficulty string) string {
	panic("unported: deliver.Deliverer._wait_for_review")
}

// A head no model reviewed (someone pushed during the run, or no model here may
// review): its votes start empty.

func (x *Deliverer) voteReset(pr int, head string, builder string /* = "" */) *pyjson.Object {
	panic("unported: deliver.Deliverer._vote_reset")
}

// Whether the set-up subscriptions could still give the reviews the head needs
// (`review_rule.reachable`).

func (x *Deliverer) reviewerSetUp(votes *pyjson.Object, difficulty string) bool {
	panic("unported: deliver.Deliverer._reviewer_set_up")
}

// The issue's labels its pull request starts with, approved or a draft: its difficulty,
// so the same models revise and review it, its priority tier (#90), and `person merges`
// (#839).

func (x *Deliverer) Carried(issue int) []string {
	panic("unported: deliver.Deliverer._carried")
}

// Act on a review run's verdict on a bot pull request.

func (x *Deliverer) SecondReview(number int, status string) {
	panic("unported: deliver.Deliverer._second_review")
}

// The pull request's head as git sees it on origin now (the API can lag a push).

func (x *Deliverer) branchHead(number int) string {
	panic("unported: deliver.Deliverer._branch_head")
}

func (x *Deliverer) failReview(number int, record *pyjson.Object) {
	panic("unported: deliver.Deliverer._fail_review")
}

func (x *Deliverer) try(action any) string {
	panic("unported: deliver.Deliverer._try")
}

// What the branch changes against the default branch, as it now stands on GitHub.

func (x *Deliverer) changed(branch string) []string {
	panic("unported: deliver.Deliverer._changed")
}

// The review rule approved the head but auto-merge cannot take it: label it `ready for
// merge` and ask the operator to merge it.

func (x *Deliverer) handToAPerson(pull *pyjson.Object, why string) string {
	panic("unported: deliver.Deliverer._hand_to_a_person")
}

// Turn on auto-merge only when nothing needs a person and CI must pass first, pinned to
// `expected_head` (the approved commit) when given.

func (x *Deliverer) autoMerge(pull *pyjson.Object, expectedHead string /* = "" */) string {
	panic("unported: deliver.Deliverer._auto_merge")
}

func (x *Deliverer) mergeTitle(pull *pyjson.Object) string {
	panic("unported: deliver.Deliverer._merge_title")
}

func (x *Deliverer) rerunCI(number int) {
	panic("unported: deliver.Deliverer._rerun_ci")
}

// The first blocking finding's claim, cut short, for a strike's reason.

func (x *Deliverer) firstFinding() string {
	panic("unported: deliver.Deliverer._first_finding")
}

func (x *Deliverer) findingsMd() string {
	panic("unported: deliver.Deliverer._findings_md")
}

func (x *Deliverer) pullBody(number int, approved bool) string {
	panic("unported: deliver.Deliverer._pull_body")
}

func (x *Deliverer) Suggestions() {
	panic("unported: deliver.Deliverer._suggestions")
}

// Start the next run at once when a lane is free and a subscription can take more.

func (x *Deliverer) Chain_() {
	panic("unported: deliver.Deliverer._chain")
}
