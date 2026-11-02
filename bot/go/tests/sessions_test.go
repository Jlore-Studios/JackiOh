// A Claude run's own session, kept sealed in its run's `work` artifact with a pointer in the
// state, and resumed by the item's next run on the same subscription, with the handoff as the
// fallback (#2748, section 9).
package tests

import (
	"archive/zip"
	"bytes"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/journal"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/sessions"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/vault"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// sessionsID is the session Claude Code names in the scripted calls.
const sessionsID = "0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0"

// sessionsSecret is claude-1's secret in these tests: its vault key comes from it.
const sessionsSecret = "claude-one-secret-0123456789"

// sessionsToken is a token a tool printed during the first run, which the kept session must not
// hold.
var sessionsToken = "ghp_" + strings.Repeat("s", 36)

// sessionsFirstLine is what the first run's session holds.
var sessionsFirstLine = `{"type":"user","session_id":"` + sessionsID + `","message":"the first run read ` +
	`src/game.txt and printed ` + sessionsToken + `"}`

// sessionsWorld is one issue (#12) on the night bot's only subscription with its secret here,
// claude-1, and the night runs on it, each its own workflow run on a fresh runner.
type sessionsWorld struct {
	t      *testing.T
	h      *flowHarness
	runs   int
	nights int
}

func newSessionsWorld(t *testing.T) *sessionsWorld {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelBuild)})
	return &sessionsWorld{t: t, h: h}
}

// keeper is a fresh runner's Sessions on `provider` under `secret`: a config directory of its
// own, and the runs' artifacts as the fake GitHub serves them.
func (w *sessionsWorld) keeper(provider, secret string) *work.Sessions {
	w.runs++
	dir := filepath.Join(w.h.root, fmt.Sprintf("claude-config-%d", w.runs))
	return work.NewSessions(w.h.cfg, w.h.cfg.Pool.Get(provider), secret, dir,
		func(record *pyjson.Object) (string, error) { return sessions.Fetch(w.h.gh, record) })
}

// sessionsZip is a directory's files as a zip, named from the directory, as upload-artifact
// packs `$RUNNER_TEMP/out`.
func sessionsZip(t *testing.T, dir string, extra map[string]string) []byte {
	t.Helper()
	var buf bytes.Buffer
	archive := zip.NewWriter(&buf)
	add := func(name string, data []byte) {
		f, err := archive.Create(name)
		if err != nil {
			t.Fatal(err)
		}
		if _, err := f.Write(data); err != nil {
			t.Fatal(err)
		}
	}
	if dir != "" {
		if err := filepath.WalkDir(dir, func(path string, entry fs.DirEntry, err error) error {
			if err != nil || entry.IsDir() {
				return err
			}
			rel, _ := filepath.Rel(dir, path)
			data, err := os.ReadFile(path)
			if err == nil {
				add(filepath.ToSlash(rel), data)
			}
			return err
		}); err != nil {
			t.Fatal(err)
		}
	}
	for name, text := range extra {
		add(name, []byte(text))
	}
	if err := archive.Close(); err != nil {
		t.Fatal(err)
	}
	return buf.Bytes()
}

// upload is the work job's upload step: `data` as run `runID`'s `work` artifact.
func (w *sessionsWorld) upload(runID string, data []byte) int {
	id := 500 + len(w.h.gh.ArtifactZips)
	w.h.gh.Artifacts[runID] = append(w.h.gh.Artifacts[runID], pyjson.New("id", id, "name", "work",
		"expired", false, "size_in_bytes", len(data)))
	w.h.gh.ArtifactZips[id] = data
	return id
}

