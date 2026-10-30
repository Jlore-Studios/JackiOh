// The difficulty system: every model a tier, every item a difficulty, and the difficulty deciding
// who may plan, build and review it, in the owner's usage order; and a self-checking builder's
// loop. (bot/tests/test_difficulty.py)
//
// Its EASY, MEDIUM and HARD, queue, busy and seats are difficultyEasy, difficultyMedium,
// difficultyHard, difficultyQueue, difficultyBusy and difficultySeats, which other files import.
package tests

import (
	"fmt"
	"os"
	"path/filepath"
	"slices"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cli"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/reviewrule"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// ---------------------------------------------------------------------------- this file's helpers

// difficultyOne is a role's single handler, as FakeRunner takes it.
func difficultyOne(h runner.Handler) []runner.Handler { return []runner.Handler{h} }

var (
	difficultyEasy   = "difficulty:easy"
	difficultyMedium = "difficulty:medium"
	difficultyHard   = "difficulty:hard"
)

var difficultyClaudeSecrets = map[string]string{"claude-1": "CLAUDE_CODE_OAUTH_TOKEN",
	"claude-2": "CLAUDE_CODE_OAUTH_TOKEN_2", "claude-3": "CLAUDE_CODE_OAUTH_TOKEN_3",
	"claude-4": "CLAUDE_CODE_OAUTH_TOKEN_4", "claude-5": "CLAUDE_CODE_OAUTH_TOKEN_5",
	"claude-6": "CLAUDE_CODE_OAUTH_TOKEN_6", "claude-7": "CLAUDE_CODE_OAUTH_TOKEN_7",
	"claude-8": "CLAUDE_CODE_OAUTH_TOKEN_8"}

// difficultyCtxOpts is ctx_for's keyword arguments; a zero field is the default.
type difficultyCtxOpts struct {
	At  time.Time         // at = NIGHT
	Env map[string]string // env = None: every secret
	// Machine is machine = MACHINE when nil; an empty slice is ().
	Machine        []string
	CommittedHours bool
}

func difficultyCtxFor(gh *fakes.FakeGitHub, opts ...difficultyCtxOpts) *hctx.Context {
	var o difficultyCtxOpts
	if len(opts) > 0 {
		o = opts[0]
	}
	env := o.Env
	if env == nil {
		env = providersSecrets(providers.SecretNames...)
	}
	machine := o.Machine
	if machine == nil {
		machine = testsupport.Machine
	}
	return testsupport.MakeCtx(gh, testsupport.Ctx{At: o.At, Cfg: testsupport.MakeConfig(
		testsupport.Config{Env: env, Machine: machine, CommittedHours: o.CommittedHours})})
}

// difficultyQueue is queue(gh, ctx, number, *labels, planned=...).
func difficultyQueue(gh *fakes.FakeGitHub, ctx *hctx.Context, number int, planned bool, labels ...string) {
	gh.AddIssue(number, fakes.Issue{Labels: append([]string{config.LabelBuild}, labels...)})
	if planned {
		ctx.Store.Update(func(s *pyjson.Object) any {
			state.Item(s, number).Update(pyjson.New("planned_at", clock.ISO(testsupport.Day)))
			return nil
		}, "state")
	}
}

// difficultyBusy gives each provider a run in progress on a working item, so it holds a lane:
// `pairs` alternate provider and item number.
func difficultyBusy(gh *fakes.FakeGitHub, ctx *hctx.Context, pairs ...any) {
	for i := 0; i+1 < len(pairs); i += 2 {
		provider, number := pairs[i].(string), pairs[i+1].(int)
		gh.AddIssue(number, fakes.Issue{Labels: []string{"bot:working"}})
		gh.Runs.Set(strconv.Itoa(number), pyjson.New("status", "in_progress"))
		ctx.Store.Update(func(s *pyjson.Object) any {
			state.Item(s, number).Update(pyjson.New("run_id", strconv.Itoa(number), "provider", provider))
			return nil
		}, "state")
	}
}

// difficultyEach is [(provider, n) for n in range(from, to)], as busy's pairs.
func difficultyEach(provider string, from, to int) []any {
	out := []any{}
	for n := from; n < to; n++ {
		out = append(out, provider, n)
	}
	return out
}

// difficultySeat is a seat as seats() gives it: (provider, model, tier).
type difficultySeat [3]string

// difficultySeats is seats(planned): each role's (provider, model, tier), or nil, but the self
// check.
func difficultySeats(planned *pyjson.Object) map[string]any {
	out := map[string]any{}
	planned.Obj("seats").Items(func(role string, v any) bool {
		if role == "self_check" {
			return true
		}
		if seat := pyjson.Obj(v); pyjson.Truthy(v) && seat != nil {
			out[role] = difficultySeat{pyjson.Str(seat.Get("provider")), pyjson.Str(seat.Get("model")),
				pyjson.Str(seat.Get("tier"))}
		} else {
			out[role] = nil
		}
		return true
	})
	return out
}

// difficultySeatAt is seats(planned)[role]: the role's seat, or nil for None; a role the plan has
// no seat for is a KeyError.
func difficultySeatAt(t *testing.T, planned *pyjson.Object, role string) any {
	t.Helper()
	seat, ok := difficultySeats(planned)[role]
	if !ok {
		t.Fatalf("KeyError: %q in %s", role, pyjson.Dumps(planned.Get("seats")))
	}
	return seat
}

// difficultySeatOf is seats(planned)[role], which must be a seat.
func difficultySeatOf(t *testing.T, planned *pyjson.Object, role string) difficultySeat {
	t.Helper()
	seat, ok := difficultySeatAt(t, planned, role).(difficultySeat)
	if !ok {
		t.Fatalf("no %s seat in %s", role, pyjson.Dumps(planned.Get("seats")))
	}
	return seat
}

// difficultyWithPool is make_ctx(gh, at=at, cfg=dataclasses.replace(cfg, pool=...)) with
// `change` applied to a copy of the pool.
func difficultyWithPool(gh *fakes.FakeGitHub, at time.Time, cfg *config.Config, change func(*providers.Pool)) *hctx.Context {
	pool := *cfg.Pool
	change(&pool)
	replaced := *cfg
	replaced.Pool = &pool
	return testsupport.MakeCtx(gh, testsupport.Ctx{At: at, Cfg: &replaced})
}

// difficultyReplaceProvider is {**pool.providers, id: dataclasses.replace(pool.get(id), ...)}.
func difficultyReplaceProvider(pool *providers.Pool, id string, change func(*providers.Provider)) {
	provider := *pool.Get(id)
	change(&provider)
	providersCopy := pool.Providers.Copy()
	providersCopy.Set(id, &provider)
	pool.Providers = providersCopy
}

func difficultyStr(planned *pyjson.Object, key string) string { return pyjson.Str(planned.Get(key)) }

// ---------------------------------------------------------------------------- DifficultyLabelTests

func TestDifficulty_NoLabelIsMediumAndTheHardestCounts(t *testing.T) {
	of := func(names ...string) map[string]bool {
		out := map[string]bool{}
		for _, n := range names {
			out[n] = true
		}
		return out
	}
	assertEqual(t, queue.DifficultyOf(of(), ""), "medium")
	assertEqual(t, queue.DifficultyOf(of(difficultyEasy), ""), "easy")
	assertEqual(t, queue.DifficultyOf(of("Difficulty:HARD"), ""), "hard")
	assertEqual(t, queue.DifficultyOf(of(difficultyEasy, difficultyHard), ""), "hard")
	assertEqual(t, queue.DifficultyOf(of(difficultyEasy, difficultyMedium), ""), "medium")
	assertEqual(t, queue.DifficultyOf(of("difficulty:trivial", "bot:build"), ""), "medium")
	// A bot PR keeps its issue's difficulty on its record.
	assertEqual(t, queue.DifficultyOf(of(config.LabelPR), "hard"), "hard")
	assertEqual(t, queue.DifficultyOf(of(difficultyEasy), "medium"), "medium")
}

func TestDifficulty_PriorityNeverDecidesWhoMayTakeIt(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := difficultyCtxFor(gh, difficultyCtxOpts{At: testsupport.Day})
	difficultyQueue(gh, ctx, 3, true, difficultyHard, "priority:high")
	planned := flowMake(ctx)
	assertEqual(t, planned.Get("action"), "none") // no strong model is open by day
}

// ---------------------------------------------------------------------------- BuilderTests

// The builder: the first free subscription in the usage order with a seat that meets the item's
// tier, on its weakest such seat.

// With the weaker models off, an easy item goes to claude-3's Sonnet, medium since #317 part 5.
func TestDifficulty_AnEasyItemIsBuiltWithSonnetOnAClaudeAccount(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := difficultyCtxFor(gh, difficultyCtxOpts{Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN_3"),
		Machine: []string{}})
	difficultyQueue(gh, ctx, 3, false, difficultyEasy)
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider"), planned.Get("difficulty")},
		[]any{"build", "claude-3", "easy"})
	assertEqual(t, difficultySeats(planned), map[string]any{
		"plan":   difficultySeat{"claude-3", "opus", "strong"},
		"build":  difficultySeat{"claude-3", "sonnet", "medium"},
		"review": difficultySeat{"claude-3", "opus", "strong"}})
	assertEqual(t, planned.Get("routing"), []any{
		"#3: stepped up from weak to medium to build it: no weak-tier model " +
			"(lite-1) is free now"})
}

