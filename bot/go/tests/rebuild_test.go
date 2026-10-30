// #316: drop a pull request that can't be saved and build it again from `main`; and #315's
// routing of a conflict revision.
//
// #141 was opened by a run whose builder failed with a 401 in 0.0 minutes, and then revised for
// two days with nothing ever giving up on it. A strike is a failure of the item's own; strikes
// survive a person's request, end only on a head that met the review rule and is green in CI, and
// at three the pull request is closed and its issue built again, its last findings kept for the
// new build.
//
// bot/tests/test_rebuild.py.
package tests

import (
	"os"
	"path/filepath"
	"slices"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stepup"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/sweep"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

const rebuildTrust = "jgoetzmann 3 id:95732896\n"

// rebuildFinding is FINDING.
func rebuildFinding() *pyjson.Object {
	return pyjson.New("severity", "blocking", "where", "SPEC.md", "claim", "R700 is cited twice",
		"evidence", "grep")
}

// rebuildWriteJSON is path.write_text(json.dumps(v)).
func rebuildWriteJSON(t *testing.T, path string, v any) {
	t.Helper()
	if err := os.WriteFile(path, []byte(pyjson.Dumps(v)), 0o666); err != nil {
		t.Fatal(err)
	}
}

func TestRebuild_DeliverOpensNoPullRequestWhenNoBuilderSessionWorked(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned, out := markersBundle(t, h, map[string]string{"src/game.txt": "left by an earlier run\n"})
	result := ratingReadResult(t, out)
	result.Update(pyjson.New("status", "not_approved", "reason", "the reviewer's answer could not be read",
		"cycles", []any{pyjson.New("n", 1, "builder", pyjson.New("role", "build", "ok", false, "changed", false,
			"error", "401 OAuth access token is invalid"))}))
	rebuildWriteJSON(t, filepath.Join(out, "result.json"), result)
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	pulls := []*pyjson.Object{}
	for _, thread := range h.gh.Threads.Values() {
		if thread.Has("pull_request") {
			pulls = append(pulls, thread)
		}
	}
	assertFalse(t, pulls)
	assertIsNone(t, h.originSHA("bot/issue-12"))
	assertIn(t, "no builder session in the run worked", strings.Join(h.gh.BotComments(12), "\n"))
}

// rebuildStrikes is StrikeTests' world.
type rebuildStrikes struct {
	t   *testing.T
	gh  *fakes.FakeGitHub
	ctx *hctx.Context
	cid int
}

func newRebuildStrikes(t *testing.T) *rebuildStrikes {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5, fakes.Issue{Labels: []string{config.LabelPROpen}})
	gh.AddPull(9, "bot/issue-5", fakes.Pull{Body: "Closes #5", Labels: []string{config.LabelPR}, SHA: "s1"})
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, TrustText: ptr(rebuildTrust)})
	flowUpdateItem(ctx, 9, pyjson.New("issue", 5))
	return &rebuildStrikes{t: t, gh: gh, ctx: ctx, cid: 900}
}

// strikes is int(items.get("5", {}).get("strikes") or 0).
func (w *rebuildStrikes) strikes() int {
	record := w.ctx.Store.Load().Obj("items").Obj("5")
	if record == nil || !record.Truthy("strikes") {
		return 0
	}
	n, ok := pyjson.AsInt(record.Get("strikes"))
	if !ok {
		w.t.Fatalf("strikes is %v", record.Get("strikes"))
	}
	return n
}

// ci is StrikeTests.ci(attempt=1, sha="s1").
func (w *rebuildStrikes) ci(attempt int, sha string) []string {
	run := pyjson.New("name", "CI", "conclusion", "failure", "head_branch", "bot/issue-5",
		"head_sha", sha, "id", 99, "run_attempt", attempt, "html_url", "u",
		"pull_requests", []any{pyjson.New("number", 9)})
	return events.Handle(w.ctx, "workflow_run",
		pyjson.New("action", "completed", "sender", fakes.Bot, "workflow_run", run))
}

// newHead is StrikeTests.new_head(sha, **record).
func (w *rebuildStrikes) newHead(sha string, record ...any) {
	w.gh.Threads.Item(9).Obj("head").Set("sha", sha)
	w.gh.Threads.Item(9).Set("labels", eventsLabels(config.LabelPR))
	flowUpdateItem(w.ctx, 9, pyjson.New(record...))
}