// night plans a run, works it with `s`, uploads its output as the run's `work` artifact and
// delivers it, as a night run's three jobs do. Returns the plan, the run's result and its output
// directory.
func (w *sessionsWorld) night(r *runner.FakeRunner, s *work.Sessions) (*pyjson.Object, *pyjson.Object, string) {
	w.t.Helper()
	w.nights++
	w.h.cfg.RunID = strconv.Itoa(9000 + w.nights) // the plan, the work and deliver all name it
	planned := plan.Make(w.h.ctx, false, nil, "auto", nil)
	assertEqual(w.t, []any{planned.Get("action"), planned.Get("provider")}, []any{"build", "claude-1"})
	out := filepath.Join(w.h.root, fmt.Sprintf("out-%d", flowOuts(w.h.root)))
	worker := work.NewWorker(w.h.cfg, planned, r, w.h.clone, filepath.Join(w.h.root, "work"), out,
		nil, nil, nil, nil, nil, nil)
	worker.Sessions = s
	worker.Run()
	w.upload(w.h.cfg.RunID, sessionsZip(w.t, out, nil))
	deliver.NewDeliverer(w.h.ctx, planned, out, w.h.deliverRepo, "", nil, "").Run()
	return planned, loadJSON(w.t, filepath.Join(out, "result.json")), out
}

// committed is whatever the state branch holds under `sessions/`: always nothing now.
func (w *sessionsWorld) committed() [][2]string {
	return w.h.gh.ListDir(sessions.LegacyDir, config.StateBranch)
}

// first is the item's first run: its builder writes half the change and its notes in session
// `sessionsID`, and its reviewer is cut off at the usage limit, so the run pauses with its work
// and its session kept. The clock then moves past the limit's reset.
func (w *sessionsWorld) first(secret string) (*pyjson.Object, string) {
	s := w.keeper("claude-1", secret)
	r := workFake(map[string]any{
		"build": sessionsBuilder(s, sessionsID, sessionsFirstLine,
			map[string]string{"src/game.txt": "half\n", work.NotesFile: "## State\nnext: the Coin\n"}),
		"review": sessionsLimited})
	_, result, out := w.night(r, s)
	assertEqual(w.t, result.Get("status"), "interrupted")
	testsupport.ClockOf(w.h.ctx).At = testsupport.Night.Add(time.Hour)
	return result, out
}

// record is #`number`'s `session` record in the state file.
func (w *sessionsWorld) record(number int) *pyjson.Object {
	return w.h.ctx.Store.Load().Obj("items").Obj(fmt.Sprint(number)).Obj("session")
}

// journal is #12's journal on the journal branch.
func (w *sessionsWorld) journal() string {
	text, _ := w.h.gh.GetFile(journal.PathFor(12), journal.Branch)
	return text
}

// sessionsAppend adds `line` to session `id`'s file, where Claude Code files a call in `cwd`
// under `s`'s config directory.
func sessionsAppend(s *work.Sessions, cwd, id, line string) {
	path := sessions.Transcript(s.ConfigDir, sessions.Project(cwd), id)
	if err := os.MkdirAll(filepath.Dir(path), 0o700); err != nil {
		panic(err)
	}
	f, err := os.OpenFile(path, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o600)
	if err != nil {
		panic(err)
	}
	defer f.Close()
	if _, err := f.WriteString(line + "\n"); err != nil {
		panic(err)
	}
}

// sessionsBuilder is a Claude builder in session `id`: it writes `edits`, adds `line` to the
// session's file and reports the build done, in that session.
func sessionsBuilder(s *work.Sessions, id, line string, edits map[string]string) runner.Handler {
	return func(request *runner.RunRequest) *runner.RunResult {
		sessionsAppend(s, request.Cwd, id, line)
		result := workBuilder(edits)(request)
		result.SessionID = id
		return result
	}
}

// sessionsLimited is a call refused at the usage limit, which resets half an hour after NIGHT.
func sessionsLimited(request *runner.RunRequest) *runner.RunResult {
	result := runner.NewRunResult(false, "")
	result.ExitCode, result.Error = 1, "You've hit your limit"
	result.ResetAt = clock.ISO(testsupport.Night.Add(30 * time.Minute))
	return result
}

// sessionsBuilds is the run's builder calls.
func sessionsBuilds(r *runner.FakeRunner) []*runner.RunRequest {
	var found []*runner.RunRequest
	for _, call := range r.Calls {
		if call.Role == "build" {
			found = append(found, call)
		}
	}
	return found
}

