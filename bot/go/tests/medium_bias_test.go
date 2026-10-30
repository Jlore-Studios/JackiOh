// #695: classification biases toward medium, while weak-tier handling is unchanged.
//
// Borderline work (close to the easy limits, or a rating whose `why` admits doubt) now rates
// `difficulty:medium`; clear-easy work still rates easy. The weak tier still builds only easy
// items (`MIN_TIER`, `easy_first`) and the review rule still counts a weak approval only toward an
// easy item (`review_rule.py`, untouched). (bot/tests/test_medium_bias.py)
//
// It also holds the helpers it and test_declined.py import from test_rating.py, under this file's
// prefix (see difficulty_test.go).
package tests

import (
	"fmt"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/easy"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/reviewrule"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// ---------------------------------------------------------------------------- test_rating.py's

// test_rating.EASY, MEDIUM
var (
	mediumBiasEasy   = "difficulty:easy"
	mediumBiasMedium = "difficulty:medium"
)

// mediumBiasRatedPlan is test_rating.rated_plan(difficulty, files=("src/game.txt",),
// why="one file, one test").
func mediumBiasRatedPlan(difficulty string, files []string, why string) string {
	if files == nil {
		files = []string{"src/game.txt"}
	}
	if why == "" {
		why = "one file, one test"
	}
	rows := []string{}
	for _, path := range files {
		rows = append(rows, fmt.Sprintf("| `%s` | change | the change |", path))
	}
	return fmt.Sprintf("<!-- bot: {\"difficulty\": \"%s\", \"why\": \"%s\"} -->\n", difficulty, why) +
		"## 1. Goal\n\nRules v2.\n\n## 2. Files to touch\n\n| Path | new/change | What |\n" +
		fmt.Sprintf("|---|---|---|\n%s\n\n## 3. Steps\n\n1. Edit it.\n", strings.Join(rows, "\n"))
}

// mediumBiasPlanner is test_rating.planner(text): a planning call that answers `text`.
func mediumBiasPlanner(text string) *runner.FakeRunner {
	return runner.NewFakeRunner(map[string][]runner.Handler{"plan": {func(request *runner.RunRequest) *runner.RunResult {
		result := runner.NewRunResult(true, text)
		result.DurationS = 300
		return result
	}}})
}

// mediumBiasDayHarness is test_rating.day_harness(test, machine=("muse", "lite")): by day, no
// Claude account (they keep to the night in tests), Muse and the weak one on the machine, and the
// planning lane on.
func mediumBiasDayHarness(t *testing.T, machine ...string) *difficultyHarness {
	if machine == nil {
		machine = []string{"muse", "lite"}
	}
	return newDifficultyHarness(t, difficultyHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: machine, At: testsupport.Day, PlanLanes: true})
}

// ---------------------------------------------------------------------------- this file's helpers

func mediumBiasTable(files []string) string {
	rows := []string{}
	for _, path := range files {
		rows = append(rows, fmt.Sprintf("| `%s` | change | the change |", path))
	}
	return "## 2. Files to touch\n\n| Path | new/change | What |\n|---|---|---|\n" +
		strings.Join(rows, "\n") + "\n"
}

// mediumBiasFiles is tuple(f"src/f{n}.txt" for n in range(count)).
func mediumBiasFiles(count int) []string {
	out := []string{}
	for n := range count {
		out = append(out, fmt.Sprintf("src/f%d.txt", n))
	}
	return out
}

// mediumBiasCounts is {f"src/f{n}.txt": 1 for n in range(count)}.
func mediumBiasCounts(count int) *pyjson.Object {
	out := pyjson.New()
	for _, f := range mediumBiasFiles(count) {
		out.Set(f, 1)
	}
	return out
}

func mediumBiasLast(comments []string) string { return comments[len(comments)-1] }

// ---------------------------------------------------------------------------- BorderlineBreachTests

func TestMediumBias_ASmallPlanAndChangeHold(t *testing.T) {
	cfg := testsupport.MakeConfig()
	assertEqual(t, easy.PlanBreach(mediumBiasTable([]string{"src/game.txt"}), cfg.Easy, cfg.ReviewPaths,
		cfg.ForbiddenPaths), "")
	assertEqual(t, easy.ChangeBreach(pyjson.New("src/game.txt", 10), cfg.Easy), "")
}

func TestMediumBias_APlanCloseToTheFileLimitIsMedium(t *testing.T) {
	cfg := testsupport.MakeConfig()
	breach := easy.PlanBreach(mediumBiasTable(mediumBiasFiles(9)), cfg.Easy, cfg.ReviewPaths, cfg.ForbiddenPaths)
	assertIn(t, "close to the limit of 10", breach)
	assertIn(t, "borderline easy is medium", breach)
	// Room to spare stays easy, and going over keeps its old message.
	assertEqual(t, easy.PlanBreach(mediumBiasTable(mediumBiasFiles(8)), cfg.Easy, cfg.ReviewPaths,
		cfg.ForbiddenPaths), "")
	assertIn(t, "over 10", easy.PlanBreach(mediumBiasTable(mediumBiasFiles(11)), cfg.Easy, cfg.ReviewPaths,
		cfg.ForbiddenPaths))
}

