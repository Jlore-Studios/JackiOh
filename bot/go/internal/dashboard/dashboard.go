// Package dashboard is bot/harness/dashboard.py.
//
// The pinned status issue (#148): one issue, pinned to the top of the issue list, that the
// `bot-status` loop rewrites every five minutes with what the night bot is doing and what it has
// done.
//
// It opens with its date, then a bar chart of how far the owner's PC's two local jobs have played (the
// learn lane's export of `LEARN_GAMES` with its shard, and gather's games of its patch, from the PC's
// heartbeat: `progress`), then a headline (what runs, what waits and on what, which subscriptions can
// start work), then draws the same facts as `/harness status` (`status.py`) for a glance: what each
// lane is doing and a timeline of the runs going now, the PC's jobs among them; every subscription
// as a box with what it is doing or why it
// is not, and its usage, always; every subscription's lanes, held and open, Squishy's too, with the
// totals and the limits on them (`lanes_table`); the queue, each item with the tier its difficulty needs and what it
// waits for (`waiting.py`, the planner's own rules on the lanes held now: never "free" for a
// subscription the planner would give no work, never "starts next" for an item it would not start);
// each subscription's usage as bars and charts against its caps; the last runs; the statistics
// (`stats.py`: the last six hours, day, week and all time, counted every 30 minutes and kept in the
// issue between counts, with which set's cards win most, `cardsets.py`, read from production once a
// day); and the full status text folded underneath (left out first when the issue would
// pass GitHub's limit, `fit`: `/harness status` gives it). Times are clock times, which
// stay true between rewrites; hovering one shows how long it had been running when the issue was
// written. Editing an issue's body notifies nobody, so the rewrites stay quiet. The issue's number
// is kept in the state file; when it is missing or the issue was closed, an open issue with the
// title and the marker, opened by the bot or someone on the trust list, is adopted, or a new one is
// opened and pinned.
//
// Every runner has a section too, low in the issue (`runners_section`): each job a runner holds now,
// the step it is on and what it is for, and the jobs queued for one, as boxes the way the
// subscriptions are drawn and as a table, each folding away. The owner's PC's group carries its load
// and the gather workflow, from a heartbeat its supervisor writes to a gist (`read_runners`). Between
// rewrites the loop redraws that section and the progress chart alone every two minutes
// (`refresh_runners`). What each runner
// and each workflow is for changes only with the pool or the workflows, so it is a comment on the issue
// (`explain`), which the section links.
//
// Squishy (#60) has a section of its own in the same issue, apart from the night bot's: what it is
// working on, its account, its queue and its trees. Squishy's own process draws it (`section`), and
// the night bot's status loop runs that process each tick and puts the section in (`update`'s
// `extra`), so each bot's facts come from its own state and labels. Its record goes in the
// statistics, folded, drawn the same way (`stats.bot_section`).
//
// The live lanes (`dict[int, str]` in the Python, item number to provider id) are a
// *pyjson.Object keyed by the item's number in decimal, in the order the Python's dict holds them.
package dashboard

import (
	"fmt"
	"regexp"
	"slices"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stats"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/waiting"
)

// Title is TITLE.
var Title = "Night bot status"

// Marker is MARKER.
var Marker = "<!-- jackioh-bot:dashboard -->"

// Labels is LABELS.
var Labels = []string{"night bot"}

// EveryMinutes is how often the `bot-status` loop rewrites the issue (`EVERY_SECONDS` in
// bot-status.yml).
var EveryMinutes = 5

// MaxBody is MAX_BODY: GitHub refuses an issue body over 65,536 characters; the issue keeps under
// this (`fit`). A variable the tests replace.
var MaxBody = 65_000

// defaultRoom is `runners_section`'s default `room`: MAX_BODY as it was when the Python defined
// the function, so a test that patches MAX_BODY leaves it as it was, as it does in the Python.
var defaultRoom = MaxBody

// How much of each list the issue shows.
var (
	QueueRows = 15
	RunRows   = 8
	BarCells  = 10
	BoxCells  = 5
)

// Result is RESULT.
var Result = map[string]string{"success": "✅", "failure": "❌", "cancelled": "⏹️", "skipped": "⏭️",
	"timed_out": "⌛", "startup_failure": "❌"}