// Sonnet at xhigh is medium (#317 part 5), so claude-3 builds a medium item with it and keeps
// Opus for planning, reviews and hard items.
func TestDifficulty_AMediumItemOnAClaudeAccountIsBuiltWithSonnet(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := difficultyCtxFor(gh, difficultyCtxOpts{Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN_3"),
		Machine: []string{}})
	difficultyQueue(gh, ctx, 3, true)
	planned := flowMake(ctx)
	assertEqual(t, difficultySeats(planned)["build"], difficultySeat{"claude-3", "sonnet", "medium"})
	assertEqual(t, planned.Get("routing"), []any{})
}

func TestDifficulty_AHardItemTakesAStrongModelOnly(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := difficultyCtxFor(gh)
	// Room past the Claude accounts' thirteen runs, so the medium models are free to refuse it.
	ctx = difficultyWithPool(gh, testsupport.Night, ctx.Cfg, func(p *providers.Pool) { p.MaxParallel = 20 })
	difficultyBusy(gh, ctx, "claude-3", 50, "claude-3", 54, "claude-7", 72, "claude-7", 73,
		"claude-8", 74, "claude-8", 75, "claude-1", 51,
		"claude-1", 55, "claude-4", 52, "claude-6", 71, "claude-5", 70,
		"claude-2", 53, "claude-2", 56)
	difficultyQueue(gh, ctx, 3, true, difficultyHard)
	planned := flowMake(ctx)
	assertEqual(t, planned.Get("action"), "none") // agy and muse are free and medium, gpt weak
	assertIn(t, "no subscription can take the queue now", difficultyStr(planned, "reason"))
}

// For easy items the lanes fill in tier order (#695): the weak model first while it has a lane,
// the weak one; then the medium models, Muse on its four lanes (#1273), agy on its one and Codex
// on its one; then the Sonnet seats in usage order, claude-3, claude-7, claude-8, claude-1,
// claude-4, claude-6 and claude-5 once each, and only then claude-3's second lane (two lanes,
// uncapped) and claude-8's second (two lanes): with every account's room level, the load balancer
// gives the next run to one holding none (`plan.balance`); and claude-2 builds only after all of
// them (`build_last`). Past that every lane is held, and the rest wait.
func TestDifficulty_TheUsageOrder(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := difficultyCtxFor(gh, difficultyCtxOpts{Machine: testsupport.AllMachine})
	for number := 3; number < 21; number++ {
		difficultyQueue(gh, ctx, number, true, difficultyEasy)
		ctx.Store.Update(func(s *pyjson.Object) any {
			state.Item(s, number).Update(pyjson.New("queued_at", fmt.Sprintf("2026-09-%02dT00:00:00Z", number+10)))
			return nil
		}, "state")
	}
	ctx = difficultyWithPool(gh, testsupport.Night, ctx.Cfg, func(p *providers.Pool) {
		p.MaxParallel, p.MachineParallel = 20, 20
		difficultyReplaceProvider(p, "lite", func(lite *providers.Provider) { lite.Lanes = 1 })
	})
	order := []any{}
	for range 18 {
		planned := flowMake(ctx)
		if planned.Get("action") == "none" {
			break
		}
		number, _ := pyjson.AsInt(planned.Get("number"))
		gh.Runs.Set(strconv.Itoa(number), pyjson.New("status", "in_progress"))
		ctx.Store.Update(func(s *pyjson.Object) any {
			state.Item(s, number).Update(pyjson.New("run_id", strconv.Itoa(number)))
			return nil
		}, "state")
		order = append(order, []any{planned.Get("provider"), difficultySeatOf(t, planned, "build")[1]})
	}
	want := []any{[]any{"lite", "lite-1"}}
	for range 4 {
		want = append(want, []any{"muse", "muse-spark-1.3-contributor"})
	}
	for _, p := range [][2]string{{"agy", "gemini-3.8-flash-high"}, {"gpt", "gpt-5.6-sol"},
		{"claude-3", "sonnet"}, {"claude-7", "sonnet"}, {"claude-8", "sonnet"}, {"claude-1", "sonnet"},
		{"claude-4", "sonnet"}, {"claude-6", "sonnet"}, {"claude-5", "sonnet"}, {"claude-3", "sonnet"},
		{"claude-8", "sonnet"}, {"claude-2", "sonnet"}} {
		want = append(want, []any{p[0], p[1]})
	}
	assertEqual(t, order, want)
}

