package queue

// The own review run (#2748, the owner's decision of 2026-11-02; `own_review_run`).
//
// A run that changes code builds it and runs the checks, and its deliver opens or pushes the pull
// request and hands its lane to the change's own review run: a second run on the same
// subscription, which reviews the change in fresh adversarial sessions, fixes what they find in the
// same run and approves it. Between the two the pull request's record holds the reservation
// (`own_review`): the subscription it is for, since when, until when its lane stays reserved, the
// head run 1 pushed and the checks run 1 ran on it. The pull request carries `bot:reviewing` from
// then until its own review run delivers. The reservation is queued in the state file, as an
// after-merge spot-check is, since its live label keeps it out of every label's list
// (`Candidates`); `claimed` marks the run that took it up.

import (
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// OwnReviewKey is the record's field that holds the reservation.
var OwnReviewKey = "own_review"

// OwnReviewOf is the record's own review reservation, or nil for none.
func OwnReviewOf(record *pyjson.Object) *pyjson.Object {
	found, ok := record.Get(OwnReviewKey).(*pyjson.Object)
	if !ok || found == nil || !found.Truthy("provider") {
		return nil
	}
	return found
}

// OwnReviewWaits is: the record's pull request waits for its own review run: reserved, and no run
// has taken it up yet.
func OwnReviewWaits(record *pyjson.Object) bool {
	found := OwnReviewOf(record)
	return found != nil && !found.Truthy("claimed")
}

// OwnReviewReserved is: the lane of the own review's subscription is still reserved for it at
// `now`: it waits, its reservation has not run out (`until`), and it is not held for that
// subscription instead (`hold`: a capped subscription keeps no lane while it cannot start).
func OwnReviewReserved(record *pyjson.Object, now time.Time) bool {
	if !OwnReviewWaits(record) {
		return false
	}
	if hold := record.Obj("hold"); hold != nil && !hold.Truthy("lifted") && hold.Truthy("owner") {
		return false
	}
	until := clock.ParseISOValue(OwnReviewOf(record).Get("until"))
	return !until.IsZero() && now.Before(until)
}

// OwnReviewsWaiting is the threads whose own review run waits, in the state file's order.
func OwnReviewsWaiting(s *pyjson.Object) []int {
	items := s.Obj("items")
	var out []int
	for _, key := range items.Keys() {
		if !py.IsDecimal(key) {
			continue
		}
		if record, ok := items.Get(key).(*pyjson.Object); ok && OwnReviewWaits(record) {
			out = append(out, py.MustInt(key))
		}
	}
	return out
}

// RequeueOwnReview puts a claimed own review back to waiting (its run died): its subscription takes
// it up again as its own work, with no lane reserved for it meanwhile.
func RequeueOwnReview(record *pyjson.Object, at time.Time) {
	found := OwnReviewOf(record)
	if found == nil {
		return
	}
	found.Pop("claimed")
	found.Set("until", clock.ISO(at))
}
