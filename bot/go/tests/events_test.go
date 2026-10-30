// Events: comment commands, labels, assignment, reviews, CI results, closed pull requests.
//
// bot/tests/test_events.py. It also holds the small helpers the seven files ported with it
// (events, sweep, rating, rebuild, suspend, hardening, human) share.
package tests

import (
	"fmt"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// ---------------------------------------------------------------------------- shared helpers

// eventsUpdateState is `ctx.store.update(lambda s: s.update(**fields))`.
func eventsUpdateState(ctx *hctx.Context, kv ...any) {
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.Update(pyjson.New(kv...))
		return nil
	}, "state")
}

// eventsLabels is a thread's `labels` field: [{"name": n} for n in names].
func eventsLabels(names ...string) []any {
	out := []any{}
	for _, n := range names {
		out = append(out, pyjson.New("name", n))
	}
	return out
}

// ---------------------------------------------------------------------------- test_events.py

const eventsTrust = "jgoetzmann 3 id:95732896\nhelper 2\nreader 1\n"

var (
	eventsHelper = pyjson.New("login", "helper", "id", 7)
	eventsReader = pyjson.New("login", "reader", "id", 8)
)

// eventsCommentOpts is comment_event's keyword arguments; a zero field is the default.
type eventsCommentOpts struct {
	User        *pyjson.Object // = OPERATOR
	Association string         // = "OWNER"
	PR          bool           // = False
}

func eventsCommentEvent(number int, body string, opts ...eventsCommentOpts) *pyjson.Object {
	var o eventsCommentOpts
	if len(opts) > 0 {
		o = opts[0]
	}
	user := o.User
	if user == nil {
		user = fakes.Operator
	}
	association := o.Association
	if association == "" {
		association = "OWNER"
	}
	issue := pyjson.New("number", number, "title", "t")
	if o.PR {
		issue.Set("pull_request", pyjson.New())
	}
	return pyjson.New("action", "created", "sender", user, "issue", issue,
		"comment", pyjson.New("id", 501, "body", body, "user", user, "author_association", association))
}

// eventsComments is CommentTests' world.
type eventsComments struct {
	t   *testing.T
	gh  *fakes.FakeGitHub
	ctx *hctx.Context
}

func newEventsComments(t *testing.T) *eventsComments {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5, fakes.Issue{Title: "Make the Coin shiny"})
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, TrustText: ptr(eventsTrust)})
	return &eventsComments{t: t, gh: gh, ctx: ctx}
}

func (w *eventsComments) send(body string, opts ...eventsCommentOpts) []string {
	return events.Handle(w.ctx, "issue_comment", eventsCommentEvent(5, body, opts...))
}

func (w *eventsComments) reply() string {
	w.t.Helper()
	return lastOf(w.t, w.gh.BotComments(5))
}

func (w *eventsComments) reactions(cid ...int) []string {
	id := 501
	if len(cid) > 0 {
		id = cid[0]
	}
	out := []string{}
	for _, r := range w.gh.Reacted {
		if r.ID == id {
			out = append(out, r.Content)
		}
	}
	return out
}

func (w *eventsComments) items() any { return dig(w.t, w.ctx.Store.Load(), "items") }

func TestEvents_BuildQueuesTheIssue(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness build")
	assertEqual(t, w.gh.LabelNames(5), setOf(config.LabelBuild))
	assertIn(t, "Queued #5", w.reply())
	assertIn(t, fakes.Reaction{ID: 501, Content: "eyes"}, w.gh.Reacted)
	assertEqual(t, w.gh.Dispatches, []fakes.Dispatch{})
	assertEqual(t, dig(t, w.items(), "5", "requested_by"), "jgoetzmann")
}

func TestEvents_ForceStartsARunNow(t *testing.T) {
	w := newEventsComments(t)
	w.send("@jgoetzmann-bot build --force")
	assertEqual(t, w.gh.Dispatches[0].Inputs, map[string]string{"item": "5", "force": "true", "mode": "build"})
}

func TestEvents_ForceNeedsTheOperator(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness build --force", eventsCommentOpts{User: eventsHelper, Association: "COLLABORATOR"})
	assertEqual(t, w.gh.Dispatches, []fakes.Dispatch{})
	assertEqual(t, w.gh.LabelNames(5), setOf())
	assertIn(t, "operator level", w.reply())
}

func TestEvents_AMaintainerMayBuildWithoutForce(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness build", eventsCommentOpts{User: eventsHelper, Association: "COLLABORATOR"})
	assertEqual(t, w.gh.LabelNames(5), setOf(config.LabelBuild))
}

