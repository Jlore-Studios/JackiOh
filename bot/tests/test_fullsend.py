"""Fullsend (#505): an issue split into parts that land on one branch of its own, not `main`, and
the reconcile that turns that branch into one pull request into `main`.

These run in the night bot's process, which has the mode (`.harness/config.json`). Squishy's case
is in `squishy_cases.py`.
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from harness import commands, config, events, issueplan, sweep
from harness import plan as plan_mod
from harness import queue as queue_mod
from harness.config import LABEL_BUILD, LABEL_TREE, LABEL_WORKING, LABELS, MODE_LABELS
from harness.deliver import Deliverer
from harness.runner import FakeRunner, RunRequest, RunResult
from harness.work import Worker

from tests.fakes import OPERATOR, FakeGitHub, git, make_origin, push_branch
from tests.support import NIGHT, make_config, make_ctx

DONE = '<!-- bot: {"status": "done", "title": "Part done"} -->\n## What changed\nMy part.'
APPROVE = '<!-- review: {"verdict": "approve", "findings": []} -->\nFine.'
GATES = [{"name": "rules exist", "run": "test -f src/game.txt", "timeout_minutes": 1}]
FULLSEND = "bot:fullsend"
TREE = {"done": False, "summary": "Rules and client, side by side.", "issues": [
    {"key": "rules", "title": "Part 1: the rules", "body": "Write `src/rules.txt`.",
     "plan": "1. Write src/rules.txt.", "difficulty": "medium", "labels": ["patch"],
     "blocked_by": []},
    {"key": "client", "title": "Part 2: the client", "body": "Write `src/client.txt`.",
     "plan": "1. Write src/client.txt.", "difficulty": "medium", "labels": ["patch"],
     "blocked_by": []}]}
PARENT, RULES, CLIENT = 40, 41, 42  # the parts are opened in build order
ONTO = "bot/issue-40"


def answer(tree: dict) -> str:
    return f"How I cut it.\n\n```json\n{json.dumps(tree)}\n```"


def builder(edits: dict[str, str], prompts: list[str] | None = None):
    def handler(request: RunRequest) -> RunResult:
        if prompts is not None:
            prompts.append(request.prompt)
        for name, text in edits.items():
            target = request.cwd / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(text)
        return RunResult(True, DONE)
    return handler


def builds(edits: dict[str, str]) -> FakeRunner:
    """A runner whose builder writes `edits`: a part's run calls nothing else."""
    return FakeRunner({"build": builder(edits)})


class World:
    """A repository, a fake GitHub and the night bot's three jobs (plan, work, deliver)."""

    def __init__(self, env: dict | None = None, machine: tuple[str, ...] = (),
                 **overrides) -> None:
        self.root = Path(tempfile.mkdtemp())
        self.origin, self.clone = make_origin(self.root)
        self.gh = FakeGitHub()
        env = {"GITHUB_SERVER_URL": f"file://{self.root / 'remote'}", "BOT_GITHUB_TOKEN": "t" * 20,
               **(env or {})}
        self.cfg = make_config(env=env, machine=machine, gates=GATES,
                               install={"run": "true", "timeout_minutes": 1}, max_review_cycles=2,
                               **overrides)
        self.ctx = make_ctx(self.gh, cfg=self.cfg, at=NIGHT)
        self.gh.branch_checks = set(self.cfg.required_checks)
        self.deliver_repo = self.root / "deliver"
        git(self.root, "clone", "-q", str(self.origin), str(self.deliver_repo))
        self.gh.ensure_label("patch", "ededed", "patch")
        self.runs = 0

    def claim(self) -> dict:
        return plan_mod.make(self.ctx)

    def work(self, planned: dict, runner: FakeRunner) -> Path:
        self.runs += 1
        out = self.root / f"out-{self.runs}"
        Worker(self.cfg, planned, runner, self.clone, self.root / f"work-{self.runs}", out).run()
        return out

    def deliver(self, planned: dict, out: Path) -> dict:
        Deliverer(self.ctx, planned, out, self.deliver_repo).run()
        return json.loads((out / "result.json").read_text())

    def run(self, runner: FakeRunner) -> tuple[dict, dict]:
        planned = self.claim()
        if planned["action"] == "none":
            return planned, {}
        return planned, self.deliver(planned, self.work(planned, runner))

    def split(self) -> dict:
        self.gh.add_issue(PARENT, title="A big project", body="Do it all.\n\nDone when: it works.",
                          labels=(LABEL_BUILD, FULLSEND))
        _, result = self.run(FakeRunner({"plan": lambda r: RunResult(True, answer(TREE))}))
        return result

    def tip(self, branch: str) -> str:
        return git(self.origin, "rev-parse", f"refs/heads/{branch}")

    def has(self, branch: str, path: str) -> bool:
        try:
            git(self.origin, "cat-file", "-e", f"refs/heads/{branch}:{path}")
        except AssertionError:
            return False
        return True


