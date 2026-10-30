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
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// ---------------------------------------------------------------------------- borrowed helpers
//
// What test_needs_plan.py imports from test_difficulty.py (EASY, busy, queue, seats) and
// test_providers.py (secrets), copied here under this file's prefix until difficulty_test.go and
// providers_test.go hold theirs. Harness, builder, reviewer and APPROVE are waste_test.go's.

// needsPlanEasy is test_difficulty.EASY.
const needsPlanEasy = "difficulty:easy"

// needsPlanSecrets is test_providers.secrets(*names).
func needsPlanSecrets(names ...string) map[string]string {
	return map[string]string{"HARNESS_SECRETS_SET": strings.Join(names, " ")}
}

// needsPlanQueue is test_difficulty.queue(gh, ctx, number, *labels, planned=planned).
func needsPlanQueue(gh *fakes.FakeGitHub, ctx *hctx.Context, number int, planned bool, labels ...string) {
	gh.AddIssue(number, fakes.Issue{Labels: append([]string{config.LabelBuild}, labels...)})
	if planned {
		ctx.Store.Update(func(s *pyjson.Object) any {
			state.Item(s, number).Set("planned_at", clock.ISO(testsupport.Day))
			return nil
		}, "state")
	}
}

// needsPlanHeld is one (provider, item) of test_difficulty.busy.
type needsPlanHeld struct {
	provider string
	number   int
}

// needsPlanBusy is test_difficulty.busy(gh, ctx, *providers_and_items): give each provider a run
// in progress on a working item, so it holds a lane.
func needsPlanBusy(gh *fakes.FakeGitHub, ctx *hctx.Context, held ...needsPlanHeld) {
	for _, h := range held {
		gh.AddIssue(h.number, fakes.Issue{Labels: []string{"bot:working"}})
		gh.Runs.Set(strconv.Itoa(h.number), pyjson.New("status", "in_progress"))
		ctx.Store.Update(func(s *pyjson.Object) any {
			state.Item(s, h.number).Set("run_id", strconv.Itoa(h.number)).Set("provider", h.provider)
			return nil
		}, "state")
	}
}

// needsPlanSeat is test_difficulty.seats(planned)[role]: the seat's (provider, model, tier), or
// nil for None. A role the plan has no seat for is a KeyError.
func needsPlanSeat(t *testing.T, planned *pyjson.Object, role string) *[3]string {
	t.Helper()
	seat, ok := planned.Obj("seats").Lookup(role)
	if !ok {
		t.Fatalf("KeyError: %q", role)
	}
	if !pyjson.Truthy(seat) {
		return nil
	}
	s := pyjson.Obj(seat)
	return &[3]string{py.Str(s.Get("provider")), py.Str(s.Get("model")), py.Str(s.Get("tier"))}
}

// needsPlanSeatIs is assertEqual(seats(planned)[role], want).
func needsPlanSeatIs(t *testing.T, planned *pyjson.Object, role string, want [3]string) {
	t.Helper()
	got := needsPlanSeat(t, planned, role)
	if got == nil || *got != want {
		t.Fatalf("%v != %v", got, want)
	}
}

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
		env = needsPlanSecrets(providers.SecretNames...)
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

// needsPlanMake is plan_mod.make(ctx).
func needsPlanMake(ctx *hctx.Context) *pyjson.Object { return plan.Make(ctx, false, nil, "auto", nil) }

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

// needsPlanSetItem is `ctx.store.update(lambda s: state_item(s, number).update(**fields))`.
func needsPlanSetItem(ctx *hctx.Context, number int, fields *pyjson.Object) {
	ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, number).Update(fields)
		return nil
	}, "state")
}

// needsPlanFour is the four Claude accounts busy() holds in several tests: claude-3, claude-1,
// claude-4 and claude-2 on #50 to #53, so no strong model is free.
func needsPlanFour() []needsPlanHeld {
	held := []needsPlanHeld{}
	for i, p := range []string{"claude-3", "claude-1", "claude-4", "claude-2"} {
		held = append(held, needsPlanHeld{p, 50 + i})
	}
	return held
}

// ---------------------------------------------------------------------------- IssuePlanTests

