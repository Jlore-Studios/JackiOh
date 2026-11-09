package tests

// The own review run (#2748, the owner's decision of 2026-11-02; `own_review_run`): a build or a
// revision builds and runs the checks, opens or pushes its pull request (a draft, which CI runs
// on), and hands its lane to a second run on the same subscription, which reviews the change in
// fresh adversarial sessions, fixes what they find, reads its CI, approves it, marks it ready and
// turns auto-merge on. Through plan, work and deliver against a real repository and the fake
// GitHub, with the committed `own_review_run` (`flowHarnessOpts.Chain`).

import (
	"fmt"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/controller"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// ownReviewHarness is a flow harness with the committed chain, by day: Muse and agy and Codex on
// the machine, every Claude account's secret set but off its hours.
func ownReviewHarness(t *testing.T, machine []string, overrides ...*pyjson.Object) *flowHarness {
	t.Helper()
	if machine == nil {
		machine = testsupport.Machine
	}
	o := flowHarnessOpts{Env: crossAll, Machine: machine, At: testsupport.Day, Chain: true}
	if len(overrides) > 0 {
		o.Overrides = overrides[0]
	}
	return newFlowHarness(t, o)
}

// ownReviewBuild is night 1 of the chain: `provider` (Muse by default) plans and builds issue 12,
// runs the checks and reviews nothing; its deliver opens a draft pull request carrying
// `bot:reviewing`, reserves its lane for its own review run, and starts the next run at once.
func ownReviewBuild(t *testing.T, h *flowHarness, provider string, labels ...string) (int, *pyjson.Object) {
	t.Helper()
	if provider == "" {
		provider = "muse"
	}
	h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: append([]string{config.LabelBuild}, labels...)})
	r := workFake(map[string]runner.Handler{"build": crossTimedBuilder(map[string]string{"src/game.txt": "rules v2\n"}),
		"review": workReviewer(workApprove)})
	dispatched := len(h.gh.Dispatches)
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("provider"), planned.Get("own_review_next"), result.Get("status"),
		result.Get("review_next")}, []any{provider, true, "built", "own"})
	assertNotIn(t, "review", workRoles(r)) // the build reviews nothing itself
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	assertEqual(t, h.gh.GetPull(pr).Get("draft"), true) // CI runs on a draft
	assertIn(t, config.LabelOwnReview, h.gh.LabelNames(pr))
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	reserved := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), queue.OwnReviewKey)
	assertEqual(t, []any{reserved.Get("provider"), reserved.Get("head")}, []any{provider, result.Get("head")})
	assertEqual(t, len(h.gh.Dispatches), dispatched+1) // the own review run is started at once
	crossNextRun(h, "778")
	return pr, result
}

// ownReviewLaneState is what a test of its reserved lane prints when it fails (#3031: these two
// failed once on `main`'s CI and never again, and the assertion alone did not say why): the pull
// request's and its issue's records, the labels on both, and the lanes as `plan.ReadLanes` read them.
func ownReviewLaneState(h *flowHarness, pr int, lanes *plan.Lanes) string {
	items := h.ctx.Store.Load().Obj("items")
	return fmt.Sprintf("\nnow %s\n#%d record: %s\n#%d labels: %v\n#12 record: %s\n#12 labels: %v\nlanes: %s",
		clock.ISO(h.ctx.Now()), pr, pyjson.Dumps(items.Obj(strconv.Itoa(pr))), pr, h.gh.LabelNames(pr),
		pyjson.Dumps(items.Obj("12")), h.gh.LabelNames(12), lanes.Describe())
}

// Muse builds a medium item, and its deliver opens the pull request as a draft, so CI starts, and
// hands Muse's lane to its own review run: the next run is Muse's own review, a fresh adversarial
// session of Muse's reviewer, which approves it; its deliver records the votes (Muse built it,
// Muse approved it, which ships a medium item), marks the pull request ready and turns auto-merge
// on, with `bot:merging`, and Muse owns the merge.
func TestOwnReview_ABuildHandsItsLaneToItsOwnReviewWhichApprovesAndTurnsAutoMergeOn(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, built := ownReviewBuild(t, h, "")
	assertIn(t, "Its own review run is next, on `muse`", lastOf(t, h.gh.BotComments(12)))
	assertIn(t, "### Its own review", crossStr(t, h.gh.GetPull(pr), "body"))
	r := workFake(map[string]runner.Handler{"review": workReviewer(workApprove)})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("action"), planned.Get("number"), planned.Get("provider"),
		planned.Get("own_review"), dig(t, planned, "seats", "review", "model")},
		[]any{"review", pr, "muse", true, "muse-spark-1.3-contributor"})
	assertEqual(t, workRoles(r), []string{"review"})
	assertEqual(t, []any{result.Get("status"), result.Get("head")}, []any{"approved", built.Get("head")})
	assertEqual(t, h.gh.GetPull(pr).Get("draft"), false)
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelMerging, "difficulty:medium"))
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	assertNotIn(t, queue.OwnReviewKey, record)
	assertEqual(t, []any{dig(t, record, "votes", "built_by"), dig(t, record, "cleared", "approver", "provider"),
		record.Get("merge_owner")}, []any{"muse", "muse", "muse"})
	assertIn(t, "Auto-merge is on", lastOf(t, h.gh.BotComments(pr)))
	// Nothing more is queued for it.
	crossNextRun(h, "779")
	planned, _ = h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, planned.Get("action"), "none")
}

