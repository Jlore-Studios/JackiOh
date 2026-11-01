// Package status is bot/harness/status.py.
//
// The `status` report: halt, which subscriptions are running what, each subscription, and what
// is queued.
package status

import (
	"fmt"
	"math"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/declined"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/reviewrule"
)

// Doing is what a run is doing to an item, by the item's kind (`queue.KIND_ORDER`).
var Doing = map[string]string{"plan": "planning", "build": "building", "revise": "revising",
	"review": "reviewing"}

// The lanes (`held`, `live`) are dict[int, str] in the Python: item number (0 for a survey) ->
// provider id. They are *pyjson.Object here, keyed by the number in decimal, in the Python's order.

func laneNumber(key string) int { n, _ := strconv.Atoi(key); return n }

// MachineText is how many runs are on the bot's machine: "2 of 6 on the night box".
func MachineText(pool *providers.Pool, live *pyjson.Object) string {
	night := 0
	for _, key := range live.Keys() {
		if pool.OnMachine(py.Str(live.Get(key))) {
			night++
		}
	}
	return fmt.Sprintf("%d of %d on the night box", night, pool.MachineParallel)
}

func numbers(items []*pyjson.Object) string {
	if len(items) == 0 {
		return "none"
	}
	shown := make([]string, len(items))
	for i, item := range items {
		shown[i] = "#" + py.Str(item.Get("number"))
	}
	return strings.Join(shown, ", ")
}

// usageArg is a reading as the `usage` argument: a None reading stays None.
func usageArg(usage *pyjson.Object) any {
	if usage == nil {
		return nil
	}
	return usage
}

// capOf is caps.get(window): the cap, and false for None.
func capOf(caps *pyjson.Object, window string) (float64, bool) {
	v := caps.Get(window)
	if v == nil {
		return 0, false
	}
	f, _ := pyjson.AsFloat(v)
	return f, true
}

// CapText is the cap on a usage window now: "cap 90%" or "no cap", and for a subscription whose
// cap changes with its hours (`off_hours`) the other one too: "cap 50% now, 70% in its hours".
// With its reading, a cap its last hour or its late week raised says so (`caps_at`).
func CapText(ctx *hctx.Context, provider *providers.Provider, window string, usage any /* = None */) string {
	cfg := ctx.Cfg
	now := ctx.Now()
	cap_, capped := capOf(provider.CapsAt(now, cfg.Timezone, usage, time.Time{}), window)
	text := "no cap"
	if capped {
		text = "cap " + py.Percent(cap_, 0)
	}
	usual, usualCapped := capOf(provider.CapsAt(now, cfg.Timezone, nil, time.Time{}), window)
	if capped && usualCapped && cap_ != usual {
		held, heldCapped := capOf(provider.HeldCaps(now, cfg.Timezone, usage), window)
		if heldCapped && held != cap_ { // its last hour lifted it
			return fmt.Sprintf("%s now (its last hour), %s after the reset", text, py.Percent(held, 0))
		}
		return fmt.Sprintf("%s now (its late week, from %s), "+
			"%s after the reset", text, provider.LateWeek.Describe(), py.Percent(usual, 0))
	}
	hours := provider.Schedule.Window(cfg.Timezone)
	if hours == nil || !provider.OffHours.Has(window) {
		return text
	}
	own, owned := capOf(provider.Limits.Stops(), window)
	offHours, _ := pyjson.AsFloat(provider.OffHours.Get(window))
	if hours.IsOpen(now) {
		outside := 1.0
		if owned {
			outside = own
		}
		outside = math.Min(outside, offHours)
		return fmt.Sprintf("%s now, %s outside its hours", text, py.Percent(outside, 0))
	}
	inHours := "none"
	if owned {
		inHours = py.Percent(own, 0)
	}
	return fmt.Sprintf("%s now, ", text) + inHours + " in its hours"
}

// windowSpan is providers.WINDOWS[window].
func windowSpan(window string) time.Duration {
	span, ok := providers.Windows[window]
	if !ok {
		panic(py.KeyError(window))
	}
	return span
}

