package tests

// A pull request settles its merge conflicts before it goes ready, and the mechanical ones resolve
// themselves (#2948, the owner's decision of 2026-11-03): the labels (`bot:resolving`,
// `bot:waiting-turn`), run 2's final check (`work/settle.go`, `deliver/settle.go`): its test-merges
// with `main` and after the open pull requests ahead of it, the wait for its turn, its CI read again;
// the lane that owns a pull request (a conflict goes to its merge owner first, and a person-merged one
// gives that up once its CI is green); Mergiraf and the `union` resolver. Through plan, work and
// deliver against real repositories and the fake GitHub, with the committed chain
// (`ownReviewHarness`).

import (
	"archive/tar"
	"bytes"
	"compress/gzip"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/resolve"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/sweep"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// settleRecord is pull request `number`'s record in the state file.
func settleRecord(t *testing.T, h *flowHarness, number int) *pyjson.Object {
	t.Helper()
	return crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(number))
}

// settleOther opens bot pull request `number` from `branch`, which changes `files` on `main`, its
// head also at `refs/pull/<n>/head` as GitHub keeps it, and, with `cleared`, records its head as
// approved `ago` before now: a pull request ahead of the one under test when it conflicts with it.
func settleOther(t *testing.T, h *flowHarness, number int, branch string, files map[string]string,
	cleared bool, ago time.Duration, labels ...string) string {
	t.Helper()
	sha := fakes.PushBranch(h.origin, h.root, branch, files)
	fakes.Git(h.origin, "update-ref", fmt.Sprintf("refs/pull/%d/head", number), sha)
	h.gh.AddPull(number, branch, fakes.Pull{Labels: append([]string{config.LabelPR}, labels...), SHA: sha})
	if cleared {
		flowUpdateItem(h.ctx, number, pyjson.New("cleared", pyjson.New("sha", sha,
			"at", clock.ISO(h.ctx.Now().Add(-ago)), "by", "agy (gemini, medium)"), "merge_owner", "agy"))
	}
	return sha
}

// ---------------------------------------------------------------------------- the labels

// Both labels are the bot's own (`config.Labels`), within GitHub's length: `bot:resolving` is the
// live label of a revision that resolves a merge conflict, beside `bot:fixing-ci`, and
// `bot:waiting-turn` a state label, which any other state takes off. The status and the lanes table
// say what a resolution does.
func TestSettle_TheLabelsAreWhereTheyBelong(t *testing.T) {
	for _, name := range []string{config.LabelResolving, config.LabelWaitingTurn} {
		assertIn(t, name, config.Labels)
		assertTrue(t, len([]rune(pyjson.Str(pyjson.List(config.Labels.Get(name))[1]))) <= config.LabelDescriptionMax)
	}
	assertEqual(t, []string{config.LabelResolving, config.LabelWaitingTurn}, []string{"bot:resolving", "bot:waiting-turn"})
	assertEqual(t, config.LiveRun("revise", "revise", "", "conflict", false, true), "resolving")
	assertIn(t, config.LabelResolving, config.LiveLabels)
	assertTrue(t, config.IsLive(setOf(config.LabelResolving)))
	assertFalse(t, config.IsLive(setOf(config.LabelWaitingTurn)))
	assertIn(t, config.LabelWaitingTurn, queue.StateLabels)
	assertEqual(t, status.DoingOf(pyjson.New("live", "resolving")), "resolving the conflicts of")
	for _, doc := range [][]string{{"docs", "issues-and-patches.md"}, {"bot", "README.md"}} {
		text := readText(t, filepath.Join(append([]string{testsupport.Root}, doc...)...))
		for _, name := range []string{config.LabelResolving, config.LabelWaitingTurn} {
			assertIn(t, "`"+name+"`", text, doc)
		}
	}
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddPull(9, "bot/issue-1", fakes.Pull{Labels: []string{config.LabelPR, config.LabelWaitingTurn}})
	queue.SetStateLabel(ctx, 9, gh.LabelNames(9), config.LabelRevise)
	assertEqual(t, gh.LabelNames(9), setOf(config.LabelPR, config.LabelRevise))
}

// ---------------------------------------------------------------------------- run 2's final check

// A draft that merges cleanly with `main`, and overlaps no other open pull request, goes ready with
// auto-merge on, and `main` is not merged into it: the merge queue tests the two together, and a
// merge would only start CI again.
func TestSettle_ACleanTestMergeGoesReadyWithoutMergingMain(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, built := ownReviewBuild(t, h, "")
	moved := fakes.PushBranch(h.origin, h.root, "main", map[string]string{"README.md": "news\n"})
	r := workFake(map[string]runner.Handler{"review": workReviewer(workApprove)})
	_, result := h.night(r)
	assertEqual(t, workRoles(r), []string{"review"})
	assertEqual(t, []any{result.Get("status"), result.Get("head")}, []any{"approved", built.Get("head")})
	assertEqual(t, dig(t, result, "settle", "main", "state"), "clean")
	assertEqual(t, dig(t, result, "settle", "main", "sha"), moved)
	assertEqual(t, dig(t, result, "settle", "pulls"), []any{})
	assertEqual(t, h.originSHA("bot/issue-12"), built.Get("head")) // nothing pushed: main stays out
	assertEqual(t, h.gh.GetPull(pr).Get("draft"), false)
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertIn(t, "clean, so `main` is not merged in", lastOf(t, h.gh.BotComments(pr)))
}

