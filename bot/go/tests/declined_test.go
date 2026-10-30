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
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// declinedAsk is test_flow.ask: a comment from the operator, through the event handler.
func declinedAsk(h *difficultyHarness, number int, body string, cid int) {
	events.Handle(h.ctx, "issue_comment", pyjson.New(
		"action", "created", "sender", fakes.Operator.Copy(), "issue", pyjson.New("number", number),
		"comment", pyjson.New("id", cid, "body", body, "user", fakes.Operator.Copy(),
			"author_association", "OWNER")))
}

// declinedSurvey is survey(*titles): a survey call that suggests `titles`.
func declinedSurvey(titles ...string) *runner.FakeRunner {
	items := []string{}
	for _, title := range titles {
		items = append(items, pyjson.Dumps(pyjson.New("title", title, "body", "## Why\nx")))
	}
	text := "<!-- suggestions: [" + strings.Join(items, ",") + "] -->"
	return runner.NewFakeRunner(map[string][]runner.Handler{"suggest": {difficultyReviewer(text)}})
}

// declinedPlanRunner is FakeRunner({"plan": lambda r: RunResult(True, text)}).
func declinedPlanRunner(text string) *runner.FakeRunner {
	return runner.NewFakeRunner(map[string][]runner.Handler{"plan": {difficultyReviewer(text)}})
}

// ---------------------------------------------------------------------------- IdleTests

func TestDeclined_QueuedWorkThatWaitsForAnotherSubscriptionNoLongerStopsASurvey(t *testing.T) {
	is := difficultyCheckOf(t)
	// By day only Muse is free, and a hard item needs a strong model: Muse has nothing it can
	// start, so after 30 minutes it surveys, though the queue is not empty.
	h := mediumBiasDayHarness(t, "muse")
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:hard"}})
	look := plan.Peek_(h.ctx, false, nil, "auto")
	is.Equal([]bool{look.Work, look.Idle}, []bool{false, true})
	testsupport.IdleFor(h.ctx, 29)
	is.False(plan.Peek_(h.ctx, false, nil, "auto").Work)
	testsupport.IdleFor(h.ctx, 30)
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	is.Equal([]any{planned.Get("action"), planned.Get("provider"), planned.Get("count")},
		[]any{"suggest", "muse", 10})
}

func TestDeclined_NoSecondSurveyWhileOneRuns(t *testing.T) {
	is := difficultyCheckOf(t)
	h := newDifficultyHarness(t)
	testsupport.IdleFor(h.ctx, 45)
	h.ctx.Store.Update(func(s *pyjson.Object) any {
		s.Obj("suggest").Update(pyjson.New("provider", "claude-3", "run_id", "9"))
		return nil
	}, "state")
	is.Equal(plan.SuggestionsDue(h.ctx, h.ctx.Store.Load(), false), 0)
	is.Equal(plan.SuggestionsDue(h.ctx, h.ctx.Store.Load(), true), 10)
}

func TestDeclined_TheCapStillHolds(t *testing.T) {
	is := difficultyCheckOf(t)
	h := newDifficultyHarness(t)
	for n := 20; n < 30; n++ {
		h.gh.AddIssue(n, fakes.Issue{Labels: []string{config.LabelSuggestion}})
	}
	testsupport.IdleFor(h.ctx, 120)
	is.Equal(plan.Make(h.ctx, false, nil, "auto", nil).Get("action"), "none")
}

func TestDeclined_AnUnfinishedSurveyGivesTheClockBack(t *testing.T) {
	is := difficultyCheckOf(t)
	h := newDifficultyHarness(t)
	testsupport.IdleFor(h.ctx, 45)
	before := h.ctx.Store.Load().Obj("suggest").Get("idle_since")
	planned, _ := h.night(runner.NewFakeRunner(map[string][]runner.Handler{"suggest": {
		func(r *runner.RunRequest) *runner.RunResult {
			result := runner.NewRunResult(false, "")
			result.ExitCode, result.Error = 1, "boom"
			return result
		}}}))
	is.Equal(planned.Get("action"), "suggest")
	is.Equal(h.ctx.Store.Load().Obj("suggest").Get("idle_since"), before)
}

// ---------------------------------------------------------------------------- LogTests

