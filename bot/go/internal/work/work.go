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
	"fmt"
	"math"
	"os"
	"os/signal"
	"path/filepath"
	"strings"
	"sync"
	"syscall"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/disk"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gates"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/memory"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/prompts"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/resolve"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
)

// BuilderTools is BUILDER_TOOLS.
var BuilderTools = []string{
	"Agent", "Task", "Bash", "Read", "Edit", "Write", "MultiEdit", "Glob", "Grep",
	"NotebookEdit", "TodoWrite", "EnterWorktree", "ExitWorktree",
}

// ReaderTools is READER_TOOLS.
var ReaderTools = []string{"Agent", "Task", "Bash", "Read", "Glob", "Grep", "TodoWrite"}

// PlannerTools: a planner reads like a reviewer, and writes one file: its draft
// (`PLAN_DRAFT_FILE`). The harness puts back anything else it changes, as it always did.
var PlannerTools = joined(ReaderTools, "Edit", "Write", "MultiEdit")

// NetworkDeny: speed bumps, not a sandbox: a shell can always reach the network another way. The
// model job holds no GitHub write token, and the deliver job checks everything it publishes.
var NetworkDeny = func() []string {
	var out []string
	for _, tool := range []string{
		"curl", "wget", "nc", "ncat", "netcat", "ssh", "scp", "sftp", "rsync", "telnet", "gh",
		"git push", "git remote",
	} {
		out = append(out, "Bash("+tool+":*)")
	}
	return out
}()

// BuilderDeny is BUILDER_DENY.
var BuilderDeny = joined([]string{"WebFetch", "WebSearch"}, NetworkDeny...)

// ReaderDeny is READER_DENY.
var ReaderDeny = joined(BuilderDeny, "Edit", "Write", "MultiEdit", "NotebookEdit", "Bash(git commit:*)")

// PlannerDeny is PLANNER_DENY.
var PlannerDeny = joined(BuilderDeny, "NotebookEdit", "Bash(git commit:*)")

// joined is a tuple + a tuple: a new slice.
func joined(a []string, b ...string) []string {
	return append(append([]string{}, a...), b...)
}

// DiffInPrompt is DIFF_IN_PROMPT.
var DiffInPrompt = 60_000

// InstantFailureS: a call that fails sooner than this with nothing to show (no text) is the
// subscription's failure, not the item's: its CLI or login is broken (#317 part 1). A weak
// subscription's CLI failed every call in seconds from 2026-10-05 07:30Z, and each one was
// charged to its item as a build or an unreadable review.
var InstantFailureS = 60

// NotesFile: the builder's running notes, at the top of the worktree. Git ignores it
// (`info/exclude`), so it is never committed; the harness reads it when the run ends and hands
// it to the next agent.
var NotesFile = ".bot-notes.md"

// NotesChars is NOTES_CHARS.
var NotesChars = 8000

// NotesAsk is NOTES_ASK.
var NotesAsk = "\n\n## Keep notes for whoever picks this up\n\nKeep `" + NotesFile + "` at the " +
	"top of this worktree (git ignores it, so it is never delivered) in two\nparts, and update " +
	"it as you go, not only at the end:\n\n- `## State`, rewritten in place: the plan, what is " +
	"done, what is next, the decisions you made\n  and why, and the dead ends you hit. Keep it " +
	"short enough to read in a minute: it is what the\n  next agent reads first, so it must say " +
	"where the work stands without the log.\n- `## Log`, added to: before each step that takes " +
	"a while (a subagent, a test run, a merge, a\n  large edit), one line saying what you are " +
	"about to do and why; after it, one line saying what\n  came of it.\n\nYour session can be " +
	"cut off at any moment (a usage limit, the clock, a cancelled job), and the\nnext agent, " +
	"possibly another model, starts from this file, the journal of earlier runs and the\nbranch."

// PlanDraftFile: a planner's draft: the plan as it stands, which the planner keeps at the top of
// the worktree while it reads (`DRAFT_ASK`). A planning call cut off mid-way (a usage cap, the
// clock, a cancel) ended with nothing kept: #37 lost 20 Opus minutes of planning so, then another
// 4, and the next planner started from scratch each time. Its draft now goes into the handoff and
// the journal, and the next planner, on any subscription, goes on from it. Git ignores it.
var PlanDraftFile = ".bot-plan.md"

// DraftAsk is DRAFT_ASK.
var DraftAsk = "\n\n## Keep a draft as you go\n\nKeep your plan as it stands in `" + PlanDraftFile +
	"` at the top of this worktree (git ignores it,\nand it is the one file you may write): what " +
	"you have read and found, the files to touch so far,\nand the plan's sections as far as you " +
	"have got. Write it once you know the shape of the change,\nand update it after each step " +
	"that takes a while, not only at the end. Your session can be cut\noff at any moment (a " +
	"usage limit, the clock, a cancelled job): the next planner, possibly another\nmodel, starts " +
	"from that file and this run's journal. Your final message is still the whole plan."

// FullsendDir: a one-shot's scratch (#60): fullsend's spec, notes and damage report, and the
// worktree each of its agents works in. Git ignores it in every worktree, and the path guard
// refuses it.
var FullsendDir = ".fullsend/"

// JournalRunsShown: how many earlier runs a first prompt lists from the journal
// (`_journal_text`); the file in the worktree has them all.
var JournalRunsShown = 20

// DiedReason: what a killed job's last checkpoint says (`Worker._checkpoint`, #342):
// `result.json` holds this until the run's own end writes over it, so a job that never gets
// there still hands its work on.
var DiedReason = "the model job stopped before it finished (cancelled, past its time limit, or its " +
	"runner was lost); this is its last checkpoint, after {calls} model call(s)"

// SwitchUpAfter: rounds in a row the reviewer sends back while the builder runs on a lane's
// weaker model, before the run moves the builder to the lane's strongest one (`Worker._switch`):
// Sonnet gets two tries on a Claude account, then its Opus takes over on the same lane.
var SwitchUpAfter = 2

