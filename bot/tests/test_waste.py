"""The wasted runs and quota the audit in #160 found, each as the scenario that showed it."""

from __future__ import annotations

import json
import tempfile
import unittest
from datetime import timedelta
from pathlib import Path

from harness import gates as gates_mod
from harness import plan as plan_mod
from harness import providers
from harness import queue as queue_mod
from harness.__main__ import make_probe
from harness.config import Gate, LABEL_BLOCKED, LABEL_BUILD, LABEL_HUMAN, LABEL_PR, LABEL_REVISE
from harness.deliver import Deliverer
from harness.runner import FakeRunner, RunResult
from harness.state import item as state_item
from harness.issueplan import PLAN_CHARS, PLAN_WORDS
from harness.work import PLAN_CUT, Worker

from tests.fakes import FakeGitHub, make_origin
from tests.support import DAY, MACHINE, make_config, make_ctx
from tests.test_cross import ALL
from tests.test_flow import Harness
from tests.test_work import APPROVE, GATES, builder, reviewer

AT = DAY


def timed_out(name: str) -> gates_mod.GateResult:
    return gates_mod.GateResult(name, "pnpm test", False, gates_mod.TIMED_OUT, 2400.0,
                                "(timed out after 2400s)")


class TimedOutCheckTests(unittest.TestCase):
    """`pnpm test` never finished inside its 40 minutes on GitHub's runners; the harness then ran
    it again on a copy of main, which timed out too, and called it "also red on main"."""

    def test_a_timed_out_gate_is_inconclusive_and_does_not_block(self):
        results = [timed_out("unit")]
        gates_mod.mark_inconclusive(results, {"unit"})
        self.assertTrue(results[0].inconclusive)
        self.assertTrue(gates_mod.green(results))
        self.assertEqual(gates_mod.failures_text(results), "")
        self.assertIn("timed out: inconclusive, left to CI", gates_mod.table(results))

    def test_a_timed_out_install_still_fails(self):
        results = [timed_out("install")]
        gates_mod.mark_inconclusive(results, {"unit"})
        self.assertFalse(results[0].inconclusive)
        self.assertFalse(gates_mod.green(results))

    def test_a_timed_out_gate_is_never_run_again_on_main(self):
        root = Path(tempfile.mkdtemp())
        _, clone = make_origin(root)
        cfg = make_config(gates=[{"name": "unit", "run": "false", "timeout_minutes": 1}],
                          install={"run": "true", "timeout_minutes": 1})
        worker = Worker(cfg, {"action": "build", "number": 12, "branch": "bot/issue-12"},
                        FakeRunner({}), clone, root / "work", root / "out")
        results = [timed_out("unit")]
        worker._mark_pre_existing(results)
        self.assertTrue(results[0].inconclusive)
        self.assertFalse(results[0].pre_existing)
        self.assertIsNone(worker._base_wt)  # no copy of main was installed or run

    def test_the_committed_test_gate_runs_only_related_tests_in_minutes(self):
        gate = next(g for g in make_config().gates if g.name == "web tests")
        self.assertIsInstance(gate, Gate)
        self.assertIn("vitest run --project web --changed origin/main", gate.run)
        # As CI's web unit tests: the audio, fx and asset tests run in the daily super run.
        for skipped in ("'**/src/audio/**'", "'**/src/fx/**'",
                        "'**/src/cards/art/convention.test.ts'"):
            self.assertIn(f"--exclude {skipped}", gate.run)
        self.assertLessEqual(gate.timeout_minutes, 20)


