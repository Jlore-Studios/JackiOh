package tests

// Fullsend as #2748 (section 2) reworked it, for the night bot: a queued parent with open
// sub-issues is adopted as a fullsend tree by itself; the split is hard, goes first among the
// builds as `priority:high` would, and nothing reviews or revises it; the reconcile may send a part
// it finds largely wrong back to the subscription that built it, up to `fullsend_send_backs`, and
// runs again once the part has landed; and the reconcile, hard, ships on a strong approval, its own
// lane's Opus's included (2026-11-02).
// They use fullsend_test.go's world. Squishy's cases are in squishy_cases_test.go.

import (
	"fmt"
	"strconv"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/commands"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/sweep"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/waiting"
)

// fullsendParentOf opens issue `parent` with one open sub-issue (`parent`+1) and one closed one
// (`parent`+2), and labels the parent `labels`.
func fullsendParentOf(t *testing.T, gh *fakes.FakeGitHub, parent int, labels ...string) {
	t.Helper()
	gh.AddIssue(parent, fakes.Issue{Title: "A tracker", Body: "Its pieces are its sub-issues.", Labels: labels})
	gh.AddIssue(parent+1, fakes.Issue{Title: "Its open piece", Labels: []string{"difficulty:medium"}})
	gh.AddIssue(parent+2, fakes.Issue{Title: "Its finished piece", State: "closed"})
	for _, child := range []int{parent + 1, parent + 2} {
		gh.AddSubIssue(parent, crossInt(t, gh.GetIssue(child), "id"))
	}
}

// fullsendAdopted checks that `parent` was adopted as a fullsend tree by itself, `said` the reply.
func fullsendAdopted(t *testing.T, world *fullsendWorld, parent int, said string) {
	t.Helper()
	assertIn(t, fmt.Sprintf("#%d was queued with open sub-issues, so I adopted it as a fullsend tree by itself",
		parent), said)
	assertIn(t, fmt.Sprintf("#%d's sub-issues are its fullsend parts now, with no split: #%d land on `bot/issue-%d`",
		parent, parent+1, parent), said)
	assertIn(t, "To stop it, comment `/harness stop` here: it and its parts leave the queue.", said)
	assertIn(t, "remove its sub-issues before you queue it", said)
	assertEqual(t, world.gh.LabelNames(parent), setOf(config.LabelTree))
	items := crossItems(t, world.ctx)
	tree := crossObj(t, items, strconv.Itoa(parent), "tree")
	assertEqual(t, []any{tree.Get("mode"), tree.Get("onto"), tree.Get("adopted")},
		[]any{"fullsend", fmt.Sprintf("bot/issue-%d", parent), true})
	part := crossObj(t, items, strconv.Itoa(parent+1))
	assertEqual(t, []any{part.Get("onto"), part.Get("part_of")}, []any{fmt.Sprintf("bot/issue-%d", parent), parent})
	assertEqual(t, world.gh.LabelNames(parent+1), setOf("difficulty:medium", config.LabelBuild))
	assertNotIn(t, "onto", items.Obj(strconv.Itoa(parent+2)))
	assertIn(t, [2]string{fmt.Sprintf("bot/issue-%d", parent), "head-of-main"}, world.gh.CreatedBranches)
}

// A queued parent with open sub-issues is a fullsend tree's, however it was queued: by its label,
// by `/harness build`, by an assignment, or by a label an issue form's triage put on with no
// event, which the claim of the queued build finds.
func TestFullsendRework_AQueuedParentWithOpenSubIssuesIsAdoptedAsFullsend(t *testing.T) {
	world := newFullsendWorld(t)
	gh := world.gh
	fullsendParentOf(t, gh, 70, config.LabelBuild)
	fullsendParentOf(t, gh, 75)
	fullsendParentOf(t, gh, 80)
	labelled := events.OnIssueChange(world.ctx, pyjson.New("action", "labeled", "issue", gh.GetIssue(70),
		"label", pyjson.New("name", config.LabelBuild), "sender", fakes.Operator), false)
	fullsendAdopted(t, world, 70, labelled[0])
	asked := events.RunCommands(world.ctx, commands.Parse("/harness build", "jgoetzmann-bot"), gh.GetIssue(75),
		fakes.Operator, 3, nil)
	fullsendAdopted(t, world, 75, asked[0])
	assigned := events.OnIssueChange(world.ctx, pyjson.New("action", "assigned", "issue", gh.GetIssue(80),
		"assignee", pyjson.New("login", "jgoetzmann-bot"), "sender", fakes.Operator), false)
	fullsendAdopted(t, world, 80, assigned[0])
	// An issue form's `bot:build` (triage puts it on with a token that starts no workflow): the
	// claim of the queued build adopts it instead of building it, and builds nothing else this run.
	fullsendParentOf(t, gh, 90, config.LabelBuild)
	for _, p := range []int{71, 76, 81} {
		gh.UpdateIssue(p, pyjson.New("state", "closed")) // the earlier trees' parts are not in the way
	}
	planned := world.claim()
	assertEqual(t, planned.Get("action"), "none")
	fullsendAdopted(t, world, 90, lastOf(t, gh.BotComments(90)))
	planned = world.claim()
	assertEqual(t, []any{planned.Get("number"), planned.Get("onto"), planned.Get("part_of")},
		[]any{91, "bot/issue-90", 90})
}