// ownReviewCalls is the roles a run's calls ran in, and the model of each.
func ownReviewCalls(r *runner.FakeRunner) [][2]string {
	got := [][2]string{}
	for _, c := range r.Calls {
		got = append(got, [2]string{c.Role, c.Model})
	}
	return got
}

// ownReviewPrompt is the prompt of the run's first call in `role`.
func ownReviewPrompt(t *testing.T, r *runner.FakeRunner, role string) string {
	t.Helper()
	for _, c := range r.Calls {
		if c.Role == role {
			return c.Prompt
		}
	}
	t.Fatalf("no %s call in %v", role, workRoles(r))
	return ""
}

// The own review finds a blocking problem: its builder fixes it in the same run, the checks run
// again, and a fresh review session approves the fix, which is pushed and ships (Muse built the
// new head too, so its votes name Muse; the rounds count across the review's rounds).
func TestOwnReview_ItFindsProblemsFixesThemInTheSameRunAndApproves(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, built := ownReviewBuild(t, h, "")
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"review": {workReviewer(workChanges("It skips the replay check.")), workReviewer(workApprove)},
		"fix":    {workBuilder(map[string]string{"src/game.txt": "rules v2, with the replay check\n"})}})
	_, result := h.night(r)
	assertEqual(t, workRoles(r), []string{"review", "fix", "review"})
	assertIn(t, "It skips the replay check.", ownReviewPrompt(t, r, "fix"))
	assertIn(t, "This is the change's own review run", ownReviewPrompt(t, r, "review"))
	assertEqual(t, result.Get("status"), "approved")
	head := crossStr(t, result, "head")
	assertNotEqual(t, head, built.Get("head"))
	assertEqual(t, h.originSHA("bot/issue-12"), head)
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	assertEqual(t, []any{dig(t, record, "votes", "sha"), dig(t, record, "votes", "built_by"),
		record.Get("merge_owner")}, []any{head, "muse", "muse"})
	// Its rounds count where its build's did: on the issue, as a build's own review always did.
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", "12", "verdict_rounds"), 2)
	assertNotIn(t, "verdict_rounds", record)
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertEqual(t, h.gh.AutoMergeHeads[fmt.Sprintf("PR_%d", pr)], head)
	said := lastOf(t, h.gh.BotComments(pr))
	assertIn(t, "approved it on round 2 of 2", said)
	assertIn(t, "pushed `"+head[:12]+"`", said)
}

// A required check red in CI on the head its build pushed is a finding of the own review run, which
// fixes it in the same run with the red job's log, though the reviewer approved: only the jobs a
// required check gathers count, and the gathering job (`web`) gives way to the one that failed.
func TestOwnReview_ARedCICheckItReadsIsFixedInTheSameRun(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, built := ownReviewBuild(t, h, "")
	h.gh.Runs.Set("900", pyjson.New("id", 900, "name", "CI", "workflow", "ci.yml", "head_sha", built.Get("head"),
		"status", "completed", "conclusion", "failure", "created_at", "2026-09-29T17:05:00Z",
		"html_url", "https://github.com/jgoetzmann/JackiOh/actions/runs/900"))
	h.gh.Jobs["900"] = []*pyjson.Object{
		pyjson.New("id", 9001, "name", "web (unit 1/2)", "conclusion", "failure",
			"steps", []any{pyjson.New("name", "vitest", "conclusion", "failure")}),
		pyjson.New("id", 9002, "name", "web", "conclusion", "failure", "steps", []any{}),
		pyjson.New("id", 9003, "name", "rust", "conclusion", "success", "steps", []any{}),
		pyjson.New("id", 9004, "name", "lint (an optional job)", "conclusion", "failure", "steps", []any{})}
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"review": {workReviewer(workApprove), workReviewer(workApprove)},
		"fix":    {workBuilder(map[string]string{"src/game.txt": "rules v2, the unit test fixed\n"})}})
	_, result := h.night(r)
	assertEqual(t, workRoles(r), []string{"review", "fix", "review"})
	fix := ownReviewPrompt(t, r, "fix")
	assertIn(t, "The required check `web (unit 1/2)` is red in CI on `"+crossStr(t, built, "head")[:12]+"`", fix)
	assertIn(t, "log of job 9001", fix)
	assertNotIn(t, "The required check `web` is red", fix)
	assertNotIn(t, "lint (an optional job)", fix)
	assertIn(t, "Each red required check is a blocking finding", ownReviewPrompt(t, r, "review"))
	first := pyjson.Objs(result.Get("cycles"))[0]
	assertEqual(t, dig(t, first, "ci", "red"), []any{"web (unit 1/2)"})
	assertEqual(t, result.Get("status"), "approved")
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
}