// rebuildPassing is `lambda sha: [{"name": n, "conclusion": "success"} for n in names]`.
func rebuildPassing(names []string) func(sha string) []*pyjson.Object {
	return func(sha string) []*pyjson.Object {
		out := []*pyjson.Object{}
		for _, n := range names {
			out = append(out, pyjson.New("name", n, "conclusion", "success"))
		}
		return out
	}
}

func TestRebuild_StrikesSurviveAPersonsRequest(t *testing.T) {
	w := newRebuildStrikes(t)
	stepup.Strike(w.ctx, 9, "its reviewer did not approve it", "")
	stepup.Strike(w.ctx, 9, "CI still failed after 1 fix(es)", "")
	assertEqual(t, w.strikes(), 2)
	w.cid++
	events.Handle(w.ctx, "issue_comment", pyjson.New(
		"action", "created", "sender", fakes.Operator,
		"issue", pyjson.New("number", 9, "title", "t", "pull_request", pyjson.New()),
		"comment", pyjson.New("id", w.cid, "body", "@jgoetzmann-bot please name it better",
			"user", fakes.Operator, "author_association", "OWNER")))
	assertIn(t, config.LabelRevise, w.gh.LabelNames(9))
	assertEqual(t, w.strikes(), 2)
}

func TestRebuild_OnlyAnApprovedHeadThatIsGreenClearsThem(t *testing.T) {
	w := newRebuildStrikes(t)
	stepup.Strike(w.ctx, 9, "its reviewer did not approve it", "")
	flowUpdateItem(w.ctx, 9, pyjson.New("cleared", pyjson.New("sha", "s1", "at", clock.ISO(testsupport.Day), "by", "`opus` (strong)")))
	required := slices.Clone(w.ctx.Cfg.RequiredChecks)
	w.gh.Patch.CheckRuns = rebuildPassing(required[:len(required)-1])
	sweep.GreenHeads(w.ctx, testsupport.Day)
	assertEqual(t, w.strikes(), 1) // approved, but one required check has not passed
	w.gh.Patch.CheckRuns = rebuildPassing(required)
	notes := sweep.GreenHeads(w.ctx, testsupport.Day)
	assertEqual(t, w.strikes(), 0)
	assertIn(t, "#9: approved and green, so its strikes are cleared", notes)
}

func TestRebuild_AGreenHeadNobodyApprovedClearsNothing(t *testing.T) {
	w := newRebuildStrikes(t)
	stepup.Strike(w.ctx, 9, "its reviewer did not approve it", "")
	w.gh.Patch.CheckRuns = rebuildPassing(w.ctx.Cfg.RequiredChecks)
	sweep.GreenHeads(w.ctx, testsupport.Day)
	assertEqual(t, w.strikes(), 1)
}

func TestRebuild_AMergeClearsThem(t *testing.T) {
	w := newRebuildStrikes(t)
	stepup.Strike(w.ctx, 9, "its reviewer did not approve it", "")
	pull := w.gh.GetPull(9).Copy()
	pull.Update(pyjson.New("merged", true, "state", "closed",
		"base", pyjson.New("ref", w.ctx.Cfg.DefaultBranch)))
	events.Handle(w.ctx, "pull_request_target", pyjson.New("action", "closed", "sender", fakes.Bot,
		"pull_request", pull))
	assertEqual(t, w.strikes(), 0)
}

func TestRebuild_EveryCIFixThatLeftItRedIsAStrike(t *testing.T) {
	w := newRebuildStrikes(t)
	w.ci(1, "s1") // re-run once
	w.ci(2, "s1") // then a first fix is queued: no strike, nothing was fixed yet
	assertEqual(t, w.strikes(), 0)
	w.newHead("s2", "ci_fixed_head", "s2") // the fix pushed s2
	w.ci(1, "s2")
	w.ci(2, "s2") // and CI is still red on it
	assertEqual(t, w.strikes(), 1)
}