// A `tracking` issue, a `human` one, Squishy's, and a sub-issue (some tree's part, or a tracker's
// piece) are built as they are asked, never adopted: trees never nest.
func TestFullsendRework_ATrackingHumanSquishysOrSubIssueParentIsNeverAdopted(t *testing.T) {
	world := newFullsendWorld(t)
	gh := world.gh
	fullsendParentOf(t, gh, 70, config.LabelTracking)
	fullsendParentOf(t, gh, 73, config.LabelHuman)
	fullsendParentOf(t, gh, 76, "squishy:build")
	fullsendParentOf(t, gh, 79)
	gh.AddIssue(85, fakes.Issue{Title: "The tracker #79 is a piece of"})
	gh.AddSubIssue(85, crossInt(t, gh.GetIssue(79), "id"))
	plain := ""
	replies := map[int]string{}
	for _, p := range []int{70, 73, 76, 79} {
		replies[p] = queue.QueueBuild(world.ctx, p, "jgoetzmann", false, false, nil, &plain)
	}
	assertIn(t, "#70 is labelled `tracking`: a tracker or status issue", replies[70])
	assertIn(t, "#73 is labelled `human`: people do it", replies[73])
	assertIn(t, "#76 is Squishy's (@squishy-squooby)", replies[76])
	assertIn(t, "Queued #79; I will build it", replies[79])
	for _, p := range []int{70, 73, 76, 79} {
		assertNotIn(t, "adopted", replies[p])
		assertNotIn(t, config.LabelTree, gh.LabelNames(p))
		assertNotIn(t, "tree", crossItems(t, world.ctx).Obj(strconv.Itoa(p)))
		assertNotIn(t, "onto", crossItems(t, world.ctx).Obj(strconv.Itoa(p+1)))
	}
	assertEqual(t, len(gh.CreatedBranches), 0)
	// Its claim builds the sub-issue as asked: a plain build, from `main`.
	planned := world.claim()
	assertEqual(t, []any{planned.Get("number"), planned.Get("action"), planned.Get("mode")}, []any{79, "build", nil})
	assertNotIn(t, "onto", planned)
}

// The split is always hard, so Opus takes it, and goes first among the builds as a
// `priority:high` item does, though no label says so: a person's priority label stays as it is,
// and the queue and the status issue show it as high.
func TestFullsendRework_TheSplitIsHardAndGoesFirstAmongBuildsAsHigh(t *testing.T) {
	world := newFullsendWorld(t)
	gh := world.gh
	gh.AddIssue(30, fakes.Issue{Title: "An older plain build", Labels: []string{config.LabelBuild}})
	gh.AddIssue(31, fakes.Issue{Title: "A medium priority build", Labels: []string{config.LabelBuild, "priority:medium"}})
	gh.AddIssue(fullsendParent, fakes.Issue{Title: "A big project", Labels: []string{config.LabelBuild, fullsendLabel}})
	got := []any{}
	for _, c := range queue.Candidates(world.ctx, world.ctx.Store.Load(), nil, nil) {
		got = append(got, []any{c.Number, c.Priority, c.Difficulty})
	}
	assertEqual(t, got, []any{[]any{fullsendParent, 0, "hard"}, []any{31, 1, "medium"}, []any{30, 2, "medium"}})
	assertEqual(t, gh.LabelNames(fullsendParent), setOf(config.LabelBuild, fullsendLabel))
	issues := gh.ListIssues("", "open", 300, "")
	explained := waiting.Explain(world.ctx, world.ctx.Store.Load(), pyjson.New(), issues, "")
	assertEqual(t, waitingRow(t, explained, fullsendParent).Priority, "high")
	table := dashboard.QueueTable(world.ctx, explained, issues)
	assertTrue(t, strings.HasPrefix(table[2], fmt.Sprintf("| #%d | A big project |", fullsendParent)), table)
	assertTrue(t, strings.HasSuffix(table[2], "| high |"), table[2])
	planned := world.claim()
	assertEqual(t, []any{planned.Get("number"), planned.Get("mode"), planned.Get("difficulty"), planned.Get("priority")},
		[]any{fullsendParent, "fullsend", "hard", "high"})
	assertEqual(t, []any{dig(t, planned, "seats", "build", "tier"), dig(t, planned, "seats", "build", "model")},
		[]any{"strong", "opus"})
	assertIn(t, config.LabelSplitting, gh.LabelNames(fullsendParent))
}

