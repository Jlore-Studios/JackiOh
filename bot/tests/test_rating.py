"""#317 parts 4, 8 and 10: the bot rates each item's difficulty when it plans it, under the easy
rule; three failures of an item's own raise it a step and rebuild its pull request from `main`;
`/harness rebuild` and `/harness review [strong|medium]`."""

from __future__ import annotations

import unittest

from harness import events, issueplan, stepup
from harness import plan as plan_mod
from harness.commands import parse
from harness.config import (LABEL_BLOCKED, LABEL_BUILD, LABEL_CROSS, LABEL_NEEDS_PLAN, LABEL_PR,
                            LABEL_PR_OPEN)
from harness.runner import FakeRunner, RunResult
from harness.state import item as state_item

from tests.fakes import OPERATOR, FakeGitHub
from tests.support import DAY, make_config, make_ctx
from tests.test_difficulty import seats
from tests.test_flow import Harness
from tests.test_work import APPROVE, builder, reviewer

EASY, MEDIUM, HARD = "difficulty:easy", "difficulty:medium", "difficulty:hard"


def rated_plan(difficulty, files=("src/game.txt",), why="one file, one test"):
    rows = "\n".join(f"| `{path}` | change | the change |" for path in files)
    return (f'<!-- bot: {{"difficulty": "{difficulty}", "why": "{why}"}} -->\n'
            "## 1. Goal\n\nRules v2.\n\n## 2. Files to touch\n\n| Path | new/change | What |\n"
            f"|---|---|---|\n{rows}\n\n## 3. Steps\n\n1. Edit it.\n")


def planner(text):
    return FakeRunner({"plan": lambda request: RunResult(True, text, duration_s=300)})


def day_harness(test, machine=("muse", "devin")):
    """By day: no Claude account (they keep to the night in tests), Muse and Devin on the
    machine, and the planning lane on."""
    return Harness(test, env={"HARNESS_SECRETS_SET": ""}, machine=machine, at=DAY,
                   plan_lanes=None)


