// The after-merge spot-check (#2748, section 5): every 5th medium or hard bot pull request that
// merges (`spot_check_every`, a counter in the state file; no fullsend reconcile or follow-up) gets
// a review run by a strong model on a subscription other than its builder's (and its approver's
// when another is free), of the change as it merged: the independent measure of an item's own
// lane's approval, which ships it (2026-11-02). It blocks nothing: an approval records `clean`, and
// blocking findings open one follow-up issue, whose own pull request is never sampled. Every check
// is recorded.
package tests

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cli"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/spotcheck"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// spotCheckClose closes bot pull request #200+n of issue #100+n, labelled `labels` too, merged
// into `main` by commit `m<n>` when `merged`, and hands GitHub's event to the harness.
func spotCheckClose(gh *fakes.FakeGitHub, ctx *hctx.Context, n int, merged bool, labels ...string) []string {
	issue, pr := 100+n, 200+n
	if gh.Threads.Get(issue) == nil {
		gh.AddIssue(issue, fakes.Issue{Labels: []string{config.LabelPROpen}})
	}
	pull := gh.AddPull(pr, fmt.Sprintf("bot/issue-%d", issue), fakes.Pull{Title: fmt.Sprintf("Patch: change %d", n),
		Body: fmt.Sprintf("Closes #%d", issue), Labels: append([]string{config.LabelPR}, labels...),
		SHA: fmt.Sprintf("h%d", n)})
	pull.Set("state", "closed")
	if merged {
		pull.Update(pyjson.New("merged", true, "merge_commit_sha", fmt.Sprintf("m%d", n)))
	}
	return events.Handle(ctx, "pull_request_target", pyjson.New("action", "closed", "sender", fakes.Bot,
		"pull_request", pull))
}

// spotCheckCount is the state file's spot-check count, and the pull requests of its checks.
func spotCheckCount(t *testing.T, ctx *hctx.Context) (int, []int) {
	t.Helper()
	s := ctx.Store.Load()
	prs := []int{}
	for _, c := range spotcheck.Checks(s) {
		prs = append(prs, py64(c.Get("pr")))
	}
	return py64(s.Obj(spotcheck.Key).Get("count")), prs
}

func TestSpotCheck_TheNightBotSamplesOneInFiveAndSquishyNone(t *testing.T) {
	assertEqual(t, testsupport.MakeConfig().SpotCheckEvery, 5)
	assertTrue(t, testsupport.MakeConfig().SpotChecks())
	squishy := loadJSON(t, testsupport.Root+"/.squishy/config.json")
	assertEqual(t, squishy.Get("spot_check_every"), 0)
	off := testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("spot_check_every", 0)})
	assertFalse(t, off.SpotChecks())
}

