// Package waiting is bot/harness/waiting.py.
//
// What the queue waits for, and what each subscription can do now: the status issue's Queue and
// its Subscriptions (#148).
//
// Worked out with the planner's own rules, never a copy of them: the queue is `queue.candidates`,
// and who takes what is `plan.pairs` run on the lanes held now, one item at a time as the night
// runs claim them (`plan.fill_lanes`). So the issue says an item starts next only when the planner
// would start it, and calls a subscription idle only when it may start work and nothing queued is
// for it. An item no run can take says the tier its difficulty needs (an Opus for a hard one, or a
// revision that answers a strong audit), which subscriptions have such a seat, and when the first
// of them can start one: at a reset, at its hours, after its back-off, or when its run ends. An
// item held for its owner (#2748, `plan.Holds`) says for whom, until when and why.
package waiting

import (
	"fmt"
	"slices"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/reviewrule"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/spotcheck"
)

// TIER_WORDS: what each tier may take, said once above the queue: the router tries the weakest
// that may.
var TierWords = map[string]string{"weak": "any model", "medium": "a medium or strong model", "strong": "a strong model"}

// DIFFICULTY_MARK
var DifficultyMark = map[string]string{"medium": "🟡", "hard": "🔴", "unrated": "⚪"}

// tierRank is providers.TIER_RANK (`{name: rank for rank, name in enumerate(TIERS)}`): the rank,
// and whether `tier` has one.
func tierRank(tier string) (int, bool) {
	rank := slices.Index(providers.Tiers, tier)
	return rank, rank >= 0
}

// rank is TIER_RANK[tier].
func rank(tier string) int {
	r, _ := tierRank(tier)
	return r
}

// recordOf is `state["items"].get(str(number)) or {}`.
func recordOf(s *pyjson.Object, number int) *pyjson.Object {
	if record := s.Obj("items").Obj(strconv.Itoa(number)); record != nil {
		return record
	}
	return pyjson.New()
}

// liveNumber is a key of the lanes (`dict[int, str]` in the Python, its keys written out here).
func liveNumber(key string) int {
	n, err := strconv.Atoi(key)
	if err != nil {
		panic(fmt.Errorf("ValueError: a lane's key %s is not an item number", py.Repr(key)))
	}
	return n
}

func refs(numbers []int) string {
	listed := make([]string, len(numbers))
	for i, n := range numbers {
		listed[i] = fmt.Sprintf("#%d", n)
	}
	return strings.Join(listed, ", ")
}

// unique is `dict.fromkeys(xs)`: each once, in the order first seen.
func unique(xs []string) []string {
	var out []string
	for _, x := range xs {
		if !slices.Contains(out, x) {
			out = append(out, x)
		}
	}
	return out
}

// One subscription now: `kind` is busy, next (the planner gives it an item at the next
// run), idle (it may start work, and nothing queued is for it), reviews (capped for long runs,
// but under the cap a review goes up to), full (no lane free where it runs), capped, closed
// (outside its hours), broken (backing off after a run that could not work) or off. `until` is
// when it can start a build or a plan again, when that is known (zero when not).
type Now struct {
	Kind  string
	Text  string
	Until time.Time // default: None
	Items []int     // default: field(default_factory=list)
}

// One queued item, and what it waits for.
type Row struct {
	Number int
	Title  string
	// What the next run does to it: plan, build, revise or review.
	Step string
	// Its difficulty, or `unrated` (routed as medium until its planner rates it).
	Difficulty string
	// The weakest tier that may take that step, and why it is not the difficulty's own.
	Tier     string
	TierWhy  string
	Priority string
	// Who takes it at the next run (`plan.Assignment`), or None (nil).
	Starts *plan.Assignment // default: None
	// Why nothing takes it now, in a line.
	Waiting string // default: ""
	// True for an item the planner leaves out of the queue (`human`, a blocker, open parts).
	Held   bool // default: False
	Forced bool // default: False
	// Able is the subscriptions with a seat that may take its next step (`Able`), whatever holds
	// them now; HeldFor the one it is held for (`bot:waiting`, #2748), "" for none; QueuedAt when
	// it was queued. The usage history counts the queue for each subscription from them (#2749).
	Able     []string // default: ()
	HeldFor  string   // default: ""
	QueuedAt string   // default: ""
}

