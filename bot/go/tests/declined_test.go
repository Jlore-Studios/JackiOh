// #797: a survey once a free lane has had nothing to start for 30 minutes, and a log of the
// suggestions that came to nothing, which no survey offers again. (bot/tests/test_declined.py)
package tests

import (
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/declined"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// declinedSurvey is survey(*titles): a survey call that suggests `titles`.
func declinedSurvey(titles ...string) *runner.FakeRunner {
	items := []string{}
	for _, title := range titles {
		items = append(items, pyjson.Dumps(pyjson.New("title", title, "body", "## Why\nx")))
	}
	text := "<!-- suggestions: [" + strings.Join(items, ",") + "] -->"
	return runner.NewFakeRunner(map[string][]runner.Handler{"suggest": {workReviewer(text)}})
}

// declinedPlanRunner is FakeRunner({"plan": lambda r: RunResult(True, text)}).
func declinedPlanRunner(text string) *runner.FakeRunner {
	return runner.NewFakeRunner(map[string][]runner.Handler{"plan": {workReviewer(text)}})
}

// ---------------------------------------------------------------------------- IdleTests

func TestDeclined_QueuedWorkThatWaitsForAnotherSubscriptionNoLongerStopsASurvey(t *testing.T) {
	// By day only Muse is free, and a hard item needs a strong model: Muse has nothing it can
	// start, so after 30 minutes it surveys, though the queue is not empty.
	h := ratingDayHarness(t, "muse")
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:hard"}})
	look := plan.Peek_(h.ctx, false, nil, "auto")
	assertEqual(t, []bool{look.Work, look.Idle}, []bool{false, true})
	testsupport.IdleFor(h.ctx, 29)
	assertFalse(t, plan.Peek_(h.ctx, false, nil, "auto").Work)
	testsupport.IdleFor(h.ctx, 30)
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider"), planned.Get("count")},
		[]any{"suggest", "muse", 10})
}

func TestDeclined_NoSecondSurveyWhileOneRuns(t *testing.T) {
	h := newFlowHarness(t)
	testsupport.IdleFor(h.ctx, 45)
	h.ctx.Store.Update(func(s *pyjson.Object) any {
		s.Obj("suggest").Update(pyjson.New("provider", "claude-3", "run_id", "9"))
		return nil
	}, "state")
	assertEqual(t, plan.SuggestionsDue(h.ctx, h.ctx.Store.Load(), false), 0)
	assertEqual(t, plan.SuggestionsDue(h.ctx, h.ctx.Store.Load(), true), 10)
}

func TestDeclined_TheCapStillHolds(t *testing.T) {
	h := newFlowHarness(t)
	for n := 20; n < 30; n++ {
		h.gh.AddIssue(n, fakes.Issue{Labels: []string{config.LabelSuggestion}})
	}
	testsupport.IdleFor(h.ctx, 120)
	assertEqual(t, plan.Make(h.ctx, false, nil, "auto", nil).Get("action"), "none")
}

func TestDeclined_AnUnfinishedSurveyGivesTheClockBack(t *testing.T) {
	h := newFlowHarness(t)
	testsupport.IdleFor(h.ctx, 45)
	before := h.ctx.Store.Load().Obj("suggest").Get("idle_since")
	planned, _ := h.night(runner.NewFakeRunner(map[string][]runner.Handler{"suggest": {
		func(r *runner.RunRequest) *runner.RunResult {
			result := runner.NewRunResult(false, "")
			result.ExitCode, result.Error = 1, "boom"
			return result
		}}}))
	assertEqual(t, planned.Get("action"), "suggest")
	assertEqual(t, h.ctx.Store.Load().Obj("suggest").Get("idle_since"), before)
}

// ---------------------------------------------------------------------------- LogTests

func TestDeclined_ARejectVoteIsLogged(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(21, fakes.Issue{Title: "Patch v0.3.Y: shinier coins", Labels: []string{config.LabelSuggestion}})
	flowAsk(h, 21, "reject", 61)
	entries := declined.Entries(h.ctx.Store.Load())
	assertEqual(t, len(entries), 1)
	entry := entries[0]
	assertEqual(t, []any{entry.Get("number"), entry.Get("title"), entry.Get("why")},
		[]any{21, "Patch v0.3.Y: shinier coins", "rejected by @jgoetzmann"})
}