class ModeTests(unittest.TestCase):
    def test_fullsend_is_a_night_bot_mode(self):
        self.assertEqual(config.MODES, ("fullsend",))
        self.assertEqual(MODE_LABELS, {FULLSEND: "fullsend"})
        for name in (FULLSEND, LABEL_TREE):
            self.assertIn(name, LABELS)
            self.assertIn("the night bot", LABELS[name][1])
            self.assertLessEqual(len(LABELS[name][1]), config.LABEL_DESCRIPTION_MAX)
        self.assertTrue(commands.offered("fullsend"))
        self.assertFalse(commands.offered("oneshot"))
        self.assertFalse(commands.offered("split"))
        text = commands.help_text("jgoetzmann-bot")
        self.assertIn("`fullsend [notes]`", text)
        self.assertIn("`build`, `fullsend`, `revise` or `suggest`", text)
        self.assertIn("`/harness fullsend`", commands.help_text("jgoetzmann-bot", "fullsend"))
        self.assertEqual([(c.verb, c.args) for c in commands.parse(
            "/harness fullsend\n@jgoetzmann-bot fullsend", "jgoetzmann-bot")],
            [("fullsend", ""), ("fullsend", "")])

    def test_the_label_queues_it_in_fullsend_mode_at_hard(self):
        world = World()
        world.gh.add_issue(PARENT, labels=(FULLSEND,))
        events.on_issue_change(world.ctx, {"action": "labeled", "issue": world.gh.get_issue(PARENT),
                                           "label": {"name": FULLSEND}, "sender": OPERATOR},
                               is_pr=False)
        self.assertEqual(world.gh.label_names(PARENT), {LABEL_BUILD, FULLSEND})
        self.assertIn("split it into parts that land on one branch",
                      world.gh.bot_comments(PARENT)[-1])
        found = queue_mod.candidates(world.ctx, world.ctx.store.load())
        self.assertEqual([(c.number, c.mode, c.difficulty, c.planned) for c in found],
                         [(PARENT, "fullsend", "hard", True)])
        planned = world.claim()
        self.assertEqual((planned["action"], planned["mode"], planned["difficulty"]),
                         ("build", "fullsend", "hard"))
        self.assertEqual(planned["seats"]["build"]["tier"], "strong")
        self.assertIn("Splitting this now for fullsend", world.gh.bot_comments(PARENT)[-1])

    def test_the_command_queues_it_and_squishys_fullsend_is_left_alone(self):
        world = World()
        world.gh.add_issue(3)
        asked = commands.parse("/harness fullsend", "jgoetzmann-bot")
        replies = events.run_commands(world.ctx, asked, world.gh.get_issue(3), OPERATOR, 3)
        self.assertIn("Queued #3", replies[0])
        self.assertEqual(world.gh.label_names(3), {LABEL_BUILD, FULLSEND})
        world.gh.add_issue(4, labels=("squishy:fullsend",))
        self.assertIn("is Squishy's (@squishy-squooby)",
                      queue_mod.queue_build(world.ctx, 4, by="jgoetzmann", mode="fullsend"))
        self.assertEqual(world.gh.label_names(4), {"squishy:fullsend"})


