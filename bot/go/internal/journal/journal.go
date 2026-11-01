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
	"fmt"
	"math/rand/v2"
	"regexp"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
)

// Branch is BRANCH = IDENTITY.journal_branch.
var Branch = config.Identity.JournalBranch

// JournalFile is where the work job puts the item's journal in its worktree: beside the notes,
// ignored by git.
var JournalFile string = ".bot-journal.md"

// RunMarker: each run's section starts with this line, so the file can be cut at a run.
var RunMarker string = "<!-- jackioh-bot:run -->"

// ReadChars is how much of the journal the plan carries to the work job: its end, from a run's
// start.
var ReadChars int = 100000

// FileChars: a file over this drops its oldest runs' notes, then its oldest runs, until it fits
// (the Contents API reads files up to 1 MB). The branch's history keeps every version.
var FileChars int = 300000

// NotesChars is how much of a run's notes its section keeps.
var NotesChars int = 20000

// MaxSteps is the steps of a run its section lists, and CellChars the characters of one cell.
var MaxSteps int = 200

// CellChars is CELL_CHARS.
var CellChars int = 300

// MaxAttempts is MAX_ATTEMPTS.
var MaxAttempts int = 6

// Cut is what `compact` leaves where it cut the log.
var Cut string = "(earlier lines of the log were cut here)\n"

// logHeading is _LOG, `^##\s+Log\b.*$` (multiline). RE2's `\b` is ASCII only, so Python's
// Unicode boundary after "Log" is written out: the line ends there, or goes on with a character
// that is not a word character.
var logHeading = regexp.MustCompile(`(?m)^##` + py.ReS + `+Log(?:[^` + py.WordClass + `\n].*)?$`)

var details = regexp.MustCompile(`(?s)\n<details><summary>The (?:builder's notes|planner's draft).*?</details>\n`)

// PathFor is journal.path_for.
func PathFor(number int) string {
	return strconv.Itoa(number) + ".md"
}

// Key is the item a run's journal belongs to: the issue a pull request closes, else the thread.
func Key(plan *pyjson.Object) int {
	v := plan.Get("issue_number")
	if !pyjson.Truthy(v) {
		v = plan.Get("number")
	}
	if !pyjson.Truthy(v) {
		v = 0
	}
	n, err := py.Int(v)
	if err != nil {
		return 0
	}
	return n
}

// Compact is `notes` cut to `limit` characters, keeping what the next agent needs: everything
// before `## Log` (the plan the harness seeded and the State the builder rewrites) whole, or its
// first half of `limit` when it alone is longer, then as much of the end of the log as fits.
// Notes with no `## Log` heading keep their end, as before.
func Compact(notes string, limit int) string {
	if py.Len(notes) <= limit {
		return notes
	}
	found := logHeading.FindStringIndex(notes)
	if found == nil {
		return py.SliceFrom(notes, -limit)
	}
	state := py.Rstrip(notes[:found[0]]) + "\n\n"
	if py.Len(state) > py.FloorDiv(limit, 2) {
		state = py.Rstrip(py.SliceTo(state, py.FloorDiv(limit, 2))) + "\n(the rest of the state was cut here)\n\n"
	}
	heading := notes[found[0]:found[1]] + "\n"
	room := max(0, limit-py.Len(state)-py.Len(heading)-py.Len(Cut))
	log := strings.TrimLeft(notes[found[1]:], "\n")
	tail := ""
	if room > 0 {
		tail = py.SliceFrom(log, -room)
	}
	return state + heading + Cut + tail
}

// Cell is journal._cell.
func Cell(text any) string {
	s := ""
	if text != nil {
		s = py.Str(text)
	}
	flat := strings.ReplaceAll(strings.Join(strings.Fields(s), " "), "|", "\\|")
	if py.Len(flat) <= CellChars {
		return flat
	}
	return py.SliceTo(flat, CellChars-1) + "…"
}

// Clock is journal._clock.
func Clock(at any) string {
	text := ""
	if pyjson.Truthy(at) {
		text = py.Str(at)
	}
	if py.Len(text) >= 16 {
		return py.Slice(text, 11, 16)
	}
	return text
}

// tokenTotal is sum(int(tokens.get(name) or 0) for name in ("input", "output")).
func tokenTotal(tokens *pyjson.Object) int {
	total := 0
	for _, name := range []string{"input", "output"} {
		total += tokenCount(tokens, name)
	}
	return total
}

// tokenCount is int(tokens.get(name) or 0).
func tokenCount(tokens *pyjson.Object, name string) int {
	v := tokens.Get(name)
	if !pyjson.Truthy(v) {
		return 0
	}
	n, err := py.Int(v)
	if err != nil {
		panic(err)
	}
	return n
}

