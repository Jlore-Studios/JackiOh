// The status loop's planner (`controller.py`, #1455): with `controller` on, bot-status's loop plans
// the work and starts each night run with its plan, so the run skips its gate and plan jobs; and it
// delivers each such run once it ends (#1537), so the run skips its deliver job too.
// (bot/tests/test_controller.py)
package tests

import (
	"archive/zip"
	"bytes"
	"fmt"
	"io/fs"
	"net/http"
	"os"
	"path/filepath"
	"slices"
	"sort"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cli"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/controller"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/sweep"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

var controllerStatus = ".github/workflows/bot-status.yml"

// controllerNightYML is the repository's .github/workflows/bot-night.yml.
var controllerNightYML = filepath.Join(testsupport.Root, ".github", "workflows", "bot-night.yml")

// controllerLoopCtx is loop_ctx(gh, at=DAY, loop=True).
func controllerLoopCtx(gh *fakes.FakeGitHub, loop bool, at ...time.Time) *hctx.Context {
	when := testsupport.Day
	if len(at) > 0 {
		when = at[0]
	}
	cfg := testsupport.MakeConfig(testsupport.Config{Env: providersSecrets(providers.SecretNames...),
		Machine: []string{"muse"}, Overrides: pyjson.New("controller", true)})
	replaced := *cfg
	replaced.ControllerLoop, replaced.RunID = loop, "777"
	return testsupport.MakeCtx(gh, testsupport.Ctx{At: when, Cfg: &replaced})
}

// controllerListing makes GitHub list each dispatched night run under the title bot-night.yml's
// run-name gives (run_id = 5001).
func controllerListing(c *fakes.FakeGitHub) {
	runID := 5001
	real := c.Patch.DispatchWorkflow
	if real == nil {
		real = c.Own().DispatchWorkflow
	}
	c.Patch.DispatchWorkflow = func(workflow string, ref string, inputs map[string]string) {
		real(workflow, ref, inputs)
		if inputs["plan_ref"] != "" {
			number := runID + len(c.Dispatches)
			c.Runs.Set(strconv.Itoa(number), pyjson.New("id", number, "status", "queued",
				"path", ".github/workflows/bot-night.yml",
				"display_title", fmt.Sprintf("bot-night %s on %s", inputs["plan_ref"], inputs["runs_on"])))
		}
	}
}

func controllerRefuse() { herr.Raise(&herr.GitHubError{Msg: "no", Status: 500}) }

func controllerStarted(notes []string) []string {
	out := []string{}
	for _, n := range notes {
		if strings.HasPrefix(n, "started") {
			out = append(out, n)
		}
	}
	return out
}

func controllerAny(notes []string, prefix string) bool {
	return slices.ContainsFunc(notes, func(n string) bool { return strings.HasPrefix(n, prefix) })
}

func controllerFetch(t *testing.T, ctx *hctx.Context, ref string) *pyjson.Object {
	t.Helper()
	planned, err := controller.Fetch(ctx, ref)
	if err != nil {
		t.Fatalf("fetch %s: %v", ref, err)
	}
	return planned
}

// ---------------------------------------------------------------------------- StartTests

func TestController_TheLoopClaimsWritesThePlanAndStartsTheRunWithIt(t *testing.T) {
	c := fakes.NewFakeGitHub()
	c.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	controllerListing(c)
	ctx := controllerLoopCtx(c, true)
	notes := controller.Start(ctx, noSleep)
	assertIn(t, "started build #3 on muse (run 5002)", notes)
	var planned []fakes.Dispatch
	for _, d := range c.Dispatches {
		if d.Inputs["plan_ref"] != "" {
			planned = append(planned, d)
		}
	}
	assertEqual(t, len(planned), 1)
	inputs := planned[0].Inputs
	assertEqual(t, []string{inputs["runs_on"], inputs["cli"], inputs["secret"], inputs["shared"]},
		[]string{"night-vm-muse", "muse", "", "false"})
	assertTrue(t, controller.Ref.MatchString(inputs["plan_ref"]), inputs["plan_ref"])
	text, _ := c.GetFile(fmt.Sprintf("plans/%s.json", inputs["plan_ref"]), config.StateBranch)
	written := loadsObject(t, string([]byte(text)))
	assertEqual(t, []any{written.Get("number"), written.Get("provider")}, []any{3, "muse"})
	record := ctx.Store.Load().Obj("items").Obj("3")
	assertEqual(t, []any{record.Get("run_id"), record.Get("provider")}, []any{"5002", "muse"})
	assertIn(t, config.LabelWorking, c.LabelNames(3))
	// The lock is the loop's only while it plans.
	assertEqual(t, ctx.Store.Load().Get("planning"), nil)
}

func TestController_ItPlansAgainUntilNothingMoreCanStart(t *testing.T) {
	c := fakes.NewFakeGitHub()
	for n := 3; n < 10; n++ {
		c.AddIssue(n, fakes.Issue{Labels: []string{config.LabelBuild}})
	}
	controllerListing(c)
	ctx := controllerLoopCtx(c, true)
	notes := controller.Start(ctx, noSleep)
	started := controllerStarted(notes)
	// Muse has four lanes of its own (one runner on the night box, three on the Muse box), and
	// fills gpt's and agy's on the night box while they are off (`fill_in`): six at most; the
	// seventh item waits (by day no Claude account works).
	assertEqual(t, len(started), 6, notes)
	assertTrue(t, strings.HasPrefix(notes[len(notes)-1], "nothing more to start"), notes)
	// Each plan run is the loop's: none of them started another run as a plan job's `fill_lanes`
	// does.
	runners := []string{}
	for _, d := range c.Dispatches {
		runners = append(runners, d.Inputs["runs_on"])
	}
	assertEqual(t, runners, []string{"night-vm-muse", "night-vm-muse", "night-vm-muse", "night-vm-muse",
		"night-vm-muse-fill", "night-vm-muse-fill"})
}

func TestController_ARunNotListedYetKeepsItsClaimPendingUntilFound(t *testing.T) {
	c := fakes.NewFakeGitHub()
	c.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	ctx := controllerLoopCtx(c, true)
	notes := controller.Start(ctx, noSleep)
	assertIn(t, "started build #3 on muse (its run is not listed yet)", notes)
	runID := pyjson.Str(ctx.Store.Load().Obj("items").Obj("3").Get("run_id"))
	assertTrue(t, strings.HasPrefix(runID, plan.Pending), runID)
	// Pending holds the lane for a while, then counts as dead.
	assertEqual(t, plan.RunStatus(ctx, runID), "alive")
	later := testsupport.MakeCtx(c, testsupport.Ctx{At: testsupport.Day.Add(plan.PendingGrace + time.Minute),
		Cfg: ctx.Cfg})
	assertEqual(t, plan.RunStatus(later, runID), "dead")
	// Once GitHub lists it, the next tick puts its id on the claim.
	ref := runID[len(plan.Pending):]
	c.Runs.Set("6001", pyjson.New("id", 6001, "status", "in_progress",
		"display_title", fmt.Sprintf("bot-night %s on night-vm-muse", ref)))
	assertEqual(t, plan.RunStatus(ctx, runID), "alive")
	assertEqual(t, controller.Resolve(ctx), []string{"found the run of #3: 6001"})
	assertEqual(t, ctx.Store.Load().Obj("items").Obj("3").Get("run_id"), "6001")
}

func TestController_ADispatchGithubRefusesLeavesTheClaimDeadForHousekeeping(t *testing.T) {
	c := fakes.NewFakeGitHub()
	c.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	ctx := controllerLoopCtx(c, true)
	c.Patch.DispatchWorkflow = func(string, string, map[string]string) { controllerRefuse() }
	notes := controller.Start(ctx, noSleep)
	assertTrue(t, controllerAny(notes, "could not start build #3"), notes)
	assertEqual(t, ctx.Store.Load().Obj("items").Obj("3").Get("run_id"), "")
	assertEqual(t, plan.RunStatus(ctx, ""), "dead")
}

func TestController_AnotherPlannerHoldingTheLockWaitsTheLoopATick(t *testing.T) {
	c := fakes.NewFakeGitHub()
	c.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	ctx := controllerLoopCtx(c, true)
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.Set("planning", pyjson.New("owner", "run 42", "at", clock.ISO(testsupport.Day)))
		return nil
	}, "state")
	notes := controller.Start(ctx, noSleep)
	assertEqual(t, notes, []string{"another planner holds the planning lock: the next tick plans"})
	assertEqual(t, c.Dispatches, []fakes.Dispatch{})
	// A lock older than its TTL was left by a planner that died.
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.Set("planning", pyjson.New("owner", "run 42",
			"at", clock.ISO(testsupport.Day.Add(-plan.LockTtl-time.Second))))
		return nil
	}, "state")
	controllerListing(c)
	assertTrue(t, controllerAny(controller.Start(ctx, noSleep), "started"))
}

