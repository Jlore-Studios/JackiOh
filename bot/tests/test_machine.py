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

from machine import credits, starter

#: The repository's root, two directories above this file (bot/tests/).
REPO_ROOT = Path(__file__).resolve().parents[2]

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
                                     only={"night-vm-gpt", "night-vm-agy", "night-vm-muse"})
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
        self.assertIn("muse=night-vm-muse=4", handed)
        self.assertIn("agy=night-vm-agy=1", handed)

    def test_each_box_names_its_own_subscriptions_and_runners(self):
        self.assertEqual(self.handed("gpt", "agy", "muse=1")[3:],
                         ["", "gpt=night-vm-gpt=1", "agy=night-vm-agy=1", "muse=night-vm-muse=1"])
        self.assertEqual(self.handed("muse=1", box="b2")[3:], ["b2", "muse=night-vm-muse=1"])
        self.assertEqual(self.handed("muse=0")[4:], ["muse=night-vm-muse=0"])

    def test_the_night_box_takes_muses_fill_in_runners(self):
        """Muse's four lanes of its own are one on the night box and three on the Muse box,
        and its fill-in runners (providers.json `fill_in`) on the night box, one per lane of
        gpt's and agy's."""
        self.assertIn("muse:fill=night-vm-muse-fill=2", self.handed())
        self.assertEqual(self.handed("gpt", "agy", "muse=1", "muse:fill")[4:],
                         ["gpt=night-vm-gpt=1", "agy=night-vm-agy=1", "muse=night-vm-muse=1",
                          "muse:fill=night-vm-muse-fill=2"])
        self.assertEqual(self.handed("muse=3", box="b2")[3:], ["b2", "muse=night-vm-muse=3"])
        self.assertEqual(self.handed("muse:fill=1")[4:], ["muse:fill=night-vm-muse-fill=1"])
        self.assertNotEqual(self.run_script("gpt:fill").returncode, 0)
        # Its runners live apart from the subscription's own, which never count them as lanes.
        text = REGISTER.read_text(encoding="utf-8")
        self.assertIn('base="$first${kind:+-$kind}"', text)
        self.assertIn('for dir in "$base" "$base"-*; do', text)

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
        self.assertLess(removal.index('busy "$dir"'), removal.index("config.sh remove"))
        self.assertEqual(subprocess.run(["bash", "-n", str(REGISTER)]).returncode, 0)


#: A runner's own config.sh, as far as the on-box script sees it: its root is where its bin really
#: is (a self-update leaves bin a link to bin.<version>), its registration is that root's .runner,
#: and configuring writes that root's svc.sh, named for the runner.
FAKE_CONFIG = r'''#!/bin/bash
root="$(dirname "$(readlink -f ./bin)")"
if [ "$1" = remove ]; then rm -f "$root/.runner"; exit 0; fi
[ -f "$root/.runner" ] && { echo "Cannot configure the runner because it is already configured."; exit 1; }
while [ $# -gt 0 ]; do case "$1" in --name) n="$2"; shift;; --labels) l="$2"; shift;; esac; shift; done
printf '{"agentName":"%s","labels":"%s"}\n' "$n" "$l" > "$root/.runner"
cat > "$root/svc.sh" <<SVC
#!/bin/bash
name="actions.runner.$n.service"
case "\$1" in
  install) echo "\$name" > .service; touch "\$UNITS/\$name";;
  uninstall) rm -f .service "\$UNITS/\$name";;
  start) touch "\$SVCSTATE/\$name";;
  stop) rm -f "\$SVCSTATE/\$name";;
esac
SVC
chmod +x "$root/svc.sh"
'''


