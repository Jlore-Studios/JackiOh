// No request is lost: every way in either answers at once or is found again by the sweep.
//
// bot/tests/test_sweep.py.
package tests

import (
	"slices"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/sweep"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/triage"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/trust"
)

const sweepTrust = "jgoetzmann 3 id:95732896\nhelper 2\n"

var (
	sweepSquishy   = pyjson.New("login", "squishy-squooby", "id", 334289562)
	sweepAnHourAgo = clock.ISO(testsupport.Day.Add(-time.Hour))
	sweepJustNow   = clock.ISO(testsupport.Day.Add(-2 * time.Minute))
	sweepLastWeek  = clock.ISO(testsupport.Day.Add(-5 * 24 * time.Hour))
)

// sweepBase is Base's world.
type sweepBase struct {
	t   *testing.T
	gh  *fakes.FakeGitHub
	ctx *hctx.Context
}

func newSweepBase(t *testing.T) *sweepBase {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5, fakes.Issue{Title: "Make the Coin shiny"}).Set("type", pyjson.New("name", "Task")) // typed already
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, TrustText: ptr(sweepTrust)})
	// The repository has every label the bot uses, as it does once the sweep made them.
	for _, name := range config.Labels.Keys() {
		gh.Labels.Set(name, pyjson.New("name", name))
	}
	// The first sweep ever only sets its baseline; these tests start from a later one.
	eventsUpdateState(ctx, "last_sweep", pyjson.New("since", "2026-09-01T00:00:00Z"))
	return &sweepBase{t: t, gh: gh, ctx: ctx}
}

func (w *sweepBase) replies(number int) []string { return w.gh.BotComments(number) }

func (w *sweepBase) lastReply(number int) string {
	w.t.Helper()
	return lastOf(w.t, w.replies(number))
}

// sweepComment is add_comment(number, body, user, association="OWNER", created_at=...).
func sweepComment(gh *fakes.FakeGitHub, number int, body string, user *pyjson.Object, association, createdAt string) *pyjson.Object {
	return gh.AddComment(number, body, fakes.Comment{User: user, Association: association, CreatedAt: createdAt})
}

// sweepAsTask is `["type"] = {"name": "Task"}` on what add_issue returned.
func sweepAsTask(thread *pyjson.Object) { thread.Set("type", pyjson.New("name", "Task")) }

// ---------------------------------------------------------------------------- LostCommentTests

func TestSweep_ACommandNobodyAnsweredIsAnsweredByTheSweep(t *testing.T) {
	w := newSweepBase(t)
	comment := sweepComment(w.gh, 5, "/harness build", fakes.Operator, "", sweepAnHourAgo)
	notes := sweep.Sweep(w.ctx)
	assertEqual(t, w.gh.LabelNames(5), setOf(config.LabelBuild, config.LabelNeedsPlan)) // queued, then planned first
	assertIn(t, "Queued #5", w.lastReply(5))
	assertIn(t, fakes.Reaction{ID: comment.Int("id"), Content: "rocket"}, w.gh.Reacted)
	assertIn(t, "answered a comment on #5", notes[0])
	assertEqual(t, sweep.Sweep(w.ctx), []string{}) // answered once, never twice
}

func TestSweep_ABareApproveVoteIsAnsweredByTheSweep(t *testing.T) {
	// #929: a vote is no command, so the sweep read past it and nobody ever answered it.
	w := newSweepBase(t)
	w.gh.AddIssue(6, fakes.Issue{Title: "A suggestion", Labels: []string{config.LabelSuggestion}})
	sweepComment(w.gh, 6, "approve", fakes.Operator, "", sweepAnHourAgo)
	notes := sweep.Sweep(w.ctx)
	assertEqual(t, w.gh.LabelNames(6), setOf(config.LabelBuild, config.LabelNeedsPlan)) // queued, then planned first
	assertIn(t, "Queued #6", w.lastReply(6))
	assertIn(t, "answered a comment on #6", notes[0])
	assertEqual(t, sweep.Sweep(w.ctx), []string{}) // answered once, never twice
}

func TestSweep_ABareRejectVoteIsAnsweredByTheSweep(t *testing.T) {
	w := newSweepBase(t)
	w.gh.AddIssue(6, fakes.Issue{Title: "A suggestion", Labels: []string{config.LabelSuggestion}})
	sweepComment(w.gh, 6, "reject", fakes.Operator, "", sweepAnHourAgo)
	notes := sweep.Sweep(w.ctx)
	issue := w.gh.GetIssue(6)
	assertEqual(t, []any{dig(t, issue, "state"), dig(t, issue, "state_reason")}, []any{"closed", "not_planned"})
	assertIn(t, "Rejected: I closed #6", w.lastReply(6))
	assertIn(t, "answered a comment on #6", notes[0])
	assertEqual(t, sweep.Sweep(w.ctx), []string{})
}

func TestSweep_WhatTheSweepLeavesAlone(t *testing.T) {
	w := newSweepBase(t)
	sweepComment(w.gh, 5, "/harness build", fakes.Operator, "", sweepJustNow)  // handler may run
	sweepComment(w.gh, 5, "/harness build", fakes.Operator, "", sweepLastWeek) // too old
	sweepComment(w.gh, 5, "/harness halt", fakes.Stranger, "NONE", sweepAnHourAgo)
	sweepComment(w.gh, 5, "a plain comment", fakes.Operator, "", sweepAnHourAgo)
	w.gh.CreateComment(5, "/harness build") // the bot's own words
	answered := sweepComment(w.gh, 5, "/harness status", fakes.Operator, "", sweepAnHourAgo)
	w.gh.React(answered.Int("id"), "rocket", false)
	before := len(w.replies(5))
	assertEqual(t, sweep.Sweep(w.ctx), []string{})
	assertEqual(t, len(w.replies(5)), before)
	assertFalse(t, dig(t, w.ctx.Store.Load(), "halted"))
}

