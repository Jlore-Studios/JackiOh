// Package titles is bot/harness/titles.py.
//
// Titles after the patch numbers follow the cards (#1360): every thread opens `Patch: <what it
// does>`, and a shipped card change `Patch v<version>: <what it does>`.
//
//   - `what_it_does(title)` is a title without its prefix, the old ones included (`Patch v0.3.X`,
//     `Patch v0.3.Y`, `vX.Y.0`, `Night bot`, `CI`, `Architecture`) and the new, with the `(part n of
//     m)` a part carries after its prefix kept apart.
//
//   - `new_title(title, version)` is the title in the new form: `Patch: …`, or `Patch v<version>: …`
//     for a shipped card change, its part kept.
//
//   - `harness titles shipped --before <sha> --after <sha>` (`patches-ship.yml`, once a
//     promotion merges, #1370) retitles each patch the push shipped: the pull request whose commit
//     added its fragment and the issues that pull request closes become `Patch v<version>: …`, and each
//     gets a comment naming the version. A version is read from `patches.json` at the two commits,
//     never parsed (R105).
//
//   - `harness titles retcon [--write] [--numbers 1,2] [--limit N] [--csv path]` (#1372)
//     retitles every issue and pull request, closed ones included, from a reviewed dry run: a shipped
//     card change's threads become `Patch v<version>: …` (the version from `patches.json`, linked by its
//     `source` and by the issues its fragment's pull request closes), every other thread `Patch: …`.
//     Titles a workflow or a bot finds by their text, the `production merge` threads, the status issue
//     and the tools' own titles are left alone (`left_alone`). Open issues whose old prefix names their
//     kind (`Architecture`, `CI`, `Night bot`: internal; `vX.Y.0`: app) get its `kind:*` label; a
//     pull request gets its own from the classifier. Without `--write` it writes nothing, only the CSV.
//
// Nothing fires on a title edit (triage runs on `opened` and `labeled`; `bot-commands` and
// `squishy-commands` on `labeled` and `assigned`), and an edit made with the workflow's token starts
// no workflow at all.
//
// The patterns are Python's `re` patterns with Python's classes spelled out, since Go's are ASCII:
// `\d` is any script's decimal digit (\p{Nd}), `\s` Python's whitespace (`py.ReS`), and `\b`, which
// RE2 has no Unicode form of, a check of the neighbouring character against Python's `\w`
// (letters, digits, `_`).
package titles

