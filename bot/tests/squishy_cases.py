"""Squishy's own cases (#60), in a process that is Squishy from the start.

`test_squishy.SquishyProcessTests` runs this module with `HARNESS_HOME=.squishy` (the labels, the
slash command and the branches are fixed when a process starts, so the night bot's process cannot
check them). Discovery skips it (its name is not `test_*.py`), and it skips itself in any other
process.
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from harness import commands, config, dashboard, events, journal, sweep
from harness import plan as plan_mod
from harness import queue as queue_mod
from harness.config import LABEL_PR_OPEN, LABEL_TREE, LABELS, MODE_LABELS
from harness.deliver import Deliverer
from harness.issueplan import START as PLAN_START
from harness.runner import FakeRunner, RunRequest, RunResult
from harness.work import Worker

from tests.fakes import OPERATOR, FakeGitHub, git, make_origin
from tests.support import NIGHT, make_config, make_ctx

ENV = {"HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN_SQUISHY"}
DONE = '<!-- bot: {"status": "done", "title": "Rules v2, in one shot"} -->\n## What changed\nAll.'
APPROVE = '<!-- review: {"verdict": "approve", "findings": []} -->\nFine.'
GATES = [{"name": "rules exist", "run": "test -f src/game.txt", "timeout_minutes": 1}]
TREE = {"done": False, "summary": "Engine first, then the client.", "issues": [
    {"key": "engine", "title": "Part 1: the engine", "body": "Rule it in `engine.ts`.",
     "plan": "1. Write the rule. 2. Test it.", "difficulty": "hard", "labels": ["patch", "human"],
     "blocked_by": []},
    {"key": "client", "title": "Part 2: the client", "body": "Draw it.", "plan": "1. Draw.",
     "difficulty": "easy", "labels": ["patch", "squishy:build"], "blocked_by": ["engine"]}]}


def setUpModule() -> None:
    if config.HOME != ".squishy":
        raise unittest.SkipTest("run as Squishy (HARNESS_HOME=.squishy) by test_squishy.py")


def answer(tree: dict) -> str:
    return f"How I cut it.\n\n```json\n{json.dumps(tree)}\n```"


class World:
    """A repository, a fake GitHub and Squishy's three jobs (plan, work, deliver)."""

    def __init__(self, at=NIGHT) -> None:
        self.root = Path(tempfile.mkdtemp())
        self.origin, self.clone = make_origin(self.root)
        self.gh = FakeGitHub()
        env = {**ENV, "GITHUB_SERVER_URL": f"file://{self.root / 'remote'}",
               "BOT_GITHUB_TOKEN": "t" * 20}
        self.cfg = make_config(env=env, gates=GATES,
                               install={"run": "true", "timeout_minutes": 1}, max_review_cycles=2)
        self.ctx = make_ctx(self.gh, cfg=self.cfg, at=at)
        self.gh.branch_checks = set(self.cfg.required_checks)
        self.deliver_repo = self.root / "deliver"
        git(self.root, "clone", "-q", str(self.origin), str(self.deliver_repo))
        for name in ("patch", "human", "priority:high"):
            self.gh.ensure_label(name, "ededed", name)

    def ask(self, number: int, body: str) -> list[str]:
        return events.run_commands(self.ctx, commands.parse(body, self.cfg.bot_login),
                                   self.gh.get_issue(number), OPERATOR, 3)

    def run(self, runner: FakeRunner) -> tuple[dict, dict]:
        planned = plan_mod.make(self.ctx)
        out = self.root / f"out-{len(list(self.root.glob('out-*')))}"
        if planned["action"] != "none":
            Worker(self.cfg, planned, runner, self.clone, self.root / "work", out).run()
            Deliverer(self.ctx, planned, out, self.deliver_repo).run()
        result = json.loads((out / "result.json").read_text()) if (out / "result.json").exists() \
            else {}
        return planned, result


