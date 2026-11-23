package tests

// test_cross.py: several subscriptions end to end: the review rule (#2748: one medium or strong
// approval for a medium item, one strong approval for a hard one, from any subscription, the
// builder's own lane's included since 2026-11-02), review runs, `difficulty:hard`, handoff and the
// vault, through plan, work and deliver against a real repository and the fake GitHub.
//
// It also holds test_cross.py's ALL, timed_builder, next_run, MEDIUM and
// ReviewRuleTests.build_on_medium, which other files import (crossAll, crossTimedBuilder,
// crossNextRun, crossMedium, crossBuildOnMedium, crossBuildForAudit), and small helpers that test_fullsend.py,
// test_resolve.py, test_review_fixes.py and test_plan_draft.py's ports share.

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/vault"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// ---------------------------------------------------------------------------- the file's helpers

// crossAnd is `a & b` of two sets.
func crossAnd(a, b map[string]bool) map[string]bool {
	out := map[string]bool{}
	for k := range a {
		if b[k] {
			out[k] = true
		}
	}
	return out
}

// crossObj is crossKey's object.
func crossObj(t *testing.T, v any, keys ...any) *pyjson.Object {
	t.Helper()
	found := dig(t, v, keys...)
	o, ok := found.(*pyjson.Object)
	if !ok {
		t.Fatalf("crossObj: %s is not an object", show(found))
	}
	return o
}

// crossStr is crossKey's string.
func crossStr(t *testing.T, v any, keys ...any) string {
	t.Helper()
	found := dig(t, v, keys...)
	s, ok := found.(string)
	if !ok {
		t.Fatalf("crossStr: %s is not a string", show(found))
	}
	return s
}

// crossInt is int(v[...]).
func crossInt(t *testing.T, v any, keys ...any) int {
	t.Helper()
	found := dig(t, v, keys...)
	n, ok := pyjson.AsInt(found)
	if !ok {
		t.Fatalf("crossInt: %s is not a number", show(found))
	}
	return n
}

// crossMkdtemp is tempfile.mkdtemp().
func crossMkdtemp(t *testing.T) string {
	t.Helper()
	dir, err := os.MkdirTemp("", "tmp")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { os.RemoveAll(dir) })
	return dir
}

// crossPut is `target.parent.mkdir(parents=True, exist_ok=True); target.write_text(text)`.
func crossPut(target, text string) {
	if err := os.MkdirAll(filepath.Dir(target), 0o777); err != nil {
		panic(err)
	}
	if err := os.WriteFile(target, []byte(text), 0o666); err != nil {
		panic(err)
	}
}

// crossModels is [c.model for c in runner.calls].
func crossModels(r *runner.FakeRunner) []string {
	out := []string{}
	for _, c := range r.Calls {
		out = append(out, c.Model)
	}
	return out
}

// crossCount is list.count(x).
func crossCount(xs []string, x string) int {
	n := 0
	for _, s := range xs {
		if s == x {
			n++
		}
	}
	return n
}

// crossAnswer is a handler that answers `text`: `lambda r: RunResult(True, text)`.
func crossAnswer(text string) runner.Handler {
	return func(*runner.RunRequest) *runner.RunResult { return runner.NewRunResult(true, text) }
}

// crossGitOK is git(...) in a `try: ... except AssertionError:`: false when git failed.
func crossGitOK(cwd string, args ...string) (string, bool) {
	var out string
	if _, failed := herr.Catch[*fakes.PyError](func() { out = fakes.Git(cwd, args...) }); failed {
		return "", false
	}
	return out, true
}

// ---------------------------------------------------------------------------- test_cross.py's

// crossPullOf is int(h.gh.list_pulls(head=branch)[0]["number"]).
func crossPullOf(t *testing.T, gh *fakes.FakeGitHub, branch string) int {
	t.Helper()
	pulls := gh.ListPulls("open", branch, 200)
	if len(pulls) == 0 {
		t.Fatalf("IndexError: no pull request from %s", branch)
	}
	return crossInt(t, pulls[0], "number")
}

// crossItems is ctx.store.load()["items"].
func crossItems(t *testing.T, ctx *hctx.Context) *pyjson.Object {
	t.Helper()
	return crossObj(t, ctx.Store.Load(), "items")
}

// crossAll is ALL: every subscription's secret set.
var crossAll = map[string]string{"HARNESS_SECRETS_SET": strings.Join(providers.SecretNames, " ")}

// crossTimedBuilder is timed_builder(edits, minutes): the builder, its call ten minutes long.
func crossTimedBuilder(edits map[string]string, minutes ...float64) runner.Handler {
	m := 10.0
	if len(minutes) > 0 {
		m = minutes[0]
	}
	inner := workBuilder(edits)
	return func(request *runner.RunRequest) *runner.RunResult {
		result := inner(request)
		result.DurationS = m * 60
		return result
	}
}

