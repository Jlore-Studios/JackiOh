"""Squishy (#60): the night bot's harness run as a second bot, and how the two live side by side.

These run in the night bot's process. Squishy's own cases (its labels, commands, modes, splits and
section) need a process that is Squishy from the start, since the labels and branches are fixed
when the process starts: `SquishyProcessTests` runs `tests/squishy_cases.py` with
`HARNESS_HOME=.squishy`.
"""

from __future__ import annotations

import argparse
import contextlib
import io
import json
import os
import re
import subprocess
import sys
import unittest
from pathlib import Path
from unittest import mock

import harness.__main__ as main_mod
from harness import config, dashboard, events, identity, providers, split
from harness import queue as queue_mod
from harness.config import LABEL_BUILD

from tests.fakes import OPERATOR, FakeGitHub
from tests.support import make_config, make_ctx
from tests.test_dashboard import offline
from tests.test_workflows import job, read

BOT_DIR = Path(__file__).resolve().parents[1]
SQUISHY = {"login": "squishy-squooby", "id": 334289562}


class SquishyProcessTests(unittest.TestCase):
    def test_squishys_own_cases_pass_as_squishy(self):
        """`tests/squishy_cases.py` in a process started as Squishy."""
        env = {**os.environ, "HARNESS_HOME": ".squishy", "PYTHONDONTWRITEBYTECODE": "1"}
        proc = subprocess.run([sys.executable, "-m", "unittest", "tests.squishy_cases"],
                              cwd=BOT_DIR, env=env, capture_output=True, text=True, timeout=600)
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertRegex(proc.stderr, r"Ran [1-9]\d* tests")
        self.assertNotIn("skipped", proc.stderr.splitlines()[-1])


class IdentityTests(unittest.TestCase):
    def test_this_process_is_the_night_bot(self):
        self.assertEqual(config.HOME, ".harness")
        self.assertEqual((config.LABEL_BUILD, config.SLASH, config.STATE_BRANCH, config.MARKER),
                         ("bot:build", "/harness", "bot-state", "<!-- jackioh-bot -->"))
        self.assertEqual(config.MODES, ())
        self.assertEqual([o.login for o in config.OTHERS], ["squishy-squooby"])
        self.assertEqual(queue_mod.branch_for_issue(7), "bot/issue-7")

    def test_squishys_config_says_who_it_is(self):
        found = identity.load(config.REPO_ROOT, ".squishy")
        self.assertEqual((found.name, found.command, found.label_prefix, found.branch_prefix),
                         ("Squishy", "squishy", "squishy:", "squishy/"))
        self.assertEqual((found.state_branch, found.journal_branch, found.workflow),
                         ("squishy-state", "squishy-journal", "squishy-run.yml"))
        self.assertEqual(found.modes, ("oneshot", "split", "split-bot"))
        self.assertFalse(found.suggestions)
        self.assertEqual([o.login for o in found.others], ["jgoetzmann-bot"])
        self.assertNotEqual(found.marker, config.MARKER)

    def test_squishys_subscription_is_its_own_on_githubs_runners(self):
        raw = json.loads((config.REPO_ROOT / ".squishy" / "providers.json").read_text())
        pool = providers.parse(raw)
        self.assertEqual(list(pool.providers), ["claude-squishy"])
        provider = pool.get("claude-squishy")
        self.assertEqual((provider.cli, provider.model, provider.tier, provider.runs_on),
                         ("claude", "opus", "strong", "ubuntu-latest"))
        self.assertEqual(provider.secret, "CLAUDE_CODE_OAUTH_TOKEN_SQUISHY")
        self.assertNotIn("suggest", provider.roles)
        self.assertEqual((pool.machine_parallel, pool.plan_lanes), (0, 0))
        night = {p.secret for p in make_config().pool.providers.values()}
        self.assertNotIn(provider.secret, night)

    def test_both_configs_keep_each_others_switches_out_of_reach(self):
        for home in (".harness", ".squishy"):
            raw = json.loads((config.REPO_ROOT / home / "config.json").read_text())
            for path in (".github/", ".harness/", ".squishy/", "bot/", ".claude/"):
                self.assertIn(path, raw["forbidden_paths"], home)

    def test_a_bad_identity_is_refused(self):
        with self.assertRaises(ValueError):
            identity.parse({"label_prefix": "squishy"})
        with self.assertRaises(ValueError):
            identity.parse({"modes": ["teleport"]})
        with self.assertRaises(ValueError):
            identity.parse({"colour": "blue"})
        self.assertEqual(identity.parse(None), identity.Identity())