// Took is a step's cost in one cell: its minutes, then its tokens and USD cost when the call
// reported any (#695).
func Took(step *pyjson.Object) string {
	took := step.Get("minutes")
	text := ""
	if _, isNumber := py.Number(took); isNumber {
		text = py.Str(took) + " min"
	}
	tokens, _ := step.Get("tokens").(*pyjson.Object)
	if total := tokenTotal(tokens); total > 0 {
		if text != "" {
			text = fmt.Sprintf("%s, %d tok", text, total)
		} else {
			text = fmt.Sprintf("%d tok", total)
		}
	}
	if cost, isNumber := py.Number(step.Get("cost_usd")); isNumber && cost > 0 {
		if text != "" {
			text = fmt.Sprintf("%s, $%.2f", text, cost)
		} else {
			text = fmt.Sprintf("$%.2f", cost)
		}
	}
	return text
}

// StepRows is one table row per step the work job recorded (`work.Worker._step`). A step that is
// not an object is skipped, and still counted.
func StepRows(steps []any) []string {
	rows := []string{}
	for i, raw := range steps[:min(len(steps), MaxSteps)] {
		n := i + 1
		step, ok := raw.(*pyjson.Object)
		if !ok {
			continue
		}
		what := "?"
		if step.Truthy("step") {
			what = py.Str(step.Get("step"))
		}
		if what == "call" {
			model := ""
			if step.Truthy("model") {
				model = " (`" + py.Str(step.Get("model")) + "`)"
			}
			role := "call"
			if step.Truthy("role") {
				role = py.Str(step.Get("role"))
			}
			what = role + model
		}
		rows = append(rows, fmt.Sprintf("| %d | %s | %s | %s | %s | %s |", n, Clock(step.Get("at")), Cell(what),
			Cell(step.Get("why")), Took(step), Cell(step.Get("outcome"))))
	}
	if len(steps) > MaxSteps {
		rows = append(rows, fmt.Sprintf("| … | | %d more steps | | | |", len(steps)-MaxSteps))
	}
	return rows
}

// fence is the notes or draft in a Markdown fence that a ```` inside cannot close.
func fence(summary, text string) []string {
	return []string{"", "<details><summary>" + summary + "</summary>", "",
		"````markdown", strings.ReplaceAll(text, "````", "``​``"), "````", "", "</details>"}
}

// Section is the journal's section for one run, from the model job's result.
func Section(result *pyjson.Object, provider string, models string, action string, number int, at string, runURL string /* = "" */) string {
	status := py.OrStr(result.Get("status"), "unknown")
	if s, ok := result.Get("interrupt").(string); ok && s == "died" {
		status = "died"
	}
	reason := strings.TrimSpace(redact.Redact(py.OrStr(result.Get("reason"), ""), nil))
	link := "a run"
	if runURL != "" {
		link = "[run](" + runURL + ")"
	}
	ended := "."
	if reason != "" {
		ended = ": " + Cell(reason)
	}
	lines := []string{RunMarker, fmt.Sprintf("## %s UTC · %s #%d · `%s` · %s",
		strings.ReplaceAll(py.SliceTo(at, 16), "T", " "), action, number, provider, status), "",
		fmt.Sprintf("%s on `%s` (%s). It ended **%s**", link, provider, models, status) + ended}
	minutes, calls := result.Get("minutes"), result.Get("model_calls")
	if minutes != nil || calls != nil {
		lines = append(lines, fmt.Sprintf("Model calls: %s, %s model minutes.", py.Str(py.Or(calls, 0)), py.Str(py.Or(minutes, 0))))
	}
	tokens, _ := result.Get("tokens").(*pyjson.Object)
	total := tokenTotal(tokens)
	cost, isNumber := py.Number(result.Get("cost_usd"))
	if total > 0 || (isNumber && cost > 0) {
		cached := tokenCount(tokens, "cache_read") + tokenCount(tokens, "cache_write")
		line := fmt.Sprintf("Tokens: %d in / %d out / %d cached", tokenCount(tokens, "input"),
			tokenCount(tokens, "output"), cached)
		if isNumber {
			line += fmt.Sprintf(", $%.2f", cost)
		}
		lines = append(lines, line+".")
	}
	if result.Truthy("head") {
		line := "Its work ended at `" + py.SliceTo(py.Str(result.Get("head")), 12) + "`"
		if result.Truthy("branch") {
			line += " on `" + py.Str(result.Get("branch")) + "`."
		} else {
			line += "."
		}
		lines = append(lines, line)
	}
	if steps, _ := result.Get("steps").([]any); len(steps) > 0 {
		lines = append(lines, "", "| # | When (UTC) | Step | Why | Took | What came of it |",
			"|---|---|---|---|---|---|")
		lines = append(lines, StepRows(steps)...)
	}
	handoff, _ := result.Get("handoff").(*pyjson.Object)
	notes := strings.TrimSpace(redact.Redact(py.OrStr(handoff.Get("notes"), ""), nil))
	if notes != "" {
		notes = Compact(notes, NotesChars)
		lines = append(lines, fence("The builder's notes when it stopped", notes)...)
	}
	draft := py.SliceTo(strings.TrimSpace(redact.Redact(py.OrStr(handoff.Get("draft"), ""), nil)), NotesChars)
	if draft != "" {
		lines = append(lines, fence("The planner's draft when it stopped", draft)...)
	}
	return py.Rstrip(strings.Join(lines, "\n")) + "\n"
}

