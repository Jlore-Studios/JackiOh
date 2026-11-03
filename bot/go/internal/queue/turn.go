package queue

// Waiting its turn (#2948, section 2; `own_review_run`).
//
// A change's own review run approves its draft, and its final check test-merges it with `main` and
// after each open pull request whose changed files overlap it (`work.Worker.finalCheck`). When it
// conflicts once a pull request ahead of it has landed (approved earlier and conflict-free), deliver
// keeps it waiting its turn rather than marking it ready: `bot:waiting-turn` goes on, and its record
// holds `waiting_turn`: the subscription and the model that approved it, since when, until when at
// most (`hold_minutes`), the pull requests it waits behind and the head it approved. No lane is held
// while it waits: it is the lane's own, not a run's. When each it waits behind has closed (merged,
// the usual end), or the wait runs out, it is taken up again (`plan.TakeTurn`): a conflict revision
// (`source: conflict`, `bot:resolving`) owned by the same subscription, on the same model, whose lane
// is kept for it from then (`turn`, for `own_review_reserve_minutes`), as a build's is for its own
// review run. Its clearance carries across a resolution that changes only the conflicted files
// (`deliver.Carry`), so it then goes ready and auto-merges with no further review.
//
// A pull request a person merges (`ready for merge`, `person merges`) gives up its precedence once
// its CI is green (`PersonMerges`, `deliver.ahead`): it waits for a person, not for a lane, so no
// other waits behind it then, and its merge owner gives up its lane's claim on it (`sweep`'s
// `person_merges`).

import (
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// WaitingTurnKey is the record's field of a pull request waiting its turn.
var WaitingTurnKey = "waiting_turn"

// TurnKey is the record's field that keeps the lane for a pull request whose turn came, until its
// conflict revision takes it.
var TurnKey = "turn"

// GaveUpKey is the record's field of a pull request a person merges whose merge owner gave up its
// claim once its CI went green: the head, the subscription and when.
var GaveUpKey = "merge_owner_gave_up"

// WaitingTurnOf is the record's wait for its turn, or nil for none.
func WaitingTurnOf(record *pyjson.Object) *pyjson.Object {
	found, ok := record.Get(WaitingTurnKey).(*pyjson.Object)
	if !ok || found == nil || !found.Truthy("provider") {
		return nil
	}
	return found
}

// Behind is the pull requests a wait for its turn waits behind.
func Behind(waiting *pyjson.Object) []int {
	out := []int{}
	for _, n := range pyjson.List(waiting.Get("behind")) {
		if number, err := py.Int(n); err == nil && number != 0 {
			out = append(out, number)
		}
	}
	return out
}

// WaitingTurns is the pull requests waiting their turn, in the state file's order.
func WaitingTurns(s *pyjson.Object) []int {
	items := s.Obj("items")
	var out []int
	for _, key := range items.Keys() {
		if !py.IsDecimal(key) {
			continue
		}
		if record, ok := items.Get(key).(*pyjson.Object); ok && WaitingTurnOf(record) != nil {
			out = append(out, py.MustInt(key))
		}
	}
	return out
}

// TurnOf is the record's lane kept for its turn's conflict revision, or nil for none.
func TurnOf(record *pyjson.Object) *pyjson.Object {
	found, ok := record.Get(TurnKey).(*pyjson.Object)
	if !ok || found == nil || !found.Truthy("provider") {
		return nil
	}
	return found
}

// TurnReserved is: the lane is still kept for the record's turn at `now`: kept, not run out
// (`until`), and the item not held for its owner instead (`hold`), as `BuildReserved` reads a build's.
func TurnReserved(record *pyjson.Object, now time.Time) bool {
	found := TurnOf(record)
	if found == nil {
		return false
	}
	if hold := record.Obj("hold"); hold != nil && !hold.Truthy("lifted") && hold.Truthy("owner") {
		return false
	}
	until := clock.ParseISOValue(found.Get("until"))
	return !until.IsZero() && now.Before(until)
}

// PersonMerges is: a person merges the pull request with these labels (`ready for merge`, `person
// merges`), not auto-merge.
func PersonMerges(names map[string]bool) bool {
	return names[config.LabelReady] || names[config.LabelPersonMerges]
}

// GaveUp is: the record's merge owner gave up its claim on `head` (`GaveUpKey`).
func GaveUp(record *pyjson.Object, head string) bool {
	return head != "" && pyjson.Equal(record.Obj(GaveUpKey).Get("sha"), head)
}
