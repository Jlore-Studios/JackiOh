package tests

// test_review_fixes.py: the adversarial review of the subscriptions change, each finding as the
// scenario it found.

import (
	"fmt"
	"path/filepath"
	"strconv"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/vault"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// reviewFixesMediumPR is medium_pr: a pull request a medium model (Muse, the first of them)
// built and approved by day, waiting for its review run.
func reviewFixesMediumPR(t *testing.T, h *flowHarness) int {
	t.Helper()
	h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: []string{config.LabelBuild}})
	h.night(workFake(map[string]runner.Handler{"build": crossTimedBuilder(map[string]string{"src/game.txt": "rules v2\n"}),
		"review": workReviewer(workApprove)}))
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelCross, "difficulty:medium"))
	return pr
}

// reviewFixesEmptyOut is empty_out.
func reviewFixesEmptyOut(t *testing.T, h *flowHarness, name string) string {
	out := filepath.Join(h.root, name)
	mkdir(t, out)
	return out
}

// ---------------------------------------------------------------------------- AutoMergeTests

func TestReviewFixes_ARevisionByAnotherModelTurnsStrongAutoMergeOff(t *testing.T) {
	// Muse keeps the medium item it planned (one lane, one owner, #725), and its in-run
	// approval waits for a Strong final; the Strong audit turns auto-merge on. Then Muse
	// revises by day, whose medium approval alone never ships a medium item, so the strong
	// approval of the old head is gone.
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: []string{"muse"}})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	flowUpdateItem(h.ctx, 12, pyjson.New("planned_at", clock.ISO(testsupport.Day), "planned_tier", "medium",
		"planned_by_id", "muse"))
	h.night(workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)}))
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	crossNextRun(h, "778")
	h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)})) // claude-3 audits at night
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)                             // Strong: auto-merge on
	// A revision is queued by a path that leaves auto-merge alone, and Muse does it by day:
	// its medium approval alone is not enough, so the strong approval of the old head is gone.
	thread := h.gh.Threads.Item(pr)
	thread.Set("labels", append(thread.List("labels"), pyjson.New("name", config.LabelRevise)))
	testsupport.ClockOf(h.ctx).At = testsupport.Day
	planned, result := h.night(workFake(map[string]runner.Handler{
		"revise": workBuilder(map[string]string{"src/game.txt": "v2.1\n"}),
		"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("provider"), result.Get("status")}, []any{"muse", "approved"})
	assertNotIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelCross))
}

func TestReviewFixes_AutoMergeIsPinnedToTheApprovedCommit(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr := reviewFixesMediumPR(t, h)
	// By day no Strong seat is free; at night claude-3 reviews, and its one strong
	// approval pins auto-merge to the head (#695).
	h.committedHours()
	planned, _ := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("provider"), dig(t, planned, "seats", "review", "tier")}, []any{"claude-3", "strong"})
	votes := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "votes")
	assertEqual(t, h.gh.AutoMergeHeads[fmt.Sprintf("PR_%d", pr)], dig(t, votes, "sha"))
}

func TestReviewFixes_ARejectionStandsUntilItsModelApprovesOrTheHeadMoves(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine})
	d := deliver.NewDeliverer(h.ctx, pyjson.New("action", "review", "provider", "agy"), reviewFixesEmptyOut(t, h, "o"),
		h.deliverRepo, "", nil, "")
	medium := func() *pyjson.Object { return pyjson.New("gpt", "medium", "muse", "medium", "gemini", "medium") }
	// Only a Strong approval ships (#695): medium ones, weak with medium, never do.
	assertFalse(t, d.RuleMet(pyjson.New("approvals", []any{"gpt", "muse"}, "tiers", medium()), "medium"))
	assertFalse(t, d.RuleMet(pyjson.New("approvals", []any{"gpt", "muse"}, "tiers", medium(),
		"rejections", []any{"gemini"}), "medium")) // votes are by family
	assertFalse(t, d.RuleMet(pyjson.New("approvals", []any{"gpt", "gemini"}, "tiers", medium(),
		"rejections", []any{}), "medium"))
	assertTrue(t, d.RuleMet(pyjson.New("approvals", []any{"claude"}, "tiers", pyjson.New("claude", "strong")), "medium"))
	assertFalse(t, d.RuleMet(pyjson.New("approvals", []any{"claude"}, "tiers", pyjson.New("claude", "strong"),
		"rejections", []any{"gemini"}), "medium"))
	assertTrue(t, d.RuleMet(pyjson.New("approvals", []any{"claude", "gemini"},
		"tiers", pyjson.New("claude", "strong", "gemini", "medium"),
		"rejections", []any{}), "medium"))
}

func TestReviewFixes_OnlyTheBotsOwnPullRequestsGetASecondReview(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	h.gh.AddPull(30, "feature/someone", fakes.Pull{Labels: []string{config.LabelCross}})
	assertNotEqual(t, plan.Make(h.ctx, false, nil, "auto", nil).Get("number"), 30)
}

