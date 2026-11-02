package usage

import (
	"math"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"
)

// ---------------------------------------------------------------------- what the issue reads

// Reach is how far back the usage issue reads the history: the longest any of its sections looks
// (section 3's four weeks), with a week more, so the oldest week drawn has its start. TicksReach
// is the ticks' (the largest file): the longest a section over ticks looks (sections 4 and 7's
// fourteen days, the suggestions').
var (
	Reach      = 5 * WeekSpan
	TicksReach = 14 * 24 * time.Hour
)

// Facts is what the usage issue is drawn from, read once per drawing from a clone of the usage
// branch (`ReadFacts`) or, in the tests, built from rows: the rows of each kind and the names of
// the files under `usage/`. Each list may come in any order; the drawing sorts its own copies.
type Facts struct {
	Now      time.Time
	Readings []*Reading
	Ticks    []*Tick
	Pools    []*PoolTick
	Sessions []*Session
	Weeks    []*Week
	Runs     []*Run
	// Files is the names of the files under `usage/` (`readings-2026-11.csv`, …), sorted.
	Files []string

	ix *index
}

// ReadFacts is the history under `root` (a clone of the usage branch) as of `now`, as far back as
// the issue draws (`Reach`, `TicksReach`; weeks whole).
func ReadFacts(root string, now time.Time) (*Facts, error) {
	f := &Facts{Now: utc(now)}
	var err error
	from := f.Now.Add(-Reach)
	if f.Readings, err = ReadReadings(root, from, time.Time{}); err != nil {
		return nil, err
	}
	if f.Ticks, err = ReadTicks(root, f.Now.Add(-TicksReach), time.Time{}); err != nil {
		return nil, err
	}
	if f.Pools, err = ReadPools(root, f.Now.Add(-TicksReach), time.Time{}); err != nil {
		return nil, err
	}
	if f.Sessions, err = ReadSessions(root, from, time.Time{}); err != nil {
		return nil, err
	}
	if f.Weeks, err = ReadWeeks(root, time.Time{}, time.Time{}); err != nil {
		return nil, err
	}
	if f.Runs, err = ReadRuns(root, from, time.Time{}); err != nil {
		return nil, err
	}
	entries, err := os.ReadDir(filepath.Join(root, Dir))
	if err != nil && !os.IsNotExist(err) {
		return nil, err
	}
	for _, entry := range entries {
		if !entry.IsDir() && (strings.HasSuffix(entry.Name(), ".csv") || strings.HasSuffix(entry.Name(), ".json")) {
			f.Files = append(f.Files, entry.Name())
		}
	}
	sort.Strings(f.Files)
	return f, nil
}

// index is the facts by subscription, each list oldest first, and what the sections and the
// suggestions derive from them more than once: each run's readings, the runs' spans and the
// rises outside them. Built once, on first use (`Facts.of`).
type index struct {
	readings map[string][]*Reading
	ticks    map[string][]*Tick
	runs     map[string][]*Run
	sessions map[string][]*Session
	weeks    map[string][]*Week
	pools    []*PoolTick
	byRun    map[string][]*Reading
	spans    map[string][][2]time.Time
	rises    map[string][]Rise
	// suggested is `Suggestions`, once drawn.
	suggested []*Suggestion
}

