package usage

import (
	"slices"
	"time"
)

// sameCells is: the two rows write the same cells (times compared as they are written, to the
// second in UTC).
func sameCells(a, b Row) bool { return slices.Equal(Encode(a), Encode(b)) }

// The row types. Each field's `csv` tag is its column, in the header's order; the issue's columns
// (#2749, section 1) come first, as it names them, and the ones this history adds come last.

// Reading is one real usage reading (`usage/readings-YYYY-MM.csv`), one row per subscription and
// `observed_at`.
type Reading struct {
	ObservedAt   time.Time `csv:"observed_at"`
	Subscription string    `csv:"subscription"`
	// Source is ping (a Claude account's Haiku turn before a run), stream (what a call streamed:
	// Claude's rate-limit events, Codex's), usage-command (Muse's and agy's `/usage`, no model
	// call) or refusal (a call the subscription refused); "" for a reading the loop found in the
	// state file that no run handed on.
	Source string `csv:"source"`
	// During is before (the run's start), call (while a call ran, or its end), after (Muse's and
	// agy's reading after a call) or tick (the loop's, from the state file).
	During       string    `csv:"during"`
	RunID        string    `csv:"run_id"`
	Util5h       Num       `csv:"util_5h"`
	Resets5h     time.Time `csv:"resets_5h"`
	Util7d       Num       `csv:"util_7d"`
	Resets7d     time.Time `csv:"resets_7d"`
	Status       string    `csv:"status"`
	Cap5h        Num       `csv:"cap_5h_in_force"`
	Cap7d        Num       `csv:"cap_7d_in_force"`
	RefusedUntil time.Time `csv:"refused_until"` // added: a refusal's end
}

// Key is the subscription and when it was read.
func (x *Reading) Key() string { return x.Subscription + "|" + ISO(x.ObservedAt) }

// When is when it was read.
func (x *Reading) When() time.Time { return x.ObservedAt }

// same is: `x` says what `other` says, caps in force included, whenever and however each was
// read: a run keeps only the first of such readings in a row (`Note`).
func (x *Reading) same(other *Reading) bool {
	a, b := *x, *other
	a.ObservedAt, b.ObservedAt = time.Time{}, time.Time{}
	a.Source, b.Source, a.During, b.During, a.RunID, b.RunID = "", "", "", "", "", ""
	return sameCells(&a, &b)
}

// sameUsage is: `x` reads the same windows as `other` (their utilizations, resets and status).
func (x *Reading) sameUsage(other *Reading) bool {
	return x.Util5h == other.Util5h && ISO(x.Resets5h) == ISO(other.Resets5h) &&
		x.Util7d == other.Util7d && ISO(x.Resets7d) == ISO(other.Resets7d) && x.Status == other.Status
}

// Tick is one subscription at one status tick (`usage/ticks-YYYY-MM.csv`): written when anything
// in it changed since its last row, and at least hourly (`TickEvery`).
type Tick struct {
	At              time.Time `csv:"at"`
	Subscription    string    `csv:"subscription"`
	Enabled         Num       `csv:"enabled"`
	Suspended       Num       `csv:"suspended"`
	InHours         Num       `csv:"in_hours"`
	LastHourNow     Num       `csv:"last_hour_now"`
	LateWeekNow     Num       `csv:"late_week_now"`
	Cap5hNow        Num       `csv:"cap_5h_now"`
	Cap7dNow        Num       `csv:"cap_7d_now"`
	HardCap         Num       `csv:"hard_cap"`
	StartHeadroom5h Num       `csv:"start_headroom_5h"`
	Util5h          Num       `csv:"util_5h"`
	Util7d          Num       `csv:"util_7d"`
	ReadingAt       time.Time `csv:"reading_at"`
	ReadingAgeMin   Num       `csv:"reading_age_min"`
	Resets5h        time.Time `csv:"resets_5h"`
	Resets7d        time.Time `csv:"resets_7d"`
	RefusedUntil    time.Time `csv:"refused_until"`
	LanesAllowed    Num       `csv:"lanes_allowed"`
	LanesHeld       Num       `csv:"lanes_held"`
	RunsHeld        Num       `csv:"runs_held"`
	PlanningHeld    Num       `csv:"planning_held"`
	State           string    `csv:"state"`
	QueuedForIt     Num       `csv:"queued_for_it"`
	WaitingForIt    Num       `csv:"waiting_for_it"`
	// Added: the items held for it (`bot:waiting`, #2748), and its runs by kind.
	HeldForIt  Num `csv:"held_for_it"`
	RunsPlan   Num `csv:"runs_plan"`
	RunsBuild  Num `csv:"runs_build"`
	RunsRevise Num `csv:"runs_revise"`
	RunsReview Num `csv:"runs_review"`
}

