package plan

import (
	"fmt"
	"slices"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/reviewrule"
)

// Assignment is one run's work: its subscription, and the seat each role runs on there.
// `action` is `plan` (a planning run of its own), `build`, `revise` or `review` (a review run).
type Assignment struct {
	Provider   *providers.Provider
	Action     string
	Difficulty string
	// A planning session before the build, in this run.
	Plan *providers.Seat // default: None
	// The builder (and its fixes).
	Build *providers.Seat // default: None
	// The reviewer: in a build or revise run, its own adversarial reviewer (None hands the change
	// to a review run); in a review run, the one reviewing.
	Review *providers.Seat // default: None
	// What the router decided that a reader should know: a step up in tier, and why.
	Notes []string // default: field(default_factory=list)
	// The runner label it takes (`runner_for`): "" is its subscription's own (`runs_on`).
	RunsOn string // default: ""
}

// SelfCheck is plan.Assignment.self_check.
func (x *Assignment) SelfCheck() bool {
	return x.Build != nil && x.Build.SelfCheck
}

// FillsIn is: it runs in another subscription's lane, on its fill-in runners
// (`Provider.fill_in`).
func (x *Assignment) FillsIn() bool {
	fill := x.Provider.FillIn
	return fill != nil && x.RunsOn == fill.RunsOn
}

// Describe is plan.Assignment.describe.
func (x *Assignment) Describe() string {
	var parts []string
	for _, role := range []struct {
		name string
		seat *providers.Seat
	}{{"plan", x.Plan}, {"build", x.Build}, {"review", x.Review}} {
		if role.seat != nil {
			parts = append(parts, role.name+" on "+role.seat.Describe())
		}
	}
	if x.Build != nil && x.Review == nil && x.Action != "review" {
		part := "review in a review run"
		if x.SelfCheck() {
			part += " after its self check"
		}
		parts = append(parts, part)
	}
	if len(parts) == 0 {
		return "on " + x.Provider.Describe()
	}
	return strings.Join(parts, "; ")
}

// seatDict is `seat.to_dict() if seat else None`.
func seatDict(seat *providers.Seat) any {
	if seat == nil {
		return nil
	}
	return seat.ToDict()
}

// Seats is what the model job runs each role on (`work.py`).
func (x *Assignment) Seats() *pyjson.Object {
	return pyjson.New("plan", seatDict(x.Plan), "build", seatDict(x.Build),
		"review", seatDict(x.Review), "self_check", x.SelfCheck())
}

// FreeProviders is plan.free_providers.
func FreeProviders(ctx *hctx.Context, state *pyjson.Object, lanes *Lanes, role string, forced bool, quietOK string) []*providers.Provider {
	return filter(ctx.Cfg.Pool.Ordered(), func(p *providers.Provider) bool {
		return Usable(ctx, state, p, lanes, role, forced, quietOK)
	})
}

// Balance is a subscription's place in the load balancer (`balance`): minus its room in steps,
// then the runs it holds. Lower goes first.
type Balance map[string][2]int

// Level is LEVEL: every subscription's place while the load balancer is off: all level, so the
// usage order decides.
var Level = [2]int{0, 0}

// place is `spread.get(provider_id, LEVEL)`.
func (b Balance) place(providerID string) [2]int {
	if at, ok := b[providerID]; ok {
		return at
	}
	return Level
}

// Balance_ is plan.balance: the load balancer (`balance` in providers.json): each
// subscription's key, so that among seats the router would otherwise order by the usage order
// alone, the one with the most room under its caps goes first (`providers.room`, in steps of
// `BALANCE_STEP`), then the one holding the fewest runs, and only then the usage order. Work
// spreads over a tier's subscriptions instead of spending the first in `priority` to its cap
// (claude-3 took 168 runs in a week to claude-5's 13) while the rest sit idle. Empty when it is
// off.
func Balance_(ctx *hctx.Context, state *pyjson.Object, lanes *Lanes) Balance {
	pool := ctx.Cfg.Pool
	if !pool.Balance {
		return Balance{}
	}
	at := ctx.Now()
	spread := Balance{}
	for _, provider := range pool.Ordered() {
		room := providers.Room(provider, providers.PeekRecord(state, provider.ID), at, ctx.Cfg.Timezone)
		held := lanes.Count(provider.ID) + lanes.PlanningBy(provider.ID)
		spread[provider.ID] = [2]int{-py.Round(room / providers.BalanceStep), held}
	}
	return spread
}

