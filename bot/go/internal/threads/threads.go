// Package threads is bot/harness/threads.py.
//
// What the model is told about an issue or a pull request, assembled from GitHub.
//
// Only people the trust file names at level 1 or above are quoted, apart from the issue's own body
// (someone trusted chose to build it). The bot's own comments are left out. Everything is returned
// already fenced as data.
package threads

import (
	"fmt"
	"sort"
	"strings"
	"unicode"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/prompts"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/trust"
)

// MAX_CHARS
var MaxChars int = 60000

// key is `entry[k]`: a KeyError when the entry is no object holding it.
func key(entry any, k string) any { return py.Item(pyjson.Obj(entry), k) }

// closes is `_CLOSES.search(text)` and its group 1:
// `(?i)\b(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?)\s+#(\d+)\b`.
func closes(text string) (int, bool) {
	rs := []rune(text)
	sOrD := func(i int) bool { return i < len(rs) && (py.FoldEq(rs[i], 's') || py.FoldEq(rs[i], 'd')) }
	// Each alternative's ends, the optional part taken first.
	stems := []func(p int) []int{
		func(p int) []int { // close[sd]?
			end, ok := py.FoldAt(rs, p, "close")
			if !ok {
				return nil
			}
			if sOrD(end) {
				return []int{end + 1, end}
			}
			return []int{end}
		},
		func(p int) []int { // fix(?:e[sd])?
			end, ok := py.FoldAt(rs, p, "fix")
			if !ok {
				return nil
			}
			if end < len(rs) && py.FoldEq(rs[end], 'e') && sOrD(end+1) {
				return []int{end + 2, end}
			}
			return []int{end}
		},
		func(p int) []int { // resolve[sd]?
			end, ok := py.FoldAt(rs, p, "resolve")
			if !ok {
				return nil
			}
			if sOrD(end) {
				return []int{end + 1, end}
			}
			return []int{end}
		},
	}
	// \s+#(\d+)\b after a stem ending at e.
	rest := func(e int) (int, bool) {
		i := e
		for i < len(rs) && py.IsSpace(rs[i]) {
			i++
		}
		if i == e || i >= len(rs) || rs[i] != '#' {
			return 0, false
		}
		j := i + 1
		for j < len(rs) && unicode.IsDigit(rs[j]) {
			j++
		}
		if j == i+1 || !py.Boundary(rs, j) {
			return 0, false
		}
		return py.Atoi(string(rs[i+1 : j])), true
	}
	for p := range rs {
		if !py.Boundary(rs, p) {
			continue
		}
		for _, stem := range stems {
			for _, e := range stem(p) {
				if n, ok := rest(e); ok {
					return n, true
				}
			}
		}
	}
	return 0, false
}

func isBot(entry *pyjson.Object, botLogin string) bool {
	login := py.Lower(py.Str(entry.Obj("user").GetOr("login", "")))
	return login == py.Lower(botLogin) || login == "github-actions[bot]" ||
		strings.Contains(py.OrStr(entry.Get("body"), ""), config.Marker)
}

func trusted(entry *pyjson.Object, tr *trust.Trust) bool {
	user := entry.Obj("user")
	// Trust.level reads the id as int(user_id), and only against a line that pins one; GitHub's
	// ids are numbers.
	var userID *int
	if n, ok := pyjson.AsInt(user.Get("id")); ok {
		userID = &n
	}
	login := user.GetOr("login", "")
	return tr.Level(py.OrStr(login, ""), userID, py.OrStr(entry.Get("author_association"), "")) >= 1
}

func clip(text string) string {
	if py.Len(text) <= MaxChars {
		return text
	}
	return py.SliceTo(text, MaxChars) + "\n\n(cut here: too long)"
}

// IssueThread is threads.issue_thread.
func IssueThread(gh gh.Client, trust *trust.Trust, number int, botLogin string) string {
	issue := gh.GetIssue(number)
	author := py.Str(issue.Obj("user").GetOr("login", "?"))
	body := py.OrStr(issue.Get("body"), "")
	if body == "" {
		body = "(no description)"
	}
	parts := []string{fmt.Sprintf("# #%d: %s", number, py.Str(issue.GetOr("title", ""))),
		fmt.Sprintf("Opened by @%s.", author), "", body}
	skipped := 0
	for _, comment := range gh.ListComments(number, 300) {
		if isBot(comment, botLogin) {
			continue
		}
		if !trusted(comment, trust) {
			skipped++
			continue
		}
		who := py.Str(comment.Obj("user").GetOr("login", "?"))
		parts = append(parts, "", fmt.Sprintf("---\nComment by @%s at %s:", who,
			py.Str(comment.GetOr("created_at", ""))), "", py.OrStr(comment.Get("body"), ""))
	}
	if skipped > 0 {
		parts = append(parts, "", fmt.Sprintf("(%d comment(s) from people outside the trust list were left out.)", skipped))
	}
	return prompts.Data(clip(strings.Join(parts, "\n")), fmt.Sprintf("Issue #%d", number))
}

