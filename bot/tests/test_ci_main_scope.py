"""ci.yml's scope on a push to main (.github/ci/main_scope.py, #3564): every piece the merge-queue build
of the same commit skipped, and none it ran and passed. What it may never do is skip a piece the
build did not run, or emit a scope no pull request could have, which the summary jobs would misread."""

from __future__ import annotations

import importlib.util
import io
import random
import re
import shutil
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

#: The repository's root, two directories above this file (bot/tests/).
REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / ".github" / "ci" / "main_scope.py"
CI_YML = REPO_ROOT / ".github" / "workflows" / "ci.yml"

_spec = importlib.util.spec_from_file_location("ci_main_scope", SCRIPT)
main_scope = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(main_scope)


def ci_scope_lines():
    """scripts/ci-scope.sh's lines (its `LINES=`), in order: what it prints with no arguments."""
    text = (REPO_ROOT / "scripts" / "ci-scope.sh").read_text(encoding="utf-8").replace("\r\n", "\n")
    match = re.search(r'^LINES="([a-z0-9_ ]*)"$', text, re.M)
    assert match, "scripts/ci-scope.sh has no LINES="
    return match.group(1).split()


EVERY = {name: "true" for name in ci_scope_lines()}


def queue(true=(), full="true", bot="false"):
    """A queue build's scope: `full`, the lines in `true` on, the rest off, and `bot`."""
    lines = {name: "false" for name in EVERY}
    lines["full"] = full
    for name in true:
        lines[name] = "true"
    if any(lines.get(name) == "true" for name in lines if name.startswith("db_")):
        lines["db"] = "true"
    lines["bot"] = bot
    return lines


def on(lines):
    return sorted(name for name, value in lines.items() if value == "true")


#: What main runs after a queue build that ran everything: the lines in ALWAYS, `db` and `full`.
ALWAYS_ON = sorted({name for name, _ in main_scope.ALWAYS} | {"db", "full"})


