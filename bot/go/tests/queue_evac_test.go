package tests

// Evacuating the merge queue (bot/go/internal/events/evac.go, bot/README.md "Evacuating the merge
// queue"): a pull request whose current queue build failed a required job of its own leaves the
// queue at the next look, not when the build's slowest job ends; only then is the rest of the build
// stopped and its CI fix queued. A failure a build ahead may have caused waits for that build, a
// build the queue rebuilt is never read, and nothing is cancelled while the queue still holds it.

import (
	"fmt"
	"strconv"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/sweep"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// evacMain is `main`'s head in these worlds.
const evacMain = "aaaa"

type evacWorld struct {
	gh  *fakes.FakeGitHub
	ctx *hctx.Context
}

// newEvacWorld has bot pull request #9, its head `s1` cleared by the reviews, first in `main`'s
// merge queue.
func newEvacWorld() *evacWorld {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5, fakes.Issue{Labels: []string{config.LabelPROpen}})
	gh.AddPull(9, "bot/issue-5", fakes.Pull{Body: "Closes #5", Labels: []string{config.LabelPR}, SHA: "s1"})
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, TrustText: ptr(eventsTrust)})
	flowUpdateItem(ctx, 9, pyjson.New("cleared", pyjson.New("sha", "s1")))
	gh.Patch.BranchSHA = func(branch string) string {
		if branch == "main" {
			return evacMain
		}
		return ""
	}
	w := &evacWorld{gh: gh, ctx: ctx}
	w.queue(9)
	return w
}

// queue puts these pull requests in the merge queue, in this order.
func (w *evacWorld) queue(numbers ...int) {
	w.gh.MergeQueueOrder = nil
	w.gh.MergeQueue = map[string]bool{}
	for _, n := range numbers {
		id := fmt.Sprintf("PR_%d", n)
		w.gh.MergeQueue[id] = true
		w.gh.MergeQueueOrder = append(w.gh.MergeQueueOrder, id)
	}
}

// build is CI run `id`, the queue's build of pull request `pr` standing on `base`, its head `head`,
// `status` (its conclusion `conclusion` once completed), with `jobs`.
func (w *evacWorld) build(id, pr int, base, head, status, conclusion string, jobs ...*pyjson.Object) {
	var ended any
	if status == "completed" {
		ended = conclusion
	}
	w.gh.Runs.Set(strconv.Itoa(id), pyjson.New("id", id, "name", "CI", "workflow", "ci.yml", "event", "merge_group",
		"status", status, "conclusion", ended, "head_branch", fmt.Sprintf("gh-readonly-queue/main/pr-%d-%s", pr, base),
		"head_sha", head, "run_attempt", 1, "pull_requests", []any{},
		"html_url", fmt.Sprintf("https://github.com/x/actions/runs/%d", id)))
	w.gh.Jobs[strconv.Itoa(id)] = jobs
}

// evacJob is a job named `name` that ended with `conclusion`; evacGoing one still going.
func evacJob(id int, name, conclusion string) *pyjson.Object {
	return pyjson.New("id", id, "name", name, "status", "completed", "conclusion", conclusion,
		"completed_at", clock.ISO(testsupport.Day), "steps", []any{pyjson.New("name", "step", "conclusion", conclusion)})
}

func evacGoing(id int, name string) *pyjson.Object {
	return pyjson.New("id", id, "name", name, "status", "in_progress", "conclusion", nil, "steps", []any{})
}

func (w *evacWorld) queued() []string { return sortedKeys(w.gh.MergeQueue) }