// ShownSteps is how the statistics block shrinks when the issue would pass `MAX_BODY`: fewer
// merged pull requests listed.
var ShownSteps = []int{stats.MergesShown, py.FloorDiv(stats.MergesShown, 2), 10, 0}

// Local is _local: `at` in the bot's time zone.
func Local(ctx *hctx.Context, at time.Time) time.Time {
	if at.IsZero() {
		panic(noneAttr("astimezone"))
	}
	return at.In(clock.Zone(ctx.Cfg.Timezone))
}

// clockText is _clock: `20:27` today (Central time), `Fri 18:47` on another day.
func clockText(ctx *hctx.Context, at time.Time) string {
	local, today := Local(ctx, at), Local(ctx, ctx.Now())
	ly, lm, ld := local.Date()
	ty, tm, td := today.Date()
	if ly == ty && lm == tm && ld == td {
		return local.Format("15:04")
	}
	return local.Format("Mon 15:04")
}

// hover is _hover: `text` that shows `title` when hovered: a link with a title when there is
// somewhere to go, otherwise a span (GitHub keeps `title` on both, and strips `<abbr>`).
func hover(text, title, url string) string {
	if url != "" {
		return "[" + text + "](" + url + ` "` + strings.ReplaceAll(title, `"`, "'") + `")`
	}
	return `<span title="` + htmlEscape(title) + `">` + text + "</span>"
}

// runningFor is _running_for: how long a run had been going when this rewrite was written.
func runningFor(ctx *hctx.Context, since time.Time) string {
	written := clockText(ctx, ctx.Now())
	elapsed := clock.HumanDelta(ctx.Now().Sub(since))
	if elapsed == "now" {
		return "just started when this was written at " + written
	}
	return "running " + elapsed + " when this was written at " + written
}

// runURL is _run_url.
func runURL(ctx *hctx.Context, runID any) string {
	if !pyjson.Truthy(runID) || strings.HasPrefix(py.Str(runID), "pending:") {
		return "" // the status loop started it and has not found its run yet (#1455)
	}
	return ctx.Cfg.ServerURL + "/" + ctx.Cfg.Repo + "/actions/runs/" + py.Str(runID)
}

// Cell is _cell: text for a table cell: one line, no pipes, clipped.
func Cell(text any, limit int /* = 70 */) string {
	flat := strings.ReplaceAll(strings.Join(strings.Fields(py.OrStr(text, "")), " "), "|", `\|`)
	if py.Len(flat) <= limit {
		return flat
	}
	return py.SliceTo(flat, limit-1) + "…"
}

// Bar is bar: `▰▰▰▱▱▱▱▱▱▱ 30%`, or a dash with no reading (nil).
func Bar(fraction *float64, cells int /* = BAR_CELLS */) string {
	if fraction == nil {
		return "—"
	}
	f := *fraction // min(max(float(fraction), 0.0), 1.0), Python's way round with NaN and -0.0
	if 0.0 > f {
		f = 0.0
	}
	if 1.0 < f {
		f = 1.0
	}
	full := py.Round(f * float64(cells))
	return strings.Repeat("▰", full) + strings.Repeat("▱", max(0, cells-full)) + " " + py.Percent(f, 0)
}

// barOf is bar(fraction, cells) with a reading.
func barOf(fraction float64, cells int) string { return Bar(&fraction, cells) }

// windowReading is _window_reading: a usage window as (used now, when it reset since the reading,
// what the reading said, and whether there is one). Used now is the reading, 0 once the window has
// reset since it (as the planner counts it), or none with no reading; a reading from before the
// subscription's plan changed counts as none.
func windowReading(ctx *hctx.Context, provider *providers.Provider, entry *pyjson.Object,
	window string) (used float64, reset time.Time, read float64, ok bool) {
	usage := providers.CurrentUsage(provider, entry)
	reading, _ := usage.Get(window).(*pyjson.Object)
	if reading == nil {
		return 0, time.Time{}, 0, false
	}
	read, err := py.Float(reading.GetOr("utilization", 0))
	if err != nil {
		return 0, time.Time{}, 0, false
	}
	resets := clock.ParseISOValue(reading.Get("resets_at"))
	if !resets.IsZero() && !resets.After(ctx.Now()) {
		return 0.0, resets, read, true
	}
	return read, time.Time{}, read, true
}