// The lane its build reserved is its own review run's alone: another queued item does not take it
// meanwhile, while the reservation lasts (`own_review_reserve_minutes`); the own review run does.
func TestOwnReview_TheReservedLaneIsNotTakenByAnotherItem(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"agy"}, At: testsupport.Day, Chain: true})
	assertEqual(t, h.cfg.OwnReviewReserveMinutes, 30)
	pr, _ := ownReviewBuild(t, h, "agy")
	h.gh.AddIssue(14, fakes.Issue{Title: "Something else", Labels: []string{config.LabelBuild}})
	lanes := plan.ReadLanes(h.ctx, h.ctx.Store.Load())
	assertEqual(t, []any{lanes.Held[pr], lanes.Reserved[pr]}, []any{"agy", true}, ownReviewLaneState(h, pr, lanes))
	assertIn(t, "(reserved for its own review)", lanes.Describe())
	builds := plan.Make(h.ctx, false, nil, "build", nil)
	assertEqual(t, builds.Get("action"), "none")
	assertNotIn(t, "14", fmt.Sprint(h.ctx.Store.Load().Obj("items").Obj("14").Get("run_id")))
	planned := flowMake(h.ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("number"), planned.Get("provider"), planned.Get("own_review")},
		[]any{"review", pr, "agy", true})
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), queue.OwnReviewKey, "claimed"), true)
}

// The reservation runs out (no own review run took it up in `own_review_reserve_minutes`): the lane
// is free again, and the subscription's next run goes on with its own review, its own work, before
// the other queued item.
func TestOwnReview_AReservationThatRunsOutStillLeavesItsOwnReviewFirst(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"agy"}, At: testsupport.Day, Chain: true})
	pr, _ := ownReviewBuild(t, h, "agy")
	h.gh.AddIssue(14, fakes.Issue{Title: "Something else", Labels: []string{config.LabelBuild}})
	testsupport.ClockOf(h.ctx).At = testsupport.Day.Add(31 * time.Minute)
	lanes := plan.ReadLanes(h.ctx, h.ctx.Store.Load())
	assertEqual(t, len(lanes.Held), 0)
	before := ownReviewLaneState(h, pr, lanes)
	planned := flowMake(h.ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("number"), planned.Get("provider")},
		[]any{"review", pr, "agy"}, before+"\nplanned: "+pyjson.Dumps(planned))
}

// A run that died while it reviewed puts its own review back on its lane: the next run of that
// subscription takes it up again, and it is never sent to a final audit for it.
func TestOwnReview_ADeadOwnReviewRunGoesBackToItsLane(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, _ := ownReviewBuild(t, h, "")
	planned := flowMake(h.ctx) // run 778 claims it, and dies before its work job starts
	assertEqual(t, []any{planned.Get("number"), planned.Get("provider")}, []any{pr, "muse"})
	crossNextRun(h, "779")
	again := flowMake(h.ctx)
	assertEqual(t, []any{again.Get("action"), again.Get("number"), again.Get("provider"), again.Get("own_review")},
		[]any{"review", pr, "muse", true})
	assertIn(t, "ended without finishing; it is back in the queue", strings.Join(h.gh.BotComments(pr), "\n"))
	assertIn(t, config.LabelOwnReview, h.gh.LabelNames(pr))
	assertNotIn(t, config.LabelCross, h.gh.LabelNames(pr))
}

// `/harness stop` on a pull request waiting for its own review run takes it out of the queue: its
// reservation and `bot:reviewing` go, and no run takes it up.
func TestOwnReview_AStopDropsTheReservation(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, _ := ownReviewBuild(t, h, "")
	assertIn(t, "out of the queue", queue.Stop(h.ctx, pr, "jgoetzmann"))
	assertNotIn(t, config.LabelOwnReview, h.gh.LabelNames(pr))
	assertNotIn(t, queue.OwnReviewKey, crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr)))
	assertEqual(t, flowMake(h.ctx).Get("action"), "none")
}

