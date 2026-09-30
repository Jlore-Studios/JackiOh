"""Starting only when the subscription is quiet, and never mistaking the partner bot for a person."""

from __future__ import annotations

import json
import unittest
from datetime import datetime, timedelta, timezone

from harness import plan as plan_mod
from harness import quiet
from harness.config import LABEL_BUILD, LABEL_PR, LABEL_WORKING, Partner, Quiet
from harness.errors import GitHubError
from harness.runner import ping_usage
from harness.state import item as state_item

from tests.fakes import FakeGitHub
from tests.support import DAY, NIGHT, make_config, make_ctx

T0 = datetime(2026, 9, 30, 3, 0, tzinfo=timezone.utc)
SETTINGS = Quiet(enabled=True, interval_minutes=10, max_wait_minutes=40, ping_model="haiku",
                 partners=())
PARTNER = Partner("jgoetzmann/bright-bots-harness",
                  {"implement.yml": ("Run planned items",), "discover.yml": ("Discover and propose",)})


def usage(five: float, seven: float = 0.5, resets: str = "2026-09-30T05:00:00Z",
          status: str = "allowed") -> dict:
    return {"five_hour": {"utilization": five, "resets_at": resets},
            "seven_day": {"utilization": seven, "resets_at": "2026-10-02T00:00:00Z"},
            "status": status}


class World:
    """A fake clock that `sleep` advances, and a queue of usage readings."""

    def __init__(self, readings: list) -> None:
        self.at = T0
        self.readings = list(readings)
        self.pings = 0
        self.sleeps: list[float] = []

    def now(self) -> datetime:
        return self.at

    def sleep(self, seconds: float) -> None:
        self.sleeps.append(seconds)
        self.at += timedelta(seconds=seconds)

    def ping(self):
        self.pings += 1
        return self.readings.pop(0) if self.readings else None


def verdict(readings: list, partner=lambda t1, t2: None) -> tuple[quiet.Verdict, World]:
    world = World(readings)
    return quiet.wait_for_quiet(SETTINGS, world.ping, partner, world.now, world.sleep), world


class CompareTests(unittest.TestCase):
    def s(self, five, seven=0.5, resets="r1", status="allowed"):
        return quiet.Sample(T0, five, seven, resets, "w", status)

    def test_cases(self):
        self.assertEqual(quiet.compare(self.s(0.10), self.s(0.10)), "quiet")
        self.assertEqual(quiet.compare(self.s(0.10), self.s(0.11)), "rose")
        self.assertEqual(quiet.compare(self.s(0.10, 0.50), self.s(0.10, 0.51)), "rose")
        self.assertEqual(quiet.compare(self.s(0.40), self.s(0.02, resets="r2")), "inconclusive")
        self.assertEqual(quiet.compare(self.s(0.10), self.s(0.10, status="rejected")), "refused")


class WaitTests(unittest.TestCase):
    def test_quiet_after_one_interval(self):
        result, world = verdict([usage(0.10), usage(0.10)])
        self.assertTrue(result.quiet)
        self.assertEqual((world.pings, world.sleeps), (2, [600.0]))

    def test_a_rise_waits_and_looks_again(self):
        result, world = verdict([usage(0.10), usage(0.13), usage(0.13)])
        self.assertTrue(result.quiet)
        self.assertEqual(world.pings, 3)
        self.assertEqual(len(result.samples), 3)

    def test_a_rise_while_the_partner_spends_is_excused(self):
        seen = []
        def partner(t1, t2):
            seen.append((t1, t2))
            return "jgoetzmann/bright-bots-harness implement.yml: Run planned items"
        result, _ = verdict([usage(0.10), usage(0.20)], partner)
        self.assertTrue(result.quiet)
        self.assertIn("Run planned items", result.excused_by)
        self.assertEqual(seen, [(T0, T0 + timedelta(minutes=10))])

    def test_a_person_who_keeps_working_holds_the_run_back(self):
        result, world = verdict([usage(0.10 + 0.02 * i) for i in range(10)])
        self.assertFalse(result.quiet)
        self.assertIn("40 minutes", result.reason)
        self.assertEqual(world.pings, 5)  # one reading, then one every ten minutes for forty

    def test_a_window_that_turned_over_is_read_again(self):
        result, world = verdict([usage(0.60, resets="a"), usage(0.01, resets="b"), usage(0.01, resets="b")])
        self.assertTrue(result.quiet)
        self.assertEqual(world.pings, 3)

    def test_refused_and_unreadable(self):
        result, _ = verdict([usage(0.10), usage(0.10, status="rejected")])
        self.assertFalse(result.quiet)
        self.assertIn("refused", result.reason)
        result, world = verdict([None])
        self.assertFalse(result.quiet)
        self.assertEqual(world.sleeps, [])


