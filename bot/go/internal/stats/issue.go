package stats

import (
	"fmt"
	"slices"
	"sort"
	"strings"
	"time"
	"unicode"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// ------------------------------------------------------------------ the issue

// bar is one of a bar chart's bars: its name and its value, an int or a float64 (a float stays one
// through `round(value, 1)`, so it is written "2.0").
type bar struct {
	name  string
	value any
}

func barNumber(v any) float64 { f, _ := pyjson.AsFloat(v); return f }

func bars(title, axis string, counts []bar, zeros bool, empty string) []string {
	var rows []bar
	for _, row := range counts {
		if zeros || barNumber(row.value) > 0 {
			rows = append(rows, row)
		}
	}
	some := false
	for _, row := range rows {
		if barNumber(row.value) > 0 {
			some = true
		}
	}
	if !some {
		return []string{fmt.Sprintf("_%s: %s._", title, empty)}
	}
	names := make([]string, len(rows))
	values := make([]string, len(rows))
	for i, row := range rows {
		names[i] = `"` + row.name + `"`
		if f, ok := row.value.(float64); ok {
			values[i] = pyjson.FloatRepr(py.RoundN(f, 1))
		} else {
			values[i] = py.Str(row.value)
		}
	}
	return []string{"```mermaid", "xychart-beta", fmt.Sprintf(`    title "%s"`, title),
		fmt.Sprintf("    x-axis [%s]", strings.Join(names, ", ")), fmt.Sprintf(`    y-axis "%s"`, axis),
		fmt.Sprintf("    bar [%s]", strings.Join(values, ", ")), "```"}
}

func line(title, axis string, days []string, values []int) []string {
	some := false
	for _, v := range values {
		if v != 0 {
			some = true
		}
	}
	if !some {
		return []string{fmt.Sprintf("_%s: nothing yet._", title)}
	}
	reprs := make([]string, len(days))
	for i, day := range days {
		reprs[i] = py.Repr(day)
	}
	shown := make([]string, len(values))
	for i, v := range values {
		shown[i] = itoa(v)
	}
	return []string{"```mermaid", "xychart-beta", fmt.Sprintf(`    title "%s"`, title),
		strings.ReplaceAll(fmt.Sprintf("    x-axis [%s]", strings.Join(reprs, ", ")), "'", `"`),
		fmt.Sprintf(`    y-axis "%s"`, axis), fmt.Sprintf("    line [%s]", strings.Join(shown, ", ")), "```"}
}

// Local is stats._local.
func Local(ctx *hctx.Context, at time.Time) time.Time {
	return at.In(clock.Zone(ctx.Cfg.Timezone))
}

// Pct is stats._pct.
func Pct(part float64, whole float64) string {
	if whole != 0 {
		return fmt.Sprintf("%.0f%%", 100*part/whole)
	}
	return "—"
}

func linesText(pulls []*pyjson.Object) string {
	added, removed := 0, 0
	for _, p := range pulls {
		added += py.MustInt(py.Or(p.Get("additions"), 0))
	}
	for _, p := range pulls {
		removed += py.MustInt(py.Or(p.Get("deletions"), 0))
	}
	return fmt.Sprintf("+%s / −%s", py.Thousands(added), py.Thousands(removed))
}

func total(pulls []*pyjson.Object, key string) int {
	sum := 0
	for _, p := range pulls {
		sum += py.MustInt(py.Or(p.Get(key), 0))
	}
	return sum
}

func medianToMerge(merged []*pyjson.Object) string {
	var took []time.Duration
	for _, p := range merged {
		if !clock.ParseISOValue(p.Get("merged_at")).IsZero() && !clock.ParseISOValue(p.Get("created_at")).IsZero() {
			took = append(took, minus(clock.ParseISOValue(p.Get("merged_at")), clock.ParseISOValue(p.Get("created_at"))))
		}
	}
	if len(took) == 0 {
		return "—"
	}
	return clock.HumanDelta(medianDuration(took))
}

// tokensText is stats._tokens: a window's token counts as `in / out / cached`, compact (#695).
func tokensText(w *Window) string {
	tin, tout, cached := 0, 0, 0
	for _, pid := range w.Tokens.Keys() {
		tin += tokensOf(w, pid)[0]
	}
	for _, pid := range w.Tokens.Keys() {
		tout += tokensOf(w, pid)[1]
	}
	for _, pid := range w.Tokens.Keys() {
		cached += tokensOf(w, pid)[2]
	}
	if tin == 0 && tout == 0 && cached == 0 {
		return "—"
	}
	return fmt.Sprintf("%s / %s / %s", py.Thousands(tin), py.Thousands(tout), py.Thousands(cached))
}

func titleOf(pull *pyjson.Object) string {
	title := strings.ReplaceAll(py.OrStr(pull.Get("title"), ""), "|", `\|`)
	if py.Len(title) <= TitleShown {
		return title
	}
	return strings.TrimRightFunc(py.SliceTo(title, TitleShown), unicode.IsSpace) + "…"
}

func whoText(providers []string) string {
	shown := make([]string, len(providers))
	for i, p := range providers {
		shown[i] = "`" + p + "`"
	}
	if text := strings.Join(shown, ", "); text != "" {
		return text
	}
	return "—"
}

func countStarts(w *Window, action string) int {
	n := 0
	for _, e := range w.Starts() {
		if e.Action == action {
			n++
		}
	}
	return n
}

func sumOf(c *py.Counter, keys []string) int {
	n := 0
	for _, k := range keys {
		n += c.Get(k)
	}
	return n
}

// row is one of a table's rows: its label, and its cell in a window.
type row struct {
	label string
	cell  func(w *Window) string
}

func table(windows []*Window, rows []row) []string {
	names := make([]string, len(windows))
	for i, w := range windows {
		names[i] = w.Name
	}
	lines := []string{"| | " + strings.Join(names, " | ") + " |", "|---|" + strings.Repeat("---|", len(windows))}
	for _, r := range rows {
		cells := make([]string, len(windows))
		for i, w := range windows {
			cells[i] = r.cell(w)
		}
		lines = append(lines, fmt.Sprintf("| %s | ", r.label)+strings.Join(cells, " | ")+" |")
	}
	return lines
}

// Summary is one row per measure, one column per window.
func Summary(windows []*Window) []string {
	rows := []row{
		{"Runs started", func(w *Window) string { return itoa(len(w.Starts())) }},
		{"… plans · builds · revisions · reviews", func(w *Window) string {
			counts := []string{}
			for _, a := range []string{"plan", "build", "revise", "review"} {
				counts = append(counts, itoa(countStarts(w, a)))
			}
			return strings.Join(counts, " · ")
		}},
		{"Outcomes that delivered something", func(w *Window) string {
			done, all := sumOf(w.Outcomes(), Done), w.Outcomes().Total()
			return fmt.Sprintf("%d of %d (%s)", done, all, Pct(float64(done), float64(all)))
		}},
		{"Paused on a usage limit", func(w *Window) string { return itoa(w.Outcomes().Get("paused")) }},
		{"Failed review or failed", func(w *Window) string {
			return itoa(w.Outcomes().Get("not approved") + w.Outcomes().Get("failed"))
		}},
		{"Could not run", func(w *Window) string { return itoa(w.Outcomes().Get("infra")) }},
		{"Pull requests opened", func(w *Window) string { return itoa(len(w.Opened)) }},
		{"Pull requests merged", func(w *Window) string { return itoa(len(w.Merged)) }},
		{"Issues closed by them", func(w *Window) string {
			closed := map[int]bool{}
			for _, p := range w.Merged {
				for _, m := range Closes.FindAllStringSubmatch(py.OrStr(p.Get("body"), ""), -1) {
					closed[py.MustInt(m[1])] = true
				}
			}
			return itoa(len(closed))
		}},
		{"Lines merged (added / removed)", func(w *Window) string { return linesText(w.Merged) }},
		{"Files changed in them", func(w *Window) string { return py.Thousands(total(w.Merged, "changed_files")) }},
		{"Commits in them", func(w *Window) string { return py.Thousands(total(w.Merged, "commits")) }},
		{"Median time from opening to merge", func(w *Window) string { return medianToMerge(w.Merged) }},
		{"Model hours", func(w *Window) string { return fmt.Sprintf("%.1f", py.Sum(floatsOf(w.Minutes))/60) }},
		{"Model tokens (in / out / cached)", tokensText},
		{"Model cost (USD)", func(w *Window) string { return fmt.Sprintf("$%.2f", py.Sum(floatsOf(w.Cost))) }},
		{fmt.Sprintf("`%s` workflow runs", config.NightWorkflow), func(w *Window) string {
			return itoa(len(w.Runs))
		}},
		{"Requests answered (`Queued …`)", func(w *Window) string {
			n := 0
			for _, e := range w.Events {
				if e.Kind == "queued" {
					n++
				}
			}
			return itoa(n)
		}},
	}
	return table(windows, rows)
}

// floatsOf is a Window's minutes or cost, in order.
func floatsOf(d *pyjson.Object) []float64 {
	var out []float64
	for _, pid := range d.Keys() {
		f, _ := d.Get(pid).(float64)
		out = append(out, f)
	}
	return out
}

// BySubscription is stats.by_subscription: provider id -> *py.Counter.
func BySubscription(w *Window, who *Credits) *pyjson.Object {
	counts := pyjson.New()
	kinds := outcomeKinds()
	for _, event := range w.Events {
		if event.Provider == "" {
			continue
		}
		c := counterAt(counts, event.Provider)
		if event.Kind == "start" {
			c.Add("start", 1)
			c.Add("start:"+event.Action, 1)
		} else if slices.Contains(kinds, event.Kind) {
			c.Add(event.Kind, 1)
		}
	}
	// Pull requests count under keys of their own: "opened" is also an outcome (a comment).
	for _, pull := range w.Opened {
		counterAt(counts, who.Builder(pull)).Add("pr:opened", 1)
	}
	for _, pull := range w.Merged {
		c := counterAt(counts, who.Builder(pull))
		c.Add("pr:merged", 1)
		c.Add("added", py.MustInt(py.Or(pull.Get("additions"), 0)))
		c.Add("removed", py.MustInt(py.Or(pull.Get("deletions"), 0)))
	}
	return counts
}

// order is stats._order: the subscriptions with anything in the window: the busiest first, the
// unrecorded last.
func order(w *Window, counts *pyjson.Object) []string {
	seen := map[string]bool{}
	var pids []string
	add := func(pid string) {
		if !seen[pid] {
			seen[pid] = true
			pids = append(pids, pid)
		}
	}
	for _, pid := range counts.Keys() {
		add(pid)
	}
	for _, pid := range w.Minutes.Keys() {
		if minutesOf(w, pid) > 0 {
			add(pid)
		}
	}
	for _, pid := range w.Tokens.Keys() {
		if tokensOf(w, pid) != [3]int{} {
			add(pid)
		}
	}
	for _, pid := range w.Cost.Keys() {
		if costOf(w, pid) > 0 {
			add(pid)
		}
	}
	var out []string
	for _, pid := range pids {
		if counterAt(counts, pid).Len() > 0 || minutesOf(w, pid) != 0 || tokensOf(w, pid) != [3]int{} ||
			costOf(w, pid) > 0 {
			out = append(out, pid)
		}
	}
	sort.SliceStable(out, func(i, j int) bool {
		a, b := out[i], out[j]
		if (a == Unrecorded) != (b == Unrecorded) {
			return b == Unrecorded
		}
		ca, cb := counterAt(counts, a), counterAt(counts, b)
		if ca.Get("start") != cb.Get("start") {
			return ca.Get("start") > cb.Get("start")
		}
		if ca.Get("pr:merged") != cb.Get("pr:merged") {
			return ca.Get("pr:merged") > cb.Get("pr:merged")
		}
		return a < b
	})
	return out
}

func nameOf(pid string) string {
	if pid != Unrecorded {
		return "`" + pid + "`"
	}
	return "_" + Unrecorded + "_"
}

// Section is one window, folded: what each subscription did in it, its tokens and cost, and bar
// charts of runs and lines by subscription (all time adds model hours and what runs ended in).
func Section(ctx *hctx.Context, w *Window, who *Credits, facts *Facts) []string {
	counts := BySubscription(w, who)
	order := order(w, counts)
	hasTokens := false
	for _, pid := range w.Tokens.Keys() {
		if tokensOf(w, pid) != [3]int{} || costOf(w, pid) > 0 {
			hasTokens = true
			break
		}
	}
	active := len(w.Events) > 0 || len(w.Opened) > 0 || len(w.Merged) > 0
	if !active && !hasTokens {
		return []string{fmt.Sprintf("<details><summary><b>%s</b>: nothing happened</summary>", w.Name), "",
			"Nothing happened.", "", "</details>", ""}
	}
	runs := 0
	for _, pid := range order {
		runs += counterAt(counts, pid).Get("start")
	}
	summary := fmt.Sprintf("<b>%s</b>: %d runs, %d pull requests merged, %s lines", w.Name, runs,
		len(w.Merged), linesText(w.Merged))
	lines := []string{fmt.Sprintf("<details><summary>%s</summary>", summary), ""}
	kinds := outcomeKinds()
	if active {
		lines = append(lines, "| Subscription | Models | Runs | Plans | Builds | Revisions | Reviews "+
			"| Outcomes delivered | PRs opened | Merged | Lines merged | Paused | Failed "+
			"| Could not run | Model time |", "|---|"+strings.Repeat("---|", 14))
		models := pyjson.New()
		for _, event := range w.Starts() {
			if event.Provider != "" {
				counterAt(models, event.Provider).Add(event.Model, 1)
			}
		}
		for _, pid := range order {
			c := counterAt(counts, pid)
			ended := sumOf(c, kinds)
			done := sumOf(c, Done)
			var shown []string
			for _, m := range counterAt(models, pid).MostCommon(3) {
				shown = append(shown, "`"+m.Key+"`")
			}
			modelList := strings.Join(shown, ", ")
			if modelList == "" {
				modelList = "—"
			}
			mins := minutesOf(w, pid)
			kept := len(spentOf(facts, pid))
			modelTime := fmt.Sprintf("%.1f h", mins/60)
			if w.Start.IsZero() && kept > 0 {
				modelTime += fmt.Sprintf(" (its last %d runs)", kept)
			}
			lines = append(lines, fmt.Sprintf("| %s | %s | %d | %d | %d "+
				"| %d | %d | %d of %d "+
				"| %d | %d | +%s / −%s "+
				"| %d "+
				"| %d | %d | %s |",
				nameOf(pid), modelList, c.Get("start"), c.Get("start:plan"), c.Get("start:build"),
				c.Get("start:revise"), c.Get("start:review"), done, ended,
				c.Get("pr:opened"), c.Get("pr:merged"), py.Thousands(c.Get("added")), py.Thousands(c.Get("removed")),
				c.Get("paused"),
				c.Get("failed")+c.Get("not approved"), c.Get("infra"), modelTime))
		}
		lines = append(lines, "")
	}
	type tokenRow struct {
		pid  string
		tok  [3]int
		cost float64
	}
	var rows []tokenRow
	for _, pid := range order {
		if tok, cost := tokensOf(w, pid), costOf(w, pid); tok != [3]int{} || cost > 0 {
			rows = append(rows, tokenRow{pid, tok, cost})
		}
	}
	lines = append(lines, "**Tokens and cost**", "")
	if len(rows) == 0 {
		lines = append(lines, "No call reported token counts in this window.", "")
	} else {
		lines = append(lines, "| Subscription | Input tokens | Output tokens | Cached tokens | Cost (USD) |",
			"|---|---|---|---|---|")
		for _, r := range rows {
			lines = append(lines, fmt.Sprintf("| %s | %s | %s | %s | $%.2f |", nameOf(r.pid),
				py.Thousands(r.tok[0]), py.Thousands(r.tok[1]), py.Thousands(r.tok[2]), r.cost))
		}
		lines = append(lines, "")
	}
	if active {
		started, added := []bar{}, []bar{}
		for _, pid := range order {
			started = append(started, bar{pid, counterAt(counts, pid).Get("start")})
			added = append(added, bar{pid, counterAt(counts, pid).Get("added")})
		}
		lines = append(lines, bars(fmt.Sprintf("Runs started, by subscription (%s)", strings.ToLower(w.Name)),
			"runs", started, false, "none")...)
		lines = append(lines, "")
		lines = append(lines, bars(fmt.Sprintf("Lines added in merged pull requests, by builder (%s)",
			strings.ToLower(w.Name)), "lines", added, false, "none")...)
		lines = append(lines, "")
		if w.Start.IsZero() {
			hours := []bar{}
			for _, pid := range order {
				hours = append(hours, bar{pid, minutesOf(w, pid) / 60})
			}
			lines = append(lines, bars("Model hours, by subscription (all time)", "hours", hours, false,
				"nothing yet")...)
			lines = append(lines, "")
			outcomes := w.Outcomes()
			endings := []bar{}
			for _, kind := range kinds {
				endings = append(endings, bar{kind, outcomes.Get(kind)})
			}
			lines = append(lines, bars("What runs ended in (all time)", "runs", endings, true, "nothing yet")...)
			lines = append(lines, "")
		}
	}
	return append(lines, "</details>", "")
}

// OverTime is the last two weeks day by day: runs started, pull requests merged and lines added.
func OverTime(ctx *hctx.Context, w *Window) []string {
	now := ctx.Now()
	var days []string
	for i := DaysShown - 1; i >= 0; i-- {
		days = append(days, Local(ctx, now).AddDate(0, 0, -i).Format("01-02"))
	}

	day := func(at any) string {
		when := clock.ParseISOValue(at)
		if when.IsZero() {
			return ""
		}
		return Local(ctx, when).Format("01-02")
	}

	perDay := py.NewCounter()
	for _, e := range w.Starts() {
		perDay.Add(day(e.At), 1)
	}
	mergedDay := py.NewCounter()
	for _, p := range w.Merged {
		mergedDay.Add(day(p.Get("merged_at")), 1)
	}
	addedDay := py.NewCounter()
	for _, p := range w.Merged {
		addedDay.Add(day(p.Get("merged_at")), py.MustInt(py.Or(p.Get("additions"), 0)))
	}
	perDays := make([]int, len(days))
	merged, added := []bar{}, []bar{}
	for i, d := range days {
		perDays[i] = perDay.Get(d)
		merged = append(merged, bar{d, mergedDay.Get(d)})
		added = append(added, bar{d, addedDay.Get(d)})
	}
	lines := append(line(fmt.Sprintf("Runs started per day (last %d days)", DaysShown), "runs", days, perDays), "")
	lines = append(lines, bars(fmt.Sprintf("Pull requests merged per day (last %d days)", DaysShown),
		"merged", merged, true, "nothing yet")...)
	lines = append(lines, "")
	lines = append(lines, bars(fmt.Sprintf("Lines added per day in merged pull requests (last %d days)", DaysShown),
		"lines", added, true, "nothing yet")...)
	return append(lines, "")
}

func builtText(built string) string {
	if built != Unrecorded {
		return "`" + built + "`"
	}
	return Unrecorded
}

// MergedTable is every pull request merged, newest first, with who planned, built, revised and
// approved it, folded.
func MergedTable(ctx *hctx.Context, merged []*pyjson.Object, who *Credits, shown int) []string {
	if len(merged) == 0 {
		return []string{"_No pull request has merged yet._", ""}
	}
	head_ := fmt.Sprintf("<b>Pull requests merged</b>: the newest %d of "+
		"%d, with who planned, built, revised and approved each", min(shown, len(merged)), len(merged))
	if shown == 0 {
		return []string{fmt.Sprintf("_%d pull requests merged; the list is left out to keep this issue "+
			"under GitHub's size limit._", len(merged)), ""}
	}
	lines := []string{fmt.Sprintf("<details><summary>%s</summary>", head_), "",
		"| Pull request | Merged | Planned | Built | Revised | Approved | Open for | Lines " +
			"| Files |", "|---|---|---|---|---|---|---|---|---|"}
	for _, p := range head(merged, shown) {
		number := py.MustInt(p.Get("number"))
		took := minus(clock.ParseISOValue(p.Get("merged_at")), clock.ParseISOValue(p.Get("created_at")))
		built := who.Builder(p)
		when := Local(ctx, clock.ParseISOValue(p.Get("merged_at"))).Format("Mon 01-02 15:04")
		lines = append(lines, fmt.Sprintf("| #%d %s | %s | %s "+
			"| %s "+
			"| %s | %s "+
			"| %s | %s | %d |",
			number, titleOf(p), when, whoText(who.Planned[number]),
			builtText(built),
			whoText(who.Revised[number]), whoText(who.Approved[number]),
			clock.HumanDelta(took), linesText([]*pyjson.Object{p}), py.MustInt(py.Or(p.Get("changed_files"), 0))))
	}
	if len(merged) > shown {
		lines = append(lines, fmt.Sprintf("\n_… and %d older ones._", len(merged)-shown))
	}
	return append(lines, "", "</details>", "")
}

// Modes is Squishy's modes (#60), one row each, one column per window.
func Modes(windows []*Window) []string {
	rows := []row{
		{"One-shot builds started", func(w *Window) string { return itoa(countStarts(w, "oneshot")) }},
		{"Splits run", func(w *Window) string { return itoa(countStarts(w, "split")) }},
		{"Sub-issues they opened", func(w *Window) string {
			n := 0
			for _, e := range w.Events {
				if e.Kind == "split" {
					n += e.Count
				}
			}
			return itoa(n)
		}},
		{"Trees closed, their end state holding", func(w *Window) string {
			n := 0
			for _, e := range w.Events {
				if e.Kind == "closed tree" {
					n++
				}
			}
			return itoa(n)
		}},
	}
	return table(windows, rows)
}

var (
	SectionStart = "<!-- jackioh-bot:statistics-section -->"
	SectionEnd   = "<!-- /jackioh-bot:statistics-section -->"
)

// SectionMerges is how many of its merged pull requests a bot's section lists.
var SectionMerges = 10

func windowsOf(now time.Time, facts *Facts) []*Window {
	windows := make([]*Window, len(Windows))
	for i, w := range Windows {
		windows[i] = Window_(w.Name, w.Span, now, facts)
	}
	return windows
}

// BotSection is this bot's section of the night bot's statistics (#60): Squishy's record, drawn by
// its own process from its own comments, pull requests, runs and state, and folded in the status
// issue.
func BotSection(ctx *hctx.Context) string {
	facts := Collect(ctx)
	now := ctx.Now()
	who := Credits_(facts.Events, facts.Pulls)
	windows := windowsOf(now, facts)
	lines := []string{SectionStart, fmt.Sprintf("<details><summary>🫧 <b>%s's record</b>: "+
		"%d pull requests merged</summary>", config.Title, len(windows[len(windows)-1].Merged)), "",
		fmt.Sprintf("_What @%s has done, read the same way as the night bot's record "+
			"above. What it is doing now is in its section above, and `%s status`._", ctx.Cfg.BotLogin,
			config.Slash), ""}
	lines = append(append(lines, Summary(windows)...), "")
	if pyjson.Truthy(config.Modes) {
		lines = append(append(append(lines, "**Its modes**", ""), Modes(windows)...), "")
	}
	merged := windows[len(windows)-1].Merged
	lines = append(lines, fmt.Sprintf("**Its pull requests merged** (%d)", len(merged)), "")
	if len(merged) == 0 {
		lines = append(lines, "None yet.", "")
	} else {
		lines = append(lines, "| Pull request | Built | Approved | Open for | Lines |", "|---|---|---|---|---|")
		for _, p := range head(merged, SectionMerges) {
			number := py.MustInt(p.Get("number"))
			took := minus(clock.ParseISOValue(p.Get("merged_at")), clock.ParseISOValue(p.Get("created_at")))
			built := who.Builder(p)
			lines = append(lines, fmt.Sprintf("| #%d %s "+
				"| %s "+
				"| %s | %s "+
				"| %s |", number, titleOf(p), builtText(built), whoText(who.Approved[number]),
				clock.HumanDelta(took), linesText([]*pyjson.Object{p})))
		}
		if len(merged) > SectionMerges {
			lines = append(lines, fmt.Sprintf("\n_… and %d older ones._", len(merged)-SectionMerges))
		}
		lines = append(lines, "")
	}
	return strings.Join(append(lines, "</details>", "", SectionEnd), "\n")
}

// Block is the statistics block of the status issue, between its markers: the four windows side
// by side, the last two weeks, which set's cards win most (`cards`, `cardsets.section`), each
// window folded, the merged pull requests folded, and the other bots' sections (`extra`,
// `bot_section`) folded at the end.
func Block(ctx *hctx.Context, facts *Facts, shown int /* = MERGES_SHOWN */, extra []string /* = () */, cards string /* = "" */) string {
	now := ctx.Now()
	who := Credits_(facts.Events, facts.Pulls)
	windows := windowsOf(now, facts)
	pulls := facts.Pulls
	var first time.Time
	if len(facts.Events) > 0 {
		first = clock.ParseISOValue(facts.Events[0].At)
	}
	openNow, closedUnmerged := 0, 0
	for _, p := range pulls {
		if p.Get("state") == "open" {
			openNow++
		}
	}
	for _, p := range pulls {
		if p.Get("state") == "closed" && !pyjson.Truthy(p.Get("merged_at")) {
			closedUnmerged++
		}
	}
	endedRuns := py.NewCounter()
	for _, r := range facts.Runs {
		endedRuns.Add(r[1], 1)
	}
	counted := Local(ctx, now).Format("15:04")
	since := "—"
	if !first.IsZero() {
		since = Local(ctx, first).Format("2006-01-02")
	}
	var endings []string
	for _, e := range endedRuns.MostCommon(-1) {
		endings = append(endings, fmt.Sprintf("%s %d", e.Key, e.N))
	}
	endingsText := strings.Join(endings, ", ")
	if endingsText == "" {
		endingsText = "none yet"
	}
	// Under the heading, a dropdown that starts open (dashboard.fold, which this module cannot
	// import: dashboard imports it).
	lines := []string{BlockStart, "## 📊 What it has done", "",
		"<details open><summary>Runs, pull requests, lines and model time, over the last 6 " +
			"hours, day, week and all time</summary>", "",
		fmt.Sprintf("_Counted at %s, afresh every %d "+
			"minutes. Active since "+
			"%s. Now: "+
			"%d of its pull requests open, %d closed without merging, "+
			"%d commits on `main` by the bot itself. "+
			"`%s` runs ended: "+
			"%s._", counted, int(StatsEvery/time.Minute), since, openNow, closedUnmerged,
			facts.CommitsOnMain, config.NightWorkflow, endingsText), ""}
	lines = append(append(append(lines, "### The four windows", ""), Summary(windows)...), "")
	lines = append(append(lines, "### The last two weeks", ""), OverTime(ctx, windows[len(windows)-1])...)
	if cards != "" {
		lines = append(lines, strings.TrimSpace(cards), "")
	}
	lines = append(lines, "### Per window", "",
		"<sub>Runs, outcomes and who did what come from the bot's comments; lines, files "+
			"and commits from its merged pull requests; model time, tokens and cost from the "+
			"state file's dated record of each subscription's runs. A run counts in the window "+
			"it started in, an outcome (a pull request opened, a revision pushed, a plan, an "+
			"approval, or a pause, failure or run that could not start) in the window it was "+
			"posted in, and a pull request in the window it was opened or merged in.</sub>", "")
	for _, w := range windows {
		lines = append(lines, Section(ctx, w, who, facts)...)
	}
	lines = append(lines, MergedTable(ctx, windows[len(windows)-1].Merged, who, shown)...)
	for _, part := range extra {
		lines = append(lines, strings.TrimSpace(part), "")
	}
	return strings.Join(append(lines, "", "</details>", BlockEnd), "\n")
}

// Kept is the statistics block a rewrite of the status issue keeps between counts, or "" (None).
func Kept(body string) string {
	start, end := strings.Index(body, BlockStart), strings.Index(body, BlockEnd)
	if start < 0 || end < start {
		return ""
	}
	return body[start : end+len(BlockEnd)]
}

// Due is stats.due.
func Due(ctx *hctx.Context, state *pyjson.Object) bool {
	last := clock.ParseISOValue(state.Obj("statistics").Get("at"))
	return last.IsZero() || ctx.Now().Sub(last) >= StatsEvery
}

// Counted notes in the state file that the statistics were counted now.
func Counted(ctx *hctx.Context) {
	at := clock.ISO(ctx.Now())
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.Set("statistics", pyjson.New("at", at))
		return nil
	}, "statistics")
}