func TestDeclined_TheSurveyReadsEveryOneAndARepeatIsNotOpened(t *testing.T) {
	h := newFlowHarness(t)
	// Closed by hand long ago (#777's way), and rejected by a vote.
	h.gh.AddIssue(30, fakes.Issue{Title: "Architecture: a fuzz monitor for piles", State: "closed",
		Labels: []string{config.LabelSuggestion}})
	h.gh.Threads.Item(30).Set("state_reason", "not_planned")
	h.gh.AddIssue(31, fakes.Issue{Title: "Patch v0.3.Y: shinier coins", Labels: []string{config.LabelSuggestion}})
	flowAsk(h, 31, "reject", 62)
	testsupport.IdleFor(h.ctx)
	r := declinedSurvey("Patch v0.3.Y: Shinier coins!", "Patch v0.3.Y: a new idea")
	h.night(r)
	prompt := r.Calls[0].Prompt
	assertIn(t, "## Suggestions that came to nothing", prompt)
	assertIn(t, "#30 Architecture: a fuzz monitor for piles: closed unbuilt by a person "+
		"(not planned)", prompt)
	assertIn(t, "#31 Patch v0.3.Y: shinier coins: rejected by @jgoetzmann", prompt)
	made := []string{}
	for _, thread := range h.gh.Threads.Values() {
		labelled := false
		for _, label := range thread.List("labels") {
			if pyjson.Obj(label).Get("name") == config.LabelSuggestion {
				labelled = true
			}
		}
		if labelled && thread.Get("state") == "open" {
			made = append(made, pyjson.Str(thread.Get("title")))
		}
	}
	assertEqual(t, made, []string{"Patch v0.3.Y: a new idea"}) // the reworded repeat was dropped
}

func TestDeclined_AnApprovedSuggestionTheBotCannotBuildIsLogged(t *testing.T) {
	h := ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Title: "Night bot: tune the sweep", Labels: []string{config.LabelSuggestion}})
	flowAsk(h, 12, "approve", 63)
	h.night(declinedPlanRunner(ratingRatedPlan("medium", []string{"bot/harness/sweep.py"}, "")))
	assertIn(t, config.LabelHumanRequired, h.gh.LabelNames(12))
	entries := declined.Entries(h.ctx.Store.Load())
	assertEqual(t, len(entries), 1)
	entry := entries[0]
	assertEqual(t, entry.Get("number"), 12)
	assertTrue(t, strings.HasPrefix(pyjson.Str(entry.Get("why")), "the bot could not build it: Every file its plan "+
		"touches"), entry.Get("why"))
}

func TestDeclined_APlainIssueHandedToPeopleIsNotADeclinedSuggestion(t *testing.T) {
	h := ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Title: "Night bot: tune the sweep", Labels: []string{config.LabelBuild}})
	h.night(declinedPlanRunner(ratingRatedPlan("medium", []string{"bot/harness/sweep.py"}, "")))
	assertIn(t, config.LabelHumanRequired, h.gh.LabelNames(12))
	assertEqual(t, declined.Entries(h.ctx.Store.Load()), []*pyjson.Object{})
}

func TestDeclined_TheLogKeepsOneEntryEachAndTheNewest(t *testing.T) {
	s := pyjson.New("suggest", pyjson.New())
	for n := range declined.Limit + 5 {
		declined.Note(s, n, "Idea "+strconv.Itoa(n), "rejected",
			clock.ISO(clock.ParseISO("2026-10-13T00:00:00Z").Add(time.Duration(n)*time.Minute)))
	}
	assertFalse(t, declined.Note(s, declined.Limit+4, "Again", "rejected", "now"))
	log := declined.Entries(s)
	assertEqual(t, []any{len(log), log[0].Get("number")}, []any{declined.Limit, 5})
	assertTrue(t, declined.Known(s, "idea 7"))
	assertFalse(t, declined.Known(s, "idea 4")) // dropped as the oldest
}
