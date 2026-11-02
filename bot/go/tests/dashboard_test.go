// The pinned status issue (`dashboard.py`): opened and pinned once, rewritten every sweep, adopted
// again when the state file loses it, and drawn from the same facts as `/harness status`.
//
// bot/tests/test_dashboard.py, ported (#467).
package tests

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"regexp"
	"slices"
	"sort"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cardsets"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cli"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	hstate "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stats"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/waiting"
)

// dashboardRoom is runners_section's default `room`: MAX_BODY as it was when the Python defined
// the function, before any test patched it (read here when the package starts, as Python reads a
// default when it defines the function).
var dashboardRoom = dashboard.MaxBody

// dashboardOpened is opened.
func dashboardOpened(gh *fakes.FakeGitHub) []int {
	numbers := []int{}
	for n, thread := range gh.Threads.All() {
		if pyjson.Equal(thread.Get("title"), dashboard.Title) {
			numbers = append(numbers, n)
		}
	}
	return numbers
}

// dashboardOffline is offline: the loop's contexts on a fake GitHub, so a test never reaches the
// network. A nil `gh` is a new FakeGitHub. It holds until `restore` (or the test's end).
func dashboardOffline(t *testing.T, gh *fakes.FakeGitHub) (restore func()) {
	if gh == nil {
		gh = fakes.NewFakeGitHub()
	}
	return patch(t, &cli.Ctx, func(cfg *config.Config, write bool) *hctx.Context {
		return testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night, Cfg: cfg})
	})
}

// dashboardUpdateFunc is dashboard.update's signature, for a test's replacement of it.
type dashboardUpdateFunc = func(ctx *hctx.Context, extra []string, withStats bool,
	statsExtra func() []string, forceStats bool, readCards cardsets.Fetch) string

// dashboardUpdate is dashboard.update(ctx) with its defaults.
func dashboardUpdate(ctx *hctx.Context) string {
	return dashboard.Update(ctx, nil, false, nil, false, nil)
}

// dashboardUpdateStats is dashboard.update(ctx, stats=True[, force_stats=force]).
func dashboardUpdateStats(ctx *hctx.Context, force bool) string {
	return dashboard.Update(ctx, nil, true, nil, force, nil)
}

// dashboardRender is dashboard.render(ctx, extra, statistics) with the rest of its defaults.
func dashboardRender(ctx *hctx.Context, extra []string, statistics string) string {
	return dashboard.Render(ctx, extra, statistics, nil, true)
}

// dashboardAgo is `iso(NIGHT - timedelta(minutes=minutes))`.
func dashboardAgo(minutes int) string {
	return clock.ISO(testsupport.Night.Add(-time.Duration(minutes) * time.Minute))
}

// dashboardGitHubError is side_effect=GitHubError(msg, status).
func dashboardGitHubError(msg string, status int) *herr.GitHubError {
	return &herr.GitHubError{Msg: msg, Status: status}
}

// dashboardGist is `{"g1": {"files": {PC_FILE: {"content": json.dumps(beat)}}}}`.
func dashboardGist(beat *pyjson.Object) map[string]*pyjson.Object {
	return map[string]*pyjson.Object{"g1": pyjson.New("files", pyjson.New(dashboard.PCFile,
		pyjson.New("content", pyjson.Dumps(beat))))}
}

// dashboardGistEnv is `{"JACKIOH_PC_GIST": gist} if gist else {}`.
func dashboardGistEnv(gist string) map[string]string {
	if gist == "" {
		return map[string]string{}
	}
	return map[string]string{"JACKIOH_PC_GIST": gist}
}

// dashboardMerged is `{**base, **changes}`.
func dashboardMerged(base, changes *pyjson.Object) *pyjson.Object {
	return base.Copy().Update(changes)
}

// dashboardAfter is `text.split(sep)[1]`.
func dashboardAfter(t *testing.T, text, sep string) string {
	t.Helper()
	parts := strings.Split(text, sep)
	if len(parts) < 2 {
		t.Fatalf("%q is not in:\n%s", sep, showContainer(sep, text))
	}
	return parts[1]
}

// dashboardBefore is `text.split(sep)[0]`.
func dashboardBefore(text, sep string) string { return strings.SplitN(text, sep, 2)[0] }

// ============================================================================ TrainingBoxTests

// The training box's lanes in the status diagram, from their pull requests (#1272).

// dashboardTrainingSetUp is TrainingBoxTests.setUp: the lanes' drafts as they were drawn until
// #1677 stopped them, so a lane named again draws the same way.
func dashboardTrainingSetUp(t *testing.T) {
	patch(t, &dashboard.TrainingLanes, []string{"improve", "unban"})
}

// dashboardBoxes is TrainingBoxTests.boxes.
func dashboardBoxes(gh *fakes.FakeGitHub) string {
	return strings.Join(dashboard.TrainingBoxes(testsupport.MakeCtx(gh, testsupport.Ctx{
		At: testsupport.Night, Cfg: testsupport.MakeConfig()})), "\n")
}

// dashboardLane is TrainingBoxTests.lane.
func dashboardLane(gh *fakes.FakeGitHub, number int, lane, title string, draft bool,
	updated time.Duration) {
	pull := gh.AddPull(number, "ai/"+lane, fakes.Pull{Title: title})
	pull.Update(pyjson.New("draft", draft, "updated_at", clock.ISO(testsupport.Night.Add(-updated))))
}

func TestDashboard_ADraftIsASessionAtWorkAndAnIdleTitleSaysSo(t *testing.T) {
	dashboardTrainingSetUp(t)
	gh := fakes.NewFakeGitHub()
	dashboardLane(gh, 70, "improve", "AI gen 3 (improve): in training, nothing promoted yet",
		true, 20*time.Minute)
	dashboardLane(gh, 71, "unban", "AI gen 9 (unban): idle, the AI on main bans no card",
		true, 3*24*time.Hour)
	body := dashboardBoxes(gh)
	assertIn(t, `subgraph training["The training box, on Muse: 1 of 2 lanes working"]`, body)
	assertIn(t, `t0["<b>improve</b><br/>🟢 a Muse session on #70<br/>updated`, body)
	assertIn(t, ":::busy", body)
	// An idle unban lane's draft goes days unchanged, and that is not a stopped lane.
	assertIn(t, `t1["<b>unban</b><br/>⏸️ idle on #71<br/>the AI on main bans no card"]`+
		`:::paused`, body)
	assertIn(t, "t0 ~~~ t1", body)
	assertNotIn(t, "gather", body) // the owner's PC's now: the runners section draws it
}

func TestDashboard_APromotionWaitsOnCIAndADraftLeftUnchangedIsAStoppedLane(t *testing.T) {
	dashboardTrainingSetUp(t)
	gh := fakes.NewFakeGitHub()
	dashboardLane(gh, 72, "improve", "AI gen 4 (improve): the opening plays its two-drops first",
		false, 5*time.Minute)
	dashboardLane(gh, 73, "unban", "AI gen 9 (unban): in training, nothing promoted yet",
		true, dashboard.TrainingStale+time.Hour)
	body := dashboardBoxes(gh)
	assertIn(t, "0 of 2 lanes working", body)
	assertIn(t, `t0["<b>improve</b><br/>▶️ promotion #72<br/>waiting on CI<br/>updated`,
		body)
	assertIn(t, ":::next", body)
	assertIn(t, `t1["<b>unban</b><br/>⛔ #73 unchanged<br/>updated`, body)
	assertIn(t, "is the lane running?\"]:::broken", body)
}

// #1677 stopped `improve` and `unban`, and gather left the box for the owner's PC (gather.yml),
// so an `ai/` draft left open draws nothing, and nor does the box.
func TestDashboard_WithTheLanesStoppedNoTrainingBoxIsDrawn(t *testing.T) {
	dashboardTrainingSetUp(t)
	gh := fakes.NewFakeGitHub()
	dashboardLane(gh, 74, "improve", "AI gen 14 (improve): in training, nothing promoted yet",
		true, 20*time.Minute)
	restore := patch(t, &dashboard.TrainingLanes, []string{})
	assertEqual(t, dashboardBoxes(gh), "")
	restore()
}

func TestDashboard_UnreadablePullRequestsDrawNoTrainingBox(t *testing.T) {
	dashboardTrainingSetUp(t)
	gh := fakes.NewFakeGitHub()
	gh.Patch.ListPulls = func(string, string, int) []*pyjson.Object {
		panic(dashboardGitHubError("boom", 0))
	}
	assertEqual(t, dashboardBoxes(gh), "")
	gh.Patch.ListPulls = nil
}

// ============================================================================ LearnLaneTests

// The learn lane's section (#466): read from GitHub alone, since no harness runs the lane.

// dashboardRun is LearnLaneTests.run_with's keyword arguments: Event defaults to "schedule",
// Status to "completed", and nil Jobs to [].
type dashboardRun struct {
	Number int
	Event  string
	Status string
	Jobs   []*pyjson.Object
}

// dashboardRunWith is LearnLaneTests.run_with.
func dashboardRunWith(gh *fakes.FakeGitHub, runID int, o dashboardRun) {
	event, status := o.Event, o.Status
	if event == "" {
		event = "schedule"
	}
	if status == "" {
		status = "completed"
	}
	gh.Runs.Set(strconv.Itoa(runID), pyjson.New("id", runID, "workflow", dashboard.LearnWorkflow,
		"event", event, "status", status, "conclusion", "success", "run_number", o.Number,
		"created_at", clock.ISO(testsupport.Night.Add(-24*time.Hour)),
		"html_url", fmt.Sprintf("https://github.com/x/y/actions/runs/%d", runID)))
	jobs := o.Jobs
	if len(jobs) == 0 {
		jobs = []*pyjson.Object{}
	}
	gh.Jobs[strconv.Itoa(runID)] = jobs
}

// dashboardRetrain is LearnLaneTests.retrain: each step (its key, its conclusion), in order.
func dashboardRetrain(steps ...[2]string) []*pyjson.Object {
	names := map[string]string{"fit": "retrain and screen (training/ml/retrain.sh)",
		"gate":    "the gate on main as it is now (training/ml/learn-promote.sh)",
		"deliver": "deliver (push ai/learn, open its pull request, auto-merge)",
		"moving":  "main kept moving, so another run on this export",
		"waits":   "a promotion still open, so this one waits"}
	list := []any{}
	for _, s := range steps {
		list = append(list, pyjson.New("name", names[s[0]], "conclusion", s[1]))
	}
	return []*pyjson.Object{pyjson.New("name", "plan", "conclusion", "success"),
		pyjson.New("name", "export / plan the shards", "conclusion", "success"),
		pyjson.New("name", "retrain, screen, gate", "conclusion", "success", "steps", list)}
}

func TestDashboard_EachWayARunEnds(t *testing.T) {
	outcome := func(jobs []*pyjson.Object, status ...string) string {
		s := "completed"
		if len(status) > 0 {
			s = status[0]
		}
		return dashboard.LearnOutcome(pyjson.New("status", s), jobs)
	}
	job := pyjson.New
	assertEqual(t, outcome([]*pyjson.Object{}, "in_progress"), "🔄 running")
	assertEqual(t, outcome([]*pyjson.Object{job("name", "plan", "conclusion", "success"),
		job("name", "retrain, screen, gate", "conclusion", "skipped")}),
		"⏭️ waited on its open pull request")
	// Before the retrain went on with the shards that finished, a failed shard stopped the run.
	assertEqual(t, outcome([]*pyjson.Object{job("name", "export / shard from 1", "conclusion", "failure",
		"runner_name", "pc-selfplay-1021011405")}),
		"❌ the export failed")
	assertEqual(t, outcome(dashboardRetrain([2]string{"fit", "success"}, [2]string{"gate", "skipped"})),
		"screened out")
	assertEqual(t, outcome(dashboardRetrain([2]string{"fit", "success"}, [2]string{"gate", "success"},
		[2]string{"deliver", "skipped"}, [2]string{"moving", "skipped"})), "gated out")
	assertEqual(t, outcome(dashboardRetrain([2]string{"fit", "success"}, [2]string{"gate", "success"},
		[2]string{"deliver", "success"})), "🎉 promoted")
	assertIn(t, "LEARN_LANE_TOKEN", outcome(dashboardRetrain([2]string{"fit", "success"},
		[2]string{"gate", "success"}, [2]string{"deliver", "failure"})))
	assertEqual(t, outcome(dashboardRetrain([2]string{"fit", "success"}, [2]string{"gate", "success"},
		[2]string{"deliver", "skipped"}, [2]string{"moving", "success"})),
		"🔁 main kept moving: another run on the export")
	// A pass while the lane's last promotion is still open waits behind it, as its artifact.
	assertEqual(t, outcome(dashboardRetrain([2]string{"fit", "success"}, [2]string{"gate", "success"},
		[2]string{"waits", "success"}, [2]string{"deliver", "skipped"}, [2]string{"moving", "skipped"})),
		"⏸️ passed, behind a promotion still open: kept as its artifact")
	assertEqual(t, outcome(dashboardRetrain([2]string{"fit", "success"}, [2]string{"gate", "success"},
		[2]string{"waits", "skipped"}, [2]string{"deliver", "success"})), "🎉 promoted")
	// A run from before the lane gated on the newest main ("the gate, once") reads the same.
	old := dashboardRetrain([2]string{"fit", "success"})
	old[2].Set("steps", append(old[2].List("steps"),
		pyjson.New("name", "the gate, once (promote --lane learn)", "conclusion", "skipped")))
	assertEqual(t, outcome(old), "screened out")
}

// Every job runs on the owner's PC, and GitHub fails a job no runner takes in 24 hours: a job
// that ended with no runner is the PC off, asleep or busy, not the lane broken.
func TestDashboard_ARunThePcsRunnerNeverTookOrTookInPart(t *testing.T) {
	outcome := func(jobs []*pyjson.Object, conclusion ...string) string {
		c := "failure"
		if len(conclusion) > 0 {
			c = conclusion[0]
		}
		return dashboard.LearnOutcome(pyjson.New("status", "completed", "conclusion", c), jobs)
	}
	never := func(name string) *pyjson.Object {
		return pyjson.New("name", name, "conclusion", "cancelled", "runner_name", nil)
	}
	took := func(name string, conclusion ...string) *pyjson.Object {
		c := "success"
		if len(conclusion) > 0 {
			c = conclusion[0]
		}
		return pyjson.New("name", name, "conclusion", c, "runner_name", "pc-selfplay-1021011405")
	}
	skipped := func() *pyjson.Object {
		return pyjson.New("name", "retrain, screen, gate", "conclusion", "skipped")
	}
	assertEqual(t, outcome([]*pyjson.Object{never("plan"), skipped()}), dashboard.LearnNoRunner)
	// The retrain goes on with the shards that finished; with none, the shards step stops it.
	noneFinished := []*pyjson.Object{took("plan"), took("export / plan the shards"),
		never("export / shard from 1"), never("export / shard from 5001"),
		took("retrain, screen, gate", "failure").Set("steps", []any{
			pyjson.New("name", "the export's shards", "conclusion", "failure")})}
	assertEqual(t, outcome(noneFinished), dashboard.LearnNoRunner)
	noneFinished[2] = took("export / shard from 1", "failure")
	assertEqual(t, outcome(noneFinished), "❌ the export failed")
	// Some shards lost, the rest retrained on: the run reads as the retrain ended.
	some := dashboardRetrain([2]string{"fit", "success"}, [2]string{"gate", "skipped"})
	some = slices.Insert(some, 1, never("export / shard from 5001"), took("export / shard from 1"))
	assertEqual(t, outcome(some, "cancelled"), "screened out")
	assertEqual(t, outcome([]*pyjson.Object{took("plan"), skipped()}, "cancelled"), "⏹️ cancelled")
	// Stopped in its export, by a person or a new set (learn-set.yml): cancelled, not lost.
	stopped := []*pyjson.Object{took("plan"), took("export / round 1 / shard from 1"),
		took("export / round 1 / shard from 5001", "cancelled"),
		never("export / round 1 / shard from 10001")}
	assertEqual(t, outcome(stopped, "cancelled"), "⏹️ cancelled")
}