// crossNextRun is next_run: the next night is another workflow run: its votes count apart from
// the last one's.
func crossNextRun(h *flowHarness, runID string) {
	cfg := *h.cfg
	cfg.RunID = runID
	h.cfg = &cfg
	h.ctx.Cfg = &cfg
}

// crossMedium is MEDIUM: each medium subscription's model and family, for the tests that build
// on one of them.
var crossMedium = map[string][2]string{"muse": {"muse-spark-1.3-contributor", "muse"},
	"agy": {"gemini-3.8-flash-high", "gemini"}}

// ---------------------------------------------------------------------------- ReviewRuleTests

// crossBuildOnMedium is ReviewRuleTests.build_on_medium: by day the Claude accounts are closed:
// Muse, first of the medium models (#317 part 9), or `provider` when it is the only one, plans,
// builds and reviews it in one run, and its own approval, a medium vote from the subscription that
// built it, ships it (2026-11-02, #2748).
func crossBuildOnMedium(t *testing.T, h *flowHarness, provider string, edits map[string]string) (int, *pyjson.Object) {
	t.Helper()
	if provider == "" {
		provider = "muse"
	}
	if len(edits) == 0 {
		edits = map[string]string{"src/game.txt": "rules v2\n"}
	}
	h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: []string{config.LabelBuild}})
	r := workFake(map[string]runner.Handler{"build": crossTimedBuilder(edits), "review": workReviewer(workApprove)})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("provider"), result.Get("status")}, []any{provider, "approved"})
	assertEqual(t, workRoles(r), []string{"plan", "build", "review"})
	models := map[string]bool{}
	for _, c := range r.Calls {
		models[c.Model] = true
	}
	assertEqual(t, models, setOf(crossMedium[provider][0]))
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	return pr, result
}

// crossBuildForAudit is crossBuildOnMedium's run with an in-run reviewer that never approves:
// Muse (or `provider`) builds it, its own reviewer asks for changes in each of the lane's rounds
// (2 in these tests), and the kept work opens a pull request that waits for its final audit
// (#725, #2748), with no approval on its head.
func crossBuildForAudit(t *testing.T, h *flowHarness, provider string) (int, *pyjson.Object) {
	t.Helper()
	if provider == "" {
		provider = "muse"
	}
	h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: []string{config.LabelBuild}})
	r := workFake(map[string]runner.Handler{"build": crossTimedBuilder(map[string]string{"src/game.txt": "rules v2\n"}),
		"fix":    workBuilder(map[string]string{"src/game.txt": "rules v2, fixed\n"}),
		"review": workReviewer(workChanges("It skips the replay check."))})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("provider"), result.Get("status")}, []any{provider, "built"})
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelCross, "difficulty:medium"))
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	return pr, result
}

// Muse builds a medium item and its own run's reviewer, a fresh adversarial session on Muse's
// lane, approves it: that ships it at the end of the run (2026-11-02, #2748), with no review run.
// Its votes and its clearance say Muse both built it and approved it, so the statistics and the
// spot-check can tell it from a head another subscription approved.
func TestCross_AMediumItemShipsOnItsOwnLanesReview(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, result := crossBuildOnMedium(t, h, "", nil)
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelMerging, "difficulty:medium"))
	votes := dig(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "votes")
	assertEqual(t, votes, pyjson.New("sha", result.Get("head"), "builder", "muse", "built_by", "muse",
		"approvals", []any{"muse"}, "rejections", []any{},
		"tiers", pyjson.New("muse", "medium"),
		"reviews", []any{pyjson.New("family", "muse", "tier", "medium", "provider", "muse", "run", "777")}))
	cleared := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "cleared")
	assertEqual(t, []any{cleared.Get("sha"), cleared.Get("built_by"), dig(t, cleared, "approver", "provider"),
		dig(t, cleared, "approver", "tier"), cleared.Get("by")},
		[]any{result.Get("head"), "muse", "muse", "medium", "`muse` (medium)"})
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "merge_owner"), "muse")
	assertIn(t, "Auto-merge is on", lastOf(t, h.gh.BotComments(12)))
	assertEqual(t, dig(t, h.ctx.Store.Load(), "providers", "muse", "spent", -1, "minutes"), 10.0)
	// No review run is queued for it: the next run finds nothing to do.
	crossNextRun(h, "778")
	planned, _ := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, planned.Get("action"), "none")
}

