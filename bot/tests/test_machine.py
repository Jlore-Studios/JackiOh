"""The machine's starter (bot/machine/starter.py): which queued jobs wake the machine."""

from __future__ import annotations

import unittest
from datetime import datetime, timezone

from machine import starter

NOW = datetime(2026, 10, 2, 18, 0, tzinfo=timezone.utc)


def fake_api(runs: dict[str, list[dict]], jobs: dict[int, list[dict]]):
    calls = []

    def get(path: str) -> dict:
        calls.append(path)
        if "/jobs" in path:
            run_id = int(path.split("/runs/")[1].split("/")[0])
            return {"jobs": jobs.get(run_id, [])}
        status = path.split("status=")[1].split("&")[0]
        return {"workflow_runs": runs.get(status, [])}
    return get, calls


def job(name, status, labels, created="2026-10-02T17:50:00Z"):
    return {"name": name, "status": status, "labels": labels, "created_at": created}


class StarterTests(unittest.TestCase):
    def test_a_queued_job_on_the_machine_wakes_it(self):
        get, calls = fake_api({"in_progress": [{"id": 7}]}, {7: [
            job("gate", "completed", ["ubuntu-latest"]),
            job("work", "queued", ["night-vm-gpt"])]})
        found = starter.waiting_jobs("123", ["bot-night.yml"], get, NOW)
        self.assertEqual(found, [{"run": 7, "job": "work", "label": "night-vm-gpt"}])
        self.assertTrue(all(c.startswith("/repositories/123/") for c in calls))

    def test_nothing_else_wakes_it(self):
        get, _ = fake_api({"in_progress": [{"id": 7}], "queued": [{"id": 7}, {"id": 8}]}, {
            7: [job("work", "in_progress", ["night-vm-gpt"]),        # already running there
                job("deliver", "queued", ["ubuntu-latest"])],         # GitHub's runner
            8: [job("work", "waiting", ["night-vm-agy"])]})           # an approval, not a runner
        self.assertEqual(starter.waiting_jobs("123", ["bot-night.yml"], get, NOW), [])

    def test_triage_wakes_it_too(self):
        self.assertEqual(starter.DEFAULT_WORKFLOWS.split(","), ["bot-night.yml", "triage.yml"])
        get, calls = fake_api({"in_progress": [{"id": 9}]}, {9: [
            job("classify", "queued", ["night-vm-devin"])]})
        found = starter.waiting_jobs("123", starter.DEFAULT_WORKFLOWS.split(","), get, NOW)
        self.assertEqual(found[0]["label"], "night-vm-devin")
        self.assertTrue(any("/workflows/triage.yml/" in c for c in calls))

    def test_a_job_whose_runner_is_missing_stops_waking_it(self):
        get, _ = fake_api({"in_progress": [{"id": 7}]}, {7: [
            job("work", "queued", ["night-vm-muse"], created="2026-10-02T14:00:00Z")]})
        self.assertEqual(starter.waiting_jobs("123", ["bot-night.yml"], get, NOW), [])

    def test_a_box_wakes_only_for_its_own_runners(self):
        """With two boxes, each starter watches its labels: a train job wakes the
        training box, and the night box sleeps through it (and the other way round)."""
        get, _ = fake_api({"in_progress": [{"id": 7}]}, {7: [
            job("work", "queued", ["night-vm-gpt"]),
            job("train", "queued", ["night-vm-devin-train"])]})
        train = starter.waiting_jobs("123", ["bot-night.yml"], get, NOW,
                                     only={"night-vm-devin-train"})
        self.assertEqual([w["label"] for w in train], ["night-vm-devin-train"])
        night = starter.waiting_jobs("123", ["bot-night.yml"], get, NOW,
                                     only={"night-vm-gpt", "night-vm-agy", "night-vm-muse",
                                           "night-vm-devin"})
        self.assertEqual([w["label"] for w in night], ["night-vm-gpt"])


if __name__ == "__main__":
    unittest.main()
