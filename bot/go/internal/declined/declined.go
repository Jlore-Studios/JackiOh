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
package declined

import (
	"fmt"
	"regexp"
	"strconv"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Limit is the most entries kept, oldest dropped first: the survey's prompt carries every one.
var Limit int = 200

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
		if toInt(orZero(e.Get("number"))) == number {
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
	return true
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
		reason := orDefault(issue.Get("state_reason"), "closed")
		number := toInt(issue.Get("number"))
		if Note(state, number, orDefault(issue.Get("title"), ""),
			fmt.Sprintf("closed unbuilt by a person (%s)", strings.ReplaceAll(reason, "_", " ")),
			orDefault(issue.Get("closed_at"), at)) {
			added = append(added, toInt(issue.Get("number")))
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
		if key(orDefault(e.Get("title"), "")) == k {
			return true
		}
	}
	return false
}

// Lines is the log as the survey reads it, one suggestion a line, or "(none)".
func Lines(state *pyjson.Object) string {
	var found []string
	for _, e := range Entries(state) {
		found = append(found, fmt.Sprintf("- #%s %s: %s", py.Str(e.Get("number")), py.Str(e.Get("title")),
			py.Str(e.Get("why"))))
	}
	if len(found) == 0 {
		return "(none)"
	}
	return strings.Join(found, "\n")
}

// orDefault is `str(v or def)`.
func orDefault(v any, def string) string {
	if !pyjson.Truthy(v) {
		return def
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

// toInt is int(v) for a JSON value; what int() refuses raises (panics), as in Python.
func toInt(v any) int {
	switch x := v.(type) {
	case bool:
		if x {
			return 1
		}
		return 0
	case string:
		n, err := strconv.Atoi(strings.ReplaceAll(py.Strip(x), "_", ""))
		if err != nil {
			panic(fmt.Errorf("invalid literal for int() with base 10: %s", py.Repr(x)))
		}
		return n
	}
	if n, ok := pyjson.AsInt(v); ok {
		return n
	}
	panic(fmt.Errorf("int() argument must be a string, a bytes-like object or a real number, not '%s'",
		"NoneType"))
}
