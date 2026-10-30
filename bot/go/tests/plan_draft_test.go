package tests

// test_plan_draft.py: a planning run cut off mid-call keeps its draft, and the next planner goes
// on from it.
//
// #37 was planned three times on 2026-10-06 and never got a plan: `claude-1` read for 20 minutes
// and was stopped at its 40% cap, `claude-4` started just under its 70% cap and was stopped four
// minutes in, and `claude-5` stopped before it started. The planner was a reader, so it wrote
// nothing down; its transcript's end was never kept; and each run started from scratch. Now the
// planner keeps a draft (`work.PLAN_DRAFT_FILE`), a cut-off run hands it on as a `draft` handoff
// and in the journal, and the planning lane starts only `start_headroom` under each cap.

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/issueplan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/journal"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

var planDraftDraft = "## Goal\nRules v2.\n\n## Files to touch\n| `src/game.txt` | change | the rules |"
var planDraftPlan = "1. **Goal.** Rules v2.\n2. **Steps.** Edit src/game.txt; run the checks."

// planDraftCutOffPlanner is cut_off_planner: a planner that writes its draft, strays into a
// tracked file, and is stopped at its cap.
func planDraftCutOffPlanner(request *runner.RunRequest) *runner.RunResult {
	mustWriteText(filepath.Join(request.Cwd, work.PlanDraftFile), planDraftDraft+"\n")
	mustWriteText(filepath.Join(request.Cwd, "src", "game.txt"), "a planner's edit\n")
	r := runner.NewRunResult(false, "")
	r.ExitCode, r.DurationS, r.Error = 1, 1200, "stopped mid-call"
	r.Extra = pyjson.New("usage_stop", "5-hour usage is 70%, at or over its 70% cap")
	return r
}

// planDraftTrailRunner is TrailRunner: a fake that leaves a transcript, whose end is something to
// hand on.
type planDraftTrailRunner struct {
	*runner.FakeRunner
}

func newPlanDraftTrailRunner(handlers map[string]runner.Handler) *planDraftTrailRunner {
	return &planDraftTrailRunner{workFake(handlers)}
}

func (x *planDraftTrailRunner) Run(request *runner.RunRequest) *runner.RunResult {
	if err := os.MkdirAll(filepath.Dir(request.Transcript), 0o777); err != nil {
		panic(err)
	}
	mustWriteText(request.Transcript, "{}\n")
	return x.FakeRunner.Run(request)
}

func (x *planDraftTrailRunner) Trail(transcript string) string {
	return "- Read: SPEC.md §4\n- said: the change is in src/game.txt"
}

// planDraftHarness is planning_harness.
func planDraftHarness(t *testing.T) *flowHarness {
	h := newFlowHarness(t, flowHarnessOpts{Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN_2"), Machine: testsupport.Machine,
		At: testsupport.Day, CommittedPlanLanes: true})
	h.committedHours()
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Make the rules v2."})
	return h
}