func TestSweep_TheOtherBotsWordsAreNeverReportedAsAnswered(t *testing.T) {
	// #504: a comment from Squishy's account that names the night bot is no command of its
	// own, so every sweep left it unanswered, and every sweep said it had answered it.
	w := newSweepBase(t)
	w.ctx.Trust = trust.TrustParse(sweepTrust + "squishy-squooby 3 id:334289562\n")
	for _, body := range []string{"@jgoetzmann\n\n#5 is the night bot's (@jgoetzmann-bot): it carries the " +
		"night bot's labels, so I leave it alone.",
		"/harness revised. Just pushed the rust patch in, see if this is still needed"} {
		sweepComment(w.gh, 5, body, sweepSquishy, "COLLABORATOR", sweepAnHourAgo)
	}
	assertEqual(t, sweep.Sweep(w.ctx), []string{})
	assertEqual(t, sweep.Sweep(w.ctx), []string{})
	assertEqual(t, w.replies(5), []string{})
}

func TestSweep_ALineCommentOnADiffIsAnsweredToo(t *testing.T) {
	w := newSweepBase(t)
	w.gh.AddPull(9, "bot/issue-5", fakes.Pull{Labels: []string{config.LabelPR}})
	w.gh.AddReviewComment(9, "@jgoetzmann-bot rename this", fakes.Comment{User: fakes.Operator, CreatedAt: sweepAnHourAgo})
	sweep.Sweep(w.ctx)
	assertIn(t, config.LabelRevise, w.gh.LabelNames(9))
}

// ---------------------------------------------------------------------------- HandlerTests

func TestSweep_ALineCommentEventQueuesARevisionAndReactsOnTheRightEndpoint(t *testing.T) {
	w := newSweepBase(t)
	w.gh.AddPull(9, "bot/issue-5", fakes.Pull{Labels: []string{config.LabelPR}})
	payload := pyjson.New("action", "created", "sender", fakes.Operator, "pull_request", w.gh.GetPull(9),
		"comment", pyjson.New("id", 77, "body", "@jgoetzmann-bot use a constant here",
			"user", fakes.Operator, "author_association", "OWNER"))
	events.Handle(w.ctx, "pull_request_review_comment", payload)
	assertIn(t, config.LabelRevise, w.gh.LabelNames(9))
	assertIn(t, fakes.Reaction{ID: 77, Content: "rocket"}, w.gh.Reacted)
}

func TestSweep_AnEditThatAddsACommandIsHeardOnce(t *testing.T) {
	w := newSweepBase(t)
	payload := pyjson.New("action", "edited", "sender", fakes.Operator, "issue", pyjson.New("number", 5),
		"comment", pyjson.New("id", 88, "body", "on second thought\n/harness build",
			"user", fakes.Operator, "author_association", "OWNER"))
	events.Handle(w.ctx, "issue_comment", payload)
	assertEqual(t, w.gh.LabelNames(5), setOf(config.LabelBuild))
	count := len(w.replies(5))
	events.Handle(w.ctx, "issue_comment", payload) // edited again: already answered
	assertEqual(t, len(w.replies(5)), count)
}

func TestSweep_NamingTheBotMidSentenceGetsAnAnswerNotSilence(t *testing.T) {
	w := newSweepBase(t)
	payload := pyjson.New("action", "created", "sender", fakes.Operator, "issue", pyjson.New("number", 5),
		"comment", pyjson.New("id", 90, "body", "thanks, @jgoetzmann-bot could you also add sound?",
			"user", fakes.Operator, "author_association", "OWNER"))
	events.Handle(w.ctx, "issue_comment", payload)
	assertIn(t, "I saw my name", w.lastReply(5))
	assertIn(t, fakes.Reaction{ID: 90, Content: "rocket"}, w.gh.Reacted)
	assertEqual(t, w.gh.LabelNames(5), setOf()) // a hint, never a guessed build
}

func TestSweep_AFailingCommandIsAnsweredAndItsNeighboursStillRun(t *testing.T) {
	w := newSweepBase(t)
	// The Python replaces `ctx.store.update` with one that raises StateConflict for every write
	// but a claim. StateStore.Update is a method, so the same comes from below it: every write
	// whose message is not a claim's loses its compare-and-set until the store gives up with
	// StateConflict (without its backing off).
	w.ctx.Store.Sleep = func(float64) {}
	w.gh.Patch.PutFile = func(path, text, branch, sha, message string) {
		if strings.HasPrefix(message, "claim") {
			w.gh.Own().PutFile(path, text, branch, sha, message)
			return
		}
		herr.Raise(&herr.GitHubError{Msg: "the state kept changing", Status: 409})
	}
	payload := pyjson.New("action", "created", "sender", fakes.Operator, "issue", pyjson.New("number", 5),
		"comment", pyjson.New("id", 91, "body", "/harness halt\n/harness status",
			"user", fakes.Operator, "author_association", "OWNER"))
	events.Handle(w.ctx, "issue_comment", payload)
	reply := w.lastReply(5)
	assertIn(t, "That failed (StateConflict", reply)
	assertIn(t, "Night bot status", reply)
	assertIn(t, fakes.Reaction{ID: 91, Content: "rocket"}, w.gh.Reacted)
}

