"""The codex, gemini and muse backends, each driven against a fake executable."""

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
from harness.runner import CodexCli, GeminiCli, MuseCli, RunRequest

from tests.support import ROOT
from tests.test_logins import CODEX_AUTH, GEMINI_CREDS, MUSE_AUTH

#: Every provider secret is in this process's environment; a child must see only its own login.
OTHER_SECRETS = {"CLAUDE_CODE_OAUTH_TOKEN": "claude-token-value", "CODEX_AUTH_JSON": "{\"x\": 1}",
                 "GEMINI_OAUTH_CREDS": "{\"y\": 2}", "MUSE_AUTH": "muse-secret-value",
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
    sessions = os.path.join(os.environ["CODEX_HOME"], "sessions", "2026", "10", "02")
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

FAKE_GEMINI = PRELUDE + textwrap.dedent('''\
    prompt = sys.stdin.read()
    dump(prompt)
    if mode == "auth":
        print("Manual authorization is required but the current session is non-interactive.",
              file=sys.stderr)
        sys.exit(41)
    emit({{"type": "init", "session_id": "s1", "model": "gemini-3-pro-preview"}})
    emit({{"type": "message", "role": "assistant", "content": "Let me look.", "delta": True}})
    emit({{"type": "tool_use", "tool_name": "run_shell_command", "tool_id": "1",
          "parameters": {{"command": "pnpm test"}}}})
    emit({{"type": "tool_result", "tool_id": "1", "status": "success", "output": "ok"}})
    if mode == "quota":
        emit({{"type": "result", "status": "error", "error": {{"type": "TerminalQuotaError",
              "message": "You have exhausted your daily quota on this model."}}}})
        sys.exit(1)
    if mode == "throttle":
        emit({{"type": "result", "status": "error", "error": {{"type": "RateLimit",
              "message": "Quota exceeded for requests per minute (RESOURCE_EXHAUSTED)."}}}})
        sys.exit(1)
    emit({{"type": "message", "role": "assistant", "content": "<!-- review: {{\\"verdict\\": ",
          "delta": True}})
    emit({{"type": "message", "role": "assistant", "content": "\\"approve\\", \\"findings\\": []}} -->",
          "delta": True}})
    emit({{"type": "result", "status": "success", "stats": {{"total_tokens": 99}}}})
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
        login = logins.prepare(self.pool.get("gpt"), json.dumps(CODEX_AUTH), "", self.tmp / "home")
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

    def test_a_used_refresh_token_is_infrastructure(self):
        result = self.run_fake(self.backend(), "auth")
        self.assertTrue(result.infra)
        self.assertFalse(result.rate_limited)


class GeminiTests(Base):
    fake, name = FAKE_GEMINI, "gemini"

    def backend(self):
        login = logins.prepare(self.pool.get("gemini"), json.dumps(GEMINI_CREDS), "",
                               self.tmp / "home")
        return GeminiCli(str(self.bin), login)

    def test_the_answer_is_what_comes_after_the_last_tool(self):
        backend = self.backend()
        result = self.run_fake(backend, read_only=True)
        self.assertTrue(result.ok, result.error)
        self.assertEqual(result.text, '<!-- review: {"verdict": "approve", "findings": []} -->')
        argv = self.seen()["argv"]
        self.assertIn("--skip-trust", argv)
        self.assertEqual(argv[argv.index("--approval-mode") + 1], "yolo")
        policy = Path(argv[argv.index("--policy") + 1]).read_text()
        for tool in ("web_fetch", "google_web_search", "write_file", "replace"):
            self.assertIn(f'toolName = "{tool}"', policy)
        settings = json.loads((Path(self.seen()["env"]["GEMINI_CLI_HOME"]) / ".gemini" /
                               "settings.json").read_text())
        self.assertEqual(settings["model"]["maxSessionTurns"], 42)
        self.assertEqual(settings["security"]["auth"]["selectedType"], "oauth-personal")
        self.assert_only_its_own_login()
        trail = backend.trail(self.tmp / "t" / "01.jsonl")
        self.assertIn("run_shell_command", trail)

    def test_a_missing_login_is_infrastructure(self):
        result = self.run_fake(self.backend(), "auth")
        self.assertEqual(result.exit_code, 41)
        self.assertTrue(result.infra)

    def test_a_spent_daily_quota_parks_it_until_midnight_pacific(self):
        result = self.run_fake(self.backend(), "quota")
        self.assertTrue(result.rate_limited)
        self.assertTrue(result.reset_at.endswith(":05:00Z"))

    def test_a_per_minute_throttle_parks_it_only_briefly(self):
        result = self.run_fake(self.backend(), "throttle")
        self.assertTrue(result.rate_limited)
        self.assertEqual(result.reset_at, "+PT60M")


class MuseTests(Base):
    fake, name = FAKE_MUSE, "muse"

    def backend(self, secret=None):
        login = logins.prepare(self.pool.get("muse"), secret or json.dumps(MUSE_AUTH), "",
                               self.tmp / "home")
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


if __name__ == "__main__":
    unittest.main()
