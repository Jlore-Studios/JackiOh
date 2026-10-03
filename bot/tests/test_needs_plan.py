"""The Needs plan stage: every queued item gets a strong model's plan first, on a planning lane of
its own that takes no build lane, and the plan goes into the issue's description, where the
builder (Devin above all) starts from it."""

from __future__ import annotations

import unittest

from harness import issueplan
from harness import plan as plan_mod
from harness import providers
from harness.clock import iso
from harness.config import LABEL_BUILD, LABEL_NEEDS_PLAN, LABEL_WORKING, MARKER
from harness.runner import FakeRunner
from harness.state import item as state_item

from tests.fakes import FakeGitHub
from tests.support import ALL_MACHINE, DAY, MACHINE, NIGHT, make_config, make_ctx, with_lanes
from tests.test_difficulty import EASY, busy, queue, seats
from tests.test_flow import Harness
from tests.test_providers import secrets
from tests.test_work import APPROVE, builder, reviewer


def lane_ctx(gh, *, at=NIGHT, machine=ALL_MACHINE, max_parallel=20, env=None):
    """Every subscription, the committed planning lane, and room on the build lanes."""
    cfg = make_config(env=env or secrets(*providers.SECRETS), machine=machine, plan_lanes=None)
    return make_ctx(gh, at=at, cfg=with_lanes(cfg, max_parallel, max_parallel))


def running(gh, ctx, planned):
    """The claimed run goes on: its lane stays held."""
    gh.runs[str(planned["number"])] = {"status": "in_progress"}
    ctx.store.update(lambda s, n=planned["number"]: state_item(s, n).update(run_id=str(n)))


def planned_by(ctx, number, tier="strong"):
    """A planning run delivered: the item has its plan and its lane is free again."""
    ctx.store.update(lambda s: state_item(s, number).update(
        planned_at=iso(NIGHT), planned_tier=tier, run_id=None, action=None))


class IssuePlanTests(unittest.TestCase):
    def test_the_plan_section_goes_in_comes_out_and_is_replaced(self):
        body = issueplan.with_plan("Make the rules v2.", "1. Edit `src/a.ts`.", "`claude-3`", "run 7")
        self.assertTrue(body.startswith("Make the rules v2.\n\n" + issueplan.START + "\n## Plan\n"))
        self.assertIn("_Written by `claude-3` (run 7)", body)
        self.assertNotIn(MARKER, body)  # the description must not read as the bot's own
        self.assertEqual(issueplan.plan_of(body), "1. Edit `src/a.ts`.")
        self.assertEqual(issueplan.without_plan(body), "Make the rules v2.")
        again = issueplan.with_plan(body, "1. Edit `src/b.ts`.", "`claude-1`")
        self.assertEqual(again.count(issueplan.START), 1)
        self.assertEqual(issueplan.plan_of(again), "1. Edit `src/b.ts`.")
        # A person's edit is the plan; text they add after the section stays theirs.
        edited = again.replace("src/b.ts", "src/c.ts") + "\nAlso: thanks."
        self.assertEqual(issueplan.plan_of(edited), "1. Edit `src/c.ts`.")
        self.assertEqual(issueplan.without_plan(edited), "Make the rules v2.\n\nAlso: thanks.")
        self.assertEqual(issueplan.plan_of("No plan here."), "")
        self.assertEqual(issueplan.plan_of(None), "")

    def test_a_long_plan_gives_way_to_the_task(self):
        task = "t" * 30_000
        body = issueplan.with_plan(task, "p" * 60_000, "`claude-3`")
        self.assertLessEqual(len(body), issueplan.BODY_LIMIT)
        self.assertTrue(body.startswith(task))
        self.assertIn("cut to fit the description", body)


class StageTests(unittest.TestCase):
    def test_queued_items_without_a_strong_plan_carry_the_label(self):
        gh = FakeGitHub()
        ctx = lane_ctx(gh)
        busy(gh, ctx, *[(p, 50 + i) for i, p in enumerate(
            ("claude-3", "claude-1", "claude-4", "claude-2"))])  # nobody plans yet
        queue(gh, ctx, 3, planned=False)                  # no plan
        queue(gh, ctx, 4, EASY)                           # an easy item with a strong plan
        ctx.store.update(lambda s: state_item(s, 4).update(planned_tier="strong"))
        queue(gh, ctx, 5, EASY)                           # an easy item with a medium plan
        ctx.store.update(lambda s: state_item(s, 5).update(planned_tier="medium"))
        queue(gh, ctx, 6)                                 # a medium item with a medium plan
        ctx.store.update(lambda s: state_item(s, 6).update(planned_tier="medium"))
        gh.add_issue(7, labels=(LABEL_NEEDS_PLAN,))       # not queued any more
        plan_mod.make(ctx)
        labelled = {n for n in (3, 4, 5, 6, 7) if LABEL_NEEDS_PLAN in gh.label_names(n)}
        self.assertEqual(labelled, {3, 5})


