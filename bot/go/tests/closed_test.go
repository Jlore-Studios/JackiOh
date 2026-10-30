// An issue closed while a run works on it: the run keeps its lane until it ends, its work is not
// delivered, and nothing queues it again.
//
// bot/tests/test_closed.py, ported (PORTING.md §7).
package tests

import (
	"fmt"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// closedAll is test_cross's ALL, which test_closed imports: every subscription's secret set.
// (cross_test.go holds the twin of test_cross itself.)
func closedAll() map[string]string {
	return map[string]string{"HARNESS_SECRETS_SET": strings.Join(providers.SecretNames, " ")}
}

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
		testsupport.Config{Env: closedAll(), Machine: testsupport.Machine})})
	flowUpdate(ctx, func(s *pyjson.Object) {
		for _, run := range closedRuns {
			state.Item(s, run.number).Update(pyjson.New("run_id", run.runID, "provider", run.provider))
		}
	})
	planned := flowMake(ctx)
	flowEq(t, planned.Get("action"), "none")
	flowIn(t, "`gpt` is busy", planned.Str("reason")) // its lane is still the closed #5's
	text := status.Report(ctx)
	// Its six runs are on the machine, whose slots are apart from GitHub's lanes.
	flowIn(t, "0 of 8 lanes on GitHub's runners, 8 free; 6 of "+
		fmt.Sprintf("%d on the night box", ctx.Cfg.Pool.MachineParallel), text)
	flowIn(t, "`gpt` (codex", text)
}

func TestClosed_ItsWorkIsNotDelivered(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: closedAll()})
	h.GH.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: flowLabels(config.LabelBuild)})
	build := workBuilder(map[string]any{"src/game.txt": "rules v2\n"})
	closedMeanwhile := func(request *runner.RunRequest) *runner.RunResult {
		h.GH.Threads.Item(12).Set("state", "closed")
		return build(request)
	}
	_, result := h.Night(workFake(map[string]any{"build": closedMeanwhile, "review": workReviewer(workApprove)}))
	flowEq(t, result.Get("status"), "approved") // the run itself finished its work
	flowEq(t, len(h.GH.ListPulls("open", "bot/issue-12", 200)), 0)
	flowEq(t, h.OriginSHA("bot/issue-12"), "")
	labels := h.GH.LabelNames(12)
	flowFalse(t, labels[config.LabelWorking] || labels[config.LabelBuild], labels)
	flowIn(t, "was closed while a run was working on it", flowLast(t, h.GH.BotComments(12)))
}

func TestClosed_ADeadRunsLabelIsClearedNotRequeued(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5, fakes.Issue{Labels: flowLabels(config.LabelWorking), State: "closed"})
	gh.Runs.Set("51", pyjson.New("status", "completed", "conclusion", "cancelled"))
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{Cfg: testsupport.MakeConfig(
		testsupport.Config{Env: closedAll(), Machine: testsupport.Machine})})
	flowUpdate(ctx, func(s *pyjson.Object) {
		state.Item(s, 5).Update(pyjson.New("run_id", "51", "provider", "gpt"))
	})
	flowIn(t, "#5", plan.HousekeepingDue(ctx, ctx.Store.Load()))
	notes := plan.Housekeeping(ctx, ctx.Store.Load())
	flowEq(t, gh.LabelNames(5), flowSet())
	flowEq(t, gh.BotComments(5), []string{}) // no "back in the queue"
	found := false
	for _, note := range notes {
		found = found || strings.Contains(note, "#5 was closed")
	}
	flowTrue(t, found, notes)
}
