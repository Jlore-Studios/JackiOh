// Package dashboard is bot/harness/dashboard.py.
//
// The pinned status issue (#148): one issue, pinned to the top of the issue list, that the
// `bot-status` loop rewrites every five minutes with what the night bot is doing and what it has
// done.
//
// It opens with its date, then a bar chart of how far the owner's PC's two local jobs have played (the
// learn lane's export of `LEARN_GAMES` with its shard, and gather's games of its patch, from the PC's
// heartbeat: `progress`), then a headline (what runs, what waits and on what, which subscriptions can
// start work), then draws the same facts as `/harness status` (`status.py`) for a glance: what each
// lane is doing and a timeline of the runs going now, the PC's jobs among them; every subscription
// as a box with what it is doing or why it
// is not, and its usage, always; every subscription's lanes, held and open, Squishy's too, with the
// totals and the limits on them (`lanes_table`); the queue, each item with the tier its difficulty needs and what it
// waits for (`waiting.py`, the planner's own rules on the lanes held now: never "free" for a
// subscription the planner would give no work, never "starts next" for an item it would not start);
// each subscription's usage as bars and charts against its caps; the last runs; the statistics
// (`stats.py`: the last six hours, day, week and all time, counted every 30 minutes and kept in the
// issue between counts, with which set's cards win most, `cardsets.py`, read from production once a
// day); and the full status text folded underneath (left out first when the issue would
// pass GitHub's limit, `fit`: `/harness status` gives it). Times are clock times, which
// stay true between rewrites; hovering one shows how long it had been running when the issue was
// written. Editing an issue's body notifies nobody, so the rewrites stay quiet. The issue's number
// is kept in the state file; when it is missing or the issue was closed, an open issue with the
// title and the marker, opened by the bot or someone on the trust list, is adopted, or a new one is
// opened and pinned.
//
// Every runner has a section too, low in the issue (`runners_section`): each job a runner holds now,
// the step it is on and what it is for, and the jobs queued for one, as boxes the way the
// subscriptions are drawn and as a table, each folding away. The owner's PC's group carries its load
// and the gather workflow, from a heartbeat its supervisor writes to a gist (`read_runners`). Between
// rewrites the loop redraws that section and the progress chart alone every two minutes
// (`refresh_runners`). What each runner
// and each workflow is for changes only with the pool or the workflows, so it is a comment on the issue
// (`explain`), which the section links.
//
// Squishy (#60) has a section of its own in the same issue, apart from the night bot's: what it is
// working on, its account, its queue and its trees. Squishy's own process draws it (`section`), and
// the night bot's status loop runs that process each tick and puts the section in (`update`'s
// `extra`), so each bot's facts come from its own state and labels. Its record goes in the
// statistics, folded, drawn the same way (`stats.bot_section`).
package dashboard

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/waiting"
	"time"
)

// TITLE = "Night bot status"
var Title string = "Night bot status"

// MARKER = "<!-- jackioh-bot:dashboard -->"
var Marker string = "<!-- jackioh-bot:dashboard -->"

// LABELS = ("night bot",)
var Labels = []string{"night bot"}

// EVERY_MINUTES = 5
var EveryMinutes int = 5

// MAX_BODY = 65_000
var MaxBody int = 65000

// QUEUE_ROWS = 15
var QueueRows int = 15

// RUN_ROWS = 8
var RunRows int = 8

// BAR_CELLS = 10
var BarCells int = 10

// BOX_CELLS = 5
var BoxCells int = 5

// RESULT = {"success": "✅", "failure": "❌", "cancelled": "⏹️", "skipped": "⏭️", "timed_out": "⌛", "startup_failure": "❌"}
var Result = map[string]string{"success": "✅", "failure": "❌", "cancelled": "⏹️", "skipped": "⏭️", "timed_out": "⌛", "startup_failure": "❌"}

// SHOWN_STEPS = (stats_mod.MERGES_SHOWN, stats_mod.MERGES_SHOWN // 2, 10, 0)
var ShownSteps any // unported

// _local is dashboard._local.

func Local(ctx *hctx.Context, at time.Time) time.Time {
	panic("unported: dashboard._local")
}

// Text for a table cell: one line, no pipes, clipped.

func Cell(text any, limit int /* = 70 */) string {
	panic("unported: dashboard._cell")
}

// `▰▰▰▱▱▱▱▱▱▱ 30%`, or a dash with no reading.