// SwitchAsk: what a builder on a lane with two models is told (`Worker._switch_ask`).
var SwitchAsk = "\n\n## Your model\n\nThis run's lane can run {models}, and this pass runs on " +
	"`{model}`. You choose what the next pass of\nthis run runs on: when this needs more than you " +
	"can give it (a subtle rule, a hard conflict, a fix\nyou could not find), put " +
	"`\"next_model\": \"{up}\"` in your report's header; when what is left is plain\nwork, " +
	"`\"next_model\": \"{down}\"`. Leave it out to stay on `{model}`.{floor}"

// PlanCut is PLAN_CUT.
var PlanCut = "\n\n…(the plan was cut here; the planner wrote more)"

// SelfCheckContext is SELF_CHECK_CONTEXT.
var SelfCheckContext = "This is a self check, not a review. You are the same model that built\n" +
	"this change, in a fresh session, and nothing you say here approves it: an independent " +
	"reviewer\njudges it afterwards. Adversarially find every reason your own change should not " +
	"ship, as that\nreviewer would, and report each as a blocking finding. Self check {n} of at " +
	"most {cap}."

// SettingsFiles is SETTINGS_FILES.
var SettingsFiles = []string{".claude/settings.json", ".claude/settings.local.json", ".mcp.json"}

// Manifests: the files that say what the install installs: the web's pnpm workspace and the
// Rust workspace (its manifests, its lockfile and the toolchain it pins). A change to one runs
// the install again.
var Manifests = []string{"package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml", ".npmrc",
	"Cargo.toml", "Cargo.lock", "rust-toolchain.toml"}

// LongActions: the runs that start only `start_headroom` under each cap (`_start_check`; the
// router holds the same line in `plan._usable` and `plan.lane_planners`): a build, a revision,
// and a plan, which an Opus planner can spend 20 minutes on (#37 was cut off 4 minutes into a
// plan that started just under its cap).
var LongActions = []string{"build", "revise", "plan"}

// KindStatus: how an interruption ends the item: `stop` and `infra` have statuses of their own;
// the rest (`budget`, `usage`, `halt`, and `disk` for the machine's disk filling up during the
// work) are `interrupted` and requeue.
var KindStatus = map[string]string{"stop": "stopped", "infra": "infra"}

// Interrupt: stop the item now and hand what exists to the deliver job.
type Interrupt struct {
	Reason string
	Kind   string
	// ResetAt is "" for None.
	ResetAt string
}

// NewInterrupt is Interrupt(...).
func NewInterrupt(reason string, kind string /* = "budget" */, resetAt string /* = None */) *Interrupt {
	return &Interrupt{Reason: reason, Kind: kind, ResetAt: resetAt}
}

func (e *Interrupt) Error() string { return e.Reason }

// Probe is what a probe returns when the run must stop: the reason, and its kind (`halt`, `stop`
// for a person stopping this item, `suspend` for the run's subscription suspended, or `usage`).
// ok false is Python's None.
type Probe func(lastUsage *pyjson.Object) (reason, kind string, ok bool)

// IsManifest is work._is_manifest.
func IsManifest(path string) bool {
	return contains(Manifests, path[strings.LastIndex(path, "/")+1:])
}

func contains(list []string, s string) bool {
	for _, e := range list {
		if e == s {
			return true
		}
	}
	return false
}

// or is Python's `s or def` on a string.
func or(s, def string) string {
	if s != "" {
		return s
	}
	return def
}

// fixWhy is why a fix pass runs: what sent the change back, and the first thing to fix.
func fixWhy(after string, findings []*verdicts.Finding) string {
	if len(findings) == 0 {
		return after + " sent it back"
	}
	first := findings[0]
	return fmt.Sprintf("%s sent it back with %d blocking finding(s); first `%s`: %s", after,
		len(findings), first.Where, first.Claim)
}

// verdictOutcome is what a review call came to, for the journal: its verdict and its first
// finding.
func verdictOutcome(review *verdicts.Review, result *runner.RunResult) string {
	if !review.Readable {
		return "no verdict could be read: " + callOutcome(result)
	}
	blocking := review.Blocking()
	if len(blocking) == 0 {
		return fmt.Sprintf("approve (%d note(s))", len(review.Notes()))
	}
	first := blocking[0]
	return fmt.Sprintf("changes: %d blocking; first `%s`: %s", len(blocking), first.Where, first.Claim)
}

// callOutcome is how one model call ended, in a few words, for the journal.
func callOutcome(result *runner.RunResult) string {
	turns := ""
	if result.Turns != nil && *result.Turns != 0 {
		turns = fmt.Sprintf(", %d turns", *result.Turns)
	}
	if result.Extra.Truthy("usage_stop") {
		return "cut off at its usage cap: " + py.Str(result.Extra.Get("usage_stop"))
	}
	if result.RateLimited() {
		return "refused: the usage limit"
	}
	if result.TimedOut {
		return "timed out" + turns
	}
	if !result.OK {
		return py.SliceTo(redact.Redact("failed"+turns+": "+or(result.Error, "no error given"), nil), 300)
	}
	return "ended" + turns
}

// modelRunner is what a Worker's runner does: one model call (`runner.run`).
type modelRunner interface {
	Run(request *runner.RunRequest) *runner.RunResult
}

// trailer is a runner that boils a transcript down to its end (`runner.trail`), if it has one.
type trailer interface {
	Trail(transcript string) string
}

// AskedModel is the model the builder asked its next pass to run on, and the round it asked in.
type AskedModel struct {
	Model string
	Cycle int
}

