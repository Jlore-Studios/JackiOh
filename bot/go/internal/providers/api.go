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
package providers

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"time"
)

// PROVIDERS_PATH = Path(HOME) / "providers.json"
var ProvidersPath any // unported

// CLIS = ("claude", "codex", "agy", "muse")
var Clis = []string{"claude", "codex", "agy", "muse"}

// LOGINS = ("secret", "machine")
var Logins = []string{"secret", "machine"}

// SECRET_CLIS = ("claude", "codex", "muse")
var SecretClis = []string{"claude", "codex", "muse"}

// HOSTED_PREFIXES = ("ubuntu-", "windows-", "macos-")
var HostedPrefixes = []string{"ubuntu-", "windows-", "macos-"}

// Whether `label` is one of GitHub's own runners rather than the bot's machine.

func Hosted(label string) bool {
	panic("unported: providers.hosted")
}

// ROLES = ("plan", "build", "fix", "revise", "review", "suggest")
var Roles = []string{"plan", "build", "fix", "revise", "review", "suggest"}

// TIERS = ("weak", "medium", "strong")
var Tiers = []string{"weak", "medium", "strong"}

// TIER_RANK = {name: rank for rank, name in enumerate(TIERS)}
var TierRank any // unported

// SECRETS: tuple[str, ...] = ( "CLAUDE_CODE_OAUTH_TOKEN", "CLAUDE_CODE_OAUTH_TOKEN_2", "CLAUDE_CODE_OAUTH_TOKEN_3", "CLAUDE_CODE_OAUTH_TOKEN_4", "CLAUDE_CODE_OAUT
var SecretNames = []string{"CLAUDE_CODE_OAUTH_TOKEN", "CLAUDE_CODE_OAUTH_TOKEN_2", "CLAUDE_CODE_OAUTH_TOKEN_3", "CLAUDE_CODE_OAUTH_TOKEN_4", "CLAUDE_CODE_OAUTH_TOKEN_5", "CLAUDE_CODE_OAUTH_TOKEN_6", "CLAUDE_CODE_OAUTH_TOKEN_7", "CLAUDE_CODE_OAUTH_TOKEN_8", "CODEX_AUTH_JSON", "MUSE_AUTH", "CLAUDE_CODE_OAUTH_TOKEN_SQUISHY"}

// INFRA_BACKOFFS = (timedelta(minutes=50), timedelta(hours=2), timedelta(hours=8))
var InfraBackoffs any // unported

// INFRA_ASK_AFTER = 3
var InfraAskAfter int = 3

// WINDOWS = {"five_hour": timedelta(hours=5), "seven_day": timedelta(days=7)}
var Windows any // unported

// WINDOW_NAMES = {"five_hour": "5-hour", "seven_day": "7-day"}
var WindowNames = map[string]string{"five_hour": "5-hour", "seven_day": "7-day"}

// LAST_HOUR = timedelta(hours=1)
var LastHour any // unported

// LAST_HOUR_SLACK = 0.01
var LastHourSlack float64 = 0.01

// LATE_WEEK_SPAN = timedelta(days=1)
var LateWeekSpan any // unported

// WEEKDAYS = ("mon", "tue", "wed", "thu", "fri", "sat", "sun")
var Weekdays = []string{"mon", "tue", "wed", "thu", "fri", "sat", "sun"}

// When a provider may start work: `always`, or a daily `window` in the bot's time zone.

type Schedule struct {
	Mode  string
	Start string // default: ""
	End   string // default: ""
}

func (x *Schedule) Window(zoneName string) *clock.Window {
	panic("unported: providers.Schedule.window")
}

func (x *Schedule) IsOpen(zoneName string, at time.Time) bool {
	panic("unported: providers.Schedule.is_open")
}

func (x *Schedule) Describe(zoneName string) string {
	panic("unported: providers.Schedule.describe")
}