class OnBoxTests(unittest.TestCase):
    """register-runners.sh's on-box script, run against a fake home and stubbed sudo, systemctl and
    pgrep, on a runner that has updated itself (its bin a link to bin.<version>). Copying that link
    made every second runner of a user run as its first: "already configured", and a fill-in
    runner registered in the first's directory under the first's service name (2026-10-26)."""

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        for name in ("bin", "units", "svc"):
            (self.tmp / name).mkdir()
        self.home = self.tmp / "home" / "agent-muse"
        self.first = self.home / "actions-runner"
        for name in ("bin", "externals"):
            (self.first / f"{name}.2.0").mkdir(parents=True)
            (self.first / f"{name}.2.0" / "x").write_text("x")
            (self.first / name).symlink_to(self.first / f"{name}.2.0")
        (self.first / "config.sh").write_text(FAKE_CONFIG)
        for name in ("env.sh", "run.sh", "run-helper.sh.template", ".env", ".path"):
            (self.first / name).write_text("")
        stubs = {"sudo": '[ "$1" = "-u" ] && shift 2\nexec "$@"',
                 "systemctl": 'name="${@: -1}"\nif [ -f "$SVCSTATE/$name" ]; then '
                              '[ "$2" = "--quiet" ] || echo active; exit 0; fi\n'
                              '[ "$2" = "--quiet" ] || echo inactive; exit 3',
                 "pgrep": "exit 1"}
        for name, body in stubs.items():
            (self.tmp / "bin" / name).write_text(f"#!/bin/bash\n{body}\n")
        for path in [*(self.tmp / "bin").iterdir(), self.first / "config.sh", self.first / "env.sh",
                     self.first / "run.sh"]:
            path.chmod(0o755)
        script = REGISTER.read_text(encoding="utf-8").split("<<'EOF'\n", 1)[1].split("\nEOF\n")[0]
        self.remote = self.tmp / "remote.sh"
        self.remote.write_text(script.replace("/home/", f"{self.tmp}/home/")
                               .replace("/run/night-vm-last-busy", f"{self.tmp}/last-busy"))
        self.env = {**os.environ, "PATH": f"{self.tmp / 'bin'}{os.pathsep}{os.environ['PATH']}",
                    "SVCSTATE": str(self.tmp / "svc"), "UNITS": str(self.tmp / "units")}
        self.shell(self.first, "./config.sh --name night-vm-muse --labels night-vm-muse && "
                               "./svc.sh install && ./svc.sh start")

    def shell(self, cwd, command):
        subprocess.run(["bash", "-c", command], cwd=cwd, env=self.env, check=True,
                       capture_output=True)

    def register(self, *entries, box=""):
        return subprocess.run(["bash", str(self.remote), "OWNER/REPO", "REG", "REMOVE", box,
                               *entries], env=self.env, capture_output=True, text=True)

    def runner(self, name):
        directory = self.home / name
        registered = directory / ".runner"
        return (directory.joinpath("bin").is_symlink(),
                registered.read_text().split('"')[3] if registered.exists() else "",
                (directory / ".service").read_text().strip()
                if (directory / ".service").exists() else "")

    def test_each_copy_is_a_runner_of_its_own(self):
        done = self.register("muse=night-vm-muse=0", "muse:fill=night-vm-muse-fill=2")
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn("night-vm-muse: removed from this box", done.stdout)
        self.assertEqual(self.runner("actions-runner-fill"),
                         (False, "night-vm-muse-fill", "actions.runner.night-vm-muse-fill.service"))
        self.assertEqual(self.runner("actions-runner-fill-2"),
                         (False, "night-vm-muse-fill-2",
                          "actions.runner.night-vm-muse-fill-2.service"))
        self.assertEqual(self.runner("actions-runner")[1:], ("", ""))
        self.assertEqual(sorted(p.name for p in (self.tmp / "units").iterdir()),
                         ["actions.runner.night-vm-muse-fill-2.service",
                          "actions.runner.night-vm-muse-fill.service"])
        # Run again, it changes nothing.
        again = self.register("muse=night-vm-muse=0", "muse:fill=night-vm-muse-fill=2")
        self.assertEqual((again.returncode, again.stdout.count("active as agent-muse")), (0, 2))

    def broken(self):
        """What the copy of the links left on the night box on 2026-10-26: the fill-in runner's
        registration in the first runner's directory, its service under the first's name, and
        copies that run as the first."""
        self.shell(self.first, "./svc.sh stop && ./svc.sh uninstall && ./config.sh remove && "
                               "./config.sh --name night-vm-muse-fill --labels night-vm-muse-fill")
        for name in ("actions-runner-fill", "actions-runner-fill-2", "actions-runner-2"):
            copy = self.home / name
            copy.mkdir()
            (copy / "bin").symlink_to(self.first / "bin.2.0")
            shutil.copy(self.first / "config.sh", copy / "config.sh")
        (self.home / "actions-runner-fill" / ".service").write_text(
            "actions.runner.night-vm-muse.service\n")
        stale = self.home / "actions-runner-fill" / "svc.sh"
        stale.write_text((self.first / "svc.sh").read_text().replace("night-vm-muse-fill",
                                                                     "night-vm-muse"))
        stale.chmod(0o755)
        (self.tmp / "units" / "actions.runner.night-vm-muse.service").write_text("")

    def test_a_copy_linked_into_the_first_runner_is_made_again(self):
        self.broken()
        done = self.register("muse=night-vm-muse=0", "muse:fill=night-vm-muse-fill=2")
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(self.runner("actions-runner-fill")[:2], (False, "night-vm-muse-fill"))
        self.assertEqual(self.runner("actions-runner-fill-2")[:2], (False, "night-vm-muse-fill-2"))
        self.assertFalse((self.home / "actions-runner-2").exists())
        self.assertEqual(sorted(p.name for p in (self.tmp / "units").iterdir()),
                         ["actions.runner.night-vm-muse-fill-2.service",
                          "actions.runner.night-vm-muse-fill.service"])

    def test_the_first_runner_drops_the_registration_left_in_its_directory(self):
        """Three jobs a box: the night box keeps one Muse runner of its own (`muse=1`), in the
        directory that holds night-vm-muse-fill's registration. It drops that registration's
        files (not the registration on GitHub, which the fill-in lane takes over with
        `--replace`), so the order of the ids changes nothing."""
        for order in (("muse=night-vm-muse=1", "muse:fill=night-vm-muse-fill=2"),
                      ("muse:fill=night-vm-muse-fill=2", "muse=night-vm-muse=1")):
            self.setUp()
            self.broken()
            done = self.register(*order)
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            self.assertIn("night-vm-muse: dropped night-vm-muse-fill's registration", done.stdout)
            self.assertNotIn("removal", done.stdout)
            self.assertEqual(self.runner("actions-runner"),
                             (True, "night-vm-muse", "actions.runner.night-vm-muse.service"))
            self.assertEqual(self.runner("actions-runner-fill"),
                             (False, "night-vm-muse-fill",
                              "actions.runner.night-vm-muse-fill.service"))
            self.assertEqual(self.runner("actions-runner-fill-2")[:2],
                             (False, "night-vm-muse-fill-2"))
            self.assertFalse((self.home / "actions-runner-2").exists())
            self.assertEqual(sorted(p.name for p in (self.tmp / "units").iterdir()),
                             ["actions.runner.night-vm-muse-fill-2.service",
                              "actions.runner.night-vm-muse-fill.service",
                              "actions.runner.night-vm-muse.service"])
            self.assertEqual(self.register(*order).stdout.count("active as agent-muse"), 3)

    def test_a_job_on_a_linked_copy_is_left_alone(self):
        (self.tmp / "bin" / "pgrep").write_text(
            '#!/bin/bash\ncase "$*" in *bin.2.0/Runner.Worker*) exit 0;; esac\nexit 1\n')
        done = self.register("muse=night-vm-muse=0")
        self.assertIn("night-vm-muse: busy with a job", done.stdout)
        self.assertEqual(self.runner("actions-runner")[1], "night-vm-muse")

    def test_the_muse_box_adds_its_second_and_third_runners(self):
        self.shell(self.first, "./svc.sh stop && ./svc.sh uninstall && ./config.sh remove && "
                               "./config.sh --name night-vm-muse-b2 --labels night-vm-muse && "
                               "./svc.sh install && ./svc.sh start")
        done = self.register("muse=night-vm-muse=3", box="b2")
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        for lane in (2, 3):
            self.assertEqual(self.runner(f"actions-runner-{lane}"),
                             (False, f"night-vm-muse-{lane}-b2",
                              f"actions.runner.night-vm-muse-{lane}-b2.service"))
        self.assertEqual(self.runner("actions-runner")[1:],
                         ("night-vm-muse-b2", "actions.runner.night-vm-muse-b2.service"))
        self.assertNotIn("dropped", done.stdout)


