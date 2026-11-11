package tests

// One lane, one model (#2903, the owner's decision of 2026-11-03, a follow-up to #2748): an item's
// plan takes a lane of its planner's own subscription when that one has room; the plan's delivery
// keeps that lane for the build, which starts at once on the model that planned it; the build hands
// the lane to its own review run, which reviews and fixes on the same model; each run starts a fresh
// session. Without room the hold and the handoff go as before, and with no lane free the plan goes
// to the planning lane and its build to its planner first. Through plan, work and deliver against a
// real repository and the fake GitHub, with the committed `own_review_run`, `plan_lanes` and
// `hold_minutes`.

import (
	"fmt"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/sessions"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// The sessions Claude Code names in the scripted calls: the planner's, the build's and the own
// review run's fix pass's.
const (
	oneLanePlanSession  = "2b3c4d5e-6f70-4b1c-8d2e-3f4a5b6c7d01"
	oneLaneBuildSession = "2b3c4d5e-6f70-4b1c-8d2e-3f4a5b6c7d02"
	oneLaneFixSession   = "2b3c4d5e-6f70-4b1c-8d2e-3f4a5b6c7d03"
)

// oneLaneWorld is a flow harness with the committed chain (`own_review_run`) and planning lane
// (`plan_lanes`), and its night runs, each its own workflow run on a fresh runner whose Claude
// sessions are kept in its `work` artifact, as on GitHub.
type oneLaneWorld struct {
	t      *testing.T
	h      *flowHarness
	nights int
}

func newOneLaneWorld(t *testing.T, env map[string]string, machine []string, at time.Time) *oneLaneWorld {
	t.Helper()
	return &oneLaneWorld{t: t, h: newFlowHarness(t, flowHarnessOpts{Env: env, Machine: machine, At: at,
		Chain: true, CommittedPlanLanes: true})}
}

// oneLaneClaude3 is a world at NIGHT with claude-3 alone (two lanes, no caps): its Sonnet is the
// medium tier's only seat, its Opus the strong tier's.
func oneLaneClaude3(t *testing.T) *oneLaneWorld {
	return newOneLaneWorld(t, map[string]string{"HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN_3"}, nil, testsupport.Night)
}

// sessions is the next run's Sessions on `provider`: a config directory of its own, and the runs'
// artifacts as the fake GitHub serves them.
func (w *oneLaneWorld) sessions(provider string) *work.Sessions {
	dir := filepath.Join(w.h.root, fmt.Sprintf("claude-config-%d", w.nights+1))
	return work.NewSessions(w.h.cfg, w.h.cfg.Pool.Get(provider), sessionsSecret, dir,
		func(record *pyjson.Object) (string, error) { return sessions.Fetch(w.h.gh, record) })
}

// night is the next night run (run 9001, 9002, …): it plans, works with `r` and `s` (nil for no
// sessions), uploads its output as its run's `work` artifact and delivers it. Returns the plan and
// the run's result ({} when it planned nothing).
func (w *oneLaneWorld) night(r *runner.FakeRunner, s *work.Sessions) (*pyjson.Object, *pyjson.Object) {
	w.t.Helper()
	w.nights++
	runID := strconv.Itoa(9000 + w.nights)
	crossNextRun(w.h, runID)
	planned := flowMake(w.h.ctx)
	if pyjson.Equal(planned.Get("action"), "none") {
		return planned, pyjson.New()
	}
	out := filepath.Join(w.h.root, fmt.Sprintf("out-%d", flowOuts(w.h.root)))
	worker := work.NewWorker(w.h.cfg, planned, r, w.h.clone, filepath.Join(w.h.root, "work"), out,
		nil, nil, nil, nil, nil, nil)
	worker.Sessions, worker.GH = s, w.h.gh
	worker.Run()
	data := sessionsZip(w.t, out, nil)
	id := 700 + len(w.h.gh.ArtifactZips)
	w.h.gh.Artifacts[runID] = append(w.h.gh.Artifacts[runID], pyjson.New("id", id, "name", "work",
		"expired", false, "size_in_bytes", len(data)))
	w.h.gh.ArtifactZips[id] = data
	deliver.NewDeliverer(w.h.ctx, planned, out, w.h.deliverRepo, "", nil, "").Run()
	return planned, loadJSON(w.t, filepath.Join(out, "result.json"))
}

// record is #`number`'s record in the state file.
func (w *oneLaneWorld) record(number int) *pyjson.Object {
	return w.h.ctx.Store.Load().Obj("items").Obj(strconv.Itoa(number))
}

// oneLanePlanner is a planner that answers a plan rated `difficulty`, in session `id` when `s`
// keeps sessions.
func oneLanePlanner(s *work.Sessions, id, difficulty string) runner.Handler {
	return func(request *runner.RunRequest) *runner.RunResult {
		if s != nil {
			sessionsAppend(s, request.Cwd, id, `{"type":"user","message":"the planner read the task"}`)
		}
		result := runner.NewRunResult(true, ratingRatedPlan(difficulty, nil, ""))
		result.SessionID, result.DurationS = id, 300
		return result
	}
}

// oneLaneCalls is the run's calls as (role, model, the session each resumed).
func oneLaneCalls(r *runner.FakeRunner) [][3]string {
	got := [][3]string{}
	for _, c := range r.Calls {
		got = append(got, [3]string{c.Role, c.Model, c.Resume})
	}
	return got
}

// plan is night 1: issue 12 (`labels`) is planned on `provider`'s `model` in a lane of its own,
// rated `difficulty`, and its delivery keeps that lane for the build and starts it at once.
func (w *oneLaneWorld) plan(provider, model, difficulty string, s *work.Sessions, labels ...string) {
	t := w.t
	t.Helper()
	w.h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Body: "Make the rules v2.",
		Labels: append([]string{config.LabelBuild}, labels...)})
	r := workFake(map[string]runner.Handler{"plan": oneLanePlanner(s, oneLanePlanSession, difficulty)})
	dispatched := len(w.h.gh.Dispatches)
	planned, result := w.night(r, s)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider"), planned.Get("build_next"),
		dig(t, planned, "seats", "plan", "model"), result.Get("status")}, []any{"plan", provider, true, model, "planned"})
	assertIn(t, fmt.Sprintf("in a lane of `%s`", provider), strings.Join(stringsOf(planned.List("routing")), "\n"))
	assertIn(t, fmt.Sprintf("in a lane of `%s`: it is difficulty:", provider), w.h.gh.BotComments(12)[0])
	record := w.record(12)
	assertEqual(t, []any{record.Get("owner"), record.Get(queue.PlanModelKey)}, []any{provider, model})
	// The finished plan's session goes: no run goes on from it.
	assertEqual(t, record.Obj("session").Len(), 0)
	// Its lane is kept for the build, on the model that planned it, for `own_review_reserve_minutes`,
	// and the build is started at once.
	kept := crossObj(t, record, queue.BuildNextKey)
	until := w.h.ctx.Now().Add(time.Duration(w.h.cfg.OwnReviewReserveMinutes) * time.Minute)
	assertEqual(t, []any{kept.Get("provider"), kept.Get("model"), kept.Get("until")},
		[]any{provider, model, clock.ISO(until)})
	lanes := plan.ReadLanes(w.h.ctx, w.h.ctx.Store.Load())
	assertEqual(t, []any{lanes.Held[12], lanes.Reserved[12], lanes.Builds[12]}, []any{provider, true, true})
	assertIn(t, "#12 with `"+provider+"` (reserved for its build)", lanes.Describe())
	said := lastOf(t, w.h.gh.BotComments(12))
	assertIn(t, "Its build is next, in this lane", said)
	assertIn(t, "the model that planned it, in a fresh session", said)
	assertEqual(t, len(w.h.gh.Dispatches), dispatched+1)
}

