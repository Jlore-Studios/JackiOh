"""The machine's starter (bot/machine/starter.py): which queued jobs wake the machine. The
credits watch (bot/machine/credits.py): when an issue says the credits run low."""

from __future__ import annotations

import re
import subprocess
import unittest
from datetime import datetime, timezone

from harness.config import REPO_ROOT
from machine import credits, starter

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

    def test_only_the_night_bot_wakes_it(self):
        """Triage runs no model there since #664, so only bot-night's jobs need the machine."""
        self.assertEqual(starter.DEFAULT_WORKFLOWS.split(","), ["bot-night.yml"])

    def test_a_job_whose_runner_is_missing_stops_waking_it(self):
        get, _ = fake_api({"in_progress": [{"id": 7}]}, {7: [
            job("work", "queued", ["night-vm-muse"], created="2026-10-02T14:00:00Z")]})
        self.assertEqual(starter.waiting_jobs("123", ["bot-night.yml"], get, NOW), [])

    def test_a_box_wakes_only_for_its_own_runners(self):
        """With two boxes that sleep (`RUNNER_LABELS`), each starter watches its labels: a job
        for the other box's runner wakes that box, and the night box sleeps through it (and the
        other way round). The training box is always on, so it has no starter."""
        get, _ = fake_api({"in_progress": [{"id": 7}]}, {7: [
            job("work", "queued", ["night-vm-gpt"]),
            job("other", "queued", ["night-vm-other"])]})
        other = starter.waiting_jobs("123", ["bot-night.yml"], get, NOW,
                                     only={"night-vm-other"})
        self.assertEqual([w["label"] for w in other], ["night-vm-other"])
        night = starter.waiting_jobs("123", ["bot-night.yml"], get, NOW,
                                     only={"night-vm-gpt", "night-vm-agy", "night-vm-muse",
                                           "night-vm-devin"})
        self.assertEqual([w["label"] for w in night], ["night-vm-gpt"])


def plan(amount):
    state = {"accountPlanType": "FREE", "accountPlanExpirationDate": "2027-04-02T17:02:40Z"}
    if amount is not None:
        state["accountPlanRemainingCredits"] = {"amount": amount, "unit": "USD"}
    return state


class CreditsTests(unittest.TestCase):
    def test_the_issue_opens_only_under_the_floor(self):
        self.assertIsNone(credits.issue(plan(156.53)))
        self.assertIsNone(credits.issue(plan(credits.FLOOR)))
        wanted = credits.issue(plan(19.99))
        self.assertEqual(wanted["title"], "AWS credits are under $20")
        self.assertEqual(wanted["labels"], ["human"])
        self.assertIn("$19.99", wanted["body"])
        self.assertIn("ending 2027-04-02", wanted["body"])

    def test_a_plan_without_credits_opens_nothing(self):
        self.assertIsNone(credits.issue(plan(None)))

    def test_an_issue_ever_opened_with_the_title_stops_another(self):
        def call(items):
            return lambda method, path, body: {"items": items}
        self.assertTrue(credits.already_opened(
            "o/r", call([{"title": credits.TITLE, "state": "closed"}])))
        self.assertFalse(credits.already_opened(
            "o/r", call([{"title": "AWS credits are under $20 again?"}])))
        self.assertFalse(credits.already_opened("o/r", call([])))


SETUP = REPO_ROOT / "bot" / "machine" / "setup.sh"


class SetupTests(unittest.TestCase):
    """setup.sh, read as text: what each box gets (README.md, Building it; The training box)."""

    text = SETUP.read_text(encoding="utf-8")
    training, _, night = text.partition('if [ -n "$training" ]; then')[2].partition("\n  exit 0\nfi\n")

    def test_it_parses(self):
        self.assertEqual(subprocess.run(["bash", "-n", str(SETUP)]).returncode, 0)

    def test_every_user_that_runs_the_checks_gets_the_pinned_rust(self):
        toml = (REPO_ROOT / "rust-toolchain.toml").read_text(encoding="utf-8")
        channel = re.search(r'^channel = "([^"]+)"', toml, re.M).group(1)
        self.assertIn(f"RUST_TOOLCHAIN={channel}\n", self.text)
        self.assertIn('toolchain install "$RUST_TOOLCHAIN" \\\n        --profile minimal '
                      '--component rustfmt,clippy', self.text)
        self.assertIn('install_rust "$user"', self.training)
        self.assertIn('install_rust "$user"', self.night)
        # The night box's runners find it first on their PATH, every lane's runner alike.
        self.assertIn('for dir in "/home/$user"/actions-runner*; do', self.night)
        self.assertIn('echo "/home/$user/.cargo/bin:/usr/local/bin:', self.night)

    def test_the_training_box_runs_three_lanes_forever_and_never_powers_off(self):
        self.assertIn('TRAIN_LANES="improve unban gather"', self.text)
        self.assertIn('user="agent-train-$lane"', self.training)
        self.assertIn("apt-get install -y -qq gh", self.training)
        unit = self.training.split("jackioh-train@.service <<'EOF'\n", 1)[1].split("\nEOF\n", 1)[0]
        for line in ("User=agent-train-%i",
                     "ExecStart=/bin/sh /home/agent-train-%i/JackiOh/training/loop.sh %i",
                     "Restart=always", "RestartSec=60", "Environment=DEVIN_MODEL=swe-2-max",
                     "Environment=RAYON_NUM_THREADS=2",
                     "Environment=JACKIOH_TRAINING_OUT=/home/agent-train-%i/training-out/%i",
                     "WantedBy=multi-user.target"):
            self.assertIn(f"\n{line}\n", f"\n{unit}\n", line)
        self.assertIn("systemctl enable --now jackioh-train@improve jackioh-train@unban"
                      " jackioh-train@gather\n", self.training)
        # No idle stop, no runner and no CloudWatch agent: those are the night box's alone.
        for night_only in ("night-vm-idle-stop.timer <<", "actions-runner", "amazon-cloudwatch-agent"):
            self.assertNotIn(night_only, self.training, night_only)
            self.assertIn(night_only, self.night, night_only)
        self.assertIn("systemctl disable --now night-vm-idle-stop.timer", self.training)

    def test_the_night_box_has_a_chrome_for_the_ui_walkthroughs(self):
        """#816: scripts/browse.sh finds google-chrome-stable on PATH; the training box needs none."""
        self.assertIn("google-chrome-stable_current_amd64.deb", self.night)
        self.assertIn("apt-get install -y -qq --only-upgrade google-chrome-stable", self.night)
        self.assertNotIn("google-chrome", self.training)
        browse = (REPO_ROOT / "scripts" / "browse.sh").read_text(encoding="utf-8")
        self.assertIn("google-chrome-stable", browse)


if __name__ == "__main__":
    unittest.main()
