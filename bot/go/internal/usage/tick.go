package usage

import (
	"slices"
	"strconv"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/waiting"
)

// TickEvery is the longest a subscription's tick rows (and the pool's) go without one: a row is
// written whenever anything in it changed, and at least this often. A tick row stands for its
// state until the next one, so a reader takes each row to hold until the next, or `TickCovers`.
var TickEvery = time.Hour

// TickCovers is the longest one tick row stands for: a gap longer than `TickEvery` and a few
// ticks means the loop was not running, and nothing is known of it.
var TickCovers = TickEvery + 15*time.Minute

// States is a tick's `state`, from what the status issue says of the subscription now
// (`waiting.Subscriptions`, `waiting.HeldBy`, in that order of precedence):
//
//   - working: a run of it is going (`busy`), whatever else holds it;
//   - ready: the planner gives it an item at the next run (`next`);
//   - idle: it may start work, and nothing queued is for it (`idle`);
//   - capped: its usage holds it: over a cap, within `start_headroom` of one for a long run (it
//     may still take reviews, `reviews`), a refusal not yet over, or a minute budget spent
//     (`capped`);
//   - outside-hours: outside its hours, with no `off_hours` caps (`closed`);
//   - backing-off: its last run could not work, and it is left alone a while (`broken`);
//   - off: switched off in providers.json, off from its `off_from` day, suspended, its secret not
//     set, or the bot halted (`off`);
//   - no-runner: it may start, but no runner slot is free where it runs (`full`).
var States = map[string]string{"busy": "working", "next": "ready", "idle": "idle", "capped": "capped",
	"reviews": "capped", "closed": "outside-hours", "broken": "backing-off", "off": "off", "full": "no-runner"}

// TickFacts is what one status tick knows, all of it read already for the status issue
// (`dashboard.Look`): the state file, the lanes held now (item number written out -> subscription)
// and the queue explained.
type TickFacts struct {
	State     *pyjson.Object
	Live      *pyjson.Object
	Explained *waiting.Explained
}

// TickRows is one row per subscription, in the usage order, and the pool's row, for a tick at
// `ctx.Now()`. It reads nothing from GitHub.
func TickRows(ctx *hctx.Context, facts TickFacts) ([]*Tick, *PoolTick) {
	cfg, now := ctx.Cfg, utc(ctx.Now())
	zone := cfg.Timezone
	state, live := facts.State, facts.Live
	if live == nil {
		live = pyjson.New()
	}
	lanes := waiting.LanesOf(ctx, state, live)
	var rows []*waiting.Row
	nowOf := pyjson.New()
	if facts.Explained != nil {
		rows, nowOf = facts.Explained.Rows, facts.Explained.Now
	}
	var ticks []*Tick
	for _, provider := range cfg.Pool.Ordered() {
		entry := providers.PeekRecord(state, provider.ID)
		reading := providers.CurrentUsage(provider, entry)
		tick := &Tick{At: now, Subscription: provider.ID,
			Enabled:         Flag(provider.Enabled && !providers.SwitchedOffByDate(provider, now, zone)),
			Suspended:       Flag(providers.Suspension(state, provider.ID) != nil),
			InHours:         Flag(provider.Schedule.IsOpen(zone, now)),
			LastHourNow:     Flag(!providers.LastHourOf(provider, reading, now, zone).IsZero()),
			LateWeekNow:     Flag(providers.LateWeekHolds(provider, reading, now, zone)),
			StartHeadroom5h: capOf(provider.Limits.Headroom, "five_hour"),
		}
		caps := provider.CapsAt(now, zone, reading, time.Time{})
		tick.Cap5hNow, tick.Cap7dNow = capOf(caps, "five_hour"), capOf(caps, "seven_day")
		if provider.HardCap != nil {
			tick.HardCap = Pct(*provider.HardCap)
		}
		if reading.Len() > 0 {
			row := FromUsage(reading, clock.ParseISOValue(reading.Get("observed_at")), "", "", "", "", nil)
			tick.Util5h, tick.Resets5h, tick.Util7d, tick.Resets7d = row.Util5h, row.Resets5h, row.Util7d, row.Resets7d
			tick.ReadingAt = row.ObservedAt
			if !row.ObservedAt.IsZero() {
				tick.ReadingAgeMin = N(round1(now.Sub(row.ObservedAt).Minutes()))
			}
		}
		if until := utc(clock.ParseISOValue(entry.Get("refused_until"))); until.After(now) {
			tick.RefusedUntil = until
		}
		tick.LanesAllowed = N(float64(lanes.LanesOf(provider)))
		tick.LanesHeld = N(float64(lanes.Used(provider)))
		tick.RunsHeld = N(float64(lanes.AllBy(provider.ID)))
		tick.PlanningHeld = N(float64(lanes.PlanningBy(provider.ID)))
		byKind := map[string]int{}
		for number, held := range lanes.Held {
			if held == provider.ID && number != 0 {
				byKind[py.OrStr(state.Obj("items").Obj(strconv.Itoa(number)).Get("action"), "")]++
			}
		}
		tick.RunsPlan, tick.RunsBuild = N(float64(byKind["plan"])), N(float64(byKind["build"]))
		tick.RunsRevise, tick.RunsReview = N(float64(byKind["revise"])), N(float64(byKind["review"]))
		if held, ok := nowOf.Get(provider.ID).(*waiting.Now); ok && held != nil {
			tick.State = States[held.Kind]
		}
		queued, waitingFor, heldFor := 0, 0, 0
		for _, row := range rows {
			if row.Held {
				continue
			}
			if slices.Contains(row.Able, provider.ID) {
				queued++
				if row.Starts == nil {
					waitingFor++
				}
			}
			if row.HeldFor == provider.ID {
				heldFor++
			}
		}
		tick.QueuedForIt, tick.WaitingForIt, tick.HeldForIt = N(float64(queued)), N(float64(waitingFor)),
			N(float64(heldFor))
		ticks = append(ticks, tick)
	}
	return ticks, poolRow(ctx, now, lanes.Held, lanes.Planning, rows)
}