// The counter: every 5th medium or hard bot pull request merged into `main` is queued, once (both
// ship on their own lane's approval since 2026-11-02); a fullsend tree's reconcile, a follow-up
// issue's own (by its record or its body's marker), one closed unmerged and one merged elsewhere are
// not counted.
func TestSpotCheck_EveryFifthMediumOrHardMergeIsQueuedButNoReconcileOrFollowUp(t *testing.T) {
	gh, ctx := newFlowPlanWorld()
	for n := 1; n <= 3; n++ {
		spotCheckClose(gh, ctx, n, true, "difficulty:medium")
	}
	spotCheckClose(gh, ctx, 4, true, "difficulty:hard")
	count, prs := spotCheckCount(t, ctx)
	assertEqual(t, []any{count, prs}, []any{4, []int{}})
	gh.AddIssue(105, fakes.Issue{Labels: []string{config.LabelPROpen, fullsendLabel}})
	spotCheckClose(gh, ctx, 5, true) // the tree's reconcile
	flowUpdateItem(ctx, 106, pyjson.New("spot_check_of", 41))
	gh.AddIssue(106, fakes.Issue{Labels: []string{config.LabelPROpen}})
	spotCheckClose(gh, ctx, 6, true)
	gh.AddIssue(107, fakes.Issue{Body: spotcheck.Marker(42) + "\nThe spot-check found …", Labels: []string{config.LabelPROpen}})
	spotCheckClose(gh, ctx, 7, true)
	spotCheckClose(gh, ctx, 8, false)
	gh.AddIssue(110, fakes.Issue{Labels: []string{config.LabelPROpen}})
	gh.AddPull(210, "bot/issue-110", fakes.Pull{Body: "Closes #110", Labels: []string{config.LabelPR}})
	gh.Threads.Item(210).Update(pyjson.New("state", "closed", "merged", true, "base", pyjson.New("ref", "release")))
	events.Handle(ctx, "pull_request_target", pyjson.New("action", "closed", "sender", fakes.Bot,
		"pull_request", gh.Threads.Item(210)))
	count, prs = spotCheckCount(t, ctx)
	assertEqual(t, []any{count, prs}, []any{4, []int{}})
	// The fifth merge, medium (no difficulty label is medium), whose builder and approver are on
	// record.
	flowUpdateItem(ctx, 209, pyjson.New(
		"votes", pyjson.New("sha", "h9", "builder", "muse", "built_by", "muse"),
		"cleared", pyjson.New("sha", "h9", "approver", pyjson.New("provider", "agy", "family", "gemini",
			"model", "gemini-3.8-flash-high", "tier", "medium")),
		"merge_owner", "agy"))
	said := spotCheckClose(gh, ctx, 9, true)
	assertIn(t, "#209 is medium or hard merge 5, so it is queued for an after-merge spot-check (one in 5)", said)
	spotCheckClose(gh, ctx, 9, true) // GitHub's event delivered twice
	count, prs = spotCheckCount(t, ctx)
	assertEqual(t, []any{count, prs}, []any{5, []int{209}})
	entry := spotcheck.Checks(ctx.Store.Load())[0]
	assertEqual(t, entry, pyjson.New("pr", 209, "issue", 109, "title", "Patch: change 9", "n", 5, "every", 5,
		"merged_at", "2026-09-30T03:00:00Z", "sha", "m9", "head", "h9", "difficulty", "medium",
		"builder", pyjson.New("provider", "muse", "family", "muse"),
		"approver", pyjson.New("provider", "agy", "family", "gemini", "model", "gemini-3.8-flash-high", "tier", "medium"),
		"status", "queued", "queued_at", "2026-09-30T03:00:00Z", "tries", 0))
	// It is a review run in the queue, on a strong seat, of a pull request closed by its merge.
	var found *queue.Candidate
	for _, c := range queue.Candidates(ctx, ctx.Store.Load(), nil, nil) {
		if c.Number == 209 {
			found = c
		}
	}
	if found == nil {
		t.Fatal("the spot-check is not in the queue")
	}
	assertEqual(t, []any{found.Kind, found.SpotCheck, found.BuiltBy, found.ApprovedBy, found.ReviewTiers},
		[]any{"review", true, "muse", "agy", []string{"strong"}})
	// The tenth merge is the next, a hard one this time, its own lane's Opus approval on record.
	for n := 11; n <= 14; n++ {
		spotCheckClose(gh, ctx, n, true)
	}
	flowUpdateItem(ctx, 215, pyjson.New(
		"votes", pyjson.New("sha", "h15", "builder", "claude", "built_by", "claude-3"),
		"cleared", pyjson.New("sha", "h15", "built_by", "claude-3", "approver", pyjson.New("provider", "claude-3",
			"family", "claude", "model", "opus", "tier", "strong")),
		"merge_owner", "claude-3"))
	spotCheckClose(gh, ctx, 15, true, "difficulty:hard")
	count, prs = spotCheckCount(t, ctx)
	assertEqual(t, []any{count, prs}, []any{10, []int{209, 215}})
	hard := spotcheck.Checks(ctx.Store.Load())[1]
	assertEqual(t, []any{hard.Get("difficulty"), hard.Get("builder"), dig(t, hard, "approver", "provider")},
		[]any{"hard", pyjson.New("provider", "claude-3", "family", "claude"), "claude-3"})
	// Off, nothing is counted.
	gh2, ctx2 := newFlowPlanWorld()
	ctx2.Cfg = testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("spot_check_every", 0)})
	for n := 1; n <= 5; n++ {
		spotCheckClose(gh2, ctx2, n, true)
	}
	count, prs = spotCheckCount(t, ctx2)
	assertEqual(t, []any{count, prs}, []any{0, []int{}})
}