TREE = """I cut it in two: the engine first, then the client.

```json
{"done": false, "summary": "Engine, then client.",
 "issues": [
  {"key": "client", "title": "Part 2: the client", "body": "Draw it.", "plan": "1. draw",
   "difficulty": "easy", "labels": ["patch"], "blocked_by": ["engine", "#77"]},
  {"key": "engine", "title": "Part 1: the engine", "body": "Rule it.", "plan": "1. rule",
   "difficulty": "hard", "labels": ["patch", "bot:build"], "blocked_by": []}
 ]}
```"""


class SplitTests(unittest.TestCase):
    def test_a_tree_is_read_and_ordered_by_what_blocks_what(self):
        tree = split.parse(TREE)
        self.assertTrue(tree.ok, tree.problems)
        self.assertEqual([i.key for i in split.ordered(tree.issues)], ["engine", "client"])
        client = tree.issues[0]
        self.assertEqual((client.blocked_by, client.blocked_by_issues), (("engine",), (77,)))
        self.assertEqual(split.from_dict(tree.to_dict()).to_dict(), tree.to_dict())

    def test_a_header_works_as_well_as_a_fenced_block(self):
        tree = split.parse('<!-- split: {"done": true, "summary": "It holds."} -->')
        self.assertTrue(tree.ok)
        self.assertTrue(tree.done)

    def test_a_tree_that_cannot_be_used_says_why(self):
        def problems(raw):
            return split.from_dict(raw).problems
        self.assertTrue(split.parse("no tree here").problems)
        self.assertIn("go round in a circle", " ".join(problems({"issues": [
            {"key": "a", "title": "A", "body": "a", "blocked_by": ["b"]},
            {"key": "b", "title": "B", "body": "b", "blocked_by": ["a"]}]})))
        self.assertIn("unknown key", " ".join(problems({"issues": [
            {"key": "a", "title": "A", "body": "a", "blocked_by": ["z"]}]})))
        self.assertIn("needs a title and a body", " ".join(problems({"issues": [{"key": "a"}]})))
        self.assertIn("`done` is true", " ".join(problems({"done": True, "issues": [
            {"key": "a", "title": "A", "body": "a"}]})))
        self.assertIn("at most", " ".join(problems({"issues": [
            {"key": f"k{i}", "title": "T", "body": "b"} for i in range(split.MAX_ISSUES + 1)]})))
        self.assertIn("no sub-issues", " ".join(problems({"done": False, "issues": []})))

    def test_the_checklist_names_each_one_and_what_it_waits_for(self):
        tree = split.parse(TREE)
        engine, client = split.ordered(tree.issues)
        text = split.checklist(40, [(engine, 41), (client, 42)], builder="me",
                               summary="Engine, then client.", link="the run")
        self.assertIn("- [ ] #41\n- [ ] #42 (after #41, #77)", text)
        self.assertIn("Blocked by #41, #77.", split.body(client, 40, "Squishy", [41, 77]))


class NightBotLeavesSquishyAloneTests(unittest.TestCase):
    """The night bot never takes an issue that is Squishy's: its labels on it, or assigned to
    it. Squishy's process has the same rule the other way (`squishy_cases.py`)."""

    def setUp(self):
        self.gh = FakeGitHub()
        self.ctx = make_ctx(self.gh)

    def test_a_squishy_label_keeps_it_squishys(self):
        self.gh.add_issue(5, labels=("squishy:oneshot", "squishy:build"))
        reply = queue_mod.queue_build(self.ctx, 5, by="jgoetzmann")
        self.assertIn("is Squishy's (@squishy-squooby)", reply)
        self.assertNotIn(LABEL_BUILD, self.gh.label_names(5))
        self.gh.add_issue(6, labels=("squishy:tree", LABEL_BUILD))
        skipped: list[str] = []
        self.assertEqual(queue_mod.candidates(self.ctx, self.ctx.store.load(), skipped), [])
        self.assertIn("#6 skipped: it is Squishy's", skipped)

    def test_an_old_planned_label_does_not(self):
        self.gh.add_issue(7, labels=("squishy:planned",))
        self.assertIn("Queued #7", queue_mod.queue_build(self.ctx, 7, by="jgoetzmann"))

    def test_assigned_to_squishy_keeps_it_squishys(self):
        self.gh.add_issue(8)
        self.gh.add_assignees(8, ["squishy-squooby"])
        self.assertIn("is Squishy's", queue_mod.queue_build(self.ctx, 8, by="jgoetzmann"))

    def test_squishys_own_labels_are_never_offered_by_triage(self):
        from harness import triage
        self.assertTrue(triage._bot_only("squishy:split"))
        self.assertTrue(triage._bot_only("bot:build"))
        self.assertFalse(triage._bot_only("patch"))


