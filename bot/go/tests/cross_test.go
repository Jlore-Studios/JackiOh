package tests

// test_cross.py: several subscriptions end to end: the review rule (one strong approval, or two
// medium ones, the same model twice included, or for an easy item a weak and a medium one),
// review runs, `difficulty:hard`, handoff and the vault, through plan, work and deliver against a
// real repository and the fake GitHub.
//
// It also holds test_cross.py's ALL, timed_builder, next_run, MEDIUM and
// ReviewRuleTests.build_on_medium, which other files import (crossAll, crossTimedBuilder,
// crossNextRun, crossMedium, crossBuildOnMedium), and small helpers that test_fullsend.py,
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
// builds and reviews it in one run, and its own approval is one medium vote.
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

// Muse's in-run approval never ships a medium item (#725 keeps Strong-only): the head waits for
// its Strong final audit, and no medium review run goes out meanwhile.
func TestCross_AMediumItemWaitsForItsStrongFinal(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, result := crossBuildOnMedium(t, h, "", nil)
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelCross, "difficulty:medium"))
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	votes := dig(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "votes")
	assertEqual(t, votes, pyjson.New("sha", result.Get("head"), "builder", "muse",
		"approvals", []any{"muse"}, "rejections", []any{},
		"tiers", pyjson.New("muse", "medium"),
		"reviews", []any{pyjson.New("family", "muse", "tier", "medium", "run", "777")}))
	assertIn(t, "`muse` (medium) approved it, so it waits for one strong model's final audit (Opus)",
		lastOf(t, h.gh.BotComments(12)))
	assertEqual(t, dig(t, h.ctx.Store.Load(), "providers", "muse", "spent", -1, "minutes"), 10.0)
	// No strong model is free by day: no review run goes out at all.
	planned, _ := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, planned.Get("action"), "none")
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelCross, "difficulty:medium"))
	// With a strong seat free, Opus audits, and that ships it.
	h.committedHours() // claude-3 works all day
	crossNextRun(h, "778")
	planned, review := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider"), dig(t, planned, "seats", "review", "tier")},
		[]any{"review", "claude-3", "strong"})
	assertEqual(t, []any{review.Get("status"), review.Get("verdict")}, []any{"reviewed", "approve"})
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
}

// agy builds and its own reviewer approves; with no Strong seat set up, no review run can take
// it to the rule, so the head waits for a person to merge it (#725: no medium review runs go out
// for a medium item).
func TestCross_WithNoStrongSeatSetUpItWaitsForAPerson(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"agy"}, At: testsupport.Day})
	pr, _ := crossBuildOnMedium(t, h, "agy", nil)
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	assertIn(t, "No subscription that could give that review is set up", lastOf(t, h.gh.BotComments(12)))
	crossNextRun(h, "778")
	planned, _ := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, planned.Get("action"), "none")
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelCross, "difficulty:medium"))
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
	assertFalse(t, d.RuleMet(votes, "medium"))
}

// The weak one builds an easy item and checks itself, which is no review; the lane's Medium
// final audit then ships it, with no Opus gate (#725). No weak review run goes out.
func TestCross_AnEasyItemShipsOnItsMediumAudit(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"lite", "agy"}, At: testsupport.Day})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:easy"},
		Body: "<!-- jackioh-bot:plan -->\n## Plan\n1. Edit src/game.txt.\n<!-- /jackioh-bot:plan -->"})
	planned, result := h.night(workFake(map[string]runner.Handler{
		"build":      workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"self_check": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("provider"), result.Get("status")}, []any{"lite", "built"})
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	assertIn(t, "a medium model's final audit", lastOf(t, h.gh.BotComments(12)))
	// The Medium audit approves, and that ships the easy item.
	crossNextRun(h, "778")
	planned, review := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("provider"), dig(t, planned, "seats", "review", "tier")}, []any{"agy", "medium"})
	assertEqual(t, review.Get("verdict"), "approve")
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	votes := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "votes")
	got := []any{}
	for _, r := range pyjson.Objs(votes.Get("reviews")) {
		got = append(got, []any{dig(t, r, "family"), dig(t, r, "tier")})
	}
	assertEqual(t, got, []any{[]any{"gemini", "medium"}})
}