// A split opens sub-issues, not a pull request: its one call reads and answers, nothing reviews
// its answer, and no revision or review run of it is queued after.
func TestFullsendRework_NothingReviewsOrRevisesASplit(t *testing.T) {
	world := newFullsendWorld(t)
	world.gh.AddIssue(fullsendParent, fakes.Issue{Title: "A big project", Body: "Do it all.",
		Labels: []string{config.LabelBuild, fullsendLabel}})
	r := workFake(map[string]runner.Handler{"plan": crossAnswer(fullsendAnswer(fullsendTree())),
		"review": workReviewer(workChanges("Never asked.")), "fix": workBuilder(map[string]string{"x": "x\n"})})
	planned, result := world.run(r)
	assertEqual(t, []any{planned.Get("number"), result.Get("status")}, []any{fullsendParent, "split"})
	assertEqual(t, workRoles(r), []string{"plan"})
	assertEqual(t, len(world.gh.ListPulls("all", "", 200)), 0)
	assertEqual(t, world.gh.LabelNames(fullsendParent), setOf(config.LabelTree))
	for _, c := range queue.Candidates(world.ctx, world.ctx.Store.Load(), nil, nil) {
		assertTrue(t, c.Number != fullsendParent, c.Number)
		assertEqual(t, c.Kind, "build")
	}
	next := world.claim()
	assertEqual(t, []any{next.Get("number"), next.Get("action"), next.Get("onto")},
		[]any{fullsendRules, "build", fullsendOnto})
}

// fullsendSendBack is the reconcile's answer that sends #41 back.
var fullsendSendBack = `<!-- bot: {"status": "send_back", "send_back": [{"part": 41, "findings": ` +
	`"It writes the client, not the rules: src/rules.txt holds the client's screens."}]} -->` +
	"\nPart 1 built the wrong thing; part 2 is fine."

// fullsendLandedWorld is tree #40, split on Opus, whose two parts Muse built and landed; the sweep
// has queued its reconcile.
func fullsendLandedWorld(t *testing.T, env map[string]string) *fullsendWorld {
	t.Helper()
	world := newFullsendWorld(t, fullsendWorldOpts{Env: env, Machine: []string{"muse"}})
	world.split()
	for _, edits := range []map[string]string{{"src/rules.txt": "the client\n"}, {"src/client.txt": "the client\n"}} {
		planned, result := world.run(fullsendBuilds(edits))
		assertEqual(t, []any{planned.Get("provider"), result.Get("status")}, []any{"muse", "part"})
	}
	for _, part := range []int{fullsendRules, fullsendClient} {
		assertEqual(t, dig(t, world.ctx.Store.Load(), "items", strconv.Itoa(part), "built_by"), "muse")
	}
	assertEqual(t, sweep.Trees(world.ctx, testsupport.Night),
		[]string{fmt.Sprintf("#%d: its parts have all closed; queued its reconcile", fullsendParent)})
	return world
}

