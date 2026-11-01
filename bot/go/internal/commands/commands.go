// Package commands is bot/harness/commands.py.
//
// Comment commands: `/harness <verb> [args]`, `/harness-<verb>` and `@<bot> <verb or request>`.
//
// The slash is the running bot's (`config.SLASH`): `/harness` for the night bot, `/squishy` for
// Squishy (#60), whose modes add `oneshot` and `split`. Both have `fullsend` (#505).
//
// A command is a line of its own. Lines inside fenced code blocks and quoted lines (`> ...`) are
// never commands, so quoting the bot back at it cannot re-run anything. At most `MAX_COMMANDS` are
// read from one comment, top to bottom.
//
// Both prefixes read the rest of the line the same way: a verb (or an alias) runs; a single word
// that is a near miss of a verb runs nothing and gets a "did you mean"; anything else is a request:
// on an issue it asks for a build, on a pull request for a revision, with those words as the notes.
// The one difference is plain English: after `@<bot>`, a control verb followed by words that do not
// fit it is a request ("@bot start with option A"), unless a colon marks the words as the verb's
// own ("@bot halt: away this week").
package commands

import (
	"fmt"
	"slices"
	"strings"
	"unicode"
	"unicode/utf8"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/trust"
)

// VERBS
var Verbs = []string{"build", "revise", "review", "rebuild", "stop", "status", "help", "halt",
	"start", "suggest", "run", "suspend", "oneshot", "split", "fullsend"}

// ALIASES, in order (`misspelt` and `help <verb>` read it in order).
var Aliases = pyjson.New(
	"work", "build",
	"fix", "revise",
	"update", "revise",
	"resume", "start",
	"unhalt", "start",
	"go", "run",
	"suggestions", "suggest",
)

// LEVELS: the level each verb needs. `request` is naming the bot with free text; `typo` is a near
// miss of a verb, which only gets a reply; `approve` and `reject` are a vote on a suggestion
// (`VOTES`).
var Levels = pyjson.New(
	"status", 1,
	"help", 1,
	"typo", 1,
	"build", 2,
	"oneshot", 2,
	"split", 2,
	"fullsend", 2,
	"revise", 2,
	"review", 2,
	"rebuild", 2,
	"stop", 2,
	"suggest", 2,
	"request", 2,
	"approve", 2,
	"reject", 2,
	"halt", 3,
	"start", 3,
	"run", 3,
	"suspend", 3,
)

// FORCE_FLAG: `--force` lifts the subscriptions' hours and nothing else (their usage caps hold).
// Operator only.
var ForceFlag string = "--force"

// FORCE_LEVEL
var ForceLevel int = 3

// MAX_COMMANDS
var MaxCommands int = 10

// _FENCE = ^\s*(```|~~~)
func isFence(line string) bool {
	text := py.Lstrip(line)
	return strings.HasPrefix(text, "```") || strings.HasPrefix(text, "~~~")
}

// matchSlash is `_SLASH.match(line)` and its group 1:
// `^\s*(?:@[\w-]+\s+)*` + SLASH + `(?:-|[ \t]+|$)(.*)$`, case-insensitive.
func matchSlash(line string) (string, bool) {
	rs := []rune(line)
	i := 0
	for i < len(rs) && py.IsSpace(rs[i]) {
		i++
	}
	// The `(?:@[\w-]+\s+)*` repeats as often as it can, and gives back one at a time.
	starts := []int{i}
	for i < len(rs) && rs[i] == '@' {
		j := i + 1
		for j < len(rs) && (py.IsWord(rs[j]) || rs[j] == '-') {
			j++
		}
		if j == i+1 {
			break
		}
		k := j
		for k < len(rs) && py.IsSpace(rs[k]) {
			k++
		}
		if k == j {
			break
		}
		i = k
		starts = append(starts, i)
	}
	slash := config.Slash
	for n := len(starts) - 1; n >= 0; n-- {
		end, ok := py.FoldAt(rs, starts[n], slash)
		if !ok {
			continue
		}
		switch {
		case end < len(rs) && rs[end] == '-':
			end++
		case end < len(rs) && (rs[end] == ' ' || rs[end] == '\t'):
			for end < len(rs) && (rs[end] == ' ' || rs[end] == '\t') {
				end++
			}
		case end == len(rs):
		default:
			continue
		}
		return string(rs[end:]), true
	}
	return "", false
}

