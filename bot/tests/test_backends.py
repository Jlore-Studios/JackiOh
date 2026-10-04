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
    if sys.argv[1:] == ["-p", "/usage"]:  # its quota table: pool, limit, share left, reset
        week = "4" if mode in ("spent", "slow") else "40"
        for row in (("Gemini Models", "Weekly Limit Remaining", week + "%", "2026-10-09T17:47:52Z"),
                    ("Gemini Models", "Five Hour Limit Remaining", "33%", "2026-10-04T16:18:35Z"),
                    ("Claude and GPT models", "Weekly Limit Remaining", "100%",
                     "2026-10-11T15:04:01Z"),
                    ("Claude and GPT models", "Five Hour Limit Remaining", "100%",
                     "2026-10-04T20:04:01Z")):
            print("\\t".join(row))
        sys.exit(0)
    prompt = sys.stdin.read()
    dump(prompt)
    if mode == "slow":
        import time
        time.sleep(30)
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
    if argv[:1] != ["exec"]:  # the TUI, in a pseudo-terminal: what `/usage` is read from
        import time, tty
        workspace = argv[argv.index("--workspace") + 1]
        def draw(text):
            os.write(1, text.encode("utf-8"))
        def answer():
            got = b""
            while not got.endswith(b"\\r"):
                got += os.read(0, 1)
            with open(os.path.join(workspace, "typed.log"), "ab") as log:
                log.write(got.strip() + b"\\n")
            return got.strip()
        tty.setraw(0)
        draw("\\x1b[6n")
        where = b""
        while not where.endswith(b"R"):
            where += os.read(0, 1)
        if not os.path.exists(os.path.join(workspace, ".trusted")):
            draw("Do you trust this workspace?\\r\\n> 1 Trust and continue\\r\\n  2 Quit\\r\\n")
            if answer() != b"1":
                sys.exit(3)
            open(os.path.join(workspace, ".trusted"), "w").close()
        draw("\\x1b7 Muse Code 1.4.2\\x1b8\\r\\n\\u276f \\r\\n")
        if answer() == b"/usage":
            week = "97" if mode == "spent" else "35"
            draw("\\u2502 Session usage \\u2502\\r\\n"
                 "\\u2502 Subscription \\u00b7 Muse Code High Usage \\u2502\\r\\n"
                 "\\u2502 Current 5% used \\u00b7 Resets at 6:42 PM \\u2502\\r\\n"
                 "\\u2502 Weekly " + week + "% used \\u00b7 Resets Oct 5 at 12:00 AM \\u2502\\r\\n")
        time.sleep(30)
        sys.exit(0)
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
        self.assertIsNone(result.usage)  # no reading is tried with a login that failed

    def gemini(self, mode="ok", usage_stop=None):
        import dataclasses
        request = dataclasses.replace(self.request(), model="gemini-3.8-flash-high",
                                      usage_stop=usage_stop)
        with mock.patch.dict(os.environ, {**OTHER_SECRETS, "FAKE_MODE": mode,
                                          "FAKE_DUMP": str(self.dump)}):
            return self.backend().run(request)

    def test_each_call_ends_with_a_reading_of_its_usage(self):
        """agy's stream carries no usage, so its own `/usage` is read after the call: the
        Gemini pool's row for a Gemini model."""
        result = self.gemini()
        self.assertTrue(result.ok, result.error)
        self.assertEqual(result.usage["seven_day"],
                         {"utilization": 0.6, "resets_at": "2026-10-09T17:47:52Z"})
        self.assertEqual(result.usage["five_hour"]["utilization"], 0.67)
        self.assertEqual(self.seen()["prompt"].endswith("THE TASK"), True)  # the call's, not /usage's

    def test_a_call_past_its_cap_stops_at_the_next_reading(self):
        """The call streams nothing, so the watcher reads `/usage` every POLL_SECONDS and stops
        the call once `usage_stop` says so."""
        from harness import runner as runner_mod
        stop = lambda usage: ("week at 96%" if usage["seven_day"]["utilization"] >= 0.95
                              else None)
        with mock.patch.object(runner_mod, "WATCH_SECONDS", 0.1), \
                mock.patch.object(runner_mod, "POLL_SECONDS", 0.2):
            result = self.gemini("slow", usage_stop=stop)
        self.assertFalse(result.ok)
        self.assertEqual(result.extra["usage_stop"], "week at 96%")
        self.assertLess(result.duration_s, 20)
        self.assertEqual(result.usage["seven_day"]["utilization"], 0.96)

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
        backend = MuseCli(str(self.bin), login)
        backend.usage_dir = self.tmp / "usage"
        return backend

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

    def test_each_call_ends_with_a_reading_of_its_usage_panel(self):
        """`muse exec` runs no slash commands, so the TUI is started in a pseudo-terminal of its
        own: the cursor query answered, the workspace trusted the one time it asks, `/usage`
        typed, the panel read. Nothing else is ever typed, so no model is called."""
        backend = self.backend()
        result = self.run_fake(backend)
        self.assertTrue(result.ok, result.error)
        self.assertEqual(result.usage["seven_day"]["utilization"], 0.35)
        self.assertEqual(result.usage["five_hour"]["utilization"], 0.05)
        again = backend.read_usage("the-model")
        self.assertEqual(again["seven_day"]["utilization"], 0.35)
        typed = (self.tmp / "usage" / "typed.log").read_text().split()
        self.assertEqual(typed, ["1", "/usage", "/usage"])  # trusted once, then only /usage
        self.assertTrue(self.seen()["prompt"].endswith("THE TASK"))

    def test_a_refusal_that_names_no_reset_waits_for_the_full_window(self):
        result = self.run_fake(self.backend(), "limit")
        self.assertTrue(result.rate_limited)
        self.assertEqual(result.usage["seven_day"]["utilization"], 0.35)
        self.assertEqual(result.reset_at, "+PT60M")  # nothing is full: the hour stands
        with mock.patch.dict(os.environ, {"FAKE_MODE": "spent", "FAKE_DUMP": str(self.dump)}):
            usage = self.backend().read_usage("the-model")
        self.assertEqual(usage["seven_day"]["utilization"], 0.97)

    def test_no_panel_is_no_reading(self):
        missing = MuseCli(str(self.tmp / "no-such-muse"))
        missing.usage_dir = self.tmp / "usage"
        self.assertIsNone(missing.read_usage("the-model"))


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