// A seat marked `takes_over` stands in for another subscription: with the weak one free it builds
// nothing, and it never builds a medium item, which its account builds with Opus. No committed
// seat stands in now (every Claude account switches between its two models), so the test marks
// claude-4's Sonnet itself.
func TestDifficulty_ASonnetStandInBuildsOnlyEasyItemsTheWeakOneCannotTake(t *testing.T) {
	standIn := func(gh *fakes.FakeGitHub, machine []string) *hctx.Context {
		raw := testsupport.WithWeak(testsupport.RawProviders())
		raw.Obj("providers").Items(func(name string, v any) bool {
			provider := pyjson.Obj(v)
			if provider.Get("login") == "machine" {
				provider.Set("enabled", slices.Contains(machine, name))
			}
			if provider.Get("cli") == "claude" {
				provider.Set("schedule", pyjson.New("mode", "window", "start", "21:00", "end", "07:00"))
				provider.Pop("off_hours")
			}
			return true
		})
		raw.Set("plan_lanes", 0)
		pyjson.Obj(raw.Obj("providers").Obj("claude-4").List("extra_models")[0]).Set("takes_over", "lite")
		cfg := testsupport.MakeConfig(testsupport.Config{Env: providersSecrets(providers.SecretNames...),
			Machine: machine})
		pool := providers.Parse(raw)
		pool.MaxParallel, pool.MachineParallel = 20, 20
		replaced := *cfg
		replaced.Pool = pool
		return testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night, Cfg: &replaced})
	}
	gh := fakes.NewFakeGitHub()
	ctx := standIn(gh, []string{"lite"})
	difficultyBusy(gh, ctx, "claude-3", 50, "claude-3", 59, "claude-7", 57, "claude-7", 67,
		"claude-1", 51,
		"claude-1", 58)
	difficultyQueue(gh, ctx, 3, true, difficultyEasy)
	assertEqual(t, difficultySeats(flowMake(ctx))["build"], difficultySeat{"lite", "lite-1", "weak"})
	gh = fakes.NewFakeGitHub()
	ctx = standIn(gh, []string{"lite"})
	difficultyBusy(gh, ctx, append([]any{"claude-3", 50, "claude-3", 59, "claude-7", 57, "claude-7", 67,
		"claude-8", 68, "claude-8", 75, "claude-1", 51,
		"claude-1", 58},
		difficultyEach("lite", 60, 66)...)...)
	difficultyQueue(gh, ctx, 3, true, difficultyEasy)
	assertEqual(t, difficultySeats(flowMake(ctx))["build"], difficultySeat{"claude-4", "sonnet", "medium"})
	gh = fakes.NewFakeGitHub()
	ctx = standIn(gh, []string{})
	difficultyBusy(gh, ctx, "claude-3", 50, "claude-3", 59, "claude-7", 57, "claude-7", 67,
		"claude-8", 68, "claude-8", 75, "claude-1", 51,
		"claude-1", 58, "claude-6", 71, "claude-5", 70,
		"claude-2", 53, "claude-2", 56)
	difficultyQueue(gh, ctx, 3, true, difficultyMedium)
	assertEqual(t, difficultySeats(flowMake(ctx))["build"], difficultySeat{"claude-4", "opus", "strong"})
	// A stand-in never reviews: the review rule counts the run's reviewer, Opus.
	assertEqual(t, ctx.Cfg.Pool.BestSeat(ctx.Cfg.Pool.Get("claude-4"), "medium").Model, "opus")
}

// Each Claude account runs Opus and Sonnet both: the router puts a medium item on its weakest
// seat that meets the tier, Sonnet, and keeps Opus for the strong roles.
func TestDifficulty_EveryClaudeAccountBuildsAMediumItemOnSonnet(t *testing.T) {
	for _, account := range []string{"claude-1", "claude-2", "claude-3", "claude-4", "claude-5",
		"claude-6", "claude-7", "claude-8"} {
		t.Run("account="+account, func(t *testing.T) {
			gh := fakes.NewFakeGitHub()
			ctx := difficultyCtxFor(gh, difficultyCtxOpts{
				Env: providersSecrets(difficultyClaudeSecrets[account]), Machine: []string{}})
			difficultyQueue(gh, ctx, 3, true, difficultyMedium)
			assertEqual(t, difficultySeats(flowMake(ctx))["build"], difficultySeat{account, "sonnet", "medium"})
			gh = fakes.NewFakeGitHub()
			ctx = difficultyCtxFor(gh, difficultyCtxOpts{
				Env: providersSecrets(difficultyClaudeSecrets[account]), Machine: []string{}})
			difficultyQueue(gh, ctx, 3, true, difficultyHard)
			assertEqual(t, difficultySeats(flowMake(ctx))["build"], difficultySeat{account, "opus", "strong"})
		})
	}
}

func TestDifficulty_SonnetOnlyWhileClaude1OrClaude3IsOpen(t *testing.T) {
	// claude-3, claude-7 and claude-8 full (claude-3's two lanes, claude-7's one, claude-8's
	// three), claude-1 open and the weak one full: claude-1 builds the easy item with Sonnet.
	gh := fakes.NewFakeGitHub()
	ctx := difficultyCtxFor(gh, difficultyCtxOpts{Machine: []string{"lite"}})
	difficultyBusy(gh, ctx, append([]any{"claude-3", 50, "claude-3", 59, "claude-7", 72, "claude-7", 73,
		"claude-8", 74, "claude-8", 75},
		difficultyEach("lite", 60, 66)...)...)
	difficultyQueue(gh, ctx, 3, true, difficultyEasy)
	assertEqual(t, difficultySeats(flowMake(ctx))["build"], difficultySeat{"claude-1", "sonnet", "medium"})
	// Neither open and the medium models busy: the weak one builds it, the default, while claude-2
	// (build_last, kept for planning and review) stays free.
	gh = fakes.NewFakeGitHub()
	ctx = difficultyCtxFor(gh, difficultyCtxOpts{Machine: testsupport.AllMachine})
	difficultyBusy(gh, ctx, "claude-3", 50, "claude-3", 63, "claude-7", 72, "claude-7", 73,
		"claude-8", 74, "claude-8", 75, "claude-1", 51,
		"claude-1", 64, "claude-4", 52, "claude-6", 71, "claude-5", 70, "agy", 53)
	ctx = difficultyWithPool(gh, testsupport.Night, ctx.Cfg, func(p *providers.Pool) {
		p.MaxParallel, p.MachineParallel = 20, 20
	})
	difficultyBusy(gh, ctx, "muse", 54, "muse", 62, "muse", 66, "muse", 67, // Muse has four lanes
		"gpt", 55)
	difficultyQueue(gh, ctx, 3, true, difficultyEasy)
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("provider"), difficultySeatOf(t, planned, "build")[1]}, []any{"lite", "lite-1"})
	// Only with all of the weak one's lanes taken does claude-2 build it, on its Sonnet seat: its
	// Opus stays for planning and review.
	gh.Threads.Item(3).Set("state", "open")
	gh.Threads.Item(3).Set("labels", []any{pyjson.New("name", config.LabelBuild), pyjson.New("name", difficultyEasy)})
	difficultyBusy(gh, ctx, difficultyEach("lite", 56, 62)...)
	assertEqual(t, difficultySeats(flowMake(ctx))["build"], difficultySeat{"claude-2", "sonnet", "medium"})
}