class PlanningLaneTests(unittest.TestCase):
    def test_planning_comes_first_easy_first_and_takes_no_build_lane(self):
        """claude-3 plans while it builds; claude-1, which the quiet check guards, plans only
        when it holds nothing else; Devin builds once a strong model planned it."""
        gh = FakeGitHub()
        ctx = lane_ctx(gh)
        busy(gh, ctx, ("claude-3", 50), ("claude-4", 52), ("agy", 53), ("muse", 54), ("gpt", 55))
        queue(gh, ctx, 3, "priority:high", planned=False)    # medium and urgent
        queue(gh, ctx, 4, EASY, "priority:low", planned=False)  # easy: Devin's, once planned
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], planned["number"], planned["provider"]),
                         ("plan", 4, "claude-3"))  # building #50 does not stop it planning
        self.assertEqual(seats(planned)["plan"], ("claude-3", "opus", "strong"))
        self.assertEqual(gh.label_names(4), {LABEL_WORKING, LABEL_NEEDS_PLAN, EASY,
                                             "priority:low"})
        running(gh, ctx, planned)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], planned["number"], planned["provider"]),
                         ("plan", 3, "claude-1"))  # claude-1 holds nothing else
        running(gh, ctx, planned)
        # Both planning lanes are taken, and claude-1, planning, does not build too.
        self.assertNotIn(plan_mod.make(ctx)["action"], ("plan", "build"))
        planned_by(ctx, 4)
        gh.runs.pop("4")
        gh.threads[4]["labels"] = [{"name": n} for n in (LABEL_BUILD, EASY, "priority:low")]
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], planned["number"], planned["provider"]),
                         ("build", 4, "devin"))
        self.assertIsNone(seats(planned)["plan"])  # built from the plan, not planned again

    def test_claude_1_never_plans_while_it_builds(self):
        """The quiet check cannot tell a second run of the bot's on claude-1 from its owner."""
        gh = FakeGitHub()
        ctx = lane_ctx(gh, machine=(), env=secrets("CLAUDE_CODE_OAUTH_TOKEN"))
        busy(gh, ctx, ("claude-1", 50))
        queue(gh, ctx, 3, planned=False)
        self.assertNotIn(plan_mod.make(ctx)["action"], ("plan", "build"))
        gh.runs.pop("50")
        self.assertEqual(plan_mod.make(ctx)["action"], "plan")

    def test_a_planning_run_starts_with_every_build_lane_taken(self):
        gh = FakeGitHub()
        ctx = lane_ctx(gh, max_parallel=2)
        busy(gh, ctx, ("agy", 53), ("muse", 54))
        queue(gh, ctx, 3, planned=False)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], planned["provider"]), ("plan", "claude-3"))
        # With the planning lane off, every lane is busy.
        gh = FakeGitHub()
        ctx = make_ctx(gh, at=NIGHT, cfg=with_lanes(make_config(
            env=secrets(*providers.SECRETS), machine=MACHINE), 2))
        busy(gh, ctx, ("agy", 53), ("muse", 54))
        queue(gh, ctx, 3, planned=False)
        self.assertIn("every lane is busy", plan_mod.make(ctx)["reason"])

    def test_only_a_strong_model_plans_on_the_lane(self):
        """By day (the Claude accounts keep to the night here) no strong model is free: nothing
        is planned on the lane, and a medium builder plans its own item in its run."""
        gh = FakeGitHub()
        ctx = lane_ctx(gh, at=DAY, machine=ALL_MACHINE)
        queue(gh, ctx, 3, planned=False)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], planned["provider"]), ("build", "agy"))
        self.assertEqual(seats(planned)["plan"], ("agy", "gemini-3.8-flash-high", "medium"))
        self.assertNotIn(LABEL_NEEDS_PLAN, gh.label_names(3))  # this run plans it first
        # An easy item no strong model planned is never Devin's.
        gh = FakeGitHub()
        ctx = lane_ctx(gh, at=DAY, machine=("devin",))
        queue(gh, ctx, 3, EASY, planned=False)
        self.assertEqual(plan_mod.make(ctx)["action"], "none")
        self.assertIn(LABEL_NEEDS_PLAN, gh.label_names(3))

    def test_a_plan_written_in_a_build_run_goes_into_the_description_too(self):
        h = Harness(self, env=secrets(), machine=MACHINE, at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD,), body="Make the rules v2.")
        runner = FakeRunner({"build": builder({"src/game.txt": "rules v2\n"}),
                             "review": reviewer(APPROVE)})
        planned, result = h.night(runner)
        self.assertEqual((planned["action"], planned["provider"], result["status"]),
                         ("build", "agy", "approved"))
        record = h.ctx.store.load()["items"]["12"]
        self.assertEqual(record["planned_tier"], "medium")
        self.assertTrue(issueplan.plan_of(h.gh.threads[12]["body"]))
        self.assertNotIn(LABEL_NEEDS_PLAN, h.gh.label_names(12))


if __name__ == "__main__":
    unittest.main()