class RatingTests(unittest.TestCase):
    def test_a_medium_planner_rates_an_unrated_item_easy_and_it_is_cleared(self):
        h = day_harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,), body="Make the rules v2.")
        runner = planner(rated_plan("easy"))
        planned, result = h.night(runner)
        self.assertEqual((planned["action"], planned["provider"], result["status"]),
                         ("plan", "muse", "planned"))
        self.assertEqual(planned["rating"], {"difficulty": "medium", "source": "", "by": ""})
        self.assertIn("Rate how hard it is", runner.calls[0].prompt)
        self.assertIn("At most 10 files changed", runner.calls[0].prompt)
        self.assertIn(EASY, h.gh.label_names(12))
        record = h.ctx.store.load()["items"]["12"]
        self.assertEqual((record["difficulty_by"]["difficulty"], record["difficulty_by"]["tier"]),
                         ("easy", "medium"))
        said = h.gh.bot_comments(12)[-1]
        self.assertIn("Rated `difficulty:easy` by `muse`", said)
        self.assertIn("Cleared and queued to build", said)
        self.assertNotIn("<!-- bot:", issueplan.plan_of(h.gh.threads[12]["body"]))
        # Devin builds it from Muse's plan, which meets an easy item's floor.
        planned = plan_mod.make(h.ctx)
        self.assertEqual((planned["action"], planned["provider"]), ("build", "devin"))

    def test_a_medium_rating_by_a_medium_planner_waits_for_a_strong_one(self):
        h = day_harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,), body="Make the rules v2.")
        h.night(planner(rated_plan("medium", why="a new ruling")))
        self.assertIn(MEDIUM, h.gh.label_names(12))
        self.assertIn(LABEL_NEEDS_PLAN, h.gh.label_names(12))
        self.assertIn("A strong model plans `difficulty:medium` work, so one plans it next",
                      h.gh.bot_comments(12)[-1])
        self.assertEqual(plan_mod.make(h.ctx)["action"], "none")  # no strong model by day

    def test_a_rating_is_never_lowered_and_a_persons_label_never_replaced(self):
        # By night claude-1, strong, plans the item Muse rated medium; its easy rating does not
        # lower it.
        h = Harness(self, machine=("muse",), plan_lanes=None)
        h.gh.add_issue(12, labels=(LABEL_BUILD, MEDIUM), body="Make the rules v2.")
        h.ctx.store.update(lambda s: state_item(s, 12).update(
            difficulty_by={"difficulty": "medium", "provider": "muse", "tier": "medium"},
            planned_at="2026-09-29T00:00:00Z", planned_tier="medium"))
        runner = planner(rated_plan("easy"))
        planned, _ = h.night(runner)
        self.assertEqual((planned["action"], planned["provider"]), ("plan", "claude-1"))
        self.assertIn("you may rate it higher, never lower", runner.calls[0].prompt)
        self.assertIn(MEDIUM, h.gh.label_names(12))
        self.assertIn("a rating is never lowered", h.gh.bot_comments(12)[-1])
        self.assertIn("Cleared and queued", h.gh.bot_comments(12)[-1])
        # A person's label: the planner is told not to rate, and a rating it gives anyway is
        # ignored.
        h = day_harness(self, machine=("muse",))
        h.gh.add_issue(12, labels=(LABEL_BUILD, EASY), body="Make the rules v2.")
        runner = planner(rated_plan("hard"))
        planned, _ = h.night(runner)
        self.assertEqual(planned["rating"]["source"], "person")
        self.assertIn("A person set this item's difficulty", runner.calls[0].prompt)
        self.assertEqual({n for n in h.gh.label_names(12) if n.startswith("difficulty:")}, {EASY})

    def test_an_easy_plan_that_breaks_the_rule_is_medium(self):
        h = day_harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,), body="Make the rules v2.")
        h.night(planner(rated_plan("easy", files=("src/game.txt", "SPEC.md"))))
        self.assertIn(MEDIUM, h.gh.label_names(12))
        said = h.gh.bot_comments(12)[-1]
        self.assertIn("The easy rule does not hold (lines 3–5: the plan touches `SPEC.md`)", said)
        h = day_harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,), body="Make the rules v2.")
        h.night(planner(rated_plan("easy", files=[f"src/f{n}.txt" for n in range(11)])))
        self.assertIn("line 1: the plan touches 11 files, over 10", h.gh.bot_comments(12)[-1])

    def test_a_build_run_that_rates_its_item_out_of_reach_stops_after_planning(self):
        """With the planning lane off, Muse plans the unrated item in its own build run; it rates
        it hard, which Muse may not build, so the run stops after planning."""
        h = Harness(self, env={"HARNESS_SECRETS_SET": ""}, machine=("muse",), at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD,), body="Make the rules v2.")
        planned, result = h.night(FakeRunner({"plan": lambda r: RunResult(
            True, rated_plan("hard", why="the resolution loop")), "build": builder({})}))
        self.assertEqual((planned["action"], result["status"]), ("build", "planned"))
        self.assertIn("rated difficulty:hard", result["reason"])
        self.assertIn(HARD, h.gh.label_names(12))
        self.assertIn(LABEL_BUILD, h.gh.label_names(12))
        self.assertIsNone(h.origin_sha("bot/issue-12"))

    def test_a_weak_builders_change_that_breaks_the_rule_is_not_pushed(self):
        h = Harness(self, env={"HARNESS_SECRETS_SET": ""}, machine=("devin",))
        h.gh.add_issue(12, labels=(LABEL_BUILD, EASY), body=issueplan.with_plan(
            "Make it.", "1. Edit src/game.txt.", "a person"))
        many = {f"src/f{n}.txt": "x\n" for n in range(11)}
        planned, result = h.night(FakeRunner({"build": builder(many),
                                              "self_check": reviewer(APPROVE)}))
        self.assertEqual((planned["provider"], result["status"]), ("devin", "built"))
        self.assertEqual(h.gh.list_pulls(), [])
        self.assertIn("breaks the easy rule (line 1: it changes 11 files, over 10)",
                      h.gh.bot_comments(12)[-1])
        # A person labelled it easy, so the label stays, under a medium floor.
        self.assertIn(EASY, h.gh.label_names(12))
        self.assertIn(LABEL_BUILD, h.gh.label_names(12))
        self.assertEqual(h.ctx.store.load()["items"]["12"]["difficulty_floor"], "medium")
        self.assertIsNotNone(h.origin_sha("bot/issue-12"))  # the work is kept on its branch
        self.assertEqual(plan_mod.make(h.ctx)["action"], "none")  # Devin may not take it now


