// Package declined is bot/harness/declined.py.
//
// Suggestions that came to nothing (#797), kept so that no survey offers one again.
//
// A survey sees only the latest issues (`plan.suggestion_plan`), so a suggestion rejected weeks ago
// falls out of its view and comes back. This log does not: it lives in the state file
// (`suggest.declined`), newest last, and every survey reads all of it. A suggestion goes in when:
//
//   - a person rejects it with a `reject` comment (`events.answer_vote`);
//   - a person closes it unbuilt (`sync`, read from GitHub at each survey);
//   - it was approved, and the bot then handed it to people as work only a person can do
//     (`deliver.Deliverer._needs_person`) or failed it for good (`deliver.Deliverer._fail`).
//
// A suggestion the survey offers again anyway, by a title already here, is not opened
// (`deliver.Deliverer._suggestions`).
//
// An entry may carry the person's `reason`: the rest of their `reject` comment, or the first
// comment a trusted person leaves on it after it closed (`events.OnComment`). The survey reads the
// reasons, so it learns what gets rejected and not only which titles.
//
// Beside it, the log of decisions (`suggest.decided`, newest last) holds every suggestion people
// decided on, approved or not, which the judge reads as its rolling window of what people want
// now (`suggestions.auto_approve.window`, `prompts/judge.md`): each one approved (`Approved`, as it
// is queued) and each that came to nothing (`Note`), with its reason when one comes.
package declined