class FakeReader:
    def __init__(self, runs: dict, jobs: dict, fail: bool = False) -> None:
        self.runs, self.jobs, self.fail = runs, jobs, fail

    def list_runs(self, workflow, limit=10):
        if self.fail:
            raise GitHubError("boom", 500)
        return self.runs.get(workflow, [])

    def list_jobs(self, run_id):
        return self.jobs.get(run_id, [])


def step(name, started, ended=None):
    iso = lambda d: d.strftime("%Y-%m-%dT%H:%M:%SZ") if d else None
    return {"name": name, "started_at": iso(started), "completed_at": iso(ended)}


class PartnerTests(unittest.TestCase):
    t1, t2 = T0, T0 + timedelta(minutes=10)

    def spending(self, steps, status="in_progress", updated=None):
        runs = {"implement.yml": [{"id": 1, "status": status,
                                   "updated_at": (updated or self.t2).strftime("%Y-%m-%dT%H:%M:%SZ")}]}
        jobs = {1: [{"name": "implement", "steps": steps}]}
        return quiet.partner_spending((PARTNER,), lambda repo: FakeReader(runs, jobs), self.t1, self.t2)

    def test_a_spending_step_inside_the_interval(self):
        found = self.spending([step("Run planned items (harness run --item)", self.t1 - timedelta(hours=1))])
        self.assertIn("Run planned items", found)

    def test_a_step_that_ended_before_or_never_started(self):
        before = step("Run planned items", self.t1 - timedelta(hours=2), self.t1 - timedelta(minutes=1))
        self.assertIsNone(self.spending([before]))
        self.assertIsNone(self.spending([step("Run planned items", None)]))

    def test_waiting_at_the_partners_own_gate_is_not_spending(self):
        gate = step("Wait until the subscription is quiet (harness quiet)", self.t1)
        self.assertIsNone(self.spending([gate]))

    def test_an_old_finished_run_is_not_read_and_errors_excuse_nothing(self):
        old = self.t1 - timedelta(hours=3)
        self.assertIsNone(self.spending([step("Run planned items", old - timedelta(hours=1))],
                                        status="completed", updated=old))
        failing = quiet.partner_spending((PARTNER,), lambda repo: FakeReader({}, {}, fail=True),
                                         self.t1, self.t2)
        self.assertIsNone(failing)

    def test_the_reader_falls_back_to_no_token(self):
        made = []
        class Client:
            def __init__(self, repo, token):
                self.token = token
                made.append(token)
            def list_runs(self, workflow, limit=10):
                if self.token:
                    raise GitHubError("Resource not accessible by integration", 403)
                return [{"id": 7}]
        reader = quiet.PartnerReader("x/y", "actions-token", factory=Client)
        self.assertEqual(reader.list_runs("implement.yml"), [{"id": 7}])
        self.assertEqual(made, ["actions-token", ""])