// Ranked is `seats` in the order the router tries them: the load balancer's (`spread`), then
// the usage order (`priority`), then the tier's own preference order.
func Ranked(pool *providers.Pool, seats []*providers.Seat, spread Balance /* = None */) []*providers.Seat {
	// {name: i for i, name in enumerate(pool.priority)}: a name given twice keeps its last place.
	order := map[string]int{}
	for i, name := range pool.Priority {
		order[name] = i
	}
	return py.SortedBy(seats, func(seat *providers.Seat) []any {
		place, ok := order[seat.Provider.ID]
		if !ok {
			place = len(order)
		}
		return []any{spread.place(seat.Provider.ID), place, seat.Rank}
	})
}

// seatKey is `(TIER_RANK[s.tier], s.rank)`.
func seatKey(seat *providers.Seat) []any { return []any{tierRank[seat.Tier], seat.Rank} }

// BuilderSeat is who builds an item of `difficulty`: the first free seat in the lane's tier
// order (#695: Muse before agy before GPT before Sonnet), on its weakest seat that meets
// the tier; with a note when that seat is above the tier, saying why (none of that tier is
// free, or the tier order puts this one first). Among subscriptions whose seat is the same
// model (the Claude accounts' Sonnet or Opus), the load balancer (`spread`) goes before the
// usage order. One lane, one owner (#725): a medium or
// hard item stays with the subscription that planned it (`avoid`) when that one can build
// it, so planner and builder are the same; an easy item still goes weak-first, since its
// planner (medium) must not spend its lane building what a weak seat builds. A revision
// that answers a final audit (`audit_tier`) stays at the auditor's tier or above, so the
// auditor fixes and pushes it.
//
// A fullsend `part` (#505), whose plan is on record by now, goes first to the subscriptions
// whose strongest seat is weakest, and one with no seat of its tier may build it on a seat of
// `PART_FLOOR`: Muse builds a hard part from Opus's plan, and the strong models keep to the
// planning and to the reconcile that checks and reviews every part.
func BuilderSeat(pool *providers.Pool, offered []*providers.Provider, difficulty string, part bool /* = False */, avoid string /* = "" */, auditTier string /* = "" */, spread Balance /* = None */) (*providers.Seat, string) {
	floor := config.MinTier[difficulty]
	if auditTier != "" && rankOr(auditTier, -1) > rankOr(floor, -1) {
		floor = auditTier
	}
	free := map[string]bool{}
	for _, provider := range offered {
		free[provider.ID] = true
	}

	builds := func(provider *providers.Provider, seat *providers.Seat) bool {
		// A stand-in (`takes_over`, a Sonnet seat for a weak one) builds only an easy item, and only
		// while the subscription it stands in for cannot take it (#317 part 9).
		if seat.TakesOver != "" && (difficulty != "easy" || free[seat.TakesOver]) {
			return false
		}
		if providers.TierAtLeast(seat.Tier, floor) {
			return true
		}
		return part && providers.TierAtLeast(seat.Tier, config.PartFloor) &&
			pool.BestSeat(provider, floor) == nil
	}
	buildingSeats := func(provider *providers.Provider) []*providers.Seat {
		return filter(pool.Seats(provider), func(seat *providers.Seat) bool { return builds(provider, seat) })
	}

	// Its weakest seat that may build this item, as (tier, tier-rank): the lane order.
	weakestKey := func(provider *providers.Provider) []any {
		seats := buildingSeats(provider)
		if len(seats) == 0 {
			return []any{len(tierRank), 0}
		}
		return seatKey(py.MinBy(seats, seatKey))
	}

	strongest := func(provider *providers.Provider) int {
		if seat := pool.BestSeat(provider, "weak"); seat != nil {
			return tierRank[seat.Tier]
		}
		return 0
	}
	// One lane, one owner (#725): a medium or hard item stays with the subscription that
	// planned it when that one can build it; an easy item still goes first to an `easy_first`
	// subscription (its medium planner must not spend its lane on weak work), and a
	// `build_last` one (claude-2) after the rest.
	ownerFirst := (difficulty == "medium" || difficulty == "hard") && !part && auditTier == "" &&
		slices.ContainsFunc(offered, func(p *providers.Provider) bool {
			return p.ID == avoid && !p.BuildLast && len(buildingSeats(p)) > 0
		})
	others := filter(offered, func(p *providers.Provider) bool { return p.ID != avoid })
	fallback := avoid != "" && difficulty == "easy" &&
		!slices.ContainsFunc(others, func(p *providers.Provider) bool { return len(buildingSeats(p)) > 0 })
	base := others
	if fallback || ownerFirst {
		base = offered
	}
	order := py.SortedBy(base, func(provider *providers.Provider) []any {
		strength := 0
		if part {
			strength = strongest(provider)
		}
		place := slices.Index(pool.Priority, provider.ID)
		if place < 0 {
			place = len(pool.Priority)
		}
		return []any{weakestKey(provider), !(provider.EasyFirst && difficulty == "easy"), strength,
			provider.BuildLast, spread.place(provider.ID), place}
	})
	if ownerFirst {
		owned := filter(order, func(p *providers.Provider) bool { return p.ID == avoid })
		order = append(owned, filter(order, func(p *providers.Provider) bool { return p.ID != avoid })...)
	}
	type candidateSeats struct {
		provider *providers.Provider
		seats    []*providers.Seat
	}
	usable := make([]candidateSeats, 0, len(order))
	for _, provider := range order {
		usable = append(usable, candidateSeats{provider, buildingSeats(provider)})
	}
	for index, entry := range usable {
		provider, seats := entry.provider, entry.seats
		if len(seats) == 0 {
			continue
		}
		seat := py.MinBy(seats, seatKey)
		prefix := ""
		if ownerFirst && provider.ID == avoid {
			prefix = fmt.Sprintf("its planner `%s` keeps it: one lane, one owner. ", avoid)
		} else if fallback && provider.ID == avoid {
			prefix = fmt.Sprintf("its planner `%s` also builds it: no other lane seat is free. ", avoid)
		}
		if !providers.TierAtLeast(seat.Tier, floor) {
			return seat, prefix + fmt.Sprintf("a fullsend part, built on %s from its plan: its "+
				"tree's reconcile checks and reviews it", seat.Tier)
		}
		if seat.Tier == floor {
			return seat, py.Rstrip(prefix)
		}
		var later []string
		for _, rest := range usable[index+1:] {
			for _, s := range rest.seats {
				if s.Tier == floor {
					later = append(later, s.Describe())
				}
			}
		}
		if len(later) > 0 {
			return seat, prefix + fmt.Sprintf("built on %s though difficulty:%s allows %s: the tier "+
				"order puts `%s` before %s", seat.Tier, difficulty, floor, provider.ID,
				strings.Join(later, ", "))
		}
		var names []string
		for _, entry := range pool.TierEntries(floor) {
			names = append(names, entry.Model)
		}
		models := strings.Join(names, ", ")
		if models == "" {
			models = "none listed"
		}
		return seat, prefix + fmt.Sprintf("stepped up from %s to %s to build it: no %s-tier model "+
			"(%s) is free now", floor, seat.Tier, floor, models)
	}
	return nil, ""
}

