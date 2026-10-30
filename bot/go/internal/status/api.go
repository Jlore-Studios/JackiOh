// Package status is bot/harness/status.py.
//
// The `status` report: halt, which subscriptions are running what, each subscription, and what
// is queued.
package status

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"time"
)

// DOING = {"plan": "planning", "build": "building", "revise": "revising", "review": "reviewing"}
var Doing = map[string]string{"plan": "planning", "build": "building", "revise": "revising", "review": "reviewing"}

// How many runs are on the bot's machine: "2 of 6 on the night box".

func MachineText(pool any, live *pyjson.Object) string {
	panic("unported: status.machine_text")
}

// The cap on a usage window now: "cap 90%" or "no cap", and for a subscription whose cap
// changes with its hours (`off_hours`) the other one too: "cap 50% now, 70% in its hours".
// With its reading, a cap its last hour or its late week raised says so (`caps_at`).

func CapText(ctx *hctx.Context, provider *providers.Provider, window string, usage any /* = None */) string {
	panic("unported: status.cap_text")
}

// What a reader needs beside a usage window's reading: its cap now (`cap_text`), the line a
// build, a revision or a plan must start under (`start_headroom`), the minutes it has spent of
// a budget, and when the window resets (`when` writes the time: "in 3h 10m" by default).

func WindowNotes(ctx *hctx.Context, provider *providers.Provider, entry *pyjson.Object, window string, when func(time.Time) string /* = None */) []string {
	panic("unported: status.window_notes")
}

// The state record of a lane: an item's, or the suggestion survey's (number 0).

func Record(state *pyjson.Object, number int) *pyjson.Object {
	panic("unported: status._record")
}

// record_of = _record
var RecordOf any // unported

// The lanes the state file says are held (item number, 0 for a survey -> provider), and those
// of them whose run GitHub has not seen end.

func LanesNow(ctx *hctx.Context, state *pyjson.Object) (*pyjson.Object, *pyjson.Object) {
	panic("unported: status.lanes_now")
}

// `held` without the runs GitHub says have ended (the next plan requeues their items). A run
// GitHub cannot read counts as still going, as it does for the plan (`plan.read_lanes`).

func LiveLanes(ctx *hctx.Context, state *pyjson.Object, held *pyjson.Object) *pyjson.Object {
	panic("unported: status.live_lanes")
}

// Which subscriptions are running now: on what, for how long, and in which run.

func RunningLines(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object) []string {
	panic("unported: status.running_lines")
}

// provider_lines is status.provider_lines.

func ProviderLines(ctx *hctx.Context, state *pyjson.Object, held *pyjson.Object) []string {
	panic("unported: status.provider_lines")
}

// report is status.report.

func Report(ctx *hctx.Context) string {
	panic("unported: status.report")
}