class SetupTests(unittest.TestCase):
    """setup.sh, read as text: what each box gets (README.md, Building it; The training box)."""

    text = SETUP.read_text(encoding="utf-8")
    training, _, night = text.partition('if [ -n "$training" ]; then')[2].partition("\n  exit 0\nfi\n")

    def test_it_parses(self):
        self.assertEqual(subprocess.run(["bash", "-n", str(SETUP)]).returncode, 0)

    def test_no_apt_run_restarts_a_runner_in_the_middle_of_its_job(self):
        """needrestart restarted runners after apt runs (unattended upgrades and this script's),
        and each restart killed the job the runner held."""
        rule = "$nrconf{override_rc}{qr(^actions\\.runner\\.)} = 0;"
        self.assertIn(f"/etc/needrestart/conf.d/night-vm.conf <<'EOF'\n{rule}\n", self.text)
        self.assertLess(self.text.index(rule), self.text.index("apt-get update"))

    def test_both_boxes_swap_to_compressed_memory_before_the_swapfile(self):
        """Three jobs overrun an 8 GB box; paging to the swapfile on EBS stalled one for 766 s."""
        shared = self.text.split('if [ -n "$training" ]; then', 1)[0]
        self.assertIn("apt-get install -y -qq zram-tools", shared)
        self.assertIn("printf 'ALGO=zstd\\nPERCENT=50\\nPRIORITY=100\\n' > /etc/default/zramswap",
                      shared)
        self.assertIn("systemctl restart zramswap", shared)
        self.assertIn("swapon /swapfile", shared)  # the swapfile stays, below it

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

    def test_the_training_lanes_run_muse(self):
        """#1272: the improve and unban lanes run Muse, which both boxes install."""
        self.assertIn("\n  install_muse\n", self.training)
        self.assertIn("\ninstall_muse\n", self.night)
        loop = (REPO_ROOT / "training" / "loop.sh").read_text(encoding="utf-8")
        self.assertIn('muse exec --yolo --disable-web-tools --workspace "$repo"', loop)
        self.assertIn('--model "$MUSE_MODEL"', loop)

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