func TestSweep_AForceThatCannotStartARunStillQueuesIt(t *testing.T) {
	w := newSweepBase(t)
	// The Python replaces `ctx.dispatch`; hctx.Context.Dispatch is a method, and all it does is
	// ask the Actions client to dispatch, so the refusal comes from there.
	w.gh.Patch.DispatchWorkflow = func(workflow, ref string, inputs map[string]string) {
		herr.Raise(&herr.GitHubError{Msg: "dispatch refused", Status: 422})
	}
	payload := pyjson.New("action", "created", "sender", fakes.Operator, "issue", pyjson.New("number", 5),
		"comment", pyjson.New("id", 92, "body", "/harness build --force", "user", fakes.Operator,
			"author_association", "OWNER"))
	events.Handle(w.ctx, "issue_comment", payload)
	assertEqual(t, w.gh.LabelNames(5), setOf(config.LabelBuild))
	assertTrue(t, dig(t, w.ctx.Store.Load(), "items", "5", "forced"))
	assertIn(t, "the next hourly run starts it", w.lastReply(5))
}

// ---------------------------------------------------------------------------- LostEventTests

func TestSweep_AnAssignmentNothingAnsweredIsQueued(t *testing.T) {
	w := newSweepBase(t)
	w.gh.Threads.Item(5).Set("assignees", []any{fakes.Bot})
	notes := sweep.Sweep(w.ctx)
	assertEqual(t, w.gh.LabelNames(5), setOf(config.LabelBuild, config.LabelNeedsPlan)) // queued, then planned first
	assertIn(t, "assigned here but never queued", w.lastReply(5))
	assertIn(t, "from an assignment", notes[0])
	assertEqual(t, sweep.Sweep(w.ctx), []string{})
}

func TestSweep_AnAssignmentAlreadyHandledOrStoppedIsLeftAlone(t *testing.T) {
	w := newSweepBase(t)
	w.gh.Threads.Item(5).Set("assignees", []any{fakes.Bot})
	flowUpdateItem(w.ctx, 5, pyjson.New("queued_at", sweepAnHourAgo))
	sweepAsTask(w.gh.AddIssue(6))
	w.gh.Threads.Item(6).Set("assignees", []any{fakes.Bot})
	flowUpdateItem(w.ctx, 6, pyjson.New("stop_requested", true))
	assertEqual(t, sweep.Sweep(w.ctx), []string{})
}

func TestSweep_AReviewAskingForChangesThatNothingAnswered(t *testing.T) {
	w := newSweepBase(t)
	w.gh.AddPull(9, "bot/issue-5", fakes.Pull{Labels: []string{config.LabelPR}})
	flowUpdateItem(w.ctx, 9, pyjson.New("feedback_since", sweepLastWeek))
	w.gh.Reviews.Set(9, []*pyjson.Object{pyjson.New("id", 61, "state", "CHANGES_REQUESTED", "body", "No.", "user", fakes.Operator,
		"author_association", "OWNER", "submitted_at", sweepAnHourAgo)})
	sweep.Sweep(w.ctx)
	assertIn(t, config.LabelRevise, w.gh.LabelNames(9))
	assertIn(t, "Queued a revision of #9", w.lastReply(9))
}

func TestSweep_AReviewTheBotAlreadyAnsweredIsNotRedone(t *testing.T) {
	w := newSweepBase(t)
	w.gh.AddPull(9, "bot/issue-5", fakes.Pull{Labels: []string{config.LabelPR}})
	review := pyjson.New("id", 62, "state", "CHANGES_REQUESTED", "body", "No.", "user", fakes.Operator,
		"author_association", "OWNER", "submitted_at", sweepAnHourAgo)
	w.gh.Reviews.Set(9, []*pyjson.Object{review})
	events.Handle(w.ctx, "pull_request_review", pyjson.New("action", "submitted", "sender", fakes.Operator,
		"review", review,
		"pull_request", w.gh.GetPull(9)))
	w.gh.Threads.Item(9).Set("labels", eventsLabels(config.LabelPR, "difficulty:medium")) // landed
	assertEqual(t, sweep.Sweep(w.ctx), []string{})
}

// A failed CI run nothing answered gets its CI fix from the sweep, with no re-run first (#2948).
func TestSweep_AFailedCIRunNothingAnsweredIsFixed(t *testing.T) {
	w := newSweepBase(t)
	w.gh.AddPull(9, "bot/issue-5", fakes.Pull{Labels: []string{config.LabelPR}, SHA: "s1"})
	w.gh.Runs.Set("99", pyjson.New("id", 99, "name", "CI", "head_sha", "s1", "head_branch", "bot/issue-5",
		"status", "completed", "conclusion", "failure", "run_attempt", 1,
		"created_at", sweepAnHourAgo, "html_url", "u", "pull_requests", []any{}))
	sweep.Sweep(w.ctx)
	assertEqual(t, w.gh.Reruns, []any{})
	assertIn(t, config.LabelRevise, w.gh.LabelNames(9))
	assertEqual(t, sweep.Sweep(w.ctx), []string{}) // queued: nothing more to do
}

func TestSweep_ABlockedOrGreenPRIsLeftAlone(t *testing.T) {
	w := newSweepBase(t)
	w.gh.AddPull(9, "bot/issue-5", fakes.Pull{Labels: []string{config.LabelPR, config.LabelBlocked, "difficulty:medium"},
		SHA: "s1"})
	w.gh.AddPull(10, "bot/issue-6", fakes.Pull{Labels: []string{config.LabelPR, "difficulty:medium"}, SHA: "s2"})
	w.gh.Runs.Set("1", pyjson.New("id", 1, "name", "CI", "head_sha", "s1", "status", "completed",
		"conclusion", "failure", "created_at", sweepAnHourAgo))
	w.gh.Runs.Set("2", pyjson.New("id", 2, "name", "CI", "head_sha", "s2", "status", "completed",
		"conclusion", "success", "created_at", sweepAnHourAgo))
	assertEqual(t, sweep.Sweep(w.ctx), []string{})
}