class SplitTests(unittest.TestCase):
    def test_the_split_sets_up_the_branch_and_opens_parts(self):
        world = World()
        prompts: list[str] = []

        def split(request: RunRequest) -> RunResult:
            prompts.append(request.prompt)
            return RunResult(True, answer(TREE))
        world.gh.add_issue(PARENT, title="A big project", body="Do it all.",
                           labels=(LABEL_BUILD, FULLSEND, "priority:high"))
        _, result = world.run(FakeRunner({"plan": split}))
        self.assertEqual(result["status"], "split")
        self.assertIn("## This is a fullsend split", prompts[0])
        self.assertIn(f"Each part lands on `{ONTO}`", prompts[0])
        # The branch the parts land on, made from main's head.
        self.assertEqual(world.tip(ONTO), world.tip("main"))
        for part in (RULES, CLIENT):
            self.assertEqual(world.gh.label_names(part), {"patch", "difficulty:medium",
                                                          "priority:high", LABEL_BUILD})
            self.assertIn(f"Its build lands on `{ONTO}`, not `main` (fullsend).",
                          world.gh.get_issue(part)["body"])
            record = world.ctx.store.load()["items"][str(part)]
            self.assertEqual((record["onto"], record["part_of"]), (ONTO, PARENT))
        self.assertEqual(world.gh.label_names(PARENT), {"priority:high", LABEL_TREE})
        self.assertIn(f"This is a fullsend tree: each part lands on `{ONTO}`",
                      world.gh.bot_comments(PARENT)[-1])
        tree = world.ctx.store.load()["items"][str(PARENT)]["tree"]
        self.assertEqual((tree["mode"], tree["children"], tree["onto"]),
                         ("fullsend", [RULES, CLIENT], ONTO))

    def test_fullsend_waits_while_parts_are_open(self):
        world = World()
        world.split()
        reply = queue_mod.queue_build(world.ctx, PARENT, by="jgoetzmann", mode="fullsend")
        self.assertIn(f"#{PARENT}'s parts are still open (#{RULES}, #{CLIENT})", reply)
        self.assertNotIn(LABEL_BUILD, world.gh.label_names(PARENT))
        # A queue label put on by hand does not build it either.
        world.gh.add_labels(PARENT, [LABEL_BUILD])
        skipped: list[str] = []
        found = queue_mod.candidates(world.ctx, world.ctx.store.load(), skipped)
        self.assertNotIn(PARENT, [c.number for c in found])
        self.assertIn(f"#{PARENT} skipped: its fullsend parts #{RULES}, #{CLIENT} are still open",
                      skipped)
        reply = queue_mod.queue_build(world.ctx, PARENT, by="jgoetzmann", label_present=True)
        self.assertIn("parts are still open", reply)
        self.assertNotIn(LABEL_BUILD, world.gh.label_names(PARENT))


