// The auditor fixes its own findings (#2748, section 4): a final audit that asks for changes makes
// the auditor's subscription the owner of the fix, which revises it itself, ahead of its new work;
// the fix is a new head, which its own run's reviewer's approval at the item's tier ships
// (2026-11-02). A capped auditor's fix waits for it a while and is then handed off, as any owner's
// work.
package tests

import (
	"fmt"
	"strconv"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// auditFixWithout is the harness's next night with only `machine` of the machine's subscriptions
// switched on, as run `runID`.
func auditFixWithout(h *flowHarness, machine []string, runID string) {
	cfg := testsupport.MakeConfig(testsupport.Config{Env: h.setup.env, Machine: machine,
		Overrides: h.setup.overrides})
	cfg.RunID = runID
	h.cfg, h.ctx.Cfg = cfg, cfg
}

// Builder X (Muse) builds and its own reviewer never approves; Y (agy) gives the final audit and
// asks for changes, so agy owns the fix and makes it itself, though Muse comes first in the medium
// tier's order; agy's own run's approval of its fix ships it (2026-11-02, #2748), with agy as the
// merge owner and no second review run.
func TestAuditFix_TheAuditorFixesItsFindingsAndItsOwnApprovalShipsTheFix(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, built := crossBuildForAudit(t, h, "")
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", "12", "owner"), "muse")
	// The final audit: agy, a medium model on another subscription.
	crossNextRun(h, "778")
	planned, review := h.night(workFake(map[string]runner.Handler{
		"review": workReviewer(workChanges("It skips the replay check."))}))
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider"), review.Get("verdict")},
		[]any{"review", "agy", "changes"})
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelRevise, "difficulty:medium"))
	issue := crossObj(t, h.ctx.Store.Load(), "items", "12")
	assertEqual(t, issue.Get("owner"), "agy")
	assertIn(t, fmt.Sprintf("its final audit of `%s` asked for changes, and the auditor fixes its own findings",
		crossStr(t, built, "head")[:12]), issue.Str("owner_why"))
	said := lastOf(t, h.gh.BotComments(pr))
	assertIn(t, "found blocking problems, so it fixes them itself: a revision is queued for `agy`, which owns the fix", said)
	assertIn(t, "That fix is a new commit, which ships once a medium or strong model approves it, its own run's "+
		"reviewer included.", said)
	// agy revises it, its own work first: Muse is free and comes first in the tier's order.
	crossNextRun(h, "779")
	r := workFake(map[string]runner.Handler{
		"revise": workBuilder(map[string]string{"src/game.txt": "rules v2 + replay\n"}),
		"review": workReviewer(workApprove)})
	planned, fixed := h.night(r)
	assertEqual(t, []any{planned.Get("action"), planned.Get("number"), planned.Get("provider"), fixed.Get("status")},
		[]any{"revise", pr, "agy", "approved"})
	assertIn(t, "its owner `agy` keeps it: one lane, one owner", strings.Join(stringsOf(planned.List("routing")), "\n"))
	assertIn(t, "It skips the replay check.", r.Calls[0].Prompt)
	// The fixer's own run's approval ships its fix: agy's run built the new head and approved it.
	votes := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "votes")
	assertEqual(t, []any{votes.Get("sha"), votes.Get("built_by"), votes.Get("approvals")},
		[]any{fixed.Get("head"), "agy", []any{"gemini"}})
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelMerging, "difficulty:medium"))
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	assertEqual(t, []any{record.Get("audit_tier"), record.Get("merge_owner"), dig(t, record, "cleared", "sha"),
		dig(t, record, "cleared", "built_by"), dig(t, record, "cleared", "approver", "provider")},
		[]any{"", "agy", fixed.Get("head"), "agy", "agy"})
}

// A hard item: claude-3 builds it and its own Opus review ships it; a person then asks for a final
// audit (`/harness review strong`), which claude-7 (Opus on another account, ahead of the
// builder's own) gives and asks for changes, so claude-7 owns the fix. Capped on its 5-hour
// session with its reset 40 minutes off, claude-7 is waited for (`bot:waiting`); with its reset
// three hours off, the hold is released and the fix goes at once to another strong seat, which
// owns it from then on.
func TestAuditFix_ACappedAuditorsFixWaitsForItThenIsHandedOff(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:hard"}})
	planned, _ := h.night(workFake(map[string]runner.Handler{
		"build": workBuilder(map[string]string{"src/game.txt": "v2\n"}), "review": workReviewer(workApprove)}))
	assertEqual(t, planned.Get("provider"), "claude-3")
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	queue.QueueReview(h.ctx, pr, "jgoetzmann", "strong", "", nil)
	crossNextRun(h, "778")
	planned, review := h.night(workFake(map[string]runner.Handler{
		"review": workReviewer(workChanges("The replay hash ignores the new field."))}))
	assertEqual(t, []any{planned.Get("provider"), review.Get("verdict")}, []any{"claude-7", "changes"})
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", "12", "owner"), "claude-7")
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "audit_tier"), "strong")
	// claude-7 over its 5-hour cap, its reset 40 minutes off: its fix waits for it.
	ownershipReading(h.ctx, "claude-7", 0.97, 40, 0.2, 3000)
	planned = flowMake(h.ctx)
	assertEqual(t, planned.Get("action"), "none")
	assertIn(t, fmt.Sprintf("#%d held for `claude-7` until 22:40", pr), planned.Str("reason"))
	assertIn(t, config.LabelWaiting, h.gh.LabelNames(pr))
	assertIn(t, "**Held for `claude-7`**", lastOf(t, ownershipHoldComments(h.gh, pr)))
	// Its reset three hours off: handed off at once, at the item's tier (strong for a hard item).
	ownershipReading(h.ctx, "claude-7", 0.97, 180, 0.2, 3000)
	planned = flowMake(h.ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("number"), dig(t, planned, "seats", "build", "tier")},
		[]any{"revise", pr, "strong"})
	taker := planned.Str("provider")
	assertTrue(t, taker != "claude-7" && strings.HasPrefix(taker, "claude-"), taker)
	assertNotIn(t, config.LabelWaiting, h.gh.LabelNames(pr))
	issue := crossObj(t, h.ctx.Store.Load(), "items", "12")
	assertEqual(t, issue.Get("owner"), taker)
	assertIn(t, "released from its hold: `claude-7` 5-hour usage is 97%", issue.Str("owner_why"))
	assertIn(t, "it can start again only in 3h 00m", issue.Str("owner_why"))
	assertIn(t, "**Released to anyone**", lastOf(t, ownershipHoldComments(h.gh, pr)))
}