// ---------------------------------------------------------------------------- LockTests

func TestController_TheLoopRenewsTheLockBeforeEachStart(t *testing.T) {
	c := fakes.NewFakeGitHub()
	for _, n := range []int{3, 4} {
		c.AddIssue(n, fakes.Issue{Labels: []string{config.LabelBuild}})
	}
	controllerListing(c)
	ctx := controllerLoopCtx(c, true)
	taken := []string{}
	real := plan.TakePlanningLock
	func() {
		defer func() { plan.TakePlanningLock = real }()
		plan.TakePlanningLock = func(c *hctx.Context, owner string) bool {
			taken = append(taken, owner)
			return real(c, owner)
		}
		controller.Start(ctx, noSleep)
	}()
	// Once to start, then again before every later start (two runs, then the last plan).
	assertEqual(t, taken, []string{"loop 777", "loop 777", "loop 777"})
	assertEqual(t, ctx.Store.Load().Get("planning"), nil)
}

// ---------------------------------------------------------------------------- CheckTests

func controllerPlanned(changes ...any) *pyjson.Object {
	return pyjson.New("provider", "claude-3", "runs_on", "ubuntu-latest", "cli", "claude",
		"secret", "CLAUDE_CODE_OAUTH_TOKEN_3", "shared", false).Update(pyjson.New(changes...))
}