// Header is journal.header.
func Header(number int) string {
	return fmt.Sprintf("# Journal of #%d\n\n", number) +
		fmt.Sprintf("Every night-bot run on #%d, and on the pull requests that close it, adds a ", number) +
		"section here when it ends, oldest first (`bot/harness/journal.py`). The next run " +
		fmt.Sprintf("reads the end of this file, as `%s` in its worktree, before it starts.\n", JournalFile)
}

// Split is journal._split.
func Split(text string) (string, []string) {
	parts := strings.Split(text, "\n"+RunMarker+"\n")
	runs := []string{}
	for _, part := range parts[1:] {
		runs = append(runs, RunMarker+"\n"+part)
	}
	return parts[0], runs
}

// joinedLen is len("\n".join([head, *runs])).
func joinedLen(head string, runs []string) int {
	n := py.Len(head)
	for _, run := range runs {
		n += 1 + py.Len(run)
	}
	return n
}

// Trim is the file cut to `limit`: the oldest runs lose their notes first, then whole runs go.
func Trim(text string, limit int /* = FILE_CHARS */) string {
	if py.Len(text) <= limit {
		return text
	}
	head, runs := Split(text)
	i := 0
	for joinedLen(head, runs) > limit && i < len(runs) {
		runs[i] = details.ReplaceAllLiteralString(runs[i], "\n")
		i++
	}
	dropped := 0
	for joinedLen(head, runs) > limit && len(runs) > 1 {
		runs = runs[1:]
		dropped++
	}
	if dropped > 0 {
		head = py.Rstrip(head) + fmt.Sprintf("\n\n_%d older run(s) dropped from this file; the ", dropped) +
			"branch's history keeps them._\n"
	}
	return strings.Join(append([]string{head}, runs...), "\n")
}

// pySleep is time.sleep(seconds).
func pySleep(seconds float64) { time.Sleep(time.Duration(seconds * float64(time.Second))) }

// Append adds one run's section to the item's file, creating the branch and the file as needed.
// Raises GitHubError when GitHub keeps refusing; the caller logs it. A nil sleep is time.sleep.
func Append(gh gh.Client, number int, text string, sleep func(float64) /* = time.sleep */) {
	if sleep == nil {
		sleep = pySleep
	}
	path := PathFor(number)
	for range MaxAttempts {
		old, sha := gh.GetFile(path, Branch)
		start := Header(number) + "\n"
		if old != "" {
			start = py.Rstrip(old) + "\n\n"
		}
		updated := Trim(start+text, FileChars)
		// `old is None`: the Go client gives ("", "") for (None, None), and a file that is there
		// always has a blob sha.
		if old == "" && sha == "" && gh.BranchSHA(Branch) == "" {
			created := false
			if exc, ok := herr.Catch[*herr.GitHubError](func() {
				gh.CreateOrphanBranch(Branch, path, updated, fmt.Sprintf("journal #%d: start [skip ci]", number),
					config.NoDeploy)
				created = true
			}); ok && exc.Status != 422 { // 422: another run made the branch first
				herr.Raise(exc)
			}
			if created {
				return
			}
			continue
		}
		written := false
		if exc, ok := herr.Catch[*herr.GitHubError](func() {
			gh.PutFile(path, updated, Branch, sha, fmt.Sprintf("journal #%d [skip ci]", number))
			written = true
		}); ok && exc.Status != 409 && exc.Status != 422 {
			herr.Raise(exc)
		}
		if written {
			return
		}
		sleep(uniform(0.05, 0.3)) // another run wrote in between: read it again
	}
	herr.Raise(&herr.GitHubError{Msg: fmt.Sprintf("%s on %s kept changing; gave up after %d attempts",
		path, Branch, MaxAttempts), Status: 409})
}

// uniform is random.uniform(a, b).
func uniform(a, b float64) float64 { return a + (b-a)*rand.Float64() }

// Read is the end of an item's journal, from the start of a run, or "" (none yet, or unreadable).
func Read(gh gh.Client, number int, limit int /* = READ_CHARS */) string {
	text := ""
	if err := herr.Try(func() { text, _ = gh.GetFile(PathFor(number), Branch) }); err != nil {
		return "" // a journal that cannot be read never stops a claim
	}
	if text == "" {
		return ""
	}
	if py.Len(text) <= limit {
		return text
	}
	tail := py.SliceFrom(text, -limit)
	if at := strings.Index(tail, RunMarker); at >= 0 {
		tail = tail[at:]
	}
	return Header(number) + "\n_(earlier runs are on the branch, not shown here)_\n\n" + tail
}

// RunsIn is one line per run in a journal: its section's heading.
func RunsIn(text string) []string {
	found := []string{}
	for _, part := range strings.Split(text, RunMarker)[1:] {
		for _, line := range py.Splitlines(part) {
			if strings.HasPrefix(line, "## ") {
				found = append(found, strings.TrimSpace(line[3:]))
				break
			}
		}
	}
	return found
}

// URL is journal.url.
func URL(serverURL string, repo string, number int) string {
	return fmt.Sprintf("%s/%s/blob/%s/%s", serverURL, repo, Branch, PathFor(number))
}
