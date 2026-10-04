"""The codex, agy and muse backends, each driven against a fake executable."""

from __future__ import annotations

import json
import os
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest import mock

from harness import logins
from harness.providers import load as load_pool
from harness.runner import AgyCli, CodexCli, DevinCli, MuseCli, RunRequest

from tests.support import ROOT, secret_login
from tests.test_logins import CODEX_AUTH, MUSE_AUTH

#: Every provider secret is in this process's environment; a child must see only its own login.
OTHER_SECRETS = {"CLAUDE_CODE_OAUTH_TOKEN": "claude-token-value", "CODEX_AUTH_JSON": "{\"x\": 1}",
                 "MUSE_AUTH": "muse-secret-value",
                 "BOT_GITHUB_TOKEN": "ghp_" + "x" * 36, "META_API_KEY": "meta-key-value",
                 "HARNESS_PROVIDER_SECRET": "the-one-secret-value"}

PRELUDE = '''#!{python}
import json, os, sys
mode = os.environ.get("FAKE_MODE", "ok")
def dump(prompt):
    with open(os.environ["FAKE_DUMP"], "w") as f:
        json.dump({{"argv": sys.argv[1:], "env": dict(os.environ), "prompt": prompt}}, f)
def emit(obj):
    print(json.dumps(obj), flush=True)
'''

FAKE_CODEX = PRELUDE + textwrap.dedent('''\
    prompt = sys.stdin.read()
    dump(prompt)
    argv = sys.argv[1:]
    last = argv[argv.index("--output-last-message") + 1]
    home = os.environ.get("CODEX_HOME") or os.path.join(os.environ["HOME"], ".codex")
    sessions = os.path.join(home, "sessions", "2026", "10", "02")
    os.makedirs(sessions, exist_ok=True)
    with open(os.path.join(sessions, "rollout-1.jsonl"), "w") as f:
        f.write(json.dumps({{"type": "event_msg", "payload": {{"type": "token_count",
            "rate_limits": {{"primary": {{"used_percent": 42.0, "window_minutes": 300,
                                          "resets_at": 1790710800}},
                             "secondary": {{"used_percent": 100.0 if mode == "limit" else 61.0,
                                            "window_minutes": 10080, "resets_at": 1791190800}}}}}}}}) + "\\n")
    emit({{"type": "thread.started", "thread_id": "t1"}})
    emit({{"type": "item.completed", "item": {{"type": "command_execution", "command": "pnpm test",
          "exit_code": 0}}}})
    if mode == "limit":
        emit({{"type": "turn.failed", "error": {{"message": "You\\u2019ve hit your usage limit. Visit "
              "https://chatgpt.com/settings/usage to purchase more credits or try again at 3:45 PM."}}}})
        sys.exit(1)
    if mode == "auth":
        emit({{"type": "turn.failed", "error": {{"message": "Your access token could not be "
              "refreshed because your refresh token was already used. Please log out and sign in again."}}}})
        sys.exit(1)
    emit({{"type": "item.completed", "item": {{"type": "agent_message", "text": "working on it"}}}})
    open(last, "w").write("<!-- bot: {{\\"status\\": \\"done\\", \\"title\\": \\"T\\"}} -->\\nreport")
    emit({{"type": "turn.completed", "usage": {{"input_tokens": 10, "output_tokens": 5}}}})
''')