// doing is _doing: what a held lane is doing: its verb (`building`), the item (`#37`, or
// `survey`), and since when.
func doing(state *pyjson.Object, number int) (string, string, time.Time) {
	record := status.Record(state, number)
	if number == 0 {
		return "surveying", "survey", clock.ParseISOValue(record.Get("last_run"))
	}
	kind := py.Str(record.Get("kind"))
	if pyjson.Equal(record.Get("action"), "plan") {
		kind = "plan"
	}
	verb, ok := status.Doing[kind]
	if !ok {
		verb = "working on"
	}
	return verb, "#" + strconv.Itoa(number), clock.ParseISOValue(record.Get("started_at"))
}

// lane is one live lane: the item's number (0 for the suggestion survey) and its provider's id.
type lane struct {
	number   int
	provider string
}

// lanesIn is live.items().
func lanesIn(live *pyjson.Object) []lane {
	var out []lane
	live.Items(func(key string, v any) bool {
		n, _ := strconv.Atoi(key)
		out = append(out, lane{n, py.Str(v)})
		return true
	})
	return out
}

// byStart is _by_start: the live lanes, the longest-running first.
func byStart(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object) []lane {
	lanes := lanesIn(live)
	started := make(map[int]time.Time, len(lanes))
	for _, l := range lanes {
		_, _, since := doing(state, l.number)
		if since.IsZero() {
			since = ctx.Now()
		}
		started[l.number] = since
	}
	sort.SliceStable(lanes, func(i, j int) bool {
		a, b := started[lanes[i].number], started[lanes[j].number]
		if !a.Equal(b) {
			return a.Before(b)
		}
		return lanes[i].number < lanes[j].number
	})
	return lanes
}

// ------------------------------------------------------------------ what is running now

// RunningTable is running_table: what each lane is doing now. The start time links to the run
// doing it; hovering it shows how long the run had been going when the issue was written.
func RunningTable(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object,
	issues []*pyjson.Object) []string {
	if live.Len() == 0 {
		return nil
	}
	cfg := ctx.Cfg
	titles := map[int]any{}
	for _, issue := range issues {
		titles[py.MustInt(issue.Get("number"))] = issue.Get("title")
	}
	var rows []string
	for _, l := range byStart(ctx, state, live) {
		record := status.Record(state, l.number)
		verb, item, since := doing(state, l.number)
		where := "GitHub"
		if cfg.Pool.OnMachine(l.provider) {
			where = "machine"
		}
		model := ""
		if provider := cfg.Pool.Get(l.provider); provider != nil {
			model = " `" + provider.Model + "`"
		}
		url := runURL(ctx, record.Get("run_id"))
		var started string
		switch {
		case !since.IsZero():
			started = hover(clockText(ctx, since), runningFor(ctx, since), url)
		case url != "":
			started = "[run](" + url + ")"
		default:
			started = "—"
		}
		title, ok := titles[l.number]
		if !ok {
			title = ""
		}
		if l.number == 0 {
			item, verb = "—", "suggestion survey"
		}
		rows = append(rows, "| "+item+" | "+Cell(title, 50)+" | "+verb+" | `"+l.provider+"`"+model+
			" ("+where+") | "+started+" |")
	}
	return append([]string{"| Item | Title | Doing | Subscription | Started (Central) |",
		"|---|---|---|---|---|"}, rows...)
}

// tick is _tick: a tick interval that puts a handful of labels on the time axis.
func tick(span time.Duration) string {
	for _, t := range []struct {
		limit time.Duration
		tick  string
	}{{time.Hour, "10minute"}, {3 * time.Hour, "30minute"}, {8 * time.Hour, "1hour"}} {
		if span <= t.limit {
			return t.tick
		}
	}
	return "2hour"
}

// TimelineStyle is TIMELINE_STYLE: the timeline's bars carry their own colours, as the boxes do
// (`BOX_STYLES`): dark blue on light blue in a bar. A bar too short for its name draws it beside the
// bar, on the page itself (Mermaid draws every section's band at a fifth of its colour), so that text
// is GitHub's muted grey, which reads on its light page and its dark one alike (4.5:1 and 3.9:1).
// Bars are plain tasks, not `active` ones: Mermaid gives an active bar's name one colour inside the
// bar and beside it, and GitHub's dark theme made it dark, unreadable beside a short bar.
var TimelineStyle = `%%{init: {"themeVariables": {"taskBkgColor": "#ddf4ff", "taskBorderColor": ` +
	`"#54aeff", "taskTextColor": "#0a3069", "taskTextOutsideColor": "#6e7781"}}}%%`

