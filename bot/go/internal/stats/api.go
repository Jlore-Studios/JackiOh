// Package stats is bot/harness/stats.py.
//
// The statistics in the pinned status issue (#148): what the night bot has done over the last six
// hours, the last day, the last week and all time, counted afresh every 30 minutes by the
// `bot-status` loop (`dashboard --stats`) and kept in the issue between counts (`kept`).
//
// Where the rest of the issue says what the bot is doing now, this block keeps the record. It opens
// with one table that sets the four windows side by side and charts of the last two weeks, then
// folds each window into a section of its own: what each subscription did in it (plans, builds,
// revisions, reviews, pull requests opened and merged, lines merged, pauses, failures, model time,
// tokens and cost) with bar charts of runs and lines by subscription; then every pull request
// merged, newest first, with who planned, built, revised and approved it. The charts are bars and
// lines, never pies. Everything is read from GitHub and the state file each count, so it holds
// nothing of its own but when it last counted:
//
//   - **Runs and outcomes** come from the bot's own comments. A run's starting comment ("Starting work
//     on this now …, on `claude-1` (claude, `opus`, strong)") names its subscription and model, and
//     every later comment carrying the same run link is joined to it.
//   - **Who did what to a pull request**: its builder from the "Opened #n, built on …" comment on its
//     issue (for one opened before that comment named a builder, the last build started on its issue
//     before it was opened); its planner from "Planned on …" on its issue; its revisers from
//     "Revision pushed … on …" on it; its approvers from the seats a comment says approved it.
//   - **Pull requests, merges, commits and lines** come from the bot's pull requests; the closed
//     issues from their `Closes #n` lines; commits on the default branch from the commit list.
//   - **Model minutes** come from each subscription's `spent` record in the state file. Every entry is
//     dated, so a window sums the entries in it; all time is its last `providers.SPENT_KEEP` runs.
//   - **Model tokens and cost** come from each subscription's `tokens_spent` record beside it
//     (#695): per run its input, output and cached tokens and its USD cost, summed the same way.
//
// Squishy (#60) has a folded section of its own at the end, apart from the night bot's record: the
// same four windows from its own comments, pull requests, runs and state, and its modes (one-shot
// builds, splits, the sub-issues they opened, the trees closed). Squishy's own process draws it
// (`bot_section`), and the night bot's loop runs that process at each count and puts it in
// (`block`'s `extra`).
//
// This used to be an issue of its own, "Night bot statistics" (#190); it is now part of the status
// issue, so one pinned issue says what the bot is doing and what it has done.
package stats

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"time"
)

// BLOCK_START = "<!-- jackioh-bot:statistics -->"
var BlockStart string = "<!-- jackioh-bot:statistics -->"

// BLOCK_END = "<!-- /jackioh-bot:statistics -->"
var BlockEnd string = "<!-- /jackioh-bot:statistics -->"

// STATS_EVERY = timedelta(minutes=30)
var StatsEvery any // unported

// WINDOWS: tuple[tuple[str, timedelta | None], ...] = ( ("Last 6 hours", timedelta(hours=6)), ("Last 24 hours", timedelta(hours=24)), ("Last 7 days", timedelta(da
var Windows [][]any // unported

// MERGES_SHOWN = 40
var MergesShown int = 40

// TITLE_SHOWN = 70
var TitleShown int = 70

// DAYS_SHOWN = 14
var DaysShown int = 14

// LIMIT_COMMENTS = 5000
var LimitComments int = 5000

// UNRECORDED = "not recorded"
var Unrecorded string = "not recorded"

// EVER = "2000-01-01T00:00:00Z"
var Ever string = "2000-01-01T00:00:00Z"

// LIMIT_PULLS = 500
var LimitPulls int = 500

// LIMIT_RUNS = 1000
var LimitRuns int = 1000

// RUN_LINK = re.compile(r"actions/runs/(\d+)")
var RunLink any // unported

// SEAT = re.compile(r"`([a-z][a-z0-9-]*)` \(([a-z]+), `?([A-Za-z0-9][A-Za-z0-9.\-]*)`?")
var Seat any // unported

// APPROVED_BY = re.compile(r"`([a-z][a-z0-9-]*)` \([a-z]+, `?[A-Za-z0-9][A-Za-z0-9.\-]*`?" r"(?:, [a-z]+)?\),? (?:reviewed it adversarially and )?approved it")
var ApprovedBy any // unported

// OPENED = re.compile(r"\bOpened #(\d+)")
var Opened any // unported

// SPLIT_INTO = re.compile(r"\bSplit #\d+ into (\d+) sub-issue")
var SplitInto any // unported

// CLOSES = re.compile(r"(?i)\b(?:closes|fixes|resolves) #(\d+)")
var Closes any // unported

// BRANCH_ISSUE = re.compile(r"^" + re.escape(BRANCH_PREFIX) + r"issue-(\d+)$")
var BranchIssue any // unported

// OUTCOMES: tuple[tuple[str, tuple[str, ...]], ...] = ( ("start", ("Starting work on this now", "Starting a revision now", "Starting a review now", "Planning this
var Outcomes [][]any // unported

// ACTIONS = (("Starting work on this now", "build"), ("Starting a revision now", "revise"), ("Starting a review now", "review"), ("Planning this now", "plan"), ("
var Actions any // unported

// DONE = ("opened", "revised", "planned", "approved", "split", "closed tree")
var Done = []string{"opened", "revised", "planned", "approved", "split", "closed tree"}

// NOT_DONE = ("paused", "not approved", "failed", "infra")
var NotDone = []string{"paused", "not approved", "failed", "infra"}

