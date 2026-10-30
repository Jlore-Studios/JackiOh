// Package work is bot/harness/work.py.
//
// The model job: one item through builder -> checks -> adversarial review, a review run of a bot
// pull request, a planning run, or a suggestion survey, all on the one subscription `plan` chose.
//
// Each role runs on the seat `plan` gave it (`plan["seats"]`: a model and its tier on this
// subscription): a planning session first when the item has no plan yet, the builder and its
// fixes, and the run's own adversarial reviewer. A builder whose seat has `self_check` (a weak one)
// checks its own change before any review: build, checks, a self check by a fresh session of the same
// model, and on findings a fix and the checks and the self check again, up to
// `max_self_check_rounds`. A clean self check is never an approval. A run whose subscription has no
// seat of at least medium to review in the run (only weak ones) ends with the change built, for a
// review run to judge; a weak model reviews only in a review run, and only an easy item.
//
// A fullsend part (#505, `plan["onto"]`) is one builder pass from its tree's branch and nothing
// more (`_part`): its tree's reconcile, an ordinary build of the parent on that branch, runs the
// checks and the review for every part at once.
//
// This job holds no GitHub write credential. It writes `result.json` and a git bundle of the branch
// to the output directory; the deliver job checks the bundle itself and pushes it. Prompts are read
// into memory when the job starts, so nothing the model writes to disk can change what a later
// call is asked.
package work

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gates"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/resolve"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
	"time"
)

// BUILDER_TOOLS = ( "Agent", "Task", "Bash", "Read", "Edit", "Write", "MultiEdit", "Glob", "Grep", "NotebookEdit", "TodoWrite", "EnterWorktree", "ExitWorktree", )
var BuilderTools = []string{"Agent", "Task", "Bash", "Read", "Edit", "Write", "MultiEdit", "Glob", "Grep", "NotebookEdit", "TodoWrite", "EnterWorktree", "ExitWorktree"}

// READER_TOOLS = ("Agent", "Task", "Bash", "Read", "Glob", "Grep", "TodoWrite")
var ReaderTools = []string{"Agent", "Task", "Bash", "Read", "Glob", "Grep", "TodoWrite"}

// PLANNER_TOOLS = READER_TOOLS + ("Edit", "Write", "MultiEdit")
var PlannerTools any // unported

// NETWORK_DENY = tuple(f"Bash({tool}:*)" for tool in ( "curl", "wget", "nc", "ncat", "netcat", "ssh", "scp", "sftp", "rsync", "telnet", "gh", "git push", "git rem
var NetworkDeny any // unported

// BUILDER_DENY = ("WebFetch", "WebSearch") + NETWORK_DENY
var BuilderDeny any // unported

// READER_DENY = BUILDER_DENY + ("Edit", "Write", "MultiEdit", "NotebookEdit", "Bash(git commit:*)")
var ReaderDeny any // unported

// PLANNER_DENY = BUILDER_DENY + ("NotebookEdit", "Bash(git commit:*)")
var PlannerDeny any // unported

// DIFF_IN_PROMPT = 60_000
var DiffInPrompt int = 60000

// INSTANT_FAILURE_S = 60
var InstantFailureS int = 60

// NOTES_FILE = ".bot-notes.md"
var NotesFile string = ".bot-notes.md"

// NOTES_CHARS = 8000
var NotesChars int = 8000

// NOTES_ASK = f""" ## Keep notes for whoever picks this up Keep `{NOTES_FILE}` at the top of this worktree (git ignores it, so it is never delivered) in two parts
var NotesAsk any // unported

// PLAN_DRAFT_FILE = ".bot-plan.md"
var PlanDraftFile string = ".bot-plan.md"

// DRAFT_ASK = f""" ## Keep a draft as you go Keep your plan as it stands in `{PLAN_DRAFT_FILE}` at the top of this worktree (git ignores it, and it is the one fil
var DraftAsk any // unported

// FULLSEND_DIR = ".fullsend/"
var FullsendDir string = ".fullsend/"