// build is night 2: `provider` builds #12 on `model`, the planner's, in the lane its plan kept, in
// a fresh session, and hands the lane to its own review run. Returns the pull request.
func (w *oneLaneWorld) build(provider, model string, s *work.Sessions) int {
	t := w.t
	t.Helper()
	builder := workBuilder(map[string]string{"src/game.txt": "rules v2\n"})
	if s != nil {
		builder = sessionsBuilder(s, oneLaneBuildSession, `{"type":"user","message":"the build"}`,
			map[string]string{"src/game.txt": "rules v2\n"})
	}
	r := workFake(map[string]runner.Handler{"build": builder})
	dispatched := len(w.h.gh.Dispatches)
	planned, result := w.night(r, s)
	assertEqual(t, []any{planned.Get("action"), planned.Get("number"), planned.Get("provider"),
		dig(t, planned, "seats", "build", "model"), planned.Get("own_review_next"), planned.Has("session")},
		[]any{"build", 12, provider, model, true, false})
	assertIn(t, "its planner builds it on", strings.Join(stringsOf(planned.List("routing")), "\n"))
	assertEqual(t, oneLaneCalls(r), [][3]string{{"build", model, ""}}) // no planning again, nothing resumed
	assertEqual(t, []any{result.Get("status"), result.Get("review_next")}, []any{"built", "own"})
	if s != nil {
		assertEqual(t, []any{dig(t, result, "session", "resumed"), dig(t, result, "session", "fallback")},
			[]any{false, "none was kept for this item"})
	}
	assertFalse(t, w.record(12).Has(queue.BuildNextKey)) // its lane is this run's now
	pr := crossPullOf(t, w.h.gh, "bot/issue-12")
	reserved := crossObj(t, w.record(pr), queue.OwnReviewKey)
	assertEqual(t, []any{reserved.Get("provider"), reserved.Get("model")}, []any{provider, model})
	assertIn(t, "Its own review run is next, on `"+provider+"`", lastOf(t, w.h.gh.BotComments(12)))
	assertEqual(t, len(w.h.gh.Dispatches), dispatched+1) // its own review run is started at once
	return pr
}