// A medium item whose own run's reviewer never approved it waits for its final audit, a medium
// model on another subscription first: agy, by day, though Muse built it and comes first in the
// tier's order (#2748). agy's approval ships it, with no Opus gate.
func TestCross_AMediumItemsFinalAuditGoesToAnotherSubscriptionFirst(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, result := crossBuildForAudit(t, h, "")
	votes := dig(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "votes")
	assertEqual(t, votes, pyjson.New("sha", result.Get("head"), "builder", "muse", "built_by", "muse",
		"approvals", []any{}, "rejections", []any{}, "tiers", pyjson.New(), "reviews", []any{}))
	assertIn(t, "Its own reviewer read it 2 time(s) in the run without approving it, so it waits for one "+
		"medium or strong model's approval in a final audit, another subscription's when one is free or "+
		"else its own", lastOf(t, h.gh.BotComments(12)))
	crossNextRun(h, "778")
	planned, review := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider"), dig(t, planned, "seats", "review", "tier")},
		[]any{"review", "agy", "medium"})
	assertEqual(t, []any{review.Get("status"), review.Get("verdict")}, []any{"reviewed", "approve"})
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	reviews := []any{}
	for _, r := range pyjson.Objs(dig(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "votes", "reviews")) {
		reviews = append(reviews, []any{dig(t, r, "family"), dig(t, r, "provider")})
	}
	assertEqual(t, reviews, []any{[]any{"gemini", "agy"}})
	cleared := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "cleared")
	assertEqual(t, []any{cleared.Get("built_by"), dig(t, cleared, "approver", "provider")}, []any{"muse", "agy"})
}

// agy alone is set up: its own reviewer never approves in the run, so the head waits for its final
// audit, and with no other subscription set up its own lane gives it (2026-11-02, #2748): a fresh
// review run on agy, whose approval ships it.
func TestCross_WithNoOtherSubscriptionSetUpItsOwnLaneGivesTheFinalAudit(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"agy"}, At: testsupport.Day})
	pr, _ := crossBuildForAudit(t, h, "agy")
	assertNotIn(t, "No subscription that could give that review is set up", lastOf(t, h.gh.BotComments(12)))
	crossNextRun(h, "778")
	planned, review := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider"), review.Get("verdict")},
		[]any{"review", "agy", "approve"})
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelMerging, "difficulty:medium"))
}

func TestCross_ADeliverJobRunAgainDoesNotCountItsReviewTwice(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"agy"}, At: testsupport.Day})
	pr, result := crossBuildOnMedium(t, h, "agy", nil)
	d := deliver.NewDeliverer(h.ctx, pyjson.New("action", "review", "provider", "agy"),
		filepath.Join(h.root, "unused"), h.deliverRepo, "", nil, "")
	d.ReviewSeat = h.cfg.Pool.Seat("agy", "")
	votes := d.Vote(pr, crossStr(t, result, "head"), true, false, nil) // run 777 again
	assertEqual(t, len(dig(t, votes, "reviews").([]any)), 1)
	assertTrue(t, d.RuleMet(votes, "medium")) // agy built it, and its own lane's approval ships it (2026-11-02)
}

// An item still labelled `difficulty:easy` (retired in #2748) is a medium one: Muse builds it from
// the plan in its description, and its own run's medium approval ships it (2026-11-02).
func TestCross_AnItemStillLabelledEasyShipsAsAMediumOne(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"lite", "muse", "agy"}, At: testsupport.Day})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:easy"},
		Body: "<!-- jackioh-bot:plan -->\n## Plan\n1. Edit src/game.txt.\n<!-- /jackioh-bot:plan -->"})
	planned, result := h.night(workFake(map[string]runner.Handler{
		"build":  workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("provider"), planned.Get("difficulty"), result.Get("status")},
		[]any{"muse", "medium", "approved"})
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertIn(t, "Auto-merge is on", lastOf(t, h.gh.BotComments(12)))
	votes := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "votes")
	got := []any{}
	for _, r := range pyjson.Objs(votes.Get("reviews")) {
		got = append(got, []any{dig(t, r, "family"), dig(t, r, "tier")})
	}
	assertEqual(t, got, []any{[]any{"muse", "medium"}})
}

// Only claude-3 is set up: it plans, builds and reviews the item, and its own strong review ships
// it (2026-11-02, #2748), with no review run.
func TestCross_TheBuildersOwnStrongReviewShipsIt(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN_3"}})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	r := workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("provider"), result.Get("status")}, []any{"claude-3", "approved"})
	// Sonnet, medium, builds the (unrated, so medium) item; Opus plans and reviews it.
	got := [][2]string{}
	for _, c := range r.Calls {
		got = append(got, [2]string{c.Role, c.Model})
	}
	assertEqual(t, got, [][2]string{{"plan", "opus"}, {"build", "sonnet"}, {"review", "opus"}})
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelMerging, "difficulty:medium"))
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	assertEqual(t, []any{dig(t, record, "votes", "built_by"), dig(t, record, "cleared", "built_by"),
		dig(t, record, "cleared", "approver", "provider"), dig(t, record, "cleared", "approver", "tier")},
		[]any{"claude-3", "claude-3", "claude-3", "strong"})
}