class PlanCutTests(unittest.TestCase):
    """Only the last 20,000 characters of a plan were kept, so #160's Plan began mid-table."""

    def test_a_long_plan_keeps_its_beginning(self):
        root = Path(tempfile.mkdtemp())
        _, clone = make_origin(root)
        cfg = make_config(gates=GATES, install={"run": "true", "timeout_minutes": 1})
        long_plan = "## Goal\nThe first step.\n" + "x" * (PLAN_CHARS * 2) + "\nThe last step."
        runner = FakeRunner({"plan": lambda request: RunResult(True, long_plan)})
        plan = {"action": "plan", "number": 12, "title": "Rules v2", "branch": "bot/issue-12",
                "thread": "Please make the rules v2."}
        result = Worker(cfg, plan, runner, clone, root / "work", root / "out").run()
        self.assertEqual(result["status"], "planned", result.get("reason"))
        text = result["plan"]["text"]
        self.assertTrue(text.startswith("## Goal\nThe first step."))
        self.assertTrue(text.endswith(PLAN_CUT))
        self.assertLessEqual(len(text), PLAN_CHARS)
        self.assertTrue(result["handoff"]["notes"].startswith("# Plan"))

    def run_plan(self, text):
        root = Path(tempfile.mkdtemp())
        _, clone = make_origin(root)
        cfg = make_config(gates=GATES, install={"run": "true", "timeout_minutes": 1})
        runner = FakeRunner({"plan": lambda request: RunResult(True, text)})
        plan = {"action": "plan", "number": 12, "title": "Rules v2", "branch": "bot/issue-12",
                "thread": "Please make the rules v2."}
        return runner, Worker(cfg, plan, runner, clone, root / "work", root / "out").run()

    def test_a_plan_over_the_old_cut_is_kept_whole(self):
        """#208: plans were cut at 20,000 characters (#307's among them)."""
        plan = "## Goal\nThe first step.\n" + "x" * 25_000 + "\nThe last step."
        self.assertTrue(20_000 < len(plan) < PLAN_CHARS)
        _, result = self.run_plan(plan)
        self.assertEqual(result["status"], "planned")
        self.assertEqual(result["plan"]["text"], plan)

    def test_the_plan_prompt_states_its_budget(self):
        runner, _ = self.run_plan("## Goal\nRules v2.")
        prompt = runner.calls[0].prompt
        self.assertIn(f"under {PLAN_WORDS:,} words", prompt)
        self.assertIn(f"first {PLAN_CHARS:,} characters", prompt)
        self.assertIn("Say each thing once", prompt)

    def test_a_plan_the_description_refused_goes_whole_into_its_comment(self):
        from harness.errors import GitHubError
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,), body="Make the rules v2.")

        def refuse(number, body):
            raise GitHubError("body is too long", 422)
        h.gh.set_issue_body = refuse
        plan = "## Goal\nThe first step.\n" + "x" * 25_000 + "\nThe last step."
        planned = plan_mod.make(h.ctx)
        out = h.root / "out-plan"
        runner = FakeRunner({"plan": lambda request: RunResult(True, plan)})
        Worker(h.cfg, {**planned, "action": "plan"}, runner, h.clone, h.root / "work", out).run()
        Deliverer(h.ctx, {**planned, "action": "plan"}, out, h.deliver_repo).run()
        said = h.gh.bot_comments(12)[-1]
        self.assertIn("could not take the plan", said)
        self.assertIn("The last step.", said)


class ForbiddenOnlyTests(unittest.TestCase):
    """Muse built #160 for 73 minutes; every path was in `bot/`, so delivery refused it all."""

    def test_a_build_that_only_changes_forbidden_paths_stops_and_asks_a_person(self):
        h = Harness(self)
        h.gh.add_issue(12, "Tune the bot", labels=(LABEL_BUILD,))
        runner = FakeRunner({"build": builder({"bot/harness/tuned.py": "x = 1\n"}),
                             "review": reviewer(APPROVE)})
        planned, result = h.night(runner)
        self.assertEqual(result["status"], "blocked", result.get("reason"))
        self.assertNotIn("review", [c.role for c in runner.calls])  # no rounds spent on it
        self.assertNotIn("bundle", result)
        self.assertIsNone(h.origin_sha("bot/issue-12"))
        self.assertIn(LABEL_BLOCKED, h.gh.label_names(12))
        said = h.gh.bot_comments(12)[-1]
        self.assertIn("`bot/harness/tuned.py`", said)
        self.assertIn("A person has to make this change", said)

    def test_a_build_with_work_outside_them_goes_on(self):
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        runner = FakeRunner({"build": builder({"bot/x.py": "1\n", "src/game.txt": "v2\n"}),
                             "fix": builder({}), "review": reviewer(APPROVE)})
        _, result = h.night(runner)
        self.assertIn("review", [c.role for c in runner.calls])
        self.assertNotEqual(result["status"], "blocked")