func TestSessions_AKeptSessionResumesOnTheSameSubscription(t *testing.T) {
	w := newSessionsWorld(t)
	result, out := w.first(sessionsSecret)
	kept := crossObj(t, result, "session", "kept")
	assertEqual(t, []any{kept.Get("id"), kept.Get("kind")}, []any{sessionsID, "build"})
	// Kept sealed, never readable, in the run's `work` artifact; the state holds only a pointer
	// to it, and nothing session-sized is committed to the state branch.
	text := readText(t, filepath.Join(out, sessions.OutFile))
	assertTrue(t, vault.LooksSealedBlob(text, sessions.MaxBytes))
	assertNotIn(t, "the first run read", text)
	assertNotIn(t, sessionsToken, text)
	record := w.record(12)
	artifact := sessions.WorkArtifact(w.h.gh.Artifacts["9001"])
	assertEqual(t, []any{record.Get("id"), record.Get("provider"), record.Get("kind"), record.Get("head"),
		record.Get("run"), record.Get("artifact"), record.Get("artifact_name")},
		[]any{sessionsID, "claude-1", "build", result.Get("head"), "9001", artifact.Get("id"), "work"})
	assertEqual(t, record.Get("bytes"), len(strings.TrimSpace(text)))
	assertIn(t, "interrupted", record.Str("ended"))
	assertEqual(t, len(w.committed()), 0)

	// The next run there, on a fresh runner, restores it where Claude Code files it and goes on
	// in it, told only what it lacks.
	s := w.keeper("claude-1", sessionsSecret)
	restored := ""
	build := func(request *runner.RunRequest) *runner.RunResult {
		restored = mustReadText(sessions.Transcript(s.ConfigDir, sessions.Project(request.Cwd), sessionsID))
		return sessionsBuilder(s, sessionsID, `{"type":"user","message":"the second run"}`,
			map[string]string{"src/game.txt": "whole\n"})(request)
	}
	r := workFake(map[string]any{"build": build, "review": workReviewer(workApprove)})
	planned, second, _ := w.night(r, s)
	assertEqual(t, crossObj(t, planned, "session").Get("id"), sessionsID)
	builds := sessionsBuilds(r)
	assertEqual(t, len(builds), 1)
	assertEqual(t, builds[0].Resume, sessionsID)
	assertIn(t, "## Resuming your own session", builds[0].Prompt)
	assertIn(t, "## What changed since", builds[0].Prompt)
	assertNotIn(t, "## Picking up from another agent", builds[0].Prompt)
	assertIn(t, "the first run read src/game.txt", restored)
	assertIn(t, "[REDACTED]", restored)
	assertNotIn(t, sessionsToken, restored)
	assertEqual(t, second.Get("status"), "approved")
	session := crossObj(t, second, "session")
	assertEqual(t, []any{session.Get("resumed"), session.Get("tried")}, []any{true, true})
	assertFalse(t, session.Has("fallback"))
	assertEqual(t, crossObj(t, session, "from").Get("provider"), "claude-1")
	call := crossObj(t, session, "call")
	assertEqual(t, []any{call.Get("role"), call.Get("ok")}, []any{"build", true})
	assertTrue(t, call.Has("minutes"))
	assertIn(t, "Session: resumed `0f1e2d3c`, kept by the run on `claude-1`", w.journal())
	// The approved build hands the work on to its pull request: the pointer goes.
	assertEqual(t, w.record(12).Len(), 0)
	assertIn(t, "no run of #12 goes on from it", w.journal())
	assertEqual(t, len(w.committed()), 0)
}

