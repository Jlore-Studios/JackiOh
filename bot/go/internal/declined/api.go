// Package declined is bot/harness/declined.py.
//
// Suggestions that came to nothing (#797), kept so that no survey offers one again.
//
// A survey sees only the latest issues (`plan.suggestion_plan`), so a suggestion rejected weeks ago
// falls out of its view and comes back. This log does not: it lives in the state file
// (`suggest.declined`), newest last, and every survey reads all of it. A suggestion goes in when:
//
//   - a person rejects it with a `reject` comment (`events.answer_vote`);
//   - a person closes it unbuilt (`sync`, read from GitHub at each survey);
//   - it was approved, and the bot then handed it to people as work only a person can do
//     (`deliver.Deliverer._needs_person`) or failed it for good (`deliver.Deliverer._fail`).
//
// A suggestion the survey offers again anyway, by a title already here, is not opened
// (`deliver.Deliverer._suggestions`).
package declined

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// LIMIT = 200
var Limit int = 200

// entries is declined.entries.

func Entries(state *pyjson.Object) []*pyjson.Object {
	panic("unported: declined.entries")
}

// Log suggestion `number` as come to nothing, once: True when it was not logged yet.

func Note(state *pyjson.Object, number int, title string, why string, at string) bool {
	panic("unported: declined.note")
}

// Log every closed suggestion in `closed` (issues labelled `bot:suggestion`) that was closed
// unbuilt: a suggestion that is built loses the label as it is queued, so one still carrying it
// when closed never was. Returns the numbers newly logged.

func Sync(state *pyjson.Object, closed []*pyjson.Object, at string) []int {
	panic("unported: declined.sync")
}

// A suggestion of this title came to nothing before.

func Known(state *pyjson.Object, title string) bool {
	panic("unported: declined.known")
}

// The log as the survey reads it, one suggestion a line, or "(none)".

func Lines(state *pyjson.Object) string {
	panic("unported: declined.lines")
}
