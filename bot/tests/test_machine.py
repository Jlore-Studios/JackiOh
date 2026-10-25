"""The machine's starter (bot/machine/starter.py): which queued jobs wake the machine. The
credits watch (bot/machine/credits.py): when an issue says the credits run low."""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
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


REGISTER = REPO_ROOT / "bot" / "machine" / "register-runners.sh"


class RegisterRunnersTests(unittest.TestCase):
    """register-runners.sh with `gh` and `on-machine.sh` stubbed: what it hands each box (#1677:
    the night box and the Muse box split the subscriptions, one Muse runner on each)."""

    def run_script(self, *args: str, box: str = "") -> subprocess.CompletedProcess:
        with tempfile.TemporaryDirectory() as tmp:
            here, bin_dir = Path(tmp, "machine"), Path(tmp, "bin")
            here.mkdir()
            bin_dir.mkdir()
            shutil.copy(REGISTER, here / "register-runners.sh")
            # The box gets the remote script and its arguments: the stub keeps the arguments.
            (here / "on-machine.sh").write_text('#!/bin/bash\nshift\nprintf "%s\\n" "$@" > "$HANDED"\n')
            (bin_dir / "gh").write_text('#!/bin/bash\n'
                                       'case "$*" in *registration-token*) echo REG;; '
                                       '*remove-token*) echo REMOVE;; esac\n')
            for stub in (here / "on-machine.sh", bin_dir / "gh"):
                stub.chmod(0o755)
            env = {**os.environ, "PATH": f"{bin_dir}{os.pathsep}{os.environ['PATH']}", "BOX": box,
                   "HANDED": str(Path(tmp, "handed"))}
            done = subprocess.run(["bash", str(here / "register-runners.sh"), "OWNER/REPO", *args],
                                  cwd=REPO_ROOT, env=env, capture_output=True, text=True)
            handed = Path(tmp, "handed")
            done.stdout = handed.read_text() if handed.exists() else ""
            return done

    def handed(self, *args: str, box: str = "") -> list[str]:
        done = self.run_script(*args, box=box)
        self.assertEqual(done.returncode, 0, done.stderr)
        return done.stdout.splitlines()

    def test_with_no_ids_every_machine_subscription_gets_its_lanes(self):
        handed = self.handed()
        self.assertEqual(handed[:4], ["OWNER/REPO", "REG", "REMOVE", ""])
        self.assertIn("muse=night-vm-muse=2", handed)
        self.assertIn("agy=night-vm-agy=1", handed)

    def test_each_box_names_its_own_subscriptions_and_runners(self):
        self.assertEqual(self.handed("gpt", "agy", "devin", "muse=1")[3:],
                         ["", "gpt=night-vm-gpt=1", "agy=night-vm-agy=1",
                          "devin=night-vm-devin=6", "muse=night-vm-muse=1"])
        self.assertEqual(self.handed("muse=1", box="b2")[3:], ["b2", "muse=night-vm-muse=1"])
        self.assertEqual(self.handed("muse=0")[4:], ["muse=night-vm-muse=0"])

    def test_it_refuses_an_unknown_id_a_bad_count_or_a_bad_suffix(self):
        for args, box, message in ((("nobody",), "", "nobody: no provider"),
                                   (("muse=two",), "", "muse=two: the runners here"),
                                   (("claude-1",), "", "claude-1: no provider"),
                                   (("muse=1",), "B 2", "BOX=B 2")):
            done = self.run_script(*args, box=box)
            self.assertNotEqual(done.returncode, 0, args)
            self.assertIn(message, done.stderr)

    def test_the_box_names_its_runners_and_removes_those_past_its_count(self):
        text = REGISTER.read_text(encoding="utf-8")
        self.assertIn('name="$label${box:+-$box}"', text)
        self.assertIn('name="$label-$lane${box:+-$box}"', text)
        self.assertIn('./config.sh remove --token "$remove_token"', text)
        # A runner in the middle of a job is never removed.
        removal = text.split("Past this box's count")[1].split("for lane in $(seq")[0]
        self.assertLess(removal.index("Runner.Worker"), removal.index("config.sh remove"))
        self.assertEqual(subprocess.run(["bash", "-n", str(REGISTER)]).returncode, 0)


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

    def test_the_training_box_keeps_three_lanes_starts_only_gather_and_never_powers_off(self):
        self.assertIn('TRAIN_LANES="improve unban gather"', self.text)
        self.assertIn('user="agent-train-$lane"', self.training)
        self.assertIn("apt-get install -y -qq gh", self.training)
        unit = self.training.split("jackioh-train@.service <<'EOF'\n", 1)[1].split("\nEOF\n", 1)[0]
        for line in ("User=agent-train-%i",
                     "ExecStart=/bin/sh /home/agent-train-%i/JackiOh/training/loop.sh %i",
                     "Restart=always", "RestartSec=60",
                     "Environment=MUSE_MODEL=muse-spark-1.3-contributor",
                     "Environment=MUSE_NO_AUTO_UPDATE=1",
                     "Environment=RAYON_NUM_THREADS=2",
                     "Environment=JACKIOH_TRAINING_OUT=/home/agent-train-%i/training-out/%i",
                     "WantedBy=multi-user.target"):
            self.assertIn(f"\n{line}\n", f"\n{unit}\n", line)
        # #1677: the two LLM lanes are stopped, and setup never starts them again.
        self.assertIn("systemctl enable --now jackioh-train@gather\n", self.training)
        self.assertNotIn("systemctl enable --now jackioh-train@improve", self.training)
        # No idle stop, no runner and no CloudWatch agent: those are the night box's alone.
        for night_only in ("night-vm-idle-stop.timer <<", "actions-runner", "amazon-cloudwatch-agent"):
            self.assertNotIn(night_only, self.training, night_only)
            self.assertIn(night_only, self.night, night_only)
        self.assertIn("systemctl disable --now night-vm-idle-stop.timer", self.training)

    def test_the_training_lanes_run_muse_and_devin_stays_on_the_night_box(self):
        """#1272: the improve and unban lanes run Muse; Devin is the night bot's weak builder
        alone, so only the night box installs it."""
        self.assertIn("\n  install_muse\n", self.training)
        for devin in ("install_devin", "DEVIN_MODEL", "devin --version"):
            self.assertNotIn(devin, self.training, devin)
        self.assertIn("\ninstall_muse\ninstall_devin\n", self.night)
        loop = (REPO_ROOT / "training" / "loop.sh").read_text(encoding="utf-8")
        self.assertIn('muse exec --yolo --disable-web-tools --workspace "$repo"', loop)
        self.assertIn('--model "$MUSE_MODEL"', loop)
        self.assertNotIn("devin", loop.lower())

    def test_the_training_lanes_clean_their_own_disk(self):
        """The training box has no clean-up timer: each lane's cycle starts by giving back its own
        builds but the release one when the disk is short, and its test builds carry no debug info,
        as the night box's jobs (`bot-night.yml`)."""
        loop = (REPO_ROOT / "training" / "loop.sh").read_text(encoding="utf-8")
        self.assertIn('disk_floor="${TRAIN_DISK_FLOOR_GB:-20}"', loop)
        self.assertIn("cycle() {\n  cd \"$repo\" || return 1\n", loop)
        cycle = loop[loop.index("cycle() {"):]
        self.assertLess(cycle.index("free_disk"), cycle.index("gather"))
        self.assertIn('find "$repo/target" -mindepth 1 -maxdepth 1 ! -name release', loop)
        for setting in ("CARGO_PROFILE_DEV_DEBUG=0", "CARGO_PROFILE_TEST_DEBUG=0"):
            self.assertIn(setting, loop)
            self.assertIn(setting, (REPO_ROOT / ".github/workflows/bot-night.yml").read_text())

    def test_the_night_box_has_a_chrome_for_the_ui_walkthroughs(self):
        """#816: scripts/browse.sh finds google-chrome-stable on PATH; the training box needs none."""
        self.assertIn("google-chrome-stable_current_amd64.deb", self.night)
        self.assertIn("apt-get install -y -qq --only-upgrade google-chrome-stable", self.night)
        self.assertNotIn("google-chrome", self.training)
        browse = (REPO_ROOT / "scripts" / "browse.sh").read_text(encoding="utf-8")
        self.assertIn("google-chrome-stable", browse)


if __name__ == "__main__":
    unittest.main()
