package usage

import (
	"fmt"
	"math"
	"slices"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// The tuning suggestions (#2749, section 15): what the history says about the limits in
// `.harness/providers.json` and the hold in `.harness/config.json`, as a table of `subscription |
// key | now | suggested | the evidence`. Computed, never applied: nothing here writes a file, and a
// person edits providers.json. "Now" is the config as the loop loaded it. One function per rule,
// each reading the facts it names and returning nil when its threshold is not met; every threshold
// is named here, once (CLAUDE.md rule 9).

// The thresholds.
var (
	// SuggestDays is how far back a rule reads, unless it says otherwise: two weeks.
	SuggestDays = 14
	// SuggestMinSessions is the fewest closed sessions a rule over sessions reads before it says
	// anything: fewer are chance.
	SuggestMinSessions = 4

	// A 5-hour cap may go up `CapRaisePoints` when it was hit, with work queued for the account, in
	// at least `CapHitShare` of its sessions over `SuggestDays`, and its readings rose outside the
	// bot's runs by no more than `ReserveUnusedPoints` (nobody used the reserve the cap keeps).
	CapRaisePoints      = 10.0
	CapHitShare         = 0.5
	ReserveUnusedPoints = 0.0
	// CapStep is what a lowered 5-hour cap is rounded down to a multiple of, in points.
	CapStep = 5.0

	// A 5-hour cap, or the lanes, should come down when the week is on course to pass its weekly
	// cap (or 100%) before its reset: the trend of the last `ProjectSpan` of its readings in this
	// week, read over at least `ProjectMinSpan`, projected to the reset.
	ProjectSpan    = 3 * 24 * time.Hour
	ProjectMinSpan = 12 * time.Hour

	// Two lanes may be too many when the second lane's runs pushed a session past its cap by more
	// than `providers.LastHourSlack` in `LaneSessions` or more sessions within `LaneWeek`.
	LaneSessions = 3
	LaneWeek     = 7 * 24 * time.Hour

	// `start_headroom` may be too small when, of the runs (at least `HeadroomMinRuns`) that started
	// inside it over `SuggestDays`, more than `HeadroomCutShare` were cut off at the 5-hour cap
	// within `HeadroomCutMinutes` of their start; it may go up `HeadroomStep` points.
	HeadroomCutMinutes = 10.0
	HeadroomCutShare   = 0.5
	HeadroomMinRuns    = 4
	HeadroomStep       = 5.0

	// `last_hour` is being wasted when, in `LastHourWastedSessions` or more sessions over
	// `SuggestDays`, its last hour lifted the cap while work was queued for it and no run of it went
	// (its usage rose less than `RisePoints` in that hour): a routing problem, not a cap.
	LastHourWastedSessions = 3

	// `hold_minutes` needs moving when, over at least `HoldMinHolds` holds in `SuggestDays`, held
	// items waited longer (the median hold) than a handed-off item waited to start (its runs' median
	// `queue_wait_min`), or more than `HoldRanOutShare` of the holds ran out.
	HoldMinHolds    = 4
	HoldRanOutShare = 0.5
	// HoldStep is what a suggested `hold_minutes` is rounded up to a multiple of, in minutes.
	HoldStep = 5.0

	// The plan load may need to move when a medium subscription reached its weekly cap before day
	// `PlanLoadDay` of its week in `PlanLoadWeeks` closed weeks running (its plans back to Opus), or,
	// planning nothing, left at least `PlanLoadSpare` percent of its week unused in as many (plans
	// to it).
	PlanLoadDay   = 6
	PlanLoadWeeks = 2
	PlanLoadSpare = 50.0
)

// Suggestion is one row of the table: the subscription it is about ("" for the pool's or the
// config's own), the key (`limits.five_hour`, or `hold_minutes (config.json)`), its value now as
// loaded, the suggested one, and the evidence.
type Suggestion struct {
	Subscription string
	Key          string
	Now          string
	Suggested    string
	Evidence     string
}

// Suggestions is every rule's suggestion, the subscriptions' in the issue's order of them, then
// the pool's. Computed once per facts.
func Suggestions(ctx *hctx.Context, f *Facts) []*Suggestion {
	x := f.of()
	if x.suggested != nil {
		return x.suggested
	}
	out := []*Suggestion{}
	for _, p := range subscriptionsOf(ctx, f) {
		for _, rule := range []func(*hctx.Context, *Facts, *providers.Provider) *Suggestion{
			SuggestRaiseCap, SuggestWeekPace, SuggestFewerLanes, SuggestHeadroom, SuggestLastHour, SuggestPlanLoad} {
			if s := rule(ctx, f, p); s != nil {
				out = append(out, s)
			}
		}
	}
	if s := SuggestHoldMinutes(ctx, f); s != nil {
		out = append(out, s)
	}
	x.suggested = out
	return out
}

// days is `n` days.
func days(n int) time.Duration { return time.Duration(n) * 24 * time.Hour }

// share is a providers.json share as percent points.
func share(v float64) float64 { return round1(v * 100) }

// pctText is percent points as `70%`.
func pctText(v float64) string { return N(round1(v)).String() + "%" }

// sessionsSince is `p`'s sessions that reset in [from, now).
func sessionsSince(f *Facts, id string, from time.Time) []*Session {
	return since(f.of().sessions[id], func(s *Session) time.Time { return s.ResetAt }, from)
}

// runsSince is `id`'s runs that ended at or after `from`.
func runsSince(f *Facts, id string, from time.Time) []*Run {
	return since(f.of().runs[id], func(r *Run) time.Time { return r.EndedAt }, from)
}

// SuggestRaiseCap: a 5-hour cap may go up `CapRaisePoints` when the account hit it, with work
// queued for it, in at least `CapHitShare` of its sessions over `SuggestDays`, and no reading rose
// outside the bot's runs (`RisesOutside`): the reserve the cap keeps went unused. The cap it hit
// is its `off_hours` one when most of those sessions held that, else its own; it goes no higher
// than its hard cap, or 100%.
func SuggestRaiseCap(ctx *hctx.Context, f *Facts, p *providers.Provider) *Suggestion {
	if p.Limits.Mode != "caps" {
		return nil
	}
	from := f.Now.Add(-days(SuggestDays))
	sessions := sessionsSince(f, p.ID, from)
	if len(sessions) < SuggestMinSessions {
		return nil
	}
	var hit []*Session
	held := map[float64]int{}
	for _, s := range sessions {
		if !s.HitCapAt.IsZero() && s.MinutesCappedQueued.V > 0 {
			hit = append(hit, s)
			held[s.CapMin.V]++
		}
	}
	if len(hit) == 0 || float64(len(hit)) < CapHitShare*float64(len(sessions)) {
		return nil
	}
	if rose := risen(f.of().rises[p.ID], from, f.Now); rose > ReserveUnusedPoints {
		return nil
	}
	key, now := "limits.five_hour", p.Limits.FiveHour
	if off, isNumber := py.Number(p.OffHours.Get("five_hour")); isNumber && (now == nil ||
		held[share(off)] > held[share(*now)]) {
		key, now = "off_hours.five_hour", &off
	}
	if now == nil {
		return nil
	}
	ceiling := 100.0
	if p.HardCap != nil {
		ceiling = share(*p.HardCap)
	}
	current := share(*now)
	suggested := min(current+CapRaisePoints, ceiling)
	if suggested <= current {
		return nil
	}
	return &Suggestion{Subscription: p.ID, Key: key, Now: pctText(current), Suggested: pctText(suggested),
		Evidence: fmt.Sprintf("hit its cap with work queued in %d of its %d sessions over %d days, and no "+
			"reading rose outside the bot's runs: nobody used the reserve", len(hit), len(sessions), SuggestDays)}
}

// SuggestWeekPace: a 5-hour cap, or the lanes, should come down when the week is on course to pass
// its weekly cap in force (or 100%) before it resets: the least-squares trend of the week's
// readings over the last `ProjectSpan` (at least `ProjectMinSpan` of them), projected from the
// newest to the reset. The suggestion scales by the share of that pace the cap leaves room for:
// its lanes when it has more than one of its own, else its 5-hour cap (100% when it has none),
// rounded down to `CapStep`; a subscription with no 5-hour reading, which no 5-hour cap holds, its
// budget of model minutes in a rolling 5 hours (`five_hour_minutes`), from its runs' pace.
func SuggestWeekPace(ctx *hctx.Context, f *Facts, p *providers.Provider) *Suggestion {
	var newest *Reading
	for _, r := range f.of().readings[p.ID] {
		if r.Util7d.OK && !r.Resets7d.IsZero() && !r.ObservedAt.After(f.Now) {
			newest = r
		}
	}
	if newest == nil || !newest.Resets7d.After(f.Now) {
		return nil
	}
	from := f.Now.Add(-ProjectSpan)
	if start := newest.Resets7d.Add(-WeekSpan); start.After(from) {
		from = start
	}
	var xs, ys []float64
	var first time.Time
	for _, r := range f.of().readings[p.ID] {
		if !r.Util7d.OK || r.ObservedAt.Before(from) || r.ObservedAt.After(f.Now) ||
			absDuration(r.Resets7d.Sub(newest.Resets7d)) > WeekSlack {
			continue
		}
		if first.IsZero() {
			first = r.ObservedAt
		}
		xs, ys = append(xs, r.ObservedAt.Sub(from).Hours()), append(ys, r.Util7d.V)
	}
	slope, ok := trend(xs, ys)
	if !ok || slope <= 0 || newest.ObservedAt.Sub(first) < ProjectMinSpan {
		return nil
	}
	projected := round1(newest.Util7d.V + slope*newest.Resets7d.Sub(newest.ObservedAt).Hours())
	limit := 100.0
	if newest.Cap7d.OK {
		limit = newest.Cap7d.V
	}
	if projected <= limit {
		return nil
	}
	fits := max(0, (limit-newest.Util7d.V)/(projected-newest.Util7d.V))
	evidence := fmt.Sprintf("its week read %s at %s, rising %s points an hour over the last %d days: %s by "+
		"its reset at %s, past its weekly cap of %s", pctText(newest.Util7d.V), localText(ctx, newest.ObservedAt),
		N(py.RoundN(slope, 2)).String(), int(ProjectSpan.Hours()/24), pctText(projected),
		localText(ctx, newest.Resets7d), pctText(limit))
	if lanes := p.Lanes; lanes > 1 && !ctx.Cfg.Pool.LanesByCap.Matches(p) {
		suggested := min(max(1, int(math.Floor(float64(lanes)*fits))), lanes-1)
		return &Suggestion{Subscription: p.ID, Key: "lanes", Now: fmt.Sprint(lanes), Suggested: fmt.Sprint(suggested),
			Evidence: evidence}
	}
	read5h := false
	for _, r := range f.of().readings[p.ID] {
		read5h = read5h || r.Util5h.OK
	}
	if !read5h {
		// No 5-hour reading (agy's and Codex's `/usage` read the week alone), so no 5-hour cap can
		// hold it: a budget of model minutes in a rolling 5 hours can, at its pace's share.
		spent := 0.0
		for _, run := range runsSince(f, p.ID, from) {
			spent += run.ModelMinutes.V
		}
		if spent <= 0 {
			return nil
		}
		perSession := spent / (f.Now.Sub(from).Hours() / SessionSpan.Hours())
		nowText, current := "none", math.Inf(1)
		if p.Limits.Mode == "caps" && p.Limits.FiveHourMinutes != nil {
			current = float64(*p.Limits.FiveHourMinutes)
			nowText = N(current).String()
		}
		suggested := max(CapStep, math.Floor(min(perSession, current)*fits/CapStep)*CapStep)
		if suggested >= current {
			return nil
		}
		return &Suggestion{Subscription: p.ID, Key: "limits.five_hour_minutes", Now: nowText,
			Suggested: N(suggested).String(), Evidence: evidence + fmt.Sprintf("; it spent %s model minutes a 5 hours",
				N(round1(perSession)).String())}
	}
	current, nowText := 100.0, "none"
	if p.Limits.Mode == "caps" && p.Limits.FiveHour != nil {
		current = share(*p.Limits.FiveHour)
		nowText = pctText(current)
	}
	suggested := max(CapStep, math.Floor(current*fits/CapStep)*CapStep)
	if suggested >= current {
		suggested = max(CapStep, current-CapStep)
	}
	return &Suggestion{Subscription: p.ID, Key: "limits.five_hour", Now: nowText, Suggested: pctText(suggested),
		Evidence: evidence}
}

// SuggestFewerLanes: two lanes may be too many when, in `LaneSessions` or more sessions that reset
// in the last `LaneWeek`, the account went more than `providers.LastHourSlack` past its cap
// (`overshoot_max`) while two of its runs went at once (a tick of the session with two lanes or
// runs held). Its lanes are `lanes_by_cap`'s for an account that rule decides (every such
// account's), else its own `lanes`.
func SuggestFewerLanes(ctx *hctx.Context, f *Facts, p *providers.Provider) *Suggestion {
	lanes, key := p.Lanes, "lanes"
	if rule := ctx.Cfg.Pool.LanesByCap; rule.Matches(p) {
		lanes, key = rule.High, "lanes_by_cap.high"
	}
	if lanes < 2 {
		return nil
	}
	slack := share(providers.LastHourSlack)
	count, most := 0, 0.0
	for _, s := range sessionsSince(f, p.ID, f.Now.Add(-LaneWeek)) {
		if !s.OvershootMax.OK || s.OvershootMax.V <= slack {
			continue
		}
		two := false
		for _, t := range f.of().ticks[p.ID] {
			if in(t.At, s.StartedAt, s.ResetAt) && (t.LanesHeld.V >= 2 || t.RunsHeld.V >= 2) {
				two = true
				break
			}
		}
		if two {
			count, most = count+1, max(most, s.OvershootMax.V)
		}
	}
	if count < LaneSessions {
		return nil
	}
	evidence := fmt.Sprintf("%d sessions in the last %d days went more than %s past its cap with two of its "+
		"runs going (the most %s)", count, int(LaneWeek.Hours()/24), pointsText(slack), pointsText(most))
	if key != "lanes" {
		evidence += "; `lanes_by_cap` is every " + p.CLI + " account's"
	}
	return &Suggestion{Subscription: p.ID, Key: key, Now: fmt.Sprint(lanes), Suggested: fmt.Sprint(lanes - 1),
		Evidence: evidence}
}

// SuggestHeadroom: `start_headroom` may be too small when, of the account's runs over
// `SuggestDays` that started inside it (their first 5-hour reading within the headroom of the cap
// in force on it; at least `HeadroomMinRuns` of them), more than `HeadroomCutShare` were cut off at
// the 5-hour cap within `HeadroomCutMinutes` of their start.
func SuggestHeadroom(ctx *hctx.Context, f *Facts, p *providers.Provider) *Suggestion {
	headroom := capOf(p.Limits.Headroom, "five_hour")
	if !headroom.OK || headroom.V <= 0 {
		return nil
	}
	inside, cut := 0, 0
	for _, run := range runsSince(f, p.ID, f.Now.Add(-days(SuggestDays))) {
		readings := f.of().byRun[run.RunID]
		if run.RunID == "" || len(readings) == 0 || !readings[0].Util5h.OK || !readings[0].Cap5h.OK {
			continue
		}
		start := readings[0]
		if start.Util5h.V < start.Cap5h.V-headroom.V {
			continue
		}
		inside++
		if run.StopReason == "5-hour cap" && run.EndedAt.Sub(start.ObservedAt).Minutes() < HeadroomCutMinutes {
			cut++
		}
	}
	if inside < HeadroomMinRuns || float64(cut) <= HeadroomCutShare*float64(inside) {
		return nil
	}
	return &Suggestion{Subscription: p.ID, Key: "start_headroom.five_hour", Now: pctText(headroom.V),
		Suggested: pctText(headroom.V + HeadroomStep),
		Evidence: fmt.Sprintf("%d of the %d runs that started within %s of its cap over %d days were cut off "+
			"at it within %s minutes", cut, inside, pctText(headroom.V), SuggestDays, N(HeadroomCutMinutes).String())}
}

// SuggestLastHour: `last_hour` is being wasted when, in `LastHourWastedSessions` or more of the
// account's sessions over `SuggestDays`, a tick of its last hour shows the cap lifted
// (`last_hour_now`) with work queued for it and no run of it going, and its usage rose less than
// `RisePoints` in that hour (`last_hour_used`): the work went elsewhere, a routing problem, not a
// cap.
func SuggestLastHour(ctx *hctx.Context, f *Facts, p *providers.Provider) *Suggestion {
	if !p.TakesLastHours() {
		return nil
	}
	wasted := 0
	for _, s := range sessionsSince(f, p.ID, f.Now.Add(-days(SuggestDays))) {
		if s.LastHourUsed.OK && s.LastHourUsed.V >= RisePoints {
			continue
		}
		for _, t := range f.of().ticks[p.ID] {
			if in(t.At, s.ResetAt.Add(-providers.LastHour), s.ResetAt) && t.LastHourNow.V > 0 &&
				t.QueuedForIt.V > 0 && t.RunsHeld.V == 0 {
				wasted++
				break
			}
		}
	}
	if wasted < LastHourWastedSessions {
		return nil
	}
	return &Suggestion{Subscription: p.ID, Key: "last_hour", Now: "true",
		Suggested: "true: route its queued work to it in its last hour",
		Evidence: fmt.Sprintf("in %d sessions over %d days its last hour lifted the cap with work queued for it "+
			"and no run of it going, and went unused: a routing problem, not a cap", wasted, SuggestDays)}
}

// SuggestHoldMinutes: `hold_minutes` (config.json) needs moving when, over at least `HoldMinHolds`
// holds in `SuggestDays` (`Holds`, from every subscription's ticks), held items waited longer
// (the median hold) than a handed-off item waited to start (the median `queue_wait_min` of the
// runs whose item changed hands): down to that wait; or more than `HoldRanOutShare` of the holds
// ran out: down to the median of the holds that were lifted (0 when none was). Rounded up to
// `HoldStep`.
func SuggestHoldMinutes(ctx *hctx.Context, f *Facts) *Suggestion {
	if !ctx.Cfg.Holds() {
		return nil
	}
	from := f.Now.Add(-days(SuggestDays))
	var holds []HoldSpan
	for _, p := range subscriptionsOf(ctx, f) {
		holds = append(holds, Holds(f.of().ticks[p.ID], from, f.Now, ctx.Cfg.HoldMinutes)...)
	}
	if len(holds) < HoldMinHolds {
		return nil
	}
	var lengths, lifted, waits []float64
	ranOut := 0
	for _, h := range holds {
		lengths = append(lengths, h.Minutes)
		if h.RanOut {
			ranOut++
		} else {
			lifted = append(lifted, h.Minutes)
		}
	}
	for _, p := range subscriptionsOf(ctx, f) {
		for _, run := range runsSince(f, p.ID, from) {
			if run.HandedOff != "" && run.QueueWaitMin.OK {
				waits = append(waits, run.QueueWaitMin.V)
			}
		}
	}
	current := float64(ctx.Cfg.HoldMinutes)
	suggested := current
	var why []string
	held, _ := median(lengths)
	if wait, ok := median(waits); ok && held > wait {
		suggested = min(suggested, math.Ceil(wait/HoldStep)*HoldStep)
		why = append(why, fmt.Sprintf("held items waited a median %s minutes, longer than a handed-off item's %s",
			N(round1(held)).String(), N(round1(wait)).String()))
	}
	if float64(ranOut) > HoldRanOutShare*float64(len(holds)) {
		down := 0.0
		if m, ok := median(lifted); ok {
			down = math.Ceil(m/HoldStep) * HoldStep
		}
		suggested = min(suggested, down)
		why = append(why, fmt.Sprintf("%d of the %d holds ran out", ranOut, len(holds)))
	}
	if len(why) == 0 || suggested >= current {
		return nil
	}
	evidence := fmt.Sprintf("over %d days: %s", SuggestDays, why[0])
	if len(why) > 1 {
		evidence += "; " + why[1]
	}
	return &Suggestion{Key: "hold_minutes (config.json)", Now: N(current).String(),
		Suggested: N(suggested).String(), Evidence: evidence}
}

// SuggestPlanLoad: the plan load may need to move when a medium subscription reached its weekly cap
// before day `PlanLoadDay` of its week in each of its last `PlanLoadWeeks` closed weeks: its plans
// back to Opus (`plan` out of its `roles`); or the reverse, when it plans nothing and left at least
// `PlanLoadSpare` percent of each of those weeks unused, never capped.
func SuggestPlanLoad(ctx *hctx.Context, f *Facts, p *providers.Provider) *Suggestion {
	if p.Tier != "medium" {
		return nil
	}
	weeks := f.of().weeks[p.ID]
	if len(weeks) < PlanLoadWeeks {
		return nil
	}
	weeks = weeks[len(weeks)-PlanLoadWeeks:]
	early, spare := true, true
	var when []string
	for _, w := range weeks {
		day := weekDay(w)
		early = early && day > 0 && day < PlanLoadDay
		spare = spare && day == 0 && w.UnusedAtReset.OK && w.UnusedAtReset.V >= PlanLoadSpare
		if day > 0 {
			when = append(when, fmt.Sprintf("day %d", day))
		} else {
			when = append(when, pctText(w.UnusedAtReset.V)+" unused")
		}
	}
	plans := len(p.Roles) == 0 || slices.Contains(p.Roles, "plan")
	listed := fmt.Sprintf("its last %d weeks: %s", PlanLoadWeeks, joinAnd(when))
	switch {
	case early && plans:
		return &Suggestion{Subscription: p.ID, Key: "roles", Now: "plans",
			Suggested: "without `plan`: its plans back to Opus",
			Evidence:  fmt.Sprintf("reached its weekly cap before day %d in %s", PlanLoadDay, listed)}
	case spare && !plans:
		return &Suggestion{Subscription: p.ID, Key: "roles", Now: "does not plan",
			Suggested: "with `plan`: plans from Opus to it",
			Evidence: fmt.Sprintf("never reached its weekly cap, with at least %s unused, in %s",
				pctText(PlanLoadSpare), listed)}
	}
	return nil
}

// weekDay is the day of its week (1 to 7) a closed week first reached its cap, 0 when it never did.
func weekDay(w *Week) int {
	if w.FirstCappedAt.IsZero() {
		return 0
	}
	return int(w.FirstCappedAt.Sub(w.ResetAt.Add(-WeekSpan)).Hours()/24) + 1
}

// joinAnd is `a`, `a and b`, `a, b and c`.
func joinAnd(parts []string) string {
	switch len(parts) {
	case 0:
		return ""
	case 1:
		return parts[0]
	}
	out := ""
	for i, part := range parts[:len(parts)-1] {
		if i > 0 {
			out += ", "
		}
		out += part
	}
	return out + " and " + parts[len(parts)-1]
}
