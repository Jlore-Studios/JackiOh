package queue

// A stop only a person can unblock (#2986, sections 2 and 3; the owner's decision of 2026-11-04).
//
// A builder or reviser that stops to ask leaves its question on the item's record (`question`), and
// deliver keeps what it asked beside it (`decision`): when, the options and the default (the one
// that stays inside the issue's own limits, and what it gives up), the subscription and model that
// asked, and when it goes ahead (`ends`, `decision_wait_hours` after it was asked; none for a
// question with no default, or a bot that never goes ahead). A hand-over (`human required`) keeps
// `human_required` there instead, and never goes ahead. Both carry `needs decision` beside
// `bot:blocked` (or `human required`). `plan.SyncDecisions` takes one past its end up again on the
// lane that asked, to build its default (`went_ahead`, which its claim reads), drops one a person
// answered, and takes the label off a closed thread.

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// DecisionKey is the record's field of an item stopped on a question or a hand-over.
var DecisionKey = "decision"

// QuestionChars is how much of a question the stop keeps, for the status issue and the go-ahead.
var QuestionChars = 4000

// DecisionOf is the record's stop for a decision, or nil for none.
func DecisionOf(record *pyjson.Object) *pyjson.Object {
	found, ok := record.Get(DecisionKey).(*pyjson.Object)
	if !ok || found == nil || !found.Truthy("asked_at") {
		return nil
	}
	return found
}

// HasDefault is: the stop names a default the lane that asked may build.
func HasDefault(decision *pyjson.Object) bool {
	return decision != nil && !decision.Truthy("human_required") && py.MustInt(py.Or(decision.Get("default"), 0)) > 0
}

// DefaultOption is the stop's default option's text, "" for none.
func DefaultOption(decision *pyjson.Object) string {
	if !HasDefault(decision) {
		return ""
	}
	options := pyjson.List(decision.Get("options"))
	chosen := py.MustInt(decision.Get("default"))
	if chosen > len(options) {
		return ""
	}
	return py.Str(options[chosen-1])
}

// WentAhead is the stop's default go-ahead, when the lane that asked took it up again ("" for
// none): its claim builds the default.
func WentAhead(decision *pyjson.Object) string {
	if decision == nil {
		return ""
	}
	return py.OrStr(decision.Get("went_ahead"), "")
}

// Decisions is the items stopped for a decision, in the state file's order.
func Decisions(s *pyjson.Object) []int {
	items := s.Obj("items")
	var out []int
	for _, key := range items.Keys() {
		if !py.IsDecimal(key) {
			continue
		}
		if record, ok := items.Get(key).(*pyjson.Object); ok && DecisionOf(record) != nil {
			out = append(out, py.MustInt(key))
		}
	}
	return out
}