func TestPlanDraft_ACutOffPlanKeepsItsDraftAndTheNextPlannerGoesOnFromIt(t *testing.T) {
	h := planDraftHarness(t)
	r := newPlanDraftTrailRunner(map[string]runner.Handler{"plan": planDraftCutOffPlanner})
	planned, first := h.night(r)
	assertEqual(t, []any{planned.Get("action"), first.Get("status"), first.Get("interrupt")},
		[]any{"plan", "interrupted", "usage"})
	call := r.Calls[0]
	assertEqual(t, call.AllowedTools, work.PlannerTools)
	assertIn(t, "Write", call.AllowedTools)
	assertNotIn(t, "Write", call.DisallowedTools)
	assertIn(t, "Bash(git commit:*)", call.DisallowedTools)
	assertIn(t, strings.TrimSpace(work.DraftAsk), call.Prompt)
	assertIn(t, "The one file you write is your draft, `"+work.PlanDraftFile+"`", call.Prompt)
	assertEqual(t, dig(t, first, "handoff", "draft"), planDraftDraft)
	// The draft is kept for the next run, with the end of the planner's session.
	record := crossObj(t, h.ctx.Store.Load(), "items", "12")
	assertEqual(t, dig(t, record, "handoff", "kind"), "draft")
	assertEqual(t, dig(t, record, "handoff", "draft"), planDraftDraft)
	assertIn(t, "the change is in src/game.txt", crossStr(t, record, "handoff", "trail"))
	assertIn(t, "Its draft so far is kept, and the next planning run goes on from it.", lastOf(t, h.gh.BotComments(12)))
	journalText, _ := h.gh.GetFile("12.md", journal.Branch)
	assertIn(t, "The planner's draft when it stopped", journalText)
	assertIn(t, "Rules v2.", journalText)
	// Nothing was built, and the planner's stray edit went nowhere.
	assertEqual(t, h.originSHA("bot/issue-12"), "")
	assertNotIn(t, "a planner's edit", fakes.Git(h.origin, "show", "main:src/game.txt"))
	assertIn(t, config.LabelBuild, h.gh.LabelNames(12))

	seen := map[string]string{}

	finishing := func(request *runner.RunRequest) *runner.RunResult {
		seen["draft"] = mustReadText(filepath.Join(request.Cwd, work.PlanDraftFile))
		seen["prompt"] = request.Prompt
		return runner.NewRunResult(true, planDraftPlan)
	}
	planned, second := h.night(workFake(map[string]runner.Handler{"plan": finishing}))
	assertEqual(t, []any{planned.Get("action"), second.Get("status")}, []any{"plan", "planned"})
	assertEqual(t, dig(t, planned, "handoff", "kind"), "draft")
	assertEqual(t, strings.TrimSpace(seen["draft"]), planDraftDraft)
	assertIn(t, "## An unfinished plan from an earlier run", seen["prompt"])
	// The lane's weakest planner meeting the floor plans first (#695): Muse, not claude-2.
	assertIn(t, "`muse` (muse) was cut off before it finished", seen["prompt"])
	assertIn(t, "which is now `"+work.PlanDraftFile+"` at the top of your worktree", seen["prompt"])
	assertIn(t, "the change is in src/game.txt", seen["prompt"])
	// The finished plan supersedes the draft everywhere.
	assertNotIn(t, "draft", crossObj(t, second, "handoff"))
	assertFalse(t, exists(filepath.Join(h.root, "work", "item-12", work.PlanDraftFile)))
	record = crossObj(t, h.ctx.Store.Load(), "items", "12")
	assertEqual(t, dig(t, record, "handoff", "kind"), "plan")
	assertNotIn(t, "draft", crossObj(t, record, "handoff"))
	assertIn(t, "Edit src/game.txt", issueplan.PlanOf(crossStr(t, h.gh.Threads.Item(12), "body")))
}

// #37's third planning run (`claude-5`) stopped before any model call: the draft the run before
// left stays for the one after.
func TestPlanDraft_ARunStoppedBeforeItStartsKeepsTheLastDraft(t *testing.T) {
	h := planDraftHarness(t)
	h.night(workFake(map[string]runner.Handler{"plan": planDraftCutOffPlanner}))
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	assertEqual(t, planned.Get("action"), "plan")
	out := filepath.Join(h.root, "out-stopped")
	worker := work.NewWorker(h.cfg, planned, runner.NewFakeRunner(nil), h.clone, filepath.Join(h.root, "work"), out,
		nil, nil, nil, nil, nil, nil)
	worker.StartUsage = pyjson.New("status", "allowed", "five_hour", pyjson.New(
		"utilization", 0.95, "resets_at", "2099-01-01T00:00:00Z"))
	result := worker.Run()
	assertEqual(t, []any{result.Get("status"), result.Get("model_calls")}, []any{"interrupted", 0})
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	handoff := crossObj(t, h.ctx.Store.Load(), "items", "12", "handoff")
	assertEqual(t, []any{dig(t, handoff, "kind"), dig(t, handoff, "draft")}, []any{"draft", planDraftDraft})
	assertNotIn(t, "draft so far is kept", lastOf(t, h.gh.BotComments(12)))
}

// No draft yet, only the end of the session: still a planner's handoff, never one a builder is
// told to pick up from.
func TestPlanDraft_APlannerCutOffBeforeAnyDraftStillHandsOnWhatItFound(t *testing.T) {
	h := planDraftHarness(t)
	stopped := runner.NewRunResult(false, "")
	stopped.ExitCode, stopped.DurationS, stopped.Error = 1, 240, "stopped mid-call"
	stopped.Extra = pyjson.New("usage_stop", "5-hour usage is 70%, at or over its 70% cap")
	h.night(newPlanDraftTrailRunner(map[string]runner.Handler{
		"plan": func(*runner.RunRequest) *runner.RunResult { return stopped }}))
	handoff := crossObj(t, h.ctx.Store.Load(), "items", "12", "handoff")
	assertEqual(t, dig(t, handoff, "kind"), "draft")
	assertNotIn(t, "draft", handoff)
	assertIn(t, "the change is in src/game.txt", crossStr(t, handoff, "trail"))
	assertIn(t, "The end of its session is kept for the next planning run.", lastOf(t, h.gh.BotComments(12)))
	seen := map[string]string{}

	finishing := func(request *runner.RunRequest) *runner.RunResult {
		seen["prompt"] = request.Prompt
		return runner.NewRunResult(true, planDraftPlan)
	}
	h.night(workFake(map[string]runner.Handler{"plan": finishing}))
	assertIn(t, "It left no draft, only the end of its session below", seen["prompt"])
	assertNotIn(t, "Picking up from another agent", seen["prompt"])
}