// WindowNotes is what a reader needs beside a usage window's reading: its cap now (`cap_text`),
// the line a build, a revision or a plan must start under (`start_headroom`), the minutes it has
// spent of a budget, and when the window resets (`when` writes the time: "in 3h 10m" by default).
func WindowNotes(ctx *hctx.Context, provider *providers.Provider, entry *pyjson.Object, window string, when func(time.Time) string /* = None */) []string {
	now := ctx.Now()
	if when == nil {
		when = func(at time.Time) string { return "in " + clock.HumanDelta(at.Sub(now)) }
	}
	current := usageArg(providers.CurrentUsage(provider, entry))
	cap_, capped := capOf(provider.CapsAt(now, ctx.Cfg.Timezone, current, time.Time{}), window)
	budget := provider.Limits.Budgets().Get(window)
	usage, _ := entry.Get("usage").(*pyjson.Object)
	reading, isReading := usage.Get(window).(*pyjson.Object)
	var notes []string
	if capped || (budget == nil && isReading) {
		notes = append(notes, CapText(ctx, provider, window, current))
	}
	if capped {
		start := providers.StartUnder(provider, window, cap_)
		if start < cap_ {
			notes = append(notes, "builds and plans start under "+py.Percent(start, 0))
		}
	}
	if budget != nil {
		since := now.Add(-windowSpan(window))
		notes = append(notes, fmt.Sprintf("%.0f of %s min", providers.MinutesSpent(entry, since), py.Str(budget)))
	}
	if isReading {
		observed := clock.ParseISOValue(usage.Get("observed_at"))
		resets := clock.ParseISOValue(reading.Get("resets_at"))
		if resets.IsZero() && !observed.IsZero() {
			resets = observed.Add(windowSpan(window))
		}
		if !resets.IsZero() && resets.After(now) {
			notes = append(notes, "resets "+when(resets))
		}
	}
	return notes
}

func usageText(provider *providers.Provider, entry *pyjson.Object, ctx *hctx.Context) string {
	usage, isDict := py.Or(entry.Get("usage"), pyjson.New()).(*pyjson.Object)
	var parts []string
	noted := map[string]bool{} // the windows whose notes say their minutes
	for _, w := range [][2]string{{"five_hour", "5-hour"}, {"seven_day", "7-day"}} {
		window, label := w[0], w[1]
		var reading *pyjson.Object
		if isDict {
			reading, _ = usage.Get(window).(*pyjson.Object)
		}
		if reading == nil {
			continue
		}
		resets := clock.ParseISOValue(reading.Get("resets_at"))
		if !resets.IsZero() && !resets.After(ctx.Now()) {
			parts = append(parts, label+": reset since the reading")
			continue
		}
		text := fmt.Sprintf("%s %s", label, py.Percent(py.MustFloat(reading.GetOr("utilization", 0)), 0))
		notes := WindowNotes(ctx, provider, entry, window, nil)
		noted[window] = true
		if len(notes) > 0 {
			text += fmt.Sprintf(" (%s)", strings.Join(notes, "; "))
		}
		parts = append(parts, text)
	}
	budgets := provider.Limits.Budgets()
	for _, window := range budgets.Keys() {
		if noted[window] {
			continue
		}
		since := ctx.Now().Add(-windowSpan(window))
		spent := providers.MinutesSpent(entry, since)
		parts = append(parts, fmt.Sprintf("%s %.0f/%s min", providers.WindowNames[window], spent,
			py.Str(budgets.Get(window))))
	}
	var observed time.Time
	if isDict {
		observed = clock.ParseISOValue(usage.Get("observed_at"))
	}
	if !observed.IsZero() && len(parts) > 0 {
		parts[len(parts)-1] += fmt.Sprintf(" (read %s ago)", clock.HumanDelta(ctx.Now().Sub(observed)))
	}
	if text := strings.Join(parts, "; "); text != "" {
		return text
	}
	return "no reading yet"
}

// Record is status._record: the state record of a lane: an item's, or the suggestion survey's
// (number 0). A missing record reads as an empty one.
func Record(state *pyjson.Object, number int) *pyjson.Object {
	if number == 0 {
		if survey := state.Obj("suggest"); survey.Len() > 0 {
			return survey
		}
		return pyjson.New()
	}
	// A record that is there but not an object reads as empty too; the Python's `.get` on it
	// raised AttributeError.
	if record, ok := state.Obj("items").Get(strconv.Itoa(number)).(*pyjson.Object); ok {
		return record
	}
	return pyjson.New()
}

// RecordOf is status.record_of (= _record).
var RecordOf = Record