// Each run starts the next as it ends (#1976); the schedule, twice a day, starts the lane again.
func TestDashboard_TheNextScheduledRunIsTheNext0623Or1823UTC(t *testing.T) {
	isoformat := func(at time.Time) string { return at.Format("2006-01-02T15:04:05-07:00") }
	before := testsupport.Night // 2026-09-30 03:00 UTC
	assertEqual(t, isoformat(dashboard.NextLearnRun(before)), "2026-09-30T06:23:00+00:00")
	after := time.Date(2026, 9, 30, 7, 0, 0, 0, time.UTC)
	assertEqual(t, isoformat(dashboard.NextLearnRun(after)), "2026-09-30T18:23:00+00:00")
	at := time.Date(2026, 9, 30, 6, 23, 0, 0, time.UTC)
	assertEqual(t, isoformat(dashboard.NextLearnRun(at)), "2026-09-30T18:23:00+00:00")
	evening := time.Date(2026, 9, 30, 18, 23, 0, 0, time.UTC)
	assertEqual(t, isoformat(dashboard.NextLearnRun(evening)), "2026-10-01T06:23:00+00:00")
}

// dashboardLearnLane is "\n".join(dashboard.learn_lane(make_ctx(gh, at=NIGHT, cfg=make_config()))).
func dashboardLearnLane(gh *fakes.FakeGitHub) string {
	return strings.Join(dashboard.LearnLane(testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night,
		Cfg: testsupport.MakeConfig()}), nil), "\n")
}

func TestDashboard_TheSectionSaysWhatIsOnMainItsPullRequestAndItsRuns(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.Files[[2]string{"main", "crates/ai/generation.json"}] = [2]string{pyjson.Dumps(pyjson.New(
		"generation", 13, "lane", "learn", "date", "2026-09-29", "vsParent", "220/400")), "sha1"}
	pull := gh.AddPull(1477, "ai/learn", fakes.Pull{Title: "AI gen 14 (learn): the learned evaluation"})
	pull.Update(pyjson.New("created_at", clock.ISO(testsupport.Night.Add(-2*time.Hour)),
		"html_url", "https://github.com/x/y/pull/1477"))
	dashboardRunWith(gh, 31, dashboardRun{Number: 20, Jobs: dashboardRetrain([2]string{"fit", "success"},
		[2]string{"gate", "success"}, [2]string{"deliver", "success"})})
	dashboardRunWith(gh, 32, dashboardRun{Number: 21, Event: "workflow_run"})
	dashboardRunWith(gh, 33, dashboardRun{Number: 22, Status: "in_progress"})
	body := dashboardLearnLane(gh)
	assertIn(t, "## 🧠 The learn lane", body)
	assertIn(t, "- **The AI on main:** gen 13 (learn), promoted 2026-09-29, 220/400 against its "+
		"parent.", body)
	assertIn(t, "▶️ [#1477](https://github.com/x/y/pull/1477) open since", body)
	assertIn(t, "CI decides, and auto-merge lands it.", body)
	assertIn(t, "- **Next scheduled run:** Wed Sep 30 01:23 (Central).", body)
	assertIn(t, "| 🎉 promoted | [20](https://github.com/x/y/actions/runs/31) |", body)
	assertIn(t, "| 🔄 running | [22]", body)
	// A watch run (CI on the lane's pull request) is not one of the lane's runs.
	assertNotIn(t, "[21]", body)
}

// A scheduled run queued behind a running one is cancelled when the next schedule queues another
// (the `learn` concurrency group holds one waiting): it never ran, so it is no row, and the table
// still shows `LEARN_ROWS` runs. One cancelled after a job began is a row.
func TestDashboard_AScheduledRunCancelledBeforeItRanIsLeftOut(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	for n := 0; n < dashboard.LearnRows; n++ {
		dashboardRunWith(gh, 40+n, dashboardRun{Number: 10 + n})
	}
	for n := 0; n < 3; n++ {
		dashboardRunWith(gh, 50+n, dashboardRun{Number: 20 + n})
		gh.Runs.Obj(strconv.Itoa(50+n)).Set("conclusion", "cancelled")
	}
	dashboardRunWith(gh, 60, dashboardRun{Number: 30, Jobs: []*pyjson.Object{pyjson.New("name", "plan",
		"conclusion", "cancelled", "runner_name", "pc-selfplay-1")}})
	gh.Runs.Obj("60").Set("conclusion", "cancelled")
	// Newest first, as GitHub lists them.
	keys := gh.Runs.Keys()
	sort.SliceStable(keys, func(i, j int) bool {
		a, _ := strconv.Atoi(keys[i])
		b, _ := strconv.Atoi(keys[j])
		return -a < -b
	})
	sorted := pyjson.New()
	for _, k := range keys {
		sorted.Set(k, gh.Runs.Get(k))
	}
	gh.Runs = sorted
	body := dashboardLearnLane(gh)
	assertIn(t, "| ⏹️ cancelled | [30]", body)
	for _, number := range []int{20, 21, 22} {
		assertNotIn(t, fmt.Sprintf("[%d]", number), body)
	}
	shown := 0
	for n := 10; n < 10+dashboard.LearnRows; n++ {
		if strings.Contains(body, fmt.Sprintf("[%d]", n)) {
			shown++
		}
	}
	assertEqual(t, shown, dashboard.LearnRows-1)
}

func TestDashboard_APromotionOpenTooLongIsOneTheNextRunCloses(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	pull := gh.AddPull(1477, "ai/learn", fakes.Pull{Title: "AI gen 14 (learn): the learned evaluation"})
	pull.Update(pyjson.New("created_at", clock.ISO(testsupport.Night.Add(-dashboard.LearnPRStale-time.Hour))))
	body := dashboardLearnLane(gh)
	assertIn(t, "⛔ [#1477]", body)
	assertIn(t, "the next run closes it.", body)
	assertIn(t, "- **The AI on main:** no generation.json.", body)
	assertIn(t, "No runs yet.", body)
}

// The PC's heartbeat (JACKIOH_PC_GIST): whether it is working, and on which job, and its CPU and
// memory in use, as percentages, and without its runners, which is what decides whether it lends
// itself.
func TestDashboard_TheSectionSaysWhetherTheOwnersPCWorksAndHowLoadedItIs(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	beat := pyjson.New("at", dashboardAgo(1), "calm", true, "cpu", 7.4, "memGB", 11.2,
		"cpuAll", 43.6, "memAllGB", 20.8, "memTotalGB", 31.9, "logical", 16,
		"runners", []any{})

	says := func(changes *pyjson.Object) string {
		t.Helper()
		gh.Gists = dashboardGist(dashboardMerged(beat, changes))
		ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night, Cfg: testsupport.MakeConfig(
			testsupport.Config{Env: map[string]string{"JACKIOH_PC_GIST": "g1"}})})
		body := strings.Join(dashboard.LearnLane(ctx, dashboard.ReadRunners(ctx)), "\n")
		return dashboardBefore(dashboardAfter(t, body, "- **The owner's PC:** "), "\n")
	}

	assertEqual(t, says(pyjson.New()), "calm, lending itself, not working: no job, CPU 44%, memory 65% "+
		"of 32 GB (7% and 35% without its runners) · heard 21:59.")
	runner := pyjson.New("name", "pc-selfplay-1", "label", dashboard.LocalLabel, "busy", false)
	assertIn(t, "not working: a runner waits for a job, CPU 44%", says(pyjson.New("runners", []any{runner})))
	// Busy in the heartbeat before GitHub lists its job: working, on what not yet known.
	busy := dashboardMerged(runner, pyjson.New("busy", true, "cpus", 5.8))
	assertIn(t, "calm, lending itself, working on a job, CPU 44%", says(pyjson.New("runners", []any{busy})))
	gh.Runs.Set("40", pyjson.New("id", 40, "status", "in_progress", "workflow", "selfplay.yml",
		"path", ".github/workflows/selfplay.yml", "created_at", clock.ISO(testsupport.Night)))
	gh.Jobs["40"] = []*pyjson.Object{pyjson.New("name", "export / shard from 1", "status", "in_progress",
		"labels", []any{dashboard.LocalLabel}, "runner_name", "pc-selfplay-1")}
	assertIn(t, "calm, lending itself, working on selfplay (export / shard from 1), CPU 44%",
		says(pyjson.New("runners", []any{busy})))
	assertIn(t, "working on selfplay (export / shard from 1), slowed while the PC is busy, "+
		"CPU 44%", says(pyjson.New("calm", false, "runners",
		[]any{dashboardMerged(busy, pyjson.New("throttled", true))})))
	// Without the runners read, the heartbeat alone; without a heartbeat, why not.
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night, Cfg: testsupport.MakeConfig(
		testsupport.Config{Env: map[string]string{"JACKIOH_PC_GIST": "g1"}})})
	assertIn(t, "working on a job, CPU 44%", strings.Join(dashboard.LearnLane(ctx, nil), "\n"))
	body := dashboardLearnLane(gh)
	assertIn(t, "- **The owner's PC:** no heartbeat set up (JACKIOH_PC_GIST).", body)
}

func TestDashboard_UnreadableGithubStillDrawsTheSection(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.Patch.ListPulls = func(string, string, int) []*pyjson.Object {
		panic(dashboardGitHubError("down", 502))
	}
	gh.Patch.ListRuns = func(string, string, int, bool) []*pyjson.Object {
		panic(dashboardGitHubError("down", 502))
	}
	body := dashboardLearnLane(gh)
	gh.Patch.ListPulls, gh.Patch.ListRuns = nil, nil
	assertIn(t, "- **Its pull request:** could not be read.", body)
	assertIn(t, "Its runs could not be read.", body)
}

func TestDashboard_TheIssueCarriesItAfterTheNightBotsRuns(t *testing.T) {
	ctx := testsupport.MakeCtx(fakes.NewFakeGitHub(), testsupport.Ctx{At: testsupport.Night,
		Cfg: testsupport.MakeConfig()})
	body := dashboardRender(ctx, nil, "")
	assertLess(t, strIndex(t, body, "## 🏃 Last night-bot runs"), strIndex(t, body, "## 🧠 The learn lane"))
}

// ============================================================================ DashboardTests

// dashboardSetUp is DashboardTests.setUp.
func dashboardSetUp() (*fakes.FakeGitHub, *hctx.Context) {
	gh := fakes.NewFakeGitHub()
	return gh, testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night})
}