func TestNeedsPlan_ThePlanSectionGoesInComesOutAndIsReplaced(t *testing.T) {
	body := issueplan.WithPlan("Make the rules v2.", "1. Edit `src/a.ts`.", "`claude-3`", "run 7")
	workflowsTrue(t, strings.HasPrefix(body, "Make the rules v2.\n\n"+issueplan.Start+"\n## Plan\n"))
	workflowsIn(t, "_Written by `claude-3` (run 7)", body)
	workflowsNotIn(t, config.Marker, body) // the description must not read as the bot's own
	workflowsEqual(t, issueplan.PlanOf(body), "1. Edit `src/a.ts`.")
	workflowsEqual(t, issueplan.WithoutPlan(body), "Make the rules v2.")
	again := issueplan.WithPlan(body, "1. Edit `src/b.ts`.", "`claude-1`", "")
	workflowsEqual(t, strings.Count(again, issueplan.Start), 1)
	workflowsEqual(t, issueplan.PlanOf(again), "1. Edit `src/b.ts`.")
	// A person's edit is the plan; text they add after the section stays theirs.
	edited := strings.ReplaceAll(again, "src/b.ts", "src/c.ts") + "\nAlso: thanks."
	workflowsEqual(t, issueplan.PlanOf(edited), "1. Edit `src/c.ts`.")
	workflowsEqual(t, issueplan.WithoutPlan(edited), "Make the rules v2.\n\nAlso: thanks.")
	workflowsEqual(t, issueplan.PlanOf("No plan here."), "")
	workflowsEqual(t, issueplan.PlanOf(""), "") // plan_of(None): the Go body is "" for None
}

func TestNeedsPlan_ALongPlanGivesWayToTheTask(t *testing.T) {
	task := strings.Repeat("t", 30_000)
	body := issueplan.WithPlan(task, strings.Repeat("p", 60_000), "`claude-3`", "")
	workflowsLessEqual(t, py.Len(body), issueplan.BodyLimit)
	workflowsTrue(t, strings.HasPrefix(body, task))
	workflowsIn(t, "cut to fit the description", body)
}

// ---------------------------------------------------------------------------- StageTests

func TestNeedsPlan_QueuedItemsWithoutAStrongPlanCarryTheLabel(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh)
	needsPlanBusy(gh, ctx, needsPlanFour()...)      // nobody plans yet
	needsPlanQueue(gh, ctx, 3, false)               // no plan
	needsPlanQueue(gh, ctx, 4, true, needsPlanEasy) // an easy item with a strong plan
	needsPlanSetItem(ctx, 4, pyjson.New("planned_tier", "strong"))
	needsPlanQueue(gh, ctx, 5, true, needsPlanEasy) // an easy item with a medium plan
	needsPlanSetItem(ctx, 5, pyjson.New("planned_tier", "medium"))
	needsPlanQueue(gh, ctx, 6, true) // a medium item with a medium plan
	needsPlanSetItem(ctx, 6, pyjson.New("planned_tier", "medium"))
	gh.AddIssue(7, fakes.Issue{Labels: []string{config.LabelNeedsPlan}}) // not queued any more
	needsPlanMake(ctx)
	labelled := map[int]bool{}
	for _, n := range []int{3, 4, 5, 6, 7} {
		if gh.LabelNames(n)[config.LabelNeedsPlan] {
			labelled[n] = true
		}
	}
	// A medium plan is enough for an easy or medium item; only a hard one needs a strong model's
	// (#317 part 6, #695).
	workflowsEqualMaps(t, labelled, map[int]bool{3: true})
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
	ctx := needsPlanLaneCtx(gh, needsPlanLane{Env: needsPlanSecrets(names...)})
	needsPlanBusy(gh, ctx, needsPlanHeld{"claude-3", 50}, needsPlanHeld{"claude-4", 52},
		needsPlanHeld{"agy", 53}, needsPlanHeld{"muse", 54}, needsPlanHeld{"gpt", 55})
	needsPlanQueue(gh, ctx, 3, false, "priority:high")               // medium and urgent
	needsPlanQueue(gh, ctx, 4, false, needsPlanEasy, "priority:low") // easy: the weak one's, once planned
	planned := needsPlanMake(ctx)
	needsPlanIs(t, planned, []string{"action", "number", "provider"}, "plan", 4, "claude-1") // claude-1 holds nothing else
	needsPlanSeatIs(t, planned, "plan", [3]string{"claude-1", "opus", "strong"})
	workflowsEqualMaps(t, gh.LabelNames(4), workflowsSet(config.LabelWorking, config.LabelNeedsPlan, needsPlanEasy,
		"priority:low"))
	needsPlanRunning(gh, ctx, planned)
	planned = needsPlanMake(ctx)
	needsPlanIs(t, planned, []string{"action", "number", "provider"}, "plan", 3, "claude-3") // building #50 does not stop it planning
	needsPlanRunning(gh, ctx, planned)
	// Both planning lanes are taken, and claude-1, planning, does not build too.
	action := py.Str(needsPlanMake(ctx).Get("action"))
	workflowsTrue(t, action != "plan" && action != "build", action)
	needsPlanPlannedBy(ctx, 4)
	gh.Runs.Delete("4")
	labels := []any{}
	for _, n := range []string{config.LabelBuild, needsPlanEasy, "priority:low"} {
		labels = append(labels, pyjson.New("name", n))
	}
	gh.Threads.Item(4).Set("labels", labels)
	planned = needsPlanMake(ctx)
	needsPlanIs(t, planned, []string{"action", "number", "provider"}, "build", 4, "lite")
	workflowsTrue(t, needsPlanSeat(t, planned, "plan") == nil) // built from the plan, not planned again
}