// issueRef is `_ISSUE_REF = ^#\d+$` for a word with no whitespace in it.
func issueRef(word string) bool {
	rest, ok := strings.CutPrefix(word, "#")
	if !ok || rest == "" {
		return false
	}
	for _, r := range rest {
		if !unicode.IsDigit(r) {
			return false
		}
	}
	return true
}

// Command is commands.Command.
type Command struct {
	Verb  string
	Args  string
	Force bool
	Line  string
	// For a `typo`: the verb or alias the word was probably meant to be.
	Meant string // default: ""
}

// Level is Command.level.
func (x *Command) Level() int {
	need := 3
	if v, ok := Levels.Lookup(x.Verb); ok {
		need, _ = pyjson.AsInt(v)
	}
	if x.Force {
		return max(need, ForceLevel)
	}
	return need
}

func stripForce(args string) (string, bool) {
	tokens := py.Split(args)
	force := false
	kept := []string{}
	for _, t := range tokens {
		if py.Lower(t) == ForceFlag {
			force = true
		} else {
			kept = append(kept, t)
		}
	}
	return py.Strip(strings.Join(kept, " ")), force
}

func canonical(word string) (string, bool) {
	word = strings.Trim(py.Lower(word), ":,.!?")
	if alias, ok := Aliases.Lookup(word); ok {
		word = alias.(string)
	}
	if slices.Contains(Verbs, word) {
		return word, true
	}
	return "", false
}

// distance is the edits from `a` to `b`, a swap of two neighbouring letters counting as one.
func distance(a, b string) int {
	ra, rb := []rune(a), []rune(b)
	first := make([]int, len(rb)+1)
	for j := range first {
		first[j] = j
	}
	rows := [][]int{first}
	for i := 1; i <= len(ra); i++ {
		row := make([]int, len(rb)+1)
		row[0] = i
		last := rows[len(rows)-1]
		for j := 1; j <= len(rb); j++ {
			cost := 1
			if ra[i-1] == rb[j-1] {
				cost = 0
			}
			row[j] = min(last[j]+1, row[j-1]+1, last[j-1]+cost)
			if i > 1 && j > 1 && ra[i-1] == rb[j-2] && ra[i-2] == rb[j-1] {
				row[j] = min(row[j], rows[len(rows)-2][j-2]+1)
			}
		}
		rows = append(rows, row)
	}
	last := rows[len(rows)-1]
	return last[len(last)-1]
}

// The verb or alias a request was probably meant to be, or None ("").
//
// Only a request of one word (an `#n` aside) counts: "stauts" is a typo, but "do the thing"
// is plain words even though "do" is one letter from "go". A long word may be two edits off,
// a short one only one.
func Misspelt(args string) string {
	var words []string
	for _, w := range py.Split(args) {
		if !issueRef(w) {
			words = append(words, w)
		}
	}
	if len(words) != 1 {
		return ""
	}
	word := strings.Trim(py.Lower(words[0]), ":,.!?")
	if py.Len(word) < 2 {
		return ""
	}
	if _, ok := canonical(word); ok {
		return ""
	}
	limit := 2
	if py.Len(word) <= 4 {
		limit = 1
	}
	best, bestName := -1, ""
	for _, name := range append(slices.Clone(Verbs), Aliases.Keys()...) {
		gap := distance(word, name)
		if gap <= limit && (best < 0 || gap < best) {
			best, bestName = gap, name
		}
	}
	return bestName
}

// The lines of `body` that may carry a command: outside fences, not quoted.
func CommandLines(body string) []string {
	lines := []string{}
	fenced := false
	for _, line := range py.Splitlines(body) {
		if isFence(line) {
			fenced = !fenced
			continue
		}
		if fenced || strings.HasPrefix(py.Lstrip(line), ">") {
			continue
		}
		lines = append(lines, line)
	}
	return lines
}