func Bar(fraction *float64, cells int /* = BAR_CELLS */) string {
	panic("unported: dashboard.bar")
}

// What each lane is doing now. The start time links to the run doing it; hovering it shows
// how long the run had been going when the issue was written.

func RunningTable(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object, issues []*pyjson.Object) []string {
	panic("unported: dashboard.running_table")
}

// TIMELINE_STYLE = ('%%{init: {"themeVariables": {"taskBkgColor": "#ddf4ff", "taskBorderColor": ' '"#54aeff", "taskTextColor": "#0a3069", "taskTextOutsideColor":
var TimelineStyle string = `%%{init: {"themeVariables": {"taskBkgColor": "#ddf4ff", "taskBorderColor": "#54aeff", "taskTextColor": "#0a3069", "taskTextOutsideColor": "#6e7781"}}}%%`

// A Mermaid timeline of the runs going now: GitHub's runners, then the machine, the
// longest-running first, each bar named by its subscription, what it does and for how long;
// then, with the runners `found`, the jobs on the owner's PC (the learn lane's, gather's), each
// named by its workflow and job. (A gantt chart reads `#` as a comment, so items are named
// without it.)

func Timeline(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object, found *Found /* = None */) []string {
	panic("unported: dashboard.timeline")
}

// BOX_STYLES = ( " classDef busy fill:#1f883d,stroke:#116329,color:#ffffff", " classDef next fill:#0969da,stroke:#0550ae,color:#ffffff", " classDef idle fill:#ddf
var BoxStyles = []string{"    classDef busy fill:#1f883d,stroke:#116329,color:#ffffff", "    classDef next fill:#0969da,stroke:#0550ae,color:#ffffff", "    classDef idle fill:#ddf4ff,stroke:#54aeff,color:#0a3069", "    classDef paused fill:#fff8c5,stroke:#d4a72c,color:#4d2d00", "    classDef closed fill:#eaeef2,stroke:#8c959f,color:#24292f", "    classDef broken fill:#ffebe9,stroke:#cf222e,color:#82071e", "    classDef off fill:#f6f8fa,stroke:#d0d7de,color:#6e7781,stroke-dasharray:4 3"}

// The `classDef`s of `BOX_STYLES` and `extra` that `lines` use (`:::name`): a chart declaring
// the rest only spends the issue's room (`MAX_BODY`).

func Styles(lines []string, extra []string /* = () */) []string {
	panic("unported: dashboard.styles")
}

// STYLE = {"busy": "busy", "next": "next", "idle": "idle", "reviews": "paused", "full": "paused", "capped": "paused", "closed": "closed", "broken": "broken", "off
var Style = map[string]string{"busy": "busy", "next": "next", "idle": "idle", "reviews": "paused", "full": "paused", "capped": "paused", "closed": "closed", "broken": "broken", "off": "off"}

// LEGEND = ("🟢 working · ▶️ takes queued work at the next run · ⚪ idle: it may start work, and " "nothing queued is for it · ⏸️ at or near a usage cap (when it ca
var Legend string = "🟢 working · ▶️ takes queued work at the next run · ⚪ idle: it may start work, and nothing queued is for it · ⏸️ at or near a usage cap (when it can start a build or a plan again), or no lane free where it runs · 🌙 outside its hours · ⛔ backing off after a run that could not work (a login or the CLI) · ⚫ off or suspended. Usage is the last reading (5h: the 5-hour session, 7d: the week), against its cap now."

// A subscription with no run going, as its box's style and why it is not working, by usage
// and hours alone (`waiting.held_by`): `idle` when it may start a build, a revision or a plan.
// The queue and the lanes say more (`waiting.subscriptions`).

func Account(ctx *hctx.Context, state *pyjson.Object, provider any) (string, string) {
	panic("unported: dashboard._account")
}

// TRAINING_LANES: tuple[str, ...] = ()
var TrainingLanes = []string{}

// TRAINING_STALE = timedelta(hours=6)
var TrainingStale any // unported

// The training box's lanes as a subgraph, read from their pull requests: a draft is a Muse
// session at work (or an idle unban lane, which its title says), one ready for review is a
// promotion waiting on CI, and none is a lane between sessions or stopped. [] with no lane in
// `TRAINING_LANES`, or when the pull requests cannot be read.

func TrainingBoxes(ctx *hctx.Context) []string {
	panic("unported: dashboard.training_boxes")
}