// A draft that conflicts with `main` merges it once: the conflicts the harness cannot settle go to a
// fix pass of the same model, the checks run again, and the reviewer reviews the merge only; the
// merged head is pushed, approved and ready, with auto-merge on.
func TestSettle_AConflictWithMainIsMergedAndResolvedOnce(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, built := ownReviewBuild(t, h, "")
	moved := fakes.PushBranch(h.origin, h.root, "main", map[string]string{"src/game.txt": "rules from main\n"})
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"review": {workReviewer(workApprove), workReviewer(workApprove)},
		"fix":    {workBuilder(map[string]string{"src/game.txt": "rules v2, and main's\n"})}})
	_, result := h.night(r)
	assertEqual(t, workRoles(r), []string{"review", "fix", "review"})
	assertEqual(t, ownReviewCalls(r)[1][1], ownReviewCalls(r)[0][1]) // the same model fixes it
	assertIn(t, "The final check merged `main` (`"+moved[:12]+"`) into the approved head", ownReviewPrompt(t, r, "fix"))
	assertIn(t, "`src/game.txt`", ownReviewPrompt(t, r, "fix"))
	second := r.Calls[2].Prompt
	assertIn(t, "Review only that merge", second)
	assertIn(t, "you approved `"+crossStr(t, built, "head")[:12]+"`", second)
	assertEqual(t, result.Get("status"), "approved")
	assertEqual(t, dig(t, result, "settle", "main", "state"), "merged")
	assertEqual(t, dig(t, result, "settle", "main", "conflicts"), []any{"src/game.txt"})
	assertEqual(t, dig(t, result, "cycles", 0, "final_check"), "`main` conflicted; the builder resolves what the harness left")
	head := crossStr(t, result, "head")
	assertEqual(t, h.originSHA("bot/issue-12"), head)
	assertEqual(t, fakes.Git(h.origin, "merge-base", "--is-ancestor", moved, head), "")
	assertEqual(t, fakes.Git(h.origin, "show", head+":src/game.txt"), "rules v2, and main's")
	assertEqual(t, h.gh.GetPull(pr).Get("draft"), false)
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertEqual(t, h.gh.AutoMergeHeads[fmt.Sprintf("PR_%d", pr)], head)
	assertIn(t, "Its final check found it conflicts with `main`", lastOf(t, h.gh.BotComments(pr)))
	// Once: the merge's own review is the last of it, and nothing more is queued.
	crossNextRun(h, "779")
	planned, _ := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, planned.Get("action"), "none")
}

// A conflict with `main` the harness settles itself (a resolver of its config) costs no builder
// session: the harness commits the merge, the checks run, and the reviewer reviews it.
func TestSettle_AMergeTheHarnessSettlesSpendsNoBuilderSession(t *testing.T) {
	h := ownReviewHarness(t, nil, pyjson.New("resolvers", []any{pyjson.New("path", "src/list.txt", "take", "union")}))
	h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: []string{config.LabelBuild}})
	r := workFake(map[string]runner.Handler{"build": crossTimedBuilder(map[string]string{
		"src/game.txt": "rules v2\n", "src/list.txt": "one\nbranch\n"})})
	h.night(r)
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	crossNextRun(h, "778")
	fakes.PushBranch(h.origin, h.root, "main", map[string]string{"src/list.txt": "one\nmain\n"})
	review := runner.NewFakeRunner(map[string][]runner.Handler{
		"review": {workReviewer(workApprove), workReviewer(workApprove)}})
	_, result := h.night(review)
	assertEqual(t, workRoles(review), []string{"review", "review"})
	assertEqual(t, dig(t, result, "cycles", 1, "builder", "role"), "merge")
	assertIn(t, "kept both sides' lines (union)", review.Calls[1].Prompt)
	assertEqual(t, result.Get("status"), "approved")
	assertEqual(t, fakes.Git(h.origin, "show", h.originSHA("bot/issue-12")+":src/list.txt"), "one\nbranch\nmain")
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
}

// The draft's CI, read at the run's start while still running, is read again before it goes ready:
// a required job red now is fixed in the same run, the checks and a fresh review again, and that
// counts as a CI fix toward `max_failures`.
func TestSettle_RedCIAtTheEndOfRunTwoIsFixedInTheSameRun(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, built := ownReviewBuild(t, h, "")
	h.gh.Runs.Set("900", pyjson.New("id", 900, "name", "CI", "workflow", "ci.yml", "head_sha", built.Get("head"),
		"status", "in_progress", "conclusion", nil, "created_at", "2026-09-29T17:05:00Z",
		"html_url", "https://github.com/jgoetzmann/JackiOh/actions/runs/900"))
	h.gh.Jobs["900"] = []*pyjson.Object{pyjson.New("id", 9001, "name", "web (unit 1/2)", "conclusion", nil,
		"steps", []any{})}
	turnsRed := func(request *runner.RunRequest) *runner.RunResult {
		// CI ends red while the first review reads the change.
		h.gh.Runs.Obj("900").Set("status", "completed").Set("conclusion", "failure")
		h.gh.Jobs["900"] = []*pyjson.Object{pyjson.New("id", 9001, "name", "web (unit 1/2)",
			"conclusion", "failure", "steps", []any{pyjson.New("name", "vitest", "conclusion", "failure")})}
		return runner.NewRunResult(true, workApprove)
	}
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"review": {turnsRed, workReviewer(workApprove)},
		"fix":    {workBuilder(map[string]string{"src/game.txt": "rules v2, the unit test fixed\n"})}})
	_, result := h.night(r)
	assertEqual(t, workRoles(r), []string{"review", "fix", "review"})
	assertIn(t, "(still running)", crossStr(t, result, "cycles", 0, "ci", "read"))
	assertIn(t, "The required check `web (unit 1/2)` is red in CI", ownReviewPrompt(t, r, "fix"))
	assertIn(t, "log of job 9001", ownReviewPrompt(t, r, "fix"))
	assertEqual(t, dig(t, result, "settle", "ci", "red"), []any{"web (unit 1/2)"})
	assertEqual(t, dig(t, result, "settle", "ci", "fixed"), true)
	assertEqual(t, result.Get("status"), "approved")
	record := settleRecord(t, h, pr)
	assertEqual(t, []any{record.Get("ci_fixes"), record.Get("ci_fixed_head")}, []any{1, result.Get("head")})
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertIn(t, "which counts as a CI fix", lastOf(t, h.gh.BotComments(pr)))
}