func TestDashboard_OpenedAndPinnedOnceThenRewrittenInPlace(t *testing.T) {
	gh, ctx := dashboardSetUp()
	note := dashboardUpdate(ctx)
	issues := dashboardOpened(gh)
	if len(issues) != 1 {
		t.Fatalf("status issues: %v, want one", issues)
	}
	number := issues[0]
	assertEqual(t, note, fmt.Sprintf("opened #%d and pinned it", number))
	assertEqual(t, gh.Pinned, []string{fmt.Sprintf("I_%d", number)})
	assertEqual(t, gh.LabelNames(number), map[string]bool{"night bot": true})
	assertIn(t, dashboard.Marker, gh.Threads.Get(number).Str("body"))
	assertTrue(t, pyjson.Equal(ctx.Store.Load().Get("dashboard"), pyjson.New("issue", number)),
		"state's dashboard")
	gh.AddIssue(7, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:easy", "priority:high"}})
	assertEqual(t, dashboardUpdate(ctx), fmt.Sprintf("rewrote #%d", number))
	assertEqual(t, dashboardOpened(gh), []int{number}) // no second issue, no second pin
	assertEqual(t, gh.Pinned, []string{fmt.Sprintf("I_%d", number)})
	body := gh.Threads.Get(number).Str("body")
	row := ""
	for _, line := range py.Splitlines(body) {
		if strings.HasPrefix(line, "| #7 |") {
			row = line
			break
		}
	}
	if row == "" {
		t.Fatalf("no row for #7 in:\n%s", body)
	}
	assertIn(t, "| plan, then build | 🟢 easy → medium<br/><sub>a plan first</sub> | "+
		"▶️ **starts at the next run** on `claude-1` (`sonnet`, medium) | high |", row)
}

func TestDashboard_ALostNumberAdoptsATrustedIssueNeverAStrangers(t *testing.T) {
	gh, ctx := dashboardSetUp()
	body := "old\n\n" + dashboard.Marker
	gh.AddIssue(40, fakes.Issue{Title: dashboard.Title, Body: body, User: fakes.Stranger})
	gh.AddIssue(41, fakes.Issue{Title: dashboard.Title, Body: body, User: fakes.Operator})
	assertEqual(t, dashboardUpdate(ctx), "rewrote #41")
	assertTrue(t, pyjson.Equal(ctx.Store.Load().Get("dashboard"), pyjson.New("issue", 41)),
		"state's dashboard")
	assertEqual(t, gh.Threads.Get(40).Str("body"), body) // the stranger's is left alone
}

func TestDashboard_AClosedIssueIsReplacedAndAFullPinListIsSaid(t *testing.T) {
	gh, ctx := dashboardSetUp()
	gh.AddIssue(41, fakes.Issue{Title: dashboard.Title, Body: dashboard.Marker, User: fakes.Operator,
		State: "closed"})
	flowUpdate(ctx, func(s *pyjson.Object) { s.Set("dashboard", pyjson.New("issue", 41)) }, "state")
	gh.Pinned = []string{"a", "b", "c"}
	note := dashboardUpdate(ctx)
	var others []int
	for _, n := range dashboardOpened(gh) {
		if n != 41 {
			others = append(others, n)
		}
	}
	if len(others) != 1 {
		t.Fatalf("new status issues: %v, want one", others)
	}
	number := others[0]
	assertIn(t, fmt.Sprintf("opened #%d; could not pin it (422)", number), note)
	assertTrue(t, pyjson.Equal(ctx.Store.Load().Get("dashboard"), pyjson.New("issue", number)),
		"state's dashboard")
}

func TestDashboard_TheVisuals(t *testing.T) {
	gh, ctx := dashboardSetUp()
	for _, number := range []int{37, 49} {
		gh.AddIssue(number, fakes.Issue{Labels: []string{config.LabelWorking}})
	}
	gh.Runs.Update(pyjson.New("101", pyjson.New("status", "in_progress"),
		"102", pyjson.New("status", "in_progress")))
	ago := dashboardAgo
	flowUpdate(ctx, func(s *pyjson.Object) {
		hstate.Item(s, 37).Update(pyjson.New("provider", "claude-1", "kind", "build", "run_id", "101",
			"started_at", ago(47)))
		hstate.Item(s, 49).Update(pyjson.New("provider", "muse", "kind", "revise", "run_id", "102",
			"started_at", ago(123)))
		s.Obj("providers").Set("claude-1", pyjson.New("usage", pyjson.New("five_hour",
			pyjson.New("utilization", 0.3))))
	}, "seed")
	gh.AddPull(9, "bot/issue-3", fakes.Pull{Title: "Make it so", Labels: []string{config.LabelPR,
		config.LabelRevise, "difficulty:hard"}})
	gh.Runs.Set("200", pyjson.New("id", 200, "status", "completed", "conclusion", "success",
		"event", "schedule", "created_at", ago(30), "run_number", 812,
		"run_started_at", ago(30), "updated_at", ago(26),
		"html_url", "https://github.com/x/y/actions/runs/200"))
	body := dashboardRender(ctx, nil, "")
	// What each lane is doing, the longest-running first. The start time is a clock time that
	// links to the run; hovering it says how long it had run when this was written.
	runs := "https://github.com/jgoetzmann/JackiOh/actions/runs"
	assertIn(t, "| Item | Title | Doing | Subscription | Started (Central) |", body)
	museRow := "| #49 | An issue | revising | `muse` `muse-spark-1.3-contributor` (machine) " +
		"| [19:57](" + runs + "/102 \"running 2h 03m when this was written at 22:00\") |"
	claudeRow := "| #37 | An issue | building | `claude-1` `opus` (GitHub) " +
		"| [21:13](" + runs + "/101 \"running 47m when this was written at 22:00\") |"
	assertIn(t, museRow, body)
	assertIn(t, claudeRow, body)
	assertLess(t, strIndex(t, body, museRow), strIndex(t, body, claudeRow))
	// The date first, then how far the PC's two local jobs have played (no heartbeat here).
	assertTrue(t, strings.HasPrefix(body, "**Last updated at 2026-09-29 22:00 CDT**\n\n"+
		dashboard.ProgressStart+"\n```mermaid\n"), "the body's start")
	assertIn(t, "<sub>The owner's PC: no heartbeat set up (JACKIOH_PC_GIST).</sub>", body)
	assertIn(t, dashboard.ProgressEnd+"\n\n# 🤖 ", body)
	assertIn(t, "`bot-status` rewrites this issue every 5 minutes", body)
	// A timeline of the runs going now: GitHub's runners, then the machine.
	assertIn(t, "```mermaid\n"+dashboard.TimelineStyle+"\ngantt\n    title Runs going now, "+
		"as of 22:00 (Central time)", body)
	assertIn(t, "    tickInterval 30minute\n    todayMarker off", body)
	assertIn(t, "    section GitHub's runners\n    claude-1 · building 37 · 47m :"+
		"2026-09-29 21:13, 2026-09-29 22:00", body)
	assertIn(t, "    section The night box\n    muse · revising 49 · 2h 03m :", body)
	gantt := dashboardBefore(dashboardAfter(t, body, dashboard.TimelineStyle+"\ngantt"), "```")
	assertNotIn(t, "#", gantt) // a gantt chart reads `#` as a comment
	// Every subscription as a box, with what it does or why not and its usage, always: the
	// Claude accounts, then the night box's.
	assertIn(t, `subgraph hosted["Claude accounts, on GitHub's runners: 1 of 8 working"]`,
		body)
	assertIn(t, `h0["<b>claude-1</b><br/>🟢 building #37 · since 21:13<br/>5h ▰▰▱▱▱ 30%"]`+
		`:::busy`, body)
	assertIn(t, `h1["<b>claude-2</b><br/>⚫ off: no secret set<br/>5h not read yet · cap 90%`+
		`<br/>7d not read yet · cap 90%"]:::off`, body)
	slots := ctx.Cfg.Pool.MachineParallel
	assertIn(t, fmt.Sprintf(`subgraph machine["The night box: 1 of %d slots in use"]`, slots), body)
	assertIn(t, `m3["<b>muse</b><br/>🟢 revising #49 · since 19:57<br/>5h not read yet`+
		`<br/>7d not read yet"]:::busy`, body)
	assertIn(t, "m0 ~~~ m1 ~~~ m2 ~~~ m3", body)
	assertNotIn(t, ":::free", body) // no empty slot is drawn as a free subscription
	// With its LLM lanes stopped (#1677) and gather on the owner's PC, no training box.
	assertNotIn(t, "subgraph training", body)
	assertNotIn(t, "machine ~~~ training", body)
	// GitHub's lanes count claude-1's run; Muse's is in the night box's slots.
	assertIn(t, "Lanes on GitHub's runners: 1 of 8 in use", body)
	assertNotIn(t, "pie", body)
	// Each subscription, with its usage as a bar.
	assertIn(t, "| `claude-1` |", body)
	assertIn(t, "| `claude-1` | 🟢 building #37 |", body)
	assertIn(t, "▰▰▰▱▱▱▱▱▱▱ 30%", body)
	// The queue and the last runs.
	assertIn(t, "| #9 | Make it so | revise | 🔴 hard → strong | ⏳ **a strong model (Opus)**: "+
		"busy `claude-1` | — |", body)
	assertIn(t, "> 🟢 **2 running** · 📋 **1 queued**: 1 waits for a strong model · 🔋 "+
		"**none of 12 subscriptions can start work now**", body)
	assertIn(t, `| <span title="took 4m">Tue 21:30</span> | schedule | ✅ `+
		`| [812](https://github.com/x/y/actions/runs/200) |`, body)
	// The full status, folded.
	assertIn(t, "<details><summary>The full status</summary>", body)
	assertIn(t, "**Night bot status**", body)
}

func TestDashboard_NothingRunning(t *testing.T) {
	_, ctx := dashboardSetUp()
	body := dashboardRender(ctx, nil, "")
	assertIn(t, "Nothing is running right now.", body)
	slots := ctx.Cfg.Pool.MachineParallel
	assertIn(t, fmt.Sprintf("The night box: 0 of %d slots in use", slots), body)
	assertIn(t, "Lanes on GitHub's runners: 0 of 8 in use", body)
	assertIn(t, "Nothing is queued.", body)
}

// `harness dashboard --every 600 --for 1200`, as bot-status runs it (with other figures): a
// rewrite now and every `--every` until the time is up, and a tick that fails is only a warning.
func TestDashboard_TheLoopRewritesItEveryTenMinutesAndOutlivesABadTick(t *testing.T) {
	now := 1000.0
	var ticks []float64

	update := func(*hctx.Context, []string, bool, func() []string, bool, cardsets.Fetch) string {
		ticks = append(ticks, now)
		if len(ticks) == 2 {
			panic(errors.New("network down"))
		}
		return "rewrote #148"
	}

	patch[dashboardUpdateFunc](t, &dashboard.Update, update)
	dashboardOffline(t, nil)
	patch(t, &cli.Monotonic, func() float64 { return now })
	patch(t, &cli.Sleep, func(seconds float64) { now += seconds })
	out, _ := capture(t, &cli.Stdout)
	code := cli.CmdDashboard(testsupport.MakeConfig(), &cli.Args{Every: 600, ForSeconds: 1200})
	assertEqual(t, code, 0)
	assertEqual(t, ticks, []float64{1000.0, 1600.0, 2200.0})
	assertIn(t, "::warning::the status issue was not updated: network down", out.String())
	assertEqual(t, strings.Count(out.String(), "dashboard: rewrote #148"), 2)
}

// `--sweep`: each tick runs the sweep first (it starts a night run when a lane and work are
// free), so a broken chain of runs restarts within ten minutes; a failed sweep is only a warning
// and the rewrite still happens.
func TestDashboard_TheLoopSweepsBeforeEachRewrite(t *testing.T) {
	now := 0.0
	type step struct {
		what string
		at   float64
	}
	var order []step

	sweep := func(*hctx.Context) []string {
		order = append(order, step{"sweep", now})
		if len(order) == 1 {
			panic(errors.New("rate limited"))
		}
		return []string{"started a night run"}
	}

	update := func(*hctx.Context, []string, bool, func() []string, bool, cardsets.Fetch) string {
		order = append(order, step{"rewrite", now})
		return "rewrote #148"
	}

	patch(t, &cli.Sweep, sweep)
	dashboardOffline(t, nil)
	patch[dashboardUpdateFunc](t, &dashboard.Update, update)
	patch(t, &cli.Monotonic, func() float64 { return now })
	patch(t, &cli.Sleep, func(s float64) { now += s })
	out, _ := capture(t, &cli.Stdout)
	cli.CmdDashboard(testsupport.MakeConfig(), &cli.Args{Every: 600, ForSeconds: 600, Sweep: true})
	assertEqual(t, order, []step{{"sweep", 0.0}, {"rewrite", 0.0}, {"sweep", 600.0}, {"rewrite", 600.0}})
	assertIn(t, "::warning::the sweep failed: rate limited", out.String())
	assertIn(t, "sweep: started a night run", out.String())
}

// `--stats` puts them in the status issue at each rewrite (counted when due); `harness stats
// --force` rewrites the issue now with them counted afresh.
func TestDashboard_TheLoopCarriesTheStatisticsAndHarnessStatsCountsThemNow(t *testing.T) {
	type call struct {
		stats, forceStats bool
		readCards         cardsets.Fetch
	}
	var calls []call

	update := func(_ *hctx.Context, _ []string, withStats bool, _ func() []string, forceStats bool,
		readCards cardsets.Fetch) string {
		calls = append(calls, call{withStats, forceStats, readCards})
		return "rewrote #148"
	}

	patch[dashboardUpdateFunc](t, &dashboard.Update, update)
	dashboardOffline(t, nil)
	out, _ := capture(t, &cli.Stdout)
	cli.CmdDashboard(testsupport.MakeConfig(), &cli.Args{Stats: true})
	cli.CmdStats(testsupport.MakeConfig(), &cli.Args{Force: true})
	var flags [][2]bool
	for _, c := range calls {
		flags = append(flags, [2]bool{c.stats, c.forceStats})
	}
	assertEqual(t, flags, [][2]bool{{true, false}, {true, true}})
	// Both read which set's cards win most from production (cardsets.py).
	fetchJSON := reflect.ValueOf(cardsets.FetchJSON).Pointer()
	var reads []bool
	for _, c := range calls {
		reads = append(reads, c.readCards != nil && reflect.ValueOf(c.readCards).Pointer() == fetchJSON)
	}
	assertEqual(t, reads, []bool{true, true}, "read_cards is cardsets.fetch_json")
	assertIn(t, "dashboard: rewrote #148", out.String())
}

// dashboardHeldBy is why `provider` may not start a run (`waiting.HeldBy`), the kind and text a
// box without a run shows, or two empty strings when it may start one.
func dashboardHeldBy(ctx *hctx.Context, st *pyjson.Object, provider *providers.Provider) [2]string {
	if held := waiting.HeldBy(ctx, st, provider, false, true); held != nil {
		return [2]string{held.Kind, held.Text}
	}
	return [2]string{}
}

// A Claude account with no run: open, at a cap, outside its hours, resting after a run that
// could not work, or off, each with the time it changes.
func TestDashboard_EachIdleAccountSaysWhyInItsBox(t *testing.T) {
	gh, _ := dashboardSetUp()
	later := clock.ISO(testsupport.Night.Add(70 * time.Minute))
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night,
		Cfg: testsupport.MakeConfig(testsupport.Config{Env: crossAll})}) // every secret set

	flowUpdate(ctx, func(s *pyjson.Object) {
		s.Obj("providers").Set("claude-2", pyjson.New("usage", pyjson.New("five_hour",
			pyjson.New("utilization", 0.95, "resets_at", later))))
		s.Obj("providers").Set("claude-3", pyjson.New("infra", pyjson.New(
			"at", clock.ISO(testsupport.Night.Add(-10*time.Minute)), "reason", "401", "streak", 2)))
	}, "seed")
	st := ctx.Store.Load()
	pool := ctx.Cfg.Pool
	assertEqual(t, dashboardHeldBy(ctx, st, pool.Get("claude-1")), [2]string{})
	assertEqual(t, dashboardHeldBy(ctx, st, pool.Get("claude-2")),
		[2]string{"capped", "⏸️ 5-hour 95% of its 90% cap<br/>starts again 23:10"})
	assertEqual(t, dashboardHeldBy(ctx, st, pool.Get("claude-3")),
		[2]string{"broken", "⛔ its last run could not work<br/>tries again 23:50"})
	day := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night.Add(12 * time.Hour), Cfg: ctx.Cfg})
	assertEqual(t, dashboardHeldBy(day, day.Store.Load(), pool.Get("claude-1")),
		[2]string{"closed", "🌙 outside its hours<br/>opens 21:00"})
	unset := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night, Cfg: testsupport.MakeConfig(
		testsupport.Config{Env: map[string]string{"HARNESS_SECRETS_SET": ""}})})
	assertEqual(t, dashboardHeldBy(unset, st, unset.Cfg.Pool.Get("claude-4")),
		[2]string{"off", "⚫ off: no secret set"})
}