func TestSessions_AnExpiredArtifactFallsBackToTheHandoff(t *testing.T) {
	w := newSessionsWorld(t)
	w.first(sessionsSecret)
	// Past its retention GitHub answers the artifact's download with 410 Gone.
	w.h.gh.Patch.DownloadArtifact = func(id any) []byte {
		panic(&herr.GitHubError{Msg: "GET /actions/artifacts/" + fmt.Sprint(id) + "/zip: HTTP 410", Status: 410})
	}
	s := w.keeper("claude-1", sessionsSecret)
	r := workFake(map[string]any{"build": workBuilder(map[string]string{"src/game.txt": "whole\n"}),
		"review": workReviewer(workApprove)})
	_, result, _ := w.night(r, s)
	builds := sessionsBuilds(r)
	assertEqual(t, len(builds), 1)
	assertEqual(t, builds[0].Resume, "")
	assertIn(t, "## Picking up from another agent", builds[0].Prompt)
	assertIn(t, "next: the Coin", builds[0].Prompt)
	assertEqual(t, result.Get("status"), "approved")
	session := crossObj(t, result, "session")
	assertEqual(t, []any{session.Get("resumed"), session.Get("tried")}, []any{false, true})
	assertIn(t, "it could not be had: run 9001's `work` artifact is gone (HTTP 410)", session.Str("fallback"))
	assertIn(t, "since the kept one could not be resumed: it could not be had: run 9001's `work` "+
		"artifact is gone (HTTP 410)", w.journal())
	// So does a pointer from before deliver named the artifact, once the run's listing has none.
	w.h.gh.Patch.DownloadArtifact = nil
	w.h.gh.Artifacts["9001"][0].Set("expired", true)
	_, err := sessions.Fetch(w.h.gh, pyjson.New("run", "9001"))
	assertIn(t, "run 9001 has no `work` artifact left", err.Error())
	_, err = sessions.Fetch(w.h.gh, pyjson.New("path", "sessions/12-claude-1.bin"))
	assertIn(t, "its record names no run whose artifact holds it", err.Error())
}

func TestSessions_ACutOffPlannerGoesOnInItsSessionAndItsBuilderStartsFresh(t *testing.T) {
	w := newSessionsWorld(t)
	s := w.keeper("claude-1", sessionsSecret)
	cutOff := func(request *runner.RunRequest) *runner.RunResult {
		sessionsAppend(s, request.Cwd, sessionsID, `{"type":"user","message":"the planner read the rules"}`)
		mustWriteText(filepath.Join(request.Cwd, work.PlanDraftFile), "1. **Goal.** Rules v2.\n")
		result := sessionsLimited(request)
		result.SessionID = sessionsID
		return result
	}
	_, first, _ := w.night(workFake(map[string]any{"plan": cutOff}), s)
	assertEqual(t, crossObj(t, first, "session", "kept").Get("kind"), "plan")
	assertEqual(t, w.record(12).Get("kind"), "plan")
	testsupport.ClockOf(w.h.ctx).At = testsupport.Night.Add(time.Hour)

	s = w.keeper("claude-1", sessionsSecret)
	planner := func(request *runner.RunRequest) *runner.RunResult {
		result := runner.NewRunResult(true, runner.FakePlan)
		result.SessionID = request.Resume
		return result
	}
	r := workFake(map[string]any{"plan": planner, "build": workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)})
	_, result, _ := w.night(r, s)
	var plans []*runner.RunRequest
	for _, call := range r.Calls {
		if call.Role == "plan" {
			plans = append(plans, call)
		}
	}
	assertEqual(t, len(plans), 1)
	assertEqual(t, plans[0].Resume, sessionsID)
	assertIn(t, "You were planning this and were cut off", plans[0].Prompt)
	assertNotIn(t, "## An unfinished plan from an earlier run", plans[0].Prompt)
	// The builder is a session of its own, as ever; the planner's resume stays on record.
	assertEqual(t, sessionsBuilds(r)[0].Resume, "")
	assertEqual(t, result.Get("status"), "approved")
	session := crossObj(t, result, "session")
	assertEqual(t, []any{session.Get("resumed"), crossObj(t, session, "call").Get("role")}, []any{true, "plan"})
	assertFalse(t, session.Has("fallback"))
}