FAKE_AGY = PRELUDE + textwrap.dedent('''\
    prompt = sys.stdin.read()
    dump(prompt)
    if mode == "auth":
        print("Error: authentication required; run agy to sign in.", file=sys.stderr)
        sys.exit(1)
    def step(index, kind, state, **fields):
        emit({{"event": "step_update", "step_update": {{"step_index": index, "step_type": kind,
              "state": state, **fields}}}})
    emit({{"event": "init", "init": {{"session_id": "s1", "model": "gemini-3.8-flash-high"}}}})
    step(0, "agent_response", "RUNNING", text_delta="Let me ")
    step(0, "agent_response", "DONE", text_delta="look.")
    step(1, "tool", "DONE", tool_info={{"name": "run_command",
                                         "parameters": {{"CommandLine": "pnpm test"}}}})
    if mode == "quota":
        emit({{"event": "result", "result": {{"status": "ERROR", "num_turns": 2,
              "error": "RESOURCE_EXHAUSTED: You have exhausted your quota on this model."}}}})
        sys.exit(1)
    if mode == "individual":
        emit({{"event": "result", "result": {{"status": "ERROR", "num_turns": 1,
              "error": "Individual quota reached. Please upgrade your subscription to increase "
                       "your limits. Resets in 1h44m44s."}}}})
        sys.exit(1)
    answer = '<!-- review: {{"verdict": "approve", "findings": []}} -->'
    step(2, "agent_response", "DONE", text_delta=answer)
    emit({{"event": "result", "result": {{"status": "SUCCESS", "response": answer, "num_turns": 3,
          "error": "", "usage": {{"input_tokens": 10}}}}}})
''')

FAKE_DEVIN = PRELUDE + textwrap.dedent('''\
    argv = sys.argv[1:]
    prompt = open(argv[argv.index("--prompt-file") + 1]).read()
    dump(prompt)
    if mode == "auth":
        print("Not logged in. Run `devin auth login` to authenticate.", file=sys.stderr)
        sys.exit(1)
    with open(argv[argv.index("--export") + 1], "w") as f:
        json.dump({{"schema_version": 1, "steps": [
            {{"source": "system", "message": "You are Devin"}},
            {{"source": "user", "message": "THE TASK"}},
            {{"source": "agent", "message": "", "tool_calls": [{{"tool_call_id": "1",
              "function_name": "exec", "arguments": {{"command": "pnpm test"}}}}]}},
            {{"source": "agent", "message": "", "tool_calls": [{{"tool_call_id": "2",
              "function_name": "read_file", "arguments": {{"path": "src/game.txt"}}}}]}},
            {{"source": "agent", "message": "All green.", "tool_calls": []}}]}}, f)
    if mode == "limit":
        print("Error: usage limit reached for swe-2-max", file=sys.stderr)
        sys.exit(1)
    print("\\x1b[1mWelcome to Devin CLI!\\x1b[0m")
    print()
    print("<!-- bot: {{\\"status\\": \\"done\\", \\"title\\": \\"D\\"}} -->")
    print("report")
''')

FAKE_MUSE = PRELUDE + textwrap.dedent('''\
    argv = sys.argv[1:]
    prompt = open(argv[argv.index("--prompt-file") + 1]).read()
    dump(prompt)
    print("step 1: reading", file=sys.stderr)
    if mode == "limit":
        print("Error: usage limit reached. Wait for usage to reset at 18:00.", file=sys.stderr)
        sys.exit(1)
    print("<!-- bot: {{\\"status\\": \\"done\\", \\"title\\": \\"M\\"}} -->")
    print("report")
''')


class Base(unittest.TestCase):
    fake = ""
    name = ""

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.bin = self.tmp / self.name
        self.bin.write_text(self.fake.format(python=sys.executable))
        self.bin.chmod(0o755)
        self.dump = self.tmp / "dump.json"
        self.pool = load_pool(ROOT)

    def request(self, read_only=False, max_turns=42) -> RunRequest:
        return RunRequest(role="review" if read_only else "build", prompt="THE TASK", cwd=self.tmp,
                          system_append="SYSTEM TEXT", allowed_tools=(), disallowed_tools=(),
                          max_turns=max_turns, timeout_s=60, model="the-model", effort="high",
                          transcript=self.tmp / "t" / "01.jsonl", read_only=read_only,
                          extra_dirs=("/repo/.git",))

    def run_fake(self, backend, mode="ok", read_only=False):
        with mock.patch.dict(os.environ, {**OTHER_SECRETS, "FAKE_MODE": mode,
                                          "FAKE_DUMP": str(self.dump)}):
            return backend.run(self.request(read_only))

    def seen(self) -> dict:
        return json.loads(self.dump.read_text())

    def assert_only_its_own_login(self, keep: tuple[str, ...] = ()):
        env = self.seen()["env"]
        for key in OTHER_SECRETS:
            if key not in keep:
                self.assertNotIn(key, env)