// CI still running at the end of run 2 blocks nothing: it goes ready, and red CI later comes back
// to its lane by events. A CI fix the item has used up is not made in the run either.
func TestSettle_CIStillRunningAtTheEndGoesReady(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, built := ownReviewBuild(t, h, "")
	h.gh.Runs.Set("900", pyjson.New("id", 900, "name", "CI", "workflow", "ci.yml", "head_sha", built.Get("head"),
		"status", "in_progress", "conclusion", nil, "created_at", "2026-09-29T17:05:00Z"))
	h.gh.Jobs["900"] = []*pyjson.Object{}
	r := workFake(map[string]runner.Handler{"review": workReviewer(workApprove)})
	_, result := h.night(r)
	assertEqual(t, workRoles(r), []string{"review"})
	assertIn(t, "(still running)", crossStr(t, result, "settle", "ci", "read"))
	assertEqual(t, result.Get("status"), "approved")
	assertNotIn(t, "ci_fixes", settleRecord(t, h, pr))
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
}

// ---------------------------------------------------------------------------- waiting its turn

// settleWaiting is run 2 of issue 12's draft (src/game.txt "rules v2") with #40 ahead of it: open,
// approved an hour before, conflict-free with `main`, and changing src/game.txt its own way. The
// draft waits its turn behind #40.
func settleWaiting(t *testing.T, h *flowHarness) (int, *pyjson.Object) {
	t.Helper()
	pr, _ := ownReviewBuild(t, h, "")
	settleOther(t, h, 40, "feature/a", map[string]string{"src/game.txt": "rules from #40\n"}, true, time.Hour)
	r := workFake(map[string]runner.Handler{"review": workReviewer(workApprove)})
	_, result := h.night(r)
	assertEqual(t, result.Get("status"), "approved")
	return pr, result
}

// A draft that conflicts once a pull request ahead of it lands (approved earlier, conflict-free)
// waits its turn: approved, still a draft, auto-merge off, `bot:waiting-turn`, its lane's
// subscription and model on record, up to `hold_minutes`; no lane is held meanwhile.
func TestSettle_AConflictWithAPullRequestAheadWaitsItsTurn(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, result := settleWaiting(t, h)
	pulls := pyjson.Objs(dig(t, result, "settle", "pulls"))
	assertEqual(t, len(pulls), 1)
	assertEqual(t, []any{pulls[0].Get("number"), pulls[0].Get("overlap"), pulls[0].Get("conflicts")},
		[]any{40, []any{"src/game.txt"}, []any{"src/game.txt"}})
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelWaitingTurn, "difficulty:medium"))
	assertEqual(t, h.gh.GetPull(pr).Get("draft"), true)
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	record := settleRecord(t, h, pr)
	waiting := crossObj(t, record, queue.WaitingTurnKey)
	assertEqual(t, []any{waiting.Get("provider"), waiting.Get("model"), waiting.Get("behind"), waiting.Get("head")},
		[]any{"muse", "muse-spark-1.3-contributor", []any{40}, result.Get("head")})
	ends := clock.ParseISOValue(waiting.Get("ends")).Sub(clock.ParseISOValue(waiting.Get("since")))
	assertEqual(t, ends, time.Duration(h.cfg.HoldMinutes)*time.Minute)
	assertEqual(t, h.cfg.HoldMinutes, 120)
	assertEqual(t, dig(t, record, "cleared", "sha"), result.Get("head")) // approved, on record
	said := lastOf(t, h.gh.BotComments(pr))
	assertIn(t, "it conflicts once #40 land(s)", said)
	assertIn(t, "`bot:waiting-turn`", said)
	lanes := plan.ReadLanes(h.ctx, h.ctx.Store.Load())
	assertNotIn(t, pr, lanes.Held)
	assertIn(t, fmt.Sprintf("- Waiting their turn (`bot:waiting-turn`): #%d behind #40, on `muse` (until ", pr),
		status.Report(h.ctx))
	// Nothing takes it while #40 is open and its wait lasts.
	crossNextRun(h, "779")
	planned, _ := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, planned.Get("action"), "none")
	assertIn(t, config.LabelWaitingTurn, h.gh.LabelNames(pr))
}