// A medium item's review run takes a medium seat on another subscription first (#2748), agy, even
// with a strong one free.
func TestCross_AReviewRunOfAMediumItemTakesAMediumSeatFirst(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, _ := crossBuildForAudit(t, h, "")
	h.committedHours() // claude-3 works all day: a strong model is free too
	planned, review := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"review", "agy"})
	assertEqual(t, dig(t, planned, "seats", "review", "tier"), "medium")
	assertEqual(t, review.Get("verdict"), "approve")
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge) // one medium approval is enough
}

// Usage pauses never reach a verdict, so they never spend one (#725): two rejections, then a
// pause, then one more rejection uses up the lane's 3 rounds across the two runs.
func TestCross_APauseDoesNotSpendAVerdictRound(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day,
		Overrides: pyjson.New("max_review_cycles", 3)})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})

	limited := func(*runner.RunRequest) *runner.RunResult {
		r := runner.NewRunResult(false, "")
		r.ExitCode, r.Error, r.ResetAt = 1, "You've hit your limit", "2026-09-30T05:00:00Z"
		return r
	}

	planned, result := h.night(runner.NewFakeRunner(map[string][]runner.Handler{
		"build":  {workBuilder(map[string]string{"src/game.txt": "v2\n"})},
		"fix":    {workBuilder(map[string]string{"src/game.txt": "v3\n"})},
		"review": {workReviewer(workChanges("No.")), workReviewer(workChanges("No.")), limited}}))
	assertEqual(t, []any{planned.Get("provider"), result.Get("status")}, []any{"muse", "interrupted"})
	record := crossObj(t, h.ctx.Store.Load(), "items", "12")
	assertEqual(t, []any{dig(t, record, "verdict_rounds"), dig(t, record, "round_tier")}, []any{2, "medium"})
	// One round left in the lane: the pause parked Muse, so agy (same tier) picks it up
	// and one more rejection ends the lane phase.
	r := workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"fix":    workBuilder(map[string]string{"src/game.txt": "v3\n"}),
		"review": workReviewer(workChanges("No."))})
	planned, result = h.night(r)
	assertEqual(t, []any{planned.Get("provider"), result.Get("status")}, []any{"agy", "built"})
	assertEqual(t, crossCount(workRoles(r), "review"), 1)
	assertIn(t, "its final audit, another subscription's when one is free", crossStr(t, result, "reason"))
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelCross, "difficulty:medium"))
	record = crossObj(t, h.ctx.Store.Load(), "items", "12")
	assertEqual(t, []any{dig(t, record, "verdict_rounds"), dig(t, record, "round_tier")}, []any{3, "medium"})
}

// A Strong audit's rejection is answered at its own tier (#725): the auditor, claude-3, owns the
// fix (#2748) and makes it with Opus in its own run, and its own reviewer's approval ships the fix
// (2026-11-02), with no second review run.
func TestCross_AStrongAuditsFixShipsOnItsOwnRunsApproval(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	h.committedHours() // claude-3 works all day: a strong model is free too
	pr, _ := crossBuildForAudit(t, h, "")
	flowUpdateItem(h.ctx, pr, pyjson.New("review_floor", "strong")) // `/harness review strong`
	planned, _ := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workChanges("It skips the replay check."))}))
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"review", "claude-3"})
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	assertEqual(t, dig(t, record, "audit_tier"), "strong")
	r := workFake(map[string]runner.Handler{"revise": workBuilder(map[string]string{"src/game.txt": "rules v2 + replay\n"}),
		"review": workReviewer(workApprove)})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider"), result.Get("status")},
		[]any{"revise", "claude-3", "approved"})
	assertEqual(t, crossModels(r), []string{"opus", "opus"})
	assertIn(t, "It skips the replay check.", r.Calls[0].Prompt)
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertNotIn(t, config.LabelCross, h.gh.LabelNames(pr))
	cleared := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "cleared")
	assertEqual(t, []any{cleared.Get("built_by"), dig(t, cleared, "approver", "provider")},
		[]any{"claude-3", "claude-3"})
}

