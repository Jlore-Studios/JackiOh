package tests

// Red CI on a bot pull request stops early and goes to its lane, and a local pass re-runs CI
// without a commit (#2948, section 4; the owner's decision of 2026-11-03): the status loop's early
// stop (`events.StopRedCI`, `events.StopRedRuns`, `dashboard.StopRed`), `events.OnCI` ignoring the
// bot's own cancellations and re-running nothing first, the CI reader of an own review run on a run
// the bot stopped, and the fix run that judges: its failing check passing locally re-runs CI with
// no commit and costs no CI fix, once a head (`ci_reruns`).

import (
	"fmt"
	"regexp"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// ciEarlyRun puts CI run `id` on `gh`, still going, on `branch` at `sha` for pull request `pr`, with
// `jobs`.
func ciEarlyRun(gh *fakes.FakeGitHub, id int, branch, sha string, pr int, jobs ...*pyjson.Object) *pyjson.Object {
	run := pyjson.New("id", id, "name", "CI", "workflow", "ci.yml", "event", "pull_request",
		"status", "in_progress", "conclusion", nil, "head_branch", branch, "head_sha", sha, "run_attempt", 1,
		"created_at", "2026-09-29T16:50:00Z", "html_url", fmt.Sprintf("https://github.com/x/actions/runs/%d", id),
		"pull_requests", []any{pyjson.New("number", pr)})
	gh.Runs.Set(strconv.Itoa(id), run)
	gh.Jobs[strconv.Itoa(id)] = jobs
	return run
}

// ciEarlyRed is a job that failed at `at`, and ciEarlyGoing one still going.
func ciEarlyRed(id int, name string, at time.Time) *pyjson.Object {
	return pyjson.New("id", id, "name", name, "status", "completed", "conclusion", "failure",
		"completed_at", clock.ISO(at), "steps", []any{pyjson.New("name", "cargo test", "conclusion", "failure")})
}

func ciEarlyGoing(id int, name string) *pyjson.Object {
	return pyjson.New("id", id, "name", name, "status", "in_progress", "conclusion", nil, "steps", []any{})
}

// ciEarlyCompleted is the completed event of `gh`'s run `id` as it stands now.
func ciEarlyCompleted(gh *fakes.FakeGitHub, id int) *pyjson.Object {
	return pyjson.New("action", "completed", "sender", fakes.Bot, "workflow_run", gh.GetRun(id))
}

// ciEarlyAt moves the context's clock on by `d`.
func ciEarlyAt(w *eventsReviewWorld, d time.Duration) {
	c := testsupport.ClockOf(w.ctx)
	c.At = c.At.Add(d)
}

// A required job red on the head, with another job still going: the first look notes it and
// waits; the run is stopped once three minutes have passed since the job failed, and its CI fix is
// queued with the failed job named.
func TestCIEarly_ARedRequiredJobIsStoppedAfterThreeMinutes(t *testing.T) {
	assertEqual(t, events.CIEarlyMinutes, 3)
	w := newEventsReviewWorld()
	now := w.ctx.Now()
	ciEarlyRun(w.gh, 900, "bot/issue-5", "s1", 9, ciEarlyRed(1, "rust (test engine)", now),
		ciEarlyGoing(2, "web (unit 1/4)"))
	notes := events.StopRedRuns(w.ctx)
	assertEqual(t, len(notes), 1)
	assertIn(t, "#9: `rust (test engine)` red in CI run 900", notes[0])
	assertEqual(t, dig(t, w.ctx.Store.Load(), events.CIRedSeenKey, "900/1", "at"), clock.ISO(now))
	ciEarlyAt(w, 2*time.Minute)
	events.StopRedRuns(w.ctx)
	assertEqual(t, []any{w.gh.Cancels, w.gh.LabelNames(9)}, []any{[]any{}, setOf(config.LabelPR)})
	ciEarlyAt(w, time.Minute)
	notes = events.StopRedRuns(w.ctx)
	assertIn(t, "stopped the rest of it; queued a CI fix", notes[0])
	assertEqual(t, w.gh.Cancels, []any{900})
	assertIn(t, config.LabelRevise, w.gh.LabelNames(9))
	record := dig(t, w.ctx.Store.Load(), "items", "9")
	assertEqual(t, []any{dig(t, record, "source"), dig(t, record, "ci_run_id"), dig(t, record, "ci_sha"),
		dig(t, record, "ci_fixes"), dig(t, record, "ci_early"), dig(t, record, "ci_on_head")},
		[]any{"ci", 900, "s1", 1, true, true})
	said := lastOf(t, w.gh.BotComments(9))
	assertIn(t, "CI's required job(s) `rust (test engine)` failed on `s1`, so I stopped the rest of its run early", said)
	assertIn(t, "Queued a revision of #9", said)
	assertEqual(t, dig(t, w.ctx.Store.Load(), events.CIRedSeenKey), pyjson.New()) // done with
	// Stopped once: a later look does nothing more.
	assertEqual(t, events.StopRedRuns(w.ctx), []string{})
	assertEqual(t, w.gh.Cancels, []any{900})
}

// With no job of the run still going, nothing more can report: it is stopped at once.
func TestCIEarly_ItStopsSoonerWhenNothingElseRuns(t *testing.T) {
	w := newEventsReviewWorld()
	ciEarlyRun(w.gh, 900, "bot/issue-5", "s1", 9, ciEarlyRed(1, "db (sql)", w.ctx.Now()),
		pyjson.New("id", 2, "name", "web (typecheck, eslint)", "status", "completed", "conclusion", "success"))
	events.StopRedRuns(w.ctx)
	assertEqual(t, w.gh.Cancels, []any{900})
	assertIn(t, config.LabelRevise, w.gh.LabelNames(9))
}

// The run is in the state file's list of runs the bot stopped, with its attempt, head and when,
// before the cancel goes out, so its completed event can never be taken for a red run.
func TestCIEarly_TheRunIsRecordedBeforeItIsCancelled(t *testing.T) {
	w := newEventsReviewWorld()
	ciEarlyRun(w.gh, 900, "bot/issue-5", "s1", 9, ciEarlyRed(1, "rust (fmt, clippy)", w.ctx.Now()))
	var atCancel *pyjson.Object
	w.gh.Patch.CancelRun = func(runID any) {
		atCancel = w.ctx.Store.Load()
		w.gh.Own().CancelRun(runID)
	}
	events.StopRedRuns(w.ctx)
	assertEqual(t, dig(t, atCancel, events.CIStoppedKey), []any{pyjson.New("id", 900, "attempt", 1,
		"sha", "s1", "pr", 9, "at", clock.ISO(w.ctx.Now()))})
	assertEqual(t, w.gh.Cancels, []any{900})
}

// A person's pull request, `main`, the merge queue's build and a run that is no pull request's
// are never stopped, nor a bot branch whose pull request is not the bot's.
func TestCIEarly_APersonsBranchMainAndTheMergeQueueAreNeverStopped(t *testing.T) {
	w := newEventsReviewWorld()
	now := w.ctx.Now()
	w.gh.AddPull(10, "feature/mine", fakes.Pull{SHA: "p1"})
	w.gh.AddPull(11, "bot/issue-7", fakes.Pull{SHA: "b1"}) // the label came off: not the bot's now
	ciEarlyRun(w.gh, 901, "feature/mine", "p1", 10, ciEarlyRed(1, "rust (test engine)", now))
	ciEarlyRun(w.gh, 902, "bot/issue-7", "b1", 11, ciEarlyRed(2, "rust (test engine)", now))
	ciEarlyRun(w.gh, 903, "main", "m1", 9, ciEarlyRed(3, "rust (test engine)", now)).Set("event", "push")
	ciEarlyRun(w.gh, 904, "gh-readonly-queue/main/pr-9-abc", "s1", 9, ciEarlyRed(4, "rust (test engine)", now)).
		Set("event", "merge_group")
	ciEarlyRun(w.gh, 905, "bot/issue-5", "s1", 9, ciEarlyRed(5, "rust (test engine)", now)).Set("event", "workflow_dispatch")
	ciEarlyAt(w, 10*time.Minute)
	assertEqual(t, events.StopRedRuns(w.ctx), []string{})
	assertEqual(t, w.gh.Cancels, []any{})
	assertEqual(t, w.gh.BotComments(9), []string{})
}

// A run on a head its pull request has moved on from is not stopped: CI on the new head is what
// counts, and GitHub cancels the old one by itself.
func TestCIEarly_AStaleHeadIsNotStopped(t *testing.T) {
	w := newEventsReviewWorld()
	w.gh.Threads.Item(9).Obj("head").Set("sha", "s2")
	ciEarlyRun(w.gh, 900, "bot/issue-5", "s1", 9, ciEarlyRed(1, "rust (test engine)", w.ctx.Now()))
	assertEqual(t, events.StopRedRuns(w.ctx), []string{})
	assertEqual(t, []any{w.gh.Cancels, w.gh.LabelNames(9)}, []any{[]any{}, setOf(config.LabelPR)})
}

// The completed event of the run the bot stopped is ignored, whatever its conclusion: no re-run, no
// second CI fix, nothing counted. A re-run of it is a new attempt, which is CI again.
func TestCIEarly_OnCIIgnoresTheBotsOwnCancelledRun(t *testing.T) {
	w := newEventsReviewWorld()
	ciEarlyRun(w.gh, 900, "bot/issue-5", "s1", 9, ciEarlyRed(1, "rust (test engine)", w.ctx.Now()))
	events.StopRedRuns(w.ctx)
	comments := len(w.gh.BotComments(9))
	w.gh.RemoveLabel(9, config.LabelRevise) // even with no fix queued any more
	for _, conclusion := range []string{"cancelled", "failure"} {
		w.gh.Runs.Obj("900").Set("conclusion", conclusion)
		assertEqual(t, events.Handle(w.ctx, "workflow_run", ciEarlyCompleted(w.gh, 900)),
			[]string{"ignored: the bot stopped run 900 itself, once a required job had failed"})
	}
	assertEqual(t, []any{w.gh.Reruns, w.gh.FullReruns, len(w.gh.BotComments(9)),
		dig(t, w.ctx.Store.Load(), "items", "9", "ci_fixes")}, []any{[]any{}, []any{}, comments, 1})
	// Its second attempt, red too, is CI again: a CI fix.
	w.gh.Runs.Obj("900").Set("run_attempt", 2).Set("conclusion", "failure")
	assertEqual(t, events.Handle(w.ctx, "workflow_run", ciEarlyCompleted(w.gh, 900)), []string{"#9: queued a CI fix"})
	assertEqual(t, dig(t, w.ctx.Store.Load(), "items", "9", "ci_fixes"), 2)
}

// The state file keeps the runs it stopped for `CIStoppedDays` and at most `CIStoppedKeep` of them.
func TestCIEarly_TheStoppedListIsBounded(t *testing.T) {
	w := newEventsReviewWorld()
	patch(t, &events.CIStoppedKeep, 2)
	old := clock.ISO(w.ctx.Now().Add(-8 * 24 * time.Hour))
	flowUpdate(w.ctx, func(s *pyjson.Object) {
		s.Set(events.CIStoppedKey, []any{pyjson.New("id", 1, "attempt", 1, "sha", "x", "pr", 3, "at", old)})
	})
	for i, sha := range []string{"s1", "s2", "s3"} {
		w.gh.Threads.Item(9).Obj("head").Set("sha", sha)
		w.gh.RemoveLabel(9, config.LabelRevise)
		ciEarlyRun(w.gh, 900+i, "bot/issue-5", sha, 9, ciEarlyRed(i, "rust (test engine)", w.ctx.Now()))
		events.StopRedRuns(w.ctx)
	}
	var ids []any
	for _, e := range pyjson.Objs(w.ctx.Store.Load().Get(events.CIStoppedKey)) {
		ids = append(ids, e.Get("id"))
	}
	assertEqual(t, ids, []any{901, 902})
}

// A pull request a run holds (its own review run is next: `bot:reviewing`) has its red CI stopped
// but no fix queued: that run reads the head's CI, and the CI reader takes the run the bot cancelled
// for red, with its failed job, not for a run still going or a cancellation that says nothing; the
// gathering job, skipped when its run was cancelled, gives way to the piece that failed.
func TestCIEarly_ALiveItemIsStoppedNotQueuedAndItsRunFixesTheRed(t *testing.T) {
	h := ownReviewHarness(t, nil)
	pr, built := ownReviewBuild(t, h, "")
	head := crossStr(t, built, "head")
	h.gh.Threads.Item(pr).Obj("head").Set("sha", head)
	assertIn(t, config.LabelOwnReview, h.gh.LabelNames(pr))
	ciEarlyRun(h.gh, 900, "bot/issue-12", head, pr, ciEarlyRed(9001, "rust (test engine)", h.ctx.Now()),
		pyjson.New("id", 9002, "name", "what the change touches", "status", "completed", "conclusion", "success"),
		ciEarlyGoing(9003, "web (unit 1/4)"))
	testsupport.ClockOf(h.ctx).At = testsupport.ClockOf(h.ctx).At.Add(4 * time.Minute)
	notes := events.StopRedRuns(h.ctx)
	assertIn(t, "the run that holds it, or is queued for it, reads its CI", notes[0])
	assertEqual(t, h.gh.Cancels, []any{900})
	assertNotIn(t, config.LabelRevise, h.gh.LabelNames(pr))
	assertFalse(t, crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr)).Has("ci_fixes"))
	// As GitHub ends it: the piece still going cancelled, the summary job skipped.
	h.gh.Jobs["900"] = append(h.gh.Jobs["900"], pyjson.New("id", 9004, "name", "rust", "status", "completed",
		"conclusion", "skipped"))
	assertEqual(t, dig(t, h.gh.GetRun(900), "conclusion"), "cancelled")
	assertEqual(t, events.Handle(h.ctx, "workflow_run", ciEarlyCompleted(h.gh, 900)),
		[]string{"ignored: the bot stopped run 900 itself, once a required job had failed"})
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"review": {workReviewer(workApprove), workReviewer(workApprove)},
		"fix":    {workBuilder(map[string]string{"src/game.txt": "rules v2, the engine test fixed\n"})}})
	_, result := h.night(r)
	assertEqual(t, workRoles(r), []string{"review", "fix", "review"})
	fix := ownReviewPrompt(t, r, "fix")
	assertIn(t, "The required check `rust (test engine)` is red in CI on `"+head[:12]+"`", fix)
	assertIn(t, "log of job 9001", fix)
	assertNotIn(t, "web (unit 1/4)", fix)
	assertNotIn(t, "The required check `rust` is red", fix)
	assertEqual(t, dig(t, pyjson.Objs(result.Get("cycles"))[0], "ci", "red"), []any{"rust (test engine)"})
	assertEqual(t, result.Get("status"), "approved")
}

