package usage

import (
	"regexp"
	"slices"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// ---------------------------------------------------------------------- a run's readings

// ResultKey is where a run's readings go in its `result.json`: a list of readings, numbers and
// times only (`Note`), which deliver hands on to the history (`ReadingsOf`).
var ResultKey = "readings"

// MaxRunReadings is the most readings one run's result keeps; past it, each newer reading takes
// the last one's place, so the first ones and the newest stay.
var MaxRunReadings = 400

// A run keeps a reading only when it says something its last kept one did not: a window's
// utilization moved by `ThinPoints` or more, a reset, its status, a cap in force or a refusal
// changed, or `ThinEvery` passed since. A call's watch sees a reading every 15 seconds, most of
// them a tenth of a point apart, so this keeps a run to a few dozen rows.
var (
	ThinPoints = 1.0
	ThinEvery  = 10 * time.Minute
)

// FactsKey is where a claim puts what the run row needs from it in the plan (`plan.UsageFacts`).
var FactsKey = plan.UsageFacts

// Note is `list` (a run's readings so far, as its result carries them) with one more: `reading`
// (as a runner reads it: `{five_hour: {utilization, resets_at}, seven_day: {…}, status}`), read at
// `at` from `source` `during` the run, with `caps`, the caps in force then (window to share,
// `providers.Provider.CapsAt`), and, for a refusal, when it ends. One that says what the run's last
// reading said adds nothing.
func Note(list []any, at time.Time, source, during string, reading *pyjson.Object, caps *pyjson.Object,
	refusedUntil time.Time) []any {
	row := FromUsage(reading, at, "", source, during, "", caps)
	row.RefusedUntil = utc(refusedUntil)
	if row.Status == "" && source == "refusal" {
		row.Status = "rejected"
	}
	if len(list) > 0 {
		if last := readingOf(list[len(list)-1]); last != nil && !newsIn(row, last) {
			return list
		}
	}
	entry := pyjson.New("observed_at", ISO(row.ObservedAt), "source", row.Source, "during", row.During,
		"util_5h", numOrNone(row.Util5h), "resets_5h", isoOrNone(row.Resets5h),
		"util_7d", numOrNone(row.Util7d), "resets_7d", isoOrNone(row.Resets7d),
		"status", strOrNone(row.Status), "cap_5h", numOrNone(row.Cap5h), "cap_7d", numOrNone(row.Cap7d),
		"refused_until", isoOrNone(row.RefusedUntil))
	if len(list) >= MaxRunReadings {
		out := append([]any(nil), list[:MaxRunReadings-1]...)
		return append(out, entry)
	}
	return append(list, entry)
}

// newsIn is: `row` says something `last` did not (`ThinPoints`, `ThinEvery`).
func newsIn(row, last *Reading) bool {
	moved := func(a, b Num) bool { return a.OK != b.OK || (a.OK && absFloat(a.V-b.V) >= ThinPoints) }
	if moved(row.Util5h, last.Util5h) || moved(row.Util7d, last.Util7d) ||
		row.ObservedAt.Sub(last.ObservedAt) >= ThinEvery {
		return true
	}
	a, b := *row, *last
	a.Util5h, b.Util5h, a.Util7d, b.Util7d = Num{}, Num{}, Num{}, Num{}
	return !a.same(&b)
}

func absFloat(v float64) float64 {
	if v < 0 {
		return -v
	}
	return v
}

// FromUsage is a reading row from `reading` (a runner's or the state file's, windows as shares),
// read at `at`, with the caps in force then (window to share; nil or {} for none).
func FromUsage(reading *pyjson.Object, at time.Time, subscription, source, during, runID string,
	caps *pyjson.Object) *Reading {
	row := &Reading{ObservedAt: utc(at), Subscription: subscription, Source: source, During: during,
		RunID: runID}
	window := func(name string) (Num, time.Time) {
		w := reading.Obj(name)
		if w == nil {
			return Num{}, time.Time{}
		}
		var util Num
		if share, isNumber := py.Number(w.Get("utilization")); isNumber {
			util = Pct(share)
		}
		return util, utc(clock.ParseISOValue(w.Get("resets_at")))
	}
	row.Util5h, row.Resets5h = window("five_hour")
	row.Util7d, row.Resets7d = window("seven_day")
	if status, ok := reading.Get("status").(string); ok {
		row.Status = status
	}
	row.Cap5h, row.Cap7d = capOf(caps, "five_hour"), capOf(caps, "seven_day")
	return row
}

// capOf is the cap `caps` holds for `window`, as a percent; unknown when it holds none.
func capOf(caps *pyjson.Object, window string) Num {
	if share, isNumber := py.Number(caps.Get(window)); isNumber {
		return Pct(share)
	}
	return Num{}
}

// ReadingsOf is the readings a run's result carries (`Note`), as rows of `subscription`'s run
// `runID`. A result from before the history, or one with none, has none.
func ReadingsOf(result *pyjson.Object, subscription, runID string) []*Reading {
	var out []*Reading
	for _, raw := range pyjson.List(result.Get(ResultKey)) {
		if row := readingOf(raw); row != nil && !row.ObservedAt.IsZero() {
			row.Subscription, row.RunID = subscription, runID
			out = append(out, row)
		}
	}
	return out
}

// readingOf is one of a result's readings as a row (no subscription or run yet), or nil.
func readingOf(raw any) *Reading {
	entry, ok := raw.(*pyjson.Object)
	if !ok || entry == nil {
		return nil
	}
	num := func(key string) Num {
		if v, isNumber := py.Number(entry.Get(key)); isNumber {
			return N(v)
		}
		return Num{}
	}
	when := func(key string) time.Time { return utc(clock.ParseISOValue(entry.Get(key))) }
	// Deliver trusts nothing of the model job's but what it can check: the words are the
	// history's own, or none.
	source, during := entry.Str("source"), entry.Str("during")
	if !slices.Contains(Sources, source) {
		source = ""
	}
	if !slices.Contains(Durings, during) {
		during = ""
	}
	return &Reading{ObservedAt: when("observed_at"), Source: source, During: during,
		Util5h: num("util_5h"), Resets5h: when("resets_5h"), Util7d: num("util_7d"), Resets7d: when("resets_7d"),
		Status: Word(entry.Str("status")), Cap5h: num("cap_5h"), Cap7d: num("cap_7d"),
		RefusedUntil: when("refused_until")}
}

// Sources and Durings are the words a reading's `source` and `during` may hold.
var (
	Sources = []string{"ping", "stream", "usage-command", "refusal"}
	Durings = []string{"before", "call", "after", "tick"}
)

// word is a short id: letters, digits and `._:+-`, at most 64 of them.
var word = regexp.MustCompile(`^[A-Za-z0-9._:+-]{1,64}$`)

// Word is `text` when it is a short id (a status, an outcome, a model), else "": the history
// holds ids and numbers, never text a model wrote.
func Word(text string) string {
	if word.MatchString(text) {
		return text
	}
	return ""
}

// RefusalEnd is when a refusal named by a runner (`RunResult.ResetAt`: a time, or `+PT…` from
// `at`) ends; the zero time when it names nothing readable.
func RefusalEnd(resetAt string, at time.Time) time.Time {
	if strings.HasPrefix(resetAt, "+PT") {
		return utc(at.Add(durationOf(resetAt)))
	}
	return utc(clock.ParseISO(resetAt))
}

// durationOf is `+PT1H30M` as a duration (`providers.Duration`'s reading, kept here so this file
// needs nothing of providers).
func durationOf(text string) time.Duration {
	text = strings.TrimPrefix(strings.ToUpper(py.Strip(text)), "+PT")
	var total time.Duration
	number := ""
	for _, r := range text {
		switch {
		case r >= '0' && r <= '9':
			number += string(r)
		case r == 'H' || r == 'M' || r == 'S':
			n, _ := strconv.Atoi(number)
			unit := map[rune]time.Duration{'H': time.Hour, 'M': time.Minute, 'S': time.Second}[r]
			total += time.Duration(n) * unit
			number = ""
		}
	}
	return total
}

// ---------------------------------------------------------------------- a run's row

// RunFacts is what deliver knows of a run as it delivers it.
type RunFacts struct {
	// At is when it is delivered, its end when its result names none.
	At           time.Time
	RunID        string
	Subscription string
	// Models is the models its seats ran.
	Models   []string
	Plan     *pyjson.Object
	Result   *pyjson.Object
	Readings []*Reading
}

// RunOf is a delivered run's row: the numbers its journal section gives, its readings at its start
// and end, and what its claim recorded (`FactsKey`).
func RunOf(f RunFacts) *Run {
	result, plan := f.Result, f.Plan
	status := Word(py.OrStr(result.Get("status"), "unknown"))
	interrupt := py.OrStr(result.Get("interrupt"), "")
	if interrupt == "died" {
		status = "died" // as the journal's section says it
	}
	ended := utc(clock.ParseISOValue(result.Get("finished_at")))
	if ended.IsZero() {
		ended = utc(f.At)
	}
	row := &Run{EndedAt: ended, RunID: f.RunID, Subscription: f.Subscription,
		Models: strings.Join(f.Models, "+"), Kind: py.OrStr(plan.Get("action"), ""),
		Difficulty: py.OrStr(plan.Get("difficulty"), ""), Outcome: status,
		StopReason: StopReason(status, interrupt, py.OrStr(result.Get("reason"), ""))}
	if n, err := py.Int(plan.Get("number")); err == nil && n != 0 {
		row.Item = strconv.Itoa(n)
	}
	if minutes, isNumber := py.Number(result.Get("minutes")); isNumber {
		row.ModelMinutes = N(round1(minutes))
	}
	if calls, isNumber := py.Number(result.Get("model_calls")); isNumber {
		row.ModelCalls = N(calls)
	}
	if tokens := result.Obj("tokens"); tokens.Len() > 0 {
		count := func(name string) float64 { v, _ := py.Number(tokens.Get(name)); return v }
		row.TokensIn, row.TokensOut = N(count("input")), N(count("output"))
		row.TokensCached = N(count("cache_read") + count("cache_write"))
	}
	if cost, isNumber := py.Number(result.Get("cost_usd")); isNumber && cost > 0 {
		row.USDReported = N(py.RoundN(cost, 4))
	}
	row.Util5hStart, row.Util5hEnd = ends(f.Readings, func(r *Reading) Num { return r.Util5h })
	row.Util7dStart, row.Util7dEnd = ends(f.Readings, func(r *Reading) Num { return r.Util7d })
	facts := plan.Obj(FactsKey)
	queued := clock.ParseISOValue(facts.Get("queued_at"))
	claimed := clock.ParseISOValue(facts.Get("claimed_at"))
	if !queued.IsZero() && !claimed.IsZero() && !claimed.Before(queued) {
		row.QueueWaitMin = N(round1(claimed.Sub(queued).Minutes()))
	}
	row.PreviousSubscription = py.OrStr(facts.Get("previous_provider"), "")
	row.HandedOff = HandoffReason(py.OrStr(facts.Get("handed_off"), ""))
	row.Session = "none"
	if session := result.Obj("session"); session != nil {
		row.Session = "fresh"
		if session.Truthy("resumed") {
			row.Session = "resumed"
		}
	}
	return row
}

// ends is the first and the last known value of `of` among `readings`, in their order.
func ends(readings []*Reading, of func(*Reading) Num) (Num, Num) {
	var first, last Num
	for _, r := range readings {
		if v := of(r); v.OK {
			if !first.OK {
				first = v
			}
			last = v
		}
	}
	return first, last
}

// StopReason is why a run stopped short, from its outcome, its interrupt's kind (the result's
// `interrupt`, "" when it has none, as no journal section names it) and its reason: 5-hour cap,
// weekly cap, refusal, time budget, halt, stop (a person's, a closed item, a suspension), infra
// or died; "" for a run that was not cut off.
func StopReason(status, interrupt, reason string) string {
	switch {
	case status == "died" || interrupt == "died":
		return "died"
	case status == "infra" || interrupt == "infra":
		return "infra"
	case interrupt == "budget" || strings.Contains(reason, "time budget"):
		return "time budget"
	case interrupt == "halt" || strings.HasPrefix(reason, "halted") || strings.Contains(reason, "was halted"):
		return "halt"
	case interrupt == "stop" || interrupt == "suspend" || status == "stopped" ||
		strings.Contains(reason, "was suspended"):
		return "stop"
	}
	if interrupt != "usage" && (interrupt != "" || status != "interrupted") {
		return ""
	}
	switch {
	case strings.Contains(reason, "5-hour usage") || strings.Contains(reason, "the 5-hour window"):
		return "5-hour cap"
	case strings.Contains(reason, "7-day usage") || strings.Contains(reason, "the 7-day window"):
		return "weekly cap"
	case strings.Contains(reason, "usage limit") || strings.Contains(reason, "refused"):
		return "refusal"
	}
	return ""
}

// HandoffReason is why an item came to a subscription from another, from its record's
// `owner_why` as the claim that took it up left it (`plan.takeOver`): person (a person's
// `/harness handoff`), time budget, cap (its owner's usage held it: a cap, a refusal, a hold
// that ran out), other (its hours, its lanes, a suspension, …), or "" when it did not change
// hands.
func HandoffReason(why string) string {
	lowered := py.Lower(why)
	switch {
	case lowered == "":
		return ""
	case strings.Contains(lowered, "released by @"):
		return "person"
	case strings.Contains(lowered, "time budget"):
		return "time budget"
	}
	for _, word := range []string{"usage", "cap", "refused", "hold", "limit"} {
		if strings.Contains(lowered, word) {
			return "cap"
		}
	}
	return "other"
}

// ---------------------------------------------------------------------- helpers

// utc is `t` as a cell holds it: UTC, to the second, no monotonic reading; the zero time stays
// zero.
func utc(t time.Time) time.Time {
	if t.IsZero() {
		return time.Time{}
	}
	return t.UTC().Truncate(time.Second)
}

func numOrNone(n Num) any {
	if !n.OK {
		return nil
	}
	return n.V
}

func isoOrNone(t time.Time) any {
	if t.IsZero() {
		return nil
	}
	return ISO(t)
}

func strOrNone(s string) any {
	if s == "" {
		return nil
	}
	return s
}