// The weak one is weak, so a medium item passes it by: with only claude-2 and the weak one free,
// claude-2 builds it, on Sonnet.
func TestDifficulty_Claude2BuildsAMediumItemTheWeakOneMayNot(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := difficultyCtxFor(gh, difficultyCtxOpts{Machine: testsupport.AllMachine})
	ctx = difficultyWithPool(gh, testsupport.Night, ctx.Cfg, func(p *providers.Pool) {
		p.MaxParallel, p.MachineParallel = 20, 20
	})
	difficultyBusy(gh, ctx, "claude-3", 50, "claude-3", 57, "claude-7", 72, "claude-7", 73,
		"claude-8", 74, "claude-8", 75, "claude-1", 51,
		"claude-1", 58, "claude-4", 52, "claude-6", 71, "claude-5", 70, "agy", 53,
		"muse", 54, "muse", 56, "muse", 60, "muse", 61, "gpt", 55)
	difficultyQueue(gh, ctx, 3, true, difficultyMedium)
	assertEqual(t, difficultySeats(flowMake(ctx))["build"], difficultySeat{"claude-2", "sonnet", "medium"})
}

func TestDifficulty_WithNoModelOfItsTierFreeItStepsUpAndSaysWhy(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := difficultyCtxFor(gh, difficultyCtxOpts{At: testsupport.Day, Machine: []string{"agy"}}) // by day: no Claude, and the weak one off
	difficultyQueue(gh, ctx, 3, true, difficultyEasy)
	planned := flowMake(ctx)
	assertEqual(t, difficultySeats(planned)["build"], difficultySeat{"agy", "gemini-3.8-flash-high", "medium"})
	assertEqual(t, planned.Get("routing"), []any{
		"#3: stepped up from weak to medium to build it: no weak-tier model " +
			"(lite-1) is free now"})
	routing := []string{}
	for _, line := range planned.List("routing") {
		routing = append(routing, pyjson.Str(line))
	}
	assertIn(t, "stepped up from weak to medium", difficultyStr(planned, "assignment")+strings.Join(routing, " "))
	// Medium with only a strong model free steps up to strong: claude-2 by day, whose Sonnet is
	// switched off for the test, so only its Opus is left.
	gh = fakes.NewFakeGitHub()
	ctx = difficultyCtxFor(gh, difficultyCtxOpts{At: testsupport.Day,
		Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN_2"), Machine: []string{}, CommittedHours: true})
	ctx = difficultyWithPool(gh, testsupport.Day, ctx.Cfg, func(p *providers.Pool) {
		difficultyReplaceProvider(p, "claude-2", func(c *providers.Provider) { c.ExtraModels = []*providers.ExtraModel{} })
	})
	difficultyQueue(gh, ctx, 3, true)
	planned = flowMake(ctx)
	assertEqual(t, difficultySeats(planned)["build"], difficultySeat{"claude-2", "opus", "strong"})
	assertIn(t, "stepped up from medium to strong", pyjson.Str(planned.List("routing")[0]))
}

func TestDifficulty_APersonsPullRequestIsRevisedOnlyWithAReviewerInTheRun(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := difficultyCtxFor(gh, difficultyCtxOpts{At: testsupport.Day, Env: providersSecrets(),
		Machine: []string{"lite"}})
	gh.AddPull(9, "feature/someone", fakes.Pull{Labels: []string{"bot:revise", difficultyEasy}})
	// Only the weak one is free, and it cannot review: a person's PR gets no review run, so it
	// waits.
	assertEqual(t, flowMake(ctx).Get("action"), "none")
	// The bot's own pull request does get a review run, so the weak one may revise it.
	gh.AddPull(10, "bot/issue-1", fakes.Pull{Labels: []string{config.LabelPR, "bot:revise", difficultyEasy}})
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("number"), planned.Get("provider")}, []any{10, "lite"})
}

func TestDifficulty_RevisionsFollowTheBuildingRule(t *testing.T) {
	// The weak one, weak and `easy_first`, revises an easy pull request first; without it, the
	// lane's weakest medium seat does: Muse (#695).
	for _, c := range []struct {
		machine []string
		seat    difficultySeat
	}{
		{[]string{"lite", "muse"}, difficultySeat{"lite", "lite-1", "weak"}},
		{testsupport.Machine, difficultySeat{"muse", "muse-spark-1.3-contributor", "medium"}},
	} {
		gh := fakes.NewFakeGitHub()
		ctx := difficultyCtxFor(gh, difficultyCtxOpts{Machine: c.machine})
		gh.AddPull(9, "bot/issue-1", fakes.Pull{Labels: []string{config.LabelPR, "bot:revise", difficultyEasy}})
		planned := flowMake(ctx)
		assertEqual(t, []any{planned.Get("action"), difficultySeats(planned)["build"]}, []any{"revise", c.seat})
		assertEqual(t, difficultySeats(planned)["plan"], nil) // a revision is not planned again
	}
}

// ---------------------------------------------------------------------------- PlannerTests

// The lane's weakest planner meeting the floor plans it (#695): Muse plans an unrated item, while
// a hard one still takes a strong planner (claude-2, kept for planning).
func TestDifficulty_AStrongPlannerPlansOnThePlanningLane(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, Cfg: testsupport.MakeConfig(
		testsupport.Config{Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN_2"), Machine: testsupport.Machine,
			CommittedHours: true, CommittedPlanLanes: true})})
	difficultyQueue(gh, ctx, 3, false)
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"plan", "muse"})
	assertEqual(t, difficultySeats(planned), map[string]any{
		"plan":  difficultySeat{"muse", "muse-spark-1.3-contributor", "medium"},
		"build": nil, "review": nil})
	assertIn(t, "needs a plan; `muse`", pyjson.Str(planned.List("routing")[0]))
	difficultyQueue(gh, ctx, 4, false, difficultyHard)
	planned = flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"plan", "claude-2"})
	assertEqual(t, difficultySeats(planned)["plan"], difficultySeat{"claude-2", "opus", "strong"})
	said := gh.BotComments(3)
	assertIn(t, "goes into this issue's description", said[len(said)-1])
}