// claude-4 and claude-6 at 01:00, outside their hours: under their 50% cap there but not
// `start_headroom` under it, so the planner starts no build or plan on them. Their boxes and rows
// say so, and when each can start one again; the usage cells carry the caps in and outside their
// hours, the line a build starts under and the reset; and the full status says the same.
func TestDashboard_AnAccountTooCloseToItsCapToStartIsPausedNotOpen(t *testing.T) {
	gh, _ := dashboardSetUp()
	cfg := testsupport.MakeConfig(testsupport.Config{Env: crossAll, CommittedHours: true})
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: providersEarly, Cfg: cfg})
	flowUpdate(ctx, func(s *pyjson.Object) {
		s.Obj("providers").Update(providersTooCloseToStart().Obj("providers"))
	}, "seed")
	st := ctx.Store.Load()
	pool := cfg.Pool
	assertEqual(t, dashboardHeldBy(ctx, st, pool.Get("claude-4")),
		[2]string{"reviews", "⏸️ 5-hour 49% of its 50% cap<br/>long runs start under 45%; " +
			"reviews only<br/>starts again 03:00"})
	assertEqual(t, dashboardHeldBy(ctx, st, pool.Get("claude-6")),
		[2]string{"reviews", "⏸️ 5-hour 45% of its 50% cap<br/>long runs start under 45%; " +
			"reviews only<br/>starts again 01:50"})
	body := dashboardRender(ctx, nil, "")
	beforeQueue := dashboardBefore(body, "## 📋 Queue")
	assertNotIn(t, "⚪ idle", py.SliceTo(dashboardAfter(t, beforeQueue, "claude-4"), 80))
	assertIn(t, "| Subscription | Now | 5-hour session | 7-day week | Read | Models (tier) "+
		"| Hours |", body)
	assertIn(t, "| `claude-4` | ⏸️ 5-hour 49% of its 50% cap<br/>long runs start under 45%; "+
		"reviews only<br/>starts again 03:00 | ▰▰▰▰▰▱▱▱▱▱ 49%<br/><sub>cap 50% now, "+
		"70% in its hours · builds and plans start under 45% · resets 04:10</sub> "+
		"| —<br/><sub>not reported</sub> | "+
		`<span title="read at Thu 23:37">1h 22m ago</span> |`, body)
	assertIn(t, "`claude-4` (claude: `opus` strong, `sonnet` medium; 03:00–15:00 "+
		"America/Chicago, outside them up to 50% of 5-hour): 5-hour usage is 49%, "+
		"too close to its 50% cap outside its hours", body)
	assertIn(t, "Usage: 5-hour 49% (cap 50% now, 70% in its hours; builds and plans start "+
		"under 45%; resets in 3h 10m) (read 1h 22m ago).", body)
	// In its hours the other cap is the one outside them; an uncapped account says so.
	day := testsupport.MakeCtx(gh, testsupport.Ctx{At: providersEarly.Add(3 * time.Hour), Cfg: cfg})
	assertEqual(t, status.CapText(day, pool.Get("claude-4"), "five_hour", nil),
		"cap 70% now, 50% outside its hours")
	assertEqual(t, status.CapText(day, pool.Get("claude-3"), "five_hour", nil), "no cap")
	// A refusal already names its reset: no second time for it.
	assertIn(t, "claude-3` (claude: `opus` strong, `sonnet` medium; any time): it refused "+
		"a call; its limit resets at 2026-10-09T12:00:00Z. Usage:", body)
}

// The lanes section: each subscription's lanes, how many its runs hold and how many are open, the
// Claude accounts by number, then agy, gpt and Muse, then any other by name (the tests' weak one),
// then Squishy's from the comment its section carries; Muse's fill-in counts in the lane it fills,
// and a planning run of claude-3's (no caps) on the planning lane beside its own.
func TestDashboard_EverySubscriptionsLanesWithTheTotals(t *testing.T) {
	gh, _ := dashboardSetUp()
	cfg := testsupport.MakeConfig(testsupport.Config{Env: providersSecrets(providers.SecretNames...),
		Machine: testsupport.AllMachine, CommittedPlanLanes: true})
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night, Cfg: cfg})
	runs := []struct {
		number   int
		provider string
		extra    *pyjson.Object
	}{{37, "claude-8", pyjson.New()}, {38, "claude-8", pyjson.New()},
		{39, "claude-3", pyjson.New("action", "plan")}, {49, "muse", pyjson.New()},
		{50, "muse", pyjson.New("runs_on", "night-vm-muse-fill")}}
	for _, r := range runs {
		gh.AddIssue(r.number, fakes.Issue{Labels: []string{config.LabelWorking}})
		gh.Runs.Set(strconv.Itoa(r.number), pyjson.New("status", "in_progress"))
		flowUpdate(ctx, func(s *pyjson.Object) {
			hstate.Item(s, r.number).Update(pyjson.New("provider", r.provider, "kind", "build",
				"run_id", strconv.Itoa(r.number)).Update(r.extra))
		}, "state")
	}
	squishy := pyjson.New("id", "claude-squishy", "bot", "Squishy", "claude", true, "github", true,
		"where", "GitHub's runners", "lanes", 2, "used", 1, "runs", 1,
		"secret", "CLAUDE_CODE_OAUTH_TOKEN_SQUISHY", "startable", true)
	part := dashboard.SectionStart + "\n" + dashboard.LanesMark + pyjson.Dumps([]any{squishy}) + " -->\n" +
		"## 🫧 Squishy\n" + dashboard.SectionEnd
	body := dashboardRender(ctx, []string{part}, "")
	lanes := body[strIndex(t, body, "## 🛣️ Lanes"):strIndex(t, body, "## 📋 Queue")]
	var rows []string
	for _, line := range py.Splitlines(lanes) {
		if strings.HasPrefix(line, "| ") {
			rows = append(rows, line)
		}
	}
	if len(rows) > 0 {
		rows = rows[1:]
	}
	want := []string{
		"| `claude-1` | GitHub's runners | 1 | 0 | 1 |",
		"| `claude-2` | GitHub's runners | 1 | 0 | 1 |",
		"| `claude-3` | GitHub's runners | 2 | 0 <sub>(+1 planning, on the planning lane)</sub>" +
			" | 2 |"}
	for _, n := range []int{4, 5, 6, 7} {
		want = append(want, fmt.Sprintf("| `claude-%d` | GitHub's runners | 1 | 0 | 1 |", n))
	}
	want = append(want,
		"| `claude-8` | GitHub's runners | 2 | 2 | 0 |",
		"| `agy` | the night box | 1 | 0 | 1 |",
		"| `gpt` | the night box | 1 | 1 <sub>(1 of them `muse`'s)</sub> | 0 |",
		"| `muse` | the night box and the Muse box | 4 | 1 <sub>(+1 filling in `gpt`'s or "+
			"`agy`'s)</sub> | 3 |",
		"| `lite` | `night-vm-lite` | 6 | 0 | 6 |",
		"| `claude-squishy` (Squishy) | GitHub's runners | 2 | 1 | 1 |",
		"| **All 9 Claude accounts** | | **12** | **3** | **9** |",
		"| **All 13 subscriptions** | | **24** | **5** | **19** |")
	assertEqual(t, rows, want)
	// Under them, the limits on the lanes, each with how much of it is held now.
	var limits []string
	for _, line := range py.Splitlines(lanes) {
		if strings.HasPrefix(line, "- **") {
			limits = append(limits, line)
		}
	}
	var names []string
	for _, line := range limits {
		names = append(names, strings.Split(line, "**")[1])
	}
	assertEqual(t, names, []string{"Each account:", "Accounts:", "At once:", "Planning:", "The AWS boxes:",
		"Usage and hours:", "Now:"})
	if len(limits) != 7 {
		t.FailNow()
	}
	assertIn(t, "the code sets no top", limits[0])
	assertIn(t, "the workflows hand over 9 Claude secrets (`providers.SECRETS`), one an "+
		"account: 8 the night bot's, 1 Squishy's, so none is free: another account "+
		"needs a new secret there and in the workflows.", limits[1])
	assertIn(t, "GitHub's runners take 8 of the bots' runs, planning runs too (4 now: 3 the "+
		"night bot's, 1 Squishy's), the organization's 20 hosted jobs less the 12 "+
		"kept for CI (#1455). So the 12 Claude lanes never all run at once: past 8,",
		limits[2])
	assertIn(t, "runs 6 plans at once beside the accounts' own lanes (1 now)", limits[3])
	assertIn(t, "6 runs at once (2 now)", limits[4])
	assertTrue(t, strings.HasSuffix(limits[6],
		"9 Claude lanes are open on accounts that may start work, and GitHub's runners have "+
			"4 free, so there is room for 4 more Claude runs."), limits[6])
	// A subscription switched off has no open lane, and the totals say how many are off; with no
	// Squishy row, its secret is the one free.
	ctx = testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night}) // only claude-1's secret, the machine's off
	table := strings.Join(dashboard.LanesTable(ctx, ctx.Store.Load(), pyjson.New(), nil, ""), "\n")
	assertIn(t, "| `claude-8` | GitHub's runners | 2 | 0 | ⚫ off |", table)
	assertIn(t, "| **All 8 Claude accounts** (7 off) | | **10** | **0** | **1** |", table)
	assertIn(t, "one an account: 8 the night bot's; free: `CLAUDE_CODE_OAUTH_TOKEN_SQUISHY`.",
		table)
	assertIn(t, "**Now:** 1 Claude lane is open on accounts that may start work, and GitHub's "+
		"runners have 8 free, so there is room for 1 more Claude run.", table)
	// Halted, no lane may start work.
	table = strings.Join(dashboard.LanesTable(ctx, ctx.Store.Load(), pyjson.New(), nil, "HALT is on main"),
		"\n")
	assertIn(t, "**Now:** 0 Claude lanes are open on accounts that may start work", table)
}

func TestDashboard_ABotsSectionCarriesItsLanesAndABrokenCommentAddsNone(t *testing.T) {
	_, ctx := dashboardSetUp()
	section := dashboard.Section(ctx)
	var found []*pyjson.Object
	for _, r := range dashboard.MarkedLanes([]string{section}) {
		if pyjson.Equal(r.Get("id"), "claude-8") {
			found = append(found, r)
		}
	}
	if len(found) != 1 {
		t.Fatalf("claude-8's rows: %d, want one", len(found))
	}
	row := found[0]
	got := []any{row.Get("bot"), row.Get("lanes"), row.Get("used"), row.Get("off"), row.Get("startable"),
		row.Get("secret")}
	want := []any{py.Str(config.Title), 2, 0, true, false, "CLAUDE_CODE_OAUTH_TOKEN_8"}
	assertTrue(t, pyjson.Equal(got, want), fmt.Sprintf("%#v != %#v", got, want))
	assertLess(t, strIndex(t, section, dashboard.LanesMark), strIndex(t, section, "## 🫧"))
	for _, broken := range []string{"[{]", `{"id": "x"}`, `[{"lanes": 2}]`, `[{"id": "x", "lanes": "two"}]`} {
		assertEqual(t, len(dashboard.MarkedLanes([]string{dashboard.LanesMark + broken + " -->"})), 0, broken)
	}
	assertEqual(t, len(dashboard.MarkedLanes([]string{dashboard.LanesMark + "[]"})), 0)
}

func TestDashboard_ACellStaysOneLineWithoutPipes(t *testing.T) {
	assertEqual(t, dashboard.Cell("a | b\nc", 70), "a \\| b c")
	assertEqual(t, dashboard.Bar(nil, dashboard.BarCells), "—")
	full := 1.4
	assertEqual(t, dashboard.Bar(&full, dashboard.BarCells), strings.Repeat("▰", 10)+" 100%")
}

func TestDashboard_AFailureNeverFailsTheSweep(t *testing.T) {
	broken := func(*hctx.Context, []string, bool, func() []string, bool, cardsets.Fetch) string {
		panic(dashboardGitHubError("server error", 502))
	}
	cfg := testsupport.MakeConfig()
	patch[dashboardUpdateFunc](t, &dashboard.Update, broken)
	dashboardOffline(t, nil)
	out, _ := capture(t, &cli.Stdout)
	assertEqual(t, cli.CmdDashboard(cfg, &cli.Args{}), 0)
	assertIn(t, "::warning::the status issue was not updated", out.String())
}

// ============================================================================ StatisticsTests

// The statistics live in the status issue (#148; they had an issue of their own, #190): counted
// afresh every 30 minutes, kept as they are between counts, and cut to fit GitHub's limit on an
// issue's body.

// dashboardNoCount is mock.patch.object(stats, "collect", side_effect=AssertionError("counted")),
// until `restore`.
func dashboardNoCount(t *testing.T) (restore func()) {
	return patch(t, &stats.Collect, func(*hctx.Context) *stats.Facts {
		panic(errors.New("counted"))
	})
}

func TestDashboard_CountedEvery30MinutesAndKeptBetween(t *testing.T) {
	gh, ctx := statsWorld()
	note := dashboardUpdateStats(ctx, false)
	issues := dashboardOpened(gh)
	if len(issues) != 1 {
		t.Fatalf("status issues: %v, want one", issues)
	}
	number := issues[0]
	bodyOf := func() string { return gh.Threads.Get(number).Str("body") }
	assertEqual(t, note, fmt.Sprintf("opened #%d and pinned it, with the statistics counted afresh", number))
	body := bodyOf()
	block := stats.Kept(body)
	assertIn(t, "## 📊 What it has done", block)
	assertIn(t, "| Runs started | 4 | 5 | 6 | 6 |", block)
	assertLess(t, strIndex(t, body, "## 🏃 Last night-bot runs"), strIndex(t, body, stats.BlockStart))
	assertLess(t, strIndex(t, body, stats.BlockEnd), strIndex(t, body, dashboard.Marker))
	assertTrue(t, pyjson.Equal(ctx.Store.Load().Get("statistics"),
		pyjson.New("at", clock.ISO(testsupport.Day))), "state's statistics")
	// Twenty minutes on, nothing is counted: the block the issue holds is kept as it is.
	soon := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day.Add(20 * time.Minute)})
	restore := dashboardNoCount(t)
	assertEqual(t, dashboardUpdateStats(soon, false), fmt.Sprintf("rewrote #%d", number))
	restore()
	assertEqual(t, stats.Kept(bodyOf()), block)
	assertIn(t, "Last updated at 2026-09-29 12:20 CDT", bodyOf())
	// Forced, or once 30 minutes have passed, they are counted afresh.
	assertTrue(t, strings.HasSuffix(dashboardUpdateStats(soon, true), "afresh"))
	later := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day.Add(51 * time.Minute)})
	assertTrue(t, strings.HasSuffix(dashboardUpdateStats(later, false), "afresh"))
	// No issue of their own: one pinned issue, and no "Night bot statistics" in it.
	assertEqual(t, dashboardOpened(gh), []int{number})
	assertEqual(t, gh.Pinned, []string{fmt.Sprintf("I_%d", number)})
	for _, thread := range gh.Threads.Values() {
		assertTrue(t, !pyjson.Equal(thread.Get("title"), "Night bot statistics"), "a statistics issue")
	}
	// Without --stats (bot-commands.yml's rewrite after a sweep) nothing is counted, and the
	// block the issue holds is kept rather than wiped until the loop's next tick.
	held := stats.Kept(bodyOf())
	restore = dashboardNoCount(t)
	assertEqual(t, dashboardUpdate(later), fmt.Sprintf("rewrote #%d", number))
	restore()
	assertEqual(t, stats.Kept(bodyOf()), held)
	// An issue that holds none carries none.
	gh.Threads.Get(number).Set("body", "no statistics")
	dashboardUpdate(later)
	assertEqual(t, stats.Kept(bodyOf()), "") // None
}

