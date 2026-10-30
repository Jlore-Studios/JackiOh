// Muse fills gpt's and agy's lanes on the night box while they cannot start work (`fill_in` in
// providers.json): Muse's own four lanes are one on the night box and three on the Muse box, and
// each box runs three jobs at most, so Muse runs six at most. (bot/tests/test_fill_in.py)
package tests

import (
	"fmt"
	"strconv"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/controller"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/memory"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/waiting"
)

var fillInFill = "night-vm-muse-fill"

// fillInCtxFor is ctx_for(gh, machine=MACHINE): by day, every secret set.
func fillInCtxFor(gh *fakes.FakeGitHub, machine ...string) *hctx.Context {
	if machine == nil {
		machine = testsupport.Machine
	}
	return testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, Cfg: testsupport.MakeConfig(
		testsupport.Config{Env: map[string]string{"HARNESS_SECRETS_SET": strings.Join(providers.SecretNames, " ")},
			Machine: machine})})
}

// fillInRefused: `providerID` refused a call a minute ago and waits two hours: it cannot start
// work.
func fillInRefused(ctx *hctx.Context, providerID string) {
	ctx.Store.Update(func(s *pyjson.Object) any {
		providers.NoteUsage(s, providerID, nil, "+PT2H", testsupport.Day, 0.0, nil, nil)
		return nil
	}, "state")
}

// fillInTaken is one claim take_all made: (number, provider, runs_on).
type fillInTaken struct {
	Number   int
	Provider string
	RunsOn   string
}

// fillInTakeAll plans until nothing more starts; each claim's run stays alive. (number, provider,
// runs_on) for each, and the last plan's reason.
func fillInTakeAll(gh *fakes.FakeGitHub, ctx *hctx.Context) ([]fillInTaken, string) {
	taken, planned := []fillInTaken{}, pyjson.New()
	for range 10 {
		planned = plan.Make(ctx, false, nil, "auto", nil)
		if planned.Get("action") == "none" {
			break
		}
		number, _ := pyjson.AsInt(planned.Get("number"))
		gh.Runs.Set(strconv.Itoa(number), pyjson.New("status", "in_progress"))
		ctx.Store.Update(func(s *pyjson.Object) any {
			state.Item(s, number).Update(pyjson.New("run_id", strconv.Itoa(number)))
			return nil
		}, "state")
		taken = append(taken, fillInTaken{number, pyjson.Str(planned.Get("provider")), pyjson.Str(planned.Get("runs_on"))})
	}
	return taken, pyjson.Str(planned.GetOr("reason", ""))
}

// fillInSetUp is FillInTests.setUp: nine queued items.
func fillInSetUp() *fakes.FakeGitHub {
	gh := fakes.NewFakeGitHub()
	for n := 3; n < 12; n++ {
		gh.AddIssue(n, fakes.Issue{Labels: []string{config.LabelBuild}})
	}
	return gh
}

// ---------------------------------------------------------------------------- FillInTests

func TestFillIn_WithGptAndAgyFreeEachKeepsItsLane(t *testing.T) {
	gh := fillInSetUp()
	ctx := fillInCtxFor(gh)
	taken, reason := fillInTakeAll(gh, ctx)
	assertEqual(t, taken, []fillInTaken{{3, "muse", "night-vm-muse"}, {4, "muse", "night-vm-muse"},
		{5, "muse", "night-vm-muse"}, {6, "muse", "night-vm-muse"},
		{7, "agy", "night-vm-agy"}, {8, "gpt", "night-vm-gpt"}})
	assertIn(t, "`muse` is busy", reason)
}