// of is the facts' index.
func (f *Facts) of() *index {
	if f.ix != nil {
		return f.ix
	}
	x := &index{readings: map[string][]*Reading{}, ticks: map[string][]*Tick{}, runs: map[string][]*Run{},
		sessions: map[string][]*Session{}, weeks: map[string][]*Week{}, byRun: map[string][]*Reading{},
		rises: map[string][]Rise{}}
	readings := sortedBy(f.Readings, func(r *Reading) time.Time { return r.ObservedAt })
	for _, r := range readings {
		x.readings[r.Subscription] = append(x.readings[r.Subscription], r)
		if r.RunID != "" {
			x.byRun[r.RunID] = append(x.byRun[r.RunID], r)
		}
	}
	for _, t := range sortedBy(f.Ticks, func(t *Tick) time.Time { return t.At }) {
		x.ticks[t.Subscription] = append(x.ticks[t.Subscription], t)
	}
	runs := sortedBy(f.Runs, func(r *Run) time.Time { return r.EndedAt })
	for _, r := range runs {
		x.runs[r.Subscription] = append(x.runs[r.Subscription], r)
	}
	for _, s := range sortedBy(f.Sessions, func(s *Session) time.Time { return s.ResetAt }) {
		x.sessions[s.Subscription] = append(x.sessions[s.Subscription], s)
	}
	for _, w := range sortedBy(f.Weeks, func(w *Week) time.Time { return w.ResetAt }) {
		x.weeks[w.Subscription] = append(x.weeks[w.Subscription], w)
	}
	x.pools = sortedBy(f.Pools, func(p *PoolTick) time.Time { return p.At })
	x.spans = RunSpans(runs, readings)
	for sub, list := range x.readings {
		x.rises[sub] = RisesOutside(list, x.spans[sub])
	}
	f.ix = x
	return x
}

// sortedBy is a copy of `rows`, sorted stably by `at`.
func sortedBy[T any](rows []T, at func(T) time.Time) []T {
	out := append([]T(nil), rows...)
	sort.SliceStable(out, func(i, j int) bool { return at(out[i]).Before(at(out[j])) })
	return out
}

// since is the rows of `rows` (oldest first) at or after `from`.
func since[T any](rows []T, at func(T) time.Time, from time.Time) []T {
	i := sort.Search(len(rows), func(i int) bool { return !at(rows[i]).Before(from) })
	return rows[i:]
}

// ---------------------------------------------------------------------- runs and rises

// RunSpans is when each subscription's runs were going, by subscription: from a run's first
// reading (its start ping) to its last, or, for one without two readings of its own, from its
// model minutes before its end to its end. A run's last reading is its last call's end, and
// nothing it does after (its delivery) spends the subscription.
func RunSpans(runs []*Run, readings []*Reading) map[string][][2]time.Time {
	first, last := map[string]time.Time{}, map[string]time.Time{}
	for _, r := range readings {
		if r.RunID == "" {
			continue
		}
		if at, ok := first[r.RunID]; !ok || r.ObservedAt.Before(at) {
			first[r.RunID] = r.ObservedAt
		}
		if at, ok := last[r.RunID]; !ok || r.ObservedAt.After(at) {
			last[r.RunID] = r.ObservedAt
		}
	}
	out := map[string][][2]time.Time{}
	for _, run := range runs {
		start, end := first[run.RunID], last[run.RunID]
		if run.RunID == "" || start.IsZero() || !end.After(start) {
			start = run.EndedAt.Add(-time.Duration(run.ModelMinutes.V * float64(time.Minute)))
			end = run.EndedAt
		}
		out[run.Subscription] = append(out[run.Subscription], [2]time.Time{start, end})
	}
	return out
}

// Rise is a rise of a subscription's 5-hour reading while none of its runs was going: someone
// else's use of the account (the owner's or another harness's, section 12), or of the reserve a
// cap leaves (sections 5 and 15).
type Rise struct {
	At     time.Time
	Points float64
}

// RisePoints is the least a reading must rise to count: a Claude reading is in whole points.
var RisePoints = 1.0

// RisesOutside is each rise of a subscription's 5-hour usage between two of its readings (oldest
// first) while none of its runs (`spans`) was going: from the earlier reading in the same session,
// or from 0 when its session reset between them (from the later one's start). Readings with no
// reset to tell their session by name no rise.
func RisesOutside(readings []*Reading, spans [][2]time.Time) []Rise {
	var out []Rise
	var prev *Reading
	for _, r := range readings {
		if !r.Util5h.OK {
			continue
		}
		last := prev
		prev = r
		if last == nil || last.Resets5h.IsZero() || r.Resets5h.IsZero() {
			continue
		}
		base, from := last.Util5h.V, last.ObservedAt
		switch {
		case absDuration(r.Resets5h.Sub(last.Resets5h)) <= ResetSlack:
		case !last.Resets5h.After(r.ObservedAt):
			base, from = 0, last.Resets5h
			if start := r.Resets5h.Add(-SessionSpan); start.After(from) {
				from = start
			}
		default:
			continue
		}
		points := round1(r.Util5h.V - base)
		if points < RisePoints || !r.ObservedAt.After(from) || during(spans, from, r.ObservedAt) {
			continue
		}
		out = append(out, Rise{At: r.ObservedAt, Points: points})
	}
	return out
}