func controllerGiven(changes ...string) map[string]string {
	out := map[string]string{"runs_on": "ubuntu-latest", "cli": "claude", "secret": "CLAUDE_CODE_OAUTH_TOKEN_3",
		"shared": "false"}
	for i := 0; i+1 < len(changes); i += 2 {
		out[changes[i]] = changes[i+1]
	}
	return out
}

func TestController_TheDispatchMustNameTheSubscriptionsOwnRunnerCLIAndSecret(t *testing.T) {
	ctx := controllerLoopCtx(fakes.NewFakeGitHub(), true)
	assertEqual(t, controller.Check(ctx, controllerPlanned(), controllerGiven()), []string{})
	// Another subscription's secret, or the bot's own token, never reaches a model job.
	for _, bad := range [][2]string{{"secret", "CLAUDE_CODE_OAUTH_TOKEN"}, {"secret", "BOT_GITHUB_TOKEN"},
		{"runs_on", "night-vm-gpt"}, {"cli", "codex"}, {"shared", "true"}} {
		assertTrue(t, len(controller.Check(ctx, controllerPlanned(), controllerGiven(bad[0], bad[1]))) > 0, bad)
	}
	assertTrue(t, len(controller.Check(ctx, controllerPlanned("secret", "BOT_GITHUB_TOKEN"),
		controllerGiven("secret", "BOT_GITHUB_TOKEN"))) > 0)
	assertTrue(t, len(controller.Check(ctx, controllerPlanned("provider", "nobody"), controllerGiven())) > 0)
}

func TestController_FetchReadsOnlyAPlanReference(t *testing.T) {
	c := fakes.NewFakeGitHub()
	ctx := controllerLoopCtx(c, true)
	for _, ref := range []string{"../state", "plans/x", "abcd-1", ""} {
		_, err := controller.Fetch(ctx, ref)
		_, value := err.(*controller.ValueError)
		assertTrue(t, value, ref, err)
	}
	_, err := controller.Fetch(ctx, "0123abcd-1760000000") // no such file
	_, value := err.(*controller.ValueError)
	assertTrue(t, value, err)
}

func TestController_PlanFetchStopsTheJobOnAMismatch(t *testing.T) {
	c := fakes.NewFakeGitHub()
	c.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	controllerListing(c)
	ctx := controllerLoopCtx(c, true)
	controller.Start(ctx, noSleep)
	ref := c.Dispatches[len(c.Dispatches)-1].Inputs["plan_ref"]
	args, _, ok := cli.Parser().ParseArgs([]string{"plan-fetch", "--ref", ref, "--out", "/dev/null", "--check"})
	assertTrue(t, ok)
	env := map[string]string{"PLAN_RUNS_ON": "night-vm-muse", "PLAN_CLI": "muse", "PLAN_SECRET": "",
		"PLAN_SHARED": "false"}
	patch(t, &cli.Ctx, func(cfg *config.Config, write bool) *hctx.Context { return ctx })
	patch(t, &cli.GithubEnv, func() map[string]string { return env })
	_, restore := capture(t, &cli.Stdout)
	assertEqual(t, cli.CmdPlanFetch(ctx.Cfg, args), 0)
	restore()
	env["PLAN_SECRET"] = "CLAUDE_CODE_OAUTH_TOKEN_3"
	_, restore = capture(t, &cli.Stderr)
	assertEqual(t, cli.CmdPlanFetch(ctx.Cfg, args), 2)
	restore()
}