// Timeline is timeline: a Mermaid timeline of the runs going now: GitHub's runners, then the
// machine, the longest-running first, each bar named by its subscription, what it does and for how
// long; then, with the runners `found`, the jobs on the owner's PC (the learn lane's, gather's),
// each named by its workflow and job. (A gantt chart reads `#` as a comment, so items are named
// without it.)
func Timeline(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object,
	found *Found /* = None */) []string {
	var pcJobs [][2]*pyjson.Object
	if found != nil {
		for _, pair := range found.Busy {
			if order, _ := Where(pair[1]); order == 3 {
				pcJobs = append(pcJobs, pair)
			}
		}
	}
	if live.Len() == 0 && len(pcJobs) == 0 {
		return []string{"Nothing is running right now."}
	}
	pool := ctx.Cfg.Pool
	now := Local(ctx, ctx.Now())
	const layout = "2006-01-02 15:04" // "%Y-%m-%d %H:%M"
	rows := map[string][]string{"hosted": nil, "night": nil, "pc": nil}
	earliest := now

	add := func(where, name string, since time.Time) {
		start := now
		if !since.IsZero() {
			start = Local(ctx, since)
		}
		if start.Before(earliest) {
			earliest = start
		}
		end := now // a run that just started still shows
		if start.Add(time.Minute).After(now) {
			end = start.Add(time.Minute)
		}
		took := clock.HumanDelta(now.Sub(start))
		if took == "now" {
			took = "just started"
		}
		name += " · " + took
		name = strings.ReplaceAll(strings.ReplaceAll(name, ":", " "), "#", "")
		rows[where] = append(rows[where], "    "+name+" :"+start.Format(layout)+", "+end.Format(layout))
	}
	for _, l := range byStart(ctx, state, live) {
		verb, item, since := doing(state, l.number)
		where := "hosted"
		if pool.OnMachine(l.provider) {
			where = "night"
		}
		add(where, l.provider+" · "+verb+" "+strings.TrimLeft(item, "#"), since)
	}
	for _, pair := range pcJobs {
		add("pc", jobName(pair[0], pair[1]), clock.ParseISOValue(pair[1].Get("started_at")))
	}
	lines := []string{"```mermaid", TimelineStyle, "gantt",
		"    title Runs going now, as of " + now.Format("15:04") + " (Central time)",
		"    dateFormat YYYY-MM-DD HH:mm", "    axisFormat %H:%M",
		"    tickInterval " + tick(now.Sub(earliest)), "    todayMarker off"}
	for _, s := range [][2]string{{"hosted", "GitHub's runners"}, {"night", "The night box"},
		{"pc", "The owner's PC"}} {
		if len(rows[s[0]]) > 0 {
			lines = append(lines, "    section "+s[1])
			lines = append(lines, rows[s[0]]...)
		}
	}
	return append(lines, "```")
}

// BoxStyles is BOX_STYLES: the boxes' colours: they carry their own text colour, so they read in
// GitHub's light and dark themes alike.
var BoxStyles = []string{
	"    classDef busy fill:#1f883d,stroke:#116329,color:#ffffff",
	"    classDef next fill:#0969da,stroke:#0550ae,color:#ffffff",
	"    classDef idle fill:#ddf4ff,stroke:#54aeff,color:#0a3069",
	"    classDef paused fill:#fff8c5,stroke:#d4a72c,color:#4d2d00",
	"    classDef closed fill:#eaeef2,stroke:#8c959f,color:#24292f",
	"    classDef broken fill:#ffebe9,stroke:#cf222e,color:#82071e",
	"    classDef off fill:#f6f8fa,stroke:#d0d7de,color:#6e7781,stroke-dasharray:4 3",
}

var styleUse = regexp.MustCompile(`:::(\w+)`)