// An item nobody has rated or planned may be rated and planned by a medium model (#317 part 8):
// Muse, first of them, plans it in its own run.
func TestDifficulty_WithNoStrongModelFreeAMediumOnePlansInTheSameRun(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := difficultyCtxFor(gh, difficultyCtxOpts{At: testsupport.Day})
	difficultyQueue(gh, ctx, 3, false)
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), difficultySeats(planned)["plan"]},
		[]any{"build", difficultySeat{"muse", "muse-spark-1.3-contributor", "medium"}})
}

// A rated medium item is planned by a medium model (#695): with none free on the lane, a medium
// builder plans it in its own run, as it plans an easy one.
func TestDifficulty_AMediumModelPlansARatedMediumItemInItsRun(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := difficultyCtxFor(gh, difficultyCtxOpts{At: testsupport.Day})
	difficultyQueue(gh, ctx, 3, false, difficultyMedium)
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), difficultySeatOf(t, planned, "plan")[2]}, []any{"build", "medium"})
	difficultyQueue(gh, ctx, 4, false, difficultyEasy)
	planned = flowMake(ctx)
	assertEqual(t, []any{planned.Get("number"), difficultySeatOf(t, planned, "plan")[2]}, []any{4, "medium"})
}

func TestDifficulty_WithNoPlannerFreeNothingIsBuilt(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := difficultyCtxFor(gh, difficultyCtxOpts{At: testsupport.Day, Env: providersSecrets(),
		Machine: []string{"lite"}})
	difficultyQueue(gh, ctx, 3, false, difficultyEasy)
	planned := flowMake(ctx)
	assertEqual(t, planned.Get("action"), "none")
}

func TestDifficulty_APlanningRunHandsItsPlanToTheBuilder(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN_2"),
		Machine: testsupport.Machine, At: testsupport.Day, CommittedPlanLanes: true})
	h.committedHours()
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Make the rules v2."})
	planText := "1. **Goal.** Rules v2.\n2. **Steps.** Edit src/game.txt; run the checks."
	planner := workFake(map[string][]runner.Handler{"plan": difficultyOne(func(request *runner.RunRequest) *runner.RunResult {
		return runner.NewRunResult(true, planText)
	})})
	planned, result := h.night(planner)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider"), result.Get("status")},
		[]any{"plan", "muse", "planned"})
	calls := []any{}
	for _, c := range planner.Calls {
		calls = append(calls, []any{c.Role, c.Model, c.ReadOnly})
	}
	assertEqual(t, calls, []any{[]any{"plan", "muse-spark-1.3-contributor", true}})
	record := h.ctx.Store.Load().Obj("items").Obj("12")
	assertEqual(t, record.Obj("handoff").Get("kind"), "plan")
	assertIn(t, "Rules v2", pyjson.Str(record.Obj("handoff").Get("notes")))
	assertTrue(t, record.Truthy("planned_at"))
	assertEqual(t, record.Get("planned_tier"), "medium")
	assertEqual(t, record.Get("planned_by_id"), "muse")
	// Out of the Needs plan stage; no rating line in the plan, so it is labelled medium (#735).
	assertEqual(t, h.gh.LabelNames(12), map[string]bool{config.LabelBuild: true, "bot:planned": true,
		"difficulty:medium": true})
	said := h.gh.BotComments(12)
	assertIn(t, "Planned on `muse` (muse, `muse-spark-1.3-contributor`, medium)", said[len(said)-1])
	assertIn(t, "in this issue's description", said[len(said)-1])
	body := pyjson.Str(h.gh.Threads.Item(12).Get("body"))
	assertTrue(t, strings.HasPrefix(body, "Make the rules v2.\n\n<!-- jackioh-bot:plan -->"), body)
	assertIn(t, "## Plan", body)
	assertIn(t, "Edit src/game.txt", body)
	assertEqual(t, h.originSHA("bot/issue-12"), "") // nothing built yet
	// A person corrects the plan in the description: the builder follows their version.
	h.gh.Threads.Item(12).Set("body", strings.ReplaceAll(body, "Edit src/game.txt", "Edit src/game.txt only"))
	// The next run builds from it, on the cheapest model its difficulty allows.
	seen := map[string]string{}
	building := func(request *runner.RunRequest) *runner.RunResult {
		notes, err := os.ReadFile(filepath.Join(request.Cwd, work.NotesFile))
		if err != nil {
			panic(err)
		}
		seen["notes"] = string(notes)
		return workBuilder(map[string]string{"src/game.txt": "rules v2\n"})(request)
	}
	r := workFake(map[string][]runner.Handler{"build": difficultyOne(building),
		"review": difficultyOne(workReviewer(workApprove))})
	planned, result = h.night(r)
	// One lane, one owner (#725): Muse keeps the medium item it planned. Its own review approves,
	// but the rule still waits for a Strong seat (`bot:cross-review`).
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider"), result.Get("status")},
		[]any{"build", "muse", "approved"})
	roles := []string{}
	for _, c := range r.Calls {
		roles = append(roles, c.Role)
	}
	assertEqual(t, roles, []string{"build", "review"})
	pulls := h.gh.ListPulls("open", "bot/issue-12", 200)
	assertEqual(t, len(pulls), 1)
	number, _ := pyjson.AsInt(pulls[0].Get("number"))
	assertIn(t, "bot:cross-review", h.gh.LabelNames(number))
	assertIn(t, "## The plan", r.Calls[0].Prompt)
	assertIn(t, "**Plan** section of the issue's description", r.Calls[0].Prompt)
	assertIn(t, "Edit src/game.txt only", r.Calls[0].Prompt)
	assertIn(t, "Edit src/game.txt only", seen["notes"])
}

// ---------------------------------------------------------------------------- ReviewRunTests

// difficultyPull is ReviewRunTests.pull: a bot pull request waiting for a review run.
func difficultyPull(gh *fakes.FakeGitHub, ctx *hctx.Context, labels []string, votes *pyjson.Object) {
	gh.AddPull(9, "bot/issue-5", fakes.Pull{Labels: append([]string{config.LabelPR, config.LabelCross}, labels...),
		SHA: "h1"})
	if votes == nil {
		votes = pyjson.New("sha", "h1", "builder", "gpt", "approvals", []any{"gpt"},
			"tiers", pyjson.New("gpt", "medium"))
	}
	ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, 9).Update(pyjson.New("votes", pyjson.DeepCopy(votes)))
		return nil
	}, "state")
}