// Between counts the issue keeps its statistics; when more runners are busy than the issue has
// room for, the full status is left out, then fewer are listed, rather than the statistics
// counted again.
func TestDashboard_MoreRunnersBusyAreCutToTheRoomBeforeACount(t *testing.T) {
	gh, ctx := statsWorld()
	dashboardUpdateStats(ctx, false)
	issues := dashboardOpened(gh)
	if len(issues) != 1 {
		t.Fatalf("status issues: %v, want one", issues)
	}
	number := issues[0]
	for n := 0; n < 12; n++ {
		id := strconv.Itoa(900 + n)
		gh.Runs.Set(id, pyjson.New("id", 900+n, "status", "in_progress",
			"path", ".github/workflows/ci.yml", "event", "push",
			"display_title", "Patch: something", "created_at", clock.ISO(testsupport.Day)))
		gh.Jobs[id] = []*pyjson.Object{pyjson.New("name", fmt.Sprintf("shard %d", n), "status", "in_progress",
			"labels", []any{"ubuntu-latest"}, "runner_name", fmt.Sprintf("GitHub Actions %d", n))}
	}
	soon := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day.Add(20 * time.Minute)})
	kept := stats.Kept(gh.Threads.Get(number).Str("body"))
	full := dashboard.Render(soon, nil, kept, nil, true)
	restoreCount, restoreMax := dashboardNoCount(t), patch(t, &dashboard.MaxBody, py.Len(full)-1)
	assertEqual(t, dashboardUpdateStats(soon, false), fmt.Sprintf("rewrote #%d", number))
	restoreCount()
	restoreMax()
	body := gh.Threads.Get(number).Str("body")
	assertIn(t, dashboard.FullLeftOut, body)
	assertNotIn(t, "…and ", body)
	assertEqual(t, stats.Kept(body), kept)
	lean := dashboard.Render(soon, nil, kept, nil, false)
	restoreCount, restoreMax = dashboardNoCount(t), patch(t, &dashboard.MaxBody, py.Len(lean)-500)
	assertEqual(t, dashboardUpdateStats(soon, false), fmt.Sprintf("rewrote #%d", number))
	restoreCount()
	restoreMax()
	body = gh.Threads.Get(number).Str("body")
	assertIn(t, "…and ", body)
	assertEqual(t, stats.Kept(body), kept)
}

func TestDashboard_ABodyTooLongForGithubListsFewerMergesThenNone(t *testing.T) {
	gh, ctx := statsWorld()
	for n := 100; n < 160; n++ {
		statsMergedPull(gh, n, 3, statsMerged{Opened: "2026-09-29T12:00:00Z", Merged: "2026-09-29T15:00:00Z",
			Additions: 1, Deletions: 1})
	}
	facts := stats.Collect(ctx)
	view := dashboard.Look(ctx)
	full := dashboard.Render(ctx, nil, statsBlock(ctx, facts), view, true)
	assertIn(t, "<summary>The full status</summary>", full)

	draw := func(shown int) string { return stats.Block(ctx, facts, shown, nil, "") }
	// The full status goes first, every merge still listed: it is the sections' facts again.
	restore := patch(t, &dashboard.MaxBody, py.Len(full)-1)
	body := dashboard.Fit(ctx, view, nil, draw)
	restore()
	assertNotIn(t, "<summary>The full status</summary>", body)
	assertIn(t, dashboard.FullLeftOut, body)
	assertTrue(t, strings.HasSuffix(body, dashboard.Marker))
	assertIn(t, fmt.Sprintf("_… and %d older ones._", 64-stats.MergesShown), body)
	// Then fewer merges.
	lean := dashboard.Render(ctx, nil, statsBlock(ctx, facts), view, false)
	restore = patch(t, &dashboard.MaxBody, py.Len(lean)-1)
	body = dashboard.Fit(ctx, view, nil, draw)
	restore()
	assertIn(t, fmt.Sprintf("_… and %d older ones._", 64-py.FloorDiv(stats.MergesShown, 2)), body)
	// Then fewer busy runners, with no merge listed, before the statistics go.
	busy := pyjson.New("name", "work", "status", "in_progress", "labels", []any{"ubuntu-latest"},
		"runner_name", "GitHub Actions 1")
	gh.Runs.Set("900", pyjson.New("id", 900, "status", "in_progress", "path", "x/ci.yml", "event", "push"))
	gh.Jobs["900"] = []*pyjson.Object{}
	for n := 0; n < 6; n++ {
		gh.Jobs["900"] = append(gh.Jobs["900"], busy.Copy().Set("runner_name", fmt.Sprintf("GitHub Actions %d", n)))
	}
	view = dashboard.Look(ctx)
	view.Found = dashboard.ReadRunners(ctx)
	view.Runners = dashboard.RunnersSection(ctx, view.State, view.Found, dashboardRoom)
	bare := dashboard.Render(ctx, nil, stats.Block(ctx, facts, 0, nil, ""), view, false)
	restore = patch(t, &dashboard.MaxBody, py.Len(bare)-1)
	body = dashboard.Fit(ctx, view, nil, draw)
	restore()
	assertTrue(t, regexp.MustCompile(`\n…and \d more\.\n`).MatchString(body), "no `…and N more.`")
	assertTrue(t, py.Len(body) <= py.Len(bare)-1, "the body is not shorter")
	assertTrue(t, stats.Kept(body) != "", "the statistics are left out")
	view.Runners = dashboard.RunnersSection(ctx, view.State, view.Found, 0)
	least := dashboard.Render(ctx, nil, stats.Block(ctx, facts, 0, nil, ""), view, false)
	restore = patch(t, &dashboard.MaxBody, py.Len(least)-1)
	body = dashboard.Fit(ctx, view, nil, draw)
	restore()
	assertEqual(t, stats.Kept(body), "") // left out last of all
	assertTrue(t, strings.HasSuffix(body, dashboard.Marker))
}

// ============================================================================ FreshEachTickTests

// The loop's job runs for hours, and GitHub fixes its secrets when its run is created (hours
// before it starts, for a run queued behind the last loop): each tick reads the subscriptions from
// the default branch again and takes the newest plan job's record of the secrets.

// dashboardLoop is FreshEachTickTests.loop: the config each rewrite saw, and what the loop
// printed. A nil `between` does nothing.
func dashboardLoop(t *testing.T, gh *fakes.FakeGitHub, cfg *config.Config, ticks int,
	between func(tick int)) ([]*config.Config, string) {
	var seen []*config.Config
	now := 0.0

	update := func(ctx *hctx.Context, _ []string, _ bool, _ func() []string, _ bool, _ cardsets.Fetch) string {
		seen = append(seen, ctx.Cfg)
		if between != nil {
			between(len(seen))
		}
		return "rewrote #148"
	}

	defer patch[dashboardUpdateFunc](t, &dashboard.Update, update)()
	defer dashboardOffline(t, gh)()
	defer patch(t, &cli.Monotonic, func() float64 { return now })()
	defer patch(t, &cli.Sleep, func(s float64) { now += s })()
	out, restore := capture(t, &cli.Stdout)
	defer restore()
	cli.CmdDashboard(cfg, &cli.Args{Every: 600, ForSeconds: 600 * (ticks - 1)})
	return seen, out.String()
}

// dashboardHas is Secrets.has as Python's True, False or None.
func dashboardHas(secrets *providers.Secrets, name string) any {
	if has := secrets.Has(name); has != nil {
		return *has
	}
	return nil
}

func TestDashboard_ASecretAPlanJobSawAfterThisRunWasCreatedShowsAtTheNextTick(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.Runs.Set("777", pyjson.New("status", "in_progress", "created_at", "2026-09-30T01:00:00Z"))
	cfg := testsupport.MakeConfig(testsupport.Config{Env: map[string]string{
		"HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN"}})
	store := testsupport.MakeCtx(gh, testsupport.Ctx{Cfg: cfg}).Store
	// A record older than this run says nothing new: its own list stands.
	store.Update(func(s *pyjson.Object) any {
		s.Set("secrets", pyjson.New("set", []any{"CLAUDE_CODE_OAUTH_TOKEN"}, "at", "2026-09-30T00:30:00Z"))
		return nil
	}, "seed")

	addToken4 := func(tick int) {
		if tick == 1 { // a plan job, after this run was created, has token 4
			store.Update(func(s *pyjson.Object) any {
				s.Set("secrets", pyjson.New(
					"set", []any{"CLAUDE_CODE_OAUTH_TOKEN", "CLAUDE_CODE_OAUTH_TOKEN_4"},
					"at", "2026-09-30T02:00:00Z"))
				return nil
			}, "plan")
		}
	}

	seen, _ := dashboardLoop(t, gh, cfg, 2, addToken4)
	var has4 []any
	for _, c := range seen {
		has4 = append(has4, dashboardHas(c.Secrets, "CLAUDE_CODE_OAUTH_TOKEN_4"))
	}
	assertEqual(t, has4, []any{false, true})
	for _, c := range seen {
		assertEqual(t, dashboardHas(c.Secrets, "CLAUDE_CODE_OAUTH_TOKEN"), any(true))
	}
}

func TestDashboard_ASubscriptionChangedOnMainShowsAtTheNextTick(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	raw := testsupport.RawProviders()
	raw.Obj("providers").Obj("claude-4").Obj("limits").Set("five_hour", 0.33)

	changeMain := func(tick int) {
		if tick == 1 {
			gh.Files[[2]string{"main", ".harness/providers.json"}] = [2]string{pyjson.Dumps(raw), "b1"}
		}
		if tick == 2 {
			gh.Files[[2]string{"main", ".harness/providers.json"}] = [2]string{"{not json", "b2"}
		}
	}

	seen, out := dashboardLoop(t, gh, testsupport.MakeConfig(), 3, changeMain)
	var caps []any
	for _, c := range seen {
		caps = append(caps, c.Pool.Get("claude-4").Limits.Stops().Get("five_hour"))
	}
	if len(caps) != 3 {
		t.Fatalf("ticks: %d, want 3", len(caps))
	}
	// The file is not on main in the first tick; then the change; then a broken file keeps it.
	assertTrue(t, pyjson.Equal(caps[1:], []any{0.33, 0.33}), fmt.Sprintf("caps %#v", caps))
	assertTrue(t, !pyjson.Equal(caps[0], 0.33), fmt.Sprintf("caps %#v", caps))
	assertIn(t, "::warning::kept the subscriptions and secrets as they were", out)
}

// A subscription on main that only newer code reads (claude-7's new secret, which kept a loop
// started before it drawing six accounts): the loop ends, without a rewrite, and the next one
// starts on main's code.
func TestDashboard_SubscriptionsThisCheckoutCannotReadEndTheLoop(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	raw := testsupport.RawProviders()
	raw.Obj("providers").Obj("claude-4").Set("secret", "CLAUDE_CODE_OAUTH_TOKEN_99")

	changeMain := func(tick int) {
		if tick == 1 {
			gh.Files[[2]string{"main", ".harness/providers.json"}] = [2]string{pyjson.Dumps(raw), "b1"}
		}
	}

	seen, out := dashboardLoop(t, gh, testsupport.MakeConfig(), 3, changeMain)
	assertEqual(t, len(seen), 1)
	assertIn(t, "::notice::this checkout cannot read the subscriptions on main", out)
	assertIn(t, "CLAUDE_CODE_OAUTH_TOKEN_99", out)
	assertNotIn(t, "::warning::kept the subscriptions", out)
}

func TestDashboard_APlanJobRecordsTheSecretsItSaw(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	both := "CLAUDE_CODE_OAUTH_TOKEN CLAUDE_CODE_OAUTH_TOKEN_5"
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{Cfg: testsupport.MakeConfig(testsupport.Config{
		Env: map[string]string{"HARNESS_SECRETS_SET": both}})})
	plan.Make(ctx, false, nil, "auto", nil)
	got := ctx.Store.Load().Get("secrets")
	want := pyjson.New("set", []any{"CLAUDE_CODE_OAUTH_TOKEN", "CLAUDE_CODE_OAUTH_TOKEN_5"},
		"at", clock.ISO(testsupport.Night))
	assertTrue(t, pyjson.Equal(got, want), fmt.Sprintf("%s != %s", pyjson.Dumps(got), pyjson.Dumps(want)))
}

func TestDashboard_TheRecordAndWhichListWins(t *testing.T) {
	night := testsupport.Night
	known := &providers.Secrets{Present: map[string]bool{"A": true}, Known: true}
	record := providers.SecretsRecord(known, pyjson.New(), night)
	assertTrue(t, pyjson.Equal(record, pyjson.New("set", []any{"A"}, "at", clock.ISO(night))),
		pyjson.Dumps(record))
	st := pyjson.New("secrets", record)
	// The same list is written again only every SECRETS_NOTE_EVERY; a new one at once.
	assertTrue(t, providers.SecretsRecord(known, st, night.Add(10*time.Minute)) == nil, "not None")
	assertTrue(t, providers.SecretsRecord(known, st, night.Add(providers.SecretsNoteEvery)) != nil, "None")
	assertTrue(t, providers.SecretsRecord(&providers.Secrets{Present: map[string]bool{"A": true, "B": true},
		Known: true}, st, night.Add(time.Minute)) != nil, "None")
	// Run by hand, nothing is known, so nothing is written.
	assertTrue(t, providers.SecretsRecord(&providers.Secrets{Present: map[string]bool{}, Known: false},
		pyjson.New(), night) == nil, "not None")
	own := &providers.Secrets{Present: map[string]bool{"B": true}, Known: true}
	assertEqual(t, providers.NewerSecrets(own, st, night.Add(-time.Hour)).Present, map[string]bool{"A": true})
	assertTrue(t, providers.NewerSecrets(own, st, night.Add(time.Hour)) == own, "not own")
	assertEqual(t, providers.NewerSecrets(own, st, time.Time{}).Present, map[string]bool{"A": true})
	assertTrue(t, providers.NewerSecrets(own, pyjson.New(), time.Time{}) == own, "not own")
}

// ============================================================================ RunnersTests

// Every runner and what it is doing (`runners_section`): each job a runner holds, the step it is
// on and what it is for, the jobs queued for one, the self-hosted labels with no job, and what
// each runner and workflow is for; redrawn alone every two minutes (`refresh_runners`).

// dashboardRunners is RunnersTests: its world, set up as setUp does.
type dashboardRunners struct {
	t   *testing.T
	gh  *fakes.FakeGitHub
	ctx *hctx.Context
}