func TestSessions_AnotherSubscriptionStartsFreshAndDropsTheOneKept(t *testing.T) {
	w := newSessionsWorld(t)
	// The item's last run, on claude-3, kept its session (sealed under claude-3's secret) and
	// handed its work on.
	other := t.TempDir()
	project := sessions.Project("/elsewhere/item-12")
	sessionsAppend(&work.Sessions{ConfigDir: other}, "/elsewhere/item-12", sessionsID, sessionsFirstLine)
	sealed, err := sessions.Seal(other, project, sessionsID, "claude-three-secret-0123456789")
	if err != nil {
		t.Fatal(err)
	}
	artifact := w.upload("8000", sessionsZip(t, "", map[string]string{sessions.OutFile: sealed + "\n"}))
	flowUpdateItem(w.h.ctx, 12, pyjson.New(
		"handoff", pyjson.New("provider", "claude-3", "family", "claude", "kind", "work",
			"reason", "`claude-3` reached its usage limit", "notes", "## State\nnext: the Coin\n", "trail", ""),
		"session", pyjson.New("id", sessionsID, "provider", "claude-3", "model", "opus", "kind", "build",
			"cwd", "/elsewhere/item-12", "project", project, "head", "", "run", "8000",
			"artifact", artifact, "artifact_name", "work")))

	s := w.keeper("claude-1", sessionsSecret)
	r := workFake(map[string]any{
		"build":  sessionsBuilder(s, "1a2b3c4d-0000-1111-2222-333344445555", `{"type":"user"}`, map[string]string{"src/game.txt": "v2\n"}),
		"review": sessionsLimited})
	_, result, _ := w.night(r, s)
	builds := sessionsBuilds(r)
	assertEqual(t, builds[0].Resume, "")
	assertIn(t, "## Picking up from another agent", builds[0].Prompt)
	assertIn(t, "next: the Coin", builds[0].Prompt)
	session := crossObj(t, result, "session")
	assertEqual(t, session.Get("resumed"), false)
	assertFalse(t, session.Has("tried"))
	assertIn(t, "the one kept is `claude-3`'s, and this run is on `claude-1`", session.Str("fallback"))
	// The work changed hands: claude-3's pointer is dropped, and claude-1's own kept instead.
	assertEqual(t, []any{w.record(12).Get("provider"), w.record(12).Get("id"), w.record(12).Get("run")},
		[]any{"claude-1", "1a2b3c4d-0000-1111-2222-333344445555", "9001"})
	assertIn(t, "Session: fresh: the one kept is `claude-3`'s", w.journal())
	assertEqual(t, len(w.committed()), 0)
}

func TestSessions_ASessionThatWillNotOpenFallsBackToTheHandoff(t *testing.T) {
	w := newSessionsWorld(t)
	w.first(sessionsSecret)
	// A fresh login was pasted into claude-1's secret since: the old session no longer opens.
	s := w.keeper("claude-1", "a-fresh-login-pasted-in-0123")
	r := workFake(map[string]any{"build": workBuilder(map[string]string{"src/game.txt": "whole\n"}),
		"review": workReviewer(workApprove)})
	_, result, _ := w.night(r, s)
	builds := sessionsBuilds(r)
	assertEqual(t, len(builds), 1)
	assertEqual(t, builds[0].Resume, "")
	assertIn(t, "## Picking up from another agent", builds[0].Prompt)
	assertIn(t, "next: the Coin", builds[0].Prompt)
	assertEqual(t, result.Get("status"), "approved")
	session := crossObj(t, result, "session")
	assertEqual(t, []any{session.Get("resumed"), session.Get("tried")}, []any{false, true})
	assertIn(t, "it could not be opened: it was sealed under another secret", session.Str("fallback"))
	assertIn(t, "Session: fresh, with the handoff, since the kept one could not be resumed: it could "+
		"not be opened", w.journal())
}