// `caps`: no new run past these fractions of the 5-hour or 7-day allowance, or past these
// minutes of model time in a rolling 5 hours or 7 days (for a CLI that reports no usage).
// `none`: use whatever there is; a refusal parks the provider until the limit resets.

type Limits struct {
	Mode            string
	FiveHour        *float64       // default: None
	SevenDay        *float64       // default: None
	FiveHourMinutes *int           // default: None
	SevenDayMinutes *int           // default: None
	Headroom        *pyjson.Object // default: field(default_factory=dict)
}

func (x *Limits) Stops() *pyjson.Object {
	panic("unported: providers.Limits.stops")
}

func (x *Limits) Budgets() *pyjson.Object {
	panic("unported: providers.Limits.budgets")
}

// Provider is providers.Provider.

type Provider struct {
	ID           string
	Enabled      bool
	CLI          string
	Family       string
	Model        string
	Effort       string
	Secret       string
	Schedule     *Schedule
	Limits       *Limits
	Tier         string
	QuietCheck   bool
	Roles        []string
	Env          map[string]string // default: field(default_factory=dict)
	Login        string            // default: "secret"
	RunsOn       string            // default: "ubuntu-latest"
	OffFrom      time.Time         // default: None
	OffReason    string            // default: ""
	UsageSince   time.Time         // default: None
	Lanes        int               // default: 1
	FillIn       *FillIn           // default: None
	SelfCheck    bool              // default: False
	BuildLast    bool              // default: False
	EasyFirst    bool              // default: False
	ExtraModels  []*ExtraModel     // default: ()
	OffHours     *pyjson.Object    // default: field(default_factory=dict)
	FixEffort    string            // default: ""
	ReviewEffort string            // default: ""
	LastHour     bool              // default: False
	LateWeek     *LateWeek         // default: None
}

func (x *Provider) Describe() string {
	panic("unported: providers.Provider.describe")
}

// Its hours, and how far it may go outside them.

func (x *Provider) Hours(zoneName string) string {
	panic("unported: providers.Provider.hours")
}

// The usage caps that hold at `at`: its own, and outside its window the tighter of
// those and its `off_hours` ones. With its reading (`usage`), its late week raises them
// (`late_week_holds`) and its last hour lifts the 5-hour one (`last_hour_of`).
// `last_hour` is the reset of a session whose last hour a run already took
// (`work.Worker`): the 5-hour cap stays lifted until then, however far past it the run
// has gone.

func (x *Provider) CapsAt(at time.Time, zoneName string, usage any /* = None */, lastHour time.Time /* = None */) *pyjson.Object {
	panic("unported: providers.Provider.caps_at")
}

// `caps_at` without its last hour: the caps a reading is held to before that lifts the
// 5-hour one.

func (x *Provider) HeldCaps(at time.Time, zoneName string, usage any /* = None */) *pyjson.Object {
	panic("unported: providers.Provider.held_caps")
}

// It may take a session's last hour: always (`last_hour`), or in its late week.

func (x *Provider) TakesLastHours() bool {
	panic("unported: providers.Provider.takes_last_hours")
}

// `late_week` in providers.json, `{"from": "Sun 17:00", "seven_day": 1.0, "five_hour": 0.8,
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

func (x *LateWeek) Caps() *pyjson.Object {
	panic("unported: providers.LateWeek.caps")
}

// The latest start at or before `at`.

func (x *LateWeek) Since(at time.Time, zoneName string) time.Time {
	panic("unported: providers.LateWeek.since")
}

func (x *LateWeek) Describe() string {
	panic("unported: providers.LateWeek.describe")
}

// The lanes of other subscriptions on the machine that this one fills in (`fill_in` in
// providers.json): Muse runs in gpt's and agy's lanes on the night box while one of them cannot
// start work (capped, off, outside its hours, backing off), on runners of its own there labelled
// `runs_on`, one per lane it may fill. Those subscriptions and its runs there share their lanes,
// never more at once than theirs: a subscription whose lane Muse fills waits for it
// (`plan.fill_in_full`).