// Explained is waiting.Explained.
type Explained struct {
	Rows []*Row
	// Each subscription now, by id: a *Now for each, in the pool's order.
	Now    *pyjson.Object
	Halted string // default: ""
	// After is the lanes once the planner has handed out every queued item it can start (the
	// lanes now while halted): what is left free on them is the excess compute (`plan.Spare`,
	// #3659). nil: not worked out.
	After *plan.Lanes // default: None
}

// The models of `tier`, short: "Opus", or "Muse, agy (Gemini), Sonnet".
func TierNames(pool *providers.Pool, tier string) string {
	var names []string
	for _, entry := range pool.TierEntries(tier) {
		var owners []*providers.Provider
		for _, p := range pool.Ordered() {
			if slices.ContainsFunc(pool.Seats(p), func(s *providers.Seat) bool { return s.Model == entry.Model }) {
				owners = append(owners, p)
			}
		}
		if len(owners) == 0 {
			continue
		}
		if !slices.ContainsFunc(owners, func(p *providers.Provider) bool { return p.CLI != "claude" }) {
			names = append(names, py.Capitalize(entry.Model))
		} else {
			ids := make([]string, len(owners))
			for i, p := range owners {
				ids[i] = p.ID
			}
			names = append(names, strings.Join(unique(ids), ", "))
		}
	}
	if joined := strings.Join(unique(names), ", "); joined != "" {
		return joined
	}
	return "none"
}

// `plan.read_lanes` from the lanes the issue has read already (`status.lanes_now`), so the
// queue costs no second look at the runs, with the lanes in force now (`plan.lanes_in_force`:
// a Claude account's by its 5-hour cap, #2748). `live` is item number (its key written out) ->
// provider id.
func LanesOf(ctx *hctx.Context, s *pyjson.Object, live *pyjson.Object) *plan.Lanes {
	pool := ctx.Cfg.Pool
	planning := map[int]bool{}
	for _, key := range live.Keys() {
		if n := liveNumber(key); n != 0 && queue.PlanningPool(recordOf(s, n)) {
			planning[n] = true
		}
	}
	filling := map[int]bool{}
	for _, key := range live.Keys() {
		if n := liveNumber(key); n != 0 && plan.FillsIn(pool, recordOf(s, n)) {
			filling[n] = true
		}
	}
	machine := map[string]bool{}
	for _, p := range pool.Ordered() {
		if pool.OnMachine(p.ID) {
			machine[p.ID] = true
		}
	}
	held := map[int]string{}
	for _, key := range live.Keys() {
		held[liveNumber(key)] = live.Str(key)
	}
	return &plan.Lanes{Limit: pool.MaxParallel, Held: held, Planning: planning,
		PlanLimit: pool.PlanLanes, MachineLimit: pool.MachineParallel, Machine: machine,
		Filling: filling, InForce: plan.LanesInForce(ctx, s)}
}

// (step, tier, why): what the next run does to `c`, the weakest tier that may do it
// (`plan.assign`, `plan.review_seat`), and why that tier is not its difficulty's own.
func Need(c *queue.Candidate) (string, string, string) {
	difficulty := config.AsDifficulty(c.Difficulty)
	if c.SpotCheck {
		return "review", spotcheck.Tier, "an after-merge spot-check" // #2748, section 5
	}
	if c.OwnReview {
		// Its own review run (#2748, 2026-11-02): on the subscription whose run built it, at the
		// item's tier, which its lane reviews and fixes at.
		return "review", config.MinTier[difficulty], fmt.Sprintf("its own review run, on `%s`", c.Owner)
	}
	if c.Kind == "review" {
		// The weakest tier whose approval ships it (#2748), on any subscription, its builder's
		// included (2026-11-02): medium for a medium item; strong for a hard one, or for votes
		// cast before #2748 (`review_rule.tiers`).
		tiers := c.ReviewTiers
		if len(tiers) == 0 {
			tiers = reviewrule.Shipping(difficulty)
		}
		tier, why := tiers[0], ""
		if tier == "strong" {
			why = "its final audit"
		}
		if r, ok := tierRank(c.ReviewFloor); ok && r > rank(tier) {
			tier, why = c.ReviewFloor, fmt.Sprintf("a person asked for a %s review", c.ReviewFloor)
		}
		return "review", tier, why
	}
	if queue.NeedsPlan(c) {
		why := "a plan first"
		if !c.Rated {
			why = "rates and plans it first"
		}
		return "plan", queue.PlanFloor(c), why
	}
	tier, why := config.MinTier[difficulty], ""
	if c.Kind == "build" && c.Part && queue.PlanMeets(c) && rank(tier) > rank(config.PartFloor) {
		tier, why = config.PartFloor, "a fullsend part, built from its plan"
	}
	if c.Kind == "revise" {
		// A revision that answers a final audit is the auditor's own fix (#2748, section 4): its
		// subscription owns it and fixes it at the audit's tier, but handed off it goes to the
		// item's own tier, which is the weakest that may do it.
		if (!c.BotPR || c.Conflict) && rank(tier) < rank("medium") {
			tier = "medium"
			if c.Conflict {
				why = "a conflict with main wants a reviewer in its run"
			} else {
				why = "a person's pull request wants a reviewer in its run"
			}
		}
	}
	return c.Kind, tier, why
}

