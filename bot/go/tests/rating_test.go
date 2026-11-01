// #317 parts 4, 8 and 10: the bot rates each item's difficulty when it plans it, under the easy
// rule; three failures of an item's own raise it a step and rebuild its pull request from `main`;
// `/harness rebuild` and `/harness review [strong|medium]`.
//
// bot/tests/test_rating.py. Its rated_plan, planner, day_harness, EASY and MEDIUM are
// ratingRatedPlan, ratingPlanner, ratingDayHarness, ratingEasy and ratingMedium, which other files
// import.
package tests

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/commands"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/issueplan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stepup"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// ---------------------------------------------------------------------------- the file's helpers

// ratingReadResult is json.loads((out / "result.json").read_text()), or {} when it is not there.
func ratingReadResult(t *testing.T, out string) *pyjson.Object {
	t.Helper()
	text, err := os.ReadFile(filepath.Join(out, "result.json"))
	if errors.Is(err, os.ErrNotExist) {
		return pyjson.New()
	}
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(text)
	if err != nil {
		t.Fatal(err)
	}
	return v.(*pyjson.Object)
}

// ---------------------------------------------------------------------------- test_rating.py

const (
	ratingEasy   = "difficulty:easy"
	ratingMedium = "difficulty:medium"
	ratingHard   = "difficulty:hard"
)

// ratingRatedPlan is rated_plan(difficulty, files=("src/game.txt",), why="one file, one test").
func ratingRatedPlan(difficulty string, files []string, why string) string {
	if files == nil {
		files = []string{"src/game.txt"}
	}
	if why == "" {
		why = "one file, one test"
	}
	rows := make([]string, 0, len(files))
	for _, path := range files {
		rows = append(rows, fmt.Sprintf("| `%s` | change | the change |", path))
	}
	return fmt.Sprintf("<!-- bot: {\"difficulty\": \"%s\", \"why\": \"%s\"} -->\n", difficulty, why) +
		"## 1. Goal\n\nRules v2.\n\n## 2. Files to touch\n\n| Path | new/change | What |\n" +
		"|---|---|---|\n" + strings.Join(rows, "\n") + "\n\n## 3. Steps\n\n1. Edit it.\n"
}

// ratingPlanner is planner(text): a runner whose planner answers `text` in five minutes.
func ratingPlanner(text string) *runner.FakeRunner {
	return workFake(map[string]runner.Handler{"plan": func(request *runner.RunRequest) *runner.RunResult {
		result := runner.NewRunResult(true, text)
		result.DurationS = 300
		return result
	}})
}

// ratingDayHarness is day_harness(test, machine=("muse", "lite")): by day, no Claude account
// (they keep to the night in tests), Muse and the weak one on the machine, and the planning lane
// on.
func ratingDayHarness(t *testing.T, machine ...string) *flowHarness {
	if machine == nil {
		machine = []string{"muse", "lite"}
	}
	return newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: machine, At: testsupport.Day, CommittedPlanLanes: true})
}

// ratingDifficulties is {n for n in labels if n.startswith("difficulty:")}.
func ratingDifficulties(labels map[string]bool) map[string]bool {
	out := map[string]bool{}
	for n := range labels {
		if strings.HasPrefix(n, "difficulty:") {
			out[n] = true
		}
	}
	return out
}

// ratingPrompt is runner.calls[i].prompt.
func ratingPrompt(t *testing.T, fake *runner.FakeRunner, i int) string {
	t.Helper()
	if i >= len(fake.Calls) {
		t.Fatalf("IndexError: the runner had %d calls", len(fake.Calls))
	}
	return fake.Calls[i].Prompt
}

// ---------------------------------------------------------------------------- RatingTests