// ciEarlyCleared is a bot pull request its own review run approved: ready, auto-merge on, Muse its
// merge owner, at the head GitHub has.
func ciEarlyCleared(t *testing.T) (*flowHarness, int, string) {
	t.Helper()
	h := ownReviewHarness(t, nil)
	pr, built := ownReviewBuild(t, h, "")
	_, result := h.night(workFake(map[string]runner.Handler{"review": workReviewer(workApprove)}))
	assertEqual(t, result.Get("status"), "approved")
	head := crossStr(t, built, "head")
	h.gh.Threads.Item(pr).Obj("head").Set("sha", head) // as GitHub has it
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "cleared", "sha"), head)
	assertEqual(t, h.gh.AutoMergeHeads[fmt.Sprintf("PR_%d", pr)], head)
	return h, pr, head
}

// ciEarlyStop stops CI run `id` on the cleared head, red in `rust (test engine)` (job `id`1) with
// `web (unit 1/4)` cancelled, and queues its CI fix.
func ciEarlyStop(t *testing.T, h *flowHarness, pr int, head string, id int) {
	t.Helper()
	ciEarlyRun(h.gh, id, "bot/issue-12", head, pr, ciEarlyRed(id*10+1, "rust (test engine)", h.ctx.Now()),
		ciEarlyGoing(id*10+2, "web (unit 1/4)"))
	testsupport.ClockOf(h.ctx).At = testsupport.ClockOf(h.ctx).At.Add(4 * time.Minute)
	notes := events.StopRedRuns(h.ctx)
	assertIn(t, "queued a CI fix", strings.Join(notes, "\n"))
	assertNotIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge) // held while it is fixed
}