// _MENTION_ARGS: in the `@bot <verb>` form, a control verb counts only when what follows fits it;
// otherwise the line is a request in plain words ("@bot start with option A" asks for a build, it
// does not lift a halt). A colon right after the verb ("@bot halt: away this week") says the words
// are the verb's own. `/harness <verb>` is always a command.
//
// Each is its pattern's `.match(args)` on what `_strip_force` leaves, which holds no newline.
var mentionArgs = map[string]func(string) bool{
	"status":   empty,     // ^$
	"help":     oneWord,   // ^\S*$
	"stop":     empty,     // ^$
	"rebuild":  empty,     // ^$
	"review":   reviewArg, // ^(?i:strong|medium)?$
	"halt":     empty,     // ^$
	"start":    oneWord,   // ^\S*$
	"suggest":  empty,     // ^$
	"run":      runArg,    // ^(#?\d+)?$
	"suspend":  oneWord,   // ^\S*$
	"oneshot":  empty,     // ^$
	"split":    splitArg,  // ^(?i:bot)?$
	"fullsend": empty,     // ^$
}

func empty(args string) bool { return args == "" }

func oneWord(args string) bool { return !strings.ContainsFunc(args, py.IsSpace) }

func reviewArg(args string) bool {
	return args == "" || py.FoldEqual(args, "strong") || py.FoldEqual(args, "medium")
}

func runArg(args string) bool {
	if args == "" {
		return true
	}
	digits := strings.TrimPrefix(args, "#")
	return digits != "" && !strings.ContainsFunc(digits, func(r rune) bool { return !unicode.IsDigit(r) })
}

func splitArg(args string) bool { return args == "" || py.FoldEqual(args, "bot") }

// _WRAP
const wrap = "`*_~"

// listMark is `_LIST_MARK.sub("", line)`: `^\s*(?:[-*+]|\d+[.)])\s+` taken off the front.
func listMark(line string) string {
	rs := []rune(line)
	i := 0
	for i < len(rs) && py.IsSpace(rs[i]) {
		i++
	}
	switch {
	case i < len(rs) && (rs[i] == '-' || rs[i] == '*' || rs[i] == '+'):
		i++
	default:
		j := i
		for j < len(rs) && unicode.IsDigit(rs[j]) {
			j++
		}
		if j == i || j >= len(rs) || (rs[j] != '.' && rs[j] != ')') {
			return line
		}
		i = j + 1
	}
	j := i
	for j < len(rs) && py.IsSpace(rs[j]) {
		j++
	}
	if j == i {
		return line
	}
	return string(rs[j:])
}

// unmark is a line without the markdown a person may wrap a command in: a list marker, and
// backticks or emphasis around the whole command.
func unmark(line string) string {
	text := py.Strip(listMark(line))
	for text != "" {
		first, _ := utf8.DecodeRuneInString(text)
		last, _ := utf8.DecodeLastRuneInString(text)
		if !strings.ContainsRune(wrap, first) || !strings.ContainsRune(wrap, last) {
			break
		}
		if len(text) == 1 {
			text = ""
			break
		}
		text = py.Strip(text[1 : len(text)-1])
	}
	return strings.TrimRight(strings.TrimLeft(text, wrap), wrap)
}

// The command in what follows a prefix. `plain_words` is the `@bot` form.
func Read(rest string, line string, plainWords bool) *Command {
	first, tail, _ := strings.Cut(rest, " ")
	verb, ok := "", false
	if first != "" {
		verb, ok = canonical(first)
	}
	args, force := stripForce(tail)
	if ok && plainWords && !strings.HasSuffix(first, ":") {
		if shape, has := mentionArgs[verb]; has && !shape(args) {
			ok = false
		}
	}
	if ok {
		return &Command{Verb: verb, Args: args, Force: force, Line: py.Strip(line)}
	}
	args, force = stripForce(rest)
	if meant := Misspelt(args); meant != "" {
		return &Command{Verb: "typo", Args: args, Force: force, Line: py.Strip(line), Meant: meant}
	}
	return &Command{Verb: "request", Args: args, Force: force, Line: py.Strip(line)}
}