class ClosedDuringTheRunTests(unittest.TestCase):
    """Devin stayed on #125 for hours after #163 closed it."""

    def test_the_probe_stops_a_run_whose_thread_closed(self):
        gh = FakeGitHub()
        gh.add_issue(5)
        ctx = make_ctx(gh)
        probe = make_probe(ctx, 5)
        self.assertIsNone(probe(None))
        gh.threads[5]["state"] = "closed"
        self.assertEqual(probe(None), ("#5 was closed", "stop"))

    def test_github_failing_to_answer_does_not_stop_it(self):
        ctx = make_ctx(FakeGitHub())
        self.assertIsNone(make_probe(ctx, 404)(None))  # no such thread: GitHub says 404


class BrokenLoginTests(unittest.TestCase):
    """claude-3's token was refused for 6+ hours and was tried every 50 minutes regardless."""

    def test_each_failure_in_a_row_waits_longer(self):
        state: dict = {}
        pool = make_config(env=ALL, committed_hours=True).pool  # claude-3 works all day
        claude = pool.get("claude-3")
        secrets = providers.Secrets.of(ALL)
        waits = []
        for _ in range(4):
            providers.note_infra(state, "claude-3", "401 OAuth access token is invalid", AT)
            waits.append(providers.infra_backoff(state["providers"]["claude-3"]["infra"]))
        self.assertEqual(waits, [timedelta(minutes=50), timedelta(hours=2), timedelta(hours=8),
                                 timedelta(hours=8)])
        why = providers.availability(claude, state, AT + timedelta(hours=7), "America/Chicago",
                                     secrets, forced=False)
        self.assertIn("4 times in a row", why or "")
        self.assertIsNone(providers.availability(claude, state, AT + timedelta(hours=9),
                                                 "America/Chicago", secrets))

    def test_a_failure_that_is_not_the_providers_keeps_the_streak(self):
        state: dict = {}
        providers.note_infra(state, "gpt", "refused", AT)
        self.assertEqual(providers.note_infra(state, "gpt", "install red on main", AT,
                                              escalate=False), 1)

    def test_a_streak_asks_a_person_once_and_a_working_run_ends_it(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=AT)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned = plan_mod.make(h.ctx)
        provider = planned["provider"]

        def deliver(result: dict) -> None:
            out = h.root / f"out-{len(list(h.root.glob('out-*')))}"
            out.mkdir()
            (out / "result.json").write_text(json.dumps(result))
            Deliverer(h.ctx, planned, out, h.deliver_repo).run()

        refused = {"status": "infra", "reason": "401 OAuth access token is invalid",
                   "model_calls": 1}
        for _ in range(providers.INFRA_ASK_AFTER + 1):
            deliver(refused)
        asked = [n for n, t in h.gh.threads.items() if provider in t["title"]
                 and "times in a row" in t["title"]]
        self.assertEqual(len(asked), 1)  # once per streak, not once per run
        issue = asked[0]
        self.assertIn(LABEL_HUMAN, h.gh.label_names(issue))
        self.assertIn("once every 8 hours", h.gh.threads[issue]["body"])
        deliver({"status": "failed", "reason": "the harness failed", "model_calls": 2})
        self.assertNotIn("infra", h.ctx.store.load()["providers"][provider])
        self.assertEqual(h.gh.threads[issue]["state"], "closed")
        self.assertIn("works again", h.gh.bot_comments(issue)[-1])


class HonestReportTests(unittest.TestCase):
    """The pull request said "the reviewer still had blocking findings" after one unreadable
    review (#176 fixed the comment); one-commit PRs landed on main as "bot: build pass 1 for #85"."""

    def test_an_unapproved_pull_request_says_how_many_rounds_it_took_and_why(self):
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        h.night(FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                            "review": reviewer("LGTM")}))
        body = h.gh.list_pulls(head="bot/issue-12")[0]["body"]
        self.assertIn("It stopped without an approval after 1 of 2 round(s): the reviewer's "
                      "answer could not be read twice in a row, so this is a draft", body)
        self.assertNotIn("still had blocking findings", body)

    def test_the_squash_commit_takes_the_pull_requests_title(self):
        h = Harness(self, env=ALL, machine=MACHINE)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        h.night(FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                            "review": reviewer(APPROVE)}))
        pr = int(h.gh.list_pulls(head="bot/issue-12")[0]["number"])
        self.assertEqual(h.gh.merge_titles[f"PR_{pr}"], f"Make the rules v2 (#{pr})")


