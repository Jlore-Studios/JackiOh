// test_needs_plan.py: the Needs plan stage: every queued item gets a strong model's plan first, on
// a planning lane of its own that takes no build lane, and the plan goes into the issue's
// description, where the builder (a weak one above all) starts from it.
package tests

import (
	"slices"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/issueplan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// ---------------------------------------------------------------------------- this file's helpers

// needsPlanLane is lane_ctx's keyword arguments; a zero field is the default.
type needsPlanLane struct {
	At time.Time // at = NIGHT (the zero time is Night)
	// Machine is machine = ALL_MACHINE: nil is the default, an empty slice `()`.
	Machine     []string
	MaxParallel int               // max_parallel = 20 (0 is the default)
	Env         map[string]string // env = None: every secret set
}

// needsPlanLaneCtx is lane_ctx(gh, ...): every subscription, the committed planning lane, and
// room on the build lanes.
func needsPlanLaneCtx(gh *fakes.FakeGitHub, opts ...needsPlanLane) *hctx.Context {
	var o needsPlanLane
	if len(opts) > 0 {
		o = opts[0]
	}
	at := o.At
	if at.IsZero() {
		at = testsupport.Night
	}
	machine := o.Machine
	if machine == nil {
		machine = testsupport.AllMachine
	}
	maxParallel := o.MaxParallel
	if maxParallel == 0 {
		maxParallel = 20
	}
	env := o.Env
	if len(env) == 0 { // `env or secrets(*providers.SECRETS)`
		env = providersSecrets(providers.SecretNames...)
	}
	cfg := testsupport.MakeConfig(testsupport.Config{Env: env, Machine: machine, CommittedPlanLanes: true})
	return testsupport.MakeCtx(gh, testsupport.Ctx{At: at, Cfg: testsupport.WithLanes(cfg, maxParallel, maxParallel)})
}

// needsPlanRunning is running(gh, ctx, planned): the claimed run goes on: its lane stays held.
func needsPlanRunning(gh *fakes.FakeGitHub, ctx *hctx.Context, planned *pyjson.Object) {
	n := needsPlanNumber(planned)
	gh.Runs.Set(strconv.Itoa(n), pyjson.New("status", "in_progress"))
	ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, n).Set("run_id", strconv.Itoa(n))
		return nil
	}, "state")
}

// needsPlanPlannedBy is planned_by(ctx, number, tier="strong"): a planning run delivered: the item
// has its plan and its lane is free again.
func needsPlanPlannedBy(ctx *hctx.Context, number int) {
	ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, number).Set("planned_at", clock.ISO(testsupport.Night)).Set("planned_tier", "strong").
			Set("run_id", nil).Set("action", nil)
		return nil
	}, "state")
}

// needsPlanNumber is `planned["number"]`.
func needsPlanNumber(planned *pyjson.Object) int {
	n, _ := pyjson.AsInt(planned.Get("number"))
	return n
}

// needsPlanIs is assertEqual((planned[k] for k in keys), want).
func needsPlanIs(t *testing.T, planned *pyjson.Object, keys []string, want ...any) {
	t.Helper()
	got := []any{}
	for _, k := range keys {
		got = append(got, planned.Get(k))
	}
	if !pyjson.Equal(got, want) {
		t.Fatalf("%s != %s", py.Repr(got), py.Repr(want))
	}
}

// needsPlanFour is the four Claude accounts busy() holds in several tests: claude-3, claude-1,
// claude-4 and claude-2 on #50 to #53, so no strong model is free.
func needsPlanFour() []any {
	held := []any{}
	for i, p := range []string{"claude-3", "claude-1", "claude-4", "claude-2"} {
		held = append(held, p, 50+i)
	}
	return held
}

// ---------------------------------------------------------------------------- IssuePlanTests

func TestNeedsPlan_ThePlanSectionGoesInComesOutAndIsReplaced(t *testing.T) {
	body := issueplan.WithPlan("Make the rules v2.", "1. Edit `src/a.ts`.", "`claude-3`", "run 7")
	assertTrue(t, strings.HasPrefix(body, "Make the rules v2.\n\n"+issueplan.Start+"\n## Plan\n"))
	assertIn(t, "_Written by `claude-3` (run 7)", body)
	assertNotIn(t, config.Marker, body) // the description must not read as the bot's own
	assertEqual(t, issueplan.PlanOf(body), "1. Edit `src/a.ts`.")
	assertEqual(t, issueplan.WithoutPlan(body), "Make the rules v2.")
	again := issueplan.WithPlan(body, "1. Edit `src/b.ts`.", "`claude-1`", "")
	assertEqual(t, strings.Count(again, issueplan.Start), 1)
	assertEqual(t, issueplan.PlanOf(again), "1. Edit `src/b.ts`.")
	// A person's edit is the plan; text they add after the section stays theirs.
	edited := strings.ReplaceAll(again, "src/b.ts", "src/c.ts") + "\nAlso: thanks."
	assertEqual(t, issueplan.PlanOf(edited), "1. Edit `src/c.ts`.")
	assertEqual(t, issueplan.WithoutPlan(edited), "Make the rules v2.\n\nAlso: thanks.")
	assertEqual(t, issueplan.PlanOf("No plan here."), "")
	assertEqual(t, issueplan.PlanOf(""), "") // plan_of(None): the Go body is "" for None
}