class CodexTests(Base):
    fake, name = FAKE_CODEX, "codex"

    def backend(self):
        login = logins.prepare(secret_login(self.pool.get("gpt"), "CODEX_AUTH_JSON"),
                               json.dumps(CODEX_AUTH), "", self.tmp / "home")
        return CodexCli(str(self.bin), login)

    def test_success_usage_and_argv(self):
        result = self.run_fake(self.backend())
        self.assertTrue(result.ok, result.error)
        self.assertTrue(result.text.startswith("<!-- bot:"))
        self.assertEqual(result.usage["five_hour"]["utilization"], 0.42)
        self.assertEqual(result.usage["seven_day"]["resets_at"], "2026-10-05T09:00:00Z")
        seen = self.seen()
        argv = " ".join(seen["argv"])
        for part in ("exec --json", "--sandbox workspace-write",
                     "sandbox_workspace_write.network_access=false", "--model the-model",
                     'model_reasoning_effort="high"', "--add-dir /repo/.git"):
            self.assertIn(part, argv)
        self.assertEqual(seen["argv"][-1], "-")
        self.assertTrue(seen["prompt"].startswith("SYSTEM TEXT"))
        self.assertIn("You are running in the codex CLI", seen["prompt"])
        self.assertTrue(seen["prompt"].endswith("THE TASK"))
        self.assert_only_its_own_login()
        self.assertTrue(seen["env"]["CODEX_HOME"].endswith("codex"))
        transcript = (self.tmp / "t" / "01.jsonl").read_text()
        self.assertIn("thread.started", transcript)
        self.assertIn("ran `pnpm test`", self.backend().trail(self.tmp / "t" / "01.jsonl"))

    def test_a_usage_limit_parks_it_until_the_full_window_resets(self):
        result = self.run_fake(self.backend(), "limit")
        self.assertFalse(result.ok)
        self.assertTrue(result.rate_limited)
        self.assertEqual(result.reset_at, "2026-10-05T09:00:00Z")

    def test_a_machine_login_uses_the_clis_own_home(self):
        login = logins.prepare(self.pool.get("gpt"), "", "", self.tmp / "unused")
        machine_home = self.tmp / "machine-home"
        with mock.patch.dict(os.environ, {"HOME": str(machine_home)}):
            result = self.run_fake(CodexCli(str(self.bin), login))
        self.assertTrue(result.ok, result.error)
        self.assertNotIn("CODEX_HOME", self.seen()["env"])
        self.assertEqual(self.seen()["env"]["HOME"], str(machine_home))
        self.assertEqual(result.usage["five_hour"]["utilization"], 0.42)  # read from ~/.codex
        self.assert_only_its_own_login()

    def test_a_used_refresh_token_is_infrastructure(self):
        result = self.run_fake(self.backend(), "auth")
        self.assertTrue(result.infra)
        self.assertFalse(result.rate_limited)