// `20:27` today (Central time), `Fri 18:47` on another day (`plan.Clock`).
func Clock(ctx *hctx.Context, at time.Time) string {
	return plan.Clock(ctx, at)
}

// Why `provider` may not start a run now, as a `Now`, or None (nil) when it may (its lanes
// aside). In the order `providers.availability` checks.
func HeldBy(ctx *hctx.Context, s *pyjson.Object, provider *providers.Provider, forced bool /* = False */, starting bool /* = True */) *Now {
	cfg, now := ctx.Cfg, ctx.Now()
	reason := providers.Availability(provider, s, now, cfg.Timezone, cfg.Secrets, forced, starting)
	if reason == "" {
		return nil
	}
	if !provider.Enabled || providers.SwitchedOffByDate(provider, now, cfg.Timezone) {
		return &Now{Kind: "off", Text: "⚫ switched off"}
	}
	if providers.Suspension(s, provider.ID) != nil {
		return &Now{Kind: "off", Text: "⚫ suspended"}
	}
	if provider.Login == "secret" {
		if has := cfg.Secrets.Has(provider.Secret); has != nil && !*has {
			return &Now{Kind: "off", Text: "⚫ off: no secret set"}
		}
	}
	window := provider.Schedule.Window(cfg.Timezone)
	if !forced && provider.OffHours.Len() == 0 && window != nil && !window.IsOpen(now) {
		opens := window.NextOpen(now)
		return &Now{Kind: "closed", Text: "🌙 outside its hours<br/>opens " + Clock(ctx, opens), Until: opens}
	}
	entry := providers.PeekRecord(s, provider.ID)
	infra := entry.Obj("infra")
	if infra == nil {
		infra = pyjson.New()
	}
	failed := clock.ParseISOValue(infra.Get("at"))
	if !forced && !failed.IsZero() && now.Sub(failed) < providers.InfraBackoff(infra) {
		back := failed.Add(providers.InfraBackoff(infra))
		return &Now{Kind: "broken", Text: "⛔ its last run could not work<br/>tries again " + Clock(ctx, back),
			Until: back}
	}
	frees := providers.FreesAt(provider, entry, now, cfg.Timezone)
	// Under its cap but too close to it for a long run: a review, which goes up to the cap, may
	// still start (`plan._usable`).
	reviews := starting && slices.Contains(provider.Roles, "review") &&
		providers.Availability(provider, s, now, cfg.Timezone, cfg.Secrets, forced, false) == ""
	kind := "capped"
	if reviews {
		kind = "reviews"
	}
	return &Now{Kind: kind, Text: "⏸️ " + CappedText(ctx, provider, entry, frees, reviews), Until: frees}
}

// floatOr0 is `float(v or 0)`.
func floatOr0(v any) float64 {
	if !pyjson.Truthy(v) {
		return 0
	}
	switch x := v.(type) {
	case bool:
		return 1
	case string:
		f, err := strconv.ParseFloat(py.Strip(x), 64)
		if err != nil {
			panic(fmt.Errorf("ValueError: could not convert string to float: %s", py.Repr(x)))
		}
		return f
	}
	f, ok := pyjson.AsFloat(v)
	if !ok {
		panic(fmt.Errorf("TypeError: float() argument must be a string or a real number, not %T", v))
	}
	return f
}