func TestSessions_AResumeTheCLIRefusesGoesOnInAFreshSession(t *testing.T) {
	w := newSessionsWorld(t)
	w.first(sessionsSecret)
	s := w.keeper("claude-1", sessionsSecret)
	refused := func(request *runner.RunRequest) *runner.RunResult {
		result := runner.NewRunResult(false, "")
		result.ExitCode, result.Error = 1, "No conversation found with session ID: "+request.Resume
		return result
	}
	r := workFake(map[string]any{"build": []runner.Handler{refused,
		workBuilder(map[string]string{"src/game.txt": "whole\n"})}, "review": workReviewer(workApprove)})
	_, result, _ := w.night(r, s)
	builds := sessionsBuilds(r)
	assertEqual(t, len(builds), 2)
	assertEqual(t, []any{builds[0].Resume, builds[1].Resume}, []any{sessionsID, ""})
	assertIn(t, "## Resuming your own session", builds[0].Prompt)
	assertIn(t, "## Picking up from another agent", builds[1].Prompt)
	assertNotEqual(t, builds[0].Transcript, builds[1].Transcript)
	assertEqual(t, result.Get("status"), "approved")
	session := crossObj(t, result, "session")
	assertEqual(t, []any{session.Get("resumed"), session.Get("tried")}, []any{false, true})
	assertIn(t, "the CLI could not resume it: No conversation found", session.Str("fallback"))
	assertEqual(t, crossObj(t, session, "call").Get("ok"), false)
	// Both calls are steps of their own: the attempt, and the fresh session that went on.
	var calls []*pyjson.Object
	for _, step := range pyjson.Objs(result.Get("steps")) {
		if step.Get("step") == "call" && step.Get("role") == "build" {
			calls = append(calls, step)
		}
	}
	assertEqual(t, len(calls), 2)
	assertIn(t, "could not resume it: No conversation found", calls[0].Str("outcome"))
	assertIn(t, "in a fresh session, as its own could not be resumed", calls[1].Str("why"))
	assertIn(t, "since the kept one could not be resumed: the CLI could not resume it", w.journal())
}

func TestSessions_ASessionOverTheCapIsNotKept(t *testing.T) {
	old := sessions.MaxBytes
	sessions.MaxBytes = 100
	t.Cleanup(func() { sessions.MaxBytes = old })
	w := newSessionsWorld(t)
	result, out := w.first(sessionsSecret)
	session := crossObj(t, result, "session")
	assertIn(t, "over the 100-byte cap", session.Str("not_kept"))
	assertFalse(t, session.Has("kept"))
	assertFalse(t, exists(filepath.Join(out, sessions.OutFile)))
	assertEqual(t, w.record(12).Len(), 0)
	assertIn(t, "This run's: kept no session: sealed, it is", w.journal())
	assertIn(t, "over the 100-byte cap", w.journal())
}

func TestSessions_TheSwitchTurnsItOff(t *testing.T) {
	on := testsupport.MakeConfig()
	off := testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("resume_sessions", false)})
	assertTrue(t, on.ResumeSessions) // the night bot's `.harness/config.json`
	assertFalse(t, off.ResumeSessions)
	fetch := func(*pyjson.Object) (string, error) { return "", nil }
	assertTrue(t, work.NewSessions(on, on.Pool.Get("claude-1"), "s", "/config", fetch) != nil)
	assertTrue(t, work.NewSessions(off, off.Pool.Get("claude-1"), "s", "/config", fetch) == nil)
	// Only Claude resumes for now.
	assertTrue(t, work.NewSessions(on, on.Pool.Get("muse"), "s", "/config", fetch) == nil)
	assertTrue(t, work.NewSessions(on, on.Pool.Get("claude-1"), "", "/config", fetch) == nil)

	// A run without them keeps nothing and resumes nothing, whatever its plan names.
	w := newWorkWorld(t)
	planned := workBuildPlan()
	planned.Set("session", pyjson.New("id", sessionsID, "provider", "claude-3", "model", "opus", "kind", "build"))
	dir := filepath.Join(w.root, "claude-config")
	r := workFake(map[string]any{
		"build": sessionsBuilder(&work.Sessions{ConfigDir: dir}, sessionsID, `{"type":"user"}`,
			map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)})
	result := w.worker(r, workWorkerOpts{Plan: planned}).Run()
	assertEqual(t, result.Get("status"), "approved")
	assertEqual(t, sessionsBuilds(r)[0].Resume, "")
	assertFalse(t, result.Has("session"))
	assertFalse(t, exists(filepath.Join(w.out, sessions.OutFile)))
}

