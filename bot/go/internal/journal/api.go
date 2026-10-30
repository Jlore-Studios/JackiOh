// Package journal is bot/harness/journal.py.
//
// An item's journal: every run on it, in order, on the unprotected `bot-journal` branch (#342).
//
// A run that does not finish its item hands the next one its notes and the end of its session
// (`work.Worker._handoff`), but that handoff is one slot in the state file: the next run overwrites
// it, a finished item drops it, and nobody can read it from the pull request. The journal keeps
// them all. Deliver appends one section per run to `<issue>.md`, whatever the run did and however it
// ended: what it was, a line per step the harness took (each model call, why it ran and what came of
// it; each check run; the merges of `main`), the builder's notes as they stood at the end, and a
// cut-off planner's draft (`work.PLAN_DRAFT_FILE`), so a plan's progress is never lost either. A pull
// request's runs go to the file of the issue it closes, so one file holds the build and every
// revision and review after it.
//
// The plan job reads the end of the file into the plan (`read`), and the work job puts it in the
// worktree as `JOURNAL_FILE`, which git ignores, so the next agent, on any subscription and any
// CLI, can read every run before it. The model job needs no GitHub credential for that, which is
// how it works on the bot's machine too. The builder's own notes come in two parts (`work.NOTES_ASK`):
// a `## State` it rewrites in place, which is the compressed history, and a `## Log` it adds to;
// `compact` keeps the first whole and the end of the second when the notes must be cut.
//
// Writes go through the Contents API with the blob sha as a compare-and-set, as the state file's do
// (`state.py`). The journal is a record, never a gate: a failed read or write is logged, and the
// run's delivery goes on.
package journal

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// BRANCH = IDENTITY.journal_branch
var Branch any // unported

// JOURNAL_FILE = ".bot-journal.md"
var JournalFile string = ".bot-journal.md"

// RUN_MARKER = "<!-- jackioh-bot:run -->"
var RunMarker string = "<!-- jackioh-bot:run -->"

// READ_CHARS = 100_000
var ReadChars int = 100000

// FILE_CHARS = 300_000
var FileChars int = 300000

// NOTES_CHARS = 20_000
var NotesChars int = 20000

// MAX_STEPS = 200
var MaxSteps int = 200

// CELL_CHARS = 300
var CellChars int = 300

// MAX_ATTEMPTS = 6
var MaxAttempts int = 6

// CUT = "(earlier lines of the log were cut here)\n"
var Cut string = `(earlier lines of the log were cut here)
`

// path_for is journal.path_for.

func PathFor(number int) string {
	panic("unported: journal.path_for")
}

// The item a run's journal belongs to: the issue a pull request closes, else the thread.

func Key(plan *pyjson.Object) int {
	panic("unported: journal.key")
}

// `notes` cut to `limit` characters, keeping what the next agent needs: everything before
// `## Log` (the plan the harness seeded and the State the builder rewrites) whole, or its first
// half of `limit` when it alone is longer, then as much of the end of the log as fits. Notes
// with no `## Log` heading keep their end, as before.

func Compact(notes string, limit int) string {
	panic("unported: journal.compact")
}

// _cell is journal._cell.

func Cell(text any) string {
	panic("unported: journal._cell")
}

// _clock is journal._clock.

func Clock(at any) string {
	panic("unported: journal._clock")
}

// A step's cost in one cell: its minutes, then its tokens and USD cost when the call
// reported any (#695).

func Took(step *pyjson.Object) string {
	panic("unported: journal._took")
}

// One table row per step the work job recorded (`work.Worker._step`).

func StepRows(steps []*pyjson.Object) []string {
	panic("unported: journal.step_rows")
}

// The journal's section for one run, from the model job's result.

func Section(result *pyjson.Object, provider string, models string, action string, number int, at string, runURL string /* = "" */) string {
	panic("unported: journal.section")
}

// header is journal.header.

func Header(number int) string {
	panic("unported: journal.header")
}

// _split is journal._split.

func Split(text string) (string, []string) {
	panic("unported: journal._split")
}

// The file cut to `limit`: the oldest runs lose their notes first, then whole runs go.

func Trim(text string, limit int /* = FILE_CHARS */) string {
	panic("unported: journal.trim")
}

// Add one run's section to the item's file, creating the branch and the file as needed.
// Raises GitHubError when GitHub keeps refusing; the caller logs it.

func Append(gh gh.Client, number int, text string, sleep func(float64) /* = time.sleep */) {
	panic("unported: journal.append")
}

// The end of an item's journal, from the start of a run, or "" (none yet, or unreadable).

func Read(gh gh.Client, number int, limit int /* = READ_CHARS */) string {
	panic("unported: journal.read")
}

// One line per run in a journal: its section's heading.

func RunsIn(text string) []string {
	panic("unported: journal.runs_in")
}

// url is journal.url.

func URL(serverURL string, repo string, number int) string {
	panic("unported: journal.url")
}
