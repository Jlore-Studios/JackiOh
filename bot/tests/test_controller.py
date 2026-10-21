"""The status loop's planner (`controller.py`, #1455): with `controller` on, bot-status's loop plans
the work and starts each night run with its plan, so the run skips its gate and plan jobs."""

from __future__ import annotations

import dataclasses
import io
import json
import unittest
from contextlib import redirect_stdout
from datetime import timedelta
from unittest import mock

from harness import __main__ as main_mod
from harness import controller, deliver as deliver_mod, plan as plan_mod, providers, sweep
from harness.clock import iso
from harness.config import LABEL_BUILD, LABEL_WORKING, STATE_BRANCH
from harness.errors import GitHubError
from harness.state import item as state_item

from tests.fakes import FakeGitHub
from tests.support import DAY, NIGHT, make_config, make_ctx
from tests.test_providers import secrets

STATUS = ".github/workflows/bot-status.yml"


def no_sleep(_seconds: float) -> None:
    pass


def loop_ctx(gh: FakeGitHub, *, at=DAY, loop: bool = True):
    cfg = make_config(env=secrets(*providers.SECRETS), machine=("muse",), controller=True)
    return make_ctx(gh, at=at, cfg=dataclasses.replace(cfg, controller_loop=loop, run_id="777"))


def listing(gh: FakeGitHub, run_id: int = 5001):
    """Make GitHub list each dispatched night run under the title bot-night.yml's run-name gives."""
    real = gh.dispatch_workflow

    def dispatch(workflow, ref, inputs):
        real(workflow, ref, inputs)
        if inputs.get("plan_ref"):
            number = run_id + len(gh.dispatches)
            gh.runs[str(number)] = {"id": number, "status": "queued",
                                    "path": ".github/workflows/bot-night.yml",
                                    "display_title": f"bot-night {inputs['plan_ref']} on "
                                                     f"{inputs['runs_on']}"}

    gh.dispatch_workflow = dispatch