// newDashboardRunners is RunnersTests.setUp.
func newDashboardRunners(t *testing.T) *dashboardRunners {
	gh := fakes.NewFakeGitHub()
	r := &dashboardRunners{t: t, gh: gh, ctx: testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night,
		Cfg: testsupport.MakeConfig()})}
	// The tests' weak subscription has runners on the night box that take no slot of their own,
	// as the committed weak one's did until #2129, so the boxes' other runners stay tested.
	patch(t, &dashboard.BoxSlots, []dashboard.BoxSlot{
		{Place: "night box", Owners: []string{"gpt", "agy", "muse"}, Extras: []string{testsupport.Weak}},
		{Place: "Muse box", Owners: []string{"muse", "muse", "muse"}, Extras: []string{}}})
	ago := dashboardAgo

	flowUpdate(r.ctx, func(s *pyjson.Object) {
		hstate.Item(s, 37).Update(pyjson.New("provider", "claude-1", "kind", "build", "run_id", "101",
			"started_at", ago(47)))
		// A run the status loop started, before it found the run's id (#1455).
		hstate.Item(s, 49).Update(pyjson.New("provider", "muse", "kind", "revise",
			"run_id", "pending:49-1790000000", "started_at", ago(20)))
	})
	night := ".github/workflows/bot-night.yml"
	gh.Runs.Update(pyjson.New(
		"101", pyjson.New("id", 101, "status", "in_progress", "path", night, "display_title", "bot-night",
			"event", "workflow_dispatch", "created_at", ago(48)),
		"102", pyjson.New("id", 102, "status", "in_progress", "path", night,
			"display_title", "bot-night 49-1790000000 on night-vm-muse",
			"event", "workflow_dispatch", "created_at", ago(20)),
		"103", pyjson.New("id", 103, "status", "in_progress", "path", ".github/workflows/ci.yml",
			"display_title", "Patch: a Sets tab", "event", "pull_request",
			"pull_requests", []any{pyjson.New("number", 1966)}, "created_at", ago(4)),
		"104", pyjson.New("id", 104, "status", "queued", "path", ".github/workflows/learn.yml",
			"display_title", "learn", "event", "schedule", "created_at", ago(90)),
	))
	doing := pyjson.New("name", "Build, check and review", "status", "in_progress")
	gh.Jobs["101"] = []*pyjson.Object{
		pyjson.New("name", "plan", "status", "completed", "labels", []any{"ubuntu-latest"}),
		pyjson.New("name", "work", "status", "in_progress", "labels", []any{"ubuntu-latest"},
			"runner_name", "GitHub Actions 1000123", "started_at", ago(47),
			"html_url", "https://github.com/x/y/actions/runs/101/job/1",
			"steps", []any{pyjson.New("name", "Set up job", "status", "completed"), doing})}
	gh.Jobs["102"] = []*pyjson.Object{
		pyjson.New("name", "work", "status", "in_progress", "labels", []any{"night-vm-muse"},
			"runner_name", "night-vm-muse-b2", "started_at", ago(19),
			"html_url", "https://github.com/x/y/actions/runs/102/job/3",
			"steps", []any{pyjson.New("name", "Set up job", "status", "completed")})}
	gh.Jobs["103"] = []*pyjson.Object{
		pyjson.New("name", "rust (test engine)", "status", "in_progress",
			"labels", []any{"ubuntu-latest"}, "runner_name", "GitHub Actions 1000124",
			"started_at", ago(3), "steps", []any{pyjson.New("name", "cargo test", "status", "in_progress")}),
		pyjson.New("name", "web (unit 2/4)", "status", "queued", "labels", []any{"ubuntu-latest"},
			"created_at", ago(4)),
		pyjson.New("name", "web (unit 3/4)", "status", "queued", "labels", []any{"ubuntu-latest"},
			"created_at", ago(3))}
	gh.Jobs["104"] = []*pyjson.Object{
		pyjson.New("name", "plan", "status", "queued", "labels", []any{dashboard.LocalLabel},
			"created_at", ago(90))}
	return r
}

// section is RunnersTests.section.
func (r *dashboardRunners) section() string {
	return dashboard.RunnersSection(r.ctx, r.ctx.Store.Load(), nil, dashboardRoom)
}

// diagram is `self.section().split("```mermaid\n")[1].split("```")[0]`.
func (r *dashboardRunners) diagram() string {
	r.t.Helper()
	return dashboardBefore(dashboardAfter(r.t, r.section(), "```mermaid\n"), "```")
}

func TestDashboard_EachBusyRunnerSaysWhatItDoesAndWhatFor(t *testing.T) {
	r := newDashboardRunners(t)
	body := r.section()
	assertTrue(t, strings.HasPrefix(body, dashboard.RunnersStart+"\n## 🖥️ Every runner"), "its start")
	assertTrue(t, strings.HasSuffix(body, dashboard.RunnersEnd), "its end")
	assertIn(t, "🟢 **3 runners busy**: 2 on GitHub's (of the 20 the organization may hold), "+
		"1 on the Muse box · ⏳ **3 jobs queued** for a runner · read at 22:00", body)
	// GitHub's runners first, the longest-running first; the night bot's runs name the item they
	// work on, found by the run's id or the pending claim's reference in its title.
	hosted := "| `GitHub Actions 1000123` (Linux) | bot-night · work | build, check and review " +
		"| building #37 on `claude-1` | [47m](https://github.com/x/y/actions/runs/101/job/1) |"
	ci := "| `GitHub Actions 1000124` (Linux) | ci · rust (test engine) | cargo test " +
		"| #1966 Patch: a Sets tab | 3m |"
	muse := "| `night-vm-muse-b2` (Muse box) | bot-night · work | after Set up job " +
		"| revising #49 on `muse` | [19m](https://github.com/x/y/actions/runs/102/job/3) |"
	for _, row := range []string{hosted, ci, muse} {
		assertIn(t, row, body)
	}
	assertLess(t, strIndex(t, body, hosted), strIndex(t, body, ci))
	assertLess(t, strIndex(t, body, ci), strIndex(t, body, muse))
	// Each part folds away, open at first.
	assertIn(t, "<details open><summary><b>The runners, as boxes</b></summary>", body)
	assertIn(t, "<details open><summary><b>Every busy runner's job</b>, as a table (3)"+
		"</summary>", body)
	assertLess(t, strIndex(t, body, "```mermaid"), strIndex(t, body, "| Runner |"))
}

// The runners drawn as the subscriptions are: GitHub's runners, the night box with the labels no
// job holds, the Muse box, the owner's PC, and the jobs queued, by label.
func TestDashboard_TheRunnersAsBoxes(t *testing.T) {
	r := newDashboardRunners(t)
	diagram := r.diagram()
	assertTrue(t, strings.HasPrefix(diagram, dashboard.RunnerWrap+"\nflowchart TB\n"), "its start")
	assertIn(t, `    subgraph gh["GitHub's runners: 2 busy, of the 20 the organization may `+
		`hold"]`, diagram)
	assertIn(t, `        gh0["<b>bot-night · work</b><br/>building #37 on claude-1<br/>build, `+
		`check and review<br/>47m"]:::busy`, diagram)
	assertIn(t, "        gh0 ~~~ gh1\n", diagram)
	// Each AWS box as its slots (#2069): the night box's gpt's, agy's and one of Muse's own
	// lanes, the first two Muse's to fill while they are free; the Muse box's three of Muse's.
	assertIn(t, `    subgraph nb["The night box (AWS): no job, so idle or switched off"]`,
		diagram)
	assertIn(t, `nb0["<b>Slot 1 · gpt's lane</b><br/>⚪ free<br/>muse may fill it"]:::idle`,
		diagram)
	assertIn(t, `nb2["<b>Slot 3 · muse's lane</b><br/>⚪ free"]:::idle`, diagram)
	assertIn(t, `    subgraph mb["The Muse box (AWS): 1 of 3 slots busy"]`, diagram)
	assertIn(t, `mb0["<b>Slot 1 · muse's lane</b><br/>🟢 muse<br/>revising #49 on muse<br/>`+
		`after Set up job<br/>19m"]:::busy`, diagram)
	assertIn(t, `mb2["<b>Slot 3 · muse's lane</b><br/>⚪ free"]:::idle`, diagram)
	assertNotIn(t, "subgraph nj[", diagram) // every machine subscription has its box
	// The queued jobs by label, the most first, each with its longest wait and its first.
	assertIn(t, `    subgraph qu["Queued for a runner: 3 jobs"]`, diagram)
	assertIn(t, `qu0["<b>ubuntu-latest</b><br/>⏳ 2 jobs<br/>the longest 4m<br/>ci · web `+
		`(unit 2/4)"]:::next`, diagram)
	assertIn(t, `qu1["<b>jackioh-selfplay-local</b><br/>⏳ 1 job<br/>the longest 1h 30m`+
		`<br/>learn · plan"]:::next`, diagram)
	assertIn(t, "    gh ~~~ nb ~~~ mb ~~~ pc ~~~ qu\n", diagram)
	assertIn(t, "classDef busy fill:#1f883d", diagram)
	assertNotIn(t, "ghost", diagram) // one row each: nothing to pad
	// Only the styles its boxes use: the rest would spend the issue's room.
	for _, m := range regexp.MustCompile(`classDef (\w+)`).FindAllStringSubmatch(diagram, -1) {
		assertIn(t, ":::"+m[1], diagram)
	}
	assertNotIn(t, "classDef broken", diagram)
	// A label no runner holds but a job waits for: the box is starting, or full.
	r.gh.Jobs["104"][0].Set("labels", []any{"night-vm-gpt"})
	diagram = r.diagram()
	assertIn(t, `["<b>Slot 1 · gpt's lane</b><br/>⏳ a gpt job is queued<br/>its box is `+
		`starting, or full"]:::next`, diagram)
}

// Muse's fill-in runners take gpt's or agy's slot while it has no job of its own (#2028); the weak
// one's runners on the night box take none, and follow the slots.
func TestDashboard_AFillInTakesAFreeSlotAndOtherRunnersFollowTheSlots(t *testing.T) {
	r := newDashboardRunners(t)
	night := func(jobs [][2]string) string {
		t.Helper()
		r.gh.Jobs["103"] = []*pyjson.Object{}
		for _, j := range jobs {
			r.gh.Jobs["103"] = append(r.gh.Jobs["103"], pyjson.New("name", "work", "status", "in_progress",
				"labels", []any{j[0]}, "runner_name", j[1], "started_at", clock.ISO(testsupport.Night)))
		}
		diagram := r.diagram()
		return dashboardBefore(diagram[strIndex(t, diagram, "    subgraph nb["):], "    end")
	}
	box := night([][2]string{{"night-vm-muse-fill", "night-vm-muse-fill"},
		{"night-vm-gpt", "night-vm-gpt"},
		{"night-vm-muse-fill", "night-vm-muse-fill-2"}})
	// Two fill-ins and gpt's own job: the second fill-in has no free lane it may take.
	assertIn(t, `subgraph nb["The night box (AWS): 2 of 3 slots busy · 1 more on its other `+
		`runners"]`, box)
	assertIn(t, `nb0["<b>Slot 1 · gpt's lane</b><br/>🟢 gpt<br/>`, box)
	assertIn(t, `nb1["<b>Slot 2 · agy's lane</b><br/>🟢 muse fills it<br/>`, box)
	assertIn(t, `nb2["<b>Slot 3 · muse's lane</b><br/>⚪ free"]`, box) // no fill-in's
	assertIn(t, `nb3["<b>night-vm-muse-fill-2</b><br/>🟢 ci · work<br/>`, box)
	box = night([][2]string{{"night-vm-lite", "night-vm-lite-3"}})
	assertIn(t, `subgraph nb["The night box (AWS): 0 of 3 slots busy · 1 more on its other `+
		`runners"]`, box)
	assertIn(t, `nb3["<b>night-vm-lite-3</b><br/>🟢 ci · work<br/>`, box)
	// A machine subscription no box's slots name has its labels drawn apart.
	restore := patch(t, &dashboard.BoxSlots, []dashboard.BoxSlot{
		{Place: "night box", Owners: []string{"gpt", "muse"}, Extras: []string{}},
		{Place: "Muse box", Owners: []string{"muse"}, Extras: []string{}}})
	diagram := r.diagram()
	restore()
	assertIn(t, `subgraph nj["Other AWS labels with no job"]`, diagram)
	assertIn(t, "<b>night-vm-agy</b><br/>💤 no job", diagram)
}

func TestDashboard_EachSectionFoldsAwayUnderItsHeading(t *testing.T) {
	r := newDashboardRunners(t)
	body := dashboardRender(r.ctx, []string{dashboard.Section(r.ctx)}, "")
	for _, heading := range []string{"## ⚡ Running now", "## 🛣️ Lanes", "## 📋 Queue",
		"## 🔋 Subscriptions", "## 🏃 Last night-bot runs", "## 🧠 The learn lane",
		"## 🫧 " + py.Str(config.Title)} {
		after := strings.TrimLeft(body[strIndex(t, body, heading)+len(heading):], "\n")
		assertTrue(t, strings.HasPrefix(after, "<details open><summary>"), heading)
	}
	assertTrue(t, strings.HasSuffix(dashboard.Section(r.ctx), "</details>\n\n"+dashboard.SectionEnd))
}

// pc is RunnersTests.pc: the section's owner's PC group, with `beat` as its heartbeat in gist
// `gist` (a nil beat leaves the gists as they are; "" is no gist set up).
func (r *dashboardRunners) pc(beat *pyjson.Object, gist string) string {
	r.t.Helper()
	cfg := testsupport.MakeConfig(testsupport.Config{Env: dashboardGistEnv(gist)})
	r.ctx = testsupport.MakeCtx(r.gh, testsupport.Ctx{At: testsupport.Night, Cfg: cfg})
	if beat != nil {
		r.gh.Gists = dashboardGist(beat)
	}
	diagram := r.diagram()
	return dashboardBefore(diagram[strIndex(r.t, diagram, "    subgraph pc["):], "    end")
}

// beat is RunnersTests.beat(minutes_ago, **fields); a nil `fields` is none.
func (r *dashboardRunners) beat(minutesAgo int, fields *pyjson.Object) *pyjson.Object {
	runners := []any{pyjson.New("name", "pc-gather-1", "label", dashboard.GatherLabel, "busy", true,
		"cpus", 1.9, "throttled", false),
		pyjson.New("name", "pc-lite-2", "label", "night-vm-lite", "busy", false)}
	return pyjson.New("at", dashboardAgo(minutesAgo), "calm", true, "cpu", 7.4,
		"memGB", 11.2, "logical", 16, "runners", runners).Update(fields)
}

// gatherGoing is RunnersTests.gather_going.
func (r *dashboardRunners) gatherGoing(status string) {
	ago := dashboardAgo
	r.gh.Runs.Set("300", pyjson.New("id", 300, "status", "in_progress", "event", "push",
		"path", ".github/workflows/gather.yml",
		"display_title", "Patch v0.3.40", "created_at", ago(13)))
	job := pyjson.New("name", "gather the newest patch", "status", status,
		"labels", []any{dashboard.GatherLabel}, "created_at", ago(13))
	if status == "in_progress" {
		job.Update(pyjson.New("runner_name", "pc-gather-1", "started_at", ago(12),
			"steps", []any{pyjson.New("name", "build", "status", "completed"),
				pyjson.New("name", "play (cargo jackioh stats --series "+
					"gather)", "status", "in_progress")}))
	}
	r.gh.Jobs["300"] = []*pyjson.Object{job}
}