class StepUpTests(unittest.TestCase):
    def ctx(self):
        gh = FakeGitHub()
        return gh, make_ctx(gh, cfg=make_config())

    def test_three_strikes_raise_a_bot_rated_item_a_step(self):
        gh, ctx = self.ctx()
        gh.add_issue(12, labels=(EASY,))
        ctx.store.update(lambda s: state_item(s, 12).update(
            difficulty_by={"difficulty": "easy", "provider": "muse"}))
        self.assertFalse(stepup.strike(ctx, 12, "the run failed: a"))
        self.assertFalse(stepup.strike(ctx, 12, "its reviewer did not approve it: b"))
        self.assertTrue(stepup.strike(ctx, 12, "the run failed: c"))
        self.assertIn(MEDIUM, gh.label_names(12))
        self.assertNotIn(EASY, gh.label_names(12))
        self.assertIn(LABEL_BUILD, gh.label_names(12))
        said = gh.bot_comments(12)[-1]
        self.assertIn("This failed 3 times at `difficulty:easy`", said)
        self.assertIn("the run failed: a; its reviewer did not approve it: b; the run failed: c",
                      said)
        record = ctx.store.load()["items"]["12"]
        self.assertEqual((record["strikes"], record["difficulty_by"]["difficulty"]), (0, "medium"))

    def test_a_persons_label_or_hard_asks_a_person(self):
        gh, ctx = self.ctx()
        gh.add_issue(12, labels=(EASY, LABEL_BUILD))
        for n in range(3):
            stepup.strike(ctx, 12, f"failure {n}")
        self.assertEqual(gh.label_names(12), {EASY, LABEL_BLOCKED})
        self.assertIn("A person set `difficulty:easy`, so I do not change it: should it be "
                      "`difficulty:medium`?", gh.bot_comments(12)[-1])
        gh.add_issue(13, labels=(HARD,))
        ctx.store.update(lambda s: state_item(s, 13).update(difficulty_by={"difficulty": "hard"}))
        for n in range(3):
            stepup.strike(ctx, 13, f"failure {n}")
        self.assertIn(LABEL_BLOCKED, gh.label_names(13))
        self.assertIn("no stronger model", gh.bot_comments(13)[-1])

    def test_strikes_on_a_pull_request_count_on_its_issue_and_rebuild_it(self):
        gh, ctx = self.ctx()
        gh.add_issue(12, labels=(LABEL_PR_OPEN,))
        gh.add_pull(40, "bot/issue-12", body="Closes #12", labels=(LABEL_PR, LABEL_CROSS),
                    sha="h1")
        ctx.store.update(lambda s: state_item(s, 40).update(issue=12, wip={"sha": "w"}))
        for n in range(3):
            stepup.strike(ctx, 40, f"rejected {n}")
        self.assertEqual(gh.threads[40]["state"], "closed")
        self.assertIn(HARD, gh.label_names(12))  # unrated, so medium, and one step up is hard
        self.assertEqual(gh.label_names(12) & {LABEL_BUILD, LABEL_PR_OPEN}, {LABEL_BUILD})
        self.assertIn(("bot/old/issue-12-20260930-0300", "h1"), gh.created_branches)
        self.assertIn("bot/issue-12", gh.deleted_branches)
        self.assertIn("bot/wip/40", gh.deleted_branches)
        record = ctx.store.load()["items"]["12"]
        self.assertEqual((record["previous_pr"], record["previous_branch"]),
                         (40, "bot/old/issue-12-20260930-0300"))
        self.assertTrue(ctx.store.load()["items"]["40"]["rebuilt"])


class CommandTests(unittest.TestCase):
    def ctx(self):
        gh = FakeGitHub()
        return gh, make_ctx(gh, cfg=make_config())

    def run_command(self, ctx, number, line):
        command = parse(line, "jgoetzmann-bot")[0]
        thread = ctx.gh.get_issue(number)
        return events.execute(ctx, command, thread, dict(OPERATOR))

    def test_rebuild_closes_the_pull_request_and_queues_the_issue(self):
        gh, ctx = self.ctx()
        gh.add_issue(12, labels=(LABEL_PR_OPEN,))
        gh.add_pull(40, "bot/issue-12", body="Closes #12", labels=(LABEL_PR,), sha="h1")
        reply = self.run_command(ctx, 40, "/harness rebuild")
        self.assertEqual(reply, "Closed #40 and queued #12 to build again from `main`.")
        self.assertEqual(gh.threads[40]["state"], "closed")
        self.assertIn(LABEL_BUILD, gh.label_names(12))
        self.assertIn("@jgoetzmann asked for it", ctx.store.load()["items"]["12"]["previous_why"])
        # The same from the issue; and an issue with no pull request has nothing to rebuild.
        gh.add_issue(13)
        self.assertIn("no open pull request of mine to rebuild",
                      self.run_command(ctx, 13, "@jgoetzmann-bot rebuild"))
        # The new build is told about the old pull request.
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["number"], planned["previous_pr"]), (12, 40))

    def test_review_queues_a_review_run_at_a_tier_and_no_revision(self):
        gh, ctx = self.ctx()
        gh.add_pull(40, "bot/issue-12", body="Closes #12", labels=(LABEL_PR,), sha="h1")
        reply = self.run_command(ctx, 40, "/harness review strong check the replay")
        self.assertIn("Queued a review run of #40, by a strong model or stronger", reply)
        self.assertEqual(gh.label_names(40), {LABEL_PR, LABEL_CROSS})
        record = ctx.store.load()["items"]["40"]
        self.assertEqual((record["review_floor"], record["review_notes"]),
                         ("strong", "check the replay"))
        # By night a strong model is free: it takes the review, with the notes.
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], seats(planned)["review"][2]), ("review", "strong"))
        self.assertEqual(planned["review_notes"], "check the replay")
        # By day only medium models are free (in tests): a strong-floor review waits.
        gh2 = FakeGitHub()
        ctx2 = make_ctx(gh2, at=DAY, cfg=make_config(machine=("muse",)))
        gh2.add_pull(40, "bot/issue-12", body="Closes #12", labels=(LABEL_PR,), sha="h1")
        self.run_command(ctx2, 40, "/harness review strong")
        self.assertEqual(plan_mod.make(ctx2)["action"], "none")
        self.run_command(ctx2, 40, "/harness review medium")
        self.assertEqual(plan_mod.make(ctx2)["provider"], "muse")
        # On an issue it says where it works.
        gh.add_issue(13)
        self.assertIn("one of my pull requests", self.run_command(ctx, 13, "/harness review"))


if __name__ == "__main__":
    unittest.main()
