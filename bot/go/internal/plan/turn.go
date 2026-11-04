package plan

// A pull request's turn (#2948, section 2; `queue/turn.go` has the rules). Its own review run
// approved it, and deliver kept it waiting behind a pull request ahead of it (`waiting_turn`,
// `bot:waiting-turn`). `SyncTurns`, in every plan's housekeeping, in the sweep and when a pull
// request closes (`events.OnPullClosed`), takes it up again once each it waits behind has closed, or
// once its wait has run out (`hold_minutes`): `TakeTurn` queues its conflict revision for the
// subscription that approved it, on the same model, with its lane kept for it.

import (
	"fmt"
	"slices"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	hstate "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
)

// MergeOwner is the lane that owns bot pull request `record`'s head `sha` until it merges (#2948):
// the subscription whose approval shipped it (`merge_owner`, the last approver), while the reviews'
// clearance is of that head; "" when none does (not cleared at it, or its merge owner gave it up
// once it waited for a person with its CI green, `sweep.PersonMerges`). Its fixes go to it first,
// ahead of its new work: a conflict's resolution (`Housekeeping`, `bot:resolving`) and a CI fix
// (`events.fixCI`, `bot:fixing-ci`), each made its owner's with `Own`.
func MergeOwner(record *pyjson.Object, sha string) string {
	owner := py.OrStr(record.Get("merge_owner"), "")
	if owner == "" || sha == "" || !pyjson.Equal(queue.Cleared(record).Get("sha"), sha) {
		return ""
	}
	return owner
}

// SyncTurns takes up every pull request waiting its turn whose turn came: each it waits behind has
// closed, or its wait ran out. One closed meanwhile, or no longer labelled `bot:waiting-turn` (a
// revision, a stop or a person took it out of that state), waits no more. `closed` is a pull request
// that has just closed (`events.OnPullClosed`), read as closed whatever GitHub lists yet; 0 for none.
// Returns what it did.
func SyncTurns(ctx *hctx.Context, closed int) []string {
	notes := []string{}
	state := ctx.Store.Load()
	for _, number := range queue.WaitingTurns(state) {
		waiting := queue.WaitingTurnOf(recordOf(state, strconv.Itoa(number)))
		var note string
		if exc := herr.Try(func() { note = syncTurn(ctx, number, waiting, closed) }); exc != nil {
			note = fmt.Sprintf("#%d: could not see whether its turn came (%s)", number, py.SliceTo(herr.Str(exc), 200))
		}
		if note != "" {
			notes = append(notes, note)
		}
	}
	return notes
}

// syncTurn is SyncTurns for one pull request.
func syncTurn(ctx *hctx.Context, number int, waiting *pyjson.Object, closed int) string {
	pull := ctx.GH.GetPull(number)
	names := queue.LabelNames(pull)
	if !eqStr(pull.Get("state"), "open") || !names[config.LabelWaitingTurn] {
		ctx.Store.Update(func(s *pyjson.Object) any {
			hstate.Item(s, number).Pop(queue.WaitingTurnKey)
			return nil
		}, fmt.Sprintf("turn #%d dropped", number))
		return fmt.Sprintf("#%d no longer waits its turn", number)
	}
	var open []int
	for _, ahead := range queue.Behind(waiting) {
		if ahead == closed {
			continue
		}
		var other *pyjson.Object
		if _, failed := herr.Catch[*herr.GitHubError](func() { other = ctx.GH.GetPull(ahead) }); failed {
			continue // gone: nothing to wait for
		}
		if eqStr(other.Get("state"), "open") {
			open = append(open, ahead)
		}
	}
	if len(open) == 0 {
		return TakeTurn(ctx, number, fmt.Sprintf("%s, which it waited behind, closed", refsOf(queue.Behind(waiting))))
	}
	if ends := clock.ParseISOValue(waiting.Get("ends")); !ends.IsZero() && !ctx.Now().Before(ends) {
		return TakeTurn(ctx, number, fmt.Sprintf("it waited %d minutes for %s, the most a pull request "+
			"waits its turn (`hold_minutes`), so it goes ahead anyway", ctx.Cfg.HoldMinutes, refsOf(open)))
	}
	return ""
}

// TakeTurn takes pull request `number`, waiting its turn, up again (`why`): a conflict revision
// (`source: conflict`, `bot:revise`, then `bot:resolving`) that merges `main` and resolves it, owned by
// the subscription that approved it, on the same model, its lane kept for it from now
// (`own_review_reserve_minutes`, `queue.TurnKey`). Its clearance carries across a resolution that
// changes only the conflicted files (`deliver.Carry`). A pull request handed off meanwhile (its owner
// is another, or none) is queued for whoever takes it, with no lane kept. Returns what it did.
func TakeTurn(ctx *hctx.Context, number int, why string) string {
	state := ctx.Store.Load()
	waiting := queue.WaitingTurnOf(recordOf(state, strconv.Itoa(number)))
	if waiting == nil {
		return ""
	}
	pull := ctx.GH.GetPull(number)
	names := queue.LabelNames(pull)
	provider, model := waiting.Str("provider"), waiting.Str("model")
	owned := queue.OwnerOf(state, number) == provider
	now := ctx.Now()
	until := now.Add(time.Duration(ctx.Cfg.OwnReviewReserveMinutes) * time.Minute)
	ctx.Store.Update(func(s *pyjson.Object) any {
		entry := hstate.Item(s, number)
		entry.Pop(queue.WaitingTurnKey)
		entry.Update(pyjson.New("kind", "revise", "source", "conflict", "queued_at", clock.ISO(now),
			"failures", 0, "stop_requested", false))
		if owned {
			Own(s, number, provider, "its turn came ("+why+"), and the lane that approved it resolves it", now)
			entry.Set(queue.TurnKey, pyjson.New("provider", provider, "model", model, "since", clock.ISO(now),
				"until", clock.ISO(until), "why", py.SliceTo(why, 300)))
		}
		return nil
	}, fmt.Sprintf("turn #%d", number))
	queue.SetStateLabel(ctx, number, names, config.LabelRevise)
	who := fmt.Sprintf("`%s` takes it up again on `%s`, the model that approved it, in a fresh session, "+
		"with its lane kept for it until %s", provider, model, Clock(ctx, until))
	if !owned {
		who = "It was handed off meanwhile, so the next subscription that may revise it takes it"
	}
	ctx.GH.CreateComment(number, fmt.Sprintf("▶️ **Its turn** (`%s` off): %s. %s: it merges `main`, "+
		"resolves the conflicts (Mergiraf and the resolvers first, then its builder), and once its reviewer "+
		"approves a resolution that changes only the conflicted files, it goes ready with auto-merge on "+
		"(`%s` while it runs).", config.LabelWaitingTurn, why, who, config.LabelResolving))
	return fmt.Sprintf("#%d: its turn came (%s)", number, why)
}

// refsOf is "#3", "#3 and #5", "#3, #5 and #7".
func refsOf(numbers []int) string {
	sorted := slices.Clone(numbers)
	slices.Sort(sorted)
	parts := make([]string, len(sorted))
	for i, n := range sorted {
		parts[i] = "#" + strconv.Itoa(n)
	}
	switch len(parts) {
	case 0:
		return "none"
	case 1:
		return parts[0]
	}
	return fmt.Sprintf("%s and %s", strings.Join(parts[:len(parts)-1], ", "), parts[len(parts)-1])
}
