"""The bot machine's disk (harness/disk.py, bot/machine/clean.sh): a job's readings, the room it
makes, the refusal under the floor, the pause on a full disk, and the one issue for people."""

from __future__ import annotations

import errno
import json
import os
import subprocess
import tempfile
import time
import unittest
from datetime import datetime, timezone
from pathlib import Path
from unittest import mock

from harness import disk
from harness import plan as plan_mod
from harness.config import LABEL_BUILD, LABEL_HUMAN
from harness.deliver import Deliverer
from harness.runner import FakeRunner, RunRequest, RunResult
from harness.work import Worker

from tests.fakes import FakeGitHub, make_origin
from tests.support import ROOT, make_config, make_ctx
from tests.test_flow import Harness
from tests.test_work import APPROVE, GATES, builder, reviewer

GIB = disk.GIB


def found(percent: int, free_gib: float, at: str = "2026-10-04T20:43:51Z",
          total_gib: float = 30) -> dict:
    return {"total": int(total_gib * GIB), "free": int(free_gib * GIB), "percent": percent,
            "at": at}


class RulesTests(unittest.TestCase):
    def test_too_full_is_the_share_used_or_the_floor(self):
        self.assertTrue(disk.too_full(found(80, 6)))
        self.assertTrue(disk.too_full(found(60, 2.9)))   # a big disk with little left
        self.assertFalse(disk.too_full(found(79, 6)))
        self.assertTrue(disk.recovered(found(70, 9)))
        self.assertFalse(disk.recovered(found(75, 7)))   # between the lines: neither
        self.assertFalse(disk.too_full(found(75, 7)))

    def test_a_full_disk_in_any_error(self):
        self.assertTrue(disk.is_full(OSError(errno.ENOSPC, "No space left on device")))
        self.assertTrue(disk.is_full(RuntimeError("git commit: fatal: No space left on device")))
        self.assertFalse(disk.is_full(OSError(errno.ENOENT, "missing")))

    def test_a_reading_is_dfs_share(self):
        got = disk.reading(Path(tempfile.gettempdir()), datetime.now(timezone.utc))
        self.assertTrue(0 <= got["percent"] <= 100)
        self.assertLessEqual(got["free"], got["total"])

    def test_only_valid_readings_from_a_result_count_and_the_newest_wins(self):
        result = {"disk": {"start": found(95, 1), "after_clean": found(60, 12),
                           "end": {"total": "lots", "free": 1, "percent": 50}}}
        self.assertEqual(disk.newest(result)["percent"], 60)
        self.assertIsNone(disk.newest({"disk": {"start": {"total": 1, "free": 5, "percent": 3}}}))
        self.assertIsNone(disk.newest({"disk": "full"}))
        self.assertIsNone(disk.newest({}))

    def test_an_older_reading_never_replaces_a_newer_one(self):
        state: dict = {}
        disk.note(state, found(50, 15, "2026-10-04T21:00:00Z"), "night-vm-devin", "")
        disk.note(state, found(90, 3, "2026-10-04T20:00:00Z"), "night-vm-muse", "")
        self.assertEqual(state["machine_disk"]["reading"]["percent"], 50)
        disk.note(state, found(85, 4, "2026-10-04T22:00:00Z"), "night-vm-muse-2", "u")
        self.assertEqual(state["machine_disk"]["runner"], "night-vm-muse-2")