func TestEvents_AnAskerMayOnlyAsk(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness status\n/harness build", eventsCommentOpts{User: eventsReader, Association: "COLLABORATOR"})
	assertIn(t, "Night bot status", w.reply())
	assertIn(t, "maintainer level", w.reply())
	assertEqual(t, w.gh.LabelNames(5), setOf())
}

func TestEvents_StrangersAreIgnoredSilently(t *testing.T) {
	w := newEventsComments(t)
	out := w.send("/harness halt", eventsCommentOpts{User: fakes.Stranger, Association: "NONE"})
	assertIn(t, "not on the trust list", out[0])
	assertEqual(t, w.gh.BotComments(5), []string{})
	assertFalse(t, dig(t, w.ctx.Store.Load(), "halted"))
	// A listed name without an id needs GitHub's word too.
	w.send("/harness build", eventsCommentOpts{User: eventsHelper, Association: "CONTRIBUTOR"})
	assertEqual(t, w.gh.LabelNames(5), setOf())
}

func TestEvents_TheBotsOwnCommentsAreIgnored(t *testing.T) {
	w := newEventsComments(t)
	out := events.Handle(w.ctx, "issue_comment",
		eventsCommentEvent(5, "/harness build\n"+config.Marker, eventsCommentOpts{User: fakes.Bot}))
	assertEqual(t, out, []string{"ignored: the bot's own event"})
	out = events.Handle(w.ctx, "issue_comment",
		eventsCommentEvent(5, "/harness build\n"+config.Marker, eventsCommentOpts{User: fakes.Operator}))
	assertEqual(t, out, []string{"ignored: the bot's own comment"})
}

func TestEvents_QuotingTheBotStillLetsAPersonCommand(t *testing.T) {
	w := newEventsComments(t)
	w.send("> earlier reply\n> " + config.Marker + "\n/harness build")
	assertEqual(t, w.gh.LabelNames(5), setOf(config.LabelBuild))
}

func TestEvents_HaltAndStart(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness halt going on holiday")
	s := w.ctx.Store.Load()
	assertTrue(t, dig(t, s, "halted"))
	assertEqual(t, dig(t, s, "halt", "reason"), "going on holiday")
	w.send("/harness start")
	assertFalse(t, dig(t, w.ctx.Store.Load(), "halted"))
}

func TestEvents_HaltNeedsTheOperator(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness halt", eventsCommentOpts{User: eventsHelper, Association: "COLLABORATOR"})
	assertFalse(t, dig(t, w.ctx.Store.Load(), "halted"))
}

func TestEvents_StatusReportsTheQueueAndWindow(t *testing.T) {
	w := newEventsComments(t)
	w.gh.AddIssue(6, fakes.Issue{Labels: []string{config.LabelBuild}})
	w.send("/harness status")
	text := w.reply()
	assertIn(t, "Queued to build: #6", text)
	assertIn(t, "`claude-1` (claude: `opus` strong, `sonnet` medium; 21:00–07:00 "+
		"America/Chicago): outside its "+
		"hours", text)
	assertIn(t, "`claude-2` (claude: `opus` strong, `sonnet` medium; 21:00–07:00 "+
		"America/Chicago): its secret "+
		"`CLAUDE_CODE_OAUTH_TOKEN_2` is not set", text)
}

func TestEvents_FreeTextOnAnIssueAsksForABuild(t *testing.T) {
	w := newEventsComments(t)
	w.send("@jgoetzmann-bot please also make it sparkle")
	assertEqual(t, w.gh.LabelNames(5), setOf(config.LabelBuild))
	assertIn(t, "read your notes", w.reply())
}

func TestEvents_ABuildRequestOnAnIssueWithAPRRevisesThePR(t *testing.T) {
	w := newEventsComments(t)
	w.gh.Threads.Item(5).Set("labels", eventsLabels(config.LabelPROpen))
	w.gh.AddPull(9, "bot/issue-5", fakes.Pull{Labels: []string{config.LabelPR}})
	w.gh.Threads.Item(9).Set("auto_merge", pyjson.New("merge_method", "squash"))
	w.gh.AutoMerge["PR_9"] = "squash"
	w.send("@jgoetzmann-bot also make it sparkle")
	assertEqual(t, w.gh.LabelNames(9), setOf(config.LabelPR, config.LabelRevise))
	assertNotIn(t, "PR_9", w.gh.AutoMerge)
}