// LinkedIssue is threads.linked_issue: the issue the pull request's body says it closes, or nil.
func LinkedIssue(pull *pyjson.Object) *int {
	if n, ok := closes(py.OrStr(pull.Get("body"), "")); ok {
		return &n
	}
	return nil
}

// PullText is threads.pull_text.
func PullText(pull *pyjson.Object) string {
	body := py.OrStr(pull.Get("body"), "")
	if body == "" {
		body = "(no description)"
	}
	body = strings.ReplaceAll(body, config.Marker, "")
	text := fmt.Sprintf("# PR #%s: %s\n\n%s", py.Str(pull.Get("number")), py.Str(pull.GetOr("title", "")), body)
	return prompts.Data(clip(text), fmt.Sprintf("Pull request #%s", py.Str(pull.Get("number"))))
}

// NO_FEEDBACK: what `pull_feedback` says when no trusted person wrote anything new
// (`work.Worker._merge_only`).
var NoFeedback string = "(no new comments from trusted people)"

// `text` is what `pull_feedback` gives #`number` when nobody trusted wrote anything new (or
// nothing at all): equal to it, never merely holding its words, which a comment could quote.
func NoFeedback_(text string, number int) bool {
	return py.Strip(text) == "" ||
		py.Strip(text) == py.Strip(prompts.Data(NoFeedback, fmt.Sprintf("Feedback on #%d", number)))
}

type entry struct {
	at   string
	body string
}

// Trusted comments, reviews and line comments on a PR newer than `since`, and trusted
// comments on the issue it closes, since a request made there is routed to the PR.
func PullFeedback(gh gh.Client, trust *trust.Trust, number int, botLogin string, since string, issue *int /* = None */) string {
	cutoff := clock.ParseISO(since)
	var entries []entry
	if issue != nil && *issue != 0 {
		// The PR's own feedback still goes through whatever this raises.
		herr.Try(func() {
			for _, comment := range gh.ListComments(*issue, 300) {
				at := clock.ParseISO(py.OrStr(comment.Get("created_at"), ""))
				if isBot(comment, botLogin) || !trusted(comment, trust) {
					continue
				}
				if cutoff.IsZero() || at.IsZero() || at.After(cutoff) {
					who := py.Str(key(key(comment, "user"), "login"))
					entries = append(entries, entry{py.Str(comment.GetOr("created_at", "")),
						fmt.Sprintf("Comment by @%s on issue #%d:\n\n%s", who, *issue, py.OrStr(comment.Get("body"), ""))})
				}
			}
		})
	}

	newer := func(e *pyjson.Object, k string) bool {
		at := clock.ParseISO(py.OrStr(e.Get(k), ""))
		return cutoff.IsZero() || at.IsZero() || at.After(cutoff)
	}

	for _, comment := range gh.ListComments(number, 300) {
		if !isBot(comment, botLogin) && trusted(comment, trust) && newer(comment, "created_at") {
			who := py.Str(key(key(comment, "user"), "login"))
			entries = append(entries, entry{py.Str(comment.GetOr("created_at", "")),
				fmt.Sprintf("Comment by @%s:\n\n%s", who, py.OrStr(comment.Get("body"), ""))})
		}
	}
	for _, review := range gh.ListReviews(number) {
		body := py.Strip(py.OrStr(review.Get("body"), ""))
		state := py.Str(review.GetOr("state", ""))
		if isBot(review, botLogin) || !trusted(review, trust) {
			continue
		}
		if !newer(review, "submitted_at") || (body == "" && state != "CHANGES_REQUESTED") {
			continue
		}
		who := py.Str(key(key(review, "user"), "login"))
		entries = append(entries, entry{py.Str(review.GetOr("submitted_at", "")),
			fmt.Sprintf("Review by @%s (%s):\n\n%s", who, strings.ReplaceAll(py.Lower(state), "_", " "), body)})
	}
	for _, line := range gh.ListReviewComments(number) {
		if isBot(line, botLogin) || !trusted(line, trust) || !newer(line, "created_at") {
			continue
		}
		who := py.Str(key(key(line, "user"), "login"))
		at := line.Get("line")
		if !pyjson.Truthy(at) {
			at = line.Get("original_line")
		}
		if !pyjson.Truthy(at) {
			at = "?"
		}
		where := fmt.Sprintf("%s:%s", py.Str(line.GetOr("path", "?")), py.Str(at))
		entries = append(entries, entry{py.Str(line.GetOr("created_at", "")),
			fmt.Sprintf("Line comment by @%s on %s:\n\n%s", who, where, py.OrStr(line.Get("body"), ""))})
	}
	sort.SliceStable(entries, func(i, j int) bool { return entries[i].at < entries[j].at })
	if len(entries) == 0 {
		return prompts.Data(NoFeedback, fmt.Sprintf("Feedback on #%d", number))
	}
	bodies := make([]string, len(entries))
	for i, e := range entries {
		bodies[i] = e.body
	}
	return prompts.Data(clip(strings.Join(bodies, "\n\n---\n\n")), fmt.Sprintf("Feedback on #%d", number))
}
