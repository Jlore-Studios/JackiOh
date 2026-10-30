// test_waste.py: the wasted runs and quota the audit in #160 found, each as the scenario that
// showed it.
package tests

import (
	"fmt"
	"os"
	"path/filepath"
	"slices"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cli"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gates"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/issueplan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// ---------------------------------------------------------------------------- the file's helpers

// wasteWorker is Worker(cfg, plan, runner, repo_dir, work_dir, out_dir) with no keyword argument.
func wasteWorker(cfg *config.Config, planned *pyjson.Object, r any, repo, workDir, out string) *work.Worker {
	return work.NewWorker(cfg, planned, r, repo, workDir, out, nil, nil, nil, nil, nil, nil)
}

// wasteStr is a string field of a result, `d[key]` as text.
func wasteStr(o *pyjson.Object, key string) string { return py.Str(o.Get(key)) }

// ---------------------------------------------------------------------------- the tests

// wasteAt is AT = DAY.
var wasteAt = testsupport.Day

// wasteTimedOut is timed_out(name).
func wasteTimedOut(name string) *gates.GateResult {
	return &gates.GateResult{Name: name, Run: "pnpm test", OK: false, ExitCode: gates.TimedOut,
		Seconds: 2400.0, Tail: "(timed out after 2400s)"}
}

// ---------------------------------------------------------------------------- TimedOutCheckTests
//
// `pnpm test` never finished inside its 40 minutes on GitHub's runners; the harness then ran it
// again on a copy of main, which timed out too, and called it "also red on main".

func TestWaste_ATimedOutGateIsInconclusiveAndDoesNotBlock(t *testing.T) {
	results := []*gates.GateResult{wasteTimedOut("unit")}
	gates.MarkInconclusive(results, setOf("unit"))
	assertTrue(t, results[0].Inconclusive)
	assertTrue(t, gates.Green(results))
	assertEqual(t, gates.FailuresText(results), "")
	assertIn(t, "timed out: inconclusive, left to CI", gates.Table(results))
}

func TestWaste_ATimedOutInstallStillFails(t *testing.T) {
	results := []*gates.GateResult{wasteTimedOut("install")}
	gates.MarkInconclusive(results, setOf("unit"))
	assertFalse(t, results[0].Inconclusive)
	assertFalse(t, gates.Green(results))
}

func TestWaste_ATimedOutGateIsNeverRunAgainOnMain(t *testing.T) {
	root := t.TempDir()
	_, clone := fakes.MakeOrigin(root)
	cfg := testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New(
		"gates", []any{pyjson.New("name", "unit", "run", "false", "timeout_minutes", 1)},
		"install", workInstall())})
	worker := wasteWorker(cfg, pyjson.New("action", "build", "number", 12, "branch", "bot/issue-12"),
		runner.NewFakeRunner(nil), clone, filepath.Join(root, "work"), filepath.Join(root, "out"))
	results := []*gates.GateResult{wasteTimedOut("unit")}
	worker.MarkPreExisting(results)
	assertTrue(t, results[0].Inconclusive)
	assertFalse(t, results[0].PreExisting)
	assertTrue(t, worker.BaseWt == nil) // no copy of main was installed or run
}

func TestWaste_TheCommittedTestGateRunsOnlyRelatedTestsInMinutes(t *testing.T) {
	var gate *config.Gate // assertIsInstance(gate, Gate): the type says so
	for _, g := range testsupport.MakeConfig().Gates {
		if g.Name == "web tests" {
			gate = g
			break
		}
	}
	if gate == nil {
		t.Fatal("StopIteration: no gate named web tests")
	}
	assertIn(t, "vitest run --project web --changed origin/main", gate.Run)
	// As CI's web unit tests: the audio, fx and asset tests run in the daily super run.
	for _, skipped := range []string{"'**/src/audio/**'", "'**/src/fx/**'",
		"'**/src/cards/art/convention.test.ts'"} {
		assertIn(t, "--exclude "+skipped, gate.Run)
	}
	assertLessEqual(t, gate.TimeoutMinutes, 20)
}

// ---------------------------------------------------------------------------- PlanCutTests
//
// Only the last 20,000 characters of a plan were kept, so #160's Plan began mid-table.