// ciEarlyPass is a CI fix's builder that ran the failing check locally, found it passing, and
// changed nothing.
var ciEarlyPass = "<!-- bot: {\"status\": \"done\", \"title\": \"Make the rules v2\", \"ci_local\": \"passed\"} -->\n" +
	"`cargo test -p jackioh-engine` passes here: the failure was the runner's, not the branch's."

// The CI fix goes to the lane whose approval shipped the head, told that the run was stopped early,
// with the failed job's log and not the cancelled one's, and to run the failing check first. It
// passes there, with nothing changed: the run ends with no commit (the merge of a `main` that moved
// is dropped), no checks and no review, and its delivery runs the whole CI run again, which costs
// none of its CI fixes, and turns auto-merge on again for the head the reviews had cleared.
func TestCIEarly_ALocalPassRerunsCIWithNoCommitAndCostsNoCIFix(t *testing.T) {
	h, pr, head := ciEarlyCleared(t)
	ciEarlyStop(t, h, pr, head, 900)
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", "12", "owner"), "muse")
	fakes.PushBranch(h.origin, h.root, "main", map[string]string{"README.md": "news\n"})
	crossNextRun(h, "779")
	r := workFake(map[string]runner.Handler{"revise": workBuilder(map[string]string{}, ciEarlyPass)})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("action"), planned.Get("number"), planned.Get("provider"),
		planned.Get("source"), planned.Get("ci_reruns_left")}, []any{"revise", pr, "muse", "ci", 1})
	feedback := planned.Str("feedback")
	assertIn(t, plan.CIEarlyNote, feedback)
	assertIn(t, "log of job 9001", feedback)
	assertNotIn(t, "log of job 9002", feedback)
	assertIn(t, "Before you change anything, run the failing check locally", feedback)
	assertIn(t, `add `+"`"+`"ci_local": "passed"`+"`", feedback)
	assertEqual(t, workRoles(r), []string{"revise"})
	assertEqual(t, []any{result.Get("status"), result.Get("head"), dig(t, result, "ci_local", "passed"),
		result.Get("gates"), result.Has("bundle")}, []any{"built", head, true, []any{}, false})
	assertEqual(t, h.originSHA("bot/issue-12"), head)
	assertEqual(t, []any{h.gh.FullReruns, h.gh.Reruns}, []any{[]any{900}, []any{}})
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	assertEqual(t, []any{record.Get("ci_fixes"), dig(t, record, "ci_reruns", head), record.Get("source")},
		[]any{0, 1, ""})
	assertEqual(t, h.gh.AutoMergeHeads[fmt.Sprintf("PR_%d", pr)], head)
	assertIn(t, config.LabelMerging, h.gh.LabelNames(pr))
	assertNotIn(t, config.LabelRevise, h.gh.LabelNames(pr))
	assertFalse(t, config.IsLive(h.gh.LabelNames(pr)))
	said := lastOf(t, h.gh.BotComments(pr))
	assertIn(t, "it ran the failing check locally, and it passed, with nothing to change, so I pushed nothing", said)
	assertIn(t, "I asked CI to run `"+head[:12]+"` again in full (run 900, every job), with no commit.", said)
	assertIn(t, "That costs none of its CI fixes", said)
}

