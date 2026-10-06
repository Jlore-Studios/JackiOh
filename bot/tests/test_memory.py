"""#312: the machine's memory, read by its own jobs, kept by deliver and shown on the status issue.

CloudWatch keeps only the night box's CPU, and the box powers off when idle, so nothing showed
whether a busy day ran it short of memory. Each machine job now reads the kernel's memory stall
time, swapping and available memory as it goes.
"""

from __future__ import annotations

import json
import tempfile
import unittest
from datetime import timedelta
from pathlib import Path

from harness import dashboard, memory
from harness.clock import iso
from harness.runner import FakeRunner
from harness.work import Worker

from tests.fakes import make_origin
from tests.support import DAY, make_config, make_ctx
from tests.test_work import APPROVE, GATES, builder, reviewer

GIB = 1024 ** 3


def proc(root: Path, *, available_kib: int, swap_free_kib: int, swapped: int, some_us: int,
         full_us: int) -> Path:
    (root / "pressure").mkdir(parents=True, exist_ok=True)
    (root / "meminfo").write_text(
        f"MemTotal:        8000000 kB\nMemFree:          100000 kB\n"
        f"MemAvailable:    {available_kib} kB\nSwapTotal:       8000000 kB\n"
        f"SwapFree:        {swap_free_kib} kB\n")
    (root / "vmstat").write_text(f"nr_free_pages 1000\npswpin {swapped}\npswpout 0\n")
    (root / "pressure" / "memory").write_text(
        f"some avg10=0.00 avg60=0.00 avg300=0.00 total={some_us}\n"
        f"full avg10=0.00 avg60=0.00 avg300=0.00 total={full_us}\n")
    return root


class ReadingTests(unittest.TestCase):
    def test_it_reads_the_kernels_counters(self):
        root = proc(Path(tempfile.mkdtemp()), available_kib=4_000_000, swap_free_kib=7_000_000,
                    swapped=12, some_us=2_500_000, full_us=1_000_000)
        found = memory.reading(DAY, root)
        self.assertEqual((found["total"], found["available"], found["swap_used"]),
                         (8_000_000 * 1024, 4_000_000 * 1024, 1_000_000 * 1024))
        self.assertEqual((found["swapped"], found["stall_some"], found["stall_full"]),
                         (12, 2.5, 1.0))

    def test_no_kernel_figures_is_no_reading(self):
        self.assertIsNone(memory.reading(DAY, Path(tempfile.mkdtemp())))

    def test_a_jobs_run_is_the_change_between_its_readings(self):
        record: dict = {}
        base = {"total": 8 * GIB, "swap_used": 0, "swapped": 100, "stall_some": 10.0,
                "stall_full": 2.0, "at": iso(DAY)}
        memory.add(record, {**base, "available": 5 * GIB})
        memory.add(record, {**base, "available": 1 * GIB, "swap_used": GIB, "stall_full": 9.5})
        memory.add(record, {**base, "available": 3 * GIB, "swapped": 400, "stall_some": 30.0,
                            "stall_full": 12.0, "at": iso(DAY + timedelta(minutes=20))})
        found = memory.summary(record)
        self.assertEqual((found["low_available"], found["high_swap"], found["swapped"]),
                         (GIB, GIB, 300))
        self.assertEqual((found["stall_some"], found["stall_full"]), (20.0, 10.0))
        self.assertIsNone(memory.summary({}))