// ---------------------------------------------------------------------------- PruneTests

func TestController_PlanFilesOlderThanAWeekGo(t *testing.T) {
	c := fakes.NewFakeGitHub()
	ctx := controllerLoopCtx(c, true)
	c.CreateOrphanBranch(config.StateBranch, "state.json", "{}", "state", nil)
	old := fmt.Sprintf("0123abcd-%d", testsupport.Day.Add(-8*24*time.Hour).Unix())
	young := fmt.Sprintf("4567abcd-%d", testsupport.Day.Add(-24*time.Hour).Unix())
	for _, ref := range []string{old, young} {
		c.PutFile(fmt.Sprintf("plans/%s.json", ref), "{}", config.StateBranch, "", "plan")
	}
	c.PutFile("plans/notes.txt", "x", config.StateBranch, "", "other")
	assertEqual(t, controller.Prune(ctx), []string{"pruned 1 plan file(s) older than 7 days"})
	// get_file's text is None for a missing file: the Go client says so with an empty sha.
	_, sha := c.GetFile(fmt.Sprintf("plans/%s.json", old), config.StateBranch)
	assertEqual(t, sha, "")
	_, sha = c.GetFile(fmt.Sprintf("plans/%s.json", young), config.StateBranch)
	assertNotEqual(t, sha, "")
	_, sha = c.GetFile("plans/notes.txt", config.StateBranch)
	assertNotEqual(t, sha, "")
}

// ---------------------------------------------------------------------------- WhileTheLoopGoesTests

// Only the loop plans while it goes: every other path that used to start a run leaves the work to
// it, and bot-night's gate defers to it.

// controllerAlive is WhileTheLoopGoesTests.alive: bot-status's loop is running (and ticking, with
// a context).
func controllerAlive(c *fakes.FakeGitHub, ctx *hctx.Context) {
	c.Runs.Set("900", pyjson.New("id", 900, "status", "in_progress", "path", controllerStatus))
	if ctx != nil {
		ctx.Store.Update(func(s *pyjson.Object) any {
			s.Set("loop_tick", clock.ISO(ctx.Now()))
			return nil
		}, "state")
	}
}

func TestController_ALoopWhoseTicksStoppedCountsAsGone(t *testing.T) {
	c := fakes.NewFakeGitHub()
	ctx := controllerLoopCtx(c, false)
	controllerAlive(c, ctx)
	assertTrue(t, plan.LoopAlive(ctx))
	later := testsupport.MakeCtx(c, testsupport.Ctx{At: testsupport.Day.Add(plan.LoopStale + time.Minute), Cfg: ctx.Cfg})
	assertFalse(t, plan.LoopAlive(later))
	// Without `controller` nothing defers to a loop.
	off := *ctx.Cfg
	off.Controller = false
	assertFalse(t, plan.LoopAlive(testsupport.MakeCtx(c, testsupport.Ctx{At: testsupport.Day, Cfg: &off})))
}

func TestController_TheLoopsSweepRecordsItsTick(t *testing.T) {
	c := fakes.NewFakeGitHub()
	ctx := controllerLoopCtx(c, true)
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.Set("last_sweep", pyjson.New("at", clock.ISO(testsupport.Day), "since", clock.ISO(testsupport.Day)))
		return nil
	}, "state")
	patch(t, &controller.Sleep, noSleep)
	sweep.Sweep(ctx)
	assertEqual(t, ctx.Store.Load().Get("loop_tick"), clock.ISO(testsupport.Day))
}

func TestController_AnotherSweepStartsNoRun(t *testing.T) {
	c := fakes.NewFakeGitHub()
	c.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	ctx := controllerLoopCtx(c, false)
	controllerAlive(c, ctx)
	assertEqual(t, sweep.NightRun(ctx, testsupport.Day), []string{})
	assertEqual(t, c.Dispatches, []fakes.Dispatch{})
	// With no loop going it starts a plain run, as before.
	c.Runs.Delete("900")
	assertEqual(t, len(sweep.NightRun(ctx, testsupport.Day)), 1)
	_, given := c.Dispatches[0].Inputs["plan_ref"]
	assertFalse(t, given)
}