// poolRow is the pools' row: the runner slots and planning lanes the held runs take, and the
// queue by kind and difficulty.
func poolRow(ctx *hctx.Context, now time.Time, held map[int]string, planning map[int]bool,
	rows []*waiting.Row) *PoolTick {
	pool := ctx.Cfg.Pool
	hosted, machine := 0, 0
	for _, id := range held {
		if pool.OnMachine(id) {
			machine++
		} else {
			hosted++
		}
	}
	row := &PoolTick{At: now, GithubSlotsUsed: N(float64(hosted)), AWSSlotsUsed: N(float64(machine)),
		PlanLanesUsed: N(float64(len(planning))), GithubSlots: N(float64(pool.MaxParallel)),
		AWSSlots: N(float64(pool.MachineParallel)), PlanLanes: N(float64(pool.PlanLanes))}
	counts := map[string]int{}
	queued, heldFor := 0, 0
	var oldest time.Time
	for _, r := range rows {
		if r.Held {
			continue
		}
		queued++
		difficulty := r.Difficulty
		if difficulty != "medium" && difficulty != "hard" {
			difficulty = "unrated"
		}
		counts[r.Step+"_"+difficulty]++
		if r.HeldFor != "" {
			heldFor++
		}
		if at := clock.ParseISO(r.QueuedAt); !at.IsZero() && (oldest.IsZero() || at.Before(oldest)) {
			oldest = at
		}
	}
	count := func(key string) Num { return N(float64(counts[key])) }
	row.PlanMedium, row.PlanHard, row.PlanUnrated = count("plan_medium"), count("plan_hard"), count("plan_unrated")
	row.BuildMedium, row.BuildHard, row.BuildUnrated = count("build_medium"), count("build_hard"), count("build_unrated")
	row.ReviseMedium, row.ReviseHard, row.ReviseUnrated = count("revise_medium"), count("revise_hard"),
		count("revise_unrated")
	row.ReviewMedium, row.ReviewHard, row.ReviewUnrated = count("review_medium"), count("review_hard"),
		count("review_unrated")
	row.Queued, row.HeldForOwner = N(float64(queued)), N(float64(heldFor))
	if !oldest.IsZero() && !oldest.After(now) {
		row.OldestWaitMin = N(round1(now.Sub(oldest).Minutes()))
	}
	return row
}

// StateReadings is the newest reading the state file keeps for each subscription
// (`providers.<id>.usage`), as a reading row of the loop's (`during` tick, no source: whatever run
// took it handed it on itself, if it could), with the caps in force when it was read, in the
// usage order. One with no `observed_at` has no row.
func StateReadings(ctx *hctx.Context, state *pyjson.Object) []*Reading {
	var out []*Reading
	for _, provider := range ctx.Cfg.Pool.Ordered() {
		reading := providers.CurrentUsage(provider, providers.PeekRecord(state, provider.ID))
		observed := clock.ParseISOValue(reading.Get("observed_at"))
		if reading.Len() == 0 || observed.IsZero() {
			continue
		}
		caps := provider.CapsAt(observed, ctx.Cfg.Timezone, reading, time.Time{})
		out = append(out, FromUsage(reading, observed, provider.ID, "", "tick", "", caps))
	}
	return out
}