// The reconcile finds part #41 largely wrong and sends it back to Muse, which built it: the run
// ships nothing, #41 is reopened and owned by Muse, which rebuilds it onto the branch with the
// findings in its prompt, and once it has landed the sweep queues the reconcile again, which may
// send back only the part that has not gone back yet.
func TestFullsendRework_TheReconcileSendsAPartBackToItsBuilderAndRunsAgain(t *testing.T) {
	world := fullsendLandedWorld(t, nil)
	before := world.tip(fullsendOnto)
	r := workFake(map[string]runner.Handler{
		"build":  workBuilder(map[string]string{"src/merged.txt": "half a reconcile\n"}, fullsendSendBack),
		"review": workReviewer(workApprove)})
	planned, result := world.run(r)
	assertEqual(t, []any{planned.Get("number"), planned.Get("mode"), dig(t, planned, "seats", "build", "model")},
		[]any{fullsendParent, "fullsend", "opus"})
	reconciler := planned.Str("provider")
	assertEqual(t, planned.Get("send_backs"), []any{
		pyjson.New("part", fullsendRules, "title", "Part 1: the rules", "built_by", "muse", "sent_backs", 0),
		pyjson.New("part", fullsendClient, "title", "Part 2: the client", "built_by", "muse", "sent_backs", 0)})
	assertIn(t, "## First, judge each part", r.Calls[0].Prompt)
	assertIn(t, "- #41 (Part 1: the rules), built by `muse`", r.Calls[0].Prompt)
	assertIn(t, "A part goes back at most 1 time(s)", r.Calls[0].Prompt)
	// The run ends at the send-back: no checks, no review, nothing pushed, no pull request.
	assertEqual(t, workRoles(r), []string{"build"})
	assertEqual(t, result.Get("status"), "sent_back")
	assertEqual(t, dig(t, result, "send_back", 0, "part"), fullsendRules)
	assertEqual(t, world.tip(fullsendOnto), before)
	assertEqual(t, len(world.gh.ListPulls("all", "", 200)), 0)
	// #41 is reopened and queued, owned by Muse, the subscription that built it.
	assertEqual(t, world.gh.GetIssue(fullsendRules).Get("state"), "open")
	assertIn(t, config.LabelBuild, world.gh.LabelNames(fullsendRules))
	assertEqual(t, world.gh.GetIssue(fullsendClient).Get("state"), "closed")
	record := crossObj(t, world.ctx.Store.Load(), "items", strconv.Itoa(fullsendRules))
	assertEqual(t, []any{record.Get("owner"), record.Get("send_backs"), dig(t, record, "sent_back", "built_by"),
		dig(t, record, "sent_back", "reconciler")}, []any{"muse", 1, "muse", reconciler})
	assertIn(t, fmt.Sprintf("sent back by #%d's reconcile", fullsendParent), record.Str("owner_why"))
	said := lastOf(t, world.gh.BotComments(fullsendRules))
	assertIn(t, "sent this one back to `muse`, the subscription that built it (send-back 1 of 1)", said)
	assertIn(t, "src/rules.txt holds the client's screens", said)
	// The parent waits for its parts again.
	assertEqual(t, world.gh.LabelNames(fullsendParent), setOf(config.LabelTree))
	assertIn(t, "sent them back to the subscriptions that built them, with its findings: #41 to `muse`",
		lastOf(t, world.gh.BotComments(fullsendParent)))
	assertEqual(t, sweep.Trees(world.ctx, testsupport.Night), []string{})
	for _, kept := range []string{"handoff", "session"} {
		assertNotIn(t, kept, crossItems(t, world.ctx).Obj(strconv.Itoa(fullsendParent))) // the next one starts afresh
	}
	// Muse rebuilds #41 onto the branch, the findings in its prompt.
	prompts := []string{}
	planned, result = world.run(workFake(map[string]runner.Handler{
		"build": fullsendBuilder(map[string]string{"src/rules.txt": "the rules\n"}, &prompts)}))
	assertEqual(t, []any{planned.Get("number"), planned.Get("provider"), planned.Get("onto"), result.Get("status")},
		[]any{fullsendRules, "muse", fullsendOnto, "part"})
	assertIn(t, "its builder `muse` builds it again: its tree's reconcile sent it back",
		strings.Join(stringsOf(planned.List("routing")), "\n"))
	assertIn(t, fmt.Sprintf("#%d's reconcile read every part of the tree and sent this one back to you", fullsendParent),
		prompts[0])
	assertIn(t, "src/rules.txt holds the client's screens", prompts[0])
	assertEqual(t, fakes.Git(world.origin, "show", "refs/heads/"+fullsendOnto+":src/rules.txt"), "the rules")
	assertEqual(t, world.gh.GetIssue(fullsendRules).Get("state"), "closed")
	record = crossObj(t, world.ctx.Store.Load(), "items", strconv.Itoa(fullsendRules))
	assertEqual(t, []any{record.Get("built_by"), record.Get("send_backs")}, []any{"muse", 1})
	assertNotIn(t, "sent_back", record)
	// Its parts have all closed again: the reconcile is queued again, and only #42 may go back now.
	assertEqual(t, sweep.Trees(world.ctx, testsupport.Night),
		[]string{fmt.Sprintf("#%d: its parts have all closed; queued its reconcile", fullsendParent)})
	again := world.claim()
	assertEqual(t, []any{again.Get("number"), again.Get("mode"), again.Get("difficulty")},
		[]any{fullsendParent, "fullsend", "hard"})
	assertEqual(t, again.Get("send_backs"), []any{
		pyjson.New("part", fullsendClient, "title", "Part 2: the client", "built_by", "muse", "sent_backs", 0)})
}