// Worker is work.Worker.
type Worker struct {
	Cfg    *config.Config
	Plan   *pyjson.Object
	Runner any
	Repo   *git.Git
	// WorkDir and OutDir are Path(work_dir) and Path(out_dir).
	WorkDir string
	OutDir  string
	Probe   Probe
	// AfterCall: run after every model call: keeps a login the CLI just refreshed
	// (`logins.seal`), so a job that is killed later still hands it on.
	AfterCall func()
	Now       func() time.Time
	Env       map[string]string
	Started   time.Time
	Deadline  time.Time
	// Templates holds each prompt's template text (`prompts.load`), read when the job starts.
	Templates map[string]string
	System    string
	Provider  *providers.Provider
	// OnMachine and Gates: the checks this run runs. On the bot's machine, only those marked for
	// it (`Gate.machine`): every job there shares two vCPUs, and CI on the pull request runs the
	// rest anyway.
	OnMachine bool
	Gates     []*config.Gate
	// DiskReader: how full the disk is (`disk.reading`), read on the machine only: its
	// subscriptions share one disk, and a full one fails whichever job writes next.
	DiskReader func() *pyjson.Object
	// MemoryReader: the machine's memory (`memory.reading`), read on the machine only (#312): as
	// the run starts, after each model call and check run, and as it ends.
	MemoryReader func() *pyjson.Object
	// BuildSeat, ReviewSeat and PlanSeat: the model each role runs on. A plan from before seats
	// runs every role on the subscription's main model, its reviewer included.
	BuildSeat  *providers.Seat
	ReviewSeat *providers.Seat
	PlanSeat   *providers.Seat
	SelfCheck  bool
	MainSeat   *providers.Seat
	// Switchable: the seats this run may move its builder between (`_switch`): the account's own
	// seats, when they span more than one tier, as each Claude account's Opus and Sonnet do.
	Switchable []*providers.Seat
	// SentBack: rounds in a row sent back while the builder ran below the lane's strongest seat.
	SentBack int
	// Asked: the model the builder asked its next pass to run on, and the round it asked in.
	Asked    *AskedModel
	PlanText string
	Minutes  float64
	// Tokens and CostUSD: this run's token totals ({input, output, cache_read, cache_write}) and
	// cost in USD, summed over its model calls that reported any (#695).
	Tokens  *pyjson.Object
	CostUSD float64
	// BuildTranscript: the last builder's or planner's transcript, whose end the handoff carries
	// ("" for None).
	BuildTranscript string
	// Planning: a planning session began (`_planning`); with no plan in the result, it was cut
	// off.
	Planning bool
	Who      *git.Identity
	// LastUsage is the latest usage reading.
	LastUsage *pyjson.Object
	// StartUsage: a fresh usage reading taken just before the run (`cmd_work`'s ping), if any.
	StartUsage *pyjson.Object
	// LastHour: the reset of the 5-hour session whose last hour this run took
	// (`providers.last_hour_of`): its 5-hour cap stays lifted until then, however far past it the
	// run goes. The zero time is None.
	LastHour time.Time
	Calls    int
	Wt       *git.Git
	BaseRef  string
	BaseSHA  string
	StartSHA string
	// InstalledAt and ApprovedSHA are "" for None.
	InstalledAt   string
	ApprovedSHA   string
	BaseWt        *git.Git
	BaseGateCache *pyjson.Object
	// PutBack: the forbidden paths the last path guard put back (`_guard`).
	PutBack []string
	// WipUsed: the last cut-off revision's work this run starts from (`bot/wip/<pr>`), if any.
	WipUsed string
	// Resolved: the conflicts of the merge of `main` the harness resolved itself (`_resolve`).
	Resolved []*resolve.Resolution
	// CarryBuild and CarryReview: for a conflict revision of a cleared change: what its builder
	// and its reviewer are told (`_cleared_notes`).
	CarryBuild  string
	CarryReview string
	Result      *pyjson.Object
	// Steps is self.result["steps"]: each new step goes into both.
	Steps []*pyjson.Object
	// Dying: a cancel or a kill arrived (`_on_signal`), or the run is saving what it has.
	Dying bool
	// Signalled: the stop a signal asked for (`_on_signal`), kept so that one swallowed by a
	// reading's `except Exception` is raised again at the next checkpoint or call (`_signalled`).
	Signalled string
	// Trapped: the signal handlers `_trap_signals` replaced, put back when the run ends: each
	// signal, and whether it was ignored before.
	Trapped map[syscall.Signal]bool
	// ReviewFunc, when set, stands in for Review wherever the run calls it: Python's tests
	// replace the bound method (`worker._review = …`).
	ReviewFunc func(cycle int, report *verdicts.BuildReport, results []*gates.GateResult,
		previous []*verdicts.Finding, context string, maxCycles *int, role string) *verdicts.Review

	sigMu sync.Mutex
	sigCh chan os.Signal
}