// When the pull request ahead merges, the same subscription and model take the draft up again in a
// fresh session: a conflict revision (`bot:resolving` while it runs) with its lane kept for it. It
// merges `main` and resolves the conflict, and a resolution that changes only the conflicted files
// keeps the clearance: it goes ready with auto-merge on, with no further review run.
func TestSettle_ItsTurnComesWhenThePullRequestAheadMergesAndTheClearanceCarries(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, approved := settleWaiting(t, h)
	// #40 merges: its change is on `main`.
	fakes.PushBranch(h.origin, h.root, "main", map[string]string{"src/game.txt": "rules from #40\n"})
	h.gh.Threads.Item(40).Set("state", "closed").Set("merged", true)
	notes := events.Handle(h.ctx, "pull_request", pyjson.New("action", "closed",
		"pull_request", h.gh.GetPull(40)))
	assertIn(t, fmt.Sprintf("#%d: its turn came", pr), strings.Join(notes, "\n"))
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelRevise, "difficulty:medium"))
	record := settleRecord(t, h, pr)
	assertNotIn(t, queue.WaitingTurnKey, record)
	assertEqual(t, []any{record.Get("source"), dig(t, record, "turn", "provider"), dig(t, record, "turn", "model")},
		[]any{"conflict", "muse", "muse-spark-1.3-contributor"})
	assertEqual(t, queue.OwnerOf(h.ctx.Store.Load(), pr), "muse")
	lanes := plan.ReadLanes(h.ctx, h.ctx.Store.Load())
	assertEqual(t, []any{lanes.Held[pr], lanes.Turns[pr]}, []any{"muse", true})
	assertIn(t, "(reserved for its turn)", lanes.Describe())
	assertIn(t, "**Its turn**", lastOf(t, h.gh.BotComments(pr)))
	crossNextRun(h, "780")
	flowUpdateItem(h.ctx, pr, pyjson.New("session", pyjson.New("provider", "muse", "run", "777"))) // not resumed
	claimed := map[string]bool{}
	r := workFake(map[string]runner.Handler{
		"revise": func(request *runner.RunRequest) *runner.RunResult {
			claimed = h.gh.LabelNames(pr)
			return workBuilder(map[string]string{"src/game.txt": "rules v2, after #40\n"})(request)
		},
		"review": workReviewer(workApprove)})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider"), planned.Get("source"), planned.Get("turn"),
		dig(t, planned, "seats", "build", "model")}, []any{"revise", "muse", "conflict", true, "muse-spark-1.3-contributor"})
	assertTrue(t, claimed[config.LabelResolving])
	assertFalse(t, planned.Has("session")) // a fresh session
	assertEqual(t, workRoles(r), []string{"revise", "review"})
	assertEqual(t, result.Get("status"), "approved")
	head := crossStr(t, result, "head")
	assertNotEqual(t, head, approved.Get("head"))
	assertEqual(t, h.gh.GetPull(pr).Get("draft"), false)
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertEqual(t, h.gh.AutoMergeHeads[fmt.Sprintf("PR_%d", pr)], head)
	after := settleRecord(t, h, pr)
	assertEqual(t, []any{dig(t, after, "cleared", "sha"), dig(t, after, "cleared", "carried_from")},
		[]any{head, approved.Get("head")})
	assertNotIn(t, queue.TurnKey, after)
	assertIn(t, "the clearance carries over", lastOf(t, h.gh.BotComments(pr)))
}

// A pull request ahead that does not merge within the wait (`hold_minutes`) holds it no longer: it
// is taken up anyway, merges `main` (cleanly, as #40 has not landed), and goes ready with no builder
// session, its reviewer's approval of the clean merge carrying the clearance.
func TestSettle_ItGoesAheadAfterTheBound(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, _ := settleWaiting(t, h)
	assertEqual(t, plan.SyncTurns(h.ctx, 0), []string{})
	fakes.PushBranch(h.origin, h.root, "main", map[string]string{"README.md": "news\n"})
	sweep.Sweep(h.ctx) // the first sweep only starts the record
	testsupport.ClockOf(h.ctx).At = h.ctx.Now().Add(time.Duration(h.cfg.HoldMinutes) * time.Minute)
	notes := sweep.Sweep(h.ctx)
	assertIn(t, fmt.Sprintf("#%d: its turn came (it waited 120 minutes for #40", pr), strings.Join(notes, "\n"))
	assertIn(t, config.LabelRevise, h.gh.LabelNames(pr))
	crossNextRun(h, "781")
	r := workFake(map[string]runner.Handler{"review": workReviewer(workApprove)})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("provider"), planned.Get("turn")}, []any{"muse", true})
	assertEqual(t, workRoles(r), []string{"review"}) // no builder: the harness merged `main`
	assertEqual(t, dig(t, result, "cycles", 0, "builder", "role"), "merge")
	assertEqual(t, result.Get("status"), "approved")
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertEqual(t, h.gh.GetPull(pr).Get("draft"), false)
	assertIn(t, "merged it cleanly", lastOf(t, h.gh.BotComments(pr)))
}

