package usage

import (
	"sort"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// SettleAfter is how long after its reset a session or a week is derived: a run's readings reach
// the history only once it is delivered and the loop next drains the spool, and a run lasts up to
// its job budget (`job_budget_minutes`, 330), so a window waits this long to collect them.
var SettleAfter = 6 * time.Hour

// DeriveLookback is how far back a loop's first derivation looks for windows to derive (the
// backfill's reach back to `BackfillSince` among them): one that reset longer ago was derived
// before, or has nothing to derive from. DeriveRecent is how far back each later flush of the
// loop looks: the windows that closed since its last one, with a margin, so a flush reads days of
// rows, not weeks.
var (
	DeriveLookback = 40 * 24 * time.Hour
	DeriveRecent   = 2 * 24 * time.Hour
)

// ResetSlack is how far apart two readings' 5-hour resets may be and still name one session:
// Claude names its reset to the second, but Muse's and agy's `/usage` say "resets in 2h 13m", so
// their reset moves by a minute or so between readings.
var ResetSlack = 10 * time.Minute

// WeekSlack is the same for the weekly window, whose `/usage` reset is coarser still.
var WeekSlack = 3 * time.Hour

// SessionSpan and WeekSpan are how long a window lasts: a session starts this long before its
// reset (`providers.Windows`).
var (
	SessionSpan = providers.Windows["five_hour"]
	WeekSpan    = providers.Windows["seven_day"]
)

// Unused at a reset (`unused_at_reset`, sessions and weeks alike) is 100 minus the last reading
// before the reset: the share of the whole window that was never spent, whatever cap held it.
// The share under the cap left unspent is `cap_max - util_last_read`, from the same row.

// Derive appends every session and week that reset within `lookback` of `now` and at least
// `SettleAfter` before it that the history does not hold yet, from the readings, ticks and runs it
// holds. Running it again adds nothing: a window is known by its subscription and its reset,
// within `ResetSlack` (`WeekSlack`). Returns how many sessions and weeks it added.
func (h *History) Derive(now time.Time, lookback time.Duration) (int, int, error) {
	from := now.Add(-lookback)
	reach := from.Add(-WeekSpan) // a week that reset at `from` began this long before
	readings, err := ReadReadings(h.Root, reach, time.Time{})
	if err != nil {
		return 0, 0, err
	}
	ticks, err := ReadTicks(h.Root, reach, time.Time{})
	if err != nil {
		return 0, 0, err
	}
	runs, err := ReadRuns(h.Root, reach, time.Time{})
	if err != nil {
		return 0, 0, err
	}
	haveSessions, err := ReadSessions(h.Root, from.Add(-ResetSlack), time.Time{})
	if err != nil {
		return 0, 0, err
	}
	haveWeeks, err := ReadWeeks(h.Root, time.Time{}, time.Time{})
	if err != nil {
		return 0, 0, err
	}
	sessions := DeriveSessions(readings, ticks, runs, haveSessions, now)
	weeks := DeriveWeeks(readings, ticks, runs, haveWeeks, now)
	var rows []Row
	added := [2]int{}
	for _, s := range sessions {
		if !s.ResetAt.Before(from) { // an older one is in the history already, or out of reach
			rows, added[0] = append(rows, s), added[0]+1
		}
	}
	for _, w := range weeks {
		if !w.ResetAt.Before(from) {
			rows, added[1] = append(rows, w), added[1]+1
		}
	}
	if _, err := h.Append(rows...); err != nil {
		return 0, 0, err
	}
	return added[0], added[1], nil
}

// window is one window of one subscription, as its readings name it: its reset and the readings
// whose reset is that one.
type window struct {
	subscription string
	reset        time.Time
	readings     []*Reading
}

// windows groups each subscription's readings by the reset they name (`resets` of each, zero for
// none): readings whose resets lie within `slack` of the first of a group are one window, which
// takes the reset most of them name (the earliest, on a tie). A reading taken at or after its own
// reset, or before its window began, names no window.
func windows(readings []*Reading, resets func(*Reading) time.Time, span, slack time.Duration) []*window {
	bySub := map[string][]*Reading{}
	var subs []string
	for _, r := range readings {
		at := resets(r)
		if at.IsZero() || !r.ObservedAt.Before(at) || r.ObservedAt.Before(at.Add(-span-slack)) {
			continue
		}
		if _, seen := bySub[r.Subscription]; !seen {
			subs = append(subs, r.Subscription)
		}
		bySub[r.Subscription] = append(bySub[r.Subscription], r)
	}
	var out []*window
	for _, sub := range subs {
		list := bySub[sub]
		sort.SliceStable(list, func(i, j int) bool { return resets(list[i]).Before(resets(list[j])) })
		var group []*Reading
		flush := func() {
			if len(group) == 0 {
				return
			}
			count := map[time.Time]int{}
			best := time.Time{}
			for _, r := range group {
				at := resets(r)
				count[at]++
				if best.IsZero() || count[at] > count[best] || (count[at] == count[best] && at.Before(best)) {
					best = at
				}
			}
			sort.SliceStable(group, func(i, j int) bool { return group[i].ObservedAt.Before(group[j].ObservedAt) })
			out = append(out, &window{subscription: sub, reset: best, readings: group})
			group = nil
		}
		for _, r := range list {
			if len(group) > 0 && resets(r).Sub(resets(group[0])) > slack {
				flush()
			}
			group = append(group, r)
		}
		flush()
	}
	return out
}

// known is: `have` holds a window of `sub` that resets within `slack` of `reset`.
func known[T any](have []*T, sub string, reset time.Time, slack time.Duration, of func(*T) (string, time.Time)) bool {
	for _, row := range have {
		s, at := of(row)
		if s == sub && absDuration(at.Sub(reset)) <= slack {
			return true
		}
	}
	return false
}

func absDuration(d time.Duration) time.Duration {
	if d < 0 {
		return -d
	}
	return d
}

// DeriveSessions is the 5-hour sessions `readings` name that reset at least `SettleAfter` before
// `now` and `have` does not hold, each from its readings, the ticks and the runs of its five hours:
//
//   - cap_min and cap_max: the lowest and highest 5-hour cap in force in its readings and ticks;
//   - peak_util, util_last_read and unused_at_reset (100 minus the last reading);
//   - hit_cap_at: its first reading at or over the cap in force then (100 where it had none), a
//     refusal, or a tick of it at or over its cap now;
//   - minutes_capped: its ticks' minutes in `capped` (each tick standing until the next, at most
//     `TickCovers`), and minutes_capped_with_work_queued those with work queued it may take;
//   - last_hour_used: how many points its usage rose in its last hour (`providers.LastHour`);
//   - runs, model_minutes, cut_offs (stopped at a 5-hour cap) and refusals: the runs that ended in it;
//   - overshoot_max: its highest reading minus the cap in force then (negative: it stayed under).
func DeriveSessions(readings []*Reading, ticks []*Tick, runs []*Run, have []*Session, now time.Time) []*Session {
	var out []*Session
	for _, w := range windows(readings, func(r *Reading) time.Time { return r.Resets5h }, SessionSpan, ResetSlack) {
		reset, sub := w.reset, w.subscription
		start := reset.Add(-SessionSpan)
		if now.Before(reset.Add(SettleAfter)) ||
			known(have, sub, reset, ResetSlack, func(s *Session) (string, time.Time) { return s.Subscription, s.ResetAt }) {
			continue
		}
		s := &Session{Subscription: sub, StartedAt: start, ResetAt: reset, ReadingsCount: N(float64(len(w.readings)))}
		var caps []float64
		var hit time.Time
		earliest := func(at time.Time) {
			if hit.IsZero() || at.Before(hit) {
				hit = at
			}
		}
		var peak, last, over Num
		for _, r := range w.readings {
			if r.Cap5h.OK {
				caps = append(caps, r.Cap5h.V)
			}
			if !r.Util5h.OK {
				continue
			}
			peak = maxNum(peak, r.Util5h)
			last = r.Util5h
			limit := 100.0
			if r.Cap5h.OK {
				limit = r.Cap5h.V
				over = maxNum(over, N(round1(r.Util5h.V-r.Cap5h.V)))
			}
			if r.Util5h.V >= limit {
				earliest(r.ObservedAt)
			}
		}
		if !peak.OK {
			continue // nothing read of it
		}
		for _, r := range readings {
			if r.Subscription == sub && r.Source == "refusal" && r.Resets5h.IsZero() && in(r.ObservedAt, start, reset) {
				earliest(r.ObservedAt)
			}
		}
		mine := ticksOf(ticks, sub)
		for _, t := range mine {
			if !in(t.At, start, reset) {
				continue
			}
			if t.Cap5hNow.OK {
				caps = append(caps, t.Cap5hNow.V)
				if t.Util5h.OK && t.Util5h.V >= t.Cap5hNow.V && absDuration(t.Resets5h.Sub(reset)) <= ResetSlack {
					earliest(t.At)
				}
			}
		}
		if len(caps) > 0 {
			low, high := caps[0], caps[0]
			for _, c := range caps {
				low, high = min(low, c), max(high, c)
			}
			s.CapMin, s.CapMax = N(low), N(high)
		}
		s.PeakUtil, s.UtilLastRead, s.UnusedAtReset = peak, last, N(round1(max(0, 100-last.V)))
		s.HitCapAt, s.OvershootMax = hit, over
		capped, queued := cappedMinutes(mine, start, reset)
		s.MinutesCapped, s.MinutesCappedQueued = N(round1(capped)), N(round1(queued))
		s.LastHourUsed = lastHourRise(w.readings, reset)
		count, minutes, cutOffs, refusals := 0, 0.0, 0, 0
		for _, r := range runs {
			if r.Subscription != sub || !in(r.EndedAt, start, reset) {
				continue
			}
			count++
			minutes += r.ModelMinutes.V
			switch r.StopReason {
			case "5-hour cap":
				cutOffs++
			case "refusal":
				refusals++
			}
		}
		s.Runs, s.ModelMinutes = N(float64(count)), N(round1(minutes))
		s.CutOffs, s.Refusals = N(float64(cutOffs)), N(float64(refusals))
		out = append(out, s)
	}
	return out
}

// DeriveWeeks is the weeks `readings` name that reset at least `SettleAfter` before `now` and
// `have` does not hold: peak_util, util_at_reset and unused_at_reset (100 minus it), first_capped_at
// (its first reading at or over its weekly cap in force, 100 where it had none, or a tick of it at
// or over its cap now), days_refused (the days, to a tenth, between a refusal and its end, from its
// readings and ticks, within the week), and runs, model_hours, tokens (in and out) and
// usd_reported of the runs that ended in it.
func DeriveWeeks(readings []*Reading, ticks []*Tick, runs []*Run, have []*Week, now time.Time) []*Week {
	var out []*Week
	for _, w := range windows(readings, func(r *Reading) time.Time { return r.Resets7d }, WeekSpan, WeekSlack) {
		reset, sub := w.reset, w.subscription
		start := reset.Add(-WeekSpan)
		if now.Before(reset.Add(SettleAfter)) ||
			known(have, sub, reset, WeekSlack, func(x *Week) (string, time.Time) { return x.Subscription, x.ResetAt }) {
			continue
		}
		week := &Week{Subscription: sub, ResetAt: reset, ReadingsCount: N(float64(len(w.readings)))}
		var peak, last Num
		var capped time.Time
		earliest := func(at time.Time) {
			if capped.IsZero() || at.Before(capped) {
				capped = at
			}
		}
		for _, r := range w.readings {
			if !r.Util7d.OK {
				continue
			}
			peak, last = maxNum(peak, r.Util7d), r.Util7d
			limit := 100.0
			if r.Cap7d.OK {
				limit = r.Cap7d.V
			}
			if r.Util7d.V >= limit {
				earliest(r.ObservedAt)
			}
		}
		if !peak.OK {
			continue
		}
		var refused [][2]time.Time
		for _, r := range readings {
			if r.Subscription == sub && r.RefusedUntil.After(r.ObservedAt) && in(r.ObservedAt, start, reset) {
				refused = append(refused, [2]time.Time{r.ObservedAt, r.RefusedUntil})
			}
		}
		for _, t := range ticksOf(ticks, sub) {
			if !in(t.At, start, reset) {
				continue
			}
			if t.RefusedUntil.After(t.At) {
				refused = append(refused, [2]time.Time{t.At, t.RefusedUntil})
			}
			if t.Cap7dNow.OK && t.Util7d.OK && t.Util7d.V >= t.Cap7dNow.V && absDuration(t.Resets7d.Sub(reset)) <= WeekSlack {
				earliest(t.At)
			}
		}
		week.PeakUtil, week.UtilAtReset, week.UnusedAtReset = peak, last, N(round1(max(0, 100-last.V)))
		week.FirstCappedAt = capped
		week.DaysRefused = N(round1(covered(refused, start, reset).Hours() / 24))
		count, minutes, tokens, usd, reported := 0, 0.0, 0.0, 0.0, false
		for _, r := range runs {
			if r.Subscription != sub || !in(r.EndedAt, start, reset) {
				continue
			}
			count++
			minutes += r.ModelMinutes.V
			tokens += r.TokensIn.V + r.TokensOut.V
			if r.USDReported.OK {
				usd, reported = usd+r.USDReported.V, true
			}
		}
		week.Runs, week.ModelHours, week.Tokens = N(float64(count)), N(round1(minutes/60)), N(tokens)
		if reported {
			week.USDReported = N(py.RoundN(usd, 2))
		}
		out = append(out, week)
	}
	return out
}

// in is: `at` falls in [from, to).
func in(at, from, to time.Time) bool { return !at.IsZero() && !at.Before(from) && at.Before(to) }

func maxNum(a, b Num) Num {
	if !a.OK || (b.OK && b.V > a.V) {
		return b
	}
	return a
}

// ticksOf is `sub`'s ticks, oldest first.
func ticksOf(ticks []*Tick, sub string) []*Tick {
	var out []*Tick
	for _, t := range ticks {
		if t.Subscription == sub {
			out = append(out, t)
		}
	}
	sort.SliceStable(out, func(i, j int) bool { return out[i].At.Before(out[j].At) })
	return out
}

// cappedMinutes is the minutes within [from, to) that `ticks` (one subscription's, oldest first)
// stood in `capped`, each until the next or `TickCovers`, and of them those with work queued that
// it may take.
func cappedMinutes(ticks []*Tick, from, to time.Time) (float64, float64) {
	capped, queued := 0.0, 0.0
	for i, t := range ticks {
		end := t.At.Add(TickCovers)
		if i+1 < len(ticks) && ticks[i+1].At.Before(end) {
			end = ticks[i+1].At
		}
		lo, hi := t.At, end
		if lo.Before(from) {
			lo = from
		}
		if hi.After(to) {
			hi = to
		}
		if !hi.After(lo) || States["capped"] != t.State {
			continue
		}
		minutes := hi.Sub(lo).Minutes()
		capped += minutes
		if t.QueuedForIt.V > 0 {
			queued += minutes
		}
	}
	return capped, queued
}

// lastHourRise is how many points a session's usage rose in its last hour (`providers.LastHour`
// before `reset`): its highest reading there over its last one before it (or its lowest there,
// when none came before); unknown when nothing was read in that hour.
func lastHourRise(readings []*Reading, reset time.Time) Num {
	from := reset.Add(-providers.LastHour)
	var base, low, high Num
	for _, r := range readings { // oldest first
		if !r.Util5h.OK {
			continue
		}
		if r.ObservedAt.Before(from) {
			base = r.Util5h
			continue
		}
		if !low.OK || r.Util5h.V < low.V {
			low = r.Util5h
		}
		high = maxNum(high, r.Util5h)
	}
	if !high.OK {
		return Num{}
	}
	if !base.OK {
		base = low
	}
	return N(round1(max(0, high.V-base.V)))
}

// covered is how much of [from, to) the `spans` cover, overlaps counted once.
func covered(spans [][2]time.Time, from, to time.Time) time.Duration {
	sort.SliceStable(spans, func(i, j int) bool { return spans[i][0].Before(spans[j][0]) })
	var total time.Duration
	var cursor time.Time
	for _, span := range spans {
		lo, hi := span[0], span[1]
		if lo.Before(from) {
			lo = from
		}
		if hi.After(to) {
			hi = to
		}
		if !cursor.IsZero() && lo.Before(cursor) {
			lo = cursor
		}
		if hi.After(lo) {
			total += hi.Sub(lo)
			cursor = hi
		}
	}
	return total
}