class BotsDoNotTalkForEverTests(unittest.TestCase):
    """The bots name each other in their replies; only a bare `stop` from the other bot is a
    command, and nothing answers it with a hint."""

    def setUp(self):
        self.gh = FakeGitHub()
        self.ctx = make_ctx(self.gh)

    def comment(self, number: int, body: str, user: dict) -> list[str]:
        self.gh.add_comment(number, body, user)
        comment = self.gh.comments[number][-1]
        return events.on_comment(self.ctx, {"action": "created", "comment": comment,
                                            "issue": self.gh.get_issue(number)})

    def test_squishys_reply_naming_the_night_bot_is_no_request(self):
        self.gh.add_issue(9, labels=(LABEL_BUILD,))
        out = self.comment(9, "@jgoetzmann-bot\n\n#9 is the night bot's (@jgoetzmann-bot): it "
                              "carries the night bot's labels.", SQUISHY)
        self.assertEqual(out, ["no command"])
        self.assertEqual(self.gh.bot_comments(9), [])

    def test_a_bare_stop_from_squishy_stops_its_sub_issue(self):
        self.gh.add_issue(10, labels=(LABEL_BUILD,))
        self.comment(10, "@jgoetzmann-bot stop\n\njgoetzmann stopped the tree this belongs to "
                         "(#4).", SQUISHY)
        self.assertNotIn(LABEL_BUILD, self.gh.label_names(10))
        self.assertTrue(self.ctx.store.load()["items"]["10"]["stop_requested"])

    def test_a_person_still_gets_every_command(self):
        self.gh.add_issue(11)
        self.comment(11, "@jgoetzmann-bot make it blue", OPERATOR)
        self.assertIn(LABEL_BUILD, self.gh.label_names(11))

    def test_oneshot_and_split_are_squishys(self):
        self.gh.add_issue(12)
        self.comment(12, "/harness oneshot", OPERATOR)
        self.assertIn("I have no `oneshot` mode. Squishy (@squishy-squooby) may",
                      self.gh.bot_comments(12)[-1])
        self.assertNotIn(LABEL_BUILD, self.gh.label_names(12))


class StatusSectionTests(unittest.TestCase):
    """Squishy shows in the pinned status issue in a section of its own, drawn by its own
    process and set apart from the night bot's facts."""

    def test_the_section_goes_after_the_night_bots_facts(self):
        ctx = make_ctx(FakeGitHub())
        body = dashboard.render(ctx, ("<!-- s -->\n## 🫧 Squishy\n\nIts facts.",))
        self.assertLess(body.index("## Last night-bot runs"), body.index("## 🫧 Squishy"))
        self.assertLess(body.index("## 🫧 Squishy"), body.index("The full status"))
        self.assertIn("---\n\n<!-- s -->", body)

    def test_the_loop_draws_squishy_in_a_process_of_its_own(self):
        seen: list[tuple[str, ...]] = []
        calls: list[tuple[list[str], dict]] = []

        def run(argv, **kwargs):
            calls.append((argv, kwargs["env"]))
            out = "## 🫧 Squishy\n" if "--section-only" in argv else "swept\n"
            return subprocess.CompletedProcess(argv, 0, out, "")

        def update(ctx, extra=()):
            seen.append(extra)
            return "rewrote #148"

        out = io.StringIO()
        env = {"SQUISHY_GITHUB_TOKEN": "s" * 20, "BOT_GITHUB_TOKEN": "n" * 20,
               "HARNESS_HOME": ".harness"}
        with mock.patch.object(main_mod.subprocess, "run", run), offline(), \
                mock.patch.object(main_mod.sweep_mod, "sweep", lambda ctx: []), \
                mock.patch.object(dashboard, "update", update), \
                mock.patch.dict(os.environ, env), contextlib.redirect_stdout(out):
            main_mod.cmd_dashboard(make_config(), argparse.Namespace(sweep=True, companions=True))
        self.assertEqual(seen, [("## 🫧 Squishy\n",)])
        self.assertEqual([argv[3:] for argv, _ in calls], [["sweep"],
                                                           ["dashboard", "--section-only"]])
        for _, child in calls:
            self.assertEqual(child["HARNESS_HOME"], ".squishy")
            self.assertEqual(child["BOT_GITHUB_TOKEN"], "s" * 20)  # Squishy's, not the night bot's
        self.assertIn("Squishy: swept", out.getvalue())

    def test_without_its_token_squishy_is_drawn_but_never_swept(self):
        other = config.OTHERS[0]
        env = config.companion_env(other, {"BOT_GITHUB_TOKEN": "n" * 20, "PATH": "/bin"})
        self.assertNotIn("BOT_GITHUB_TOKEN", env)
        self.assertEqual(env["HARNESS_HOME"], ".squishy")

    def test_no_companions_unless_asked(self):
        def run(*args, **kwargs):
            raise AssertionError("no process should start")
        with mock.patch.object(main_mod.subprocess, "run", run), offline(), \
                mock.patch.object(dashboard, "update", lambda ctx, extra=(): "rewrote #148"), \
                contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(main_mod.cmd_dashboard(make_config(), argparse.Namespace()), 0)


