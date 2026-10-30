// Package memory is bot/harness/memory.py.
//
// The bot machine's memory (#312): whether its jobs run short of it, read by the jobs themselves.
//
// Each box has 2 vCPUs and 7.6 GB of RAM, with compressed swap in memory and 8 GB of swapfile, and
// CloudWatch keeps only its CPU unless the agent `bot/machine/setup.sh` installs is allowed to
// publish (the instance role needs `CloudWatchAgentServerPolicy`). The box powers off when idle, so `free` shows only the present.
// So every model job on the machine (`work.Worker`) reads the kernel's own counters as it starts,
// after each model call and check run, and as it ends, into `result.json`'s `memory`:
//
//   - the memory pressure stall time (`/proc/pressure/memory`, PSI): how long some task, and every
//     task at once (`full`), waited on memory during the job;
//   - the pages swapped in and out (`/proc/vmstat`);
//   - the least memory available at any reading (`/proc/meminfo`'s `MemAvailable`), and the most
//     swap in use.
//
// The deliver job keeps the last `KEEP` runs' figures in the state file (`machine_memory`), and the
// status issue shows the last day of them (`line`): with stalls and swapping near zero the box has
// memory to spare and `machine_parallel` is held by its CPU; with seconds of `full` stall a run, it
// is short of memory, and fewer machine lanes or a bigger box is the answer
// (`bot/machine/README.md`, Memory). Each box's runs are its own (#1677): a run on the second box
// keeps its runner's suffix as its `box` (`box_of`), and the line gives each box its own verdict, so
// a box short of memory is not hidden by one with memory to spare.
package memory

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"time"
)

// PROC = Path("/proc")
var Proc any // unported

// KEEP = 60
var Keep int = 60

// WINDOW = timedelta(hours=24)
var Window any // unported

// SHORT_FULL_S = 5.0
var ShortFullS float64 = 5.0

// GIB = 1024 ** 3
var Gib any // unported

// The machine's memory now, or None where the kernel does not say (not Linux).

func Reading(at time.Time, proc string /* = PROC */) *pyjson.Object {
	panic("unported: memory.reading")
}

// Fold one reading into a job's `memory` record: the first and the last, the least
// available and the most swap used.

func Add(record *pyjson.Object, found *pyjson.Object) {
	panic("unported: memory.add")
}

// A job's memory over its run: stall seconds, pages swapped, the least available.

func Summary(record *pyjson.Object) *pyjson.Object {
	panic("unported: memory.summary")
}

// The box a runner is on, from its name (`bot/machine/register-runners.sh`): its label, then
// its lane past the first, then the second box's suffix (`night-vm-muse-2-b2`). "" is the night
// box, whose names end at the label or the lane.

func BoxOf(runner string, label string) string {
	panic("unported: memory.box_of")
}

// Keep one machine run's summary, the newest `KEEP` of them.

func Note(state *pyjson.Object, found *pyjson.Object, runner string, box string /* = "" */) {
	panic("unported: memory.note")
}

// The status issue's line on the machine's memory over the last day, one verdict a box, or
// "".

func Line(state *pyjson.Object, now time.Time) string {
	panic("unported: memory.line")
}