class MainScopeTests(unittest.TestCase):
    def test_what_the_queue_ran_is_off_and_what_it_skipped_runs(self):
        # 6a8b8d5b0's build (2026-11-09): the web, e2e, the Rust tools and infra ran; the rest did not.
        built = queue(true=("web", "e2e", "rust_tools_wasm", "infra"))
        lines, why = main_scope.scope(EVERY, built, ["apps/web/src/App.tsx"])
        for name in ("web", "web_promote", "e2e", "rust_tools_wasm", "infra"):
            self.assertEqual(lines[name], "false", name)
        for name in ("clippy", "rust_engine", "rust_server", "db_sql", "db_store", "db_deploy", "db", "bot"):
            self.assertEqual(lines[name], "true", name)
        self.assertEqual(lines["full"], "true")
        self.assertIn("passed", why)

    def test_it_prints_every_line_ci_scope_has_and_bot_in_its_order(self):
        lines, _ = main_scope.scope(EVERY, queue(true=("web",)), [])
        self.assertEqual(list(lines), list(EVERY) + ["bot"])

    def test_a_queue_that_ran_everything_leaves_only_the_deploy_rehearsal(self):
        built = queue(true=[name for name in EVERY if name not in ("full", "db")], bot="true")
        lines, _ = main_scope.scope(EVERY, built, ["crates/engine/src/lib.rs"])
        self.assertEqual(on(lines), ALWAYS_ON)

    def test_the_deploy_rehearsal_runs_on_main_even_when_the_queue_ran_it(self):
        # Its `deploy-image` artifact is what staging's deploy loads (scripts/server-image.sh, step
        # 2), read from the push-to-main run alone, so the queue's run of it cannot stand in.
        built = queue(true=("db_sql", "db_store", "db_deploy", "rust_server"))
        lines, why = main_scope.scope(EVERY, built, ["crates/server/src/main.rs"])
        self.assertEqual((lines["db_sql"], lines["db_store"], lines["rust_server"]), ("false",) * 3)
        self.assertEqual((lines["db_deploy"], lines["db"], lines["full"]), ("true",) * 3)
        self.assertIn("what main always runs (db_deploy)", why)
        self.assertNotIn("db_deploy", why.split("what main always runs")[0])

    def test_bot_selftest_runs_unless_the_queue_ran_it(self):
        self.assertEqual(main_scope.scope(EVERY, queue(bot="false"), [])[0]["bot"], "true")
        self.assertEqual(main_scope.scope(EVERY, queue(bot="true"), [])[0]["bot"], "false")
        self.assertEqual(main_scope.scope(EVERY, queue(bot=""), [])[0]["bot"], "true")

    def test_a_queue_build_that_ran_no_full_job_tested_none_of_them(self):
        # A skip-list-only commit (the bot's code, say): its build ran bot selftest alone.
        built = queue(true=("web", "clippy"), full="false", bot="true")
        lines, _ = main_scope.scope(EVERY, built, ["bot/go/main.go"])
        self.assertEqual(on(lines), sorted(EVERY))
        self.assertEqual(lines["bot"], "false")

    def test_only_an_exact_true_counts_as_run(self):
        built = queue(true=("web",))
        built["clippy"] = ""
        built.pop("rust_server")
        lines, _ = main_scope.scope(EVERY, built, [])
        self.assertEqual(lines["clippy"], "true")
        self.assertEqual(lines["rust_server"], "true")
        self.assertEqual(lines["web"], "false")

    def test_a_line_only_main_knows_runs(self):
        every = dict(EVERY, rust_new="true")
        lines, _ = main_scope.scope(every, queue(true=("web",)), [])
        self.assertEqual(lines["rust_new"], "true")

    def test_the_promotion_tests_wait_for_the_web_they_run_in(self):
        # web-promote's job needs `web`: with the web tested, the promotion tests go to super.yml.
        lines, _ = main_scope.scope(EVERY, queue(true=("web",)), [])
        self.assertEqual((lines["web"], lines["web_promote"]), ("false", "false"))
        lines, _ = main_scope.scope(EVERY, queue(true=("e2e",)), [])
        self.assertEqual((lines["web"], lines["web_promote"]), ("true", "true"))

    def test_db_is_any_of_its_suites(self):
        lines, _ = main_scope.scope(EVERY, queue(true=("db_sql",)), [])
        self.assertEqual((lines["db_sql"], lines["db_store"], lines["db"]), ("false", "true", "true"))
        lines, _ = main_scope.scope(EVERY, queue(true=("db_sql", "db_store", "db_deploy")), [])
        self.assertEqual((lines["db_sql"], lines["db_store"]), ("false", "false"))
        self.assertEqual((lines["db_deploy"], lines["db"]), ("true", "true"))

    def test_with_no_build_or_no_range_everything_runs(self):
        for built, changed in ((None, []), (queue(true=("web",)), None)):
            lines, why = main_scope.scope(EVERY, built, changed)
            self.assertEqual(on(lines), sorted(list(EVERY) + ["bot"]))
            self.assertIn("everything runs", why)

    def test_a_change_to_what_keys_the_caches_or_to_the_ci_runs_everything(self):
        built = queue(true=[name for name in EVERY if name not in ("full", "db")], bot="true")
        for path in ("Cargo.lock", "Cargo.toml", "crates/engine/Cargo.toml", "rust-toolchain.toml",
                     ".cargo/config.toml", "pnpm-lock.yaml", ".github/workflows/ci.yml",
                     ".github/actions/setup/action.yml", ".github/ci/main_scope.py"):
            lines, why = main_scope.scope(EVERY, built, ["apps/web/src/App.tsx", path])
            self.assertEqual(on(lines), sorted(list(EVERY) + ["bot"]), path)
            self.assertIn(path, why)
        for path in ("crates/engine/src/Cargo.toml.md", "apps/web/package.json", "docs/github.md"):
            lines, _ = main_scope.scope(EVERY, built, [path])
            self.assertEqual(on(lines), ALWAYS_ON, path)

    def test_every_scope_is_one_a_pull_request_could_have(self):
        # What the summary jobs rely on: `db` is any of its suites, `web_promote` only with `web`,
        # `full` off only with every line off. And no line the build did not run is ever off.
        names = [name for name in EVERY if name not in ("full", "db")]
        rng = random.Random(3564)
        for _ in range(500):
            built = queue(true=[name for name in names if rng.random() < 0.5],
                          full=rng.choice(["true", "true", "false", ""]), bot=rng.choice(["true", "false"]))
            lines, _ = main_scope.scope(EVERY, built, [])
            suites = [lines[name] for name in lines if name.startswith("db_")]
            self.assertEqual(lines["db"], "true" if "true" in suites else "false")
            if lines["web_promote"] == "true":
                self.assertEqual(lines["web"], "true")
            others = [lines[name] for name in names]
            self.assertEqual(lines["full"], "true" if "true" in others else "false")
            for name in names:
                ran = built["full"] == "true" and built[name] == "true"
                if not ran and name != "web_promote":
                    self.assertEqual(lines[name], "true", f"{name} skipped by the build, so it runs")
            if built["bot"] != "true":
                self.assertEqual(lines["bot"], "true")
            for name, _ in main_scope.ALWAYS:
                self.assertEqual(lines[name], "true", f"{name} always runs on main")

    def test_every_line_that_always_runs_is_one_ci_scope_prints(self):
        # A line renamed in scripts/ci-scope.sh would otherwise leave ALWAYS holding nothing.
        for name, why in main_scope.ALWAYS:
            self.assertIn(name, EVERY)
            self.assertTrue(why)