func TestNeedsPlan_ALongPlanGivesWayToTheTask(t *testing.T) {
	task := strings.Repeat("t", 30_000)
	body := issueplan.WithPlan(task, strings.Repeat("p", 60_000), "`claude-3`", "")
	assertLessEqual(t, py.Len(body), issueplan.BodyLimit)
	assertTrue(t, strings.HasPrefix(body, task))
	assertIn(t, "cut to fit the description", body)
}

// ---------------------------------------------------------------------------- StageTests

func TestNeedsPlan_QueuedItemsWithoutAStrongPlanCarryTheLabel(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh)
	difficultyBusy(gh, ctx, needsPlanFour()...)       // nobody plans yet
	difficultyQueue(gh, ctx, 3, false)                // no plan
	difficultyQueue(gh, ctx, 4, true, difficultyEasy) // an easy item with a strong plan
	flowUpdateItem(ctx, 4, pyjson.New("planned_tier", "strong"))
	difficultyQueue(gh, ctx, 5, true, difficultyEasy) // an easy item with a medium plan
	flowUpdateItem(ctx, 5, pyjson.New("planned_tier", "medium"))
	difficultyQueue(gh, ctx, 6, true) // a medium item with a medium plan
	flowUpdateItem(ctx, 6, pyjson.New("planned_tier", "medium"))
	gh.AddIssue(7, fakes.Issue{Labels: []string{config.LabelNeedsPlan}}) // not queued any more
	flowMake(ctx)
	labelled := map[int]bool{}
	for _, n := range []int{3, 4, 5, 6, 7} {
		if gh.LabelNames(n)[config.LabelNeedsPlan] {
			labelled[n] = true
		}
	}
	// A medium plan is enough for an easy or medium item; only a hard one needs a strong model's
	// (#317 part 6, #695).
	assertEqual(t, labelled, map[int]bool{3: true})
}

// ---------------------------------------------------------------------------- PlanningLaneTests

// claude-3 plans while it builds; claude-1, capped like the rest, plans only when it holds
// nothing else; the weak one builds once a strong model planned it. The load balancer
// (`plan.balance`) gives the first plan to claude-1, which holds nothing, over claude-3, which
// builds; claude-2, claude-5, claude-6, claude-7 and claude-8, idle too, would plan the second,
// so they have no secret here.
func TestNeedsPlan_PlanningComesFirstEasyFirstAndTakesNoBuildLane(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	names := []string{}
	for _, s := range providers.SecretNames {
		if !slices.Contains([]string{"CLAUDE_CODE_OAUTH_TOKEN_2", "CLAUDE_CODE_OAUTH_TOKEN_5",
			"CLAUDE_CODE_OAUTH_TOKEN_6", "CLAUDE_CODE_OAUTH_TOKEN_7", "CLAUDE_CODE_OAUTH_TOKEN_8"}, s) {
			names = append(names, s)
		}
	}
	ctx := needsPlanLaneCtx(gh, needsPlanLane{Env: providersSecrets(names...)})
	difficultyBusy(gh, ctx, "claude-3", 50, "claude-4", 52,
		"agy", 53, "muse", 54, "gpt", 55)
	difficultyQueue(gh, ctx, 3, false, "priority:high")                // medium and urgent
	difficultyQueue(gh, ctx, 4, false, difficultyEasy, "priority:low") // easy: the weak one's, once planned
	planned := flowMake(ctx)
	needsPlanIs(t, planned, []string{"action", "number", "provider"}, "plan", 4, "claude-1") // claude-1 holds nothing else
	assertEqual(t, difficultySeatAt(t, planned, "plan"), difficultySeat{"claude-1", "opus", "strong"})
	assertEqual(t, gh.LabelNames(4), setOf(config.LabelWorking, config.LabelNeedsPlan, difficultyEasy,
		"priority:low"))
	needsPlanRunning(gh, ctx, planned)
	planned = flowMake(ctx)
	needsPlanIs(t, planned, []string{"action", "number", "provider"}, "plan", 3, "claude-3") // building #50 does not stop it planning
	needsPlanRunning(gh, ctx, planned)
	// Both planning lanes are taken, and claude-1, planning, does not build too.
	action := py.Str(flowMake(ctx).Get("action"))
	assertTrue(t, action != "plan" && action != "build", action)
	needsPlanPlannedBy(ctx, 4)
	gh.Runs.Delete("4")
	labels := []any{}
	for _, n := range []string{config.LabelBuild, difficultyEasy, "priority:low"} {
		labels = append(labels, pyjson.New("name", n))
	}
	gh.Threads.Item(4).Set("labels", labels)
	planned = flowMake(ctx)
	needsPlanIs(t, planned, []string{"action", "number", "provider"}, "build", 4, "lite")
	assertIsNone(t, difficultySeatAt(t, planned, "plan")) // built from the plan, not planned again
}