// A planned item's build starts in the planner's lane at once, on the planner's model, in a fresh
// session: claude-3 plans the medium item on Sonnet in a lane of its own; the plan's delivery keeps
// that lane for the build and starts the next run, which is that build, on Sonnet, from the plan
// (no planning in its run), resuming nothing: the finished plan's session is not kept.
func TestOneLane_APlannedItemsBuildStartsInThePlannersLaneAtOnceOnItsModelFresh(t *testing.T) {
	w := oneLaneClaude3(t)
	w.plan("claude-3", "sonnet", "medium", w.sessions("claude-3"), "difficulty:medium")
	// The kept lane is the build's alone: with claude-3's other lane busy, another queued item, planned
	// already, does not take it meanwhile; the build does.
	difficultyBusy(w.h.gh, w.h.ctx, "claude-3", 50)
	w.h.gh.AddIssue(14, fakes.Issue{Title: "Something else", Labels: []string{config.LabelBuild}})
	flowUpdateItem(w.h.ctx, 14, pyjson.New("planned_at", clock.ISO(testsupport.Day), "planned_tier", "strong"))
	queued := queue.Candidates(w.h.ctx, w.h.ctx.Store.Load(), nil, nil)
	pairs := plan.Pairs(w.h.ctx, w.h.ctx.Store.Load(), queued, plan.ReadLanes(w.h.ctx, w.h.ctx.Store.Load()),
		false, plan.AnyQuiet, nil)
	assertEqual(t, len(pairs), 1)
	assertEqual(t, []any{pairs[0].Candidate.Number, pairs[0].Assignment.Provider.ID}, []any{12, "claude-3"})
	// Out of the way of the runs the build starts: #50's run ends with its item closed, and #14 closes.
	w.h.gh.Runs.Set("50", pyjson.New("status", "completed"))
	for _, n := range []int{14, 50} {
		w.h.gh.Threads.Item(n).Set("state", "closed")
	}
	w.build("claude-3", "sonnet", w.sessions("claude-3"))
}

// Run 2 follows on the same lane, every call a fresh agent: the own review run reviews on the
// account's strongest seat (Opus) and fixes what it finds on the model that built the change
// (Sonnet), and none of its calls resumes a session, so changing model between a review and a fix
// rereads nothing, though the build's session is kept on the pull request (for a CI fix after the
// merge queue, which may resume it).
func TestOneLane_ItsOwnReviewRunFollowsOnTheSameLaneFresh(t *testing.T) {
	w := oneLaneClaude3(t)
	w.plan("claude-3", "sonnet", "medium", w.sessions("claude-3"), "difficulty:medium")
	pr := w.build("claude-3", "sonnet", w.sessions("claude-3"))
	assertEqual(t, w.record(pr).Obj("session").Get("id"), oneLaneBuildSession)
	s := w.sessions("claude-3")
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"review": {workReviewer(workChanges("It skips the replay check.")), workReviewer(workApprove)},
		"fix": {sessionsBuilder(s, oneLaneFixSession, `{"type":"user","message":"the fix"}`,
			map[string]string{"src/game.txt": "rules v2, with the replay check\n"})}})
	reviewed, result := w.night(r, s)
	assertEqual(t, []any{reviewed.Get("action"), reviewed.Get("number"), reviewed.Get("provider"),
		reviewed.Get("own_review"), reviewed.Has("session")}, []any{"review", pr, "claude-3", true, false})
	assertEqual(t, oneLaneCalls(r), [][3]string{{"review", "opus", ""}, {"fix", "sonnet", ""}, {"review", "opus", ""}})
	assertEqual(t, result.Get("status"), "approved")
	assertFalse(t, pyjson.Truthy(dig(t, result, "session", "resumed")))
	// Its own fix pass's session is what the pull request keeps now, for a CI fix.
	assertEqual(t, w.record(pr).Obj("session").Get("id"), oneLaneFixSession)
	assertIn(t, fmt.Sprintf("PR_%d", pr), w.h.gh.AutoMerge)
}

