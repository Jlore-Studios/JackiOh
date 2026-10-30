// Package providers is bot/harness/providers.py.
//
// The subscriptions the bot can spend: `.harness/providers.json`, and whether each is usable now.
//
// A *provider* is one subscription behind one CLI: a Claude account (`claude`), ChatGPT through
// the Codex CLI (`codex`), Google through the Antigravity CLI (`agy`), or Meta through Muse Code
// (`muse`). Each has its own hours (`schedule`), its own limits (`limits`), the runner its model job
// runs on (`runs_on`: GitHub's `ubuntu-latest`, or its own runner on the bot's machine,
// `night-vm-<id>`, which runs as a Linux user of its own; `bot/machine/README.md`), and its login
// (`login`): a `secret` the workflows hand to that model job and to nothing else, or a login made
// once on the machine in that user's home (`machine`), which never leaves it. A run uses exactly one
// provider from start to finish; `plan` chooses it and records it on the item it claims, so a
// provider works on one item at a time and at most `max_parallel` items run at once.
//
// Every model a provider can run is a *seat* with a *tier* (`weak`, `medium` or `strong`): its main
// `model` with its `tier`, and any `extra_models` (every Claude account runs Sonnet, medium, as well
// as Opus, strong, and a run switches between them: `work.Worker._switch`). The tier attaches to the model a role runs on, not to the subscription. The
// top-level `tiers` map lists each tier's models in the order the router tries them, after the
// subscriptions' own order (`priority`, the usage order). A provider with `self_check` (a weak
// one) checks its own builds before any review.
//
// Whether a provider may start a run now is `availability()`; its reading of the state file is
// `state["providers"][<id>]` (usage readings, a refusal's reset time, minutes spent), which
// `deliver` keeps up to date after every run.
//
// In Go, Python's None is the zero value where a type has no nil: "" for a reason or a zone name,
// the zero time.Time for a datetime or a date.
package providers

