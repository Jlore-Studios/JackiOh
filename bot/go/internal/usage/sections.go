package usage

import (
	"fmt"
	"math"
	"slices"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// The usage issue's sections (#2749, section 2), 1 to 16 in the issue's order, each folded under
// its heading (`dashboard.Fold`). A section whose data cannot exist yet (no week closed, no tick
// recorded) says so in a line rather than drawing an empty chart. Hours are the bot's time zone's.

// How far back each section looks.
var (
	// HeadlineDays is section 1's span, and LanesDays sections 8 and 9's: a week.
	HeadlineDays = 7
	LanesDays    = 7
	// RecentHours is section 2's span: two days.
	RecentHours = 48
	// WeeksShown is how many weeks sections 3 and 13 show.
	WeeksShown = 4
	// DaysShown is the span of the day-by-day sections (4, 5, 6, 7, 10, 11, 12 and 14): two weeks,
	// as the statistics' last two weeks (`stats.DaysShown`).
	DaysShown = 14
	// OvershootShare is section 11's percentile: the 90th.
	OvershootShare = 0.9
)

// SharedAccounts is the Claude accounts someone besides the bot spends (section 12): claude-1,
// which its owner and bright-bots-harness use too (its note in providers.json). A subscription the
// pool does not hold is left out.
var SharedAccounts = []string{"claude-1"}

// CutReasons are the stop reasons a cut-off is (section 4's second table): the usage stopped it.
var CutReasons = []string{"5-hour cap", "weekly cap", "refusal"}

// IdleReasons are section 7's reasons a subscription with room under its caps ran nothing (a tick
// of it that is neither capped nor outside its hours, with no run held), from the tick's state,
// in the order they stack from the top: nothing queued it may take (`idle`), no lane or runner slot
// free where it runs (`no-runner`), backing off, and suspended.
var IdleReasons = []string{"nothing queued for it", "no lane or runner slot free", "backing off", "suspended"}

// idleReason is the index in `IdleReasons` of why `t` ran nothing with room, -1 when it is not
// idle with room.
func idleReason(t *Tick) int {
	if t.RunsHeld.V > 0 || t.State == States["capped"] || t.State == States["closed"] {
		return -1
	}
	switch {
	case t.Suspended.V > 0:
		return 3
	case t.State == States["idle"]:
		return 0
	case t.State == States["full"]:
		return 1
	case t.State == States["broken"]:
		return 2
	}
	return -1
}

// Detail is how much the issue draws (`Fit`): the points of its hourly charts (sections 2, 8 and
// 9) are `HourStep` hours wide, those of its daily ones (3, 5, 6, 7, 10 and 12) `DayStep` days,
// and with `ClaudeHours` section 4 holds the Claude accounts alone.
type Detail struct {
	Name        string
	HourStep    int
	DayStep     int
	ClaudeHours bool
}

// Shrinks is the order the issue shrinks in while it would pass GitHub's limit on an issue's body
// (`dashboard.MaxBody`), each a named step: full detail, then fewer points in each hourly chart,
// then in each daily one, then section 4 for the Claude accounts alone. Past the last, whole
// sections are cut from the bottom (`Cuts`).
var Shrinks = []Detail{
	{Name: "full detail", HourStep: 1, DayStep: 1},
	{Name: "hourly charts at 2-hour points", HourStep: 2, DayStep: 1},
	{Name: "hourly charts at 3-hour points", HourStep: 3, DayStep: 1},
	{Name: "hourly charts at 6-hour points", HourStep: 6, DayStep: 1},
	{Name: "daily charts at 2-day points", HourStep: 6, DayStep: 2},
	{Name: "section 4 for the Claude accounts alone", HourStep: 6, DayStep: 2, ClaudeHours: true},
}

// section is one of the issue's sections: its number, heading, the summary it folds under and what
// draws it.
type section struct {
	n       int
	heading string
	summary string
	draw    func(d *drawing) []string
}

// sections is the issue's sections, in order.
var sections = []section{
	{1, "Headline", "Each subscription over the last 7 days", (*drawing).headline},
	{2, "5-hour usage, last 48 hours", "Each Claude account's readings and the cap in force, with its resets", (*drawing).fiveHour},
	{3, "Weekly usage, last 4 weeks", "Each subscription with a weekly window: its readings, its cap and its resets", (*drawing).weekly},
	{4, "Hour of day, last 14 days", "The mean 5-hour usage, the cut-offs and the share capped, by hour of day", (*drawing).hourOfDay},
	{5, "Left on the table", "The share of each session unused at its reset, beside readings that rose while no bot run went", (*drawing).leftOver},
	{6, "Time capped", "Minutes at or over the cap with work queued it may take: the cost of a cap", (*drawing).timeCapped},
	{7, "Idle with room", "Minutes with room under the caps and no run, and why", (*drawing).idle},
	{8, "Lanes and runners", "Lanes allowed and held per account, and the runner slots and planning lanes against their limits", (*drawing).lanes},
	{9, "The queue", "Items waiting by kind and difficulty, the wait to start, and the items held for their owner", (*drawing).queue},
	{10, "Cost", "Model minutes, tokens and reported USD per account per day, and per item approved", (*drawing).cost},
	{11, "Overshoot", "How far past its cap a run went when that cap stopped it", (*drawing).overshoot},
	{12, "The shared account", "Readings of the shared account that rose while no bot run went", (*drawing).shared},
	{13, "Weekly walls", "When each medium subscription reached its weekly cap, and the days it was refused", (*drawing).walls},
	{14, "Holds and handoffs", "Items held for their owner, handoffs by reason, and resumed sessions against fresh ones", (*drawing).holds},
	{15, "Tuning suggestions", "What the history suggests for providers.json: computed, never applied", (*drawing).suggestions},
	{16, "The raw data", "The branch's files, for a spreadsheet", (*drawing).raw},
}

// Kept are the sections never cut for room: the suggestions and the raw data.
var Kept = []int{15, 16}

// Cuts is the order whole sections are cut in when the last shrink is not enough: from the bottom,
// `Kept` aside.
func Cuts() []int {
	var out []int
	for i := len(sections) - 1; i >= 0; i-- {
		if !slices.Contains(Kept, sections[i].n) {
			out = append(out, sections[i].n)
		}
	}
	return out
}

// drawing is one drawing of the issue: the facts, the detail, and what it reads them with.
type drawing struct {
	ctx    *hctx.Context
	f      *Facts
	x      *index
	detail Detail
	zone   *time.Location
	now    time.Time
	subs   []*providers.Provider
}

func newDrawing(ctx *hctx.Context, f *Facts, detail Detail) *drawing {
	return &drawing{ctx: ctx, f: f, x: f.of(), detail: detail, zone: clock.Zone(ctx.Cfg.Timezone), now: f.Now,
		subs: subscriptionsOf(ctx, f)}
}

// subscriptionsOf is the subscriptions the issue shows, as the status issue's lanes table orders
// them: the Claude accounts by number, then `dashboard.LaneOrder`, then any other by id; each one
// switched on, or with rows in the facts.
func subscriptionsOf(ctx *hctx.Context, f *Facts) []*providers.Provider {
	x := f.of()
	var out []*providers.Provider
	for _, p := range ctx.Cfg.Pool.Ordered() {
		if p.Enabled || len(x.readings[p.ID]) > 0 || len(x.ticks[p.ID]) > 0 || len(x.runs[p.ID]) > 0 {
			out = append(out, p)
		}
	}
	rank := func(p *providers.Provider) (int, int, string) {
		if n, err := strconv.Atoi(strings.TrimPrefix(p.ID, "claude-")); err == nil && p.CLI == "claude" {
			return 0, n, ""
		}
		if i := slices.Index(dashboard.LaneOrder, p.ID); i >= 0 {
			return 1, i, ""
		}
		return 2, 0, p.ID
	}
	sort.SliceStable(out, func(i, j int) bool {
		ag, ar, aid := rank(out[i])
		bg, br, bid := rank(out[j])
		if ag != bg {
			return ag < bg
		}
		if ar != br {
			return ar < br
		}
		return aid < bid
	})
	return out
}

// claude is the subscriptions on the Claude CLI.
func (d *drawing) claude() []*providers.Provider {
	var out []*providers.Provider
	for _, p := range d.subs {
		if p.CLI == "claude" {
			out = append(out, p)
		}
	}
	return out
}

// ---------------------------------------------------------------------- text

const dash = "—"

// localText is `at` in the bot's time zone: `Mon 11-02 15:04`.
func localText(ctx *hctx.Context, at time.Time) string {
	return at.In(clock.Zone(ctx.Cfg.Timezone)).Format("Mon 01-02 15:04")
}

func (d *drawing) when(at time.Time) string { return localText(d.ctx, at) }

// clockOf is `at`'s local time of day: `15:04`, with its weekday when it is not today's.
func (d *drawing) clockOf(at time.Time) string {
	local, today := at.In(d.zone), d.now.In(d.zone)
	if local.YearDay() == today.YearDay() && local.Year() == today.Year() {
		return local.Format("15:04")
	}
	return local.Format("Mon 15:04")
}

// numText is a number with at most one decimal.
func numText(v float64) string { return N(round1(v)).String() }

// pointsText is `1 point`, `2.5 points`.
func pointsText(v float64) string {
	if round1(v) == 1 {
		return "1 point"
	}
	return numText(v) + " points"
}

// tokensText is a token count, short: `850`, `12k`, `3.4M`.
func tokensText(v float64) string {
	switch {
	case v >= 1e6:
		return N(py.RoundN(v/1e6, 1)).String() + "M"
	case v >= 1e3:
		return N(math.Round(v/1e3)).String() + "k"
	}
	return N(math.Round(v)).String()
}

// usdText is reported USD: `$970`, `$12.34`.
func usdText(v float64) string {
	if v >= 100 {
		return "$" + py.Thousands(int(math.Round(v)))
	}
	return "$" + strconv.FormatFloat(py.RoundN(v, 2), 'f', 2, 64)
}

// code is `id` in backticks.
func code(id string) string { return "`" + id + "`" }

// codes is each id in backticks, joined.
func codes(ids []string) string {
	out := make([]string, len(ids))
	for i, id := range ids {
		out[i] = code(id)
	}
	return strings.Join(out, ", ")
}

// note is a line in small print.
func note(text string) string { return "<sub>" + text + "</sub>" }

// anchor is a subscription's first row in section 15, which section 1 links.
func anchor(id string) string {
	if id == "" {
		id = "pool"
	}
	return "usage-suggestion-" + id
}

// ---------------------------------------------------------------------- 1. headline

func (d *drawing) headline() []string {
	from := d.now.Add(-days(HeadlineDays))
	lines := []string{
		"| Subscription | Sessions at its cap | Unused at reset (median) | Capped with work queued | Weekly peak | " +
			"Reached its weekly cap | Cut-offs | Model hours | Tokens (in and out) | Reported USD | Suggested change |",
		"|---|--:|--:|--:|--:|---|---|--:|--:|--:|---|"}
	first := map[string]*Suggestion{}
	for _, s := range Suggestions(d.ctx, d.f) {
		if _, seen := first[s.Subscription]; !seen && s.Subscription != "" {
			first[s.Subscription] = s
		}
	}
	for _, p := range d.subs {
		sessions := sessionsSince(d.f, p.ID, from)
		atCap, cell := 0, dash
		var unused []float64
		for _, s := range sessions {
			if !s.HitCapAt.IsZero() {
				atCap++
			}
			if s.UnusedAtReset.OK {
				unused = append(unused, s.UnusedAtReset.V)
			}
		}
		if len(sessions) > 0 {
			cell = fmt.Sprintf("%d of %d (%s)", atCap, len(sessions), pctText(100*float64(atCap)/float64(len(sessions))))
		}
		unusedCell := dash
		if m, ok := median(unused); ok {
			unusedCell = pctText(m)
		}
		cappedCell := dash
		if ticks := d.x.ticks[p.ID]; len(ticks) > 0 {
			_, queued := cappedMinutes(ticks, from, d.now)
			cappedCell = numText(queued/float64(HeadlineDays)) + " min a day"
		}
		peak, reached := Num{}, time.Time{}
		for _, r := range since(d.x.readings[p.ID], readingAt, from) {
			if !r.Util7d.OK {
				continue
			}
			peak = maxNum(peak, r.Util7d)
			limit := 100.0
			if r.Cap7d.OK {
				limit = r.Cap7d.V
			}
			if r.Util7d.V >= limit && reached.IsZero() {
				reached = r.ObservedAt
			}
		}
		for _, t := range since(d.x.ticks[p.ID], tickAt, from) {
			if t.Cap7dNow.OK && t.Util7d.OK && t.Util7d.V >= t.Cap7dNow.V && (reached.IsZero() || t.At.Before(reached)) {
				reached = t.At
			}
		}
		peakCell, reachedCell := dash, dash
		if peak.OK {
			peakCell, reachedCell = pctText(peak.V), "not this week"
		}
		if !reached.IsZero() {
			reachedCell = d.when(reached)
		}
		cuts := map[string]int{}
		var reasons []string
		minutes, tokens, usd, reported := 0.0, 0.0, 0.0, false
		for _, r := range runsSince(d.f, p.ID, from) {
			if r.StopReason != "" {
				if cuts[r.StopReason] == 0 {
					reasons = append(reasons, r.StopReason)
				}
				cuts[r.StopReason]++
			}
			minutes += r.ModelMinutes.V
			tokens += r.TokensIn.V + r.TokensOut.V
			if r.USDReported.OK {
				usd, reported = usd+r.USDReported.V, true
			}
		}
		cutCell := "none"
		if len(reasons) > 0 {
			parts := make([]string, len(reasons))
			for i, reason := range reasons {
				parts[i] = fmt.Sprintf("%s %d", reason, cuts[reason])
			}
			cutCell = strings.Join(parts, ", ")
		}
		usdCell := dash
		if reported {
			usdCell = usdText(usd)
		}
		suggested := ""
		if s := first[p.ID]; s != nil {
			suggested = fmt.Sprintf("[%s %s → %s](#%s)", code(s.Key), s.Now, s.Suggested, anchor(p.ID))
		}
		lines = append(lines, fmt.Sprintf("| %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |", code(p.ID), cell,
			unusedCell, cappedCell, peakCell, reachedCell, cutCell, numText(minutes/60), tokensText(tokens), usdCell,
			suggested))
	}
	return append(lines, "", note("Sessions and unused shares from `sessions-*.csv` (a session counts six hours "+
		"after its reset), minutes capped from `ticks-*.csv`, the weekly peak from `readings-*.csv`, the rest "+
		"from `runs-*.csv`. Reported USD is the Claude CLI's API-price figure, not a bill."))
}

func readingAt(r *Reading) time.Time { return r.ObservedAt }
func tickAt(t *Tick) time.Time       { return t.At }
func poolAt(p *PoolTick) time.Time   { return p.At }
func runAt(r *Run) time.Time         { return r.EndedAt }

// ---------------------------------------------------------------------- 2. 5-hour usage

// resetsIn is the resets `readings` (one subscription's) name that fall in [from, to).
func resetsIn(readings []*Reading, resets func(*Reading) time.Time, span, slack time.Duration, from, to time.Time) []time.Time {
	var out []time.Time
	for _, w := range windows(readings, resets, span, slack) {
		if in(w.reset, from, to) {
			out = append(out, w.reset)
		}
	}
	sort.Slice(out, func(i, j int) bool { return out[i].Before(out[j]) })
	return out
}

// capLine is, for each point of `b`, the cap in force: a tick's (`cap_5h_now` or `cap_7d_now`) in
// it or at its start, else a reading's (`cap_*_in_force`), else 100 (no cap).
func (d *drawing) capLine(id string, b Bounds, tickCap func(*Tick) Num, readingCap func(*Reading) Num) []float64 {
	fromTicks, known := atPoints(d.x.ticks[id], tickAt, func(t *Tick) (float64, bool) {
		if c := tickCap(t); c.OK {
			return c.V, true
		}
		return 100, true
	}, b)
	fromReadings, read := atPoints(d.x.readings[id], readingAt, func(r *Reading) (float64, bool) {
		c := readingCap(r)
		return c.V, c.OK
	}, b)
	out := make([]float64, b.Points())
	for i := range out {
		switch {
		case known[i]:
			out[i] = fromTicks[i]
		case read[i]:
			out[i] = fromReadings[i]
		default:
			out[i] = 100
		}
	}
	return out
}

func (d *drawing) fiveHour() []string {
	b := Hourly(d.now, d.zone, time.Duration(RecentHours)*time.Hour, d.detail.HourStep)
	labels := b.Labels(d.zone, "Mon 15h")
	var lines []string
	var unread []string
	for _, p := range d.claude() {
		readings := d.x.readings[p.ID]
		if len(since(readings, readingAt, b.From())) == 0 {
			unread = append(unread, p.ID)
			continue
		}
		values, _ := levels(readings, func(r *Reading) Num { return r.Util5h }, func(r *Reading) time.Time { return r.Resets5h }, b)
		caps := d.capLine(p.ID, b, func(t *Tick) Num { return t.Cap5hNow }, func(r *Reading) Num { return r.Cap5h })
		lines = append(lines, chart(p.ID+": 5-hour usage (green) and the cap in force (grey), %", labels, "%",
			max(100, highest(values, caps)), ReadingPalette, lineOf(values), lineOf(caps))...)
		var marks []string
		for _, at := range resetsIn(readings, func(r *Reading) time.Time { return r.Resets5h }, SessionSpan, ResetSlack, b.From(), d.now) {
			marks = append(marks, d.clockOf(at))
		}
		if len(marks) > 0 {
			lines = append(lines, note(p.ID+"'s resets: "+strings.Join(marks, ", ")+"."))
		}
		lines = append(lines, "")
	}
	if len(lines) == 0 {
		return []string{fmt.Sprintf("No Claude account was read in the last %d hours.", RecentHours)}
	}
	if len(unread) > 0 {
		lines = append(lines, fmt.Sprintf("No reading in the last %d hours: %s (a Claude account is read only by "+
			"its runs).", RecentHours, codes(unread)))
	}
	return lines
}

// ---------------------------------------------------------------------- 3. weekly usage

func (d *drawing) weekly() []string {
	b := Daily(d.now, d.zone, WeeksShown*7, d.detail.DayStep)
	labels := b.Labels(d.zone, "01-02")
	var lines []string
	for _, p := range d.subs {
		readings := d.x.readings[p.ID]
		weekly := false
		for _, r := range since(readings, readingAt, b.From()) {
			weekly = weekly || r.Util7d.OK
		}
		if !weekly {
			continue
		}
		values, _ := levels(readings, func(r *Reading) Num { return r.Util7d }, func(r *Reading) time.Time { return r.Resets7d }, b)
		caps := d.capLine(p.ID, b, func(t *Tick) Num { return t.Cap7dNow }, func(r *Reading) Num { return r.Cap7d })
		lines = append(lines, chart(p.ID+": weekly usage (green) and the weekly cap in force (grey), %", labels, "%",
			max(100, highest(values, caps)), ReadingPalette, lineOf(values), lineOf(caps))...)
		var marks []string
		for _, at := range resetsIn(readings, func(r *Reading) time.Time { return r.Resets7d }, WeekSpan, WeekSlack, b.From(), d.now) {
			marks = append(marks, d.when(at))
		}
		if len(marks) > 0 {
			lines = append(lines, note(p.ID+"'s weeks reset: "+strings.Join(marks, ", ")+"."))
		}
		lines = append(lines, "")
	}
	if len(lines) == 0 {
		return []string{fmt.Sprintf("No subscription read a weekly window in the last %d weeks.", WeeksShown)}
	}
	return lines
}

// ---------------------------------------------------------------------- 4. hour of day

func (d *drawing) hourOfDay() []string {
	b := Hourly(d.now, d.zone, days(DaysShown), 1)
	from := b.From()
	subs := d.subs
	if d.detail.ClaudeHours {
		subs = d.claude()
	}
	header := "| Subscription |"
	rule := "|---|"
	for h := range 24 {
		header += fmt.Sprintf(" %02d |", h)
		rule += ":-:|"
	}
	hourOf := func(at time.Time) int { return at.In(d.zone).Hour() }
	var usageRows, cutRows, cappedRows []string
	cutCounts := map[string][24]int{}
	cutTop := 0
	for _, p := range subs {
		var counts [24]int
		for _, r := range runsSince(d.f, p.ID, from) {
			if slices.Contains(CutReasons, r.StopReason) {
				counts[hourOf(r.EndedAt)]++
				cutTop = max(cutTop, counts[hourOf(r.EndedAt)])
			}
		}
		cutCounts[p.ID] = counts
	}
	for _, p := range subs {
		values, known := levels(d.x.readings[p.ID], func(r *Reading) Num { return r.Util5h },
			func(r *Reading) time.Time { return r.Resets5h }, b)
		var sum [24]float64
		var count [24]int
		for i := range values {
			if known[i] {
				sum[hourOf(b[i])] += values[i]
				count[hourOf(b[i])]++
			}
		}
		row := "| " + code(p.ID) + " |"
		for h := range 24 {
			cell := NoBlock
			if count[h] > 0 {
				cell = block(sum[h]/float64(count[h]), 100)
			}
			row += " " + cell + " |"
		}
		usageRows = append(usageRows, row)

		counts := cutCounts[p.ID]
		total := 0
		row = "| " + code(p.ID) + " |"
		for h := range 24 {
			cell := NoBlock
			if counts[h] > 0 {
				cell = block(float64(counts[h]), float64(cutTop))
			}
			total += counts[h]
			row += " " + cell + " |"
		}
		cutRows = append(cutRows, row+fmt.Sprintf(" %d |", total))

		covered, capped := make([]float64, b.Points()), make([]float64, b.Points())
		eachSpan(d.x.ticks[p.ID], tickAt, from, d.now, func(t *Tick, lo, hi time.Time) {
			b.AddMinutes(covered, lo, hi, 1)
			if t.State == States["capped"] {
				b.AddMinutes(capped, lo, hi, 1)
			}
		})
		var coveredBy, cappedBy [24]float64
		for i := range covered {
			coveredBy[hourOf(b[i])] += covered[i]
			cappedBy[hourOf(b[i])] += capped[i]
		}
		row = "| " + code(p.ID) + " |"
		for h := range 24 {
			cell := NoBlock
			if coveredBy[h] > 0 {
				cell = block(cappedBy[h]/coveredBy[h], 1)
			}
			row += " " + cell + " |"
		}
		cappedRows = append(cappedRows, row)
	}
	lines := []string{fmt.Sprintf("**The mean 5-hour usage at each hour** (`readings-*.csv`; %s is 0%%, %s 100%%, "+
		"%s nothing read):", Blocks[0], Blocks[len(Blocks)-1], NoBlock), "", header, rule}
	lines = append(append(lines, usageRows...), "",
		fmt.Sprintf("**Cut-offs at each hour** (`runs-*.csv`, stopped by a 5-hour cap, a weekly cap or a refusal; "+
			"%s is the most in any hour, %d, %s none):", Blocks[len(Blocks)-1], cutTop, NoBlock), "",
		header+" All |", rule+"--:|")
	lines = append(append(lines, cutRows...), "",
		fmt.Sprintf("**The share of each hour spent capped** (`ticks-*.csv`, state `capped`; %s all of it, %s no "+
			"tick):", Blocks[len(Blocks)-1], NoBlock), "", header, rule)
	lines = append(lines, cappedRows...)
	if d.detail.ClaudeHours {
		lines = append(lines, "", note("The Claude accounts alone, to keep this issue under GitHub's limit."))
	}
	return lines
}

// ---------------------------------------------------------------------- 5. left on the table

// LeftPalette: the unused share at reset in grey bars, the points risen outside the bot's runs as
// a red line.
var LeftPalette = "#8c959f, #cf222e"

func (d *drawing) leftOver() []string {
	b := Daily(d.now, d.zone, DaysShown, d.detail.DayStep)
	labels := b.Labels(d.zone, "01-02")
	var lines, none []string
	closed := false
	for _, p := range d.subs {
		sum, count := make([]float64, b.Points()), make([]int, b.Points())
		for _, s := range sessionsSince(d.f, p.ID, b.From()) {
			if i := b.Index(s.ResetAt); i >= 0 && s.UnusedAtReset.OK {
				sum[i] += s.UnusedAtReset.V
				count[i]++
				closed = true
			}
		}
		rose := make([]float64, b.Points())
		for _, r := range d.x.rises[p.ID] {
			if i := b.Index(r.At); i >= 0 {
				rose[i] += r.Points
			}
		}
		unused := make([]float64, b.Points())
		some := false
		for i := range unused {
			if count[i] > 0 {
				unused[i], some = sum[i]/float64(count[i]), true
			}
		}
		if !some && !any0(rose) {
			none = append(none, p.ID)
			continue
		}
		lines = append(lines, chart(p.ID+": unused at each reset (grey bars, % of the session, the day's mean) "+
			"and points risen while no bot run went (red)", labels, "%", max(100, highest(unused, rose)), LeftPalette,
			barOf(unused), lineOf(rose))...)
		lines = append(lines, "")
	}
	if !closed && len(lines) == 0 {
		return []string{fmt.Sprintf("No 5-hour session has closed in the last %d days yet (a session counts six "+
			"hours after its reset).", DaysShown)}
	}
	if len(none) > 0 {
		lines = append(lines, "No session closed and no reading rose outside the bot's runs: "+codes(none)+".", "")
	}
	return append(lines, note("Unused at reset is `unused_at_reset` (`sessions-*.csv`): 100 less the last reading. "+
		"A rise outside the bot's runs is one between two readings (`readings-*.csv`) while none of its runs "+
		"(`runs-*.csv`, from its first reading to its last) went: a reserve someone uses shows it, one nobody "+
		"uses does not."))
}

// ---------------------------------------------------------------------- 6. time capped

func (d *drawing) timeCapped() []string {
	b := Daily(d.now, d.zone, DaysShown, d.detail.DayStep)
	labels := b.Labels(d.zone, "01-02")
	var lines, never []string
	ticked := false
	for _, p := range d.subs {
		ticks := since(d.x.ticks[p.ID], tickAt, b.From().Add(-TickCovers))
		ticked = ticked || len(ticks) > 0
		minutes := make([]float64, b.Points())
		eachSpan(ticks, tickAt, b.From(), d.now, func(t *Tick, lo, hi time.Time) {
			if t.State == States["capped"] && t.QueuedForIt.V > 0 {
				b.AddMinutes(minutes, lo, hi, 1)
			}
		})
		if !any0(minutes) {
			never = append(never, p.ID)
			continue
		}
		lines = append(lines, chart(p.ID+": minutes at its cap with work queued for it, per day", labels, "minutes",
			0, "", barOf(minutes))...)
		lines = append(lines, "")
	}
	if !ticked {
		return []string{fmt.Sprintf("No tick recorded in the last %d days yet.", DaysShown)}
	}
	if len(never) > 0 {
		lines = append(lines, "Never capped with work queued for it: "+codes(never)+".", "")
	}
	return append(lines, note("From `ticks-*.csv`: minutes in state `capped` (over a cap, within `start_headroom` "+
		"of one, refused) while `queued_for_it` was above 0, each tick standing until the next."))
}

// ---------------------------------------------------------------------- 7. idle with room

func (d *drawing) idle() []string {
	b := Daily(d.now, d.zone, DaysShown, d.detail.DayStep)
	labels := b.Labels(d.zone, "01-02")
	var lines, busy []string
	ticked := false
	for _, p := range d.subs {
		ticks := since(d.x.ticks[p.ID], tickAt, b.From().Add(-TickCovers))
		ticked = ticked || len(ticks) > 0
		by := make([][]float64, len(IdleReasons))
		for i := range by {
			by[i] = make([]float64, b.Points())
		}
		eachSpan(ticks, tickAt, b.From(), d.now, func(t *Tick, lo, hi time.Time) {
			if reason := idleReason(t); reason >= 0 {
				b.AddMinutes(by[reason], lo, hi, 1)
			}
		})
		if !any0(by...) {
			busy = append(busy, p.ID)
			continue
		}
		// Stacked: each bar is the sum of the reasons from it down, drawn over the ones above.
		var series []Series
		for i := range by {
			stack := make([]float64, b.Points())
			for j := i; j < len(by); j++ {
				for k := range stack {
					stack[k] += by[j][k]
				}
			}
			series = append(series, barOf(stack))
		}
		lines = append(lines, chart(p.ID+": minutes idle with room, per day, by why", labels, "minutes", 0,
			StackPalette, series...)...)
		lines = append(lines, "")
	}
	if !ticked {
		return []string{fmt.Sprintf("No tick recorded in the last %d days yet.", DaysShown)}
	}
	if len(busy) > 0 {
		lines = append(lines, "Never idle with room: "+codes(busy)+".", "")
	}
	return append(lines, note(fmt.Sprintf("Each bar, top to bottom: %s (blue), %s (purple), %s (yellow), %s (red). "+
		"From `ticks-*.csv`: minutes neither capped nor outside its hours with no run held (`runs_held` 0), by the "+
		"tick's state (`idle`, `no-runner`, `backing-off`) or `suspended`.", IdleReasons[0], IdleReasons[1],
		IdleReasons[2], IdleReasons[3])))
}

// ---------------------------------------------------------------------- 8. lanes and runners

func (d *drawing) lanes() []string {
	b := Hourly(d.now, d.zone, days(LanesDays), d.detail.HourStep)
	labels := b.Labels(d.zone, "Mon 15h")
	var lines []string
	for _, p := range d.subs {
		ticks := since(d.x.ticks[p.ID], tickAt, b.From().Add(-TickCovers))
		if len(ticks) == 0 {
			continue
		}
		held, _ := atPoints(ticks, tickAt, func(t *Tick) (float64, bool) { return t.LanesHeld.V, t.LanesHeld.OK }, b)
		allowed, _ := atPoints(ticks, tickAt, func(t *Tick) (float64, bool) { return t.LanesAllowed.V, t.LanesAllowed.OK }, b)
		lines = append(lines, chart(p.ID+": lanes held (green) and allowed (grey)", labels, "lanes",
			max(1, highest(held, allowed)), ReadingPalette, lineOf(held), lineOf(allowed))...)
		lines = append(lines, "")
	}
	pools := since(d.x.pools, poolAt, b.From().Add(-TickCovers))
	if len(lines) == 0 && len(pools) == 0 {
		return []string{fmt.Sprintf("No tick recorded in the last %d days yet.", LanesDays)}
	}
	pool := d.ctx.Cfg.Pool
	for _, c := range []struct {
		name        string
		used, limit func(*PoolTick) Num
		fallback    int
	}{
		{"GitHub runner slots", func(x *PoolTick) Num { return x.GithubSlotsUsed }, func(x *PoolTick) Num { return x.GithubSlots }, pool.MaxParallel},
		{"AWS slots", func(x *PoolTick) Num { return x.AWSSlotsUsed }, func(x *PoolTick) Num { return x.AWSSlots }, pool.MachineParallel},
		{"Planning lanes", func(x *PoolTick) Num { return x.PlanLanesUsed }, func(x *PoolTick) Num { return x.PlanLanes }, pool.PlanLanes},
	} {
		if len(pools) == 0 {
			break
		}
		used, _ := atPoints(pools, poolAt, func(x *PoolTick) (float64, bool) { return c.used(x).V, c.used(x).OK }, b)
		limit, known := atPoints(pools, poolAt, func(x *PoolTick) (float64, bool) { return c.limit(x).V, c.limit(x).OK }, b)
		for i := range limit {
			if !known[i] {
				limit[i] = float64(c.fallback)
			}
		}
		lines = append(lines, chart(fmt.Sprintf("%s used (green) of their limit (grey)", c.name), labels, "slots",
			max(1, highest(used, limit)), ReadingPalette, lineOf(used), lineOf(limit))...)
		lines = append(lines, "")
	}
	return append(lines, note("From `ticks-*.csv` (`lanes_held`, `lanes_allowed`: the lanes in force for a run "+
		"starting then) and `pools-*.csv` (`*_used` against `github_slots`, `aws_slots`, `plan_lanes`); each "+
		"point is the most it reached."))
}

// ---------------------------------------------------------------------- 9. the queue

// QueueKinds are the queue's kinds, what an item's next run does.
var QueueKinds = []string{"plan", "build", "revise", "review"}

func (d *drawing) queue() []string {
	b := Hourly(d.now, d.zone, days(LanesDays), d.detail.HourStep)
	labels := b.Labels(d.zone, "Mon 15h")
	pools := since(d.x.pools, poolAt, b.From().Add(-TickCovers))
	var lines []string
	if len(pools) == 0 {
		lines = append(lines, fmt.Sprintf("No pool row recorded in the last %d days yet.", LanesDays), "")
	}
	columns := map[string][3]func(*PoolTick) Num{
		"plan":   {func(x *PoolTick) Num { return x.PlanMedium }, func(x *PoolTick) Num { return x.PlanHard }, func(x *PoolTick) Num { return x.PlanUnrated }},
		"build":  {func(x *PoolTick) Num { return x.BuildMedium }, func(x *PoolTick) Num { return x.BuildHard }, func(x *PoolTick) Num { return x.BuildUnrated }},
		"revise": {func(x *PoolTick) Num { return x.ReviseMedium }, func(x *PoolTick) Num { return x.ReviseHard }, func(x *PoolTick) Num { return x.ReviseUnrated }},
		"review": {func(x *PoolTick) Num { return x.ReviewMedium }, func(x *PoolTick) Num { return x.ReviewHard }, func(x *PoolTick) Num { return x.ReviewUnrated }},
	}
	var empty []string
	for _, kind := range QueueKinds {
		if len(pools) == 0 {
			break
		}
		var series []Series
		var all [][]float64
		for _, of := range columns[kind] {
			values, _ := atPoints(pools, poolAt, func(x *PoolTick) (float64, bool) { return of(x).V, of(x).OK }, b)
			series, all = append(series, lineOf(values)), append(all, values)
		}
		if !any0(all...) {
			empty = append(empty, kind)
			continue
		}
		lines = append(lines, chart("Queued to "+kind+": medium (blue), hard (red), unrated (grey)", labels, "items",
			max(1, highest(all...)), TriplePalette, series...)...)
		lines = append(lines, "")
	}
	if len(empty) > 0 {
		lines = append(lines, fmt.Sprintf("Nothing queued in the last %d days to %s.", LanesDays, joinAnd(empty)), "")
	}
	if held, _ := atPoints(pools, poolAt, func(x *PoolTick) (float64, bool) { return x.HeldForOwner.V, x.HeldForOwner.OK }, b); any0(held) {
		lines = append(lines, chart("Items held for their owner (bot:waiting)", labels, "items", max(1, highest(held)),
			"", lineOf(held))...)
		lines = append(lines, "")
	}
	from := d.now.Add(-days(LanesDays))
	var holds []HoldSpan
	for _, p := range d.subs {
		holds = append(holds, Holds(d.x.ticks[p.ID], from, d.now, d.ctx.Cfg.HoldMinutes)...)
	}
	if len(holds) > 0 {
		var lengths []float64
		longest := 0.0
		for _, h := range holds {
			lengths = append(lengths, h.Minutes)
			longest = max(longest, h.Minutes)
		}
		m, _ := median(lengths)
		lines = append(lines, fmt.Sprintf("Held for their owner in the last %d days: %d holds, a median %s minutes, "+
			"the longest %s (section 14 has them by subscription).", LanesDays, len(holds), numText(m), numText(longest)), "")
	}
	rows := []string{"| Kind | Runs | Median wait to start | Longest |", "|---|--:|--:|--:|"}
	for _, kind := range QueueKinds {
		var waits []float64
		for _, p := range d.subs {
			for _, r := range runsSince(d.f, p.ID, from) {
				if r.Kind == kind && r.QueueWaitMin.OK {
					waits = append(waits, r.QueueWaitMin.V)
				}
			}
		}
		if m, ok := median(waits); ok {
			rows = append(rows, fmt.Sprintf("| %s | %d | %s min | %s min |", kind, len(waits), numText(m),
				numText(highest(waits))))
		}
	}
	if len(rows) > 2 {
		lines = append(append(lines, fmt.Sprintf("**The wait to start**, runs of the last %d days (`queue_wait_min`):", LanesDays), ""), rows...)
		lines = append(lines, "")
	}
	return append(lines, note("From `pools-*.csv` (`queued_<kind>_<difficulty>`, `held_for_owner`; each point the "+
		"most it reached), `ticks-*.csv` (`held_for_it`) and `runs-*.csv`."))
}

// ---------------------------------------------------------------------- 10. cost

func (d *drawing) cost() []string {
	b := Daily(d.now, d.zone, DaysShown, d.detail.DayStep)
	labels := b.Labels(d.zone, "01-02")
	from := b.From()
	header, rule := "| Subscription |", "|---|"
	for _, label := range labels {
		header += " " + label + " |"
		rule += "--:|"
	}
	cellOf := func(v float64, text func(float64) string) string {
		if v == 0 {
			return NoBlock
		}
		return text(v)
	}
	var minutesRows, tokenRows, usdRows []string
	totals := []string{"| Subscription | Runs | Model minutes | Tokens in | Tokens out | Tokens cached | Reported USD |",
		"|---|--:|--:|--:|--:|--:|--:|"}
	for _, p := range d.subs {
		runs := runsSince(d.f, p.ID, from)
		if len(runs) == 0 {
			continue
		}
		minutes, tokens, usd := make([]float64, b.Points()), make([]float64, b.Points()), make([]float64, b.Points())
		var in, out, cached, spent, minutesAll float64
		reported := false
		for _, r := range runs {
			if i := b.Index(r.EndedAt); i >= 0 {
				minutes[i] += r.ModelMinutes.V
				tokens[i] += r.TokensIn.V + r.TokensOut.V + r.TokensCached.V
				usd[i] += r.USDReported.V
			}
			minutesAll += r.ModelMinutes.V
			in, out, cached = in+r.TokensIn.V, out+r.TokensOut.V, cached+r.TokensCached.V
			if r.USDReported.OK {
				spent, reported = spent+r.USDReported.V, true
			}
		}
		rows := [3]string{"| " + code(p.ID) + " |", "| " + code(p.ID) + " |", "| " + code(p.ID) + " |"}
		for i := range minutes {
			rows[0] += " " + cellOf(minutes[i], numText) + " |"
			rows[1] += " " + cellOf(tokens[i], tokensText) + " |"
			rows[2] += " " + cellOf(usd[i], usdText) + " |"
		}
		minutesRows, tokenRows, usdRows = append(minutesRows, rows[0]), append(tokenRows, rows[1]), append(usdRows, rows[2])
		usdCell := dash
		if reported {
			usdCell = usdText(spent)
		}
		totals = append(totals, fmt.Sprintf("| %s | %d | %s | %s | %s | %s | %s |", code(p.ID), len(runs),
			numText(minutesAll), tokensText(in), tokensText(out), tokensText(cached), usdCell))
	}
	if len(minutesRows) == 0 {
		return []string{fmt.Sprintf("No run delivered in the last %d days yet.", DaysShown)}
	}
	lines := []string{"**Model minutes per day:**", "", header, rule}
	lines = append(append(lines, minutesRows...), "", "**Tokens per day** (in, out and cached):", "", header, rule)
	lines = append(append(lines, tokenRows...), "", "**Reported USD per day:**", "", header, rule)
	lines = append(append(lines, usdRows...), "", fmt.Sprintf("**The last %d days:**", DaysShown), "")
	lines = append(append(lines, totals...), "")
	lines = append(lines, d.perItem(from)...)
	return append(lines, note("From `runs-*.csv`. Reported USD is the Claude CLI's API-price figure, not a bill."))
}

// perItem is the cost of each item approved since `from`, by the subscription that built it: the
// model minutes and reported USD of every run on the item in that time. The files do not see a
// merge, so an approval (an `approved` run, which turns auto-merge on) stands for it.
func (d *drawing) perItem(from time.Time) []string {
	var all []*Run
	for _, p := range d.subs {
		all = append(all, runsSince(d.f, p.ID, from)...)
	}
	all = sortedBy(all, runAt)
	builder := map[string]string{}
	approved := []string{}
	minutes, usd := map[string]float64{}, map[string]float64{}
	for _, r := range all {
		if r.Item == "" {
			continue
		}
		minutes[r.Item] += r.ModelMinutes.V
		usd[r.Item] += r.USDReported.V
		if r.Kind == "build" {
			builder[r.Item] = r.Subscription
		}
		if r.Outcome == "approved" && builder[r.Item] != "" && !slices.Contains(approved, r.Item) {
			approved = append(approved, r.Item)
		}
	}
	if len(approved) == 0 {
		return []string{fmt.Sprintf("No item built and approved in the last %d days yet.", DaysShown), ""}
	}
	per := map[string][][2]float64{}
	var order []string
	for _, item := range approved {
		who := builder[item]
		if _, seen := per[who]; !seen {
			order = append(order, who)
		}
		per[who] = append(per[who], [2]float64{minutes[item], usd[item]})
	}
	rows := []string{fmt.Sprintf("**Per item approved** in the last %d days (the files do not see a merge; an "+
		"`approved` run turns auto-merge on), by the subscription that built it:", DaysShown), "",
		"| Built by | Items | Model minutes an item (median) | Reported USD an item (median) |", "|---|--:|--:|--:|"}
	for _, who := range order {
		var m, u []float64
		for _, pair := range per[who] {
			m, u = append(m, pair[0]), append(u, pair[1])
		}
		mm, _ := median(m)
		mu, _ := median(u)
		rows = append(rows, fmt.Sprintf("| %s | %d | %s | %s |", code(who), len(per[who]), numText(mm), usdText(mu)))
	}
	return append(rows, "")
}

// ---------------------------------------------------------------------- 11. overshoot

func (d *drawing) overshoot() []string {
	from := d.now.Add(-days(DaysShown))
	rows := []string{"| Subscription | Stopped at its 5-hour cap | Highest past it | 90th percentile | " +
		"Stopped at its weekly cap | Highest past it | 90th percentile |", "|---|--:|--:|--:|--:|--:|--:|"}
	for _, p := range d.subs {
		var five, week []float64
		fiveRuns, weekRuns := 0, 0
		for _, r := range runsSince(d.f, p.ID, from) {
			var util, limit func(*Reading) Num
			switch r.StopReason {
			case "5-hour cap":
				fiveRuns++
				util, limit = func(x *Reading) Num { return x.Util5h }, func(x *Reading) Num { return x.Cap5h }
			case "weekly cap":
				weekRuns++
				util, limit = func(x *Reading) Num { return x.Util7d }, func(x *Reading) Num { return x.Cap7d }
			default:
				continue
			}
			readings := d.x.byRun[r.RunID]
			for i := len(readings) - 1; i >= 0 && r.RunID != ""; i-- {
				if u, c := util(readings[i]), limit(readings[i]); u.OK && c.OK {
					if r.StopReason == "5-hour cap" {
						five = append(five, round1(u.V-c.V))
					} else {
						week = append(week, round1(u.V-c.V))
					}
					break
				}
			}
		}
		if fiveRuns+weekRuns == 0 {
			continue
		}
		cells := func(values []float64) (string, string) {
			if len(values) == 0 {
				return dash, dash
			}
			p90, _ := percentile(values, OvershootShare)
			return pointsText(highest(values)), pointsText(p90)
		}
		fiveMax, fiveP := cells(five)
		weekMax, weekP := cells(week)
		rows = append(rows, fmt.Sprintf("| %s | %d | %s | %s | %d | %s | %s |", code(p.ID), fiveRuns, fiveMax, fiveP,
			weekRuns, weekMax, weekP))
	}
	if len(rows) == 2 {
		return []string{fmt.Sprintf("No run stopped at a cap in the last %d days.", DaysShown)}
	}
	return append(rows, "", note(fmt.Sprintf("Points past the cap in force on the run's last reading (`readings-*.csv` "+
		"by `run_id`), for the runs of the last %d days `runs-*.csv` says stopped there; a highest under 0 "+
		"stopped short of it. They tune `LastHourSlack` (%s), `HardCapMargin` (%s) and `start_headroom` "+
		"(section 15 has each subscription's).", DaysShown, pointsText(share(providers.LastHourSlack)),
		pointsText(share(providers.HardCapMargin)))))
}

// ---------------------------------------------------------------------- 12. the shared account

func (d *drawing) shared() []string {
	b := Daily(d.now, d.zone, DaysShown, d.detail.DayStep)
	labels := b.Labels(d.zone, "01-02")
	var lines []string
	for _, id := range SharedAccounts {
		if d.ctx.Cfg.Pool.Get(id) == nil {
			continue
		}
		if len(since(d.x.readings[id], readingAt, b.From())) == 0 {
			lines = append(lines, fmt.Sprintf("%s was not read in the last %d days.", code(id), DaysShown), "")
			continue
		}
		rose := make([]float64, b.Points())
		onDays := 0
		for _, r := range d.x.rises[id] {
			if i := b.Index(r.At); i >= 0 {
				if rose[i] == 0 {
					onDays++
				}
				rose[i] += r.Points
			}
		}
		if !any0(rose) {
			lines = append(lines, fmt.Sprintf("%s's readings did not rise while no bot run went in the last %d days: "+
				"nobody else spent it that the bot saw.", code(id), DaysShown), "")
			continue
		}
		lines = append(lines, chart(id+": points its 5-hour usage rose while no bot run went, per day", labels,
			"points", 0, "", barOf(rose))...)
		lines = append(lines, fmt.Sprintf("%s rose %s points outside the bot's runs in the last %d days, on %d of "+
			"them, the most %s in one.", code(id), numText(risen(d.x.rises[id], b.From(), d.now)), DaysShown, onDays,
			numText(highest(rose))), "")
	}
	if len(lines) == 0 {
		return []string{"The pool holds no shared account (`usage.SharedAccounts`)."}
	}
	return append(lines, note("The account's owner and bright-bots-harness spend it too. A rise is one between two "+
		"readings while no run of the bot's went (section 5); the bot reads it only in its runs, so a rise shows "+
		"at its next run's first reading."))
}

// ---------------------------------------------------------------------- 13. weekly walls

func (d *drawing) walls() []string {
	rows := []string{"| Subscription | Week | Peak | Reached its cap | Days refused |", "|---|---|--:|---|--:|"}
	from := d.now.Add(-time.Duration(WeeksShown) * WeekSpan)
	var medium []string
	for _, p := range d.subs {
		if p.Tier != "medium" {
			continue
		}
		medium = append(medium, p.ID)
		for _, w := range since(d.x.weeks[p.ID], func(w *Week) time.Time { return w.ResetAt }, from) {
			reached := "never"
			if day := weekDay(w); day > 0 {
				reached = fmt.Sprintf("day %d, %s", day, d.when(w.FirstCappedAt))
			}
			rows = append(rows, fmt.Sprintf("| %s | to %s | %s | %s | %s |", code(p.ID), d.when(w.ResetAt),
				pctText(w.PeakUtil.V), reached, numText(w.DaysRefused.V)))
		}
		if row := d.thisWeek(p); row != "" {
			rows = append(rows, row)
		}
	}
	if len(medium) == 0 {
		return []string{"The pool holds no medium subscription."}
	}
	if len(rows) == 2 {
		return []string{fmt.Sprintf("No week of %s read yet: a week counts six hours after its reset.", codes(medium))}
	}
	return append(rows, "", note("Closed weeks from `weeks.csv` (`first_capped_at`: its first reading at its weekly "+
		"cap in force, 100% for these; `days_refused`), this week's from `readings-*.csv` so far."))
}

// thisWeek is the row of `p`'s week going now, from its readings: its peak, when it reached its
// cap, and the days refused so far; "" when no reading names it.
func (d *drawing) thisWeek(p *providers.Provider) string {
	readings := d.x.readings[p.ID]
	var reset time.Time
	for _, r := range readings {
		if r.Util7d.OK && r.Resets7d.After(d.now) {
			reset = r.Resets7d
		}
	}
	if reset.IsZero() {
		return ""
	}
	start := reset.Add(-WeekSpan)
	var peak Num
	var reached time.Time
	var refused [][2]time.Time
	for _, r := range readings {
		if !in(r.ObservedAt, start, d.now.Add(time.Second)) {
			continue
		}
		if r.RefusedUntil.After(r.ObservedAt) {
			refused = append(refused, [2]time.Time{r.ObservedAt, r.RefusedUntil})
		}
		if !r.Util7d.OK || absDuration(r.Resets7d.Sub(reset)) > WeekSlack {
			continue
		}
		peak = maxNum(peak, r.Util7d)
		limit := 100.0
		if r.Cap7d.OK {
			limit = r.Cap7d.V
		}
		if r.Util7d.V >= limit && reached.IsZero() {
			reached = r.ObservedAt
		}
	}
	for _, t := range since(d.x.ticks[p.ID], tickAt, start) {
		if t.RefusedUntil.After(t.At) {
			refused = append(refused, [2]time.Time{t.At, t.RefusedUntil})
		}
	}
	text := "not yet"
	if !reached.IsZero() {
		text = fmt.Sprintf("day %d, %s", int(reached.Sub(start).Hours()/24)+1, d.when(reached))
	}
	return fmt.Sprintf("| %s | this one, to %s | %s | %s | %s |", code(p.ID), d.when(reset), pctText(peak.V), text,
		numText(covered(refused, start, d.now).Hours()/24))
}

// ---------------------------------------------------------------------- 14. holds and handoffs

// HandoffReasons are the reasons an item changed hands (`HandoffReason`), in the order section 14
// lists them.
var HandoffReasons = []string{"cap", "time budget", "person", "other"}

func (d *drawing) holds() []string {
	from := d.now.Add(-days(DaysShown))
	var lines []string
	hold := d.ctx.Cfg.HoldMinutes
	if hold <= 0 {
		lines = append(lines, "Nothing is held for its owner: `hold_minutes` is 0.", "")
	} else {
		rows := []string{"| Subscription | Holds | Median minutes | Longest | Ran out |", "|---|--:|--:|--:|--:|"}
		for _, p := range d.subs {
			spans := Holds(d.x.ticks[p.ID], from, d.now, hold)
			if len(spans) == 0 {
				continue
			}
			var lengths []float64
			out := 0
			for _, h := range spans {
				lengths = append(lengths, h.Minutes)
				if h.RanOut {
					out++
				}
			}
			m, _ := median(lengths)
			rows = append(rows, fmt.Sprintf("| %s | %d | %s | %s | %d |", code(p.ID), len(spans), numText(m),
				numText(highest(lengths)), out))
		}
		if len(rows) == 2 {
			lines = append(lines, fmt.Sprintf("No item was held for its owner in the last %d days.", DaysShown), "")
		} else {
			lines = append(append(lines, fmt.Sprintf("**Held for their owner** (`bot:waiting`) in the last %d days, "+
				"against a hold of %d minutes (a hold that lasted it ran out; a shorter one was lifted by its owner "+
				"or released by a person):", DaysShown, hold), ""), rows...)
			lines = append(lines, "")
		}
	}
	var runs []*Run
	for _, p := range d.subs {
		runs = append(runs, runsSince(d.f, p.ID, from)...)
	}
	rows := []string{"| Why | Runs | Median wait to start |", "|---|--:|--:|"}
	for _, reason := range HandoffReasons {
		var waits []float64
		count := 0
		for _, r := range runs {
			if r.HandedOff == reason {
				count++
				if r.QueueWaitMin.OK {
					waits = append(waits, r.QueueWaitMin.V)
				}
			}
		}
		if count == 0 {
			continue
		}
		wait := dash
		if m, ok := median(waits); ok {
			wait = numText(m) + " min"
		}
		rows = append(rows, fmt.Sprintf("| %s | %d | %s |", reason, count, wait))
	}
	if len(rows) == 2 {
		lines = append(lines, fmt.Sprintf("No item changed hands in the last %d days.", DaysShown), "")
	} else {
		lines = append(append(lines, "**Handoffs by reason** (`handed_off`):", ""), rows...)
		lines = append(lines, "")
	}
	sessions := []string{"| Session | Runs | Model minutes | Median a run |", "|---|--:|--:|--:|"}
	for _, kind := range []string{"resumed", "fresh"} {
		var minutes []float64
		for _, r := range runs {
			if r.Session == kind {
				minutes = append(minutes, r.ModelMinutes.V)
			}
		}
		if len(minutes) > 0 {
			m, _ := median(minutes)
			total := 0.0
			for _, v := range minutes {
				total += v
			}
			sessions = append(sessions, fmt.Sprintf("| %s | %d | %s | %s |", kind, len(minutes), numText(total), numText(m)))
		}
	}
	if len(sessions) == 2 {
		lines = append(lines, fmt.Sprintf("No Claude run kept a session in the last %d days.", DaysShown), "")
	} else {
		lines = append(append(lines, "**Resumed sessions against fresh ones** (`session`), by model minutes:", ""), sessions...)
		lines = append(lines, "")
	}
	return append(lines, note("Holds from `ticks-*.csv` (`held_for_it`, each stretch one hold); handoffs and "+
		"sessions from `runs-*.csv`."))
}

// ---------------------------------------------------------------------- 15. suggestions

func (d *drawing) suggestions() []string {
	var lines []string
	all := Suggestions(d.ctx, d.f)
	if len(all) == 0 {
		lines = append(lines, "No suggestion: no rule below finds its evidence in the history.", "")
	} else {
		lines = append(lines, "| Subscription | providers.json key | Now | Suggested | The evidence |", "|---|---|---|---|---|")
		named := map[string]bool{}
		for _, s := range all {
			who := code(s.Subscription)
			if s.Subscription == "" {
				who = "all"
			}
			if !named[s.Subscription] {
				who = `<a name="` + anchor(s.Subscription) + `"></a>` + who
				named[s.Subscription] = true
			}
			lines = append(lines, fmt.Sprintf("| %s | %s | %s | %s | %s |", who, code(s.Key), s.Now, s.Suggested,
				dashboard.Cell(s.Evidence, 400)))
		}
		lines = append(lines, "")
	}
	return append(lines, note(fmt.Sprintf("Computed, never applied: a person edits `.harness/providers.json` (and "+
		"`.harness/config.json`). The rules (`bot/go/internal/usage/suggest.go`): a 5-hour cap may go up %s points "+
		"when it was hit with work queued in at least %s of its sessions over %d days (at least %d) and no reading "+
		"rose outside the bot's runs; a 5-hour cap or the lanes should come down when the last %d days' trend "+
		"takes the week past its weekly cap before its reset; two lanes may be too many when %d sessions in %d "+
		"days went more than `LastHourSlack` (%s) past the cap with two runs going; `start_headroom` may "+
		"be too small when more than %s of the runs started inside it (at least %d) were cut off within %s "+
		"minutes; `last_hour` is wasted when %d sessions let its lift go unused with work queued; `hold_minutes` "+
		"needs moving when holds (at least %d) last longer than a handoff's wait or more than %s run out; the plan "+
		"load may move when a medium subscription reaches its weekly cap before day %d in %d weeks running.",
		numText(CapRaisePoints), pctText(100*CapHitShare), SuggestDays, SuggestMinSessions, int(ProjectSpan.Hours()/24),
		LaneSessions, int(LaneWeek.Hours()/24), pointsText(share(providers.LastHourSlack)), pctText(100*HeadroomCutShare),
		HeadroomMinRuns, numText(HeadroomCutMinutes), LastHourWastedSessions, HoldMinHolds,
		pctText(100*HoldRanOutShare), PlanLoadDay, PlanLoadWeeks)))
}

// ---------------------------------------------------------------------- 16. the raw data

// RawHost is where a public repository's raw files are read from, for a spreadsheet's
// `IMPORTDATA`.
var RawHost = "https://raw.githubusercontent.com"

// KindWords is what each kind's file holds, for section 16.
var KindWords = map[string]string{"readings": "every real usage reading", "ticks": "each subscription at each tick",
	"pools": "the runner slots, planning lanes and queue at each tick", "sessions": "each closed 5-hour session",
	"weeks": "each closed week", "runs": "each delivered run", "backfill": "what the backfill read"}

func (d *drawing) raw() []string {
	cfg := d.ctx.Cfg
	branch := branchName()
	tree := cfg.ServerURL + "/" + cfg.Repo + "/tree/" + branch + "/" + Dir
	if len(d.f.Files) == 0 {
		return []string{"Nothing is on `" + branch + "` yet: its first push makes it."}
	}
	month := d.now.Format("2006-01")
	rows := []string{"| File | Holds | Link | Raw |", "|---|---|---|---|"}
	var sessionsRaw string
	older := false
	for _, name := range d.f.Files {
		kind := strings.SplitN(strings.TrimSuffix(strings.TrimSuffix(name, ".csv"), ".json"), "-", 2)[0]
		if strings.Count(name, "-") > 1 && !strings.Contains(name, month) {
			older = true // an older month's: the folder has it
			continue
		}
		path := Dir + "/" + name
		rawURL := RawHost + "/" + cfg.Repo + "/" + branch + "/" + path
		if kind == "sessions" {
			sessionsRaw = rawURL
		}
		rows = append(rows, fmt.Sprintf("| `%s` | %s | [view](%s/%s/blob/%s/%s) | [raw](%s) |", name,
			py.OrStr(KindWords[kind], dash), cfg.ServerURL, cfg.Repo, branch, path, rawURL))
	}
	if sessionsRaw == "" {
		sessionsRaw = RawHost + "/" + cfg.Repo + "/" + branch + "/" + Dir + "/sessions-" + month + ".csv"
	}
	folder := "The files are in [the branch's folder](" + tree + ")."
	if older {
		folder = "Older months are in [the branch's folder](" + tree + ")."
	}
	return append(rows, "", folder+" Each file opens in a spreadsheet; its columns are in [the usage history]("+
		d.readme()+"-history).", "",
		"- If the repository is public, a Google Sheet reads a file live: `=IMPORTDATA(\""+sessionsRaw+"\")`.",
		"- Otherwise download the file (its raw link) and use File → Import.")
}

// readme is the bot's manual's link to the usage issue's section, less its last word: `…-issue`
// and `…-history` complete it.
func (d *drawing) readme() string {
	cfg := d.ctx.Cfg
	return cfg.ServerURL + "/" + cfg.Repo + "/blob/" + cfg.DefaultBranch + "/bot/README.md#the-usage"
}