// The head ran again after a local pass and is red again: the next CI fix is told a local pass is
// not enough now, and one that passes locally again and changes nothing is a CI fix like any (it
// counts, and its own review run, which reads the red CI, comes next); CI does not run again.
func TestCIEarly_ASecondLocalPassOnTheSameHeadCounts(t *testing.T) {
	h, pr, head := ciEarlyCleared(t)
	ciEarlyStop(t, h, pr, head, 900)
	crossNextRun(h, "779")
	h.night(workFake(map[string]runner.Handler{"revise": workBuilder(map[string]string{}, ciEarlyPass)}))
	assertEqual(t, h.gh.FullReruns, []any{900})
	// Its second attempt goes red too, and is stopped.
	h.gh.Runs.Obj("900").Set("status", "in_progress").Set("conclusion", nil).Set("run_attempt", 2)
	h.gh.Jobs["900"] = []*pyjson.Object{ciEarlyRed(9011, "rust (test engine)", h.ctx.Now())}
	assertIn(t, "queued a CI fix", strings.Join(events.StopRedRuns(h.ctx), "\n"))
	assertEqual(t, dig(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "ci_fixes"), 1)
	crossNextRun(h, "780")
	r := workFake(map[string]runner.Handler{"revise": workBuilder(map[string]string{}, ciEarlyPass)})
	planned, result := h.night(r)
	assertEqual(t, planned.Get("ci_reruns_left"), 0)
	assertIn(t, "a local pass is not enough now", planned.Str("feedback"))
	assertEqual(t, []any{result.Get("status"), result.Get("review_next"), result.Has("ci_local")},
		[]any{"built", "own", false})
	assertNotEqual(t, pyjson.List(dig(t, result, "cycles", 0, "gates")), []any{}) // the checks ran
	assertEqual(t, h.gh.FullReruns, []any{900})
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	assertEqual(t, []any{record.Get("ci_fixes"), dig(t, record, "ci_reruns", head)}, []any{1, 1})
	assertIn(t, config.LabelOwnReview, h.gh.LabelNames(pr))
}