// The build leaves its subscription at its 5-hour cap: its own review run cannot start, so the
// hold applies, as the item's owner's (`bot:waiting`, its comment, no lane kept meanwhile); when the
// hold (`hold_minutes`) runs out before the subscription can start, its review is handed off to its
// final audit on another subscription (Muse), whose approval ships it.
func TestOwnReview_ACapAfterTheBuildHoldsItsOwnReviewThenHandsItToAFinalAudit(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Machine: []string{"muse"}, Chain: true})
	h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: []string{config.LabelBuild}})
	flowUpdateItem(h.ctx, 12, pyjson.New("owner", "claude-1", "owner_why", "it planned it"))
	planned, built := h.night(workFake(map[string]runner.Handler{
		"build": workBuilder(map[string]string{"src/game.txt": "rules v2\n"})}))
	assertEqual(t, []any{planned.Get("provider"), built.Get("review_next")}, []any{"claude-1", "own"})
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	// The build took claude-1 to its cap; the session resets in 50 minutes.
	ownershipReading(h.ctx, "claude-1", 0.99, 50, 0.2, 5000)
	crossNextRun(h, "778")
	held := flowMake(h.ctx)
	assertNotEqual(t, held.Get("number"), pr)
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelOwnReview, config.LabelWaiting,
		"difficulty:medium"))
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "hold", "owner"), "claude-1")
	said := lastOf(t, h.gh.BotComments(pr))
	assertIn(t, "**Its own review is held for `claude-1`**", said)
	assertIn(t, "it goes to its final audit on another subscription instead", said)
	assertFalse(t, plan.ReadLanes(h.ctx, h.ctx.Store.Load()).Reserved[pr]) // a capped lane keeps no slot
	// The hold runs out and claude-1 still cannot start: its final audit goes to Muse.
	hold := h.cfg.HoldMinutes
	testsupport.ClockOf(h.ctx).At = testsupport.Night.Add(time.Duration(hold+1) * time.Minute)
	ownershipReading(h.ctx, "claude-1", 0.99, hold+90, 0.2, 5000)
	crossNextRun(h, "779")
	audit, review := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, []any{audit.Get("action"), audit.Get("number"), audit.Get("provider"), audit.Truthy("own_review"),
		review.Get("verdict")}, []any{"review", pr, "muse", false, "approve"})
	assertIn(t, "**Its own review is handed off**", strings.Join(h.gh.BotComments(pr), "\n"))
	assertEqual(t, h.gh.GetPull(pr).Get("draft"), false)
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertNotIn(t, queue.OwnReviewKey, crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr)))
}

// A hard item's chain is Opus on the same Claude account: claude-3 plans and builds it on Opus,
// and its own review run, Opus on claude-3 in a fresh session, approves it, which ships it.
func TestOwnReview_AHardItemsChainIsOpusOnTheSameAccount(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN_3"},
		Chain: true})
	h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: []string{config.LabelBuild, "difficulty:hard"}})
	r := workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "rules v2\n"})})
	planned, built := h.night(r)
	assertEqual(t, []any{planned.Get("provider"), built.Get("review_next")}, []any{"claude-3", "own"})
	assertEqual(t, ownReviewCalls(r), [][2]string{{"plan", "opus"}, {"build", "opus"}})
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	crossNextRun(h, "778")
	r = workFake(map[string]runner.Handler{"review": workReviewer(workApprove)})
	reviewed, result := h.night(r)
	assertEqual(t, []any{reviewed.Get("provider"), dig(t, reviewed, "seats", "review", "model"),
		dig(t, reviewed, "seats", "review", "tier"), dig(t, reviewed, "seats", "build", "model"), result.Get("status")},
		[]any{"claude-3", "opus", "strong", "opus", "approved"})
	assertEqual(t, ownReviewCalls(r), [][2]string{{"review", "opus"}})
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	cleared := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "cleared")
	assertEqual(t, []any{cleared.Get("built_by"), dig(t, cleared, "approver", "provider"), dig(t, cleared, "approver", "tier")},
		[]any{"claude-3", "claude-3", "strong"})
}

// The own review's verdict rounds run out without an approval: a medium item goes to its final
// audit (another subscription first, the pull request marked ready for it), whose approval ships it.
func TestOwnReview_AMediumItemWhoseRoundsRunOutGoesToItsFinalAudit(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, _ := ownReviewBuild(t, h, "")
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"review": {workReviewer(workChanges("No.")), workReviewer(workChanges("Still no."))},
		"fix":    {workBuilder(map[string]string{"src/game.txt": "rules v2, again\n"})}})
	_, result := h.night(r)
	assertEqual(t, workRoles(r), []string{"review", "fix", "review"})
	assertEqual(t, result.Get("status"), "built")
	assertIn(t, "its final audit, another subscription's when one is free", crossStr(t, result, "reason"))
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelCross, "difficulty:medium"))
	assertEqual(t, h.gh.GetPull(pr).Get("draft"), false)
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	assertIn(t, "used its review rounds (2 of 2) without an approval", lastOf(t, h.gh.BotComments(pr)))
	crossNextRun(h, "779")
	audit, review := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, []any{audit.Get("provider"), audit.Truthy("own_review"), review.Get("verdict")},
		[]any{"agy", false, "approve"})
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
}