// wastePlanItem is the plan every PlanCutTests run takes.
func wastePlanItem() *pyjson.Object {
	return pyjson.New("action", "plan", "number", 12, "title", "Rules v2", "branch", "bot/issue-12",
		"thread", "Please make the rules v2.")
}

// wasteRunPlan is PlanCutTests.run_plan(text): a planning run whose planner answers `text`.
func wasteRunPlan(t *testing.T, text string) (*runner.FakeRunner, *pyjson.Object) {
	t.Helper()
	root := t.TempDir()
	_, clone := fakes.MakeOrigin(root)
	cfg := testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("gates", workGates,
		"install", workInstall())})
	r := workFake(map[string]runner.Handler{"plan": func(*runner.RunRequest) *runner.RunResult {
		return runner.NewRunResult(true, text)
	}})
	return r, wasteWorker(cfg, wastePlanItem(), r, clone, filepath.Join(root, "work"), filepath.Join(root, "out")).Run()
}

func TestWaste_ALongPlanKeepsItsBeginning(t *testing.T) {
	longPlan := "## Goal\nThe first step.\n" + strings.Repeat("x", issueplan.PlanChars*2) + "\nThe last step."
	_, result := wasteRunPlan(t, longPlan)
	assertEqual(t, result.Get("status"), "planned", result.Get("reason"))
	text := wasteStr(result.Obj("plan"), "text")
	assertTrue(t, strings.HasPrefix(text, "## Goal\nThe first step."))
	assertTrue(t, strings.HasSuffix(text, work.PlanCut))
	assertLessEqual(t, py.Len(text), issueplan.PlanChars)
	assertTrue(t, strings.HasPrefix(wasteStr(result.Obj("handoff"), "notes"), "# Plan"))
}

// #208: plans were cut at 20,000 characters (#307's among them).
func TestWaste_APlanOverTheOldCutIsKeptWhole(t *testing.T) {
	planText := "## Goal\nThe first step.\n" + strings.Repeat("x", 25_000) + "\nThe last step."
	assertTrue(t, 20_000 < py.Len(planText) && py.Len(planText) < issueplan.PlanChars)
	_, result := wasteRunPlan(t, planText)
	assertEqual(t, result.Get("status"), "planned")
	assertEqual(t, result.Obj("plan").Get("text"), planText)
}

func TestWaste_ThePlanPromptStatesItsBudget(t *testing.T) {
	r, _ := wasteRunPlan(t, "## Goal\nRules v2.")
	prompt := r.Calls[0].Prompt
	assertIn(t, "under "+py.Thousands(issueplan.PlanWords)+" words", prompt)
	assertIn(t, "first "+py.Thousands(issueplan.PlanChars)+" characters", prompt)
	assertIn(t, "Say each thing once", prompt)
}

func TestWaste_APlanTheDescriptionRefusedGoesWholeIntoItsComment(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Make the rules v2."})
	h.gh.Patch.SetIssueBody = func(int, string) *pyjson.Object {
		herr.Raise(&herr.GitHubError{Msg: "body is too long", Status: 422})
		return nil
	}
	planText := "## Goal\nThe first step.\n" + strings.Repeat("x", 25_000) + "\nThe last step."
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	out := filepath.Join(h.root, "out-plan")
	r := workFake(map[string]runner.Handler{"plan": func(*runner.RunRequest) *runner.RunResult {
		return runner.NewRunResult(true, planText)
	}})
	asPlan := planned.Copy().Set("action", "plan")
	wasteWorker(h.cfg, asPlan, r, h.clone, filepath.Join(h.root, "work"), out).Run()
	deliver.NewDeliverer(h.ctx, planned.Copy().Set("action", "plan"), out, h.deliverRepo, "", nil, "").Run()
	said := lastOf(t, h.gh.BotComments(12))
	assertIn(t, "could not take the plan", said)
	assertIn(t, "The last step.", said)
}

// ---------------------------------------------------------------------------- ForbiddenOnlyTests
//
// Muse built #160 for 73 minutes; every path was in `bot/`, so delivery refused it all.

