package providers

import (
	"fmt"
	"regexp"
	"slices"
	"sort"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/identity"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// ---------------------------------------------------------------------- the state of each one

// LegacyProvider is LEGACY_PROVIDER: the first Claude account inherits the readings the state
// file kept before there were several subscriptions (`usage`, `rate_limited_until` at the top
// level).
var LegacyProvider string = "claude-1"

// SpentKeep is SPENT_KEEP: how many past runs' minutes a provider keeps, for `*_minutes` budgets.
var SpentKeep int = 200

// Record is the state file's record for one provider, created empty when missing.
func Record(state *pyjson.Object, providerID string) *pyjson.Object {
	providers := pyjson.Obj(state.SetDefault("providers", pyjson.New()))
	entry := pyjson.Obj(providers.SetDefault(providerID, pyjson.New()))
	if providerID == LegacyProvider && !entry.Has("usage") && state.Truthy("usage") {
		entry.Set("usage", state.Get("usage"))
	}
	if providerID == LegacyProvider && !entry.Has("refused_until") && state.Truthy("rate_limited_until") {
		entry.Set("refused_until", state.Get("rate_limited_until"))
	}
	return entry
}

// PeekRecord is `record` without creating anything.
func PeekRecord(state *pyjson.Object, providerID string) *pyjson.Object {
	entry := state.Obj("providers").Obj(providerID).Copy()
	if providerID == LegacyProvider {
		entry.SetDefault("usage", state.Get("usage"))
		entry.SetDefault("refused_until", state.Get("rate_limited_until"))
	}
	return entry
}

// Suspension is the `/harness suspend` holding this provider (`by`, `at`, `reason`), or None. Only
// `/harness resume <id>` lifts it: neither `--force` nor a bare `/harness start` does.
func Suspension(state *pyjson.Object, providerID string) *pyjson.Object {
	if held := PeekRecord(state, providerID).Obj("suspended"); held != nil {
		return held.Copy()
	}
	return nil
}

// NoteUsage keeps a run's newest usage reading, any refusal, the minutes it spent, and the tokens
// and USD cost its calls reported (#695). "" stands for None in `resetAt`.
func NoteUsage(state *pyjson.Object, providerID string, usage *pyjson.Object, resetAt string, at time.Time, minutes float64 /* = 0.0 */, tokens *pyjson.Object /* = None */, costUSD *float64 /* = None */) {
	entry := Record(state, providerID)
	if usage.Len() > 0 {
		reading := usage.Copy()
		reading.Set("observed_at", clock.ISO(at))
		entry.Set("usage", reading)
	}
	if resetAt != "" {
		resets := clock.ParseISO(resetAt)
		if resets.IsZero() && strings.HasPrefix(resetAt, "+PT") {
			resets = at.Add(Duration(resetAt))
			if resetAt == DefaultPark {
				// The refusal named no reset: a window the last reading had nearly full is the
				// likely cause, so wait for it (gpt's week at 89% was retried every hour).
				held := NearFullReset(entry.Get("usage"), at)
				if !held.IsZero() && held.After(resets) {
					resets = held
				}
			}
		}
		if !resets.IsZero() {
			entry.Set("refused_until", clock.ISO(resets))
		}
	}
	if minutes > 0 {
		spent := listOf(entry.Get("spent"))
		spent = append(spent, pyjson.New("at", clock.ISO(at), "minutes", py.RoundN(minutes, 1)))
		entry.Set("spent", lastN(spent, SpentKeep))
	}
	if tokens.Len() > 0 && (tokens.Truthy("input") || tokens.Truthy("output")) {
		// A run's token counts and cost beside its minutes, for the statistics issue (#695).
		counted := listOf(entry.Get("tokens_spent"))
		cost := 0.0
		if costUSD != nil {
			cost = *costUSD
		}
		counted = append(counted, pyjson.New(
			"at", clock.ISO(at),
			"input", py.MustInt(py.Or(tokens.Get("input"), 0)),
			"output", py.MustInt(py.Or(tokens.Get("output"), 0)),
			"cache", py.MustInt(py.Or(tokens.Get("cache_read"), 0))+py.MustInt(py.Or(tokens.Get("cache_write"), 0)),
			"cost", py.RoundN(cost, 4)))
		entry.Set("tokens_spent", lastN(counted, SpentKeep))
	}
}

// NearFull is NEAR_FULL: a window at least this full in the last reading is taken to be why a
// refusal came.
var NearFull float64 = 0.85

// NearFullReset is when the latest-resetting window that the reading had nearly full resets, if
// any (the zero time when none).
func NearFullReset(usage any, at time.Time) time.Time {
	obj, ok := mapping(usage)
	if !ok {
		return time.Time{}
	}
	var found time.Time
	for _, window := range windowOrder {
		reading, ok := mapping(obj.Get(window))
		if !ok {
			continue
		}
		resets := clock.ParseISOValue(reading.Get("resets_at"))
		utilization, isNumber := py.Number(reading.Get("utilization"))
		if !resets.IsZero() && resets.After(at) && isNumber && utilization >= NearFull {
			if found.IsZero() || resets.After(found) {
				found = resets
			}
		}
	}
	return found
}

// NoteInfra is: a run on this provider could not work: leave it alone for its backoff
// (`infra_backoff`). Returns the failures in a row. `escalate` False (a failure that was not the
// provider's, such as an install that fails on untouched main) keeps the streak where it was.
func NoteInfra(state *pyjson.Object, providerID string, reason string, at time.Time, escalate bool /* = True */) int {
	entry := Record(state, providerID)
	previous, ok := mapping(entry.Get("infra"))
	if !ok {
		previous = pyjson.New()
	}
	streak := streakOf(previous)
	if escalate || previous.Len() == 0 {
		if previous.Len() > 0 {
			streak++
		} else {
			streak = 1
		}
	}
	infra := pyjson.New()
	if previous.Truthy("issue") {
		infra.Set("issue", previous.Get("issue"))
	}
	infra.Set("at", clock.ISO(at))
	infra.Set("reason", py.SliceTo(reason, 500))
	infra.Set("streak", streak)
	entry.Set("infra", infra)
	return streak
}

// ClearInfra is: a run on this provider got a model call through: its failure streak is over.
// Returns the issue `deliver` opened about it, if any, to close.
func ClearInfra(state *pyjson.Object, providerID string) *int {
	entry := state.Obj("providers").Obj(providerID)
	if entry == nil {
		return nil
	}
	infra, ok := mapping(entry.Get("infra"))
	if !ok {
		return nil
	}
	entry.Delete("infra")
	issue := infra.Get("issue")
	if !pyjson.Truthy(issue) {
		return nil
	}
	n := py.MustInt(issue)
	return &n
}

// streakOf is _streak.
func streakOf(infra *pyjson.Object) int {
	if infra.Len() == 0 {
		return 0
	}
	n, err := py.Int(py.Or(infra.Get("streak"), 1))
	if err != nil {
		return 1
	}
	if n < 1 {
		return 1
	}
	return n
}

// InfraBackoff is how long a provider is left alone after its last failure, by the failures in a
// row.
func InfraBackoff(infra *pyjson.Object) time.Duration {
	return InfraBackoffs[min(max(streakOf(infra), 1), len(InfraBackoffs))-1]
}

// durationPattern is _duration's `\+?PT(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?`, with Python's Unicode
// `\d`.
var durationPattern = regexp.MustCompile(`\A\+?PT(?:(\p{Nd}+)H)?(?:(\p{Nd}+)M)?(?:(\p{Nd}+)S)?\z`)

// Duration is _duration: `+PT90M`, `+PT2H`, `+PT1H30M`: how long a relative refusal lasts (30
// minutes if unread).
func Duration(text string) time.Duration {
	match := durationPattern.FindStringSubmatch(strings.ToUpper(strings.TrimSpace(text)))
	if match == nil || (match[1] == "" && match[2] == "" && match[3] == "") {
		return 30 * time.Minute
	}
	hours, minutes, seconds := py.Atoi(match[1]), py.Atoi(match[2]), py.Atoi(match[3])
	return time.Duration(hours)*time.Hour + time.Duration(minutes)*time.Minute +
		time.Duration(seconds)*time.Second
}

// Refusal is why this provider's usage says to start nothing now, or None (""). With the bot's
// time zone, outside its window its `off_hours` caps hold too, and its late week's and its last
// hour's (`Provider.caps_at`, which takes `last_hour`). `starting` (a build, a revision or a plan
// about to start) leaves `start_headroom` under each cap; a run going on stops at the cap
// itself, or `HardCapMargin` under it where that is its hard cap (`hard_cap`).
func Refusal(provider *Provider, entry *pyjson.Object, at time.Time, zoneName string /* = None */, starting bool /* = False */, lastHour time.Time /* = None */) string {
	until := clock.ParseISOValue(entry.Get("refused_until"))
	if !until.IsZero() && until.After(at) {
		return fmt.Sprintf("it refused a call; its limit resets at %s", clock.ISO(until))
	}
	usage := CurrentUsage(provider, entry)
	observed := clock.ParseISOValue(usage.Get("observed_at"))
	caps := provider.CapsAt(at, zoneName, usage, lastHour)
	outside := !pyjson.Equal(provider.CapsAt(at, zoneName, nil, time.Time{}), provider.Limits.Stops())
	for _, window := range caps.Keys() {
		limit := caps.Float(window)
		stop := limit
		hard := provider.HardCap != nil && limit >= *provider.HardCap
		if hard {
			stop = limit - HardCapMargin // so the call it stops ends at the cap at most
		}
		if starting {
			stop = pyMin(stop, StartUnder(provider, window, limit))
		}
		reading, ok := mapping(usage.Get(window))
		if !ok {
			continue
		}
		resets := clock.ParseISOValue(reading.Get("resets_at"))
		if resets.IsZero() && !observed.IsZero() {
			resets = observed.Add(Windows[window])
		}
		if resets.IsZero() || !resets.After(at) {
			continue // that window has reset since the reading, or cannot be dated
		}
		utilization, isNumber := py.Number(reading.Get("utilization"))
		if isNumber && utilization >= stop {
			where := ""
			if outside {
				where = " outside its hours"
			}
			if hard && !starting && utilization < limit {
				return fmt.Sprintf("%s usage is %s, within %s of its %s hard cap%s, where a run stops "+
					"so that no call passes it; it resets at %s", WindowNames[window],
					py.Percent(utilization, 0), py.Percent(HardCapMargin, 0), py.Percent(limit, 0), where,
					clock.ISO(resets))
			}
			if starting && stop < limit {
				return fmt.Sprintf("%s usage is %s, too close to its %s cap%s to start a long run "+
					"(it starts under %s); it resets at %s", WindowNames[window], py.Percent(utilization, 0),
					py.Percent(limit, 0), where, py.Percent(stop, 0), clock.ISO(resets))
			}
			return fmt.Sprintf("%s usage is %s, at or over its %s cap%s; it resets at %s",
				WindowNames[window], py.Percent(utilization, 0), py.Percent(limit, 0), where, clock.ISO(resets))
		}
	}
	budgets := provider.Limits.Budgets()
	for _, window := range budgets.Keys() {
		budget := budgets.Int(window)
		used := MinutesSpent(entry, at.Add(-Windows[window]))
		if used >= float64(budget) {
			return fmt.Sprintf("it has spent %.0f of its %d minutes in the %s window", used, budget,
				WindowNames[window])
		}
	}
	return ""
}

// BalanceStep is BALANCE_STEP: the load balancer's step (`room`, `plan.balance`): two
// subscriptions whose room rounds to the same step count as level, and the one holding fewer
// runs, then the usage order, goes first. A reading is only as fresh as its subscription's last
// run, so a closer split means nothing.
var BalanceStep float64 = 0.1

// Room is how much of its allowance this provider has free now, from 0 to 1: its tightest window,
// each the share of its cap (1 where it has none) its last reading leaves free, and of its
// minute budgets. As in `refusal`, a window that has reset since the reading is all free, and
// a refusal not yet over leaves nothing.
func Room(provider *Provider, entry *pyjson.Object, at time.Time, zoneName string /* = None */) float64 {
	until := clock.ParseISOValue(entry.Get("refused_until"))
	if !until.IsZero() && until.After(at) {
		return 0.0
	}
	usage := CurrentUsage(provider, entry)
	observed := clock.ParseISOValue(usage.Get("observed_at"))
	caps := provider.CapsAt(at, zoneName, usage, time.Time{})
	free := []float64{1.0}
	for _, window := range windowOrder {
		reading, ok := mapping(usage.Get(window))
		if !ok {
			continue
		}
		resets := clock.ParseISOValue(reading.Get("resets_at"))
		if resets.IsZero() && !observed.IsZero() {
			resets = observed.Add(Windows[window])
		}
		utilization, isNumber := py.Number(reading.Get("utilization"))
		if resets.IsZero() || !resets.After(at) || !isNumber {
			continue
		}
		limit := py.MustFloat(caps.GetOr(window, 1.0))
		share := 0.0
		if limit > 0 {
			share = pyMax(0.0, limit-utilization) / limit
		}
		free = append(free, share)
	}
	budgets := provider.Limits.Budgets()
	for _, window := range budgets.Keys() {
		budget := budgets.Int(window)
		used := MinutesSpent(entry, at.Add(-Windows[window]))
		share := 0.0
		if budget > 0 {
			share = pyMax(0.0, 1-used/float64(budget))
		}
		free = append(free, share)
	}
	least := free[0]
	for _, f := range free[1:] {
		least = pyMin(least, f)
	}
	return least
}

// StartUnder is the usage a build, a revision or a plan must be under to start, under `cap`.
func StartUnder(provider *Provider, window string, cap_ float64) float64 {
	return pyMax(0.0, cap_-py.MustFloat(provider.Limits.Headroom.GetOr(window, 0.0)))
}

// LastHourOf is when its 5-hour session resets, if `usage` finds `provider` in that session's last hour
// (`last_hour`, or its late week's): the reset at most `LAST_HOUR` away, and the reading no
// further over the 5-hour cap that holds now than `LAST_HOUR_SLACK`, so nobody but the bot
// spent the session (a cap of 50% read at 50%). None (the zero time) otherwise: then the cap
// holds as ever.
func LastHourOf(provider *Provider, usage any, at time.Time, zoneName string) time.Time {
	obj, ok := mapping(usage)
	if !provider.TakesLastHours() || !ok {
		return time.Time{}
	}
	if !provider.LastHour && !LateWeekHolds(provider, usage, at, zoneName) {
		return time.Time{} // its last hours are its late week's alone
	}
	held, hasCap := provider.HeldCaps(at, zoneName, usage).Lookup("five_hour")
	reading, ok := mapping(obj.Get("five_hour"))
	if !hasCap || held == nil || py.MustFloat(held) >= provider.ceiling() || !ok { // nothing to lift
		return time.Time{}
	}
	limit := py.MustFloat(held)
	resets := clock.ParseISOValue(reading.Get("resets_at"))
	used, isNumber := py.Number(reading.Get("utilization"))
	if resets.IsZero() || !(at.Before(resets) && !resets.After(at.Add(LastHour))) ||
		!isNumber || used > limit+LastHourSlack {
		return time.Time{}
	}
	return resets
}

// LateWeekHolds is whether `provider`'s late week (`late_week`) raises its caps at `at`: its start has
// passed, and the week its reading names resets after `at` and at most `LATE_WEEK_SPAN` after
// that start.
func LateWeekHolds(provider *Provider, usage any, at time.Time, zoneName string) bool {
	late := provider.LateWeek
	var reading *pyjson.Object
	readingOK := false
	if obj, ok := mapping(usage); ok {
		reading, readingOK = mapping(obj.Get("seven_day"))
	}
	if late == nil || zoneName == "" || !readingOK {
		return false
	}
	resets := clock.ParseISOValue(reading.Get("resets_at"))
	return !resets.IsZero() && resets.After(at) && resets.Sub(late.Since(at, zoneName)) <= LateWeekSpan
}

// FreesAt is when a subscription whose usage stops a build or a plan starting now
// (`refusal(..., starting=True)`) may start one again, with nothing more spent: the first of
// its refusal's end, its readings' resets, its session's last hour, its late week's start and
// its window's opening that clears it. None (the zero time) when none of those does (a minute
// budget, say).
func FreesAt(provider *Provider, entry *pyjson.Object, at time.Time, zoneName string) time.Time {
	var times []time.Time
	add := func(t time.Time) {
		for _, seen := range times {
			if seen.Equal(t) {
				return
			}
		}
		times = append(times, t)
	}
	until := clock.ParseISOValue(entry.Get("refused_until"))
	if !until.IsZero() && until.After(at) {
		add(until)
	}
	usage := CurrentUsage(provider, entry)
	observed := clock.ParseISOValue(usage.Get("observed_at"))
	for _, window := range windowOrder {
		reading, ok := mapping(usage.Get(window))
		if !ok {
			continue
		}
		resets := clock.ParseISOValue(reading.Get("resets_at"))
		if resets.IsZero() && !observed.IsZero() {
			resets = observed.Add(Windows[window])
		}
		if !resets.IsZero() && resets.After(at) {
			add(resets)
			if window == "five_hour" && provider.TakesLastHours() && resets.Add(-LastHour).After(at) {
				add(resets.Add(-LastHour)) // its last hour lifts the cap
			}
		}
	}
	if provider.LateWeek != nil {
		add(provider.LateWeek.Since(at.Add(Windows["seven_day"]), zoneName))
	}
	window := provider.Schedule.Window(zoneName)
	if window != nil && provider.OffHours.Len() > 0 && !window.IsOpen(at) {
		add(window.NextOpen(at)) // its in-hours caps are the looser ones
	}
	sort.SliceStable(times, func(i, j int) bool { return times[i].Before(times[j]) })
	for _, when := range times {
		if Refusal(provider, entry, when, zoneName, true, time.Time{}) == "" {
			return when
		}
	}
	return time.Time{}
}

// MinutesSpent is minutes_spent.
func MinutesSpent(entry *pyjson.Object, since time.Time) float64 {
	total := 0.0
	var spent any = []any{}
	if entry.Truthy("spent") {
		spent = entry.Get("spent")
	}
	for _, item := range py.MustIter(spent) {
		var at time.Time
		run, ok := mapping(item)
		if ok {
			at = clock.ParseISOValue(run.Get("at"))
		}
		if !at.IsZero() && !at.Before(since) {
			total += py.MustFloat(py.Or(run.Get("minutes"), 0))
		}
	}
	return total
}

// Secrets is which provider secrets the workflow has. `known` is False outside a workflow (an
// operator running the harness by hand), where nothing can be told and nothing is ruled out.
type Secrets struct {
	Present map[string]bool
	Known   bool
}

// SecretsOf is Secrets.of.
func SecretsOf(env map[string]string) *Secrets {
	if set, ok := env["HARNESS_SECRETS_SET"]; ok {
		present := map[string]bool{}
		for _, name := range strings.Fields(set) {
			present[name] = true
		}
		return &Secrets{Present: present, Known: true}
	}
	present := map[string]bool{}
	for _, k := range SecretNames {
		if env[k] != "" {
			present[k] = true
		}
	}
	return &Secrets{Present: present, Known: false}
}

// Has is has: true or false, or nil (None) when nothing can be told.
func (x *Secrets) Has(name string) *bool {
	yes, no := true, false
	if x.Present[name] {
		return &yes
	}
	if x.Known {
		return &no
	}
	return nil
}

// SecretsNoteEvery is SECRETS_NOTE_EVERY: how often a plan job rewrites its record of which
// secrets its workflow has when the list has not changed (`secrets_record`).
var SecretsNoteEvery = 30 * time.Minute

// SecretsRecord is the `state["secrets"]` record a plan job writes: which secrets its workflow has, and
// when. None outside a workflow, or when the record there says the same and is recent.
func SecretsRecord(secrets *Secrets, state *pyjson.Object, now time.Time) *pyjson.Object {
	if !secrets.Known {
		return nil
	}
	names := sortedNames(secrets.Present)
	record := state.Obj("secrets")
	at := clock.ParseISOValue(record.Get("at"))
	if pyjson.Equal(record.Get("set"), pyjson.ListOf(names)) && !at.IsZero() && now.Sub(at) < SecretsNoteEvery {
		return nil
	}
	return pyjson.New("set", pyjson.ListOf(names), "at", clock.ISO(now))
}

// NewerSecrets is which secrets to go by: the newest plan job's record (`secrets_record`) when it is newer
// than `since`, when this process's own list was fixed (or `since` is unknown); else its own.
// GitHub fixes a run's secrets when the run is created, so a long run, or one queued for hours,
// would otherwise miss a secret added since. The zero `since` is None.
func NewerSecrets(secrets *Secrets, state *pyjson.Object, since time.Time) *Secrets {
	record := state.Obj("secrets")
	at := clock.ParseISOValue(record.Get("at"))
	names, isList := record.Get("set").([]any)
	if at.IsZero() || !isList || (!since.IsZero() && !at.After(since)) {
		return secrets
	}
	present := map[string]bool{}
	for _, name := range names {
		present[py.Str(name)] = true
	}
	return &Secrets{Present: present, Known: true}
}

// offHours is _off_hours: each share is a float64, or nil for a null (which the Python keeps).
func offHours(raw any, where string) *pyjson.Object {
	if noneOrEmpty(raw) {
		return pyjson.New()
	}
	obj, ok := mapping(raw)
	if !ok {
		configError(`%s: expected an object such as {"five_hour": 0.4}`, where)
	}
	if unknown := unknownKeys(obj, windowOrder); len(unknown) > 0 {
		configError("%s: unknown windows %s", where, strings.Join(unknown, ", "))
	}
	shares := pyjson.New()
	obj.Items(func(window string, share any) bool {
		if found := fraction(share, where+"."+window); found != nil {
			shares.Set(window, *found)
		} else {
			shares.Set(window, nil)
		}
		return true
	})
	return shares
}

// lanes is _lanes.
func lanes(raw any, where string) int {
	n, err := py.Int(raw)
	if err != nil {
		configError("%s: %s is not a number", where, py.Repr(raw))
	}
	if n < 1 {
		configError("%s: must be at least 1", where)
	}
	return n
}

// offFrom is _off_from: a date at midnight UTC, or the zero time for None.
func offFrom(raw any, where string) time.Time {
	if raw == nil || raw == "" {
		return time.Time{}
	}
	day, ok := dateFromISO(py.Str(raw))
	if !ok {
		configError("%s: %s is not a date (YYYY-MM-DD)", where, py.Repr(raw))
	}
	return day
}

// usageSince is _usage_since: the zero time for None.
func usageSince(raw any, where string) time.Time {
	if raw == nil || raw == "" {
		return time.Time{}
	}
	when := clock.ParseISO(py.Str(raw))
	if when.IsZero() {
		configError("%s: %s is not a time (YYYY-MM-DDTHH:MM:SSZ)", where, py.Repr(raw))
	}
	return when
}

// CurrentUsage is `entry`'s last usage reading, or {} when there is none or it was taken before the
// subscription's plan changed (`usage_since`), so the old plan's usage holds nothing back. It is
// never nil.
func CurrentUsage(provider *Provider, entry *pyjson.Object) *pyjson.Object {
	usage, ok := mapping(entry.Get("usage"))
	if !ok {
		return pyjson.New()
	}
	observed := clock.ParseISOValue(usage.Get("observed_at"))
	if !provider.UsageSince.IsZero() && (observed.IsZero() || observed.Before(provider.UsageSince)) {
		return pyjson.New()
	}
	return usage
}

// SwitchedOffByDate is whether `provider`'s `off_from` day has come, in the bot's time zone.
func SwitchedOffByDate(provider *Provider, at time.Time, zoneName string) bool {
	if provider.OffFrom.IsZero() {
		return false
	}
	year, month, day := at.In(clock.Zone(zoneName)).Date()
	return !time.Date(year, month, day, 0, 0, 0, 0, time.UTC).Before(provider.OffFrom)
}

// Availability is why `provider` may not start a run now, or None ("") when it may. Busy-ness is
// the caller's.
func Availability(provider *Provider, state *pyjson.Object, at time.Time, zoneName string, secrets *Secrets, forced bool /* = False */, starting bool /* = False */) string {
	if !provider.Enabled {
		return "switched off in providers.json"
	}
	if SwitchedOffByDate(provider, at, zoneName) {
		return fmt.Sprintf("switched off from %s (`off_from` in providers.json)", provider.OffFrom.Format("2006-01-02"))
	}
	if held := Suspension(state, provider.ID); held != nil {
		by, why := "", ""
		if held.Truthy("by") {
			by = " by @" + py.Str(held.Get("by"))
		}
		if held.Truthy("reason") {
			why = " (" + py.Str(held.Get("reason")) + ")"
		}
		return fmt.Sprintf("suspended%s%s; `%s resume %s` lifts it", by, why, identity.Current.Slash(), provider.ID)
	}
	if has := secrets.Has(provider.Secret); provider.Login == "secret" && has != nil && !*has {
		return fmt.Sprintf("its secret `%s` is not set", provider.Secret)
	}
	if !forced && provider.OffHours.Len() == 0 && !provider.Schedule.IsOpen(zoneName, at) {
		window := provider.Schedule.Window(zoneName)
		opens := at
		if window != nil {
			opens = window.NextOpen(at)
		}
		return fmt.Sprintf("outside its hours (%s; opens in %s)", provider.Schedule.Describe(zoneName),
			clock.HumanDelta(opens.Sub(at)))
	}
	entry := PeekRecord(state, provider.ID)
	infra, ok := mapping(entry.Get("infra"))
	if !ok {
		infra = pyjson.New()
	}
	failed := clock.ParseISOValue(infra.Get("at"))
	backoff := InfraBackoff(infra)
	if !forced && !failed.IsZero() && at.Sub(failed) < backoff {
		streak := streakOf(infra)
		times := ""
		if streak > 1 {
			times = fmt.Sprintf(", %d times in a row", streak)
		}
		return fmt.Sprintf("its last run could not work%s (%s); it is left alone until %s", times,
			py.SliceTo(py.OrStr(infra.Get("reason"), ""), 120), clock.ISO(failed.Add(backoff)))
	}
	return Refusal(provider, entry, at, zoneName, starting, time.Time{})
}

// UsageFreesAt is when `provider`'s usage lets it start a build or a plan again, if `reason` (its
// `availability(..., starting=True)`) is its usage and nothing else: not its hours, its login
// or a suspension, which no reset lifts. "" stands for None in `reason`, the zero time in the
// result.
func UsageFreesAt(provider *Provider, state *pyjson.Object, at time.Time, zoneName string, reason string) time.Time {
	entry := PeekRecord(state, provider.ID)
	if reason == "" || reason != Refusal(provider, entry, at, zoneName, true, time.Time{}) {
		return time.Time{}
	}
	return FreesAt(provider, entry, at, zoneName)
}

// StartReason is why `provider` may not start a build, a revision or a plan now, or None ("")
// when it may: what an idle subscription waits for, and what `plan` holds it to
// (`start_headroom` under each cap). One its usage holds says whether it still takes reviews,
// which go up to the cap, and when it may start one again.
func StartReason(provider *Provider, state *pyjson.Object, at time.Time, zoneName string, secrets *Secrets) string {
	reason := Availability(provider, state, at, zoneName, secrets, false, true)
	entry := PeekRecord(state, provider.ID)
	if reason == "" || reason != Refusal(provider, entry, at, zoneName, true, time.Time{}) {
		return reason // free, or held by something no reset lifts
	}
	if slices.Contains(provider.Roles, "review") && Availability(provider, state, at, zoneName, secrets, false, false) == "" {
		reason += "; it takes reviews only"
	}
	frees := FreesAt(provider, entry, at, zoneName)
	if !frees.IsZero() && !frees.Equal(clock.ParseISOValue(entry.Get("refused_until"))) { // that one says so
		reason += "; it can start a build or a plan again in " + clock.HumanDelta(frees.Sub(at))
	}
	return reason
}

// WhenFree is when a queued item can expect a run: now, or when the soonest subscription opens or
// has room under its caps again.
func WhenFree(pool *Pool, state *pyjson.Object, at time.Time, zoneName string, secrets *Secrets) string {
	var nowFree []string
	for _, p := range pool.Ordered() {
		if StartReason(p, state, at, zoneName, secrets) == "" {
			nowFree = append(nowFree, "`"+p.ID+"`")
		}
	}
	if len(nowFree) > 0 {
		return fmt.Sprintf("in the next run (%s can take it now)", strings.Join(nowFree, ", "))
	}
	var soonest time.Time
	soonestText := ""
	found := false
	for _, provider := range pool.Ordered() {
		window := provider.Schedule.Window(zoneName)
		var opens time.Time
		var text string
		if window != nil && Availability(provider, state, at, zoneName, secrets, true, true) == "" {
			opens = window.NextOpen(at)
			text = fmt.Sprintf("when `%s` opens (%s, in %s)", provider.ID,
				provider.Schedule.Describe(zoneName), clock.HumanDelta(opens.Sub(at)))
		} else {
			reason := Availability(provider, state, at, zoneName, secrets, false, true)
			frees := UsageFreesAt(provider, state, at, zoneName, reason)
			if frees.IsZero() {
				continue
			}
			opens = frees
			text = fmt.Sprintf("when `%s` has room under its caps again (in %s)", provider.ID,
				clock.HumanDelta(frees.Sub(at)))
		}
		if !found || opens.Before(soonest) {
			soonest, soonestText, found = opens, text, true
		}
	}
	if found {
		return soonestText
	}
	return fmt.Sprintf("when a subscription is free again (`%s status` says why none is now)", identity.Current.Slash())
}
