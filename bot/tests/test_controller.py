"""The status loop's planner (`controller.py`, #1455): with `controller` on, bot-status's loop plans
the work and starts each night run with its plan, so the run skips its gate and plan jobs; and it
delivers each such run once it ends (#1537), so the run skips its deliver job too."""

from __future__ import annotations

import dataclasses
import io
import json
import unittest
import urllib.error
import zipfile
from contextlib import redirect_stdout
from datetime import timedelta
from pathlib import Path
from unittest import mock

from harness import __main__ as main_mod
from harness import controller, deliver as deliver_mod, gh as gh_mod, plan as plan_mod, providers
from harness import sweep
from harness.clock import iso
from harness.config import LABEL_BUILD, LABEL_PR, LABEL_WORKING, STATE_BRANCH
from harness.errors import GitHubError
from harness.runner import FakeRunner
from harness.state import item as state_item
from harness.work import Worker

from tests.fakes import FakeGitHub
from tests.support import DAY, NIGHT, make_config, make_ctx
from tests.test_flow import Harness
from tests.test_providers import secrets
from tests.test_work import APPROVE, builder, reviewer

STATUS = ".github/workflows/bot-status.yml"
NIGHT_YML = Path(__file__).resolve().parents[2] / ".github" / "workflows" / "bot-night.yml"


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


def zipped(folder: Path, extra: dict[str, bytes] | None = None) -> bytes:
    """`folder` as GitHub hands back an upload of it: a zip of its files, by relative path."""
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        for path in sorted(folder.rglob("*")):
            if path.is_file():
                archive.write(path, path.relative_to(folder).as_posix())
        for name, data in (extra or {}).items():
            archive.writestr(name, data)
    return buffer.getvalue()


def ended(gh: FakeGitHub, run_id: str, at) -> None:
    gh.runs[run_id].update(status="completed", conclusion="success", updated_at=iso(at))