class AgyTests(Base):
    fake, name = FAKE_AGY, "agy"

    def backend(self):
        return AgyCli(str(self.bin), logins.prepare(self.pool.get("agy"), "", "", self.tmp / "x"))

    def test_the_answer_success_and_argv(self):
        backend = self.backend()
        result = self.run_fake(backend, read_only=True)
        self.assertTrue(result.ok, result.error)
        self.assertEqual(result.text, '<!-- review: {"verdict": "approve", "findings": []} -->')
        self.assertEqual(result.turns, 3)
        seen = self.seen()
        argv = seen["argv"]
        for flag in ("--dangerously-skip-permissions", "--disable-slash-commands"):
            self.assertIn(flag, argv)
        self.assertEqual(argv[argv.index("--output-format") + 1], "stream-json")
        self.assertEqual(argv[argv.index("--model") + 1], "the-model")
        self.assertEqual(argv[argv.index("--effort") + 1], "high")
        self.assertEqual(argv[argv.index("--add-dir") + 1], "/repo/.git")
        self.assertTrue(seen["prompt"].startswith("SYSTEM TEXT"))
        self.assertTrue(seen["prompt"].endswith("THE TASK"))
        self.assert_only_its_own_login()
        trail = backend.trail(self.tmp / "t" / "01.jsonl")
        self.assertIn("said: Let me look.", trail)
        self.assertIn("run_command: pnpm test", trail)

    def test_a_missing_login_is_infrastructure(self):
        result = self.run_fake(self.backend(), "auth")
        self.assertFalse(result.ok)
        self.assertTrue(result.infra)
        self.assertIn("authentication required", result.error)

    def test_a_spent_quota_parks_it(self):
        result = self.run_fake(self.backend(), "quota")
        self.assertTrue(result.rate_limited)
        self.assertEqual(result.reset_at, "+PT60M")
        self.assertIn("exhausted your quota", result.error)

    def test_its_plans_individual_quota_parks_it_until_the_reset_it_names(self):
        """agy's answer once the plan's allowance is spent. Unread, every run went on to the
        checks and a review that failed the same way, and the next run started minutes later."""
        result = self.run_fake(self.backend(), "individual")
        self.assertTrue(result.rate_limited)
        self.assertFalse(result.infra)
        self.assertEqual(result.reset_at, "+PT106M")  # 1h44m44s, rounded up, plus a minute

    def test_a_named_reset_is_how_long_it_stays_parked(self):
        from datetime import datetime, timedelta, timezone
        from harness import providers as providers_mod
        from harness.runner import park_for
        self.assertEqual(park_for("Resets in 1h44m44s."), "+PT106M")
        self.assertEqual(park_for("Your limit resets in 3h"), "+PT181M")
        self.assertEqual(park_for("resets in 12m"), "+PT13M")
        self.assertEqual(park_for("Usage limit reached."), "+PT60M")
        at = datetime(2026, 10, 3, 23, 33, tzinfo=timezone.utc)
        state = {"providers": {}}
        providers_mod.note_usage(state, "agy", None, "+PT106M", at)
        self.assertEqual(state["providers"]["agy"]["refused_until"], "2026-10-04T01:19:00Z")
        providers_mod.note_usage(state, "agy", None, "+PT1H30M", at)
        self.assertEqual(state["providers"]["agy"]["refused_until"], "2026-10-04T01:03:00Z")
        self.assertEqual(providers_mod._duration("+PT2H"), timedelta(hours=2))
        # Codex names days: gpt's weekly limit holds it until the week resets, not an hour.
        self.assertEqual(park_for("You've hit your usage limit. Upgrade to Pro or try again in "
                                  "5 days 2 hours 22 minutes."), "+PT7343M")

    def test_a_refusal_that_names_no_reset_waits_for_the_nearly_full_window(self):
        """gpt's refusal named no time, so it was retried every hour although its last reading
        had its week at 89%, resetting days later."""
        from datetime import datetime, timezone
        from harness import providers as providers_mod
        from harness.runner import DEFAULT_PARK
        at = datetime(2026, 10, 3, 23, 47, tzinfo=timezone.utc)
        week = {"utilization": 0.89, "resets_at": "2026-10-09T23:47:06Z"}
        state = {"providers": {"gpt": {"usage": {"seven_day": week}}}}
        providers_mod.note_usage(state, "gpt", None, DEFAULT_PARK, at)
        self.assertEqual(state["providers"]["gpt"]["refused_until"], "2026-10-09T23:47:06Z")
        # A window with room left, or a reset the refusal named, keeps the shorter park.
        state = {"providers": {"gpt": {"usage": {"seven_day": {**week, "utilization": 0.4}}}}}
        providers_mod.note_usage(state, "gpt", None, DEFAULT_PARK, at)
        self.assertEqual(state["providers"]["gpt"]["refused_until"], "2026-10-04T00:47:00Z")
        state = {"providers": {"gpt": {"usage": {"seven_day": week}}}}
        providers_mod.note_usage(state, "gpt", None, "+PT90M", at)
        self.assertEqual(state["providers"]["gpt"]["refused_until"], "2026-10-04T01:17:00Z")