class IdentityCases(unittest.TestCase):
    def test_everything_is_squishys(self):
        self.assertEqual((config.LABEL_BUILD, config.SLASH, config.STATE_BRANCH),
                         ("squishy:build", "/squishy", "squishy-state"))
        self.assertEqual(config.NIGHT_WORKFLOW, "squishy-run.yml")
        self.assertEqual(journal.BRANCH, "squishy-journal")
        self.assertEqual(queue_mod.branch_for_issue(5), "squishy/issue-5")
        self.assertEqual(queue_mod.wip_branch(5), "squishy/wip/5")
        self.assertEqual(config.COMMIT_PREFIX, "squishy")
        self.assertEqual(set(MODE_LABELS), {"squishy:oneshot", "squishy:split",
                                            "squishy:split-bot"})

    def test_its_labels_and_no_suggestions(self):
        self.assertNotIn("squishy:suggestion", LABELS)
        self.assertNotIn("squishy:approved", LABELS)
        for name in ("squishy:build", "squishy:oneshot", "squishy:split", "squishy:split-bot",
                     LABEL_TREE, "human"):
            self.assertIn(name, LABELS)
        self.assertIn("Squishy", LABELS["squishy:working"][1])
        self.assertTrue(all(len(d) <= config.LABEL_DESCRIPTION_MAX for _, d in LABELS.values()))
        self.assertFalse(make_config(env=ENV).suggestions_enabled)

    def test_its_commands(self):
        bot = "squishy-squooby"
        self.assertEqual([(c.verb, c.args) for c in commands.parse(
            "/squishy oneshot\n/squishy split bot\n@squishy-squooby split", bot)],
            [("oneshot", ""), ("split", "bot"), ("split", "")])
        self.assertEqual(commands.parse("/harness build", bot), [])
        text = commands.help_text(bot)
        self.assertIn("`oneshot [notes]`", text)
        self.assertIn("`split [bot] [notes]`", text)
        self.assertNotIn("`suggest`", text)
        self.assertIn("`/squishy help`", commands.pointer(bot))


class QueueCases(unittest.TestCase):
    def setUp(self):
        self.world = World()
        self.gh = self.world.gh

    def test_oneshot_and_split_queue_in_their_mode(self):
        self.gh.add_issue(3)
        reply = self.world.ask(3, "/squishy oneshot")[0]
        self.assertIn("build it in one run with fullsend", reply)
        self.assertEqual(self.gh.label_names(3), {"squishy:build", "squishy:oneshot"})
        self.world.ask(3, "/squishy split bot")
        self.assertEqual(self.gh.label_names(3), {"squishy:build", "squishy:split-bot"})
        self.world.ask(3, "/squishy build")
        self.assertEqual(self.gh.label_names(3), {"squishy:build"})

    def test_a_mode_label_queues_it_in_that_mode(self):
        self.gh.add_issue(4, labels=("squishy:split",))
        events.on_issue_change(self.world.ctx, {"action": "labeled", "issue": self.gh.get_issue(4),
                                                "label": {"name": "squishy:split"},
                                                "sender": OPERATOR}, is_pr=False)
        self.assertEqual(self.gh.label_names(4), {"squishy:build", "squishy:split"})
        found = queue_mod.candidates(self.world.ctx, self.world.ctx.store.load())
        self.assertEqual([(c.number, c.mode, c.planned) for c in found], [(4, "split", True)])

    def test_the_night_bots_issues_are_left_alone(self):
        self.gh.add_issue(5, labels=("bot:build",))
        reply = self.world.ask(5, "/squishy oneshot")[0]
        self.assertIn("is the night bot's (@jgoetzmann-bot)", reply)
        self.assertEqual(self.gh.label_names(5), {"bot:build"})
        self.gh.add_issue(6)
        self.gh.add_assignees(6, ["jgoetzmann-bot"])
        self.assertIn("the night bot's", self.world.ask(6, "/squishy build")[0])

    def test_no_suggestions(self):
        self.gh.add_issue(7)
        self.assertEqual(self.world.ask(7, "/squishy suggest"), ["Squishy makes no suggestions."])

    def test_a_mode_never_runs_over_an_open_pull_request(self):
        self.gh.add_issue(8, labels=(LABEL_PR_OPEN,))
        self.gh.add_pull(9, "squishy/issue-8")
        self.assertIn("`/squishy rebuild`", self.world.ask(8, "/squishy oneshot")[0])