class WorkerTests(unittest.TestCase):
    def run_on(self, runs_on: str, readings: list[dict]) -> dict:
        root = Path(tempfile.mkdtemp())
        _, clone = make_origin(root)
        cfg = make_config(gates=GATES, install={"run": "true", "timeout_minutes": 1})
        plan = {"action": "build", "number": 12, "title": "Rules v2", "branch": "bot/issue-12",
                "thread": "Please make the rules v2.", "runs_on": runs_on}
        queue = list(readings)
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                             "review": reviewer(APPROVE)})
        Worker(cfg, plan, runner, clone, root / "work", root / "out",
               memory_reader=lambda: queue.pop(0) if len(queue) > 1 else queue[0]).run()
        return json.loads((root / "out" / "result.json").read_text())

    def test_a_machine_job_records_its_memory(self):
        base = {"total": 8 * GIB, "swap_used": 0, "swapped": 0, "stall_some": 0.0,
                "stall_full": 0.0, "at": iso(DAY)}
        result = self.run_on("night-vm-muse", [{**base, "available": 6 * GIB},
                                               {**base, "available": 2 * GIB, "stall_full": 7.0}])
        self.assertGreaterEqual(result["memory"]["readings"], 3)  # start, each call, checks, end
        self.assertEqual(memory.summary(result["memory"])["stall_full"], 7.0)
        self.assertEqual(result["memory"]["low_available"], 2 * GIB)

    def test_a_job_on_githubs_runners_reads_nothing(self):
        result = self.run_on("ubuntu-latest", [{"total": 1, "available": 1, "swap_used": 0,
                                                "at": iso(DAY)}])
        self.assertNotIn("memory", result)


@unittest.skipUnless(Path("/proc/meminfo").is_file(), "the kernel's counters are Linux's")
class DeliverTests(unittest.TestCase):
    def test_deliver_keeps_a_machine_runs_memory(self):
        from tests.test_flow import Harness
        from harness.config import LABEL_BUILD
        h = Harness(self, env={"HARNESS_SECRETS_SET": ""}, machine=("muse",), at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned, _ = h.night(FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                                         "review": reviewer(APPROVE)}))
        self.assertEqual(planned["provider"], "muse")
        kept = h.ctx.store.load()["machine_memory"]
        self.assertEqual(len(kept), 1)
        self.assertGreater(kept[0]["total"], 0)
        self.assertEqual(kept[0]["runner"], "night-vm-muse")


class LineTests(unittest.TestCase):
    def entry(self, hours_ago: float, **values) -> dict:
        return {"at": iso(DAY - timedelta(hours=hours_ago)), "total": 8 * GIB,
                "low_available": 3 * GIB, "high_swap": 0, "stall_some": 0.5, "stall_full": 0.0,
                "swapped": 0, "runner": "night-vm-muse", **values}

    def test_the_status_issue_says_whether_the_box_ran_short(self):
        state: dict = {}
        for entry in (self.entry(30, stall_full=60.0), self.entry(2), self.entry(1)):
            memory.note(state, entry, entry["runner"])
        text = memory.line(state, DAY)
        self.assertIn("(2 runs): memory to spare", text)  # the run 30 hours ago is out of it
        self.assertIn("Least available 3.0 GB of 8.0 GB", text)
        memory.note(state, self.entry(0.5, stall_full=12.0, swapped=40,
                                      low_available=GIB // 2), "night-vm-agy")
        text = memory.line(state, DAY)
        self.assertIn("**short of memory**", text)
        self.assertIn("1 run(s) stalled on memory for 5 s or more (longest 12 s); 1 swapped", text)
        self.assertEqual(memory.line({}, DAY), "")

    def test_the_state_keeps_the_newest_runs_only(self):
        state: dict = {}
        for n in range(memory.KEEP + 5):
            memory.note(state, self.entry(0, swapped=n), "night-vm-muse")
        self.assertEqual(len(state["machine_memory"]), memory.KEEP)
        self.assertEqual(state["machine_memory"][-1]["swapped"], memory.KEEP + 4)

    def test_the_status_issue_carries_the_line(self):
        ctx = make_ctx(at=DAY)
        ctx.store.update(lambda s: memory.note(s, self.entry(1), "night-vm-muse"))
        self.assertIn("**Machine memory, last 24 hours** (1 runs)", dashboard.render(ctx))


if __name__ == "__main__":
    unittest.main()