// JOURNAL_RUNS_SHOWN = 20
var JournalRunsShown int = 20

// DIED_REASON = ("the model job stopped before it finished (cancelled, past its time limit, or its " "runner was lost); this is its last checkpoint, after {calls}
var DiedReason string = "the model job stopped before it finished (cancelled, past its time limit, or its runner was lost); this is its last checkpoint, after {calls} model call(s)"

// SWITCH_UP_AFTER = 2
var SwitchUpAfter int = 2

// SWITCH_ASK = """ ## Your model This run's lane can run {models}, and this pass runs on `{model}`. You choose what the next pass of this run runs on: when this n
var SwitchAsk string = "\n\n## Your model\n\nThis run's lane can run {models}, and this pass runs on `{model}`. You choose what the next pass of\nthis run runs on: when this needs more than you can give it (a subtle rule, a hard conflict, a fix\nyou could not find), put `\"next_model\": \"{up}\"` in your report's header; when what is left is plain\nwork, `\"next_model\": \"{down}\"`. Leave it out to stay on `{model}`.{floor}"

// PLAN_CUT = "\n\n…(the plan was cut here; the planner wrote more)"
var PlanCut string = `

…(the plan was cut here; the planner wrote more)`

// SELF_CHECK_CONTEXT = """This is a self check, not a review. You are the same model that built this change, in a fresh session, and nothing you say here approves
var SelfCheckContext string = `This is a self check, not a review. You are the same model that built
this change, in a fresh session, and nothing you say here approves it: an independent reviewer
judges it afterwards. Adversarially find every reason your own change should not ship, as that
reviewer would, and report each as a blocking finding. Self check {n} of at most {cap}.`

// SETTINGS_FILES = (".claude/settings.json", ".claude/settings.local.json", ".mcp.json")
var SettingsFiles = []string{".claude/settings.json", ".claude/settings.local.json", ".mcp.json"}

