// The last approver owns the merge (#2748, section 4): the subscription whose approval shipped a
// head is recorded as its merge owner, and a red check or the merge queue's failure on that head
// queues the CI fix with it as the owner, ahead of its new work. The fix is a new head, which
// another subscription approves. The CI fixes stay capped at three, then `bot:blocked`.
package tests

import (
	"fmt"
	"strconv"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// mergeOwnerQueueFailed is CI failing on the merge queue's build of pull request `pr`.
func mergeOwnerQueueFailed(h *flowHarness, pr int) []string {
	run := pyjson.New("name", "CI", "conclusion", "cancelled",
		"head_branch", fmt.Sprintf("gh-readonly-queue/main/pr-%d-abc", pr), "head_sha", "mg", "id", 77,
		"html_url", "q", "pull_requests", []any{}, "event", "merge_group")
	return events.Handle(h.ctx, "workflow_run", pyjson.New("action", "completed", "sender", fakes.Bot, "workflow_run", run))
}

// Muse builds, agy's final audit approves: agy owns the merge. The merge queue takes it out red, so
// the CI fix is agy's, and agy takes it before a new build of higher priority; agy's own approval of
// its fix does not ship it, and Muse's audit of the fix does, which makes Muse the merge owner.
func TestMergeOwner_TheLastApproverGetsTheCIFixFirstAndItsFixNeedsAnotherApprover(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, _ := crossBuildOnMedium(t, h, "", nil)
	crossNextRun(h, "778")
	planned, review := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("provider"), review.Get("verdict")}, []any{"agy", "approve"})
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	head := crossStr(t, record, "cleared", "sha")
	h.gh.Threads.Item(pr).Obj("head").Set("sha", head) // as GitHub has it
	assertEqual(t, record.Get("merge_owner"), "agy")
	assertEqual(t, dig(t, record, "cleared", "approver"), pyjson.New("provider", "agy", "family", "gemini",
		"model", "gemini-4-argon-high", "tier", "medium"))
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", "12", "owner"), "muse") // the builder's, until now
	// While it waits for CI and the merge queue, no lane is held: nothing is queued for it.
	assertIn(t, config.LabelMerging, h.gh.LabelNames(pr))
	assertEqual(t, flowMake(h.ctx).Get("action"), "none")
	// A new build of higher priority is queued meanwhile, with its plan.
	h.gh.AddIssue(40, fakes.Issue{Labels: []string{config.LabelBuild, config.LabelPriorityHigh},
		Body: "<!-- jackioh-bot:plan -->\n## Plan\n1. Edit src/other.txt.\n<!-- /jackioh-bot:plan -->"})
	// The merge queue takes the cleared head out, red: the CI fix is the merge owner's.
	assertEqual(t, mergeOwnerQueueFailed(h, pr), []string{fmt.Sprintf("#%d: queued a CI fix", pr)})
	assertIn(t, config.LabelRevise, h.gh.LabelNames(pr))
	assertNotIn(t, config.LabelMerging, h.gh.LabelNames(pr))
	issue := crossObj(t, h.ctx.Store.Load(), "items", "12")
	assertEqual(t, issue.Get("owner"), "agy")
	assertIn(t, fmt.Sprintf("its approval shipped `%s`, and the last approver owns the merge", head[:12]),
		issue.Str("owner_why"))
	assertIn(t, "`agy`, whose approval shipped this head, owns the merge, so it fixes it first",
		lastOf(t, h.gh.BotComments(pr)))
	// agy takes its CI fix first, before the high-priority build.
	crossNextRun(h, "779")
	planned, fixed := h.night(workFake(map[string]runner.Handler{
		"revise": workBuilder(map[string]string{"src/game.txt": "rules v2, green\n"}),
		"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("action"), planned.Get("number"), planned.Get("provider"),
		planned.Get("source"), fixed.Get("status")}, []any{"revise", pr, "agy", "ci", "approved"})
	record = crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	assertEqual(t, []any{record.Get("ci_fixes"), record.Get("ci_fixed_head"), dig(t, record, "votes", "built_by")},
		[]any{1, fixed.Get("head"), "agy"})
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	assertIn(t, config.LabelCross, h.gh.LabelNames(pr))
	// The fix's audit goes to another subscription (Muse first, which has not approved it), still
	// ahead of the new build, and its approval ships the fix.
	crossNextRun(h, "780")
	planned, second := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("action"), planned.Get("number"), planned.Get("provider"), second.Get("verdict")},
		[]any{"review", pr, "muse", "approve"})
	assertIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "merge_owner"), "muse")
}