func TestEvents_StopTakesItOutAndTellsARunningJob(t *testing.T) {
	w := newEventsComments(t)
	w.gh.Threads.Item(5).Set("labels", eventsLabels(config.LabelWorking))
	w.send("/harness stop")
	// The run still holds it until it stops, so a new request waits for that run.
	assertEqual(t, w.gh.LabelNames(5), setOf(config.LabelWorking))
	assertTrue(t, dig(t, w.items(), "5", "stop_requested"))
	assertIn(t, "next checkpoint", w.reply())
}

func TestEvents_SuggestAndRun(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness suggest")
	assertTrue(t, dig(t, w.ctx.Store.Load(), "suggest", "requested"))
	w.send("/harness suggest --force")
	assertEqual(t, lastOf(t, w.gh.Dispatches).Inputs["mode"], "suggest")
	w.send("/harness run #5")
	assertEqual(t, lastOf(t, w.gh.Dispatches).Inputs, map[string]string{"item": "5", "force": "true", "mode": "auto"})
}

func TestEvents_AMisspeltVerbRunsNothingAndAsks(t *testing.T) {
	w := newEventsComments(t)
	w.send("@jgoetzmann-bot stauts")
	assertIn(t, "`stauts` is not a command; did you mean `status`?", w.reply())
	assertEqual(t, w.gh.LabelNames(5), setOf())
	assertEqual(t, w.gh.Reacted, []fakes.Reaction{{ID: 501, Content: "eyes"}, {ID: 501, Content: "rocket"}})
	assertNotIn(t, "5", w.items())
}

func TestEvents_FreeTextAfterTheSlashAsksForABuildToo(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness please make it sparkle")
	assertEqual(t, w.gh.LabelNames(5), setOf(config.LabelBuild))
	assertIn(t, "read your notes", w.reply())
}

func TestEvents_HelpAndStatus(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness help")
	assertIn(t, "**Commands.**", w.reply())
	w.send("@jgoetzmann-bot help build")
	assertIn(t, "**`build [notes]`** (level 2, maintainer; also written `work`)", w.reply())
	w.send("/harness status")
	assertIn(t, "Night bot status", w.reply())
	assertIn(t, "lists the commands", w.reply())
	assertNotIn(t, "| Verb |", w.reply())
}

func TestEvents_WorkForAModelGetsAThumbsUpAndIsKeptOnTheRecord(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness build")
	assertEqual(t, w.reactions(), []string{"eyes", "+1", "rocket"})
	assertEqual(t, dig(t, w.items(), "5", "asks"), []any{"c:501"})
}

func TestEvents_ACommandWithNoModelWorkGetsNoThumbsUp(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness status")
	assertEqual(t, w.reactions(), []string{"eyes", "rocket"})
	w.gh.Threads.Item(5).Set("state", "closed")
	payload := eventsCommentEvent(5, "/harness build")
	comment := eventsCommentEvent(5, "").Obj("comment").Copy()
	comment.Set("id", 502)
	comment.Set("body", "/harness build")
	payload.Set("comment", comment)
	events.Handle(w.ctx, "issue_comment", payload)
	assertIn(t, "is closed", w.reply())
	assertEqual(t, w.reactions(502), []string{"eyes", "rocket"})
}

func TestEvents_ANoteDuringARunWaitsForTheNextPass(t *testing.T) {
	w := newEventsComments(t)
	w.gh.Threads.Item(5).Set("labels", eventsLabels(config.LabelWorking))
	w.send("@jgoetzmann-bot also make it sparkle")
	assertEqual(t, w.reactions(), []string{"eyes", "+1", "rocket"})
	record := dig(t, w.items(), "5")
	assertEqual(t, []any{dig(t, record, "pending_request"), dig(t, record, "asks")},
		[]any{true, []any{"c:501"}})
}

func TestEvents_StopAnswersWhatWasWaiting(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness build")
	payload := eventsCommentEvent(5, "")
	payload.Set("comment", pyjson.New("id", 502, "body", "/harness stop", "user", fakes.Operator,
		"author_association", "OWNER"))
	events.Handle(w.ctx, "issue_comment", payload)
	assertEqual(t, w.reactions(), []string{"eyes", "+1", "rocket", "confused"})
	assertEqual(t, w.reactions(502), []string{"eyes", "rocket"})
	assertEqual(t, dig(t, w.items(), "5", "asks"), []any{})
}

