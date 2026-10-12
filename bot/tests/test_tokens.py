"""Every model call's token counts and USD cost, from each CLI to the statistics issue
(#695)."""

from __future__ import annotations

import json
import sys
import unittest

from harness import journal, providers as providers_mod, stats
from harness.runner import AgyCli, ClaudeCli, CodexCli, cost_from_result, tokens_from_block

from tests.fakes import FakeGitHub
from tests.support import DAY, make_ctx
from tests.test_backends import FAKE_AGY, FAKE_CODEX, Base


class MappingTests(unittest.TestCase):
    def test_a_claude_result_block_maps_all_four_counts(self):
        self.assertEqual(tokens_from_block({"input_tokens": 100, "output_tokens": 20,
                                            "cache_creation_input_tokens": 30,
                                            "cache_read_input_tokens": 400}),
                         {"input": 100, "output": 20, "cache_read": 400, "cache_write": 30})

    def test_a_partial_block_maps_what_it_names(self):
        self.assertEqual(tokens_from_block({"input_tokens": 10, "output_tokens": 5}),
                         {"input": 10, "output": 5, "cache_read": 0, "cache_write": 0})
        self.assertEqual(tokens_from_block({"input_tokens": 10}),
                         {"input": 10, "output": 0, "cache_read": 0, "cache_write": 0})

    def test_a_block_with_no_counts_is_none(self):
        self.assertIsNone(tokens_from_block({}))
        self.assertIsNone(tokens_from_block(None))
        self.assertIsNone(tokens_from_block({"status": "SUCCESS"}))

    def test_cost_comes_from_total_cost_usd(self):
        self.assertEqual(cost_from_result({"total_cost_usd": 0.0123}), 0.0123)
        self.assertIsNone(cost_from_result({}))
        self.assertIsNone(cost_from_result(None))


FAKE_CLAUDE_TOKENS = '''#!{python}
import json, sys
sys.stdin.read()
def emit(obj):
    print(json.dumps(obj), flush=True)
emit({{"type": "system", "subtype": "init"}})
emit({{"type": "rate_limit_event", "rate_limit_info": {{"status": "allowed",
      "unifiedWindows": {{"five_hour": {{"utilization": 0.1, "resetsAt": 1790710800}}}}}}}})
emit({{"type": "result", "is_error": False, "result": "done", "num_turns": 2,
      "total_cost_usd": 0.042,
      "usage": {{"input_tokens": 1000, "output_tokens": 50,
                "cache_creation_input_tokens": 200, "cache_read_input_tokens": 5000}}}})
'''


class ClaudeTokenTests(Base):
    fake, name = FAKE_CLAUDE_TOKENS, "claude"

    def test_a_claude_call_reports_its_tokens_and_cost(self):
        result = self.run_fake(ClaudeCli(str(self.bin)))
        self.assertTrue(result.ok, result.error)
        self.assertEqual(result.tokens, {"input": 1000, "output": 50, "cache_read": 5000,
                                         "cache_write": 200})
        self.assertAlmostEqual(result.cost_usd, 0.042)


class CodexTokenTests(Base):
    fake, name = FAKE_CODEX, "codex"

    def test_a_codex_call_reports_its_turn_tokens(self):
        from harness import logins
        from harness.providers import load as load_pool
        from tests.support import ROOT, secret_login
        from tests.test_logins import CODEX_AUTH
        login = logins.prepare(secret_login(self.pool.get("gpt"), "CODEX_AUTH_JSON"),
                               json.dumps(CODEX_AUTH), "", self.tmp / "home")
        result = self.run_fake(CodexCli(str(self.bin), login))
        self.assertTrue(result.ok, result.error)
        self.assertEqual(result.tokens, {"input": 10, "output": 5, "cache_read": 0,
                                         "cache_write": 0})
        self.assertIsNone(result.cost_usd)


class AgyTokenTests(Base):
    fake, name = FAKE_AGY, "agy"

    def test_an_agy_call_reports_its_result_tokens(self):
        from harness import logins
        result = self.run_fake(AgyCli(str(self.bin),
                                      logins.prepare(self.pool.get("agy"), "", "",
                                                     self.tmp / "x")), read_only=True)
        self.assertTrue(result.ok, result.error)
        self.assertEqual(result.tokens, {"input": 10, "output": 0, "cache_read": 0,
                                         "cache_write": 0})