func TestCross_FindingsSendItBackAndItStopsAfterThree(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	h.committedHours() // claude-3 works all day: a strong model is free too
	pr, _ := crossBuildForAudit(t, h, "")
	h.night(workFake(map[string]runner.Handler{"review": workReviewer(workChanges("It skips the replay check."))}))
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelRevise, "difficulty:medium"))
	assertEqual(t, []any{dig(t, record, "source"), dig(t, record, "cross_rounds")}, []any{"cross-review", 1})
	// agy's audit asked for changes, so agy owns the fix and revises it itself (#2748), though Muse,
	// which built the head, comes first in the medium tier's order.
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", "12", "owner"), "agy")
	// Two failed revisions are strikes two and three (each failed for the item's own sake),
	// so the third steps it up instead of blocking it for a person.
	seen := map[string]string{}

	broken := func(request *runner.RunRequest) *runner.RunResult {
		seen["prompt"] = request.Prompt
		r := runner.NewRunResult(false, "I cannot fix this; the approach is wrong.")
		r.ExitCode, r.Error, r.DurationS = 1, "the model gave up", 120.0
		return r
	}

	for range 2 {
		planned, result := h.night(workFake(map[string]runner.Handler{"revise": broken}))
		assertEqual(t, []any{planned.Get("action"), planned.Get("provider"), result.Get("status")},
			[]any{"revise", "agy", "failed"})
	}
	assertIn(t, "It skips the replay check.", seen["prompt"])
	// Three strikes (#317 parts 4 and 8): instead of blocking it for a
	// person, the bot raises the issue a step and builds it again from main.
	assertEqual(t, h.gh.Threads.Item(pr).Get("state"), "closed")
	assertIn(t, "difficulty:hard", h.gh.LabelNames(12))
	assertIn(t, config.LabelBuild, h.gh.LabelNames(12))
	said := lastOf(t, h.gh.BotComments(12))
	assertIn(t, "This failed 3 times at `difficulty:medium`", said)
	assertIn(t, fmt.Sprintf("#%d is closed and #12 is queued to build again from `main`", pr), said)
	assertIn(t, "Closed to build #12 again from `main`", lastOf(t, h.gh.BotComments(pr)))
	record = crossObj(t, h.ctx.Store.Load(), "items", "12")
	assertEqual(t, []any{dig(t, record, "previous_pr"), dig(t, record, "strikes")}, []any{pr, 0})
	assertTrue(t, strings.HasPrefix(crossStr(t, record, "previous_branch"), "bot/old/issue-12-"))
}

func TestCross_AReviewOfAHeadThatMovedDoesNotCount(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	h.committedHours() // claude-3 works all day: a strong model is free too
	pr, _ := crossBuildForAudit(t, h, "")
	moved := func(request *runner.RunRequest) *runner.RunResult {
		// Someone pushes to the branch while the reviewer reads it.
		fakes.PushBranch(h.origin, h.root, "bot/issue-12", map[string]string{"src/game.txt": "someone else\n"}, "bot/issue-12")
		return workReviewer(workApprove)(request)
	}
	h.night(workFake(map[string]runner.Handler{"review": moved}))
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelCross, "difficulty:medium"))
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	assertIn(t, "does not count", lastOf(t, h.gh.BotComments(pr)))
}

// A hard item is planned, built and reviewed in its run by Opus alone.
func TestCross_AHardItemIsPlannedAndBuiltByOpus(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:hard"}})
	r := workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("provider"), planned.Get("difficulty"), result.Get("status")},
		[]any{"claude-3", "hard", "approved"})
	assertEqual(t, workRoles(r), []string{"plan", "build", "review"})
	models := map[string]bool{}
	for _, c := range r.Calls {
		models[c.Model] = true
	}
	assertEqual(t, models, setOf("opus"))
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "difficulty"), "hard")
}

// A hard item ships on its own lane's Opus review (2026-11-02, #2748): claude-3 builds it with
// Opus and its own run's reviewer, Opus on the same account in a fresh adversarial session,
// approves it, which ships it at the end of that run with no review run. A medium seat's approval
// of a hard item still ships nothing: the head waits for a strong one, which its own lane's Opus
// may give when no other account's is free.
func TestCross_AHardItemShipsOnItsOwnLanesOpusReview(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:hard"}})
	r := workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("provider"), result.Get("status"), difficultySeatOf(t, planned, "review")},
		[]any{"claude-3", "approved", difficultySeat{"claude-3", "opus", "strong"}})
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelMerging, "difficulty:hard"))
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	assertEqual(t, []any{dig(t, record, "votes", "built_by"), dig(t, record, "cleared", "built_by"),
		dig(t, record, "cleared", "approver", "provider"), dig(t, record, "cleared", "approver", "model"),
		dig(t, record, "cleared", "approver", "tier")},
		[]any{"claude-3", "claude-3", "claude-3", "opus", "strong"})
	crossNextRun(h, "778")
	planned, _ = h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, planned.Get("action"), "none") // no review run
	// The rule itself: a strong approval from the builder's own subscription ships a hard head, a
	// medium one, from any subscription, does not.
	d := deliver.NewDeliverer(h.ctx, pyjson.New("action", "review", "provider", "claude-3"),
		filepath.Join(h.root, "unused"), h.deliverRepo, "", nil, "")
	own := pyjson.New("sha", "h", "builder", "claude", "built_by", "claude-3", "approvals", []any{"claude"},
		"rejections", []any{}, "reviews", []any{pyjson.New("family", "claude", "tier", "strong", "provider", "claude-3")})
	assertTrue(t, d.RuleMet(own, "hard"))
	medium := pyjson.New("sha", "h", "builder", "claude", "built_by", "claude-3", "approvals", []any{"muse"},
		"rejections", []any{}, "reviews", []any{pyjson.New("family", "muse", "tier", "medium", "provider", "muse")})
	assertFalse(t, d.RuleMet(medium, "hard"))
	assertTrue(t, d.RuleMet(medium, "medium"))
}