// LanesNow is the lanes the state file says are held (item number, 0 for a survey -> provider),
// and those of them whose run GitHub has not seen end.
func LanesNow(ctx *hctx.Context, state *pyjson.Object) (*pyjson.Object, *pyjson.Object) {
	held := pyjson.New()
	for _, issue := range plan.WorkingThreads(ctx) { // closed ones too: a run goes on until it ends
		record := state.Obj("items").Obj(py.Str(issue.Get("number")))
		held.Set(strconv.Itoa(issue.Int("number")),
			py.Str(py.Or(record.Get("provider"), providers.LegacyProvider)))
	}
	survey := state.Obj("suggest")
	if survey.Truthy("provider") {
		held.Set("0", py.Str(survey.Get("provider")))
	}
	return held, LiveLanes(ctx, state, held)
}

// LiveLanes is `held` without the runs GitHub says have ended (the next plan requeues their items).
// A run GitHub cannot read counts as still going, as it does for the plan (`plan.read_lanes`).
func LiveLanes(ctx *hctx.Context, state *pyjson.Object, held *pyjson.Object) *pyjson.Object {
	live := pyjson.New()
	for _, key := range held.Keys() {
		if plan.RunStatus(ctx, Record(state, laneNumber(key)).Get("run_id")) != "dead" {
			live.Set(key, held.Get(key))
		}
	}
	return live
}

// RunningLines is which subscriptions are running now: on what, for how long, and in which run.
func RunningLines(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object) []string {
	cfg := ctx.Cfg
	lanes := cfg.Pool.MaxParallel

	planningRun := func(number int) bool {
		return number != 0 && Record(state, number).Get("action") == "plan"
	}
	planning := 0
	for _, key := range live.Keys() {
		if planningRun(laneNumber(key)) {
			planning++
		}
	}
	// GitHub's runners and the machine have lanes of their own (`plan.Lanes`).
	hosted := 0
	for _, key := range live.Keys() {
		if !planningRun(laneNumber(key)) && !cfg.Pool.OnMachine(py.Str(live.Get(key))) {
			hosted++
		}
	}
	free := max(0, lanes-hosted)
	planningText := ""
	if cfg.Pool.PlanLanes != 0 {
		planningText = fmt.Sprintf("; %d of %d planning lanes", planning, cfg.Pool.PlanLanes)
	}
	if live.Len() == 0 {
		text := fmt.Sprintf("- Running now: nothing (%d of %d lanes free on GitHub's runners", free, lanes)
		if cfg.Pool.MachineParallel != 0 {
			text += fmt.Sprintf(", %d on the night box", cfg.Pool.MachineParallel)
		}
		if cfg.Pool.PlanLanes != 0 {
			text += fmt.Sprintf(", and %d for planning", cfg.Pool.PlanLanes)
		}
		return []string{text + ")."}
	}
	order := map[string]int{}
	for i, providerID := range cfg.Pool.Priority {
		order[providerID] = i
	}
	rank := func(providerID string) int {
		if i, ok := order[providerID]; ok {
			return i
		}
		return len(order)
	}
	lines := []string{fmt.Sprintf("- **Running now** (%d of %d lanes on GitHub's runners, %d free; "+
		"%s%s):", hosted, lanes, free, MachineText(cfg.Pool, live), planningText)}
	keys := live.Keys()
	sort.SliceStable(keys, func(i, j int) bool {
		ri, rj := rank(py.Str(live.Get(keys[i]))), rank(py.Str(live.Get(keys[j])))
		if ri != rj {
			return ri < rj
		}
		return laneNumber(keys[i]) < laneNumber(keys[j])
	})
	for _, key := range keys {
		number, providerID := laneNumber(key), py.Str(live.Get(key))
		record := Record(state, number)
		var what string
		var since time.Time
		if number != 0 {
			doing := py.Str(record.Get("kind"))
			if record.Get("action") == "plan" {
				doing = "plan"
			}
			does, ok := Doing[doing]
			if !ok {
				does = "working on"
			}
			what = fmt.Sprintf("%s #%d", does, number)
			since = clock.ParseISOValue(record.Get("started_at"))
		} else {
			what = "a suggestion survey"
			since = clock.ParseISOValue(record.Get("last_run"))
		}
		provider := cfg.Pool.Get(providerID)
		name := "`" + providerID + "`"
		if provider != nil {
			name += fmt.Sprintf(" (%s, `%s`)", provider.CLI, provider.Model)
		}
		var detail []string
		if !since.IsZero() {
			elapsed := clock.HumanDelta(ctx.Now().Sub(since))
			if elapsed == "now" {
				detail = append(detail, "just started")
			} else {
				detail = append(detail, "for "+elapsed)
			}
		}
		if runID := record.Get("run_id"); pyjson.Truthy(runID) && !strings.HasPrefix(py.Str(runID), "pending:") {
			detail = append(detail, fmt.Sprintf("[run](%s/%s/actions/runs/%s)", cfg.ServerURL, cfg.Repo, py.Str(runID)))
		}
		text := fmt.Sprintf("  - %s: %s", name, what)
		if len(detail) > 0 {
			text += ", " + strings.Join(detail, ", ")
		}
		lines = append(lines, text+".")
	}
	return lines
}