// A real failure: the CI fix (here from CI's completed event, red with no re-run first) changes the
// code, the full checks run on it, and the fix is pushed for its own review run; CI is not re-run,
// and the fix counts.
func TestCIEarly_ARealFailureIsFixedAndTheFullChecksRun(t *testing.T) {
	h, pr, head := ciEarlyCleared(t)
	ciEarlyRun(h.gh, 900, "bot/issue-12", head, pr, ciEarlyRed(9001, "rust (test engine)", h.ctx.Now()))
	h.gh.Runs.Obj("900").Set("status", "completed").Set("conclusion", "failure")
	assertEqual(t, events.Handle(h.ctx, "workflow_run", ciEarlyCompleted(h.gh, 900)),
		[]string{fmt.Sprintf("#%d: queued a CI fix", pr)})
	assertEqual(t, h.gh.Reruns, []any{})
	crossNextRun(h, "779")
	r := workFake(map[string]runner.Handler{"revise": workBuilder(map[string]string{"src/game.txt": "rules v2, fixed\n"})})
	planned, result := h.night(r)
	assertNotIn(t, plan.CIEarlyNote, planned.Str("feedback"))
	assertIn(t, "Before you change anything, run the failing check locally", planned.Str("feedback"))
	assertEqual(t, []any{result.Get("status"), result.Get("review_next")}, []any{"built", "own"})
	gates := pyjson.Objs(dig(t, result, "cycles", 0, "gates"))
	assertEqual(t, len(gates), 2)
	fixed := crossStr(t, result, "head")
	assertNotEqual(t, fixed, head)
	assertEqual(t, h.originSHA("bot/issue-12"), fixed)
	assertEqual(t, []any{h.gh.FullReruns, h.gh.Reruns}, []any{[]any{}, []any{}})
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	assertEqual(t, []any{record.Get("ci_fixes"), record.Get("ci_fixed_head")}, []any{1, fixed})
	assertIn(t, config.LabelOwnReview, h.gh.LabelNames(pr))
}

