"""The merge queue's slice of the daily fuzz (scripts/fuzz-slice.sh, ci.yml's `rust (fuzz slice k/K)`):
every seed of each wave played by exactly one shard, and a change failed only for a seed it broke,
never for one that fails on its base too, so a red main never blocks every merge behind it. The
script runs against a stand-in for jackioh that fails the seeds a table names, on the head's build or
the base's."""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

#: The repository's root, two directories above this file (bot/tests/).
REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts" / "fuzz-slice.sh"
CI_YML = REPO_ROOT / ".github" / "workflows" / "ci.yml"

#: The stand-in: `jackioh fuzz [--handicap|--duos] --from F --seeds N [--json]`, logged to
#: $FAKE_LOG; it fails each seed in F..F+N-1 that $FAKE_FAILS (JSON: {wave: [seeds]}) names, as
#: jackioh does: exit 1, and with --json the failures in its report. $FAKE_CRASH set: exit 101, no report.
FAKE = r"""#!/usr/bin/env python3
import json, os, sys
args = sys.argv[1:]
with open(os.environ["FAKE_LOG"], "a") as log:
    log.write(os.environ.get("FAKE_NAME", "jackioh") + " " + " ".join(args) + "\n")
if os.environ.get("FAKE_CRASH"):
    sys.exit(101)
assert args[0] == "fuzz", args
wave = "handicap" if "--handicap" in args else "duos" if "--duos" in args else "plain"
first = int(args[args.index("--from") + 1])
count = int(args[args.index("--seeds") + 1])
fails = [s for s in json.loads(os.environ.get("FAKE_FAILS", "{}")).get(wave, []) if first <= s < first + count]
if "--json" in args:
    print(json.dumps({"from": first, "ran": count, "passed": count - len(fails),
                      "failures": [{"seed": s, "message": f"I21 on seed {s}"} for s in fails]}, indent=2))
sys.exit(1 if fails else 0)
"""