// Event is stats.Event.

type Event struct {
	At        string
	Number    int
	Kind      string
	Run       string   // default: ""
	Provider  string   // default: ""
	Model     string   // default: ""
	Action    string   // default: ""
	PR        *int     // default: None
	Approvers []string // default: ()
	Count     int      // default: 0
}

// Everything the issue shows, gathered once.

type Facts struct {
	Events        []*Event         // default: field(default_factory=list)
	Pulls         []*pyjson.Object // default: field(default_factory=list)
	CommitsOnMain int              // default: 0
	Runs          [][2]string      // default: field(default_factory=list)
	Spent         *pyjson.Object   // default: field(default_factory=dict)
	Counted       *pyjson.Object   // default: field(default_factory=dict)
}

// classify is stats.classify.

func Classify(body string) string {
	panic("unported: stats.classify")
}

// One bot comment as an event: what happened, in which run, on which subscription.

func Parse(comment *pyjson.Object) *Event {
	panic("unported: stats.parse")
}

// Each event with its run's subscription and model when it names none itself.

func Joined(events []*Event) []*Event {
	panic("unported: stats.joined")
}

// collect is stats.collect.

func Collect(ctx *hctx.Context) *Facts {
	panic("unported: stats.collect")
}

// Who planned, built, revised and approved each of the bot's pull requests.

type Credits struct {
	Built    *pyjson.Object // default: field(default_factory=dict)
	Planned  *pyjson.Object // default: field(default_factory=dict)
	Revised  *pyjson.Object // default: field(default_factory=dict)
	Approved *pyjson.Object // default: field(default_factory=dict)
}

func (x *Credits) Builder(pull *pyjson.Object) string {
	panic("unported: stats.Credits.builder")
}

// The issue a pull request builds: its branch `bot/issue-<n>`, or its first `Closes #n`.

func IssueOf(pull *pyjson.Object) *int {
	panic("unported: stats.issue_of")
}

// credits is stats.credits.

func Credits_(events []*Event, pulls []*pyjson.Object) *Credits {
	panic("unported: stats.credits")
}

// Everything that happened from `start` to now (`start` None: all time).

type Window struct {
	Name    string
	Start   time.Time
	Events  []*Event
	Opened  []*pyjson.Object
	Merged  []*pyjson.Object
	Runs    [][2]string
	Minutes *pyjson.Object
	Tokens  *pyjson.Object
	Cost    *pyjson.Object
}

func (x *Window) Starts() []*Event {
	panic("unported: stats.Window.starts")
}

func (x *Window) Outcomes() any {
	panic("unported: stats.Window.outcomes")
}

// window is stats.window.

func Window_(name string, span *time.Duration, now time.Time, facts *Facts) *clock.Window {
	panic("unported: stats.window")
}

// _local is stats._local.

func Local(ctx *hctx.Context, at time.Time) time.Time {
	panic("unported: stats._local")
}

// _pct is stats._pct.

func Pct(part float64, whole float64) string {
	panic("unported: stats._pct")
}

// One row per measure, one column per window.

func Summary(windows []*clock.Window) []string {
	panic("unported: stats.summary")
}

// by_subscription is stats.by_subscription.

func BySubscription(w *clock.Window, who *Credits) *pyjson.Object {
	panic("unported: stats.by_subscription")
}

// One window, folded: what each subscription did in it, its tokens and cost, and bar charts of
// runs and lines by subscription (all time adds model hours and what runs ended in).

func Section(ctx *hctx.Context, w *clock.Window, who *Credits, facts *Facts) []string {
	panic("unported: stats.section")
}

// The last two weeks day by day: runs started, pull requests merged and lines added.

func OverTime(ctx *hctx.Context, w *clock.Window) []string {
	panic("unported: stats.over_time")
}

// Every pull request merged, newest first, with who planned, built, revised and approved it,
// folded.

func MergedTable(ctx *hctx.Context, merged []*pyjson.Object, who *Credits, shown int) []string {
	panic("unported: stats.merged_table")
}

// Squishy's modes (#60), one row each, one column per window.

func Modes(windows []*clock.Window) []string {
	panic("unported: stats.modes")
}

// SECTION_START = "<!-- jackioh-bot:statistics-section -->"
var SectionStart string = "<!-- jackioh-bot:statistics-section -->"

// SECTION_END = "<!-- /jackioh-bot:statistics-section -->"
var SectionEnd string = "<!-- /jackioh-bot:statistics-section -->"

// SECTION_MERGES = 10
var SectionMerges int = 10

// This bot's section of the night bot's statistics (#60): Squishy's record, drawn by its own
// process from its own comments, pull requests, runs and state, and folded in the status issue.

func BotSection(ctx *hctx.Context) string {
	panic("unported: stats.bot_section")
}

// The statistics block of the status issue, between its markers: the four windows side by
// side, the last two weeks, which set's cards win most (`cards`, `cardsets.section`), each
// window folded, the merged pull requests folded, and the other bots' sections (`extra`,
// `bot_section`) folded at the end.

func Block(ctx *hctx.Context, facts *Facts, shown int /* = MERGES_SHOWN */, extra []string /* = () */, cards string /* = "" */) string {
	panic("unported: stats.block")
}

// The statistics block a rewrite of the status issue keeps between counts, or None.

func Kept(body string) string {
	panic("unported: stats.kept")
}

// due is stats.due.

func Due(ctx *hctx.Context, state *pyjson.Object) bool {
	panic("unported: stats.due")
}

// Note in the state file that the statistics were counted now.

func Counted(ctx *hctx.Context) {
	panic("unported: stats.counted")
}