class SquishyWorkflowTests(unittest.TestCase):
    run_text = read("squishy-run.yml")
    commands_text = read("squishy-commands.yml")

    def test_every_job_runs_as_squishy(self):
        for text in (self.run_text, self.commands_text):
            self.assertIn("  HARNESS_HOME: .squishy\n", text)
            self.assertNotIn("secrets.BOT_GITHUB_TOKEN", text)
            self.assertIn("BOT_GITHUB_TOKEN: ${{ secrets.SQUISHY_GITHUB_TOKEN }}", text)

    def test_claude_runs_on_githubs_runners_with_only_its_own_secret(self):
        work = job(self.run_text, "work")
        self.assertIn("runs-on: ubuntu-latest", work)
        self.assertNotIn("night-vm", self.run_text)
        self.assertNotIn("BOT_GITHUB_TOKEN", work)
        self.assertEqual(set(re.findall(r"\$\{\{\s*(secrets[^}]*?)\s*\}\}", work)),
                         {"secrets[needs.plan.outputs.secret]"})
        grants = dict(re.findall(r"^\s+([\w-]+):\s*(\w+)",
                                 re.search(r"permissions:\n((?:\s{6}.*\n)+)", work).group(1), re.M))
        self.assertEqual(grants, {"contents": "read"})
        # Never the step name bright-bots-harness reads as the shared subscription's spending.
        self.assertNotIn("name: Build, check and review\n", self.run_text)
        self.assertIn("group: squishy-run-plan", job(self.run_text, "plan"))
        self.assertIn(".squishy/HALT", job(self.run_text, "gate"))
        self.assertIn(".squishy/HALT", job(self.run_text, "plan"))

    def test_every_secret_list_names_every_secret(self):
        for name in ("squishy-run.yml", "squishy-commands.yml", "bot-status.yml"):
            for found in re.findall(r"HARNESS_SECRETS_SET: >-\n((?:\s+\$\{\{.*\}\}\n)+)",
                                    read(name)):
                named = re.findall(r"secrets\.(\w+) != '' && '(\w+)'", found)
                self.assertEqual([a for a, _ in named], list(providers.SECRETS), name)

    def test_its_ears_hear_only_squishy(self):
        text = self.commands_text
        for wanted in ("github.actor != 'squishy-squooby'", "'/squishy'", "'@squishy-squooby'",
                       "startsWith(github.event.label.name, 'squishy:')",
                       "github.event.assignee.login == 'squishy-squooby'",
                       "startsWith(github.event.workflow_run.head_branch, 'squishy/')",
                       "contains(github.event.pull_request.labels.*.name, 'squishy:pr')"):
            self.assertIn(wanted, text)
        self.assertNotIn("'bot:", text)
        self.assertNotIn("secrets[", text)
        self.assertNotIn("claude-code", text)

    def test_the_status_loop_runs_squishy_beside_the_night_bot(self):
        refresh = job(read("bot-status.yml"), "refresh")
        self.assertIn("SQUISHY_GITHUB_TOKEN: ${{ secrets.SQUISHY_GITHUB_TOKEN }}", refresh)
        self.assertIn("--companions", refresh)
        self.assertIn("python -m harness dashboard --companions", read("bot-commands.yml"))

    def test_selftest_lints_and_loads_squishy(self):
        text = read("bot-selftest.yml")
        self.assertIn("HARNESS_HOME=.squishy python3.13 -m harness window", text)
        for name in ("squishy-run.yml", "squishy-commands.yml"):
            self.assertIn(f".github/workflows/{name}", text)

    def test_fullsend_is_in_the_repository(self):
        skill = config.REPO_ROOT / ".claude" / "skills" / "fullsend"
        for name in ("SKILL.md", "LICENSE", "agents/builder.md", "agents/spec-tester.md",
                     "agents/reconciler.md", "agents/culler.md"):
            self.assertTrue((skill / name).is_file(), name)
        self.assertIn("v2.2.0", (skill / "SOURCE.md").read_text())


if __name__ == "__main__":
    unittest.main()
