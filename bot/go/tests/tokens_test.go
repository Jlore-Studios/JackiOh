package tests

// bot/tests/test_tokens.py: every model call's token counts and USD cost, from each CLI to the
// statistics issue (#695).

import (
	"fmt"
	"path/filepath"
	"strings"
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

// tokensBlock is stats.block(ctx, facts) with its defaults.
func tokensBlock(ctx *hctx.Context, facts *stats.Facts) string {
	return stats.Block(ctx, facts, stats.MergesShown, []string{}, "")
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
	body := tokensBlock(ctx, stats.Collect(ctx))
	assertIn(t, "| Model tokens (in / out / cached) | 1,000 / 50 / 200 |", body)
	assertIn(t, "| Model cost (USD) | $0.04 |", body)
	assertIn(t, "| `lite` | 1,000 | 50 | 200 | $0.04 |", body)
}

// tokensRun is test_stats.RUN.
const tokensRun = "https://github.com/Jlore-Studios/JackiOh/actions/runs"

// tokensSay is test_stats.say: a comment of the bot's at `at`.
func tokensSay(gh *fakes.FakeGitHub, number int, body, at string) {
	gh.AddComment(number, body, fakes.Comment{User: fakes.Bot.Copy(), Association: "COLLABORATOR", CreatedAt: at})
}

// tokensMergedPull is test_stats.merged_pull (body "" is None: "Closes #issue").
func tokensMergedPull(gh *fakes.FakeGitHub, number, issue int, opened, merged string, additions, deletions,
	files, commits int, body string) *pyjson.Object {
	if body == "" {
		body = fmt.Sprintf("Closes #%d", issue)
	}
	pull := gh.AddPull(number, fmt.Sprintf("bot/issue-%d", issue), fakes.Pull{
		Title: fmt.Sprintf("Patch v0.2.Y: work for #%d", issue), Body: body, User: fakes.Bot.Copy()})
	pull.Update(pyjson.New("created_at", opened, "merged_at", merged, "state", "closed", "additions", additions,
		"deletions", deletions, "changed_files", files, "commits", commits))
	return pull
}

// tokensStatsWorld is test_stats.world(): a week of the bot's work, as test_stats builds it.
func tokensStatsWorld() (*fakes.FakeGitHub, *hctx.Context) {
	gh := fakes.NewFakeGitHub()
	for _, number := range []int{3, 4, 5, 6} {
		gh.AddIssue(number, fakes.Issue{Title: fmt.Sprintf("Patch v0.2.Y: item %d", number)})
	}
	// #9, for issue 3, inside the six hours: planned on claude-3, built on lite (its own reviewer
	// claude-2), revised on muse (which reviewed its own revision), and approved again by agy.
	tokensSay(gh, 3, "Planning this now ([run]("+tokensRun+"/12)), on `claude-3` (claude, `opus`, strong) "+
		"(`bot:needs-plan`).", "2026-09-29T11:30:00Z")
	tokensSay(gh, 3, "Planned on `claude-3` (claude, `opus`, strong) ([run]("+tokensRun+"/12)). The plan is in "+
		"this issue's description.", "2026-09-29T11:45:00Z")
	tokensSay(gh, 3, "Starting work on this now ([run]("+tokensRun+"/13)): it is difficulty:easy. I build it on "+
		"`lite` (lite, `lite-1`, weak).", "2026-09-29T11:50:00Z")
	tokensSay(gh, 3, "Opened #9, built on `lite` (lite, `lite-1`, weak). Its adversarial reviewer, "+
		"`claude-2` (claude, `opus`, strong), approved it on round 1 of 10.",
		"2026-09-29T12:00:00Z")
	tokensSay(gh, 9, "Starting a revision now ([run]("+tokensRun+"/14)), because of: CI. I build it on `muse` "+
		"(muse, `muse-spark`, medium).", "2026-09-29T13:50:00Z")
	tokensSay(gh, 9, "Revision pushed ([run]("+tokensRun+"/14)) on `muse` (muse, `muse-spark`, medium); `muse` "+
		"(muse, `muse-spark`, medium) reviewed it adversarially and approved it",
		"2026-09-29T14:00:00Z")
	tokensSay(gh, 9, "Starting a review now ([run]("+tokensRun+"/15)), on `agy` (agy, `gemini-3.1-pro`, "+
		"medium).", "2026-09-29T14:30:00Z")
	tokensSay(gh, 9, "Second review ([run]("+tokensRun+"/15)): `agy` (agy, `gemini-3.1-pro`, medium) approved "+
		"it. Auto-merge is on.", "2026-09-29T14:40:00Z")
	tokensSay(gh, 4, "@MaxGoetzmann Queued #4; I will build it in the next run.", "2026-09-29T16:30:00Z")
	tokensMergedPull(gh, 9, 3, "2026-09-29T12:00:00Z", "2026-09-29T15:00:00Z", 40, 5, 4, 3, "")
	// #10, for issue 4, inside the day: no "Opened" comment named its builder, so it is the last
	// build started on its issue before it was opened (gpt).
	tokensSay(gh, 4, "Starting work on this now ([run]("+tokensRun+"/20)): it is difficulty:medium. I build it "+
		"on `gpt` (codex, `gpt-5.6-terra`, medium).", "2026-09-28T23:00:00Z")
	tokensMergedPull(gh, 10, 4, "2026-09-29T00:00:00Z", "2026-09-29T03:00:00Z", 100, 10, 2, 1, "")
	// #11, for issue 5, inside the week: the only build on its issue started after it was opened,
	// so nobody is recorded as its builder; that build failed.
	tokensSay(gh, 5, "Starting work on this now ([run]("+tokensRun+"/30)): it is difficulty:medium. I build it "+
		"on `claude-1` (claude, `opus`, strong).", "2026-09-26T10:00:00Z")
	tokensSay(gh, 5, "This run failed ([run]("+tokensRun+"/30)): RuntimeError", "2026-09-26T10:30:00Z")
	tokensMergedPull(gh, 11, 5, "2026-09-26T00:00:00Z", "2026-09-26T12:00:00Z", 7, 1, 1, 1, "")
	// #12, for issue 6, only in all time.
	tokensMergedPull(gh, 12, 6, "2026-09-10T00:00:00Z", "2026-09-11T00:00:00Z", 1000, 0, 1, 1, "Fixes #6")
	// An open one, and a person's.
	gh.AddPull(13, "bot/issue-7", fakes.Pull{Title: "Still open", User: fakes.Bot.Copy()}).Update(
		pyjson.New("created_at", "2026-09-29T16:00:00Z"))
	gh.AddPull(20, "someone/else", fakes.Pull{Title: "A person's change"}).Update(pyjson.New(
		"created_at", "2026-09-29T16:00:00Z", "merged_at", "2026-09-29T16:30:00Z", "state", "closed",
		"additions", 5000, "deletions", 0))
	gh.Commits = []*pyjson.Object{pyjson.New("sha", "a", "author", pyjson.New("login", fakes.Bot.Get("login"))),
		pyjson.New("sha", "b", "author", pyjson.New("login", "jgoetzmann"))}
	gh.Runs.Update(pyjson.New(
		"1", pyjson.New("status", "completed", "conclusion", "success", "created_at", "2026-09-29T16:00:00Z"),
		"2", pyjson.New("status", "completed", "conclusion", "failure", "created_at", "2026-09-20T00:00:00Z"),
		"3", pyjson.New("status", "completed", "conclusion", "success")))
	ctx := ts.MakeCtx(gh, ts.Ctx{At: ts.Day})
	ctx.Store.Update(func(s *pyjson.Object) any {
		pyjson.Obj(s.SetDefault("providers", pyjson.New())).Update(pyjson.New(
			"lite", pyjson.New("spent", []any{pyjson.New("at", "2026-09-29T12:00:00Z", "minutes", 120.0),
				pyjson.New("at", "2026-09-20T00:00:00Z", "minutes", 60.0)})))
		return nil
	}, "state")
	return gh, ctx
}

// tokensStatsSection is test_stats.section: one window's folded section of the block.
func tokensStatsSection(body, name string) string {
	start := strings.Index(body, "<details><summary><b>"+name+"</b>")
	if start < 0 {
		panic("ValueError: substring not found")
	}
	end := strings.Index(body[start:], "</details>")
	if end < 0 {
		panic("ValueError: substring not found")
	}
	return body[start : start+end]
}

func TestTokens_AWindowWithoutTokensSaysSo(t *testing.T) {
	_, ctx := tokensStatsWorld()
	body := tokensBlock(ctx, stats.Collect(ctx))
	six := tokensStatsSection(body, "Last 6 hours")
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