// ProviderLines is status.provider_lines.
func ProviderLines(ctx *hctx.Context, state *pyjson.Object, held *pyjson.Object) []string {
	cfg := ctx.Cfg
	busy := map[string]int{}
	for _, key := range held.Keys() {
		busy[py.Str(held.Get(key))] = laneNumber(key)
	}
	var lines []string
	for _, provider := range cfg.Pool.Ordered() {
		entry := providers.PeekRecord(state, provider.ID)
		// An idle subscription waits for a build, a revision or a plan, which start only
		// `start_headroom` under each cap (`plan._usable`).
		reason := providers.StartReason(provider, state, ctx.Now(), cfg.Timezone, cfg.Secrets)
		var nowDoing string
		if number, ok := busy[provider.ID]; ok {
			nowDoing = "**running a survey**"
			if number != 0 {
				nowDoing = fmt.Sprintf("**working on #%d**", number)
			}
		} else if reason == "" {
			// Nothing holds it back; whether the queue has work for it is the planner's
			// (the status issue's Queue says which item it takes next, if any).
			nowDoing = "idle, nothing holds it back"
		} else {
			nowDoing = reason
		}
		var seats []string
		for _, seat := range cfg.Pool.Seats(provider) {
			text := fmt.Sprintf("`%s` %s", seat.Model, seat.Tier)
			if seat.SelfCheck {
				text += ", self-check"
			}
			seats = append(seats, text)
		}
		lines = append(lines, fmt.Sprintf("  - `%s` (%s: %s; "+
			"%s): %s. "+
			"Usage: %s.", provider.ID, provider.CLI, strings.Join(seats, ", "),
			provider.Hours(cfg.Timezone), nowDoing, usageText(provider, entry, ctx)))
	}
	return lines
}