class OneShotCases(unittest.TestCase):
    def test_the_builder_is_told_to_run_fullsend_and_its_change_ships_like_any_build(self):
        world = World()
        world.gh.add_issue(12, title="Rules v2", body="Make the rules v2.",
                           labels=("squishy:build", "squishy:oneshot"))
        prompts: list[str] = []

        def build(request: RunRequest) -> RunResult:
            prompts.append(request.prompt)
            (request.cwd / "src" / "game.txt").write_text("rules v2\n")
            return RunResult(True, DONE)

        planned, result = world.run(FakeRunner({"build": build, "review": lambda r: RunResult(
            True, APPROVE)}))
        self.assertEqual((planned["action"], planned["mode"]), ("build", "oneshot"))
        self.assertEqual(planned["branch"], "squishy/issue-12")
        self.assertIsNone(planned["seats"]["plan"])  # it writes its own spec: no planner first
        self.assertEqual(planned["provider"], "claude-squishy")
        self.assertIn("one-shot build", world.gh.bot_comments(12)[0])
        prompt = prompts[0]
        self.assertIn(".claude/skills/fullsend/SKILL.md", prompt)
        self.assertIn("git worktree add -b squishy/oneshot-12/<slice> .fullsend/worktrees/<slice>",
                      prompt)
        self.assertIn('git commit -m\n   "fullsend: phase N for #12"', prompt)
        self.assertEqual(result["status"], "approved")
        exclude = Path(git(world.clone, "rev-parse", "--path-format=absolute",
                           "--git-common-dir")) / "info" / "exclude"
        self.assertIn("/.fullsend/", exclude.read_text())
        pulls = world.gh.list_pulls(state="open", head="squishy/issue-12")
        self.assertEqual(len(pulls), 1)
        self.assertIn("squishy:pr", world.gh.label_names(pulls[0]["number"]))