// A medium item Sonnet planned on a Claude account is built by Sonnet and approved by Opus, the
// account's strongest seat, whose approval ships it (a strong approval of a medium item), in the
// build's own lane: Sonnet fixes what Opus finds, each call a fresh session.
func TestOneLane_AMediumItemSonnetPlannedIsBuiltBySonnetAndApprovedByOpus(t *testing.T) {
	w := oneLaneClaude3(t)
	w.plan("claude-3", "sonnet", "medium", nil, "difficulty:medium")
	pr := w.build("claude-3", "sonnet", nil)
	r := workFake(map[string]runner.Handler{"review": workReviewer(workApprove)})
	reviewed, result := w.night(r, nil)
	assertEqual(t, []any{dig(t, reviewed, "seats", "review", "model"), dig(t, reviewed, "seats", "build", "model"),
		result.Get("status")}, []any{"opus", "sonnet", "approved"})
	assertEqual(t, oneLaneCalls(r), [][3]string{{"review", "opus", ""}})
	assertIn(t, "its own review run: `claude-3`, whose run built it, reviews it on `claude-3` (claude, `opus`, "+
		"strong) and fixes what it finds on `claude-3` (claude, `sonnet`, medium), the model that built it",
		strings.Join(stringsOf(reviewed.List("routing")), "\n"))
	cleared := crossObj(t, w.record(pr), "cleared")
	assertEqual(t, []any{dig(t, cleared, "approver", "provider"), dig(t, cleared, "approver", "tier")},
		[]any{"claude-3", "strong"})
	assertIn(t, fmt.Sprintf("PR_%d", pr), w.h.gh.AutoMerge)
}

// A hard item is Opus throughout, on the same account: Opus plans it in a lane of claude-3's, builds
// it there, and its own review run reviews and fixes on Opus, whose approval ships it.
func TestOneLane_AHardItemIsOpusThroughout(t *testing.T) {
	w := oneLaneClaude3(t)
	w.plan("claude-3", "opus", "hard", nil, "difficulty:hard")
	pr := w.build("claude-3", "opus", nil)
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"review": {workReviewer(workChanges("No replay check.")), workReviewer(workApprove)},
		"fix":    {workBuilder(map[string]string{"src/game.txt": "rules v2, checked\n"})}})
	reviewed, result := w.night(r, nil)
	assertEqual(t, []any{reviewed.Get("own_review"), dig(t, reviewed, "seats", "review", "model"),
		dig(t, reviewed, "seats", "build", "model"), result.Get("status")}, []any{true, "opus", "opus", "approved"})
	assertEqual(t, oneLaneCalls(r), [][3]string{{"review", "opus", ""}, {"fix", "opus", ""}, {"review", "opus", ""}})
	assertEqual(t, dig(t, w.record(pr), "cleared", "approver", "tier"), "strong")
	assertIn(t, fmt.Sprintf("PR_%d", pr), w.h.gh.AutoMerge)
}