// Planning comes first: claude-3 (two lanes, no caps) plans two items at once, before it builds
// anything, so the medium models have plans to build from. Its third item it plans and builds in
// one run, on a build lane.
func TestNeedsPlan_AnUncappedAccountPlansAsManyItemsAtOnceAsItsLanes(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh, needsPlanLane{Machine: []string{}, Env: needsPlanSecrets("CLAUDE_CODE_OAUTH_TOKEN_3")})
	for _, number := range []int{3, 4, 5} {
		needsPlanQueue(gh, ctx, number, false)
	}
	made := []*pyjson.Object{}
	for range 2 {
		made = append(made, needsPlanMake(ctx))
		needsPlanRunning(gh, ctx, made[len(made)-1])
	}
	needsPlanIs(t, made[0], []string{"action", "number", "provider"}, "plan", 3, "claude-3")
	needsPlanIs(t, made[1], []string{"action", "number", "provider"}, "plan", 4, "claude-3")
	third := needsPlanMake(ctx)
	needsPlanIs(t, third, []string{"action", "number"}, "build", 5)
	needsPlanSeatIs(t, third, "plan", [3]string{"claude-3", "opus", "strong"})
}

// A fullsend part waits on its plan for a medium subscription (Muse) to build it, so the lane
// plans it before an older, harder item that is no part.
func TestNeedsPlan_TheLanePlansFullsendPartsBeforeTheRest(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh, needsPlanLane{Machine: []string{}, Env: needsPlanSecrets("CLAUDE_CODE_OAUTH_TOKEN_3")})
	needsPlanQueue(gh, ctx, 3, false, "difficulty:hard")
	needsPlanQueue(gh, ctx, 4, false)
	needsPlanSetItem(ctx, 4, pyjson.New("onto", "bot/issue-9", "part_of", 9))
	planned := needsPlanMake(ctx)
	needsPlanIs(t, planned, []string{"action", "number"}, "plan", 4)
}

// One lane (#695): while it holds a run it takes nothing more, neither a second build nor a plan
// — two runs deciding from one reading go past a cap together. Once the run ends, it plans.
func TestNeedsPlan_Claude1NeverPlansWhileItBuilds(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh, needsPlanLane{Machine: []string{}, Env: needsPlanSecrets("CLAUDE_CODE_OAUTH_TOKEN")})
	needsPlanBusy(gh, ctx, needsPlanHeld{"claude-1", 50})
	needsPlanQueue(gh, ctx, 3, false)
	wasteEq(t, needsPlanMake(ctx).Get("action"), "none")
	gh.Runs.Delete("50")
	wasteEq(t, needsPlanMake(ctx).Get("action"), "plan")
}

// claude-2 (capped) has one lane (#695): while it holds a run it takes nothing more, neither a
// second build nor a plan. Once the run ends, it plans; the mid-run refusal checks still guard the
// spending.
func TestNeedsPlan_ACappedSubscriptionTakesNothingWhileHolding(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh, needsPlanLane{Machine: []string{}, Env: needsPlanSecrets("CLAUDE_CODE_OAUTH_TOKEN_2")})
	needsPlanBusy(gh, ctx, needsPlanHeld{"claude-2", 50})
	needsPlanQueue(gh, ctx, 3, false)
	wasteEq(t, needsPlanMake(ctx).Get("action"), "none")
	gh.Runs.Delete("50")
	planned := needsPlanMake(ctx)
	needsPlanIs(t, planned, []string{"action", "provider"}, "plan", "claude-2")
	needsPlanRunning(gh, ctx, planned)
	needsPlanQueue(gh, ctx, 4, true)
	wasteEq(t, needsPlanMake(ctx).Get("action"), "none") // the lane is held again
}