// Planning comes first: claude-3 (two lanes, no caps) plans two items at once, before it builds
// anything, so the medium models have plans to build from. Its third item it plans and builds in
// one run, on a build lane.
func TestNeedsPlan_AnUncappedAccountPlansAsManyItemsAtOnceAsItsLanes(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh, needsPlanLane{Machine: []string{}, Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN_3")})
	for _, number := range []int{3, 4, 5} {
		difficultyQueue(gh, ctx, number, false)
	}
	made := []*pyjson.Object{}
	for range 2 {
		made = append(made, flowMake(ctx))
		needsPlanRunning(gh, ctx, made[len(made)-1])
	}
	needsPlanIs(t, made[0], []string{"action", "number", "provider"}, "plan", 3, "claude-3")
	needsPlanIs(t, made[1], []string{"action", "number", "provider"}, "plan", 4, "claude-3")
	third := flowMake(ctx)
	needsPlanIs(t, third, []string{"action", "number"}, "build", 5)
	assertEqual(t, difficultySeatAt(t, third, "plan"), difficultySeat{"claude-3", "opus", "strong"})
}

// A fullsend part waits on its plan for a medium subscription (Muse) to build it, so the lane
// plans it before an older, harder item that is no part.
func TestNeedsPlan_TheLanePlansFullsendPartsBeforeTheRest(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh, needsPlanLane{Machine: []string{}, Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN_3")})
	difficultyQueue(gh, ctx, 3, false, "difficulty:hard")
	difficultyQueue(gh, ctx, 4, false)
	flowUpdateItem(ctx, 4, pyjson.New("onto", "bot/issue-9", "part_of", 9))
	planned := flowMake(ctx)
	needsPlanIs(t, planned, []string{"action", "number"}, "plan", 4)
}

// One lane (#695): while it holds a run it takes nothing more, neither a second build nor a plan
// — two runs deciding from one reading go past a cap together. Once the run ends, it plans.
func TestNeedsPlan_Claude1NeverPlansWhileItBuilds(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh, needsPlanLane{Machine: []string{}, Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN")})
	difficultyBusy(gh, ctx, "claude-1", 50)
	difficultyQueue(gh, ctx, 3, false)
	assertEqual(t, flowMake(ctx).Get("action"), "none")
	gh.Runs.Delete("50")
	assertEqual(t, flowMake(ctx).Get("action"), "plan")
}

// claude-2 (capped) has one lane (#695): while it holds a run it takes nothing more, neither a
// second build nor a plan. Once the run ends, it plans; the mid-run refusal checks still guard the
// spending.
func TestNeedsPlan_ACappedSubscriptionTakesNothingWhileHolding(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh, needsPlanLane{Machine: []string{}, Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN_2")})
	difficultyBusy(gh, ctx, "claude-2", 50)
	difficultyQueue(gh, ctx, 3, false)
	assertEqual(t, flowMake(ctx).Get("action"), "none")
	gh.Runs.Delete("50")
	planned := flowMake(ctx)
	needsPlanIs(t, planned, []string{"action", "provider"}, "plan", "claude-2")
	needsPlanRunning(gh, ctx, planned)
	difficultyQueue(gh, ctx, 4, true)
	assertEqual(t, flowMake(ctx).Get("action"), "none") // the lane is held again
}

// At 87% of its 5-hour window claude-2 (cap 90%, long runs start under 85%) starts neither a
// build nor a plan: #37's plan on claude-4 started just under its 70% cap and was cut off four
// minutes in. Under 85% it plans.
func TestNeedsPlan_HeadroomHoldsBackPlansAsItDoesBuilds(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh, needsPlanLane{Machine: []string{}, Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN_2")})
	later := clock.ISO(testsupport.Night.Add(2 * time.Hour))

	reading := func(utilization float64) {
		ctx.Store.Update(func(s *pyjson.Object) any {
			pyjson.Obj(s.SetDefault("providers", pyjson.New())).Update(pyjson.New("claude-2", pyjson.New(
				"usage", pyjson.New("five_hour", pyjson.New("utilization", utilization, "resets_at", later),
					"observed_at", clock.ISO(testsupport.Night)))))
			return nil
		}, "state")
	}
	reading(0.87)
	difficultyQueue(gh, ctx, 3, true) // planned: a build
	assertEqual(t, flowMake(ctx).Get("action"), "none")
	difficultyQueue(gh, ctx, 4, false) // not planned: a plan
	assertEqual(t, flowMake(ctx).Get("action"), "none")
	reading(0.8)
	planned := flowMake(ctx)
	needsPlanIs(t, planned, []string{"action", "number"}, "plan", 4)
}

// A planning run takes none of its subscription's lanes, but it holds a runner: a slot on the
// machine, or one of GitHub's lanes (#1455), so `max_parallel` caps every model job on GitHub's
// runners.
func TestNeedsPlan_APlanningRunTakesARunnerWhereItRuns(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh, needsPlanLane{MaxParallel: 2})
	// GitHub's two lanes and the machine's two slots, each held apart from the other: no runner is
	// free for a plan either.
	difficultyBusy(gh, ctx, "agy", 53, "muse", 54, "claude-4", 55,
		"claude-5", 56)
	difficultyQueue(gh, ctx, 3, false)
	assertEqual(t, flowMake(ctx).Get("action"), "none")
	// A slot free on the machine: a planner there plans, though GitHub's lanes are full.
	gh = fakes.NewFakeGitHub()
	ctx = needsPlanLaneCtx(gh, needsPlanLane{MaxParallel: 2})
	difficultyBusy(gh, ctx, "agy", 53, "claude-4", 55, "claude-5", 56)
	difficultyQueue(gh, ctx, 3, false)
	planned := flowMake(ctx)
	assertEqual(t, planned.Get("action"), "plan")
	assertTrue(t, ctx.Cfg.Pool.OnMachine(py.Str(planned.Get("provider"))), planned.Get("provider"))
	// A lane free on GitHub's runners: the strong planner there goes first.
	gh = fakes.NewFakeGitHub()
	ctx = needsPlanLaneCtx(gh, needsPlanLane{MaxParallel: 2})
	difficultyBusy(gh, ctx, "agy", 53, "muse", 54, "claude-4", 55)
	difficultyQueue(gh, ctx, 3, false)
	planned = flowMake(ctx)
	needsPlanIs(t, planned, []string{"action", "provider"}, "plan", "claude-3")
	// With the planning lane off, every lane is busy.
	gh = fakes.NewFakeGitHub()
	ctx = testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night, Cfg: testsupport.WithLanes(
		testsupport.MakeConfig(testsupport.Config{Env: providersSecrets(providers.SecretNames...),
			Machine: testsupport.Machine}), 2)})
	difficultyBusy(gh, ctx, "agy", 53, "muse", 54, "claude-4", 55,
		"claude-5", 56)
	difficultyQueue(gh, ctx, 3, false)
	assertIn(t, "every lane is busy", py.Str(flowMake(ctx).Get("reason")))
}

// By day (the Claude accounts keep to the night here) no strong model is free: an item nobody has
// rated or planned may still be rated and planned on the lane by a medium model (#317 part 8),
// Muse first; a rated medium item takes a medium planner too (#695).
func TestNeedsPlan_AMediumModelRatesAndPlansAnUnratedItemOnTheLane(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh, needsPlanLane{At: testsupport.Day, Machine: testsupport.AllMachine})
	difficultyQueue(gh, ctx, 3, false)
	planned := flowMake(ctx)
	needsPlanIs(t, planned, []string{"action", "provider"}, "plan", "muse")
	assertEqual(t, difficultySeatAt(t, planned, "plan"), difficultySeat{"muse", "muse-spark-1.3-contributor", "medium"})
	gh = fakes.NewFakeGitHub()
	ctx = needsPlanLaneCtx(gh, needsPlanLane{At: testsupport.Day, Machine: testsupport.AllMachine})
	difficultyQueue(gh, ctx, 3, false, "difficulty:medium")
	planned = flowMake(ctx)
	needsPlanIs(t, planned, []string{"action", "provider"}, "plan", "muse")
	assertEqual(t, difficultySeatAt(t, planned, "plan"), difficultySeat{"muse", "muse-spark-1.3-contributor", "medium"})
	// An easy item nobody planned is never the weak one's.
	gh = fakes.NewFakeGitHub()
	ctx = needsPlanLaneCtx(gh, needsPlanLane{At: testsupport.Day, Machine: []string{"lite"}})
	difficultyQueue(gh, ctx, 3, false, difficultyEasy)
	assertEqual(t, flowMake(ctx).Get("action"), "none")
	assertTrue(t, gh.LabelNames(3)[config.LabelNeedsPlan])
}

func TestNeedsPlan_APlanWrittenInABuildRunGoesIntoTheDescriptionToo(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: providersSecrets(), Machine: testsupport.Machine, At: testsupport.Day})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Make the rules v2."})
	r := workFake(map[string]runner.Handler{"build": workBuilder(map[string]string{"src/game.txt": "rules v2\n"}),
		"review": workReviewer(workApprove)})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider"), result.Get("status")},
		[]any{"build", "muse", "approved"})
	record := h.ctx.Store.Load().Obj("items").Obj("12")
	assertEqual(t, record.Get("planned_tier"), "medium")
	assertTrue(t, issueplan.PlanOf(wasteStr(h.gh.Threads.Item(12), "body")) != "")
	assertTrue(t, !h.gh.LabelNames(12)[config.LabelNeedsPlan])
}