func TestFillIn_MuseFillsTheLaneOfOneThatCannotStart(t *testing.T) {
	gh := fillInSetUp()
	// Room for every lane on the machine (`machine_parallel`, six, is its own test's), so what
	// holds gpt back is its lane.
	ctx := fillInCtxFor(gh)
	ctx = testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, Cfg: testsupport.WithLanes(ctx.Cfg, 8, 20)})
	fillInRefused(ctx, "gpt")
	taken, _ := fillInTakeAll(gh, ctx)
	// Muse, first in the tier order, takes its own four lanes, then gpt's lane before agy takes
	// its own.
	assertEqual(t, taken, []fillInTaken{{3, "muse", "night-vm-muse"}, {4, "muse", "night-vm-muse"},
		{5, "muse", "night-vm-muse"}, {6, "muse", "night-vm-muse"},
		{7, "muse", fillInFill}, {8, "agy", "night-vm-agy"}})
	s := ctx.Store.Load()
	assertEqual(t, s.Obj("items").Obj("7").Get("runs_on"), fillInFill)
	lanes := plan.ReadLanes(ctx, s)
	assertEqual(t, []any{lanes.Filling, lanes.Count("muse")}, []any{map[int]bool{7: true}, 5})
	assertTrue(t, lanes.Full(ctx.Cfg.Pool.Get("muse")))
	// gpt's reset comes while Muse holds its lane: gpt waits for it, so the night box never runs
	// more than three jobs (Muse's own runner there, and the two lanes).
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.Obj("providers").Obj("gpt").Pop("refused_until")
		return nil
	}, "state")
	again := plan.Make(ctx, false, nil, "auto", nil)
	assertEqual(t, again.Get("action"), "none")
	assertIn(t, "`gpt` waits for its lane on its box, which `muse` fills now", pyjson.Str(again.Get("reason")))
	assertTrue(t, plan.FillInFull(ctx.Cfg.Pool, ctx.Cfg.Pool.Get("gpt"), plan.ReadLanes(ctx, ctx.Store.Load())))
	live := pyjson.New()
	for _, c := range taken {
		live.Set(strconv.Itoa(c.Number), c.Provider)
	}
	explained := waiting.Explain(ctx, ctx.Store.Load(), live, nil, "")
	gpt := explained.Now.Get("gpt").(*waiting.Now)
	assertEqual(t, gpt.Kind, "full")
	assertIn(t, "`muse` fills it", gpt.Text)
	// Once Muse's run there ends, the lane is gpt's again.
	gh.Runs.Set("7", pyjson.New("status", "completed"))
	planned := plan.Make(ctx, false, nil, "auto", nil)
	assertEqual(t, []any{planned.Get("provider"), planned.Get("runs_on")}, []any{"gpt", "night-vm-gpt"})
}

func TestFillIn_WithBothOffMuseTakesBothLanesAndRunsSixAtMost(t *testing.T) {
	gh := fillInSetUp()
	ctx := fillInCtxFor(gh, "muse")
	taken, reason := fillInTakeAll(gh, ctx)
	runners := []string{}
	for _, c := range taken {
		runners = append(runners, c.RunsOn)
	}
	assertEqual(t, runners, []string{"night-vm-muse", "night-vm-muse", "night-vm-muse", "night-vm-muse",
		fillInFill, fillInFill})
	assertIn(t, "`muse` is busy", reason)
	assertEqual(t, ctx.Cfg.Pool.MachineParallel, 6)
}

func TestFillIn_APlanningRunOfGptOrAgyHoldsASharedLane(t *testing.T) {
	gh := fillInSetUp()
	ctx := fillInCtxFor(gh)
	pool := ctx.Cfg.Pool
	lanes := &plan.Lanes{Limit: 8, MachineLimit: 6, Machine: setOf(testsupport.Machine...)}
	lanes = lanes.WithRun(40, "gpt", "plan", false).WithRun(41, "muse", "build", true)
	assertTrue(t, plan.FillInFull(pool, pool.Get("agy"), lanes))
	assertFalse(t, plan.FillInFull(pool, pool.Get("muse"), lanes))
	assertEqual(t, []int{lanes.FillingBy("muse"), lanes.AllBy("gpt")}, []int{1, 1})
	// Muse's fill-in takes none of its own lanes.
	assertFalse(t, lanes.Full(pool.Get("muse")))
}

func TestFillIn_TheSurveyNeverFillsALane(t *testing.T) {
	gh := fillInSetUp()
	ctx := fillInCtxFor(gh, "muse")
	lanes := &plan.Lanes{Limit: 8, Held: map[int]string{40: "muse", 41: "muse", 42: "muse", 43: "muse"},
		MachineLimit: 6, Machine: setOf(testsupport.Machine...)}
	s := ctx.Store.Load()
	muse := ctx.Cfg.Pool.Get("muse")
	assertEqual(t, plan.RunnerFor(ctx, s, muse, lanes, "build"), fillInFill)
	assertEqual(t, plan.RunnerFor(ctx, s, muse, lanes, "suggest"), "")
}