class FuzzSlice(unittest.TestCase):
    def setUp(self):
        self.dir = Path(tempfile.mkdtemp(prefix="fuzz-slice-"))
        self.addCleanup(shutil.rmtree, self.dir, True)
        self.log = self.dir / "log.txt"
        self.log.touch()
        for name in ("head", "base"):
            path = self.dir / name
            path.write_text(FAKE, encoding="utf-8")
            path.chmod(0o755)

    def run_script(self, *args, fails=None, name="head", crash=False, seeds=1600, duos=200):
        env = dict(os.environ)
        env.update({
            "JACKIOH": str(self.dir / "head"),
            "OUT": str(self.dir / "out"),
            "GITHUB_OUTPUT": str(self.dir / "github_output"),
            "SEEDS": str(seeds),
            "DUOS_SEEDS": str(duos),
            "FAKE_LOG": str(self.log),
            "FAKE_NAME": name,
            "FAKE_FAILS": json.dumps(fails or {}),
        })
        if crash:
            env["FAKE_CRASH"] = "1"
        return subprocess.run(["bash", str(SCRIPT), *args], env=env, capture_output=True, text=True)

    def played(self, who="head"):
        """The (wave, first seed, count) of each run of the stand-in named `who`."""
        out = []
        for line in self.log.read_text(encoding="utf-8").splitlines():
            parts = line.split()
            if parts[0] != who:
                continue
            wave = "handicap" if "--handicap" in parts else "duos" if "--duos" in parts else "plain"
            out.append((wave, int(parts[parts.index("--from") + 1]), int(parts[parts.index("--seeds") + 1])))
        return out

    def test_the_shards_play_every_seed_of_each_wave_once_for_any_count(self):
        for total in (1, 2, 3, 4, 7, 8):
            self.log.write_text("", encoding="utf-8")
            for k in range(1, total + 1):
                r = self.run_script("play", str(k), str(total), seeds=1601, duos=203)
                self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
            for wave, n in (("plain", 1601), ("handicap", 1601), ("duos", 203)):
                seeds = [s for w, first, count in self.played() if w == wave for s in range(first, first + count)]
                self.assertEqual(seeds, list(range(1, n + 1)), f"{wave} over {total} shard(s)")

    def test_a_clean_shard_says_so_and_never_needs_the_base(self):
        r = self.run_script("play", "2", "4")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertEqual(self.played(), [("plain", 401, 400), ("handicap", 401, 400), ("duos", 51, 50)])
        self.assertIn("failed=false", (self.dir / "github_output").read_text(encoding="utf-8"))
        self.assertIn("plain wave, seeds 401..800: every seed passed", r.stdout)
        judged = self.run_script("judge", str(self.dir / "base"))
        self.assertEqual(judged.returncode, 0, judged.stdout + judged.stderr)
        self.assertEqual(self.played("base"), [])

    def test_a_failing_seed_is_listed_and_the_job_goes_on_to_the_base(self):
        r = self.run_script("play", "1", "4", fails={"plain": [267, 337], "duos": [9]})
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("failed=true", (self.dir / "github_output").read_text(encoding="utf-8"))
        self.assertEqual((self.dir / "out" / "failed.txt").read_text(encoding="utf-8").split("\n"),
                         ["plain 267", "plain 337", "duos 9", ""])
        self.assertIn("  seed 267: I21 on seed 267", r.stdout)

    def test_a_seed_that_fails_on_the_base_too_is_mains_and_passes_the_change(self):
        self.run_script("play", "1", "4", fails={"plain": [267], "handicap": [12]})
        r = self.run_script("judge", str(self.dir / "base"), name="base", fails={"plain": [267], "handicap": [12]})
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertEqual(self.played("base"), [("plain", 267, 1), ("handicap", 12, 1)])
        self.assertIn("::warning::the plain wave's seed 267 fails on the base too", r.stdout)
        self.assertIn("0 seed(s) this change broke, 2 main's already", r.stdout)

    def test_a_seed_that_passes_on_the_base_fails_the_change_and_says_how_to_replay_it(self):
        self.run_script("play", "1", "4", fails={"plain": [267], "handicap": [12]})
        r = self.run_script("judge", str(self.dir / "base"), name="base", fails={"plain": [267]})
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("::error::the handicap wave's seed 12 fails on this build and passes on its base", r.stdout)
        self.assertIn("cargo jackioh fuzz --handicap --from 12 --seeds 1", r.stdout)
        self.assertIn("1 seed(s) this change broke, 1 main's already", r.stdout)

    def test_a_plain_seed_replays_without_a_wave_flag(self):
        self.run_script("play", "1", "4", fails={"plain": [5]})
        r = self.run_script("judge", str(self.dir / "base"), name="base")
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("Replay it: cargo jackioh fuzz --from 5 --seeds 1", r.stdout)

    def test_a_wave_that_ends_with_nothing_to_read_fails_at_once(self):
        r = self.run_script("play", "1", "4", crash=True)
        self.assertEqual(r.returncode, 1)
        self.assertIn("failed (exit 101) with no failure to read", r.stdout + r.stderr)

    def test_it_refuses_a_shard_past_its_count_and_a_missing_base(self):
        self.assertEqual(self.run_script("play", "5", "4").returncode, 2)
        self.assertEqual(self.run_script("play", "0", "4").returncode, 2)
        self.run_script("play", "1", "4", fails={"plain": [5]})
        r = self.run_script("judge", str(self.dir / "nowhere"))
        self.assertEqual(r.returncode, 1)
        self.assertIn("no base build", r.stderr)

    def test_ci_yml_runs_it_in_the_queue_and_holds_rust_to_it(self):
        ci = CI_YML.read_text(encoding="utf-8")
        job = re.search(r"\n  rust-fuzz-slice:\n(.*?)\n\n  [a-z]", ci, re.S)
        self.assertIsNotNone(job, "ci.yml has no rust-fuzz-slice job")
        body = job.group(1)
        self.assertIn("bash scripts/fuzz-slice.sh play", body)
        self.assertIn("bash scripts/fuzz-slice.sh judge", body)
        self.assertIn("needs.changes.outputs.fuzz_slice_shards", body)
        # Never on a pull request: the queue is the gate (the owner's call of 2026-11-25).
        self.assertIn("github.event_name != 'pull_request'", body)
        self.assertIn("needs.changes.outputs.rust_tools_wasm == 'true'", body)
        summary = re.search(r"\n  rust:\n(.*?)\n\n  #", ci, re.S).group(1)
        self.assertIn("rust-fuzz-slice", summary)


if __name__ == "__main__":
    unittest.main()