// A red pull request people hold (`human required`, `human`, `tracking`) gets no CI fix and no
// comment, from the event or from the sweep that replays it every tick: #2964 got the same "CI
// failed … I leave it alone" comment about forty times (#3073).
func TestSweep_ARedPRPeopleHoldGetsNoComment(t *testing.T) {
	for _, label := range []string{config.LabelHumanRequired, config.LabelHuman, config.LabelTracking} {
		w := newSweepBase(t)
		w.gh.AddPull(9, "bot/issue-5", fakes.Pull{Labels: []string{config.LabelPR, label, "difficulty:medium"},
			SHA: "s1"})
		run := pyjson.New("id", 99, "name", "CI", "head_sha", "s1", "head_branch", "bot/issue-5",
			"status", "completed", "conclusion", "failure", "run_attempt", 1,
			"created_at", sweepAnHourAgo, "html_url", "u", "pull_requests", []any{pyjson.New("number", 9)})
		w.gh.Runs.Set("99", run)
		assertEqual(t, events.Handle(w.ctx, "workflow_run", pyjson.New("action", "completed", "workflow_run", run)),
			[]string{"#9: kept by `" + label + "` for people; left alone"})
		for range 3 {
			assertEqual(t, sweep.Sweep(w.ctx), []string{})
		}
		assertEqual(t, w.replies(9), []string(nil))
		assertNotIn(t, config.LabelRevise, w.gh.LabelNames(9))
		assertEqual(t, w.gh.Reruns, []any{})
	}
}

func TestSweep_TheSweepRecordsWhenItRan(t *testing.T) {
	w := newSweepBase(t)
	sweep.Sweep(w.ctx)
	assertIn(t, "at", dig(t, w.ctx.Store.Load(), "last_sweep"))
}

func TestSweep_TheFirstSweepReplaysNothing(t *testing.T) {
	_ = newSweepBase(t)
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5)
	sweepComment(gh, 5, "/harness build", fakes.Operator, "", sweepAnHourAgo)
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, TrustText: ptr(sweepTrust)})
	assertIn(t, "first sweep", sweep.Sweep(ctx)[0])
	assertEqual(t, gh.LabelNames(5), setOf())
	testsupport.ClockOf(ctx).At = testsupport.Day.Add(30 * time.Minute)
	sweepComment(gh, 5, "/harness build", fakes.Operator, "", clock.ISO(testsupport.Day.Add(5*time.Minute)))
	sweep.Sweep(ctx)
	assertEqual(t, gh.LabelNames(5), setOf(config.LabelBuild, config.LabelNeedsPlan)) // queued, then planned first
}

func TestSweep_AnEditedCommentIsLeftToItsEditEvent(t *testing.T) {
	w := newSweepBase(t)
	comment := sweepComment(w.gh, 5, "/harness halt", fakes.Operator, "", sweepAnHourAgo)
	comment.Set("updated_at", clock.ISO(testsupport.Day.Add(-30*time.Minute)))
	assertEqual(t, sweep.Sweep(w.ctx), []string{})
	assertFalse(t, dig(t, w.ctx.Store.Load(), "halted"))
}

func TestSweep_OneBadCommentDoesNotStopTheRest(t *testing.T) {
	w := newSweepBase(t)
	w.gh.Comments.Set(404, []*pyjson.Object{pyjson.New("id", 1, "body", "/harness build", "user", fakes.Operator,
		"author_association", "OWNER", "created_at", sweepAnHourAgo,
		"issue_url", "https://api.github.com/repos/x/y/issues/404")})
	sweepComment(w.gh, 5, "/harness build", fakes.Operator, "", sweepAnHourAgo)
	notes := sweep.Sweep(w.ctx)
	assertEqual(t, w.gh.LabelNames(5), setOf(config.LabelBuild, config.LabelNeedsPlan)) // queued, then planned first
	assertTrue(t, slices.ContainsFunc(notes, func(n string) bool { return strings.Contains(n, "skipped one") }))
}

func TestSweep_AReviewBodyCommandNobodyAnswered(t *testing.T) {
	w := newSweepBase(t)
	w.gh.AddPull(9, "feature/x")
	w.gh.Reviews.Set(9, []*pyjson.Object{pyjson.New("id", 55, "state", "COMMENTED", "body", "@jgoetzmann-bot fix the lint",
		"user", fakes.Operator, "author_association", "OWNER",
		"submitted_at", sweepAnHourAgo)})
	sweep.Sweep(w.ctx)
	assertIn(t, config.LabelRevise, w.gh.LabelNames(9))
	assertEqual(t, sweep.Sweep(w.ctx), []string{})
}

// ---------------------------------------------------------------------------- NightRunTests
//
// GitHub drops scheduled `bot-night` runs; the sweep starts one when work waits and none is
// going, so a dropped hour costs ten minutes.

func (w *sweepBase) atNight() {
	testsupport.ClockOf(w.ctx).At = testsupport.Night
	eventsUpdateState(w.ctx, "last_sweep", pyjson.New("since", clock.ISO(testsupport.Night.Add(-24*time.Hour))))
}