func TestPlanDraft_AFinishedPlanHandsOnNoDraft(t *testing.T) {
	root := crossMkdtemp(t)
	_, clone := fakes.MakeOrigin(root)
	cfg := testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("gates", workGates,
		"install", pyjson.New("run", "true", "timeout_minutes", 1))})

	drafting := func(request *runner.RunRequest) *runner.RunResult {
		mustWriteText(filepath.Join(request.Cwd, work.PlanDraftFile), planDraftDraft)
		return runner.NewRunResult(true, planDraftPlan)
	}
	p := pyjson.New("action", "plan", "number", 12, "title", "Rules v2", "branch", "bot/issue-12",
		"thread", "Please make the rules v2.")
	result := work.NewWorker(cfg, p, workFake(map[string]runner.Handler{"plan": drafting}), clone,
		filepath.Join(root, "work"), filepath.Join(root, "out"), nil, nil, nil, nil, nil, nil).Run()
	assertEqual(t, result.Get("status"), "planned")
	assertNotIn(t, "draft", crossObj(t, result, "handoff"))
	assertTrue(t, strings.HasPrefix(crossStr(t, result, "handoff", "notes"), "# Plan"))
	assertNotIn(t, "## Files to touch", crossStr(t, result, "handoff", "notes"))
}

// A build run that plans first gives its planner the draft; its builder starts from the plan
// that planner wrote, and is not told about the draft.
func TestPlanDraft_ABuilderNeverStartsFromADraft(t *testing.T) {
	root := crossMkdtemp(t)
	_, clone := fakes.MakeOrigin(root)
	cfg := testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("gates", workGates,
		"install", pyjson.New("run", "true", "timeout_minutes", 1))})
	handoff := pyjson.New("kind", "draft", "provider", "claude-4", "family", "claude",
		"reason", "`claude-4` stops mid-call", "notes", "", "trail", "", "draft", planDraftDraft)
	seat := func() *pyjson.Object { return pyjson.New("provider", "claude-1", "model", "opus", "tier", "strong") }
	p := pyjson.New("action", "build", "number", 12, "title", "Rules v2", "branch", "bot/issue-12",
		"thread", "Please make the rules v2.", "handoff", handoff,
		"seats", pyjson.New("plan", seat(), "build", seat(), "review", seat()))
	worker := work.NewWorker(cfg, p, runner.NewFakeRunner(nil), clone, filepath.Join(root, "work"),
		filepath.Join(root, "out"), nil, nil, nil, nil, nil, nil)
	planner := worker.HandoffText(false)
	assertIn(t, "## An unfinished plan from an earlier run", planner)
	assertIn(t, planDraftDraft, planner)
	builder := worker.HandoffText(true)
	assertNotIn(t, "unfinished plan", builder)
	assertNotIn(t, "Picking up from another agent", builder)
	assertNotIn(t, "picking up", worker.FirstWhy("build", []string{}))
}

// A person writes the plan after a planner was cut off: the builder starts from theirs, and the
// draft is dropped.
func TestPlanDraft_APlanInTheDescriptionReplacesADraft(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, Cfg: testsupport.MakeConfig(testsupport.Config{
		Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN_2"), Machine: testsupport.Machine, CommittedPlanLanes: true})})
	body := issueplan.WithPlan("Make the rules v2.", planDraftPlan, "a person", "")
	gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: body})
	ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, 12).Update(pyjson.New("handoff", pyjson.New(
			"kind", "draft", "provider", "claude-4", "family", "claude", "notes", "",
			"trail", "the end of claude-4's session", "draft", planDraftDraft)))
		return nil
	}, "state")
	planned := plan.Make(ctx, true, nil, "auto", nil)
	assertEqual(t, planned.Get("action"), "build")
	assertEqual(t, dig(t, planned, "handoff", "kind"), "plan")
	assertNotIn(t, "draft", crossObj(t, planned, "handoff"))
	assertNotIn(t, "trail", crossObj(t, planned, "handoff"))
	assertIn(t, "Edit src/game.txt", crossStr(t, planned, "handoff", "notes"))
	assertEqual(t, dig(t, planned, "handoff", "provider"), "the issue's description")
}