func TestMediumBias_AChangeCloseToALimitIsMedium(t *testing.T) {
	cfg := testsupport.MakeConfig()
	assertIn(t, "borderline easy is medium", easy.ChangeBreach(mediumBiasCounts(9), cfg.Easy))
	assertIn(t, "close to the limit of 400", easy.ChangeBreach(pyjson.New("src/big.txt", 350), cfg.Easy))
	assertIn(t, "over 10", easy.ChangeBreach(mediumBiasCounts(11), cfg.Easy))
}

// A `max_files: 1` lane (fullsend parts) still builds a one-file change as easy.
func TestMediumBias_TinyLimitsHaveNoBorderlineZone(t *testing.T) {
	cfg := testsupport.MakeConfig()
	tiny := *cfg.Easy
	tiny.MaxFiles = 1
	assertEqual(t, easy.PlanBreach(mediumBiasTable([]string{"src/client.txt"}), &tiny, cfg.ReviewPaths,
		cfg.ForbiddenPaths), "")
	assertEqual(t, easy.ChangeBreach(pyjson.New("src/client.txt", 1), &tiny), "")
}

func TestMediumBias_AnUnsureWhyIsDoubtAndAConfidentOneIsNot(t *testing.T) {
	for _, why := range []string{"torn between easy and medium",
		"borderline easy, but the files fit",
		"not sure about the shared surface",
		"close to the limit on files",
		"could go either way"} {
		assertTrue(t, easy.UncertainWhy(why), why)
	}
	for _, why := range []string{"one file, one test", "a single copy-paste", ""} {
		assertFalse(t, easy.UncertainWhy(why), why)
	}
}

// ---------------------------------------------------------------------------- RatingBiasTests

func TestMediumBias_AClearEasyItemStillRatesEasy(t *testing.T) {
	h := mediumBiasDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{"bot:build"}, Body: "Make the rules v2."})
	r := mediumBiasPlanner(mediumBiasRatedPlan("easy", nil, ""))
	h.night(r)
	assertIn(t, mediumBiasEasy, h.gh.LabelNames(12))
	assertIn(t, "Rated `difficulty:easy`", mediumBiasLast(h.gh.BotComments(12)))
}

func TestMediumBias_APlanCloseToTheLimitRatesMedium(t *testing.T) {
	h := mediumBiasDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{"bot:build"}, Body: "Make the rules v2."})
	h.night(mediumBiasPlanner(mediumBiasRatedPlan("easy", mediumBiasFiles(9), "")))
	assertIn(t, mediumBiasMedium, h.gh.LabelNames(12))
	assertIn(t, "borderline easy is medium", mediumBiasLast(h.gh.BotComments(12)))
}

func TestMediumBias_AnUnsureRatingRatesMedium(t *testing.T) {
	h := mediumBiasDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{"bot:build"}, Body: "Make the rules v2."})
	h.night(mediumBiasPlanner(mediumBiasRatedPlan("easy", nil, "torn between easy and medium")))
	assertIn(t, mediumBiasMedium, h.gh.LabelNames(12))
	assertIn(t, "was unsure", mediumBiasLast(h.gh.BotComments(12)))
}

func TestMediumBias_ThePlannerIsToldToPreferMediumWhenInDoubt(t *testing.T) {
	h := mediumBiasDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{"bot:build"}, Body: "Make the rules v2."})
	r := mediumBiasPlanner(mediumBiasRatedPlan("easy", nil, ""))
	h.night(r)
	assertIn(t, "When in doubt", r.Calls[0].Prompt)
	assertIn(t, "At most 10 files changed", r.Calls[0].Prompt)
}

func TestMediumBias_TheRuleKeepsTheTiebreakerAndNamesBorderline(t *testing.T) {
	assertIn(t, "choose the higher", easy.Rule)
	assertIn(t, "borderline easy is **medium**", easy.Rule)
	assertIn(t, "room to spare", easy.Rule)
}

// ---------------------------------------------------------------------------- WeakTierUntouchedTests

func TestMediumBias_WeakBuildsOnlyEasy(t *testing.T) {
	assertEqual(t, config.MinTier, map[string]string{"easy": "weak", "medium": "medium", "hard": "strong"})
}

func TestMediumBias_AWeakApprovalNeverShips(t *testing.T) {
	// A weak approval is a finding round, not a vote (#725): no weak review runs go out, and it
	// never ships anything at any difficulty.
	assertTrue(t, reviewrule.Met([]string{"weak", "medium"}, "easy")) // the medium ships it
	assertFalse(t, reviewrule.Met([]string{"weak", "medium"}, "medium"))
	assertFalse(t, reviewrule.Met([]string{"weak", "medium"}, "hard"))
	assertFalse(t, reviewrule.Helps([]string{}, "weak", "easy"))
	assertFalse(t, reviewrule.Helps([]string{}, "weak", "medium"))
	assertFalse(t, reviewrule.Reachable([]string{}, []string{"weak"}, "easy"))
	assertTrue(t, reviewrule.Reachable([]string{"medium"}, []string{"weak"}, "easy")) // already met
	// Only a Strong approval ships medium and hard; a medium one ships easy.
	assertTrue(t, reviewrule.Met([]string{"strong"}, "hard"))
	assertFalse(t, reviewrule.Met([]string{"medium", "medium"}, "hard"))
	assertFalse(t, reviewrule.Met([]string{"weak"}, "easy"))
	assertTrue(t, reviewrule.Met([]string{"medium"}, "easy"))
}
