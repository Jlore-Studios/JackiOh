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
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"time"
)

// ZONE = "America/Chicago"
var Zone string = "America/Chicago"

// WAKE_AT = time(6, 0)
var WakeAt any // unported

// EVERY = timedelta(hours=4)
var Every any // unported

// EARLY = timedelta(hours=1)
var Early any // unported

// LATE = timedelta(hours=2)
var Late any // unported

// MODEL = "claude-haiku-5-5"
var Model string = "claude-haiku-5-5"

// EFFORT = "low"
var Effort string = "low"

// WORKFLOW = "claude-wake.yml"
var Workflow string = "claude-wake.yml"

// The wake time a run started at `now` is for: the one at most `EARLY` ahead of it or
// `LATE` behind it, None between them (nothing to do).

func WakeTime(now time.Time) time.Time {
	panic("unported: wake.wake_time")
}

// Whether a run of `WORKFLOW` already finished green since the wake time `now` is for: the
// schedule's later firings queue behind the first, and find the sessions started.

func AlreadyWoken(runs []*pyjson.Object, now time.Time) bool {
	panic("unported: wake.already_woken")
}

// The Claude subscriptions to wake, as the workflow's matrix (`id`, `secret`, `home`), and a
// line for each one left alone and why.

func Accounts(cfg *config.Config, state *pyjson.Object, now time.Time, home string) ([]map[string]string, []string) {
	panic("unported: wake.accounts")
}

// `accounts` for this bot now, or none while it is halted (`HALT` or `/harness halt`).

func ToWake(ctx any, home string) ([]map[string]string, []string) {
	panic("unported: wake.to_wake")
}

// One turn of `MODEL` at `EFFORT` on `provider_id`'s subscription, signed in with the job's
// `CLAUDE_CODE_OAUTH_TOKEN`; whether it started the session or its usage limit refused it (no
// failure: there is nothing to start until the limit resets), and what it said.

func Wake(cfg *config.Config, providerID string) (bool, string) {
	panic("unported: wake.wake")
}