// during is: one of `spans` was going at some time in (from, to).
func during(spans [][2]time.Time, from, to time.Time) bool {
	for _, s := range spans {
		if s[0].Before(to) && s[1].After(from) {
			return true
		}
	}
	return false
}

// risen is the points `rises` rose in [from, to).
func risen(rises []Rise, from, to time.Time) float64 {
	total := 0.0
	for _, r := range rises {
		if in(r.At, from, to) {
			total += r.Points
		}
	}
	return round1(total)
}

// ---------------------------------------------------------------------- holds

// HoldSpan is one stretch of ticks in which items were held for a subscription (`held_for_it`,
// `bot:waiting`, #2748): when it began, how many minutes it lasted, and whether it lasted the
// whole hold (`hold_minutes`, less a tick of the loop: `HoldTick`), so ran out rather than being
// lifted by its owner or released by a person.
type HoldSpan struct {
	Subscription string
	From         time.Time
	Minutes      float64
	RanOut       bool
}

// HoldTick is how far short of `hold_minutes` a hold may end and still count as run out: the
// loop sees a hold begin and end a tick late at most.
var HoldTick = 5 * time.Minute

// Holds is the holds `ticks` (one subscription's, oldest first) show in [from, to), against a
// hold of `holdMinutes`.
func Holds(ticks []*Tick, from, to time.Time, holdMinutes int) []HoldSpan {
	var out []HoldSpan
	var end time.Time
	eachSpan(ticks, func(t *Tick) time.Time { return t.At }, from, to, func(t *Tick, lo, hi time.Time) {
		if t.HeldForIt.V <= 0 {
			return
		}
		if len(out) > 0 && !lo.After(end) {
			out[len(out)-1].Minutes += hi.Sub(lo).Minutes()
		} else {
			out = append(out, HoldSpan{Subscription: t.Subscription, From: lo, Minutes: hi.Sub(lo).Minutes()})
		}
		end = hi
	})
	full := (time.Duration(holdMinutes)*time.Minute - HoldTick).Minutes()
	for i := range out {
		out[i].Minutes = round1(out[i].Minutes)
		out[i].RanOut = holdMinutes > 0 && out[i].Minutes >= full
	}
	return out
}

// ---------------------------------------------------------------------- numbers

// median is the median of `values`, false for none.
func median(values []float64) (float64, bool) {
	if len(values) == 0 {
		return 0, false
	}
	sorted := append([]float64(nil), values...)
	sort.Float64s(sorted)
	n := len(sorted)
	if n%2 == 1 {
		return sorted[n/2], true
	}
	return (sorted[n/2-1] + sorted[n/2]) / 2, true
}

// percentile is the nearest-rank `share` percentile of `values` (0.9: the 90th), false for none.
func percentile(values []float64, share float64) (float64, bool) {
	if len(values) == 0 {
		return 0, false
	}
	sorted := append([]float64(nil), values...)
	sort.Float64s(sorted)
	rank := int(math.Ceil(share*float64(len(sorted)))) - 1
	return sorted[min(max(rank, 0), len(sorted)-1)], true
}

// trend is the least-squares slope of `ys` over `xs`, false when the `xs` do not spread.
func trend(xs, ys []float64) (float64, bool) {
	n := float64(len(xs))
	if len(xs) < 2 {
		return 0, false
	}
	var sx, sy, sxx, sxy float64
	for i := range xs {
		sx, sy, sxx, sxy = sx+xs[i], sy+ys[i], sxx+xs[i]*xs[i], sxy+xs[i]*ys[i]
	}
	d := n*sxx - sx*sx
	if d <= 0 {
		return 0, false
	}
	return (n*sxy - sx*sy) / d, true
}