class UsageTextTests(unittest.TestCase):
    """agy's `/usage` table and Muse's `/usage` panel as the bot reads them, from what each one
    printed on the machine on 2026-10-04."""

    AGY = ("Gemini Models\tWeekly Limit Remaining\t2%\t2026-10-09T17:47:52Z\n"
           "Gemini Models\tFive Hour Limit Remaining\t33%\t2026-10-04T16:18:35Z\n"
           "Claude and GPT models\tWeekly Limit Remaining\t100%\t2026-10-11T15:04:01Z\n"
           "Claude and GPT models\tFive Hour Limit Remaining\t100%\t2026-10-04T20:04:01Z\n")
    #: The panel as the TUI drew it, its control sequences gone: one line, boxes and all.
    MUSE = ("\u2502 Turns 3 \u2502 \u2502 Subagents none \u2502 \u2502 \u2502 \u2502 Subscription "
            "\u00b7 Muse Code High Usage \u2502 \u2502 Current 5% used \u00b7 Resets at 6:42 PM "
            "\u2502 \u2502 Weekly 35% used \u00b7 Resets Oct 5 at 12:00 AM \u2502\u2514\u2500\u2518")

    def test_agy_reads_the_pool_its_model_draws_on(self):
        from harness.runner import parse_agy_usage
        gemini = parse_agy_usage(self.AGY, "gemini-3.8-flash-high")
        self.assertEqual(gemini["seven_day"],
                         {"utilization": 0.98, "resets_at": "2026-10-09T17:47:52Z"})
        self.assertEqual(gemini["five_hour"]["utilization"], 0.67)
        claude = parse_agy_usage(self.AGY, "claude-opus-4-6-thinking")
        self.assertEqual((claude["seven_day"]["utilization"], claude["five_hour"]["utilization"]),
                         (0.0, 0.0))
        self.assertIsNone(parse_agy_usage("Fetching usage...\nerror: offline\n", "gemini-3.8"))

    def test_muse_reads_the_5_hour_window_and_the_week_with_their_resets(self):
        from datetime import datetime, timezone
        from harness.runner import parse_muse_usage
        now = datetime(2026, 10, 4, 15, 30, tzinfo=timezone.utc)
        usage = parse_muse_usage(self.MUSE, now)
        self.assertEqual(usage["five_hour"],
                         {"utilization": 0.05, "resets_at": "2026-10-04T18:42:00Z"})
        self.assertEqual(usage["seven_day"],
                         {"utilization": 0.35, "resets_at": "2026-10-05T00:00:00Z"})
        self.assertIsNone(parse_muse_usage("\u2502 Session usage \u2502 Turns 3", now))

    def test_muse_prints_the_machines_own_time(self):
        from datetime import datetime, timedelta, timezone
        from harness.runner import _muse_reset
        central = timezone(timedelta(hours=-5))
        now = datetime(2026, 10, 4, 10, 30, tzinfo=central)
        self.assertEqual(_muse_reset("at 6:42 PM", now), "2026-10-04T23:42:00Z")
        self.assertEqual(_muse_reset("at 9:15 AM", now), "2026-10-05T14:15:00Z")  # tomorrow's
        self.assertEqual(_muse_reset("Oct 5 at 12:00 AM", now), "2026-10-05T05:00:00Z")
        self.assertEqual(_muse_reset("Jan 2 at 12:30 PM", now), "2027-01-02T17:30:00Z")
        self.assertEqual(_muse_reset("in 3h 5m", now), "2026-10-04T18:35:00Z")
        self.assertIsNone(_muse_reset("soon", now))
        self.assertIsNone(_muse_reset("Feb 30 at 1:00 AM", now))

