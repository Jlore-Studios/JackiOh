"""`harness.wake`: the morning start of the Claude sessions (claude-wake.yml)."""

from __future__ import annotations

import json
import unittest
from datetime import datetime, timedelta, timezone

from harness import config as config_mod
from harness import wake
from harness.clock import iso
from harness.runner import ping_usage

from tests.fakes import FakeGitHub
from tests.support import make_config, make_ctx

#: 06:00 in Chicago on a CDT day (11:00 UTC) and on a CST day (12:00 UTC).
CDT_SIX = datetime(2026, 10, 22, 11, 0, tzinfo=timezone.utc)
CST_SIX = datetime(2026, 12, 22, 12, 0, tzinfo=timezone.utc)
SET = " ".join(f"CLAUDE_CODE_OAUTH_TOKEN{n}" for n in ("", "_2", "_3", "_4", "_5", "_6", "_7"))


class WindowTests(unittest.TestCase):
    def test_a_run_before_six_sleeps_until_six_in_either_season(self):
        for six in (CDT_SIX, CST_SIX):
            self.assertEqual(wake.wake_time(six - timedelta(hours=3)), six)
            self.assertEqual(wake.until_wake(six - timedelta(hours=3)), timedelta(hours=3))

    def test_every_firing_of_the_schedule_lands_in_the_window(self):
        """`13 8,9,10,11 * * *`: each firing either sleeps until six or, past it, wakes at once."""
        text = (config_mod.REPO_ROOT / ".github" / "workflows" / wake.WORKFLOW).read_text(
            encoding="utf-8")
        self.assertIn('- cron: "13 8,9,10,11 * * *"', text)
        for day in (CDT_SIX, CST_SIX):
            for hour in (8, 9, 10, 11):
                fired = day.replace(hour=hour, minute=13)
                self.assertIsNotNone(wake.until_wake(fired), fired)

    def test_a_late_run_wakes_at_once_and_a_much_later_one_does_nothing(self):
        self.assertEqual(wake.until_wake(CDT_SIX + timedelta(minutes=40)), timedelta(0))
        self.assertIsNone(wake.until_wake(CDT_SIX + wake.LATE))
        self.assertIsNone(wake.until_wake(CDT_SIX - wake.EARLY - timedelta(minutes=1)))

    def test_a_run_that_finished_green_since_six_has_woken_them(self):
        green = {"conclusion": "success", "updated_at": iso(CDT_SIX + timedelta(minutes=2))}
        early = {"conclusion": "success", "updated_at": iso(CDT_SIX - timedelta(hours=2))}
        failed = {"conclusion": "failure", "updated_at": iso(CDT_SIX + timedelta(minutes=2))}
        later = CDT_SIX + timedelta(minutes=30)
        self.assertTrue(wake.already_woken([early, green], later))
        self.assertFalse(wake.already_woken([early, failed], later))


class AccountTests(unittest.TestCase):
    def cfg(self, secrets: str = SET):
        return make_config(env={"HARNESS_SECRETS_SET": secrets})

    def test_every_claude_subscription_with_its_secret_is_woken(self):
        found, notes = wake.accounts(self.cfg(), {"providers": {}}, CDT_SIX, ".harness")
        self.assertEqual({a["id"] for a in found}, {f"claude-{n}" for n in range(1, 8)})
        self.assertEqual({a["secret"] for a in found}, set(SET.split()))
        self.assertEqual({a["home"] for a in found}, {".harness"})
        self.assertEqual(notes, [])

    def test_a_running_session_a_missing_secret_and_a_suspension_are_left_alone(self):
        running = {"usage": {"five_hour": {"utilization": 0.3,
                                           "resets_at": iso(CDT_SIX + timedelta(hours=2))}}}
        over = {"usage": {"five_hour": {"utilization": 0.9,
                                        "resets_at": iso(CDT_SIX - timedelta(minutes=5))}}}
        state = {"providers": {"claude-1": running, "claude-2": over,
                               "claude-3": {"suspended": {"by": "jgoetzmann"}}}}
        found, notes = wake.accounts(self.cfg(SET.replace("CLAUDE_CODE_OAUTH_TOKEN_4", "")),
                                     state, CDT_SIX, ".harness")
        ids = [a["id"] for a in found]
        self.assertNotIn("claude-1", ids)
        self.assertIn("claude-2", ids, "a session whose window has ended is not running")
        self.assertNotIn("claude-3", ids)
        self.assertNotIn("claude-4", ids)
        self.assertEqual(len(notes), 3, notes)


    def test_a_halted_bot_wakes_nothing(self):
        gh = FakeGitHub()
        ctx = make_ctx(gh, at=CDT_SIX, cfg=self.cfg())
        self.assertEqual(len(wake.to_wake(ctx, ".harness")[0]), 7)
        ctx.store.update(lambda s: s.update(halted=True), "halt")
        self.assertEqual(wake.to_wake(ctx, ".harness"), ([], ["halted: a wake is a model call"]))


class PingTests(unittest.TestCase):
    def test_the_wake_is_one_turn_of_haiku_at_low_effort(self):
        seen = {}

        def run(argv, **kwargs):
            seen["argv"] = argv

            class Proc:
                stdout = json.dumps({"type": "rate_limit_event", "rate_limit_info": {
                    "status": "allowed", "unifiedWindows": {
                        "five_hour": {"utilization": 0.01, "resetsAt": 1790710800}}}})
            return Proc()

        found = ping_usage("claude", wake.MODEL, run=run, effort=wake.EFFORT)
        self.assertAlmostEqual(found["five_hour"]["utilization"], 0.01)
        argv = seen["argv"]
        self.assertEqual(argv[argv.index("--model") + 1], "claude-haiku-5-5")
        self.assertEqual(argv[argv.index("--effort") + 1], "low")
        self.assertEqual(argv[argv.index("--max-turns") + 1], "1")

    def test_the_quiet_ping_names_no_effort(self):
        seen = {}

        def run(argv, **kwargs):
            seen["argv"] = argv

            class Proc:
                stdout = ""
            return Proc()

        ping_usage("claude", "haiku", run=run)
        self.assertNotIn("--effort", seen["argv"])


if __name__ == "__main__":
    unittest.main()