// Precedence: the pull request approved earliest and conflict-free goes first. Of four open ones
// that overlap the draft and conflict with it once landed, it waits only for #40 (approved, clean
// with `main`): #41 conflicts with `main` itself, so it drops out of the order; #42 was never
// approved; #43 waits for a person (`ready for merge`) with its CI green, so it gives up its
// precedence. #44 overlaps but does not conflict.
func TestSettle_TheEarliestApprovedConflictFreePullRequestGoesFirst(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, _ := ownReviewBuild(t, h, "")
	settleOther(t, h, 40, "feature/a", map[string]string{"src/game.txt": "rules from #40\n"}, true, time.Hour)
	settleOther(t, h, 41, "feature/b", map[string]string{"src/game.txt": "rules from #41\n",
		"README.md": "hello from #41\n"}, true, 2*time.Hour)
	settleOther(t, h, 42, "feature/c", map[string]string{"src/game.txt": "rules from #42\n"}, false, 0)
	green := settleOther(t, h, 43, "feature/d", map[string]string{"src/game.txt": "rules from #43\n"}, true,
		3*time.Hour, config.LabelReady)
	h.gh.Patch.CheckRuns = func(sha string) []*pyjson.Object {
		if sha != green {
			return []*pyjson.Object{}
		}
		return rebuildPassing(h.cfg.RequiredChecks)(sha)
	}
	settleOther(t, h, 44, "feature/e", map[string]string{"src/game.txt": "rules v2\n", "docs/e.md": "e\n"}, true, time.Hour)
	fakes.PushBranch(h.origin, h.root, "main", map[string]string{"README.md": "hello from main\n"})
	_, result := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	byNumber := map[int]*pyjson.Object{}
	for _, p := range pyjson.Objs(dig(t, result, "settle", "pulls")) {
		byNumber[p.Int("number")] = p
	}
	assertEqual(t, len(byNumber), 5)
	assertEqual(t, byNumber[41].Get("problem"), "it conflicts with `main` itself")
	assertEqual(t, byNumber[44].Get("conflicts"), []any{})
	for _, n := range []int{40, 42, 43} {
		assertEqual(t, byNumber[n].Get("conflicts"), []any{"src/game.txt"}, n)
	}
	assertEqual(t, dig(t, settleRecord(t, h, pr), queue.WaitingTurnKey, "behind"), []any{40})
	assertIn(t, config.LabelWaitingTurn, h.gh.LabelNames(pr))
}

// When the work job cannot read the open pull requests (its read-only client refused), it says so,
// and deliver test-merges them itself with its own client: the draft still waits its turn.
func TestSettle_DeliverTestMergesWhatTheWorkJobCouldNotRead(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, _ := ownReviewBuild(t, h, "")
	settleOther(t, h, 40, "feature/a", map[string]string{"src/game.txt": "rules from #40\n"}, true, time.Hour)
	refuse := false
	h.gh.Patch.ListPulls = func(state string, head string, limit int) []*pyjson.Object {
		if refuse && head == "" {
			refuse = false
			panic(&herr.GitHubError{Msg: "Resource not accessible by integration", Status: 403})
		}
		return h.gh.Own().ListPulls(state, head, limit)
	}
	reviewing := func(request *runner.RunRequest) *runner.RunResult {
		refuse = true // the work job's next read of the open pull requests is refused
		return runner.NewRunResult(true, workApprove)
	}
	_, result := h.night(workFake(map[string]runner.Handler{"review": reviewing}))
	assertIn(t, "the open pull requests could not be read", crossStr(t, result, "settle", "pulls_read"))
	assertEqual(t, dig(t, settleRecord(t, h, pr), queue.WaitingTurnKey, "behind"), []any{40})
	assertIn(t, config.LabelWaitingTurn, h.gh.LabelNames(pr))
}

// With nothing ahead of it but pull requests that drop out, it goes ready as any clean draft does.
// A head GitHub keeps no `refs/pull/<n>/head` for is fetched from its branch.
func TestSettle_NothingAheadOfItGoesReady(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, _ := ownReviewBuild(t, h, "")
	settleOther(t, h, 42, "feature/c", map[string]string{"src/game.txt": "rules from #42\n"}, false, 0)
	fakes.Git(h.origin, "update-ref", "-d", "refs/pull/42/head")
	_, result := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, dig(t, result, "settle", "pulls", 0, "conflicts"), []any{"src/game.txt"})
	assertNotIn(t, queue.WaitingTurnKey, settleRecord(t, h, pr))
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
}

// The one-run flow (Squishy's, `own_review_run` off) has no final check: its approval ships as
// before, with no `settle` in its result, whatever overlaps it.
func TestSettle_TheOneRunFlowHasNoFinalCheck(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Machine: []string{}})
	assertFalse(t, h.cfg.OwnReviewRun)
	h.gh.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelBuild)})
	settleOther(t, h, 40, "feature/a", map[string]string{"src/game.txt": "rules from #40\n"}, true, time.Hour)
	_, result := h.night(workFake(map[string]any{"build": workBuilder(map[string]any{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)}))
	assertEqual(t, result.Get("status"), "approved")
	assertFalse(t, result.Has("settle"))
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	assertNotIn(t, config.LabelWaitingTurn, h.gh.LabelNames(pr))
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
}

// ---------------------------------------------------------------------------- the owned lane

// settleConflicted is bot pull request #30 (issue 7's, which `muse` owns), whose head `agy`'s
// approval shipped (its merge owner), and which `main` left conflicted.
func settleConflicted(t *testing.T, h *flowHarness, labels ...string) {
	t.Helper()
	h.gh.AddIssue(7, fakes.Issue{Labels: []string{config.LabelPROpen}})
	h.gh.AddPull(30, "bot/issue-7", fakes.Pull{Labels: append([]string{config.LabelPR, "difficulty:medium"}, labels...),
		SHA: "abc", Body: "Closes #7"})
	h.gh.Threads.Item(30).Set("mergeable_state", "dirty")
	flowUpdateItem(h.ctx, 7, pyjson.New("owner", "muse", "owner_since", "2026-09-29T10:00:00Z"))
	flowUpdateItem(h.ctx, 30, pyjson.New("issue", 7, "cleared", pyjson.New("sha", "abc", "at", "2026-09-29T12:00:00Z",
		"by", "agy (gemini, medium)"), "merge_owner", "agy"))
}

