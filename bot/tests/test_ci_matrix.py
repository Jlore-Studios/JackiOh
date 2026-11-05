"""ci.yml's sharded jobs (.github/ci/matrix.py, #3072): the `rust-test` matrix it expands from
.github/ci/rust-test.toml and scripts/ci-shards.toml, the web unit and e2e smoke counts, and the
names one shard of a group runs. What the night bot may change (a count, a new `rules_<n>`) must never
drop a test, and what it may not (a group's flags) stays under .github/."""

from __future__ import annotations

import importlib.util
import io
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

#: The repository's root, two directories above this file (bot/tests/).
REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / ".github" / "ci" / "matrix.py"

_spec = importlib.util.spec_from_file_location("ci_matrix", SCRIPT)
matrix = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(matrix)

#: ci.yml's `rust (test <group>)` jobs as #2964 left them, in order: the move to the generator kept
#: every one, under its name.
JOBS = [
    "engine rules", "engine rules 2", "engine rules 3", "engine", "engine golden", "cards lib",
    "cards", "server", "server api", "server practice", "server practice rulings", "ai", "ai slow",
    "luau", "tools fuzz", "tools duos, selfplay", "tools, wasm",
]

RULES = ("-p jackioh-engine --features jackioh-engine/testkit --test {} "
         "--config profile.test.package.jackioh-engine.opt-level=1 "
         "--config profile.test.package.jackioh-cards.opt-level=0")


class Repo:
    """A copy of the three files the generator reads, to change one at a time."""

    def __init__(self):
        self.dir = Path(tempfile.mkdtemp(prefix="ci-matrix-"))
        for path in (matrix.GROUPS, matrix.COUNTS, "crates/engine/Cargo.toml"):
            (self.dir / path).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(REPO_ROOT / path, self.dir / path)

    def edit(self, path, old, new):
        file = self.dir / path
        text = file.read_text(encoding="utf-8")
        assert old in text, f"{old!r} is not in {path}"
        file.write_text(text.replace(old, new, 1), encoding="utf-8")

    def outputs(self):
        return matrix.outputs(self.dir)

    def close(self):
        shutil.rmtree(self.dir)