// ---------------------------------------------------------------------------- HandoffTests

func TestCross_TheNextAgentGetsTheNotesAndTheBranch(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})

	limited := func(*runner.RunRequest) *runner.RunResult {
		r := runner.NewRunResult(false, "")
		r.ExitCode, r.Error, r.ResetAt = 1, "hit your limit", "2026-09-30T04:00:00Z"
		return r
	}

	first := workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "half\n",
		work.NotesFile: "plan: rules v2\nnext: the Coin\n"}),
		"review": limited})
	planned, result := h.night(first)
	assertEqual(t, []any{planned.Get("provider"), result.Get("status")}, []any{"muse", "interrupted"})
	record := crossObj(t, h.ctx.Store.Load(), "items", "12")
	assertIn(t, "next: the Coin", crossStr(t, record, "handoff", "notes"))
	assertEqual(t, dig(t, record, "handoff", "provider"), "muse")
	assertTrue(t, pyjson.Truthy(dig(t, record, "planned_at"))) // planned in that run: not planned again
	// The notes never reach the branch.
	assertNotIn(t, work.NotesFile, fakes.Git(h.origin, "ls-tree", "-r", "--name-only", crossStr(t, result, "head")))
	// Muse is parked until its reset, so the lane's tier order picks agy (#725: the
	// owner keeps it only when it can build it). The next run is agy, told what happened.
	second := workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "whole\n"}),
		"review": workReviewer(workApprove)})
	planned, result = h.night(second)
	assertEqual(t, []any{planned.Get("provider"), result.Get("status")}, []any{"agy", "approved"})
	assertEqual(t, workRoles(second), []string{"build", "review"})
	prompt := second.Calls[0].Prompt
	assertIn(t, "## Picking up from another agent", prompt)
	assertIn(t, "next: the Coin", prompt)
	assertNotIn(t, "handoff", crossObj(t, h.ctx.Store.Load(), "items", "12"))
}

// ---------------------------------------------------------------------------- VaultDeliveryTests

func TestCross_DeliverKeepsASealedLoginAndRefusesAnythingElse(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	assertEqual(t, planned.Get("provider"), "muse")
	out := filepath.Join(h.root, "out-vault")
	mkdir(t, out)
	writeText(t, filepath.Join(out, "result.json"), pyjson.Dumps(pyjson.New("status", "failed", "reason", "boom")))
	sealed, err := vault.Seal(pyjson.New("provider", "muse", "files", pyjson.New("auth.json", "{}")), "the-secret")
	if err != nil {
		t.Fatal(err)
	}
	writeText(t, filepath.Join(out, "vault.enc"), sealed)
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	text, _ := h.gh.GetFile("vault/muse.enc", "_bot-state")
	assertEqual(t, strings.TrimSpace(text), sealed)
	// The next plan hands it to the model job, which alone can open it.
	planned = plan.Make(h.ctx, false, nil, "auto", nil)
	assertEqual(t, strings.TrimSpace(crossStr(t, planned, "vault")), sealed)
	writeText(t, filepath.Join(out, "vault.enc"), "not a vault")
	log := dig(t, deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run(), "log")
	assertIn(t, "ignored a vault that is not sealed", log)
}

// ---------------------------------------------------------------------------- ConflictCarryTests

// crossResolve is resolve(extra): a builder that resolves the conflict `main` left in
// src/game.txt, and writes `extra`.
func crossResolve(extra map[string]string) runner.Handler {
	return func(request *runner.RunRequest) *runner.RunResult {
		target := filepath.Join(request.Cwd, "src", "game.txt")
		text := mustReadText(target)
		if !strings.Contains(text, "<<<<<<<") {
			panic(&fakes.PyError{Type: "AssertionError", Msg: text})
		}
		crossPut(target, "rules v2, with main's\n")
		for name, content := range extra {
			crossPut(filepath.Join(request.Cwd, name), content)
		}
		return runner.NewRunResult(true, workDone)
	}
}

// crossConflicted is ConflictCarryTests.conflicted: `main` moves on and the branch no longer
// merges.
func crossConflicted(h *flowHarness, pr int, main map[string]string) {
	if len(main) == 0 {
		main = map[string]string{"src/game.txt": "rules from main\n"}
	}
	fakes.PushBranch(h.origin, h.root, "main", main)
	h.gh.Threads.Item(pr).Set("mergeable_state", "dirty")
}

// crossClearedPR is ConflictCarryTests.cleared_pr: Muse builds and approves it by day, which meets
// the rule (2026-11-02, #2748: its own lane's approval ships it), and then `main` conflicts with it.
func crossClearedPR(t *testing.T, h *flowHarness, edits, main map[string]string) (int, string) {
	t.Helper()
	pr, built := crossBuildOnMedium(t, h, "", edits)
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	assertEqual(t, dig(t, record, "cleared", "sha"), built.Get("head"))
	assertEqual(t, dig(t, record, "cleared", "by"), "`muse` (medium)")
	crossConflicted(h, pr, main)
	crossNextRun(h, "779")
	return pr, crossStr(t, built, "head")
}

