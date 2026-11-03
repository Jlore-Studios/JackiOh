package deliver

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/usage"
)

// The usage history (#2749, `internal/usage`): a delivery hands the status loop its run's readings
// and its run's row through the state file's spool (`usage.Spool`), which the loop pushes to the
// usage branch at its next flush. A delivery never pushes the branch itself, wherever it runs.

// usageReadings is the run's readings as rows of the history, its subscription's and its run's;
// none without `usage_history`, or for a result from before it.
func (x *Deliverer) usageReadings() []usage.Row {
	if !x.Cfg.RecordsUsage() {
		return nil
	}
	var rows []usage.Row
	for _, r := range usage.ReadingsOf(x.Result, x.Provider.ID, x.Cfg.RunID) {
		rows = append(rows, r)
	}
	return rows
}

// usageModels is the models the run's seats ran, each once.
func (x *Deliverer) usageModels() []string {
	var models []string
	for _, s := range []*providers.Seat{x.BuildSeat, x.ReviewSeat} {
		if s != nil {
			models = append(models, s.Model)
		}
	}
	return uniqStrings(models)
}

// recordRun spools the run's own row for the usage history: what its journal section says, as
// numbers, with its readings at its start and end. A record, never a gate: a failure is logged,
// and the delivery stands.
func (x *Deliverer) recordRun() {
	if !x.Cfg.RecordsUsage() {
		return
	}
	run := usage.RunOf(usage.RunFacts{At: x.Ctx.Now(), RunID: x.Cfg.RunID, Subscription: x.Provider.ID,
		Models: x.usageModels(), Plan: x.Plan, Result: x.Result,
		Readings: usage.ReadingsOf(x.Result, x.Provider.ID, x.Cfg.RunID)})
	if err := herr.Try(func() {
		x.Ctx.Store.Update(func(s *pyjson.Object) any {
			usage.Spool(s, run)
			return nil
		}, "usage "+x.Provider.ID)
	}); err != nil {
		x.log("the usage history did not get this run: " + redact.Redact(herr.Str(err), nil))
	}
}