// The runners refresh of the status loop, which reads the runs going anyway, stops them too.
func TestCIEarly_TheRunnersRefreshStopsWhatItRead(t *testing.T) {
	w := newEventsReviewWorld()
	w.gh.AddIssue(148, fakes.Issue{Title: dashboard.Title, User: fakes.Operator,
		Body: "top\n" + dashboard.RunnersStart + "\nold\n" + dashboard.RunnersEnd})
	flowUpdate(w.ctx, func(s *pyjson.Object) { s.Set("dashboard", pyjson.New("issue", 148)) })
	ciEarlyRun(w.gh, 900, "bot/issue-5", "s1", 9, ciEarlyRed(1, "bot selftest", w.ctx.Now()))
	said := dashboard.RefreshRunners(w.ctx)
	assertIn(t, "redrew #148's runners; #9: `bot selftest` red in CI run 900; stopped the rest of it; queued a CI fix", said)
	assertEqual(t, w.gh.Cancels, []any{900})
}

// The early stop and the CI reader count the jobs a required check needs: every job of ci.yml that
// one of the five checks needs, on a pull request, directly or through another, is a required job
// (`config.RequiredJob`); a job nothing needs is not.
func TestCIEarly_EveryJobARequiredCheckNeedsCountsAsRequired(t *testing.T) {
	text := workflowsRead(t, "ci.yml")
	jobsAt := strings.Index(text, "\njobs:\n")
	assertTrue(t, jobsAt >= 0, "ci.yml has jobs")
	heads := regexp.MustCompile(`(?m)^  ([a-z0-9-]+):\n`).FindAllStringSubmatchIndex(text[jobsAt:], -1)
	nameRe := regexp.MustCompile(`(?m)^    name: (.+)$`)
	needsRe := regexp.MustCompile(`(?m)^    needs: (.+)$`)
	// An expression in a name reads as it does on a pull request: `inputs.e2e` is unset, so
	// `a && 'x' || 'y'` is 'y'; a matrix value is any.
	pick := regexp.MustCompile(`\$\{\{[^}]*\|\|\s*'([^']*)'\s*\}\}`)
	expr := regexp.MustCompile(`\$\{\{[^}]*\}\}`)
	names, needs := map[string]string{}, map[string][]string{}
	for i, at := range heads {
		end := len(text) - jobsAt
		if i+1 < len(heads) {
			end = heads[i+1][0]
		}
		id, block := text[jobsAt:][at[2]:at[3]], text[jobsAt:][at[1]:end]
		if m := nameRe.FindStringSubmatch(block); m != nil {
			names[id] = expr.ReplaceAllString(pick.ReplaceAllString(strings.Trim(m[1], `"`), "$1"), "1")
		}
		if m := needsRe.FindStringSubmatch(block); m != nil {
			for _, need := range strings.Split(strings.Trim(m[1], "[] "), ",") {
				needs[id] = append(needs[id], strings.TrimSpace(need))
			}
		}
	}
	required := config.Load("", map[string]string{}).RequiredChecks
	var todo, reached []string
	for id, name := range names {
		if ciEarlyContains(required, name) {
			todo = append(todo, id)
		}
	}
	assertEqual(t, len(todo), len(required), "every required check is a job", names)
	seen := map[string]bool{}
	for len(todo) > 0 {
		id := todo[0]
		todo = todo[1:]
		if seen[id] {
			continue
		}
		seen[id] = true
		reached = append(reached, id)
		todo = append(todo, needs[id]...)
	}
	assertIn(t, "changes", reached)
	assertIn(t, "e2e-component", reached)
	for _, id := range reached {
		assertTrue(t, config.RequiredJob(names[id], required), "a job a required check needs", id, names[id])
	}
	assertFalse(t, config.RequiredJob(names["queue-cancel-rust"], required))
	assertFalse(t, config.RequiredJob("lint (an optional job)", required))
}

// ciEarlyContains is `x in xs`.
func ciEarlyContains(xs []string, x string) bool {
	for _, s := range xs {
		if s == x {
			return true
		}
	}
	return false
}