// MANIFESTS = ("package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml", ".npmrc", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml")
var Manifests = []string{"package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml", ".npmrc", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml"}

// LONG_ACTIONS = ("build", "revise", "plan")
var LongActions = []string{"build", "revise", "plan"}

// KIND_STATUS = {"stop": "stopped", "infra": "infra"}
var KindStatus = map[string]string{"stop": "stopped", "infra": "infra"}

// Stop the item now and hand what exists to the deliver job.

type Interrupt struct {
	Reason  any // reason
	Kind    any // kind
	ResetAt any // reset_at
}

// NewInterrupt is Interrupt(...).

func NewInterrupt(reason string, kind string /* = "budget" */, resetAt string /* = None */) *Interrupt {
	panic("unported: work.Interrupt.__init__")
}

// Probe = Callable[[dict | None], "tuple[str, str] | None"]
var Probe any // unported

// _is_manifest is work._is_manifest.

func IsManifest(path string) bool {
	panic("unported: work._is_manifest")
}

// Worker is work.Worker.

type Worker struct {
	Cfg             any                   // cfg
	Plan            any                   // plan
	Runner          any                   // runner
	Repo            any                   // Git(repo_dir, env)
	WorkDir         any                   // Path(work_dir)
	OutDir          any                   // Path(out_dir)
	Probe           any                   // probe
	AfterCall       any                   // after_call
	Now             any                   // now or (lambda: clock_now(cfg.now_override))
	Env             any                   // dict(env) if env is not None else child_env()
	Started         any                   // self.now()
	Deadline        any                   // self.started + timedelta(minutes=cfg.job_budget_minutes)
	Templates       any                   // {name: prompts.load(name) for name in prompts.NAMES}
	System          any                   // self.templates["system"].substitute( bot=cfg.bot_login, repo=cfg.repo, title=NAME.removeprefix("the "), who=NAME if NAME.startswith("the ") else f"{NAME}, a bot
	Provider        any                   // cfg.pool.get(plan.get("provider")) or cfg.pool.ordered()[0]
	OnMachine       any                   // not hosted(str(plan.get("runs_on") or self.provider.runs_on))
	Gates           any                   // [gate for gate in cfg.gates if gate.machine or not self.on_machine]
	DiskReader      any                   // disk_reader or (lambda: disk_mod.reading( self.work_dir if self.work_dir.exists() else cfg.root, self.now()))
	MemoryReader    any                   // memory_reader or (lambda: memory_mod.reading(self.now()))
	BuildSeat       any                   // self._seat(seats, "build") or main
	ReviewSeat      any                   // self._seat(seats, "review") if seats is not None else main
	PlanSeat        any                   // self._seat(seats, "plan")
	SelfCheck       any                   // bool(seats.get("self_check")) if seats is not None else False
	MainSeat        any                   // main
	Switchable      any                   // own if len({seat.tier for seat in own}) > 1 else []
	SentBack        any                   // 0
	Asked           any                   // None
	PlanText        any                   // ""
	Minutes         any                   // 0.0
	Tokens          any                   // {"input": 0, "output": 0, "cache_read": 0, "cache_write": 0}
	CostUSD         any                   // 0.0
	BuildTranscript string                // None
	Planning        any                   // False
	Who             any                   // Identity.bot(cfg.bot_login, cfg.bot_user_id)
	LastUsage       *pyjson.Object        // None
	StartUsage      *pyjson.Object        // None
	LastHour        time.Time             // None
	Calls           any                   // 0
	Wt              *git.Git              // None
	BaseRef         any                   // f"origin/{cfg.default_branch}"
	BaseSHA         any                   // ""
	StartSHA        any                   // ""
	InstalledAt     string                // None
	ApprovedSHA     string                // None
	BaseWt          *git.Git              // None
	BaseGateCache   *pyjson.Object        // {}
	PutBack         []string              // []
	WipUsed         any                   // ""
	Resolved        []*resolve.Resolution // []
	CarryBuild      any                   // ""
	CarryReview     any                   // ""
	Result          *pyjson.Object        // { "version": 1, "action": plan.get("action"), "number": plan.get("number"), "status": "failed", "reason": "", "started_at": iso(self.started), "cycles": [], "pr
	Steps           []*pyjson.Object      // self.result["steps"]
	Dying           any                   // False
	Signalled       any                   // ""
	Trapped         *pyjson.Object        // {}
}

// NewWorker is Worker(...).

func NewWorker(cfg *config.Config, plan *pyjson.Object, runner any, repoDir string, workDir string, outDir string, probe any /* = None */, afterCall func() /* = None */, now func() time.Time /* = None */, env map[string]string /* = None */, diskReader func() *pyjson.Object /* = None */, memoryReader func() *pyjson.Object /* = None */) *Worker {
	panic("unported: work.Worker.__init__")
}

// The seat `plan` gave `role`, if it is one this subscription has.

func (x *Worker) seat(seats *pyjson.Object, role string) any {
	panic("unported: work.Worker._seat")
}

func (x *Worker) SeatFor(role string) any {
	panic("unported: work.Worker.seat_for")
}

// In-run review verdict rounds for this item's difficulty (#725): 3 for a medium or
// easy item, then it goes to its final audit; 10 for a hard one, which stays with Opus.

func (x *Worker) CyclesCap() int {
	panic("unported: work.Worker._cycles_cap")
}

// Verdict rounds earlier runs used with this owner tier (#725): usage pauses, infra
// failures, halts and stops never reach a verdict, so they never spend one; a new owner
// tier starts its own count.

func (x *Worker) roundsSpent() int {
	panic("unported: work.Worker._rounds_spent")
}

// The run used every verdict round with this owner (#725): a hard item stays
// unapproved, for the strike path; a medium or easy item's kept work goes to its lane's
// final audit (Strong for medium, Medium for easy).

func (x *Worker) roundsOut(total int) (string, string) {
	panic("unported: work.Worker._rounds_out")
}

// The item's difficulty as this run knows it: the planner's rating when it rated it in
// this run, else the plan's.

func (x *Worker) Difficulty() string {
	panic("unported: work.Worker._difficulty")
}

// Move this run's builder to the lane's seat running `model`: each Claude account runs
// Opus and Sonnet both, and its run switches between them as the work needs, never under
// the item's difficulty (`MIN_TIER`). The reviewer stays as assigned, since the review
// rule counts its tier. Returns whether it moved.

func (x *Worker) Switch(model string, why string, cycle int) bool {
	panic("unported: work.Worker._switch")
}

// The builder's note on choosing its next pass's model, on a lane with two models.

func (x *Worker) switchAsk() string {
	panic("unported: work.Worker._switch_ask")
}

func (x *Worker) SecondsLeft() float64 {
	panic("unported: work.Worker.seconds_left")
}

// Raise Interrupt when a halt, a stop, the usage stop or the clock says to stop.

func (x *Worker) Check() {
	panic("unported: work.Worker.check")
}

func (x *Worker) Render(name string, values *pyjson.Object) string {
	panic("unported: work.Worker.render")
}

// Delete untracked Claude settings in the worktree, so no call inherits a hook or an MCP
// server an earlier call planted.

func (x *Worker) clearSettings(cwd string) {
	panic("unported: work.Worker._clear_settings")
}

// Record one step of the run for the item's journal (`journal.section`): a model call,
// a check run, a merge of `main`. Returns the entry, which the caller may fill in.

func (x *Worker) Step(step string, why string /* = "" */, fields *pyjson.Object) *pyjson.Object {
	panic("unported: work.Worker._step")
}

// What the last model call came to, once its caller knows (a verdict, a status).

func (x *Worker) cameOfIt(outcome string) {
	panic("unported: work.Worker._came_of_it")
}

// One model call. A `reader` may read and run things but not edit them; a `drafting`
// reader (the planner) may also write its draft (`PLAN_DRAFT_FILE`), and the end of its
// session goes into the handoff, as a builder's does.

func (x *Worker) Call(role string, prompt string, cwd string, reader bool, why string /* = "" */, drafting bool /* = False */) *runner.RunResult {
	panic("unported: work.Worker.call")
}

// While a call runs: the reading it just streamed, against the provider's caps (and
// its `off_hours` ones outside its window). A reason stops the call there.

func (x *Worker) usageStop(usage *pyjson.Object) string {
	panic("unported: work.Worker._usage_stop")
}

// `usage`, read now, against the provider's caps. A reading that finds its session's
// last hour (`providers.last_hour_of`) lifts the 5-hour cap for the rest of the run, until
// that session resets: the run's own spending past the cap must not stop it.

func (x *Worker) refusal(usage *pyjson.Object, starting bool /* = False */) string {
	panic("unported: work.Worker._refusal")
}

// Before each model call: the latest reading against the caps, with no headroom margin
// (a run going on stops at the cap itself, `providers.refusal`). Short of room the run
// stops here, with its commits bundled and its handoff written (`_interrupted`), instead
// of starting a call the cap kills halfway (#695). The first call's reading is the run's
// `_start_check`, so this repeats it.

func (x *Worker) callHeadroom() {
	panic("unported: work.Worker._call_headroom")
}

// The fresh reading taken just before the run: a run over its cap, or a build, a
// revision or a plan without `start_headroom` under it, ends here before any model work.
// Without it a run started from whatever reading the last run left, none at all after a
// reset.

func (x *Worker) startCheck() {
	panic("unported: work.Worker._start_check")
}

// The repository's git directory, which a worktree's git commands write to and a
// sandboxed CLI must be told about.

func (x *Worker) gitDirs(cwd string) []string {
	panic("unported: work.Worker._git_dirs")
}

// Write `result.json`: the run's end, or (`checkpoint`) what a job killed from here on
// should hand on, as a pause of kind `died` (#342).

func (x *Worker) WriteResult(checkpoint bool /* = False */) string {
	panic("unported: work.Worker.write_result")
}

// After a builder pass, a check run or a review: leave in `result.json` what this job
// should hand on if it is killed before its end (#342), its commits bundled. The upload
// step and the deliver job run whatever happens to this one (`if: always()`), and the
// run's own end writes over it. A build or a revision only: nothing else has work to keep.

func (x *Worker) checkpoint() {
	panic("unported: work.Worker._checkpoint")
}

// GitHub cancels a job (a person, or its time limit) with SIGINT, then SIGTERM, then a
// kill. Stop as a pause, so the work is committed, bundled and handed on (`_interrupted`),
// instead of ending as `failed` with nothing kept. A second signal waits for the save.

func (x *Worker) OnSignal(signum int, frame any) {
	panic("unported: work.Worker._on_signal")
}

// Raise the stop a signal asked for again, if something caught it on the way.

func (x *Worker) signalled() {
	panic("unported: work.Worker._signalled")
}

// Catch SIGINT and SIGTERM for the run (`_on_signal`). Only the main thread may.

func (x *Worker) trapSignals() *pyjson.Object {
	panic("unported: work.Worker._trap_signals")
}

// While the run saves what it has (a pause, a failure, its end), a cancel waits: a stop
// raised in the middle of the save would leave half of it.

func (x *Worker) holdSignals() {
	panic("unported: work.Worker._hold_signals")
}

func (x *Worker) Run() *pyjson.Object {
	panic("unported: work.Worker.run")
}

// Fold the machine's memory now into the result's `memory` (`memory.add`).

func (x *Worker) readMemory() {
	panic("unported: work.Worker._read_memory")
}

func (x *Worker) readDisk(when string) *pyjson.Object {
	panic("unported: work.Worker._read_disk")
}

// On the machine, before any work: room for this job (`disk.py`). Short of
// `CLEAN_BELOW` free, clean what this user's jobs can do without first; still short of
// `FLOOR`, stop on the machine's account, never the item's.

func (x *Worker) diskCheck() {
	panic("unported: work.Worker._disk_check")
}

func (x *Worker) install() *gates.GateResult {
	panic("unported: work.Worker._install")
}

func (x *Worker) prepare(mergeMain bool /* = True */, install bool /* = True */) []string {
	panic("unported: work.Worker._prepare")
}

// The conflicts the merge of `main` left that the harness settles itself (`resolve.py`,
// `resolvers` in the config); the rest are the builder's. Returns those.

func (x *Worker) Resolve(conflicts []string, commands bool) []string {
	panic("unported: work.Worker._resolve")
}

// What the builder is told of the files the harness resolved ("" when none): one it took
// stays as it is; one a command regenerated is that command's output from the merged tree,
// so it goes stale when the builder changes what the command reads.

func (x *Worker) resolvedNote() string {
	panic("unported: work.Worker._resolved_note")
}

// What every reviewer of the run is told of the files the harness resolved ("" when
// none): by a rule, so the branch's side of each is gone on purpose.

func (x *Worker) resolvedReviewNote() string {
	panic("unported: work.Worker._resolved_review_note")
}

// Where a revision's worktree starts: the last cut-off revision's work on
// `bot/wip/<pr>` (#317 part 3), when it started from the pull request's head as it is now
// and descends from it; otherwise (someone pushed since) the pull request's head. The
// work's `start` stays the pull request's head either way, which deliver checks.

func (x *Worker) wipStart(number int, start string) string {
	panic("unported: work.Worker._wip_start")
}

// Tell git to ignore the notes file and the journal in every worktree of this clone
// (the machine keeps its clone between jobs, so this lasts there).

func (x *Worker) excludeNotes() {
	panic("unported: work.Worker._exclude_notes")
}

// Start the notes file from the handoff's notes (a planner's plan, or an earlier
// builder's notes), so this run's builder keeps them going, and the draft file from a
// cut-off planner's draft, so this run's planner does; and put the item's journal beside
// them (#342).

func (x *Worker) seedNotes() {
	panic("unported: work.Worker._seed_notes")
}

// What the next agent needs if this run did not finish the item: the builder's notes
// and the end of its last session. None when there is neither.

func (x *Worker) Handoff() *pyjson.Object {
	panic("unported: work.Worker._handoff")
}

// The planner's draft, while no plan is finished: its beginning, where the goal and
// the first steps are. A finished plan supersedes it, so a run that has one hands on none.

func (x *Worker) draft() string {
	panic("unported: work.Worker._draft")
}

// What a first prompt gets when other agents worked on this before: the last one's
// handoff, and the journal of every earlier run (#342).

func (x *Worker) HandoffText(builder bool /* = True */) string {
	panic("unported: work.Worker._handoff_text")
}

// What a planner is told when an earlier planning run was cut off before it finished:
// where its draft is, and the end of its session.

func (x *Worker) draftSection(handoff *pyjson.Object) string {
	panic("unported: work.Worker._draft_section")
}

// The section a first prompt gets when earlier runs left a journal (`journal.py`).

func (x *Worker) journalText(builder bool /* = True */) string {
	panic("unported: work.Worker._journal_text")
}

// The section a first prompt gets when another agent worked on this before.

func (x *Worker) handoffSection(builder bool /* = True */) string {
	panic("unported: work.Worker._handoff_section")
}

// What a revision is told about the last cut-off run's work (#317 part 3).

func (x *Worker) wipText() string {
	panic("unported: work.Worker._wip_text")
}

func (x *Worker) branchState() string {
	panic("unported: work.Worker._branch_state")
}

func (x *Worker) GateList() string {
	panic("unported: work.Worker._gate_list")
}

func (x *Worker) findingsText(findings []*verdicts.Finding, label string /* = "Blocking findings" */) string {
	panic("unported: work.Worker._findings_text")
}

// Why the run's first builder pass runs, for the journal.

func (x *Worker) FirstWhy(role string, conflicts []string) string {
	panic("unported: work.Worker._first_why")
}

func (x *Worker) FirstPrompt(conflicts []string) (string, string) {
	panic("unported: work.Worker._first_prompt")
}

// What a conflict revision of a cleared change tells its builder and its reviewer, when
// it starts from the commit the review rule cleared: only its own review stands between
// the resolution and `main`. ("", "") for any other run. Deliver decides the carry from
// git alone (`deliver._carry`).

func (x *Worker) clearedNotes(conflicts []string) (string, string) {
	panic("unported: work.Worker._cleared_notes")
}

func (x *Worker) fixPrompt(cycle int, findings []*verdicts.Finding, failures string, label string /* = "Blocking findings" */) string {
	panic("unported: work.Worker._fix_prompt")
}

func (x *Worker) commit(message string) bool {
	panic("unported: work.Worker._commit")
}

// Changed files that still hold a conflict marker line.

func (x *Worker) unresolvedMarkers() []string {
	panic("unported: work.Worker._unresolved_markers")
}

// Commits the branch may carry content from: where it started, and the main it merged.

func (x *Worker) anchors() []string {
	panic("unported: work.Worker._anchors")
}

// Put back forbidden paths the work changed, and report them, conflict markers left
// behind, and an empty change.

func (x *Worker) guard() []*verdicts.Finding {
	panic("unported: work.Worker._guard")
}

// Install again when a manifest changed (or never succeeded), then every gate.

func (x *Worker) checks() []*gates.GateResult {
	panic("unported: work.Worker._checks")
}

// A check run, for the journal, then a checkpoint.

func (x *Worker) checksStep(results []*gates.GateResult) {
	panic("unported: work.Worker._checks_step")
}

// Run each gate this change left red again on the untouched base. A gate that timed out
// is not: it is inconclusive (`gates.mark_inconclusive`), and on the base it would only
// spend its whole timeout again.

func (x *Worker) MarkPreExisting(results []*gates.GateResult) {
	panic("unported: work.Worker._mark_pre_existing")
}

func (x *Worker) baseWorktree() *git.Git {
	panic("unported: work.Worker._base_worktree")
}

func (x *Worker) Review(cycle int, report *verdicts.BuildReport, results []*gates.GateResult, previous []*verdicts.Finding, context string /* = "" */, maxCycles *int /* = None */, role string /* = "review" */) *verdicts.Review {
	panic("unported: work.Worker._review")
}

// A planning session before any building: the planner reads the task and the code and
// writes the plan, which the harness puts at the top of the notes file (and so into the
// handoff) for the builder, this run's or a later one's. It keeps a draft as it goes
// (`PLAN_DRAFT_FILE`), which a cut-off run hands on (`_handoff`) for the next planner.

func (x *Worker) planning() {
	panic("unported: work.Worker._planning")
}

func (x *Worker) ratingSource() string {
	panic("unported: work.Worker._rating_source")
}

// What the planner is told about rating the item (#317 part 8).

func (x *Worker) ratingAsk() string {
	panic("unported: work.Worker._rating_ask")
}

func (x *Worker) easyRule() string {
	panic("unported: work.Worker._easy_rule")
}

// A round the reviewer sent back, built on `built_on`. On a lane with two models,
// `SWITCH_UP_AFTER` of them in a row on the weaker one move the builder to the stronger for
// the next pass; one the stronger built counts towards nothing.

func (x *Worker) sentBack(cycle int, builtOn any) {
	panic("unported: work.Worker._sent_back")
}

// A build run whose planner rated the item (an unrated one, or one the bot rated before
// and the planner rated higher) stops after planning when the rating needs a stronger model
// than this run's builder, or a stronger planner than this one (`PLAN_FLOOR`): the item goes
// back to the queue, rated, for those. A lane with two models switches instead.

func (x *Worker) ratedOut() bool {
	panic("unported: work.Worker._rated_out")
}

// A planning run: the plan, and nothing built. The deliver job keeps it as the item's
// handoff and queues the item to build from it.

func (x *Worker) planRun() {
	panic("unported: work.Worker._plan_run")
}

// A plan that leaves the bot nothing it may build (#793, `human.plan_needs`): the run
// ends `blocked`, flagged `human_required`, before any builder is spent on it, and the
// plan goes into the description for the person who does it.

func (x *Worker) needsAPerson() bool {
	panic("unported: work.Worker._needs_a_person")
}

// One builder session and its commit. Returns its report, or None when it stopped to
// ask a person (the run then ends as `blocked`), and the round's record.

func (x *Worker) buildPass(cycle int, role string, prompt string, why string /* = "" */) (any, *pyjson.Object) {
	panic("unported: work.Worker._build_pass")
}

// A conflict revision with nothing left for a builder: the harness resolved every
// conflict the merge of `main` left (`_resolve`), and nothing else was asked of it (no new
// trusted feedback, no earlier findings, no cut-off work to pick up). Its checks and its
// reviewer judge the merge as it stands, so no builder session is spent on it; findings the
// review sends back go to the builder as on any round.

func (x *Worker) MergeOnly(conflicts []string) bool {
	panic("unported: work.Worker._merge_only")
}

// Round one of a `_merge_only` revision: commit the merge as the harness resolved it,
// with no builder session. Returns the report the pull request shows in the builder's place
// and the round's record, shaped as `_build_pass`'s.

func (x *Worker) mergePass(cycle int) (any, *pyjson.Object) {
	panic("unported: work.Worker._merge_pass")
}

// Self checks until one is clean: a fresh session of the builder's own model reads the
// change as a reviewer would; on findings the builder fixes them, the checks run and the
// self check goes again, up to `max_self_check_rounds`. Returns the last report, checks and
// path guard, and the findings still open (none after a clean self check). The report is
// None when a fix stopped to ask a person.

func (x *Worker) selfCheckLoop(cycle int, entry *pyjson.Object, report any, results []*gates.GateResult, guard []*verdicts.Finding) (any, []*gates.GateResult, []*verdicts.Finding, []*verdicts.Finding) {
	panic("unported: work.Worker._self_check_loop")
}

func (x *Worker) Item() {
	panic("unported: work.Worker._item")
}

// Before a change leaves the run with no reviewer in it: one fix pass when conflict
// markers are left in it and there is time. Returns the marker finding still open (None when
// there are none), and the builder's latest report (None when that pass stopped to ask a
// person, which ends the run).

func (x *Worker) clearMarkers(cycle int, entry *pyjson.Object, report any) (*verdicts.Finding, any) {
	panic("unported: work.Worker._clear_markers")
}

// Before a review: merge `main` again when it moved since the run merged it and merges
// cleanly (#160, #317 part 12), so what is approved still merges; `main` moves about every
// half hour and a long run's approval was often stale before it was delivered. A merge
// that would conflict is left for the conflict path, and a conflict revision of a cleared
// change is left alone (deliver redoes its merge to check the carry). True when merged.

func (x *Worker) catchUp() bool {
	panic("unported: work.Worker._catch_up")
}

// A build whose whole change was in paths the bot may not change (`bot/`, `.harness/`,
// `.github/`, ...): the path guard put it all back, so no fix or review can deliver it. The
// run ends now, asking a person, with nothing bundled, instead of spending its rounds.

func (x *Worker) OnlyForbidden() bool {
	panic("unported: work.Worker._only_forbidden")
}

// A halt or a stop said during the last review still counts: no finished change is
// handed on past one. The clock, the usage stop and a suspension of the run's subscription
// no longer matter here: its model calls are over, so the finished change is handed on.

func (x *Worker) lastCheckpoint() {
	panic("unported: work.Worker._last_checkpoint")
}

// A review run: the lane's final audit (#725). A model reads a bot pull request (a
// strong one for a medium or hard item, the weakest shipping seat for an easy one).
// Nothing is built or pushed: the verdict goes
// to `deliver`, which merges once the approvals for the same commit meet the review rule
// (`review_rule.py`), or asks for a revision at the auditor's tier.
// Nothing is installed or run either: the reviewer holds another subscription's login, and
// the builder's code (a postinstall script, a test) must not run beside it. CI runs every
// check on the pull request before it can merge.

func (x *Worker) SecondReview() {
	panic("unported: work.Worker._second_review")
}

// A split (#60): one session reads the issue and the code and answers with sub-issues,
// each small enough for one run (`split.py`); nothing is built, installed or committed. The
// deliver job opens them. With `children` in the plan, every sub-issue the issue had has
// closed and this is its close-out: `done`, or what is still missing.

func (x *Worker) Split() {
	panic("unported: work.Worker._split")
}

// A fullsend part (#505): one builder pass from its tree's branch (`onto`, the base
// `_prepare` set), and the path guard, and nothing else: no checks, no self check, no
// catch-up with `main` and no review. The parts land on that branch blind to each other, as
// fullsend's builders write their slices, and the tree's reconcile checks and reviews them
// all at once. The deliver job lands it (`deliver.Deliverer._land_part`).

func (x *Worker) Part() {
	panic("unported: work.Worker._part")
}

// Commit what the worktree holds, so the deliver job can keep it.

func (x *Worker) saveWip(reason string) {
	panic("unported: work.Worker._save_wip")
}

func (x *Worker) Interrupted(stop *Interrupt) {
	panic("unported: work.Worker._interrupted")
}

// Record the branch's head and bundle what it has beyond where it started.
//
// An approved change is bundled exactly as the reviewer saw it: a commit that appeared
// after the review (a process the model left behind, say) is dropped.

func (x *Worker) finish() {
	panic("unported: work.Worker._finish")
}

// The branch's head, what it changed, and a bundle of what it has beyond where it
// started, in the result: at the run's end (`_finish`) and at each checkpoint.

func (x *Worker) recordHead() {
	panic("unported: work.Worker._record_head")
}

func (x *Worker) suggest() {
	panic("unported: work.Worker._suggest")
}

// Every suggestion class and the priority it opens with, in pickup order (`PRIORITY_TIERS`:
// a class with no priority comes before one with `low`), for `suggest.md`.

func ClassPriorities(priorities map[string]string) string {
	panic("unported: work.class_priorities")
}

// Problems with the prompt files: a missing template or a stray `$`.

func CheckTemplates() []string {
	panic("unported: work.check_templates")
}
