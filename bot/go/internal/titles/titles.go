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
// Nothing fires on a title edit (triage runs on `opened` and `labeled`; `bot-commands` and
// `squishy-commands` on `labeled` and `assigned`), and an edit made with the workflow's token starts
// no workflow at all. (The one-time `retcon` of every thread, #1372, ran and was removed; git
// history has it.)
//
// The patterns are Python's `re` patterns with Python's classes spelled out, since Go's are ASCII:
// `\d` is any script's decimal digit (\p{Nd}), `\s` Python's whitespace (`py.ReS`), and `\b`, which
// RE2 has no Unicode form of, a check of the neighbouring character against Python's `\w`
// (letters, digits, `_`).
package titles

import (
	"fmt"
	"regexp"
	"strings"

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

// PatchesJSON is where the shipped patches are listed.
var PatchesJSON = "crates/cards/patches/patches.json"

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
		return py.MustInt(m[1])
	}
	if m := merge.FindStringSubmatch(subject); m != nil {
		return py.MustInt(m[1])
	}
	return 0
}

// ClosedBy is the issues a pull request's body closes, in order, each once.
func ClosedBy(body string) []int {
	seen := []int{}
	for _, n := range findAllAtWordStart(closes, body) {
		number := py.MustInt(n)
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
// a version `before` lists (the version retcon, #1371) shipped nothing: it retitles no thread and
// comments on none.
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
		version := py.OrStr(entry.Get("version"), "")
		for _, commit := range py.Each(entry.Get("commits")) {
			number := PullOf(subjectOf(py.Str(commit)))
			if number == 0 {
				continue
			}
			var pull *pyjson.Object
			if _, failed := herr.Catch[*herr.GitHubError](func() { pull = gh.GetPull(number) }); failed {
				continue
			}
			threads := []thread{{number, pull, true}}
			for _, issue := range ClosedBy(py.OrStr(pull.Get("body"), "")) {
				var data *pyjson.Object
				if _, failed := herr.Catch[*herr.GitHubError](func() { data = gh.GetIssue(issue) }); failed {
					continue
				}
				threads = append(threads, thread{issue, data, false})
			}
			for _, t := range threads {
				title := py.OrStr(t.data.Get("title"), "")
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