// A hard item's own review rounds running out are a strike, as G left it: the pull request is
// blocked and will not merge by itself.
func TestOwnReview_AHardItemWhoseRoundsRunOutIsStruck(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN_3"},
		Chain: true})
	h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: []string{config.LabelBuild, "difficulty:hard"}})
	h.night(workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "rules v2\n"})}))
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	crossNextRun(h, "778")
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"review": {workReviewer(workChanges("No.")), workReviewer(workChanges("Still no."))},
		"fix":    {workBuilder(map[string]string{"src/game.txt": "rules v2, again\n"})}})
	_, result := h.night(r)
	assertEqual(t, result.Get("status"), "not_approved")
	assertIn(t, config.LabelBlocked, h.gh.LabelNames(pr))
	assertNotIn(t, config.LabelCross, h.gh.LabelNames(pr))
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	assertIn(t, "so it will not merge by itself", lastOf(t, h.gh.BotComments(pr)))
	assertEqual(t, len(pyjson.List(dig(t, h.ctx.Store.Load(), "items", "12", "strike_log"))), 1)
}

// A revision a person asks for chains as a build does: its run pushes the revision (auto-merge off)
// and hands its lane to its own review run, which approves it and turns auto-merge on again.
func TestOwnReview_ARevisionAPersonAskedForChainsItsOwnReviewToo(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, _ := ownReviewBuild(t, h, "")
	h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	queue.QueueRevise(h.ctx, pr, "jgoetzmann", false, "request", false, nil, nil)
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	crossNextRun(h, "779")
	r := workFake(map[string]runner.Handler{"revise": workBuilder(map[string]string{"src/game.txt": "rules v3\n"}),
		"review": workReviewer(workApprove)})
	planned, revised := h.night(r)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider"), planned.Get("own_review_next"),
		revised.Get("status")}, []any{"revise", "muse", true, "built"})
	assertEqual(t, workRoles(r), []string{"revise"})
	assertIn(t, config.LabelOwnReview, h.gh.LabelNames(pr))
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	assertEqual(t, h.originSHA("bot/issue-12"), revised.Get("head"))
	assertIn(t, "Its own review run is next, on `muse`", lastOf(t, h.gh.BotComments(pr)))
	crossNextRun(h, "780")
	reviewed, result := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, []any{reviewed.Get("own_review"), result.Get("status")}, []any{true, "approved"})
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
}

// A conflict on a cleared change keeps its rule: its own run's reviewer reviews the resolution in
// the run, so the clearance can carry, and no own review run follows. A fullsend part chains
// nothing either (no review per part), and a person's pull request neither (its reviewer must be
// in its run).
func TestOwnReview_AConflictCarryAPartAndAPersonsPullRequestDoNotChain(t *testing.T) {
	cfg := testsupport.MakeConfig(testsupport.Config{Machine: testsupport.Machine})
	assertTrue(t, cfg.OwnReviewRun)
	muse := cfg.Pool.Get("muse")
	assignment := &plan.Assignment{Provider: muse, Action: "revise", Difficulty: "medium",
		Build: cfg.Pool.BestSeat(muse, "medium"), Review: cfg.Pool.BestSeat(muse, "medium")}
	revise := queue.NewCandidate(13, "revise", "", false, "")
	bot := map[string]bool{config.LabelPR: true}
	assertTrue(t, plan.Chains(cfg, revise, assignment, bot, pyjson.New("source", "request"), false))
	carry := pyjson.New("source", "conflict", "cleared", pyjson.New("sha", "abc"))
	assertFalse(t, plan.Chains(cfg, revise, assignment, bot, carry, false))
	assertFalse(t, plan.Chains(cfg, revise, assignment, map[string]bool{}, pyjson.New(), false))
	part := queue.NewCandidate(41, "build", "", false, "")
	part.Part = true
	build := *assignment
	build.Action = "build"
	assertFalse(t, plan.Chains(cfg, part, &build, map[string]bool{}, pyjson.New(), false))
	split := queue.NewCandidate(40, "build", "", false, "")
	split.Mode = "fullsend"
	assertFalse(t, plan.Chains(cfg, split, &build, map[string]bool{}, pyjson.New(), false))
	assertTrue(t, plan.Chains(cfg, split, &build, map[string]bool{}, pyjson.New(), true)) // its reconcile
	off := testsupport.MakeConfig(testsupport.Config{Machine: testsupport.Machine,
		Overrides: pyjson.New("own_review_run", false)})
	assertFalse(t, plan.Chains(off, revise, assignment, bot, pyjson.New("source", "request"), false))
}

// A fullsend tree's parts land with no review of their own, and chain none: only the reconcile
// hands its lane to its own review run.
func TestOwnReview_FullsendPartsDoNotChain(t *testing.T) {
	world := newFullsendWorld(t)
	world.split()
	part := world.claim()
	assertTrue(t, part.Truthy("onto"))
	assertFalse(t, part.Has("own_review_next"))
	result := world.deliver(part, world.work(part, fullsendBuilds(map[string]string{"src/game.txt": "rules A\n"})))
	assertEqual(t, result.Get("status"), "part")
	assertEqual(t, len(queue.OwnReviewsWaiting(world.ctx.Store.Load())), 0)
}

