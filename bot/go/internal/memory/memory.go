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
	"fmt"
	"math"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Proc = Path("/proc")
var Proc = "/proc"

// Keep is how many machine runs' figures the state file keeps.
var Keep int = 60

// Window is how far back the status issue's line looks.
var Window = 24 * time.Hour

// ShortFullS: a run whose `full` stall reached this many seconds ran short of memory.
var ShortFullS float64 = 5.0

// Gib = 1024 ** 3
var Gib = 1 << 30

// fields is _fields: `name value` lines (`/proc/vmstat`) or `name: value` ones (`/proc/meminfo`).
func fields(path string) (map[string]string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	found := map[string]string{}
	for _, line := range py.Splitlines(strings.ToValidUTF8(string(data), "\uFFFD")) {
		name, rest, _ := strings.Cut(strings.Replace(line, ":", " ", 1), " ")
		found[name] = py.Strip(rest)
	}
	return found, nil
}

// pressure is _pressure: `some` and `full` stall totals, in seconds, from a PSI file.
func pressure(path string) (map[string]float64, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	totals := map[string]float64{}
	for _, line := range py.Splitlines(strings.ToValidUTF8(string(data), "\uFFFD")) {
		kind, rest, _ := strings.Cut(line, " ")
		for _, part := range py.Split(rest) {
			key, value, _ := strings.Cut(part, "=")
			if key == "total" && py.IsDecimal(value) {
				n, _ := py.Int(value)
				totals[kind] = float64(n) / 1_000_000
			}
		}
	}
	return totals, nil
}

func getOr(m map[string]string, key, def string) string {
	if v, ok := m[key]; ok {
		return v
	}
	return def
}

// Reading is the machine's memory now, or None (nil) where the kernel does not say (not Linux).
func Reading(at time.Time, proc string /* = PROC */) *pyjson.Object {
	info, err := fields(filepath.Join(proc, "meminfo"))
	if err != nil {
		return nil
	}
	kib := map[string]int{}
	for _, name := range []string{"MemTotal", "MemAvailable", "SwapTotal", "SwapFree"} {
		parts := py.Split(getOr(info, name, "0"))
		if len(parts) == 0 {
			return nil
		}
		n, err := py.Int(parts[0])
		if err != nil {
			return nil
		}
		kib[name] = n * 1024
	}
	found := pyjson.New("total", kib["MemTotal"], "available", kib["MemAvailable"],
		"swap_used", kib["SwapTotal"]-kib["SwapFree"], "at", clock.ISO(at))
	if stat, err := fields(filepath.Join(proc, "vmstat")); err == nil {
		in, errIn := py.Int(getOr(stat, "pswpin", "0"))
		out, errOut := py.Int(getOr(stat, "pswpout", "0"))
		if errIn == nil && errOut == nil {
			found.Set("swapped", in+out)
		}
	}
	if stall, err := pressure(filepath.Join(proc, "pressure", "memory")); err == nil {
		found.Set("stall_some", floatOr(stall, "some")).Set("stall_full", floatOr(stall, "full"))
	}
	return found
}

func floatOr(m map[string]float64, key string) float64 {
	if v, ok := m[key]; ok {
		return v
	}
	return 0.0
}

// Add folds one reading into a job's `memory` record: the first and the last, the least
// available and the most swap used.
func Add(record *pyjson.Object, found *pyjson.Object) {
	if found == nil {
		return
	}
	record.SetDefault("start", found.Copy())
	record.Set("end", found.Copy())
	available := found.Get("available")
	record.Set("low_available", min(py.MustInt(record.GetOr("low_available", available)), py.MustInt(available)))
	record.Set("high_swap", max(py.MustInt(record.GetOr("high_swap", 0)), py.MustInt(found.Get("swap_used"))))
	record.Set("readings", py.MustInt(record.GetOr("readings", 0))+1)
}

// Summary is a job's memory over its run: stall seconds, pages swapped, the least available.
func Summary(record *pyjson.Object) *pyjson.Object {
	if record == nil {
		return nil
	}
	start, ok := record.Get("start").(*pyjson.Object)
	if !ok || start == nil {
		return nil
	}
	end := start
	if v := record.Get("end"); pyjson.Truthy(v) {
		end = pyjson.Obj(v)
	}
	delta := func(key string) float64 {
		a, errA := py.Float(end.GetOr(key, 0))
		b, errB := py.Float(start.GetOr(key, 0))
		if errA != nil || errB != nil {
			return 0.0
		}
		if d := a - b; d > 0 {
			return d
		}
		return 0.0
	}
	return pyjson.New(
		"at", py.OrStr(end.Get("at"), ""),
		"total", py.MustInt(py.Or(start.Get("total"), 0)),
		"low_available", py.MustInt(py.Or(record.Get("low_available"), 0)),
		"high_swap", py.MustInt(py.Or(record.Get("high_swap"), 0)),
		"stall_some", py.RoundN(delta("stall_some"), 1),
		"stall_full", py.RoundN(delta("stall_full"), 1),
		"swapped", int(delta("swapped")))
}

