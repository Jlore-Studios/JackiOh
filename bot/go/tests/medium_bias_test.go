// #695: classification biases toward medium, while weak-tier handling is unchanged.
//
// Borderline work (close to the easy limits, or a rating whose `why` admits doubt) now rates
// `difficulty:medium`; clear-easy work still rates easy. The weak tier still builds only easy
// items (`MIN_TIER`, `easy_first`) and the review rule still counts a weak approval only toward an
// easy item (`review_rule.py`, untouched). (bot/tests/test_medium_bias.py)
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
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

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
	h := ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{"bot:build"}, Body: "Make the rules v2."})
	r := ratingPlanner(ratingRatedPlan("easy", nil, ""))
	h.night(r)
	assertIn(t, ratingEasy, h.gh.LabelNames(12))
	assertIn(t, "Rated `difficulty:easy`", lastOf(t, h.gh.BotComments(12)))
}

func TestMediumBias_APlanCloseToTheLimitRatesMedium(t *testing.T) {
	h := ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{"bot:build"}, Body: "Make the rules v2."})
	h.night(ratingPlanner(ratingRatedPlan("easy", mediumBiasFiles(9), "")))
	assertIn(t, ratingMedium, h.gh.LabelNames(12))
	assertIn(t, "borderline easy is medium", lastOf(t, h.gh.BotComments(12)))
}

func TestMediumBias_AnUnsureRatingRatesMedium(t *testing.T) {
	h := ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{"bot:build"}, Body: "Make the rules v2."})
	h.night(ratingPlanner(ratingRatedPlan("easy", nil, "torn between easy and medium")))
	assertIn(t, ratingMedium, h.gh.LabelNames(12))
	assertIn(t, "was unsure", lastOf(t, h.gh.BotComments(12)))
}

func TestMediumBias_ThePlannerIsToldToPreferMediumWhenInDoubt(t *testing.T) {
	h := ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{"bot:build"}, Body: "Make the rules v2."})
	r := ratingPlanner(ratingRatedPlan("easy", nil, ""))
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