// The first pull request in the queue, its build red in a required job with the rest still going:
// out of the queue at the first look, the build stopped, and the CI fix queued at once with that
// build marked early; the build's completed event then queues no second fix.
func TestEvac_ARedBuildAtTheFrontLeavesTheQueueAtOnce(t *testing.T) {
	w := newEvacWorld()
	w.build(77, 9, evacMain, "bbbb", "in_progress", "",
		evacJob(1, "rust (test engine)", "failure"), evacGoing(2, "web (unit 1/4)"), evacGoing(3, "rust"))
	notes := events.EvacQueue(w.ctx)
	assertEqual(t, notes, []string{"#9: `rust (test engine)` red in its queue build (run 77); took it out of " +
		"the merge queue and stopped the rest of the build; queued a CI fix"})
	assertEqual(t, w.queued(), []string{})
	assertEqual(t, w.gh.Cancels, []any{77})
	assertIn(t, config.LabelRevise, w.gh.LabelNames(9))
	record := dig(t, w.ctx.Store.Load(), "items", "9")
	assertEqual(t, []any{dig(t, record, "source"), dig(t, record, "ci_run_id"), dig(t, record, "ci_fixes"),
		dig(t, record, "ci_early"), dig(t, record, "ci_on_head")}, []any{"ci", 77, 1, true, false})
	comment := lastOf(t, w.gh.BotComments(9))
	assertIn(t, "The merge queue's build of `main` with this on top failed `rust (test engine)`, so I took "+
		"this out of the queue at once", comment)
	assertIn(t, "https://github.com/x/actions/runs/77", comment)
	// The cancelled build's completed event is the bot's own stop: no second fix, no comment.
	before := len(w.gh.BotComments(9))
	done := events.Handle(w.ctx, "workflow_run", pyjson.New("action", "completed", "sender", fakes.Bot,
		"workflow_run", w.gh.GetRun(77)))
	assertIn(t, "the bot stopped run 77 itself", strings.Join(done, " "))
	assertEqual(t, len(w.gh.BotComments(9)), before)
	assertEqual(t, dig(t, w.ctx.Store.Load(), "items", "9", "ci_fixes"), 1)
	// Nothing red is left: the next look reads no queue.
	w.gh.Patch.QueueOrder = func(string) []int { t.Fatal("read the queue with nothing red"); return nil }
	assertEqual(t, events.EvacQueue(w.ctx), []string(nil))
}

// A build red behind one that has not passed the same job yet waits: the job may have failed for
// the pull request ahead, whose changes it holds. Once the build ahead passed that job, the one
// behind is out (a person's pull request, told why), and the one ahead stays.
func TestEvac_ARedBuildBehindOneThatHasNotPassedTheSameJobWaitsForIt(t *testing.T) {
	w := newEvacWorld()
	w.gh.AddPull(10, "people/x", fakes.Pull{SHA: "p1"})
	w.queue(9, 10)
	w.build(77, 9, evacMain, "bbbb", "in_progress", "", evacGoing(1, "rust (test engine)"))
	w.build(78, 10, "bbbb", "cccc", "in_progress", "", evacJob(2, "rust (test engine)", "failure"))
	notes := events.EvacQueue(w.ctx)
	assertEqual(t, notes, []string{"#10: `rust (test engine)` red in its queue build (run 78); waiting for " +
		"the builds ahead of it to pass the same, since it holds their changes too"})
	assertEqual(t, w.queued(), []string{"PR_10", "PR_9"})
	assertEqual(t, w.gh.Cancels, []any{})
	// Another job passing ahead clears nothing.
	w.gh.Jobs["77"] = []*pyjson.Object{evacGoing(1, "rust (test engine)"), evacJob(3, "web (unit 1/4)", "success")}
	assertIn(t, "waiting for the builds ahead", strings.Join(events.EvacQueue(w.ctx), " "))
	w.gh.Jobs["77"][0] = evacJob(1, "rust (test engine)", "success")
	notes = events.EvacQueue(w.ctx)
	assertEqual(t, notes, []string{"#10: `rust (test engine)` red in its queue build (run 78); took it out of " +
		"the merge queue and stopped the rest of the build; told its author"})
	assertEqual(t, w.queued(), []string{"PR_9"})
	assertEqual(t, w.gh.Cancels, []any{78})
	assertIn(t, "failed `rust (test engine)` ([run](https://github.com/x/actions/runs/78)), so I took this out "+
		"of the queue at once", lastOf(t, w.gh.BotComments(10)))
	assertEqual(t, w.gh.LabelNames(10), setOf())
}