class AlertTests(unittest.TestCase):
    def setUp(self):
        self.gh = FakeGitHub()
        self.ctx = make_ctx(self.gh)

    def keep(self, reading: dict, runner: str = "night-vm-muse-2") -> None:
        self.ctx.store.update(lambda s: disk.note(s, reading, runner, "https://run/1"), "test")

    def issues(self) -> list[dict]:
        return [i for i in self.gh.list_issues(labels="", state="all") if i["title"] == disk.TITLE]

    def test_it_opens_once_rewrites_with_new_readings_and_closes_when_back_under(self):
        self.keep(found(55, 13))
        self.assertIsNone(disk.alert(self.ctx))
        self.assertEqual(self.issues(), [])

        self.keep(found(97, 0.06, "2026-10-04T20:44:00Z"))
        self.assertIn("opened", disk.alert(self.ctx))
        [issue] = self.issues()
        self.assertEqual(self.gh.label_names(issue["number"]), {"night bot", LABEL_HUMAN})
        self.assertIn("97% full, 0.1 GB free of 30.0 GB", issue["body"])
        self.assertIn("bot/machine/on-machine.sh bot/machine/disk-report.sh", issue["body"])
        self.assertIsNone(disk.alert(self.ctx))  # the same reading again: nothing to say

        self.keep(found(91, 2.7, "2026-10-04T20:55:00Z"))
        self.assertIn("rewrote", disk.alert(self.ctx))
        self.assertEqual(len(self.issues()), 1)
        self.assertIn("91% full", self.gh.get_issue(issue["number"])["body"])

        self.keep(found(75, 7, "2026-10-04T21:05:00Z"))  # better, not yet back under
        self.assertIsNone(disk.alert(self.ctx))
        self.assertEqual(self.gh.get_issue(issue["number"])["state"], "open")

        self.keep(found(42, 17, "2026-10-04T21:15:00Z"))
        self.assertIn("closed", disk.alert(self.ctx))
        self.assertEqual(self.gh.get_issue(issue["number"])["state"], "closed")
        self.assertIn("back to 42% full", self.gh.bot_comments(issue["number"])[-1])
        self.assertIsNone(self.ctx.store.load()["machine_disk"]["issue"])

    def test_a_person_who_closed_it_is_not_overruled_by_the_same_reading(self):
        self.keep(found(90, 3, "2026-10-04T20:00:00Z"))
        disk.alert(self.ctx)
        [issue] = self.issues()
        self.gh.update_issue(issue["number"], state="closed", closed_at="2026-10-04T20:30:00Z")
        self.assertIsNone(disk.alert(self.ctx))
        self.assertEqual(len(self.issues()), 1)
        self.keep(found(92, 2.5, "2026-10-04T21:00:00Z"))  # a newer reading, still too full
        self.assertIn("opened", disk.alert(self.ctx))
        self.assertEqual(len(self.issues()), 2)

    def test_the_status_issue_shows_the_reading(self):
        self.assertIsNone(disk.line(self.ctx.store.load()))
        self.keep(found(40, 18))
        self.assertIn("🟢 **The machine's disk:** 40% full, 18.0 GB free of 30.0 GB",
                      disk.line(self.ctx.store.load()))


