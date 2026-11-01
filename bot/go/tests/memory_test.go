// #312: the machine's memory, read by its own jobs, kept by deliver and shown on the status issue.
//
// CloudWatch keeps only the night box's CPU, and the box powers off when idle, so nothing showed
// whether a busy day ran it short of memory. Each machine job now reads the kernel's memory
// stall time, swapping and available memory as it goes.
//
// bot/tests/test_memory.py, ported (#467).
package tests

import (
	"fmt"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/memory"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// memoryGib is GIB.
const memoryGib = 1024 * 1024 * 1024

// memoryProc is proc(root, ...): a /proc with the kernel's memory counters under `root`.
func memoryProc(t *testing.T, root string, availableKiB, swapFreeKiB, swapped, someUS, fullUS int) string {
	t.Helper()
	if err := os.MkdirAll(filepath.Join(root, "pressure"), 0o777); err != nil {
		t.Fatal(err)
	}
	mustWriteText(filepath.Join(root, "meminfo"), fmt.Sprintf(
		"MemTotal:        8000000 kB\nMemFree:          100000 kB\n"+
			"MemAvailable:    %d kB\nSwapTotal:       8000000 kB\n"+
			"SwapFree:        %d kB\n", availableKiB, swapFreeKiB))
	mustWriteText(filepath.Join(root, "vmstat"), fmt.Sprintf("nr_free_pages 1000\npswpin %d\npswpout 0\n", swapped))
	mustWriteText(filepath.Join(root, "pressure", "memory"), fmt.Sprintf(
		"some avg10=0.00 avg60=0.00 avg300=0.00 total=%d\n"+
			"full avg10=0.00 avg60=0.00 avg300=0.00 total=%d\n", someUS, fullUS))
	return root
}

// ---------------------------------------------------------------------------- ReadingTests

func TestMemory_ItReadsTheKernelsCounters(t *testing.T) {
	root := memoryProc(t, t.TempDir(), 4_000_000, 7_000_000, 12, 2_500_000, 1_000_000)
	found := memory.Reading(testsupport.Day, root)
	assertEqual(t, []any{found.Get("total"), found.Get("available"), found.Get("swap_used")},
		[]any{8_000_000 * 1024, 4_000_000 * 1024, 1_000_000 * 1024})
	assertEqual(t, []any{found.Get("swapped"), found.Get("stall_some"), found.Get("stall_full")},
		[]any{12, 2.5, 1.0})
}

func TestMemory_NoKernelFiguresIsNoReading(t *testing.T) {
	assertTrue(t, memory.Reading(testsupport.Day, t.TempDir()) == nil, "reading is None")
}

func TestMemory_AJobsRunIsTheChangeBetweenItsReadings(t *testing.T) {
	record := pyjson.New()
	base := pyjson.New("total", 8*memoryGib, "swap_used", 0, "swapped", 100, "stall_some", 10.0,
		"stall_full", 2.0, "at", clock.ISO(testsupport.Day))
	memory.Add(record, base.Copy().Set("available", 5*memoryGib))
	memory.Add(record, base.Copy().Set("available", 1*memoryGib).Set("swap_used", memoryGib).Set("stall_full", 9.5))
	memory.Add(record, base.Copy().Set("available", 3*memoryGib).Set("swapped", 400).Set("stall_some", 30.0).
		Set("stall_full", 12.0).Set("at", clock.ISO(testsupport.Day.Add(20*time.Minute))))
	found := memory.Summary(record)
	assertEqual(t, []any{found.Get("low_available"), found.Get("high_swap"), found.Get("swapped")},
		[]any{memoryGib, memoryGib, 300})
	assertEqual(t, []any{found.Get("stall_some"), found.Get("stall_full")}, []any{20.0, 10.0})
	assertTrue(t, memory.Summary(pyjson.New()) == nil, "summary({}) is None")
}

// ---------------------------------------------------------------------------- WorkerTests

func memoryRunOn(t *testing.T, runsOn string, readings []*pyjson.Object) *pyjson.Object {
	t.Helper()
	root := t.TempDir()
	_, clone := fakes.MakeOrigin(root)
	cfg := testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("gates", workGates,
		"install", workInstall())})
	plan := workPlan("action", "build", "number", 12, "title", "Rules v2", "branch", "bot/issue-12",
		"thread", "Please make the rules v2.", "runs_on", runsOn)
	queue := append([]*pyjson.Object(nil), readings...)
	r := workFake(map[string]any{"build": workBuilder(map[string]any{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)})
	reader := func() *pyjson.Object {
		next := queue[0]
		if len(queue) > 1 {
			queue = queue[1:]
		}
		return next
	}
	work.NewWorker(cfg, plan, r, clone, filepath.Join(root, "work"), filepath.Join(root, "out"), nil,
		nil, nil, nil, nil, reader).Run()
	return loadJSON(t, filepath.Join(root, "out", "result.json"))
}