func TestSweep_ADueSurveyCountsAsWorkToo(t *testing.T) {
	// #797: the first tick with a free lane and nothing to start starts the clock; once it has
	// run 30 minutes, the survey is work and a run starts for it.
	w := newSweepBase(t)
	w.atNight()
	sweep.Sweep(w.ctx)
	assertEqual(t, dig(t, w.ctx.Store.Load(), "suggest", "idle_since"), clock.ISO(testsupport.Night))
	assertEqual(t, len(w.gh.Dispatches), 0)
	testsupport.ClockOf(w.ctx).At = testsupport.Night.Add(29 * time.Minute)
	sweep.Sweep(w.ctx)
	assertEqual(t, len(w.gh.Dispatches), 0)
	testsupport.ClockOf(w.ctx).At = testsupport.Night.Add(30 * time.Minute)
	sweep.Sweep(w.ctx)
	assertEqual(t, len(w.gh.Dispatches), 1)
}

func TestSweep_QueuedWorkForAFreeLaneStopsTheSurveyClock(t *testing.T) {
	w := newSweepBase(t)
	w.atNight()
	sweep.Sweep(w.ctx)
	w.gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	sweep.Sweep(w.ctx)
	assertIsNone(t, dig(t, w.ctx.Store.Load(), "suggest", "idle_since"))
}

func TestSweep_QueuedWorkInTheWindowWithNoRunGoingStartsOne(t *testing.T) {
	w := newSweepBase(t)
	w.atNight()
	w.gh.Threads.Item(5).Set("labels", eventsLabels(config.LabelBuild))
	notes := sweep.Sweep(w.ctx)
	assertEqual(t, len(w.gh.Dispatches), 1)
	assertEqual(t, w.gh.Dispatches[0].Workflow, "bot-night.yml")
	assertIn(t, "started a night run", lastOf(t, notes))
}

func TestSweep_ARunAlreadyQueuedOrGoingIsLeftToIt(t *testing.T) {
	w := newSweepBase(t)
	w.atNight()
	w.gh.Threads.Item(5).Set("labels", eventsLabels(config.LabelBuild))
	for _, status := range []string{"queued", "in_progress", "waiting", "pending"} {
		w.gh.Runs = pyjson.New("1", pyjson.New("status", "completed"), "2", pyjson.New("status", status))
		sweep.Sweep(w.ctx)
		assertEqual(t, w.gh.Dispatches, []fakes.Dispatch{}, status)
	}
}

func TestSweep_NoRunWithoutWorkOrOutsideTheWindow(t *testing.T) {
	w := newSweepBase(t)
	w.atNight()
	w.ctx.Store.Update(func(s *pyjson.Object) any {
		s.Obj("suggest").Update(pyjson.New("last_run", clock.ISO(testsupport.Night.Add(-time.Hour))))
		return nil
	}, "state")
	sweep.Sweep(w.ctx) // nothing queued, and no survey due
	testsupport.ClockOf(w.ctx).At = testsupport.Day
	w.gh.Threads.Item(5).Set("labels", eventsLabels(config.LabelBuild))
	sweep.Sweep(w.ctx) // queued, but the window is closed
	assertEqual(t, w.gh.Dispatches, []fakes.Dispatch{})
}

func TestSweep_ForcedWorkStartsARunOutsideTheWindowButNeverWhileHalted(t *testing.T) {
	w := newSweepBase(t)
	w.gh.Threads.Item(5).Set("labels", eventsLabels(config.LabelBuild))
	flowUpdateItem(w.ctx, 5, pyjson.New("forced", true))
	eventsUpdateState(w.ctx, "halted", true)
	sweep.Sweep(w.ctx)
	assertEqual(t, w.gh.Dispatches, []fakes.Dispatch{})
	eventsUpdateState(w.ctx, "halted", false)
	sweep.Sweep(w.ctx)
	assertEqual(t, len(w.gh.Dispatches), 1)
}

func TestSweep_ARunThatCannotBeStartedIsNotedAndTriedAgainNextSweep(t *testing.T) {
	w := newSweepBase(t)
	w.atNight()
	w.gh.Threads.Item(5).Set("labels", eventsLabels(config.LabelBuild))
	w.gh.Patch.DispatchWorkflow = func(workflow, ref string, inputs map[string]string) {
		herr.Raise(&herr.GitHubError{Msg: "workflow dispatch refused", Status: 422})
	}
	notes := sweep.Sweep(w.ctx)
	assertIn(t, "night_run: could not finish", lastOf(t, notes))
}

// ---------------------------------------------------------------------------- IssueTypeTests
//
// Every open issue gets a type: triage types the ones people open as they open, and the sweep
// the rest, at once.

func (w *sweepBase) typeOf(number int) any { return w.gh.Threads.Item(number).Get("type") }

func TestSweep_EveryUntypedIssueIsTypedAtOnce(t *testing.T) {
	w := newSweepBase(t)
	alert := w.gh.AddIssue(20, fakes.Issue{Title: "CI: a job ran over 7 minutes",
		User: pyjson.New("login", "github-actions[bot]", "id", 41898282)})
	alert.Set("created_at", sweepJustNow)
	fresh := w.gh.AddIssue(21, fakes.Issue{Title: "Patch v0.2.X: the Almanac crashes on a phone"})
	fresh.Set("created_at", sweepJustNow) // one triage missed: the same rules type it
	w.gh.AddIssue(22, fakes.Issue{Title: "Patch v0.2.X: a public card and player stats page"})
	w.gh.AddIssue(23, fakes.Issue{Title: "Patch v0.2.X: Animated pass on Field Spells"}).Update(pyjson.New("type", pyjson.New("name", "Task")))
	w.gh.AddIssue(24, fakes.Issue{Title: "Easter egg: Glitch", Labels: []string{"bot:pr"}}).Update(pyjson.New("pull_request", pyjson.New()))
	notes := sweep.Sweep(w.ctx)
	assertEqual(t, w.typeOf(20), "Task")
	assertEqual(t, w.typeOf(21), "Bug")
	assertEqual(t, w.typeOf(22), "Feature")
	assertEqual(t, w.typeOf(23), pyjson.New("name", "Task")) // a type someone set stays
	truthy := map[string]bool{}
	w.gh.Threads.Item(24).Items(func(k string, v any) bool {
		if pyjson.Truthy(v) {
			truthy[k] = true
		}
		return true
	})
	assertNotIn(t, "type", truthy) // a PR has none
	assertIn(t, "#20: typed it Task", notes)
	w.gh.AddIssue(25, fakes.Issue{Title: "Make the Coin shiny"})
	sweep.Sweep(w.ctx)
	assertEqual(t, w.typeOf(25), "Task") // nothing else fits
}