import (
	"fmt"
	"regexp"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Limit is the most entries kept, oldest dropped first: the survey's prompt carries every one.
var Limit int = 200

// DecidedLimit is the most decisions kept, oldest dropped first: more than any window reads.
var DecidedLimit int = 200

// ReasonChars is how much of a person's reason an entry keeps.
var ReasonChars int = 500

// The decisions' verdicts.
const (
	VerdictApproved      = "approved"      // a person queued it
	VerdictAutoApproved  = "auto-approved" // it went ahead by itself (`plan.AutoApprove`): no person's decision
	VerdictCameToNothing = "came to nothing"
)

var words = regexp.MustCompile(`[a-z0-9]+`)

// key is _key: a title as the log compares it, lowercased, its words alone.
func key(title string) string {
	return strings.Join(words.FindAllString(py.Lower(title), -1), " ")
}

// Entries is declined.entries.
func Entries(state *pyjson.Object) []*pyjson.Object {
	return pyjson.Objs(state.Obj("suggest").Get("declined"))
}

// Note logs suggestion `number` as come to nothing, once: True when it was not logged yet.
func Note(state *pyjson.Object, number int, title string, why string, at string) bool {
	suggest, ok := state.SetDefault("suggest", pyjson.New()).(*pyjson.Object)
	log := Entries(state)
	for _, e := range log {
		if py.MustInt(py.Or(e.Get("number"), 0)) == number {
			return false
		}
	}
	log = append(log, pyjson.New(
		"number", number,
		"title", py.SliceTo(strings.Join(py.Split(title), " "), 200),
		"why", py.SliceTo(strings.Join(py.Split(why), " "), 300),
		"at", at))
	if len(log) > Limit {
		log = log[len(log)-Limit:]
	}
	if !ok || suggest == nil {
		// `suggest["declined"] = …` on a "suggest" that is not a dict
		panic(fmt.Errorf("'%s' object does not support item assignment", "NoneType"))
	}
	suggest.Set("declined", pyjson.ListOf(log))
	decide(state, number, title, VerdictCameToNothing, why, at)
	return true
}

// Entry is the log's entry for suggestion `number`, or nil.
func Entry(state *pyjson.Object, number int) *pyjson.Object {
	for _, e := range Entries(state) {
		if py.MustInt(py.Or(e.Get("number"), 0)) == number {
			return e
		}
	}
	return nil
}

// Reason puts a person's reason on suggestion `number`'s entry, and on its decision, once: True
// when the entry had none yet. Blank text, or a suggestion not in the log, changes nothing.
func Reason(state *pyjson.Object, number int, text string) bool {
	text = py.SliceTo(strings.Join(py.Split(text), " "), ReasonChars)
	entry := Entry(state, number)
	if text == "" || entry == nil || entry.Str("reason") != "" {
		return false
	}
	entry.Set("reason", text)
	for _, d := range DecidedEntries(state) {
		if py.MustInt(py.Or(d.Get("number"), 0)) == number && d.Str("reason") == "" {
			d.Set("reason", text)
		}
	}
	return true
}

// DecidedEntries is the log of decisions, newest last.
func DecidedEntries(state *pyjson.Object) []*pyjson.Object {
	return pyjson.Objs(state.Obj("suggest").Get("decided"))
}

// Approved logs suggestion `number` as approved by `by` (`queue.QueueBuild`), or as approved by
// itself when `auto`.
func Approved(state *pyjson.Object, number int, title string, by string, auto bool, at string) {
	verdict, why := VerdictApproved, "approved by @"+by
	if auto {
		verdict, why = VerdictAutoApproved, "approved by itself: no one voted on it"
	}
	decide(state, number, title, verdict, why, at)
}

// decide appends one decision, the latest for its suggestion: an earlier one for the same number
// (approved, then come to nothing) goes, so each suggestion counts once, as it ended.
func decide(state *pyjson.Object, number int, title string, verdict string, why string, at string) {
	suggest := pyjson.Obj(state.SetDefault("suggest", pyjson.New()))
	kept := []*pyjson.Object{}
	for _, d := range DecidedEntries(state) {
		if py.MustInt(py.Or(d.Get("number"), 0)) != number {
			kept = append(kept, d)
		}
	}
	kept = append(kept, pyjson.New(
		"number", number,
		"title", py.SliceTo(strings.Join(py.Split(title), " "), 200),
		"verdict", verdict,
		"why", py.SliceTo(strings.Join(py.Split(why), " "), 300),
		"at", at))
	if len(kept) > DecidedLimit {
		kept = kept[len(kept)-DecidedLimit:]
	}
	suggest.Set("decided", pyjson.ListOf(kept))
}

// DecidedLines is the latest `window` decisions as the judge reads them, newest first, one a line,
// with a tally of the people's verdicts (an approval by itself is no person's), or "(none)".
func DecidedLines(state *pyjson.Object, window int) string {
	all := DecidedEntries(state)
	if window < len(all) {
		all = all[len(all)-window:]
	}
	if len(all) == 0 || window <= 0 {
		return "(none)"
	}
	approved, declined := 0, 0
	var lines []string
	for i := len(all) - 1; i >= 0; i-- {
		d := all[i]
		switch d.Str("verdict") {
		case VerdictApproved:
			approved++
		case VerdictCameToNothing:
			declined++
		}
		line := fmt.Sprintf("- #%s %s: %s (%s)", py.Str(d.Get("number")), py.Str(d.Get("title")),
			d.Str("verdict"), py.Str(d.Get("why")))
		if r := d.Str("reason"); r != "" {
			line += ". Why: " + r
		}
		lines = append(lines, line)
	}
	head := fmt.Sprintf("The latest %d: %d approved by a person, %d came to nothing.", len(all), approved, declined)
	return head + "\n\n" + strings.Join(lines, "\n")
}

// Sync logs every closed suggestion in `closed` (issues labelled `bot:suggestion`) that was
// closed unbuilt: a suggestion that is built loses the label as it is queued, so one still
// carrying it when closed never was. Returns the numbers newly logged.
func Sync(state *pyjson.Object, closed []*pyjson.Object, at string) []int {
	added := []int{}
	for _, issue := range closed {
		if issue.Has("pull_request") || !pyjson.Equal(issue.Get("state"), "closed") {
			continue
		}
		reason := py.OrStr(issue.Get("state_reason"), "closed")
		number := py.MustInt(issue.Get("number"))
		if Note(state, number, py.OrStr(issue.Get("title"), ""),
			fmt.Sprintf("closed unbuilt by a person (%s)", strings.ReplaceAll(reason, "_", " ")),
			py.OrStr(issue.Get("closed_at"), at)) {
			added = append(added, py.MustInt(issue.Get("number")))
		}
	}
	return added
}

// Known is whether a suggestion of this title came to nothing before.
func Known(state *pyjson.Object, title string) bool {
	k := key(title)
	if k == "" {
		return false
	}
	for _, e := range Entries(state) {
		if key(py.OrStr(e.Get("title"), "")) == k {
			return true
		}
	}
	return false
}

// Lines is the log as the survey reads it, one suggestion a line, or "(none)".
func Lines(state *pyjson.Object) string {
	var found []string
	for _, e := range Entries(state) {
		line := fmt.Sprintf("- #%s %s: %s", py.Str(e.Get("number")), py.Str(e.Get("title")),
			py.Str(e.Get("why")))
		if r := e.Str("reason"); r != "" {
			line += ". Why: " + r
		}
		found = append(found, line)
	}
	if len(found) == 0 {
		return "(none)"
	}
	return strings.Join(found, "\n")
}