func TestRating_AMediumPlannerRatesAnUnratedItemEasyAndItIsCleared(t *testing.T) {
	h := ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Make the rules v2."})
	fake := ratingPlanner(ratingRatedPlan("easy", nil, ""))
	planned, result := h.night(fake)
	assertEqual(t, []any{dig(t, planned, "action"), dig(t, planned, "provider"), dig(t, result, "status")},
		[]any{"plan", "muse", "planned"})
	assertEqual(t, dig(t, planned, "rating"), pyjson.New("difficulty", "medium", "source", "", "by", ""))
	assertIn(t, "Rate how hard it is", ratingPrompt(t, fake, 0))
	assertIn(t, "At most 10 files changed", ratingPrompt(t, fake, 0))
	assertIn(t, ratingEasy, h.gh.LabelNames(12))
	record := dig(t, h.ctx.Store.Load(), "items", "12")
	assertEqual(t, []any{dig(t, record, "difficulty_by", "difficulty"), dig(t, record, "difficulty_by", "tier")},
		[]any{"easy", "medium"})
	said := lastOf(t, h.gh.BotComments(12))
	assertIn(t, "Rated `difficulty:easy` by `muse`", said)
	assertIn(t, "Cleared and queued to build", said)
	assertNotIn(t, "<!-- bot:", issueplan.PlanOf(pyjson.Str(dig(t, h.gh.Threads.Item(12), "body"))))
	// The weak one builds it from Muse's plan, which meets an easy item's floor.
	planned = plan.Make(h.ctx, false, nil, "auto", nil)
	assertEqual(t, []any{dig(t, planned, "action"), dig(t, planned, "provider")}, []any{"build", "lite"})
}

// A planner that leaves out its `<!-- bot: ... -->` line must not leave the item unlabelled
// (#735): it is `difficulty:medium`, the bot's own rating, so a later planner may raise it.
func TestRating_APlanWithNoRatingLineStillLabelsTheItemMedium(t *testing.T) {
	h := ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Make the rules v2."})
	_, rest, _ := strings.Cut(ratingRatedPlan("easy", nil, ""), "\n")
	h.night(ratingPlanner(rest))
	assertEqual(t, ratingDifficulties(h.gh.LabelNames(12)), setOf(ratingMedium))
	record := dig(t, h.ctx.Store.Load(), "items", "12")
	assertEqual(t, dig(t, record, "difficulty_by", "difficulty"), "medium")
	assertIn(t, "gave no rating, so it is `difficulty:medium`", lastOf(t, h.gh.BotComments(12)))
}

// A medium plan meets a medium item's floor (#695): no strong plan comes next, it is cleared to
// build.
func TestRating_AMediumRatingByAMediumPlannerStands(t *testing.T) {
	h := ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Make the rules v2."})
	h.night(ratingPlanner(ratingRatedPlan("medium", nil, "a new ruling")))
	assertIn(t, ratingMedium, h.gh.LabelNames(12))
	assertNotIn(t, config.LabelNeedsPlan, h.gh.LabelNames(12))
	assertIn(t, "Cleared and queued to build", lastOf(t, h.gh.BotComments(12)))
}

func TestRating_ARatingIsNeverLoweredAndAPersonsLabelNeverReplaced(t *testing.T) {
	// A hard item with only a medium plan still needs a strong one: by night claude-1, strong,
	// plans it, and its easy rating does not lower it.
	h := newFlowHarness(t, flowHarnessOpts{Machine: []string{"muse"}, CommittedPlanLanes: true})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, ratingHard}, Body: "Make the rules v2."})
	flowUpdateItem(h.ctx, 12, pyjson.New("difficulty_by", pyjson.New("difficulty", "hard", "provider", "muse", "tier", "medium"), "planned_at", "2026-09-29T00:00:00Z", "planned_tier", "medium"))
	fake := ratingPlanner(ratingRatedPlan("easy", nil, ""))
	planned, _ := h.night(fake)
	assertEqual(t, []any{dig(t, planned, "action"), dig(t, planned, "provider")}, []any{"plan", "claude-1"})
	assertIn(t, "you may rate it higher, never lower", ratingPrompt(t, fake, 0))
	assertIn(t, ratingHard, h.gh.LabelNames(12))
	assertIn(t, "a rating is never lowered", lastOf(t, h.gh.BotComments(12)))
	assertIn(t, "Cleared and queued", lastOf(t, h.gh.BotComments(12)))
	// A person's label: the planner is told not to rate, and a rating it gives anyway is
	// ignored.
	h = ratingDayHarness(t, "muse")
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, ratingEasy}, Body: "Make the rules v2."})
	fake = ratingPlanner(ratingRatedPlan("hard", nil, ""))
	planned, _ = h.night(fake)
	assertEqual(t, dig(t, planned, "rating", "source"), "person")
	assertIn(t, "A person set this item's difficulty", ratingPrompt(t, fake, 0))
	assertEqual(t, ratingDifficulties(h.gh.LabelNames(12)), setOf(ratingEasy))
}