func TestSweep_TheFallbackRules(t *testing.T) {
	_ = newSweepBase(t)
	types := triage.DefaultIssueTypes
	kind := func(title string, labels ...string) string {
		return triage.FallbackType(pyjson.New("title", title, "labels", eventsLabels(labels...)), types)
	}
	assertEqual(t, kind("Patch v0.2.X: Rendering lag with large Radiant hands"), "Bug")
	assertEqual(t, kind("CI: the Postgres test runners wait for the real server"), "Task")
	assertEqual(t, kind("Night bot: a help command"), "Task")
	assertEqual(t, kind("Patch v0.2.X: Music improvements"), "Task")
	assertEqual(t, kind("Ranked: a new leaderboard page"), "Feature")
	assertEqual(t, kind("Patch v0.2.X: a settings page", "architecture"), "Task")
	assertEqual(t, triage.FallbackType(pyjson.New("title", "x"), map[string]string{"Chore": "upkeep"}), "")
}

// ---------------------------------------------------------------------------- LabelTests
//
// The sweep creates the labels the bot uses that the repository lacks (the method labels, #307,
// shipped in #328 without existing, waiting for a person to run `python3 -m harness setup`, so no
// issue could be triaged) and deletes the ones it retired (#664).

func TestSweep_MissingLabelsAreCreatedOncePerChangeToTheList(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.Labels = pyjson.New()
	for _, name := range config.Labels.Keys() {
		if name != "human" && name != "bot:build" {
			gh.Labels.Set(name, pyjson.New("name", name))
		}
	}
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day})
	notes := sweep.Labels(ctx, testsupport.Day)
	assertEqual(t, sortedOf(notes), []string{"created the label `bot:build`",
		"created the label `human`"})
	assertEqual(t, dig(t, gh.Labels, "human", "color"), pyjson.List(config.Labels.Get("human"))[0])
	// The list has not changed since: the next tick reads nothing.
	calls := [][]string{}
	gh.Patch.EnsureLabel = func(name, color, description string) bool {
		calls = append(calls, []string{name, color, description})
		return false
	}
	gh.Patch.DeleteLabel = func(name string) bool {
		calls = append(calls, []string{name})
		return false
	}
	assertEqual(t, sweep.Labels(ctx, testsupport.Day), []string{})
	assertEqual(t, calls, [][]string{})
}

func TestSweep_ARetiredLabelIsDeletedFromEveryThread(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.Labels = pyjson.New()
	for _, name := range append(config.Labels.Keys(), "method:use-bot", "large patch") {
		gh.Labels.Set(name, pyjson.New("name", name))
	}
	gh.AddIssue(647, fakes.Issue{Labels: []string{"method:use-bot", "priority:low"}})
	notes := sweep.Labels(testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day}), testsupport.Day)
	assertEqual(t, sortedOf(notes), []string{"deleted the retired label `large patch`",
		"deleted the retired label `method:use-bot`"})
	assertNotIn(t, "method:use-bot", gh.Labels)
	assertEqual(t, gh.LabelNames(647), setOf("priority:low"))
}

// `difficulty:easy` is retired (#2748): before the sweep deletes it, every open thread that
// carries it gets `difficulty:medium` in its place, so a person's easy label is a person's medium
// label and a bot's rating stays the bot's. A closed thread loses it with the label. One thread
// GitHub refuses keeps the label in the repository, and the next tick tries again.
func TestSweep_ARetiredEasyLabelBecomesMediumBeforeItIsDeleted(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.Labels = pyjson.New()
	for _, name := range append(config.Labels.Keys(), "difficulty:easy") {
		gh.Labels.Set(name, pyjson.New("name", name))
	}
	gh.AddIssue(40, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:easy"}})            // a person's
	gh.AddIssue(41, fakes.Issue{Labels: []string{"bot:planned", "difficulty:easy"}})                // the bot's rating
	gh.AddIssue(42, fakes.Issue{Labels: []string{"human", "difficulty:easy", "difficulty:medium"}}) // both
	gh.AddIssue(43, fakes.Issue{Labels: []string{"difficulty:easy"}}).Set("state", "closed")
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day})
	flowUpdateItem(ctx, 41, pyjson.New("difficulty_by", pyjson.New("difficulty", "easy", "provider", "muse")))
	refused := true
	gh.Patch.RemoveLabel = func(number int, name string) {
		if number == 42 && refused {
			herr.Raise(&herr.GitHubError{Msg: "refused", Status: 502})
		}
		gh.Own().RemoveLabel(number, name)
	}
	notes := sweep.Labels(ctx, testsupport.Day)
	assertIn(t, "#40: `difficulty:easy` is retired, so `difficulty:medium`", notes)
	assertIn(t, "difficulty:easy", gh.Labels) // #42 still carries it
	refused = false
	notes = sweep.Labels(ctx, testsupport.Day)
	assertIn(t, "deleted the retired label `difficulty:easy`", notes)
	assertNotIn(t, "difficulty:easy", gh.Labels)
	assertEqual(t, gh.LabelNames(40), setOf(config.LabelBuild, "difficulty:medium"))
	assertEqual(t, gh.LabelNames(41), setOf("bot:planned", "difficulty:medium"))
	assertEqual(t, gh.LabelNames(42), setOf("human", "difficulty:medium"))
	assertEqual(t, gh.LabelNames(43), setOf())
	record := func(n int) *pyjson.Object { return ctx.Store.Load().Obj("items").Obj(strconv.Itoa(n)) }
	assertEqual(t, queue.RatingSource(gh.LabelNames(40), record(40)), "person")
	assertEqual(t, queue.RatingSource(gh.LabelNames(41), record(41)), "bot")
	assertEqual(t, queue.DifficultyOf(gh.LabelNames(40), ""), "medium")
}