// matchMention is `^\s*@<login>\b[:,]?\s*(.*)$` (case-insensitive) on `line`, and its group 1.
func matchMention(line string, login string) (string, bool) {
	rs := []rune(line)
	i := 0
	for i < len(rs) && py.IsSpace(rs[i]) {
		i++
	}
	if i >= len(rs) || rs[i] != '@' {
		return "", false
	}
	end, ok := py.FoldAt(rs, i+1, login)
	if !ok || !py.Boundary(rs, end) {
		return "", false
	}
	i = end
	if i < len(rs) && (rs[i] == ':' || rs[i] == ',') {
		i++
	}
	for i < len(rs) && py.IsSpace(rs[i]) {
		i++
	}
	return string(rs[i:]), true
}

// Every command in a comment, in order, at most `MAX_COMMANDS`.
func Parse(body string, botLogin string) []*Command {
	login := strings.TrimLeft(botLogin, "@")
	found := []*Command{}
	for _, raw := range CommandLines(body) {
		if len(found) >= MaxCommands {
			break
		}
		line := unmark(raw)
		if rest, ok := matchSlash(line); ok {
			found = append(found, Read(py.Strip(rest), line, false))
			continue
		}
		rest, ok := matchMention(line, login)
		if !ok {
			continue
		}
		rest = py.Strip(rest)
		if strings.HasPrefix(py.Lower(rest), config.Slash) {
			continue // `@bot /harness verb` is matched by _SLASH above
		}
		found = append(found, Read(rest, line, true))
	}
	return found
}

// VOTES: the words that answer one of the bot's suggestions (#760): `approve` queues it to build,
// `reject` closes it. A vote is a line of its own holding only the word, bare or after `@<bot>`
// or `/harness`, and it counts only on an open suggestion (`events.on_comment`).
var Votes = []string{"approve", "reject"}

// withoutMention is `re.sub(r"^@<login>\b[:,]?\s*", "", line)`, case-insensitive.
func withoutMention(line string, login string) string {
	rs := []rune(line)
	if len(rs) == 0 || rs[0] != '@' {
		return line
	}
	end, ok := py.FoldAt(rs, 1, login)
	if !ok || !py.Boundary(rs, end) {
		return line
	}
	if end < len(rs) && (rs[end] == ':' || rs[end] == ',') {
		end++
	}
	for end < len(rs) && py.IsSpace(rs[end]) {
		end++
	}
	return string(rs[end:])
}

// The first vote in a comment, `(word, line)`, or `("", "")`. Quoted and fenced lines never
// vote, so quoting a vote back cannot cast it again.
func Vote(body string, botLogin string) (string, string) {
	login := strings.TrimLeft(botLogin, "@")
	for _, raw := range CommandLines(body) {
		line := unmark(raw)
		rest, ok := matchSlash(line)
		if !ok {
			rest = withoutMention(line, login)
		}
		word := py.Lower(py.Strip(strings.TrimRight(unmark(rest), ".!")))
		if slices.Contains(Votes, word) {
			return word, py.Strip(line)
		}
	}
	return "", ""
}

// namesHandle is `(?<![\w-])@<login>(?![\w-])` searched in `line`, case-insensitive.
func namesHandle(line string, login string) bool {
	rs := []rune(line)
	edge := func(r rune) bool { return py.IsWord(r) || r == '-' }
	for i, r := range rs {
		if r != '@' || (i > 0 && edge(rs[i-1])) {
			continue
		}
		if end, ok := py.FoldAt(rs, i+1, login); ok && (end == len(rs) || !edge(rs[end])) {
			return true
		}
	}
	return false
}

// namesSlash is `(?<![\w/])` + SLASH + `\b` searched in `line`, case-insensitive.
func namesSlash(line string) bool {
	rs := []rune(line)
	slash := config.Slash
	for i := range rs {
		if i > 0 && (py.IsWord(rs[i-1]) || rs[i-1] == '/') {
			continue
		}
		if end, ok := py.FoldAt(rs, i, slash); ok && py.Boundary(rs, end) {
			return true
		}
	}
	return false
}

