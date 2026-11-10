package plan

import (
	"slices"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// SpareLanes is one subscription's share of the excess compute (#3659): how many of its lanes are
// spare, and whether they may take a hard issue (a strong seat: Opus) or only a medium one.
type SpareLanes struct {
	Provider *providers.Provider
	Lanes    int
	Hard     bool
}

// Spare is the excess compute (#3659): the lanes free on each subscription that may start a build
// or a plan now, where a runner has room for them. `lanes` are the lanes once the planner has
// handed out every queued item it can start (`waiting.Explain`'s, or the planner's own when the
// queue holds nothing it can start), so what is left free is what nothing queued is for.
//
// A subscription counts only while it builds (`roles`) on a medium or strong seat and may start a
// build or a plan (`providers.Availability`, starting): one inside its `start_headroom` of a cap (5%
// in `.harness/providers.json`), off, suspended, outside its hours or backing off counts none. The
// lanes a fill-in shares with the subscriptions it fills in for (`Provider.fill_in`: Muse in gpt's
// and agy's on the night box) count once: each free one goes to the subscription it belongs to while
// that one may start work, and to the fill-in otherwise (`fillInOpen`). The room is shared out where
// the subscriptions run, GitHub's runners (`Lanes.Free`) and the AWS boxes (`Lanes.MachineFree`), in
// the pool's order, so the total never passes either. A subscription with nothing spare is left out.
// `forced` (a run a person forced, `/harness start --force`) counts one outside its hours or backing
// off too, as `Availability` does.
func Spare(ctx *hctx.Context, state *pyjson.Object, lanes *Lanes, forced bool) []SpareLanes {
	cfg := ctx.Cfg
	pool := cfg.Pool
	hosted, machine := lanes.Free(), lanes.MachineFree()
	sharedFree := map[string]int{} // a fill-in's id -> the lanes it shares that no run holds now
	for _, p := range pool.Ordered() {
		if p.FillIn != nil {
			owners, held := shared(pool, p, lanes)
			sharedFree[p.ID] = max(0, sumLanes(owners)-held)
		}
	}
	var found []SpareLanes
	for _, p := range pool.Ordered() {
		best := pool.BestSeat(p, "medium") // a weak seat takes no work since #2748
		if best == nil || !slices.Contains(p.Roles, "build") ||
			providers.Availability(p, state, ctx.Now(), cfg.Timezone, cfg.Secrets, forced, true) != "" {
			continue
		}
		room := &hosted
		if lanes.Machine[p.ID] {
			room = &machine
		}
		n := min(max(0, lanes.LanesOf(p)-lanes.Used(p)), *room)
		if filler := pool.FillerOf(p.ID); filler != nil {
			// Its lane is one its fill-in shares: free only while no run of either holds it.
			n = min(n, sharedFree[filler.ID])
			sharedFree[filler.ID] -= n
		}
		*room -= n
		if p.FillIn != nil {
			fill := min(fillInOpen(ctx, state, p, lanes), sharedFree[p.ID], *room)
			sharedFree[p.ID] -= fill
			*room -= fill
			n += fill
		}
		if n <= 0 {
			continue
		}
		found = append(found, SpareLanes{Provider: p, Lanes: n,
			Hard: providers.TierAtLeast(best.Tier, "strong")})
	}
	return found
}

// SpareTotal is how many lanes `spare` holds: the excess compute, in lanes.
func SpareTotal(spare []SpareLanes) int {
	total := 0
	for _, s := range spare {
		total += s.Lanes
	}
	return total
}
