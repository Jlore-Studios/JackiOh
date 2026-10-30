// test_disk.py: the bot machine's disk (harness/disk.py, bot/machine/clean.sh): a job's readings,
// the room it makes, the refusal under the floor, the pause on a full disk, and the one issue for
// people.
package tests

import (
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/disk"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// diskGib is GIB = disk.GIB.
var diskGib = disk.Gib

// diskFoundAt is found()'s default `at`.
const diskFoundAt = "2026-10-04T20:43:51Z"

// diskFound is found(percent, free_gib, at="2026-10-04T20:43:51Z", total_gib=30): a reading.
func diskFound(percent int, freeGib float64, at ...string) *pyjson.Object {
	when := diskFoundAt
	if len(at) > 0 {
		when = at[0]
	}
	return diskFoundOf(percent, freeGib, when, 30)
}

func diskFoundOf(percent int, freeGib float64, at string, totalGib float64) *pyjson.Object {
	return pyjson.New("total", int(totalGib*float64(diskGib)), "free", int(freeGib*float64(diskGib)),
		"percent", percent, "at", at)
}

// diskOSError is Python's OSError(errno, strerror): "[Errno 28] No space left on device", which
// is its errno too.
type diskOSError struct {
	errno    syscall.Errno
	strerror string
}

func (e *diskOSError) Error() string { return fmt.Sprintf("[Errno %d] %s", int(e.errno), e.strerror) }
func (e *diskOSError) Unwrap() error { return e.errno }

// ---------------------------------------------------------------------------- RulesTests

func TestDisk_TooFullIsTheShareUsedOrTheFloor(t *testing.T) {
	workflowsTrue(t, disk.TooFull(diskFound(80, 6)))
	workflowsTrue(t, disk.TooFull(diskFound(60, 2.9))) // a big disk with little left
	wasteFalse(t, disk.TooFull(diskFound(79, 6)))
	workflowsTrue(t, disk.Recovered(diskFound(70, 9)))
	wasteFalse(t, disk.Recovered(diskFound(75, 7))) // between the lines: neither
	wasteFalse(t, disk.TooFull(diskFound(75, 7)))
}

func TestDisk_AFullDiskInAnyError(t *testing.T) {
	workflowsTrue(t, disk.IsFull(&diskOSError{syscall.ENOSPC, "No space left on device"}))
	workflowsTrue(t, disk.IsFull(errors.New("git commit: fatal: No space left on device"))) // RuntimeError
	wasteFalse(t, disk.IsFull(&diskOSError{syscall.ENOENT, "missing"}))
}

func TestDisk_AReadingIsDfsShare(t *testing.T) {
	got := disk.Reading(os.TempDir(), time.Now().UTC())
	percent, total, free := got.Int("percent"), got.Int("total"), got.Int("free")
	workflowsTrue(t, 0 <= percent && percent <= 100, percent)
	workflowsLessEqual(t, free, total)
}

func TestDisk_OnlyValidReadingsFromAResultCountAndTheNewestWins(t *testing.T) {
	result := pyjson.New("disk", pyjson.New("start", diskFound(95, 1), "after_clean", diskFound(60, 12),
		"end", pyjson.New("total", "lots", "free", 1, "percent", 50)))
	wasteEq(t, disk.Newest(result).Get("percent"), 60)
	workflowsTrue(t, disk.Newest(pyjson.New("disk", pyjson.New("start",
		pyjson.New("total", 1, "free", 5, "percent", 3)))) == nil)
	workflowsTrue(t, disk.Newest(pyjson.New("disk", "full")) == nil)
	workflowsTrue(t, disk.Newest(pyjson.New()) == nil)
}

func TestDisk_AnOlderReadingNeverReplacesANewerOne(t *testing.T) {
	st := pyjson.New()
	disk.Note(st, diskFound(50, 15, "2026-10-04T21:00:00Z"), "night-vm-gpt", "")
	disk.Note(st, diskFound(90, 3, "2026-10-04T20:00:00Z"), "night-vm-muse", "")
	wasteEq(t, st.Obj("machine_disk").Obj("reading").Get("percent"), 50)
	disk.Note(st, diskFound(85, 4, "2026-10-04T22:00:00Z"), "night-vm-muse-2", "u")
	wasteEq(t, st.Obj("machine_disk").Get("runner"), "night-vm-muse-2")
}

// ---------------------------------------------------------------------------- AlertTests

// diskAlert is AlertTests' fixture: a fake GitHub and its context.
type diskAlert struct {
	gh  *fakes.FakeGitHub
	ctx *hctx.Context
}

func newDiskAlert() *diskAlert {
	gh := fakes.NewFakeGitHub()
	return &diskAlert{gh: gh, ctx: testsupport.MakeCtx(gh)}
}

// keep is AlertTests.keep(reading, runner="night-vm-muse-2").
func (a *diskAlert) keep(reading *pyjson.Object) {
	a.ctx.Store.Update(func(s *pyjson.Object) any {
		disk.Note(s, reading, "night-vm-muse-2", "https://run/1")
		return nil
	}, "test")
}

// issues is AlertTests.issues(): every disk issue, open or closed.
func (a *diskAlert) issues() []*pyjson.Object {
	found := []*pyjson.Object{}
	for _, issue := range a.gh.ListIssues("", "all", 300, "") {
		if pyjson.Equal(issue.Get("title"), disk.Title) {
			found = append(found, issue)
		}
	}
	return found
}

// one is `[issue] = self.issues()`: a ValueError unless there is exactly one.
func (a *diskAlert) one(t *testing.T) *pyjson.Object {
	t.Helper()
	found := a.issues()
	if len(found) != 1 {
		t.Fatalf("ValueError: %d disk issues, not one", len(found))
	}
	return found[0]
}

// diskNumber is `issue["number"]`.
func diskNumber(issue *pyjson.Object) int {
	n, _ := pyjson.AsInt(issue.Get("number"))
	return n
}

func TestDisk_ItOpensOnceRewritesWithNewReadingsAndClosesWhenBackUnder(t *testing.T) {
	a := newDiskAlert()
	a.keep(diskFound(55, 13))
	workflowsEqual(t, disk.Alert(a.ctx), "")
	workflowsEqual(t, len(a.issues()), 0)

	a.keep(diskFound(97, 0.06, "2026-10-04T20:44:00Z"))
	workflowsIn(t, "opened", disk.Alert(a.ctx))
	issue := a.one(t)
	workflowsEqualMaps(t, a.gh.LabelNames(diskNumber(issue)), workflowsSet("night bot", config.LabelHuman))
	workflowsIn(t, "97% full, 0.1 GB free of 30.0 GB", wasteStr(issue, "body"))
	workflowsIn(t, "bot/machine/on-machine.sh bot/machine/disk-report.sh", wasteStr(issue, "body"))
	workflowsEqual(t, disk.Alert(a.ctx), "") // the same reading again: nothing to say

	a.keep(diskFound(91, 2.7, "2026-10-04T20:55:00Z"))
	workflowsIn(t, "rewrote", disk.Alert(a.ctx))
	workflowsEqual(t, len(a.issues()), 1)
	workflowsIn(t, "91% full", wasteStr(a.gh.GetIssue(diskNumber(issue)), "body"))

	a.keep(diskFound(75, 7, "2026-10-04T21:05:00Z")) // better, not yet back under
	workflowsEqual(t, disk.Alert(a.ctx), "")
	wasteEq(t, a.gh.GetIssue(diskNumber(issue)).Get("state"), "open")

	a.keep(diskFound(42, 17, "2026-10-04T21:15:00Z"))
	workflowsIn(t, "closed", disk.Alert(a.ctx))
	wasteEq(t, a.gh.GetIssue(diskNumber(issue)).Get("state"), "closed")
	workflowsIn(t, "back to 42% full", wasteLast(t, a.gh.BotComments(diskNumber(issue))))
	machineDisk := a.ctx.Store.Load().Obj("machine_disk")
	if v, ok := machineDisk.Lookup("issue"); !ok || v != nil {
		t.Fatalf("%v is not None", v)
	}
}

func TestDisk_APersonWhoClosedItIsNotOverruledByTheSameReading(t *testing.T) {
	a := newDiskAlert()
	a.keep(diskFound(90, 3, "2026-10-04T20:00:00Z"))
	disk.Alert(a.ctx)
	issue := a.one(t)
	a.gh.UpdateIssue(diskNumber(issue), pyjson.New("state", "closed", "closed_at", "2026-10-04T20:30:00Z"))
	workflowsEqual(t, disk.Alert(a.ctx), "")
	workflowsEqual(t, len(a.issues()), 1)
	a.keep(diskFound(92, 2.5, "2026-10-04T21:00:00Z")) // a newer reading, still too full
	workflowsIn(t, "opened", disk.Alert(a.ctx))
	workflowsEqual(t, len(a.issues()), 2)
}

func TestDisk_TheStatusIssueShowsTheReading(t *testing.T) {
	a := newDiskAlert()
	workflowsEqual(t, disk.Line(a.ctx.Store.Load()), "")
	a.keep(diskFound(40, 18))
	workflowsIn(t, "🟢 **The machine's disk:** 40% full, 18.0 GB free of 30.0 GB", disk.Line(a.ctx.Store.Load()))
}

// ---------------------------------------------------------------------------- WorkerTests
//
// On the machine a model job reads its disk, cleans when short and stops under the floor.

// diskWorkerSetup is WorkerTests.setUp: a repository and a config with the test gates.
type diskWorkerSetup struct {
	root, origin, clone string
	cfg                 *config.Config
}

func newDiskWorkerSetup(t *testing.T) *diskWorkerSetup {
	s := &diskWorkerSetup{root: t.TempDir()}
	s.origin, s.clone = fakes.MakeOrigin(s.root)
	s.cfg = testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("gates", wasteGates(),
		"install", wasteInstall())})
	return s
}