// At 87% of its 5-hour window claude-2 (cap 90%, long runs start under 85%) starts neither a
// build nor a plan: #37's plan on claude-4 started just under its 70% cap and was cut off four
// minutes in. Under 85% it plans.
func TestNeedsPlan_HeadroomHoldsBackPlansAsItDoesBuilds(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh, needsPlanLane{Machine: []string{}, Env: needsPlanSecrets("CLAUDE_CODE_OAUTH_TOKEN_2")})
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
	needsPlanQueue(gh, ctx, 3, true) // planned: a build
	wasteEq(t, needsPlanMake(ctx).Get("action"), "none")
	needsPlanQueue(gh, ctx, 4, false) // not planned: a plan
	wasteEq(t, needsPlanMake(ctx).Get("action"), "none")
	reading(0.8)
	planned := needsPlanMake(ctx)
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
	needsPlanBusy(gh, ctx, needsPlanHeld{"agy", 53}, needsPlanHeld{"muse", 54}, needsPlanHeld{"claude-4", 55},
		needsPlanHeld{"claude-5", 56})
	needsPlanQueue(gh, ctx, 3, false)
	wasteEq(t, needsPlanMake(ctx).Get("action"), "none")
	// A slot free on the machine: a planner there plans, though GitHub's lanes are full.
	gh = fakes.NewFakeGitHub()
	ctx = needsPlanLaneCtx(gh, needsPlanLane{MaxParallel: 2})
	needsPlanBusy(gh, ctx, needsPlanHeld{"agy", 53}, needsPlanHeld{"claude-4", 55}, needsPlanHeld{"claude-5", 56})
	needsPlanQueue(gh, ctx, 3, false)
	planned := needsPlanMake(ctx)
	wasteEq(t, planned.Get("action"), "plan")
	workflowsTrue(t, ctx.Cfg.Pool.OnMachine(py.Str(planned.Get("provider"))), planned.Get("provider"))
	// A lane free on GitHub's runners: the strong planner there goes first.
	gh = fakes.NewFakeGitHub()
	ctx = needsPlanLaneCtx(gh, needsPlanLane{MaxParallel: 2})
	needsPlanBusy(gh, ctx, needsPlanHeld{"agy", 53}, needsPlanHeld{"muse", 54}, needsPlanHeld{"claude-4", 55})
	needsPlanQueue(gh, ctx, 3, false)
	planned = needsPlanMake(ctx)
	needsPlanIs(t, planned, []string{"action", "provider"}, "plan", "claude-3")
	// With the planning lane off, every lane is busy.
	gh = fakes.NewFakeGitHub()
	ctx = testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night, Cfg: testsupport.WithLanes(
		testsupport.MakeConfig(testsupport.Config{Env: needsPlanSecrets(providers.SecretNames...),
			Machine: testsupport.Machine}), 2)})
	needsPlanBusy(gh, ctx, needsPlanHeld{"agy", 53}, needsPlanHeld{"muse", 54}, needsPlanHeld{"claude-4", 55},
		needsPlanHeld{"claude-5", 56})
	needsPlanQueue(gh, ctx, 3, false)
	workflowsIn(t, "every lane is busy", py.Str(needsPlanMake(ctx).Get("reason")))
}