// Styles is styles: the `classDef`s of `BOX_STYLES` and `extra` that `lines` use (`:::name`): a
// chart declaring the rest only spends the issue's room (`MAX_BODY`).
func Styles(lines []string, extra []string /* = () */) []string {
	used := map[string]bool{}
	for _, m := range styleUse.FindAllStringSubmatch(strings.Join(lines, "\n"), -1) {
		used[m[1]] = true
	}
	var out []string
	for _, style := range slices.Concat(BoxStyles, extra) {
		if used[strings.Fields(style)[1]] {
			out = append(out, style)
		}
	}
	return out
}

// Style is STYLE: each kind of `waiting.Now` as a box style.
var Style = map[string]string{"busy": "busy", "next": "next", "idle": "idle", "reviews": "paused",
	"full": "paused", "capped": "paused", "closed": "closed", "broken": "broken", "off": "off"}

// Legend is LEGEND.
var Legend = "🟢 working · ▶️ takes queued work at the next run · ⚪ idle: it may start work, and " +
	"nothing queued is for it · ⏸️ at or near a usage cap (when it can start a build or a " +
	"plan again), or no lane free where it runs · 🌙 outside its hours · ⛔ backing off " +
	"after a run that could not work (a login or the CLI) · ⚫ off or suspended. Usage is " +
	"the last reading (5h: the 5-hour session, 7d: the week), against its cap now."

// miniUsage is _mini_usage: a box's usage lines: `5h ▰▰▱▱▱ 42% · cap 40%`, one per window with a
// reading or a cap.
func miniUsage(ctx *hctx.Context, provider *providers.Provider, entry *pyjson.Object) []string {
	caps := provider.CapsAt(ctx.Now(), ctx.Cfg.Timezone, providers.CurrentUsage(provider, entry),
		time.Time{})
	var lines []string
	for _, w := range [][2]string{{"five_hour", "5h"}, {"seven_day", "7d"}} {
		window, short := w[0], w[1]
		used, reset, _, ok := windowReading(ctx, provider, entry, window)
		capValue := caps.Get(window)
		if !ok && capValue == nil {
			continue
		}
		capText := ""
		if capValue != nil {
			if c, _ := pyjson.AsFloat(capValue); c < 1.0 {
				capText = " · cap " + py.Percent(c, 0)
			}
		}
		switch {
		case !ok:
			lines = append(lines, short+" not read yet"+capText)
		case !reset.IsZero():
			lines = append(lines, short+" "+barOf(0.0, BoxCells)+" · reset "+clockText(ctx, reset)+capText)
		default:
			lines = append(lines, short+" "+barOf(used, BoxCells)+capText)
		}
	}
	provider.Limits.Budgets().Items(func(window string, budget any) bool {
		spent := providers.MinutesSpent(entry, ctx.Now().Add(-lookup(providers.Windows, window)))
		lines = append(lines, lookup(providers.WindowNames, window)+" "+fmt.Sprintf("%.0f", spent)+"/"+
			py.Str(budget)+" min")
		return true
	})
	if len(lines) == 0 {
		return []string{"usage not read yet"}
	}
	return lines
}

// nowFor is now[provider_id] of a `dict[str, waiting.Now]`, whose KeyError goes up.
func nowFor(now *pyjson.Object, providerID string) *waiting.Now {
	v, ok := now.Lookup(providerID)
	if !ok {
		panic(fmt.Errorf("KeyError: %s", py.Repr(providerID)))
	}
	switch n := v.(type) {
	case *waiting.Now:
		return n
	case waiting.Now:
		return &n
	}
	panic(fmt.Errorf("dashboard: %T is not a waiting.Now", v))
}

// box is _box: one subscription's box: its name, what it is doing or why not, and its usage.
func box(ctx *hctx.Context, state *pyjson.Object, provider *providers.Provider, now *waiting.Now,
	runs []int, name string) string {
	runLine := func(number int) string {
		verb, item, since := doing(state, number)
		line := "🟢 " + verb + " " + item
		if !since.IsZero() {
			line += " · since " + clockText(ctx, since)
		}
		return line
	}

	lines := []string{"<b>" + provider.ID + "</b>"}
	var style string
	if len(runs) > 0 {
		for _, n := range runs {
			lines = append(lines, runLine(n))
		}
		style = "busy"
		var queued []string
		for _, n := range now.Items {
			if !slices.Contains(runs, n) {
				queued = append(queued, "#"+strconv.Itoa(n))
			}
		}
		if len(queued) > 0 {
			lines = append(lines, "▶️ takes "+strings.Join(queued, ", ")+" next")
		}
	} else {
		lines = append(lines, now.Text)
		style = lookup(Style, now.Kind)
	}
	lines = append(lines, miniUsage(ctx, provider, providers.PeekRecord(state, provider.ID))...)
	text := strings.ReplaceAll(strings.Join(lines, "<br/>"), `"`, "'")
	return "        " + name + `["` + text + `"]:::` + style
}

