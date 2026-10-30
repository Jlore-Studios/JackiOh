// journal_test.go is bot/tests/test_journal.py: #342: every run leaves its section in the item's
// journal, the next run picks up from it, and a job that is cancelled or killed hands its work on
// instead of failing the item.
//
// Before this, a run's handoff was one slot in the state file (the next run overwrote it, a
// finished item dropped it, and nobody could read it from the pull request), it kept the last
// 8,000 characters of the notes (the end of the log, not the plan or the state at the top), and a
// cancelled job wrote its first status, `failed`, with no work kept: a strike against the item.
package tests

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"os/signal"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/journal"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// NOTES
const journalNotes = "## State\n\nRules v2 is half written; the Coin is next. Tried a table of rules: too " +
	"slow, dropped.\n\n## Log\n\n- about to write the rules: the issue asks for v2\n" +
	"- wrote half of them\n"

// ------------------------------------------------------------------ the file's helpers

// journalText is journal_text(h, number=12).
func journalText(h *flowHarness, number ...int) string {
	n := 12
	if len(number) > 0 {
		n = number[0]
	}
	text, _ := h.gh.GetFile(journal.PathFor(n), journal.Branch)
	return text
}

// journalCalls is calls(runner, role).
func journalCalls(r *runner.FakeRunner, role string) []*runner.RunRequest {
	out := []*runner.RunRequest{}
	for _, c := range r.Calls {
		if c.Role == role {
			out = append(out, c)
		}
	}
	return out
}

// journalStep is next(s for s in result["steps"] if pick(s)).
func journalStep(t *testing.T, result *pyjson.Object, pick func(*pyjson.Object) bool) *pyjson.Object {
	t.Helper()
	for _, s := range pyjson.Objs(result.Get("steps")) {
		if pick(s) {
			return s
		}
	}
	t.Fatal("StopIteration: no such step")
	return nil
}

// ------------------------------------------------------------------ CompactTests

func TestJournal_ShortNotesStayWhole(t *testing.T) {
	assertEqual(t, journal.Compact(journalNotes, 8000), journalNotes)
}

func TestJournal_TheStateStaysWholeAndTheLogKeepsItsEnd(t *testing.T) {
	var log strings.Builder
	for n := range 2000 {
		fmt.Fprintf(&log, "- step %d: did a thing for a reason\n", n)
	}
	notes := "# Plan\n\nDo the rules.\n\n## State\n\nHalf done.\n\n## Log\n\n" + log.String()
	cut := journal.Compact(notes, 4000)
	assertTrue(t, py.Len(cut) <= 4000)
	assertTrue(t, strings.HasPrefix(cut, "# Plan\n\nDo the rules.\n\n## State\n\nHalf done."))
	assertIn(t, journal.Cut, cut)
	assertTrue(t, strings.HasSuffix(strings.TrimRight(cut, " \t\n\r\v\f"), "- step 1999: did a thing for a reason"))
	assertNotIn(t, "- step 0:", cut)
}

func TestJournal_AStateTooLongKeepsItsStart(t *testing.T) {
	notes := "## State\n\n" + strings.Repeat("s", 10_000) + "\n\n## Log\n\n" + "- the last line\n"
	cut := journal.Compact(notes, 4000)
	assertTrue(t, py.Len(cut) <= 4000)
	assertIn(t, "(the rest of the state was cut here)", cut)
	assertTrue(t, strings.HasSuffix(strings.TrimRight(cut, " \t\n\r\v\f"), "- the last line"))
}

func TestJournal_NotesWithoutALogKeepTheirEndAsBefore(t *testing.T) {
	notes := strings.Repeat("x", 5000) + "the end"
	assertEqual(t, journal.Compact(notes, 100), py.SliceFrom(notes, py.Len(notes)-100))
}

// ------------------------------------------------------------------ FileTests