func TestEvents_ARequestRoutedToThePullRequestKeepsItsComment(t *testing.T) {
	w := newEventsComments(t)
	w.gh.Threads.Item(5).Set("labels", eventsLabels(config.LabelPROpen))
	w.gh.AddPull(9, "bot/issue-5", fakes.Pull{Labels: []string{config.LabelPR}})
	w.send("@jgoetzmann-bot also make it sparkle")
	assertEqual(t, dig(t, w.items(), "9", "asks"), []any{"c:501"})
}

func TestEvents_ASuggestRequestWaitsForTheSurvey(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness suggest")
	assertEqual(t, w.reactions(), []string{"eyes", "+1", "rocket"})
	assertEqual(t, dig(t, w.ctx.Store.Load(), "suggest", "asks"), []any{"c:501"})
}

func TestEvents_ALineCommentOnADiffIsKeptByItsOwnKind(t *testing.T) {
	w := newEventsComments(t)
	w.gh.AddPull(9, "feature/mine")
	events.Handle(w.ctx, "pull_request_review_comment", pyjson.New(
		"action", "created", "sender", fakes.Operator, "pull_request", pyjson.New("number", 9),
		"comment", pyjson.New("id", 77, "body", "@jgoetzmann-bot rename this", "user", fakes.Operator,
			"author_association", "OWNER")))
	assertEqual(t, dig(t, w.items(), "9", "asks"), []any{"rc:77"})
	assertIn(t, fakes.Reaction{ID: 77, Content: "+1"}, w.gh.Reacted)
}

func TestEvents_ReviseOnAPullRequestFromAForkIsRefused(t *testing.T) {
	w := newEventsComments(t)
	w.gh.AddPull(9, "their-branch", fakes.Pull{Fork: true})
	events.Handle(w.ctx, "issue_comment", eventsCommentEvent(9, "/harness revise x", eventsCommentOpts{PR: true}))
	assertIn(t, "comes from a fork", lastOf(t, w.gh.BotComments(9)))
	assertEqual(t, w.gh.LabelNames(9), setOf())
}

func TestEvents_APersonCanAskForChangesOnTheirOwnPullRequest(t *testing.T) {
	w := newEventsComments(t)
	w.gh.AddPull(9, "feature/mine")
	events.Handle(w.ctx, "issue_comment",
		eventsCommentEvent(9, "@jgoetzmann-bot fix the failing test", eventsCommentOpts{PR: true}))
	assertEqual(t, w.gh.LabelNames(9), setOf(config.LabelRevise))
}

// eventsLabelWorld is LabelAndAssignTests' world.
func eventsLabelWorld(t *testing.T) (*fakes.FakeGitHub, *hctx.Context) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5, fakes.Issue{Labels: []string{config.LabelBuild}})
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, TrustText: ptr(eventsTrust)})
	return gh, ctx
}

func TestEvents_TheBuildLabelFromATrustedPersonQueues(t *testing.T) {
	gh, ctx := eventsLabelWorld(t)
	payload := pyjson.New("action", "labeled", "sender", fakes.Operator, "label", pyjson.New("name", config.LabelBuild),
		"issue", pyjson.New("number", 5))
	events.Handle(ctx, "issues", payload)
	assertEqual(t, gh.LabelNames(5), setOf(config.LabelBuild))
	assertIn(t, "Queued #5", lastOf(t, gh.BotComments(5)))
	assertIn(t, "queued_at", dig(t, ctx.Store.Load(), "items", "5"))
}

func TestEvents_TheBuildLabelFromAnyoneElseIsRemoved(t *testing.T) {
	gh, ctx := eventsLabelWorld(t)
	payload := pyjson.New("action", "labeled", "sender", fakes.Stranger, "label", pyjson.New("name", config.LabelBuild),
		"issue", pyjson.New("number", 5))
	events.Handle(ctx, "issues", payload)
	assertEqual(t, gh.LabelNames(5), setOf())
}

func TestEvents_AssigningTheBotQueues(t *testing.T) {
	gh, ctx := eventsLabelWorld(t)
	gh.Threads.Item(5).Set("labels", []any{})
	payload := pyjson.New("action", "assigned", "sender", fakes.Operator, "assignee", fakes.Bot, "issue", pyjson.New("number", 5))
	events.Handle(ctx, "issues", payload)
	assertEqual(t, gh.LabelNames(5), setOf(config.LabelBuild))
}

func TestEvents_AssigningSomeoneElseDoesNothing(t *testing.T) {
	gh, ctx := eventsLabelWorld(t)
	gh.Threads.Item(5).Set("labels", []any{})
	payload := pyjson.New("action", "assigned", "sender", fakes.Operator, "assignee", fakes.Stranger, "issue", pyjson.New("number", 5))
	events.Handle(ctx, "issues", payload)
	assertEqual(t, gh.LabelNames(5), setOf())
}