func TestDifficulty_StrongFirstInTheUsageOrder(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := difficultyCtxFor(gh)
	// Room past the cap on GitHub's runners (`max_parallel`, #1455): the usage order is this
	// test's, the cap its own.
	ctx = difficultyWithPool(gh, testsupport.Night, ctx.Cfg, func(p *providers.Pool) { p.MaxParallel = 20 })
	difficultyPull(gh, ctx, nil, nil)
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), difficultySeats(planned)["review"]},
		[]any{"review", difficultySeat{"claude-3", "opus", "strong"}})
	difficultyBusy(gh, ctx, "claude-3", 50, "claude-3", 57, "claude-7", 72, "claude-7", 73,
		"claude-8", 74, "claude-8", 75, "claude-1", 51,
		"claude-1", 58)
	gh.Threads.Item(9).Set("labels", []any{pyjson.New("name", config.LabelPR), pyjson.New("name", config.LabelCross)})
	assertEqual(t, difficultySeatOf(t, flowMake(ctx), "review")[0], "claude-4")
}

// A medium item's final audit is Strong-only (#725): with no strong seat free, no review run goes
// out; with one free, Opus audits.
func TestDifficulty_AMediumItemWaitsForItsStrongFinal(t *testing.T) {
	votes := pyjson.New("sha", "h1", "builder", "muse", "approvals", []any{"muse"},
		"tiers", pyjson.New("muse", "medium"))
	gh := fakes.NewFakeGitHub()
	ctx := difficultyCtxFor(gh, difficultyCtxOpts{At: testsupport.Day}) // no Claude account by day: nothing may review it
	difficultyPull(gh, ctx, nil, votes)
	assertEqual(t, flowMake(ctx).Get("action"), "none")
	gh = fakes.NewFakeGitHub()
	ctx = difficultyCtxFor(gh, difficultyCtxOpts{At: testsupport.Day, CommittedHours: true}) // claude-3 all day
	difficultyPull(gh, ctx, nil, votes)
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), difficultySeats(planned)["review"]},
		[]any{"review", difficultySeat{"claude-3", "opus", "strong"}})
	assertEqual(t, planned.Get("approved"), []any{"muse"})
}

func TestDifficulty_AHardPullRequestWaitsForStrong(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := difficultyCtxFor(gh, difficultyCtxOpts{At: testsupport.Day, CommittedHours: true}) // claude-3 all day
	difficultyPull(gh, ctx, []string{difficultyHard}, nil)
	assertEqual(t, difficultySeats(flowMake(ctx))["review"], difficultySeat{"claude-3", "opus", "strong"})
	gh = fakes.NewFakeGitHub()
	ctx = difficultyCtxFor(gh, difficultyCtxOpts{At: testsupport.Day}) // no strong seat by day: a hard item waits for Opus
	difficultyPull(gh, ctx, []string{difficultyHard}, nil)
	assertEqual(t, flowMake(ctx).Get("action"), "none")
}

// No weak review runs go out at any difficulty (#725): weak builders review themselves in fresh
// sessions, and a weak approval never ships.
func TestDifficulty_NoWeakReviewRunsGoOut(t *testing.T) {
	reviewWithTheWeakOneOnly := func(labels []string, votes *pyjson.Object) *pyjson.Object {
		gh := fakes.NewFakeGitHub()
		ctx := difficultyCtxFor(gh, difficultyCtxOpts{At: testsupport.Day, Env: providersSecrets(),
			Machine: []string{"lite"}})
		difficultyPull(gh, ctx, labels, votes)
		return flowMake(ctx)
	}
	assertEqual(t, reviewWithTheWeakOneOnly([]string{difficultyEasy}, nil).Get("action"), "none")
	assertEqual(t, reviewWithTheWeakOneOnly(nil, nil).Get("action"), "none")
	assertEqual(t, reviewWithTheWeakOneOnly([]string{difficultyHard}, nil).Get("action"), "none")
	// An easy item takes the weakest seat that ships it: medium, not weak.
	gh := fakes.NewFakeGitHub()
	ctx := difficultyCtxFor(gh, difficultyCtxOpts{At: testsupport.Day, Env: providersSecrets(),
		Machine: []string{"lite", "agy"}})
	difficultyPull(gh, ctx, []string{difficultyEasy}, nil)
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), difficultySeats(planned)["review"]},
		[]any{"review", difficultySeat{"agy", "gemini-3.8-flash-high", "medium"}})
}

func TestDifficulty_WhetherTheReviewersSetUpCouldStillShipIt(t *testing.T) {
	// A strong reviewer ships anything; a medium one ships only an easy item (#725). Anything
	// weaker never ships, whatever is set up.
	assertTrue(t, reviewrule.Reachable([]string{}, []string{"strong"}, "hard"))
	assertTrue(t, reviewrule.Reachable([]string{"medium"}, []string{"strong"}, "easy"))
	assertFalse(t, reviewrule.Reachable([]string{}, []string{"medium"}, "hard"))
	assertFalse(t, reviewrule.Reachable([]string{}, []string{"medium"}, "medium"))
	assertTrue(t, reviewrule.Reachable([]string{}, []string{"medium"}, "easy"))
	assertTrue(t, reviewrule.Reachable([]string{"medium"}, []string{"weak"}, "easy")) // already met
	assertFalse(t, reviewrule.Reachable([]string{"medium"}, []string{"weak"}, "medium"))
	assertFalse(t, reviewrule.Reachable([]string{}, []string{"weak"}, "easy")) // weak alone never ships it
	assertFalse(t, reviewrule.Reachable([]string{"weak"}, []string{}, "easy"))
	assertEqual(t, reviewrule.Who([][2]string{{"muse", "medium"}, {"muse", "medium"}, {"claude", "strong"}}),
		"`muse` (medium) twice, `claude` (strong)")
}