// True when the bot's handle, or `/harness`, appears anywhere outside quotes and code, even
// mid-sentence: a request that did not parse is answered with a hint, never ignored.
func NamesTheBot(body string, botLogin string) bool {
	login := strings.TrimLeft(botLogin, "@")
	for _, line := range CommandLines(body) {
		if namesHandle(line, login) || namesSlash(line) {
			return true
		}
	}
	return false
}

// HELP_ROWS: the help table's rows, by verb. `oneshot`, `split` and `fullsend` show only for a
// bot with those modes (Squishy's three, both bots' `fullsend`), `suggest` only for one that makes
// suggestions (the night bot).
var HelpRows = [][2]string{
	{"build", "| `build [notes]` | queue this issue for the bot | issue | 2 |"},
	{"oneshot", "| `oneshot [notes]` | build this issue in one run, many agents at once (fullsend) | issue | 2 |"},
	{"split", "| `split [bot] [notes]` | break this issue into sub-issues I build, or with `bot` that the night bot builds | issue | 2 |"},
	{"fullsend", "| `fullsend [notes]` | split this issue into parts that land on one branch, then reconcile them into one pull request | issue | 2 |"},
	{"revise", "| `revise <notes>` | queue a revision of this PR with your notes | pull request | 2 |"},
	{"review", "| `review [strong\\|medium] [notes]` | queue a review run of this PR's head, by that tier or stronger; no revision | pull request | 2 |"},
	{"rebuild", "| `rebuild` | close my PR and build its issue again from `main`, its branch kept | issue or PR | 2 |"},
	{"stop", "| `stop` | take this issue or PR out of the queue and stop work on it | issue or PR | 2 |"},
	{"suggest", "| `suggest` | ask for improvement suggestions the next time a lane has nothing to start | anywhere | 2 |"},
	{"suggest", "| `approve`, `reject` | alone on a line, no prefix needed: build one of my suggestions, or close it unbuilt | suggestion | 2 |"},
	{"status", "| `status` | halt state, each subscription, the queue | anywhere | 1 |"},
	{"help", "| `help [verb]` | this list, or one command in detail | anywhere | 1 |"},
	{"halt", "| `halt [reason]` | stop all model work until `start` | anywhere | 3 |"},
	{"start", "| `start [subscription]` | lift a halt; with a subscription, lift its suspension instead | anywhere | 3 |"},
	{"run", "| `run [#n]` | start a run now, outside a subscription's hours if need be | anywhere | 3 |"},
	{"suspend", "| `suspend <subscription> [reason]` | start no new work on one subscription until `resume <subscription>` | anywhere | 3 |"},
}

// Whether this bot has the verb: the modes only where they are, suggestions only where
// they are made.
func Offered(verb string) bool {
	modes := config.Modes
	switch verb {
	case "oneshot":
		return slices.Contains(modes, "oneshot")
	case "split":
		return slices.ContainsFunc(modes, func(mode string) bool { return strings.HasPrefix(mode, "split") })
	case "fullsend":
		return slices.Contains(modes, "fullsend")
	case "suggest":
		return config.Identity.Suggestions
	}
	return true
}

// FORCED: the verbs `--force` starts at once, as the help names them: those of this bot's.
var Forced = forcedText()

func forcedText() string {
	var forced []string
	for _, verb := range []string{"build", "oneshot", "split", "fullsend", "revise", "suggest"} {
		if Offered(verb) {
			forced = append(forced, "`"+verb+"`")
		}
	}
	return strings.Join(forced[:len(forced)-1], ", ") + " or " + forced[len(forced)-1]
}

// HELP
var Help = helpTable()