// `main` conflicts with the cleared change only in `spec/INDEX.md`, which the committed
// resolvers take from `main` (`resolve.py`): the revision spends no builder session, its own
// reviewer approves the merge, and the clearance carries as for a builder's resolution.
func TestCross_AConflictTheHarnessResolvesCarriesWithNoBuilder(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, clearedSHA := crossClearedPR(t, h,
		map[string]string{"src/game.txt": "rules v2\n", "spec/INDEX.md": "| R12 |\n"},
		map[string]string{"spec/INDEX.md": "| R13 |\n"})
	r := workFake(map[string]runner.Handler{"review": workReviewer(workApprove)})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("action"), planned.Get("source")}, []any{"revise", "conflict"})
	assertEqual(t, result.Get("status"), "approved", result.Get("reason"))
	assertEqual(t, workRoles(r), []string{"review"})
	assertIn(t, "the harness resolved `spec/INDEX.md` itself (took `main`'s version)", r.Calls[0].Prompt)
	assertEqual(t, dig(t, result, "cycles", 0, "builder", "role"), "merge")
	// Delivered: auto-merge back on at the merged head, the clearance carried over it.
	assertEqual(t, h.gh.AutoMergeHeads[fmt.Sprintf("PR_%d", pr)], result.Get("head"))
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	carried := crossObj(t, record, "votes", "carried")
	assertEqual(t, []any{dig(t, carried, "from"), dig(t, carried, "conflicts")},
		[]any{clearedSHA, []any{"spec/INDEX.md"}})
	assertIn(t, "This revision only merged `main` and resolved the conflicts in `spec/INDEX.md`, and its own "+
		"reviewer approved that, so the clearance carries over", lastOf(t, h.gh.BotComments(pr)))
	check := filepath.Join(h.root, "check-merge")
	fakes.Git(h.root, "clone", "-q", h.origin, check)
	assertEqual(t, fakes.Git(check, "show", "origin/bot/issue-12:spec/INDEX.md"), "| R13 |")
	assertEqual(t, fakes.Git(check, "show", "origin/bot/issue-12:src/game.txt"), "rules v2")
}

func TestCross_AConflictOnAClearedChangeShipsOnItsOwnReview(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, clearedSHA := crossClearedPR(t, h, nil, nil)
	r := workFake(map[string]runner.Handler{"revise": crossResolve(nil), "review": workReviewer(workApprove)})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("action"), planned.Get("source"), planned.Get("provider")},
		[]any{"revise", "conflict", "muse"})
	assertEqual(t, []any{result.Get("status"), dig(t, planned, "cleared", "sha")}, []any{"approved", clearedSHA})
	comments := h.gh.BotComments(pr)
	assertIn(t, "it merges without another review run", comments[len(comments)-2])
	assertEqual(t, workRoles(r), []string{"revise", "review"})
	assertIn(t, "Resolve the conflicts and change nothing else", r.Calls[0].Prompt)
	assertIn(t, "yours is the only review of the resolution", r.Calls[1].Prompt)
	assertIn(t, "resolved the conflicts in `src/game.txt`", r.Calls[1].Prompt)
	// No review run: auto-merge is back on, pinned to the resolved head, and says so (#2748).
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelMerging, "difficulty:medium"))
	assertEqual(t, h.gh.AutoMergeHeads[fmt.Sprintf("PR_%d", pr)], result.Get("head"))
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	families := []any{}
	for _, r := range pyjson.Objs(dig(t, record, "votes", "reviews")) {
		families = append(families, dig(t, r, "family"))
	}
	assertEqual(t, families, []any{"muse"})
	carried := crossObj(t, record, "votes", "carried")
	assertEqual(t, []any{dig(t, carried, "from"), dig(t, carried, "conflicts")},
		[]any{clearedSHA, []any{"src/game.txt"}})
	// The resolved head is cleared in turn, so the next conflict carries again.
	assertEqual(t, []any{dig(t, record, "cleared", "sha"), dig(t, record, "cleared", "carried_from")},
		[]any{result.Get("head"), clearedSHA})
	assertEqual(t, []any{dig(t, record, "cleared", "by"), dig(t, record, "cleared", "resolved_by")},
		[]any{"`muse` (medium)", "`muse` (medium)"})
	comment := lastOf(t, h.gh.BotComments(pr))
	assertIn(t, "The reviews had cleared this change at `"+clearedSHA[:12]+"` (`muse` (medium)). This "+
		"revision only merged `main` and resolved the conflicts in `src/game.txt`, and its own "+
		"reviewer approved that, so the clearance carries over and no review run is needed. Auto-merge is on",
		comment)
}