// A conflict on a bot pull request goes to its merge owner first, the subscription whose approval
// shipped its head, ahead of that one's new work, as a CI fix does: `bot:resolving` while it runs.
func TestSettle_AConflictRevisionGoesToTheMergeOwnersLaneFirst(t *testing.T) {
	h := ownReviewHarness(t, nil)
	settleConflicted(t, h)
	h.gh.AddIssue(14, fakes.Issue{Title: "New work", Labels: []string{config.LabelBuild}})
	notes := plan.Housekeeping(h.ctx, h.ctx.Store.Load())
	assertIn(t, "queued #30: it conflicts with main; `agy`, its merge owner, resolves it", notes)
	state := h.ctx.Store.Load()
	assertEqual(t, queue.OwnerOf(state, 30), "agy")
	assertIn(t, "the last approver owns the merge, so it resolves the conflict", crossStr(t, state, "items", "7", "owner_why"))
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	assertEqual(t, []any{planned.Get("action"), planned.Get("number"), planned.Get("provider"), planned.Get("source")},
		[]any{"revise", 30, "agy", "conflict"})
	assertIn(t, config.LabelResolving, h.gh.LabelNames(30))
	assertEqual(t, crossStr(t, h.ctx.Store.Load(), "items", "30", "live"), "resolving")
}

// A pull request a person merges gives up its lane's ownership once its CI is green, and only then:
// its merge owner's claim goes (`merge_owner_gave_up` keeps whose it was), so a conflict later goes
// to the item's owner, not first to that lane.
func TestSettle_APersonMergedPullRequestGivesUpOwnershipOnceItsCIIsGreen(t *testing.T) {
	h := ownReviewHarness(t, nil)
	settleConflicted(t, h, config.LabelReady)
	h.gh.Threads.Item(30).Set("mergeable_state", "clean")
	assertEqual(t, sweep.PersonMerges(h.ctx, h.ctx.Now()), []string{}) // CI not green yet: it keeps it
	assertEqual(t, settleRecord(t, h, 30).Get("merge_owner"), "agy")
	h.gh.Patch.CheckRuns = rebuildPassing(h.cfg.RequiredChecks)
	assertEqual(t, sweep.PersonMerges(h.ctx, h.ctx.Now()),
		[]string{"#30: green and waiting for a person to merge it, so `agy` gives up its claim on it"})
	record := settleRecord(t, h, 30)
	assertNotIn(t, "merge_owner", record)
	assertEqual(t, []any{dig(t, record, queue.GaveUpKey, "sha"), dig(t, record, queue.GaveUpKey, "provider")},
		[]any{"abc", "agy"})
	h.gh.Threads.Item(30).Set("mergeable_state", "dirty")
	notes := plan.Housekeeping(h.ctx, h.ctx.Store.Load())
	assertIn(t, "queued #30: it conflicts with main", notes)
	assertEqual(t, queue.OwnerOf(h.ctx.Store.Load(), 30), "muse")
	// An auto-merged one keeps it, green or not.
	h2 := ownReviewHarness(t, nil)
	settleConflicted(t, h2)
	h2.gh.Patch.CheckRuns = rebuildPassing(h2.cfg.RequiredChecks)
	assertEqual(t, sweep.PersonMerges(h2.ctx, h2.ctx.Now()), []string{})
}

// ---------------------------------------------------------------------------- Mergiraf

// settleMergirafRepo is a repository whose `feature` branch and `main` both changed `app.ts` from
// the same base, with the root `.gitattributes` giving `*.ts` to Mergiraf, and a worktree of
// `feature` ready to merge `main`.
func settleMergirafRepo(t *testing.T) (*git.Git, *git.Git) {
	t.Helper()
	root := t.TempDir()
	work := filepath.Join(root, "repo")
	assertTrue(t, os.MkdirAll(work, 0o777) == nil)
	fakes.Git(work, "init", "-q", "-b", "main")
	writeText(t, filepath.Join(work, ".gitattributes"), "*.ts merge=mergiraf\n")
	writeText(t, filepath.Join(work, "app.ts"), "export const a = 1;\n")
	fakes.Git(work, "add", "-A")
	fakes.Git(work, "commit", "-q", "-m", "base")
	fakes.Git(work, "checkout", "-q", "-b", "feature")
	writeText(t, filepath.Join(work, "app.ts"), "export const a = 1;\nexport const b = 2;\n")
	fakes.Git(work, "commit", "-q", "-am", "the branch")
	fakes.Git(work, "checkout", "-q", "main")
	writeText(t, filepath.Join(work, "app.ts"), "export const a = 1;\nexport const c = 3;\n")
	fakes.Git(work, "commit", "-q", "-am", "main moves")
	repo := git.NewGit(work, nil)
	wt := git.WorktreeAdd(repo, filepath.Join(root, "wt"), "settle-feature", "feature")
	return repo, wt
}