func TestMemory_AMachineJobRecordsItsMemory(t *testing.T) {
	base := pyjson.New("total", 8*memoryGib, "swap_used", 0, "swapped", 0, "stall_some", 0.0,
		"stall_full", 0.0, "at", clock.ISO(testsupport.Day))
	result := memoryRunOn(t, "night-vm-muse", []*pyjson.Object{base.Copy().Set("available", 6*memoryGib),
		base.Copy().Set("available", 2*memoryGib).Set("stall_full", 7.0)})
	readings, _ := pyjson.AsInt(dig(t, result, "memory", "readings"))
	assertTrue(t, readings >= 3, readings) // start, each call, checks, end
	assertEqual(t, memory.Summary(result.Obj("memory")).Get("stall_full"), 7.0)
	assertEqual(t, dig(t, result, "memory", "low_available"), 2*memoryGib)
}

func TestMemory_AJobOnGithubsRunnersReadsNothing(t *testing.T) {
	result := memoryRunOn(t, "ubuntu-latest", []*pyjson.Object{pyjson.New("total", 1, "available", 1,
		"swap_used", 0, "at", clock.ISO(testsupport.Day))})
	assertFalse(t, result.Has("memory"))
}

// ---------------------------------------------------------------------------- DeliverTests

// memorySkipUnlessLinux is the class's skipUnless: the kernel's counters are Linux's.
func memorySkipUnlessLinux(t *testing.T) {
	t.Helper()
	if st, err := os.Stat("/proc/meminfo"); err != nil || !st.Mode().IsRegular() {
		t.Skip("the kernel's counters are Linux's")
	}
}

func TestMemory_DeliverKeepsAMachineRunsMemory(t *testing.T) {
	memorySkipUnlessLinux(t)
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"muse"}, At: testsupport.Day})
	h.gh.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelBuild)})
	planned, _ := h.night(workFake(map[string]any{"build": workBuilder(map[string]any{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)}))
	assertEqual(t, planned.Get("provider"), "muse")
	kept := pyjson.Objs(h.ctx.Store.Load().Get("machine_memory"))
	assertEqual(t, len(kept), 1)
	total, _ := pyjson.AsFloat(dig(t, kept[0], "total"))
	assertTrue(t, total > 0, kept[0].Get("total"))
	assertEqual(t, kept[0].Get("runner"), "night-vm-muse")
	assertFalse(t, kept[0].Has("box"))
}

func TestMemory_ARunOnTheSecondBoxKeepsItsBox(t *testing.T) {
	memorySkipUnlessLinux(t)
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"muse"}, At: testsupport.Day})
	h.gh.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelBuild)})
	t.Setenv("RUNNER_NAME", "night-vm-muse-b2")
	h.night(workFake(map[string]any{"build": workBuilder(map[string]any{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)}))
	kept := pyjson.Objs(h.ctx.Store.Load().Get("machine_memory"))
	if len(kept) == 0 {
		t.Fatal("IndexError: no machine_memory kept")
	}
	assertEqual(t, []any{kept[0].Get("runner"), kept[0].Get("box")}, []any{"night-vm-muse-b2", "b2"})
}

// ---------------------------------------------------------------------------- LineTests