// Key is the subscription and the tick.
func (x *Tick) Key() string { return x.Subscription + "|" + ISO(x.At) }

// When is the tick.
func (x *Tick) When() time.Time { return x.At }

// changed is: something in `x` differs from `last`, its time and its reading's age aside.
func (x *Tick) changed(last *Tick) bool {
	if last == nil {
		return true
	}
	a, b := *x, *last
	a.At, b.At, a.ReadingAgeMin, b.ReadingAgeMin = time.Time{}, time.Time{}, Num{}, Num{}
	return !sameCells(&a, &b)
}

// PoolTick is the shared pools at one status tick (`usage/pools-YYYY-MM.csv`, a file of its own so
// the ticks file keeps one row shape): the runner slots and planning lanes in use, and the queue
// by kind and difficulty. Written when anything in it changed, and at least hourly.
type PoolTick struct {
	At              time.Time `csv:"at"`
	GithubSlotsUsed Num       `csv:"github_slots_used"`
	AWSSlotsUsed    Num       `csv:"aws_slots_used"`
	PlanLanesUsed   Num       `csv:"plan_lanes_used"`
	// The queue by kind (what its next run does: plan, build, revise, review) and difficulty
	// (medium, hard, or unrated): one column each.
	PlanMedium    Num `csv:"queued_plan_medium"`
	PlanHard      Num `csv:"queued_plan_hard"`
	PlanUnrated   Num `csv:"queued_plan_unrated"`
	BuildMedium   Num `csv:"queued_build_medium"`
	BuildHard     Num `csv:"queued_build_hard"`
	BuildUnrated  Num `csv:"queued_build_unrated"`
	ReviseMedium  Num `csv:"queued_revise_medium"`
	ReviseHard    Num `csv:"queued_revise_hard"`
	ReviseUnrated Num `csv:"queued_revise_unrated"`
	ReviewMedium  Num `csv:"queued_review_medium"`
	ReviewHard    Num `csv:"queued_review_hard"`
	ReviewUnrated Num `csv:"queued_review_unrated"`
	OldestWaitMin Num `csv:"oldest_wait_min"`
	// Added: the limits each count is held to, the whole queue, and the items held for their
	// owner (`bot:waiting`).
	GithubSlots  Num `csv:"github_slots"`
	AWSSlots     Num `csv:"aws_slots"`
	PlanLanes    Num `csv:"plan_lanes"`
	Queued       Num `csv:"queued"`
	HeldForOwner Num `csv:"held_for_owner"`
}

// Key is the tick.
func (x *PoolTick) Key() string { return ISO(x.At) }

// When is the tick.
func (x *PoolTick) When() time.Time { return x.At }

// changed is: something in `x` differs from `last`, its time and the oldest wait (which grows
// every tick) aside.
func (x *PoolTick) changed(last *PoolTick) bool {
	if last == nil {
		return true
	}
	a, b := *x, *last
	a.At, b.At, a.OldestWaitMin, b.OldestWaitMin = time.Time{}, time.Time{}, Num{}, Num{}
	return !sameCells(&a, &b)
}

