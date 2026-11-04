package queue

// A question a person answers and the bot carries out (#2986, sections 2 and 3; #3009, section 1;
// the owner's decisions of 2026-11-04).
//
// A builder or reviser that stops to ask leaves its question on the item's record (`question`), and
// deliver keeps what it asked beside it (`decision`): when, the problem, the options (each one's
// change and trade-off), the one it suggests (`default`) and why, the subscription and model that
// asked, the people it assigned (`assigned`), and when it goes ahead (`ends`, `decision_wait_hours`
// after it was asked; none for a question with no suggestion, or a bot that never goes ahead). It
// carries `needs decision` beside `bot:blocked`. `plan.SyncDecisions` takes it up again on the lane
// that asked once a trusted person replies (`answered_at`, which its claim reads, with the reply as
// the decision) or, with no reply, past its end on its default (`went_ahead`); it drops one a person
// queued or unblocked by hand, and takes the label off a closed thread. A hand-over (`human
// required`) is never a decision: the bot archives it instead (`archive.go`). A record from before
// #3009 keeps its options as text and the default's trade-off in `gives_up`; `OptionsOf` reads both.

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
)

// DecisionKey is the record's field of an item stopped on a question.
var DecisionKey = "decision"

// QuestionChars is how much of a question, its problem or its answer the stop keeps, for the status
// issue and the run that takes it up again.
var QuestionChars = 4000

// DecisionOf is the record's stop for a decision, or nil for none.
func DecisionOf(record *pyjson.Object) *pyjson.Object {
	found, ok := record.Get(DecisionKey).(*pyjson.Object)
	if !ok || found == nil || !found.Truthy("asked_at") {
		return nil
	}
	return found
}

// OptionsOf is the stop's options: each `{"change", "tradeoff"}` (#3009), or its text with the
// default's trade-off in `gives_up` (#2986).
func OptionsOf(decision *pyjson.Object) []verdicts.Option {
	out := []verdicts.Option{}
	if decision == nil {
		return out
	}
	chosen := py.MustInt(py.Or(decision.Get("default"), 0))
	for i, raw := range pyjson.List(decision.Get("options")) {
		option := verdicts.Option{}
		if found, ok := raw.(*pyjson.Object); ok {
			option = verdicts.Option{Change: py.OrStr(found.Get("change"), ""), Tradeoff: py.OrStr(found.Get("tradeoff"), "")}
		} else {
			option.Change = py.Str(raw)
		}
		if option.Tradeoff == "" && i+1 == chosen {
			option.Tradeoff = py.OrStr(decision.Get("gives_up"), "")
		}
		out = append(out, option)
	}
	return out
}

// HasDefault is: the stop names a default (its suggestion) the lane that asked may build.
func HasDefault(decision *pyjson.Object) bool {
	return decision != nil && !decision.Truthy("human_required") && py.MustInt(py.Or(decision.Get("default"), 0)) > 0
}

// DefaultOption is the stop's default option's change, "" for none.
func DefaultOption(decision *pyjson.Object) string {
	if !HasDefault(decision) {
		return ""
	}
	options := OptionsOf(decision)
	chosen := py.MustInt(decision.Get("default"))
	if chosen > len(options) {
		return ""
	}
	return options[chosen-1].Change
}

// WentAhead is the stop's default go-ahead, when the lane that asked took it up again ("" for
// none): its claim builds the default.
func WentAhead(decision *pyjson.Object) string {
	if decision == nil {
		return ""
	}
	return py.OrStr(decision.Get("went_ahead"), "")
}

// AnsweredAt is when a trusted person's reply to the stop took it up again on the lane that asked
// ("" for none): its claim carries the reply out as the decision (#3009).
func AnsweredAt(decision *pyjson.Object) string {
	if decision == nil {
		return ""
	}
	return py.OrStr(decision.Get("answered_at"), "")
}

// Resumes is how the lane that asked takes the stop up again: "answer" (a trusted person replied),
// "default" (nobody did in time) or "" (it still waits).
func Resumes(decision *pyjson.Object) string {
	switch {
	case AnsweredAt(decision) != "":
		return "answer"
	case WentAhead(decision) != "":
		return "default"
	}
	return ""
}

// Assigned is the people the stop assigned (`decision_assignees` who were not assigned already),
// whom it unassigns once it is answered, goes ahead or closes.
func Assigned(decision *pyjson.Object) []string {
	if decision == nil {
		return nil
	}
	return pyjson.Strings(decision.Get("assigned"))
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