var boxPrefix = regexp.MustCompile(`^(-\p{Nd}+)?-?`)

// BoxOf is the box a runner is on, from its name (`bot/machine/register-runners.sh`): its label,
// then its lane past the first, then the second box's suffix (`night-vm-muse-2-b2`). "" is the
// night box, whose names end at the label or the lane.
func BoxOf(runner string, label string) string {
	rest := ""
	if strings.HasPrefix(runner, label) {
		rest = runner[len(label):]
	}
	return boxPrefix.ReplaceAllString(rest, "")
}

// Note keeps one machine run's summary, the newest `KEEP` of them.
func Note(state *pyjson.Object, found *pyjson.Object, runner string, box string /* = "" */) {
	kept := pyjson.Objs(state.Get("machine_memory"))
	entry := found.Copy()
	entry.Set("runner", runner)
	if box != "" {
		entry.Set("box", box)
	}
	kept = append(kept, entry)
	if len(kept) > Keep {
		kept = kept[len(kept)-Keep:]
	}
	state.Set("machine_memory", pyjson.ListOf(kept))
}

func gib(value int) string {
	return fmt.Sprintf("%.1f GB", float64(value)/float64(Gib))
}

// verdict is _verdict: one box's runs, the run count, whether it ran short, the least available,
// the stalls.
func verdict(runs []*pyjson.Object) string {
	total, low := py.MustInt(py.Or(runs[0].Get("total"), 0)), py.MustInt(py.Or(runs[0].Get("low_available"), 0))
	worst := py.MustFloat(py.Or(runs[0].Get("stall_full"), 0))
	short, swapped := 0, 0
	for i, e := range runs {
		if i > 0 {
			total = max(total, py.MustInt(py.Or(e.Get("total"), 0)))
			low = min(low, py.MustInt(py.Or(e.Get("low_available"), 0)))
			if f := py.MustFloat(py.Or(e.Get("stall_full"), 0)); f > worst { // max(): the first of equals
				worst = f
			}
		}
		if py.MustFloat(py.Or(e.Get("stall_full"), 0)) >= ShortFullS {
			short++
		}
		if py.MustInt(py.Or(e.Get("swapped"), 0)) > 0 {
			swapped++
		}
	}
	verdict := "memory to spare: its CPU, not its memory, sets `machine_parallel`"
	if short > 0 {
		verdict = "**short of memory**: fewer machine lanes (`machine_parallel`) or a bigger box"
	}
	return fmt.Sprintf("(%d runs): %s. Least available %s of %s; %d run(s) stalled on memory for "+
		"%s s or more (longest %s s); %d swapped.", len(runs), verdict, gib(low), gib(total), short,
		f0(ShortFullS), f0(worst), swapped)
}

// Line is the status issue's line on the machine's memory over the last day, one verdict a box,
// or "".
func Line(state *pyjson.Object, now time.Time) string {
	var order []string
	boxes := map[string][]*pyjson.Object{}
	for _, entry := range pyjson.Objs(state.Get("machine_memory")) {
		at := clock.ParseISOValue(entry.Get("at"))
		if !at.IsZero() && now.Sub(at) <= Window {
			box := py.OrStr(entry.Get("box"), "")
			if _, ok := boxes[box]; !ok {
				order = append(order, box)
			}
			boxes[box] = append(boxes[box], entry)
		}
	}
	if len(order) == 0 {
		return ""
	}
	head := "**Machine memory, last 24 hours**"
	if len(order) == 1 && order[0] == "" {
		return head + " " + verdict(boxes[""])
	}
	sort.Strings(order)
	lines := []string{head + ", a line a box:"}
	for _, box := range order {
		name := "the night box"
		if box != "" {
			name = fmt.Sprintf("the `%s` box", box)
		}
		lines = append(lines, fmt.Sprintf("- %s %s", name, verdict(boxes[box])))
	}
	return strings.Join(lines, "\n")
}

// f0 is f"{x:.0f}", which writes nan and inf as Python does.
func f0(x float64) string {
	switch {
	case math.IsNaN(x):
		return "nan"
	case math.IsInf(x, 1):
		return "inf"
	case math.IsInf(x, -1):
		return "-inf"
	}
	return fmt.Sprintf("%.0f", x)
}
