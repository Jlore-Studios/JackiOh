// #317 part 12, what was left of #160: the state file pruned, fix passes at their own effort,
// and `main` merged again before a review when it moved and merges cleanly.
//
// bot/tests/test_leftovers.py, ported (#467).
package tests

import (
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// ---------------------------------------------------------------------------- PruneTests

func TestLeftovers_ClosedItemsAreDroppedAfterAWeekAndLightenedBefore(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(1, fakes.Issue{State: "closed"})
	gh.Threads.Item(1).Set("closed_at", clock.ISO(testsupport.Night.Add(-8*24*time.Hour)))
	gh.AddIssue(2, fakes.Issue{State: "closed"})
	gh.Threads.Item(2).Set("closed_at", clock.ISO(testsupport.Night.Add(-1*24*time.Hour)))
	gh.AddIssue(3)
	flowUpdate(ctx, func(s *pyjson.Object) {
		for _, n := range []int{1, 2, 3} {
			state.Item(s, n).Update(pyjson.New("handoff", pyjson.New("notes", strings.Repeat("x", 9000)),
				"failures", 1))
		}
	})
	notes := plan.Prune(ctx, ctx.Store.Load())
	assertEqual(t, notes, []string{"pruned the state file: dropped 1 closed item(s), lightened 1"})
	items := ctx.Store.Load().Obj("items")
	assertFalse(t, items.Has("1"))
	assertEqual(t, dig(t, items, "2"), pyjson.New("failures", 1))
	assertTrue(t, pyjson.Obj(dig(t, items, "3")).Has("handoff")) // open: untouched
	// Once a day.
	gh.Threads.Item(3).Set("state", "closed")
	assertEqual(t, plan.Prune(ctx, ctx.Store.Load()), []string{})
}

// ---------------------------------------------------------------------------- FixEffortTests

func TestLeftovers_AFixPassRunsAtTheFixEffort(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Machine: []string{}})
	h.gh.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelBuild)})
	r := workFake(map[string]any{"build": workBuilder(map[string]any{"src/game.txt": "v2\n"}),
		"fix":    workBuilder(map[string]any{"src/game.txt": "v3\n"}),
		"review": []runner.Handler{workReviewer(workChanges("Not yet.")), workReviewer(workApprove)}})
	h.night(r)
	efforts := map[string]string{}
	for _, c := range r.Calls {
		efforts[c.Role] = c.Effort
	}
	// Builds run at the seat's effort, fix and review one notch down (#695).
	assertEqual(t, []string{efforts["build"], efforts["fix"], efforts["review"]}, []string{"xhigh", "high", "high"})
}

// ---------------------------------------------------------------------------- CatchUpTests

func TestLeftovers_MainThatMovedDuringTheBuildIsMergedBeforeTheReview(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Machine: []string{}})
	h.gh.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelBuild)})
	moved := map[string]string{}
	building := func(request *runner.RunRequest) *runner.RunResult {
		moved["main"] = fakes.PushBranch(h.origin, h.root, "main", map[string]string{"README.md": "news\n"})
		return workBuilder(map[string]any{"src/game.txt": "v2\n"})(request)
	}
	seen := map[string]string{}
	reviewing := func(request *runner.RunRequest) *runner.RunResult {
		seen["readme"] = readText(t, filepath.Join(request.Cwd, "README.md"))
		return workReviewer(workApprove)(request)
	}
	_, result := h.night(workFake(map[string]any{"build": building, "review": reviewing}))
	assertEqual(t, result.Get("status"), "approved")
	assertEqual(t, dig(t, result, "cycles", 0, "caught_up"), moved["main"])
	assertEqual(t, seen["readme"], "news\n")
	head := h.originSHA("bot/issue-12")
	assertEqual(t, fakes.Git(h.origin, "merge-base", "--is-ancestor", moved["main"], head), "")
}

func TestLeftovers_AMainThatWouldConflictIsLeftForTheConflictPath(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Machine: []string{}})
	h.gh.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelBuild)})
	building := func(request *runner.RunRequest) *runner.RunResult {
		fakes.PushBranch(h.origin, h.root, "main", map[string]string{"src/game.txt": "rules from main\n"})
		return workBuilder(map[string]any{"src/game.txt": "v2\n"})(request)
	}
	_, result := h.night(workFake(map[string]any{"build": building, "review": workReviewer(workApprove)}))
	assertEqual(t, result.Get("status"), "approved")
	assertFalse(t, pyjson.Obj(dig(t, result, "cycles", 0)).Has("caught_up"))
}