// The plan leaves its subscription at its 5-hour cap: its build cannot start, so the hold applies as
// for any owned item (`bot:waiting`, its comment, no lane kept meanwhile), for up to `hold_minutes`
// (120); when its owner still cannot start by then, the build is handed off, with the plan, to a
// subscription that can (gpt), on that one's own model, which owns it from then on.
func TestOneLane_ACapAfterThePlanHoldsTheBuildUpToTheHoldThenHandsItOff(t *testing.T) {
	w := newOneLaneWorld(t, map[string]string{"HARNESS_SECRETS_SET": ""}, []string{"agy", "gpt"}, testsupport.Day)
	hold := w.h.cfg.HoldMinutes
	assertEqual(t, hold, 120)
	w.plan("agy", "gemini-3.8-flash-high", "medium", nil, "difficulty:medium")
	// The plan took agy to its cap; its session resets 115 minutes on, within the hold.
	at := func(minutes int) time.Time { return testsupport.Day.Add(time.Duration(minutes) * time.Minute) }
	reading := func(resets int) {
		flowUpdate(w.h.ctx, func(s *pyjson.Object) {
			providers.Record(s, "agy").Set("usage", pyjson.New(
				"five_hour", pyjson.New("utilization", 0.99, "resets_at", clock.ISO(at(resets))),
				"seven_day", pyjson.New("utilization", 0.2, "resets_at", clock.ISO(at(5000))),
				"observed_at", clock.ISO(w.h.ctx.Now())))
		})
	}
	reading(115)
	held, _ := w.night(workFake(map[string]runner.Handler{}), nil)
	assertEqual(t, held.Get("action"), "none")
	assertIn(t, config.LabelWaiting, w.h.gh.LabelNames(12))
	kept := crossObj(t, w.record(12), "hold")
	assertEqual(t, []any{kept.Get("owner"), kept.Get("until"), kept.Get("ends")},
		[]any{"agy", clock.ISO(at(115)), clock.ISO(at(hold))})
	assertIn(t, "**Held for `agy`**", lastOf(t, w.h.gh.BotComments(12)))
	assertFalse(t, plan.ReadLanes(w.h.ctx, w.h.ctx.Store.Load()).Reserved[12]) // a capped lane keeps no slot
	// An hour on, still capped, it is still held: gpt, free, does not take it.
	testsupport.ClockOf(w.h.ctx).At = at(60)
	waiting, _ := w.night(workFake(map[string]runner.Handler{}), nil)
	assertEqual(t, waiting.Get("action"), "none")
	// Past its reset it is refused again, until after the hold ends: the build goes to gpt.
	testsupport.ClockOf(w.h.ctx).At = at(116)
	reading(hold + 90)
	r := workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "rules v2\n"})})
	built, result := w.night(r, nil)
	assertEqual(t, []any{built.Get("action"), built.Get("number"), built.Get("provider"),
		dig(t, built, "seats", "build", "model"), result.Get("status")},
		[]any{"build", 12, "gpt", "gpt-5.6-sol", "built"})
	assertNotIn(t, config.LabelWaiting, w.h.gh.LabelNames(12))
	assertIn(t, "**Released to anyone**: `agy`", strings.Join(ownershipHoldComments(w.h.gh, 12), "\n"))
	record := w.record(12)
	assertEqual(t, record.Get("owner"), "gpt")
	assertFalse(t, record.Has(queue.BuildNextKey))
}

// With no lane of its own free at planning time, the plan goes to the planning lane, as before
// #2903, and keeps no lane; its build goes to its planner first, on the planner's model, once that
// one has a lane free, ahead of Muse, free too and first in the medium tier's order.
func TestOneLane_WithNoFreeLaneThePlanUsesThePlanningLaneAndItsBuildGoesToThePlannerFirst(t *testing.T) {
	w := newOneLaneWorld(t, map[string]string{"HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN_3"},
		[]string{"muse"}, testsupport.Night)
	// claude-3's two lanes and Muse's four are all busy.
	difficultyBusy(w.h.gh, w.h.ctx, "claude-3", 50, "claude-3", 51, "muse", 52, "muse", 53, "muse", 54, "muse", 55)
	w.h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Body: "Make the rules v2.",
		Labels: []string{config.LabelBuild, "difficulty:medium"}})
	planned, result := w.night(workFake(map[string]runner.Handler{"plan": oneLanePlanner(nil, oneLanePlanSession, "medium")}), nil)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider"), planned.Has("build_next"),
		dig(t, planned, "seats", "plan", "model"), result.Get("status")}, []any{"plan", "claude-3", false, "sonnet", "planned"})
	assertIn(t, "on the planning lane", strings.Join(stringsOf(planned.List("routing")), "\n"))
	record := w.record(12)
	assertEqual(t, []any{record.Get("owner"), record.Get(queue.PlanModelKey), record.Has(queue.BuildNextKey)},
		[]any{"claude-3", "sonnet", false})
	// One lane each comes free (their runs end with their items closed): claude-3, which planned it,
	// builds it, on Sonnet.
	for _, n := range []int{51, 55} {
		w.h.gh.Runs.Set(strconv.Itoa(n), pyjson.New("status", "completed"))
		w.h.gh.Threads.Item(n).Set("state", "closed")
	}
	r := workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "rules v2\n"})})
	built, _ := w.night(r, nil)
	assertEqual(t, []any{built.Get("action"), built.Get("number"), built.Get("provider"),
		dig(t, built, "seats", "build", "model")}, []any{"build", 12, "claude-3", "sonnet"})
	assertIn(t, "its owner `claude-3` keeps it: one lane, one owner", strings.Join(stringsOf(built.List("routing")), "\n"))
}