// LanePlanners is the planning lane's planners: the strongest seat of each subscription that
// may plan now (at least medium), the strong ones first, and within a tier the one holding the
// fewest plans first, then the usage order. A planning run takes none of its subscription's
// lanes, so a subscription plans while it builds, as many items at once as it has lanes, but it
// takes a runner: a slot on the machine (`machine_full`), or one of GitHub's lanes
// (`Lanes.free`, #1455). Planning comes first, so the strong
// models write the plans the medium ones build from before they build themselves. One the quiet
// check guards, or one with caps, plans only when it holds nothing else. Which item a planner
// may take is the item's plan floor (`queue.plan_floor`): a medium one plans only easy or
// unrated items. A plan is no short call (an Opus planner spent 20 minutes on #37), so like a
// build it starts only `start_headroom` under each cap.
func LanePlanners(ctx *hctx.Context, state *pyjson.Object, lanes *Lanes, forced bool, quietOK string) []*providers.Seat {
	if lanes.PlanFree() <= 0 {
		return nil
	}
	cfg := ctx.Cfg
	var seats []*providers.Seat
	for _, provider := range cfg.Pool.Ordered() {
		seat := cfg.Pool.BestSeat(provider, "medium")
		if seat == nil || !slices.Contains(provider.Roles, "plan") {
			continue
		}
		planning := lanes.PlanningBy(provider.ID)
		if planning >= provider.Lanes {
			continue
		}
		if (provider.QuietCheck || pyjson.Truthy(provider.Limits.Stops())) &&
			(planning != 0 || lanes.Count(provider.ID) != 0) {
			continue // one run at a time on a guarded or capped subscription (`Lanes.full`)
		}
		if MachineFull(cfg.Pool, provider, lanes) || FillInFull(cfg.Pool, provider, lanes) {
			continue
		}
		if !cfg.Pool.OnMachine(provider.ID) && lanes.Free() <= 0 {
			continue // GitHub's lanes are full, and a planning run there takes one (#1455)
		}
		if quietHolds(cfg, provider, forced, quietOK) {
			continue
		}
		if providers.Availability(provider, state, ctx.Now(), cfg.Timezone, cfg.Secrets,
			forced, true) == "" {
			seats = append(seats, seat)
		}
	}
	return py.SortedBy(Ranked(cfg.Pool, seats, Balance_(ctx, state, lanes)), func(seat *providers.Seat) []any {
		return []any{-tierRank[seat.Tier], lanes.PlanningBy(seat.Provider.ID)}
	})
}