// NewWorker is Worker(...).
func NewWorker(cfg *config.Config, plan *pyjson.Object, runner any, repoDir string, workDir string, outDir string, probe Probe /* = None */, afterCall func() /* = None */, now func() time.Time /* = None */, env map[string]string /* = None */, diskReader func() *pyjson.Object /* = None */, memoryReader func() *pyjson.Object /* = None */) *Worker {
	x := &Worker{Cfg: cfg, Plan: plan, Runner: runner}
	x.Repo = git.NewGit(repoDir, env)
	x.WorkDir = workDir
	x.OutDir = outDir
	x.Probe = probe
	x.AfterCall = afterCall
	x.Now = now
	if x.Now == nil {
		x.Now = func() time.Time { return clock.Now(cfg.NowOverride) }
	}
	if env != nil {
		x.Env = make(map[string]string, len(env))
		for k, v := range env {
			x.Env[k] = v
		}
	} else {
		x.Env = config.ChildEnv(nil, nil)
	}
	x.Started = x.Now()
	x.Deadline = x.Started.Add(time.Duration(cfg.JobBudgetMinutes) * time.Minute)
	x.Templates = map[string]string{}
	for _, name := range prompts.Names {
		template, err := prompts.Load(name, promptsDir())
		herr.Must(err)
		x.Templates[name] = template.Template
	}
	name := config.Name
	who := name + ", a bot"
	if strings.HasPrefix(name, "the ") {
		who = name
	}
	x.System = x.substitute("system", map[string]string{
		"bot": cfg.BotLogin, "repo": cfg.Repo, "title": strings.TrimPrefix(name, "the "),
		"who": who})
	x.Provider = cfg.Pool.Get(py.OrStr(plan.Get("provider"), ""))
	if x.Provider == nil {
		x.Provider = cfg.Pool.Ordered()[0]
	}
	x.OnMachine = !providers.Hosted(py.OrStr(plan.Get("runs_on"), x.Provider.RunsOn))
	x.Gates = []*config.Gate{}
	for _, gate := range cfg.Gates {
		if gate.Machine || !x.OnMachine {
			x.Gates = append(x.Gates, gate)
		}
	}
	x.DiskReader = diskReader
	if x.DiskReader == nil {
		x.DiskReader = func() *pyjson.Object {
			dir := x.WorkDir
			if !exists(dir) {
				dir = cfg.Root
			}
			return disk.Reading(dir, x.Now())
		}
	}
	x.MemoryReader = memoryReader
	if x.MemoryReader == nil {
		x.MemoryReader = func() *pyjson.Object { return memory.Reading(x.Now(), py.Str(memory.Proc)) }
	}
	main := cfg.Pool.Seats(x.Provider)[0]
	seats := pyjson.Obj(plan.Get("seats"))
	x.BuildSeat = x.seat(seats, "build")
	if x.BuildSeat == nil {
		x.BuildSeat = main
	}
	x.ReviewSeat = main
	if seats != nil {
		x.ReviewSeat = x.seat(seats, "review")
	}
	x.PlanSeat = x.seat(seats, "plan")
	x.SelfCheck = seats != nil && seats.Truthy("self_check")
	x.MainSeat = main
	own := cfg.Pool.OwnSeats(x.Provider)
	tiers := map[string]bool{}
	for _, seat := range own {
		tiers[seat.Tier] = true
	}
	x.Switchable = []*providers.Seat{}
	if len(tiers) > 1 {
		x.Switchable = own
	}
	x.Tokens = pyjson.New("input", 0, "output", 0, "cache_read", 0, "cache_write", 0)
	x.Who = git.IdentityBot(cfg.BotLogin, cfg.BotUserID)
	x.BaseRef = "origin/" + cfg.DefaultBranch
	x.BaseGateCache = pyjson.New()
	x.PutBack = []string{}
	x.Resolved = []*resolve.Resolution{}
	x.Result = pyjson.New(
		"version", 1,
		"action", plan.Get("action"),
		"number", plan.Get("number"),
		"status", "failed",
		"reason", "",
		"started_at", clock.ISO(x.Started),
		"cycles", []any{},
		"provider", x.Provider.ID,
		"family", x.Provider.Family,
		"seats", pyjson.New("plan", seatDict(x.PlanSeat),
			"build", x.BuildSeat.ToDict(),
			"review", seatDict(x.ReviewSeat),
			"self_check", x.SelfCheck),
		// Every move of the builder between the lane's models (`_switch`).
		"switches", []any{},
		// What the run did, step by step, for the item's journal (`_step`, #342).
		"steps", []any{},
	)
	x.Steps = []*pyjson.Object{}
	x.Trapped = map[syscall.Signal]bool{}
	return x
}

// promptsDir is prompts.PROMPTS_DIR, the default every `prompts.load` reads from.
func promptsDir() string { return config.PromptsDir }

// seatDict is `seat.to_dict() if seat else None`.
func seatDict(seat *providers.Seat) any {
	if seat == nil {
		return nil
	}
	return seat.ToDict()
}

// planItem is self.plan[key]: a missing key raises KeyError.
func (x *Worker) planItem(key string) any {
	v, ok := x.Plan.Lookup(key)
	if !ok {
		herr.Raise(py.KeyError(key))
	}
	return v
}

// planIn is `self.plan.get(key) in values`.
func (x *Worker) planIn(key string, values ...string) bool {
	s, ok := x.Plan.Get(key).(string)
	return ok && contains(values, s)
}

// The seat `plan` gave `role`, if it is one this subscription has.
func (x *Worker) seat(seats *pyjson.Object, role string) *providers.Seat {
	entry := pyjson.Obj(seats.Get(role))
	if entry == nil {
		return nil
	}
	return x.Cfg.Pool.Seat(x.Provider.ID, py.OrStr(entry.Get("model"), ""))
}

// SeatFor is work.Worker.seat_for.
func (x *Worker) SeatFor(role string) *providers.Seat {
	switch role {
	case "plan":
		if x.PlanSeat != nil {
			return x.PlanSeat
		}
		return x.MainSeat
	case "review":
		if x.ReviewSeat != nil {
			return x.ReviewSeat
		}
		return x.BuildSeat
	case "suggest":
		return x.MainSeat
	}
	return x.BuildSeat // build, fix, revise, and the builder's own self check
}

// CyclesCap is in-run review verdict rounds for this item's difficulty (#725): 3 for a medium
// or easy item, then it goes to its final audit; 10 for a hard one, which stays with Opus.
func (x *Worker) CyclesCap() int {
	return py.MustInt(x.Cfg.MaxReviewCycles.GetOr(x.Difficulty(), 3))
}

// Verdict rounds earlier runs used with this owner tier (#725): usage pauses, infra
// failures, halts and stops never reach a verdict, so they never spend one; a new owner
// tier starts its own count.
func (x *Worker) roundsSpent() int {
	if py.OrStr(x.Plan.Get("round_tier"), "") != x.BuildSeat.Tier {
		return 0
	}
	used := x.Plan.Get("rounds_used")
	if !pyjson.Truthy(used) {
		used = 0
	}
	return py.MustInt(used)
}

// The run used every verdict round with this owner (#725): a hard item stays
// unapproved, for the strike path; a medium or easy item's kept work goes to its lane's
// final audit (Strong for medium, Medium for easy).
func (x *Worker) roundsOut(total int) (string, string) {
	if x.Difficulty() == "hard" {
		return "not_approved", fmt.Sprintf("no approval after %d review cycles", total)
	}
	final := "its Medium final audit"
	if x.Difficulty() == "medium" {
		final = "its Strong final audit"
	}
	return "built", fmt.Sprintf("used its %d review rounds with this owner; the work is kept for %s",
		total, final)
}

// Difficulty is the item's difficulty as this run knows it: the planner's rating when it rated
// it in this run, else the plan's.
func (x *Worker) Difficulty() string {
	rated := pyjson.Obj(x.Result.Obj("plan").Get("rating"))
	planned := py.OrStr(x.Plan.Get("difficulty"), "medium")
	if _, ok := config.MinTier[planned]; !ok {
		planned = "medium"
	}
	found := py.OrStr(rated.Get("difficulty"), planned)
	if _, ok := config.MinTier[found]; !ok {
		found = planned
	}
	if x.ratingSource() == "bot" {
		// The bot's own earlier rating is never lowered (`deliver._apply_rating`).
		if difficultyIndex(planned) > difficultyIndex(found) {
			found = planned
		}
	}
	return found
}