func TestWaste_ABuildThatOnlyChangesForbiddenPathsStopsAndAsksAPerson(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Title: "Tune the bot", Labels: []string{config.LabelBuild}})
	r := workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"bot/harness/tuned.py": "x = 1\n"}),
		"review": workReviewer(workApprove)})
	_, result := h.night(r)
	assertEqual(t, result.Get("status"), "blocked", result.Get("reason"))
	assertTrue(t, !slices.Contains(workRoles(r), "review")) // no rounds spent on it
	assertTrue(t, !result.Has("bundle"))
	assertIsNone(t, h.originSHA("bot/issue-12"))
	// Handed to people (#793): `human required` in place of the bot's own `bot:blocked`.
	assertTrue(t, h.gh.LabelNames(12)[config.LabelHumanRequired])
	assertTrue(t, !h.gh.LabelNames(12)[config.LabelBlocked])
	said := lastOf(t, h.gh.BotComments(12))
	assertIn(t, "`bot/harness/tuned.py`", said)
	assertIn(t, "A person has to make this change", said)
}

func TestWaste_ABuildWithWorkOutsideThemGoesOn(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	r := workFake(map[string]runner.Handler{
		"build": workBuilder(map[string]string{"bot/x.py": "1\n", "src/game.txt": "v2\n"}),
		"fix":   workBuilder(map[string]string{}), "review": workReviewer(workApprove)})
	_, result := h.night(r)
	assertTrue(t, slices.Contains(workRoles(r), "review"))
	assertFalse(t, pyjson.Equal(result.Get("status"), "blocked"))
}

// ---------------------------------------------------------------------------- ClosedDuringTheRunTests
//
// A builder stayed on #125 for hours after #163 closed it.

func TestWaste_TheProbeStopsARunWhoseThreadClosed(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5)
	ctx := testsupport.MakeCtx(gh)
	number := 5
	probe := cli.MakeProbe(ctx, &number, nil, nil)
	_, _, stop := probe(nil)
	assertFalse(t, stop)
	gh.Threads.Item(5).Set("state", "closed")
	reason, kind, stop := probe(nil)
	assertTrue(t, stop)
	assertEqual(t, [2]string{reason, kind}, [2]string{"#5 was closed", "stop"})
}

func TestWaste_GithubFailingToAnswerDoesNotStopIt(t *testing.T) {
	ctx := testsupport.MakeCtx(fakes.NewFakeGitHub())
	number := 404
	_, _, stop := cli.MakeProbe(ctx, &number, nil, nil)(nil) // no such thread: GitHub says 404
	assertFalse(t, stop)
}

// ---------------------------------------------------------------------------- BrokenLoginTests
//
// claude-3's token was refused for 6+ hours and was tried every 50 minutes regardless.

func TestWaste_EachFailureInARowWaitsLonger(t *testing.T) {
	st := pyjson.New()
	pool := testsupport.MakeConfig(testsupport.Config{Env: crossAll, CommittedHours: true}).Pool // claude-3 works all day
	claude := pool.Get("claude-3")
	secrets := providers.SecretsOf(crossAll)
	waits := []time.Duration{}
	for range 4 {
		providers.NoteInfra(st, "claude-3", "401 OAuth access token is invalid", wasteAt, true)
		waits = append(waits, providers.InfraBackoff(st.Obj("providers").Obj("claude-3").Obj("infra")))
	}
	assertEqual(t, waits, []time.Duration{50 * time.Minute, 2 * time.Hour, 8 * time.Hour, 8 * time.Hour})
	why := providers.Availability(claude, st, wasteAt.Add(7*time.Hour), "America/Chicago", secrets, false, false)
	assertIn(t, "4 times in a row", why)
	assertEqual(t, providers.Availability(claude, st, wasteAt.Add(9*time.Hour), "America/Chicago", secrets,
		false, false), "")
}

func TestWaste_AFailureThatIsNotTheProvidersKeepsTheStreak(t *testing.T) {
	st := pyjson.New()
	providers.NoteInfra(st, "gpt", "refused", wasteAt, true)
	assertEqual(t, providers.NoteInfra(st, "gpt", "install red on main", wasteAt, false), 1)
}