func TestEvents_TheReviseLabelOnAPullRequest(t *testing.T) {
	gh, ctx := eventsLabelWorld(t)
	gh.AddPull(9, "bot/issue-5", fakes.Pull{Labels: []string{config.LabelPR, config.LabelRevise}})
	payload := pyjson.New("action", "labeled", "sender", fakes.Operator, "label", pyjson.New("name", config.LabelRevise),
		"pull_request", pyjson.New("number", 9))
	events.Handle(ctx, "pull_request_target", payload)
	assertEqual(t, dig(t, ctx.Store.Load(), "items", "9", "source"), "request")
}

// eventsReviewWorld is ReviewAndCiTests' world.
type eventsReviewWorld struct {
	gh   *fakes.FakeGitHub
	pull *pyjson.Object
	ctx  *hctx.Context
}

func newEventsReviewWorld() *eventsReviewWorld {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5, fakes.Issue{Labels: []string{config.LabelPROpen}})
	pull := gh.AddPull(9, "bot/issue-5", fakes.Pull{Body: "Closes #5", Labels: []string{config.LabelPR}, SHA: "s1"})
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, TrustText: ptr(eventsTrust)})
	return &eventsReviewWorld{gh: gh, pull: pull, ctx: ctx}
}

// ci is ReviewAndCiTests.ci(attempt=1, sha="s1").
func (w *eventsReviewWorld) ci(attempt int, sha string) []string {
	run := pyjson.New("name", "CI", "conclusion", "failure", "head_branch", "bot/issue-5", "head_sha", sha,
		"id", 99, "run_attempt", attempt, "html_url", "u", "pull_requests", []any{pyjson.New("number", 9)})
	// The bot pushed the branch, so GitHub names it as the sender of the CI run.
	return events.Handle(w.ctx, "workflow_run",
		pyjson.New("action", "completed", "sender", fakes.Bot, "workflow_run", run))
}

func TestEvents_ChangesRequestedOnABotPRQueuesARevision(t *testing.T) {
	w := newEventsReviewWorld()
	payload := pyjson.New("action", "submitted", "sender", fakes.Operator, "pull_request", w.gh.GetPull(9),
		"review", pyjson.New("state", "changes_requested", "body", "Name it better.",
			"user", fakes.Operator, "author_association", "OWNER"))
	events.Handle(w.ctx, "pull_request_review", payload)
	assertIn(t, config.LabelRevise, w.gh.LabelNames(9))
}

func TestEvents_CIFailureIsRerunOnceThenFixed(t *testing.T) {
	w := newEventsReviewWorld()
	w.ci(1, "s1")
	assertEqual(t, w.gh.Reruns, []any{99})
	assertNotIn(t, config.LabelRevise, w.gh.LabelNames(9))
	w.ci(2, "s1")
	assertIn(t, config.LabelRevise, w.gh.LabelNames(9))
	record := dig(t, w.ctx.Store.Load(), "items", "9")
	assertEqual(t, []any{dig(t, record, "source"), dig(t, record, "ci_run_id"), dig(t, record, "ci_fixes")},
		[]any{"ci", 99, 1})
}

func TestEvents_ActionsCallsUseTheJobsOwnToken(t *testing.T) {
	w := newEventsReviewWorld()
	actions := fakes.NewFakeGitHub()
	w.ctx.ActionsGH = actions
	w.ci(1, "s1")
	assertEqual(t, []any{actions.Reruns, w.gh.Reruns}, []any{[]any{99}, []any{}})
	w.ctx.Dispatch(nil, true, "auto")
	assertEqual(t, len(actions.Dispatches), 1)
	assertEqual(t, w.gh.Dispatches, []fakes.Dispatch{})
}

func TestEvents_CIFixesAreCapped(t *testing.T) {
	w := newEventsReviewWorld()
	flowUpdateItem(w.ctx, 9, pyjson.New("ci_fixes", 3, "ci_reruns", pyjson.New("s1", 1)))
	w.gh.Threads.Item(9).Set("auto_merge", pyjson.New("merge_method", "squash"))
	w.gh.AutoMerge["PR_9"] = "squash"
	w.ci(2, "s1")
	assertEqual(t, w.gh.LabelNames(9), setOf(config.LabelPR, config.LabelBlocked))
	assertNotIn(t, "PR_9", w.gh.AutoMerge)
	assertIn(t, "It needs a person", lastOf(t, w.gh.BotComments(9)))
}