// difficultyIndex is `list(MIN_TIER).index(d)`: easy, medium, hard.
func difficultyIndex(d string) int {
	for i, name := range config.Difficulties {
		if name == d {
			return i
		}
	}
	return -1
}

// tierRank is providers.TIER_RANK[tier].
func tierRank(tier string) int {
	for i, name := range providers.Tiers {
		if name == tier {
			return i
		}
	}
	return -1
}

// strongestSeat is `max(seats, key=TIER_RANK[s.tier])`: the first of the strongest.
func strongestSeat(seats []*providers.Seat) *providers.Seat {
	best := seats[0]
	for _, s := range seats[1:] {
		if tierRank(s.Tier) > tierRank(best.Tier) {
			best = s
		}
	}
	return best
}

// weakestSeat is `min(seats, key=TIER_RANK[s.tier])`: the first of the weakest.
func weakestSeat(seats []*providers.Seat) *providers.Seat {
	best := seats[0]
	for _, s := range seats[1:] {
		if tierRank(s.Tier) < tierRank(best.Tier) {
			best = s
		}
	}
	return best
}

// Switch moves this run's builder to the lane's seat running `model`: each Claude account runs
// Opus and Sonnet both, and its run switches between them as the work needs, never under
// the item's difficulty (`MIN_TIER`). The reviewer stays as assigned, since the review
// rule counts its tier. Returns whether it moved.
func (x *Worker) Switch(model string, why string, cycle int) bool {
	var target *providers.Seat
	for _, seat := range x.Switchable {
		if seat.Model == model {
			target = seat
			break
		}
	}
	if target == nil || target.Model == x.BuildSeat.Model {
		return false
	}
	if !providers.TierAtLeast(target.Tier, config.MinTier[x.Difficulty()]) {
		return false
	}
	appendTo(x.Result, "switches", pyjson.New("n", cycle, "from", x.BuildSeat.Model,
		"to", target.Model, "why", why))
	x.BuildSeat = target
	x.SentBack = 0
	return true
}

// The builder's note on choosing its next pass's model, on a lane with two models.
func (x *Worker) switchAsk() string {
	if len(x.Switchable) == 0 {
		return ""
	}
	floor := config.MinTier[x.Difficulty()]
	strongest := strongestSeat(x.Switchable)
	weakest := weakestSeat(x.Switchable)
	note := ""
	if !providers.TierAtLeast(weakest.Tier, floor) {
		note = fmt.Sprintf(" This item is `difficulty:%s`, which keeps its builder on `%s`, so "+
			"asking for `%s` changes nothing.", x.Difficulty(), strongest.Model, weakest.Model)
	}
	var models []string
	for _, seat := range x.Switchable {
		models = append(models, fmt.Sprintf("`%s` (%s)", seat.Model, seat.Tier))
	}
	return pyFormat(SwitchAsk, map[string]any{"models": strings.Join(models, " and "),
		"model": x.BuildSeat.Model, "up": strongest.Model, "down": weakest.Model, "floor": note})
}

// ------------------------------------------------------------------ plumbing

// SecondsLeft is work.Worker.seconds_left.
func (x *Worker) SecondsLeft() float64 {
	return x.Deadline.Sub(x.Now()).Seconds()
}

// Check raises Interrupt when a halt, a stop, the usage stop or the clock says to stop.
func (x *Worker) Check() {
	x.signalled()
	if x.SecondsLeft() < float64(x.Cfg.MinMinutesForACall*60) {
		herr.Raise(NewInterrupt("the run's time budget is spent", "budget", ""))
	}
	if x.Probe != nil {
		if reason, kind, ok := x.Probe(x.LastUsage); ok {
			herr.Raise(NewInterrupt(reason, kind, ""))
		}
	}
}

// Render is work.Worker.render: the template `name` with every value as its str().
func (x *Worker) Render(name string, values *pyjson.Object) string {
	strs := map[string]string{}
	values.Items(func(k string, v any) bool {
		strs[k] = py.Str(v)
		return true
	})
	return x.substitute(name, strs)
}

// substitute is `self.templates[name].substitute(values)`.
func (x *Worker) substitute(name string, values map[string]string) string {
	text, ok := x.Templates[name]
	if !ok {
		herr.Raise(py.KeyError(name))
	}
	out, err := pyTemplate(text).substitute(values)
	herr.Must(err)
	return out
}

// Delete untracked Claude settings in the worktree, so no call inherits a hook or an MCP
// server an earlier call planted.
func (x *Worker) clearSettings(cwd string) {
	for _, name := range SettingsFiles {
		target := filepath.Join(cwd, name)
		if !exists(target) {
			continue
		}
		if code, _ := run(git.NewGit(cwd, x.Env), false, "ls-files", "--error-unmatch", name); code != 0 {
			unlink(target)
		}
	}
}

// Step records one step of the run for the item's journal (`journal.section`): a model call,
// a check run, a merge of `main`. Returns the entry, which the caller may fill in.
func (x *Worker) Step(step string, why string /* = "" */, fields *pyjson.Object) *pyjson.Object {
	entry := pyjson.New("at", clock.ISO(x.Now()), "step", step, "why", why)
	entry.Update(fields)
	x.Steps = append(x.Steps, entry)
	x.Result.Set("steps", pyjson.ListOf(x.Steps))
	return entry
}

// What the last model call came to, once its caller knows (a verdict, a status).
func (x *Worker) cameOfIt(outcome string) {
	for i := len(x.Steps) - 1; i >= 0; i-- {
		if s, _ := x.Steps[i].Get("step").(string); s == "call" {
			x.Steps[i].Set("outcome", py.SliceTo(outcome, 300))
			return
		}
	}
}