func TestDifficulty_TheRule(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, Machine: testsupport.Machine})
	out := filepath.Join(h.root, "o")
	if err := os.Mkdir(out, 0o777); err != nil {
		t.Fatal(err)
	}
	d := deliver.NewDeliverer(h.ctx, pyjson.New("action", "review", "provider", "agy"), out, h.deliverRepo,
		"", nil, "")
	rule := d.RuleMet

	reviews := func(pairs ...[2]string) *pyjson.Object {
		approvals := []any{}
		list := []any{}
		for _, p := range pairs {
			if !slices.ContainsFunc(approvals, func(a any) bool { return a == p[0] }) {
				approvals = append(approvals, p[0])
			}
			list = append(list, pyjson.New("family", p[0], "tier", p[1]))
		}
		return pyjson.New("approvals", approvals, "reviews", list)
	}
	strong, gemini, muse, lite := [2]string{"claude", "strong"}, [2]string{"gemini", "medium"},
		[2]string{"muse", "medium"}, [2]string{"lite", "weak"}
	for _, difficulty := range []string{"medium", "hard"} {
		// One strong approval ships; anything weaker, even two medium ones or a weak and a medium
		// one, does not (#725 keeps Strong-only for Medium/Hard finals).
		assertTrue(t, rule(reviews(strong), difficulty))
		assertFalse(t, rule(reviews(gemini, muse), difficulty))
		assertFalse(t, rule(reviews(muse, muse), difficulty))
		assertFalse(t, rule(reviews(gemini), difficulty))
		assertFalse(t, rule(reviews(lite, lite), difficulty))
		assertFalse(t, rule(reviews(lite, gemini), difficulty))
	}
	// An easy item ships on its Medium final audit (#725); a weak approval never does.
	assertTrue(t, rule(reviews(strong), "easy"))
	assertTrue(t, rule(reviews(gemini), "easy"))
	assertTrue(t, rule(reviews(gemini, muse), "easy"))
	assertTrue(t, rule(reviews(muse, muse), "easy"))
	assertTrue(t, rule(reviews(lite, gemini), "easy"))
	assertFalse(t, rule(reviews(lite, lite), "easy"))
	assertFalse(t, rule(reviews(lite), "easy"))
	// Votes kept before reviews were counted one by one: one per family.
	assertFalse(t, rule(pyjson.New("approvals", []any{"gemini", "muse"},
		"tiers", pyjson.New("gemini", "medium", "muse", "medium")), "hard"))
	assertTrue(t, rule(pyjson.New("approvals", []any{"gemini", "lite"},
		"tiers", pyjson.New("gemini", "medium", "lite", "weak")), "easy"))
	assertFalse(t, rule(pyjson.New("approvals", []any{"gemini", "lite"},
		"tiers", pyjson.New("gemini", "medium", "lite", "weak")), "medium"))
	// A rejection stands until its model approves.
	assertFalse(t, rule(pyjson.New("approvals", []any{"claude"}, "rejections", []any{"gemini"},
		"tiers", pyjson.New("claude", "strong")), "medium"))
	// A vote kept before tiers counts at its family's strongest tier.
	assertTrue(t, rule(pyjson.New("approvals", []any{"claude"}), "medium"))
	assertFalse(t, rule(pyjson.New("approvals", []any{"gpt"}), "medium"))
}

// ---------------------------------------------------------------------------- SelfCheckTests

// A builder whose seat has `self_check` (the weak one) checks its own change before any review.

type difficultySelfCheck struct {
	t      *testing.T
	root   string
	origin string
	clone  string
	out    string
}

func newDifficultySelfCheck(t *testing.T) *difficultySelfCheck {
	s := &difficultySelfCheck{t: t, root: t.TempDir()}
	s.origin, s.clone = fakes.MakeOrigin(s.root)
	s.out = filepath.Join(s.root, "out")
	return s
}

// difficultyWorkerOpts is SelfCheckTests.worker's keyword arguments; a zero field is the default.
type difficultyWorkerOpts struct {
	Provider string // provider = "lite"
	Review   bool   // review = None
	Rounds   int    // rounds = 3
	Action   string // action = "build"
}

func (s *difficultySelfCheck) worker(r *runner.FakeRunner, opts ...difficultyWorkerOpts) *work.Worker {
	var o difficultyWorkerOpts
	if len(opts) > 0 {
		o = opts[0]
	}
	if o.Provider == "" {
		o.Provider = "lite"
	}
	if o.Rounds == 0 {
		o.Rounds = 3
	}
	if o.Action == "" {
		o.Action = "build"
	}
	raw := testsupport.WithWeak(testsupport.RawProviders())
	if o.Provider == "agy" {
		raw.Obj("providers").Obj("agy").Set("self_check", true) // a self-checker with a reviewer too
	}
	cfg := testsupport.MakeConfig(testsupport.Config{Machine: testsupport.AllMachine, Overrides: pyjson.New(
		"gates", workGates, "install", pyjson.New("run", "true", "timeout_minutes", 1),
		"max_review_cycles", 3, "max_self_check_rounds", o.Rounds)})
	replaced := *cfg
	replaced.Pool = providers.Parse(raw)
	build := replaced.Pool.Seats(replaced.Pool.Get(o.Provider))[0]
	var review any
	if o.Review {
		review = build.ToDict()
	}
	planned := pyjson.New("action", o.Action, "number", 12, "title", "Rules v2", "branch", "bot/issue-12",
		"thread", "Please make the rules v2.", "provider", o.Provider,
		"seats", pyjson.New("plan", nil, "build", build.ToDict(), "review", review, "self_check", true))
	if o.Action == "revise" {
		fakes.PushBranch(s.origin, s.root, "bot/issue-12", map[string]string{"src/game.txt": "v1\n"})
		fakes.Git(s.clone, "fetch", "-q", "origin")
		planned.Update(pyjson.New("source", "request", "pull", "p", "issue", "", "feedback", "f"))
	}
	return work.NewWorker(&replaced, planned, r, s.clone, filepath.Join(s.root, "work"), s.out, nil, nil,
		nil, nil, nil, nil)
}

func TestDifficulty_ACleanFirstSelfCheck(t *testing.T) {
	s := newDifficultySelfCheck(t)
	r := workFake(map[string][]runner.Handler{
		"build":      difficultyOne(workBuilder(map[string]string{"src/game.txt": "v2\n"})),
		"self_check": difficultyOne(workReviewer(workApprove))})
	result := s.worker(r).Run()
	calls := []any{}
	for _, c := range r.Calls {
		calls = append(calls, []any{c.Role, c.Model, c.ReadOnly})
	}
	assertEqual(t, calls, []any{[]any{"build", "lite-1", false}, []any{"self_check", "lite-1", true}})
	assertIn(t, "This is a self check, not a review", r.Calls[1].Prompt)
	// The weak one has no seat that may review: the change goes to a review run, never approved.
	assertEqual(t, result.Get("status"), "built")
	assertEqual(t, len(pyjson.Obj(result.List("cycles")[0]).List("self_check")), 1)
	assertEqual(t, result.Get("self_check_findings"), []any{})
	_, err := os.Stat(filepath.Join(s.out, "branch.bundle"))
	assertTrue(t, err == nil, err)
}

func TestDifficulty_FlaggedThenFixed(t *testing.T) {
	s := newDifficultySelfCheck(t)
	r := workFake(map[string][]runner.Handler{
		"build": difficultyOne(workBuilder(map[string]string{"src/game.txt": "v2 (half)\n"})),
		"fix":   difficultyOne(workBuilder(map[string]string{"src/game.txt": "v2\n"})),
		"self_check": {workReviewer(workChanges("Only half the rules.")),
			workReviewer(workApprove)}})
	result := s.worker(r).Run()
	assertEqual(t, workRoles(r), []string{"build", "self_check", "fix", "self_check"})
	assertIn(t, "Only half the rules.", r.Calls[2].Prompt)
	assertIn(t, "Your self check's blocking findings", r.Calls[2].Prompt)
	assertEqual(t, []any{result.Get("status"), result.Get("self_check_findings")}, []any{"built", []any{}})
	rounds := pyjson.Obj(result.List("cycles")[0]).List("self_check")
	flagged := []any{}
	for _, round := range rounds {
		flagged = append(flagged, pyjson.Obj(round).Get("flagged"))
	}
	assertEqual(t, flagged, []any{1, 0})
}