func TestEvents_APersonAskingAgainDoesNotResetTheCICap(t *testing.T) {
	w := newEventsReviewWorld()
	flowUpdateItem(w.ctx, 9, pyjson.New("ci_fixes", 2))
	events.Handle(w.ctx, "issue_comment",
		eventsCommentEvent(9, "/harness revise try again", eventsCommentOpts{PR: true}))
	assertEqual(t, dig(t, w.ctx.Store.Load(), "items", "9", "ci_fixes"), 2)
}

func TestEvents_CILeavesAStoppedOrBlockedPRAlone(t *testing.T) {
	w := newEventsReviewWorld()
	flowUpdateItem(w.ctx, 9, pyjson.New("stop_requested", true))
	w.ci(3, "s1")
	assertEqual(t, w.gh.Reruns, []any{})
	assertEqual(t, w.gh.LabelNames(9), setOf(config.LabelPR))
}

func TestEvents_CIOnAStaleCommitOrAHumanPRIsIgnored(t *testing.T) {
	w := newEventsReviewWorld()
	run := pyjson.New("name", "CI", "conclusion", "failure", "head_branch", "bot/issue-5", "head_sha", "old",
		"id", 99, "run_attempt", 3, "pull_requests", []any{pyjson.New("number", 9)})
	events.Handle(w.ctx, "workflow_run", pyjson.New("action", "completed", "sender", fakes.Operator,
		"workflow_run", run))
	assertEqual(t, w.gh.LabelNames(9), setOf(config.LabelPR))
	w.gh.AddPull(10, "feature/human", fakes.Pull{SHA: "h1"})
	run = pyjson.New("name", "CI", "conclusion", "failure", "head_branch", "feature/human", "head_sha", "h1",
		"id", 98, "run_attempt", 3, "pull_requests", []any{pyjson.New("number", 10)})
	events.Handle(w.ctx, "workflow_run", pyjson.New("action", "completed", "sender", fakes.Operator,
		"workflow_run", run))
	assertEqual(t, w.gh.LabelNames(10), setOf())
}

func TestEvents_AMergedBotPRClearsTheIssue(t *testing.T) {
	w := newEventsReviewWorld()
	pull := w.gh.GetPull(9)
	pull.Set("merged", true)
	// Auto-merge is credited to whoever turned it on: the bot.
	events.Handle(w.ctx, "pull_request_target", pyjson.New("action", "closed", "sender", fakes.Bot,
		"pull_request", pull))
	assertEqual(t, w.gh.LabelNames(5), setOf())
	assertTrue(t, dig(t, w.ctx.Store.Load(), "items", "9", "merged"))
	assertEqual(t, dig(t, w.gh.GetIssue(5), "state"), "closed")
	assertEqual(t, dig(t, w.gh.GetIssue(5), "state_reason"), "completed")
}

func TestEvents_AMergedBotPRLeavesAnIssueGithubAlreadyClosed(t *testing.T) {
	w := newEventsReviewWorld()
	w.gh.UpdateIssue(5, pyjson.New("state", "closed", "state_reason", "not_planned"))
	pull := w.gh.GetPull(9)
	pull.Set("merged", true)
	events.Handle(w.ctx, "pull_request_target", pyjson.New("action", "closed", "sender", fakes.Operator,
		"pull_request", pull))
	assertEqual(t, dig(t, w.gh.GetIssue(5), "state_reason"), "not_planned")
}

func TestEvents_ABotPRMergedOutsideTheDefaultBranchLeavesTheIssueOpen(t *testing.T) {
	w := newEventsReviewWorld()
	pull := w.gh.GetPull(9)
	pull.Update(pyjson.New("merged", true, "base", pyjson.New("ref", "release")))
	events.Handle(w.ctx, "pull_request_target", pyjson.New("action", "closed", "sender", fakes.Operator,
		"pull_request", pull))
	assertEqual(t, dig(t, w.gh.GetIssue(5), "state"), "open")
}

func TestEvents_ABotPRClosedUnmergedSaysHowToRetry(t *testing.T) {
	w := newEventsReviewWorld()
	pull := w.gh.GetPull(9)
	events.Handle(w.ctx, "pull_request_target", pyjson.New("action", "closed", "sender", fakes.Operator,
		"pull_request", pull))
	assertIn(t, "closed without merging", lastOf(t, w.gh.BotComments(5)))
}