func TestSessions_AClosedItemsSessionsAreDropped(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(5, fakes.Issue{State: "closed"})
	gh.AddIssue(6)
	pointer := pyjson.New("id", sessionsID, "provider", "claude-1", "run", "9001", "artifact", 500)
	flowUpdate(ctx, func(s *pyjson.Object) {
		for _, n := range []int{5, 6} {
			state.Item(s, n).Set("session", pointer.Copy())
		}
	})
	// Files an earlier draft would have left on the state branch: no run reads them.
	for _, name := range []string{"5-claude-1.bin", "6-claude-1.bin"} {
		gh.PutFile(sessions.LegacyDir+"/"+name, "b1:sealed\n", config.StateBranch, "", "seed")
	}
	// The state file's daily prune clears a closed item's pointer, leaves an open one's, and
	// deletes every session file on the state branch.
	notes := plan.Prune(ctx, ctx.Store.Load())
	assertIn(t, "dropped #5's kept session: #5 is closed", notes)
	assertIn(t, "deleted `sessions/6-claude-1.bin` from the state branch: a kept session lives in its "+
		"run's artifact", notes)
	assertEqual(t, len(gh.ListDir(sessions.LegacyDir, config.StateBranch)), 0)
	items := ctx.Store.Load().Obj("items")
	assertFalse(t, items.Obj("5").Has("session"))
	assertTrue(t, items.Obj("6").Has("session"))

	// A bot pull request that merges drops its own and its issue's at once.
	gh.AddPull(40, "bot/issue-6", fakes.Pull{Labels: []string{config.LabelPR}, Body: "Closes #6"})
	flowUpdateItem(ctx, 40, pyjson.New("session", pointer.Copy()))
	pull := gh.GetPull(40).Copy()
	pull.Update(pyjson.New("merged", true, "state", "closed", "base", pyjson.New("ref", ctx.Cfg.DefaultBranch)))
	events.Handle(ctx, "pull_request_target", pyjson.New("action", "closed", "sender", fakes.Bot,
		"pull_request", pull))
	for _, n := range []int{6, 40} {
		assertFalse(t, ctx.Store.Load().Obj("items").Obj(fmt.Sprint(n)).Has("session"), n)
	}
}

func TestSessions_OnlySessionEncComesOutOfAnArtifact(t *testing.T) {
	sealed := "b1:" + strings.Repeat("a", 64) + ":QUJD"
	text, err := sessions.FromZip(sessionsZip(t, "", map[string]string{sessions.OutFile: sealed + "\n",
		"result.json": "{}", "branch.bundle": "bundle", "transcripts/01-build.jsonl": "{}"}))
	if err != nil {
		t.Fatal(err)
	}
	assertEqual(t, text, sealed)
	for name, files := range map[string]map[string]string{
		"a path outside it": {sessions.OutFile: sealed, "../escape": "x"},
		"an absolute path":  {sessions.OutFile: sealed, "/etc/passwd": "x"},
		"no session.enc":    {"result.json": "{}", "sub/session.enc": sealed, "./session.enc": sealed},
	} {
		_, err := sessions.FromZip(sessionsZip(t, "", files))
		assertTrue(t, err != nil, name)
	}
	_, err = sessions.FromZip([]byte("not a zip"))
	assertIn(t, "the artifact is not a zip", err.Error())
	old := sessions.MaxBytes
	sessions.MaxBytes = 10
	t.Cleanup(func() { sessions.MaxBytes = old })
	_, err = sessions.FromZip(sessionsZip(t, "", map[string]string{sessions.OutFile: sealed}))
	assertIn(t, "over the 10-byte cap", err.Error())
}