// By day (the Claude accounts keep to the night here) no strong model is free: an item nobody has
// rated or planned may still be rated and planned on the lane by a medium model (#317 part 8),
// Muse first; a rated medium item takes a medium planner too (#695).
func TestNeedsPlan_AMediumModelRatesAndPlansAnUnratedItemOnTheLane(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh, needsPlanLane{At: testsupport.Day, Machine: testsupport.AllMachine})
	needsPlanQueue(gh, ctx, 3, false)
	planned := needsPlanMake(ctx)
	needsPlanIs(t, planned, []string{"action", "provider"}, "plan", "muse")
	needsPlanSeatIs(t, planned, "plan", [3]string{"muse", "muse-spark-1.3-contributor", "medium"})
	gh = fakes.NewFakeGitHub()
	ctx = needsPlanLaneCtx(gh, needsPlanLane{At: testsupport.Day, Machine: testsupport.AllMachine})
	needsPlanQueue(gh, ctx, 3, false, "difficulty:medium")
	planned = needsPlanMake(ctx)
	needsPlanIs(t, planned, []string{"action", "provider"}, "plan", "muse")
	needsPlanSeatIs(t, planned, "plan", [3]string{"muse", "muse-spark-1.3-contributor", "medium"})
	// An easy item nobody planned is never the weak one's.
	gh = fakes.NewFakeGitHub()
	ctx = needsPlanLaneCtx(gh, needsPlanLane{At: testsupport.Day, Machine: []string{"lite"}})
	needsPlanQueue(gh, ctx, 3, false, needsPlanEasy)
	wasteEq(t, needsPlanMake(ctx).Get("action"), "none")
	workflowsTrue(t, gh.LabelNames(3)[config.LabelNeedsPlan])
}

func TestNeedsPlan_APlanWrittenInABuildRunGoesIntoTheDescriptionToo(t *testing.T) {
	h := newWasteHarness(t, wasteHarnessOpts{Env: needsPlanSecrets(), Machine: testsupport.Machine, At: testsupport.Day})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Make the rules v2."})
	r := wasteRunner(map[string]runner.Handler{"build": wasteBuilder(map[string]string{"src/game.txt": "rules v2\n"}),
		"review": wasteReviewer(wasteApprove)})
	planned, result := h.night(r)
	wasteEq(t, []any{planned.Get("action"), planned.Get("provider"), result.Get("status")},
		[]any{"build", "muse", "approved"})
	record := h.ctx.Store.Load().Obj("items").Obj("12")
	wasteEq(t, record.Get("planned_tier"), "medium")
	workflowsTrue(t, issueplan.PlanOf(wasteStr(h.gh.Threads.Item(12), "body")) != "")
	workflowsTrue(t, !h.gh.LabelNames(12)[config.LabelNeedsPlan])
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
	needsPlanBusy(gh, ctx, needsPlanFour()...) // no strong model is free
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild, needsPlanEasy}, Body: needsPlanBody()})
	planned := needsPlanMake(ctx)
	needsPlanIs(t, planned, []string{"action", "number", "provider"}, "build", 3, "lite")
	workflowsTrue(t, needsPlanSeat(t, planned, "plan") == nil) // not planned again
	workflowsTrue(t, !gh.LabelNames(3)[config.LabelNeedsPlan])
	wasteEq(t, planned.Obj("handoff").Get("notes"), "1. In `apps/web/src/cards/art/Art.tsx`, delay the draw.")
	wasteEq(t, planned.Obj("handoff").Get("provider"), "the issue's description")
	workflowsIn(t, issueplan.Start, wasteStr(gh.Threads.Item(3), "body")) // the bot left it as it was
}

func TestNeedsPlan_WithoutTheMarkersItIsOnlyTextAndABotsMediumPlanStillCountsAsMedium(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := needsPlanLaneCtx(gh)
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "## Plan\n\nSomething loose."})
	planned := needsPlanMake(ctx)
	needsPlanIs(t, planned, []string{"action", "number"}, "plan", 3)
	// The bot's own record wins over the description: a medium item a medium model planned builds
	// from that plan (#695), on Sonnet with its run's own Opus reviewer.
	gh = fakes.NewFakeGitHub()
	ctx = needsPlanLaneCtx(gh, needsPlanLane{Machine: []string{"lite"}})
	needsPlanBusy(gh, ctx, needsPlanFour()...)
	gh.AddIssue(4, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:medium"}, Body: needsPlanBody()})
	needsPlanSetItem(ctx, 4, pyjson.New("planned_at", clock.ISO(testsupport.Night), "planned_tier", "medium"))
	planned = needsPlanMake(ctx)
	needsPlanIs(t, planned, []string{"action", "number"}, "build", 4)
	// claude-3 holds a run on one of its two lanes, so the load balancer gives this one to
	// claude-7, the first idle account in the usage order (`plan.balance`).
	needsPlanSeatIs(t, planned, "build", [3]string{"claude-7", "sonnet", "medium"})
	needsPlanSeatIs(t, planned, "review", [3]string{"claude-7", "opus", "strong"})
	workflowsTrue(t, !gh.LabelNames(4)[config.LabelNeedsPlan])
}