class StartTests(unittest.TestCase):
    def test_the_loop_claims_writes_the_plan_and_starts_the_run_with_it(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        listing(gh)
        ctx = loop_ctx(gh)
        notes = controller.start(ctx, sleep=no_sleep)
        self.assertIn("started build #3 on muse (run 5002)", notes)
        [dispatch] = [d for d in gh.dispatches if d["inputs"].get("plan_ref")]
        inputs = dispatch["inputs"]
        self.assertEqual((inputs["runs_on"], inputs["cli"], inputs["secret"], inputs["shared"]),
                         ("night-vm-muse", "muse", "", "false"))
        self.assertRegex(inputs["plan_ref"], controller.REF)
        text, _ = gh.get_file(f"plans/{inputs['plan_ref']}.json", STATE_BRANCH)
        self.assertEqual((json.loads(text)["number"], json.loads(text)["provider"]), (3, "muse"))
        record = ctx.store.load()["items"]["3"]
        self.assertEqual((record["run_id"], record["provider"]), ("5002", "muse"))
        self.assertIn(LABEL_WORKING, gh.label_names(3))
        # The lock is the loop's only while it plans.
        self.assertIsNone(ctx.store.load().get("planning"))

    def test_it_plans_again_until_nothing_more_can_start(self):
        gh = FakeGitHub()
        for n in (3, 4, 5):
            gh.add_issue(n, labels=(LABEL_BUILD,))
        listing(gh)
        ctx = loop_ctx(gh)
        notes = controller.start(ctx, sleep=no_sleep)
        started = [n for n in notes if n.startswith("started")]
        # Muse has two lanes on the machine; the third item waits (by day no Claude account works).
        self.assertEqual(len(started), 2, notes)
        self.assertTrue(notes[-1].startswith("nothing more to start"), notes)
        # Each plan run is the loop's: none of them started another run as a plan job's
        # `fill_lanes` does.
        self.assertEqual(len(gh.dispatches), 2)

    def test_a_run_not_listed_yet_keeps_its_claim_pending_until_found(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        ctx = loop_ctx(gh)
        notes = controller.start(ctx, sleep=no_sleep)
        self.assertIn("started build #3 on muse (its run is not listed yet)", notes)
        run_id = ctx.store.load()["items"]["3"]["run_id"]
        self.assertTrue(run_id.startswith(plan_mod.PENDING))
        # Pending holds the lane for a while, then counts as dead.
        self.assertEqual(plan_mod.run_status(ctx, run_id), "alive")
        later = make_ctx(gh, at=DAY + plan_mod.PENDING_GRACE + timedelta(minutes=1), cfg=ctx.cfg)
        self.assertEqual(plan_mod.run_status(later, run_id), "dead")
        # Once GitHub lists it, the next tick puts its id on the claim.
        ref = run_id[len(plan_mod.PENDING):]
        gh.runs["6001"] = {"id": 6001, "status": "in_progress",
                           "display_title": f"bot-night {ref} on night-vm-muse"}
        self.assertEqual(plan_mod.run_status(ctx, run_id), "alive")
        self.assertEqual(controller.resolve(ctx), ["found the run of #3: 6001"])
        self.assertEqual(ctx.store.load()["items"]["3"]["run_id"], "6001")

    def test_a_dispatch_github_refuses_leaves_the_claim_dead_for_housekeeping(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        ctx = loop_ctx(gh)

        def refuse(*args, **kwargs):
            raise GitHubError("no", 500)

        gh.dispatch_workflow = refuse
        notes = controller.start(ctx, sleep=no_sleep)
        self.assertTrue(any(n.startswith("could not start build #3") for n in notes), notes)
        self.assertEqual(ctx.store.load()["items"]["3"]["run_id"], "")
        self.assertEqual(plan_mod.run_status(ctx, ""), "dead")

    def test_another_planner_holding_the_lock_waits_the_loop_a_tick(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        ctx = loop_ctx(gh)
        ctx.store.update(lambda s: s.update(planning={"owner": "run 42", "at": iso(DAY)}))
        notes = controller.start(ctx, sleep=no_sleep)
        self.assertEqual(notes, ["another planner holds the planning lock: the next tick plans"])
        self.assertEqual(gh.dispatches, [])
        # A lock older than its TTL was left by a planner that died.
        ctx.store.update(lambda s: s.update(planning={
            "owner": "run 42", "at": iso(DAY - plan_mod.LOCK_TTL - timedelta(seconds=1))}))
        listing(gh)
        self.assertTrue(any(n.startswith("started") for n in controller.start(ctx, sleep=no_sleep)))


class LockTests(unittest.TestCase):
    def test_the_loop_renews_the_lock_before_each_start(self):
        gh = FakeGitHub()
        for n in (3, 4):
            gh.add_issue(n, labels=(LABEL_BUILD,))
        listing(gh)
        ctx = loop_ctx(gh)
        taken = []
        real = plan_mod.take_planning_lock

        def spy(c, owner):
            taken.append(owner)
            return real(c, owner)

        with mock.patch.object(plan_mod, "take_planning_lock", spy):
            controller.start(ctx, sleep=no_sleep)
        # Once to start, then again before every later start (two runs, then the last plan).
        self.assertEqual(taken, ["loop 777"] * 3)
        self.assertIsNone(ctx.store.load().get("planning"))


class CheckTests(unittest.TestCase):
    def planned(self, ctx, **changes):
        return {"provider": "claude-3", "runs_on": "ubuntu-latest", "cli": "claude",
                "secret": "CLAUDE_CODE_OAUTH_TOKEN_3", "shared": False, **changes}

    def given(self, **changes):
        return {"runs_on": "ubuntu-latest", "cli": "claude", "secret": "CLAUDE_CODE_OAUTH_TOKEN_3",
                "shared": "false", **changes}

    def test_the_dispatch_must_name_the_subscriptions_own_runner_cli_and_secret(self):
        ctx = loop_ctx(FakeGitHub())
        self.assertEqual(controller.check(ctx, self.planned(ctx), self.given()), [])
        # Another subscription's secret, or the bot's own token, never reaches a model job.
        for bad in ({"secret": "CLAUDE_CODE_OAUTH_TOKEN"}, {"secret": "BOT_GITHUB_TOKEN"},
                    {"runs_on": "night-vm-gpt"}, {"cli": "codex"}, {"shared": "true"}):
            self.assertTrue(controller.check(ctx, self.planned(ctx), self.given(**bad)), bad)
        self.assertTrue(controller.check(ctx, self.planned(ctx, secret="BOT_GITHUB_TOKEN"),
                                         self.given(secret="BOT_GITHUB_TOKEN")))
        self.assertTrue(controller.check(ctx, self.planned(ctx, provider="nobody"), self.given()))

    def test_fetch_reads_only_a_plan_reference(self):
        gh = FakeGitHub()
        ctx = loop_ctx(gh)
        for ref in ("../state", "plans/x", "abcd-1", ""):
            with self.assertRaises(ValueError):
                controller.fetch(ctx, ref)
        with self.assertRaises(ValueError):
            controller.fetch(ctx, "0123abcd-1760000000")  # no such file

    def test_plan_fetch_stops_the_job_on_a_mismatch(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        listing(gh)
        ctx = loop_ctx(gh)
        controller.start(ctx, sleep=no_sleep)
        ref = gh.dispatches[-1]["inputs"]["plan_ref"]
        args = main_mod.parser().parse_args(["plan-fetch", "--ref", ref, "--out", "/dev/null",
                                             "--check"])
        env = {"PLAN_RUNS_ON": "night-vm-muse", "PLAN_CLI": "muse", "PLAN_SECRET": "",
               "PLAN_SHARED": "false"}
        with mock.patch.object(main_mod, "_ctx", lambda cfg, write=True: ctx), \
                mock.patch.object(main_mod.config_mod, "github_env", lambda: env), \
                redirect_stdout(io.StringIO()):
            self.assertEqual(main_mod.cmd_plan_fetch(ctx.cfg, args), 0)
        env["PLAN_SECRET"] = "CLAUDE_CODE_OAUTH_TOKEN_3"
        with mock.patch.object(main_mod, "_ctx", lambda cfg, write=True: ctx), \
                mock.patch.object(main_mod.config_mod, "github_env", lambda: env), \
                mock.patch("sys.stderr", io.StringIO()):
            self.assertEqual(main_mod.cmd_plan_fetch(ctx.cfg, args), 2)


class PruneTests(unittest.TestCase):
    def test_plan_files_older_than_a_week_go(self):
        gh = FakeGitHub()
        ctx = loop_ctx(gh)
        gh.create_orphan_branch(STATE_BRANCH, "state.json", "{}", "state")
        old = f"0123abcd-{int((DAY - timedelta(days=8)).timestamp())}"
        new = f"4567abcd-{int((DAY - timedelta(days=1)).timestamp())}"
        for ref in (old, new):
            gh.put_file(f"plans/{ref}.json", "{}", branch=STATE_BRANCH, sha=None, message="plan")
        gh.put_file("plans/notes.txt", "x", branch=STATE_BRANCH, sha=None, message="other")
        self.assertEqual(controller.prune(ctx), ["pruned 1 plan file(s) older than 7 days"])
        self.assertIsNone(gh.get_file(f"plans/{old}.json", STATE_BRANCH)[0])
        self.assertIsNotNone(gh.get_file(f"plans/{new}.json", STATE_BRANCH)[0])
        self.assertIsNotNone(gh.get_file("plans/notes.txt", STATE_BRANCH)[0])


class WhileTheLoopGoesTests(unittest.TestCase):
    """Only the loop plans while it goes: every other path that used to start a run leaves the
    work to it, and bot-night's gate defers to it."""

    def alive(self, gh, ctx=None):
        gh.runs["900"] = {"id": 900, "status": "in_progress", "path": STATUS}
        if ctx is not None:
            ctx.store.update(lambda s: s.update(loop_tick=iso(ctx.now())))

    def test_a_loop_whose_ticks_stopped_counts_as_gone(self):
        gh = FakeGitHub()
        ctx = loop_ctx(gh, loop=False)
        self.alive(gh, ctx)
        self.assertTrue(plan_mod.loop_alive(ctx))
        later = make_ctx(gh, at=DAY + plan_mod.LOOP_STALE + timedelta(minutes=1), cfg=ctx.cfg)
        self.assertFalse(plan_mod.loop_alive(later))
        # Without `controller` nothing defers to a loop.
        off = make_ctx(gh, at=DAY, cfg=dataclasses.replace(ctx.cfg, controller=False))
        self.assertFalse(plan_mod.loop_alive(off))

    def test_the_loops_sweep_records_its_tick(self):
        gh = FakeGitHub()
        ctx = loop_ctx(gh)
        ctx.store.update(lambda s: s.update(last_sweep={"at": iso(DAY), "since": iso(DAY)}))
        with mock.patch.object(controller.time, "sleep", no_sleep):
            sweep.sweep(ctx)
        self.assertEqual(ctx.store.load()["loop_tick"], iso(DAY))

    def test_another_sweep_starts_no_run(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        ctx = loop_ctx(gh, loop=False)
        self.alive(gh, ctx)
        self.assertEqual(sweep._night_run(ctx, DAY), [])
        self.assertEqual(gh.dispatches, [])
        # With no loop going it starts a plain run, as before.
        del gh.runs["900"]
        self.assertEqual(len(sweep._night_run(ctx, DAY)), 1)
        self.assertEqual(gh.dispatches[0]["inputs"].get("plan_ref"), None)

    def test_the_loops_sweep_plans_and_starts_the_run(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        listing(gh)
        ctx = loop_ctx(gh)
        with mock.patch.object(controller.time, "sleep", no_sleep):
            notes = sweep._night_run(ctx, DAY)
        self.assertIn("started build #3 on muse (run 5002)", notes)

    def test_the_gate_defers_unless_a_person_asked_for_an_item(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        ctx = loop_ctx(gh, loop=False)
        self.alive(gh, ctx)
        outputs = {}
        with mock.patch.object(main_mod, "_ctx", lambda cfg, write=True: ctx), \
                mock.patch.object(main_mod, "_output", outputs.update), \
                mock.patch.object(main_mod, "_summary", lambda text: None), \
                redirect_stdout(io.StringIO()):
            main_mod.cmd_peek(ctx.cfg, main_mod.parser().parse_args(["peek"]))
            self.assertEqual(outputs["work"], "false")
            main_mod.cmd_peek(ctx.cfg, main_mod.parser().parse_args(["peek", "--item", "3"]))
            self.assertEqual(outputs["work"], "true")

    def test_a_plan_job_waits_for_the_lock_then_plans_nothing(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        ctx = loop_ctx(gh, loop=False)
        ctx.store.update(lambda s: s.update(planning={"owner": "loop 1", "at": iso(DAY)}))
        args = main_mod.parser().parse_args(["plan", "--out", "/dev/null"])
        with mock.patch.object(main_mod, "_ctx", lambda cfg, write=True: ctx), \
                mock.patch.object(main_mod, "LOCK_TRIES", 1), \
                mock.patch.object(main_mod, "_output", lambda pairs: None), \
                mock.patch.object(main_mod, "_summary", lambda text: None), \
                mock.patch.object(main_mod.Path, "write_text", lambda *a, **k: None), \
                redirect_stdout(io.StringIO()) as out:
            main_mod.cmd_plan(ctx.cfg, args)
        self.assertIn("the status loop is planning", out.getvalue())
        self.assertNotIn(LABEL_WORKING, gh.label_names(3))

    def test_delivery_leaves_the_next_run_to_the_loop(self):
        gh = FakeGitHub()
        ctx = loop_ctx(gh, loop=False)
        self.alive(gh, ctx)
        deliverer = deliver_mod.Deliverer.__new__(deliver_mod.Deliverer)
        deliverer.ctx, deliverer.chain, deliverer.log = ctx, True, []
        deliverer._chain()
        self.assertEqual(deliverer.log, ["the status loop starts the next run (#1455)"])
        self.assertEqual(gh.dispatches, [])


if __name__ == "__main__":
    unittest.main()