func TestRating_AnEasyPlanThatBreaksTheRuleIsMedium(t *testing.T) {
	h := ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Make the rules v2."})
	h.night(ratingPlanner(ratingRatedPlan("easy", []string{"src/game.txt", "SPEC.md"}, "")))
	assertIn(t, ratingMedium, h.gh.LabelNames(12))
	said := lastOf(t, h.gh.BotComments(12))
	assertIn(t, "The easy rule does not hold (lines 3–5: the plan touches `SPEC.md`)", said)
	h = ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Make the rules v2."})
	files := []string{}
	for n := range 11 {
		files = append(files, fmt.Sprintf("src/f%d.txt", n))
	}
	h.night(ratingPlanner(ratingRatedPlan("easy", files, "")))
	assertIn(t, "line 1: the plan touches 11 files, over 10", lastOf(t, h.gh.BotComments(12)))
}

// With the planning lane off, Muse plans the unrated item in its own build run; it rates it hard,
// which Muse may not build, so the run stops after planning.
func TestRating_ABuildRunThatRatesItsItemOutOfReachStopsAfterPlanning(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"muse"}, At: testsupport.Day})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Make the rules v2."})
	planned, result := h.night(workFake(map[string]runner.Handler{
		"plan": func(r *runner.RunRequest) *runner.RunResult {
			return runner.NewRunResult(true, ratingRatedPlan("hard", nil, "the resolution loop"))
		},
		"build": workBuilder(map[string]string{})}))
	assertEqual(t, []any{dig(t, planned, "action"), dig(t, result, "status")}, []any{"build", "planned"})
	assertIn(t, "rated difficulty:hard", dig(t, result, "reason"))
	assertIn(t, ratingHard, h.gh.LabelNames(12))
	assertIn(t, config.LabelBuild, h.gh.LabelNames(12))
	assertIsNone(t, h.originSHA("bot/issue-12"))
}

func TestRating_AWeakBuildersChangeThatBreaksTheRuleIsNotPushed(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"lite"}})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, ratingEasy}, Body: issueplan.WithPlan(
		"Make it.", "1. Edit src/game.txt.", "a person", "")})
	many := map[string]string{}
	for n := range 11 {
		many[fmt.Sprintf("src/f%d.txt", n)] = "x\n"
	}
	planned, result := h.night(workFake(map[string]runner.Handler{"build": workBuilder(many),
		"self_check": workReviewer(workApprove)}))
	assertEqual(t, []any{dig(t, planned, "provider"), dig(t, result, "status")}, []any{"lite", "built"})
	assertEqual(t, h.gh.ListPulls("open", "", 200), []*pyjson.Object{})
	assertIn(t, "breaks the easy rule (line 1: it changes 11 files, over 10)",
		lastOf(t, h.gh.BotComments(12)))
	// A person labelled it easy, so the label stays, under a medium floor.
	assertIn(t, ratingEasy, h.gh.LabelNames(12))
	assertIn(t, config.LabelBuild, h.gh.LabelNames(12))
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", "12", "difficulty_floor"), "medium")
	assertIsNotNone(t, h.originSHA("bot/issue-12"))                                     // the work is kept on its branch
	assertEqual(t, dig(t, plan.Make(h.ctx, false, nil, "auto", nil), "action"), "none") // The weak one may not take it now
}

// ---------------------------------------------------------------------------- StepUpTests