// A follow-up issue is queued at the merged change's difficulty: a hard merge's findings are a hard
// item's, a medium one's (or one with none on record) a medium item's.
func TestSpotCheck_AFollowUpTakesTheMergedChangesDifficulty(t *testing.T) {
	assertEqual(t, spotcheck.FollowUpDifficulty(pyjson.New("difficulty", "hard")), "hard")
	assertEqual(t, spotcheck.FollowUpDifficulty(pyjson.New("difficulty", "medium")), "medium")
	assertEqual(t, spotcheck.FollowUpDifficulty(pyjson.New()), "medium")
	assertEqual(t, spotcheck.Difficulties, []string{"medium", "hard"})
}

// spotCheckMerged is the harness's medium item built by Muse and approved by agy's final audit
// (Muse's own reviewer did not approve it), merged into `main` as the fifth merge, so it is queued
// for a spot-check, which Opus takes up at night. Returns the pull request and the commit that
// merged it.
func spotCheckMerged(t *testing.T, h *flowHarness) (int, string) {
	t.Helper()
	pr, _ := crossBuildForAudit(t, h, "")
	crossNextRun(h, "778")
	planned, review := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("provider"), review.Get("verdict")}, []any{"agy", "approve"})
	head := crossStr(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "cleared", "sha")
	// GitHub squash-merges it into `main`.
	sha := fakes.PushBranch(h.origin, h.root, "main", map[string]string{"src/game.txt": "rules v2\n"})
	pull := h.gh.Threads.Item(pr)
	pull.Obj("head").Set("sha", head)
	pull.Update(pyjson.New("state", "closed", "merged", true, "merge_commit_sha", sha))
	flowUpdate(h.ctx, func(s *pyjson.Object) { spotcheck.Record(s).Set("count", 4) })
	said := events.Handle(h.ctx, "pull_request_target", pyjson.New("action", "closed", "sender", fakes.Bot,
		"pull_request", pull))
	assertIn(t, fmt.Sprintf("#%d is medium or hard merge 5, so it is queued for an after-merge spot-check (one in 5)", pr), said)
	testsupport.ClockOf(h.ctx).At = testsupport.Night // the Claude accounts' hours
	crossNextRun(h, "779")
	return pr, sha
}

// spotCheckEntry is the one spot-check on record.
func spotCheckEntry(t *testing.T, h *flowHarness) *pyjson.Object {
	t.Helper()
	checks := spotcheck.Checks(h.ctx.Store.Load())
	assertEqual(t, len(checks), 1)
	return checks[0]
}

// The check is a review run on Opus, on neither the builder's subscription nor the approver's, of
// the commit that merged the pull request against its parent, with its own prompt. An approval is
// recorded `clean`, with who built, approved and reviewed it, and opens nothing.
func TestSpotCheck_AMergedChangeIsReviewedByOpusAndRecordedClean(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, sha := spotCheckMerged(t, h)
	comments := len(h.gh.BotComments(pr))
	r := workFake(map[string]runner.Handler{"review": workReviewer(workApprove)})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("action"), planned.Get("number"), planned.Get("spot_check"),
		planned.Get("merge_sha"), dig(t, planned, "seats", "review", "tier")},
		[]any{"review", pr, true, sha, "strong"})
	reviewer := planned.Str("provider")
	assertTrue(t, strings.HasPrefix(reviewer, "claude-"), reviewer)
	assertEqual(t, []any{result.Get("status"), result.Get("verdict"), result.Get("reviewed_sha"), result.Get("start")},
		[]any{"reviewed", "approve", sha, sha})
	prompt := r.Calls[0].Prompt
	assertIn(t, fmt.Sprintf("# After-merge spot-check of #%d", pr), prompt)
	assertIn(t, "It blocks nothing and reverts nothing.", prompt)
	assertIn(t, "This is an after-merge spot-check. `muse` built this change", prompt)
	assertIn(t, "-rules v1\n+rules v2", prompt) // the change exactly as it merged
	entry := spotCheckEntry(t, h)
	assertEqual(t, []any{entry.Get("status"), entry.Get("verdict"), entry.Get("findings"), entry.Has("follow_up")},
		[]any{"done", "clean", 0, false})
	assertEqual(t, entry.Get("builder"), pyjson.New("provider", "muse", "family", "muse"))
	assertEqual(t, entry.Get("approver"), pyjson.New("provider", "agy", "family", "gemini",
		"model", "gemini-3.8-flash-high", "tier", "medium"))
	assertEqual(t, entry.Get("reviewer"), pyjson.New("provider", reviewer, "family", "claude", "model", "opus",
		"tier", "strong"))
	// It blocked nothing and said nothing; the run's live label is off, and nothing is queued.
	assertEqual(t, len(h.gh.BotComments(pr)), comments)
	assertFalse(t, config.IsLive(h.gh.LabelNames(pr)))
	assertEqual(t, len(spotcheck.Pending(h.ctx.Store.Load())), 0)
	assertEqual(t, flowMake(h.ctx).Get("action"), "none")
}