// worker is WorkerTests.worker(runner, readings, runs_on="night-vm-muse"): each disk reading in
// turn, the last one again and again.
func (s *diskWorkerSetup) worker(r *runner.FakeRunner, readings []*pyjson.Object, runsOn ...string) *work.Worker {
	on := "night-vm-muse"
	if len(runsOn) > 0 {
		on = runsOn[0]
	}
	planned := pyjson.New("action", "build", "number", 12, "title", "Rules v2", "branch", "bot/issue-12",
		"thread", "Please make the rules v2.", "runs_on", on)
	queued := append([]*pyjson.Object(nil), readings...)
	reader := func() *pyjson.Object {
		if len(queued) > 1 {
			first := queued[0]
			queued = queued[1:]
			return first
		}
		return queued[0]
	}
	return work.NewWorker(s.cfg, planned, r, s.clone, filepath.Join(s.root, "work"), filepath.Join(s.root, "out"),
		nil, nil, nil, nil, reader, nil)
}

// diskPatchClean is `with mock.patch.object(disk, "clean", return_value=value) as clean:`: the
// calls it took, as the mock counts them, and the end of the block, which puts disk.clean back.
func diskPatchClean(t *testing.T, value string) (*int, func()) {
	t.Helper()
	calls := 0
	saved := disk.Clean
	restore := func() { disk.Clean = saved }
	t.Cleanup(restore)
	disk.Clean = func(string, map[string]string) string {
		calls++
		return value
	}
	return &calls, restore
}