class MuseTests(Base):
    fake, name = FAKE_MUSE, "muse"

    def backend(self, secret=None):
        login = logins.prepare(secret_login(self.pool.get("muse"), "MUSE_AUTH"),
                               secret or json.dumps(MUSE_AUTH), "", self.tmp / "home")
        return MuseCli(str(self.bin), login)

    def test_the_prompt_goes_in_a_file_and_the_answer_comes_on_stdout(self):
        result = self.run_fake(self.backend())
        self.assertTrue(result.ok, result.error)
        self.assertTrue(result.text.startswith('<!-- bot: {"status": "done"'))
        seen = self.seen()
        self.assertIn("--yolo", seen["argv"])
        self.assertIn("--disable-web-tools", seen["argv"])
        self.assertEqual(seen["argv"][seen["argv"].index("--max-model-steps") + 1], "42")
        self.assertIn("THE TASK", seen["prompt"])
        self.assert_only_its_own_login()
        self.assertIn("muse", seen["env"]["XDG_CONFIG_HOME"])
        transcript = (self.tmp / "t" / "01.jsonl").read_text()
        self.assertIn("step 1: reading", transcript)

    def test_a_usage_limit_is_a_rate_limit(self):
        result = self.run_fake(self.backend(), "limit")
        self.assertTrue(result.rate_limited)
        self.assertIn("usage limit reached", result.error)

    def test_an_api_key_secret_is_handed_over_as_one(self):
        self.run_fake(self.backend("meta-key-" + "k" * 30))
        self.assertEqual(self.seen()["env"]["META_API_KEY"], "meta-key-" + "k" * 30)


class DevinTests(Base):
    fake, name = FAKE_DEVIN, "devin"

    def backend(self):
        login = logins.prepare(self.pool.get("devin"), "", "", self.tmp / "home")
        return DevinCli(str(self.bin), login)

    def test_print_mode_auto_approved_with_the_prompt_in_a_file(self):
        backend = self.backend()
        result = self.run_fake(backend)
        self.assertTrue(result.ok, result.error)
        self.assertTrue(result.text.startswith('<!-- bot: {"status": "done"'))
        seen = self.seen()
        argv = seen["argv"]
        self.assertEqual(argv[0], "-p")
        self.assertEqual(argv[argv.index("--model") + 1], "the-model")
        self.assertEqual(argv[argv.index("--permission-mode") + 1], "dangerous")
        self.assertEqual(argv[argv.index("--respect-workspace-trust") + 1], "false")
        self.assertTrue(seen["prompt"].startswith("SYSTEM TEXT"))
        self.assertTrue(seen["prompt"].endswith("THE TASK"))
        self.assert_only_its_own_login()
        self.assertNotIn("Welcome", result.text)  # the banner and its colour codes are gone
        transcript = (self.tmp / "t" / "01.jsonl").read_text()
        self.assertIn("report", transcript)
        self.assertNotIn("You are Devin", transcript)  # only what its agent did
        self.assertFalse((self.tmp / "t" / "01.export").exists())
        trail = backend.trail(self.tmp / "t" / "01.jsonl")
        for line in ("ran `pnpm test`", 'read_file: {"path": "src/game.txt"}', "said: All green."):
            self.assertIn(line, trail)

    def test_a_missing_login_is_infrastructure(self):
        result = self.run_fake(self.backend(), "auth")
        self.assertFalse(result.ok)
        self.assertTrue(result.infra)

    def test_a_usage_limit_parks_it(self):
        result = self.run_fake(self.backend(), "limit")
        self.assertTrue(result.rate_limited)
        self.assertEqual(result.reset_at, "+PT60M")


if __name__ == "__main__":
    unittest.main()