// Every subscription as a box: GitHub's runners (the Claude accounts), then the night box's
// (each with the runs it has there), each saying what it is doing, or why it is not, and its
// usage.

func LanesBoxes(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object, now *pyjson.Object /* = None */) []string {
	panic("unported: dashboard.lanes_boxes")
}

// LANE_ORDER = ("agy", "gpt", "muse")
var LaneOrder = []string{"agy", "gpt", "muse"}

// LANES_MARK = "<!-- jackioh-bot:lanes "
var LanesMark string = "<!-- jackioh-bot:lanes "

// CLAUDE_SECRET = "CLAUDE_CODE_OAUTH_TOKEN"
var ClaudeSecret string = "CLAUDE_CODE_OAUTH_TOKEN"

// Each subscription's lanes, in `LANE_ORDER`: how many it has, how many of them its runs
// hold now (`plan.Lanes.used`, what the planner holds to them), whether it is switched off or
// may start a build or a plan now (`waiting.held_by`: its usage and hours; nothing while the
// bot is `halted`), its secret and where it runs. A fill-in's runs (`FillIn`) count in the
// lanes they fill, the first of its `for` first; a planning run the planning lane holds counts
// beside its lanes (`planning`).

func LaneRows(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object, halted string /* = "" */) []*pyjson.Object {
	panic("unported: dashboard.lane_rows")
}

// This bot's lanes for the night bot's table (`marked_lanes`), named for this bot.

func LanesMark_(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object, halted string /* = "" */) string {
	panic("unported: dashboard.lanes_mark")
}

// The other bots' lanes their sections carry (`lanes_mark`), in their order. A section with
// none, or with one that cannot be read, adds no row.

func MarkedLanes(parts []string) []*pyjson.Object {
	panic("unported: dashboard.marked_lanes")
}

// _open is dashboard._open.

func Open(row *pyjson.Object) int {
	panic("unported: dashboard._open")
}

// How many lanes each subscription has, how many its runs hold and how many are open: the
// night bot's, the Claude accounts first by number (`lane_rows`), then each other bot's from its
// section (`marked_lanes`), then the totals, the Claude accounts' and everyone's. Under them,
// the limits on the lanes: an account's own, the Claude secrets the workflows hand over, the
// runs GitHub's runners, the planning lane and the AWS boxes take at once, usage and hours, and
// how many more Claude runs there is room for now.

func LanesTable(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object, others []string /* = () */, halted string /* = "" */) []string {
	panic("unported: dashboard.lanes_table")
}

// What holds the lanes, as a list (`lanes_table`), each limit with how much of it is held.

func LaneLimits(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object, rows []*pyjson.Object) []string {
	panic("unported: dashboard.lane_limits")
}

// subscription_table is dashboard.subscription_table.

func SubscriptionTable(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object, now *pyjson.Object /* = None */) []string {
	panic("unported: dashboard.subscription_table")
}

// USAGE_CHART = ('%%{init: {"themeVariables": {"xyChart": {"plotColorPalette": ' '"#d0d7de, #1f883d"}}}}%%')
var UsageChart string = `%%{init: {"themeVariables": {"xyChart": {"plotColorPalette": "#d0d7de, #1f883d"}}}}%%`

// Each subscription's 5-hour and 7-day usage as bars over its cap now, for every one with a
// reading; the ones with none are named under the chart.

func UsageCharts(ctx *hctx.Context, state *pyjson.Object) []string {
	panic("unported: dashboard.usage_charts")
}

// What each difficulty may be built by, in one line above the queue.

func DifficultyLegend(pool any) string {
	panic("unported: dashboard.difficulty_legend")
}

// STEP = {"plan": "plan", "build": "build", "revise": "revise", "review": "review run"}
var Step = map[string]string{"plan": "plan", "build": "build", "revise": "revise", "review": "review run"}

// The queue in the planner's order: what the next run does to each item, its difficulty and
// the tier that needs, and what takes it next or what it waits for.

func QueueTable(ctx *hctx.Context, explained *waiting.Explained, issues []*pyjson.Object) []string {
	panic("unported: dashboard.queue_table")
}

// MODE_KIND = {"oneshot": "one-shot (fullsend)", "split": "split", "split-bot": "split for the night bot"}
var ModeKind = map[string]string{"oneshot": "one-shot (fullsend)", "split": "split", "split-bot": "split for the night bot"}