func helpTable() string {
	var rows []string
	for _, row := range HelpRows {
		if Offered(row[0]) {
			rows = append(rows, row[1])
		}
	}
	return "**Commands.** Write one per line, starting with `{slash}` or `@{bot}`; the two work the same way.\n" +
		"\n" +
		"| Verb | What it does | Where | Level |\n" +
		"|---|---|---|---|\n" +
		strings.Join(rows, "\n") + "\n" +
		"\n" +
		"Anything else after `{slash}` or `@{bot}` is a request: a build on an issue, a revision on a PR, with your words as the notes. A single word that looks like a misspelt verb (`stauts`) runs nothing; I ask what you meant.\n" +
		"After `@{bot}`, a control verb followed by more words reads as plain English, so `@{bot} stop using the old sprite` is a request. Write the verb alone, or with a colon (`@{bot} halt: away this week`), for the command.\n" +
		"Add `--force` (level 3) to {forced} to start now, outside a subscription's hours if need be. Label an issue `difficulty:easy`, `difficulty:medium` (the default) or `difficulty:hard` to set the weakest model tier that may build it; `difficulty:hard` keeps it for Opus.\n" +
		"Labels do the same as the verbs: `{prefix}build` on an issue, `{prefix}revise` on a PR; assigning @{bot} queues the thread.\n" +
		"An issue is mine (my `{prefix}…` labels) or people's, never both: `human` keeps me off it, and `human required` is my hand-over when I find I cannot do it (I take my labels off, unassign myself, assign you and say what a person must do). Take either label off to hand it back, then `{slash} build`.\n" +
		"On your comment: 👀 seen · 👍 a model will read it · 🚀 answered · ❤️ a run has it · 🎉 done · 😕 it ended without an answer.\n"
}

// VERB_HELP: `help <verb>`: what it takes, what it does, and an example.
var VerbHelp = pyjson.New(
	"build", [3]string{"build [notes]", "Queue this issue for the bot; your notes join the request. On a " +
		"pull request it is the same as `revise`.", "{slash} build make the Coin spin too"},
	"oneshot", [3]string{"oneshot [notes]", "Build this issue in one run with fullsend: many agents write " +
		"its parts at once, each in its own worktree and branch, then the parts are " +
		"reconciled against tests written from the spec, and the change goes through the " +
		"checks and the review like any build. It falls back to a plain build, and says " +
		"why, when the issue does not suit it.", "{slash} oneshot"},
	"split", [3]string{"split [bot] [notes]", "Break this issue into sub-issues, each small enough for one " +
		"run, linked by what blocks what: for me to build, or with `bot` for the night bot. " +
		"They are queued at once, in order; `stop` here stops them all. When the last one " +
		"closes, I check this issue's end state and close it or add what is missing.",
		"{slash} split bot"},
	"fullsend", [3]string{"fullsend [notes]", "Split this issue into parts, sub-issues that each own their " +
		"files (or take the sub-issues it has already as its parts), and build each onto " +
		"one branch of this issue's own, not `main`, with no pull request, checks or " +
		"review of its own. When the last part closes, one run " +
		"reconciles the branch with fullsend, makes every check green and opens one pull " +
		"request into `main`, reviewed like any build.", "{slash} fullsend"},
	"revise", [3]string{"revise <notes>", "Queue a revision of this pull request with your notes. " +
		"Auto-merge stays off until the revision lands.", "@{bot} revise rename the helper"},
	"review", [3]string{"review [strong|medium] [notes]", "Queue a review run of the head of one of my pull " +
		"requests, and no revision: the head stays as it is. With `strong` or `medium` the " +
		"review waits for a model of that tier or stronger rather than taking a weaker " +
		"one. Your notes reach the reviewer.", "{slash} review strong check the replay"},
	"rebuild", [3]string{"rebuild", "Close my pull request for this issue, keep its branch as " +
		"`bot/old/issue-<n>-<date>`, and queue the issue to build again from `main`, at " +
		"the same difficulty. The new build is told what went wrong with the old one.",
		"{slash} rebuild"},
	"stop", [3]string{"stop", "Take this issue or pull request out of the queue. A run working on it gives " +
		"up at its next checkpoint and keeps what it has.", "{slash} stop"},
	"suggest", [3]string{"suggest", "Ask for a suggestion survey the next time a lane has nothing to " +
		"start, without waiting out the 30 idle minutes a survey otherwise waits for.",
		"@{bot} suggest"},
	"status", [3]string{"status", "Halt state, which subscriptions are running what right now, each " +
		"subscription (its hours, usage, and what it is doing), and what is queued.",
		"@{bot} status"},
	"help", [3]string{"help [verb]", "The list of commands, or one of them in detail.",
		"{slash} help build"},
	"halt", [3]string{"halt [reason]", "Stop all model work until `start`; a run already going stops at " +
		"its next checkpoint. After `@{bot}`, a colon after the verb gives the reason.",
		"@{bot} halt: away this week"},
	"start", [3]string{"start [subscription]", "Lift a halt. `start --force` also starts a run now. With a " +
		"subscription (`resume claude-3`), lift that one's suspension and leave a halt as it " +
		"is.", "{slash} start --force"},
	"run", [3]string{"run [#n]", "Start a run now, outside a subscription's hours if need be, for one item or " +
		"whatever is next in the queue.", "@{bot} run #12"},
	"suspend", [3]string{"suspend <subscription> [reason]", "Start no new work on one subscription (an id " +
		"`status` lists, such as `claude-3` or `gpt`) until `{slash} resume " +
		"<subscription>`; a run already going on it stops at its next checkpoint, keeps " +
		"its work and goes back to the queue for another subscription. `--force` does not " +
		"lift it. After `@{bot}`, a colon after the verb gives the reason.",
		"@{bot} suspend: claude-3 using it myself"},
)

