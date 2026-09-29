"""The claude CLI backend, driven against a fake `claude` executable."""

from __future__ import annotations

import json
import os
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest import mock

from harness.runner import ClaudeCli, RunRequest, parse_stream, usage_from_event

FAKE_CLAUDE = textwrap.dedent('''\
    #!{python}
    import json, os, sys, time
    prompt = sys.stdin.read()
    mode = os.environ.get("FAKE_MODE", "ok")
    with open(os.environ["FAKE_ENV_DUMP"], "w") as f:
        json.dump({{"keys": sorted(os.environ), "argv": sys.argv[1:], "prompt": prompt}}, f)
    def emit(obj):
        print(json.dumps(obj), flush=True)
    emit({{"type": "system", "subtype": "init"}})
    if mode == "sleep":
        time.sleep(30)
    status = "rejected" if mode == "limit" else "allowed"
    emit({{"type": "rate_limit_event", "rate_limit_info": {{"status": status,
          "resetsAt": 1790712000, "unifiedWindows": {{
            "five_hour": {{"utilization": 1.0 if mode == "limit" else 0.25, "resetsAt": 1790710800}},
            "seven_day": {{"utilization": 0.5, "resetsAt": 1790712000}}}}}}}})
    if mode == "limit":
        emit({{"type": "result", "is_error": True, "subtype": "success",
              "result": "You've hit your limit", "num_turns": 1}})
        sys.exit(1)
    emit({{"type": "result", "is_error": False, "result": "done: " + prompt[:20], "num_turns": 7}})
''')


class RunnerTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.bin = self.tmp / "claude"
        self.bin.write_text(FAKE_CLAUDE.format(python=sys.executable))
        self.bin.chmod(0o755)
        self.dump = self.tmp / "env.json"

    def request(self, timeout: int = 60) -> RunRequest:
        return RunRequest(
            role="build", prompt="build the thing please", cwd=self.tmp,
            system_append="system text", allowed_tools=("Bash", "Read"),
            disallowed_tools=("WebFetch",), max_turns=42, timeout_s=timeout, model="opus",
            effort="xhigh", transcript=self.tmp / "t" / "01-build.jsonl",
        )

    def run_fake(self, mode: str, timeout: int = 60):
        env = {"FAKE_MODE": mode, "FAKE_ENV_DUMP": str(self.dump),
               "BOT_GITHUB_TOKEN": "ghp_" + "x" * 36, "GITHUB_TOKEN": "actions-token-value",
               "CLAUDE_CODE_OAUTH_TOKEN": "oauth-token-value", "ANTHROPIC_API_KEY": "sk-ant-nope"}
        with mock.patch.dict(os.environ, env):
            return ClaudeCli(str(self.bin)).run(self.request(timeout))

    def test_argv(self):
        argv = ClaudeCli("claude").argv(self.request(), Path("/tmp/s.md"))
        joined = " ".join(argv)
        for part in ("--print", "--output-format stream-json", "--model opus", "--effort xhigh",
                     "--max-turns 42", "--permission-mode acceptEdits", "--allowed-tools Bash,Read",
                     "--disallowed-tools WebFetch", "--append-system-prompt-file /tmp/s.md",
                     "--strict-mcp-config"):
            self.assertIn(part, joined)
        self.assertNotIn("build the thing", joined)  # the prompt goes on stdin

    def test_success_and_environment(self):
        result = self.run_fake("ok")
        self.assertTrue(result.ok, result.error)
        self.assertEqual(result.turns, 7)
        self.assertEqual(result.text, "done: build the thing plea")
        self.assertAlmostEqual(result.usage["five_hour"]["utilization"], 0.25)
        self.assertFalse(result.rate_limited)
        dumped = json.loads(self.dump.read_text())
        self.assertIn("CLAUDE_CODE_OAUTH_TOKEN", dumped["keys"])
        self.assertIn("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", dumped["keys"])
        for secret in ("BOT_GITHUB_TOKEN", "GITHUB_TOKEN", "ANTHROPIC_API_KEY"):
            self.assertNotIn(secret, dumped["keys"])
        self.assertEqual(dumped["prompt"], "build the thing please")
        transcript = (self.tmp / "t" / "01-build.jsonl").read_text()
        self.assertIn('"type": "result"', transcript)
        self.assertFalse((self.tmp / "t" / "01-build.raw").exists())

    def test_refusal_is_a_rate_limit_with_its_reset(self):
        result = self.run_fake("limit")
        self.assertFalse(result.ok)
        self.assertTrue(result.rate_limited)
        self.assertEqual(result.reset_at, "2026-09-29T20:00:00Z")

    def test_timeout_kills_the_process(self):
        result = self.run_fake("sleep", timeout=2)
        self.assertTrue(result.timed_out)
        self.assertFalse(result.ok)

    def test_missing_binary(self):
        result = ClaudeCli(str(self.tmp / "nope")).run(self.request())
        self.assertFalse(result.ok)
        self.assertEqual(result.exit_code, 127)

    def test_parse_stream_ignores_noise(self):
        lines = ["not json", "", json.dumps({"type": "result", "result": "x"}), "[1,2]"]
        result, usage = parse_stream(lines)
        self.assertEqual(result["result"], "x")
        self.assertIsNone(usage)

    def test_overage_is_not_a_refusal(self):
        usage = usage_from_event({"rate_limit_info": {"status": "rejected", "isUsingOverage": True,
                                                      "unifiedWindows": {}}})
        self.assertEqual(usage["status"], "allowed_overage")


if __name__ == "__main__":
    unittest.main()
