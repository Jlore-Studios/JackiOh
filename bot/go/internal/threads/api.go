// Package threads is bot/harness/threads.py.
//
// What the model is told about an issue or a pull request, assembled from GitHub.
//
// Only people the trust file names at level 1 or above are quoted, apart from the issue's own body
// (someone trusted chose to build it). The bot's own comments are left out. Everything is returned
// already fenced as data.
package threads

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/trust"
)

// MAX_CHARS = 60_000
var MaxChars int = 60000

// issue_thread is threads.issue_thread.

func IssueThread(gh gh.Client, trust *trust.Trust, number int, botLogin string) string {
	panic("unported: threads.issue_thread")
}

// linked_issue is threads.linked_issue.

func LinkedIssue(pull *pyjson.Object) *int {
	panic("unported: threads.linked_issue")
}

// pull_text is threads.pull_text.

func PullText(pull *pyjson.Object) string {
	panic("unported: threads.pull_text")
}

// NO_FEEDBACK = "(no new comments from trusted people)"
var NoFeedback string = "(no new comments from trusted people)"

// `text` is what `pull_feedback` gives #`number` when nobody trusted wrote anything new (or
// nothing at all): equal to it, never merely holding its words, which a comment could quote.

func NoFeedback_(text string, number int) bool {
	panic("unported: threads.no_feedback")
}

// Trusted comments, reviews and line comments on a PR newer than `since`, and trusted
// comments on the issue it closes, since a request made there is routed to the PR.

func PullFeedback(gh gh.Client, trust *trust.Trust, number int, botLogin string, since string, issue *int /* = None */) string {
	panic("unported: threads.pull_feedback")
}