// settleFakeMergiraf is a stand-in for the pinned Mergiraf: it says its version, and as a merge
// driver keeps every line of ours and then theirs' new ones, noting each call.
func settleFakeMergiraf(t *testing.T) (string, string) {
	t.Helper()
	dir := t.TempDir()
	bin := filepath.Join(dir, "mergiraf")
	writeText(t, bin, "#!/bin/sh\n"+
		"if [ \"$1\" = --version ]; then echo \"mergiraf "+resolve.MergirafVersion+"\"; exit 0; fi\n"+
		"echo \"$@\" >> \""+filepath.Join(dir, "calls")+"\"\n"+
		"{ cat \"$4\"; grep -vxF -f \"$4\" \"$5\"; } > \"$4.merged\" && mv \"$4.merged\" \"$4\"\n")
	assertTrue(t, os.Chmod(bin, 0o755) == nil)
	return bin, filepath.Join(dir, "calls")
}

// settleEnv is the environment with `dir` first on its PATH.
func settleEnv(dir string) map[string]string {
	env := config.ChildEnv(nil, nil)
	env["PATH"] = dir + string(os.PathListSeparator) + env["PATH"]
	return env
}

// Present, the pinned Mergiraf is found on the job's PATH and registered in the worktree's own
// config alone, with the command Mergiraf documents; git runs it for the files `.gitattributes` gives
// it, and the merge is clean.
func TestSettle_MergirafIsRegisteredWhenPresent(t *testing.T) {
	repo, wt := settleMergirafRepo(t)
	bin, calls := settleFakeMergiraf(t)
	assertEqual(t, resolve.Takes(wt, []string{"app.ts", "README.md"}), []string{"app.ts"})
	how, ok := resolve.Mergiraf(repo, wt, settleEnv(filepath.Dir(bin)), t.TempDir(), 600)
	assertTrue(t, ok, how)
	assertIn(t, "found `"+bin+"`", how)
	assertEqual(t, wt.Out(false, "config", "--worktree", "--get", "merge.mergiraf.driver"), resolve.DriverCommand(bin))
	assertEqual(t, resolve.DriverCommand(bin), "'"+bin+"' merge --git %O %A %B -s %S -x %X -y %Y -p %P -l %L")
	assertEqual(t, repo.Out(false, "config", "--get", "merge.mergiraf.driver"), "") // not the checkout's
	assertEqual(t, wt.Merge("main", git.IdentityBot("jgoetzmann-bot", 1)), []string{})
	assertEqual(t, readText(t, filepath.Join(wt.Cwd, "app.ts")), "export const a = 1;\nexport const b = 2;\nexport const c = 3;\n")
	assertIn(t, "--git", readText(t, calls))
}

// Absent (not on PATH, not built, and no build possible), it is logged and git's own merge goes on:
// nothing is registered, and the file is conflicted as before.
func TestSettle_MergirafFallsBackWhenAbsent(t *testing.T) {
	repo, wt := settleMergirafRepo(t)
	install := resolve.InstallMergiraf
	t.Cleanup(func() { resolve.InstallMergiraf = install })
	resolve.InstallMergiraf = func(env map[string]string, cache string, seconds int) (string, string) {
		return "", "no network here"
	}
	how, ok := resolve.Mergiraf(repo, wt, settleEnv(t.TempDir()), t.TempDir(), 600)
	assertFalse(t, ok)
	assertIn(t, "Mergiraf "+resolve.MergirafVersion+" is not on PATH", how)
	assertIn(t, "it could not be built: no network here. Git's own merge instead", how)
	assertEqual(t, wt.Out(false, "config", "--get", "merge.mergiraf.driver"), "")
	assertEqual(t, wt.Merge("main", git.IdentityBot("jgoetzmann-bot", 1)), []string{"app.ts"})
	assertIn(t, "<<<<<<< ", readText(t, filepath.Join(wt.Cwd, "app.ts")))
}

// The pinned release is built only from a download whose checksum is the pinned one, and only from
// an archive that stays in its directory; anything else is refused, and nothing is built.
func TestSettle_MergirafIsBuiltOnlyFromItsPinnedSource(t *testing.T) {
	served := []byte("not the crate")
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.Write(served) }))
	defer server.Close()
	source, sum := resolve.MergirafSource, resolve.MergirafSHA256
	t.Cleanup(func() { resolve.MergirafSource, resolve.MergirafSHA256 = source, sum })
	resolve.MergirafSource = server.URL + "/mergiraf.crate"
	env := config.ChildEnv(nil, nil)
	bin, why := resolve.InstallMergiraf(env, t.TempDir(), 60)
	assertEqual(t, bin, "")
	assertIn(t, "its source's checksum is ", why)
	assertIn(t, "not the pinned "+sum, why)
	var archive bytes.Buffer
	zipped := gzip.NewWriter(&archive)
	tarred := tar.NewWriter(zipped)
	assertTrue(t, tarred.WriteHeader(&tar.Header{Name: "../escaped.txt", Mode: 0o644, Size: 2, Typeflag: tar.TypeReg}) == nil)
	_, err := tarred.Write([]byte("x\n"))
	assertTrue(t, err == nil && tarred.Close() == nil && zipped.Close() == nil)
	served = archive.Bytes()
	digest := sha256.Sum256(served)
	resolve.MergirafSHA256 = hex.EncodeToString(digest[:])
	cache := filepath.Join(t.TempDir(), "cache")
	bin, why = resolve.InstallMergiraf(env, cache, 60)
	assertEqual(t, bin, "")
	assertEqual(t, why, "could not unpack its source: ../escaped.txt is outside the archive")
	_, err = os.Stat(filepath.Join(cache, "escaped.txt"))
	assertTrue(t, os.IsNotExist(err))
	bin, why = resolve.InstallMergiraf(env, cache, 0)
	assertEqual(t, []string{bin, why}, []string{"", "the run has no time left to build it"})
}