// TrainingLanes is TRAINING_LANES: the AI's training lanes on the training box that keep a pull
// request open from `ai/<lane>` (training/README.md, #1272). None since 2026-10-25: #682's audit
// stopped `improve` and `unban` (#1677), and `gather` left the box for the owner's PC (gather.yml,
// training/gather.md), where the runners section draws it, so no training box is drawn. A lane named
// here again shows its draft. A variable the tests replace.
var TrainingLanes = []string{}

// TrainingStale is TRAINING_STALE: a training lane's draft unchanged for longer than a session
// (`training/loop.sh`'s TRAIN_SESSION_SECONDS, five hours; each session pushes to it as it starts)
// says the lane stopped.
var TrainingStale = 6 * time.Hour

// TrainingBoxes is training_boxes: the training box's lanes as a subgraph, read from their pull
// requests: a draft is a Muse session at work (or an idle unban lane, which its title says), one
// ready for review is a promotion waiting on CI, and none is a lane between sessions or stopped.
// [] with no lane in `TRAINING_LANES`, or when the pull requests cannot be read.
func TrainingBoxes(ctx *hctx.Context) []string {
	if len(TrainingLanes) == 0 {
		return nil
	}
	pulls := map[string]*pyjson.Object{}
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		for _, p := range ctx.GH.ListPulls("open", "", 200) {
			pulls[py.Str(p.Obj("head").Get("ref"))] = p
		}
	}); failed {
		return nil
	}
	var boxes []string
	working := 0
	for i, lane := range TrainingLanes {
		pull := pulls["ai/"+lane]
		var text, style string
		if pull == nil {
			text, style = "no pull request open<br/>between sessions, or stopped", "closed"
		} else {
			number, updated := py.Str(pull.Get("number")), clock.ParseISOValue(pull.Get("updated_at"))
			since := ""
			if !updated.IsZero() {
				since = "<br/>updated " + clockText(ctx, updated)
			}
			switch {
			case !pull.Truthy("draft"):
				text, style = "▶️ promotion #"+number+"<br/>waiting on CI"+since, "next"
			case strings.Contains(py.OrStr(pull.Get("title"), ""), "): idle,"):
				text, style = "⏸️ idle on #"+number+"<br/>the AI on main bans no card", "paused"
			case !updated.IsZero() && ctx.Now().Sub(updated) > TrainingStale:
				text, style = "⛔ #"+number+" unchanged"+since+"<br/>is the lane running?", "broken"
			default:
				text, style = "🟢 a Muse session on #"+number+since, "busy"
				working++
			}
		}
		boxes = append(boxes, fmt.Sprintf(`        t%d["<b>%s</b><br/>%s"]:::%s`, i, lane, text, style))
	}
	title := fmt.Sprintf("The training box, on Muse: %d of %d lanes working", working, len(TrainingLanes))
	lines := append([]string{`    subgraph training["` + title + `"]`, "        direction LR"}, boxes...)
	if len(TrainingLanes) > 1 {
		lines = append(lines, "        "+chain("t", len(TrainingLanes)))
	}
	return append(lines, "    end")
}

// chain is `" ~~~ ".join(f"{prefix}{i}" for i in range(n))`: a row of invisible links.
func chain(prefix string, n int) string {
	nodes := make([]string, n)
	for i := range nodes {
		nodes[i] = prefix + strconv.Itoa(i)
	}
	return strings.Join(nodes, " ~~~ ")
}