import (
	"fmt"
	"regexp"
	"slices"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/identity"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// ProvidersPath is PROVIDERS_PATH: the running bot's subscriptions (`identity.py`):
// `.harness/providers.json` for the night bot, `.squishy/providers.json` for Squishy.
var ProvidersPath = purePath(identity.Home_, "providers.json")

// DefaultPark is runner.DEFAULT_PARK, defined here so that providers need not import runner (the
// Python's one import cycle, broken this way: runner uses this one). A refusal that names no
// reset parks its provider this long.
var DefaultPark = "+PT60M"

// Clis is CLIS.
var Clis = []string{"claude", "codex", "agy", "muse"}

// Logins is LOGINS.
var Logins = []string{"secret", "machine"}

// SecretClis is SECRET_CLIS: the CLIs that can be logged in from a secret on a fresh runner
// (`logins.py`). agy keeps its login only on the machine.
var SecretClis = []string{"claude", "codex", "muse"}

// label is _LABEL, `^[A-Za-z0-9._-]+$`. Python's `$` also matches before a newline that ends the
// text, so the Go pattern allows one.
var label = regexp.MustCompile(`\A[A-Za-z0-9._-]+\n?\z`)

// HostedPrefixes is HOSTED_PREFIXES: labels of GitHub's own runners: a fresh virtual machine per
// job, with no login on it.
var HostedPrefixes = []string{"ubuntu-", "windows-", "macos-"}

// Hosted is hosted: whether `label` is one of GitHub's own runners rather than the bot's machine.
func Hosted(label string) bool {
	for _, prefix := range HostedPrefixes {
		if strings.HasPrefix(label, prefix) {
			return true
		}
	}
	return false
}

// Roles is ROLES.
var Roles = []string{"plan", "build", "fix", "revise", "review", "suggest"}

// Tiers is TIERS: the tiers, weakest first. A difficulty needs at least `MIN_TIER[difficulty]` to
// build it.
var Tiers = []string{"weak", "medium", "strong"}

// TierRank is TIER_RANK.
var TierRank = map[string]int{"weak": 0, "medium": 1, "strong": 2}

// SecretNames is SECRETS: the secrets a provider may name. The workflows can only hand a run a
// secret they list, so this is the same list as `bot-night.yml`'s (a test checks), and nothing
// else: never the bot's GitHub token.
var SecretNames = []string{
	"CLAUDE_CODE_OAUTH_TOKEN",
	"CLAUDE_CODE_OAUTH_TOKEN_2",
	"CLAUDE_CODE_OAUTH_TOKEN_3",
	"CLAUDE_CODE_OAUTH_TOKEN_4",
	"CLAUDE_CODE_OAUTH_TOKEN_5",
	"CLAUDE_CODE_OAUTH_TOKEN_6",
	"CLAUDE_CODE_OAUTH_TOKEN_7",
	"CLAUDE_CODE_OAUTH_TOKEN_8",
	"CODEX_AUTH_JSON",
	"MUSE_AUTH",
	// Squishy's own Claude account (#60), in `.squishy/providers.json`.
	"CLAUDE_CODE_OAUTH_TOKEN_SQUISHY",
}

// InfraBackoffs is INFRA_BACKOFFS: after a run on a provider could not work (its login refused,
// its CLI would not install or start), unforced runs leave that provider alone this long. The
// others carry on. Each failure in a row waits longer (the last wait repeats), so a dead login
// stops costing a run every hour; the streak ends when a run on the provider gets a model call
// through (`clear_infra`).
var InfraBackoffs = []time.Duration{50 * time.Minute, 2 * time.Hour, 8 * time.Hour}

// InfraAskAfter is INFRA_ASK_AFTER: after this many failures in a row, `deliver` opens an issue
// asking a person to fix it.
var InfraAskAfter int = 3

// Windows is WINDOWS: usage windows, and how long each lasts when a reading carries no reset time
// of its own. A Go map has no order; windowOrder is the Python dict's.
var Windows = map[string]time.Duration{"five_hour": 5 * time.Hour, "seven_day": 7 * 24 * time.Hour}

// windowOrder is the order of WINDOWS's keys, wherever the Python iterates it.
var windowOrder = []string{"five_hour", "seven_day"}

// WindowNames is WINDOW_NAMES.
var WindowNames = map[string]string{"five_hour": "5-hour", "seven_day": "7-day"}

// LastHour is LAST_HOUR: the last hour of a 5-hour session (`last_hour` in providers.json): what
// is left of a session is lost at its reset, so a subscription its owner left alone may spend it
// past its cap.
var LastHour = time.Hour

// LastHourSlack is LAST_HOUR_SLACK: how far over its 5-hour cap a reading may be and still count
// as the bot's own spending: a call the cap stops can end a point past it (its watcher looks every
// 15 seconds, at whole points). Further over, somebody else spent the session, and its last hour
// stays capped.
var LastHourSlack float64 = 0.01

// LateWeekSpan is LATE_WEEK_SPAN: the longest a late week (`late_week`) may last, from its start
// to the week's reset: a reset that moved further off raises nothing, rather than the week's cap
// for days.
var LateWeekSpan = 24 * time.Hour

// Weekdays is WEEKDAYS: weekday names as `late_week` writes them, Monday first
// (`datetime.weekday`).
var Weekdays = []string{"mon", "tue", "wed", "thu", "fri", "sat", "sun"}

// Schedule is when a provider may start work: `always`, or a daily `window` in the bot's time zone.
type Schedule struct {
	Mode  string
	Start string // default: ""
	End   string // default: ""
}

// Window is the schedule's daily window, or nil when it is `always`.
func (x *Schedule) Window(zoneName string) *clock.Window {
	if x.Mode == "window" {
		return clock.WindowOf(zoneName, x.Start, x.End)
	}
	return nil
}

// IsOpen is is_open.
func (x *Schedule) IsOpen(zoneName string, at time.Time) bool {
	window := x.Window(zoneName)
	return window == nil || window.IsOpen(at)
}

// Describe is describe.
func (x *Schedule) Describe(zoneName string) string {
	window := x.Window(zoneName)
	if window == nil {
		return "any time"
	}
	return window.Describe()
}

// Limits is `caps`: no new run past these fractions of the 5-hour or 7-day allowance, or past these
// minutes of model time in a rolling 5 hours or 7 days (for a CLI that reports no usage).
// `none`: use whatever there is; a refusal parks the provider until the limit resets.
type Limits struct {
	Mode            string
	FiveHour        *float64 // default: None
	SevenDay        *float64 // default: None
	FiveHourMinutes *int     // default: None
	SevenDayMinutes *int     // default: None
	// Headroom is how far under a cap a build, a revision or a plan must be to start
	// (`start_headroom` in providers.json): one that starts at 39% under a 40% cap is cut off
	// almost at once. Window name to a float64 share.
	Headroom *pyjson.Object // default: field(default_factory=dict)
}

// Stops is the property stops: each window's cap (a float64) under `caps`.
func (x *Limits) Stops() *pyjson.Object {
	stops := pyjson.New()
	if x.Mode != "caps" {
		return stops
	}
	if x.FiveHour != nil {
		stops.Set("five_hour", *x.FiveHour)
	}
	if x.SevenDay != nil {
		stops.Set("seven_day", *x.SevenDay)
	}
	return stops
}

// Budgets is the property budgets: each window's minutes (an int) under `caps`.
func (x *Limits) Budgets() *pyjson.Object {
	budgets := pyjson.New()
	if x.Mode != "caps" {
		return budgets
	}
	if x.FiveHourMinutes != nil {
		budgets.Set("five_hour", *x.FiveHourMinutes)
	}
	if x.SevenDayMinutes != nil {
		budgets.Set("seven_day", *x.SevenDayMinutes)
	}
	return budgets
}

// Provider is providers.Provider.
type Provider struct {
	ID      string
	Enabled bool
	CLI     string
	// Family is the model family, for the review rule: one review counts per family.
	Family   string
	Model    string
	Effort   string
	Secret   string
	Schedule *Schedule
	Limits   *Limits
	// Tier is the tier of `model`: `weak`, `medium` or `strong`.
	Tier string
	// QuietCheck: wait until nobody else is spending the subscription before an unforced run
	// (`quiet.py`).
	QuietCheck bool
	Roles      []string
	// Env is extra, non-secret environment for the CLI.
	Env map[string]string // default: field(default_factory=dict)
	// Login is `secret` (handed over by the workflow) or `machine` (logged in on the runner
	// already).
	Login string // default: "secret"
	// RunsOn is the runner label its model job runs on.
	RunsOn string // default: "ubuntu-latest"
	// OffFrom is the first day (in the bot's time zone) it takes no new work, and OffReason why:
	// a free offer that ends, say. On that day the planner also opens one issue asking a person
	// to decide. A date, at midnight UTC; the zero time is None.
	OffFrom   time.Time // default: None
	OffReason string    // default: ""
	// UsageSince is when its plan last changed: a usage reading taken before it is the old
	// plan's, so no cap holds on it (`current_usage`), and the next run reads the new plan's
	// usage afresh. The zero time is None.
	UsageSince time.Time // default: None
	// Lanes is how many items it may work on at once. On the machine each needs a runner of its
	// own with its `runs_on` label (`register-runners.sh` registers that many).
	Lanes int // default: 1
	// FillIn is the lanes it may borrow from other machine subscriptions while they cannot use
	// them (`FillIn`).
	FillIn *FillIn // default: None
	// SelfCheck: its builds check themselves before any review (`work.py`'s self-check loop).
	SelfCheck bool // default: False
	// BuildLast: it builds, fixes and revises only after every other subscription that may
	// (claude-2, kept for planning and review); it plans and reviews in its `priority` place.
	BuildLast bool // default: False
	// EasyFirst: it builds and revises easy items ahead of every other subscription while it has
	// a free lane (a weak one: it may build nothing harder, so the stronger models are kept for
	// what only they can).
	EasyFirst bool // default: False
	// ExtraModels are other models the subscription can run for a role, each with its own tier.
	ExtraModels []*ExtraModel // default: ()
	// OffHours is caps outside its `schedule` window (`{"five_hour": 0.4}`): it may work then
	// too, but only under these, and a run that goes past them there stops. No entry: it works
	// in its hours only. Window name to a float64 share (nil for a null in providers.json, which
	// the Python keeps too and fails on where it is used).
	OffHours *pyjson.Object // default: field(default_factory=dict)
	// FixEffort is the effort a fix pass runs at (#160, #317 part 12): a fix answers named
	// findings, which needs less thought than the build. "" runs it at the seat's own effort.
	FixEffort string // default: ""
	// ReviewEffort is the effort a review runs at (#695): a review checks a finished change,
	// which needs less thought than the build. "" runs it at `high`.
	ReviewEffort string // default: ""
	// LastHour is its last hour (`last_hour: true`): in the hour before its 5-hour session
	// resets, while its reading is no further over its 5-hour cap than the bot's own spending
	// takes it (so nobody else spent the session), that cap is lifted; a run that starts there
	// goes on past it until the reset (`last_hour_of`). Its weekly cap still holds.
	LastHour bool // default: False
	// LateWeek is its late week (`late_week`): from a weekday and time until its week resets,
	// higher caps, and its last hour there alone (claude-5's whole week, 80% of a session and
	// its last hour from Sunday 17:00).
	LateWeek *LateWeek // default: None
}

// Describe is describe.
func (x *Provider) Describe() string {
	return fmt.Sprintf("`%s` (%s, %s)", x.ID, x.CLI, x.Model)
}

// Hours is its hours, and how far it may go outside them.
func (x *Provider) Hours(zoneName string) string {
	text := x.Schedule.Describe(zoneName)
	if x.OffHours.Len() > 0 && x.Schedule.Mode == "window" {
		var caps []string
		x.OffHours.Items(func(window string, share any) bool {
			caps = append(caps, fmt.Sprintf("%s of %s", py.Percent(py.MustFloat(share), 0), WindowNames[window]))
			return true
		})
		text += ", outside them up to " + strings.Join(caps, " and ")
	}
	return text
}

// CapsAt is the usage caps that hold at `at`: its own, and outside its window the tighter of
// those and its `off_hours` ones. With its reading (`usage`), its late week raises them
// (`late_week_holds`) and its last hour lifts the 5-hour one (`last_hour_of`).
// `last_hour` is the reset of a session whose last hour a run already took
// (`work.Worker`): the 5-hour cap stays lifted until then, however far past it the run
// has gone.
//
// `usage` is nil for None, else the reading (a *pyjson.Object, {} included); `lastHour` is the
// zero time for None.
func (x *Provider) CapsAt(at time.Time, zoneName string, usage any /* = None */, lastHour time.Time /* = None */) *pyjson.Object {
	caps := x.HeldCaps(at, zoneName, usage)
	if !isNone(usage) && caps.Has("five_hour") &&
		((!lastHour.IsZero() && at.Before(lastHour)) || !LastHourOf(x, usage, at, zoneName).IsZero()) {
		caps.Set("five_hour", 1.0)
	}
	return caps
}

// HeldCaps is `caps_at` without its last hour: the caps a reading is held to before that lifts
// the 5-hour one.
func (x *Provider) HeldCaps(at time.Time, zoneName string, usage any /* = None */) *pyjson.Object {
	caps := x.Limits.Stops()
	if x.OffHours.Len() > 0 && zoneName != "" && !x.Schedule.IsOpen(zoneName, at) {
		x.OffHours.Items(func(window string, share any) bool {
			caps.Set(window, pyMin(py.MustFloat(caps.GetOr(window, 1.0)), py.MustFloat(share)))
			return true
		})
	}
	if !isNone(usage) && LateWeekHolds(x, usage, at, zoneName) {
		x.LateWeek.Caps().Items(func(window string, limit any) bool {
			if caps.Has(window) {
				caps.Set(window, pyMax(caps.Float(window), limit.(float64)))
			}
			return true
		})
	}
	return caps
}

// TakesLastHours is the property takes_last_hours: it may take a session's last hour: always
// (`last_hour`), or in its late week.
func (x *Provider) TakesLastHours() bool {
	return x.LastHour || (x.LateWeek != nil && x.LateWeek.LastHour)
}

// LateWeek is `late_week` in providers.json, `{"from": "Sun 17:00", "seven_day": 1.0, "five_hour": 0.8,
// "last_hour": true}`: from that weekday and time in the bot's time zone until the week resets,
// the weekly cap is `seven_day`, the 5-hour one `five_hour` (if given), and with `last_hour` a
// session's last hour lifts that (`last_hour_of`) there and nowhere else. It holds only while
// the reset is at most `LATE_WEEK_SPAN` after the start.
type LateWeek struct {
	Weekday  int
	Start    clock.TimeOfDay
	SevenDay float64
	FiveHour *float64 // default: None
	LastHour bool     // default: False
}

// Caps is the property caps: each window's cap (a float64).
func (x *LateWeek) Caps() *pyjson.Object {
	caps := pyjson.New()
	if x.FiveHour != nil {
		caps.Set("five_hour", *x.FiveHour)
	}
	caps.Set("seven_day", x.SevenDay)
	return caps
}

// Since is the latest start at or before `at`.
func (x *LateWeek) Since(at time.Time, zoneName string) time.Time {
	zone := clock.Zone(zoneName)
	local := at.In(zone)
	year, month, day := local.Date()
	day -= py.Mod(weekday(local)-x.Weekday, 7)
	start := localTime(year, month, day, x.Start.Hour, x.Start.Minute, zone)
	if start.After(at) {
		start = localTime(year, month, day-7, x.Start.Hour, x.Start.Minute, zone)
	}
	return start.UTC()
}

// Describe is describe: `Sun 17:00`.
func (x *LateWeek) Describe() string {
	name := Weekdays[x.Weekday]
	return fmt.Sprintf("%s%s %02d:%02d", strings.ToUpper(name[:1]), name[1:], x.Start.Hour, x.Start.Minute)
}

// FillIn is the lanes of other subscriptions on the machine that this one fills in (`fill_in` in
// providers.json): Muse runs in gpt's and agy's lanes on the night box while one of them cannot
// start work (capped, off, outside its hours, backing off), on runners of its own there labelled
// `runs_on`, one per lane it may fill. Those subscriptions and its runs there share their lanes,
// never more at once than theirs: a subscription whose lane Muse fills waits for it
// (`plan.fill_in_full`).
type FillIn struct {
	RunsOn string
	// Of is the subscriptions whose lanes it fills.
	Of []string
}

// ExtraModel is providers.ExtraModel.
type ExtraModel struct {
	Model  string
	Effort string
	Tier   string
	// TakesOver is the subscription this model stands in for (#317 part 9): it builds only easy
	// items, and only while that subscription cannot take them (off, backed off, or its lanes
	// full). It plans and reviews nothing. Empty: an ordinary seat.
	TakesOver string // default: ""
}

// TierEntry is one model in a tier's preference order (`tiers` in providers.json).
type TierEntry struct {
	Model string
}

// Seat is one model one subscription can run, and its tier: what a role is given to run on.
type Seat struct {
	Provider  *Provider
	Model     string
	Effort    string
	Tier      string
	SelfCheck bool
	// Rank is its place in its tier's preference order: the router tries the lowest first.
	Rank int
	// TakesOver is the subscription it stands in for (`ExtraModel.takes_over`), or "" for an
	// ordinary seat.
	TakesOver string // default: ""
}

// Family is the property family.
func (x *Seat) Family() string { return x.Provider.Family }

// Describe is describe.
func (x *Seat) Describe() string {
	return fmt.Sprintf("`%s` (%s, `%s`, %s)", x.Provider.ID, x.Provider.CLI, x.Model, x.Tier)
}

// ToDict is to_dict.
func (x *Seat) ToDict() *pyjson.Object {
	return pyjson.New("provider", x.Provider.ID, "model", x.Model, "effort", x.Effort,
		"tier", x.Tier, "self_check", x.SelfCheck)
}

// TierAtLeast is tier_at_least.
func TierAtLeast(tier string, floor string) bool {
	return tierRank(tier) >= tierRank(floor)
}

// tierRank is TIER_RANK[tier], which raises KeyError for a tier that is none of the three.
func tierRank(tier string) int {
	rank, ok := TierRank[tier]
	if !ok {
		panic(fmt.Errorf("KeyError: %s", py.Repr(tier)))
	}
	return rank
}

// Pool is providers.Pool.
type Pool struct {
	// MaxParallel is how many runs may go at once on GitHub's runners (the Claude accounts'),
	// planning runs included (#1455): the organization runs 20 hosted jobs at once, so this is
	// the share the bots' model jobs may hold while CI keeps the rest. Both bots name the same
	// pool and count each other's runs against it, first come first served
	// (`plan.others_on_github`).
	MaxParallel int
	Priority    []string
	// Providers is each provider by id, in providers.json's order: the values are *Provider.
	Providers *pyjson.Object
	// MachineParallel is how many runs may go at once on the bot's machine (`runs_on` not
	// GitHub's), apart from `max_parallel`, so neither waits on the other: its vCPUs run every
	// machine job's checks, while each of GitHub's runners has its own four.
	MachineParallel int // default: 0
	// PlanLanes is how many planning runs may go at once (the planning lane), on top of each
	// subscription's own `lanes`: a strong model plans the Needs plan stage there while its
	// builds go on. Each still takes a runner, a machine slot or one of `max_parallel` (#1455).
	PlanLanes int // default: 0
	// Tiers is each tier's models, in the order the router tries them: the values are
	// []*TierEntry.
	Tiers *pyjson.Object // default: field(default_factory=dict)
	// Balance is the load balancer (`plan.balance`): within a tier, the subscription with the
	// most room under its caps goes first, then the one holding the fewest runs, and only then
	// the usage order. Off, the usage order alone breaks the tie.
	Balance bool // default: False
}

// provider is self.providers.get(provider_id).
func (x *Pool) provider(providerID string) *Provider {
	p, _ := x.Providers.Get(providerID).(*Provider)
	return p
}

// providerList is self.providers.values().
func (x *Pool) providerList() []*Provider {
	var out []*Provider
	x.Providers.Items(func(_ string, v any) bool {
		out = append(out, v.(*Provider))
		return true
	})
	return out
}

// TierEntries is self.tiers.get(tier, ()).
func (x *Pool) TierEntries(tier string) []*TierEntry {
	entries, _ := x.Tiers.Get(tier).([]*TierEntry)
	return entries
}

// OnMachine is on_machine.
func (x *Pool) OnMachine(providerID string) bool {
	provider := x.Get(providerID)
	return provider != nil && !Hosted(provider.RunsOn)
}

// FillerOf is the subscription that fills `provider_id`'s lanes while it cannot use them, if any.
func (x *Pool) FillerOf(providerID string) *Provider {
	for _, p := range x.providerList() {
		if p.FillIn != nil && slices.Contains(x.FilledBy(p), providerID) {
			return p
		}
	}
	return nil
}

// FilledBy is the subscriptions whose lanes `filler` fills (`FillIn`): those of its `for` on the
// machine. One moved to GitHub's runners has no lane on a box to lend.
func (x *Pool) FilledBy(filler *Provider) []string {
	var of []string
	if filler.FillIn != nil {
		of = filler.FillIn.Of
	}
	found := []string{}
	for _, i := range of {
		if x.OnMachine(i) {
			found = append(found, i)
		}
	}
	return found
}

// Ordered is every provider, in `priority` order.
func (x *Pool) Ordered() []*Provider {
	out := make([]*Provider, 0, len(x.Priority))
	for _, name := range x.Priority {
		p := x.provider(name)
		if p == nil {
			panic(fmt.Errorf("KeyError: %s", py.Repr(name)))
		}
		out = append(out, p)
	}
	return out
}

// Get is get: the provider, or nil. "" stands for None.
func (x *Pool) Get(providerID string) *Provider {
	return x.provider(providerID)
}

func (x *Pool) entry(tier string, model string) (int, *TierEntry) {
	entries := x.TierEntries(tier)
	for rank, entry := range entries {
		if entry.Model == model {
			return rank, entry
		}
	}
	return len(entries), nil
}

// Seats is the models `provider` can run, its main one first.
func (x *Pool) Seats(provider *Provider) []*Seat {
	found := []*Seat{}
	add := func(model, effort, tier string, ownCheck bool, standsIn string) {
		rank, _ := x.entry(tier, model)
		found = append(found, &Seat{provider, model, effort, tier, ownCheck, rank, standsIn})
	}
	add(provider.Model, provider.Effort, provider.Tier, provider.SelfCheck, "")
	for _, m := range provider.ExtraModels {
		add(m.Model, m.Effort, m.Tier, false, m.TakesOver)
	}
	return found
}

// OwnSeats is `provider`'s seats that plan, review and build anything: every seat but a stand-in
// (`takes_over`), which builds only the easy items another subscription cannot take.
func (x *Pool) OwnSeats(provider *Provider) []*Seat {
	found := []*Seat{}
	for _, seat := range x.Seats(provider) {
		if seat.TakesOver == "" {
			found = append(found, seat)
		}
	}
	return found
}

// Seat is `provider_id`'s seat for `model`, or its main one; None when it has no such model.
// "" stands for None in both.
func (x *Pool) Seat(providerID string, model string /* = None */) *Seat {
	provider := x.Get(providerID)
	if provider == nil {
		return nil
	}
	for _, found := range x.Seats(provider) {
		if model == "" || model == found.Model {
			return found
		}
	}
	return nil
}

// BestSeat is `provider`'s strongest seat at `floor` or above, or None.
func (x *Pool) BestSeat(provider *Provider, floor string /* = "weak" */) *Seat {
	var best *Seat
	for _, s := range x.OwnSeats(provider) {
		if !TierAtLeast(s.Tier, floor) {
			continue
		}
		// max() keeps the first of equal keys: (TIER_RANK[s.tier], -s.rank).
		if best == nil || tierRank(s.Tier) > tierRank(best.Tier) ||
			(tierRank(s.Tier) == tierRank(best.Tier) && -s.Rank > -best.Rank) {
			best = s
		}
	}
	return best
}

// FamilyTier is the strongest tier any seat of `family` has: how a vote with no tier recorded
// counts.
func (x *Pool) FamilyTier(family string) string {
	best := ""
	for _, p := range x.Ordered() {
		if p.Family != family {
			continue
		}
		for _, s := range x.Seats(p) {
			if best == "" || tierRank(s.Tier) > tierRank(best) {
				best = s.Tier
			}
		}
	}
	if best == "" {
		return "weak"
	}
	return best
}