func TestDeclined_ARejectVoteIsLogged(t *testing.T) {
	is := difficultyCheckOf(t)
	h := newDifficultyHarness(t)
	h.gh.AddIssue(21, fakes.Issue{Title: "Patch v0.3.Y: shinier coins", Labels: []string{config.LabelSuggestion}})
	declinedAsk(h, 21, "reject", 61)
	entries := declined.Entries(h.ctx.Store.Load())
	is.Equal(len(entries), 1)
	entry := entries[0]
	is.Equal([]any{entry.Get("number"), entry.Get("title"), entry.Get("why")},
		[]any{21, "Patch v0.3.Y: shinier coins", "rejected by @jgoetzmann"})
}

func TestDeclined_TheSurveyReadsEveryOneAndARepeatIsNotOpened(t *testing.T) {
	is := difficultyCheckOf(t)
	h := newDifficultyHarness(t)
	// Closed by hand long ago (#777's way), and rejected by a vote.
	h.gh.AddIssue(30, fakes.Issue{Title: "Architecture: a fuzz monitor for piles", State: "closed",
		Labels: []string{config.LabelSuggestion}})
	h.gh.Threads.Item(30).Set("state_reason", "not_planned")
	h.gh.AddIssue(31, fakes.Issue{Title: "Patch v0.3.Y: shinier coins", Labels: []string{config.LabelSuggestion}})
	declinedAsk(h, 31, "reject", 62)
	testsupport.IdleFor(h.ctx)
	r := declinedSurvey("Patch v0.3.Y: Shinier coins!", "Patch v0.3.Y: a new idea")
	h.night(r)
	prompt := r.Calls[0].Prompt
	is.In("## Suggestions that came to nothing", prompt)
	is.In("#30 Architecture: a fuzz monitor for piles: closed unbuilt by a person "+
		"(not planned)", prompt)
	is.In("#31 Patch v0.3.Y: shinier coins: rejected by @jgoetzmann", prompt)
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
	is.Equal(made, []string{"Patch v0.3.Y: a new idea"}) // the reworded repeat was dropped
}

func TestDeclined_AnApprovedSuggestionTheBotCannotBuildIsLogged(t *testing.T) {
	is := difficultyCheckOf(t)
	h := mediumBiasDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Title: "Night bot: tune the sweep", Labels: []string{config.LabelSuggestion}})
	declinedAsk(h, 12, "approve", 63)
	h.night(declinedPlanRunner(mediumBiasRatedPlan("medium", []string{"bot/harness/sweep.py"}, "")))
	is.In(config.LabelHumanRequired, h.gh.LabelNames(12))
	entries := declined.Entries(h.ctx.Store.Load())
	is.Equal(len(entries), 1)
	entry := entries[0]
	is.Equal(entry.Get("number"), 12)
	is.True(strings.HasPrefix(pyjson.Str(entry.Get("why")), "the bot could not build it: Every file its plan "+
		"touches"), entry.Get("why"))
}

func TestDeclined_APlainIssueHandedToPeopleIsNotADeclinedSuggestion(t *testing.T) {
	is := difficultyCheckOf(t)
	h := mediumBiasDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Title: "Night bot: tune the sweep", Labels: []string{config.LabelBuild}})
	h.night(declinedPlanRunner(mediumBiasRatedPlan("medium", []string{"bot/harness/sweep.py"}, "")))
	is.In(config.LabelHumanRequired, h.gh.LabelNames(12))
	is.Equal(declined.Entries(h.ctx.Store.Load()), []*pyjson.Object{})
}

func TestDeclined_TheLogKeepsOneEntryEachAndTheNewest(t *testing.T) {
	is := difficultyCheckOf(t)
	s := pyjson.New("suggest", pyjson.New())
	for n := range declined.Limit + 5 {
		declined.Note(s, n, "Idea "+strconv.Itoa(n), "rejected",
			clock.ISO(clock.ParseISO("2026-10-13T00:00:00Z").Add(time.Duration(n)*time.Minute)))
	}
	is.False(declined.Note(s, declined.Limit+4, "Again", "rejected", "now"))
	log := declined.Entries(s)
	is.Equal([]any{len(log), log[0].Get("number")}, []any{declined.Limit, 5})
	is.True(declined.Known(s, "idea 7"))
	is.False(declined.Known(s, "idea 4")) // dropped as the oldest
}