// After the merge queue takes a change out, its CI fix goes back to the lane whose own review run
// approved it (the merge owner, the same subscription that built it), ahead of Muse, which is free,
// and that run resumes the Claude session its own review run kept: the own review's last fix
// pass's, which F keeps for the item's next run on the same subscription. The CI fix chains its
// own review run in turn.
func TestOwnReview_AMergeQueueFailureComesBackToTheSameLaneAndResumesItsOwnReviewsSession(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Machine: []string{"muse"}, Chain: true})
	w := &sessionsWorld{t: t, h: h}
	h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: []string{config.LabelBuild}})
	flowUpdateItem(h.ctx, 12, pyjson.New("owner", "claude-1", "owner_why", "it planned it"))
	const fixSession = "1a2b3c4d-5e6f-4a1b-8c2d-3e4f5a6b7c8d"
	night := func(r *runner.FakeRunner, s *work.Sessions, runID string) (*pyjson.Object, *pyjson.Object) {
		t.Helper()
		h.cfg.RunID = runID
		planned := plan.Make(h.ctx, false, nil, "auto", nil)
		out := filepath.Join(h.root, fmt.Sprintf("out-%d", flowOuts(h.root)))
		worker := work.NewWorker(h.cfg, planned, r, h.clone, filepath.Join(h.root, "work"), out,
			nil, nil, nil, nil, nil, nil)
		worker.Sessions, worker.GH = s, h.gh
		worker.Run()
		w.upload(runID, sessionsZip(t, out, nil))
		deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
		return planned, loadJSON(t, filepath.Join(out, "result.json"))
	}
	s := w.keeper("claude-1", sessionsSecret)
	planned, built := night(workFake(map[string]any{"build": sessionsBuilder(s, sessionsID, sessionsFirstLine,
		map[string]string{"src/game.txt": "rules v2\n"})}), s, "9001")
	assertEqual(t, []any{planned.Get("provider"), built.Get("review_next")}, []any{"claude-1", "own"})
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	// The build's own session goes on on the pull request, where its next runs are.
	assertEqual(t, w.record(pr).Get("id"), sessionsID)
	assertEqual(t, w.record(12).Len(), 0)
	s = w.keeper("claude-1", sessionsSecret)
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"review": {workReviewer(workChanges("It skips the replay check.")), workReviewer(workApprove)},
		"fix": {sessionsBuilder(s, fixSession, `{"type":"user","message":"the own review's fix"}`,
			map[string]string{"src/game.txt": "rules v2, with the replay check\n"})}})
	reviewed, approved := night(r, s, "9002")
	assertEqual(t, []any{reviewed.Get("own_review"), approved.Get("status")}, []any{true, "approved"})
	// The own review run's session, its fix pass's, is kept on the pull request for its next run.
	assertEqual(t, []any{w.record(pr).Get("id"), w.record(pr).Get("provider"), w.record(pr).Get("action")},
		[]any{fixSession, "claude-1", "review"})
	head := crossStr(t, approved, "head")
	h.gh.Threads.Item(pr).Obj("head").Set("sha", head) // as GitHub has it
	// The merge queue takes it out: CI failed on `main` with it on top.
	notes := events.OnQueueCI(h.ctx, pyjson.New("id", 950, "head_branch",
		fmt.Sprintf("gh-readonly-queue/main/pr-%d-%s", pr, head), "conclusion", "failure",
		"html_url", "https://github.com/jgoetzmann/JackiOh/actions/runs/950"))
	assertEqual(t, notes, []string{fmt.Sprintf("#%d: queued a CI fix", pr)})
	s = w.keeper("claude-1", sessionsSecret)
	resumed := ""
	fixCI := func(request *runner.RunRequest) *runner.RunResult {
		resumed = request.Resume
		result := workBuilder(map[string]string{"src/game.txt": "rules v2, green on main\n"})(request)
		result.SessionID = request.Resume
		return result
	}
	r = workFake(map[string]any{"revise": fixCI})
	fixed, result := night(r, s, "9003")
	assertEqual(t, []any{fixed.Get("action"), fixed.Get("provider"), fixed.Get("source"), fixed.Get("own_review_next")},
		[]any{"revise", "claude-1", "ci", true})
	assertEqual(t, crossObj(t, fixed, "session").Get("id"), fixSession)
	assertEqual(t, resumed, fixSession)
	assertIn(t, "## Resuming your own session", ownReviewPrompt(t, r, "revise"))
	assertEqual(t, []any{result.Get("status"), result.Get("review_next")}, []any{"built", "own"})
	assertIn(t, config.LabelOwnReview, h.gh.LabelNames(pr))
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), queue.OwnReviewKey, "provider"), "claude-1")
}