// ---------------------------------------------------------------------------- PersonsPlanTests
//
// A Plan section someone puts in the description (`issueplan.START` … `END`) is the plan: the bot
// neither plans over it nor holds the item in the Needs plan stage.

// needsPlanBody is PersonsPlanTests.body(task="Make the Almanac load faster.").
func needsPlanBody() string {
	return "Make the Almanac load faster.\n\n" + issueplan.Start + "\n## Plan\n\n1. In `apps/web/src/cards/art/Art.tsx`, " +
		"delay the draw.\n" + issueplan.End + "\n"
}

func TestNeedsPlan_ItCountsAsAStrongPlanAndTheWeakOneBuildsFromIt(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh, needsPlanLane{Machine: []string{"lite"}})
	difficultyBusy(gh, ctx, needsPlanFour()...) // no strong model is free
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild, difficultyEasy}, Body: needsPlanBody()})
	planned := flowMake(ctx)
	needsPlanIs(t, planned, []string{"action", "number", "provider"}, "build", 3, "lite")
	assertIsNone(t, difficultySeatAt(t, planned, "plan")) // not planned again
	assertTrue(t, !gh.LabelNames(3)[config.LabelNeedsPlan])
	assertEqual(t, planned.Obj("handoff").Get("notes"), "1. In `apps/web/src/cards/art/Art.tsx`, delay the draw.")
	assertEqual(t, planned.Obj("handoff").Get("provider"), "the issue's description")
	assertIn(t, issueplan.Start, wasteStr(gh.Threads.Item(3), "body")) // the bot left it as it was
}