func TestController_TheLoopsSweepPlansAndStartsTheRun(t *testing.T) {
	c := fakes.NewFakeGitHub()
	c.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	controllerListing(c)
	ctx := controllerLoopCtx(c, true)
	patch(t, &controller.Sleep, noSleep)
	notes := sweep.NightRun(ctx, testsupport.Day)
	assertIn(t, "started build #3 on muse (run 5002)", notes)
}

func TestController_TheGateDefersUnlessAPersonAskedForAnItem(t *testing.T) {
	c := fakes.NewFakeGitHub()
	c.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	ctx := controllerLoopCtx(c, false)
	controllerAlive(c, ctx)
	outputs := pyjson.New()
	patch(t, &cli.Ctx, func(cfg *config.Config, write bool) *hctx.Context { return ctx })
	patch(t, &cli.Output, func(pairs *pyjson.Object) { outputs.Update(pairs) })
	patch(t, &cli.Summary, func(text string) {})
	capture(t, &cli.Stdout)
	args, _, _ := cli.Parser().ParseArgs([]string{"peek"})
	cli.CmdPeek(ctx.Cfg, args)
	assertEqual(t, outputs.Get("work"), "false")
	args, _, _ = cli.Parser().ParseArgs([]string{"peek", "--item", "3"})
	cli.CmdPeek(ctx.Cfg, args)
	assertEqual(t, outputs.Get("work"), "true")
}

func TestController_APlanJobWaitsForTheLockThenPlansNothing(t *testing.T) {
	c := fakes.NewFakeGitHub()
	c.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	ctx := controllerLoopCtx(c, false)
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.Set("planning", pyjson.New("owner", "loop 1", "at", clock.ISO(testsupport.Day)))
		return nil
	}, "state")
	args, _, _ := cli.Parser().ParseArgs([]string{"plan", "--out", "/dev/null"})
	patch(t, &cli.Ctx, func(cfg *config.Config, write bool) *hctx.Context { return ctx })
	patch(t, &cli.LockTries, 1)
	patch(t, &cli.Output, func(pairs *pyjson.Object) {})
	patch(t, &cli.Summary, func(text string) {})
	// (The Python also replaced Path.write_text, so the plan went nowhere: it goes to /dev/null.)
	out, restore := capture(t, &cli.Stdout)
	cli.CmdPlan(ctx.Cfg, args)
	restore()
	assertIn(t, "the status loop is planning", out.String())
	assertNotIn(t, config.LabelWorking, c.LabelNames(3))
}

func TestController_DeliveryLeavesTheNextRunToTheLoop(t *testing.T) {
	c := fakes.NewFakeGitHub()
	ctx := controllerLoopCtx(c, false)
	controllerAlive(c, ctx)
	deliverer := &deliver.Deliverer{}
	deliverer.Ctx, deliverer.Chain, deliverer.Log = ctx, true, []string{}
	deliverer.Chain_()
	assertEqual(t, deliverer.Log, []string{"the status loop starts the next run (#1455)"})
	assertEqual(t, c.Dispatches, []fakes.Dispatch{})
}