// eventsQueueWorld is MergeQueueTests' world: `main`'s merge queue (infra/github.tf): a cleared
// head its build took out gets a CI fix at once; one still queued is the queue's; one taken out
// with no failed build goes back once per commit of `main`; a queued pull request is held by
// taking it out.
type eventsQueueWorld struct {
	gh  *fakes.FakeGitHub
	ctx *hctx.Context
}

// eventsQueue is MergeQueueTests.QUEUE.
var eventsQueue = map[string]string{"actor": "github-merge-queue", "type": "Bot", "reason": "CONFLICT"}

func newEventsQueueWorld() *eventsQueueWorld {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5, fakes.Issue{Labels: []string{config.LabelPROpen}})
	gh.AddPull(9, "bot/issue-5", fakes.Pull{Body: "Closes #5", Labels: []string{config.LabelPR}, SHA: "s1"})
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, TrustText: ptr(eventsTrust)})
	flowUpdateItem(ctx, 9, pyjson.New("cleared", pyjson.New("sha", "s1")))
	return &eventsQueueWorld{gh: gh, ctx: ctx}
}

// queueCI is queue_ci(branch="gh-readonly-queue/main/pr-9-abc", conclusion="cancelled").
func (w *eventsQueueWorld) queueCI(branch, conclusion string) []string {
	// A merge group's run names no pull request; its branch does. It ends cancelled once a
	// required check failed (ci.yml's `queue-cancel-*`).
	run := pyjson.New("name", "CI", "conclusion", conclusion, "head_branch", branch, "head_sha", "mg",
		"id", 77, "html_url", "q", "pull_requests", []any{}, "event", "merge_group")
	return events.Handle(w.ctx, "workflow_run",
		pyjson.New("action", "completed", "sender", fakes.Bot, "workflow_run", run))
}

const eventsQueueBranch = "gh-readonly-queue/main/pr-9-abc"

func (w *eventsQueueWorld) housekeeping() []string {
	return plan.Housekeeping(w.ctx, w.ctx.Store.Load())
}

func eventsCopyMap(m map[string]string) map[string]string {
	out := map[string]string{}
	for k, v := range m {
		out[k] = v
	}
	return out
}

func TestEvents_AClearedHeadItsBuildTookOutGetsACIFixAtOnce(t *testing.T) {
	w := newEventsQueueWorld()
	w.queueCI(eventsQueueBranch, "cancelled")
	assertEqual(t, []any{w.gh.AutoMerge, w.gh.Reruns}, []any{map[string]string{}, []any{}})
	assertIn(t, config.LabelRevise, w.gh.LabelNames(9))
	record := dig(t, w.ctx.Store.Load(), "items", "9")
	assertEqual(t, []any{dig(t, record, "source"), dig(t, record, "ci_run_id"), dig(t, record, "ci_fixes")},
		[]any{"ci", 77, 1})
	assertIn(t, "The merge queue took this out", lastOf(t, w.gh.BotComments(9)))
}

func TestEvents_OneStillQueuedAHeldOneOrAnUnclearedHeadIsLeftAlone(t *testing.T) {
	w := newEventsQueueWorld()
	w.gh.MergeQueue = map[string]bool{"PR_9": true} // built again without a pull request ahead that failed
	w.queueCI(eventsQueueBranch, "cancelled")
	w.gh.MergeQueue = map[string]bool{}
	flowUpdateItem(w.ctx, 9, pyjson.New("stop_requested", true))
	w.queueCI(eventsQueueBranch, "cancelled")
	flowUpdateItem(w.ctx, 9, pyjson.New("stop_requested", false, "cleared", pyjson.New("sha", "s0")))
	w.queueCI(eventsQueueBranch, "failure")
	w.gh.AddPull(10, "people/x", fakes.Pull{SHA: "p1"})
	w.queueCI("gh-readonly-queue/main/pr-10-abc", "cancelled")
	assertEqual(t, w.gh.AutoMerge, map[string]string{})
	assertEqual(t, w.gh.LabelNames(9), setOf(config.LabelPR))
	assertEqual(t, w.gh.BotComments(9), []string{})
	assertEqual(t, w.gh.Reruns, []any{})
}

func TestEvents_OneTakenOutWithNoFailedBuildGoesBackOncePerCommitOfMain(t *testing.T) {
	w := newEventsQueueWorld()
	w.gh.QueueRemovals["PR_9"] = eventsCopyMap(eventsQueue)
	assertIn(t, "#9: back in the merge queue (taken out: CONFLICT)", w.housekeeping())
	assertEqual(t, w.gh.AutoMergeHeads["PR_9"], "s1")
	w.gh.DisableAutoMerge("PR_9") // queued, and out again on the same `main`
	assertNotIn(t, "#9: back", strings.Join(w.housekeeping(), " "))
	w.gh.Patch.BranchSHA = func(branch string) string { return "main-2" } // `main` moved on
	w.housekeeping()
	assertIn(t, "PR_9", w.gh.AutoMerge)
}