// The items waiting for a person (`bot:blocked`), in a line, or "".

func WaitingForPeople(issues []*pyjson.Object) string {
	panic("unported: dashboard.waiting_for_people")
}

// One line: how many runs go, what the queue waits on, and which subscriptions can start
// work now.

func Headline(ctx *hctx.Context, live *pyjson.Object, explained *waiting.Explained) string {
	panic("unported: dashboard.headline")
}

// The trees the bot split (#60): each parent, how many of its sub-issues have closed, and
// who builds them.

func TreesTable(ctx *hctx.Context, state *pyjson.Object) []string {
	panic("unported: dashboard.trees_table")
}

// runs_table is dashboard.runs_table.

func RunsTable(ctx *hctx.Context) []string {
	panic("unported: dashboard.runs_table")
}

// LEARN_WORKFLOW = "learn.yml"
var LearnWorkflow string = "learn.yml"

// LEARN_BRANCH = "ai/learn"
var LearnBranch string = "ai/learn"

// LEARN_SCHEDULE = ((6, 23), (18, 23))
var LearnSchedule any // unported

// LEARN_PR_STALE = timedelta(hours=48)
var LearnPRStale any // unported

// LEARN_ROWS = 5
var LearnRows int = 5

// When learn.yml's schedule next fires (GitHub fires schedules late; never early).

func NextLearnRun(now time.Time) time.Time {
	panic("unported: dashboard.next_learn_run")
}

// LEARN_NO_RUNNER = "⏸️ the PC's runner took none of it in 24 hours"
var LearnNoRunner string = "⏸️ the PC's runner took none of it in 24 hours"

// How a learn.yml run ended, from its jobs' and steps' conclusions (learn.yml's names).

func LearnOutcome(run *pyjson.Object, jobs []*pyjson.Object) string {
	panic("unported: dashboard.learn_outcome")
}

// The learn lane's section: the AI on main, the lane's pull request, the next scheduled run,
// the owner's PC it runs on (its heartbeat: whether it is working and on what, from the runners
// `found`, and how loaded it is, with and without its runners), and its last runs with how each
// ended. Each part says so when GitHub cannot be read. Without `found` it reads the heartbeat
// alone, which says a runner is in a job but not which.

func LearnLane(ctx *hctx.Context, found *Found /* = None */) []string {
	panic("unported: dashboard.learn_lane")
}

// RUNNERS_MINUTES = 2
var RunnersMinutes int = 2

// RUNNERS_START = "<!-- jackioh-bot:runners -->"
var RunnersStart string = "<!-- jackioh-bot:runners -->"

// RUNNERS_END = "<!-- /jackioh-bot:runners -->"
var RunnersEnd string = "<!-- /jackioh-bot:runners -->"

// EXPLAINED = "<!-- jackioh-bot:runners-explained -->"
var Explained string = "<!-- jackioh-bot:runners-explained -->"

// HOSTED_RUNNERS = 20
var HostedRunners int = 20

// RUNNER_RUNS = 40
var RunnerRuns int = 40

// RUNNER_ROWS = 25
var RunnerRows int = 25

// LOCAL_LABEL = "jackioh-selfplay-local"
var LocalLabel string = "jackioh-selfplay-local"

// GATHER_LABEL = "jackioh-selfplay-local-gather"
var GatherLabel string = "jackioh-selfplay-local-gather"