// Call is one model call. A `reader` may read and run things but not edit them; a `drafting`
// reader (the planner) may also write its draft (`PLAN_DRAFT_FILE`), and the end of its
// session goes into the handoff, as a builder's does.
func (x *Worker) Call(role string, prompt string, cwd string, reader bool, why string /* = "" */, drafting bool /* = False */) *runner.RunResult {
	x.signalled()    // never start a call after a cancel
	x.callHeadroom() // never start a call the cap will kill halfway (#695)
	x.Calls++
	x.clearSettings(cwd)
	timeout := int(math.Min(float64(x.Cfg.CallTimeoutMinutes*60), x.SecondsLeft()-300))
	where := x.WorkDir
	if x.Cfg.UploadTranscripts {
		where = x.OutDir
	}
	transcript := filepath.Join(where, "transcripts", fmt.Sprintf("%02d-%s.jsonl", x.Calls, role))
	if !reader {
		prompt += NotesAsk + x.switchAsk()
	}
	if !reader || drafting {
		x.BuildTranscript = transcript
	}
	if drafting {
		prompt += DraftAsk
	}
	seat := x.SeatFor(role)
	step := x.Step("call", or(why, role), pyjson.New("role", role, "model", seat.Model, "n", x.Calls))
	tools, deny := BuilderTools, BuilderDeny
	if drafting {
		tools, deny = PlannerTools, PlannerDeny
	} else if reader {
		tools, deny = ReaderTools, ReaderDeny
	}
	maxTurns := x.Cfg.MaxTurns.Get(role)
	if !pyjson.Truthy(maxTurns) {
		v, ok := x.Cfg.MaxTurns.Lookup("review")
		if !ok {
			herr.Raise(py.KeyError("review"))
		}
		maxTurns = v
	}
	// Reviews run one notch below building (#695): the subscription's `review_effort`, else
	// `high`; planning and building keep theirs.
	effort := seat.Effort
	if role == "fix" && x.Provider.FixEffort != "" {
		effort = x.Provider.FixEffort
	} else if role == "review" {
		effort = or(x.Provider.ReviewEffort, "high")
	}
	var usageStop func(*pyjson.Object) string
	if x.Provider.Limits.Stops().Len() > 0 {
		usageStop = x.usageStop
	}
	request := &runner.RunRequest{
		Role:            role,
		Prompt:          prompt,
		Cwd:             cwd,
		SystemAppend:    x.System,
		AllowedTools:    tools,
		DisallowedTools: deny,
		MaxTurns:        py.MustInt(maxTurns),
		TimeoutS:        max(60, timeout),
		Model:           seat.Model,
		Effort:          effort,
		Transcript:      transcript,
		ExtraDirs:       x.gitDirs(cwd),
		UsageStop:       usageStop,
	}
	result := x.Runner.(modelRunner).Run(request)
	step.Set("minutes", py.RoundN(result.DurationS/60, 1))
	step.Set("outcome", callOutcome(result))
	if result.Tokens.Len() > 0 {
		these := pyjson.New()
		for _, name := range x.Tokens.Keys() {
			n := tokenCount(result.Tokens, name)
			x.Tokens.Set(name, x.Tokens.Int(name)+n)
			these.Set(name, n)
		}
		step.Set("tokens", these)
	}
	if result.CostUSD != nil && *result.CostUSD != 0 {
		x.CostUSD += *result.CostUSD
		step.Set("cost_usd", py.RoundN(*result.CostUSD, 4))
	}
	x.readMemory()
	if x.AfterCall != nil {
		x.AfterCall()
	}
	x.Minutes += result.DurationS / 60
	if result.Usage.Len() > 0 {
		x.LastUsage = result.Usage
	}
	if result.Extra.Truthy("usage_stop") {
		herr.Raise(NewInterrupt(fmt.Sprintf("`%s` stops mid-call: %s", x.Provider.ID,
			py.Str(result.Extra.Get("usage_stop"))), "usage", ""))
	}
	if result.RateLimited() {
		herr.Raise(NewInterrupt(fmt.Sprintf("`%s` reached its usage limit", x.Provider.ID), "usage",
			result.ResetAt))
	}
	if result.Infra() {
		herr.Raise(NewInterrupt(fmt.Sprintf("the %s CLI could not run: %s", x.Provider.CLI,
			py.Str(noneIfEmpty(result.Error))), "infra", ""))
	}
	if !result.OK && !result.TimedOut && strings.TrimSpace(result.Text) == "" &&
		result.DurationS < float64(InstantFailureS) {
		herr.Raise(NewInterrupt(fmt.Sprintf("the %s CLI failed at once: %s", x.Provider.CLI,
			or(result.Error, "it said nothing")), "infra", ""))
	}
	return result
}

// tokenCount is `int(tokens.get(name) or 0)`.
func tokenCount(tokens *pyjson.Object, name string) int {
	v := tokens.Get(name)
	if !pyjson.Truthy(v) {
		return 0
	}
	return py.MustInt(v)
}

// While a call runs: the reading it just streamed, against the provider's caps (and
// its `off_hours` ones outside its window). A reason stops the call there.
func (x *Worker) usageStop(usage *pyjson.Object) string {
	return x.refusal(usage, false)
}

// `usage`, read now, against the provider's caps. A reading that finds its session's
// last hour (`providers.last_hour_of`) lifts the 5-hour cap for the rest of the run, until
// that session resets: the run's own spending past the cap must not stop it.
func (x *Worker) refusal(usage *pyjson.Object, starting bool /* = False */) string {
	now := x.Now()
	reading := usage.Copy()
	reading.Set("observed_at", clock.ISO(now))
	if found := providers.LastHourOf(x.Provider, reading, now, x.Cfg.Timezone); !found.IsZero() {
		x.LastHour = found
	}
	return providers.Refusal(x.Provider, pyjson.New("usage", reading), now, x.Cfg.Timezone,
		starting, x.LastHour)
}