// A resolution that changes more than the conflicts carries nothing: it goes back under the rule,
// which its own run's reviewer meets for a medium item (2026-11-02, #2748), so it is cleared afresh.
func TestCross_AResolutionThatChangesMoreCarriesNothing(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, clearedSHA := crossClearedPR(t, h, nil, nil)
	_, result := h.night(workFake(map[string]runner.Handler{
		"revise": crossResolve(map[string]string{"src/extra.txt": "more\n"}),
		"review": workReviewer(workApprove)}))
	assertEqual(t, result.Get("status"), "approved")
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	assertNotIn(t, "carried", crossObj(t, record, "votes"))
	assertEqual(t, dig(t, record, "cleared", "sha"), result.Get("head"))
	assertNotIn(t, "carried_from", crossObj(t, record, "cleared"))
	assertNotEqual(t, result.Get("head"), clearedSHA)
	assertEqual(t, h.gh.AutoMergeHeads[fmt.Sprintf("PR_%d", pr)], result.Get("head"))
	assertIn(t, "but that does not carry over: it changed more than the conflicts (`src/extra.txt`). Auto-merge is on",
		lastOf(t, h.gh.BotComments(pr)))
}

func TestCross_ABranchThatMovedAfterTheReviewsDoesNotCarry(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, _ := crossClearedPR(t, h, nil, nil)
	// Someone pushed to the branch after the reviews cleared it; nobody reviewed that.
	fakes.PushBranch(h.origin, h.root, "bot/issue-12", map[string]string{"src/other.txt": "unreviewed\n"}, "bot/issue-12")
	r := workFake(map[string]runner.Handler{"revise": crossResolve(nil), "review": workReviewer(workApprove)})
	_, result := h.night(r)
	assertNotIn(t, "change nothing else", r.Calls[0].Prompt)
	assertNotIn(t, "yours is the only review", r.Calls[1].Prompt)
	// No carry, but the revision's own reviewer approved its head, which meets the rule (2026-11-02).
	assertNotIn(t, "carried", crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "votes"))
	assertEqual(t, h.gh.AutoMergeHeads[fmt.Sprintf("PR_%d", pr)], result.Get("head"))
	assertIn(t, "but that does not carry over: the branch had moved on from the commit they cleared",
		lastOf(t, h.gh.BotComments(pr)))
}

// Its builder's own reviewer never approved it: the change was never cleared, so its resolution
// carries nothing and goes under the rule like any revision, which its own reviewer's approval
// meets (2026-11-02, #2748).
func TestCross_AChangeShortOfTheRuleHasNothingToCarry(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, _ := crossBuildForAudit(t, h, "")
	assertNotIn(t, "cleared", crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr)))
	crossConflicted(h, pr, nil)
	crossNextRun(h, "778")
	r := workFake(map[string]runner.Handler{"revise": crossResolve(nil), "review": workReviewer(workApprove)})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("source"), result.Get("status")}, []any{"conflict", "approved"})
	assertNotIn(t, "cleared", planned)
	assertNotIn(t, "yours is the only review", r.Calls[1].Prompt)
	assertNotIn(t, "carried", crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "votes"))
	assertEqual(t, h.gh.AutoMergeHeads[fmt.Sprintf("PR_%d", pr)], result.Get("head"))
	assertNotIn(t, "cleared this change", lastOf(t, h.gh.BotComments(pr)))
}

// A conflict goes only to a builder with a reviewer in its own run (#317 part 2): the weak one
// once committed conflict markers in SPEC §11 again and again (#203, #214, #287), and since #2748
// it takes no revision at all. agy takes the conflict, cleared or not, and its own reviewer can
// carry a clearance.
func TestCross_AConflictRevisionGoesToABuilderThatReviewsInItsOwnRun(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, Cfg: testsupport.MakeConfig(testsupport.Config{
		Env: map[string]string{"HARNESS_SECRETS_SET": ""}, Machine: []string{"lite", "agy"}})})
	gh.AddPull(9, "bot/issue-2", fakes.Pull{Labels: []string{config.LabelPR, config.LabelRevise, "difficulty:easy"}})
	update := func(fields *pyjson.Object) {
		ctx.Store.Update(func(s *pyjson.Object) any { state.Item(s, 9).Update(fields); return nil }, "state")
	}
	update(pyjson.New("kind", "revise", "source", "request"))
	assertEqual(t, plan.Peek_(ctx, false, nil, "auto").Provider, "agy")
	update(pyjson.New("kind", "revise", "source", "conflict"))
	assertEqual(t, plan.Peek_(ctx, false, nil, "auto").Provider, "agy")
	update(pyjson.New("cleared", pyjson.New("sha", "abc", "by", "`claude` (strong)")))
	planned := plan.Make(ctx, false, nil, "auto", nil)
	assertEqual(t, []any{planned.Get("provider"), dig(t, planned, "cleared", "sha")}, []any{"agy", "abc"})
	assertIn(t, "it merges without another review run", lastOf(t, gh.BotComments(9)))
}