// controllerZipped is `folder` as GitHub hands back an upload of it: a zip of its files, by
// relative path, then `extra` (name, data, ...).
func controllerZipped(t *testing.T, folder string, extra ...string) []byte {
	t.Helper()
	var buffer bytes.Buffer
	archive := zip.NewWriter(&buffer)
	var paths []string
	err := filepath.WalkDir(folder, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if path != folder {
			paths = append(paths, path)
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	sort.Strings(paths)
	add := func(name string, data []byte) {
		w, err := archive.CreateHeader(&zip.FileHeader{Name: name, Method: zip.Store})
		if err == nil {
			_, err = w.Write(data)
		}
		if err != nil {
			t.Fatal(err)
		}
	}
	for _, path := range paths {
		info, err := os.Stat(path)
		if err != nil {
			t.Fatal(err)
		}
		if !info.Mode().IsRegular() {
			continue
		}
		data, err := os.ReadFile(path)
		if err != nil {
			t.Fatal(err)
		}
		rel, _ := filepath.Rel(folder, path)
		add(filepath.ToSlash(rel), data)
	}
	for i := 0; i+1 < len(extra); i += 2 {
		add(extra[i], []byte(extra[i+1]))
	}
	if err := archive.Close(); err != nil {
		t.Fatal(err)
	}
	return buffer.Bytes()
}

func controllerEnded(c *fakes.FakeGitHub, runID string, at time.Time) {
	c.Runs.Obj(runID).Update(pyjson.New("status", "completed", "conclusion", "success", "updated_at", clock.ISO(at)))
}

// ---------------------------------------------------------------------------- DeliveryTests

// The loop delivers the runs it started (#1537): bot-night gives them no deliver job, which waited
// 7 to 25 minutes for a hosted runner to run for under a minute.

// controllerStartedBuild is DeliveryTests.started: a build of #12 the loop started at night, its
// model job done: (harness, ctx, run id).
func controllerStartedBuild(t *testing.T) (*flowHarness, *hctx.Context, string) {
	t.Helper()
	h := newFlowHarness(t, flowHarnessOpts{Overrides: pyjson.New("controller", true)})
	cfg := *h.cfg
	cfg.ControllerLoop, cfg.RunID = true, "777"
	ctx := testsupport.MakeCtx(h.gh, testsupport.Ctx{At: testsupport.Night, Cfg: &cfg})
	h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: []string{config.LabelBuild}})
	controllerListing(h.gh)
	notes := controller.Start(ctx, noSleep)
	started := controllerStarted(notes)
	if len(started) != 1 {
		t.Fatalf("not one start: %q", notes)
	}
	at := strings.LastIndex(started[0], "run ")
	runID := strings.TrimRight(started[0][at+len("run "):], ")")
	ref := h.gh.Dispatches[len(h.gh.Dispatches)-1].Inputs["plan_ref"]
	planned := controllerFetch(t, ctx, ref)
	out := filepath.Join(h.root, "out")
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"build":  {workBuilder(map[string]string{"src/game.txt": "rules v2\n"})},
		"review": {workReviewer(workApprove)}})
	work.NewWorker(h.cfg, planned, r, h.clone, filepath.Join(h.root, "work"), out, nil, nil, nil, nil, nil, nil).Run()
	h.gh.Artifacts[runID] = []*pyjson.Object{pyjson.New("id", 1, "name", "work", "expired", false)}
	h.gh.ArtifactZips[1] = controllerZipped(t, out)
	controllerEnded(h.gh, runID, testsupport.Night)
	return h, ctx, runID
}

func TestController_TheLoopDeliversARunItStartedWithinATickOfItsWorkEnding(t *testing.T) {
	h, ctx, runID := controllerStartedBuild(t)
	// Until it is delivered the run holds its lane and housekeeping leaves its item alone.
	assertEqual(t, plan.RunStatus(ctx, runID), "alive")
	assertNotIn(t, "requeued #12 from a dead run", plan.Housekeeping(ctx, ctx.Store.Load()))
	scratch := filepath.Join(h.root, "scratch")
	notes := controller.Deliver(ctx, scratch)
	assertEqual(t, notes, []string{fmt.Sprintf("delivered build #12 on claude-1 (run %s): approved", runID)})
	// The same delivery the run's own deliver job made: the branch and an auto-merging pull
	// request.
	result := switchResult(t, filepath.Join(h.root, "out"))
	assertEqual(t, h.originSHA("bot/issue-12"), result.Get("head"))
	pulls := h.gh.ListPulls("open", "bot/issue-12", 200)
	assertEqual(t, len(pulls), 1)
	pull := pulls[0]
	_, merging := h.gh.AutoMerge[pyjson.Str(pull.Get("node_id"))]
	assertTrue(t, merging, pull.Get("node_id"))
	number, _ := pyjson.AsInt(pull.Get("number"))
	assertIn(t, config.LabelPR, h.gh.LabelNames(number))
	record := ctx.Store.Load().Obj(plan.Deliveries).Obj(runID)
	assertEqual(t, []any{record.Get("done"), record.Get("tries"), record.Get("status")}, []any{true, 1, "approved"})
	// Delivered once only; the scratch clone is gone; the run is over.
	assertEqual(t, controller.Deliver(ctx, scratch), []string{})
	left, err := os.ReadDir(scratch)
	if err != nil {
		t.Fatal(err)
	}
	assertEqual(t, len(left), 0)
	assertEqual(t, plan.RunStatus(ctx, runID), "dead")
	// The loop plans the next run itself, so the delivery started none.
	assertEqual(t, len(h.gh.Dispatches), 1)
}