// Before each model call: the latest reading against the caps, with no headroom margin
// (a run going on stops at the cap itself, `providers.refusal`). Short of room the run
// stops here, with its commits bundled and its handoff written (`_interrupted`), instead
// of starting a call the cap kills halfway (#695). The first call's reading is the run's
// `_start_check`, so this repeats it.
func (x *Worker) callHeadroom() {
	if x.LastUsage.Len() == 0 {
		return
	}
	if reason := x.refusal(x.LastUsage, false); reason != "" {
		herr.Raise(NewInterrupt(fmt.Sprintf("`%s` stops before its next call: %s", x.Provider.ID,
			reason), "usage", ""))
	}
}

// The fresh reading taken just before the run: a run over its cap, or a build, a
// revision or a plan without `start_headroom` under it, ends here before any model work.
// Without it a run started from whatever reading the last run left, none at all after a
// reset.
func (x *Worker) startCheck() {
	if x.StartUsage.Len() == 0 {
		return
	}
	x.LastUsage = x.StartUsage
	if reason := x.refusal(x.StartUsage, x.planIn("action", LongActions...)); reason != "" {
		herr.Raise(NewInterrupt(fmt.Sprintf("`%s` stops before it starts: %s", x.Provider.ID,
			reason), "usage", ""))
	}
}

// The repository's git directory, which a worktree's git commands write to and a
// sandboxed CLI must be told about.
func (x *Worker) gitDirs(cwd string) []string {
	code, out := run(git.NewGit(cwd, x.Env), false, "rev-parse", "--path-format=absolute",
		"--git-common-dir")
	found := ""
	if code == 0 {
		found = strings.TrimSpace(out)
	}
	if found != "" {
		return []string{found}
	}
	return []string{}
}

// WriteResult writes `result.json`: the run's end, or (`checkpoint`) what a job killed from
// here on should hand on, as a pause of kind `died` (#342).
func (x *Worker) WriteResult(checkpoint bool /* = False */) string {
	mkdirAll(x.OutDir)
	x.Result.Set("usage", objOrNone(x.LastUsage))
	x.Result.Set("finished_at", clock.ISO(x.Now()))
	x.Result.Set("model_calls", x.Calls)
	x.Result.Set("minutes", py.RoundN(x.Minutes, 1))
	x.Result.Set("tokens", x.Tokens.Copy())
	x.Result.Set("cost_usd", py.RoundN(x.CostUSD, 4))
	if handoff := x.Handoff(); handoff.Len() > 0 {
		x.Result.Set("handoff", handoff)
	}
	written := x.Result
	if checkpoint {
		written = x.Result.Copy()
		written.Set("status", "interrupted")
		written.Set("interrupt", "died")
		written.Set("reason", pyFormat(DiedReason, map[string]any{"calls": x.Calls}))
		written.Set("checkpoint", true)
	}
	path := filepath.Join(x.OutDir, "result.json")
	temp := filepath.Join(x.OutDir, "result.json.tmp")
	writeText(temp, pyjson.DumpsIndent(redact.RedactJSON(written), 2)+"\n")
	herr.Must(os.Rename(temp, path)) // a kill mid-write leaves the last whole one
	return path
}

// After a builder pass, a check run or a review: leave in `result.json` what this job
// should hand on if it is killed before its end (#342), its commits bundled. The upload
// step and the deliver job run whatever happens to this one (`if: always()`), and the
// run's own end writes over it. A build or a revision only: nothing else has work to keep.
func (x *Worker) checkpoint() {
	if x.Wt == nil || x.isDying() || !x.planIn("action", "build", "revise") {
		return
	}
	err := herr.Try(func() {
		x.recordHead()
		x.WriteResult(true)
	})
	if err == nil {
		return
	}
	if stop, ok := err.(*Interrupt); ok {
		herr.Raise(stop) // a cancel that arrived while it wrote
	}
	// A checkpoint never stops the work.
	x.Result.Set("checkpoint_error", py.SliceTo(redact.Redact(herr.Str(err), nil), 500))
}

func (x *Worker) isDying() bool {
	x.sigMu.Lock()
	defer x.sigMu.Unlock()
	return x.Dying
}

// OnSignal: GitHub cancels a job (a person, or its time limit) with SIGINT, then SIGTERM, then
// a kill. Stop as a pause, so the work is committed, bundled and handed on (`_interrupted`),
// instead of ending as `failed` with nothing kept. A second signal waits for the save.
//
// Python runs it on the main thread, where its Interrupt unwinds whatever the run was doing. A
// Go signal arrives on its own goroutine (`trapSignals`), which cannot unwind the run's: there
// the raise ends with that goroutine, and the stop it keeps (`Signalled`) is raised at the run's
// next checkpoint or call (`_signalled`), as one an `except Exception` swallowed is in Python.
func (x *Worker) OnSignal(signum int, frame any) {
	x.sigMu.Lock()
	if x.Dying {
		x.sigMu.Unlock()
		return
	}
	x.Dying = true
	signal.Ignore(syscall.SIGINT, syscall.SIGTERM)
	x.Signalled = fmt.Sprintf("the job was stopped (%s): cancelled, or past its time limit",
		signalName(signum))
	stop := NewInterrupt(x.Signalled, "died", "")
	x.sigMu.Unlock()
	herr.Raise(stop)
}

// signalName is signal.Signals(signum).name.
func signalName(signum int) string {
	switch syscall.Signal(signum) {
	case syscall.SIGHUP:
		return "SIGHUP"
	case syscall.SIGINT:
		return "SIGINT"
	case syscall.SIGQUIT:
		return "SIGQUIT"
	case syscall.SIGKILL:
		return "SIGKILL"
	case syscall.SIGUSR1:
		return "SIGUSR1"
	case syscall.SIGUSR2:
		return "SIGUSR2"
	case syscall.SIGPIPE:
		return "SIGPIPE"
	case syscall.SIGALRM:
		return "SIGALRM"
	case syscall.SIGTERM:
		return "SIGTERM"
	}
	return fmt.Sprintf("SIG%d", signum)
}

// Raise the stop a signal asked for again, if something caught it on the way.
func (x *Worker) signalled() {
	x.sigMu.Lock()
	reason := x.Signalled
	x.sigMu.Unlock()
	if reason != "" {
		herr.Raise(NewInterrupt(reason, "died", ""))
	}
}

var (
	// trapMu and trapping stand for Python's "only the main thread may": one run at a time in
	// the process catches the signals; a run beside it (another goroutine's) leaves them be.
	trapMu   sync.Mutex
	trapping bool
)