class MatrixTests(unittest.TestCase):
    def setUp(self):
        self.repo = Repo()

    def tearDown(self):
        self.repo.close()

    def jobs(self):
        return {job["group"]: job for job in self.repo.outputs()["rust_test"]}

    def test_the_committed_files_give_todays_jobs(self):
        out = matrix.outputs()
        self.assertEqual([job["group"] for job in out["rust_test"]], JOBS)
        self.assertEqual(out["web_unit_shards"], [1, 2, 3, 4])
        self.assertEqual(out["e2e_smoke_shards"], [1, 2])
        jobs = {job["group"]: job for job in out["rust_test"]}
        for n, name in enumerate(["rules", "rules_2", "rules_3"]):
            self.assertEqual(jobs[JOBS[n]]["test"], RULES.format(name))
        self.assertEqual(jobs["engine"]["binaries"], "jackioh-engine export_config golden rules rules_2 rules_3")
        self.assertEqual(jobs["cards"]["binaries"], "jackioh-cards cards")
        self.assertEqual(jobs["ai"]["binaries"], "jackioh-ai ai ai_slow")
        self.assertEqual({name for name, job in jobs.items() if job["doc"]}, {"engine", "cards", "ai"})
        self.assertEqual({name for name, job in jobs.items() if job["luau"]}, {"luau"})
        self.assertEqual({(job["shard"], job["shards"]) for job in jobs.values()}, {(1, 1)})
        self.assertEqual(jobs["tools, wasm"]["test"].split(" -- ")[1],
                         "--skip fuzz:: --skip selfplay:: --skip positions::")

    def test_a_new_rules_binary_gets_a_job_and_the_binaries_check_by_itself(self):
        self.repo.edit("crates/engine/Cargo.toml", '[[test]]\nname = "golden"',
                       '[[test]]\nname = "rules_4"\npath = "tests/rules_4.rs"\nrequired-features = ["testkit"]\n\n'
                       '[[test]]\nname = "golden"')
        jobs = self.jobs()
        self.assertEqual(jobs["engine rules 4"]["test"], RULES.format("rules_4"))
        self.assertEqual(jobs["engine rules 4"]["scope"], "engine_rules")
        self.assertEqual(jobs["engine rules 4"]["cache"], "engine-rules")
        self.assertEqual(jobs["engine"]["binaries"],
                         "jackioh-engine export_config golden rules rules_2 rules_3 rules_4")

    def test_a_count_splits_a_group_into_that_many_jobs(self):
        self.repo.edit(matrix.COUNTS, '"tools fuzz" = 1', '"tools fuzz" = 2')
        self.repo.edit(matrix.COUNTS, '"ai" = 1', '"ai" = 3')
        jobs = self.jobs()
        self.assertNotIn("tools fuzz", jobs)
        self.assertEqual([(jobs[f"tools fuzz {k}/2"]["shard"], jobs[f"tools fuzz {k}/2"]["shards"]) for k in (1, 2)],
                         [(1, 2), (2, 2)])
        self.assertEqual(jobs["tools fuzz 1/2"]["test"], jobs["tools fuzz 2/2"]["test"])
        # The doc tests and the binaries check run once, in the first shard.
        self.assertEqual([jobs[f"ai {k}/3"]["doc"] for k in (1, 2, 3)], ["-p jackioh-ai", "", ""])
        self.assertEqual([jobs[f"ai {k}/3"]["binaries"] for k in (1, 2, 3)], ["jackioh-ai ai ai_slow", "", ""])
        self.assertEqual(len(jobs), len(JOBS) + 1 + 2)

    def test_a_count_on_a_fanned_out_group_shards_each_of_its_binaries(self):
        self.repo.edit(matrix.COUNTS, '"engine rules" = 1', '"engine rules" = 2')
        names = [job["group"] for job in self.repo.outputs()["rust_test"]][:6]
        self.assertEqual(names, ["engine rules 1/2", "engine rules 2/2", "engine rules 2 1/2", "engine rules 2 2/2",
                                 "engine rules 3 1/2", "engine rules 3 2/2"])

    def test_the_web_and_e2e_counts_come_from_the_file(self):
        self.repo.edit(matrix.COUNTS, "unit = 4", "unit = 5")
        self.repo.edit(matrix.COUNTS, "smoke = 2", "smoke = 3")
        out = self.repo.outputs()
        self.assertEqual((out["web_unit_shards"], out["e2e_smoke_shards"]), ([1, 2, 3, 4, 5], [1, 2, 3]))

    def test_it_refuses_what_could_hide_a_job_or_flood_the_runners(self):
        for old, new, says in [
            ('"cards" = 1', '"cards" = 0', "from 1 to 8"),
            ('"cards" = 1', '"cards" = 9', "from 1 to 8"),
            ('"cards" = 1', '"cards" = "2"', "from 1 to 8"),
            ('"cards" = 1', '"cards" = 1.5', "from 1 to 8"),
            ('"cards" = 1', '"cards" = true', "from 1 to 8"),
            ('"cards" = 1', '"cards" = 1\n"card" = 2', "names no group"),
            ("unit = 4", "unit = 0", "from 1 to 8"),
            ("unit = 4", "units = 4", "holds only `unit`"),
            ("[e2e]\nsmoke = 2", "[e2e]", "has no `smoke`"),
            ("[web]", "[wbe]\nunit = 1\n[web]", "no job reads the table wbe"),
        ]:
            with self.subTest(new=new):
                repo = Repo()
                try:
                    repo.edit(matrix.COUNTS, old, new)
                    with self.assertRaisesRegex(matrix.Bad, says):
                        repo.outputs()
                finally:
                    repo.close()

    def test_a_pattern_that_matches_no_target_fails(self):
        self.repo.edit(matrix.GROUPS, 'tests = "rules*"', 'tests = "rulez*"')
        with self.assertRaisesRegex(matrix.Bad, "no test target of crates/engine/Cargo.toml matches rulez"):
            self.repo.outputs()

    def test_a_pattern_is_a_prefix_and_one_trailing_star(self):
        self.repo.edit(matrix.GROUPS, 'tests = "rules*"', 'tests = "r*s"')
        with self.assertRaisesRegex(matrix.Bad, "one trailing"):
            self.repo.outputs()

    def test_two_jobs_with_one_name_fail(self):
        self.repo.edit(matrix.GROUPS, 'group = "engine golden"', 'group = "engine rules 2"')
        self.repo.edit(matrix.COUNTS, '"engine golden" = 1', '"engine rules 2" = 1')
        with self.assertRaisesRegex(matrix.Bad, "two jobs would have the name engine rules 2"):
            self.repo.outputs()