// journalSection is FileTests.section(n, notes="notes").
func journalSection(n int, notes ...string) string {
	note := "notes"
	if len(notes) > 0 {
		note = notes[0]
	}
	return journal.Section(
		pyjson.New("status", "approved", "reason", fmt.Sprintf("run %d", n), "steps", []any{
			pyjson.New("at", "2026-10-06T01:02:03Z", "step", "call", "role", "build", "model", "opus",
				"why", "build | the issue", "minutes", 4.2, "outcome", "done")},
			"handoff", pyjson.New("notes", note)),
		"claude-3", "`opus`", "build", 12,
		fmt.Sprintf("2026-10-06T0%d:00:00Z", n%10), fmt.Sprintf("https://example/runs/%d", n))
}

func TestJournal_ASectionSaysWhatTheRunDidAndWhy(t *testing.T) {
	text := journalSection(1, "## State\nhalf\n")
	assertTrue(t, strings.HasPrefix(text, journal.RunMarker+"\n## 2026-10-06 01:00 UTC · build #12"))
	assertIn(t, "[run](https://example/runs/1) on `claude-3` (`opus`). It ended "+
		"**approved**: run 1", text)
	assertIn(t, "| 1 | 01:02 | build (`opus`) | build \\| the issue | 4.2 min | done |", text)
	assertIn(t, "<summary>The builder's notes when it stopped</summary>", text)
	assertIn(t, "## State\nhalf", text)
}

func TestJournal_AKilledRunReadsAsDied(t *testing.T) {
	text := journal.Section(pyjson.New("status", "interrupted", "interrupt", "died", "reason", "killed"),
		"muse", "`muse`", "revise", 40, "2026-10-06T01:00:00Z", "")
	assertIn(t, "· revise #40 · `muse` · died", text)
}

func TestJournal_AppendsMakeTheBranchThenAddToTheFile(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	journal.Append(gh, 12, journalSection(1), noSleep)
	assertIn(t, journal.Branch, gh.Branches)
	// Vercel reads the pushed commit's settings: the branch must say "no deployments".
	vercel, _ := gh.GetFile("vercel.json", journal.Branch)
	assertIn(t, `"deploymentEnabled": false`, vercel)
	journal.Append(gh, 12, journalSection(2), noSleep)
	text, _ := gh.GetFile("12.md", journal.Branch)
	assertTrue(t, strings.HasPrefix(text, "# Journal of #12"))
	assertEqual(t, len(journal.RunsIn(text)), 2)
	assertTrue(t, strings.Index(text, "run 1") < strings.Index(text, "run 2"))
}

func TestJournal_AWriteThatLostARaceReadsAgain(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	journal.Append(gh, 12, journalSection(1), noSleep)
	realPut := gh.Own().PutFile
	raced := []bool{}
	gh.Patch.PutFile = func(path, text, branch, sha, message string) {
		if len(raced) == 0 { // another run's section lands first
			raced = append(raced, true)
			other, otherSHA := gh.GetFile(path, branch)
			realPut(path, other+"\n"+journalSection(3), branch, otherSHA, "other")
			herr.Raise(&herr.GitHubError{Msg: "conflict", Status: 409})
		}
		realPut(path, text, branch, sha, message)
	}
	journal.Append(gh, 12, journalSection(2), noSleep)
	text, _ := gh.GetFile("12.md", journal.Branch)
	assertIn(t, "run 3", text)
	assertIn(t, "run 2", text)
}

func TestJournal_ALongFileDropsOldNotesThenOldRuns(t *testing.T) {
	sections := []string{}
	for n := range 20 {
		sections = append(sections, journalSection(n, strings.Repeat("n", 5000)))
	}
	text := journal.Header(12) + "\n" + strings.Join(sections, "\n")
	trimmed := journal.Trim(text, 60_000)
	assertTrue(t, py.Len(trimmed) <= 60_000)
	assertEqual(t, len(journal.RunsIn(trimmed)), 20) // every run's lines stay
	parts := strings.Split(trimmed, journal.RunMarker)
	assertNotIn(t, strings.Repeat("n", 5000), parts[1])
	assertIn(t, strings.Repeat("n", 5000), parts[len(parts)-1]) // the newest keep them
	tiny := journal.Trim(text, 2_000)
	assertTrue(t, len(journal.RunsIn(tiny)) < 20)
	assertIn(t, "older run(s) dropped from this file", tiny)
}