// Catch SIGINT and SIGTERM for the run (`_on_signal`). Only the main thread may.
func (x *Worker) trapSignals() map[syscall.Signal]bool {
	trapMu.Lock()
	defer trapMu.Unlock()
	if trapping {
		return map[syscall.Signal]bool{}
	}
	trapping = true
	old := map[syscall.Signal]bool{}
	sigs := []os.Signal{syscall.SIGINT, syscall.SIGTERM}
	for _, sig := range sigs {
		old[sig.(syscall.Signal)] = signal.Ignored(sig)
	}
	ch := make(chan os.Signal, len(sigs))
	signal.Notify(ch, sigs...)
	go func() {
		for sig := range ch {
			func() {
				defer func() { _ = recover() }() // the raise ends here: see OnSignal
				x.OnSignal(int(sig.(syscall.Signal)), nil)
			}()
		}
	}()
	x.sigCh = ch
	x.Trapped = old
	return old
}

// restoreSignals puts back the handlers `trapSignals` replaced: the default, or ignored for a
// signal that was ignored before.
func (x *Worker) restoreSignals(trapped map[syscall.Signal]bool) {
	if len(trapped) == 0 {
		return
	}
	var sigs []os.Signal
	for _, sig := range []syscall.Signal{syscall.SIGINT, syscall.SIGTERM} {
		if _, ok := trapped[sig]; ok {
			sigs = append(sigs, sig)
		}
	}
	// Go's Stop puts the default back only for a signal it relays, not one it ignores.
	signal.Notify(x.sigCh, sigs...)
	signal.Stop(x.sigCh)
	close(x.sigCh)
	for _, sig := range sigs {
		if trapped[sig.(syscall.Signal)] {
			signal.Ignore(sig)
		}
	}
	trapMu.Lock()
	trapping = false
	trapMu.Unlock()
}

// While the run saves what it has (a pause, a failure, its end), a cancel waits: a stop
// raised in the middle of the save would leave half of it.
func (x *Worker) holdSignals() {
	x.sigMu.Lock()
	x.Dying = true
	x.sigMu.Unlock()
	for _, sig := range []syscall.Signal{syscall.SIGINT, syscall.SIGTERM} {
		if _, ok := x.Trapped[sig]; ok {
			signal.Ignore(sig)
		}
	}
}

// ------------------------------------------------------------------ entry

// Run is work.Worker.run.
func (x *Worker) Run() *pyjson.Object {
	mkdirAll(x.OutDir)
	trapped := x.trapSignals()
	func() {
		defer func() {
			x.holdSignals()
			defer x.restoreSignals(trapped)
			if x.OnMachine {
				x.readDisk("end")
				x.readMemory()
			}
			x.WriteResult(false)
		}()
		err := herr.Try(func() {
			if x.OnMachine {
				x.readMemory()
				x.diskCheck()
			}
			x.startCheck()
			switch action := x.Plan.Get("action"); {
			case action == "suggest":
				x.suggest()
			case action == "plan":
				x.planRun()
			case action == "build" || action == "revise":
				x.Item()
			case action == "review":
				x.SecondReview()
			default:
				x.Result.Set("status", "nothing")
				x.Result.Set("reason", "the plan had nothing to do")
			}
		})
		if err == nil {
			return
		}
		if stop, ok := err.(*Interrupt); ok {
			x.Interrupted(stop)
			return
		}
		// Every failure must still leave a result.
		x.holdSignals()
		reason := py.SliceTo(redact.Redact(herr.Name(err)+": "+herr.Str(err), nil), 2000)
		if disk.IsFull(err) {
			// The machine's disk, not the item: a pause that keeps what was built.
			x.Interrupted(NewInterrupt("the machine's disk filled up ("+reason+")", "disk", ""))
		} else {
			x.Result.Set("status", "failed")
			x.Result.Set("reason", reason)
			x.saveWip("the harness failed")
		}
	}()
	return x.Result
}

// Fold the machine's memory now into the result's `memory` (`memory.add`).
func (x *Worker) readMemory() {
	if !x.OnMachine {
		return
	}
	_ = herr.Try(func() { // a reading never stops the work
		record, _ := x.Result.SetDefault("memory", pyjson.New()).(*pyjson.Object)
		memory.Add(record, x.MemoryReader())
	})
}

func (x *Worker) readDisk(when string) *pyjson.Object {
	record, _ := x.Result.SetDefault("disk", pyjson.New()).(*pyjson.Object)
	var found *pyjson.Object
	if err := herr.Try(func() { found = x.DiskReader() }); err != nil {
		if herr.IsOSError(err) {
			return nil
		}
		herr.Raise(err)
	}
	record.Set(when, objOrNone(found))
	return found
}

// number is a JSON number as a float64, for comparing sizes.
func number(v any) float64 {
	f, _ := pyjson.AsFloat(v)
	return f
}

// free is found["free"]: a missing key raises KeyError.
func free(found *pyjson.Object) float64 {
	v, ok := found.Lookup("free")
	if !ok {
		herr.Raise(py.KeyError("free"))
	}
	return number(v)
}

// On the machine, before any work: room for this job (`disk.py`). Short of
// `CLEAN_BELOW` free, clean what this user's jobs can do without first; still short of
// `FLOOR`, stop on the machine's account, never the item's.
func (x *Worker) diskCheck() {
	found := x.readDisk("start")
	x.Result.Obj("disk").Set("runner", os.Getenv("RUNNER_NAME"))
	if found == nil || free(found) >= number(disk.CleanBelow) {
		return
	}
	x.Result.Obj("disk").Set("cleaned", disk.Clean(x.Cfg.Root, x.Env))
	if again := x.readDisk("after_clean"); again.Len() > 0 {
		found = again
	}
	if free(found) < number(disk.Floor) {
		x.Result.Set("infra_scope", "machine")
		herr.Raise(NewInterrupt(fmt.Sprintf("the machine's disk is %s, under the %s a job needs, "+
			"even after cleaning", disk.Describe(found), disk.Size(disk.Floor)), "infra", ""))
	}
}