func TestController_ARunWhoseModelJobLeftNoUploadIsDeliveredAsOneWithNoResult(t *testing.T) {
	h, ctx, runID := controllerStartedBuild(t)
	h.gh.Artifacts[runID] = []*pyjson.Object{}
	notes := controller.Deliver(ctx, filepath.Join(h.root, "scratch"))
	assertEqual(t, len(notes), 1, notes)
	assertEqual(t, notes[0], fmt.Sprintf("delivered build #12 on claude-1 (run %s): infra "+
		"(its model job left no upload)", runID))
	var said []string
	for _, comment := range h.gh.BotComments(12) {
		if strings.Contains(comment, deliver.NoResult) {
			said = append(said, comment)
		}
	}
	assertEqual(t, len(said), 1, said)
	// Its link names the night run, not the loop's.
	assertIn(t, fmt.Sprintf("actions/runs/%s)", runID), said[0])
	assertNotIn(t, "actions/runs/777", said[0])
}

// controllerFailing is DeliveryTests.failing: a run the loop started for #3, ended, whose plan
// file is missing: every try fails.
func controllerFailing() (*fakes.FakeGitHub, *hctx.Context) {
	c := fakes.NewFakeGitHub()
	c.AddIssue(3, fakes.Issue{Labels: []string{config.LabelWorking}})
	ctx := controllerLoopCtx(c, true)
	ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, 3).Update(pyjson.New("run_id", "5002", "provider", "muse"))
		return nil
	}, "state")
	c.Runs.Set("5002", pyjson.New("id", 5002, "status", "completed", "updated_at", clock.ISO(testsupport.Day),
		"display_title", "bot-night 0123abcd-1760000000 on night-vm-muse"))
	return c, ctx
}

func TestController_AFailingDeliveryIsTriedEachTickThenLeftToHousekeeping(t *testing.T) {
	c, ctx := controllerFailing()
	for _, attempt := range []int{1, 2} {
		// Path(self.id()).parent / "unused": the id is one dotted name, so its parent is ".".
		notes := controller.Deliver(ctx, "unused")
		assertEqual(t, len(notes), 1, notes)
		note := notes[0]
		assertTrue(t, strings.HasPrefix(note, fmt.Sprintf("could not deliver run 5002, try %d of 3 "+
			"(ValueError: no plan", attempt)), note)
		assertTrue(t, strings.HasSuffix(note, fmt.Sprintf("it is tried again next tick (%d more)", 3-attempt)), note)
		// Meanwhile the item is not requeued.
		assertEqual(t, plan.RunStatus(ctx, "5002"), "alive")
	}
	notes := controller.Deliver(ctx, "")
	assertEqual(t, len(notes), 1, notes)
	assertTrue(t, strings.HasSuffix(notes[0], "its item is left to housekeeping, as a dead run's"), notes[0])
	assertEqual(t, controller.Deliver(ctx, ""), []string{})
	assertEqual(t, plan.RunStatus(ctx, "5002"), "dead")
	assertIn(t, "requeued #3 from a dead run", plan.Housekeeping(ctx, ctx.Store.Load()))
	assertEqual(t, c.LabelNames(3), map[string]bool{config.LabelBuild: true})
}

func TestController_ARunTheLoopNeverDeliveredIsHousekeepingsAfterTheWindow(t *testing.T) {
	c, ctx := controllerFailing()
	assertEqual(t, plan.RunStatus(ctx, "5002"), "alive")
	later := testsupport.MakeCtx(c, testsupport.Ctx{At: testsupport.Day.Add(plan.DeliverWithin), Cfg: ctx.Cfg})
	assertEqual(t, plan.RunStatus(later, "5002"), "dead")
	assertEqual(t, controller.Deliver(later, ""), []string{})
	// A run nobody started with a plan never waits for the loop.
	c.Runs.Set("5003", pyjson.New("id", 5003, "status", "completed", "updated_at", clock.ISO(testsupport.Day),
		"display_title", "bot-night"))
	assertEqual(t, plan.RunStatus(ctx, "5003"), "dead")
}

func TestController_NothingInADeliveryEndsTheTick(t *testing.T) {
	c, ctx := controllerFailing()
	c.Patch.ListRuns = func(string, string, int, bool) []*pyjson.Object {
		controllerRefuse()
		return nil
	}
	assertEqual(t, controller.Deliver(ctx, ""), []string{"could not list the night runs to deliver: no"})
}