type FillIn struct {
	RunsOn string
	Of     []string
}

// ExtraModel is providers.ExtraModel.

type ExtraModel struct {
	Model     string
	Effort    string
	Tier      string
	TakesOver string // default: ""
}

// One model in a tier's preference order (`tiers` in providers.json).

type TierEntry struct {
	Model string
}

// One model one subscription can run, and its tier: what a role is given to run on.

type Seat struct {
	Provider  *Provider
	Model     string
	Effort    string
	Tier      string
	SelfCheck bool
	Rank      int
	TakesOver string // default: ""
}

func (x *Seat) Family() string {
	panic("unported: providers.Seat.family")
}

func (x *Seat) Describe() string {
	panic("unported: providers.Seat.describe")
}

func (x *Seat) ToDict() *pyjson.Object {
	panic("unported: providers.Seat.to_dict")
}

// tier_at_least is providers.tier_at_least.

func TierAtLeast(tier string, floor string) bool {
	panic("unported: providers.tier_at_least")
}

// Pool is providers.Pool.

type Pool struct {
	MaxParallel     int
	Priority        []string
	Providers       *pyjson.Object
	MachineParallel int            // default: 0
	PlanLanes       int            // default: 0
	Tiers           *pyjson.Object // default: field(default_factory=dict)
	Balance         bool           // default: False
}

func (x *Pool) OnMachine(providerID string) bool {
	panic("unported: providers.Pool.on_machine")
}

// The subscription that fills `provider_id`'s lanes while it cannot use them, if any.

func (x *Pool) FillerOf(providerID string) *Provider {
	panic("unported: providers.Pool.filler_of")
}

// The subscriptions whose lanes `filler` fills (`FillIn`): those of its `for` on the
// machine. One moved to GitHub's runners has no lane on a box to lend.

func (x *Pool) FilledBy(filler *Provider) []string {
	panic("unported: providers.Pool.filled_by")
}

// Every provider, in `priority` order.

func (x *Pool) Ordered() []*Provider {
	panic("unported: providers.Pool.ordered")
}

func (x *Pool) Get(providerID string) *Provider {
	panic("unported: providers.Pool.get")
}

func (x *Pool) entry(tier string, model string) (int, *TierEntry) {
	panic("unported: providers.Pool._entry")
}

// The models `provider` can run, its main one first.

func (x *Pool) Seats(provider *Provider) []*Seat {
	panic("unported: providers.Pool.seats")
}

// `provider`'s seats that plan, review and build anything: every seat but a stand-in
// (`takes_over`), which builds only the easy items another subscription cannot take.

func (x *Pool) OwnSeats(provider *Provider) []*Seat {
	panic("unported: providers.Pool.own_seats")
}

// `provider_id`'s seat for `model`, or its main one; None when it has no such model.

func (x *Pool) Seat(providerID string, model string /* = None */) *Seat {
	panic("unported: providers.Pool.seat")
}

// `provider`'s strongest seat at `floor` or above, or None.

func (x *Pool) BestSeat(provider *Provider, floor string /* = "weak" */) *Seat {
	panic("unported: providers.Pool.best_seat")
}

// The strongest tier any seat of `family` has: how a vote with no tier recorded counts.

func (x *Pool) FamilyTier(family string) string {
	panic("unported: providers.Pool.family_tier")
}

// A `Pool` from the JSON of `.harness/providers.json`. Raises `ConfigError`.

func Parse(raw any) *Pool {
	panic("unported: providers.parse")
}

// load is providers.load.

func Load(root string) *Pool {
	panic("unported: providers.load")
}

// LEGACY_PROVIDER = "claude-1"
var LegacyProvider string = "claude-1"

// SPENT_KEEP = 200
var SpentKeep int = 200

// The state file's record for one provider, created empty when missing.

func Record(state *pyjson.Object, providerID string) *pyjson.Object {
	panic("unported: providers.record")
}

// `record` without creating anything.