func TestWaste_AStreakAsksAPersonOnceAndAWorkingRunEndsIt(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: wasteAt})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	provider := wasteStr(planned, "provider")

	deliverResult := func(result *pyjson.Object) {
		out := filepath.Join(h.root, fmt.Sprintf("out-%d", flowOuts(h.root)))
		herr.Must(os.Mkdir(out, 0o777))
		herr.Must(os.WriteFile(filepath.Join(out, "result.json"), []byte(pyjson.Dumps(result)), 0o666))
		deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	}

	for range providers.InfraAskAfter + 1 {
		deliverResult(pyjson.New("status", "infra", "reason", "401 OAuth access token is invalid",
			"model_calls", 1))
	}
	asked := []int{}
	for n, thread := range h.gh.Threads.All() {
		title := wasteStr(thread, "title")
		if strings.Contains(title, provider) && strings.Contains(title, "times in a row") {
			asked = append(asked, n)
		}
	}
	assertEqual(t, len(asked), 1) // once per streak, not once per run
	issue := asked[0]
	assertTrue(t, h.gh.LabelNames(issue)[config.LabelHuman])
	assertIn(t, "once every 8 hours", wasteStr(h.gh.Threads.Item(issue), "body"))
	deliverResult(pyjson.New("status", "failed", "reason", "the harness failed", "model_calls", 2))
	assertTrue(t, !h.ctx.Store.Load().Obj("providers").Obj(provider).Has("infra"))
	assertEqual(t, h.gh.Threads.Item(issue).Get("state"), "closed")
	assertIn(t, "works again", lastOf(t, h.gh.BotComments(issue)))
}

// ---------------------------------------------------------------------------- HonestReportTests
//
// The pull request said "the reviewer still had blocking findings" after one unreadable review
// (#176 fixed the comment); one-commit PRs landed on main as "bot: build pass 1 for #85".

// wastePull is `gh.list_pulls(head=branch)[0]`.
func wastePull(t *testing.T, gh *fakes.FakeGitHub, branch string) *pyjson.Object {
	t.Helper()
	pulls := gh.ListPulls("open", branch, 200)
	if len(pulls) == 0 {
		t.Fatalf("IndexError: no pull request from %s", branch)
	}
	return pulls[0]
}

func TestWaste_AnUnapprovedPullRequestSaysHowManyRoundsItTookAndWhy(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	h.night(workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer("LGTM")}))
	body := wasteStr(wastePull(t, h.gh, "bot/issue-12"), "body")
	assertIn(t, "It stopped without an approval after 1 of 2 round(s): the reviewer's "+
		"answer could not be read twice in a row, so this is a draft", body)
	assertNotIn(t, "still had blocking findings", body)
}

func TestWaste_TheSquashCommitTakesThePullRequestsTitle(t *testing.T) {
	// claude-3 alone, so its Opus review meets the rule (#695) and auto-merge turns on.
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN_3"}})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	h.night(workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)}))
	pr, _ := pyjson.AsInt(wastePull(t, h.gh, "bot/issue-12").Get("number"))
	// Neither the builder's title nor the fake issue's passes the title check, so it is written.
	assertEqual(t, h.gh.MergeTitles[fmt.Sprintf("PR_%d", pr)], fmt.Sprintf("Patch: Make the rules v2 (#%d)", pr))
}

// #187: the title check holds a pull request's title to docs/issues-and-patches.md, so a
// builder's plain sentence gives way to its issue's conventional title, and a builder's
// conventional title (its version numbered, say) stands.
func TestWaste_ATitleThatBreaksTheConventionGivesWayToTheIssues(t *testing.T) {
	issue := "Patch: the rules, version two"
	for _, c := range [][2]string{
		{workDone, issue},
		{strings.ReplaceAll(workDone, "Make the rules v2", "Patch v0.3.2: the rules, version two"),
			"Patch v0.3.2: the rules, version two"},
	} {
		text, want := c[0], c[1]
		// claude-3 alone, so its Opus review meets the rule (#695) and auto-merge turns on.
		h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN_3"}})
		h.gh.AddIssue(12, fakes.Issue{Title: issue, Labels: []string{config.LabelBuild}})
		h.night(workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "v2\n"}, text),
			"review": workReviewer(workApprove)}))
		pull := wastePull(t, h.gh, "bot/issue-12")
		assertEqual(t, pull.Get("title"), want)
		assertEqual(t, h.gh.MergeTitles["PR_"+py.Str(pull.Get("number"))],
			want+" (#"+py.Str(pull.Get("number"))+")")
	}
}

// ---------------------------------------------------------------------------- DependencyTests
//
// #131 said "Do not start until #125 has merged" and would have gone first; part 4 of a patch was
// queued while part 2 was open.