// RunReviewer is a build or revise run's own adversarial reviewer: its strongest seat of at
// least medium, or None, and the change goes to a review run. (`difficulty` does not change it:
// a hard item's builder is strong, so its own reviewer is too.)
func RunReviewer(pool *providers.Pool, provider *providers.Provider, difficulty string) *providers.Seat {
	if !slices.Contains(provider.Roles, "review") {
		return nil
	}
	return pool.BestSeat(provider, "medium")
}

// ReviewSeat is a review run's reviewer: the lane's final audit (#725). A medium or hard item
// waits for a strong seat (Opus); an easy item takes the weakest free seat that ships it
// (medium, then strong). No weak review runs go out: weak builders review themselves in fresh
// sessions, and a weak approval never ships. Within a tier, a family that has not approved the
// head yet goes before one that has (the same model may review it twice), then the load
// balancer's order (`spread`, `ranked`). A stand-in seat (`takes_over`) reviews nothing.
func ReviewSeat(pool *providers.Pool, offered []*providers.Provider, candidate *queue.Candidate, spread Balance /* = None */) *providers.Seat {
	var seats []*providers.Seat
	for _, provider := range offered {
		seats = append(seats, pool.OwnSeats(provider)...)
	}
	// An easy item takes the weakest seat that ships it (medium first, saving the Opus gate);
	// a medium or hard one waits for strong (#725). A person asking `/harness review <tier>`
	// gets that tier first.
	tiers := []string{"strong"}
	if candidate.Difficulty == "easy" {
		tiers = []string{"medium", "strong"}
	}
	if (candidate.ReviewFloor == "medium" || candidate.ReviewFloor == "strong") &&
		!slices.Contains(tiers, candidate.ReviewFloor) {
		tiers = append([]string{candidate.ReviewFloor}, tiers...)
	}
	for _, tier := range tiers {
		if candidate.ReviewFloor != "" && !providers.TierAtLeast(tier, candidate.ReviewFloor) {
			continue // a person asked for this tier or stronger (`/harness review strong`)
		}
		if candidate.ReviewFloor == "" && !reviewrule.Helps(candidate.ApprovalTiers, tier, candidate.Difficulty) {
			continue // a person's explicit request reviews at any tier
		}
		found := Ranked(pool, filter(seats, func(seat *providers.Seat) bool { return seat.Tier == tier }), spread)
		found = py.SortedBy(found, func(seat *providers.Seat) []any { // stable: keeps the order
			return []any{slices.Contains(candidate.Approved, seat.Family())}
		})
		if len(found) > 0 {
			return found[0]
		}
	}
	return nil
}