def picks(ctx, gh) -> tuple[list[int], list[str]]:
    skipped: list[str] = []
    state = ctx.store.load()
    return [c.number for c in queue_mod.candidates(ctx, state, skipped)], skipped


class DependencyTests(unittest.TestCase):
    """#131 said "Do not start until #125 has merged" and would have gone first; part 4 of a
    patch was queued while part 2 was open."""

    def setUp(self):
        self.gh = FakeGitHub()
        self.ctx = make_ctx(self.gh)

    def test_a_blocked_by_line_holds_the_build_until_its_blocker_closes(self):
        self.gh.add_issue(125, "Homescreen rotation")
        self.gh.add_issue(131, body="**Blocked by #125.** Do not start until #125 has merged.",
                          labels=(LABEL_BUILD,))
        numbers, skipped = picks(self.ctx, self.gh)
        self.assertEqual(numbers, [])
        self.assertEqual(skipped, ["#131 skipped: it waits for #125 to close first"])
        self.gh.threads[125]["state"] = "closed"
        self.assertEqual(picks(self.ctx, self.gh)[0], [131])

    def test_a_line_naming_several_waits_for_every_open_one(self):
        self.gh.add_issue(3)
        self.gh.add_issue(4, state="closed")
        self.gh.add_issue(9, body="Depends on #3, #4 and #12.", labels=(LABEL_BUILD,))
        self.assertIn("#9 skipped: it waits for #3 to close first", picks(self.ctx, self.gh)[1])

    def test_a_mention_in_the_plan_or_elsewhere_is_not_a_blocker(self):
        self.gh.add_issue(3)
        body = ("See #3 for context.\n\n<!-- jackioh-bot:plan -->\n## Plan\n\nThis depends on #3."
                "\n<!-- /jackioh-bot:plan -->\n")
        self.gh.add_issue(9, body=body, labels=(LABEL_BUILD,))
        self.assertEqual(picks(self.ctx, self.gh)[0], [9])

    def test_githubs_own_blocked_by_link_holds_it(self):
        self.gh.add_issue(3)
        self.gh.add_issue(9, labels=(LABEL_BUILD,))
        self.gh.block(9, 3)
        self.assertEqual(picks(self.ctx, self.gh)[1],
                         ["#9 skipped: it waits for #3 to close first"])

    def test_a_part_waits_for_the_earlier_parts_of_its_patch(self):
        self.gh.add_issue(124, "Patch v0.2.X (part 2 of 4): global animations")
        self.gh.add_issue(126, "Patch v0.2.X (part 4 of 4): more card patches",
                          labels=(LABEL_BUILD,))
        self.gh.add_issue(140, "Patch v0.2.X (part 2 of 3): another patch", labels=(LABEL_BUILD,))
        numbers, skipped = picks(self.ctx, self.gh)
        self.assertEqual(numbers, [140])  # another patch of the same name, with its own m
        self.assertEqual(skipped, ["#126 skipped: it waits for #124 to close first"])

    def test_a_forced_build_goes_ahead_anyway(self):
        self.gh.add_issue(125)
        self.gh.add_issue(131, body="Blocked by #125.", labels=(LABEL_BUILD,))
        self.ctx.store.update(lambda s: state_item(s, 131).update(forced=True))
        self.assertEqual(picks(self.ctx, self.gh)[0], [131])

    def test_revisions_are_never_held(self):
        self.gh.add_issue(125)
        self.gh.add_pull(20, "bot/issue-131", body="Closes #131. Blocked by #125.",
                         labels=(LABEL_PR, LABEL_REVISE))
        self.assertEqual(picks(self.ctx, self.gh)[0], [20])

    def test_nothing_named_asks_github_nothing_more(self):
        calls = []
        original = self.gh.list_issues
        self.gh.list_issues = lambda **kw: calls.append(kw) or original(**kw)
        self.gh.add_issue(9, labels=(LABEL_BUILD,))
        picks(self.ctx, self.gh)
        self.assertTrue(all(kw.get("labels") for kw in calls))  # only the queue's label reads


if __name__ == "__main__":
    unittest.main()