class PartTests(unittest.TestCase):
    def setUp(self):
        self.world = World()
        self.world.split()

    def test_a_part_lands_on_the_branch_without_a_pull_request(self):
        world = self.world
        prompts: list[str] = []
        runner = FakeRunner({"build": builder({"src/rules.txt": "the rules\n"}, prompts)})
        planned, result = world.run(runner)
        self.assertEqual((planned["number"], planned["onto"], planned["part_of"]),
                         (RULES, ONTO, PARENT))
        self.assertEqual(result["status"], "part")
        self.assertEqual([call.role for call in runner.calls], ["build"])  # no checks, no review
        self.assertIn(f"This is part of fullsend tree #{PARENT}. Your change lands on `{ONTO}`",
                      prompts[0])
        self.assertEqual(world.gh.list_pulls(state="all"), [])
        self.assertTrue(world.has(ONTO, "src/rules.txt"))
        self.assertFalse(world.has("main", "src/rules.txt"))
        issue = world.gh.get_issue(RULES)
        self.assertEqual((issue["state"], issue.get("state_reason")), ("closed", "completed"))
        self.assertEqual(world.gh.label_names(RULES) & {LABEL_BUILD, LABEL_WORKING}, set())
        self.assertIn(f"It landed on `{ONTO}` at `{world.tip(ONTO)[:12]}`.",
                      world.gh.bot_comments(RULES)[-1])
        self.assertEqual(world.ctx.store.load()["items"][str(RULES)]["landed"], world.tip(ONTO))

    def test_two_parts_land_side_by_side(self):
        world = self.world
        first = world.claim()
        second = world.claim()  # the second starts before the first lands
        self.assertEqual((first["number"], second["number"]), (RULES, CLIENT))
        out_first = world.work(first, builds({"src/rules.txt": "rules\n"}))
        out_second = world.work(second, builds({"src/client.txt": "ui\n"}))
        main = world.tip("main")
        self.assertEqual(world.deliver(first, out_first)["status"], "part")
        landed = world.tip(ONTO)
        self.assertEqual(git(world.origin, "rev-list", "--count", f"{main}..{landed}"), "1")
        self.assertEqual(world.deliver(second, out_second)["status"], "part")
        tip = world.tip(ONTO)
        self.assertTrue(world.has(ONTO, "src/rules.txt"))
        self.assertTrue(world.has(ONTO, "src/client.txt"))
        parents = git(world.origin, "rev-list", "--parents", "-n", "1", tip).split()[1:]
        self.assertEqual(len(parents), 2)  # joined by a merge commit, never pushed with force
        self.assertEqual(parents[0], landed)
        self.assertIn(f"land #{CLIENT} on {ONTO}",
                      git(world.origin, "log", "-1", "--format=%s", tip))
        for part in (RULES, CLIENT):
            self.assertEqual(world.gh.get_issue(part)["state"], "closed")

    def test_a_conflicting_part_is_parked(self):
        world = self.world
        first = world.claim()
        second = world.claim()
        out_first = world.work(first, builds({"src/game.txt": "rules A\n"}))
        out_second = world.work(second, builds({"src/game.txt": "rules B\n"}))
        world.deliver(first, out_first)
        landed = world.tip(ONTO)
        result = world.deliver(second, out_second)
        self.assertEqual(world.tip(ONTO), landed)  # the branch keeps the part that landed first
        self.assertEqual(world.tip(f"bot/issue-{CLIENT}"), result["head"])
        self.assertEqual(world.ctx.store.load()["items"][str(PARENT)]["parked"],
                         [f"bot/issue-{CLIENT}"])
        self.assertEqual(world.gh.get_issue(CLIENT)["state"], "closed")
        self.assertIn(f"kept on `bot/issue-{CLIENT}` instead, for #{PARENT}'s reconcile to merge",
                      world.gh.bot_comments(CLIENT)[-1])

    def test_a_later_part_starts_from_the_parts_before_it(self):
        world = self.world
        world.run(builds({"src/rules.txt": "rules\n"}))
        landed = world.tip(ONTO)
        seen: list[bool] = []

        def client(request: RunRequest) -> RunResult:
            seen.append((request.cwd / "src" / "rules.txt").is_file())
            return builder({"src/client.txt": "ui\n"})(request)
        planned, result = world.run(FakeRunner({"build": client}))
        self.assertEqual((planned["number"], result["status"]), (CLIENT, "part"))
        self.assertEqual(seen, [True])
        self.assertEqual(result["start"], landed)
        self.assertEqual(result["changed_paths"], ["src/client.txt"])  # measured against the branch
        self.assertEqual(git(world.origin, "rev-parse", f"{world.tip(ONTO)}^"), landed)

    def test_a_halt_before_delivery_lands_nothing(self):
        world = self.world
        planned = world.claim()
        out = world.work(planned, builds({"src/rules.txt": "rules\n"}))
        world.ctx.store.update(lambda s: s.update(halted=True))
        self.assertEqual(world.deliver(planned, out)["status"], "part")
        self.assertEqual(world.tip(ONTO), world.tip("main"))
        self.assertEqual(world.gh.get_issue(RULES)["state"], "open")
        self.assertIn(LABEL_BUILD, world.gh.label_names(RULES))
        self.assertTrue(world.has(f"bot/issue-{RULES}", "src/rules.txt"))  # kept for the next run

    def test_a_part_touching_a_forbidden_path_lands_nothing(self):
        world = self.world
        planned = world.claim()
        runner = builds({"src/rules.txt": "rules\n", ".harness/HALT": "no\n"})
        # The work job's guard puts such a change back; deliver refuses it all the same.
        with mock.patch.object(Worker, "_guard", lambda self: []):
            out = world.work(planned, runner)
        self.assertEqual(world.deliver(planned, out)["status"], "part")
        self.assertEqual(world.tip(ONTO), world.tip("main"))
        self.assertIn("the change touches paths the bot may not change: .harness/HALT",
                      world.gh.bot_comments(RULES)[-1])
        self.assertEqual(world.gh.get_issue(RULES)["state"], "open")
        self.assertIn(LABEL_BUILD, world.gh.label_names(RULES))