// Assign is who takes `candidate` now, and on which seats, or None when nobody can.
func Assign(ctx *hctx.Context, state *pyjson.Object, candidate *queue.Candidate, lanes *Lanes, force bool, quietOK string) *Assignment {
	pool := ctx.Cfg.Pool
	forced := force || candidate.Forced

	free := func(role string) []*providers.Provider {
		return FreeProviders(ctx, state, lanes, role, forced, quietOK)
	}

	spread := Balance_(ctx, state, lanes)
	if candidate.Kind == "review" {
		seat := ReviewSeat(pool, free("review"), candidate, spread)
		if seat == nil {
			return nil
		}
		return &Assignment{Provider: seat.Provider, Action: "review", Difficulty: candidate.Difficulty,
			Review: seat, RunsOn: RunnerFor(ctx, state, seat.Provider, lanes, "review")}
	}
	builders := free(RoleOf[candidate.Kind])
	floor := queue.PlanFloor(candidate)
	if candidate.Kind == "build" && !queue.PlanMeets(candidate) {
		// It has no plan its difficulty may build from (#317 part 6): the planning lane writes one
		// (the Needs plan stage), or a builder whose own seat meets the plan floor plans it first
		// in its run. A builder that cannot plan (a weak one) waits for the plan.
		builders = filter(builders, func(p *providers.Provider) bool {
			return slices.Contains(p.Roles, "plan") && pool.BestSeat(p, floor) != nil
		})
	}
	if candidate.Kind == "revise" && (!candidate.BotPR || candidate.Conflict) {
		// A person's pull request never gets a review run: its reviewer must be in this run. A
		// conflict with `main` lands in SPEC §11 and the rulings index nearly every time, which a
		// weak builder resolves badly (#203, #214, #287): only a builder with a real reviewer
		// in its own run takes one (a weak builder's findings-only self review does not count,
		// #317 part 2), and none at all may wait for one.
		builders = filter(builders, func(p *providers.Provider) bool {
			return RunReviewer(pool, p, candidate.Difficulty) != nil
		})
	}
	avoid, auditTier := "", ""
	if candidate.Kind == "build" {
		avoid = candidate.PlannedBy
	}
	if candidate.Kind == "revise" {
		auditTier = candidate.AuditTier
	}
	builder, note := BuilderSeat(pool, builders, candidate.Difficulty,
		candidate.Part && queue.PlanMeets(candidate), avoid, auditTier, spread)
	if builder == nil {
		return nil
	}
	var notes []string
	if note != "" {
		notes = []string{fmt.Sprintf("#%d: %s", candidate.Number, note)}
	}
	reviewer := RunReviewer(pool, builder.Provider, candidate.Difficulty)
	runsOn := RunnerFor(ctx, state, builder.Provider, lanes, RoleOf[candidate.Kind])
	if runsOn != "" && runsOn != builder.Provider.RunsOn {
		owners := make([]string, 0, len(builder.Provider.FillIn.Of))
		for _, id := range builder.Provider.FillIn.Of {
			owners = append(owners, "`"+id+"`")
		}
		notes = append(notes, fmt.Sprintf("#%d: `%s` fills a lane of %s, which cannot start work "+
			"now, on `%s`", candidate.Number, builder.Provider.ID, strings.Join(owners, ", "), runsOn))
	}
	if candidate.Kind == "build" && !queue.PlanMeets(candidate) {
		// No planner was free on the planning lane: the builder plans it first in its own run,
		// on its strongest model, which meets the item's plan floor.
		var own *providers.Seat
		if slices.Contains(builder.Provider.Roles, "plan") {
			own = pool.BestSeat(builder.Provider, floor)
		}
		if own == nil {
			return nil
		}
		return &Assignment{Provider: builder.Provider, Action: "build", Difficulty: candidate.Difficulty,
			Plan: own, Build: builder, Review: reviewer, Notes: notes, RunsOn: runsOn}
	}
	return &Assignment{Provider: builder.Provider, Action: candidate.Kind, Difficulty: candidate.Difficulty,
		Build: builder, Review: reviewer, Notes: notes, RunsOn: runsOn}
}