func PeekRecord(state *pyjson.Object, providerID string) *pyjson.Object {
	panic("unported: providers.peek_record")
}

// The `/harness suspend` holding this provider (`by`, `at`, `reason`), or None. Only
// `/harness resume <id>` lifts it: neither `--force` nor a bare `/harness start` does.

func Suspension(state *pyjson.Object, providerID string) *pyjson.Object {
	panic("unported: providers.suspension")
}

// Keep a run's newest usage reading, any refusal, the minutes it spent, and the tokens
// and USD cost its calls reported (#695).

func NoteUsage(state *pyjson.Object, providerID string, usage *pyjson.Object, resetAt string, at time.Time, minutes float64 /* = 0.0 */, tokens *pyjson.Object /* = None */, costUSD *float64 /* = None */) {
	panic("unported: providers.note_usage")
}

// NEAR_FULL = 0.85
var NearFull float64 = 0.85

// When the latest-resetting window that the reading had nearly full resets, if any.

func NearFullReset(usage any, at time.Time) time.Time {
	panic("unported: providers.near_full_reset")
}

// A run on this provider could not work: leave it alone for its backoff (`infra_backoff`).
// Returns the failures in a row. `escalate` False (a failure that was not the provider's, such
// as an install that fails on untouched main) keeps the streak where it was.

func NoteInfra(state *pyjson.Object, providerID string, reason string, at time.Time, escalate bool /* = True */) int {
	panic("unported: providers.note_infra")
}

// A run on this provider got a model call through: its failure streak is over. Returns the
// issue `deliver` opened about it, if any, to close.

func ClearInfra(state *pyjson.Object, providerID string) *int {
	panic("unported: providers.clear_infra")
}

// How long a provider is left alone after its last failure, by the failures in a row.

func InfraBackoff(infra *pyjson.Object) time.Duration {
	panic("unported: providers.infra_backoff")
}

// `+PT90M`, `+PT2H`, `+PT1H30M`: how long a relative refusal lasts (30 minutes if unread).

func Duration(text string) time.Duration {
	panic("unported: providers._duration")
}

// Why this provider's usage says to start nothing now, or None. With the bot's time zone,
// outside its window its `off_hours` caps hold too, and its late week's and its last hour's
// (`Provider.caps_at`, which takes `last_hour`). `starting` (a build, a revision or a plan
// about to start) leaves `start_headroom` under each cap; a run going on stops at the cap
// itself.

func Refusal(provider *Provider, entry *pyjson.Object, at time.Time, zoneName string /* = None */, starting bool /* = False */, lastHour time.Time /* = None */) string {
	panic("unported: providers.refusal")
}

// BALANCE_STEP = 0.1
var BalanceStep float64 = 0.1

// How much of its allowance this provider has free now, from 0 to 1: its tightest window,
// each the share of its cap (1 where it has none) its last reading leaves free, and of its
// minute budgets. As in `refusal`, a window that has reset since the reading is all free, and
// a refusal not yet over leaves nothing.

func Room(provider *Provider, entry *pyjson.Object, at time.Time, zoneName string /* = None */) float64 {
	panic("unported: providers.room")
}

// The usage a build, a revision or a plan must be under to start, under `cap`.

func StartUnder(provider *Provider, window string, cap_ float64) float64 {
	panic("unported: providers.start_under")
}

// When its 5-hour session resets, if `usage` finds `provider` in that session's last hour
// (`last_hour`, or its late week's): the reset at most `LAST_HOUR` away, and the reading no
// further over the 5-hour cap that holds now than `LAST_HOUR_SLACK`, so nobody but the bot
// spent the session (a cap of 50% read at 50%). None otherwise: then the cap holds as ever.

func LastHourOf(provider *Provider, usage any, at time.Time, zoneName string) time.Time {
	panic("unported: providers.last_hour_of")
}

// Whether `provider`'s late week (`late_week`) raises its caps at `at`: its start has
// passed, and the week its reading names resets after `at` and at most `LATE_WEEK_SPAN` after
// that start.