class EasyPartTests(unittest.TestCase):
    def test_a_weak_builders_part_is_measured_against_the_branch(self):
        """The easy rule counts only the part's own change, not the parts that landed before
        it: here one file is the limit, and an earlier part already added another."""
        world = World(env={"HARNESS_SECRETS_SET": ""}, machine=("devin",),
                      easy={"max_files": 1, "max_lines": 400})
        world.gh.add_issue(PARENT, labels=(LABEL_TREE,))
        push_branch(world.origin, world.root, ONTO, {"src/rules.txt": "rules\n"})
        world.gh.add_issue(CLIENT, labels=(LABEL_BUILD, "difficulty:easy"),
                           body=issueplan.with_plan("Write `src/client.txt`.",
                                                    "1. Write src/client.txt.", "a person"))
        world.gh.add_sub_issue(PARENT, world.gh.get_issue(CLIENT)["id"])
        world.ctx.store.update(lambda s: s["items"].setdefault(str(CLIENT), {}).update(
            onto=ONTO, part_of=PARENT))
        runner = builds({"src/client.txt": "ui\n"})
        planned, result = world.run(runner)
        self.assertEqual((planned["provider"], result["status"]), ("devin", "part"))
        self.assertEqual([call.role for call in runner.calls], ["build"])  # no self check
        self.assertTrue(world.has(ONTO, "src/client.txt"))
        self.assertEqual(world.gh.get_issue(CLIENT)["state"], "closed")
        # A part of its own that breaks the rule lands nothing, and a stronger model takes it.
        world.gh.add_issue(43, labels=(LABEL_BUILD, "difficulty:easy"), body=issueplan.with_plan(
            "Write two files.", "1. Write them.", "a person"))
        world.ctx.store.update(lambda s: s["items"].setdefault("43", {}).update(
            onto=ONTO, part_of=PARENT))
        landed = world.tip(ONTO)
        _, result = world.run(builds({"src/a.txt": "a\n", "src/b.txt": "b\n"}))
        self.assertEqual(result["status"], "part")
        self.assertEqual(world.tip(ONTO), landed)
        self.assertIn("breaks the easy rule (line 1: it changes 2 files, over 1)",
                      world.gh.bot_comments(43)[-1])
        self.assertEqual(world.gh.get_issue(43)["state"], "open")
        self.assertIn(LABEL_BUILD, world.gh.label_names(43))


class ReconcileTests(unittest.TestCase):
    def test_the_reconcile_opens_one_pull_into_main(self):
        world = World()
        world.split()
        first = world.claim()
        second = world.claim()
        out_first = world.work(first, builds({"src/game.txt": "rules A\n"}))
        out_second = world.work(second, builds({"src/game.txt": "rules B\n"}))
        world.deliver(first, out_first)
        world.deliver(second, out_second)  # parked
        world.ctx.store.update(lambda s: s.update(last_sweep={"since": "2026-09-29T00:00:00Z"}))
        notes = sweep._trees(world.ctx, NIGHT)
        self.assertEqual(notes, [f"#{PARENT}: its parts have all closed; queued its reconcile"])
        self.assertEqual(world.gh.label_names(PARENT), {LABEL_TREE, LABEL_BUILD, FULLSEND})
        self.assertIn(f"one run merges the parts on `{ONTO}`", world.gh.bot_comments(PARENT)[-1])
        self.assertIn("I will reconcile its parts into one pull request",
                      world.gh.bot_comments(PARENT)[-1])
        prompts: list[str] = []
        created: list[dict] = []
        make_pull = world.gh.create_pull

        def create_pull(**fields):
            created.append(fields)
            return make_pull(**fields)
        with mock.patch.object(world.gh, "create_pull", create_pull):
            planned, result = world.run(FakeRunner({
                "build": builder({"src/game.txt": "rules A and B\n"}, prompts),
                "review": lambda r: RunResult(True, APPROVE)}))
        self.assertEqual((planned["number"], planned["mode"], planned["difficulty"]),
                         (PARENT, "fullsend", "hard"))
        self.assertIn(f"#{RULES} [closed] Part 1: the rules", planned["children"])
        self.assertEqual(planned["parked"], [f"bot/issue-{CLIENT}"])
        self.assertNotIn("onto", planned)
        self.assertIn(f"# Reconcile fullsend tree #{PARENT}", prompts[0])
        self.assertIn(f"`origin/bot/issue-{CLIENT}`", prompts[0])
        self.assertIn(".claude/skills/fullsend/SKILL.md", prompts[0])
        self.assertEqual(result["status"], "approved")
        self.assertEqual(len(created), 1)
        self.assertEqual((created[0]["head"], created[0]["base"]), (ONTO, "main"))
        self.assertIn(f"Closes #{PARENT}", created[0]["body"])
        # The pull request holds the parts that landed and the reconcile's own change.
        self.assertEqual(git(world.origin, "show", f"refs/heads/{ONTO}:src/game.txt"),
                         "rules A and B")


if __name__ == "__main__":
    unittest.main()