// Which limit holds a capped subscription, and when it can start a build or a plan again.
func CappedText(ctx *hctx.Context, provider *providers.Provider, entry *pyjson.Object, frees time.Time, reviews bool /* = False */) string {
	now := ctx.Now()
	until := clock.ParseISOValue(entry.Get("refused_until"))
	if !until.IsZero() && until.After(now) {
		return "refused a call<br/>until " + Clock(ctx, until)
	}
	usage := providers.CurrentUsage(provider, entry)
	again := ""
	if !frees.IsZero() {
		again = "<br/>starts again " + Clock(ctx, frees)
	}
	caps := provider.CapsAt(now, ctx.Cfg.Timezone, usage, time.Time{})
	for _, window := range caps.Keys() {
		cap_, _ := pyjson.AsFloat(caps.Get(window))
		reading := usage.Obj(window)
		if reading == nil {
			continue
		}
		resets := clock.ParseISOValue(reading.Get("resets_at"))
		if !resets.IsZero() && !resets.After(now) {
			continue
		}
		used := floatOr0(reading.Get("utilization"))
		start := providers.StartUnder(provider, window, cap_)
		if used < start {
			continue
		}
		held := fmt.Sprintf("%s %s of its %s cap", providers.WindowNames[window], py.Percent(used, 0), py.Percent(cap_, 0))
		if used < cap_ {
			held += "<br/>long runs start under " + py.Percent(start, 0)
			if reviews {
				held += "; reviews only"
			}
		}
		return held + again
	}
	budgets := provider.Limits.Budgets()
	for _, window := range budgets.Keys() {
		budget := budgets.Get(window)
		spent := providers.MinutesSpent(entry, now.Add(-providers.Windows[window]))
		if limit, _ := pyjson.AsFloat(budget); spent >= limit {
			return fmt.Sprintf("%.0f of its %s min", spent, py.Str(budget)) + again
		}
	}
	return "at its limit" + again
}

// The queue in the planner's order, each item with what takes it next or what it waits
// for, and each subscription now.
func Explain(ctx *hctx.Context, s *pyjson.Object, live *pyjson.Object, issues []*pyjson.Object /* = () */, halted string /* = "" */) *Explained {
	heldBack := map[int]string{}
	q := queue.Candidates(ctx, s, nil, heldBack)
	lanes := LanesOf(ctx, s, live)
	starts := pyjson.New() // number (written out) -> its *plan.Assignment
	after := lanes
	if halted == "" {
		rest := slices.Clone(q)
		for range len(q) {
			pairs := plan.Pairs(ctx, s, rest, after, false, plan.AnyQuiet, nil)
			if len(pairs) == 0 {
				break
			}
			candidate := pairs[0].Candidate
			assignment := pairs[0].Assignment
			starts.Set(strconv.Itoa(candidate.Number), assignment)
			after = after.WithRun(candidate.Number, assignment.Provider.ID, assignment.LaneAction(),
				assignment.FillsIn())
			rest = slices.DeleteFunc(rest, func(c *queue.Candidate) bool { return c.Number == candidate.Number })
		}
	}
	now := Subscriptions(ctx, s, lanes, after, starts, halted)
	// Items held for their owners (#2748): what they wait for is that owner.
	holds := map[int]*plan.Hold{}
	if halted == "" {
		holds = plan.Holds(ctx, s, q, lanes, false, plan.AnyQuiet)
	}
	// The planner's own order (`plan.pairs`), the items that start at the next run first.
	order := slices.Clone(q)
	sort.SliceStable(order, func(i, j int) bool {
		return lessByPlanner(order[i], order[j], starts)
	})
	rows := []*Row{}
	for _, c := range order {
		step, tier, why := Need(c)
		difficulty := c.Difficulty
		if !c.Rated {
			difficulty = "unrated"
		}
		priority, ok := queue.PriorityNames[c.Priority]
		if !ok {
			priority = "none"
		}
		start, _ := starts.Get(strconv.Itoa(c.Number)).(*plan.Assignment)
		row := &Row{Number: c.Number, Title: c.Title, Step: step, Difficulty: difficulty, Tier: tier,
			TierWhy: why, Priority: priority, Starts: start, Forced: c.Forced, QueuedAt: c.QueuedAt}
		for _, p := range Able(ctx.Cfg.Pool, c, step, tier) {
			row.Able = append(row.Able, p.ID)
		}
		if held := holds[c.Number]; held != nil {
			row.HeldFor = held.Owner
		}
		if row.Starts == nil {
			row.Waiting = halted
			if held := holds[c.Number]; held != nil {
				row.Waiting = "⏸️ " + plan.HoldLine(ctx, held)
			} else if halted == "" {
				row.Waiting = WaitsFor(ctx, s, c, step, tier, after, now)
			}
		}
		rows = append(rows, row)
	}
	rows = append(rows, HeldRows(heldBack, issues)...)
	return &Explained{Rows: rows, Now: now, Halted: halted, After: after}
}

