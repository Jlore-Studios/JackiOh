"""`harness.wake`: the start of the Claude sessions every four hours (claude-wake.yml)."""

from __future__ import annotations

import json
import unittest
from datetime import datetime, timedelta, timezone

from harness import config as config_mod
from harness import wake
from harness.clock import iso
from harness.runner import PING_SYSTEM, ping_usage

from tests.fakes import FakeGitHub
from tests.support import make_config, make_ctx

#: 06:00 in Chicago on a CDT day (11:00 UTC) and on a CST day (12:00 UTC).
CDT_SIX = datetime(2026, 10, 22, 11, 0, tzinfo=timezone.utc)
CST_SIX = datetime(2026, 12, 22, 12, 0, tzinfo=timezone.utc)
SET = " ".join(f"CLAUDE_CODE_OAUTH_TOKEN{n}" for n in ("", "_2", "_3", "_4", "_5", "_6", "_7", "_8"))


#: The wake times of a CDT day and of a CST day, in UTC: 02:00, 06:00, …, 22:00 Chicago.
WAKES = [CDT_SIX + n * wake.EVERY for n in range(-1, 5)] + [CST_SIX + n * wake.EVERY
                                                           for n in range(-1, 5)]


class WindowTests(unittest.TestCase):
    def test_the_wake_times_are_six_and_every_four_hours_around_it_in_either_season(self):
        self.assertEqual(wake.EVERY, timedelta(hours=4))
        for at in WAKES:
            self.assertEqual(wake.wake_time(at), at)
            self.assertEqual(wake.wake_time(at - timedelta(minutes=47)), at)
        self.assertEqual(wake.wake_time(CDT_SIX).astimezone(wake.clock.zone(wake.ZONE)).hour, 6)

    def test_each_run_is_for_at_most_one_wake_time(self):
        self.assertEqual(timedelta(days=1) % wake.EVERY, timedelta(0))
        self.assertLessEqual(wake.EARLY + wake.LATE, wake.EVERY)

    def test_every_wake_time_has_a_firing_that_sleeps_until_it_and_two_after(self):
        """`13 * * * *`: the firing in the hour before a wake time sleeps until it; the two after
        it still wake the sessions, should GitHub drop the first."""
        text = (config_mod.REPO_ROOT / ".github" / "workflows" / wake.WORKFLOW).read_text(
            encoding="utf-8")
        self.assertIn('- cron: "13 * * * *"', text)
        for at in WAKES:
            for fired in (at - timedelta(minutes=47), at + timedelta(minutes=13),
                          at + timedelta(minutes=73)):
                self.assertEqual(wake.wake_time(fired), at, fired)

    def test_a_late_run_wakes_at_once_and_a_much_later_one_does_nothing(self):
        self.assertEqual(wake.wake_time(CDT_SIX + timedelta(minutes=40)), CDT_SIX)
        self.assertIsNone(wake.wake_time(CDT_SIX + wake.LATE))
        self.assertIsNone(wake.wake_time(CDT_SIX - wake.EARLY - timedelta(minutes=1)))

    def test_a_spring_forward_night_keeps_its_windows_apart(self):
        """2027-03-14: 02:00 Chicago does not exist, so it and 06:00 are three hours apart."""
        six = datetime(2027, 3, 14, 11, 0, tzinfo=timezone.utc)
        two = wake.wake_time(six - timedelta(hours=3))
        self.assertIsNotNone(two)
        self.assertEqual(wake.wake_time(six - timedelta(minutes=30)), six)
        self.assertEqual(wake.wake_time(two + timedelta(minutes=30)), two)

    def test_a_run_that_finished_green_since_the_wake_time_has_woken_them(self):
        green = {"conclusion": "success", "updated_at": iso(CDT_SIX + timedelta(minutes=2))}
        early = {"conclusion": "success", "updated_at": iso(CDT_SIX - timedelta(hours=2))}
        failed = {"conclusion": "failure", "updated_at": iso(CDT_SIX + timedelta(minutes=2))}
        later = CDT_SIX + timedelta(minutes=30)
        self.assertTrue(wake.already_woken([early, green], later))
        self.assertFalse(wake.already_woken([early, failed], later))

    def test_the_last_wake_does_not_count_for_the_next(self):
        """A run sleeping until ten finds six's green run, which woke six's sessions, not ten's."""
        green = {"conclusion": "success", "updated_at": iso(CDT_SIX + timedelta(minutes=2))}
        self.assertFalse(wake.already_woken([green], CDT_SIX + wake.EVERY - timedelta(minutes=47)))
        self.assertFalse(wake.already_woken([green], CDT_SIX + timedelta(hours=3)))


