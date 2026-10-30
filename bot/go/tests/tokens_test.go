package tests

// bot/tests/test_tokens.py: every model call's token counts and USD cost, from each CLI to the
// statistics issue (#695).

import (
	"path/filepath"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/journal"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/logins"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stats"
	ts "github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// tokensCounts is {"input": i, "output": o, "cache_read": r, "cache_write": w}.
func tokensCounts(input, output, cacheRead, cacheWrite int) *pyjson.Object {
	return pyjson.New("input", input, "output", output, "cache_read", cacheRead, "cache_write", cacheWrite)
}

// ---------------------------------------------------------------------------- MappingTests

func TestTokens_AClaudeResultBlockMapsAllFourCounts(t *testing.T) {
	assertEqual(t, runner.TokensFromBlock(pyjson.New("input_tokens", 100, "output_tokens", 20,
		"cache_creation_input_tokens", 30, "cache_read_input_tokens", 400)),
		pyjson.New("input", 100, "output", 20, "cache_read", 400, "cache_write", 30))
}

func TestTokens_APartialBlockMapsWhatItNames(t *testing.T) {
	assertEqual(t, runner.TokensFromBlock(pyjson.New("input_tokens", 10, "output_tokens", 5)),
		tokensCounts(10, 5, 0, 0))
	assertEqual(t, runner.TokensFromBlock(pyjson.New("input_tokens", 10)), tokensCounts(10, 0, 0, 0))
}

func TestTokens_ABlockWithNoCountsIsNone(t *testing.T) {
	assertTrue(t, runner.TokensFromBlock(pyjson.New()) == nil)
	assertTrue(t, runner.TokensFromBlock(nil) == nil)
	assertTrue(t, runner.TokensFromBlock(pyjson.New("status", "SUCCESS")) == nil)
}

func TestTokens_CostComesFromTotalCostUSD(t *testing.T) {
	cost := runner.CostFromResult(pyjson.New("total_cost_usd", 0.0123))
	assertTrue(t, cost != nil)
	assertEqual(t, *cost, 0.0123)
	assertTrue(t, runner.CostFromResult(pyjson.New()) == nil)
	assertTrue(t, runner.CostFromResult(nil) == nil)
}

// tokensFakeClaude is FAKE_CLAUDE_TOKENS, played by this test binary (backends_test.go's init).
func tokensFakeClaude() int {
	backendsStdin()
	backendsEmit(pyjson.New("type", "system", "subtype", "init"))
	backendsEmit(pyjson.New("type", "rate_limit_event", "rate_limit_info", pyjson.New("status", "allowed",
		"unifiedWindows", pyjson.New("five_hour", pyjson.New("utilization", 0.1, "resetsAt", 1790710800)))))
	backendsEmit(pyjson.New("type", "result", "is_error", false, "result", "done", "num_turns", 2,
		"total_cost_usd", 0.042,
		"usage", pyjson.New("input_tokens", 1000, "output_tokens", 50,
			"cache_creation_input_tokens", 200, "cache_read_input_tokens", 5000)))
	return 0
}

// ---------------------------------------------------------------------------- ClaudeTokenTests

func TestTokens_AClaudeCallReportsItsTokensAndCost(t *testing.T) {
	w := newBackendsWorld(t, "claude-tokens", "claude")
	result := w.runFake(runner.NewClaudeCli(w.bin, nil, nil), "ok", false)
	backendsOK(t, result)
	assertEqual(t, result.Tokens, tokensCounts(1000, 50, 5000, 200))
	assertTrue(t, result.CostUSD != nil)
	assertAlmostEqual(t, *result.CostUSD, 0.042)
}

// ---------------------------------------------------------------------------- CodexTokenTests

func TestTokens_ACodexCallReportsItsTurnTokens(t *testing.T) {
	w := newBackendsWorld(t, "codex", "codex")
	login := logins.Prepare(ts.SecretLogin(w.pool.Get("gpt"), "CODEX_AUTH_JSON"),
		pyjson.Dumps(loginsCodexAuth()), "", filepath.Join(w.tmp, "home"))
	result := w.runFake(runner.NewCodexCli(w.bin, login, nil), "ok", false)
	backendsOK(t, result)
	assertEqual(t, result.Tokens, tokensCounts(10, 5, 0, 0))
	assertTrue(t, result.CostUSD == nil)
}

// ---------------------------------------------------------------------------- AgyTokenTests

func TestTokens_AnAgyCallReportsItsResultTokens(t *testing.T) {
	w := newBackendsWorld(t, "agy", "agy")
	result := w.runFake(runner.NewAgyCli(w.bin, logins.Prepare(w.pool.Get("agy"), "", "",
		filepath.Join(w.tmp, "x")), nil), "ok", true)
	backendsOK(t, result)
	assertEqual(t, result.Tokens, tokensCounts(10, 0, 0, 0))
}

// ---------------------------------------------------------------------------- RecordingTests

func TestTokens_NoteUsageKeepsTokensBesideMinutes(t *testing.T) {
	st := pyjson.New()
	cost := 0.03
	providers.NoteUsage(st, "claude-1", nil, "", ts.Day, 12.0, tokensCounts(100, 20, 5, 6), &cost)
	entry := st.Obj("providers").Obj("claude-1")
	at := ts.Day.Format("2006-01-02T15:04:05Z")
	assertEqual(t, entry.Get("spent"), []any{pyjson.New("at", at, "minutes", 12.0)})
	assertEqual(t, entry.Get("tokens_spent"), []any{pyjson.New("at", at, "input", 100, "output", 20,
		"cache", 11, "cost", 0.03)})
}

func TestTokens_NoteUsageWithoutTokensWritesNoTokenRecord(t *testing.T) {
	st := pyjson.New()
	providers.NoteUsage(st, "claude-1", nil, "", ts.Day, 12.0, nil, nil)
	assertNotIn(t, "tokens_spent", st.Obj("providers").Obj("claude-1"))
}

// tokensSpentLite is RecordingTests' world: lite's hour of model time, its tokens and its cost.
func tokensSpentLite() *hctx.Context {
	ctx := ts.MakeCtx(fakes.NewFakeGitHub(), ts.Ctx{At: ts.Day})
	ctx.Store.Update(func(s *pyjson.Object) any {
		pyjson.Obj(s.SetDefault("providers", pyjson.New())).Update(pyjson.New(
			"lite", pyjson.New("spent", []any{pyjson.New("at", "2026-09-29T12:00:00Z", "minutes", 60.0)},
				"tokens_spent", []any{pyjson.New("at", "2026-09-29T12:00:00Z", "input", 1000,
					"output", 50, "cache", 200, "cost", 0.04)})))
		return nil
	}, "state")
	return ctx
}

func TestTokens_AWindowSumsTokensAndCost(t *testing.T) {
	ctx := tokensSpentLite()
	facts := stats.Collect(ctx)
	window := stats.Window_("Last 6 hours", nil, ts.Day, facts)
	assertEqual(t, window.Tokens.Get("lite"), [3]int{1000, 50, 200})
	cost, _ := pyjson.AsFloat(window.Cost.Get("lite"))
	assertAlmostEqual(t, cost, 0.04)
}

func TestTokens_TheIssueShowsTokensAndCost(t *testing.T) {
	ctx := tokensSpentLite()
	body := statsBlock(ctx, stats.Collect(ctx))
	assertIn(t, "| Model tokens (in / out / cached) | 1,000 / 50 / 200 |", body)
	assertIn(t, "| Model cost (USD) | $0.04 |", body)
	assertIn(t, "| `lite` | 1,000 | 50 | 200 | $0.04 |", body)
}

func TestTokens_AWindowWithoutTokensSaysSo(t *testing.T) {
	_, ctx := statsWorld()
	body := statsBlock(ctx, stats.Collect(ctx))
	six := statsSection(t, body, "Last 6 hours")
	assertIn(t, "**Tokens and cost**", six)
	assertIn(t, "No call reported token counts in this window.", six)
}

func TestTokens_JournalRowsAndSectionsCarryTokens(t *testing.T) {
	rows := journal.StepRows([]any{pyjson.New("at", "2026-09-29T12:00:00Z", "step", "call", "role", "build",
		"model", "opus", "why", "build", "minutes", 3.0, "tokens", tokensCounts(100, 20, 0, 0),
		"cost_usd", 0.05, "outcome", "ok")})
	assertIn(t, "3.0 min, 120 tok, $0.05", rows[0])
	text := journal.Section(pyjson.New("status", "done", "minutes", 3.0, "model_calls", 1,
		"tokens", tokensCounts(100, 20, 7, 3), "cost_usd", 0.05),
		"claude-1", "`opus`", "build", 3, "2026-09-29T12:00:00Z", "")
	assertIn(t, "Tokens: 100 in / 20 out / 10 cached, $0.05.", text)
}