// The same check, with the worker's probe the real one `work` gives it (`cli.MakeProbe`), which
// `h.night` leaves out: the probe stopped every spot-check at its first step because its pull
// request was closed, by its merge, so all 12 on record failed (#3133).
func TestSpotCheck_TheRealProbeLetsTheReviewOfAClosedMergedPullRequestRun(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, sha := spotCheckMerged(t, h)
	planned := flowMake(h.ctx)
	assertEqual(t, []any{planned.Get("number"), planned.Get("spot_check")}, []any{pr, true})
	r := workFake(map[string]runner.Handler{"review": workReviewer(workApprove)})
	out := filepath.Join(h.root, "out-probed")
	probe := cli.MakeProbe(h.ctx, &pr, h.cfg.Pool.Get(planned.Str("provider")), nil)
	worker := work.NewWorker(h.cfg, planned, r, h.clone, filepath.Join(h.root, "work"), out, probe, nil,
		h.ctx.Now, nil, nil, nil)
	worker.GH = h.gh
	worker.Run()
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	text, err := os.ReadFile(filepath.Join(out, "result.json"))
	assertTrue(t, err == nil, err)
	result := flowLoads(string(text))
	assertEqual(t, []any{result.Get("status"), result.Get("verdict"), result.Get("reviewed_sha")},
		[]any{"reviewed", "approve", sha}, result.Get("reason"))
	assertEqual(t, len(r.Calls), 1)
	entry := spotCheckEntry(t, h)
	assertEqual(t, []any{entry.Get("status"), entry.Get("verdict"), entry.Get("tries")}, []any{"done", "clean", 0})
}