func TestNeedsPlan_WithoutTheMarkersItIsOnlyTextAndABotsMediumPlanStillCountsAsMedium(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh)
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "## Plan\n\nSomething loose."})
	planned := flowMake(ctx)
	needsPlanIs(t, planned, []string{"action", "number"}, "plan", 3)
	// The bot's own record wins over the description: a medium item a medium model planned builds
	// from that plan (#695), on Sonnet with its run's own Opus reviewer.
	gh = fakes.NewFakeGitHub()
	ctx = needsPlanLaneCtx(gh, needsPlanLane{Machine: []string{"lite"}})
	difficultyBusy(gh, ctx, needsPlanFour()...)
	gh.AddIssue(4, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:medium"}, Body: needsPlanBody()})
	flowUpdateItem(ctx, 4, pyjson.New("planned_at", clock.ISO(testsupport.Night), "planned_tier", "medium"))
	planned = flowMake(ctx)
	needsPlanIs(t, planned, []string{"action", "number"}, "build", 4)
	// claude-3 holds a run on one of its two lanes, so the load balancer gives this one to
	// claude-7, the first idle account in the usage order (`plan.balance`).
	assertEqual(t, difficultySeatAt(t, planned, "build"), difficultySeat{"claude-7", "sonnet", "medium"})
	assertEqual(t, difficultySeatAt(t, planned, "review"), difficultySeat{"claude-7", "opus", "strong"})
	assertTrue(t, !gh.LabelNames(4)[config.LabelNeedsPlan])
}
