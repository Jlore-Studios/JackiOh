// Package wake is bot/harness/wake.py.
//
// The wake (`.github/workflows/claude-wake.yml`): at `WAKE_AT` America/Chicago and every `EVERY`
// around it, each Claude subscription whose 5-hour session is not running gets one turn of the
// smallest model at low effort, so its session starts then and resets five hours later, rather than
// at whatever hour its next run lands. A session the state file shows running is left alone, and so
// is one whose usage limit refused a call or has a window full until later, and every subscription
// of a bot that is halted, since the turn is a model call. A ping its limit refuses is reported, not
// failed: the next wake time tries it again.
//
// GitHub starts this repository's schedules late and drops some, so the workflow fires every hour
// and a run within `EARLY` before a wake time sleeps until it (`wake_time`); one that starts after
// it, within `LATE`, wakes at once. Each subscription's turn runs in a job holding that one secret.
package wake

import (
	"fmt"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
)

// Ping is runner.ping as this module holds it; a variable so the tests can replace it.
var Ping = runner.Ping

// Zone is ZONE.
var Zone string = "America/Chicago"

// WakeAt is WAKE_AT. The wake times: `WAKE_AT` in `ZONE` and every `EVERY` around it, so 02:00,
// 06:00, 10:00, 14:00, 18:00 and 22:00. `EVERY` divides a day, and `EARLY` and `LATE` add up to no
// more than it, so each run is for at most one wake time.
var WakeAt = clock.TimeOfDay{Hour: 6, Minute: 0}

// Every is EVERY.
var Every = 4 * time.Hour

// Early is EARLY: how long before a wake time a run may start and sleep until it, and Late how long
// after it a run that GitHub started late still wakes the sessions.
var Early = time.Hour

// Late is LATE.
var Late = 2 * time.Hour

// Model is MODEL.
var Model string = "claude-haiku-5-5"

// Effort is EFFORT.
var Effort string = "low"

// Workflow is WORKFLOW.
var Workflow string = "claude-wake.yml"

// WakeTime is the wake time a run started at `now` is for: the one at most `EARLY` ahead of it or
// `LATE` behind it, None (the zero time) between them (nothing to do). It is in `ZONE`, as the
// Python's is.
func WakeTime(now time.Time) time.Time {
	zone := clock.Zone(Zone)
	year, month, today := now.In(zone).Date()
	for _, day := range []int{-1, 0, 1} {
		// datetime.combine(today + day, WAKE_AT, tzinfo=zone) + step * EVERY: the steps are
		// added to the wall clock, and each wall time is then read in the zone.
		first := time.Date(year, month, today+day, WakeAt.Hour, WakeAt.Minute, WakeAt.Second, 0, time.UTC)
		for step := 0; step < int(24*time.Hour/Every); step++ {
			wall := first.Add(time.Duration(step) * Every)
			at := localTime(wall, zone)
			if !now.Before(localTime(wall.Add(-Early), zone)) && now.Before(localTime(wall.Add(Late), zone)) {
				return at
			}
		}
	}
	return time.Time{}
}

// AlreadyWoken is whether a run of `WORKFLOW` already finished green since the wake time `now` is
// for: the schedule's later firings queue behind the first, and find the sessions started.
func AlreadyWoken(runs []*pyjson.Object, now time.Time) bool {
	since := WakeTime(now)
	if since.IsZero() {
		return false
	}
	for _, run := range runs {
		ended := clock.ParseISOValue(run.Get("updated_at"))
		if pyjson.Equal(run.Get("conclusion"), "success") && !ended.IsZero() && !ended.Before(since) {
			return true
		}
	}
	return false
}

// runningUntil is _running_until.
func runningUntil(entry *pyjson.Object, now time.Time) time.Time {
	usage := entry.Obj("usage")
	reading := usage.Obj("five_hour")
	resets := clock.ParseISOValue(reading.Get("resets_at"))
	if !resets.IsZero() && resets.After(now) {
		return resets
	}
	return time.Time{}
}

// refusedUntil is _refused_until: until when the subscription's limit would refuse a ping: the
// state file says it refused a call, or has a window (the week's, say) full, that has not reset by
// `now`.
func refusedUntil(entry *pyjson.Object, now time.Time) time.Time {
	usage := entry.Obj("usage")
	times := []time.Time{clock.ParseISOValue(entry.Get("refused_until"))}
	usage.Items(func(_ string, value any) bool {
		if w, ok := value.(*pyjson.Object); ok && w != nil {
			if utilization, isNumber := py.Number(w.Get("utilization")); isNumber && utilization >= 1.0 {
				times = append(times, clock.ParseISOValue(w.Get("resets_at")))
			}
		}
		return true
	})
	var later time.Time
	for _, t := range times {
		if !t.IsZero() && t.After(now) && (later.IsZero() || t.After(later)) {
			later = t
		}
	}
	return later
}