// Blocking findings open one follow-up issue, queued to build at `difficulty:medium`, with a
// comment on the pull request that links it, and are recorded `findings`. The follow-up's own pull
// request is never sampled, even as the count's tenth.
func TestSpotCheck_FindingsOpenOneFollowUpIssueThatIsNeverSampledItself(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, sha := spotCheckMerged(t, h)
	planned, result := h.night(workFake(map[string]runner.Handler{
		"review": workReviewer(workChanges("The new rule never reaches the replay hash."))}))
	assertEqual(t, []any{planned.Get("spot_check"), result.Get("verdict")}, []any{true, "changes"})
	entry := spotCheckEntry(t, h)
	follow := py64(entry.Get("follow_up"))
	assertEqual(t, []any{entry.Get("status"), entry.Get("verdict"), entry.Get("findings")}, []any{"done", "findings", 1})
	assertTrue(t, follow != 0, "no follow-up issue")
	issue := h.gh.GetIssue(follow)
	assertEqual(t, issue.Get("title"), fmt.Sprintf("Patch: follow-up to #%d from the after-merge spot-check", pr))
	assertEqual(t, h.gh.LabelNames(follow), setOf(config.LabelBuild, "difficulty:medium"))
	body := issue.Str("body")
	for _, part := range []string{spotcheck.Marker(pr), fmt.Sprintf("found 1 blocking problem(s) in the change as it merged (`%s`)", sha[:12]),
		"- [ ] `src/game.txt`: The new rule never reaches the replay hash.", "Evidence: read it",
		fmt.Sprintf("#%d stays merged: nothing is reverted", pr), "no test skipped, ignored or weakened",
		fmt.Sprintf("`muse` (`muse`) built #%d and `agy` (`gemini-3.8-flash-high`) approved it", pr)} {
		assertIn(t, part, body)
	}
	said := lastOf(t, h.gh.BotComments(pr))
	assertIn(t, fmt.Sprintf("found 1 blocking problem(s) in this change as it merged. It blocks nothing and "+
		"reverts nothing: #%d follows them up, queued to build at `difficulty:medium`.", follow), said)
	assertIn(t, "The new rule never reaches the replay hash.", said)
	// The follow-up is queued as the bot's own medium rating, so a planner may raise it.
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(follow))
	assertEqual(t, []any{record.Get("spot_check_of"), dig(t, record, "difficulty_by", "difficulty")}, []any{pr, "medium"})
	assertEqual(t, queue.RatingSource(h.gh.LabelNames(follow), record), "bot")
	var queued *queue.Candidate
	for _, c := range queue.Candidates(h.ctx, h.ctx.Store.Load(), nil, nil) {
		if c.Number == follow {
			queued = c
		}
	}
	if queued == nil {
		t.Fatal("the follow-up is not queued")
	}
	assertEqual(t, []any{queued.Kind, queued.Difficulty, queued.Rated}, []any{"build", "medium", true})
	// Its own pull request merges as the tenth medium merge: never sampled, never counted.
	flowUpdate(h.ctx, func(s *pyjson.Object) { spotcheck.Record(s).Set("count", 9) })
	h.gh.AddPull(90, fmt.Sprintf("bot/issue-%d", follow), fakes.Pull{Body: fmt.Sprintf("Closes #%d", follow),
		Labels: []string{config.LabelPR, "difficulty:medium"}})
	h.gh.Threads.Item(90).Update(pyjson.New("state", "closed", "merged", true, "merge_commit_sha", "f1"))
	said2 := events.Handle(h.ctx, "pull_request_target", pyjson.New("action", "closed", "sender", fakes.Bot,
		"pull_request", h.gh.Threads.Item(90)))
	assertNotIn(t, "spot-check", strings.Join(said2, " "))
	count, prs := spotCheckCount(t, h.ctx)
	assertEqual(t, []any{count, prs}, []any{9, []int{pr}})
}

// spotCheckWorld is the night, every Claude account set up and no machine: a bot pull request #20
// waiting for its final audit, a spot-check of #30 (merged; claude-3 built it, claude-7 approved
// it) and a build #13 of higher priority, planned.
func spotCheckWorld(t *testing.T) (*fakes.FakeGitHub, *hctx.Context) {
	t.Helper()
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{Cfg: testsupport.MakeConfig(testsupport.Config{Env: crossAll})})
	gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelPROpen}})
	gh.AddPull(20, "bot/issue-12", fakes.Pull{Body: "Closes #12", Labels: []string{config.LabelPR, config.LabelCross}})
	flowUpdateItem(ctx, 20, pyjson.New("issue", 12, "votes", pyjson.New("sha", "abc", "builder", "claude", "built_by", "claude-1")))
	gh.AddPull(30, "bot/issue-29", fakes.Pull{Body: "Closes #29", Labels: []string{config.LabelPR}})
	gh.Threads.Item(30).Update(pyjson.New("state", "closed", "merged", true, "merge_commit_sha", "m30"))
	flowUpdate(ctx, func(s *pyjson.Object) {
		spotcheck.Record(s).Set("count", 5).Set("checks", []any{pyjson.New("pr", 30, "issue", 29, "title", "A change",
			"n", 5, "every", 5, "sha", "m30", "head", "h30", "difficulty", "medium",
			"builder", pyjson.New("provider", "claude-3", "family", "claude"),
			"approver", pyjson.New("provider", "claude-7", "family", "claude", "model", "sonnet", "tier", "medium"),
			"status", "queued", "queued_at", "2026-09-29T00:00:00Z", "tries", 0)})
	})
	gh.AddIssue(13, fakes.Issue{Labels: []string{config.LabelBuild, config.LabelPriorityHigh},
		Body: "<!-- jackioh-bot:plan -->\n## Plan\n1. Edit it.\n<!-- /jackioh-bot:plan -->"})
	return gh, ctx
}