// When the build ahead failed the same job, the one ahead is the one out: the queue builds the one
// behind again without it. A build ahead that passed whole clears every job; one skipped clears it.
func TestEvac_TheFirstRedBuildDownTheQueueIsTheOneOut(t *testing.T) {
	w := newEvacWorld()
	w.gh.AddPull(10, "people/x", fakes.Pull{SHA: "p1"})
	w.queue(9, 10)
	w.build(77, 9, evacMain, "bbbb", "in_progress", "", evacJob(1, "db (sql)", "failure"))
	w.build(78, 10, "bbbb", "cccc", "in_progress", "", evacJob(2, "db (sql)", "failure"))
	assertIn(t, "#9: `db (sql)` red in its queue build (run 77); took it out", strings.Join(events.EvacQueue(w.ctx), " "))
	assertEqual(t, w.queued(), []string{"PR_10"})
	assertEqual(t, w.gh.Cancels, []any{77})
	assertEqual(t, w.gh.BotComments(10), []string{})

	passed := newEvacWorld()
	passed.gh.AddPull(10, "people/x", fakes.Pull{SHA: "p1"})
	passed.gh.AddPull(11, "people/y", fakes.Pull{SHA: "p2"})
	passed.queue(9, 10, 11)
	passed.build(77, 9, evacMain, "bbbb", "completed", "success")
	passed.build(78, 10, "bbbb", "cccc", "in_progress", "", evacJob(2, "web (unit 2/4)", "skipped"))
	passed.build(79, 11, "cccc", "dddd", "in_progress", "", evacJob(3, "web (unit 2/4)", "timed_out"))
	assertIn(t, "#11: `web (unit 2/4)` red in its queue build (run 79); took it out",
		strings.Join(events.EvacQueue(passed.ctx), " "))
	assertEqual(t, passed.queued(), []string{"PR_10", "PR_9"})
}

// Only the current build is read: one standing on a commit that is not the head of the build ahead
// (or `main`'s, for the first) was rebuilt; one not built yet on what is ahead of it stops the look;
// one that ended is the queue's; and a job no required check gathers counts for nothing.
func TestEvac_OnlyTheCurrentBuildOfAQueuedPullRequestCounts(t *testing.T) {
	w := newEvacWorld()
	w.build(70, 9, "0000", "zzzz", "in_progress", "", evacJob(1, "rust (test engine)", "failure")) // rebuilt
	assertEqual(t, events.EvacQueue(w.ctx), []string(nil))
	w.build(77, 9, evacMain, "bbbb", "in_progress", "", evacGoing(2, "rust (test engine)"))
	assertEqual(t, events.EvacQueue(w.ctx), []string(nil))
	// A red build of a pull request that is not in the queue (it left) is never acted on.
	w.gh.AddPull(10, "people/x", fakes.Pull{SHA: "p1"})
	w.build(78, 10, "bbbb", "cccc", "in_progress", "", evacJob(3, "rust (test engine)", "failure"))
	assertEqual(t, events.EvacQueue(w.ctx), []string(nil))
	// Behind a pull request whose build has not started on `main` yet, nothing is read.
	w.queue(9, 10)
	w.gh.Runs.Pop("77")
	assertEqual(t, events.EvacQueue(w.ctx), []string(nil))
	// A build that ended red is the queue's to take out, through its check.
	w.build(77, 9, evacMain, "bbbb", "completed", "failure", evacJob(2, "rust (test engine)", "failure"))
	assertEqual(t, events.EvacQueue(w.ctx), []string(nil))
	// A job no required check gathers.
	w.build(77, 9, evacMain, "bbbb", "in_progress", "", evacJob(2, "cancel the pull request's older queue builds", "failure"))
	w.gh.Runs.Pop("78")
	assertEqual(t, events.EvacQueue(w.ctx), []string(nil))
	assertEqual(t, w.gh.Cancels, []any{})
	assertEqual(t, w.queued(), []string{"PR_10", "PR_9"})
}