class ShardTests(unittest.TestCase):
    NAMES = [f"rules::zone_{n}::r{n}000_moves_{n % 7}" for n in range(500)] + ["tests::it_works"]
    #: Doc tests, which ci.yml runs whole in shard 1: cargo splits their names on spaces.
    DOCS = ["src/lib.rs - catalog::register_all (line 12)", "src/zones.rs - Zone (line 4) - compile fail"]

    def listing(self):
        # As `cargo test -- --list --format terse` prints it, two binaries and the doc tests one after
        # the other, the same name in two of them.
        return ([f"{name}: test\n" for name in self.NAMES] + ["tests::it_works: test\n", "bench_it: benchmark\n"]
                + [f"{name}: test\n" for name in self.DOCS])

    def test_every_name_lands_in_exactly_one_shard_for_any_count(self):
        for total in range(1, matrix.MAX_SHARDS + 1):
            shards = [matrix.shard(k, total, self.listing()) for k in range(1, total + 1)]
            flat = [name for names in shards for name in names]
            self.assertEqual(sorted(flat), sorted(set(self.NAMES)), total)
            if total > 1:
                self.assertTrue(all(len(names) > len(self.NAMES) / total / 2 for names in shards), total)

    def test_a_name_keeps_its_shard_whatever_else_is_listed(self):
        alone = matrix.shard(2, 3, ["rules::zone_7::r7000_moves_0: test\n"])
        among = "rules::zone_7::r7000_moves_0" in matrix.shard(2, 3, self.listing())
        self.assertEqual(bool(alone), among)

    def test_the_command_reads_a_terse_list_and_prints_its_shard(self):
        listing = "".join(self.listing())
        printed = [
            subprocess.run([sys.executable, str(SCRIPT), "shard", str(k), "2"], input=listing, text=True,
                           capture_output=True, check=True).stdout.splitlines()
            for k in (1, 2)
        ]
        self.assertEqual(sorted(printed[0] + printed[1]), sorted(set(self.NAMES)))
        for doc in self.DOCS:
            self.assertNotIn(doc, printed[0] + printed[1])

    def test_the_command_refuses_a_shard_past_its_count(self):
        for args in (["shard", "3", "2"], ["shard", "0", "2"], ["shard", "1", "9"], ["shard", "x", "2"]):
            run = subprocess.run([sys.executable, str(SCRIPT), *args], input="", text=True, capture_output=True)
            self.assertNotEqual(run.returncode, 0, args)

    def test_the_matrix_command_prints_one_output_a_line(self):
        run = subprocess.run([sys.executable, str(SCRIPT), "matrix"], text=True, capture_output=True, check=True)
        self.assertEqual([line.split("=", 1)[0] for line in run.stdout.splitlines()],
                         ["rust_test", "web_unit_shards", "e2e_smoke_shards"])
        self.assertIn("rust (test engine rules 3)", run.stderr)


class WorkflowTests(unittest.TestCase):
    def test_ci_yml_reads_every_output_and_names_no_group_itself(self):
        text = (REPO_ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
        for output in ("rust_test", "web_unit_shards", "e2e_smoke_shards"):
            self.assertIn(f"${{{{ steps.matrix.outputs.{output} }}}}", text)
            self.assertIn(f"needs.changes.outputs.{output}", text)
        self.assertNotIn("- group: ", text)
        self.assertIn("python3 .github/ci/matrix.py matrix", text)
        self.assertIn('python3 .github/ci/matrix.py shard "$SHARD" "$SHARDS"', text)


if __name__ == "__main__":
    unittest.main()
