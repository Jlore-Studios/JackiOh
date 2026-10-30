package dashboard

// ------------------------------------------------------------------ the subscriptions

import (
	"math"
	"slices"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/waiting"
)

// nowCell is _now_cell.
func nowCell(state *pyjson.Object, now *waiting.Now, runs []int) string {
	if len(runs) > 0 {
		var parts []string
		for _, n := range runs {
			if n == 0 {
				parts = append(parts, "survey")
				continue
			}
			verb, _, _ := doing(state, n)
			parts = append(parts, verb+" #"+strconv.Itoa(n))
		}
		cell := "🟢 " + strings.Join(parts, ", ")
		var queued []string
		for _, n := range now.Items {
			if !slices.Contains(runs, n) {
				queued = append(queued, "#"+strconv.Itoa(n))
			}
		}
		if len(queued) > 0 {
			cell += "<br/>▶️ takes " + strings.Join(queued, ", ") + " next"
		}
		return cell
	}
	return now.Text
}

// SubscriptionTable is subscription_table.
func SubscriptionTable(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object,
	now *pyjson.Object /* = None */) []string {
	cfg := ctx.Cfg
	if now == nil {
		lanes := waiting.LanesOf(ctx, state, live)
		now = waiting.Subscriptions(ctx, state, lanes, lanes, pyjson.New(), "")
	}
	busy := map[string][]int{}
	for _, l := range lanesIn(live) {
		busy[l.provider] = append(busy[l.provider], l.number)
	}
	lines := []string{"| Subscription | Now | 5-hour session | 7-day week | Read | Models (tier) | Hours |",
		"|---|---|---|---|---|---|---|"}
	for _, provider := range cfg.Pool.Ordered() {
		entry := providers.PeekRecord(state, provider.ID)
		var seats []string
		for _, seat := range cfg.Pool.Seats(provider) {
			seats = append(seats, "`"+seat.Model+"` "+seat.Tier)
		}
		runs := slices.Clone(busy[provider.ID])
		sort.Ints(runs)
		cell := nowCell(state, nowFor(now, provider.ID), runs)
		lines = append(lines, "| `"+provider.ID+"` | "+cell+
			" | "+usageCell(ctx, provider, entry, "five_hour")+
			" | "+usageCell(ctx, provider, entry, "seven_day")+
			" | "+readAgo(ctx, entry)+" | "+strings.Join(seats, ", ")+
			" | "+Cell(provider.Hours(cfg.Timezone), 70)+" |")
	}
	return lines
}

// usageCell is _usage_cell: a usage window's bar, and under it its cap now (and in or outside its
// hours), the line a build or a plan must start under, and when it resets (`status.window_notes`).
// A window that reset since its reading says so and what the reading was, so 0% is never a guess
// left unsaid; one with no reading says that.
func usageCell(ctx *hctx.Context, provider *providers.Provider, entry *pyjson.Object,
	window string) string {
	notes := status.WindowNotes(ctx, provider, entry, window,
		func(at time.Time) string { return clockText(ctx, at) })
	used, reset, read, ok := windowReading(ctx, provider, entry, window)
	var cell string
	if !ok {
		cell = "—"
		if len(notes) > 0 || window == "five_hour" {
			cell = "not read yet"
		}
		if len(notes) == 0 && window == "seven_day" {
			notes = []string{"not reported"}
		}
	} else {
		cell = barOf(used, BarCells)
		if !reset.IsZero() {
			var kept []string
			for _, n := range notes {
				if !strings.HasPrefix(n, "resets") {
					kept = append(kept, n)
				}
			}
			notes = append(kept, "reset "+clockText(ctx, reset)+", not read since (it read "+
				py.Percent(read, 0)+")")
		}
	}
	if len(notes) > 0 {
		cell += "<br/><sub>" + strings.Join(notes, " · ") + "</sub>"
	}
	return cell
}

// readAgo is _read_ago: how old a subscription's usage reading is: the bars are only as fresh as
// that.
func readAgo(ctx *hctx.Context, entry *pyjson.Object) string {
	usage, _ := entry.Get("usage").(*pyjson.Object)
	observed := clock.ParseISOValue(usage.Get("observed_at"))
	if observed.IsZero() {
		return "never"
	}
	age := clock.HumanDelta(ctx.Now().Sub(observed))
	text := age + " ago"
	if age == "now" {
		text = "just now"
	}
	return hover(text, "read at "+clockText(ctx, observed), "")
}

// UsageChart is USAGE_CHART: the usage charts' colours: the cap behind in grey, the reading over
// it in green, so a bar that reaches the grey one's top is at its cap.
var UsageChart = `%%{init: {"themeVariables": {"xyChart": {"plotColorPalette": ` +
	`"#d0d7de, #1f883d"}}}}%%`

// UsageCharts is usage_charts: each subscription's 5-hour and 7-day usage as bars over its cap
// now, for every one with a reading; the ones with none are named under the chart.
func UsageCharts(ctx *hctx.Context, state *pyjson.Object) []string {
	pool := ctx.Cfg.Pool
	var lines []string
	for _, w := range [][2]string{{"five_hour", "5-hour session"}, {"seven_day", "7-day week"}} {
		window, name := w[0], w[1]
		var names, used, caps, unread []string
		for _, provider := range pool.Ordered() {
			entry := providers.PeekRecord(state, provider.ID)
			value, _, _, ok := windowReading(ctx, provider, entry, window)
			capValue := provider.CapsAt(ctx.Now(), ctx.Cfg.Timezone,
				providers.CurrentUsage(provider, entry), time.Time{}).Get(window)
			if !ok {
				if window == "five_hour" || capValue != nil {
					unread = append(unread, "`"+provider.ID+"`")
				}
				continue
			}
			names = append(names, `"`+provider.ID+`"`)
			used = append(used, strconv.Itoa(py.Round(math.Min(value, 1.0)*100)))
			c := 1.0
			if capValue != nil {
				c, _ = pyjson.AsFloat(capValue)
			}
			caps = append(caps, strconv.Itoa(py.Round(c*100)))
		}
		if len(names) == 0 {
			continue
		}
		lines = append(lines, "```mermaid", UsageChart, "xychart-beta",
			`    title "`+name+`: usage now (green) against its cap (grey), %"`,
			"    x-axis ["+strings.Join(names, ", ")+"]",
			`    y-axis "% used" 0 --> 100`,
			"    bar ["+strings.Join(caps, ", ")+"]",
			"    bar ["+strings.Join(used, ", ")+"]", "```")
		if len(unread) > 0 {
			lines = append(lines, "<sub>No "+name+" reading yet: "+strings.Join(unread, ", ")+".</sub>")
		}
		lines = append(lines, "")
	}
	return lines
}