func TestSessions_TheClaudeCLIResumesByIDAndNamesItsSession(t *testing.T) {
	dir := t.TempDir()
	args := filepath.Join(dir, "args.txt")
	bin := filepath.Join(dir, "claude")
	script := "#!/bin/sh\ncat > /dev/null\nprintf '%s\\n' \"$@\" > " + runnerQuote(args) + "\n" +
		`echo '{"type": "system", "subtype": "init", "session_id": "` + sessionsID + `"}'` + "\n" +
		`echo '{"type": "result", "is_error": false, "result": "done", "num_turns": 2, "session_id": "` +
		sessionsID + `"}'` + "\n"
	writeText(t, bin, script)
	if err := os.Chmod(bin, 0o755); err != nil {
		t.Fatal(err)
	}
	request := &runner.RunRequest{Role: "build", Prompt: "go on", Cwd: dir, SystemAppend: "system",
		AllowedTools: []string{"Read"}, MaxTurns: 5, TimeoutS: 60, Model: "opus", Effort: "xhigh",
		Transcript: filepath.Join(dir, "t", "01-build.jsonl")}
	assertNotIn(t, "--resume", runner.NewClaudeCli("claude", nil, nil).Argv(request, "/tmp/s.md"))
	request.Resume = sessionsID
	result := runner.NewClaudeCli(bin, nil, nil).Run(request)
	assertTrue(t, result.OK, result.Error)
	assertEqual(t, result.SessionID, sessionsID)
	assertIn(t, "--resume\n"+sessionsID+"\n", readText(t, args))
}

func TestSessions_ASessionIsPackedRedactedAndOnlyItsOwnFilesComeBack(t *testing.T) {
	assertEqual(t, sessions.Project("/home/runner/work/_temp/work/item-12"), "-home-runner-work--temp-work-item-12")
	assertEqual(t, []any{sessions.Family("claude-opus-4-7"), sessions.Family("opus"), sessions.Family("Sonnet")},
		[]any{"opus", "opus", "sonnet"})
	from := t.TempDir()
	project := sessions.Project("/work/item-12")
	sessionsAppend(&work.Sessions{ConfigDir: from}, "/work/item-12", sessionsID, sessionsFirstLine)
	side := filepath.Join(from, "projects", project, sessionsID)
	for _, name := range []string{"tool-results/1.txt", "subagents/agent-1.jsonl"} {
		if err := os.MkdirAll(filepath.Dir(filepath.Join(side, name)), 0o700); err != nil {
			t.Fatal(err)
		}
		writeText(t, filepath.Join(side, name), "kept beside it "+sessionsToken)
	}
	sealed, err := sessions.Seal(from, project, sessionsID, sessionsSecret)
	if err != nil {
		t.Fatal(err)
	}
	assertNotIn(t, "the first run read", sealed)
	to := t.TempDir()
	moved := sessions.Project("/work/elsewhere")
	if err := sessions.Restore(sealed, sessionsSecret, to, moved, sessionsID); err != nil {
		t.Fatal(err)
	}
	transcript := readText(t, sessions.Transcript(to, moved, sessionsID))
	assertIn(t, "the first run read", transcript)
	assertNotIn(t, sessionsToken, transcript)
	assertEqual(t, readText(t, filepath.Join(to, "projects", moved, sessionsID, "tool-results", "1.txt")),
		"kept beside it [REDACTED]")
	assertFalse(t, exists(filepath.Join(to, "projects", moved, sessionsID, "subagents")))
	assertEqual(t, sessions.Find(to, sessionsID), moved)
	// Another secret, another session's id, a login's vault: none of them opens.
	assertTrue(t, sessions.Restore(sealed, "another-secret-0123456789", t.TempDir(), moved, sessionsID) != nil)
	assertTrue(t, sessions.Restore(sealed, sessionsSecret, t.TempDir(), moved, "1a2b3c4d-0000-1111-2222-333344445555") != nil)
	login, err := vault.Seal(pyjson.New("provider", "claude-1", "files", pyjson.New()), sessionsSecret)
	if err != nil {
		t.Fatal(err)
	}
	assertFalse(t, vault.LooksSealedBlob(login, sessions.MaxBytes))
	assertTrue(t, sessions.Restore(login, sessionsSecret, t.TempDir(), moved, sessionsID) != nil)
	assertTrue(t, vault.Open(sealed, sessionsSecret) == nil)
	tampered := sealed[:len(sealed)-8] + "AAAAAAA="
	assertTrue(t, sessions.Restore(tampered, sessionsSecret, t.TempDir(), moved, sessionsID) != nil)
}