// memoryEntry is LineTests.entry(hours_ago, **values).
func memoryEntry(hoursAgo float64, values ...any) *pyjson.Object {
	at := testsupport.Day.Add(-time.Duration(hoursAgo * float64(time.Hour)))
	entry := pyjson.New("at", clock.ISO(at), "total", 8*memoryGib,
		"low_available", 3*memoryGib, "high_swap", 0, "stall_some", 0.5, "stall_full", 0.0,
		"swapped", 0, "runner", "night-vm-muse")
	return entry.Update(pyjson.New(values...))
}

func TestMemory_TheStatusIssueSaysWhetherTheBoxRanShort(t *testing.T) {
	state := pyjson.New()
	for _, entry := range []*pyjson.Object{memoryEntry(30, "stall_full", 60.0), memoryEntry(2), memoryEntry(1)} {
		memory.Note(state, entry, entry.Str("runner"), "")
	}
	text := memory.Line(state, testsupport.Day)
	assertIn(t, "(2 runs): memory to spare", text) // the run 30 hours ago is out of it
	assertIn(t, "Least available 3.0 GB of 8.0 GB", text)
	memory.Note(state, memoryEntry(0.5, "stall_full", 12.0, "swapped", 40,
		"low_available", py.FloorDiv(memoryGib, 2)), "night-vm-agy", "")
	text = memory.Line(state, testsupport.Day)
	assertIn(t, "**short of memory**", text)
	assertIn(t, "1 run(s) stalled on memory for 5 s or more (longest 12 s); 1 swapped", text)
	assertEqual(t, memory.Line(pyjson.New(), testsupport.Day), "")
}

func TestMemory_ARunnersNameSaysItsBox(t *testing.T) {
	// register-runners.sh's names: the label, a lane past the first, the second box's BOX.
	for _, c := range [][3]string{{"night-vm-muse", "night-vm-muse", ""},
		{"night-vm-lite-6", "night-vm-lite", ""},
		{"night-vm-muse-b2", "night-vm-muse", "b2"},
		{"night-vm-muse-2-b2", "night-vm-muse", "b2"},
		{"someone-else", "night-vm-muse", ""}} {
		assertEqual(t, memory.BoxOf(c[0], c[1]), c[2], c[0])
	}
}

func TestMemory_EachBoxHasItsOwnVerdict(t *testing.T) {
	// #1677: the night box with memory to spare does not hide the second box running short.
	state := pyjson.New()
	memory.Note(state, memoryEntry(1), "night-vm-muse", "")
	memory.Note(state, memoryEntry(2), "night-vm-agy", "")
	memory.Note(state, memoryEntry(1, "stall_full", 30.0, "swapped", 9), "night-vm-muse-b2", "b2")
	text := memory.Line(state, testsupport.Day)
	lines := py.Splitlines(text)
	if len(lines) != 3 {
		t.Fatalf("ValueError: %d lines after the first, not 2, in:\n%s", len(lines)-1, text)
	}
	night, second := lines[1], lines[2]
	assertIn(t, "**Machine memory, last 24 hours**, a line a box:", text)
	assertIn(t, "- the night box (2 runs): memory to spare", night)
	assertIn(t, "- the `b2` box (1 runs): **short of memory**", second)
	assertIn(t, "(longest 30 s); 1 swapped", second)
}

func TestMemory_TheStateKeepsTheNewestRunsOnly(t *testing.T) {
	state := pyjson.New()
	for n := 0; n < memory.Keep+5; n++ {
		memory.Note(state, memoryEntry(0, "swapped", n), "night-vm-muse", "")
	}
	kept := state.List("machine_memory")
	assertEqual(t, len(kept), memory.Keep)
	assertEqual(t, dig(t, state, "machine_memory", -1, "swapped"), memory.Keep+4)
}

func TestMemory_TheStatusIssueCarriesTheLine(t *testing.T) {
	ctx := testsupport.MakeCtx(nil, testsupport.Ctx{At: testsupport.Day})
	flowUpdate(ctx, func(s *pyjson.Object) { memory.Note(s, memoryEntry(1), "night-vm-muse", "") })
	assertIn(t, "**Machine memory, last 24 hours** (1 runs)", dashboard.Render(ctx, nil, "", nil, true))
}