// With the status loop planning the runs (`controller`), the tick that delivers a build is the
// tick that starts its own review run: the loop delivers run 1 (which starts nothing itself) and
// plans at once, and the lane its build reserved goes to that run, on the same subscription.
func TestOwnReview_TheLoopStartsItsOwnReviewRunInTheTickThatDeliversItsBuild(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Overrides: pyjson.New("controller", true), Chain: true})
	cfg := *h.cfg
	cfg.ControllerLoop, cfg.RunID = true, "777"
	ctx := testsupport.MakeCtx(h.gh, testsupport.Ctx{At: testsupport.Night, Cfg: &cfg})
	h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: []string{config.LabelBuild}})
	controllerListing(h.gh)
	started := controllerStarted(controller.Start(ctx, noSleep))
	assertEqual(t, len(started), 1)
	runID := strings.TrimRight(started[0][strings.LastIndex(started[0], "run ")+len("run "):], ")")
	planned := controllerFetch(t, ctx, h.gh.Dispatches[len(h.gh.Dispatches)-1].Inputs["plan_ref"])
	assertEqual(t, []any{planned.Get("action"), planned.Get("own_review_next")}, []any{"build", true})
	out := filepath.Join(h.root, "out")
	work.NewWorker(h.cfg, planned, workFake(map[string]runner.Handler{
		"build": workBuilder(map[string]string{"src/game.txt": "rules v2\n"})}), h.clone,
		filepath.Join(h.root, "work"), out, nil, nil, nil, nil, nil, nil).Run()
	h.gh.Artifacts[runID] = []*pyjson.Object{pyjson.New("id", 1, "name", "work", "expired", false)}
	h.gh.ArtifactZips[1] = controllerZipped(t, out)
	controllerEnded(h.gh, runID, testsupport.Night)
	// One tick: deliver, then plan.
	assertEqual(t, controller.Deliver(ctx, filepath.Join(h.root, "scratch")),
		[]string{fmt.Sprintf("delivered build #12 on claude-1 (run %s): built", runID)})
	assertEqual(t, len(h.gh.Dispatches), 1) // the delivery started nothing: the loop plans
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	notes := controller.Start(ctx, noSleep)
	again := controllerStarted(notes)
	assertEqual(t, len(again), 1, notes)
	assertIn(t, fmt.Sprintf("started review #%d on claude-1", pr), again[0])
	review := controllerFetch(t, ctx, h.gh.Dispatches[len(h.gh.Dispatches)-1].Inputs["plan_ref"])
	// Sonnet built the medium item, so Sonnet fixes what the review finds (#2903), and Opus, the account's
	// strongest seat, reviews it, in the same lane and a fresh session of its own.
	assertEqual(t, []any{review.Get("own_review"), review.Get("provider"), dig(t, review, "seats", "review", "model"),
		dig(t, review, "seats", "build", "model")}, []any{true, "claude-1", "opus", "sonnet"})
}

// A person's `/harness handoff` releases a waiting own review from its lane: it goes to its final
// audit, on another subscription first, and its pull request is marked ready for that.
func TestOwnReview_AHandoffSendsAWaitingOwnReviewToItsFinalAudit(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, _ := ownReviewBuild(t, h, "")
	said := plan.Handoff(h.ctx, pr, "jgoetzmann", "")
	assertIn(t, "Released from `muse`", said)
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelCross, "difficulty:medium"))
	assertEqual(t, h.gh.GetPull(pr).Get("draft"), false)
	assertIn(t, "**Its own review is handed off**: released by @jgoetzmann", strings.Join(h.gh.BotComments(pr), "\n"))
	audit, review := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, []any{audit.Get("provider"), audit.Truthy("own_review"), review.Get("verdict")},
		[]any{"agy", false, "approve"})
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
}

// A revision asked for before its own review run starts goes first: the reservation gives way to
// it, and the revision chains an own review run of its own.
func TestOwnReview_ARevisionAskedForBeforeItStartsGoesFirst(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, _ := ownReviewBuild(t, h, "")
	said := queue.QueueRevise(h.ctx, pr, "jgoetzmann", false, "request", false, nil, nil)
	assertIn(t, "Queued a revision", said)
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelRevise, "difficulty:medium"))
	assertNotIn(t, queue.OwnReviewKey, crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr)))
	planned, result := h.night(workFake(map[string]runner.Handler{
		"revise": workBuilder(map[string]string{"src/game.txt": "rules v3\n"})}))
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider"), result.Get("review_next")},
		[]any{"revise", "muse", "own"})
	assertIn(t, config.LabelOwnReview, h.gh.LabelNames(pr))
}