func TestReviewFixes_WithOnlyItsBuilderSetUpItWaitsForAStrongReviewer(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: providersSecrets(), Machine: []string{"agy"}, At: testsupport.Day})
	h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: []string{config.LabelBuild}})
	h.night(workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)}))
	comment := lastOf(t, h.gh.BotComments(12))
	// Only a Strong approval ships a medium item (#725): agy's in-run approval is recorded,
	// and the head waits for a Strong seat no setup has.
	assertIn(t, "`gemini` (medium) approved it, so it waits for one strong model's final audit (Opus)", comment)
	assertIn(t, "No subscription that could give that review is set up", comment)
}

// ---------------------------------------------------------------------------- SecondReviewTests

func TestReviewFixes_ACommentDuringTheReviewGetsItsRevisionBeforeAnyMerge(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr := reviewFixesMediumPR(t, h)
	h.committedHours() // claude-3 works all day: the Strong final can audit

	commented := func(request *runner.RunRequest) *runner.RunResult {
		flowUpdateItem(h.ctx, pr, pyjson.New("pending_request", true))
		return workReviewer(workApprove)(request)
	}

	h.night(workFake(map[string]runner.Handler{"review": commented}))
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelRevise, "difficulty:medium"))
	assertFalse(t, pyjson.Truthy(dig(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "pending_request")))
}

func TestReviewFixes_AReviewWhoseRunDiesBacksOffItsSubscriptionAndIsCounted(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Night})
	pr := reviewFixesMediumPR(t, h)
	seen := []string{}
	for _, attempt := range []int{1, 2} {
		planned := plan.Make(h.ctx, false, nil, "auto", nil)
		assertEqual(t, []any{planned.Get("action"), planned.Get("number")}, []any{"review", pr})
		seen = append(seen, crossStr(t, planned, "provider"))
		deliver.NewDeliverer(h.ctx, planned, reviewFixesEmptyOut(t, h, fmt.Sprintf("died-%d", attempt)), h.deliverRepo,
			"", nil, "").Run()
	}
	// Each death backs its subscription off; the next strong seat takes it.
	assertEqual(t, seen, []string{"claude-3", "claude-7"})
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelBlocked, "difficulty:medium"))
	assertIn(t, "died 2 times", lastOf(t, h.gh.BotComments(pr)))
}

func TestReviewFixes_ASecondReviewRunsNoneOfTheBuildersCode(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	h.committedHours() // claude-3 works all day: the Strong final can audit
	log := filepath.Join(h.root, "installs.log")
	env := map[string]string{"GITHUB_SERVER_URL": "file://" + filepath.Join(h.root, "remote"),
		"BOT_GITHUB_TOKEN": strings.Repeat("t", 20)}
	for k, v := range crossAll {
		env[k] = v
	}
	h.cfg = testsupport.MakeConfig(testsupport.Config{Env: env, Machine: testsupport.Machine,
		CommittedHours: true, // the Strong final can audit
		Overrides: pyjson.New("gates", []any{pyjson.New("name", "rules exist", "run", "true", "timeout_minutes", 1)},
			"install", pyjson.New("run", "pwd >> "+log, "timeout_minutes", 1),
			"max_review_cycles", 2)})
	h.ctx = testsupport.MakeCtx(h.gh, testsupport.Ctx{Cfg: h.cfg, At: testsupport.Day})
	pr := reviewFixesMediumPR(t, h)
	builds := strings.Count(readText(t, log), "\n")
	assertTrue(t, builds >= 1) // the build did install, so the log works
	planned, review := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("action"), review.Get("status")}, []any{"review", "reviewed"})
	assertEqual(t, strings.Count(readText(t, log), "\n"), builds)
	assertNotIn(t, fmt.Sprintf("item-%d", pr), readText(t, log))
}

// ---------------------------------------------------------------------------- InfraTests

func TestReviewFixes_ABrokenSubscriptionBacksOffAlone(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	assertEqual(t, planned.Get("provider"), "muse")
	out := reviewFixesEmptyOut(t, h, "infra")
	writeText(t, filepath.Join(out, "result.json"), pyjson.Dumps(pyjson.New("status", "infra",
		"reason", "agy's login is not a login")))
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	planned = plan.Make(h.ctx, false, nil, "auto", nil)
	assertEqual(t, []any{planned.Get("number"), planned.Get("provider")}, []any{12, "agy"})
}

func TestReviewFixes_ARunGithubCannotReadKeepsItsLaneAndItsItem(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelWorking}})
	gh.AddIssue(4, fakes.Issue{Labels: []string{config.LabelBuild}})

	gh.Patch.GetRun = func(runID any) *pyjson.Object {
		herr.Raise(&herr.GitHubError{Msg: "server error", Status: 502})
		return nil
	}

	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day,
		Cfg: testsupport.MakeConfig(testsupport.Config{Env: providersSecrets(), Machine: []string{"gpt"}})})
	ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, 3).Update(pyjson.New("run_id", "9", "provider", "gpt"))
		return nil
	}, "state")
	planned := plan.Make(ctx, false, nil, "auto", nil)
	assertEqual(t, gh.LabelNames(3), setOf(config.LabelWorking)) // not requeued
	assertEqual(t, planned.Get("action"), "none")                // gpt, its only subscription, is held
}