func TestController_AnUploadHoldingAPathOutsideItIsRefused(t *testing.T) {
	h, ctx, _ := controllerStartedBuild(t)
	h.gh.ArtifactZips[1] = controllerZipped(t, filepath.Join(h.root, "out"), "../escaped.txt", "x")
	scratch := filepath.Join(h.root, "scratch")
	notes := controller.Deliver(ctx, scratch)
	assertEqual(t, len(notes), 1, notes)
	assertIn(t, "the upload holds a path outside it", notes[0])
	_, err := os.Stat(filepath.Join(scratch, "escaped.txt"))
	assertTrue(t, os.IsNotExist(err), err)
	assertEqual(t, h.originSHA("bot/issue-12"), "")
}

func TestController_TheLoopsSweepDeliversBeforeItPlans(t *testing.T) {
	c := fakes.NewFakeGitHub()
	ctx := controllerLoopCtx(c, true)
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.Set("last_sweep", pyjson.New("at", clock.ISO(testsupport.Day), "since", clock.ISO(testsupport.Day)))
		return nil
	}, "state")
	order := []string{}
	func() {
		deliverWas, nightWas := controller.Deliver, sweep.NightRun
		defer func() { controller.Deliver, sweep.NightRun = deliverWas, nightWas }()
		controller.Deliver = func(c *hctx.Context, workspace string) []string {
			order = append(order, "deliver")
			return []string{}
		}
		sweep.NightRun = func(c *hctx.Context, since time.Time) []string {
			order = append(order, "plan")
			return []string{}
		}
		sweep.Sweep(ctx)
	}()
	assertEqual(t, order, []string{"deliver", "plan"})
	// Any other sweep leaves the delivering to the loop.
	cfg := *ctx.Cfg
	cfg.ControllerLoop = false
	other := testsupport.MakeCtx(c, testsupport.Ctx{At: testsupport.Day, Cfg: &cfg})
	patch(t, &controller.Deliver, func(c *hctx.Context, workspace string) []string {
		t.Fatal("delivered")
		return nil
	})
	assertEqual(t, sweep.Deliveries(other, testsupport.Day), []string{})
}

func TestController_BotNightGivesARunTheLoopStartedNoDeliverJob(t *testing.T) {
	data, err := os.ReadFile(controllerNightYML)
	if err != nil {
		t.Fatal(err)
	}
	text := string(data)
	at := strings.Index(text, "\n  deliver:\n")
	if at < 0 {
		t.Fatal("ValueError: substring not found")
	}
	job := text[at:]
	assertIn(t, "if: ${{ always() && !inputs.plan_ref && contains(", job)
	// The model job's upload keeps the name the loop downloads.
	assertIn(t, fmt.Sprintf("name: %s\n", controller.WorkArtifact), text[:at])
}

// ---------------------------------------------------------------------------- DownloadTests

func TestController_AnArtifactIsFetchedFromItsRedirectWithoutTheToken(t *testing.T) {
	client := gh.NewGitHub("jgoetzmann/JackiOh", "secret-token", false, nil, nil, "")
	var asked []*gh.Request
	patch(t, &gh.BuildOpener, func() gh.Opener {
		return func(request *gh.Request, timeout time.Duration) (*gh.Response, error) {
			asked = append(asked, request)
			header := http.Header{}
			header.Set("Location", "https://blob.example/work.zip")
			return nil, &gh.HTTPError{URL: request.URL, Code: 302, Msg: "Found", Header: header}
		}
	})
	patch(t, &gh.Urlopen, gh.Opener(func(request *gh.Request, timeout time.Duration) (*gh.Response, error) {
		asked = append(asked, request)
		return &gh.Response{Body: []byte("zip bytes")}, nil
	}))
	assertEqual(t, string(client.DownloadArtifact(7)), "zip bytes")
	assertTrue(t, strings.HasSuffix(asked[0].URL, "/repos/jgoetzmann/JackiOh/actions/artifacts/7/zip"), asked[0].URL)
	assertEqual(t, asked[0].Header.Get("Authorization"), "Bearer secret-token")
	// The storage URL is a plain string: no header, so no token, goes with it.
	assertEqual(t, asked[1].URL, "https://blob.example/work.zip")
	assertEqual(t, len(asked[1].Header), 0)
}