func LateWeekHolds(provider *Provider, usage any, at time.Time, zoneName string) bool {
	panic("unported: providers.late_week_holds")
}

// When a subscription whose usage stops a build or a plan starting now
// (`refusal(..., starting=True)`) may start one again, with nothing more spent: the first of
// its refusal's end, its readings' resets, its session's last hour, its late week's start and
// its window's opening that clears it. None when none of those does (a minute budget, say).

func FreesAt(provider *Provider, entry *pyjson.Object, at time.Time, zoneName string) time.Time {
	panic("unported: providers.frees_at")
}

// minutes_spent is providers.minutes_spent.

func MinutesSpent(entry *pyjson.Object, since time.Time) float64 {
	panic("unported: providers.minutes_spent")
}

// Which provider secrets the workflow has. `known` is False outside a workflow (an operator
// running the harness by hand), where nothing can be told and nothing is ruled out.

type Secrets struct {
	Present map[string]bool
	Known   bool
}

func SecretsOf(env map[string]string) *Secrets {
	panic("unported: providers.Secrets.of")
}

func (x *Secrets) Has(name string) *bool {
	panic("unported: providers.Secrets.has")
}

// SECRETS_NOTE_EVERY = timedelta(minutes=30)
var SecretsNoteEvery any // unported

// The `state["secrets"]` record a plan job writes: which secrets its workflow has, and
// when. None outside a workflow, or when the record there says the same and is recent.

func SecretsRecord(secrets *Secrets, state *pyjson.Object, now time.Time) *pyjson.Object {
	panic("unported: providers.secrets_record")
}

// Which secrets to go by: the newest plan job's record (`secrets_record`) when it is newer
// than `since`, when this process's own list was fixed (or `since` is unknown); else its own.
// GitHub fixes a run's secrets when the run is created, so a long run, or one queued for hours,
// would otherwise miss a secret added since.

func NewerSecrets(secrets *Secrets, state *pyjson.Object, since time.Time) *Secrets {
	panic("unported: providers.newer_secrets")
}

// `entry`'s last usage reading, or {} when there is none or it was taken before the
// subscription's plan changed (`usage_since`), so the old plan's usage holds nothing back.

func CurrentUsage(provider *Provider, entry *pyjson.Object) *pyjson.Object {
	panic("unported: providers.current_usage")
}

// Whether `provider`'s `off_from` day has come, in the bot's time zone.

func SwitchedOffByDate(provider *Provider, at time.Time, zoneName string) bool {
	panic("unported: providers.switched_off_by_date")
}

// Why `provider` may not start a run now, or None when it may. Busy-ness is the caller's.

func Availability(provider *Provider, state *pyjson.Object, at time.Time, zoneName string, secrets *Secrets, forced bool /* = False */, starting bool /* = False */) string {
	panic("unported: providers.availability")
}

// When `provider`'s usage lets it start a build or a plan again, if `reason` (its
// `availability(..., starting=True)`) is its usage and nothing else: not its hours, its login
// or a suspension, which no reset lifts.

func UsageFreesAt(provider *Provider, state *pyjson.Object, at time.Time, zoneName string, reason string) time.Time {
	panic("unported: providers.usage_frees_at")
}

// Why `provider` may not start a build, a revision or a plan now, or None when it may: what
// an idle subscription waits for, and what `plan` holds it to (`start_headroom` under each
// cap). One its usage holds says whether it still takes reviews, which go up to the cap, and
// when it may start one again.

func StartReason(provider *Provider, state *pyjson.Object, at time.Time, zoneName string, secrets *Secrets) string {
	panic("unported: providers.start_reason")
}

// When a queued item can expect a run: now, or when the soonest subscription opens or has
// room under its caps again.

func WhenFree(pool *Pool, state *pyjson.Object, at time.Time, zoneName string, secrets *Secrets) string {
	panic("unported: providers.when_free")
}