// WORKFLOWS = { "bot-cache.yml": "the night bot's Rust build cache, built from main for its jobs on GitHub's", "bot-commands.yml": "the night bot's ears: commands
var Workflows = map[string]string{"bot-cache.yml": "the night bot's Rust build cache, built from main for its jobs on GitHub's", "bot-commands.yml": "the night bot's ears: commands, labels, reviews, CI results; its sweep", "bot-night.yml": "the night bot: gate, plan, `work` (one item planned, built, checked, reviewed), deliver", "bot-status.yml": "this issue's loop: rewrites it, sweeps, plans and starts night runs, delivers them", "ci-duration.yml": "after CI, an issue for any job over seven minutes", "ci.yml": "CI: the five required checks on a pull request; everything on a push to main", "claude-wake.yml": "starts each Claude account's 5-hour session every 4 hours from 06:00 Central", "code-audit.yml": "every three days: the code audit's issue for the night bot", "db-size.yml": "twice a day: an issue once production's database nears its Supabase plan's cap", "deploy-watch.yml": "asks Render to deploy each push to main; an issue if it never arrives", "gather.yml": "each new patch's dev games on the owner's PC, filed as totals (GATHER_ON_PATCH)", "hygiene-delete.yml": "deletes the branches a person ticked in the hygiene issue", "hygiene.yml": "weekly: the hygiene issues for Squishy", "learn-set.yml": "a new set stops a learn export still early on the set before", "learn.yml": "the learn lane: self-play, retrain and gate of the learned evaluation, back to back", "lock-conversations.yml": "locks every conversation to collaborators", "patches-ship.yml": "ships the pending card-patch fragments after a merge", "pr-title.yml": "holds a pull request's title to the convention", "promote-production.yml": "merges main into production once a day", "selfplay-shards.yml": "one round of a self-play export's shards", "selfplay.yml": "a self-play export, on the owner's PC unless asked otherwise", "spec-fold.yml": "folds the pending §11 paragraphs and rewrites the rulings index once a day", "squishy-commands.yml": "Squishy's ears and its sweep", "squishy-run.yml": "Squishy: gate, plan, `work` on its own Claude account, deliver", "super.yml": "the daily super run: 10,000 fuzz seeds, the full gates, every e2e spec, the db", "sweep.yml": "sweeps the shadow bans again when the AI's generation moves", "title-retcon.yml": "the title retcon, by hand only", "triage.yml": "labels each new issue and pull request from fixed rules", "voice.yml": "renders a branch's voice lines (macOS `say`, Windows SAPI), checked on Linux", "winrates-refresh.yml": "daily: the Tech decks' win-rate table, once a patch"}

// HOSTED_ROLES = ( ("ubuntu-latest", "GitHub's Linux: a fresh machine each job, four vCPUs", "nearly every job: CI, every bot job that holds a write token (gate,
var HostedRoles any // unported

// LOCAL_ROLES = ( (LOCAL_LABEL, "the owner's PC: a container a job, `pc-selfplay-<time>`, only while the PC is " "otherwise idle", "`learn`, every job of it, back
var LocalRoles any // unported

// BOXES_NOTE = ("The night box and the Muse box (a runner name ending `-b2`) are the bot's two AWS " "machines, 2 vCPUs and 8 GB each; each starts when one of its
var BoxesNote string = "The night box and the Muse box (a runner name ending `-b2`) are the bot's two AWS machines, 2 vCPUs and 8 GB each; each starts when one of its jobs queues and stops after 30 idle minutes. A lane past a subscription's first is `-2`, `-3` …. A subscription logged in on the owner's PC may take its job there too (`pc-<id>-<time>`). The PC's supervisor sends a heartbeat every two minutes (its CPU and memory in use, with and without its runners, and each runner's CPUs and whether it is slowed), which the PC's group in the runners section and the learn lane show with whether it is working, the group with the gather workflow's state. bot/machine/README.md has the rest."

// The comment that says what each runner is for: GitHub's, each subscription's on the bot's
// machine (from the pool, so a subscription added there shows), and the owner's PC; then what
// each workflow is for.

func Explanation(ctx *hctx.Context) string {
	panic("unported: dashboard.explanation")
}

// Keep the status issue's comment on what each runner is for: written once, and again only
// when its text changes (the state file keeps its id and a digest), so a rewrite costs no call.
// `state` takes the record too, so the rewrite in hand links the comment.

func Explain(ctx *hctx.Context, number int, state *pyjson.Object) {
	panic("unported: dashboard.explain")
}

// PC_FILE = "pc.json"
var PCFile string = "pc.json"

// PC_QUIET = timedelta(minutes=10)
var PCQuiet any // unported

// GATHER_WORKFLOW = "gather.yml"
var GatherWorkflow string = "gather.yml"

// BOX_ROW = 6
var BoxRow int = 6

// BOX_STEPS = (12, 6, 0)
var BoxSteps = []int{12, 6, 0}

// RUNNER_LEGEND = ("🟢 in a job · ⏸️ in a job, slowed while the PC is busy · ⏳ queued for a runner · " "⚪ idle, waiting for a job · 💤 no job: idle, or its box is o
var RunnerLegend string = "🟢 in a job · ⏸️ in a job, slowed while the PC is busy · ⏳ queued for a runner · ⚪ idle, waiting for a job · 💤 no job: idle, or its box is off · ⚫ off · ❌ its last run failed. GitHub's runners exist only while they hold a job."