func TestCross_OneStrongReviewIsEnough(t *testing.T) {
	// Only claude-3 is set up: it plans, builds and reviews the item, and its one strong
	// review ships it (#695).
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
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "votes", "tiers"), pyjson.New("claude", "strong"))
}

func TestCross_AReviewRunPrefersAStrongModel(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, _ := crossBuildOnMedium(t, h, "", nil)
	h.committedHours() // claude-3 works all day: a strong model is free
	planned, review := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"review", "claude-3"})
	assertEqual(t, dig(t, planned, "seats", "review", "tier"), "strong")
	assertEqual(t, review.Get("verdict"), "approve")
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge) // one strong approval is enough
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
	assertIn(t, "Strong final audit", crossStr(t, result, "reason"))
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelCross, "difficulty:medium"))
	record = crossObj(t, h.ctx.Store.Load(), "items", "12")
	assertEqual(t, []any{dig(t, record, "verdict_rounds"), dig(t, record, "round_tier")}, []any{3, "medium"})
}

// A Strong audit's rejection is answered at its own tier (#725): Opus fixes the medium item in
// its own run, its own reviewer approves, and that ships it.
func TestCross_AStrongReviseAfterRejectionShips(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	h.committedHours() // claude-3 works all day: the Strong final can audit
	pr, _ := crossBuildOnMedium(t, h, "", nil)
	h.night(workFake(map[string]runner.Handler{"review": workReviewer(workChanges("It skips the replay check."))}))
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
}

func TestCross_FindingsSendItBackAndItStopsAfterThree(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	h.committedHours() // claude-3 works all day: the Strong final can audit
	pr, _ := crossBuildOnMedium(t, h, "", nil)
	h.night(workFake(map[string]runner.Handler{"review": workReviewer(workChanges("It skips the replay check."))}))
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelRevise, "difficulty:medium"))
	assertEqual(t, []any{dig(t, record, "source"), dig(t, record, "cross_rounds")}, []any{"cross-review", 1})
	// The revision answers the Strong audit, so it stays with a Strong seat (#725).
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
			[]any{"revise", "claude-3", "failed"})
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
	h.committedHours() // claude-3 works all day: the Strong final can audit
	pr, _ := crossBuildOnMedium(t, h, "", nil)
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

func TestCross_AHardItemIsPlannedAndBuiltByOpus(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:hard"}})
	r := workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("provider"), planned.Get("difficulty"), result.Get("status")},
		[]any{"claude-3", "hard", "approved"})
	models := map[string]bool{}
	for _, c := range r.Calls {
		models[c.Model] = true
	}
	assertEqual(t, models, setOf("opus"))
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, "difficulty:hard"))
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "difficulty"), "hard")
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
	text, _ := h.gh.GetFile("vault/muse.enc", "bot-state")
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

// crossClearedPR is ConflictCarryTests.cleared_pr: Muse builds and approves it by day; at night
// a Strong seat reviews it: the rule is met (#695), and then `main` conflicts with it.
func crossClearedPR(t *testing.T, h *flowHarness, edits, main map[string]string) (int, string) {
	t.Helper()
	pr, built := crossBuildOnMedium(t, h, "", edits)
	crossNextRun(h, "778")
	testsupport.ClockOf(h.ctx).At = testsupport.Night
	planned, _ := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("provider"), dig(t, planned, "seats", "review", "tier")},
		[]any{"claude-3", "strong"})
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	assertEqual(t, dig(t, record, "cleared", "sha"), built.Get("head"))
	assertEqual(t, dig(t, record, "cleared", "by"), "`muse` (medium), `claude` (strong)")
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
	// No review run: auto-merge is back on, pinned to the resolved head.
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, "difficulty:medium"))
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
		[]any{"`muse` (medium), `claude` (strong)", "`muse` (medium)"})
	comment := lastOf(t, h.gh.BotComments(pr))
	assertIn(t, "The reviews had cleared this change at `"+clearedSHA[:12]+"` (`muse` (medium), `claude` "+
		"(strong)). This revision only merged `main` and resolved the conflicts in `src/game.txt`, and its own "+
		"reviewer approved that, so the clearance carries over and no review run is needed. Auto-merge is on",
		comment)
}

