// Package asks is bot/harness/asks.py.
//
// Reactions that follow a request through the queue, so a person sees where it has got to.
//
// A comment that asks for model work (a build, a revision, a request in plain words, a suggestion
// survey, or notes left while a run holds the thread) is recorded as an *ask* on the state record
// it waits on: the thread's item, or `state["suggest"]`. An ask is the key its claim uses, `c:<id>`
// for a conversation comment and `rc:<id>` for a line comment on a diff. A review cannot take a
// reaction, so a review's request is not tracked. Each stage reacts to the ask:
//
// | Reaction | Meaning | Left by |
// |---|---|---|
// | 👀 `eyes` | seen | events, before acting |
// | 👍 `+1` | a model will read it: queued, or noted for the next pass | events |
// | 🚀 `rocket` | answered; the sweep reads it as "handled" | events, after replying |
// | ❤️ `heart` | a run has it: a model is reading it now | plan, claiming the thread |
// | 🎉 `hooray` | done: the run that read it finished with an answer | deliver |
// | 😕 `confused` | it ended without an answer: blocked, stopped or given up on | deliver, stop |
//
// An ask waits in `asks` until a run claims the thread, which moves it to `taken_asks`. When the
// run ends, each taken ask gets 🎉 or 😕, or goes back to waiting if the item is queued again. An
// ask that arrives while a run holds the thread waits for the next pass. The helpers that change a
// record are pure, for use inside `StateStore.update`; `react` is called once the write is done.
package asks

import (
	"strings"
	"unicode"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Seen = "eyes"
var Seen string = "eyes"

// Queued = "+1"
var Queued string = "+1"

// Answered = "rocket"
var Answered string = "rocket"

// Working = "heart"
var Working string = "heart"

// Done = "hooray"
var Done string = "hooray"

// NoAnswer = "confused"
var NoAnswer string = "confused"

// MaxAsks is the most asks one record keeps; the oldest go first.
var MaxAsks int = 20

// Ask is the comment a request came from. `recorded` says whether anything queued it.
type Ask struct {
	Key      string
	Recorded bool // default: False
}

// Key is asks.key.
func Key(commentID int, reviewComment bool /* = False */) string {
	if reviewComment {
		return "rc:" + py.Str(commentID)
	}
	return "c:" + py.Str(commentID)
}

// merge is _merge: the entries of every group in order, each once, the newest MaxAsks.
func merge(groups ...[]any) []any {
	merged := []any{}
	for _, group := range groups {
		for _, entry := range group {
			if !containsValue(merged, entry) {
				merged = append(merged, entry)
			}
		}
	}
	if len(merged) > MaxAsks {
		merged = merged[len(merged)-MaxAsks:]
	}
	return merged
}

func containsValue(list []any, v any) bool {
	for _, e := range list {
		if pyjson.Equal(e, v) {
			return true
		}
	}
	return false
}

// list is `holder.get(key) or []` iterated: a list's items (a string's characters and an
// object's keys, as Python iterates them).
func list(holder *pyjson.Object, key string) []any { return py.Each(holder.Get(key)) }

func strs(xs []any) []string {
	out := make([]string, len(xs))
	for i, x := range xs {
		out[i] = py.Str(x)
	}
	return out
}

// Note records `ask` as waiting on `holder`.
func Note(holder *pyjson.Object, ask *Ask) {
	if ask == nil {
		return
	}
	holder.Set("asks", merge(list(holder, "asks"), []any{ask.Key}))
	ask.Recorded = true
}

// Add is asks.add.
func Add(holder *pyjson.Object, keys []string) {
	holder.Set("asks", merge(list(holder, "asks"), pyjson.ListOf(keys)))
}

// Take is a run having claimed the holder: its waiting asks become taken. Returns those that
// moved.
func Take(holder *pyjson.Object) []string {
	moved := append([]any(nil), list(holder, "asks")...)
	holder.Set("taken_asks", merge(list(holder, "taken_asks"), moved))
	holder.Set("asks", []any{})
	return strs(moved)
}

// GiveBack is the run having ended without an answer and the item queued again: its asks wait
// once more.
func GiveBack(holder *pyjson.Object) {
	holder.Set("asks", merge(list(holder, "taken_asks"), list(holder, "asks")))
	holder.Set("taken_asks", []any{})
}

// PopTaken is asks.pop_taken.
func PopTaken(holder *pyjson.Object) []string {
	taken := append([]any(nil), list(holder, "taken_asks")...)
	holder.Set("taken_asks", []any{})
	return strs(taken)
}

// PopWaiting is asks.pop_waiting.
func PopWaiting(holder *pyjson.Object) []string {
	waiting := append([]any(nil), list(holder, "asks")...)
	holder.Set("asks", []any{})
	return strs(waiting)
}

// React leaves `content` on every ask in `keys`. A reaction is a courtesy: `gh.react` swallows
// its own failures.
func React(gh gh.Client, keys []string, content string) {
	for _, entry := range keys {
		kind, raw, _ := strings.Cut(entry, ":")
		if kind != "c" && kind != "rc" {
			continue
		}
		if id, ok := digits(raw); ok {
			gh.React(id, content, kind == "rc")
		}
	}
}

// digits is `raw.isdigit()` and then int(raw): Unicode decimal digits read as Python reads them.
// (A digit isdigit takes that int() refuses, such as `²`, would raise in Python; none is ever in
// a key this module wrote.)
func digits(raw string) (int, bool) {
	if raw == "" {
		return 0, false
	}
	n := 0
	for _, r := range raw {
		d, ok := decimalValue(r)
		if !ok {
			return 0, false
		}
		n = n*10 + d
	}
	return n, true
}

// decimalValue is the digit a Unicode decimal digit (category Nd) stands for: every Nd range
// runs in whole sets of ten from a zero.
func decimalValue(r rune) (int, bool) {
	if !unicode.IsDigit(r) {
		return 0, false
	}
	for _, rg := range unicode.Nd.R16 {
		if lo, hi := rune(rg.Lo), rune(rg.Hi); r >= lo && r <= hi {
			return int(r-lo) / int(rg.Stride) % 10, true
		}
	}
	for _, rg := range unicode.Nd.R32 {
		if lo, hi := rune(rg.Lo), rune(rg.Hi); r >= lo && r <= hi {
			return int(r-lo) / int(rg.Stride) % 10, true
		}
	}
	return 0, false
}