func TestDifficulty_TheCapSendsItToReviewWithTheOpenFindings(t *testing.T) {
	s := newDifficultySelfCheck(t)
	r := workFake(map[string][]runner.Handler{
		"build":      difficultyOne(workBuilder(map[string]string{"src/game.txt": "v2\n"})),
		"fix":        difficultyOne(workBuilder(map[string]string{"src/game.txt": "v2 again\n"})),
		"self_check": difficultyOne(workReviewer(workChanges("Still not sure about the Coin."))),
		"review":     difficultyOne(workReviewer(workApprove))})
	result := s.worker(r, difficultyWorkerOpts{Provider: "agy", Review: true, Rounds: 2}).Run()
	assertEqual(t, workRoles(r), []string{"build", "self_check", "fix", "self_check", "review"})
	reviewPrompt := r.Calls[len(r.Calls)-1].Prompt
	assertIn(t, "ran out with these findings still open", reviewPrompt)
	assertIn(t, "Still not sure about the Coin.", reviewPrompt)
	assertEqual(t, result.Get("status"), "approved")
	assertEqual(t, pyjson.Obj(result.List("self_check_findings")[0]).Get("claim"), "Still not sure about the Coin.")
}

func TestDifficulty_ACleanSelfCheckIsNotAnApproval(t *testing.T) {
	s := newDifficultySelfCheck(t)
	r := workFake(map[string][]runner.Handler{
		"build":      difficultyOne(workBuilder(map[string]string{"src/game.txt": "v2\n"})),
		"fix":        difficultyOne(workBuilder(map[string]string{"src/game.txt": "v2 fixed\n"})),
		"self_check": difficultyOne(workReviewer(workApprove)),
		"review": {workReviewer(workChanges("The Coin is wrong.")),
			workReviewer(workApprove)}})
	result := s.worker(r, difficultyWorkerOpts{Provider: "agy", Review: true}).Run()
	// The reviewer's findings go to a fix, which goes through the self check again.
	assertEqual(t, workRoles(r), []string{"build", "self_check", "review", "fix", "self_check", "review"})
	assertEqual(t, result.Get("status"), "approved")
	assertEqual(t, len(result.List("cycles")), 2)
}

func TestDifficulty_ARevisionChecksItselfToo(t *testing.T) {
	s := newDifficultySelfCheck(t)
	r := workFake(map[string][]runner.Handler{
		"revise":     difficultyOne(workBuilder(map[string]string{"src/game.txt": "v2\n"})),
		"self_check": difficultyOne(workReviewer(workApprove))})
	result := s.worker(r, difficultyWorkerOpts{Action: "revise"}).Run()
	assertEqual(t, workRoles(r), []string{"revise", "self_check"})
	assertEqual(t, result.Get("status"), "built")
}

func TestDifficulty_ASelfCheckedBuildWaitsForAReviewRun(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: providersSecrets(), Machine: []string{"lite"},
		At: testsupport.Day})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, difficultyEasy}})
	h.ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, 12).Update(pyjson.New("planned_at", clock.ISO(testsupport.Day)))
		return nil
	}, "state")
	planned, result := h.night(workFake(map[string][]runner.Handler{
		"build":      difficultyOne(workBuilder(map[string]string{"src/game.txt": "v2\n"})),
		"self_check": difficultyOne(workReviewer(workApprove))}))
	assertEqual(t, []any{planned.Get("provider"), result.Get("status")}, []any{"lite", "built"})
	pull := h.gh.ListPulls("open", "bot/issue-12", 200)[0]
	pr, _ := pyjson.AsInt(pull.Get("number"))
	assertFalse(t, pull.Truthy("draft"))
	assertEqual(t, h.gh.LabelNames(pr), map[string]bool{config.LabelPR: true, config.LabelCross: true,
		difficultyEasy: true})
	assertEqual(t, h.gh.AutoMerge, map[string]string{})
	votes := h.ctx.Store.Load().Obj("items").Obj(strconv.Itoa(pr)).Obj("votes")
	assertEqual(t, []any{votes.Get("builder"), votes.Get("approvals")}, []any{"lite", []any{}})
	said := h.gh.BotComments(12)
	comment := said[len(said)-1]
	assertIn(t, "It checked itself 1 time(s), the last one clean; that is not a review.", comment)
	assertIn(t, "No model has reviewed it yet", comment)
	assertIn(t, "No subscription that could give that review is set up", comment)
}

// ---------------------------------------------------------------------------- ConfigTests

func TestDifficulty_MaxSelfCheckRounds(t *testing.T) {
	assertEqual(t, testsupport.MakeConfig().MaxSelfCheckRounds, 3)
	assertEqual(t, testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("max_self_check_rounds", 5)}).MaxSelfCheckRounds, 5)
	assertRaises[*herr.ConfigError](t, func() {
		testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("max_self_check_rounds", 0)})
	})
}

func TestDifficulty_StatusAndProvidersShowTiersAndSelfCheck(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	text := status.Report(difficultyCtxFor(gh, difficultyCtxOpts{Machine: testsupport.AllMachine}))
	assertIn(t, "`lite` (codex: `lite-1` weak, self-check;", text)
	assertIn(t, "`claude-3` (claude: `opus` strong, `sonnet` medium;", text)
	// `harness providers` (and `window`, the same command), as bot selftest runs it.
	out, restore := capture(t, &cli.Stdout)
	assertEqual(t, cli.CmdProviders(testsupport.MakeConfig(testsupport.Config{
		Env: providersSecrets(providers.SecretNames...), Machine: testsupport.AllMachine}),
		&cli.Args{Offline: true}), 0)
	restore()
	printed := out.String()
	assertIn(t, "lite-1 weak self-check", printed)
	assertIn(t, "weak tier, tried in this order:", printed)
	assertIn(t, "lite-1 (self-check)", printed)
}

func TestDifficulty_PeekShowsTheDifficultyAndEachRolesTier(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := difficultyCtxFor(gh)
	difficultyQueue(gh, ctx, 3, false, difficultyEasy)
	look := plan.Peek_(ctx, false, nil, "auto")
	assertEqual(t, look.Reason, "#3 (easy) is queued to build, for `muse`: plan on "+
		"`muse` (muse, `muse-spark-1.3-contributor`, medium); build on `muse` "+
		"(muse, `muse-spark-1.3-contributor`, medium); review on `muse` (muse, "+
		"`muse-spark-1.3-contributor`, medium)")
}