func TestCross_AResolutionThatChangesMoreGoesBackToReview(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, clearedSHA := crossClearedPR(t, h, nil, nil)
	_, result := h.night(workFake(map[string]runner.Handler{
		"revise": crossResolve(map[string]string{"src/extra.txt": "more\n"}),
		"review": workReviewer(workApprove)}))
	assertEqual(t, result.Get("status"), "approved")
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelCross, "difficulty:medium"))
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	assertNotIn(t, "carried", crossObj(t, record, "votes"))
	assertEqual(t, dig(t, record, "cleared", "sha"), clearedSHA)
	assertIn(t, "but that does not carry over: it changed more than the conflicts (`src/extra.txt`). `muse` "+
		"(medium) approved it, so it waits for one strong model's final audit (Opus)",
		lastOf(t, h.gh.BotComments(pr)))
}

func TestCross_ABranchThatMovedAfterTheReviewsDoesNotCarry(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, _ := crossClearedPR(t, h, nil, nil)
	// Someone pushed to the branch after the reviews cleared it; nobody reviewed that.
	fakes.PushBranch(h.origin, h.root, "bot/issue-12", map[string]string{"src/other.txt": "unreviewed\n"}, "bot/issue-12")
	r := workFake(map[string]runner.Handler{"revise": crossResolve(nil), "review": workReviewer(workApprove)})
	h.night(r)
	assertNotIn(t, "change nothing else", r.Calls[0].Prompt)
	assertNotIn(t, "yours is the only review", r.Calls[1].Prompt)
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelCross, "difficulty:medium"))
	assertIn(t, "but that does not carry over: the branch had moved on from the commit they cleared",
		lastOf(t, h.gh.BotComments(pr)))
}

// One medium approval, short of the Strong one: the change was never cleared, so its resolution
// waits for the review run like any revision.
func TestCross_AChangeShortOfTheRuleHasNothingToCarry(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, _ := crossBuildOnMedium(t, h, "", nil)
	assertNotIn(t, "cleared", crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr)))
	crossConflicted(h, pr, nil)
	crossNextRun(h, "778")
	r := workFake(map[string]runner.Handler{"revise": crossResolve(nil), "review": workReviewer(workApprove)})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("source"), result.Get("status")}, []any{"conflict", "approved"})
	assertNotIn(t, "cleared", planned)
	assertNotIn(t, "yours is the only review", r.Calls[1].Prompt)
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelCross, "difficulty:medium"))
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	assertNotIn(t, "cleared this change", lastOf(t, h.gh.BotComments(pr)))
}

// The weak one takes an easy revision first, but never a conflict (#317 part 2): nothing reviews
// in its run, and it committed conflict markers in SPEC §11 again and again (#203, #214, #287).
// agy takes the conflict, cleared or not, and its own reviewer can carry a clearance.
func TestCross_AConflictRevisionGoesToABuilderThatReviewsInItsOwnRun(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, Cfg: testsupport.MakeConfig(testsupport.Config{
		Env: map[string]string{"HARNESS_SECRETS_SET": ""}, Machine: []string{"lite", "agy"}})})
	gh.AddPull(9, "bot/issue-2", fakes.Pull{Labels: []string{config.LabelPR, config.LabelRevise, "difficulty:easy"}})
	update := func(fields *pyjson.Object) {
		ctx.Store.Update(func(s *pyjson.Object) any { state.Item(s, 9).Update(fields); return nil }, "state")
	}
	update(pyjson.New("kind", "revise", "source", "request"))
	assertEqual(t, plan.Peek_(ctx, false, nil, "auto").Provider, "lite")
	update(pyjson.New("kind", "revise", "source", "conflict"))
	assertEqual(t, plan.Peek_(ctx, false, nil, "auto").Provider, "agy")
	update(pyjson.New("cleared", pyjson.New("sha", "abc", "by", "`claude` (strong)")))
	planned := plan.Make(ctx, false, nil, "auto", nil)
	assertEqual(t, []any{planned.Get("provider"), dig(t, planned, "cleared", "sha")}, []any{"agy", "abc"})
	assertIn(t, "it merges without another review run", lastOf(t, gh.BotComments(9)))
}