// Session is one closed 5-hour session of one subscription (`usage/sessions-YYYY-MM.csv`, by the
// month of its reset), derived from its readings, ticks and runs once its reset has passed
// (`Derive`).
type Session struct {
	Subscription  string    `csv:"subscription"`
	StartedAt     time.Time `csv:"started_at"`
	ResetAt       time.Time `csv:"reset_at"`
	CapMin        Num       `csv:"cap_min"`
	CapMax        Num       `csv:"cap_max"`
	PeakUtil      Num       `csv:"peak_util"`
	UtilLastRead  Num       `csv:"util_last_read"`
	UnusedAtReset Num       `csv:"unused_at_reset"`
	HitCapAt      time.Time `csv:"hit_cap_at"`
	MinutesCapped Num       `csv:"minutes_capped"`
	// MinutesCappedQueued is minutes_capped_with_work_queued.
	MinutesCappedQueued Num `csv:"minutes_capped_with_work_queued"`
	LastHourUsed        Num `csv:"last_hour_used"`
	Runs                Num `csv:"runs"`
	ModelMinutes        Num `csv:"model_minutes"`
	CutOffs             Num `csv:"cut_offs"`
	Refusals            Num `csv:"refusals"`
	OvershootMax        Num `csv:"overshoot_max"`
	// Added: how many readings it was derived from.
	ReadingsCount Num `csv:"readings"`
}

// Key is the subscription and the reset.
func (x *Session) Key() string { return x.Subscription + "|" + ISO(x.ResetAt) }

// When is its reset.
func (x *Session) When() time.Time { return x.ResetAt }

// Week is one closed week of one subscription (`usage/weeks.csv`), derived once its reset has
// passed.
type Week struct {
	Subscription  string    `csv:"subscription"`
	ResetAt       time.Time `csv:"reset_at"`
	PeakUtil      Num       `csv:"peak_util"`
	UtilAtReset   Num       `csv:"util_at_reset"`
	UnusedAtReset Num       `csv:"unused_at_reset"`
	FirstCappedAt time.Time `csv:"first_capped_at"`
	DaysRefused   Num       `csv:"days_refused"`
	Runs          Num       `csv:"runs"`
	ModelHours    Num       `csv:"model_hours"`
	Tokens        Num       `csv:"tokens"`
	USDReported   Num       `csv:"usd_reported"`
	// Added: how many readings it was derived from.
	ReadingsCount Num `csv:"readings"`
}

// Key is the subscription and the reset.
func (x *Week) Key() string { return x.Subscription + "|" + ISO(x.ResetAt) }

// When is its reset.
func (x *Week) When() time.Time { return x.ResetAt }

// Run is one delivered run (`usage/runs-YYYY-MM.csv`), from deliver's own facts: the numbers of
// its journal section.
type Run struct {
	EndedAt      time.Time `csv:"ended_at"`
	RunID        string    `csv:"run_id"`
	Subscription string    `csv:"subscription"`
	// Models is the models its seats ran, joined with `+` (`sonnet+opus`).
	Models     string `csv:"models"`
	Kind       string `csv:"kind"`
	Item       string `csv:"item"`
	Difficulty string `csv:"difficulty"`
	Outcome    string `csv:"outcome"`
	// StopReason is why it stopped short (`StopReason`), "" when it was not cut off.
	StopReason           string `csv:"stop_reason"`
	ModelMinutes         Num    `csv:"model_minutes"`
	TokensIn             Num    `csv:"tokens_in"`
	TokensOut            Num    `csv:"tokens_out"`
	TokensCached         Num    `csv:"tokens_cached"`
	USDReported          Num    `csv:"usd_reported"`
	Util5hStart          Num    `csv:"util_5h_start"`
	Util5hEnd            Num    `csv:"util_5h_end"`
	Util7dStart          Num    `csv:"util_7d_start"`
	Util7dEnd            Num    `csv:"util_7d_end"`
	QueueWaitMin         Num    `csv:"queue_wait_min"`
	PreviousSubscription string `csv:"previous_subscription"`
	// Added for holds and handoffs (#2749 section 14): whether its Claude call resumed the
	// item's kept session (resumed, fresh, none), why the item came to it from another
	// subscription (`HandedOff`), and its model calls.
	Session    string `csv:"session"`
	HandedOff  string `csv:"handed_off"`
	ModelCalls Num    `csv:"model_calls"`
}

// Key is its run id; one with none (a claim made before its run was listed) is its end, its
// subscription and its item.
func (x *Run) Key() string {
	if x.RunID != "" {
		return x.RunID
	}
	return ISO(x.EndedAt) + "|" + x.Subscription + "|" + x.Item
}

// When is its end.
func (x *Run) When() time.Time { return x.EndedAt }