func TestSweep_EverySweepRunsIt(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day})
	eventsUpdateState(ctx, "last_sweep", pyjson.New("at", clock.ISO(testsupport.Day), "since", clock.ISO(testsupport.Day)))
	notes := sweep.Sweep(ctx)
	assertIn(t, "created the label `human`", notes)
	assertIn(t, "bot:build", gh.Labels)
}

// GitHub refuses a longer one, so the label could never be created (#307's two).
func TestSweep_EveryDescriptionFitsGithubsLimit(t *testing.T) {
	tooLong := map[string]int{}
	config.Labels.Items(func(name string, v any) bool {
		description := pyjson.Str(pyjson.List(v)[1])
		if n := len([]rune(description)); n > config.LabelDescriptionMax {
			tooLong[name] = n
		}
		return true
	})
	assertEqual(t, tooLong, map[string]int{})
}

func TestSweep_ALabelGithubRefusesNeverStopsTheRestAndIsTriedAgain(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	refusing := func(name, color, description string) bool {
		if name == "human" {
			herr.Raise(&herr.GitHubError{Msg: "422 description is too long", Status: 422})
		}
		return gh.Own().EnsureLabel(name, color, description)
	}
	gh.Patch.EnsureLabel = refusing
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day})
	notes := sweep.Labels(ctx, testsupport.Day)
	assertIn(t, "bot:build", gh.Labels)
	assertTrue(t, slices.ContainsFunc(notes, func(n string) bool { return strings.Contains(n, "could not create the label `human`") }))
	assertNotIn(t, "labels_synced", ctx.Store.Load()) // so the next tick tries again
	gh.Patch.EnsureLabel = nil
	sweep.Labels(ctx, testsupport.Day)
	assertIn(t, "human", gh.Labels)
	assertIn(t, "labels_synced", ctx.Store.Load())
}

// ---------------------------------------------------------------------------- DifficultyLabelTests
//
// A planned issue and a bot pull request never stay without a difficulty (#735).

func (w *sweepBase) difficulties(number int) map[string]bool {
	out := map[string]bool{}
	for n := range w.gh.LabelNames(number) {
		if strings.HasPrefix(n, "difficulty:") {
			out[n] = true
		}
	}
	return out
}

func TestSweep_PlannedIssuesAndBotPullsGetADifficulty(t *testing.T) {
	w := newSweepBase(t)
	w.gh.AddIssue(30, fakes.Issue{Labels: []string{"bot:planned"}})     // planned, no rating line
	w.gh.AddIssue(31, fakes.Issue{Labels: []string{config.LabelBuild}}) // still waiting for its plan
	w.gh.AddIssue(32, fakes.Issue{Labels: []string{"bot:planned", "human"}})
	w.gh.AddIssue(33, fakes.Issue{Labels: []string{"bot:planned", "difficulty:hard"}})
	w.gh.AddPull(34, "bot/issue-33", fakes.Pull{Labels: []string{config.LabelPR}}) // its issue says hard
	w.gh.AddPull(35, "bot/issue-30", fakes.Pull{Labels: []string{config.LabelPR}}) // its issue had none
	w.gh.AddPull(36, "feature/x")                                                  // not the bot's
	notes := sweep.Sweep(w.ctx)
	assertEqual(t, w.difficulties(30), setOf("difficulty:medium"))
	assertEqual(t, w.difficulties(31), setOf()) // the planner rates it
	assertEqual(t, w.difficulties(32), setOf())
	assertEqual(t, w.difficulties(33), setOf("difficulty:hard"))
	assertEqual(t, w.difficulties(34), setOf("difficulty:hard"))
	assertEqual(t, w.difficulties(35), setOf("difficulty:medium"))
	assertEqual(t, w.difficulties(36), setOf())
	assertIn(t, "#30: planned with no difficulty, so `difficulty:medium`", notes)
	// It is the bot's own rating, so a later planner may raise it.
	record := dig(t, w.ctx.Store.Load(), "items", "30")
	assertEqual(t, dig(t, record, "difficulty_by", "difficulty"), "medium")
}