// An own review run cut off at its usage limit after a fix keeps its fixes on the branch and goes
// back to its lane: the next run there goes on from them, and the rounds it spent count.
func TestOwnReview_APausedOwnReviewGoesOnFromItsFixesOnItsLane(t *testing.T) {
	h := ownReviewHarness(t, nil, pyjson.New("max_review_cycles", 3))
	pr, built := ownReviewBuild(t, h, "")
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"review": {workReviewer(workChanges("It skips the replay check.")),
			flowLimited(clock.ISO(testsupport.Day.Add(10 * time.Minute)))},
		"fix": {workBuilder(map[string]string{"src/game.txt": "rules v2, with the replay check\n"})}})
	_, paused := h.night(r)
	assertEqual(t, paused.Get("status"), "interrupted")
	fixed := h.originSHA("bot/issue-12")
	assertNotEqual(t, fixed, built.Get("head"))
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	reserved := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), queue.OwnReviewKey)
	assertEqual(t, []any{reserved.Get("provider"), reserved.Has("claimed"), reserved.Get("rounds_used")},
		[]any{"muse", false, 1})
	assertIn(t, "Its own review run was paused", lastOf(t, h.gh.BotComments(pr)))
	testsupport.ClockOf(h.ctx).At = testsupport.Day.Add(20 * time.Minute)
	crossNextRun(h, "779")
	r = workFake(map[string]runner.Handler{"review": workReviewer(workApprove)})
	again, result := h.night(r)
	assertEqual(t, []any{again.Get("own_review"), again.Get("provider"), again.Get("rounds_used"), result.Get("status")},
		[]any{true, "muse", 1, "approved"})
	assertEqual(t, result.Get("head"), fixed)
	assertIn(t, "approved it on round 2 of 3", lastOf(t, h.gh.BotComments(pr)))
}

// A build that hands its lane on still runs the repository's checks and fixes the ones it turned
// red (up to `work.CheckFixRounds` passes), as a build always did, but reviews nothing; one still
// red after them is handed on as it stands, and its own review run's first round judges those
// checks rather than run them again, red included, so it cannot approve before a fix.
func TestOwnReview_TheBuildFixesTheChecksItTurnedRedAndReviewsNothing(t *testing.T) {
	h := ownReviewHarness(t, nil)
	h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: []string{config.LabelBuild}})
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"build": {workBuilder(map[string]any{"src/game.txt": "rules v2\n", "broken.txt": "oops\n"})},
		"fix":   {workBuilder(map[string]any{"broken.txt": nil})}})
	_, result := h.night(r)
	assertEqual(t, workRoles(r), []string{"plan", "build", "fix"})
	assertEqual(t, []any{result.Get("status"), result.Get("review_next")}, []any{"built", "own"})
	assertIn(t, "It ran the repository's checks, and they pass.", lastOf(t, h.gh.BotComments(12)))

	h = ownReviewHarness(t, nil)
	h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: []string{config.LabelBuild}})
	still := workBuilder(map[string]any{"src/game.txt": "rules v2\n", "broken.txt": "oops\n"})
	r = runner.NewFakeRunner(map[string][]runner.Handler{"build": {still}, "fix": {still, still}})
	_, result = h.night(r)
	assertEqual(t, workRoles(r), []string{"plan", "build", "fix", "fix"})
	assertEqual(t, work.CheckFixRounds, 2)
	assertIn(t, "with 1 finding(s) of its checks still open", crossStr(t, result, "reason"))
	assertIn(t, "`no broken file` still red", lastOf(t, h.gh.BotComments(12)))
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	crossNextRun(h, "778")
	r = runner.NewFakeRunner(map[string][]runner.Handler{
		"review": {workReviewer(workApprove), workReviewer(workApprove)},
		"fix":    {workBuilder(map[string]any{"broken.txt": nil})}})
	_, reviewed := h.night(r)
	assertEqual(t, workRoles(r), []string{"review", "fix", "review"})
	first := pyjson.Objs(reviewed.Get("cycles"))[0]
	assertEqual(t, first.Get("gates_from"), "its build")
	assertEqual(t, reviewed.Get("status"), "approved")
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
}

// Someone pushes to the branch while its own review run reads it: that approval is of a head the
// branch no longer has, so it counts for nothing, and the head nobody reviewed goes to its final
// audit, with auto-merge off.
func TestOwnReview_AReviewOfAHeadThatMovedDoesNotCount(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, _ := ownReviewBuild(t, h, "")
	moved := func(request *runner.RunRequest) *runner.RunResult {
		fakes.PushBranch(h.origin, h.root, "bot/issue-12", map[string]string{"src/game.txt": "someone else\n"}, "bot/issue-12")
		return workReviewer(workApprove)(request)
	}
	_, result := h.night(workFake(map[string]runner.Handler{"review": moved}))
	assertEqual(t, result.Get("status"), "approved")
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelCross, "difficulty:medium"))
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	assertIn(t, "does not count", lastOf(t, h.gh.BotComments(pr)))
	assertNotIn(t, queue.OwnReviewKey, crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr)))
}