// The PC's supervisor writes a heartbeat to a gist (JACKIOH_PC_GIST): its load, whether it lends
// itself, and each runner with its CPUs and whether it is slowed. The group's title says whether
// it is working and on which workflow; gather's box says it is gathering, and how hard the PC lets
// it.
func TestDashboard_TheOwnersPCFromItsHeartbeat(t *testing.T) {
	r := newDashboardRunners(t)
	r.gatherGoing("in_progress")
	group := r.pc(r.beat(1, nil), "g1")
	assertIn(t, `subgraph pc["The owner's PC: calm, lending itself, working on gather, `+
		`CPU 7%, 11.2 GB in use · heard 21:59"]`, group)
	assertIn(t, `pc0["<b>pc-gather-1</b><br/>🟢 gather · gather the newest patch<br/>push: `+
		`Patch v0.3.40<br/>play (cargo jackioh stats --serie…<br/>12m"]:::busy`,
		group)
	assertIn(t, `pc1["<b>pc-lite-2</b><br/>⚪ idle, waiting<br/>for a night-vm-lite job"]`+
		`:::idle`, group)
	assertIn(t, `pc2["<b>gather</b><br/>🟢 gathering<br/>play (cargo jackioh stats --serie…`+
		`<br/>for 12m<br/>on 1.9 CPUs"]:::busy`, group)
	// The PC busy with its owner's work: gather goes on, slowed.
	slowed := r.beat(1, pyjson.New("calm", false))
	pyjson.Obj(slowed.List("runners")[0]).Set("throttled", true)
	group = r.pc(slowed, "g1")
	assertIn(t, "The owner's PC: busy with its owner's work, working on gather, CPU 7%",
		group)
	assertIn(t, "for 12m<br/>⏸️ slowed: the PC is busy\"]:::paused", group)
	// A supervisor that sends the whole PC's load and its memory's size: percentages.
	group = r.pc(r.beat(1, pyjson.New("cpuAll", 43.6, "memAllGB", 20.8, "memTotalGB", 31.9)), "g1")
	assertIn(t, `subgraph pc["The owner's PC: calm, lending itself, working on gather, `+
		`CPU 44%, memory 65% · heard 21:59"]`, group)
	// PowerShell 5.1 may write a list as {"value": [...]}.
	wrapped := r.beat(1, nil)
	wrapped.Set("runners", pyjson.New("value", wrapped.Get("runners"), "Count", 2))
	assertIn(t, "on 1.9 CPUs", r.pc(wrapped, "g1"))
	// No job on the PC: not working, and whether a runner of its waits for one.
	r.gh.Runs.Delete("300")
	idle := r.beat(1, nil)
	idle.Set("runners", idle.List("runners")[1:])
	assertIn(t, "calm, lending itself, not working: a runner waits for a job, CPU 7%",
		r.pc(idle, "g1"))
	assertIn(t, "calm, lending itself, not working: no job, CPU 7%",
		r.pc(r.beat(1, pyjson.New("runners", []any{})), "g1"))
}

// Gather's label has a slot of its own on the PC, outside the learn lane's one job at a time
// (#2134), so the two run together and the group names both.
func TestDashboard_LearnAndGatherSideBySide(t *testing.T) {
	r := newDashboardRunners(t)
	r.gatherGoing("in_progress")
	r.gh.Runs.Obj("104").Set("status", "in_progress")
	r.gh.Jobs["104"] = []*pyjson.Object{pyjson.New("name", "export (shard 3)", "status", "in_progress",
		"labels", []any{dashboard.LocalLabel}, "runner_name", "pc-selfplay-1",
		"started_at", dashboardAgo(40), "steps", []any{pyjson.New("name", "play", "status", "in_progress")})}
	beat := r.beat(1, nil)
	beat.Set("runners", slices.Insert(beat.List("runners"), 0, any(pyjson.New("name", "pc-selfplay-1",
		"label", dashboard.LocalLabel, "busy", true, "cpus", 5.8, "throttled", false))))
	group := r.pc(beat, "g1")
	assertIn(t, "The owner's PC: calm, lending itself, working on learn and gather, CPU 7%",
		group)
	assertIn(t, `pc0["<b>pc-selfplay-1</b><br/>🟢 learn · export (shard 3)`, group)
	assertIn(t, `pc1["<b>pc-gather-1</b><br/>🟢 gather · gather the newest patch`, group)
	assertIn(t, `<br/>on 1.9 CPUs"]:::busy`, group)
}

// bothPlaying is RunnersTests.both_playing: a learn shard and gather both going on the PC, as
// above, with each runner's `progress` in the heartbeat as given (nil: none).
func (r *dashboardRunners) bothPlaying(learn, gather *pyjson.Object) {
	r.gatherGoing("in_progress")
	r.gh.Runs.Obj("104").Set("status", "in_progress")
	r.gh.Jobs["104"] = []*pyjson.Object{pyjson.New("name", "export (shard 3)", "status", "in_progress",
		"labels", []any{dashboard.LocalLabel}, "runner_name", "pc-selfplay-1",
		"started_at", dashboardAgo(40), "steps", []any{})}
	beat := r.beat(1, nil)
	beat.Set("runners", slices.Insert(beat.List("runners"), 0, any(pyjson.New("name", "pc-selfplay-1",
		"label", dashboard.LocalLabel, "busy", true, "cpus", 5.8, "throttled", false))))
	for i, played := range []*pyjson.Object{learn, gather} {
		if played != nil {
			pyjson.Obj(beat.List("runners")[i]).Set("progress", played)
		}
	}
	r.heartbeat(beat, "g1")
}

// heartbeat is RunnersTests.heartbeat.
func (r *dashboardRunners) heartbeat(beat *pyjson.Object, gist string) {
	r.ctx = testsupport.MakeCtx(r.gh, testsupport.Ctx{At: testsupport.Night,
		Cfg: testsupport.MakeConfig(testsupport.Config{Env: dashboardGistEnv(gist)})})
	if beat != nil {
		r.gh.Gists = dashboardGist(beat)
	}
}

// top is RunnersTests.top: the top of the issue: how far the PC's two local jobs have played.
func (r *dashboardRunners) top() string {
	return dashboard.Progress(r.ctx, dashboard.ReadRunners(r.ctx))
}

// bars is RunnersTests.bars: the progress chart's labels, its played bar and the line under it.
func (r *dashboardRunners) bars() (string, string, string) {
	r.t.Helper()
	var labels, note string
	var bars []string
	for _, line := range strings.Split(r.top(), "\n") {
		if labels == "" && strings.HasPrefix(line, "    x-axis") {
			labels = line
		}
		if strings.HasPrefix(line, "    bar ") {
			bars = append(bars, line)
		}
		if note == "" && strings.HasPrefix(line, "<sub>") {
			note = line
		}
	}
	if labels == "" || note == "" || len(bars) < 2 {
		r.t.Fatalf("the chart has no labels, two bars and a note:\n%s", r.top())
	}
	return labels, bars[1], note
}

// Under the issue's date, a Mermaid bar chart: the learn lane's export as the game it is at of
// learn.yml's 100,000, its shard's games, and gather's games, each in percent over a grey bar to
// 100, from the heartbeat's `progress` (the games each `jackioh` has written so far); under it,
// the shard's range, gather's patch and when the PC was heard.
func TestDashboard_TheTopOfTheIssueChartsHowFarThePcsTwoJobsHavePlayed(t *testing.T) {
	r := newDashboardRunners(t)
	r.bothPlaying(pyjson.New("kind", "selfplay", "games", 5000, "done", 2304, "from", 65001),
		pyjson.New("kind", "stats", "games", 2000, "done", 1320, "from", 300001, "patch", "v0.4.12d"))
	text := r.top()
	assertTrue(t, strings.HasPrefix(text, dashboard.ProgressStart+"\n```mermaid\n"+
		dashboard.ProgressChart+"\nxychart-beta horizontal\n"+
		"    title \"The owner's PC: how far its two local jobs "+
		"have played, %\"\n"), "seed")
	assertIn(t, `    x-axis ["learn: game 67,304 of 100,000", "its shard: 2,304 of 5,000", `+
		`"gather: game 1,320 of 2,000"]`+"\n"+`    y-axis "% played" 0 --> 100`+"\n"+
		"    bar [100, 100, 100]\n    bar [67.3, 46.1, 66.0]\n```\n", text)
	assertTrue(t, strings.HasSuffix(text, "<sub>The shard plays games 65,001–70,000 · gather plays "+
		"v0.4.12d · heard 21:59.</sub>\n"+dashboard.ProgressEnd), text)
	// An export larger than learn.yml's default: its shard alone, and the game it is at.
	r.bothPlaying(pyjson.New("kind", "selfplay", "games", 5000, "done", 10, "from", 100001), nil)
	labels, played, note := r.bars()
	assertEqual(t, labels, `    x-axis ["its shard: 10 of 5,000", "gather: working, no `+
		`games"]`)
	assertEqual(t, played, "    bar [0.2, 0.0]")
	assertEqual(t, note, "<sub>Learn is at game 100,010, past learn.yml's 100,000 · the "+
		"shard plays games 100,001–105,000 · gather is playing no games now (its "+
		"build) · heard 21:59.</sub>")
	// Busy but playing no games: building, retraining, gating.
	r.bothPlaying(nil, nil)
	labels, played, note = r.bars()
	assertEqual(t, labels, `    x-axis ["learn: working, no games", "gather: working, no `+
		`games"]`)
	assertEqual(t, played, "    bar [0.0, 0.0]")
	assertIn(t, "<sub>Learn is playing no games now (a build, the retrain, the screen or "+
		"the gate) · gather is playing no games now (its build)", note)
	// Jobs queued for runners the PC has not started (it is not quiet yet), then none at all.
	r.gatherGoing("queued")
	r.gh.Runs.Obj("104").Set("status", "queued")
	r.gh.Jobs["104"] = []*pyjson.Object{pyjson.New("name", "plan", "status", "queued",
		"labels", []any{dashboard.LocalLabel})}
	r.heartbeat(r.beat(1, pyjson.New("runners", []any{})), "g1")
	labels, _, _ = r.bars()
	assertEqual(t, labels, `    x-axis ["learn: waiting for its runner", `+
		`"gather: waiting for its runner"]`)
	r.gh.Runs = pyjson.New() // gh.runs.clear()
	labels, _, _ = r.bars()
	assertEqual(t, labels, `    x-axis ["learn: not playing", "gather: not `+
		`playing"]`)
	// No heartbeat to trust: the line under the chart says why.
	r.heartbeat(r.beat(30, nil), "g1")
	labels, _, note = r.bars()
	assertEqual(t, labels, `    x-axis ["learn: no heartbeat", "gather: no heartbeat"]`)
	assertEqual(t, note, "<sub>The owner's PC: not heard since 21:30.</sub>")
	r.heartbeat(nil, "")
	_, _, note = r.bars()
	assertEqual(t, note, "<sub>The owner's PC: no heartbeat set up "+
		"(JACKIOH_PC_GIST).</sub>")
}

func TestDashboard_TheExportSizeIsLearnYmlsDefault(t *testing.T) {
	text, err := os.ReadFile(filepath.Join(testsupport.Root, ".github", "workflows", "learn.yml"))
	if err != nil {
		t.Fatal(err)
	}
	assertIn(t, fmt.Sprintf("games: ${{ inputs.games || '%d' }}", dashboard.LearnGames), string(text))
}

// The runs-going-now timeline draws the PC's jobs too, by workflow and job, in readable colours:
// plain bars, whose name beside a short bar is grey, not the dark theme's dark.
func TestDashboard_TheTimelineDrawsThePcsJobs(t *testing.T) {
	r := newDashboardRunners(t)
	r.bothPlaying(nil, nil)
	found := dashboard.ReadRunners(r.ctx)
	chart := strings.Join(dashboard.Timeline(r.ctx, pyjson.New(), pyjson.New(), found), "\n")
	assertTrue(t, strings.HasPrefix(chart, "```mermaid\n"+dashboard.TimelineStyle+"\ngantt\n"), chart)
	assertIn(t, `"taskTextOutsideColor": "#6e7781"`, dashboard.TimelineStyle)
	assertIn(t, "    section The owner's PC\n"+
		"    learn · export (shard 3) · 40m :2026-09-29 21:20, 2026-09-29 22:00\n"+
		"    gather · gather the newest patch · 12m :2026-09-29 21:48, "+
		"2026-09-29 22:00", chart)
	assertNotIn(t, ":active", chart)
	assertEqual(t, dashboard.Timeline(r.ctx, pyjson.New(), pyjson.New(), nil),
		[]string{"Nothing is running right now."})
}

func TestDashboard_ARedrawKeepsTheTopLinesFresh(t *testing.T) {
	r := newDashboardRunners(t)
	r.bothPlaying(pyjson.New("kind", "selfplay", "games", 5000, "done", 2304, "from", 65001), nil)
	r.gh.AddIssue(148, fakes.Issue{Title: dashboard.Title, Body: "x", User: fakes.Operator})
	flowUpdate(r.ctx, func(s *pyjson.Object) { s.Set("dashboard", pyjson.New("issue", 148)) }, "state")
	r.gh.Threads.Get(148).Set("body",
		"date\n\n"+dashboard.ProgressStart+"\nold lines\n"+dashboard.ProgressEnd+"\n\ntop\n"+
			dashboard.RunnersStart+"\nold runners\n"+dashboard.RunnersEnd+"\nbottom")
	assertEqual(t, dashboard.RefreshRunners(r.ctx), "redrew #148's runners")
	body := r.gh.Threads.Get(148).Str("body")
	assertTrue(t, strings.HasPrefix(body, "date\n\n"+dashboard.ProgressStart+"\n```mermaid\n"), body)
	assertIn(t, `"learn: game 67,304 of 100,000"`, body)
	assertNotIn(t, "old lines", body)
	assertNotIn(t, "old runners", body)
	assertTrue(t, strings.HasSuffix(body, dashboard.RunnersEnd+"\nbottom"), "its end")
}

func TestDashboard_APCGoneQuietOrNeverHeard(t *testing.T) {
	r := newDashboardRunners(t)
	group := r.pc(r.beat(30, nil), "g1")
	assertIn(t, "The owner's PC: not heard since 21:30: off, asleep, or its supervisor "+
		"stopped", group)
	assertNotIn(t, "pc-lite-2", group) // an old heartbeat's runners are not trusted
	assertIn(t, `["<b>jackioh-selfplay-local</b><br/>💤 no job<br/>idle, or the PC is off"]`,
		group)
	assertIn(t, "The owner's PC: no heartbeat set up (JACKIOH_PC_GIST)", r.pc(nil, ""))
	r.gh.Gists = map[string]*pyjson.Object{}
	assertIn(t, "The owner's PC: its heartbeat could not be read", r.pc(nil, "g1"))
	assertIn(t, "The owner's PC: no heartbeat yet", r.pc(pyjson.New("calm", true), "g1"))
}

// Gathering now, waiting for the PC's runner, or how its last run ended: off while
// GATHER_ON_PATCH is not true (its run skipped), done, failed, or never run.
func TestDashboard_WhatTheGatherBoxSays(t *testing.T) {
	r := newDashboardRunners(t)
	gatherBox := func() string {
		t.Helper()
		return dashboardBefore(dashboardAfter(t, r.pc(r.beat(1, nil), "g1"), "<b>gather</b>"), "\n")
	}
	assertIn(t, "no run yet<br/>it plays each patch<br/>as it ships\"]:::off", gatherBox())
	r.gatherGoing("queued")
	assertIn(t, "⏳ waiting for the PC's runner<br/>for 13m\"]:::next", gatherBox())
	r.gh.Runs.Delete("300")
	for _, c := range [][2]string{{"skipped", "⚫ off: GATHER_ON_PATCH<br/>is not true (skipped " +
		"21:00)\"]:::off"},
		{"success", "⚪ not gathering<br/>last played 21:00\"]:::idle"},
		{"failure", "❌ its last run: failure<br/>21:00\"]:::broken"}} {
		r.gh.Runs.Set("301", pyjson.New("id", 301, "status", "completed", "conclusion", c[0],
			"path", ".github/workflows/gather.yml", "event", "push",
			"created_at", clock.ISO(testsupport.Night.Add(-time.Hour))))
		assertIn(t, c[1], gatherBox())
	}
}