// Pair is one item `pairs` yields, with who takes it on which seats.
type Pair struct {
	Candidate  *queue.Candidate
	Assignment *Assignment
}

// Pairs is every queued item that a subscription can take now, with who takes it on which
// seats, most urgent first. (`skipped` is the queue's: the items labelled `human`.)
//
// The Python is a generator, but one that works out every pair before it yields the first, so
// a slice is the same.
func Pairs(ctx *hctx.Context, state *pyjson.Object, q []*queue.Candidate, lanes *Lanes, force bool, quietOK string, skipped []string /* = None */) []Pair {
	// Reviews and revisions finish work already begun, so they go before new builds; among the
	// builds the harder first, since only the stronger models can take them.
	// A review run unblocks a merge and is short, so it goes before every build or revision,
	// whatever their priority (#317 part 9); then the priority tier and the usual order.
	order := py.SortedBy(q, func(c *queue.Candidate) []any {
		return []any{!(force || c.Forced), c.Kind != "review", c.Priority, queue.KindOrder[c.Kind],
			-difficultyIndex(c.Difficulty), c.QueuedAt, c.Number}
	})
	// The Needs plan stage first: a planner on the planning lane, which takes no build lane,
	// plans the items a builder that cannot plan (a weak one) waits on before the rest, then the
	// fullsend parts a medium subscription (Muse) builds from their plans, each by a model that
	// meets its plan floor (medium for an easy, medium or unrated item, strong for a hard one).
	var planning []Pair
	planners := LanePlanners(ctx, state, lanes, force, quietOK)
	if len(planners) > 0 {
		needing := py.SortedBy(filter(order, queue.NeedsPlan), func(c *queue.Candidate) []any {
			return []any{!(force || c.Forced), c.Difficulty != "easy", !c.Part}
		})
		for _, candidate := range needing {
			// The lane's weakest planner that meets its floor (#695): a medium seat plans an
			// easy or medium item, a strong one a hard item.
			fitting := py.SortedBy(filter(planners, func(s *providers.Seat) bool {
				return providers.TierAtLeast(s.Tier, queue.PlanFloor(candidate))
			}), func(s *providers.Seat) []any {
				return []any{tierRank[s.Tier], s.Rank, lanes.PlanningBy(s.Provider.ID)}
			})
			if len(fitting) == 0 {
				continue
			}
			seat := fitting[0]
			what := "writes it"
			if !candidate.Rated {
				what = "rates and plans it"
			}
			planning = append(planning, Pair{candidate, &Assignment{
				Provider: seat.Provider, Action: "plan", Difficulty: candidate.Difficulty, Plan: seat,
				Notes: []string{fmt.Sprintf("#%d: needs a plan; %s %s on the planning lane",
					candidate.Number, seat.Describe(), what)}}})
		}
	}
	var found []Pair
	if lanes.Free() > 0 || lanes.MachineFree() > 0 {
		for _, candidate := range order {
			if assignment := Assign(ctx, state, candidate, lanes, force, quietOK); assignment != nil {
				found = append(found, Pair{candidate, assignment})
			}
		}
	}
	planning = py.SortedBy(planning, func(pair Pair) []any { return []any{!(force || pair.Candidate.Forced)} })
	return append(planning, found...)
}

