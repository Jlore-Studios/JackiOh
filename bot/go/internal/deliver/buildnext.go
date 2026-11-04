package deliver

// The planner's lane (#2903, the owner's decision of 2026-11-03; `own_review_run`).
//
// A planning run that took a lane of its own subscription (`plan.InLanePlanners`; its plan says
// `build_next`) hands that lane to the item's build when it delivers its plan, as a build hands its
// lane to its own review run (`handOn`): `keepLane` reserves it on the issue's record (`build_next`,
// for `own_review_reserve_minutes`), and the build is started at once as every next run is
// (`Chain_`, or the status loop's tick). That build runs on the model that planned it
// (`planned_model`, `plan.Assign`), in a fresh session, and its own review run on that model too.

import (
	"fmt"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	planmod "github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	statemod "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
)

// plannerSeat is the seat this run planned on (its result's plan `seat`), or nil.
func (x *Deliverer) plannerSeat() *providers.Seat {
	model := py.OrStr(x.resultPlan().Obj("seat").Get("model"), "")
	if model == "" {
		return nil
	}
	return x.Cfg.Pool.Seat(x.Provider.ID, model)
}

// notePlanModel records on issue `number` the model this planning run planned on
// (`queue.PlanModelKey`), which builds the item on this subscription (#2903): only where runs chain
// (`own_review_run`), and only for a planning run of its own; a build run that planned first, or one
// with no seat on record, leaves none, so an earlier one does not stand for it.
func (x *Deliverer) notePlanModel(number int, planning bool) {
	if !x.Cfg.OwnReviewRun {
		return
	}
	seat := x.plannerSeat()
	x.Ctx.Store.Update(func(s *pyjson.Object) any {
		record := statemod.Item(s, number)
		if planning && seat != nil {
			record.Set(queue.PlanModelKey, seat.Model)
		} else {
			record.Pop(queue.PlanModelKey)
		}
		return nil
	}, fmt.Sprintf("planned model #%d", number))
}

// keepLane reserves this planning run's lane for issue `number`'s build (#2903): when the plan says
// it was planned in its subscription's own lane (`build_next`), the item is queued to build, and the
// model that planned it may build it at its difficulty (`difficulty`, as its rating left it). The
// record's `build_next` holds the subscription, the model, since when and until when the lane is
// kept (`own_review_reserve_minutes`), and the run. Returns the sentence for the comment, "" when it
// reserved nothing.
func (x *Deliverer) keepLane(number int, difficulty string) string {
	if !x.Cfg.OwnReviewRun || !x.Plan.Truthy("build_next") || !x.Labels(number)[config.LabelBuild] {
		return ""
	}
	seat := x.plannerSeat()
	if seat == nil || !providers.TierAtLeast(seat.Tier, config.MinTier[difficulty]) ||
		!providers.TierAtLeast(seat.Tier, config.PlanFloor[difficulty]) {
		return ""
	}
	now := x.Ctx.Now()
	until := now.Add(time.Duration(x.Cfg.OwnReviewReserveMinutes) * time.Minute)
	x.remember(number, pyjson.New(queue.BuildNextKey, pyjson.New("provider", x.Provider.ID,
		"model", seat.Model, "since", clock.ISO(now), "until", clock.ISO(until), "from_run", x.Cfg.RunID)))
	x.log(fmt.Sprintf("#%d: its build on %s is next; its lane is kept until %s", number, seat.Describe(),
		clock.ISO(until)))
	return fmt.Sprintf("Its build is next, in this lane: %s builds it, the model that planned it, in a "+
		"fresh session, and then hands the lane to its own review run on the same model. The lane is kept "+
		"for it until %s.", seat.Describe(), planmod.Clock(x.Ctx, until))
}

// headModel is the model that built this run's head: the last it switched to (`work.Worker.Switch`),
// or else its build seat's.
func (x *Deliverer) headModel() string {
	model := x.BuildSeat.Model
	for _, switched := range objList(x.Result.Get("switches")) {
		if to := py.OrStr(switched.Get("to"), ""); to != "" && x.Cfg.Pool.Seat(x.Provider.ID, to) != nil {
			model = to
		}
	}
	return model
}