// ---------------------------------------------------------------------------- CheckTests

func TestFillIn_ADispatchMayNameTheFillInRunnersOfItsOwnSubscriptionOnly(t *testing.T) {
	ctx := fillInCtxFor(fakes.NewFakeGitHub())
	given := func(changes map[string]string) map[string]string {
		out := map[string]string{"cli": "muse", "secret": "", "shared": "false"}
		for k, v := range changes {
			out[k] = v
		}
		return out
	}
	planned := pyjson.New("provider", "muse", "runs_on", fillInFill, "cli", "muse")
	assertEqual(t, controller.Check(ctx, planned, given(map[string]string{"runs_on": fillInFill})), []string{})
	assertTrue(t, len(controller.Check(ctx, planned, given(map[string]string{"runs_on": "night-vm-muse"}))) > 0)
	gpt := pyjson.New("provider", "gpt", "runs_on", fillInFill, "cli", "codex")
	assertTrue(t, len(controller.Check(ctx, gpt, given(map[string]string{"cli": "codex", "runs_on": fillInFill}))) > 0)
}

// ---------------------------------------------------------------------------- ParseTests

func fillInRaw(fill *pyjson.Object) *pyjson.Object {
	raw := testsupport.RawProviders()
	raw.Obj("providers").Obj("muse").Set("fill_in", fill)
	return raw
}

func TestFillIn_TheCommittedFillIn(t *testing.T) {
	pool := providers.Load(testsupport.Root)
	muse := pool.Get("muse")
	assertEqual(t, []any{muse.RunsOn, muse.Lanes, *muse.FillIn},
		[]any{"night-vm-muse", 4, providers.FillIn{RunsOn: fillInFill, Of: []string{"gpt", "agy"}}})
	assertEqual(t, []string{pool.FillerOf("gpt").ID, pool.FillerOf("agy").ID}, []string{"muse", "muse"})
	assertTrue(t, pool.FillerOf("lite") == nil)
}

func TestFillIn_ItRefusesABadFillIn(t *testing.T) {
	for _, c := range []struct {
		fill    *pyjson.Object
		message string
	}{
		{pyjson.New("runs_on", fillInFill), "needs runs_on and for"},
		{pyjson.New("runs_on", fillInFill, "for", []any{"nobody"}), "not another subscription"},
		{pyjson.New("runs_on", fillInFill, "for", []any{"muse"}), "not another subscription"},
		{pyjson.New("runs_on", "night-vm-gpt", "for", []any{"gpt"}), "runners of its own"},
		{pyjson.New("runs_on", "ubuntu-latest", "for", []any{"gpt"}), "on the machine"},
		{pyjson.New("runs_on", fillInFill, "for", []any{"gpt"}, "lanes", 2), "unknown keys"},
	} {
		caught := assertRaises[*herr.ConfigError](t, func() { providers.Parse(fillInRaw(c.fill)) },
			pyjson.Dumps(c.fill))
		assertIn(t, c.message, caught.Error(), fmt.Sprint(pyjson.Dumps(c.fill)))
	}
}

func TestFillIn_OneMovedToGithubsRunnersHasNoLaneToLend(t *testing.T) {
	raw := testsupport.RawProviders()
	raw.Obj("providers").Obj("gpt").Update(pyjson.New("login", "secret", "secret", "CODEX_AUTH_JSON",
		"runs_on", "ubuntu-latest"))
	pool := providers.Parse(raw)
	assertEqual(t, pool.FilledBy(pool.Get("muse")), []string{"agy"})
	assertTrue(t, pool.FillerOf("gpt") == nil)
}

// ---------------------------------------------------------------------------- BoxTests

func TestFillIn_AFillInRunnerIsOnTheNightBox(t *testing.T) {
	assertEqual(t, memory.BoxOf("night-vm-muse-fill-2", fillInFill), "")
	assertEqual(t, memory.BoxOf("night-vm-muse-2-b2", "night-vm-muse"), "b2")
}