// LanesBoxes is lanes_boxes: every subscription as a box: GitHub's runners (the Claude accounts),
// then the night box's (each with the runs it has there), each saying what it is doing, or why it
// is not, and its usage.
func LanesBoxes(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object,
	now *pyjson.Object /* = None */) []string {
	pool := ctx.Cfg.Pool
	if now == nil {
		now = waiting.Subscriptions(ctx, state, waiting.LanesOf(ctx, state, live),
			waiting.LanesOf(ctx, state, live), pyjson.New(), "")
	}
	runs := map[string][]int{}
	for _, l := range byStart(ctx, state, live) {
		runs[l.provider] = append(runs[l.provider], l.number)
	}
	// By name (claude-1 to claude-8), so each keeps its place from one rewrite to the next.
	var hosted, machine []*providers.Provider
	for _, p := range pool.Ordered() {
		if !pool.OnMachine(p.ID) {
			hosted = append(hosted, p)
		}
	}
	for _, p := range pool.Ordered() {
		if pool.OnMachine(p.ID) {
			machine = append(machine, p)
		}
	}
	byID := func(ps []*providers.Provider) {
		sort.SliceStable(ps, func(i, j int) bool { return ps[i].ID < ps[j].ID })
	}
	byID(hosted)
	byID(machine)
	busyHosted, onMachine, claude := 0, 0, true
	for _, p := range hosted {
		if len(runs[p.ID]) > 0 {
			busyHosted++
		}
		if p.CLI != "claude" {
			claude = false
		}
	}
	for _, p := range machine {
		onMachine += len(runs[p.ID])
	}
	hostedTitle := "GitHub's runners"
	if claude && len(hosted) > 0 {
		hostedTitle = "Claude accounts, on GitHub's runners"
	}
	if claude && len(hosted) == 1 && pool.MachineParallel == 0 {
		hostedTitle = config.Title + "'s Claude account, on GitHub's runners"
	}
	lines := []string{"```mermaid", "flowchart TB"}
	if len(hosted) > 0 {
		lines = append(lines, fmt.Sprintf(`    subgraph hosted["%s: %d of %d working"]`, hostedTitle,
			busyHosted, len(hosted)), "        direction LR")
		for i, p := range hosted {
			lines = append(lines, box(ctx, state, p, nowFor(now, p.ID), runs[p.ID], fmt.Sprintf("h%d", i)))
		}
		if len(hosted) > 1 {
			lines = append(lines, "        "+chain("h", len(hosted)))
		}
		lines = append(lines, "    end")
	}
	// A pool that never runs on the machine (Squishy's, #60) draws no machine box, and no
	// training box: the AI's training lanes run outside the harness, on a box of their own, so
	// their boxes come from their pull requests (`training_boxes`).
	if len(machine) > 0 && pool.MachineParallel != 0 {
		lines = append(lines, fmt.Sprintf(`    subgraph machine["The night box: %d of %d slots in use"]`,
			onMachine, pool.MachineParallel), "        direction LR")
		for i, p := range machine {
			lines = append(lines, box(ctx, state, p, nowFor(now, p.ID), runs[p.ID], fmt.Sprintf("m%d", i)))
		}
		if len(machine) > 1 {
			lines = append(lines, "        "+chain("m", len(machine)))
		}
		lines = append(lines, "    end")
		if len(hosted) > 0 {
			lines = append(lines, "    hosted ~~~ machine")
		}
		training := TrainingBoxes(ctx)
		lines = append(lines, training...)
		if len(training) > 0 {
			lines = append(lines, "    machine ~~~ training")
		}
	}
	lines = append(lines, Styles(lines, nil)...)
	lines = append(lines, "```")
	planning := map[int]bool{}
	for _, l := range lanesIn(live) {
		if l.number != 0 && pyjson.Equal(status.Record(state, l.number).Get("action"), "plan") {
			planning[l.number] = true
		}
	}
	// GitHub's runners have lanes of their own, apart from the night box's slots (`plan.Lanes`).
	building := 0
	for _, l := range lanesIn(live) {
		if !planning[l.number] && !pool.OnMachine(l.provider) {
			building++
		}
	}
	planNote := ""
	if pool.PlanLanes != 0 {
		planNote = fmt.Sprintf("; planning %d of %d", len(planning), pool.PlanLanes)
	}
	return append(lines, "", "<sub>"+Legend+"</sub>", "",
		fmt.Sprintf("<sub>Lanes on GitHub's runners: %d of %d in use%s.</sub>", building,
			pool.MaxParallel, planNote))
}