// What the runners section draws, read once: each busy job and each queued one with its run,
// how many runs were not read and whether the runs could be read; the gather workflow's last
// runs; and the owner's PC's heartbeat (None, with `pc_note` saying why, when there is none).

type Found struct {
	Busy   [][2]*pyjson.Object
	Queued [][2]*pyjson.Object
	Unread int              // default: 0
	Failed bool             // default: False
	Gather []*pyjson.Object // default: field(default_factory=list)
	PC     *pyjson.Object   // default: None
	PCNote string           // default: ""
}

// The jobs of the runs going and queued, from GitHub, since it lists a repository's own
// runners only to an admin, which the bot is not; the gather workflow's last runs; and the PC's
// heartbeat. Every read is a conditional request (`gh._CACHED`).

func ReadRunners(ctx *hctx.Context) *Found {
	panic("unported: dashboard.read_runners")
}

// Where a job runs, from its runner's name and its label, as (order, words).

func Where(job *pyjson.Object) (int, string) {
	panic("unported: dashboard._where")
}

// STEP_WORDS = {"Build, check and review": "build, check and review", "Work on another subscription (build, check and review)": "build, check and review"}
var StepWords = map[string]string{"Build, check and review": "build, check and review", "Work on another subscription (build, check and review)": "build, check and review"}

// The step a running job is on, or the last it finished.

func Step_(job *pyjson.Object, limit int /* = 45 */) string {
	panic("unported: dashboard._step")
}

// Text for a box: one line, clipped, with nothing Mermaid or HTML would read as markup.

func Text(value any, limit int /* = 34 */) string {
	panic("unported: dashboard._text")
}

// RUNNER_WRAP = '%%{init: {"flowchart": {"wrappingWidth": 280}}}%%'
var RunnerWrap string = `%%{init: {"flowchart": {"wrappingWidth": 280}}}%%`

// GHOST_STYLE = " classDef ghost fill:transparent,stroke:transparent,color:transparent"
var GhostStyle string = "  classDef ghost fill:transparent,stroke:transparent,color:transparent"

// A subgraph of boxes, each (its lines, its style), `BOX_ROW` to a row, each row a chain of
// invisible links; past one row, the last is padded with invisible boxes (`GHOST_STYLE`).

func Boxes(sid string, title string, boxes [][]any) []string {
	panic("unported: dashboard._boxes")
}

// LEARN_GAMES = 100_000
var LearnGames int = 100000

// PROGRESS_START = "<!-- jackioh-bot:progress -->"
var ProgressStart string = "<!-- jackioh-bot:progress -->"

// PROGRESS_END = "<!-- /jackioh-bot:progress -->"
var ProgressEnd string = "<!-- /jackioh-bot:progress -->"

// PROGRESS_CHART = ('%%{init: {"xyChart": {"height": 220}, "themeVariables": {"xyChart": ' '{"plotColorPalette": "#d0d7de, #1f883d"}}}}%%')
var ProgressChart string = `%%{init: {"xyChart": {"height": 220}, "themeVariables": {"xyChart": {"plotColorPalette": "#d0d7de, #1f883d"}}}}%%`

// The top of the issue, between `PROGRESS_START` and `PROGRESS_END`: how far the owner's
// PC's two local jobs have played, as a Mermaid bar chart in percent, with a line under it. The
// learn lane's export is the game it is at of `LEARN_GAMES`, then its shard's games; gather is
// its games of its patch. Each comes from the games its `jackioh` has written so far (the
// heartbeat's `progress`); a job playing none has a bar at 0 that says why. Without `found` it
// reads the heartbeat alone.

func Progress(ctx *hctx.Context, found *Found /* = None */) string {
	panic("unported: dashboard.progress")
}

// BOX_SLOTS = (("night box", ("gpt", "agy", "muse"), ()), ("Muse box", ("muse", "muse", "muse"), ()))
var BoxSlots any // unported

// The runners as boxes, as the subscriptions are drawn: GitHub's runners (at most `boxes`
// of them), the night box's and the Muse box's, the owner's PC with its heartbeat and the gather
// workflow, then the jobs queued for a runner, by label.

func RunnerDiagram(ctx *hctx.Context, state *pyjson.Object, found *Found, boxes int) []string {
	panic("unported: dashboard.runner_diagram")
}