// Each CI fix of a cleared head goes to the subscription whose approval shipped that head; a red
// head no approval cleared changes no owner; the fourth red head after three CI fixes is blocked
// for a person, whoever approved it.
func TestMergeOwner_TheCIFixesStayCappedAtThree(t *testing.T) {
	w := newEventsQueueWorld()
	flowUpdateItem(w.ctx, 9, pyjson.New("issue", 5, "merge_owner", "agy", "cleared", pyjson.New("sha", "s0")))
	flowUpdateItem(w.ctx, 5, pyjson.New("owner", "muse"))
	owner := func() string { return dig(t, w.ctx.Store.Load(), "items", "5", "owner").(string) }
	// CI red on a head the reviews never cleared (s1, cleared s0): its fix is the item's owner's.
	run := pyjson.New("name", "CI", "conclusion", "failure", "head_branch", "bot/issue-5", "head_sha", "s1",
		"id", 99, "html_url", "u", "pull_requests", []any{pyjson.New("number", 9)})
	flowUpdateItem(w.ctx, 9, pyjson.New("ci_reruns", pyjson.New("s1", 1)))
	events.Handle(w.ctx, "workflow_run", pyjson.New("action", "completed", "sender", fakes.Bot, "workflow_run", run))
	assertIn(t, config.LabelRevise, w.gh.LabelNames(9))
	assertEqual(t, owner(), "muse")
	assertEqual(t, dig(t, w.ctx.Store.Load(), "items", "9", "ci_fixes"), 1)
	// Two more red heads, each cleared by another approver: each fix is its approver's.
	for i, approver := range []string{"claude-3", "gpt"} {
		sha := fmt.Sprintf("s%d", i+2)
		w.gh.RemoveLabel(9, config.LabelRevise) // the last fix landed
		w.gh.Threads.Item(9).Obj("head").Set("sha", sha)
		flowUpdateItem(w.ctx, 9, pyjson.New("merge_owner", approver, "cleared", pyjson.New("sha", sha)))
		assertEqual(t, w.queueCI(eventsQueueBranch, "cancelled"), []string{"#9: queued a CI fix"})
		assertEqual(t, owner(), approver)
		assertEqual(t, dig(t, w.ctx.Store.Load(), "items", "9", "ci_fixes"), i+2)
	}
	// The fourth: three CI fixes are used up, so it stops for a person; nobody new owns it.
	w.gh.RemoveLabel(9, config.LabelRevise)
	w.gh.Threads.Item(9).Obj("head").Set("sha", "s4")
	flowUpdateItem(w.ctx, 9, pyjson.New("merge_owner", "claude-7", "cleared", pyjson.New("sha", "s4")))
	assertEqual(t, w.queueCI(eventsQueueBranch, "cancelled"), []string{"#9: blocked after 3 CI fixes"})
	assertEqual(t, w.gh.LabelNames(9), setOf(config.LabelPR, config.LabelBlocked))
	assertEqual(t, owner(), "gpt")
	assertEqual(t, dig(t, w.ctx.Store.Load(), "items", "9", "ci_fixes"), 3)
	assertIn(t, "after 3 tries at fixing it, so I stopped. It needs a person.", lastOf(t, w.gh.BotComments(9)))
}

// The CI fix's prompt says no test is skipped or disabled to get green (CLAUDE.md rule 12), and
// so does a fix pass's.
func TestMergeOwner_TheFixPromptsForbidSkippingATest(t *testing.T) {
	for _, name := range []string{"revise", "fix"} {
		text := readText(t, testsupport.Root+"/bot/prompts/"+name+".md")
		assertIn(t, "CLAUDE.md rule 12", text, name)
		assertIn(t, "never skip", strings.ToLower(text), name)
	}
}
