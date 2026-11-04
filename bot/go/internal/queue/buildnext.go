package queue

// The planner's lane (#2903, the owner's decision of 2026-11-03; `own_review_run`).
//
// An item's plan, its build (run 1) and its own review run (run 2) go on one lane with one model.
// A planning run of an item that will be built takes a lane of its own subscription when that
// subscription has room (`plan.InLanePlanners`), rather than the planning lane: its record marks it
// (`InLaneKey`), so it holds that lane as a build does (`plan.ReadLanes`). Its delivery hands the
// lane to the item's build, as a build hands its lane to its own review run (`own_review`): the
// issue's record holds the reservation (`build_next`): the subscription and the model that planned
// it, since when, until when the lane stays reserved (`own_review_reserve_minutes`), and the run
// that planned it. Until then the lane counts as held for that subscription, and its build takes it
// (`Lanes.Without`), on the planner's model (`plan.Assign`, `planned_model`); the build's claim
// drops the reservation. The item stays queued by its label (`bot:build`), as every build is. When
// its subscription cannot start the build, the hold and the handoff go as for any owned item
// (`plan.Owning`), and a handoff or a release drops it (`plan.Release`).

import (
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// BuildNextKey is the issue record's field that holds the planner's lane for its build.
var BuildNextKey = "build_next"

// InLaneKey marks a planning run's record when the run holds a lane of its own subscription, not
// the planning lane (`plan.InLanePlanners`).
var InLaneKey = "in_lane"

// PlanModelKey is the issue record's field for the model whose planning run wrote its plan: its
// subscription builds it on that model, and its own review run reviews it on it (#2903).
var PlanModelKey = "planned_model"

// BuildNextOf is the record's reservation of its planner's lane for its build, or nil for none.
func BuildNextOf(record *pyjson.Object) *pyjson.Object {
	found, ok := record.Get(BuildNextKey).(*pyjson.Object)
	if !ok || found == nil || !found.Truthy("provider") {
		return nil
	}
	return found
}

// BuildReserved is: the planner's lane is still reserved for the item's build at `now`: a
// reservation, not run out (`until`), and the item not held for its owner instead (`hold`: a capped
// subscription keeps no lane while it cannot start), as `OwnReviewReserved` reads an own review's.
func BuildReserved(record *pyjson.Object, now time.Time) bool {
	found := BuildNextOf(record)
	if found == nil {
		return false
	}
	if hold := record.Obj("hold"); hold != nil && !hold.Truthy("lifted") && hold.Truthy("owner") {
		return false
	}
	until := clock.ParseISOValue(found.Get("until"))
	return !until.IsZero() && now.Before(until)
}

// PlanningPool is: the record's run is a planning run on the planning lane (`plan_lanes`), which
// takes none of its subscription's lanes; a planning run in a lane of its own (`InLaneKey`) holds
// that lane as a build does.
func PlanningPool(record *pyjson.Object) bool {
	return record.Get("action") == "plan" && !record.Truthy(InLaneKey)
}

// BuildsNext is the issues whose planner's lane is kept for their build (`BuildNextOf`), in the
// state file's order.
func BuildsNext(s *pyjson.Object) []int {
	items := s.Obj("items")
	var out []int
	for _, key := range items.Keys() {
		if !py.IsDecimal(key) {
			continue
		}
		if record, ok := items.Get(key).(*pyjson.Object); ok && BuildNextOf(record) != nil {
			out = append(out, py.MustInt(key))
		}
	}
	return out
}