func TestJournal_ReadingKeepsTheEndFromARun(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	for n := range 30 {
		journal.Append(gh, 12, journalSection(n, strings.Repeat("n", 3000)), noSleep)
	}
	whole, _ := gh.GetFile("12.md", journal.Branch)
	read := journal.Read(gh, 12, 20_000)
	assertTrue(t, py.Len(read) < py.Len(whole))
	assertIn(t, "earlier runs are on the branch", read)
	assertIn(t, "run 29", read)
	after := strings.TrimLeft(strings.SplitN(read, journal.RunMarker, 2)[1], " \t\n\r\v\f")
	assertEqual(t, py.SliceTo(after, 3), "## ")
	assertEqual(t, journal.Read(gh, 99, journal.ReadChars), "")
}

// ------------------------------------------------------------------ OpusLaneTests
//
// The Claude accounts' runs, on GitHub's runners: a run cut off at its usage cap writes the
// journal, and the next one starts from it.

func TestJournal_ACutOffRunJournalsAndTheNextPicksUp(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:hard"}})
	first := workFake(map[string]runner.Handler{
		"build":  workBuilder(map[string]string{"src/game.txt": "half\n", work.NotesFile: journalNotes}),
		"review": wipCapped})
	planned, result := h.night(first)
	assertEqual(t, []any{planned.Get("provider"), planned.Get("runs_on"), result.Get("status")},
		[]any{"claude-3", "ubuntu-latest", "interrupted"})
	build := journalStep(t, result, func(s *pyjson.Object) bool { return pyjson.Equal(s.Get("role"), "build") })
	assertEqual(t, build.Get("model"), "opus")
	assertTrue(t, strings.HasPrefix(build.Str("why"), "build the issue"))
	assertIn(t, "committed", build.Str("outcome"))
	review := journalStep(t, result, func(s *pyjson.Object) bool { return pyjson.Equal(s.Get("role"), "review") })
	assertIn(t, "cut off at its usage cap", review.Str("outcome"))
	checks := journalStep(t, result, func(s *pyjson.Object) bool { return pyjson.Equal(s.Get("step"), "checks") })
	assertIn(t, "green", checks.Str("outcome"))
	steps := pyjson.Objs(result.Get("steps"))
	assertEqual(t, steps[len(steps)-1].Get("step"), "stop")

	text := journalText(h)
	assertEqual(t, len(journal.RunsIn(text)), 1)
	assertIn(t, "build #12 · `claude-3` · interrupted", text)
	assertIn(t, "| review (`opus`) | review of round 1", text)
	assertIn(t, "Tried a table of rules: too slow, dropped.", text)
	comments := h.gh.BotComments(12)
	assertIn(t, "blob/bot-journal/12.md", comments[len(comments)-1]) // the pause links it

	seen := map[string]string{}
	pickingUp := func(request *runner.RunRequest) *runner.RunResult {
		seen["journal"] = readText(t, filepath.Join(request.Cwd, journal.JournalFile))
		seen["file"] = readText(t, filepath.Join(request.Cwd, "src", "game.txt"))
		return workBuilder(map[string]string{"src/game.txt": "whole\n"})(request)
	}
	second := workFake(map[string]runner.Handler{"build": pickingUp, "review": workReviewer(workApprove)})
	planned, result = h.night(second)
	assertIn(t, "Tried a table of rules", planned.Str("journal"))
	assertIn(t, "Tried a table of rules", seen["journal"])
	assertEqual(t, seen["file"], "half\n") // and its code
	prompt := journalCalls(second, "build")[0].Prompt
	assertIn(t, "## The journal of earlier runs", prompt)
	assertIn(t, "build #12 · `claude-3` · interrupted", prompt)
	// A resume reads the short State first, the whole journal only for detail (#695).
	assertIn(t, "Read the `## State` section of `.bot-notes.md` first", prompt)
	assertIn(t, "not the whole file up front", prompt)
	assertNotIn(t, "Read it before anything else", prompt)
	assertIn(t, "bring the `## State` part of `.bot-notes.md` up to date", prompt)
	assertIn(t, "`## State`, rewritten in place", prompt) // the two-part notes ask
	assertEqual(t, result.Get("status"), "approved")
	tree := fakes.Git(h.origin, "ls-tree", "-r", "--name-only", result.Str("head"))
	assertNotIn(t, journal.JournalFile, tree)
	assertNotIn(t, work.NotesFile, tree)

	text = journalText(h)
	assertEqual(t, len(journal.RunsIn(text)), 2)
	assertIn(t, "· approved", journal.RunsIn(text)[1])
	pr := h.gh.ListPulls("open", "bot/issue-12", 200)[0]
	assertIn(t, "blob/bot-journal/12.md", pr.Str("body"))
}