// `bot:working` is retired (#2748): before the sweep deletes it, every thread that carries it,
// closed ones too (a run goes on after its thread closes), gets the live label of the run its
// record names, so a thread a run holds stays held: the one its claim kept (`live`), or, for a
// claim from before, what its kind, action, revision's source and fullsend tree say.
func TestSweep_TheRetiredWorkingLabelMovesToItsRunsLiveLabel(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.Labels = pyjson.New()
	for _, name := range append(config.Labels.Keys(), config.LabelWorking) {
		gh.Labels.Set(name, pyjson.New("name", name))
	}
	working := config.LabelWorking
	gh.AddIssue(50, fakes.Issue{Labels: []string{working}})                              // no record: a build
	gh.AddIssue(51, fakes.Issue{Labels: []string{working, config.LabelNeedsPlan}})       // a planning run
	gh.AddPull(52, "bot/issue-9", fakes.Pull{Labels: []string{config.LabelPR, working}}) // a CI fix
	gh.AddPull(53, "bot/issue-8", fakes.Pull{Labels: []string{config.LabelPR, working}}) // a review run
	gh.AddPull(54, "bot/issue-7", fakes.Pull{Labels: []string{config.LabelPR, working}}) // a revision
	gh.AddIssue(55, fakes.Issue{Labels: []string{working, "bot:fullsend"}})              // its reconcile
	gh.AddIssue(56, fakes.Issue{Labels: []string{working, "bot:fullsend"}})              // its split
	gh.AddIssue(57, fakes.Issue{Labels: []string{working}, State: "closed"})             // closed, still going
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day})
	for number, record := range map[int]*pyjson.Object{
		51: pyjson.New("kind", "build", "action", "plan"),
		52: pyjson.New("kind", "revise", "action", "revise", "source", "ci"),
		53: pyjson.New("kind", "review", "action", "review"),
		54: pyjson.New("kind", "revise", "action", "revise", "source", "request"),
		55: pyjson.New("kind", "build", "action", "build", "tree", pyjson.New("mode", "fullsend")),
		56: pyjson.New("kind", "build", "action", "build"),
		57: pyjson.New("kind", "build", "action", "build", "live", "building"),
	} {
		flowUpdateItem(ctx, number, record)
	}
	assertEqual(t, len(plan.WorkingThreads(ctx)), 8) // held before the move
	notes := sweep.Labels(ctx, testsupport.Day)
	assertIn(t, "#52: `bot:working` is retired, so `bot:fixing-ci`", notes)
	assertIn(t, "deleted the retired label `bot:working`", notes)
	assertNotIn(t, working, gh.Labels)
	for number, want := range map[int][]string{
		50: {config.LabelBuilding}, 51: {config.LabelPlanning, config.LabelNeedsPlan},
		52: {config.LabelPR, config.LabelFixingCI}, 53: {config.LabelPR, config.LabelReviewing},
		54: {config.LabelPR, config.LabelRevising}, 55: {config.LabelReconciling, "bot:fullsend"},
		56: {config.LabelSplitting, "bot:fullsend"}, 57: {config.LabelBuilding},
	} {
		assertEqual(t, gh.LabelNames(number), setOf(want...), number)
	}
	// Every one is held still, by its live label now.
	assertEqual(t, len(plan.WorkingThreads(ctx)), 8)
}

// `bot:merging` (#2748) comes off a bot pull request merging no more: closed, or open with
// auto-merge off and out of `main`'s merge queue. One with auto-merge on, or in the queue (whose
// REST `auto_merge` reads null), keeps it.
func TestSweep_AStaleMergingLabelComesOff(t *testing.T) {
	w := newSweepBase(t)
	for _, n := range []int{60, 61, 62, 63} {
		w.gh.AddPull(n, "bot/issue-"+strconv.Itoa(n-30), fakes.Pull{Labels: []string{config.LabelPR, config.LabelMerging}})
	}
	w.gh.Threads.Item(60).Set("auto_merge", pyjson.New("merge_method", "squash")) // merging
	w.gh.MergeQueue = map[string]bool{"PR_61": true}                              // queued
	w.gh.Threads.Item(63).Set("state", "closed")                                  // merged meanwhile
	notes := sweep.Merging(w.ctx, testsupport.Day)
	assertEqual(t, sortedOf(notes), []string{
		"#62: auto-merge is off and it is out of the merge queue, so `bot:merging` is off",
		"#63: closed, so `bot:merging` is off"})
	assertEqual(t, w.gh.LabelNames(60), setOf(config.LabelPR, config.LabelMerging))
	assertEqual(t, w.gh.LabelNames(61), setOf(config.LabelPR, config.LabelMerging))
	assertEqual(t, w.gh.LabelNames(62), setOf(config.LabelPR))
	assertEqual(t, w.gh.LabelNames(63), setOf(config.LabelPR))
	assertEqual(t, sweep.Merging(w.ctx, testsupport.Day), []string{})
}

// The production countdown (the open issue labelled `production merge`) gets `tracking` (#2748),
// and nothing else about it changes; the pull requests that carry `production merge` are left
// alone, and so is a countdown that has it.
func TestSweep_TheProductionCountdownIsTracking(t *testing.T) {
	w := newSweepBase(t)
	countdown := w.gh.AddIssue(70, fakes.Issue{Title: "Merging to production in 3 hours",
		Labels: []string{"production merge", "human"}})
	w.gh.AddPull(71, "promote/x", fakes.Pull{Labels: []string{"production merge", "human"}})
	body := countdown.Str("body")
	notes := sweep.Sweep(w.ctx)
	assertIn(t, "#70: the production countdown, so `tracking`", notes)
	assertEqual(t, w.gh.LabelNames(70), setOf("production merge", "human", config.LabelTracking))
	assertEqual(t, w.gh.LabelNames(71), setOf("production merge", "human"))
	assertEqual(t, []any{countdown.Str("title"), countdown.Str("body")}, []any{"Merging to production in 3 hours", body})
	assertEqual(t, w.replies(70), []string{})
	assertNotIn(t, "#70: the production countdown, so `tracking`", sweep.Sweep(w.ctx))
}