// SurveyProvider is plan.survey_provider.
func SurveyProvider(ctx *hctx.Context, state *pyjson.Object, lanes *Lanes, force bool, quietOK string) *providers.Provider {
	for _, provider := range ctx.Cfg.Pool.Ordered() {
		if Usable(ctx, state, provider, lanes, "suggest", force, quietOK) {
			return provider
		}
	}
	return nil
}

// MachineFull is: `provider` runs on the bot's machine (the night box), and its
// `machine_parallel` slots are all held. GitHub's runners have `max_parallel` of their own. (The
// AI's training lanes run on a box of their own, outside the harness, so no slot is kept for
// them.)
func MachineFull(pool *providers.Pool, provider *providers.Provider, lanes *Lanes) bool {
	if !pool.OnMachine(provider.ID) {
		return false
	}
	held := 0
	for _, heldID := range lanes.Held {
		if pool.OnMachine(heldID) {
			held++
		}
	}
	return held >= pool.MachineParallel
}

// WhyNone is why each subscription cannot take work now, for the run's summary and `status`.
func WhyNone(ctx *hctx.Context, state *pyjson.Object, lanes *Lanes) string {
	cfg := ctx.Cfg
	var parts []string
	for _, provider := range cfg.Pool.Ordered() {
		if lanes.Full(provider) {
			parts = append(parts, fmt.Sprintf("`%s` is busy", provider.ID))
			continue
		}
		// What an idle subscription waits for is a build, a revision or a plan, which start only
		// `start_headroom` under each cap (`_usable`, `lane_planners`).
		reason := providers.StartReason(provider, state, ctx.Now(), cfg.Timezone, cfg.Secrets)
		if reason == "" && MachineFull(cfg.Pool, provider, lanes) {
			reason = fmt.Sprintf("waits for room on the machine (%d at once)", cfg.Pool.MachineParallel)
		}
		if reason == "" && FillInFull(cfg.Pool, provider, lanes) {
			reason = fmt.Sprintf("waits for its lane on its box, which `%s` fills now",
				cfg.Pool.FillerOf(provider.ID).ID)
		}
		if reason == "" && provider.QuietCheck && cfg.Quiet.Enabled {
			reason = "waits for its owner to be quiet"
		}
		if reason == "" {
			reason = "is idle, but nothing queued is for it"
		}
		parts = append(parts, fmt.Sprintf("`%s` %s", provider.ID, reason))
	}
	return strings.Join(parts, "; ")
}

// queueOf is plan._queue. `skipped` is extended with the items the queue passes over, when not
// nil.
func queueOf(ctx *hctx.Context, state *pyjson.Object, item *int, mode string, skipped *[]string) []*queue.Candidate {
	q := queue.Candidates(ctx, state, skipped, nil)
	if item != nil {
		number := *item
		q = filter(q, func(c *queue.Candidate) bool { return c.Number == number })
	}
	if mode == "build" || mode == "revise" || mode == "review" {
		q = filter(q, func(c *queue.Candidate) bool { return c.Kind == mode })
	}
	return q
}

// QuietWaiting is true when another `bot-night` run is waiting for the shared subscription to
// be quiet, so this one should not wait too (each waiting gate reads the usage every ten
// minutes).
func QuietWaiting(ctx *hctx.Context) bool {
	waiting := false
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		for _, run := range ctx.GH.ListRuns(config.NightWorkflow, "in_progress", 10, false) {
			if py.Str(run.Get("id")) == ctx.Cfg.RunID {
				continue
			}
			for _, job := range ctx.GH.ListJobs(run.Get("id"), false) {
				for _, step := range pyjson.Objs(job.Get("steps")) {
					if eqStr(step.Get("name"), QuietStep) && eqStr(step.Get("status"), "in_progress") {
						waiting = true
						return
					}
				}
			}
		}
	}); failed {
		return false
	}
	return waiting
}