// VOTE_HELP: `help approve` and `help reject`: the votes on a suggestion (#760), which are not
// verbs.
var VoteHelp string = "**`approve`** or **`reject`** (level 2, maintainer), alone on a line of a comment on " +
	"one of my open suggestions (`{prefix}suggestion`), bare or after `{slash}` or " +
	"`@{bot}`. `approve` queues it to build, as `{prefix}build` does; `reject` closes it " +
	"unbuilt, which makes room for the next survey. Anywhere else the words are only " +
	"words.\n\nFor example: `approve`. "

// POINTER
var Pointer string = "`{slash} help` (or `@{bot} help`) lists the commands."

// format is str.format(**values) for templates whose fields are plain names.
func format(template string, values map[string]string) string {
	var b strings.Builder
	for i := 0; i < len(template); i++ {
		c := template[i]
		switch {
		case c == '{' && i+1 < len(template) && template[i+1] == '{':
			b.WriteByte('{')
			i++
		case c == '}' && i+1 < len(template) && template[i+1] == '}':
			b.WriteByte('}')
			i++
		case c == '{':
			end := strings.IndexByte(template[i:], '}')
			if end < 0 {
				panic(fmt.Errorf("ValueError: Single '{' encountered in format string"))
			}
			name := template[i+1 : i+end]
			value, ok := values[name]
			if !ok {
				panic(fmt.Errorf("KeyError: %s", py.Repr(name)))
			}
			b.WriteString(value)
			i += end
		default:
			b.WriteByte(c)
		}
	}
	return b.String()
}

// The line that says where the commands are listed.
func Pointer_(bot string) string {
	return format(Pointer, map[string]string{"bot": bot, "slash": config.Slash})
}

// The reply to `help`, or to `help <verb>`.
func HelpText(bot string, topic string /* = "" */) string {
	words := py.Split(topic)
	word := ""
	if len(words) > 0 {
		word = words[0]
	}
	said := map[string]string{"bot": bot, "slash": config.Slash,
		"prefix": config.LabelPrefix, "forced": Forced}
	if word == "" {
		return format(Help, said)
	}
	if slices.Contains(Votes, strings.Trim(py.Lower(word), "`")) && Offered("suggest") {
		return format(VoteHelp, said) + format(Pointer, said)
	}
	verb, ok := canonical(word)
	if !ok || !Offered(verb) {
		return fmt.Sprintf("I do not know `%s`. ", word) + format(Help, said)
	}
	help := VerbHelp.Get(verb).([3]string)
	usage, what, example := help[0], help[1], help[2]
	level := Levels.Int(verb)
	var aliases []string
	for _, name := range Aliases.Keys() {
		if Aliases.Str(name) == verb {
			aliases = append(aliases, "`"+name+"`")
		}
	}
	also := ""
	if len(aliases) > 0 {
		also = "; also written " + strings.Join(aliases, ", ")
	}
	return fmt.Sprintf("**`%s`** (level %d, %s%s)\n\n%s\n\nFor example: `%s`. ", usage, level,
		trust.LevelName(level), also, format(what, said), format(example, said)) + format(Pointer, said)
}