func TestDisk_UnderTheFloorAfterCleaningItDoesNoWorkAndBlamesTheMachine(t *testing.T) {
	s := newDiskWorkerSetup(t)
	r := wasteRunner(map[string]runner.Handler{"build": wasteBuilder(map[string]string{"src/game.txt": "v2\n"})})
	clean, done := diskPatchClean(t, "cleaned")
	result := s.worker(r, []*pyjson.Object{diskFound(99, 0.06), diskFound(97, 1)}).Run()
	done()
	workflowsEqual(t, *clean, 1) // assert_called_once
	workflowsEqual(t, len(r.Calls), 0)
	wasteEq(t, result.Get("status"), "infra")
	wasteEq(t, result.Get("infra_scope"), "machine")
	workflowsIn(t, "97% full, 1.0 GB free", wasteStr(result, "reason"))
	wasteEq(t, result.Obj("disk").Get("cleaned"), "cleaned")
	keys := workflowsSet(result.Obj("disk").Keys()...)
	delete(keys, "runner")
	workflowsEqualMaps(t, keys, workflowsSet("start", "after_clean", "cleaned", "end"))
}

func TestDisk_RoomEnoughNeedsNoCleanAndTheWorkGoesOn(t *testing.T) {
	s := newDiskWorkerSetup(t)
	r := wasteRunner(map[string]runner.Handler{"build": wasteBuilder(map[string]string{"src/game.txt": "rules v2\n"}),
		"review": wasteReviewer(wasteApprove)})
	clean, done := diskPatchClean(t, "")
	result := s.worker(r, []*pyjson.Object{diskFound(40, 18)}).Run()
	done()
	workflowsEqual(t, *clean, 0) // assert_not_called
	wasteEq(t, result.Get("status"), "approved")
	wasteEq(t, result.Obj("disk").Obj("start").Get("percent"), 40)
	workflowsTrue(t, result.Obj("disk").Has("end"))
}