class RecordingTests(unittest.TestCase):
    def test_note_usage_keeps_tokens_beside_minutes(self):
        state: dict = {}
        providers_mod.note_usage(state, "claude-1", None, None, DAY, minutes=12.0,
                                 tokens={"input": 100, "output": 20, "cache_read": 5,
                                         "cache_write": 6}, cost_usd=0.03)
        entry = state["providers"]["claude-1"]
        self.assertEqual(entry["spent"], [{"at": DAY.strftime("%Y-%m-%dT%H:%M:%SZ"),
                                           "minutes": 12.0}])
        self.assertEqual(entry["tokens_spent"],
                         [{"at": DAY.strftime("%Y-%m-%dT%H:%M:%SZ"), "input": 100, "output": 20,
                           "cache": 11, "cost": 0.03}])

    def test_note_usage_without_tokens_writes_no_token_record(self):
        state: dict = {}
        providers_mod.note_usage(state, "claude-1", None, None, DAY, minutes=12.0)
        self.assertNotIn("tokens_spent", state["providers"]["claude-1"])

    def test_a_window_sums_tokens_and_cost(self):
        ctx = make_ctx(FakeGitHub(), at=DAY)
        ctx.store.update(lambda s: s.setdefault("providers", {}).update(
            devin={"spent": [{"at": "2026-09-29T12:00:00Z", "minutes": 60.0}],
                   "tokens_spent": [{"at": "2026-09-29T12:00:00Z", "input": 1000,
                                      "output": 50, "cache": 200, "cost": 0.04}]}))
        facts = stats.collect(ctx)
        window = stats.window("Last 6 hours", None, DAY, facts)
        self.assertEqual(window.tokens["devin"], (1000, 50, 200))
        self.assertAlmostEqual(window.cost["devin"], 0.04)

    def test_the_issue_shows_tokens_and_cost(self):
        ctx = make_ctx(FakeGitHub(), at=DAY)
        ctx.store.update(lambda s: s.setdefault("providers", {}).update(
            devin={"spent": [{"at": "2026-09-29T12:00:00Z", "minutes": 60.0}],
                   "tokens_spent": [{"at": "2026-09-29T12:00:00Z", "input": 1000,
                                      "output": 50, "cache": 200, "cost": 0.04}]}))
        body = stats.block(ctx, stats.collect(ctx))
        self.assertIn("| Model tokens (in / out / cached) | 1,000 / 50 / 200 |", body)
        self.assertIn("| Model cost (USD) | $0.04 |", body)
        self.assertIn("| `devin` | 1,000 | 50 | 200 | $0.04 |", body)

    def test_a_window_without_tokens_says_so(self):
        from tests.test_stats import section, world
        _gh, ctx = world()
        body = stats.block(ctx, stats.collect(ctx))
        six = section(body, "Last 6 hours")
        self.assertIn("**Tokens and cost**", six)
        self.assertIn("No call reported token counts in this window.", six)

    def test_journal_rows_and_sections_carry_tokens(self):
        rows = journal.step_rows([{"at": "2026-09-29T12:00:00Z", "step": "call", "role": "build",
                                   "model": "opus", "why": "build", "minutes": 3.0,
                                   "tokens": {"input": 100, "output": 20, "cache_read": 0,
                                              "cache_write": 0}, "cost_usd": 0.05,
                                   "outcome": "ok"}])
        self.assertIn("3.0 min, 120 tok, $0.05", rows[0])
        text = journal.section({"status": "done", "minutes": 3.0, "model_calls": 1,
                                "tokens": {"input": 100, "output": 20, "cache_read": 7,
                                           "cache_write": 3}, "cost_usd": 0.05},
                               provider="claude-1", models="`opus`", action="build", number=3,
                               at="2026-09-29T12:00:00Z")
        self.assertIn("Tokens: 100 in / 20 out / 10 cached, $0.05.", text)


if __name__ == "__main__":
    unittest.main()