func TestEvents_APersonWhoTookItOutOrAHeadNeverClearedKeepsItOut(t *testing.T) {
	w := newEventsQueueWorld()
	w.gh.QueueRemovals["PR_9"] = map[string]string{"actor": "jgoetzmann", "type": "User", "reason": "MANUAL"}
	w.housekeeping()
	w.gh.QueueRemovals["PR_9"] = eventsCopyMap(eventsQueue)
	flowUpdateItem(w.ctx, 9, pyjson.New("cleared", pyjson.New("sha", "s0")))
	w.housekeeping()
	assertEqual(t, w.gh.AutoMerge, map[string]string{})
}

func TestEvents_ARevisionOrAStopTakesAQueuedPullRequestOut(t *testing.T) {
	w := newEventsQueueWorld()
	for _, body := range []string{"/harness revise name it better", "/harness stop"} {
		w.gh.MergeQueue = map[string]bool{"PR_9": true} // queued: REST `auto_merge` reads null
		events.Handle(w.ctx, "issue_comment", eventsCommentEvent(9, body, eventsCommentOpts{PR: true}))
		assertEqual(t, w.gh.MergeQueue, map[string]bool{}, body)
	}
}

// StatusTests: `/harness status` says which subscriptions are running, on what, and for how long.

func TestEvents_WhichSubscriptionsAreRunningNow(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	for _, number := range []int{37, 49, 50} {
		gh.AddIssue(number, fakes.Issue{Labels: []string{config.LabelWorking}})
	}
	gh.Runs.Update(pyjson.New("101", pyjson.New("status", "in_progress"), "102", pyjson.New("status", "queued"),
		"103", pyjson.New("status", "in_progress"))) // run 999 has ended (404)
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night})
	ago := func(minutes int) string {
		return clock.ISO(testsupport.Night.Add(-time.Duration(minutes) * time.Minute))
	}
	ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, 37).Update(pyjson.New("provider", "claude-1", "kind", "build", "run_id", "101",
			"started_at", ago(47)))
		state.Item(s, 49).Update(pyjson.New("provider", "muse", "kind", "revise", "run_id", "102",
			"started_at", ago(123)))
		state.Item(s, 50).Update(pyjson.New("provider", "gpt", "kind", "build", "run_id", "999",
			"started_at", ago(300)))
		s.Obj("suggest").Update(pyjson.New("provider", "agy", "run_id", "103", "last_run", ago(0)))
		return nil
	}, "seed")
	text := status.Report(ctx)
	runs := "https://github.com/jgoetzmann/JackiOh/actions/runs"
	assertIn(t, strings.Join([]string{
		"- **Running now** (1 of 8 lanes on GitHub's runners, 7 free; 2 of " +
			fmt.Sprint(ctx.Cfg.Pool.MachineParallel) + " on the night box):",
		"  - `claude-1` (claude, `opus`): building #37, for 47m, [run](" + runs + "/101).",
		"  - `muse` (muse, `muse-spark-1.3-contributor`): revising #49, for 2h 03m, " +
			"[run](" + runs + "/102).",
		"  - `agy` (agy, `gemini-3.8-flash-high`): a suggestion survey, just started, [run](" + runs + "/103).",
	}, "\n"), text)
	// A run that ended holds nothing, and its subscription says why it is or is not free.
	assertNotIn(t, "`gpt` (codex", strings.SplitN(text, "- Subscriptions", 2)[0])
	assertIn(t, "  - `claude-1` (claude: `opus` strong, `sonnet` medium; 21:00–07:00 "+
		"America/Chicago): "+
		"**working on #37**.", text)
	assertNotIn(t, "`gpt` (codex, `gpt-5.6-sol`, any time): **working", text)
	assertIn(t, "- Working on: #37, #49, #50 (no run is going for #50 any more; the next "+
		"plan requeues it).", text)
}

func TestEvents_NothingRunning(t *testing.T) {
	text := status.Report(testsupport.MakeCtx(fakes.NewFakeGitHub(), testsupport.Ctx{At: testsupport.Night}))
	assertIn(t, "- Running now: nothing (8 of 8 lanes free on GitHub's runners, 6 on the "+
		"night box).", text)
}