class DeliveryTests(unittest.TestCase):
    """The loop delivers the runs it started (#1537): bot-night gives them no deliver job, which
    waited 7 to 25 minutes for a hosted runner to run for under a minute."""

    def started(self):
        """A build of #12 the loop started at night, its model job done: (harness, ctx, run id)."""
        h = Harness(self, controller=True)
        ctx = make_ctx(h.gh, at=NIGHT, cfg=dataclasses.replace(h.cfg, controller_loop=True,
                                                               run_id="777"))
        h.gh.add_issue(12, "Make the rules v2", labels=(LABEL_BUILD,))
        listing(h.gh)
        notes = controller.start(ctx, sleep=no_sleep)
        [started] = [n for n in notes if n.startswith("started")]
        run_id = started.rsplit("run ", 1)[1].rstrip(")")
        ref = h.gh.dispatches[-1]["inputs"]["plan_ref"]
        planned = controller.fetch(ctx, ref)
        out = h.root / "out"
        runner = FakeRunner({"build": builder({"src/game.txt": "rules v2\n"}),
                             "review": reviewer(APPROVE)})
        Worker(h.cfg, planned, runner, h.clone, h.root / "work", out).run()
        h.gh.artifacts[run_id] = [{"id": 1, "name": "work", "expired": False}]
        h.gh.artifact_zips[1] = zipped(out)
        ended(h.gh, run_id, NIGHT)
        return h, ctx, run_id

    def test_the_loop_delivers_a_run_it_started_within_a_tick_of_its_work_ending(self):
        h, ctx, run_id = self.started()
        # Until it is delivered the run holds its lane and housekeeping leaves its item alone.
        self.assertEqual(plan_mod.run_status(ctx, run_id), "alive")
        self.assertNotIn("requeued #12 from a dead run", plan_mod.housekeeping(ctx, ctx.store.load()))
        notes = controller.deliver(ctx, workspace=h.root / "scratch")
        self.assertEqual(notes, [f"delivered build #12 on claude-1 (run {run_id}): approved"])
        # The same delivery the run's own deliver job made: the branch and an auto-merging pull
        # request.
        result = json.loads((h.root / "out" / "result.json").read_text())
        self.assertEqual(h.origin_sha("bot/issue-12"), result["head"])
        [pull] = h.gh.list_pulls(head="bot/issue-12")
        self.assertIn(pull["node_id"], h.gh.auto_merge)
        self.assertIn(LABEL_PR, h.gh.label_names(int(pull["number"])))
        record = ctx.store.load()[plan_mod.DELIVERIES][run_id]
        self.assertEqual((record["done"], record["tries"], record["status"]), (True, 1, "approved"))
        # Delivered once only; the scratch clone is gone; the run is over.
        self.assertEqual(controller.deliver(ctx, workspace=h.root / "scratch"), [])
        self.assertEqual(list((h.root / "scratch").iterdir()), [])
        self.assertEqual(plan_mod.run_status(ctx, run_id), "dead")
        # The loop plans the next run itself, so the delivery started none.
        self.assertEqual(len(h.gh.dispatches), 1)

    def test_a_run_whose_model_job_left_no_upload_is_delivered_as_one_with_no_result(self):
        h, ctx, run_id = self.started()
        h.gh.artifacts[run_id] = []
        [note] = controller.deliver(ctx, workspace=h.root / "scratch")
        self.assertEqual(note, f"delivered build #12 on claude-1 (run {run_id}): infra "
                               f"(its model job left no upload)")
        [said] = [c for c in h.gh.bot_comments(12) if deliver_mod.NO_RESULT in c]
        # Its link names the night run, not the loop's.
        self.assertIn(f"actions/runs/{run_id})", said)
        self.assertNotIn("actions/runs/777", said)

    def failing(self):
        """A run the loop started for #3, ended, whose plan file is missing: every try fails."""
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_WORKING,))
        ctx = loop_ctx(gh)
        ctx.store.update(lambda s: state_item(s, 3).update(run_id="5002", provider="muse"))
        gh.runs["5002"] = {"id": 5002, "status": "completed", "updated_at": iso(DAY),
                           "display_title": "bot-night 0123abcd-1760000000 on night-vm-muse"}
        return gh, ctx

    def test_a_failing_delivery_is_tried_each_tick_then_left_to_housekeeping(self):
        gh, ctx = self.failing()
        for attempt in (1, 2):
            [note] = controller.deliver(ctx, workspace=Path(self.id()).parent / "unused")
            self.assertTrue(note.startswith(f"could not deliver run 5002, try {attempt} of 3 "
                                            f"(ValueError: no plan"), note)
            self.assertTrue(note.endswith(f"it is tried again next tick ({3 - attempt} more)"), note)
            # Meanwhile the item is not requeued.
            self.assertEqual(plan_mod.run_status(ctx, "5002"), "alive")
        [note] = controller.deliver(ctx)
        self.assertTrue(note.endswith("its item is left to housekeeping, as a dead run's"), note)
        self.assertEqual(controller.deliver(ctx), [])
        self.assertEqual(plan_mod.run_status(ctx, "5002"), "dead")
        self.assertIn("requeued #3 from a dead run", plan_mod.housekeeping(ctx, ctx.store.load()))
        self.assertEqual(gh.label_names(3), {LABEL_BUILD})

    def test_a_run_the_loop_never_delivered_is_housekeepings_after_the_window(self):
        gh, ctx = self.failing()
        self.assertEqual(plan_mod.run_status(ctx, "5002"), "alive")
        later = make_ctx(gh, at=DAY + plan_mod.DELIVER_WITHIN, cfg=ctx.cfg)
        self.assertEqual(plan_mod.run_status(later, "5002"), "dead")
        self.assertEqual(controller.deliver(later), [])
        # A run nobody started with a plan never waits for the loop.
        gh.runs["5003"] = {"id": 5003, "status": "completed", "updated_at": iso(DAY),
                           "display_title": "bot-night"}
        self.assertEqual(plan_mod.run_status(ctx, "5003"), "dead")

    def test_nothing_in_a_delivery_ends_the_tick(self):
        gh, ctx = self.failing()

        def refuse(*args, **kwargs):
            raise GitHubError("no", 500)

        gh.list_runs = refuse
        self.assertEqual(controller.deliver(ctx), ["could not list the night runs to deliver: no"])

    def test_an_upload_holding_a_path_outside_it_is_refused(self):
        h, ctx, run_id = self.started()
        h.gh.artifact_zips[1] = zipped(h.root / "out", {"../escaped.txt": b"x"})
        [note] = controller.deliver(ctx, workspace=h.root / "scratch")
        self.assertIn("the upload holds a path outside it", note)
        self.assertFalse((h.root / "scratch" / "escaped.txt").exists())
        self.assertIsNone(h.origin_sha("bot/issue-12"))

    def test_the_loops_sweep_delivers_before_it_plans(self):
        gh = FakeGitHub()
        ctx = loop_ctx(gh)
        ctx.store.update(lambda s: s.update(last_sweep={"at": iso(DAY), "since": iso(DAY)}))
        order = []
        with mock.patch.object(controller, "deliver", lambda c: order.append("deliver") or []), \
                mock.patch.object(sweep, "_night_run", lambda c, since: order.append("plan") or []):
            sweep.sweep(ctx)
        self.assertEqual(order, ["deliver", "plan"])
        # Any other sweep leaves the delivering to the loop.
        other = make_ctx(gh, at=DAY, cfg=dataclasses.replace(ctx.cfg, controller_loop=False))
        with mock.patch.object(controller, "deliver", lambda c: self.fail("delivered")):
            self.assertEqual(sweep._deliveries(other, DAY), [])

    def test_bot_night_gives_a_run_the_loop_started_no_deliver_job(self):
        text = NIGHT_YML.read_text(encoding="utf-8")
        job = text[text.index("\n  deliver:\n"):]
        self.assertIn("if: ${{ always() && !inputs.plan_ref && contains(", job)
        # The model job's upload keeps the name the loop downloads.
        self.assertIn(f"name: {controller.WORK_ARTIFACT}\n", text[:text.index("\n  deliver:\n")])


class DownloadTests(unittest.TestCase):
    def test_an_artifact_is_fetched_from_its_redirect_without_the_token(self):
        client = gh_mod.GitHub("jgoetzmann/JackiOh", "secret-token")
        asked = []

        class Redirecting:
            def open(self, request, timeout=None):
                asked.append(request)
                raise urllib.error.HTTPError(request.full_url, 302, "Found",
                                             {"Location": "https://blob.example/work.zip"}, None)

        def fetch(url, timeout=None):
            asked.append(url)
            return io.BytesIO(b"zip bytes")

        with mock.patch.object(gh_mod.urllib.request, "build_opener", lambda *a: Redirecting()), \
                mock.patch.object(gh_mod.urllib.request, "urlopen", fetch):
            self.assertEqual(client.download_artifact(7), b"zip bytes")
        self.assertTrue(asked[0].full_url.endswith("/repos/jgoetzmann/JackiOh/actions/artifacts/7/zip"))
        self.assertEqual(asked[0].get_header("Authorization"), "Bearer secret-token")
        # The storage URL is a plain string: no header, so no token, goes with it.
        self.assertEqual(asked[1], "https://blob.example/work.zip")


if __name__ == "__main__":
    unittest.main()