// Accounts is the Claude subscriptions to wake, as the workflow's matrix (`id`, `secret`, `home`,
// in that order), and a line for each one left alone and why.
func Accounts(cfg *config.Config, state *pyjson.Object, now time.Time, home string) ([]*pyjson.Object, []string) {
	found := []*pyjson.Object{}
	notes := []string{}
	for _, provider := range cfg.Pool.Ordered() {
		if provider.CLI != "claude" || provider.Login != "secret" {
			continue
		}
		if !provider.Enabled {
			notes = append(notes, fmt.Sprintf("`%s`: turned off", provider.ID))
			continue
		}
		if has := cfg.Secrets.Has(provider.Secret); has != nil && !*has {
			notes = append(notes, fmt.Sprintf("`%s`: no `%s` secret", provider.ID, provider.Secret))
			continue
		}
		if providers.Suspension(state, provider.ID) != nil {
			notes = append(notes, fmt.Sprintf("`%s`: suspended", provider.ID))
			continue
		}
		entry := providers.PeekRecord(state, provider.ID)
		if running := runningUntil(entry, now); !running.IsZero() {
			notes = append(notes, fmt.Sprintf("`%s`: its session is running until %s", provider.ID,
				clock.ISO(running)))
			continue
		}
		if refused := refusedUntil(entry, now); !refused.IsZero() {
			notes = append(notes, fmt.Sprintf("`%s`: its usage limit refuses calls until %s",
				provider.ID, clock.ISO(refused)))
			continue
		}
		found = append(found, pyjson.New("id", provider.ID, "secret", provider.Secret, "home", home))
	}
	return found, notes
}

// ToWake is `accounts` for this bot now, or none while it is halted (`HALT` or `/harness halt`).
func ToWake(ctx *hctx.Context, home string) ([]*pyjson.Object, []string) {
	state := ctx.Store.Load()
	if ctx.RepoHalted() || state.Truthy("halted") {
		return []*pyjson.Object{}, []string{"halted: a wake is a model call"}
	}
	return Accounts(ctx.Cfg, state, ctx.Now(), home)
}

// Wake is one turn of `MODEL` at `EFFORT` on `provider_id`'s subscription, signed in with the job's
// `CLAUDE_CODE_OAUTH_TOKEN`; whether it started the session or its usage limit refused it (no
// failure: there is nothing to start until the limit resets), and what it said.
func Wake(cfg *config.Config, providerID string) (bool, string) {
	provider := cfg.Pool.Get(providerID)
	if provider == nil || provider.CLI != "claude" {
		return false, fmt.Sprintf("`%s` is not a Claude subscription here", providerID)
	}
	// run=nil is ping's default, subprocess.run.
	result, usage := Ping(cfg.ClaudeBin(), Model, nil, "", Effort)
	var text any
	if result != nil {
		text = result.Get("result")
	}
	limited := result != nil && result.Truthy("is_error") && rateLimited(py.OrStr(text, ""))
	if pyjson.Equal(usage.Get("status"), "rejected") || limited {
		var resets any = "its reset"
		if usage.Truthy("rejected_resets_at") {
			resets = usage.Get("rejected_resets_at")
		}
		return true, fmt.Sprintf("`%s`: its usage limit refused the ping, so it is left until %s; "+
			"the next wake time tries it again", providerID, py.Str(resets))
	}
	if usage.Len() == 0 {
		return false, fmt.Sprintf("`%s`: the CLI gave no usage reading, so the session may not "+
			"have started", providerID)
	}
	reading := usage.Obj("five_hour")
	var resets any = "an unknown time"
	if reading.Truthy("resets_at") {
		resets = reading.Get("resets_at")
	}
	return true, fmt.Sprintf("`%s`: the 5-hour session is running, resetting at %s", providerID, py.Str(resets))
}

// rateLimited is RATE_LIMIT_WORDS.search(text).
func rateLimited(text string) bool { return runner.RateLimitWords.MatchString(text) }

// localTime is a wall-clock time (held in UTC) read in `zone` as Python's aware datetimes are,
// with fold=0: a wall time a transition skips or repeats takes the offset from before the
// transition (Go's time.Date picks either in a gap). The result is in `zone`. Where the wall time
// does not exist (02:00 on the night the clocks go forward), the Python's datetime still prints
// 02:00 while this instant prints 03:00 CDT.
func localTime(wall time.Time, zone *time.Location) time.Time {
	unix := wall.Unix()
	offsetAt := func(at int64) int64 {
		_, offset := time.Unix(at, 0).In(zone).Zone()
		return int64(offset)
	}
	widest := max(offsetAt(unix-14*3600), offsetAt(unix+14*3600))
	return time.Unix(unix-offsetAt(unix-widest), 0).In(zone)
}