class WorkerTests(unittest.TestCase):
    """On the machine a model job reads its disk, cleans when short and stops under the floor."""

    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        self.origin, self.clone = make_origin(self.root)
        self.cfg = make_config(gates=GATES, install={"run": "true", "timeout_minutes": 1})

    def worker(self, runner, readings: list[dict], runs_on: str = "night-vm-muse") -> Worker:
        plan = {"action": "build", "number": 12, "title": "Rules v2", "branch": "bot/issue-12",
                "thread": "Please make the rules v2.", "runs_on": runs_on}
        queue = list(readings)
        return Worker(self.cfg, plan, runner, self.clone, self.root / "work", self.root / "out",
                      disk_reader=lambda: queue.pop(0) if len(queue) > 1 else queue[0])

    def test_under_the_floor_after_cleaning_it_does_no_work_and_blames_the_machine(self):
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"})})
        with mock.patch.object(disk, "clean", return_value="cleaned") as clean:
            result = self.worker(runner, [found(99, 0.06), found(97, 1)]).run()
        clean.assert_called_once()
        self.assertEqual(runner.calls, [])
        self.assertEqual(result["status"], "infra")
        self.assertEqual(result["infra_scope"], "machine")
        self.assertIn("97% full, 1.0 GB free", result["reason"])
        self.assertEqual(result["disk"]["cleaned"], "cleaned")
        self.assertEqual(set(result["disk"]) - {"runner"}, {"start", "after_clean", "cleaned", "end"})

    def test_room_enough_needs_no_clean_and_the_work_goes_on(self):
        runner = FakeRunner({"build": builder({"src/game.txt": "rules v2\n"}),
                             "review": reviewer(APPROVE)})
        with mock.patch.object(disk, "clean") as clean:
            result = self.worker(runner, [found(40, 18)]).run()
        clean.assert_not_called()
        self.assertEqual(result["status"], "approved")
        self.assertEqual(result["disk"]["start"]["percent"], 40)
        self.assertIn("end", result["disk"])

    def test_cleaning_that_makes_room_lets_it_start(self):
        runner = FakeRunner({"build": builder({"src/game.txt": "rules v2\n"}),
                             "review": reviewer(APPROVE)})
        with mock.patch.object(disk, "clean", return_value="ok"):
            result = self.worker(runner, [found(85, 4), found(60, 12)]).run()
        self.assertEqual(result["status"], "approved")

    def test_a_disk_that_fills_up_mid_run_pauses_and_keeps_the_work(self):
        def fills_up(request: RunRequest) -> RunResult:
            (request.cwd / "src" / "game.txt").write_text("half of v2\n")
            raise OSError(errno.ENOSPC, "No space left on device")
        result = self.worker(FakeRunner({"build": fills_up}), [found(50, 15)]).run()
        self.assertEqual(result["status"], "interrupted")
        self.assertEqual(result["interrupt"], "disk")
        self.assertIn("the machine's disk filled up", result["reason"])

    def test_githubs_runners_read_no_disk(self):
        runner = FakeRunner({"build": builder({"src/game.txt": "rules v2\n"}),
                             "review": reviewer(APPROVE)})
        result = self.worker(runner, [found(99, 0.01)], runs_on="ubuntu-latest").run()
        self.assertEqual(result["status"], "approved")
        self.assertNotIn("disk", result)


class DeliverTests(unittest.TestCase):
    def test_a_refusal_keeps_the_item_queued_uncounted_and_opens_the_issue(self):
        h = Harness(self, machine=("muse",))
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned = plan_mod.make(h.ctx)
        out = h.root / "out-full"
        out.mkdir()
        (out / "result.json").write_text(json.dumps({
            "status": "infra", "interrupt": "infra", "infra_scope": "machine",
            "reason": "the machine's disk is 97% full, 1.0 GB free of 30.0 GB, under the 3.0 GB "
                      "a job needs, even after cleaning",
            "disk": {"runner": "night-vm-muse-2", "start": found(99, 0.06),
                     "after_clean": found(97, 1, "2026-10-04T20:44:30Z")}}))
        Deliverer(h.ctx, {**planned, "provider": "muse"}, out, h.deliver_repo,
                  action="build", number=12, provider="muse").run()
        self.assertEqual(h.gh.label_names(12), {LABEL_BUILD})
        self.assertIn("not this item's fault", h.gh.bot_comments(12)[-1])
        state = h.ctx.store.load()
        self.assertEqual(state["items"]["12"].get("failures", 0), 0)
        self.assertEqual(state["machine_disk"]["runner"], "night-vm-muse-2")
        self.assertEqual(state["machine_disk"]["reading"]["percent"], 97)
        opened = [i for i in h.gh.list_issues(labels="") if i["title"] == disk.TITLE]
        self.assertEqual(len(opened), 1)
        self.assertEqual(state["machine_disk"]["issue"], opened[0]["number"])

    def test_a_job_on_githubs_runners_reports_nothing(self):
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned = plan_mod.make(h.ctx)
        out = h.root / "out-hosted"
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}), "review": reviewer(APPROVE)})
        Worker(h.cfg, planned, runner, h.clone, h.root / "work", out).run()
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        self.assertNotIn("machine_disk", h.ctx.store.load())


SCRIPT = ROOT / "bot" / "machine" / "clean.sh"