func TestDisk_CleaningThatMakesRoomLetsItStart(t *testing.T) {
	s := newDiskWorkerSetup(t)
	r := wasteRunner(map[string]runner.Handler{"build": wasteBuilder(map[string]string{"src/game.txt": "rules v2\n"}),
		"review": wasteReviewer(wasteApprove)})
	_, done := diskPatchClean(t, "ok")
	result := s.worker(r, []*pyjson.Object{diskFound(85, 4), diskFound(60, 12)}).Run()
	done()
	wasteEq(t, result.Get("status"), "approved")
}

func TestDisk_ADiskThatFillsUpMidRunPausesAndKeepsTheWork(t *testing.T) {
	s := newDiskWorkerSetup(t)
	fillsUp := func(request *runner.RunRequest) *runner.RunResult {
		herr.Must(os.WriteFile(filepath.Join(request.Cwd, "src", "game.txt"), []byte("half of v2\n"), 0o666))
		panic(&diskOSError{syscall.ENOSPC, "No space left on device"})
	}
	result := s.worker(wasteRunner(map[string]runner.Handler{"build": fillsUp}), []*pyjson.Object{diskFound(50, 15)}).Run()
	wasteEq(t, result.Get("status"), "interrupted")
	wasteEq(t, result.Get("interrupt"), "disk")
	workflowsIn(t, "the machine's disk filled up", wasteStr(result, "reason"))
}

func TestDisk_GithubsRunnersReadNoDisk(t *testing.T) {
	s := newDiskWorkerSetup(t)
	r := wasteRunner(map[string]runner.Handler{"build": wasteBuilder(map[string]string{"src/game.txt": "rules v2\n"}),
		"review": wasteReviewer(wasteApprove)})
	result := s.worker(r, []*pyjson.Object{diskFound(99, 0.01)}, "ubuntu-latest").Run()
	wasteEq(t, result.Get("status"), "approved")
	workflowsTrue(t, !result.Has("disk"))
}

// ---------------------------------------------------------------------------- DeliverTests

func TestDisk_ARefusalKeepsTheItemQueuedUncountedAndOpensTheIssue(t *testing.T) {
	h := newWasteHarness(t, wasteHarnessOpts{Machine: []string{"muse"}})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	out := filepath.Join(h.root, "out-full")
	herr.Must(os.Mkdir(out, 0o777))
	herr.Must(os.WriteFile(filepath.Join(out, "result.json"), []byte(pyjson.Dumps(pyjson.New(
		"status", "infra", "interrupt", "infra", "infra_scope", "machine",
		"reason", "the machine's disk is 97% full, 1.0 GB free of 30.0 GB, under the 3.0 GB "+
			"a job needs, even after cleaning",
		"disk", pyjson.New("runner", "night-vm-muse-2", "start", diskFound(99, 0.06),
			"after_clean", diskFound(97, 1, "2026-10-04T20:44:30Z"))))), 0o666))
	twelve := 12
	deliver.NewDeliverer(h.ctx, planned.Copy().Set("provider", "muse"), out, h.deliverRepo,
		"build", &twelve, "muse").Run()
	workflowsEqualMaps(t, h.gh.LabelNames(12), workflowsSet(config.LabelBuild))
	workflowsIn(t, "not this item's fault", wasteLast(t, h.gh.BotComments(12)))
	st := h.ctx.Store.Load()
	wasteEq(t, st.Obj("items").Obj("12").GetOr("failures", 0), 0)
	wasteEq(t, st.Obj("machine_disk").Get("runner"), "night-vm-muse-2")
	wasteEq(t, st.Obj("machine_disk").Obj("reading").Get("percent"), 97)
	opened := []*pyjson.Object{}
	for _, issue := range h.gh.ListIssues("", "open", 300, "") {
		if pyjson.Equal(issue.Get("title"), disk.Title) {
			opened = append(opened, issue)
		}
	}
	workflowsEqual(t, len(opened), 1)
	wasteEq(t, st.Obj("machine_disk").Get("issue"), opened[0].Get("number"))
}