class CommandTests(unittest.TestCase):
    def setUp(self):
        self.dir = Path(tempfile.mkdtemp(prefix="ci-main-scope-"))

    def tearDown(self):
        shutil.rmtree(self.dir)

    def write(self, name, lines):
        path = self.dir / name
        path.write_text("".join(f"{line}\n" for line in lines), encoding="utf-8")
        return str(path)

    def run_main(self, *args):
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            code = main_scope.main(["main_scope.py", *args])
        return code, out.getvalue(), err.getvalue()

    def test_it_reads_the_three_files_and_prints_one_line_a_row(self):
        every = self.write("every", [f"{name}=true" for name in EVERY])
        built = self.write("queue", [f"{name}={value}" for name, value in queue(true=("web", "e2e")).items()])
        changed = self.write("changed", ["apps/web/src/App.tsx", ""])
        code, out, err = self.run_main(every, built, changed)
        self.assertEqual(code, 0)
        rows = out.splitlines()
        self.assertEqual([row.split("=")[0] for row in rows], list(EVERY) + ["bot"])
        self.assertIn("web=false", rows)
        self.assertIn("e2e=false", rows)
        self.assertIn("db=true", rows)
        self.assertIn("main-scope:", err)

    def test_a_missing_queue_or_range_file_runs_everything(self):
        every = self.write("every", [f"{name}=true" for name in EVERY])
        built = self.write("queue", ["full=true", "web=true", "bot=true"])
        for args in ((every, str(self.dir / "none"), self.write("changed", [])),
                     (every, built, str(self.dir / "none"))):
            code, out, _ = self.run_main(*args)
            self.assertEqual(code, 0)
            self.assertTrue(all(row.endswith("=true") for row in out.splitlines()), out)

    def test_it_fails_when_main_has_no_lines(self):
        code, out, err = self.run_main(self.write("every", []), "queue", "changed")
        self.assertEqual((code, out), (1, ""))
        self.assertIn("no lines", err)


class WorkflowTests(unittest.TestCase):
    """ci.yml wires it: the queue keeps its scope, and a push to main reads it."""

    def setUp(self):
        self.text = CI_YML.read_text(encoding="utf-8").replace("\r\n", "\n")

    def test_the_merge_queue_build_keeps_its_scope(self):
        self.assertIn('cp "$GITHUB_OUTPUT" "$RUNNER_TEMP/scope.lines"', self.text)
        step = self.text[self.text.index("- name: keep the scope for main's run of this commit"):]
        step = step[:step.index("\n\n")]
        self.assertIn("if: ${{ github.event_name == 'merge_group' }}", step)
        self.assertIn("name: ci-scope", step)
        self.assertIn("path: ${{ runner.temp }}/scope.lines", step)

    def test_a_push_to_main_reads_the_queue_builds_scope(self):
        push = self.text[self.text.index('elif [ "$EVENT" = push ]; then'):]
        push = push[:push.index("\n          else\n")]
        self.assertIn("--event merge_group --commit \"$SHA\"", push)
        self.assertIn("--name ci-scope", push)
        self.assertIn('python3 .github/ci/main_scope.py "$every" "$queue/scope.lines" "$changed"', push)

    def test_the_deploy_image_comes_from_the_rehearsal_main_always_runs(self):
        # main_scope.ALWAYS keeps `db_deploy` on for the job that uploads `deploy-image`.
        job = self.text[self.text.index("  db-deploy-image:\n"):]
        job = job[:job.index("\n\n")]
        self.assertIn("needs.changes.outputs.db_deploy == 'true'", job)
        self.assertIn("name: deploy-image", job)
        self.assertIn("db_deploy", [name for name, _ in main_scope.ALWAYS])

    def test_the_changes_job_may_read_runs(self):
        job = self.text[self.text.index("  changes:\n"):]
        job = job[:job.index("    outputs:\n")]
        self.assertIn("actions: read", job)


if __name__ == "__main__":
    unittest.main()
