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
	// Why the item leaves its owner for this subscription, when it does (`Owning`'s `Gone`): the
	// claim records it as the new owner's `owner_why` (#2748).
	OwnerWhy string // default: ""
	// A review run that is the change's own review (#2748, 2026-11-02): on the subscription whose
	// run built it, the model that built it reviews it (`Review`) and fixes what it finds (`Build`),
	// in the same run (#2903).
	OwnReview bool // default: False
	// A planning run in a lane of its own subscription, not the planning lane (#2903, `InLanePlanners`):
	// its delivery keeps the lane for the item's build on the same model (`queue.BuildNextKey`).
	InLane bool // default: False
}

// LaneAction is what the run counts as in the lanes (`Lanes.WithRun`): a planning run in a lane of
// its own subscription (`InLane`) holds that lane as a build does; any other run is its action.
func (x *Assignment) LaneAction() string {
	if x.Action == "plan" && x.InLane {
		return "build"
	}
	return x.Action
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
// usage order. One lane, one owner (#725, #2748): an item stays with the subscription that owns
// it (`avoid`: the one that planned it or took it up last, `queue.OwnerOf`) when that one can
// build it. A revision that answers a final audit (`audit_tier`) is the auditor's own fix: the
// audit made the auditor's subscription its owner (`deliver.SecondReview`, #2748 section 4), which
// fixes it on its seat of the audit's tier. Handed off (its owner cannot take it, and no hold keeps
// it, `Owning`), it goes to the item's own tier or above, as any owner's work does.
//
// A fullsend `part` (#505), whose plan is on record by now, goes first to the subscriptions
// whose strongest seat is weakest, and one with no seat of its tier may build it on a seat of
// `PART_FLOOR`: Muse builds a hard part from Opus's plan, and the strong models keep to the
// planning and to the reconcile that checks and reviews every part.
func BuilderSeat(pool *providers.Pool, offered []*providers.Provider, difficulty string, part bool /* = False */, avoid string /* = "" */, auditTier string /* = "" */, spread Balance /* = None */) (*providers.Seat, string) {
	floor := config.MinTier[config.AsDifficulty(difficulty)]
	// The owner's own floor: an audit's tier, when it answers an audit that tier gave (#2748).
	ownFloor := floor
	if auditTier != "" && rankOr(auditTier, -1) > rankOr(floor, -1) {
		ownFloor = auditTier
	}
	floorOf := func(provider *providers.Provider) string {
		if avoid != "" && provider.ID == avoid {
			return ownFloor
		}
		return floor
	}
	builds := func(provider *providers.Provider, seat *providers.Seat) bool {
		if providers.TierAtLeast(seat.Tier, floorOf(provider)) {
			return true
		}
		return part && providers.TierAtLeast(seat.Tier, config.PartFloor) &&
			pool.BestSeat(provider, floorOf(provider)) == nil
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
	// One lane, one owner (#725, #2748): an item stays with the subscription that owns it when
	// that one can build it (an auditor its fix at its audit's tier), and a `build_last` one
	// (claude-2) goes after the rest.
	ownerFirst := !part &&
		slices.ContainsFunc(offered, func(p *providers.Provider) bool {
			return p.ID == avoid && !p.BuildLast && len(buildingSeats(p)) > 0
		})
	base := filter(offered, func(p *providers.Provider) bool { return p.ID != avoid })
	if ownerFirst {
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
		return []any{weakestKey(provider), strength, provider.BuildLast, spread.place(provider.ID), place}
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
			prefix = fmt.Sprintf("its owner `%s` keeps it: one lane, one owner. ", avoid)
		}
		if !providers.TierAtLeast(seat.Tier, floorOf(provider)) {
			return seat, prefix + fmt.Sprintf("a fullsend part, built on %s from its plan: its "+
				"tree's reconcile checks and reviews it", seat.Tier)
		}
		if seat.Tier == floorOf(provider) {
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

// LanePlanners is the planning lane's planners: every seat of at least medium of each subscription
// that may plan now (a Claude account offers its Sonnet and its Opus), the strong ones first, and
// within a tier the one holding the fewest plans first, then the load balancer's and the usage
// order. `Pairs` gives each item the weakest that meets its plan floor (`queue.plan_floor`), so a
// medium or unrated item goes to a medium planner first (Muse, agy, GPT, Sonnet, in the tier
// order; #2748) and to a strong one only when none can take it now, and a hard item to a strong
// one only. A planning run takes none of its subscription's lanes, so a subscription plans while
// it builds, as many items at once as the lanes in force for it (`Lanes.lanes_of`), but it takes
// a runner: a slot on the machine (`machine_full`), or one of GitHub's lanes (`Lanes.free`,
// #1455). One the quiet check guards, or one with caps, counts its planning runs against its own
// lanes (`Lanes.used`), as its builds do: with one lane in force (a Claude account at 50% or
// under, `lanes_by_cap`) it plans only while it holds nothing else, and Muse plans while a lane of
// its four is free. A plan is no short call (an Opus planner spent 20 minutes on #37), so like a build
// it starts only `start_headroom` under each cap.
func LanePlanners(ctx *hctx.Context, state *pyjson.Object, lanes *Lanes, forced bool, quietOK string) []*providers.Seat {
	if lanes.PlanFree() <= 0 {
		return nil
	}
	cfg := ctx.Cfg
	var seats []*providers.Seat
	for _, provider := range cfg.Pool.Ordered() {
		own := filter(cfg.Pool.Seats(provider), func(seat *providers.Seat) bool {
			return providers.TierAtLeast(seat.Tier, "medium")
		})
		if len(own) == 0 || !slices.Contains(provider.Roles, "plan") {
			continue
		}
		planning := lanes.PlanningBy(provider.ID)
		if planning >= lanes.LanesOf(provider) {
			continue
		}
		if (provider.QuietCheck || pyjson.Truthy(provider.Limits.Stops())) && lanes.Full(provider) {
			continue // a guarded or capped subscription: its plans take its own lanes (`Lanes.used`)
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
			seats = append(seats, own...)
		}
	}
	return py.SortedBy(Ranked(cfg.Pool, seats, Balance_(ctx, state, lanes)), func(seat *providers.Seat) []any {
		return []any{-tierRank[seat.Tier], lanes.PlanningBy(seat.Provider.ID)}
	})
}

// InLanePlanners is the planners that may plan an item in a lane of their own subscription (#2903,
// the owner's decision of 2026-11-03), so that its build, and then its own review run, follow in
// that lane on the same model: every seat of at least medium of each subscription that plans and
// builds, may start a build now (`Usable`: its usage under each cap less `start_headroom`, its
// hours, its quiet check, a runner where it runs) and has a lane of its own free (`Lanes.Full`; not
// a fill-in lane), in the load balancer's and the usage order. Not a `build_last` subscription
// (claude-2), which builds only after every other: it plans on the planning lane, as before. None
// unless the night bot chains its runs (`own_review_run`) and keeps a planning lane (`plan_lanes`):
// with that off a builder plans in its own run, as before.
func InLanePlanners(ctx *hctx.Context, state *pyjson.Object, lanes *Lanes, forced bool, quietOK string) []*providers.Seat {
	cfg := ctx.Cfg
	if !cfg.OwnReviewRun || lanes.PlanLimit <= 0 {
		return nil
	}
	var seats []*providers.Seat
	for _, provider := range cfg.Pool.Ordered() {
		if provider.BuildLast || !slices.Contains(provider.Roles, "plan") || lanes.Full(provider) ||
			!Usable(ctx, state, provider, lanes, "build", forced, quietOK) {
			continue
		}
		seats = append(seats, filter(cfg.Pool.Seats(provider), func(seat *providers.Seat) bool {
			return providers.TierAtLeast(seat.Tier, "medium")
		})...)
	}
	return Ranked(cfg.Pool, seats, Balance_(ctx, state, lanes))
}

// PlanChains is: an item planned now may be planned in its planner's own lane and built there next
// on the same model (#2903): a plain build of the night bot's (`own_review_run`). Not a fullsend
// part (Muse builds those from a strong plan, `PartFloor`), nor a split or a fullsend tree, whose
// mode plans for itself, nor an item planned in its description (`plan_in_issue`), which needs no
// planning run: those keep their routing.
func PlanChains(cfg *config.Config, c *queue.Candidate) bool {
	return cfg.OwnReviewRun && c.Kind == "build" && c.Mode == "" && !c.Part
}

// inLanePlan is the planning run `c` gets in a lane of its own subscription (#2903), or nil: the
// weakest of `seats` (`InLanePlanners`) that meets both its plan floor and the tier that builds it,
// since that model builds it next, in the tier's own order (Muse before agy before Codex before
// Sonnet); its owner's first when it has one (a plan it was cut off in goes on there).
func inLanePlan(c *queue.Candidate, seats []*providers.Seat, owner string) *Assignment {
	build := config.MinTier[config.AsDifficulty(c.Difficulty)]
	fitting := py.SortedBy(filter(seats, func(s *providers.Seat) bool {
		return providers.TierAtLeast(s.Tier, queue.PlanFloor(c)) && providers.TierAtLeast(s.Tier, build)
	}), func(s *providers.Seat) []any { return []any{tierRank[s.Tier], s.Rank} })
	if len(fitting) == 0 {
		return nil
	}
	seat := fitting[0]
	if own := filter(fitting, func(s *providers.Seat) bool { return s.Provider.ID == owner }); owner != "" && len(own) > 0 {
		seat = own[0]
	}
	what := "writes it"
	if !c.Rated {
		what = "rates and plans it"
	}
	return &Assignment{Provider: seat.Provider, Action: "plan", Difficulty: c.Difficulty, Plan: seat, InLane: true,
		Notes: []string{fmt.Sprintf("#%d: needs a plan; %s %s in a lane of `%s`, which its build then takes "+
			"on the same model", c.Number, seat.Describe(), what, seat.Provider.ID)}}
}

// plannersSeat is the seat a build of `c` runs on when its planner's subscription builds it (#2903):
// the model whose planning run wrote its plan (`queue.PlanModelKey`), on `builder`'s subscription,
// when that is the subscription that planned it and owns it and the model may build it; nil
// otherwise (a handed-off item, a plan from before, a model it no longer runs), and the build keeps
// `builder`. Only where `PlanChains` holds.
func plannersSeat(cfg *config.Config, c *queue.Candidate, builder *providers.Seat) *providers.Seat {
	if !PlanChains(cfg, c) || c.PlanModel == "" || c.PlannedBy == "" || builder.Provider.ID != c.PlannedBy ||
		c.Owner != c.PlannedBy || !queue.PlanMeets(c) {
		return nil
	}
	seat := cfg.Pool.Seat(c.PlannedBy, c.PlanModel)
	if seat == nil || !providers.TierAtLeast(seat.Tier, config.MinTier[config.AsDifficulty(c.Difficulty)]) {
		return nil
	}
	return seat
}

// turnSeat is the seat a pull request whose turn came is resolved on (#2948): the model that approved
// it (`queue.TurnOf`'s `model`), on `builder`'s subscription when that is its owner, and only at the
// item's tier; nil otherwise.
func turnSeat(cfg *config.Config, c *queue.Candidate, builder *providers.Seat) *providers.Seat {
	if c.Kind != "revise" || !c.TurnNext || c.TurnModel == "" || builder.Provider.ID != c.Owner {
		return nil
	}
	seat := cfg.Pool.Seat(c.Owner, c.TurnModel)
	if seat == nil || !providers.TierAtLeast(seat.Tier, config.MinTier[config.AsDifficulty(c.Difficulty)]) {
		return nil
	}
	return seat
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

// ReviewSeat is a review run's reviewer: the lane's final audit (#725), which an item gets only
// when its own run's reviewer did not approve it. It tries the tiers whose approval ships the head
// (`review_rule.tiers`), weakest first: a medium item takes a medium seat first (Muse, agy, GPT,
// Sonnet), then a strong one; a hard item waits for a strong seat (Opus). No weak review runs go
// out: a weak approval never ships. Any subscription's approval ships the head (2026-11-02,
// #2748), so the one whose run built it (`candidate.built_by`) may review it, but within a tier
// every other subscription goes first; then a family that has not approved the head yet before
// one that has (the same model may review it twice), then the tier's own order (Muse before agy
// before GPT before Sonnet, as builds go), then the load balancer's order (`spread`, `ranked`).
// An after-merge spot-check (`candidate.spot_check`, its tiers strong alone) never goes to the
// builder's subscription, and puts the one whose approval shipped the head (`approved_by`) after
// every other.
func ReviewSeat(pool *providers.Pool, offered []*providers.Provider, candidate *queue.Candidate, spread Balance /* = None */) *providers.Seat {
	var seats []*providers.Seat
	for _, provider := range offered {
		if candidate.SpotCheck && candidate.BuiltBy != "" && provider.ID == candidate.BuiltBy {
			continue // a spot-check measures its builder's lane from outside it
		}
		seats = append(seats, pool.Seats(provider)...)
	}
	tiers := slices.Clone(candidate.ReviewTiers)
	if len(tiers) == 0 {
		tiers = reviewrule.Shipping(config.AsDifficulty(candidate.Difficulty))
	}
	// A person asking `/harness review <tier>` gets that tier first.
	if (candidate.ReviewFloor == "medium" || candidate.ReviewFloor == "strong") &&
		!slices.Contains(tiers, candidate.ReviewFloor) {
		tiers = append([]string{candidate.ReviewFloor}, tiers...)
	}
	for _, tier := range tiers {
		if candidate.ReviewFloor != "" && !providers.TierAtLeast(tier, candidate.ReviewFloor) {
			continue // a person asked for this tier or stronger (`/harness review strong`)
		}
		found := Ranked(pool, filter(seats, func(seat *providers.Seat) bool { return seat.Tier == tier }), spread)
		found = py.SortedBy(found, func(seat *providers.Seat) []any { // stable: keeps the order
			own := candidate.BuiltBy != "" && seat.Provider.ID == candidate.BuiltBy
			approver := candidate.ApprovedBy != "" && seat.Provider.ID == candidate.ApprovedBy
			return []any{own, approver, slices.Contains(candidate.Approved, seat.Family()), seat.Rank}
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

	if Reserves(candidate) && !candidate.OwnReview {
		// The lane its planning run kept for it (#2903), or the lane kept for its turn (#2948).
		lanes = lanes.Without(candidate.Number)
	}
	spread := Balance_(ctx, state, lanes)
	if candidate.Kind == "review" && candidate.OwnReview {
		return ownReviewer(ctx, state, candidate, lanes.Without(candidate.Number), forced, quietOK)
	}
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
		// in its run. A builder that cannot plan waits for the plan.
		builders = filter(builders, func(p *providers.Provider) bool {
			return slices.Contains(p.Roles, "plan") && pool.BestSeat(p, floor) != nil
		})
	}
	if candidate.Kind == "revise" && (!candidate.BotPR || candidate.Conflict) {
		// A person's pull request never gets a review run: its reviewer must be in this run. A
		// conflict with `main` lands in SPEC §11 and the rulings index nearly every time, which a
		// weak builder resolved badly (#203, #214, #287): only a builder with a real reviewer
		// in its own run takes one (a findings-only self check does not count, #317 part 2), and
		// none at all may wait for one.
		builders = filter(builders, func(p *providers.Provider) bool {
			return RunReviewer(pool, p, candidate.Difficulty) != nil
		})
	}
	// Its owner first (#2748): a build's or a revision's; a fullsend part keeps its planner's, as
	// before owners.
	avoid, auditTier := "", ""
	if candidate.Kind == "build" && candidate.Part {
		avoid = candidate.PlannedBy
	} else if candidate.Kind == "build" || candidate.Kind == "revise" {
		avoid = candidate.Owner
	}
	if candidate.Kind == "revise" {
		auditTier = candidate.AuditTier
	}
	var builder *providers.Seat
	var note string
	if candidate.Kind == "build" && candidate.SentBack && candidate.Owner != "" {
		// A part its tree's reconcile sent back goes to the subscription that built it first, its
		// owner (#2748, section 2): Muse rebuilds what Muse built.
		own := filter(builders, func(p *providers.Provider) bool { return p.ID == candidate.Owner })
		if builder, note = BuilderSeat(pool, own, candidate.Difficulty, queue.PlanMeets(candidate), "", "",
			spread); builder != nil {
			note = py.Strip(fmt.Sprintf("its builder `%s` builds it again: its tree's reconcile sent it "+
				"back. %s", candidate.Owner, note))
		}
	}
	if builder == nil {
		builder, note = BuilderSeat(pool, builders, candidate.Difficulty,
			candidate.Part && queue.PlanMeets(candidate), avoid, auditTier, spread)
	}
	if builder == nil {
		return nil
	}
	if seat := plannersSeat(ctx.Cfg, candidate, builder); seat != nil {
		// One model from plan to own review (#2903): the model that planned it builds it.
		note = py.Strip(fmt.Sprintf("%s its planner builds it on %s, the model that planned it: one lane, one "+
			"model.", note, seat.Describe()))
		builder = seat
	} else if seat := turnSeat(ctx.Cfg, candidate, builder); seat != nil {
		// Its turn came (#2948): the model that approved it merges `main` and resolves it.
		note = py.Strip(fmt.Sprintf("%s its turn came: %s, the model that approved it, merges `main` and "+
			"resolves it.", note, seat.Describe()))
		builder = seat
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

// ownReviewer is who takes `candidate`'s own review run (#2748, 2026-11-02): the subscription whose
// run built the change (`candidate.Owner`), when it may start a run as long as a revision now
// (`RoleFor`), in the lane its build reserved for it (`lanes` has let that go). The model that built
// the head (`candidate.BuiltOn`, its reservation's) reviews it in fresh sessions and fixes what the
// review finds (#2903, the owner's decision of 2026-11-03: one model, not the subscription's
// strongest seat): Sonnet what Sonnet built, Opus a hard item. A reservation from before names no
// model, and its weakest seat at the item's tier does both, as its builds go. nil when it cannot
// (`Owning` then holds it for it or hands it off to a final audit).
func ownReviewer(ctx *hctx.Context, state *pyjson.Object, candidate *queue.Candidate, lanes *Lanes, forced bool, quietOK string) *Assignment {
	pool := ctx.Cfg.Pool
	provider := pool.Get(candidate.Owner)
	if provider == nil || cannotTake(pool, provider, candidate) != "" ||
		!Usable(ctx, state, provider, lanes, RoleFor(candidate), forced, quietOK) {
		return nil
	}
	floor := config.MinTier[config.AsDifficulty(candidate.Difficulty)]
	seat := pool.Seat(provider.ID, candidate.BuiltOn)
	if candidate.BuiltOn == "" || seat == nil || !providers.TierAtLeast(seat.Tier, floor) {
		seat, _ = BuilderSeat(pool, []*providers.Provider{provider}, candidate.Difficulty, false, "", "", nil)
	}
	if seat == nil {
		return nil
	}
	reviewer, builder := seat, seat
	notes := []string{fmt.Sprintf("#%d: its own review run: `%s`, whose run built it, reviews it and fixes "+
		"what it finds on %s, the model that built it", candidate.Number, provider.ID, seat.Describe())}
	return &Assignment{Provider: provider, Action: "review", Difficulty: candidate.Difficulty,
		Build: builder, Review: reviewer, Notes: notes, OwnReview: true,
		RunsOn: RunnerFor(ctx, state, provider, lanes, RoleFor(candidate))}
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
	// whatever their priority (#317 part 9); then the priority tier and the usual order. An
	// after-merge spot-check unblocks nothing, so it goes after every other review run, and still
	// before the builds (#2748, section 5).
	order := py.SortedBy(q, func(c *queue.Candidate) []any {
		return []any{!(force || c.Forced), c.Kind != "review", c.SpotCheck, c.Priority,
			queue.KindOrder[c.Kind], -difficultyIndex(c.Difficulty), c.QueuedAt, c.Number}
	})
	// Who owns each item, and whether it waits for its owner (#2748, `Owning`): an item held for
	// its owner goes to nobody else, and one that carries `bot:waiting` to its owner alone, until
	// the hold is lifted or released (`SyncHolds`, which the plan runs first).
	owning := map[int]*Ownership{}
	for _, c := range order {
		owning[c.Number] = Owning(ctx, state, c, lanes, force, quietOK)
	}
	order = filter(order, func(c *queue.Candidate) bool {
		own := owning[c.Number]
		return own.Hold == nil && !(c.Waiting && !own.Free)
	})
	elsewhere := func(c *queue.Candidate, a *Assignment) bool {
		return c.Waiting && a.Provider.ID != owning[c.Number].Owner
	}
	// An item that will be built is planned in a lane of its planner's own subscription when one
	// has room (#2903), so its build and its own review run follow there on the same model: those
	// plans take a lane as builds do, and go in the builds' order below. The rest go to the planning
	// lane.
	inLane := map[int]*Assignment{}
	if seats := InLanePlanners(ctx, state, lanes, force, quietOK); len(seats) > 0 {
		for _, candidate := range order {
			if !queue.NeedsPlan(candidate) || !PlanChains(ctx.Cfg, candidate) {
				continue
			}
			if a := inLanePlan(candidate, seats, owning[candidate.Number].Owner); a != nil && !elsewhere(candidate, a) {
				inLane[candidate.Number] = a
			}
		}
	}
	// The Needs plan stage first: a planner on the planning lane, which takes no build lane,
	// plans the fullsend parts a medium subscription (Muse) builds from their plans before the
	// rest, each by a model that meets its plan floor (medium for a medium or unrated item,
	// strong for a hard one).
	var planning []Pair
	planners := LanePlanners(ctx, state, lanes, force, quietOK)
	if len(planners) > 0 {
		needing := py.SortedBy(filter(order, func(c *queue.Candidate) bool {
			return queue.NeedsPlan(c) && inLane[c.Number] == nil
		}), func(c *queue.Candidate) []any {
			return []any{!(force || c.Forced), !c.Part}
		})
		for _, candidate := range needing {
			// The lane's weakest planner that meets its floor (#695, #2748): a medium seat plans
			// a medium or unrated item, a strong one only when no medium one can now, and a
			// strong one a hard item.
			fitting := py.SortedBy(filter(planners, func(s *providers.Seat) bool {
				return providers.TierAtLeast(s.Tier, queue.PlanFloor(candidate))
			}), func(s *providers.Seat) []any {
				return []any{tierRank[s.Tier], s.Rank, lanes.PlanningBy(s.Provider.ID)}
			})
			if len(fitting) == 0 {
				continue
			}
			seat := fitting[0]
			// Its owner plans it when it may (a plan it was cut off in goes on there, #2748).
			if owner := owning[candidate.Number].Owner; owner != "" {
				if own := filter(fitting, func(s *providers.Seat) bool { return s.Provider.ID == owner }); len(own) > 0 {
					seat = own[0]
				}
			}
			what := "writes it"
			if !candidate.Rated {
				what = "rates and plans it"
			}
			assignment := &Assignment{
				Provider: seat.Provider, Action: "plan", Difficulty: candidate.Difficulty, Plan: seat,
				Notes: []string{fmt.Sprintf("#%d: needs a plan; %s %s on the planning lane",
					candidate.Number, seat.Describe(), what)}}
			if elsewhere(candidate, assignment) {
				continue
			}
			handedOff(candidate, assignment, owning[candidate.Number])
			planning = append(planning, Pair{candidate, assignment})
		}
	}
	var found []Pair
	for _, candidate := range order {
		if assignment := inLane[candidate.Number]; assignment != nil {
			handedOff(candidate, assignment, owning[candidate.Number])
			found = append(found, Pair{candidate, assignment})
			continue
		}
		room := lanes
		if Reserves(candidate) {
			// The lane its build reserved for it (#2748), its planning run (#2903), or its turn (#2948).
			room = lanes.Without(candidate.Number)
		}
		if room.Free() <= 0 && room.MachineFree() <= 0 {
			continue
		}
		if assignment := Assign(ctx, state, candidate, lanes, force, quietOK); assignment != nil &&
			!elsewhere(candidate, assignment) {
			handedOff(candidate, assignment, owning[candidate.Number])
			found = append(found, Pair{candidate, assignment})
		}
	}
	planning = py.SortedBy(planning, func(pair Pair) []any { return []any{!(force || pair.Candidate.Forced)} })
	// Own lane first (#2748): a subscription's own work, a continuation of an item paused or cut
	// off, a revision of its pull request, an item held for it, or the build its planning run kept
	// its lane for (#2903), goes before new work on it:
	// after the forced plans, ahead of the other plans, of review runs (#317 part 9 put those
	// before every build and revision) and of every other build or revision, so a lane that a run
	// just freed takes its own next, while its session can still be resumed, and its item does not
	// change hands because a review took the lane first.
	ownWork := func(pair Pair) bool {
		c, own := pair.Candidate, owning[pair.Candidate.Number]
		return own.Owner != "" && pair.Assignment.Provider.ID == own.Owner &&
			(c.Kind == "revise" || c.Continues || c.WasHeld || c.SentBack || c.OwnReview || c.BuildNext)
	}
	var out, own, plans, rest []Pair
	for _, pair := range planning {
		switch {
		case force || pair.Candidate.Forced:
			out = append(out, pair)
		case ownWork(pair):
			own = append(own, pair)
		default:
			plans = append(plans, pair)
		}
	}
	for _, pair := range found {
		if ownWork(pair) {
			own = append(own, pair)
		} else {
			rest = append(rest, pair)
		}
	}
	return slices.Concat(out, own, plans, rest)
}

// handedOff notes an item going to a subscription other than its owner (`Owning`): why, for the
// claim (`OwnerWhy`) and for the run's routing.
func handedOff(c *queue.Candidate, assignment *Assignment, own *Ownership) {
	if own == nil || own.Owner == "" || assignment.Provider.ID == own.Owner {
		return
	}
	why := own.Gone
	if why == "" {
		why = fmt.Sprintf("its owner `%s` cannot take it now", own.Owner)
	}
	assignment.OwnerWhy = why
	assignment.Notes = append(assignment.Notes, fmt.Sprintf("#%d: handed off from its owner `%s` (%s); "+
		"`%s` owns it from this run", c.Number, own.Owner, why, assignment.Provider.ID))
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