class PingTests(unittest.TestCase):
    def test_the_ping_is_one_turn_of_the_small_model_in_an_empty_directory(self):
        seen = {}
        def run(argv, **kwargs):
            seen.update(argv=argv, cwd=kwargs["cwd"], env=kwargs["env"])
            class Proc:
                stdout = "\n".join([
                    json.dumps({"type": "rate_limit_event", "rate_limit_info": {
                        "status": "allowed", "unifiedWindows": {
                            "five_hour": {"utilization": 0.12, "resetsAt": 1790710800}}}}),
                    json.dumps({"type": "result", "result": "ok"}),
                ])
            return Proc()
        found = ping_usage("claude", "haiku", run=run)
        self.assertAlmostEqual(found["five_hour"]["utilization"], 0.12)
        self.assertIn("--max-turns", seen["argv"])
        self.assertEqual(seen["argv"][seen["argv"].index("--model") + 1], "haiku")
        self.assertIn("bot-ping-", seen["cwd"])
        self.assertNotIn("BOT_GITHUB_TOKEN", seen["env"])


class PeekTests(unittest.TestCase):
    """`peek` reads what `make` would do, without doing it."""

    def agree(self, gh, at=NIGHT, **kwargs):
        ctx = make_ctx(gh, at=at)
        before = json.dumps({n: t["labels"] for n, t in gh.threads.items()}, sort_keys=True)
        work, reason, forced = plan_mod.peek(ctx, **kwargs)
        after = json.dumps({n: t["labels"] for n, t in gh.threads.items()}, sort_keys=True)
        self.assertEqual(before, after, "peek changed labels")
        planned = plan_mod.make(ctx, **kwargs)
        self.assertEqual(work, planned["action"] != "none", (reason, planned))
        return work, reason, forced

    def test_a_queued_issue(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        self.assertEqual(self.agree(gh)[:1], (True,))

    def test_nothing_by_day(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        self.assertEqual(self.agree(gh, at=DAY)[0], False)

    def test_a_forced_item_skips_the_quiet_check(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        ctx = make_ctx(gh, at=DAY)
        ctx.store.update(lambda s: state_item(s, 3).update(forced=True))
        self.assertEqual(plan_mod.peek(ctx), (True, "#3 is queued to build", True))
        self.assertEqual(plan_mod.peek(make_ctx(FakeGitHub(), at=DAY), force=True)[::2], (True, True))

    def test_a_survey_when_idle(self):
        work, reason, _ = self.agree(FakeGitHub())
        self.assertTrue(work)
        self.assertIn("survey", reason)

    def test_a_dead_runs_item_counts_as_work(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_WORKING,))
        gh.runs["5"] = {"status": "completed"}
        ctx = make_ctx(gh)
        ctx.store.update(lambda s: (state_item(s, 3).update(run_id="5"),
                                    s.update(suggest={"last_run": "2026-09-30T02:59:00Z",
                                                      "requested": False})))
        work, reason, _ = plan_mod.peek(ctx)
        self.assertTrue(work)
        self.assertIn("left working", reason)

    def test_a_conflicted_bot_pr_counts_as_work(self):
        gh = FakeGitHub()
        gh.add_pull(9, "bot/issue-2", labels=(LABEL_PR,))["mergeable_state"] = "dirty"
        ctx = make_ctx(gh)
        ctx.store.update(lambda s: s.update(suggest={"last_run": "2026-09-30T02:59:00Z",
                                                     "requested": False}))
        self.assertIn("conflicts with main", plan_mod.peek(ctx)[1])

    def test_the_committed_config(self):
        cfg = make_config()
        self.assertTrue(cfg.quiet.enabled)
        self.assertEqual((cfg.quiet.interval_minutes, cfg.quiet.max_wait_minutes), (10, 40))
        partner = cfg.quiet.partners[0]
        self.assertEqual(partner.repo, "jgoetzmann/bright-bots-harness")
        self.assertIn("Run planned items", partner.workflows["implement.yml"])


if __name__ == "__main__":
    unittest.main()