// Nothing is cancelled while the queue still holds the pull request: a build cancelled before its
// summary jobs leaves its required checks skipped, which count as passing. The build goes on, its
// check takes it out, and its completed event gets the CI fix as ever.
func TestEvac_NothingIsCancelledWhileTheQueueStillHoldsIt(t *testing.T) {
	w := newEvacWorld()
	w.build(77, 9, evacMain, "bbbb", "in_progress", "", evacJob(1, "rust (test engine)", "failure"))
	w.gh.Patch.Dequeue = func(string) { panic(&herr.GitHubError{Msg: "not allowed", Status: 403}) }
	notes := events.EvacQueue(w.ctx)
	assertEqual(t, notes, []string{"#9: `rust (test engine)` red in its queue build (run 77), and the merge " +
		"queue kept it, so the build goes on and its check takes it out"})
	assertEqual(t, w.gh.Cancels, []any{})
	assertEqual(t, w.queued(), []string{"PR_9"})
	assertNotIn(t, "source", dig(t, w.ctx.Store.Load(), "items", "9"))
	w.gh.Patch.InMergeQueue = func(string) bool { panic(&herr.GitHubError{Msg: "graphql down", Status: 502}) }
	assertIn(t, "could not be read (graphql down)", strings.Join(events.EvacQueue(w.ctx), " "))
	assertEqual(t, w.gh.Cancels, []any{})
	// The queue takes it out once its `rust` check reports, and its completed event is no stop of the
	// bot's: the CI fix comes from `OnQueueCI`.
	w.gh.Patch.InMergeQueue = nil
	w.gh.Patch.Dequeue = nil
	w.queue()
	w.build(77, 9, evacMain, "bbbb", "completed", "cancelled", evacJob(1, "rust (test engine)", "failure"))
	done := events.Handle(w.ctx, "workflow_run", pyjson.New("action", "completed", "sender", fakes.Bot,
		"workflow_run", w.gh.GetRun(77)))
	assertEqual(t, done, []string{"#9: queued a CI fix"})
}

// A held bot pull request (a stop, a person's label) is taken out all the same, but gets no fix; and
// another bot's is left to that bot, whose CI event of the build answers it.
func TestEvac_AHeldPullRequestOrAnotherBotsLeavesTheQueueWithNoFix(t *testing.T) {
	w := newEvacWorld()
	flowUpdateItem(w.ctx, 9, pyjson.New("stop_requested", true))
	w.build(77, 9, evacMain, "bbbb", "in_progress", "", evacJob(1, "e2e smoke (1/2)", "failure"))
	assertEqual(t, events.EvacQueue(w.ctx), []string{"#9: `e2e smoke (1/2)` red in its queue build (run 77); " +
		"took it out of the merge queue and stopped the rest of the build; out of the merge queue, and held, " +
		"so left alone"})
	assertEqual(t, w.gh.LabelNames(9), setOf(config.LabelPR))
	assertEqual(t, w.gh.BotComments(9), []string{})

	other := newEvacWorld()
	squishy := config.Others[0]
	other.gh.AddPull(12, "squishy/issue-3", fakes.Pull{Labels: []string{squishy.LabelPrefix + "pr"}, SHA: "q1"})
	other.queue(12)
	other.build(80, 12, evacMain, "bbbb", "in_progress", "", evacJob(1, "what the change touches", "failure"))
	assertEqual(t, events.EvacQueue(other.ctx), []string{"#12: `what the change touches` red in its queue build " +
		"(run 80); took it out of the merge queue and stopped the rest of the build; Squishy's, whose CI event " +
		"of the build answers it"})
	assertEqual(t, other.gh.Cancels, []any{80})
	assertEqual(t, other.gh.BotComments(12), []string{})
}

// One process looks, the runner gate's (the night bot's): Squishy's sweep skips it. Every sweep looks,
// so the queue is evacuated at the sweep's pace while the status loop is down.
func TestEvac_TheNightBotsSweepLooksAndSquishysDoesNot(t *testing.T) {
	squishy := newEvacWorld()
	squishy.ctx.Cfg.BotLogin = "squishy-squooby"
	squishy.build(77, 9, evacMain, "bbbb", "in_progress", "", evacJob(1, "rust (test engine)", "failure"))
	assertEqual(t, events.EvacQueue(squishy.ctx), []string(nil))
	assertEqual(t, squishy.queued(), []string{"PR_9"})

	w := newEvacWorld()
	w.build(77, 9, evacMain, "bbbb", "in_progress", "", evacJob(1, "rust (test engine)", "failure"))
	eventsUpdateState(w.ctx, "last_sweep", pyjson.New("since", clock.ISO(testsupport.Day.Add(-60*60*1e9))))
	assertIn(t, "merge queue: #9: `rust (test engine)` red in its queue build (run 77); took it out of the "+
		"merge queue", strings.Join(sweep.Sweep(w.ctx), "\n"))
	assertEqual(t, w.queued(), []string{})
}