// A part goes back at most `fullsend_send_backs` times (1 for the night bot): a reconcile that
// names one past it is told so, and fixes it itself in the same run, which then goes through its
// checks and review as any reconcile does. With 0 none goes back at all.
func TestFullsendRework_APartGoesBackOnlyUpToTheCap(t *testing.T) {
	assertEqual(t, testsupport.MakeConfig().FullsendSendBacks, 1)
	world := fullsendLandedWorld(t, nil)
	world.ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, fullsendRules).Set("send_backs", 1) // it went back once already
		return nil
	}, "state")
	r := workFake(map[string]runner.Handler{
		"build":  workBuilder(map[string]string{}, fullsendSendBack),
		"fix":    workBuilder(map[string]string{"src/game.txt": "the rules and the client\n"}),
		"review": workReviewer(workApprove)})
	planned, result := world.run(r)
	assertEqual(t, planned.Get("send_backs"), []any{
		pyjson.New("part", fullsendClient, "title", "Part 2: the client", "built_by", "muse", "sent_backs", 0)})
	assertNotIn(t, "- #41 (", r.Calls[0].Prompt)
	assertEqual(t, workRoles(r), []string{"build", "fix", "review"})
	assertIn(t, "These parts cannot be sent back now", r.Calls[1].Prompt)
	assertEqual(t, []any{result.Get("status"), result.Get("send_back_refused")}, []any{"approved", []any{fullsendRules}})
	assertEqual(t, world.gh.GetIssue(fullsendRules).Get("state"), "closed")
	assertEqual(t, dig(t, world.ctx.Store.Load(), "items", strconv.Itoa(fullsendRules), "send_backs"), 1)
	assertEqual(t, crossInt(t, lastOf(t, world.gh.ListPulls("all", fullsendOnto, 200)), "number") > 0, true)
	// A bot whose cap is 0 sends no part back: its reconcile is told to fix every part itself.
	none := *world.ctx
	none.Cfg = testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("fullsend_send_backs", 0)})
	assertEqual(t, plan.SendBackable(&none, fullsendParent), []any{})
}

// The reconcile's pull request is hard, so it ships only on a strong approval: its own run's
// reviewer, Opus on the reconcile's own Claude account in a fresh adversarial session, gives it
// (2026-11-02, #2748), and auto-merge turns on with no review run. A medium approval of it would
// ship nothing, and a review run of it, should one be asked for, is strong.
func TestFullsendRework_TheReconcileShipsOnItsOwnLanesOpusApproval(t *testing.T) {
	world := fullsendLandedWorld(t, crossAll)
	planned, result := world.run(workFake(map[string]runner.Handler{
		"build":  fullsendBuilder(map[string]string{"src/game.txt": "the rules and the client\n"}, nil),
		"review": workReviewer(workApprove)}))
	reconciler := planned.Str("provider")
	assertTrue(t, strings.HasPrefix(reconciler, "claude-"), reconciler)
	assertEqual(t, []any{planned.Get("number"), dig(t, planned, "seats", "build", "model"),
		dig(t, planned, "seats", "review", "provider"), dig(t, planned, "seats", "review", "model"),
		dig(t, planned, "seats", "review", "tier"), result.Get("status")},
		[]any{fullsendParent, "opus", reconciler, "opus", "strong", "approved"})
	pr := crossPullOf(t, world.gh, fullsendOnto)
	record := crossObj(t, world.ctx.Store.Load(), "items", strconv.Itoa(pr))
	assertEqual(t, []any{record.Get("difficulty"), dig(t, record, "votes", "built_by"), dig(t, record, "votes", "sha")},
		[]any{"hard", reconciler, result.Get("head")})
	assertIn(t, fmt.Sprintf("PR_%d", pr), world.gh.AutoMerge)
	assertNotIn(t, config.LabelCross, world.gh.LabelNames(pr))
	assertEqual(t, []any{dig(t, record, "cleared", "built_by"), dig(t, record, "cleared", "approver", "provider"),
		dig(t, record, "cleared", "approver", "tier")}, []any{reconciler, reconciler, "strong"})
	// No review run is queued for it; one asked for would be strong, its own lane's last.
	for _, c := range queue.Candidates(world.ctx, world.ctx.Store.Load(), nil, nil) {
		assertNotEqual(t, c.Number, pr)
	}
	queue.QueueReview(world.ctx, pr, "jgoetzmann", "", "", nil)
	for _, c := range queue.Candidates(world.ctx, world.ctx.Store.Load(), nil, nil) {
		if c.Number == pr {
			assertEqual(t, []any{c.Kind, c.Difficulty, c.BuiltBy, c.ReviewTiers}, []any{"review", "hard", reconciler,
				[]string{"strong"}})
		}
	}
}