// lessByPlanner is the key `(c.number not in starts, not c.forced, c.kind != "review",
// c.spot_check, c.priority, KIND_ORDER[c.kind], -DIFFICULTIES.index(c.difficulty), c.queued_at,
// c.number)`.
func lessByPlanner(a, b *queue.Candidate, starts *pyjson.Object) bool {
	aStarts, bStarts := starts.Has(strconv.Itoa(a.Number)), starts.Has(strconv.Itoa(b.Number))
	if aStarts != bStarts {
		return aStarts
	}
	if a.Forced != b.Forced {
		return a.Forced
	}
	if aReview, bReview := a.Kind == "review", b.Kind == "review"; aReview != bReview {
		return aReview
	}
	if a.SpotCheck != b.SpotCheck {
		return b.SpotCheck // after the other review runs (#2748)
	}
	if a.Priority != b.Priority {
		return a.Priority < b.Priority
	}
	if queue.KindOrder[a.Kind] != queue.KindOrder[b.Kind] {
		return queue.KindOrder[a.Kind] < queue.KindOrder[b.Kind]
	}
	ad, bd := slices.Index(config.Difficulties, a.Difficulty), slices.Index(config.Difficulties, b.Difficulty)
	if ad != bd {
		return ad > bd
	}
	if a.QueuedAt != b.QueuedAt {
		return a.QueuedAt < b.QueuedAt
	}
	return a.Number < b.Number
}

// The labelled items the planner leaves out, each with why.
func HeldRows(heldBack map[int]string, issues []*pyjson.Object) []*Row {
	titles := map[int]string{}
	for _, t := range issues {
		v, ok := t.Lookup("number")
		if !ok {
			panic(fmt.Errorf("KeyError: 'number'"))
		}
		n, _ := pyjson.AsInt(v)
		titles[n] = py.OrStr(t.Get("title"), "")
	}
	numbers := make([]int, 0, len(heldBack))
	for n := range heldBack {
		numbers = append(numbers, n)
	}
	sort.Ints(numbers)
	rows := []*Row{}
	for _, number := range numbers {
		rows = append(rows, &Row{Number: number, Title: titles[number], Step: "build", Priority: "none",
			Waiting: heldBack[number], Held: true})
	}
	return rows
}

// Each subscription now. Busy is a run going; next is an item the planner gives it at the
// next run; idle is free to start and nothing queued is for it. One that may start but has no
// lane where it runs is full, never free. `starts` is item number (written out) -> its
// *plan.Assignment; the result is provider id -> its *Now, in the pool's order.
func Subscriptions(ctx *hctx.Context, s *pyjson.Object, lanes *plan.Lanes, after *plan.Lanes, starts *pyjson.Object, halted string /* = "" */) *pyjson.Object {
	pool := ctx.Cfg.Pool
	nexts := map[string][]int{}
	for _, key := range starts.Keys() {
		assignment := starts.Get(key).(*plan.Assignment)
		nexts[assignment.Provider.ID] = append(nexts[assignment.Provider.ID], liveNumber(key))
	}
	found := pyjson.New()
	for _, provider := range pool.Ordered() {
		var running []int
		for number, id := range lanes.Held {
			if id == provider.ID {
				running = append(running, number)
			}
		}
		sort.Ints(running)
		if next, ok := nexts[provider.ID]; ok {
			now := &Now{Kind: "next", Text: "▶️ takes " + refs(next) + " next", Items: next}
			if len(running) > 0 {
				now.Items = append(slices.Clone(running), next...)
			}
			found.Set(provider.ID, now)
			continue
		}
		if len(running) > 0 {
			found.Set(provider.ID, &Now{Kind: "busy", Text: "🟢", Items: running})
			continue
		}
		if held := HeldBy(ctx, s, provider, false, true); held != nil {
			found.Set(provider.ID, held)
			continue
		}
		switch {
		case halted != "":
			found.Set(provider.ID, &Now{Kind: "off", Text: "🛑 halted"})
		case plan.MachineFull(pool, provider, lanes):
			found.Set(provider.ID, &Now{Kind: "full", Text: fmt.Sprintf("⏸️ the night box is full<br/>"+
				"(%d at once)", pool.MachineParallel)})
		case plan.FillInFull(pool, provider, lanes):
			found.Set(provider.ID, &Now{Kind: "full", Text: "⏸️ its lane on its box is in use<br/>" +
				fmt.Sprintf("(`%s` fills it)", pool.FillerOf(provider.ID).ID)})
		case !lanes.RoomFor(provider.ID) && lanes.PlanFree() == 0:
			found.Set(provider.ID, &Now{Kind: "full", Text: fmt.Sprintf("⏸️ every lane is in use<br/>"+
				"(%d on GitHub's runners)", pool.MaxParallel)})
		default:
			found.Set(provider.ID, &Now{Kind: "idle", Text: "⚪ idle<br/>nothing queued is for it"})
		}
	}
	return found
}