func TestDashboard_ThePCGistComesFromTheEnvironment(t *testing.T) {
	assertEqual(t, testsupport.MakeConfig(testsupport.Config{Env: map[string]string{
		"JACKIOH_PC_GIST": "abc123"}}).PCGist, "abc123")
	assertEqual(t, testsupport.MakeConfig().PCGist, "")
}

// sectionWith is RunnersTests.section_with.
func (r *dashboardRunners) sectionWith(runners [][2]string) string {
	r.gh.Jobs["103"] = []*pyjson.Object{}
	for n, x := range runners {
		r.gh.Jobs["103"] = append(r.gh.Jobs["103"], pyjson.New("name", fmt.Sprintf("job %d", n),
			"status", "in_progress", "labels", []any{x[0]}, "runner_name", x[1]))
	}
	return r.section()
}

func TestDashboard_AShortLastRowIsPaddedToStartAtTheLeft(t *testing.T) {
	r := newDashboardRunners(t)
	var boxes []dashboard.Box
	for n := 0; n < dashboard.BoxRow+2; n++ {
		boxes = append(boxes, dashboard.Box{Lines: []string{fmt.Sprintf("box %d", n)}, Style: "busy"})
	}
	lines := dashboard.Boxes("gh", "Seven", boxes)
	ghosts := 0
	for _, line := range lines {
		if strings.Contains(line, ":::ghost") {
			ghosts++
		}
	}
	assertEqual(t, ghosts, dashboard.BoxRow-2)
	row := func(from, to int) string {
		var nodes []string
		for i := from; i < to; i++ {
			nodes = append(nodes, fmt.Sprintf("gh%d", i))
		}
		return "        " + strings.Join(nodes, " ~~~ ")
	}
	assertTrue(t, slices.Contains(lines, row(0, dashboard.BoxRow)), "the first row's links")
	assertTrue(t, slices.Contains(lines, row(dashboard.BoxRow, 2*dashboard.BoxRow)), "the last row's links")
	assertNotIn(t, ":::ghost", strings.Join(dashboard.Boxes("gh", "One row", boxes[:3]), "\n"))
	var runners [][2]string
	for n := 0; n < 7; n++ {
		runners = append(runners, [2]string{"ubuntu-latest", fmt.Sprintf("GitHub Actions %d", n)})
	}
	body := r.sectionWith(runners)
	assertIn(t, dashboard.GhostStyle, body)
	// Nothing a box holds reads as Mermaid's syntax or HTML.
	assertEqual(t, dashboard.Text("a \"quote\" `code` <time>", 34), "a 'quote' code ‹time›")
}

func TestDashboard_ALongListOfBusyRunnersIsCut(t *testing.T) {
	r := newDashboardRunners(t)
	r.gh.Jobs["103"] = []*pyjson.Object{}
	for n := 0; n < dashboard.RunnerRows+2; n++ {
		r.gh.Jobs["103"] = append(r.gh.Jobs["103"], pyjson.New("name", fmt.Sprintf("shard %d", n),
			"status", "in_progress", "labels", []any{"ubuntu-latest"},
			"runner_name", fmt.Sprintf("GitHub Actions %d", n)))
	}
	body := r.section()
	assertIn(t, fmt.Sprintf("🟢 **%d runners busy**", dashboard.RunnerRows+4), body)
	assertEqual(t, strings.Count(body, " (Linux) | "), dashboard.RunnerRows)
	assertIn(t, "…and 4 more.", body)
	// With no room for a table, the runners are counted and the rest still says what waits.
	tight := dashboard.RunnersSection(r.ctx, r.ctx.Store.Load(), nil, 0)
	assertNotIn(t, "| Runner |", tight)
	assertIn(t, fmt.Sprintf("%d not listed: no room.", dashboard.RunnerRows+4), tight)
	// Then fewer of GitHub's runners as boxes: none at all, at no room.
	assertIn(t, fmt.Sprintf("<b>… and %d more</b><br/>not drawn: no room", dashboard.RunnerRows+3),
		tight)
	assertIn(t, "Queued for a runner: 1 job", tight)
	assertTrue(t, strings.HasSuffix(tight, dashboard.RunnersEnd), "its end")
	roomy := dashboard.RunnersSection(r.ctx, r.ctx.Store.Load(), nil, py.Len(r.section())-1)
	assertNotIn(t, "<b>… and", roomy) // the table goes first
}

// The explanation changes only with the pool or the workflows, so it is a comment on the status
// issue, written once and again only when its text changes; the section links it.
func TestDashboard_WhatEachRunnerIsForIsACommentKeptFromThePool(t *testing.T) {
	r := newDashboardRunners(t)
	r.gh.AddIssue(148, fakes.Issue{Title: dashboard.Title, Body: "x", User: fakes.Operator})
	st := r.ctx.Store.Load()
	assertIn(t, "What each runner and workflow is for is in a comment below.",
		dashboard.RunnersSection(r.ctx, st, nil, dashboardRoom))
	dashboard.Explain(r.ctx, 148, st)
	comments := r.gh.Comments.Get(148)
	if len(comments) != 1 {
		t.Fatalf("comments on #148: %d, want one", len(comments))
	}
	comment := comments[0]
	text := comment.Str("body")
	assertTrue(t, strings.HasPrefix(text, "## 🖥️ What each runner is for"), "its start")
	assertIn(t, dashboard.Explained, text)
	assertIn(t, "| `ubuntu-latest` | GitHub's Linux", text)
	assertIn(t, "| `night-vm-lite` | the bot's AWS boxes, 6 lanes | `bot-night`'s `work` for "+
		"`lite`: the `codex` CLI with `lite-1`, as the Linux user "+
		"`agent-lite`, which holds that login alone |", text)
	assertIn(t, "| `night-vm-gpt` | the bot's AWS boxes, 1 lane | ", text)
	assertIn(t, "| `"+dashboard.LocalLabel+"` | the owner's PC", text)
	assertIn(t, "| `"+dashboard.GatherLabel+"` | the owner's PC, a slot of its own", text)
	assertIn(t, "| `voice` | renders a branch's voice lines", text)
	// None of the statistics' outcomes reads it as a run's.
	assertEqual(t, stats.Classify(text), "other")
	assertIn(t, "What each runner and workflow is for: [its comment](https://github.com/"+
		fmt.Sprintf("jgoetzmann/JackiOh/issues/148#issuecomment-%v).", comment.Get("id")),
		dashboard.RunnersSection(r.ctx, st, nil, dashboardRoom))
	// Unchanged, it costs nothing; changed, it is rewritten in place; deleted, written again.
	r.gh.Patch.UpdateComment = func(int, string) *pyjson.Object { panic(errors.New("written")) }
	r.gh.Patch.CreateComment = func(int, string) *pyjson.Object { panic(errors.New("posted")) }
	dashboard.Explain(r.ctx, 148, r.ctx.Store.Load())
	r.gh.Patch.UpdateComment, r.gh.Patch.CreateComment = nil, nil
	restore := patch(t, &dashboard.BoxesNote, "Another note.")
	dashboard.Explain(r.ctx, 148, r.ctx.Store.Load())
	restore()
	assertEqual(t, len(r.gh.Comments.Get(148)), 1)
	assertIn(t, "Another note.", r.gh.Comments.Get(148)[0].Str("body"))
	r.gh.Comments.Set(148, []*pyjson.Object{}) // .clear()
	dashboard.Explain(r.ctx, 148, r.ctx.Store.Load())
	assertEqual(t, len(r.gh.Comments.Get(148)), 1)
	assertNotIn(t, "Another note.", r.gh.Comments.Get(148)[0].Str("body"))
	// A rewrite of the issue writes it, and links it in the same rewrite.
	flowUpdate(r.ctx, func(s *pyjson.Object) {
		s.Set("dashboard", pyjson.New("issue", 148))
		s.Set("runners_explained", nil)
	}, "state")
	r.gh.Comments.Set(148, []*pyjson.Object{}) // .clear()
	dashboardUpdate(r.ctx)
	comments = r.gh.Comments.Get(148)
	if len(comments) != 1 {
		t.Fatalf("comments on #148: %d, want one", len(comments))
	}
	assertIn(t, fmt.Sprintf("#issuecomment-%v", comments[0].Get("id")), r.gh.Threads.Get(148).Str("body"))
}

func TestDashboard_EveryWorkflowSaysWhatItIsFor(t *testing.T) {
	paths, err := filepath.Glob(filepath.Join(testsupport.Root, ".github", "workflows", "*.yml"))
	if err != nil {
		t.Fatal(err)
	}
	onDisk := map[string]bool{}
	for _, path := range paths {
		onDisk[filepath.Base(path)] = true
	}
	named := map[string]bool{}
	for name := range dashboard.Workflows {
		named[name] = true
	}
	assertEqual(t, named, onDisk)
}

func TestDashboard_WhereAJobRuns(t *testing.T) {
	for _, c := range [][3]string{{"GitHub Actions 7", "ubuntu-latest", "Linux"},
		{"GitHub Actions 8", "macos-15", "macOS"},
		{"GitHub Actions 9", "windows-2022", "Windows"},
		{"night-vm-lite-3", "night-vm-lite", "night box"},
		{"night-vm-muse-b2", "night-vm-muse", "Muse box"},
		{"pc-lite-1026021400", "night-vm-lite", "owner's PC"},
		{"", dashboard.LocalLabel, "owner's PC"},
		{"", dashboard.GatherLabel, "owner's PC"},
		{"someone-elses", "gpu", "unknown box"}} {
		_, where := dashboard.Where(pyjson.New("runner_name", c[0], "labels", []any{c[1]}))
		assertEqual(t, where, c[2], c[0], " ", c[1])
	}
}

func TestDashboard_NothingRunningAndRunsThatCannotBeRead(t *testing.T) {
	r := newDashboardRunners(t)
	r.gh.Runs = pyjson.New() // gh.runs.clear()
	body := r.section()
	assertIn(t, "No runner holds a job right now.", body)
	assertNotIn(t, "Queued for a runner", body)
	r.gh.Patch.ListRepoRuns = func(string, int, bool) []*pyjson.Object {
		panic(dashboardGitHubError("down", 502))
	}
	body = r.section()
	r.gh.Patch.ListRepoRuns = nil
	assertIn(t, "The runs going could not be read at 22:00.", body)
	assertIn(t, "What each runner and workflow is for", body) // its link stands regardless
	assertTrue(t, strings.HasSuffix(body, dashboard.RunnersEnd), "its end")
}

func TestDashboard_TheIssueCarriesItLowAfterEverythingTheBotsDraw(t *testing.T) {
	r := newDashboardRunners(t)
	body := dashboardRender(r.ctx, []string{"<!-- squishy -->\n## 🫧 Squishy"}, "## 📊 stats")
	assertLess(t, strIndex(t, body, "## 📊 stats"), strIndex(t, body, dashboard.RunnersStart))
	assertLess(t, strIndex(t, body, "## 🫧 Squishy"), strIndex(t, body, dashboard.RunnersStart))
	assertLess(t, strIndex(t, body, dashboard.RunnersEnd), strIndex(t, body, "<details><summary>The full status</summary>"))
	assertIn(t, "`bot-status` rewrites this issue every 5 minutes, and its runners every 2",
		body)
}

func TestDashboard_ARedrawChangesTheRunnersSectionAlone(t *testing.T) {
	r := newDashboardRunners(t)
	r.gh.AddIssue(148, fakes.Issue{Title: dashboard.Title, Body: "x", User: fakes.Operator})
	flowUpdate(r.ctx, func(s *pyjson.Object) { s.Set("dashboard", pyjson.New("issue", 148)) }, "state")
	bodyOf := func() string { return r.gh.Threads.Get(148).Str("body") }
	assertIn(t, "has no runners section", dashboard.RefreshRunners(r.ctx))
	assertEqual(t, bodyOf(), "x") // the next rewrite draws it
	before := "top\n" + dashboard.RunnersStart + "\nold runners\n" + dashboard.RunnersEnd +
		"\nbottom"
	r.gh.Threads.Get(148).Set("body", before)
	assertEqual(t, dashboard.RefreshRunners(r.ctx), "redrew #148's runners")
	body := bodyOf()
	assertTrue(t, strings.HasPrefix(body, "top\n"+dashboard.RunnersStart), "its start")
	assertTrue(t, strings.HasSuffix(body, dashboard.RunnersEnd+"\nbottom"), "its end")
	assertNotIn(t, "old runners", body)
	assertIn(t, "building #37 on `claude-1`", body)
	// Near GitHub's limit, fewer runners are listed; past it, the redraw waits.
	r.gh.Threads.Get(148).Set("body", before)
	full := py.Len(before) + py.Len(r.section()) - py.Len(dashboard.RunnersStart+"\nold runners\n"+
		dashboard.RunnersEnd)
	restore := patch(t, &dashboard.MaxBody, full-1)
	assertEqual(t, dashboard.RefreshRunners(r.ctx), "redrew #148's runners")
	restore()
	assertTrue(t, py.Len(bodyOf()) <= full-1, "the body is not shorter")
	assertIn(t, "…and 2 more.", bodyOf())
	r.gh.Threads.Get(148).Set("body", before)
	restore = patch(t, &dashboard.MaxBody, py.Len(before))
	assertIn(t, "would pass GitHub's limit", dashboard.RefreshRunners(r.ctx))
	restore()
	assertEqual(t, bodyOf(), before)
}

// `--every 300 --runners-every 120`, as bot-status runs it: the rewrites keep their five minutes,
// and between them the runners are redrawn every two; a failed redraw is only a warning.
func TestDashboard_TheLoopRedrawsTheRunnersBetweenRewrites(t *testing.T) {
	now := 0.0
	type tick struct {
		what string
		at   float64
	}
	var ticks []tick

	update := func(*hctx.Context, []string, bool, func() []string, bool, cardsets.Fetch) string {
		ticks = append(ticks, tick{"rewrite", now})
		return "rewrote #148"
	}

	refresh := func(*hctx.Context) string {
		ticks = append(ticks, tick{"runners", now})
		if len(ticks) == 3 {
			panic(errors.New("network down"))
		}
		return "redrew #148's runners"
	}

	patch[dashboardUpdateFunc](t, &dashboard.Update, update)
	patch(t, &dashboard.RefreshRunners, refresh)
	dashboardOffline(t, nil)
	patch(t, &cli.Monotonic, func() float64 { return now })
	patch(t, &cli.Sleep, func(s float64) { now += s })
	out, _ := capture(t, &cli.Stdout)
	cli.CmdDashboard(testsupport.MakeConfig(), &cli.Args{Every: 300, RunnersEvery: 120, ForSeconds: 600})
	assertEqual(t, ticks, []tick{{"rewrite", 0.0}, {"runners", 120.0}, {"runners", 240.0},
		{"rewrite", 300.0}, {"runners", 420.0}, {"runners", 540.0},
		{"rewrite", 600.0}})
	assertIn(t, "::warning::the runners were not redrawn: network down", out.String())
	assertIn(t, "runners: redrew #148's runners", out.String())
}