// `ci_fixes` is never reset, so a head a conflict or cross-review revision made later must not be
// struck for the CI fixes before it.
func TestRebuild_AHeadNoCIFixMadeIsNoStrike(t *testing.T) {
	w := newRebuildStrikes(t)
	flowUpdateItem(w.ctx, 9, pyjson.New("ci_fixes", 1, "ci_fixed_head", "s1"))
	w.newHead("s3")
	w.ci(1, "s3")
	w.ci(2, "s3")
	assertEqual(t, w.strikes(), 0)
	assertIn(t, config.LabelRevise, w.gh.LabelNames(9)) // it is fixed, not struck
}

// rebuildByDay is make_ctx(gh, at=DAY, cfg=make_config(env={"HARNESS_SECRETS_SET": ""},
// machine=machine)).
func rebuildByDay(gh *fakes.FakeGitHub, machine ...string) *hctx.Context {
	return testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, Cfg: testsupport.MakeConfig(
		testsupport.Config{Env: map[string]string{"HARNESS_SECRETS_SET": ""}, Machine: machine})})
}

func TestRebuild_TheRebuildKeepsTheLastFindingsAndNamesTheOldPullRequest(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := rebuildByDay(gh, "muse")
	gh.AddIssue(5, fakes.Issue{Labels: []string{config.LabelPROpen}})
	gh.AddPull(9, "bot/issue-5", fakes.Pull{Body: "Closes #5", Labels: []string{config.LabelPR}, SHA: "s1"})
	flowUpdateItem(ctx, 9, pyjson.New("issue", 5, "last_findings", []any{rebuildFinding()}))
	stepup.Rebuild(ctx, 5, 9, "it failed 3 times at difficulty:medium", "")
	assertEqual(t, dig(t, gh.Threads.Item(9), "state"), "closed")
	record := dig(t, ctx.Store.Load(), "items", "5")
	assertEqual(t, []any{dig(t, record, "previous_pr"), dig(t, record, "last_findings")},
		[]any{9, []any{rebuildFinding()}})
	assertIn(t, config.LabelBuild, gh.LabelNames(5))
	planned := plan.Make(ctx, false, nil, "auto", nil)
	assertEqual(t, []any{dig(t, planned, "action"), dig(t, planned, "number")}, []any{"build", 5})
	assertEqual(t, dig(t, planned, "previous_pr"), 9)
	assertEqual(t, dig(t, planned, "previous_findings"), []any{rebuildFinding()})
}

func TestRebuild_TheRebuildStartsCleanFromMainWithNoOldWork(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := rebuildByDay(gh, "muse")
	gh.AddIssue(5, fakes.Issue{Labels: []string{config.LabelPROpen}})
	gh.AddPull(9, "bot/issue-5", fakes.Pull{Body: "Closes #5", Labels: []string{config.LabelPR}, SHA: "s1"})
	flowUpdateItem(ctx, 9, pyjson.New("issue", 5))
	flowUpdateItem(ctx, 5, pyjson.New("kind", "build", "pr", 9, "handoff", pyjson.New("pr", 9), "wip", pyjson.New("anything", true)))
	stepup.Rebuild(ctx, 5, 9, "it failed 3 times at difficulty:medium", "")
	assertIn(t, "bot/issue-5", gh.DeletedBranches)
	record := dig(t, ctx.Store.Load(), "items", "5")
	for _, key := range []string{"pr", "handoff", "wip"} {
		assertNotIn(t, key, record)
	}
	assertEqual(t, []any{dig(t, record, "failures"), dig(t, record, "strikes"), dig(t, record, "strike_log")},
		[]any{0, 0, []any{}})
}

// #315: with only the weak one free, a conflict revision is not assigned at all; with Muse free
// too, Muse takes it.
func TestRebuild_AConflictRevisionWaitsForABuilderThatReviewsInItsOwnRun(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := rebuildByDay(gh, "lite")
	gh.AddPull(9, "bot/issue-2", fakes.Pull{Labels: []string{config.LabelPR, config.LabelRevise, "difficulty:easy"}})
	flowUpdateItem(ctx, 9, pyjson.New("kind", "revise", "source", "conflict"))
	assertEqual(t, dig(t, plan.Make(ctx, false, nil, "auto", nil), "action"), "none")
	ctx = rebuildByDay(gh, "lite", "muse")
	planned := plan.Make(ctx, false, nil, "auto", nil)
	assertEqual(t, []any{dig(t, planned, "action"), dig(t, planned, "provider")}, []any{"revise", "muse"})
}