// ratingCtx is StepUpTests.ctx and CommandTests.ctx.
func ratingCtx() (*fakes.FakeGitHub, *hctx.Context) {
	gh := fakes.NewFakeGitHub()
	return gh, testsupport.MakeCtx(gh, testsupport.Ctx{Cfg: testsupport.MakeConfig()})
}

func TestRating_ThreeStrikesRaiseABotRatedItemAStep(t *testing.T) {
	gh, ctx := ratingCtx()
	gh.AddIssue(12, fakes.Issue{Labels: []string{ratingEasy}})
	flowUpdateItem(ctx, 12, pyjson.New("difficulty_by", pyjson.New("difficulty", "easy", "provider", "muse")))
	assertFalse(t, stepup.Strike(ctx, 12, "the run failed: a", ""))
	assertFalse(t, stepup.Strike(ctx, 12, "its reviewer did not approve it: b", ""))
	assertTrue(t, stepup.Strike(ctx, 12, "the run failed: c", ""))
	assertIn(t, ratingMedium, gh.LabelNames(12))
	assertNotIn(t, ratingEasy, gh.LabelNames(12))
	assertIn(t, config.LabelBuild, gh.LabelNames(12))
	said := lastOf(t, gh.BotComments(12))
	assertIn(t, "This failed 3 times at `difficulty:easy`", said)
	assertIn(t, "the run failed: a; its reviewer did not approve it: b; the run failed: c",
		said)
	record := dig(t, ctx.Store.Load(), "items", "12")
	assertEqual(t, []any{dig(t, record, "strikes"), dig(t, record, "difficulty_by", "difficulty")}, []any{0, "medium"})
}

func TestRating_APersonsLabelOrHardAsksAPerson(t *testing.T) {
	gh, ctx := ratingCtx()
	gh.AddIssue(12, fakes.Issue{Labels: []string{ratingEasy, config.LabelBuild}})
	for n := range 3 {
		stepup.Strike(ctx, 12, fmt.Sprintf("failure %d", n), "")
	}
	assertEqual(t, gh.LabelNames(12), setOf(ratingEasy, config.LabelBlocked))
	assertIn(t, "A person set `difficulty:easy`, so I do not change it: should it be "+
		"`difficulty:medium`?", lastOf(t, gh.BotComments(12)))
	gh.AddIssue(13, fakes.Issue{Labels: []string{ratingHard}})
	flowUpdateItem(ctx, 13, pyjson.New("difficulty_by", pyjson.New("difficulty", "hard")))
	for n := range 3 {
		stepup.Strike(ctx, 13, fmt.Sprintf("failure %d", n), "")
	}
	assertIn(t, config.LabelBlocked, gh.LabelNames(13))
	assertIn(t, "no stronger model", lastOf(t, gh.BotComments(13)))
}

func TestRating_StrikesOnAPullRequestCountOnItsIssueAndRebuildIt(t *testing.T) {
	gh, ctx := ratingCtx()
	gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelPROpen}})
	gh.AddPull(40, "bot/issue-12", fakes.Pull{Body: "Closes #12", Labels: []string{config.LabelPR, config.LabelCross},
		SHA: "h1"})
	flowUpdateItem(ctx, 40, pyjson.New("issue", 12, "wip", pyjson.New("sha", "w")))
	for n := range 3 {
		stepup.Strike(ctx, 40, fmt.Sprintf("rejected %d", n), "")
	}
	assertEqual(t, dig(t, gh.Threads.Item(40), "state"), "closed")
	assertIn(t, ratingHard, gh.LabelNames(12)) // unrated, so medium, and one step up is hard
	both := map[string]bool{}
	for n := range gh.LabelNames(12) {
		if n == config.LabelBuild || n == config.LabelPROpen {
			both[n] = true
		}
	}
	assertEqual(t, both, setOf(config.LabelBuild))
	assertIn(t, [2]string{"bot/old/issue-12-20260930-0300", "h1"}, gh.CreatedBranches)
	assertIn(t, "bot/issue-12", gh.DeletedBranches)
	assertIn(t, "bot/wip/40", gh.DeletedBranches)
	record := dig(t, ctx.Store.Load(), "items", "12")
	assertEqual(t, []any{dig(t, record, "previous_pr"), dig(t, record, "previous_branch")},
		[]any{40, "bot/old/issue-12-20260930-0300"})
	assertTrue(t, dig(t, ctx.Store.Load(), "items", "40", "rebuilt"))
}