// Able is the subscriptions with a seat at `tier` or above that may take `step` of `c`, in the
// usage order, whatever holds them now: a spot-check never goes to its builder's subscription,
// and an own review run only to its owner's.
func Able(pool *providers.Pool, c *queue.Candidate, step string, tier string) []*providers.Provider {
	role, ok := plan.RoleOf[step]
	if !ok {
		role = step
	}
	var able []*providers.Provider
	for _, p := range pool.Ordered() {
		if step == "review" && c.SpotCheck && c.BuiltBy != "" && p.ID == c.BuiltBy {
			continue // a spot-check never goes to its builder's subscription (#2748)
		}
		if c.OwnReview && p.ID != c.Owner {
			continue // its own review run is its builder's lane's alone (2026-11-02)
		}
		if slices.Contains(p.Roles, role) && pool.BestSeat(p, tier) != nil {
			able = append(able, p)
		}
	}
	return able
}

// Why no run takes `c` now: the kind of model it needs, and which of them frees up first.
// `now` is Subscriptions' result.
func WaitsFor(ctx *hctx.Context, s *pyjson.Object, c *queue.Candidate, step string, tier string, lanes *plan.Lanes, now *pyjson.Object) string {
	pool := ctx.Cfg.Pool
	role, ok := plan.RoleOf[step]
	if !ok {
		role = step
	}
	able := Able(pool, c, step, tier)
	if len(able) == 0 {
		return fmt.Sprintf("⚠️ no subscription has a %s seat that may %s it", tier, role)
	}
	wanted := TierWords[tier]
	if step == "plan" {
		kind := tier
		if tier == "medium" {
			kind = "medium or strong"
		}
		wanted = fmt.Sprintf("a %s planner", kind)
	}
	if tier == "strong" {
		wanted += fmt.Sprintf(" (%s)", TierNames(pool, tier))
	}
	if step == "review" && c.SpotCheck && c.BuiltBy != "" {
		wanted += fmt.Sprintf(" on a subscription other than `%s`, which built it", c.BuiltBy)
	}
	if c.OwnReview {
		wanted += fmt.Sprintf(" on `%s`, whose run built it (its own review run)", c.Owner)
	}
	var soonestAt time.Time
	soonestID, haveSoonest := "", false
	var busy, taking, idle []string
	for _, provider := range able {
		held, _ := now.Get(provider.ID).(*Now)
		if held == nil {
			continue
		}
		switch {
		case held.Kind == "busy":
			busy = append(busy, fmt.Sprintf("`%s`", provider.ID))
		case held.Kind == "next":
			var others []int
			for _, n := range held.Items {
				if n != c.Number {
					others = append(others, n)
				}
			}
			taking = append(taking, fmt.Sprintf("`%s` takes %s first", provider.ID, refs(others)))
		case !held.Until.IsZero():
			if !haveSoonest || held.Until.Before(soonestAt) {
				soonestAt, soonestID, haveSoonest = held.Until, provider.ID, true
			}
		case held.Kind == "idle" || held.Kind == "full":
			idle = append(idle, provider.ID)
		}
	}
	var parts []string
	if haveSoonest {
		parts = append(parts, fmt.Sprintf("first free `%s` at %s", soonestID, Clock(ctx, soonestAt)))
	}
	if len(busy) > 0 {
		parts = append(parts, "busy "+strings.Join(busy, ", "))
	}
	parts = append(parts, taking...)
	if len(parts) == 0 && len(idle) > 0 {
		full := slices.ContainsFunc(idle, func(p string) bool { return now.Get(p).(*Now).Kind == "full" })
		if full {
			parts = append(parts, "no lane is free where they run")
		} else {
			quoted := make([]string, len(idle))
			for i, p := range idle {
				quoted[i] = fmt.Sprintf("`%s`", p)
			}
			parts = append(parts, strings.Join(quoted, ", ")+" free, but the planner's rules pass them over")
		}
	}
	said := fmt.Sprintf("⏳ **%s**", wanted)
	if len(parts) > 0 {
		said += ": " + strings.Join(parts, " · ")
	}
	return said
}