// settleMergirafWorld is a conflict revision of #30 whose only conflict is a TypeScript file the root
// `.gitattributes` gives Mergiraf.
func settleMergirafWorld(t *testing.T) *resolveWorld {
	t.Helper()
	w := newResolveWorld(t)
	base := filepath.Join(w.root, "base")
	fakes.Git(w.root, "clone", "-q", w.origin, base)
	writeText(t, filepath.Join(base, ".gitattributes"), "*.ts merge=mergiraf\n")
	writeText(t, filepath.Join(base, "app.ts"), "export const a = 1;\n")
	fakes.Git(base, "add", "-A")
	fakes.Git(base, "commit", "-q", "-m", "attributes")
	fakes.Git(base, "push", "-q", "origin", "HEAD:main")
	w.diverge(map[string]string{"app.ts": "export const a = 1;\nexport const b = 2;\n"},
		map[string]string{"app.ts": "export const a = 1;\nexport const c = 3;\n"})
	return w
}

// Through the work job: with the driver, the merge is the harness's, no builder session is spent,
// and the reviewer hears Mergiraf merged it; without it, the builder gets the conflict as before.
func TestSettle_TheWorkJobUsesMergirafAndFallsBack(t *testing.T) {
	find := resolve.FindMergiraf
	t.Cleanup(func() { resolve.FindMergiraf = find })
	bin, _ := settleFakeMergiraf(t)
	resolve.FindMergiraf = func(env map[string]string, cache string) (string, string) { return bin, "" }
	w := settleMergirafWorld(t)
	r := workFake(map[string]runner.Handler{"review": crossAnswer(resolveApprove)})
	result := w.runWorker(r, w.plan(""))
	assertEqual(t, workRoles(r), []string{"review"})
	assertEqual(t, result.Get("status"), "approved", result.Get("reason"))
	assertEqual(t, dig(t, result, "mergiraf", "used"), true)
	assertIn(t, "merged by Mergiraf", crossStr(t, result, "resolved", 0, "how"))
	assertIn(t, "A file Mergiraf merged is a syntax-aware merge", r.Calls[0].Prompt)

	install := resolve.InstallMergiraf
	t.Cleanup(func() { resolve.InstallMergiraf = install })
	resolve.FindMergiraf = func(env map[string]string, cache string) (string, string) { return "", "not here" }
	resolve.InstallMergiraf = func(env map[string]string, cache string, seconds int) (string, string) {
		return "", "no network here"
	}
	w = settleMergirafWorld(t)
	r = workFake(map[string]runner.Handler{
		"revise": workBuilder(map[string]string{"app.ts": "export const a = 1;\nexport const b = 2;\nexport const c = 3;\n"}),
		"review": crossAnswer(resolveApprove)})
	result = w.runWorker(r, w.plan(""))
	assertEqual(t, workRoles(r), []string{"revise", "review"})
	assertEqual(t, dig(t, result, "mergiraf", "used"), false)
	assertIn(t, "no network here", crossStr(t, result, "mergiraf", "how"))
	assertIn(t, "app.ts", ownReviewPrompt(t, r, "revise"))
	assertEqual(t, result.Get("status"), "approved", result.Get("reason"))
}

// ---------------------------------------------------------------------------- the union resolver

// `take: union` keeps both sides' lines of an append-only list, as git's union merge does, and stages
// the file; a side that deleted it is left for the builder.
func TestSettle_TheUnionResolverKeepsBothSidesLines(t *testing.T) {
	list := &config.Resolver{Path: "README.md", Take: "union", TimeoutMinutes: 10}
	repo := resolveConflicted(t.TempDir(), map[string]string{"README.md": "hello\nfrom the branch\n"},
		map[string]string{"README.md": "hello\nfrom main\n"})
	left, done := resolveRunResolve(repo, []*config.Resolver{list})
	assertEqual(t, left, []string{})
	assertEqual(t, []any{done[0].Path, done[0].How, done[0].OK}, []any{"README.md", "kept both sides' lines (union)", true})
	assertEqual(t, readText(t, filepath.Join(repo.Cwd, "README.md")), "hello\nfrom the branch\nfrom main\n")
	assertEqual(t, resolveUnmerged(repo), []string{})
	gone := resolveConflicted(t.TempDir(), map[string]string{"README.md": resolveNone},
		map[string]string{"README.md": "hello\nfrom main\n"})
	left, done = resolveRunResolve(gone, []*config.Resolver{list})
	assertEqual(t, left, []string{"README.md"})
	assertEqual(t, []any{done[0].OK, done[0].WhyNot}, []any{false, "one side deleted the file, which a union does not decide"})
	parsed := testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("resolvers",
		[]any{pyjson.New("path", "docs/list.md", "take", "union")})}).Resolvers
	assertEqual(t, []any{parsed[0].Path, parsed[0].Take}, []any{"docs/list.md", "union"})
	// The committed config names no union resolver: no file here is truly an append-only list.
	for _, r := range testsupport.MakeConfig().Resolvers {
		assertNotEqual(t, r.Take, "union", r.Path)
	}
}