func TestDisk_AJobOnGithubsRunnersReportsNothing(t *testing.T) {
	h := newWasteHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	out := filepath.Join(h.root, "out-hosted")
	r := wasteRunner(map[string]runner.Handler{"build": wasteBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": wasteReviewer(wasteApprove)})
	wasteWorker(h.cfg, planned, r, h.clone, filepath.Join(h.root, "work"), out).Run()
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	workflowsTrue(t, !h.ctx.Store.Load().Has("machine_disk"))
}

// ---------------------------------------------------------------------------- CleanScriptTests
//
// clean.sh, run as it runs on the machine: as the user, in a home of its own. Skipped as root, as
// the Python's class is (`skipIf(os.geteuid() == 0, …)`).

// diskScript is SCRIPT = ROOT / "bot" / "machine" / "clean.sh".
var diskScript = filepath.Join(testsupport.Root, "bot", "machine", "clean.sh")

// diskClean is CleanScriptTests' fixture.
type diskClean struct {
	t                *testing.T
	home, tmp, bin   string
	workers          string
	pgrepStatus      *int
	runnerDir, lane2 string
}

func newDiskClean(t *testing.T) *diskClean {
	t.Helper()
	if os.Geteuid() == 0 {
		t.Skip("clean.sh runs only as the user it cleans, never as root")
	}
	c := &diskClean{t: t, home: t.TempDir(), tmp: t.TempDir(), bin: t.TempDir()}
	c.runnerDir = filepath.Join(c.home, "actions-runner")
	c.lane2 = filepath.Join(c.home, "actions-runner-2")
	return c
}

// make is CleanScriptTests.make(path, hours_old=0, content="x"): a file holding `content` (a
// folder for "dir"), last changed `hoursOld` ago.
func (c *diskClean) make(path string, hoursOld float64, content ...string) string {
	c.t.Helper()
	text := "x"
	if len(content) > 0 {
		text = content[0]
	}
	herr.Must(os.MkdirAll(filepath.Dir(path), 0o777))
	if text == "dir" {
		if err := os.Mkdir(path, 0o777); err != nil && !errors.Is(err, os.ErrExist) {
			c.t.Fatal(err)
		}
	} else if !diskExists(path) {
		herr.Must(os.WriteFile(path, []byte(text), 0o666))
	}
	when := time.Now().Add(-time.Duration(hoursOld * float64(time.Hour)))
	herr.Must(os.Chtimes(path, when, when))
	return path
}

// runScript is CleanScriptTests.run_script(*args, runner_temp=None): clean.sh with a pgrep that
// answers with the jobs this test says are running.
func (c *diskClean) runScript(runnerTemp string, args ...string) string {
	c.t.Helper()
	// A pgrep that answers with the jobs this test says are running, as pgrep -a does: one line
	// each, exit 1 for none, 2 when it cannot tell.
	pgrep := filepath.Join(c.bin, "pgrep")
	lines := strings.TrimSpace(c.workers)
	status := 1
	if lines != "" {
		status = 0
	}
	if c.pgrepStatus != nil {
		status = *c.pgrepStatus
	}
	body := "#!/bin/bash\n"
	if lines != "" {
		body += "cat <<'EOF'\n" + lines + "\nEOF\n"
	}
	body += fmt.Sprintf("exit %d\n", status)
	herr.Must(os.WriteFile(pgrep, []byte(body), 0o666))
	herr.Must(os.Chmod(pgrep, 0o755))
	env := []string{"HOME=" + c.home, "PATH=" + c.bin + ":/usr/bin:/bin", "NIGHT_VM_TMP=" + c.tmp}
	if runnerTemp != "" {
		env = append(env, "RUNNER_TEMP="+runnerTemp)
	}
	cmd := exec.Command("bash", append([]string{diskScript}, args...)...)
	cmd.Env = env
	var stdout, stderr strings.Builder
	cmd.Stdout, cmd.Stderr = &stdout, &stderr
	done := make(chan error, 1)
	if err := cmd.Start(); err != nil {
		c.t.Fatal(err)
	}
	go func() { done <- cmd.Wait() }()
	select {
	case err := <-done:
		if err != nil { // check=True: CalledProcessError
			c.t.Fatalf("clean.sh %v: %v\n%s%s", args, err, stdout.String(), stderr.String())
		}
	case <-time.After(60 * time.Second):
		_ = cmd.Process.Kill()
		c.t.Fatalf("TimeoutExpired: clean.sh %v", args)
	}
	return stdout.String() + stderr.String()
}

// layOutARunner is CleanScriptTests.lay_out_a_runner(runner).
func (c *diskClean) layOutARunner(runnerDir string) {
	c.t.Helper()
	for _, kind := range []string{"bin", "externals"} {
		c.make(filepath.Join(runnerDir, kind+".2.337.0"), 30, "dir")
		c.make(filepath.Join(runnerDir, kind+".2.336.0"), 50, "dir")
		c.make(filepath.Join(runnerDir, kind+".2.338.0"), 0.1, "dir") // an update in progress
		herr.Must(os.Symlink(filepath.Join(runnerDir, kind+".2.337.0"), filepath.Join(runnerDir, kind)))
	}
	c.make(filepath.Join(runnerDir, "_diag", "Worker_20261001.log"), 72)
	c.make(filepath.Join(runnerDir, "_diag", "Worker_20261004.log"), 1)
	c.make(filepath.Join(runnerDir, "_work", "_update", "runner.tar.gz"), 5)
	c.make(filepath.Join(runnerDir, "_work", "_update"), 5, "dir")
	c.make(filepath.Join(runnerDir, "_work", "_temp", "left-by-a-dead-job", "node_modules.txt"), 3)
	c.make(filepath.Join(runnerDir, "_work", "_temp", "left-by-a-dead-job"), 3, "dir")
}

// diskExists is Path.exists(): through a symbolic link.
func diskExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

// diskIsSymlink is Path.is_symlink().
func diskIsSymlink(path string) bool {
	info, err := os.Lstat(path)
	return err == nil && info.Mode()&os.ModeSymlink != 0
}

func TestDisk_ItGivesBackOldRunnerVersionsLogsLeftoversAndIdleCaches(t *testing.T) {
	c := newDiskClean(t)
	c.layOutARunner(c.runnerDir)
	store := c.make(filepath.Join(c.home, ".local/share/pnpm/store/v11/index.txt"), 0)
	oldTmp := c.make(filepath.Join(c.tmp, "vitest-old"), 7)
	newTmp := c.make(filepath.Join(c.tmp, "vitest-new"), 1)
	said := c.runScript("")
	workflowsIn(t, "night-vm-clean:", said)
	for _, kind := range []string{"bin", "externals"} {
		workflowsTrue(t, diskExists(filepath.Join(c.runnerDir, kind+".2.337.0")), "the current version stays")
		workflowsTrue(t, diskExists(filepath.Join(c.runnerDir, kind+".2.338.0")), "an update's stays")
		wasteFalse(t, diskExists(filepath.Join(c.runnerDir, kind+".2.336.0")), "the replaced one goes")
		workflowsTrue(t, diskIsSymlink(filepath.Join(c.runnerDir, kind)))
	}
	wasteFalse(t, diskExists(filepath.Join(c.runnerDir, "_diag", "Worker_20261001.log")))
	workflowsTrue(t, diskExists(filepath.Join(c.runnerDir, "_diag", "Worker_20261004.log")))
	wasteFalse(t, diskExists(filepath.Join(c.runnerDir, "_work", "_update")))
	wasteFalse(t, diskExists(filepath.Join(c.runnerDir, "_work", "_temp", "left-by-a-dead-job")))
	wasteFalse(t, diskExists(store), "no job is running, so the package store goes")
	wasteFalse(t, diskExists(oldTmp))
	workflowsTrue(t, diskExists(newTmp))
}

func TestDisk_ARunningJobKeepsItsRunnerTempAndTheSharedStore(t *testing.T) {
	c := newDiskClean(t)
	c.layOutARunner(c.runnerDir)
	c.layOutARunner(c.lane2)
	store := c.make(filepath.Join(c.home, ".local/share/pnpm/store/v11/index.txt"), 0)
	// Lane 2 is running a job; this run is no job of its own (the disk timer).
	c.workers = "4242 " + c.lane2 + "/bin.2.337.0/Runner.Worker spawnclient 1 2"
	c.runScript("")
	workflowsTrue(t, diskExists(store), "another job's install reads the store")
	workflowsTrue(t, diskExists(filepath.Join(c.lane2, "_work", "_temp", "left-by-a-dead-job")))
	wasteFalse(t, diskExists(filepath.Join(c.runnerDir, "_work", "_temp", "left-by-a-dead-job")))
}

func TestDisk_AfterAJobItsOwnFilesGoAndTheStoreGoesOnlyWithTheLastJob(t *testing.T) {
	c := newDiskClean(t)
	temp := filepath.Join(c.runnerDir, "_work", "_temp")
	c.make(filepath.Join(temp, "work", "item-12", "node_modules.txt"), 0)
	store := c.make(filepath.Join(c.home, ".local/share/pnpm/store/v11/index.txt"), 0)
	c.workers = "11 " + c.runnerDir + "/bin/Runner.Worker spawnclient 1 2\n" +
		"12 " + c.lane2 + "/bin/Runner.Worker spawnclient 3 4"
	c.runScript(temp, "--job")
	entries, err := os.ReadDir(temp)
	if err != nil {
		t.Fatal(err)
	}
	workflowsEqual(t, len(entries), 0)
	workflowsTrue(t, diskExists(store), "lane 2 still has a job going")
	c.workers = "11 " + c.runnerDir + "/bin/Runner.Worker spawnclient 1 2"
	c.runScript(temp, "--job")
	wasteFalse(t, diskExists(store), "the user's last job takes the store with it")
}

func TestDisk_WhenItCannotTellWhichJobsRunItLeavesTheirFilesAlone(t *testing.T) {
	c := newDiskClean(t)
	c.layOutARunner(c.runnerDir)
	store := c.make(filepath.Join(c.home, ".local/share/pnpm/store/v11/index.txt"), 0)
	two := 2
	c.pgrepStatus = &two
	c.runScript("")
	workflowsTrue(t, diskExists(store))
	workflowsTrue(t, diskExists(filepath.Join(c.runnerDir, "_work", "_temp", "left-by-a-dead-job")))
	wasteFalse(t, diskExists(filepath.Join(c.runnerDir, "bin.2.336.0")), "no job uses a replaced version")
}

func TestDisk_SetupInstallsItAsTheHookAndTheTimer(t *testing.T) {
	newDiskClean(t) // the class's skip
	setup := workflowsFile(t, filepath.Join(testsupport.Root, "bot", "machine", "setup.sh"))
	workflowsIn(t, `install -m 755 "$here/clean.sh" /usr/local/bin/night-vm-clean.sh`, setup)
	workflowsIn(t, "/usr/local/bin/night-vm-clean.sh --job || true", setup)
	workflowsIn(t, "night-vm-disk.timer", setup)
	workflowsIn(t, "OnUnitActiveSec=10min", setup)
	onMachine := workflowsFile(t, filepath.Join(testsupport.Root, "bot", "machine", "on-machine.sh"))
	workflowsIn(t, `tar czf - -C "$(dirname "$script")"`, onMachine)
}