// A spot-check goes after every other review run, since it unblocks nothing, and before the builds,
// whatever their priority; its reviewer is strong, and neither its builder's subscription nor,
// while another is free, its approver's.
func TestSpotCheck_ItGoesAfterTheOtherReviewRunsOnAStrongSeatOfAnotherSubscription(t *testing.T) {
	_, ctx := spotCheckWorld(t)
	first := flowMake(ctx)
	assertEqual(t, []any{first.Get("action"), first.Get("number"), first.Get("spot_check")}, []any{"review", 20, nil})
	second := flowMake(ctx)
	assertEqual(t, []any{second.Get("action"), second.Get("number"), second.Get("spot_check"),
		dig(t, second, "seats", "review", "tier")}, []any{"review", 30, true, "strong"})
	reviewer := second.Str("provider")
	assertTrue(t, reviewer != "claude-3" && reviewer != "claude-7", reviewer)
	assertTrue(t, spotcheck.Running(ctx.Store.Load(), 30), "the check is not running")
	third := flowMake(ctx)
	assertEqual(t, []any{third.Get("action"), third.Get("number")}, []any{"build", 13})
	// The approver reviews it only when no other strong seat is free; the builder never does.
	pool := ctx.Cfg.Pool
	c := queue.NewCandidate(30, "review", "", false, "")
	c.SpotCheck, c.BuiltBy, c.ApprovedBy, c.ReviewTiers = true, "claude-3", "claude-7", []string{"strong"}
	offered := func(ids ...string) []*providers.Provider {
		var found []*providers.Provider
		for _, id := range ids {
			found = append(found, pool.Get(id))
		}
		return found
	}
	seat := plan.ReviewSeat(pool, offered("claude-7", "claude-3", "claude-8"), c, nil)
	assertEqual(t, []any{seat.Provider.ID, seat.Tier}, []any{"claude-8", "strong"})
	seat = plan.ReviewSeat(pool, offered("claude-7", "claude-3"), c, nil)
	assertEqual(t, []any{seat.Provider.ID, seat.Tier}, []any{"claude-7", "strong"})
	assertTrue(t, plan.ReviewSeat(pool, offered("claude-3", "muse"), c, nil) == nil, "a seat for the builder")
}

// A spot-check whose run dies goes back in the queue, a try spent; the third is recorded failed.
func TestSpotCheck_ARunThatDiesGoesAgainAndIsRecordedFailedAfterThree(t *testing.T) {
	gh, ctx := spotCheckWorld(t)
	gh.Threads.Item(20).Set("state", "closed") // only the spot-check is queued
	gh.Threads.Item(13).Set("state", "closed")
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("number"), planned.Get("spot_check")}, []any{30, true})
	assertTrue(t, config.IsLive(gh.LabelNames(30)), "no live label on the merged pull request")
	for try := 1; try <= 3; try++ {
		gh.Runs.Set(ctx.Cfg.RunID, pyjson.New("status", "completed"))
		cfg := *ctx.Cfg
		cfg.RunID = strconv.Itoa(800 + try)
		ctx.Cfg = &cfg
		planned = flowMake(ctx)
		assertIn(t, "requeued the spot-check of #30 from a dead run", planned.List("housekeeping"))
		entry := spotcheck.Checks(ctx.Store.Load())[0]
		assertEqual(t, entry.Get("tries"), try)
		if try < 3 {
			assertEqual(t, []any{planned.Get("number"), entry.Get("run_id")}, []any{30, cfg.RunID})
			continue
		}
		assertEqual(t, planned.Get("action"), "none")
		assertEqual(t, []any{entry.Get("status"), entry.Get("verdict")}, []any{"done", "failed"})
	}
	assertFalse(t, config.IsLive(gh.LabelNames(30)))
}