// ---------------------------------------------------------------------------- CommandTests

// ratingRunCommand is CommandTests.run_command.
func ratingRunCommand(ctx *hctx.Context, number int, line string) string {
	command := commands.Parse(line, "jgoetzmann-bot")[0]
	thread := ctx.GH.GetIssue(number)
	return events.Execute(ctx, command, thread, fakes.Operator.Copy(), nil)
}

func TestRating_RebuildClosesThePullRequestAndQueuesTheIssue(t *testing.T) {
	gh, ctx := ratingCtx()
	gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelPROpen}})
	gh.AddPull(40, "bot/issue-12", fakes.Pull{Body: "Closes #12", Labels: []string{config.LabelPR}, SHA: "h1"})
	reply := ratingRunCommand(ctx, 40, "/harness rebuild")
	assertEqual(t, reply, "Closed #40 and queued #12 to build again from `main`.")
	assertEqual(t, dig(t, gh.Threads.Item(40), "state"), "closed")
	assertIn(t, config.LabelBuild, gh.LabelNames(12))
	assertIn(t, "@jgoetzmann asked for it", dig(t, ctx.Store.Load(), "items", "12", "previous_why"))
	// The same from the issue; and an issue with no pull request has nothing to rebuild.
	gh.AddIssue(13)
	assertIn(t, "no open pull request of mine to rebuild",
		ratingRunCommand(ctx, 13, "@jgoetzmann-bot rebuild"))
	// The new build is told about the old pull request.
	planned := plan.Make(ctx, false, nil, "auto", nil)
	assertEqual(t, []any{dig(t, planned, "number"), dig(t, planned, "previous_pr")}, []any{12, 40})
}

func TestRating_ReviewQueuesAReviewRunAtATierAndNoRevision(t *testing.T) {
	gh, ctx := ratingCtx()
	gh.AddPull(40, "bot/issue-12", fakes.Pull{Body: "Closes #12", Labels: []string{config.LabelPR}, SHA: "h1"})
	reply := ratingRunCommand(ctx, 40, "/harness review strong check the replay")
	assertIn(t, "Queued a review run of #40, by a strong model or stronger", reply)
	assertEqual(t, gh.LabelNames(40), setOf(config.LabelPR, config.LabelCross))
	record := dig(t, ctx.Store.Load(), "items", "40")
	assertEqual(t, []any{dig(t, record, "review_floor"), dig(t, record, "review_notes")},
		[]any{"strong", "check the replay"})
	// By night a strong model is free: it takes the review, with the notes.
	planned := plan.Make(ctx, false, nil, "auto", nil)
	review := difficultySeatOf(t, planned, "review")
	assertEqual(t, []any{dig(t, planned, "action"), review[2]}, []any{"review", "strong"})
	assertEqual(t, dig(t, planned, "review_notes"), "check the replay")
	// By day only medium models are free (in tests): a strong-floor review waits.
	gh2 := fakes.NewFakeGitHub()
	ctx2 := testsupport.MakeCtx(gh2, testsupport.Ctx{At: testsupport.Day, Cfg: testsupport.MakeConfig(testsupport.Config{Machine: []string{"muse"}})})
	gh2.AddPull(40, "bot/issue-12", fakes.Pull{Body: "Closes #12", Labels: []string{config.LabelPR}, SHA: "h1"})
	ratingRunCommand(ctx2, 40, "/harness review strong")
	assertEqual(t, dig(t, plan.Make(ctx2, false, nil, "auto", nil), "action"), "none")
	ratingRunCommand(ctx2, 40, "/harness review medium")
	assertEqual(t, dig(t, plan.Make(ctx2, false, nil, "auto", nil), "provider"), "muse")
	// On an issue it says where it works.
	gh.AddIssue(13)
	assertIn(t, "one of my pull requests", ratingRunCommand(ctx, 13, "/harness review"))
}
