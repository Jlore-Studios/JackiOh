// Package disk is bot/harness/disk.py.
//
// The bot machine's disk (bot/machine/README.md, Disk): what a model job there reads of it, the
// room it makes before it starts, and the one issue that tells people while it is filling up.
//
// Every subscription on the machine shares its one disk, and a full one fails whichever job writes
// next: on 2026-10-04 a revision of #206 died on `OSError: [Errno 28] No space left on device`,
// with 66 MB free when its job began, and it counted as that item's failure. So a model job there
// (`work.Worker`):
//
//   - reads the disk as it starts and as it ends, into `result.json`'s `disk`;
//   - with less than `CLEAN_BELOW` free at its start, runs `bot/machine/clean.sh` first, as the
//     machine does after every job and every ten minutes;
//   - with less than `FLOOR` free after that, does no work: the result is `infra` on the machine's
//     account (`infra_scope: "machine"`), so the item stays queued and nothing counts against it;
//   - and when the disk fills up during the work anyway, it pauses (`interrupted`, `disk`) and keeps
//     what it built, as for any pause.
//
// The deliver job keeps the newest reading in the state file (`machine_disk`, `note`), and `alert`
// opens one issue for people when a reading is `ALERT_PERCENT` full or has less than `FLOOR` free,
// rewrites it as readings come in, and closes it once one is back at `CLEAR_PERCENT` or under. The
// status loop (`bot-status.yml`, every ten minutes) runs `alert` again on the kept reading, so an
// issue that a deliver job could not open or close is settled within ten minutes.
package disk

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"time"
)

// GIB = 1024 ** 3
var Gib any // unported

// CLEAN_BELOW = 8 * GIB
var CleanBelow any // unported

// FLOOR = 3 * GIB
var Floor any // unported

// ALERT_PERCENT = 80
var AlertPercent int = 80

// CLEAR_PERCENT = 70
var ClearPercent int = 70

// CLEAN_SCRIPT = Path("bot/machine/clean.sh")
var CleanScript any // unported

// CLEAN_SECONDS = 300
var CleanSeconds int = 300

// FULL_TEXT = "No space left on device"
var FullText string = "No space left on device"

// TITLE = "Night bot: the machine's disk is filling up"
var Title string = "Night bot: the machine's disk is filling up"

// MARKER = "<!-- jackioh-bot:disk -->"
var Marker string = "<!-- jackioh-bot:disk -->"

// LABELS = ("night bot", LABEL_HUMAN)
var Labels any // unported

// READINGS = ("start", "after_clean", "end")
var Readings = []string{"start", "after_clean", "end"}

// How full the filesystem holding `path` is. `percent` is as `df` counts it: used over what
// an ordinary user can have (the blocks kept for root left out).

func Reading(path string, at time.Time) *pyjson.Object {
	panic("unported: disk.reading")
}

// Whether `exc` is a full disk.

func IsFull(exc any) bool {
	panic("unported: disk.is_full")
}

// too_full is disk.too_full.

func TooFull(found *pyjson.Object) bool {
	panic("unported: disk.too_full")
}

// recovered is disk.recovered.

func Recovered(found *pyjson.Object) bool {
	panic("unported: disk.recovered")
}

// `17.3 GB`.

func Size(n any) string {
	panic("unported: disk.size")
}

// `83% full, 5.1 GB free of 30.0 GB`.

func Describe(found *pyjson.Object) string {
	panic("unported: disk.describe")
}

// Run the machine's clean-up as this job's user, and return the end of what it said. It
// never fails the job: a script that is missing or stuck is only reported.

func Clean(root string, env map[string]string) string {
	panic("unported: disk.clean")
}

// The newest valid reading a model job's result carries, if any.

func Newest(result *pyjson.Object) *pyjson.Object {
	panic("unported: disk.newest")
}

// Keep `found` as the machine's newest reading, unless the state holds a newer one.

func Note(state *pyjson.Object, found *pyjson.Object, runner string, runURL string) {
	panic("unported: disk.note")
}

// Open, rewrite or close the disk issue for the newest kept reading. Returns what it did.

func Alert(ctx *hctx.Context) string {
	panic("unported: disk.alert")
}

// The status issue's line about the disk, or None before any machine job reported one.

func Line(state *pyjson.Object) string {
	panic("unported: disk.line")
}