// reviewFixesHeld is a held run: (number, provider).
type reviewFixesHeld struct {
	number   int
	provider string
}

func TestReviewFixes_ARunRequestIsKeptWhenEveryLaneIsBusy(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(4, fakes.Issue{Labels: []string{config.LabelBuild}})
	// The machine's three slots and GitHub's three lanes, each held apart from the other.
	held := []reviewFixesHeld{{5, "gpt"}, {6, "agy"}, {7, "muse"}, {8, "claude-4"}, {9, "claude-5"}, {10, "claude-6"}}
	for _, run := range held {
		gh.AddIssue(run.number, fakes.Issue{Labels: []string{config.LabelWorking}})
		gh.Runs.Set(strconv.Itoa(run.number), pyjson.New("status", "in_progress"))
	}
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day,
		Cfg: testsupport.WithLanes(testsupport.MakeConfig(testsupport.Config{Env: crossAll, Machine: testsupport.Machine}), 3)})
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.Set("run_requested", pyjson.New("at", "2026-09-29T16:59:00Z", "item", nil))
		for _, run := range held {
			state.Item(s, run.number).Update(pyjson.New("run_id", strconv.Itoa(run.number), "provider", run.provider))
		}
		return nil
	}, "state")
	assertIn(t, "every lane is busy", crossStr(t, plan.Make(ctx, false, nil, "auto", nil), "reason"))
	assertNotEqual(t, dig(t, ctx.Store.Load(), "run_requested"), nil)
}

// A full machine leaves GitHub's lanes to the Claude accounts, and Claude runs filling GitHub's
// lanes leave the machine's slots to its subscriptions.
func TestReviewFixes_GithubSRunnersAndTheMachineHaveLanesOfTheirOwn(t *testing.T) {
	for _, c := range []struct {
		held      []reviewFixesHeld
		onMachine bool
	}{
		{[]reviewFixesHeld{{5, "gpt"}, {6, "agy"}, {7, "muse"}}, false},
		{[]reviewFixesHeld{{5, "claude-4"}, {6, "claude-5"}, {7, "claude-6"}}, true},
	} {
		names := []string{}
		for _, run := range c.held {
			names = append(names, run.provider)
		}
		t.Run(fmt.Sprintf("held=%v", names), func(t *testing.T) {
			gh := fakes.NewFakeGitHub()
			gh.AddIssue(4, fakes.Issue{Labels: []string{config.LabelBuild}})
			for _, run := range c.held {
				gh.AddIssue(run.number, fakes.Issue{Labels: []string{config.LabelWorking}})
				gh.Runs.Set(strconv.Itoa(run.number), pyjson.New("status", "in_progress"))
			}
			ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night,
				Cfg: testsupport.WithLanes(testsupport.MakeConfig(testsupport.Config{Env: crossAll, Machine: testsupport.Machine}), 3)})
			ctx.Store.Update(func(s *pyjson.Object) any {
				for _, run := range c.held {
					state.Item(s, run.number).Update(pyjson.New("run_id", strconv.Itoa(run.number), "provider", run.provider))
				}
				return nil
			}, "state")
			planned := plan.Make(ctx, false, nil, "auto", nil)
			assertEqual(t, []any{planned.Get("action"), planned.Get("number")}, []any{"build", 4})
			assertEqual(t, ctx.Cfg.Pool.OnMachine(crossStr(t, planned, "provider")), c.onMachine)
		})
	}
}

// ---------------------------------------------------------------------------- VaultAndLoginTests

func TestReviewFixes_AVaultWriteThatRacesAnotherWriterIsTriedAgain(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	out := reviewFixesEmptyOut(t, h, "vault")
	writeText(t, filepath.Join(out, "result.json"), pyjson.Dumps(pyjson.New("status", "failed", "reason", "boom")))
	sealed, err := vault.Seal(pyjson.New("provider", "muse", "files", pyjson.New("auth.json", "{}")), "the-secret")
	if err != nil {
		t.Fatal(err)
	}
	writeText(t, filepath.Join(out, "vault.enc"), sealed)
	h.gh.Branches["bot-state"] = true
	h.gh.ConflictsToInject = 1
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	text, _ := h.gh.GetFile("vault/muse.enc", "bot-state")
	assertEqual(t, strings.TrimSpace(text), sealed)
}

func TestReviewFixes_EveryModelCallHandsARefreshedLoginOnAtOnce(t *testing.T) {
	calls := []int{}
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	r := workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)})
	work.NewWorker(h.cfg, planned, r, h.clone, filepath.Join(h.root, "work"), filepath.Join(h.root, "out-calls"),
		nil, func() { calls = append(calls, 1) }, nil, nil, nil, nil).Run()
	assertEqual(t, len(calls), len(r.Calls))
}

func TestReviewFixes_OnlyAClaudeAccountCanWaitForQuiet(t *testing.T) {
	raw := providersRaw()
	crossObj(t, raw, "providers", "gpt").Set("quiet_check", true)
	_, raised := herr.Catch[*herr.ConfigError](func() { providers.Parse(raw) })
	assertTrue(t, raised, "ConfigError not raised")
}
