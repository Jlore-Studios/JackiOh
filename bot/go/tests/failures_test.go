// Why a run's rounds failed (bot/harness/failures.py), and the labels that go with it: `bot:stuck`
// on a run that used every round, `bot:planned` on an item whose plan is in its description.
// (bot/tests/test_failures.py)
package tests

import (
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/failures"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// failuresCycleOpts is cycle's keyword arguments; a zero field is the default.
type failuresCycleOpts struct {
	Failed bool     // ok = True
	Error  any      // error = None
	Red    []string // red = ()
	Review string   // review = "blocked"
	Claims []string // claims = ("It breaks replay.",)
}

func failuresCycle(n int, opts ...failuresCycleOpts) *pyjson.Object {
	var o failuresCycleOpts
	if len(opts) > 0 {
		o = opts[0]
	}
	if o.Review == "" {
		o.Review = "blocked"
	}
	if o.Claims == nil {
		o.Claims = []string{"It breaks replay."}
	}
	red := false
	for _, r := range o.Red {
		if r == "typecheck" {
			red = true
		}
	}
	exit := 0
	if red {
		exit = 2
	}
	gates := []any{pyjson.New("name", "typecheck", "ok", !red, "exit_code", exit, "pre_existing", false,
		"skipped", ""),
		pyjson.New("name", "unit", "ok", false, "exit_code", 1, "pre_existing", true, "skipped", "")}
	var verdict *pyjson.Object
	switch o.Review {
	case "unreadable":
		verdict = pyjson.New("verdict", "unreadable", "readable", false, "findings", []any{})
	case "blocked":
		findings := []any{}
		for _, c := range o.Claims {
			findings = append(findings, pyjson.New("severity", "blocking", "where", "src/a.ts", "claim", c))
		}
		verdict = pyjson.New("verdict", "changes", "readable", true, "findings", findings)
	}
	role := "fix"
	if n == 1 {
		role = "build"
	}
	entry := pyjson.New("n", n, "builder", pyjson.New("role", role, "ok", !o.Failed, "error", o.Error,
		"minutes", 12.0, "model", "opus", "timed_out", false),
		"gates", gates)
	if verdict != nil {
		entry.Set("review", verdict)
	}
	return entry
}

// ---------------------------------------------------------------------------- CallErrorTests

// #314: a failed review or self-check call shows its error in the round-by-round table.
func TestFailures_TheReviewersAndTheSelfChecksErrorsArePrinted(t *testing.T) {
	failed := pyjson.New("verdict", "unreadable", "readable", false, "findings", []any{},
		"error", "Your daily allowance is spent")
	rounds := []*pyjson.Object{
		failuresCycle(1, failuresCycleOpts{Review: "none"}).Set("review", failed),
		failuresCycle(2, failuresCycleOpts{Review: "none"}).Set("self_check", []any{
			pyjson.New("n", 1, "review", pyjson.New("verdict", "unreadable", "readable", false,
				"findings", []any{}, "error", "model unavailable"),
				"flagged", 1)}),
	}
	text := failures.Why(rounds, 10, "")
	assertIn(t, "**review unreadable**: Your daily allowance is spent", text)
	assertIn(t, "**self-check call failed**: model unavailable", text)
}

// ---------------------------------------------------------------------------- WhyTests

func TestFailures_EveryRoundUsedSaysWhyRoundByRound(t *testing.T) {
	cycles := []*pyjson.Object{failuresCycle(1, failuresCycleOpts{Red: []string{"typecheck"}}), failuresCycle(2),
		failuresCycle(3, failuresCycleOpts{Claims: []string{"It breaks replay.", "R12 has no test."}})}
	assertTrue(t, failures.Stuck(cycles, 3))
	text := failures.Why(cycles, 3, "no approval after 3 review cycles")
	assertIn(t, "### Why it failed 3 times", text)
	assertIn(t, "**The reviewer blocked 3 of 3 reviewed round(s)**", text)
	assertIn(t, "**Checks were red in 1 round(s):** `typecheck` ×1", text)
	assertNotIn(t, "`unit`", strings.Split(text, "<details>")[0]) // red on main too: not this change's
	assertIn(t, "`src/a.ts`: It breaks replay. (×3)", text)
	assertIn(t, "| 1 | build on `opus`, 12.0 min | red: `typecheck` (exit 2) | blocked", text)
	assertIn(t, "**Builder time:** 36 min over 3 round(s).", text)
}

func TestFailures_ARunThatStoppedSoonerSaysItsRealCountAndReason(t *testing.T) {
	cycles := []*pyjson.Object{failuresCycle(1, failuresCycleOpts{Failed: true, Error: "Individual quota reached.",
		Review: "unreadable"})}
	assertFalse(t, failures.Stuck(cycles, 10))
	text := failures.Why(cycles, 10, "the reviewer's answer could not be read twice in a row")
	assertIn(t, "### Why it stopped after 1 of 10 round(s)", text)
	assertIn(t, "its answer could not be read in 1", text)
	assertIn(t, "**The builder's session failed in 1 round(s)**, most often: Individual "+
		"quota reached.", text)
	assertIn(t, "**review unreadable**", text)
}

func TestFailures_NothingRecordedStillReads(t *testing.T) {
	assertIn(t, "### Why it stopped after 0 of 10 round(s)", failures.Why(nil, 10, ""))
	assertFalse(t, failures.Stuck([]*pyjson.Object{}, 0))
}

// ---------------------------------------------------------------------------- PlannedLabelTests

func TestFailures_APlannedItemCarriesBotPlannedAndLosesItIfItNeedsAPlanAgain(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh)
	pairs := []any{}
	for i, p := range []string{"claude-3", "claude-1", "claude-4", "claude-2"} {
		pairs = append(pairs, p, 50+i)
	}
	difficultyBusy(gh, ctx, pairs...)
	difficultyQueue(gh, ctx, 3, true) // planned (a strong model's, legacy)
	difficultyQueue(gh, ctx, 4, true, "difficulty:medium")
	ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, 4).Update(pyjson.New("planned_at", clock.ISO(testsupport.Night), "planned_tier", "medium"))
		return nil
	}, "state")
	gh.AddLabels(4, []string{config.LabelPlanned}) // a medium item with a medium plan
	plan.Make(ctx, false, nil, "auto", nil)
	assertIn(t, config.LabelPlanned, gh.LabelNames(3))
	assertNotIn(t, config.LabelNeedsPlan, gh.LabelNames(3))
	// A medium plan meets a medium item's floor (#695): it keeps its plan, no new one.
	assertIn(t, config.LabelPlanned, gh.LabelNames(4))
	assertNotIn(t, config.LabelNeedsPlan, gh.LabelNames(4))
}