// wastePicks is picks(ctx, gh): the queue's numbers and what it skipped.
func wastePicks(ctx *hctx.Context) ([]int, []string) {
	skipped := []string{}
	st := ctx.Store.Load()
	numbers := []int{}
	for _, c := range queue.Candidates(ctx, st, &skipped, nil) {
		numbers = append(numbers, c.Number)
	}
	return numbers, skipped
}

func TestWaste_ABlockedByLineHoldsTheBuildUntilItsBlockerCloses(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(125, fakes.Issue{Title: "Homescreen rotation"})
	gh.AddIssue(131, fakes.Issue{Body: "**Blocked by #125.** Do not start until #125 has merged.",
		Labels: []string{config.LabelBuild}})
	numbers, skipped := wastePicks(ctx)
	assertEqual(t, numbers, []int{})
	assertEqual(t, skipped, []string{"#131 skipped: it waits for #125 to close first"})
	gh.Threads.Item(125).Set("state", "closed")
	numbers, _ = wastePicks(ctx)
	assertEqual(t, numbers, []int{131})
}

func TestWaste_ALineNamingSeveralWaitsForEveryOpenOne(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(3)
	gh.AddIssue(4, fakes.Issue{State: "closed"})
	gh.AddIssue(9, fakes.Issue{Body: "Depends on #3, #4 and #12.", Labels: []string{config.LabelBuild}})
	_, skipped := wastePicks(ctx)
	assertTrue(t, slices.Contains(skipped, "#9 skipped: it waits for #3 to close first"), skipped)
}

func TestWaste_AMentionInThePlanOrElsewhereIsNotABlocker(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(3)
	body := "See #3 for context.\n\n<!-- jackioh-bot:plan -->\n## Plan\n\nThis depends on #3." +
		"\n<!-- /jackioh-bot:plan -->\n"
	gh.AddIssue(9, fakes.Issue{Body: body, Labels: []string{config.LabelBuild}})
	numbers, _ := wastePicks(ctx)
	assertEqual(t, numbers, []int{9})
}

func TestWaste_GithubsOwnBlockedByLinkHoldsIt(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(3)
	gh.AddIssue(9, fakes.Issue{Labels: []string{config.LabelBuild}})
	gh.Block(9, 3)
	_, skipped := wastePicks(ctx)
	assertEqual(t, skipped, []string{"#9 skipped: it waits for #3 to close first"})
}

func TestWaste_APartWaitsForTheEarlierPartsOfItsPatch(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(124, fakes.Issue{Title: "Patch v0.2.X (part 2 of 4): global animations"})
	gh.AddIssue(126, fakes.Issue{Title: "Patch v0.2.X (part 4 of 4): more card patches",
		Labels: []string{config.LabelBuild}})
	gh.AddIssue(140, fakes.Issue{Title: "Patch v0.2.X (part 2 of 3): another patch", Labels: []string{config.LabelBuild}})
	numbers, skipped := wastePicks(ctx)
	assertEqual(t, numbers, []int{140}) // another patch of the same name, with its own m
	assertEqual(t, skipped, []string{"#126 skipped: it waits for #124 to close first"})
}

func TestWaste_AForcedBuildGoesAheadAnyway(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(125)
	gh.AddIssue(131, fakes.Issue{Body: "Blocked by #125.", Labels: []string{config.LabelBuild}})
	ctx.Store.Update(func(s *pyjson.Object) any { state.Item(s, 131).Set("forced", true); return nil }, "state")
	numbers, _ := wastePicks(ctx)
	assertEqual(t, numbers, []int{131})
}

func TestWaste_RevisionsAreNeverHeld(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(125)
	gh.AddPull(20, "bot/issue-131", fakes.Pull{Body: "Closes #131. Blocked by #125.",
		Labels: []string{config.LabelPR, config.LabelRevise}})
	numbers, _ := wastePicks(ctx)
	assertEqual(t, numbers, []int{20})
}

func TestWaste_NothingNamedAsksGithubNothingMore(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	calls := []string{}
	gh.Patch.ListIssues = func(labels, st string, limit int, assignee string) []*pyjson.Object {
		calls = append(calls, labels)
		return gh.Own().ListIssues(labels, st, limit, assignee)
	}
	gh.AddIssue(9, fakes.Issue{Labels: []string{config.LabelBuild}})
	wastePicks(ctx)
	// Only the queue's label reads: each call names labels (`kw.get("labels")`).
	assertTrue(t, !slices.Contains(calls, ""), calls)
}
