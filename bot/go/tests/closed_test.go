// An issue closed while a run works on it: the run keeps its lane until it ends, its work is not
// delivered, and nothing queues it again.
//
// bot/tests/test_closed.py, ported (#467).
package tests

import (
	"fmt"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// closedRuns is RUNS: runs holding every lane of the machine's subscriptions (Muse has four);
// someone closed #5 while its run went on.
var closedRuns = []struct {
	number   int
	provider string
	runID    string
}{{5, "gpt", "51"}, {6, "agy", "61"}, {7, "muse", "71"}, {9, "muse", "91"},
	{10, "muse", "101"}, {11, "muse", "111"}}

func TestClosed_ItsRunKeepsItsLane(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	for _, run := range closedRuns {
		issueState := "open"
		if run.number == 5 {
			issueState = "closed"
		}
		gh.AddIssue(run.number, fakes.Issue{Labels: flowLabels(config.LabelWorking), State: issueState})
		gh.Runs.Set(run.runID, pyjson.New("status", "in_progress"))
	}
	gh.AddIssue(8, fakes.Issue{Labels: flowLabels(config.LabelBuild)})
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, Cfg: testsupport.MakeConfig(
		testsupport.Config{Env: crossAll, Machine: testsupport.Machine})})
	flowUpdate(ctx, func(s *pyjson.Object) {
		for _, run := range closedRuns {
			state.Item(s, run.number).Update(pyjson.New("run_id", run.runID, "provider", run.provider))
		}
	})
	planned := flowMake(ctx)
	assertEqual(t, planned.Get("action"), "none")
	assertIn(t, "`gpt` is busy", planned.Str("reason")) // its lane is still the closed #5's
	text := status.Report(ctx)
	// Its six runs are on the machine, whose slots are apart from GitHub's lanes.
	assertIn(t, "0 of 8 lanes on GitHub's runners, 8 free; 6 of "+
		fmt.Sprintf("%d on the night box", ctx.Cfg.Pool.MachineParallel), text)
	assertIn(t, "`gpt` (codex", text)
}

func TestClosed_ItsWorkIsNotDelivered(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll})
	h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: flowLabels(config.LabelBuild)})
	build := workBuilder(map[string]any{"src/game.txt": "rules v2\n"})
	closedMeanwhile := func(request *runner.RunRequest) *runner.RunResult {
		h.gh.Threads.Item(12).Set("state", "closed")
		return build(request)
	}
	_, result := h.night(workFake(map[string]any{"build": closedMeanwhile, "review": workReviewer(workApprove)}))
	assertEqual(t, result.Get("status"), "approved") // the run itself finished its work
	assertEqual(t, len(h.gh.ListPulls("open", "bot/issue-12", 200)), 0)
	assertEqual(t, h.originSHA("bot/issue-12"), "")
	labels := h.gh.LabelNames(12)
	assertFalse(t, labels[config.LabelWorking] || labels[config.LabelBuild], labels)
	assertIn(t, "was closed while a run was working on it", lastOf(t, h.gh.BotComments(12)))
}

func TestClosed_ADeadRunsLabelIsClearedNotRequeued(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5, fakes.Issue{Labels: flowLabels(config.LabelWorking), State: "closed"})
	gh.Runs.Set("51", pyjson.New("status", "completed", "conclusion", "cancelled"))
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{Cfg: testsupport.MakeConfig(
		testsupport.Config{Env: crossAll, Machine: testsupport.Machine})})
	flowUpdate(ctx, func(s *pyjson.Object) {
		state.Item(s, 5).Update(pyjson.New("run_id", "51", "provider", "gpt"))
	})
	assertIn(t, "#5", plan.HousekeepingDue(ctx, ctx.Store.Load()))
	notes := plan.Housekeeping(ctx, ctx.Store.Load())
	assertEqual(t, gh.LabelNames(5), setOf())
	assertEqual(t, gh.BotComments(5), []string{}) // no "back in the queue"
	found := false
	for _, note := range notes {
		found = found || strings.Contains(note, "#5 was closed")
	}
	assertTrue(t, found, notes)
}
