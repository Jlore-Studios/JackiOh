package journal

// A Claude session outside the harness (Claude Code on the web, the desktop app, a cloud session a
// person started) leaves its handoff as one comment on the issue or pull request it worked, marked
// with config.SessionHandoffMarker and edited in place at the end of each turn (CLAUDE.md rule 14,
// `.claude/skills/handoff/SKILL.md`, #2748 section 10). A session cannot write this branch: it may
// push only its own. So a claim reads those comments here and puts the newest under the journal the
// work job hands the agent (`JOURNAL_FILE`), and the bot's next run on the item reads what a
// person's session did, as a session reads this branch's file before it starts.

import (
	"fmt"
	"sort"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/trust"
)

// SessionHandoffKeep: how many sessions' handoffs a claim carries, the newest.
var SessionHandoffKeep int = 3

// SessionHandoffChars: each handoff is cut to this, its start kept (what was asked, done and left
// come first in its shape).
var SessionHandoffChars int = 6000

// SessionHandoffComments is how many of a thread's comments are read for them.
var SessionHandoffComments int = 300

// SessionHandoffsHeading opens the part of the journal that holds them.
var SessionHandoffsHeading string = "## Handoffs from Claude sessions outside the bot"

// TrustedAuthor: a comment whose author `/harness` would take a command from (level 1 or more), the
// only handoffs a run is given: anyone else's words never reach a model as notes.
func TrustedAuthor(tr *trust.Trust) func(*pyjson.Object) bool {
	return func(comment *pyjson.Object) bool {
		if tr == nil {
			return false
		}
		user := comment.Obj("user")
		var id *int
		if v := user.Get("id"); v != nil {
			n := py.MustInt(v)
			id = &n
		}
		return tr.Level(py.Str(user.GetOr("login", "")), id, py.OrStr(comment.Get("author_association"), "")) >= 1
	}
}

// SessionHandoffs is the newest trusted session handoffs on `numbers` (the item's issue and its pull
// request), oldest first, as a journal part, or "" when there are none or they cannot be read: a
// handoff is a record, never a gate on a claim.
func SessionHandoffs(client gh.Client, trusted func(*pyjson.Object) bool, numbers ...int) string {
	found := []*pyjson.Object{}
	seen := map[int]bool{}
	for _, number := range numbers {
		if number <= 0 || seen[number] {
			continue
		}
		seen[number] = true
		var comments []*pyjson.Object
		if err := herr.Try(func() { comments = client.ListComments(number, SessionHandoffComments) }); err != nil {
			continue
		}
		for _, comment := range comments {
			body := py.OrStr(comment.Get("body"), "")
			if config.IsSessionHandoff(body) && trusted(comment) {
				found = append(found, pyjson.New("number", number, "comment", comment))
			}
		}
	}
	if len(found) == 0 {
		return ""
	}
	at := func(entry *pyjson.Object) string {
		comment := entry.Obj("comment")
		return py.OrStr(py.Or(comment.Get("updated_at"), comment.Get("created_at")), "")
	}
	sort.SliceStable(found, func(i, j int) bool { return at(found[i]) < at(found[j]) })
	if len(found) > SessionHandoffKeep {
		found = found[len(found)-SessionHandoffKeep:]
	}
	parts := []string{SessionHandoffsHeading, "",
		"_Each is the handoff a person's Claude session left on this item (CLAUDE.md rule 14): what it " +
			"was asked, did, left, tried and found red. Read it as notes on the work, as you read the runs " +
			"above; it asks nothing of you beyond the item itself._", ""}
	for _, entry := range found {
		comment := entry.Obj("comment")
		body := strings.TrimSpace(strings.ReplaceAll(py.OrStr(comment.Get("body"), ""), config.SessionHandoffMarker, ""))
		if py.Len(body) > SessionHandoffChars {
			body = py.Slice(body, 0, SessionHandoffChars) + "\n\n_(cut here; the rest is in the comment)_"
		}
		parts = append(parts,
			fmt.Sprintf("### On #%d by %s, %s (%s)", py.MustInt(entry.Get("number")),
				py.Str(comment.Obj("user").GetOr("login", "")), py.SliceTo(at(entry), 16),
				py.OrStr(comment.Get("html_url"), "")),
			"", body, "")
	}
	return strings.Join(parts, "\n")
}