// Report is status.report.
func Report(ctx *hctx.Context) string {
	cfg := ctx.Cfg
	state := ctx.Store.Load()
	lines := []string{fmt.Sprintf("**%s status**", config.Title), ""}
	halt := state.Obj("halt")
	if state.Truthy("halted") {
		by, why := "", ""
		if halt.Truthy("by") {
			by = " by @" + py.Str(halt.Get("by"))
		}
		if halt.Truthy("reason") {
			why = ": " + py.Str(halt.Get("reason"))
		}
		lines = append(lines, fmt.Sprintf("- **Halted**%s%s. `%s start` resumes it.", by, why, config.Slash))
	} else {
		lines = append(lines, "- Not halted.")
	}
	if ctx.RepoHalted() {
		lines = append(lines, "- **`.harness/HALT` is on `main`**: nothing runs until that file is deleted.")
	}
	issues := ctx.GH.ListIssues("", "open", 300, "")

	// labelled: prs nil is either, true pull requests only, false issues only.
	labelled := func(name string, prs *bool) []*pyjson.Object {
		var found []*pyjson.Object
		for _, issue := range issues {
			names := map[string]bool{}
			for _, label := range pyjson.Objs(issue.GetOr("labels", []any{})) {
				if labelName, ok := label.Get("name").(string); ok {
					names[labelName] = true
				}
			}
			isPR := issue.Has("pull_request")
			if names[name] && (prs == nil || *prs == isPR) {
				found = append(found, issue)
			}
		}
		return found
	}
	yes, no := true, false

	held, live := LanesNow(ctx, state)
	lines = append(lines, RunningLines(ctx, state, live)...)
	lines = append(lines, fmt.Sprintf("- Subscriptions (at most %d at once on GitHub's runners "+
		"and %d on the machine):", cfg.Pool.MaxParallel, cfg.Pool.MachineParallel))
	lines = append(lines, ProviderLines(ctx, state, live)...)
	var gone []int
	for _, key := range held.Keys() {
		if n := laneNumber(key); n != 0 && !live.Has(key) {
			gone = append(gone, n)
		}
	}
	sort.Ints(gone)
	endedShown := make([]string, len(gone))
	for i, n := range gone {
		endedShown[i] = fmt.Sprintf("#%d", n)
	}
	ended := strings.Join(endedShown, ", ")
	working := fmt.Sprintf("- Working on: %s", numbers(labelled(config.LabelWorking, nil)))
	if ended != "" {
		working += fmt.Sprintf(" (no run is going for %s any more; the next plan requeues it)", ended)
	}
	lines = append(lines, working+".")
	lines = append(lines, fmt.Sprintf("- **Needs plan** (rated and planned first, into the description: an easy or "+
		"unrated item by a medium or strong model, the rest by a strong one): "+
		"%s.", numbers(labelled(config.LabelNeedsPlan, &no))))
	lines = append(lines, fmt.Sprintf("- Queued to build: %s; "+
		"to revise: %s; "+
		"for a review run: %s.", numbers(labelled(config.LabelBuild, &no)),
		numbers(labelled(config.LabelRevise, &yes)), numbers(labelled(config.LabelCross, &yes))))
	lines = append(lines, fmt.Sprintf("- Waiting for a person: %s.", numbers(labelled(config.LabelBlocked, nil))))
	lines = append(lines, fmt.Sprintf("- Handed to people as only a person can do it (`%s`, "+
		"never touched): %s.", config.LabelHumanRequired, numbers(labelled(config.LabelHumanRequired, nil))))
	lines = append(lines, fmt.Sprintf("- People's work (`%s`, never touched): "+
		"%s.", config.LabelHuman, numbers(labelled(config.LabelHuman, nil))))
	lines = append(lines, fmt.Sprintf("- Open bot pull requests: %s.", numbers(labelled(config.LabelPR, &yes))))
	suggestions := labelled(config.LabelSuggestion, &no)
	idle := state.Obj("suggest").Get("idle_since")
	asked := ""
	if state.Obj("suggest").Truthy("requested") {
		asked = " (asked for more)"
	}
	idleText := ""
	if pyjson.Truthy(idle) {
		idleText = fmt.Sprintf(" (idle since %s)", py.Str(idle))
	}
	lines = append(lines, fmt.Sprintf("- Suggestions open: %d of %d"+
		"%s; a survey "+
		"once a free lane has had nothing to start for %d "+
		"minutes", len(suggestions), cfg.SuggestionsMaxOpen, asked, cfg.SuggestionsIdleMinutes)+idleText+"; "+
		fmt.Sprintf("%d that came to nothing are never offered again.", len(declined.Entries(state))))
	last := state.Obj("last_run")
	if last.Truthy("url") {
		lines = append(lines, fmt.Sprintf("- Last run: [%s](%s) at %s.", py.Str(last.GetOr("what", "run")),
			py.Str(last.Get("url")), py.Str(last.GetOr("at", "?"))))
	}
	cycles := cfg.MaxReviewCycles
	autoMerge := "off"
	if cfg.AutoMerge {
		autoMerge = "on"
	}
	lines = append(lines, fmt.Sprintf("- Up to %s build and review rounds "+
		"per item (%s for a hard one), after a "+
		"plan on the planning lane (or by its builder when none is free), which goes "+
		"into the issue's description, by a medium or strong model for an easy, medium "+
		"or unrated item and a strong one for a hard item; the planner rates an "+
		"unrated item. "+
		"An item's `difficulty:easy|medium|hard` (medium until rated) sets the weakest "+
		"tier that may build it, and three failures of its own raise it a step. A "+
		"change merges once "+
		"%s approved it. A self-checking builder "+
		"checks itself up to %d times first. Auto-merge "+
		"%s.", py.Str(cycles.GetOr("medium", 3)), py.Str(cycles.GetOr("hard", 10)), reviewrule.Summary,
		cfg.MaxSelfCheckRounds, autoMerge))
	return strings.Join(lines, "\n")
}