class AccountTests(unittest.TestCase):
    def cfg(self, secrets: str = SET):
        return make_config(env={"HARNESS_SECRETS_SET": secrets})

    def test_every_claude_subscription_with_its_secret_is_woken(self):
        found, notes = wake.accounts(self.cfg(), {"providers": {}}, CDT_SIX, ".harness")
        self.assertEqual({a["id"] for a in found}, {f"claude-{n}" for n in range(1, 9)})
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

    def test_a_session_its_limit_refuses_is_left_alone_until_it_resets(self):
        refused = {"refused_until": iso(CDT_SIX + timedelta(hours=3))}
        week = {"usage": {"five_hour": {"utilization": 0.4,
                                        "resets_at": iso(CDT_SIX - timedelta(hours=1))},
                          "seven_day": {"utilization": 1.0,
                                        "resets_at": iso(CDT_SIX + timedelta(days=2))}}}
        over = {"refused_until": iso(CDT_SIX - timedelta(minutes=5)),
                "usage": {"seven_day": {"utilization": 1.0,
                                        "resets_at": iso(CDT_SIX - timedelta(minutes=5))}}}
        state = {"providers": {"claude-1": refused, "claude-2": week, "claude-3": over}}
        found, notes = wake.accounts(self.cfg(), state, CDT_SIX, ".harness")
        ids = [a["id"] for a in found]
        self.assertNotIn("claude-1", ids)
        self.assertNotIn("claude-2", ids)
        self.assertIn("claude-3", ids, "a refusal that has run out holds nothing")
        self.assertEqual(len(notes), 2, notes)
        self.assertTrue(all("usage limit refuses calls until" in n for n in notes), notes)

    def test_every_new_claude_subscription_is_woken_and_none_of_another_cli(self):
        """The list is the pool's Claude subscriptions, however many there are."""
        cfg = self.cfg()
        claude = [p.id for p in cfg.pool.ordered()
                  if p.cli == "claude" and p.login == "secret" and p.enabled]
        found, _ = wake.accounts(cfg, {"providers": {}}, CDT_SIX, ".harness")
        self.assertEqual([a["id"] for a in found], claude)

    def test_a_halted_bot_wakes_nothing(self):
        gh = FakeGitHub()
        ctx = make_ctx(gh, at=CDT_SIX, cfg=self.cfg())
        self.assertEqual(len(wake.to_wake(ctx, ".harness")[0]), 8)
        ctx.store.update(lambda s: s.update(halted=True), "halt")
        self.assertEqual(wake.to_wake(ctx, ".harness"), ([], ["halted: a wake is a model call"]))


def ran(stdout: str, seen: dict | None = None):
    """A stand-in for `subprocess.run` whose CLI prints `stdout`."""
    def run(argv, **kwargs):
        if seen is not None:
            seen["argv"] = argv

        class Proc:
            pass
        Proc.stdout = stdout
        return Proc()
    return run


def event(status: str, five_hour: float = 0.01, **info) -> str:
    return json.dumps({"type": "rate_limit_event", "rate_limit_info": {
        "status": status, "unifiedWindows": {
            "five_hour": {"utilization": five_hour, "resetsAt": 1790710800}}, **info}})


class PingTests(unittest.TestCase):
    def test_the_wake_is_one_turn_of_haiku_at_low_effort_with_nothing_loaded(self):
        seen: dict = {}
        found = ping_usage("claude", wake.MODEL, run=ran(event("allowed"), seen),
                           effort=wake.EFFORT)
        self.assertAlmostEqual(found["five_hour"]["utilization"], 0.01)
        argv = seen["argv"]
        self.assertEqual(argv[argv.index("--model") + 1], "claude-haiku-5-5")
        self.assertEqual(argv[argv.index("--effort") + 1], "low")
        self.assertEqual(argv[argv.index("--max-turns") + 1], "1")
        self.assertEqual(argv[argv.index("--tools") + 1], "")
        self.assertEqual(argv[argv.index("--system-prompt") + 1], PING_SYSTEM)
        for flag in ("--strict-mcp-config", "--disable-slash-commands",
                     "--no-session-persistence"):
            self.assertIn(flag, argv)

    def test_the_quiet_ping_names_no_effort(self):
        seen: dict = {}
        ping_usage("claude", "haiku", run=ran("", seen))
        self.assertNotIn("--effort", seen["argv"])


class WakeTests(unittest.TestCase):
    cfg = make_config(env={"HARNESS_SECRETS_SET": SET})

    def wake(self, stdout: str) -> tuple[bool, str]:
        real = wake.ping
        wake.ping = lambda *a, **k: real(*a, run=ran(stdout), **k)
        try:
            return wake.wake(self.cfg, "claude-1")
        finally:
            wake.ping = real

    def test_a_ping_starts_the_session(self):
        ok, line = self.wake(event("allowed"))
        self.assertTrue(ok)
        self.assertIn("the 5-hour session is running", line)

    def test_a_ping_its_limit_rejects_is_reported_not_failed(self):
        ok, line = self.wake("\n".join([
            event("rejected", 1.0, resetsAt=1790710800),
            json.dumps({"type": "result", "is_error": True, "result": "You've hit your limit"})]))
        self.assertTrue(ok, line)
        self.assertIn("its usage limit refused the ping", line)
        self.assertIn(iso(datetime.fromtimestamp(1790710800, timezone.utc)), line)

    def test_a_refusal_the_cli_gives_only_in_words_is_one_too(self):
        ok, line = self.wake(json.dumps({"type": "result", "is_error": True,
                                         "result": "Claude AI usage limit reached"}))
        self.assertTrue(ok, line)
        self.assertIn("its usage limit refused the ping", line)

    def test_a_ping_that_says_nothing_fails(self):
        ok, line = self.wake(json.dumps({"type": "result", "is_error": True,
                                         "result": "connection reset"}))
        self.assertFalse(ok)
        self.assertIn("no usage reading", line)


if __name__ == "__main__":
    unittest.main()