class SplitCases(unittest.TestCase):
    def split(self, mode: str, tree: dict = TREE) -> tuple[World, dict]:
        world = World()
        world.gh.add_issue(40, title="A big project", body="Do it all.\n\nDone when: it works.",
                           labels=("squishy:build", f"squishy:{mode}", "priority:high"))
        _, result = world.run(FakeRunner({"plan": lambda r: RunResult(True, answer(tree))}))
        return world, result

    def test_split_opens_linked_sub_issues_squishy_builds(self):
        world, result = self.split("split")
        gh = world.gh
        self.assertEqual(result["status"], "split")
        engine, client = 41, 42  # opened in build order
        self.assertEqual(gh.get_issue(engine)["title"], "Part 1: the engine")
        # Its type labels as far as the repository has them, never a reserved one, then the
        # parent's priority, its difficulty and Squishy's queue label.
        self.assertEqual(gh.label_names(engine), {"patch", "difficulty:hard", "priority:high",
                                                  "squishy:build"})
        self.assertEqual(gh.label_names(client), {"patch", "difficulty:easy", "priority:high",
                                                  "squishy:build"})
        body = gh.get_issue(client)["body"]
        self.assertIn(f"Blocked by #{engine}.", body)
        self.assertIn("Part of #40, split by Squishy", body)
        self.assertIn(PLAN_START, body)
        self.assertEqual(gh.blocked_by(client)[0]["number"], engine)
        self.assertEqual([c["number"] for c in gh.list_sub_issues(40)], [engine, client])
        self.assertEqual(gh.label_names(40), {"priority:high", LABEL_TREE})
        self.assertIn(f"- [ ] #{client} (after #{engine})", gh.bot_comments(40)[-1])
        record = world.ctx.store.load()["items"]["40"]
        self.assertEqual(record["tree"]["children"], [engine, client])
        # Both are queued for Squishy, the engine first; each counts as planned.
        found = queue_mod.candidates(world.ctx, world.ctx.store.load())
        self.assertEqual([(c.number, c.planned) for c in found], [(engine, True)])

    def test_split_bot_queues_them_for_the_night_bot(self):
        world, _ = self.split("split-bot")
        self.assertEqual(world.gh.label_names(41), {"patch", "difficulty:hard", "priority:high",
                                                    "bot:build"})
        self.assertIn("queued for the night bot (@jgoetzmann-bot)", world.gh.bot_comments(40)[-1])
        self.assertEqual(queue_mod.candidates(world.ctx, world.ctx.store.load()), [])

    def test_an_answer_it_cannot_use_opens_nothing(self):
        world, result = self.split("split", {"done": False, "issues": [{"key": "a"}]})
        self.assertEqual(result["status"], "failed")
        self.assertEqual(sorted(world.gh.threads), [40])
        self.assertIn("could not be used", world.gh.bot_comments(40)[-1])

    def test_the_close_out_closes_a_tree_that_is_done(self):
        world, _ = self.split("split")
        for child in (41, 42):
            world.gh.update_issue(child, state="closed")
        world.ctx.store.update(lambda s: s.update(last_sweep={"since": "2026-09-29T00:00:00Z"}))
        notes = sweep._trees(world.ctx, NIGHT)
        self.assertEqual(notes, ["#40: its sub-issues have all closed; queued its close-out"])
        self.assertEqual(world.gh.label_names(40), {"priority:high", LABEL_TREE, "squishy:build",
                                                    "squishy:split"})
        prompts: list[str] = []

        def close_out(request: RunRequest) -> RunResult:
            prompts.append(request.prompt)
            return RunResult(True, '<!-- split: {"done": true, "summary": "It works on main."} -->')

        _, result = world.run(FakeRunner({"plan": close_out}))
        self.assertEqual(result["status"], "split")
        self.assertIn("#41 [closed] Part 1: the engine", prompts[0])
        self.assertEqual(world.gh.get_issue(40)["state"], "closed")
        self.assertNotIn(LABEL_TREE, world.gh.label_names(40))

    def test_stop_on_a_tree_stops_its_sub_issues(self):
        world, _ = self.split("split")
        world.gh.add_issue(43, labels=("bot:build",))
        world.gh.add_sub_issue(40, world.gh.get_issue(43)["id"])
        replies = world.ask(40, "/squishy stop")
        self.assertIn("Its open sub-issues leave the queue too: #41, #42, #43.", replies[0])
        self.assertNotIn("squishy:build", world.gh.label_names(41))
        self.assertIn("@jgoetzmann-bot stop", world.gh.bot_comments(43)[-1])


class SectionCases(unittest.TestCase):
    def test_its_section_of_the_status_issue(self):
        world = World()
        world.gh.add_issue(50, title="Huge", labels=("squishy:build", "squishy:oneshot"))
        world.gh.add_issue(51, title="Parent", labels=(LABEL_TREE,))
        world.gh.add_issue(52, state="closed")
        world.gh.add_issue(53)
        for child in (52, 53):
            world.gh.add_sub_issue(51, world.gh.get_issue(child)["id"])
        text = dashboard.section(world.ctx)
        self.assertTrue(text.startswith(dashboard.SECTION_START))
        self.assertIn("## 🫧 Squishy", text)
        self.assertIn("| #50 | one-shot (fullsend) |", text)
        self.assertIn("| #51 | Parent | 1 of 2 closed", text)
        self.assertIn("`claude-squishy`", text)
        self.assertIn("### Last Squishy runs", text)


if __name__ == "__main__":
    unittest.main()