// Every runner and what it is doing, as boxes (`runner_diagram`) and as a table: each job a
// runner holds now, the step it is on, what it is for and how long it has run, linked to the
// job; and a link to the comment on what each runner is for (`explain`). Each part folds
// away. Short of `room`, the table lists fewer runners, then the boxes show fewer of GitHub's.

func RunnersSection(ctx *hctx.Context, state *pyjson.Object, found *Found /* = None */, room int /* = MAX_BODY */) string {
	panic("unported: dashboard.runners_section")
}

// Redraw the status issue's runners section alone, between the loop's rewrites, and the
// PC's progress chart at its top (`progress`): the rest of the issue stays as the last rewrite
// left it, and the table lists what room it leaves.

func RefreshRunners(ctx *hctx.Context) string {
	panic("unported: dashboard.refresh_runners")
}

// What both bots' views are drawn from, read from GitHub once per rewrite: the state, the
// open threads, the live lanes, why nothing may start (halted) if so, the queue and the
// subscriptions explained, the last runs and the full status text. `learn` is the learn lane's
// section, and `runners` the runners' drawn from the runs `found`, read once per rewrite by
// `update` (None: `render` reads them).

type View struct {
	State     *pyjson.Object
	Issues    []*pyjson.Object
	Live      *pyjson.Object
	Halted    string
	Explained *waiting.Explained
	Runs      []string
	Report    string
	Learn     []string // default: None
	Runners   string   // default: None
	Found     *Found   // default: None
}

// look is dashboard.look.

func Look(ctx *hctx.Context) *View {
	panic("unported: dashboard.look")
}

// This bot's section of the shared status issue (#60): Squishy's, drawn by its own process
// and put into the night bot's issue apart from the night bot's own facts.

func Section(ctx *hctx.Context) string {
	panic("unported: dashboard.section")
}

// SECTION_START = "<!-- jackioh-bot:section -->"
var SectionStart string = "<!-- jackioh-bot:section -->"

// SECTION_END = "<!-- /jackioh-bot:section -->"
var SectionEnd string = "<!-- /jackioh-bot:section -->"

// `lines` in a dropdown under the heading before them, so a reader can fold the section
// away. It starts open: GitHub can draw a chart blank that sat in a closed one.

func Fold(summary string, lines []string) []string {
	panic("unported: dashboard.fold")
}

// The whole issue: the headline, the night bot's facts, its statistics, then each other
// bot's section (`extra`), then the full status folded (without `full`, a line saying where to
// read it instead: it is the same facts as the sections above, `fit`).

func Render(ctx *hctx.Context, extra []string /* = () */, statistics string /* = "" */, view *View /* = None */, full bool /* = True */) string {
	panic("unported: dashboard.render")
}

// FULL_LEFT_OUT = (f"The full status is left out to keep this issue under GitHub's limit: " f"`{SLASH} status` gives it.")
var FullLeftOut any // unported

// The body, while it would pass GitHub's limit on an issue's body: without the full status
// (the same facts as the sections above, and `/harness status` gives it), then listing fewer
// merged pull requests in the statistics (`draw(shown)`), then fewer busy runners (`_room`), and
// leaving the statistics out last of all.

func Fit(ctx *hctx.Context, view *View, extra []string, draw func(int) string) string {
	panic("unported: dashboard.fit")
}

// The open status issue: the one the state file names, or one to adopt.

func Find(ctx *hctx.Context, state *pyjson.Object) *pyjson.Object {
	panic("unported: dashboard.find")
}

// Rewrite the status issue, opening and pinning it first if there is none. `extra` are the
// other bots' sections (`section`), drawn by their own processes.
//
// With `stats`, the issue carries the statistics (`stats.block`): counted afresh when
// `STATS_EVERY` has passed (or `force_stats`, or the issue has none yet, or the kept block no
// longer fits), with `stats_extra` drawing the other bots' records then; otherwise the block
// the issue holds is kept as it is, so a rewrite costs a few calls, not a count's hundreds.
// Without `stats` (bot-commands.yml's rewrite after a sweep) nothing is counted, and the block
// the issue holds is kept too, while it fits. A count reads which set's cards win most from production through `read_cards` once a day
// (`cardsets.section`), and keeps the section the issue holds between reads.

func Update(ctx *hctx.Context, extra []string /* = () */, stats bool /* = False */, statsExtra func() []string /* = tuple */, forceStats bool /* = False */, readCards any /* = None */) string {
	panic("unported: dashboard.update")
}