@unittest.skipIf(os.geteuid() == 0, "clean.sh runs only as the user it cleans, never as root")
class CleanScriptTests(unittest.TestCase):
    """clean.sh, run as it runs on the machine: as the user, in a home of its own."""

    def setUp(self):
        self.home = Path(tempfile.mkdtemp())
        self.tmp = Path(tempfile.mkdtemp())
        self.bin = Path(tempfile.mkdtemp())
        self.workers = ""
        self.pgrep_status: int | None = None
        self.runner = self.home / "actions-runner"
        self.lane2 = self.home / "actions-runner-2"

    def make(self, path: Path, hours_old: float = 0, content: str = "x") -> Path:
        """A file holding `content` (a folder for "dir"), last changed `hours_old` ago."""
        path.parent.mkdir(parents=True, exist_ok=True)
        if content == "dir":
            path.mkdir(exist_ok=True)
        elif not path.exists():
            path.write_text(content)
        when = time.time() - hours_old * 3600
        os.utime(path, (when, when))
        return path

    def run_script(self, *args: str, runner_temp: Path | None = None) -> str:
        # A pgrep that answers with the jobs this test says are running, as pgrep -a does: one
        # line each, exit 1 for none, 2 when it cannot tell.
        pgrep = self.bin / "pgrep"
        lines = self.workers.strip()
        status = self.pgrep_status if self.pgrep_status is not None else (0 if lines else 1)
        pgrep.write_text("#!/bin/bash\n" + (f"cat <<'EOF'\n{lines}\nEOF\n" if lines else "")
                         + f"exit {status}\n")
        pgrep.chmod(0o755)
        env = {"HOME": str(self.home), "PATH": f"{self.bin}:/usr/bin:/bin",
               "NIGHT_VM_TMP": str(self.tmp)}
        if runner_temp:
            env["RUNNER_TEMP"] = str(runner_temp)
        proc = subprocess.run(["bash", str(SCRIPT), *args], env=env, capture_output=True,
                              text=True, timeout=60, check=True)
        return proc.stdout + proc.stderr

    def lay_out_a_runner(self, runner: Path) -> None:
        for kind in ("bin", "externals"):
            self.make(runner / f"{kind}.2.337.0", 30, "dir")
            self.make(runner / f"{kind}.2.336.0", 50, "dir")
            self.make(runner / f"{kind}.2.338.0", 0.1, "dir")  # an update in progress
            (runner / kind).symlink_to(runner / f"{kind}.2.337.0")
        self.make(runner / "_diag" / "Worker_20261001.log", 72)
        self.make(runner / "_diag" / "Worker_20261004.log", 1)
        self.make(runner / "_work" / "_update" / "runner.tar.gz", 5)
        self.make(runner / "_work" / "_update", 5, "dir")
        self.make(runner / "_work" / "_temp" / "left-by-a-dead-job" / "node_modules.txt", 3)
        self.make(runner / "_work" / "_temp" / "left-by-a-dead-job", 3, "dir")

    def test_it_gives_back_old_runner_versions_logs_leftovers_and_idle_caches(self):
        self.lay_out_a_runner(self.runner)
        store = self.make(self.home / ".local/share/pnpm/store/v11/index.txt")
        old_tmp = self.make(self.tmp / "vitest-old", 7)
        new_tmp = self.make(self.tmp / "vitest-new", 1)
        said = self.run_script()
        self.assertIn("night-vm-clean:", said)
        for kind in ("bin", "externals"):
            self.assertTrue((self.runner / f"{kind}.2.337.0").exists(), "the current version stays")
            self.assertTrue((self.runner / f"{kind}.2.338.0").exists(), "an update's stays")
            self.assertFalse((self.runner / f"{kind}.2.336.0").exists(), "the replaced one goes")
            self.assertTrue((self.runner / kind).is_symlink())
        self.assertFalse((self.runner / "_diag" / "Worker_20261001.log").exists())
        self.assertTrue((self.runner / "_diag" / "Worker_20261004.log").exists())
        self.assertFalse((self.runner / "_work" / "_update").exists())
        self.assertFalse((self.runner / "_work" / "_temp" / "left-by-a-dead-job").exists())
        self.assertFalse(store.exists(), "no job is running, so the package store goes")
        self.assertFalse(old_tmp.exists())
        self.assertTrue(new_tmp.exists())

    def test_a_running_job_keeps_its_runner_temp_and_the_shared_store(self):
        self.lay_out_a_runner(self.runner)
        self.lay_out_a_runner(self.lane2)
        store = self.make(self.home / ".local/share/pnpm/store/v11/index.txt")
        # Lane 2 is running a job; this run is no job of its own (the disk timer).
        self.workers = f"4242 {self.lane2}/bin.2.337.0/Runner.Worker spawnclient 1 2"
        self.run_script()
        self.assertTrue(store.exists(), "another job's install reads the store")
        self.assertTrue((self.lane2 / "_work" / "_temp" / "left-by-a-dead-job").exists())
        self.assertFalse((self.runner / "_work" / "_temp" / "left-by-a-dead-job").exists())

    def test_after_a_job_its_own_files_go_and_the_store_goes_only_with_the_last_job(self):
        temp = self.runner / "_work" / "_temp"
        self.make(temp / "work" / "item-12" / "node_modules.txt")
        store = self.make(self.home / ".local/share/pnpm/store/v11/index.txt")
        self.workers = (f"11 {self.runner}/bin/Runner.Worker spawnclient 1 2\n"
                        f"12 {self.lane2}/bin/Runner.Worker spawnclient 3 4")
        self.run_script("--job", runner_temp=temp)
        self.assertEqual(list(temp.iterdir()), [])
        self.assertTrue(store.exists(), "lane 2 still has a job going")
        self.workers = f"11 {self.runner}/bin/Runner.Worker spawnclient 1 2"
        self.run_script("--job", runner_temp=temp)
        self.assertFalse(store.exists(), "the user's last job takes the store with it")

    def test_devins_session_database_goes_with_its_last_job_and_its_login_stays(self):
        cli = self.home / ".local/share/devin/cli"
        db = self.make(cli / "sessions.db", 1, "x" * 100)
        login = self.make(cli / "credentials.toml", 100)
        old_log = self.make(cli / "logs" / "a.log", 9)
        new_log = self.make(cli / "logs" / "b.log", 1)
        self.workers = f"7 {self.lane2}/bin/Runner.Worker spawnclient 1 2"
        self.run_script()
        self.assertTrue(db.exists(), "another Devin job may have it open")
        self.assertFalse(old_log.exists())
        self.assertTrue(new_log.exists())
        self.workers = ""
        self.run_script()
        self.assertFalse(db.exists())
        self.assertTrue(login.exists())

    def test_when_it_cannot_tell_which_jobs_run_it_leaves_their_files_alone(self):
        self.lay_out_a_runner(self.runner)
        store = self.make(self.home / ".local/share/pnpm/store/v11/index.txt")
        self.pgrep_status = 2
        self.run_script()
        self.assertTrue(store.exists())
        self.assertTrue((self.runner / "_work" / "_temp" / "left-by-a-dead-job").exists())
        self.assertFalse((self.runner / "bin.2.336.0").exists(), "no job uses a replaced version")

    def test_setup_installs_it_as_the_hook_and_the_timer(self):
        setup = (ROOT / "bot" / "machine" / "setup.sh").read_text()
        self.assertIn('install -m 755 "$here/clean.sh" /usr/local/bin/night-vm-clean.sh', setup)
        self.assertIn("/usr/local/bin/night-vm-clean.sh --job || true", setup)
        self.assertIn("night-vm-disk.timer", setup)
        self.assertIn("OnUnitActiveSec=10min", setup)
        on_machine = (ROOT / "bot" / "machine" / "on-machine.sh").read_text()
        self.assertIn("tar czf - -C \"$(dirname \"$script\")\"", on_machine)


if __name__ == "__main__":
    unittest.main()