// The journal is a record, never a gate: a read that fails in the claim and a write that fails
// in deliver leave the run as it would have been.
func TestJournal_AJournalThatFailsNeverStopsTheRun(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:hard"}})
	real := h.gh.Own().GetFile
	h.gh.Patch.GetFile = func(path, ref string) (string, string) {
		if ref == journal.Branch {
			panic(errors.New("the Contents API is down")) // RuntimeError
		}
		return real(path, ref)
	}
	planned, result := h.night(workFake(map[string]runner.Handler{
		"build":  workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)}))
	assertNotIn(t, "journal", planned)
	assertEqual(t, result.Get("status"), "approved")
	assertTrue(t, len(h.gh.ListPulls("open", "bot/issue-12", 200)) > 0)
}

func TestJournal_APullRequestsRunsGoToItsIssuesJournal(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:hard"}})
	h.night(workFake(map[string]runner.Handler{
		"build":  workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)}))
	pr := h.gh.ListPulls("open", "bot/issue-12", 200)[0].Int("number")
	h.gh.AddLabels(pr, []string{config.LabelRevise}) // a person asked for another pass
	planned, _ := h.night(workFake(map[string]runner.Handler{
		"revise": workBuilder(map[string]string{"src/game.txt": "v3\n"}),
		"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("action"), planned.Get("issue_number")}, []any{"revise", 12})
	assertIn(t, "build #12", planned.Str("journal"))
	runs := journal.RunsIn(journalText(h))
	assertEqual(t, len(runs), 2)
	assertIn(t, fmt.Sprintf("revise #%d", pr), runs[1])
	other, _ := h.gh.GetFile(journal.PathFor(pr), journal.Branch)
	assertEqual(t, other, "") // None
}

// ------------------------------------------------------------------ MachineLaneTests
//
// The subscriptions on the bot's machine (Muse here): the same journal, read from the plan, since
// the machine holds no GitHub credential, and ignored in the clone it keeps.

func TestJournal_AMachineRunReadsAndWritesTheJournal(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	earlier := journal.Section(
		pyjson.New("status", "interrupted", "interrupt", "usage", "reason", "claude-5 stops mid-call",
			"handoff", pyjson.New("notes", journalNotes)),
		"claude-5", "`opus`", "build", 12, "2026-09-29T09:00:00Z", "")
	journal.Append(h.gh, 12, earlier, noSleep)
	seen := map[string]string{}
	building := func(request *runner.RunRequest) *runner.RunResult {
		seen["journal"] = readText(t, filepath.Join(request.Cwd, journal.JournalFile))
		return workBuilder(map[string]string{"src/game.txt": "v2\n"})(request)
	}
	r := workFake(map[string]runner.Handler{"build": building, "review": workReviewer(workApprove)})
	planned, _ := h.night(r)
	assertEqual(t, []any{planned.Get("provider"), planned.Get("runs_on")}, []any{"muse", "night-vm-muse"})
	assertIn(t, "claude-5 stops mid-call", seen["journal"])
	prompt := journalCalls(r, "build")[0].Prompt
	assertIn(t, "1 earlier run(s) worked on this", prompt)
	assertIn(t, "`## Log`, added to", prompt) // the notes ask reaches every CLI's prompt
	exclude := fakes.Git(h.clone, "rev-parse", "--git-common-dir")
	if !filepath.IsAbs(exclude) {
		exclude = filepath.Join(h.clone, exclude)
	}
	ignored := py.Splitlines(readText(t, filepath.Join(exclude, "info", "exclude")))
	assertIn(t, "/"+journal.JournalFile, ignored)
	assertIn(t, "/"+work.NotesFile, ignored)
	runs := journal.RunsIn(journalText(h))
	assertEqual(t, len(runs), 2)
	assertIn(t, "`muse`", runs[1])
}

// ------------------------------------------------------------------ KilledJobTests

// GitHub cancels with SIGINT: the run commits and bundles what it has and pauses.
func TestJournal_ACancelledJobIsAPauseThatKeepsItsWork(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	cancelled := func(request *runner.RunRequest) *runner.RunResult {
		writeText(t, filepath.Join(request.Cwd, "src", "game.txt"), "half\n")
		if err := syscall.Kill(os.Getpid(), syscall.SIGINT); err != nil {
			t.Fatal(err)
		}
		time.Sleep(5 * time.Second) // the handler stops the call before this ends
		return runner.NewRunResult(true, workDone)
	}
	_, result := h.night(workFake(map[string]runner.Handler{"build": cancelled}))
	// Put back: Python's handler is signal.default_int_handler again; Go relays SIGINT to no one
	// and no longer ignores it.
	assertFalse(t, signal.Ignored(syscall.SIGINT))
	assertEqual(t, []any{result.Get("status"), result.Get("interrupt")}, []any{"interrupted", "died"})
	assertIn(t, "SIGINT", result.Str("reason"))
	assertEqual(t, fakes.Git(h.origin, "show", h.originSHA("bot/issue-12")+":src/game.txt"), "half")
	record := h.ctx.Store.Load().Obj("items").Obj("12")
	assertFalse(t, record.Truthy("failures"))
	assertFalse(t, record.Truthy("strikes"))
	assertIn(t, config.LabelBuild, h.gh.LabelNames(12)) // queued again
	assertNotIn(t, config.LabelWorking, h.gh.LabelNames(12))
	comments := h.gh.BotComments(12)
	assertIn(t, "Paused: the job was stopped (SIGINT)", comments[len(comments)-1])
	assertIn(t, "· died", journal.RunsIn(journalText(h))[0])
}

// journalCopyTree is shutil.copytree(src, dst): dst must not exist.
func journalCopyTree(src, dst string) {
	if _, err := os.Lstat(dst); err == nil {
		panic(&fakes.PyError{Type: "FileExistsError", Msg: "[Errno 17] File exists: " + py.Repr(dst)})
	}
	err := filepath.WalkDir(src, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		rel, err := filepath.Rel(src, path)
		if err != nil {
			return err
		}
		target := filepath.Join(dst, rel)
		info, err := d.Info()
		if err != nil {
			return err
		}
		if d.IsDir() {
			return os.MkdirAll(target, info.Mode().Perm()|0o700)
		}
		data, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		return os.WriteFile(target, data, info.Mode().Perm())
	})
	if err != nil {
		panic(err)
	}
}

// A job killed outright never reaches its own end: deliver gets the checkpoint the work job wrote
// after its last step, and keeps the work.
func TestJournal_AKilledJobLeavesItsLastCheckpoint(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	out, killed := filepath.Join(h.root, "out-0"), filepath.Join(h.root, "killed")
	killedDuringReview := func(request *runner.RunRequest) *runner.RunResult {
		journalCopyTree(out, killed) // what the upload step finds if the job dies now
		return runner.NewRunResult(true, workApprove)
	}
	r := workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": killedDuringReview})
	work.NewWorker(h.cfg, planned, r, h.clone, filepath.Join(h.root, "work"), out, nil, nil, nil, nil, nil, nil).Run()
	checkpoint := loadJSON(t, filepath.Join(killed, "result.json"))
	assertEqual(t, []any{checkpoint.Get("status"), checkpoint.Get("interrupt"), checkpoint.Get("checkpoint")},
		[]any{"interrupted", "died", true})
	assertIn(t, "its last checkpoint, after 2 model call(s)", checkpoint.Str("reason"))
	info, err := os.Stat(filepath.Join(killed, checkpoint.Str("bundle")))
	assertTrue(t, err == nil && info.Mode().IsRegular())
	steps := pyjson.Objs(checkpoint.Get("steps"))
	assertEqual(t, steps[len(steps)-1].Get("step"), "checks")
	final := loadJSON(t, filepath.Join(out, "result.json"))
	assertEqual(t, final.Get("status"), "approved") // the run's own end writes over it
	assertNotIn(t, "checkpoint", final)

	deliver.NewDeliverer(h.ctx, planned, killed, h.deliverRepo, "", nil, "").Run()
	assertEqual(t, fakes.Git(h.origin, "show", h.originSHA("bot/issue-12")+":src/game.txt"), "v2")
	assertIn(t, config.LabelBuild, h.gh.LabelNames(12)) // queued again
	assertNotIn(t, config.LabelWorking, h.gh.LabelNames(12))
	assertFalse(t, len(h.gh.ListPulls("open", "bot/issue-12", 200)) > 0) // no pull request from it
	record := h.ctx.Store.Load().Obj("items").Obj("12")
	assertFalse(t, record.Truthy("failures"))
	interruptions, _ := pyjson.AsInt(record.GetOr("interruptions", 0))
	assertEqual(t, interruptions, 0) // its work moved on
	comments := h.gh.BotComments(12)
	assertIn(t, "Paused: the model job stopped before it finished", comments[len(comments)-1])
}

// The stop a signal raises can land inside a reading's `except Exception` (memory, the disk): it
// is kept, and the next checkpoint or call raises it again.
func TestJournal_ACancelSomethingSwallowedStillStopsTheRun(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	worker := work.NewWorker(h.cfg, plan.Make(h.ctx, false, nil, "auto", nil), workFake[runner.Handler](nil), h.clone,
		filepath.Join(h.root, "work"), filepath.Join(h.root, "out-0"), nil, nil, nil, nil, nil, nil)
	worker.Signalled = "the job was stopped (SIGTERM): cancelled, or past its time limit"
	stopped := assertRaises[*work.Interrupt](t, func() { worker.Check() })
	if stopped != nil {
		assertEqual(t, stopped.Kind, "died")
	}
	assertRaises[*work.Interrupt](t, func() { worker.Call("build", "go", h.clone, false, "", false) })
	assertEqual(t, worker.Calls, 0) // no call started after the cancel
}

func TestJournal_AKilledJobThatGotNowhereCounts(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	out := filepath.Join(h.root, "out-0")
	work.NewWorker(h.cfg, planned, workFake(map[string]runner.Handler{
		"build": workBuilder(map[string]string{}), "review": workReviewer(workApprove)}),
		h.clone, filepath.Join(h.root, "work"), out, nil, nil, nil, nil, nil, nil).Run()
	result := loadJSON(t, filepath.Join(out, "result.json"))
	result.Update(pyjson.New("status", "interrupted", "interrupt", "died", "reason", "killed", "head", nil))
	writeText(t, filepath.Join(out, "result.json"), pyjson.Dumps(result))
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	interruptions, _ := pyjson.AsInt(h.ctx.Store.Load().Obj("items").Obj("12").Get("interruptions"))
	assertEqual(t, interruptions, 1)
	assertNotIn(t, config.LabelPR, h.gh.LabelNames(12))
}