import (
	"fmt"
	"regexp"
	"sort"
	"strings"
	"time"
	"unicode"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// prefix is a title's prefix, old or new, and the `(part n of m)` right after it.
var prefix = regexp.MustCompile(
	`(?s)^(?:Patch(?: v\p{Nd}+\.\p{Nd}+\.(?:\p{Nd}+[a-z]?|X|Y)[a-z]?)?|v\p{Nd}+\.\p{Nd}+\.0|Night bot|CI|Architecture)` +
		`(?P<part> \(part \p{Nd}+ of \p{Nd}+\))?: (?P<rest>.*)$`)

// closes is "Closes #12", "Fixes #12, #14": the issues a pull request's body says it closes.
// Python's leading `\b` is `wordStart`.
var closes = regexp.MustCompile(`(?i)(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?)` + py.ReS + `+#(\p{Nd}+)`)

// squash and merge: a squash commit's subject ends with its pull request's number; a merge
// commit says it.
var (
	squash = regexp.MustCompile(`\(#(\p{Nd}+)\)` + py.ReS + `*$`)
	merge  = regexp.MustCompile(`^Merge pull request #(\p{Nd}+)(?:$|` + py.ReNotW + `)`)
)

// sourceNumber is `#(\d+)`: an issue a patch's `source` names.
var sourceNumber = regexp.MustCompile(`#(\p{Nd}+)`)

// PatchesJSON is where the shipped patches are listed.
var PatchesJSON = "crates/cards/patches/patches.json"

// LeftAlone matches the threads whose titles a workflow or a bot finds by their text, or that a
// tool names, which the retcon leaves as they are (#1372): ci-duration.yml's alert, super.yml's,
// deploy-watch.yml's, patches-ship.yml's refusal and its watch's stuck issue, the disk alert
// (`disk.TITLE`), the weekly hygiene issues (.github/hygiene/branches.py), the status issues
// (`dashboard.TITLE`, Squishy's), and the tools' own pull requests.
var LeftAlone = regexp.MustCompile(
	`^(?:CI: a job ran over \p{Nd}+ minutes$` +
		`|CI: daily super run failed$` +
		`|Render: the live server is not serving main's catalog$` +
		`|patches ship refused to promote the pending patches$` +
		`|CI: a patches promotion or log is stuck$` +
		`|Night bot: the machine's disk is filling up$` +
		`|Architecture: weekly hygiene(?:$|` + py.ReNotW + `)` +
		`|(?:Night bot|Squishy) status$` +
		`|patches ship: ` +
		`|Promote main to production: ` +
		`|Merging to production(?:$|` + py.ReNotW + `)` +
		`|AI gen \p{Nd}+ ` +
		`|Revert ")`)

// StatusIssue is the status issue (#148), whatever its title says.
var StatusIssue = 148

// ProductionLabel is the label of the production merge's issues and pull requests, which their
// workflow titles.
var ProductionLabel = "production merge"

// PrefixKind is one of PrefixKinds: a prefix and the kind label it names.
type PrefixKind struct {
	Pattern *regexp.Regexp
	Kind    string
}

// PrefixKinds is the kind an open issue's old prefix names (#1372): its label goes on open
// threads only, since every `labeled` event starts a `bot-commands` run. A `Patch v…` title names
// none.
var PrefixKinds = []PrefixKind{
	{regexp.MustCompile(`^(?:Architecture|CI|Night bot)(?: \(part \p{Nd}+ of \p{Nd}+\))?: `), "kind:internal"},
	{regexp.MustCompile(`^v\p{Nd}+\.\p{Nd}+\.0(?: \(part \p{Nd}+ of \p{Nd}+\))?: `), "kind:app"},
}

// WhatItDoes is `(part, rest)`: the ` (part n of m)` right after the prefix ("" for none) and the
// title without its prefix. A title with no prefix is all rest.
func WhatItDoes(title string) (string, string) {
	text := strings.Join(py.Split(title), " ")
	m := prefix.FindStringSubmatch(text)
	if m == nil {
		return "", text
	}
	return m[prefix.SubexpIndex("part")], py.Strip(m[prefix.SubexpIndex("rest")])
}

// NewTitle is the title in the new form: `Patch: …`, or `Patch v<version>: …` for a shipped card
// change.
func NewTitle(title string, version string /* = "" */) string {
	part, rest := WhatItDoes(title)
	named := ""
	if version != "" {
		named = " " + version
	}
	return "Patch" + named + part + ": " + rest
}

// PullOf is the pull request a commit on main landed, from its subject, or 0.
func PullOf(subject string) int {
	if m := squash.FindStringSubmatch(subject); m != nil {
		return pyInt(m[1])
	}
	if m := merge.FindStringSubmatch(subject); m != nil {
		return pyInt(m[1])
	}
	return 0
}

// ClosedBy is the issues a pull request's body closes, in order, each once.
func ClosedBy(body string) []int {
	seen := []int{}
	for _, n := range findAllAtWordStart(closes, body) {
		number := pyInt(n)
		if !containsInt(seen, number) {
			seen = append(seen, number)
		}
	}
	return seen
}

// findAllAtWordStart is `re.findall(r"\b<pattern>", text)` for a pattern that starts with a
// letter and has one group: Go finds the leftmost match, and one that does not start a word is
// tried again one character on, as Python's scan does.
func findAllAtWordStart(re *regexp.Regexp, text string) []string {
	var found []string
	for at := 0; at <= len(text); {
		loc := re.FindStringSubmatchIndex(text[at:])
		if loc == nil {
			break
		}
		start, end := at+loc[0], at+loc[1]
		if start > 0 && py.IsWord(lastRune(text[:start])) {
			_, size := firstRune(text[start:])
			at = start + size
			continue
		}
		found = append(found, text[at+loc[2]:at+loc[3]])
		if end == start {
			end++
		}
		at = end
	}
	return found
}

func containsInt(xs []int, n int) bool {
	for _, x := range xs {
		if x == n {
			return true
		}
	}
	return false
}

// Shipped is the patches `after` appends to `before`, in ship order. A push that renames or drops
// a version `before` lists (the version retcon, #1371) shipped nothing: its threads are the title
// retcon's (#1372), paced and reviewed, never a comment on each.
func Shipped(before []*pyjson.Object, after []*pyjson.Object) []*pyjson.Object {
	head := after
	if len(head) > len(before) {
		head = after[:len(before)]
	}
	if len(head) != len(before) {
		return []*pyjson.Object{}
	}
	for i := range before {
		if py.Str(head[i].Get("version")) != py.Str(before[i].Get("version")) {
			return []*pyjson.Object{}
		}
	}
	return append([]*pyjson.Object{}, after[len(before):]...)
}

// runGit is `_git`: `subprocess.run(["git", *args], check=True, capture_output=True,
// text=True).stdout`, raising CalledProcessError on a non-zero exit. A variable so the tests can
// replace it.
var runGit = gitOutput

// patchesAt is `_patches_at`: patches.json at `rev`, or none when git or the JSON fails.
func patchesAt(rev string) []*pyjson.Object {
	var text string
	if _, failed := herr.Catch[*calledProcessError](func() { text = runGit("show", rev+":"+PatchesJSON) }); failed {
		return []*pyjson.Object{}
	}
	v, err := pyjson.LoadsString(text)
	if err != nil {
		return []*pyjson.Object{}
	}
	return patchList(v)
}

// patchList is patches.json's list as objects (Python reads each entry with `.get`).
func patchList(v any) []*pyjson.Object {
	list := pyjson.List(v)
	out := make([]*pyjson.Object, len(list))
	for i, e := range list {
		out[i] = pyjson.Obj(e)
	}
	return out
}

// subjectOfCommit is the default `subject_of`: a commit's subject.
func subjectOfCommit(sha string) string {
	return py.Strip(runGit("log", "-1", "--format=%s", sha))
}

type thread struct {
	number int
	data   *pyjson.Object
	isPull bool
}

// RetitleShipped retitles, for each shipped patch, the pull request that added its fragment and
// the issues it closes, and comments the version on each. What it did, one line per thread.
// `subjectOf` nil is the commit's subject from git.
func RetitleShipped(gh gh.Client, entries []*pyjson.Object, subjectOf func(sha string) string /* = None */) []string {
	if subjectOf == nil {
		subjectOf = subjectOfCommit
	}
	done := []string{}
	for _, entry := range entries {
		version := str0(entry.Get("version"))
		for _, commit := range iterate(entry.Get("commits")) {
			number := PullOf(subjectOf(py.Str(commit)))
			if number == 0 {
				continue
			}
			var pull *pyjson.Object
			if _, failed := herr.Catch[*herr.GitHubError](func() { pull = gh.GetPull(number) }); failed {
				continue
			}
			threads := []thread{{number, pull, true}}
			for _, issue := range ClosedBy(str0(pull.Get("body"))) {
				var data *pyjson.Object
				if _, failed := herr.Catch[*herr.GitHubError](func() { data = gh.GetIssue(issue) }); failed {
					continue
				}
				threads = append(threads, thread{issue, data, false})
			}
			for _, t := range threads {
				title := str0(t.data.Get("title"))
				retitled := NewTitle(title, version)
				if retitled != title {
					if t.isPull {
						gh.UpdatePull(t.number, pyjson.New("title", retitled))
					} else {
						gh.UpdateIssue(t.number, pyjson.New("title", retitled))
					}
				}
				comment(gh, t.number, t.data.Truthy("locked"),
					"Shipped as **"+version+"** (`patches ship`, #1360): "+
						"its title is now `"+retitled+"`.")
				done = append(done, fmt.Sprintf("#%d: %s", t.number, retitled))
			}
		}
	}
	return done
}

// comment comments on a thread, unlocking a locked one for the comment and locking it again: the
// workflow's token cannot comment on a locked thread (`lock-conversations.yml`).
func comment(gh gh.Client, number int, locked bool, body string) {
	if !locked {
		gh.CreateComment(number, body)
		return
	}
	gh.Unlock(number)
	defer gh.Lock(number)
	gh.CreateComment(number, body)
}

// LeftAlone_ is `left_alone`: whether the retcon leaves a thread's title as it is (#1372). (The
// name LeftAlone is the pattern's, as in Python, where the two differ by case.)
func LeftAlone_(thread *pyjson.Object) bool {
	labels := labelNames(thread)
	title := strings.Join(py.Split(str0(thread.Get("title"))), " ")
	return pyInt(orZero(thread.Get("number"))) == StatusIssue || labels[ProductionLabel] ||
		LeftAlone.MatchString(title)
}

// labelNames is `{str(label.get("name") if isinstance(label, dict) else label) for label in
// thread.get("labels") or []}`.
func labelNames(thread *pyjson.Object) map[string]bool {
	names := map[string]bool{}
	for _, label := range iterate(thread.Get("labels")) {
		if o, ok := label.(*pyjson.Object); ok {
			names[py.Str(o.Get("name"))] = true
		} else {
			names[py.Str(label)] = true
		}
	}
	return names
}

// VersionsByThread is each thread of a shipped card change, to the version that shipped it: the
// issues its `source` names, the pull request whose commit added its fragment and the issues that
// pull request closes. A thread several patches name takes the newest.
func VersionsByThread(patches []*pyjson.Object, gh gh.Client, subjectOf func(sha string) string) map[int]string {
	found := map[int]string{}
	for _, entry := range patches {
		version := str0(entry.Get("version"))
		var numbers []int
		for _, m := range sourceNumber.FindAllStringSubmatch(str0(entry.Get("source")), -1) {
			numbers = append(numbers, pyInt(m[1]))
		}
		for _, commit := range iterate(entry.Get("commits")) {
			number := 0
			if _, failed := herr.Catch[*calledProcessError](func() { number = PullOf(subjectOf(py.Str(commit))) }); failed {
				number = 0
			}
			if number == 0 {
				continue
			}
			numbers = append(numbers, number)
			herr.Catch[*herr.GitHubError](func() {
				numbers = append(numbers, ClosedBy(str0(gh.GetPull(number).Get("body")))...)
			})
		}
		for _, number := range numbers {
			found[number] = version
		}
	}
	return found
}

// Retitle is one row of the retcon's CSV.
type Retitle struct {
	Number int
	Kind   string
	State  string
	Old    string
	New    string
	Label  string // default: ""
}

// PlanRetcon is every thread the retcon changes, in number order: its new title, and the kind
// label an open issue gets from its old prefix.
func PlanRetcon(threads []*pyjson.Object, versions map[int]string) []*Retitle {
	ordered := append([]*pyjson.Object{}, threads...)
	sort.SliceStable(ordered, func(i, j int) bool {
		return pyInt(ordered[i].Get("number")) < pyInt(ordered[j].Get("number"))
	})
	rows := []*Retitle{}
	for _, thread := range ordered {
		if LeftAlone_(thread) {
			continue
		}
		number := pyInt(thread.Get("number"))
		old := strings.Join(py.Split(str0(thread.Get("title"))), " ")
		newTitle := NewTitle(old, versions[number])
		pull := thread.Has("pull_request")
		state := str0(thread.Get("state"))
		label := ""
		if !pull && state == "open" {
			for _, pk := range PrefixKinds {
				if pk.Pattern.MatchString(old) {
					label = pk.Kind
					break
				}
			}
		}
		if labelNames(thread)[label] {
			label = ""
		}
		if newTitle != old || label != "" {
			kind := "issue"
			if pull {
				kind = "pull"
			}
			rows = append(rows, &Retitle{number, kind, state, old, newTitle, label})
		}
	}
	return rows
}

// WriteCsv writes the rows as Python's csv.writer does (the excel dialect: `,`, a field quoted
// only when it holds `,`, `"`, \r or \n, a quote doubled, each row ended by \r\n).
func WriteCsv(rows []*Retitle, path string) {
	var b strings.Builder
	writeRow(&b, []string{"number", "kind", "state", "old title", "new title", "label"})
	for _, row := range rows {
		writeRow(&b, []string{fmt.Sprint(row.Number), row.Kind, row.State, row.Old, row.New, row.Label})
	}
	herr.Must(writeFile(path, b.String()))
}

func writeRow(b *strings.Builder, fields []string) {
	for i, field := range fields {
		if i > 0 {
			b.WriteByte(',')
		}
		if strings.ContainsAny(field, ",\"\r\n") {
			b.WriteString(`"` + strings.ReplaceAll(field, `"`, `""`) + `"`)
		} else {
			b.WriteString(field)
		}
	}
	b.WriteString("\r\n")
}

// ApplyRetcon writes the rows: each title, then its label. Returns how many threads it changed.
// `sleep` nil is time.sleep.
func ApplyRetcon(gh gh.Client, rows []*Retitle, pause float64 /* = 0.0 */, sleep func(float64) /* = None */) int {
	if sleep == nil {
		sleep = func(seconds float64) { time.Sleep(time.Duration(seconds * float64(time.Second))) }
	}
	changed := 0
	for _, row := range rows {
		if row.New != row.Old {
			gh.UpdateIssue(row.Number, pyjson.New("title", row.New))
		}
		if row.Label != "" {
			gh.AddLabels(row.Number, []string{row.Label})
		}
		changed++
		if pause != 0 {
			sleep(pause)
		}
	}
	return changed
}

// ------------------------------------------------------------------ Python's str and int

func lastRune(s string) rune {
	rs := []rune(s)
	if len(rs) == 0 {
		return -1
	}
	return rs[len(rs)-1]
}

func firstRune(s string) (rune, int) {
	for _, r := range s {
		return r, len(string(r))
	}
	return -1, 1
}

// str0 is `str(v or "")`.
func str0(v any) string {
	if !pyjson.Truthy(v) {
		return ""
	}
	return py.Str(v)
}

// orZero is `v or 0`.
func orZero(v any) any {
	if !pyjson.Truthy(v) {
		return 0
	}
	return v
}

// iterate is `for e in value or []` over a JSON value: a list's items, an object's keys.
func iterate(value any) []any {
	switch v := value.(type) {
	case []any:
		return v
	case *pyjson.Object:
		return pyjson.ListOf(v.Keys())
	}
	return nil
}

// pyInt is int(v) for a JSON number or a string of digits; it raises ValueError (as int() does)
// for anything else.
func pyInt(v any) int {
	switch x := v.(type) {
	case string:
		n, ok := parseInt(x)
		if !ok {
			herr.Raise(&valueError{"invalid literal for int() with base 10: " + py.Repr(x)})
		}
		return n
	case bool:
		if x {
			return 1
		}
		return 0
	}
	if n, ok := pyjson.AsInt(v); ok {
		return n
	}
	herr.Raise(&typeError{fmt.Sprintf("int() argument must be a string, a bytes-like object or a real number, not '%s'", typeName(v))})
	return 0
}

// parseInt is int(text): surrounding whitespace, a sign, then digits of any one script with `_`
// between two of them.
func parseInt(text string) (int, bool) {
	s := py.Strip(text)
	neg := false
	if s != "" && (s[0] == '+' || s[0] == '-') {
		neg = s[0] == '-'
		s = s[1:]
	}
	if s == "" {
		return 0, false
	}
	n, lastDigit := 0, false
	for _, r := range s {
		switch {
		case unicode.IsDigit(r):
			n = n*10 + py.DigitValue(r)
			lastDigit = true
		case r == '_' && lastDigit:
			lastDigit = false
		default:
			return 0, false
		}
	}
	if !lastDigit {
		return 0, false
	}
	if neg {
		n = -n
	}
	return n, true
}

func typeName(v any) string {
	switch v.(type) {
	case nil:
		return "NoneType"
	case []any:
		return "list"
	case *pyjson.Object:
		return "dict"
	}
	return fmt.Sprintf("%T", v)
}

// valueError and typeError are Python's ValueError and TypeError.
type valueError struct{ msg string }

func (e *valueError) Error() string { return e.msg }

type typeError struct{ msg string }

func (e *typeError) Error() string { return e.msg }
