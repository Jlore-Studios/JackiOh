"""The conflicts the harness resolves itself (`harness/resolve.py`, `resolvers` in the config):
against real merges in real git repositories, and through the model job with a scripted model."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from harness import resolve
from harness.config import ConfigError, Resolver
from harness.git import Git
from harness.prompts import data
from harness.runner import FakeRunner, RunRequest, RunResult
from harness.threads import NO_FEEDBACK
from harness.work import Worker

from tests.fakes import git, make_origin, push_branch
from tests.support import make_config, raw_config

INDEX = "spec/INDEX.md"
GOLDEN = "crates/engine/tests/golden/games.jsonl"
GAME = "src/game.txt"
BLESS = f"printf 'blessed\\n' > {GOLDEN}"
TAKE_INDEX = Resolver(INDEX, take="main")
RUN_GOLDEN = Resolver(GOLDEN, run=BLESS, timeout_minutes=1)
BASE = {"README.md": "hello\n", GAME: "rules v1\n", INDEX: "| R1 |\n", GOLDEN: "game 1\n"}
LOTS_OF_TIME = lambda: 3600.0  # noqa: E731


def conflicted(root: Path, branch: dict[str, str | None], main: dict[str, str | None]) -> Git:
    """A clone of a repository whose `feature` branch and `main` both changed from BASE, with
    `main` merged into `feature` and the merge left open. A None deletes the file."""
    work = root / "repo"
    work.mkdir(parents=True)
    git(work, "init", "-q", "-b", "main")
    for name, text in BASE.items():
        (work / name).parent.mkdir(parents=True, exist_ok=True)
        (work / name).write_text(text)
    git(work, "add", "-A")
    git(work, "commit", "-q", "-m", "base")

    def commit(changes: dict[str, str | None], message: str) -> None:
        for name, text in changes.items():
            if text is None:
                git(work, "rm", "-q", name)
            else:
                (work / name).write_text(text)
        git(work, "add", "-A")
        git(work, "commit", "-q", "-m", message)

    git(work, "checkout", "-q", "-b", "feature")
    commit(branch, "the branch")
    git(work, "checkout", "-q", "main")
    commit(main, "main moves")
    git(work, "checkout", "-q", "feature")
    repo = Git(work)
    repo.run("merge", "--no-edit", "--no-ff", "main", check=False,
             extra_env={"GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t",
                        "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@t"})
    return repo


def unmerged(repo: Git) -> list[str]:
    return [p for p in repo.out("diff", "--name-only", "--diff-filter=U").splitlines() if p]


class ResolveTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp())

    def run_resolve(self, repo: Git, resolvers=(TAKE_INDEX, RUN_GOLDEN), **kwargs):
        return resolve.resolve(repo, unmerged(repo), tuple(resolvers), {"PATH": "/usr/bin:/bin"},
                               kwargs.pop("seconds_left", LOTS_OF_TIME), **kwargs)

    def test_take_main_takes_mains_version_and_stages_it(self):
        repo = conflicted(self.root, {INDEX: "| R1 |\n| R2 branch |\n"}, {INDEX: "| R1 |\n| R3 main |\n"})
        self.assertEqual(unmerged(repo), [INDEX])
        left, done = self.run_resolve(repo)
        self.assertEqual(left, [])
        self.assertEqual([(r.path, r.ok) for r in done], [(INDEX, True)])
        self.assertEqual((repo.cwd / INDEX).read_text(), "| R1 |\n| R3 main |\n")
        self.assertEqual(unmerged(repo), [])

    def test_take_main_follows_main_when_it_deleted_the_file(self):
        repo = conflicted(self.root, {INDEX: "| R1 |\n| R2 branch |\n"}, {INDEX: None})
        self.assertEqual(unmerged(repo), [INDEX])
        left, done = self.run_resolve(repo)
        self.assertEqual(left, [])
        self.assertIn("deleted", done[0].how)
        self.assertFalse((repo.cwd / INDEX).exists())
        self.assertEqual(unmerged(repo), [])

    def test_a_command_rewrites_the_file_once_nothing_else_is_conflicted(self):
        repo = conflicted(self.root, {INDEX: "| R2 |\n", GOLDEN: "game 1 branch\n"},
                          {INDEX: "| R3 |\n", GOLDEN: "game 1 main\n"})
        self.assertEqual(sorted(unmerged(repo)), [GOLDEN, INDEX])
        left, done = self.run_resolve(repo)
        self.assertEqual(left, [])
        self.assertTrue(all(r.ok for r in done), done)
        self.assertEqual((repo.cwd / GOLDEN).read_text(), "blessed\n")
        self.assertEqual(unmerged(repo), [])

    def test_a_command_never_runs_while_a_file_no_resolver_names_is_conflicted(self):
        """It builds the merged tree, which a code file full of markers breaks: the builder
        resolves the code first, and the traces with it."""
        repo = conflicted(self.root, {GAME: "rules branch\n", GOLDEN: "game 1 branch\n"},
                          {GAME: "rules main\n", GOLDEN: "game 1 main\n"})
        before = (repo.cwd / GOLDEN).read_bytes()
        left, done = self.run_resolve(repo, (TAKE_INDEX, Resolver(GOLDEN, run="touch ran")))
        self.assertEqual(sorted(left), [GOLDEN, GAME])
        self.assertFalse((repo.cwd / "ran").exists())
        self.assertEqual((repo.cwd / GOLDEN).read_bytes(), before)
        self.assertEqual(len(done), 1)
        self.assertFalse(done[0].ok)
        self.assertIn(f"`{GAME}`", done[0].why_not)

    def test_a_take_resolver_still_takes_while_code_is_conflicted(self):
        repo = conflicted(self.root, {GAME: "rules branch\n", INDEX: "| R2 |\n"},
                          {GAME: "rules main\n", INDEX: "| R3 |\n"})
        left, done = self.run_resolve(repo)
        self.assertEqual(left, [GAME])
        self.assertEqual([(r.path, r.ok) for r in done], [(INDEX, True)])
        self.assertIn("<<<<<<<", (repo.cwd / GAME).read_text())
        self.assertEqual(unmerged(repo), [GAME])

    def test_a_failing_command_puts_the_conflict_back_byte_for_byte(self):
        repo = conflicted(self.root, {GOLDEN: "game 1 branch\n"}, {GOLDEN: "game 1 main\n"})
        before = (repo.cwd / GOLDEN).read_bytes()
        self.assertIn(b"<<<<<<<", before)
        failing = Resolver(GOLDEN, run=f"printf 'half' > {GOLDEN}; echo boom >&2; exit 3")
        left, done = self.run_resolve(repo, (failing,))
        self.assertEqual(left, [GOLDEN])
        self.assertEqual((repo.cwd / GOLDEN).read_bytes(), before)
        self.assertEqual(unmerged(repo), [GOLDEN])
        self.assertFalse(done[0].ok)
        self.assertIn("exited 3", done[0].why_not)
        self.assertIn("boom", done[0].why_not)

    def test_a_command_that_leaves_a_marker_resolves_nothing(self):
        repo = conflicted(self.root, {GOLDEN: "game 1 branch\n"}, {GOLDEN: "game 1 main\n"})
        before = (repo.cwd / GOLDEN).read_bytes()
        left, done = self.run_resolve(repo, (Resolver(GOLDEN, run="true"),))
        self.assertEqual(left, [GOLDEN])
        self.assertIn("as the merge left it", done[0].why_not)
        self.assertEqual((repo.cwd / GOLDEN).read_bytes(), before)
        sneaky = Resolver(GOLDEN, run=f"printf '<<<<<<< ours\\nx\\n>>>>>>> theirs\\n' > {GOLDEN}")
        left, done = self.run_resolve(repo, (sneaky,))
        self.assertEqual(left, [GOLDEN])
        self.assertIn("marker", done[0].why_not)
        self.assertEqual((repo.cwd / GOLDEN).read_bytes(), before)

    def test_a_run_cut_off_mid_command_still_puts_the_conflict_back(self):
        """The job's signal handler raises inside the command (`work.Worker._on_signal`): the
        half-written file must not survive for a work-in-progress commit to take up."""
        repo = conflicted(self.root, {GOLDEN: "game 1 branch\n"}, {GOLDEN: "game 1 main\n"})
        before = (repo.cwd / GOLDEN).read_bytes()

        class Stopped(Exception):
            pass

        def cut_off(gate, cwd, env, timeout_s=None):
            (Path(cwd) / GOLDEN).write_text("half-writt")
            raise Stopped("SIGTERM")
        original = resolve.gates_mod.run_gate
        resolve.gates_mod.run_gate = cut_off
        try:
            with self.assertRaises(Stopped):
                self.run_resolve(repo, (Resolver(GOLDEN, run="anything"),))
        finally:
            resolve.gates_mod.run_gate = original
        self.assertEqual((repo.cwd / GOLDEN).read_bytes(), before)
        self.assertEqual(unmerged(repo), [GOLDEN])

    def test_a_command_never_decides_a_deletion(self):
        for branch, main in (({GOLDEN: "game 1 branch\n"}, {GOLDEN: None}),
                             ({GOLDEN: None}, {GOLDEN: "game 1 main\n"})):
            with self.subTest(main_deleted=main[GOLDEN] is None):
                repo = conflicted(Path(tempfile.mkdtemp()), branch, main)
                self.assertEqual(unmerged(repo), [GOLDEN])
                left, done = self.run_resolve(repo, (Resolver(GOLDEN, run="touch ran"),))
                self.assertEqual(left, [GOLDEN])
                self.assertIn("deleted", done[0].why_not)
                self.assertFalse((repo.cwd / "ran").exists())

    def test_a_command_waits_for_time_beyond_the_reserve(self):
        repo = conflicted(self.root, {GOLDEN: "game 1 branch\n"}, {GOLDEN: "game 1 main\n"})
        left, done = self.run_resolve(repo, (Resolver(GOLDEN, run="touch ran"),),
                                      seconds_left=lambda: 1000.0, reserve_seconds=900.0)
        self.assertEqual(left, [GOLDEN])
        self.assertIn("too little", done[0].why_not)
        self.assertFalse((repo.cwd / "ran").exists())

    def test_take_names_the_branch_it_took_from(self):
        repo = conflicted(self.root, {INDEX: "| R2 |\n"}, {INDEX: "| R3 |\n"})
        _, done = self.run_resolve(repo, base="bot/tree-5")
        self.assertEqual(done[0].how, "took `bot/tree-5`'s version")

    def test_a_command_that_deletes_the_file_resolves_nothing(self):
        repo = conflicted(self.root, {GOLDEN: "game 1 branch\n"}, {GOLDEN: "game 1 main\n"})
        before = (repo.cwd / GOLDEN).read_bytes()
        left, done = self.run_resolve(repo, (Resolver(GOLDEN, run=f"rm {GOLDEN}"),))
        self.assertEqual(left, [GOLDEN])
        self.assertIn("no file", done[0].why_not)
        self.assertEqual((repo.cwd / GOLDEN).read_bytes(), before)

    def test_no_command_runs_without_the_dependencies_or_the_time(self):
        for kwargs, why in (({"commands": False}, "did not install"),
                            ({"seconds_left": lambda: 30.0}, "too little")):
            with self.subTest(why=why):
                root = Path(tempfile.mkdtemp())
                repo = conflicted(root, {GOLDEN: "game 1 branch\n"}, {GOLDEN: "game 1 main\n"})
                left, done = self.run_resolve(repo, (Resolver(GOLDEN, run="touch ran"),), **kwargs)
                self.assertEqual(left, [GOLDEN])
                self.assertFalse((repo.cwd / "ran").exists())
                self.assertIn(why, done[0].why_not)

    def test_a_file_no_resolver_names_is_left_exactly_as_git_left_it(self):
        repo = conflicted(self.root, {GAME: "rules branch\n"}, {GAME: "rules main\n"})
        before = (repo.cwd / GAME).read_bytes()
        left, done = self.run_resolve(repo)
        self.assertEqual((left, done), ([GAME], []))
        self.assertEqual((repo.cwd / GAME).read_bytes(), before)

    def test_the_summary_says_what_was_done_and_what_is_left(self):
        done = [resolve.Resolution(INDEX, "took `main`'s version", True),
                resolve.Resolution(GOLDEN, "ran `x`", False, "it exited 1: no")]
        line = resolve.summary(done, [GOLDEN, GAME])
        self.assertIn(f"`{INDEX}`: took `main`'s version", line)
        self.assertIn(f"`{GOLDEN}`: ran `x`, not resolved: it exited 1: no", line)
        self.assertIn("2 file(s) left for the builder", line)


class ConfigTests(unittest.TestCase):
    def parse(self, resolvers):
        return make_config(resolvers=resolvers).resolvers

    def test_the_committed_resolvers(self):
        """INDEX.md takes `main`'s version (the daily spec fold rewrites it, and no branch should
        touch it); the golden traces are blessed from the merged tree, naming the file outright
        so a binary built elsewhere cannot write somewhere else."""
        resolvers = {r.path: r for r in make_config().resolvers}
        self.assertEqual(set(resolvers), {INDEX, GOLDEN})
        self.assertEqual(resolvers[INDEX].take, "main")
        self.assertEqual(resolvers[GOLDEN].run, f"cargo jackioh golden bless --file {GOLDEN}")
        self.assertGreaterEqual(resolvers[GOLDEN].timeout_minutes, 10)
        squishy = Path(__file__).resolve().parents[2] / ".squishy" / "config.json"
        import json
        self.assertEqual(json.loads(squishy.read_text())["resolvers"], raw_config()["resolvers"])

    def test_none_is_none(self):
        raw = raw_config()
        raw.pop("resolvers", None)
        from harness import config as config_mod
        from tests.support import ROOT, TEST_ENV, test_pool
        self.assertEqual(config_mod.parse(raw, ROOT, TEST_ENV, pool=test_pool()).resolvers, ())

    def test_a_resolver_names_one_file_and_one_way(self):
        for bad, why in (
                ([{"path": INDEX}], "exactly one"),
                ([{"path": INDEX, "take": "main", "run": "x"}], "exactly one"),
                ([{"path": INDEX, "take": "branch"}], "`take` is one of"),
                ([{"path": "spec/*.md", "take": "main"}], "no glob"),
                ([{"path": "../etc/passwd", "take": "main"}], "no glob"),
                ([{"path": "/etc/passwd", "take": "main"}], "no glob"),
                ([{"path": INDEX, "take": "main", "when": "always"}], "unknown keys"),
                ([{"path": GOLDEN, "run": "x", "timeout_minutes": 0}], "at least 1"),
                ([{"path": INDEX, "take": "main"}, {"path": INDEX, "run": "x"}], "already"),
                ({"path": INDEX}, "expected a list"),
                ([{"path": "./spec/INDEX.md", "take": "main"}], "plain repository path"),
                ([{"path": "spec//INDEX.md", "take": "main"}], "plain repository path"),
                ([{"path": "spec/", "take": "main"}], "plain repository path"),
                ([{"path": "spec\\INDEX.md", "take": "main"}], "plain repository path"),
                ([{"path": GOLDEN, "run": True}], "strings"),
                ([{"path": GOLDEN, "run": ["cargo", "x"]}], "strings"),
                ([{"path": INDEX, "take": "main", "timeout_minutes": 5}], "for a `run` resolver"),
                ([{"path": GOLDEN, "run": "x", "timeout_minutes": 2.9}], "whole number"),
                ([{"path": GOLDEN, "run": "x", "timeout_minutes": "7"}], "whole number"),
                ([{"path": GOLDEN, "run": "x", "timeout_minutes": True}], "whole number"),
                ([{"path": GOLDEN, "run": "x"}, {"path": GAME, "run": "y"}], "one `run` resolver")):
            with self.subTest(bad=bad):
                with self.assertRaises(ConfigError) as caught:
                    self.parse(bad)
                self.assertIn(why, str(caught.exception))


DONE = '<!-- bot: {"status": "done", "title": "PR"} -->\n## What changed\nResolved.'
APPROVE = '<!-- review: {"verdict": "approve", "findings": []} -->\nRead the merge; fine.'
CHANGES = ('<!-- review: {"verdict": "changes", "findings": [{"severity": "blocking", '
           '"where": "src/game.txt", "claim": "say v3", "evidence": "read it"}]} -->\nNo.')
GATES = [{"name": "rules exist", "run": f"test -f {GAME}", "timeout_minutes": 1},
         {"name": "no markers", "run": "! git grep -q -E '^(<<<<<<< |>>>>>>> )' -- .",
          "timeout_minutes": 1}]


class WorkerTests(unittest.TestCase):
    """A conflict revision whose every conflict the harness settles spends no builder session:
    its checks and its reviewer judge the merge. Anything else reaches the builder, told what the
    harness already did."""

    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        self.origin, self.clone = make_origin(self.root, dict(BASE))
        self.cfg = make_config(gates=GATES, install={"run": "true", "timeout_minutes": 1},
                               max_review_cycles=3,
                               resolvers=[{"path": INDEX, "take": "main"},
                                          {"path": GOLDEN, "run": BLESS, "timeout_minutes": 1}])

    def diverge(self, branch: dict[str, str], main: dict[str, str]) -> str:
        start = push_branch(self.origin, self.root, "feature/x", branch)
        push_branch(self.origin, self.root, "main", main)
        git(self.clone, "fetch", "-q", "origin")
        return start

    def plan(self, feedback: str = data(NO_FEEDBACK, "Feedback on #30"), **extra) -> dict:
        return {"action": "revise", "number": 30, "title": "PR", "branch": "feature/x",
                "source": "conflict", "pull": "p", "issue": "", "feedback": feedback,
                "thread": "t", **extra}

    def run_worker(self, runner: FakeRunner, plan: dict, cfg=None) -> dict:
        worker = Worker(cfg or self.cfg, plan, runner, self.clone, self.root / "work",
                        self.root / "out")
        return worker.run()

    def head(self) -> tuple[Path, str]:
        check = self.root / "check"
        if not check.exists():
            git(self.root, "clone", "-q", str(self.origin), str(check))
        git(check, "fetch", "-q", str(self.root / "out" / "branch.bundle"),
            "+refs/heads/feature/x:refs/heads/got")
        return check, git(check, "rev-parse", "got")

    def test_a_conflict_only_in_resolver_files_spends_no_builder_session(self):
        start = self.diverge({INDEX: "| R1 |\n| R2 |\n", GOLDEN: "game 1 branch\n", GAME: "rules v2\n"},
                             {INDEX: "| R1 |\n| R3 |\n", GOLDEN: "game 1 main\n"})
        runner = FakeRunner({"review": lambda r: RunResult(True, APPROVE)})
        result = self.run_worker(runner, self.plan())
        self.assertEqual(result["status"], "approved", result.get("reason"))
        self.assertEqual([c.role for c in runner.calls], ["review"])
        check, head = self.head()
        parents = git(check, "rev-list", "--parents", "-n", "1", head).split()
        self.assertEqual(parents[1], start)
        self.assertEqual(git(check, "show", f"{head}:{INDEX}"), "| R1 |\n| R3 |")
        self.assertEqual(git(check, "show", f"{head}:{GOLDEN}"), "blessed")
        self.assertEqual(git(check, "show", f"{head}:{GAME}"), "rules v2")
        builder = result["cycles"][0]["builder"]
        self.assertEqual((builder["role"], builder["model"], builder["ok"]), ("merge", "the harness", True))
        self.assertTrue(all(g["ok"] for g in result["cycles"][0]["gates"]))
        # The take resolvers go first, then the command once nothing else is conflicted.
        self.assertEqual([(r["path"], r["ok"]) for r in result["resolved"]],
                         [(INDEX, True), (GOLDEN, True)])
        self.assertIn("no builder ran", result["report"])
        self.assertIn(f"`{GOLDEN}`: ran `{BLESS}`", result["report"])

    def test_a_comment_that_quotes_the_empty_feedback_is_still_feedback(self):
        self.diverge({INDEX: "| R2 |\n"}, {INDEX: "| R3 |\n"})
        runner = FakeRunner({"revise": lambda r: RunResult(True, DONE),
                             "review": lambda r: RunResult(True, APPROVE)})
        quoting = data(f"Comment by @jgoetzmann:\n\nYou said {NO_FEEDBACK}; rename rules to laws.",
                       "Feedback on #30")
        self.run_worker(runner, self.plan(feedback=quoting))
        self.assertEqual(runner.calls[0].role, "revise")

    def test_every_reviewer_hears_the_harness_resolved_by_a_rule(self):
        self.diverge({INDEX: "| R2 |\n", GAME: "rules v2\n"}, {INDEX: "| R3 |\n"})
        runner = FakeRunner({"review": lambda r: RunResult(True, APPROVE)})
        result = self.run_worker(runner, self.plan())
        self.assertEqual(result["status"], "approved", result.get("reason"))
        review = runner.calls[0].prompt
        self.assertIn("a file it took from `main` keeps `main`'s version on purpose", review)
        self.assertIn(f"- `{INDEX}`: took `main`'s version", review)
        step = next(s for s in result["steps"] if s["step"] == "resolve")
        self.assertIn("minutes", step)
        self.assertIn(f"`{INDEX}`: took `main`'s version", step["outcome"])

    def test_a_merge_whose_tree_is_the_branchs_still_records_main(self):
        """`main` changed only the traces, and blessing the merged tree gives the branch's own:
        the merged tree is the branch's, yet the merge commit must still name `main`, or the pull
        request would conflict for ever."""
        self.diverge({GOLDEN: "game branch\n"}, {GOLDEN: "game main\n"})
        main = git(self.clone, "rev-parse", "origin/main")
        cfg = make_config(gates=GATES, install={"run": "true", "timeout_minutes": 1},
                          max_review_cycles=3,
                          resolvers=[{"path": GOLDEN, "run": f"printf 'game branch\\n' > {GOLDEN}",
                                      "timeout_minutes": 1}])
        runner = FakeRunner({"review": lambda r: RunResult(True, APPROVE)})
        result = self.run_worker(runner, self.plan(), cfg)
        self.assertEqual(result["status"], "approved", result.get("reason"))
        self.assertEqual([c.role for c in runner.calls], ["review"])
        check, head = self.head()
        self.assertIn(main, git(check, "rev-list", "--parents", "-n", "1", head).split()[1:])
        self.assertEqual(git(check, "show", f"{head}:{GOLDEN}"), "game branch")

    def test_the_review_still_judges_the_harness_merge_and_sends_findings_to_the_builder(self):
        self.diverge({INDEX: "| R2 |\n", GAME: "rules v2\n"}, {INDEX: "| R3 |\n"})

        def fix(request: RunRequest) -> RunResult:
            (request.cwd / GAME).write_text("rules v3\n")
            return RunResult(True, DONE)
        runner = FakeRunner({"review": [lambda r: RunResult(True, CHANGES), lambda r: RunResult(True, APPROVE)],
                             "fix": fix})
        result = self.run_worker(runner, self.plan())
        self.assertEqual(result["status"], "approved", result.get("reason"))
        self.assertEqual([c.role for c in runner.calls], ["review", "fix", "review"])
        self.assertIn("say v3", runner.calls[1].prompt)
        self.assertIn(f"- `{INDEX}`: took `main`'s version", runner.calls[1].prompt)
        check, head = self.head()
        self.assertEqual(git(check, "show", f"{head}:{GAME}"), "rules v3")

    def test_red_checks_on_the_harness_merge_are_not_approved(self):
        """A harness merge the checks turn red is not approved, whatever its reviewer says: the
        red goes back as a finding, as on any round."""
        self.diverge({INDEX: "| R2 |\n", GAME: "rules v2\n"}, {INDEX: "| R3 |\n"})
        # Red on the merged head only: `main` never had v2, so the red is this change's.
        red = {"name": "no v2", "run": f"! grep -q v2 {GAME}", "timeout_minutes": 1}
        cfg = make_config(gates=[red],
                          install={"run": "true", "timeout_minutes": 1}, max_review_cycles=1,
                          resolvers=[{"path": INDEX, "take": "main"}])
        runner = FakeRunner({"review": lambda r: RunResult(True, APPROVE),
                             "fix": lambda r: RunResult(True, DONE)})
        result = self.run_worker(runner, self.plan(), cfg)
        self.assertNotEqual(result["status"], "approved")
        self.assertEqual(runner.calls[0].role, "review")

    def test_new_feedback_still_goes_to_the_builder_told_what_the_harness_did(self):
        self.diverge({INDEX: "| R2 |\n"}, {INDEX: "| R3 |\n"})
        seen = {}

        def revise(request: RunRequest) -> RunResult:
            seen["index"] = (request.cwd / INDEX).read_text()
            return RunResult(True, DONE)
        runner = FakeRunner({"revise": revise, "review": lambda r: RunResult(True, APPROVE)})
        feedback = data("Comment by @jgoetzmann:\n\nPlease also rename it.", "Feedback on #30")
        result = self.run_worker(runner, self.plan(feedback=feedback))
        self.assertEqual(result["status"], "approved", result.get("reason"))
        self.assertEqual([c.role for c in runner.calls], ["revise", "review"])
        self.assertEqual(seen["index"], "| R3 |\n")
        prompt = runner.calls[0].prompt
        self.assertIn("The harness resolved these conflicts itself", prompt)
        self.assertIn(f"- `{INDEX}`: took `main`'s version", prompt)
        # Never "clean": the merge is still open, staged, for the harness to commit.
        self.assertNotIn("was clean", prompt)
        self.assertIn("the harness resolved every one (below). The merge is staged, not committed",
                      prompt)

    def test_a_code_conflict_reaches_the_builder_with_only_what_is_left(self):
        self.diverge({INDEX: "| R2 |\n", GAME: "rules branch\n", GOLDEN: "game branch\n"},
                     {INDEX: "| R3 |\n", GAME: "rules main\n", GOLDEN: "game main\n"})
        seen = {}

        def revise(request: RunRequest) -> RunResult:
            seen["index"] = (request.cwd / INDEX).read_text()
            seen["golden"] = (request.cwd / GOLDEN).read_text()
            (request.cwd / GAME).write_text("rules main and branch\n")
            (request.cwd / GOLDEN).write_text("game main and branch\n")
            return RunResult(True, DONE)
        runner = FakeRunner({"revise": revise, "review": lambda r: RunResult(True, APPROVE)})
        result = self.run_worker(runner, self.plan())
        self.assertEqual(result["status"], "approved", result.get("reason"))
        self.assertEqual([c.role for c in runner.calls], ["revise", "review"])
        self.assertEqual(seen["index"], "| R3 |\n")
        self.assertIn("<<<<<<<", seen["golden"])  # the traces wait for the code: the builder's
        prompt = runner.calls[0].prompt
        listed = prompt.split("Files merged with conflict markers")[1].split("The harness resolved")[0]
        self.assertIn(GAME, listed)
        self.assertIn(GOLDEN, listed)
        self.assertNotIn(INDEX, listed)
        self.assertIn(f"- `{INDEX}`: took `main`'s version", prompt)
        unresolved = [r for r in result["resolved"] if not r["ok"]]
        self.assertEqual([r["path"] for r in unresolved], [GOLDEN])

    def test_earlier_findings_still_go_to_the_builder(self):
        self.diverge({INDEX: "| R2 |\n"}, {INDEX: "| R3 |\n"})
        runner = FakeRunner({"revise": lambda r: RunResult(True, DONE),
                             "review": lambda r: RunResult(True, APPROVE)})
        findings = [{"severity": "blocking", "where": GAME, "claim": "still open", "evidence": "e"}]
        self.run_worker(runner, self.plan(previous_findings=findings))
        self.assertEqual(runner.calls[0].role, "revise")

    def test_a_ci_revision_is_never_merge_only(self):
        self.diverge({INDEX: "| R2 |\n"}, {INDEX: "| R3 |\n"})
        runner = FakeRunner({"revise": lambda r: RunResult(True, DONE),
                             "review": lambda r: RunResult(True, APPROVE)})
        self.run_worker(runner, {**self.plan(), "source": "ci"})
        self.assertEqual(runner.calls[0].role, "revise")

    def test_without_resolvers_the_builder_gets_every_conflict_as_before(self):
        self.diverge({INDEX: "| R2 |\n"}, {INDEX: "| R3 |\n"})
        cfg = make_config(gates=GATES[:1], install={"run": "true", "timeout_minutes": 1},
                          max_review_cycles=3, resolvers=[])
        seen = {}

        def revise(request: RunRequest) -> RunResult:
            seen["index"] = (request.cwd / INDEX).read_text()
            (request.cwd / INDEX).write_text("| R2 |\n| R3 |\n")
            return RunResult(True, DONE)
        runner = FakeRunner({"revise": revise, "review": lambda r: RunResult(True, APPROVE)})
        result = self.run_worker(runner, self.plan(), cfg)
        self.assertEqual(result["status"], "approved", result.get("reason"))
        self.assertIn("<<<<<<<", seen["index"])
        self.assertNotIn("resolved", result)
        self.assertNotIn("The harness resolved", runner.calls[0].prompt)
        # The standing guidance is true with no resolvers too: the builder takes `main`'s index.
        self.assertIn("`spec/INDEX.md` or the end of `spec/11-rulings.md`", runner.calls[0].prompt)
        self.assertIn("takes\n  `main`'s side", runner.calls[0].prompt)

    def test_a_cleared_changes_reviewer_hears_the_harness_resolved_it(self):
        start = self.diverge({INDEX: "| R2 |\n", GAME: "rules v2\n"}, {INDEX: "| R3 |\n"})
        runner = FakeRunner({"review": lambda r: RunResult(True, APPROVE)})
        result = self.run_worker(runner, self.plan(cleared={"sha": start, "by": "Opus"}))
        self.assertEqual(result["status"], "approved", result.get("reason"))
        self.assertEqual([c.role for c in runner.calls], ["review"])
        review = runner.calls[0].prompt
        self.assertIn(f"the harness resolved `{INDEX}` itself (took `main`'s version)", review)
        self.assertIn("the files the harness resolved follow its rule", review)


if __name__ == "__main__":
    unittest.main()
