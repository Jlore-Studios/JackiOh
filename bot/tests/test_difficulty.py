"""The difficulty system: every model a tier, every item a difficulty, and the difficulty deciding
who may plan, build and review it, in the owner's usage order; and a self-checking builder's loop.
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import tempfile
import unittest
from pathlib import Path

from harness import plan as plan_mod
from harness import providers, queue as queue_mod
from harness.clock import iso
from harness.config import LABEL_BUILD, LABEL_CROSS, LABEL_PR
from harness.errors import ConfigError
from harness.runner import FakeRunner, RunResult
from harness.state import item as state_item
from harness.work import NOTES_FILE, Worker

from tests.fakes import FakeGitHub, make_origin
from tests.support import ALL_MACHINE, DAY, MACHINE, NIGHT, make_config, make_ctx
from tests.test_cross import ALL
from tests.test_flow import Harness
from tests.test_providers import raw_providers, secrets
from tests.test_work import APPROVE, GATES, builder, changes, reviewer

EASY, MEDIUM, HARD = "difficulty:easy", "difficulty:medium", "difficulty:hard"


def ctx_for(gh, *, at=NIGHT, env=None, machine=MACHINE, committed_hours=False):
    return make_ctx(gh, at=at, cfg=make_config(env=env or secrets(*providers.SECRETS),
                                               machine=machine, committed_hours=committed_hours))


def queue(gh, ctx, number, *labels, planned=True):
    gh.add_issue(number, labels=(LABEL_BUILD, *labels))
    if planned:
        ctx.store.update(lambda s: state_item(s, number).update(planned_at=iso(DAY)))


def busy(gh, ctx, *providers_and_items):
    """Give each provider a run in progress on a working item, so it holds a lane."""
    for provider, number in providers_and_items:
        gh.add_issue(number, labels=("bot:working",))
        gh.runs[str(number)] = {"status": "in_progress"}
        ctx.store.update(lambda s, n=number, p=provider: state_item(s, n).update(
            run_id=str(n), provider=p))


def seats(planned):
    return {role: (seat["provider"], seat["model"], seat["tier"]) if seat else None
            for role, seat in planned["seats"].items() if role != "self_check"}


class DifficultyLabelTests(unittest.TestCase):
    def test_no_label_is_medium_and_the_hardest_counts(self):
        of = queue_mod.difficulty_of
        self.assertEqual(of(set()), "medium")
        self.assertEqual(of({EASY}), "easy")
        self.assertEqual(of({"Difficulty:HARD"}), "hard")
        self.assertEqual(of({EASY, HARD}), "hard")
        self.assertEqual(of({EASY, MEDIUM}), "medium")
        self.assertEqual(of({"difficulty:trivial", "bot:build"}), "medium")
        # A bot PR keeps its issue's difficulty on its record.
        self.assertEqual(of({LABEL_PR}, "hard"), "hard")
        self.assertEqual(of({EASY}, "medium"), "medium")

    def test_priority_never_decides_who_may_take_it(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY)
        queue(gh, ctx, 3, HARD, "priority:high")
        planned = plan_mod.make(ctx)
        self.assertEqual(planned["action"], "none")  # no strong model is open by day


class BuilderTests(unittest.TestCase):
    """The builder: the first free subscription in the usage order with a seat that meets the
    item's tier, on its weakest such seat."""

    def test_an_easy_item_is_built_with_sonnet_on_a_claude_account(self):
        """With Devin off, an easy item goes to claude-3's Sonnet, medium since #317 part 5."""
        gh = FakeGitHub()
        ctx = ctx_for(gh)
        queue(gh, ctx, 3, EASY, planned=False)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], planned["provider"], planned["difficulty"]),
                         ("build", "claude-3", "easy"))
        self.assertEqual(seats(planned), {"plan": ("claude-3", "opus", "strong"),
                                          "build": ("claude-3", "sonnet", "medium"),
                                          "review": ("claude-3", "opus", "strong")})
        self.assertEqual(planned["routing"], [
            "#3: stepped up from weak to medium to build it: no weak-tier model (swe-2-max) is "
            "free now"])

    def test_a_medium_item_on_a_claude_account_is_built_with_sonnet(self):
        """Sonnet at xhigh is medium (#317 part 5), so claude-3 builds a medium item with it and
        keeps Opus for planning, reviews and hard items."""
        gh = FakeGitHub()
        ctx = ctx_for(gh)
        queue(gh, ctx, 3)
        planned = plan_mod.make(ctx)
        self.assertEqual(seats(planned)["build"], ("claude-3", "sonnet", "medium"))
        self.assertEqual(planned["routing"], [])

    def test_a_hard_item_takes_a_strong_model_only(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh)
        busy(gh, ctx, ("claude-3", 50), ("claude-3", 54), ("claude-1", 51), ("claude-1", 55),
             ("claude-4", 52), ("claude-6", 71), ("claude-5", 70), ("claude-2", 53),
             ("claude-2", 56))
        queue(gh, ctx, 3, HARD)
        planned = plan_mod.make(ctx)
        self.assertEqual(planned["action"], "none")  # agy, muse and gpt are free, and medium
        self.assertIn("no subscription can take the queue now", planned["reason"])

    def test_the_usage_order(self):
        """For easy items Devin first while it has a lane (`easy_first`); then claude-3, then
        claude-1 (each twice: two lanes) with Sonnet; then claude-4, claude-6 and claude-5 with
        Sonnet, their stand-in for Devin while Devin's lanes are full (`takes_over`, #317 part
        9); then the medium models, Muse first on both its lanes; and claude-2 builds only after
        all of them (`build_last`)."""
        gh = FakeGitHub()
        ctx = ctx_for(gh, machine=ALL_MACHINE)
        for number in range(3, 14):
            queue(gh, ctx, number, EASY)
            ctx.store.update(lambda s, n=number: state_item(s, n).update(
                queued_at=f"2026-09-{n + 10:02d}T00:00:00Z"))
        ctx = make_ctx(gh, at=NIGHT, cfg=dataclasses.replace(
            ctx.cfg, pool=dataclasses.replace(ctx.cfg.pool, max_parallel=20, machine_parallel=20,
                                              providers={**ctx.cfg.pool.providers, "devin":
                                                         dataclasses.replace(ctx.cfg.pool.get(
                                                             "devin"), lanes=1)})))
        order = []
        for _ in range(11):
            planned = plan_mod.make(ctx)
            if planned["action"] == "none":
                break
            gh.runs[str(planned["number"])] = {"status": "in_progress"}
            ctx.store.update(lambda s, n=planned["number"]: state_item(s, n).update(
                run_id=str(n)))
            order.append((planned["provider"], seats(planned)["build"][1]))
        self.assertEqual(order, [("devin", "swe-2-max"), ("claude-3", "sonnet"),
                                 ("claude-3", "sonnet"),
                                 ("claude-1", "sonnet"), ("claude-1", "sonnet"),
                                 ("claude-4", "sonnet"),
                                 ("claude-6", "sonnet"), ("claude-5", "sonnet"),
                                 ("muse", "muse-spark-1.3-contributor"),
                                 ("muse", "muse-spark-1.3-contributor"),
                                 ("agy", "gemini-3.8-flash-high")])

    def test_a_sonnet_stand_in_builds_only_easy_items_devin_cannot_take(self):
        """claude-4's Sonnet stands in for Devin (`takes_over`): with Devin free it builds
        nothing, and it never builds a medium item, which claude-4 builds with Opus."""
        gh = FakeGitHub()
        ctx = ctx_for(gh, machine=("devin",))
        busy(gh, ctx, ("claude-3", 50), ("claude-3", 59), ("claude-1", 51), ("claude-1", 58))
        queue(gh, ctx, 3, EASY)
        self.assertEqual(seats(plan_mod.make(ctx))["build"], ("devin", "swe-2-max", "weak"))
        gh = FakeGitHub()
        ctx = ctx_for(gh, machine=("devin",))
        ctx = make_ctx(gh, at=NIGHT, cfg=dataclasses.replace(
            ctx.cfg, pool=dataclasses.replace(ctx.cfg.pool, max_parallel=20, machine_parallel=20)))
        busy(gh, ctx, ("claude-3", 50), ("claude-3", 59), ("claude-1", 51), ("claude-1", 58),
             *[("devin", n) for n in range(60, 66)])
        queue(gh, ctx, 3, EASY)
        self.assertEqual(seats(plan_mod.make(ctx))["build"], ("claude-4", "sonnet", "medium"))
        gh = FakeGitHub()
        ctx = ctx_for(gh, machine=())
        busy(gh, ctx, ("claude-3", 50), ("claude-3", 59), ("claude-1", 51), ("claude-1", 58))
        queue(gh, ctx, 3, MEDIUM)
        self.assertEqual(seats(plan_mod.make(ctx))["build"], ("claude-4", "opus", "strong"))
        # A stand-in never reviews: the review rule counts the run's reviewer, Opus.
        self.assertEqual(ctx.cfg.pool.best_seat(ctx.cfg.pool.get("claude-4"), "medium").model,
                         "opus")

    def test_sonnet_only_while_claude_1_or_claude_3_is_open(self):
        # claude-3 full (both lanes), claude-1 open and Devin full: claude-1 builds the easy
        # item with Sonnet.
        gh = FakeGitHub()
        ctx = ctx_for(gh, machine=("devin",))
        busy(gh, ctx, ("claude-3", 50), ("claude-3", 59), *[("devin", n) for n in range(60, 66)])
        queue(gh, ctx, 3, EASY)
        self.assertEqual(seats(plan_mod.make(ctx))["build"], ("claude-1", "sonnet", "medium"))
        # Neither open and the medium models busy: Devin builds it, the default, while claude-2
        # (build_last, kept for planning and review) stays free.
        gh = FakeGitHub()
        ctx = ctx_for(gh, machine=ALL_MACHINE)
        busy(gh, ctx, ("claude-3", 50), ("claude-3", 63), ("claude-1", 51), ("claude-1", 64),
             ("claude-4", 52), ("claude-6", 71), ("claude-5", 70), ("agy", 53))
        ctx = make_ctx(gh, at=NIGHT, cfg=dataclasses.replace(
            ctx.cfg, pool=dataclasses.replace(ctx.cfg.pool, max_parallel=20, machine_parallel=20)))
        busy(gh, ctx, ("muse", 54), ("muse", 62), ("gpt", 55))  # Muse has two lanes
        queue(gh, ctx, 3, EASY)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["provider"], seats(planned)["build"][1]), ("devin", "swe-2-max"))
        # Only with all of Devin's lanes taken does claude-2 build it, with Opus (no Sonnet).
        gh.threads[3]["state"] = "open"
        gh.threads[3]["labels"] = [{"name": LABEL_BUILD}, {"name": EASY}]
        busy(gh, ctx, *[("devin", n) for n in range(56, 62)])
        self.assertEqual(seats(plan_mod.make(ctx))["build"], ("claude-2", "opus", "strong"))

    def test_claude_2_builds_a_medium_item_devin_may_not(self):
        """Devin is weak, so a medium item passes it by: with only claude-2 and Devin free,
        claude-2 builds it."""
        gh = FakeGitHub()
        ctx = ctx_for(gh, machine=ALL_MACHINE)
        ctx = make_ctx(gh, at=NIGHT, cfg=dataclasses.replace(
            ctx.cfg, pool=dataclasses.replace(ctx.cfg.pool, max_parallel=20, machine_parallel=20)))
        busy(gh, ctx, ("claude-3", 50), ("claude-3", 57), ("claude-1", 51), ("claude-1", 58),
             ("claude-4", 52), ("claude-6", 71), ("claude-5", 70), ("agy", 53), ("muse", 54),
             ("muse", 56), ("gpt", 55))
        queue(gh, ctx, 3, MEDIUM)
        self.assertEqual(seats(plan_mod.make(ctx))["build"], ("claude-2", "opus", "strong"))

    def test_with_no_model_of_its_tier_free_it_steps_up_and_says_why(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY, machine=("agy",))  # by day: no Claude, and Devin off
        queue(gh, ctx, 3, EASY)
        planned = plan_mod.make(ctx)
        self.assertEqual(seats(planned)["build"], ("agy", "gemini-3.8-flash-high", "medium"))
        self.assertEqual(planned["routing"], [
            "#3: stepped up from weak to medium to build it: no weak-tier model (swe-2-max) is "
            "free now"])
        self.assertIn("stepped up from weak to medium", planned["assignment"] + " ".join(
            planned["routing"]))
        # Medium with only a strong model free steps up to strong.
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY, env=secrets("CLAUDE_CODE_OAUTH_TOKEN_2"), machine=(),
                      committed_hours=True)
        queue(gh, ctx, 3)
        planned = plan_mod.make(ctx)
        self.assertEqual(seats(planned)["build"], ("claude-2", "opus", "strong"))
        self.assertIn("stepped up from medium to strong", planned["routing"][0])

    def test_a_persons_pull_request_is_revised_only_with_a_reviewer_in_the_run(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY, env=secrets(), machine=("devin",))
        gh.add_pull(9, "feature/someone", labels=("bot:revise", EASY))
        # Only Devin is free, and it cannot review: a person's PR gets no review run, so it waits.
        self.assertEqual(plan_mod.make(ctx)["action"], "none")
        # The bot's own pull request does get a review run, so Devin may revise it.
        gh.add_pull(10, "bot/issue-1", labels=(LABEL_PR, "bot:revise", EASY))
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["number"], planned["provider"]), (10, "devin"))

    def test_revisions_follow_the_building_rule(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh)
        gh.add_pull(9, "bot/issue-1", labels=(LABEL_PR, "bot:revise", EASY))
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], seats(planned)["build"]),
                         ("revise", ("claude-3", "sonnet", "medium")))
        self.assertIsNone(seats(planned)["plan"])  # a revision is not planned again


class PlannerTests(unittest.TestCase):
    def test_a_strong_planner_plans_on_the_planning_lane(self):
        """By day claude-2, strong and kept for planning, plans it on the planning lane before
        agy builds it."""
        gh = FakeGitHub()
        ctx = make_ctx(gh, at=DAY, cfg=make_config(env=secrets("CLAUDE_CODE_OAUTH_TOKEN_2"),
                                                   machine=MACHINE, committed_hours=True,
                                                   plan_lanes=None))
        queue(gh, ctx, 3, planned=False)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], planned["provider"]), ("plan", "claude-2"))
        self.assertEqual(seats(planned), {"plan": ("claude-2", "opus", "strong"),
                                          "build": None, "review": None})
        self.assertIn("needs a plan; `claude-2`", planned["routing"][0])
        self.assertIn("goes into this issue's description", gh.bot_comments(3)[-1])

    def test_with_no_strong_model_free_a_medium_one_plans_in_the_same_run(self):
        """An item nobody has rated or planned may be rated and planned by a medium model
        (#317 part 8): Muse, first of them, plans it in its own run."""
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY)
        queue(gh, ctx, 3, planned=False)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], seats(planned)["plan"]),
                         ("build", ("muse", "muse-spark-1.3-contributor", "medium")))

    def test_a_medium_model_never_plans_a_rated_medium_item(self):
        """A medium or hard item is planned by a strong model only (#317 part 6): with none free,
        a medium builder does not plan it in its own run, though it plans an easy one."""
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY)
        queue(gh, ctx, 3, MEDIUM, planned=False)
        self.assertEqual(plan_mod.make(ctx)["action"], "none")
        queue(gh, ctx, 4, EASY, planned=False)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["number"], seats(planned)["plan"][2]), (4, "medium"))

    def test_with_no_planner_free_nothing_is_built(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY, env=secrets(), machine=("devin",))
        queue(gh, ctx, 3, EASY, planned=False)
        planned = plan_mod.make(ctx)
        self.assertEqual(planned["action"], "none")

    def test_a_planning_run_hands_its_plan_to_the_builder(self):
        h = Harness(self, env=secrets("CLAUDE_CODE_OAUTH_TOKEN_2"), machine=MACHINE, at=DAY,
                    plan_lanes=None)
        h.committed_hours()
        h.gh.add_issue(12, labels=(LABEL_BUILD,), body="Make the rules v2.")
        plan_text = "1. **Goal.** Rules v2.\n2. **Steps.** Edit src/game.txt; run the checks."
        planner = FakeRunner({"plan": lambda request: RunResult(True, plan_text)})
        planned, result = h.night(planner)
        self.assertEqual((planned["action"], planned["provider"], result["status"]),
                         ("plan", "claude-2", "planned"))
        self.assertEqual([(c.role, c.model, c.read_only) for c in planner.calls],
                         [("plan", "opus", True)])
        record = h.ctx.store.load()["items"]["12"]
        self.assertEqual(record["handoff"]["kind"], "plan")
        self.assertIn("Rules v2", record["handoff"]["notes"])
        self.assertTrue(record["planned_at"])
        self.assertEqual(record["planned_tier"], "strong")
        self.assertEqual(h.gh.label_names(12), {LABEL_BUILD, "bot:planned"})  # out of the Needs plan stage
        self.assertIn("Planned on `claude-2` (claude, `opus`, strong)", h.gh.bot_comments(12)[-1])
        self.assertIn("in this issue's description", h.gh.bot_comments(12)[-1])
        body = h.gh.threads[12]["body"]
        self.assertTrue(body.startswith("Make the rules v2.\n\n<!-- jackioh-bot:plan -->"))
        self.assertIn("## Plan", body)
        self.assertIn("Edit src/game.txt", body)
        self.assertIsNone(h.origin_sha("bot/issue-12"))  # nothing built yet
        # A person corrects the plan in the description: the builder follows their version.
        h.gh.threads[12]["body"] = body.replace("Edit src/game.txt", "Edit src/game.txt only")
        # The next run builds from it, on the cheapest model its difficulty allows.
        seen = {}
        def building(request):
            seen["notes"] = (request.cwd / NOTES_FILE).read_text()
            return builder({"src/game.txt": "rules v2\n"})(request)
        runner = FakeRunner({"build": building, "review": reviewer(APPROVE)})
        planned, result = h.night(runner)
        self.assertEqual((planned["action"], planned["provider"], result["status"]),
                         ("build", "muse", "approved"))
        self.assertEqual([c.role for c in runner.calls], ["build", "review"])
        self.assertIn("## The plan", runner.calls[0].prompt)
        self.assertIn("**Plan** section of the issue's description", runner.calls[0].prompt)
        self.assertIn("Edit src/game.txt only", runner.calls[0].prompt)
        self.assertIn("Edit src/game.txt only", seen["notes"])


class ReviewRunTests(unittest.TestCase):
    def pull(self, gh, ctx, labels=(), votes=None):
        gh.add_pull(9, "bot/issue-5", labels=(LABEL_PR, LABEL_CROSS, *labels), sha="h1")
        ctx.store.update(lambda s: state_item(s, 9).update(votes=votes or {
            "sha": "h1", "builder": "gpt", "approvals": ["gpt"], "tiers": {"gpt": "medium"}}))

    def test_strong_first_in_the_usage_order(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh)
        self.pull(gh, ctx)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], seats(planned)["review"]),
                         ("review", ("claude-3", "opus", "strong")))
        busy(gh, ctx, ("claude-3", 50), ("claude-3", 57), ("claude-1", 51), ("claude-1", 58))
        gh.threads[9]["labels"] = [{"name": LABEL_PR}, {"name": LABEL_CROSS}]
        self.assertEqual(seats(plan_mod.make(ctx))["review"][0], "claude-4")

    def test_otherwise_a_medium_model_another_family_first_then_the_same_one(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY)
        self.pull(gh, ctx)
        planned = plan_mod.make(ctx)
        self.assertEqual(seats(planned)["review"], ("muse", "muse-spark-1.3-contributor", "medium"))
        self.assertEqual(planned["approved"], ["gpt"])
        # With only gpt set up, gpt reviews it again: the same medium model may give both.
        gh2 = FakeGitHub()
        ctx2 = ctx_for(gh2, at=DAY, machine=("gpt",))
        self.pull(gh2, ctx2)
        self.assertEqual(seats(plan_mod.make(ctx2))["review"], ("gpt", "gpt-5.6-terra", "medium"))

    def test_a_hard_pull_request_takes_a_strong_model_first_or_a_medium_one(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY, committed_hours=True)  # claude-3 all day
        self.pull(gh, ctx, labels=(HARD,))
        self.assertEqual(seats(plan_mod.make(ctx))["review"], ("claude-3", "opus", "strong"))
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY)  # no Claude account by day: two medium approvals do
        self.pull(gh, ctx, labels=(HARD,))
        self.assertEqual(seats(plan_mod.make(ctx))["review"],
                         ("muse", "muse-spark-1.3-contributor", "medium"))

    def test_a_weak_model_reviews_only_an_easy_item_short_of_a_weak_approval(self):
        def review_with_devin_only(labels, votes=None):
            gh = FakeGitHub()
            ctx = ctx_for(gh, at=DAY, env=secrets(), machine=("devin",))
            self.pull(gh, ctx, labels=labels, votes=votes)
            return plan_mod.make(ctx)

        planned = review_with_devin_only((EASY,))  # gpt approved it: a weak approval completes it
        self.assertEqual((planned["action"], seats(planned)["review"]),
                         ("review", ("devin", "swe-2-max", "weak")))
        self.assertEqual(review_with_devin_only(())["action"], "none")  # medium: weak never counts
        self.assertEqual(review_with_devin_only((HARD,))["action"], "none")
        # An easy item with a weak approval already needs a medium or strong one, not another weak.
        weak_only = {"sha": "h1", "builder": "cognition", "approvals": ["cognition"],
                     "tiers": {"cognition": "weak"}}
        self.assertEqual(review_with_devin_only((EASY,), weak_only)["action"], "none")

    def test_whether_the_reviewers_set_up_could_still_ship_it(self):
        from harness import review_rule
        self.assertTrue(review_rule.reachable([], {"medium"}, "hard"))  # one medium model, twice
        self.assertFalse(review_rule.reachable([], {"weak"}, "easy"))  # Devin alone never ships it
        self.assertTrue(review_rule.reachable(["medium"], {"weak"}, "easy"))
        self.assertFalse(review_rule.reachable(["medium"], {"weak"}, "medium"))
        self.assertFalse(review_rule.reachable(["weak"], set(), "easy"))
        self.assertEqual(review_rule.who([("muse", "medium"), ("muse", "medium"),
                                          ("claude", "strong")]),
                         "`muse` (medium) twice, `claude` (strong)")

    def test_the_rule(self):
        h = Harness(self, env=ALL, machine=MACHINE)
        from harness.deliver import Deliverer
        out = h.root / "o"
        out.mkdir()
        d = Deliverer(h.ctx, {"action": "review", "provider": "agy"}, out, h.deliver_repo)
        rule = d._rule_met

        def reviews(*pairs):
            return {"approvals": list(dict.fromkeys(f for f, _ in pairs)),
                    "reviews": [{"family": f, "tier": t} for f, t in pairs]}

        strong, gemini, muse, devin = (("claude", "strong"), ("gemini", "medium"),
                                       ("muse", "medium"), ("cognition", "weak"))
        for difficulty in ("easy", "medium", "hard"):
            # One strong approval, or two medium ones (the same model twice too), at every
            # difficulty.
            self.assertTrue(rule(reviews(strong), difficulty))
            self.assertTrue(rule(reviews(gemini, muse), difficulty))
            self.assertTrue(rule(reviews(muse, muse), difficulty))
            self.assertFalse(rule(reviews(gemini), difficulty))
            self.assertFalse(rule(reviews(devin, devin), difficulty))
        # A weak and a medium approval: only for an easy item.
        self.assertTrue(rule(reviews(devin, gemini), "easy"))
        self.assertFalse(rule(reviews(devin, gemini), "medium"))
        self.assertFalse(rule(reviews(devin, gemini), "hard"))
        # Votes kept before reviews were counted one by one: one per family.
        self.assertTrue(rule({"approvals": ["gemini", "muse"],
                              "tiers": {"gemini": "medium", "muse": "medium"}}, "hard"))
        self.assertTrue(rule({"approvals": ["gemini", "cognition"],
                              "tiers": {"gemini": "medium", "cognition": "weak"}}, "easy"))
        self.assertFalse(rule({"approvals": ["gemini", "cognition"],
                               "tiers": {"gemini": "medium", "cognition": "weak"}}))
        # A rejection stands until its model approves.
        self.assertFalse(rule({"approvals": ["claude"], "rejections": ["gemini"],
                               "tiers": {"claude": "strong"}}))
        # A vote kept before tiers counts at its family's strongest tier.
        self.assertTrue(rule({"approvals": ["claude"]}))
        self.assertFalse(rule({"approvals": ["gpt"]}))


class SelfCheckTests(unittest.TestCase):
    """A builder whose seat has `self_check` (Devin) checks its own change before any review."""

    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        self.origin, self.clone = make_origin(self.root)
        self.out = self.root / "out"

    def worker(self, runner, *, provider="devin", review=None, rounds=3, action="build"):
        raw = raw_providers()
        if provider == "agy":
            raw["providers"]["agy"]["self_check"] = True  # a self-checker with a reviewer too
        cfg = make_config(gates=GATES, install={"run": "true", "timeout_minutes": 1},
                          max_review_cycles=3, max_self_check_rounds=rounds, machine=ALL_MACHINE)
        cfg = dataclasses.replace(cfg, pool=providers.parse(raw))
        build = cfg.pool.seats(cfg.pool.get(provider))[0]
        plan = {"action": action, "number": 12, "title": "Rules v2", "branch": "bot/issue-12",
                "thread": "Please make the rules v2.", "provider": provider,
                "seats": {"plan": None, "build": build.to_dict(),
                          "review": build.to_dict() if review else None, "self_check": True}}
        if action == "revise":
            from tests.fakes import push_branch
            push_branch(self.origin, self.root, "bot/issue-12", {"src/game.txt": "v1\n"})
            from tests.fakes import git
            git(self.clone, "fetch", "-q", "origin")
            plan.update(source="request", pull="p", issue="", feedback="f")
        return Worker(cfg, plan, runner, self.clone, self.root / "work", self.out)

    def test_a_clean_first_self_check(self):
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                             "self_check": reviewer(APPROVE)})
        result = self.worker(runner).run()
        self.assertEqual([(c.role, c.model, c.read_only) for c in runner.calls],
                         [("build", "swe-2-max", False), ("self_check", "swe-2-max", True)])
        self.assertIn("This is a self check, not a review", runner.calls[1].prompt)
        # Devin has no seat that may review: the change goes to a review run, never approved.
        self.assertEqual(result["status"], "built")
        self.assertEqual(len(result["cycles"][0]["self_check"]), 1)
        self.assertEqual(result["self_check_findings"], [])
        self.assertTrue((self.out / "branch.bundle").exists())

    def test_flagged_then_fixed(self):
        runner = FakeRunner({"build": builder({"src/game.txt": "v2 (half)\n"}),
                             "fix": builder({"src/game.txt": "v2\n"}),
                             "self_check": [reviewer(changes("Only half the rules.")),
                                            reviewer(APPROVE)]})
        result = self.worker(runner).run()
        self.assertEqual([c.role for c in runner.calls],
                         ["build", "self_check", "fix", "self_check"])
        self.assertIn("Only half the rules.", runner.calls[2].prompt)
        self.assertIn("Your self check's blocking findings", runner.calls[2].prompt)
        self.assertEqual((result["status"], result["self_check_findings"]), ("built", []))
        rounds = result["cycles"][0]["self_check"]
        self.assertEqual([r["flagged"] for r in rounds], [1, 0])

    def test_the_cap_sends_it_to_review_with_the_open_findings(self):
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                             "fix": builder({"src/game.txt": "v2 again\n"}),
                             "self_check": reviewer(changes("Still not sure about the Coin.")),
                             "review": reviewer(APPROVE)})
        result = self.worker(runner, provider="agy", review=True, rounds=2).run()
        self.assertEqual([c.role for c in runner.calls],
                         ["build", "self_check", "fix", "self_check", "review"])
        review_prompt = runner.calls[-1].prompt
        self.assertIn("ran out with these findings still open", review_prompt)
        self.assertIn("Still not sure about the Coin.", review_prompt)
        self.assertEqual(result["status"], "approved")
        self.assertEqual(result["self_check_findings"][0]["claim"], "Still not sure about the Coin.")

    def test_a_clean_self_check_is_not_an_approval(self):
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                             "fix": builder({"src/game.txt": "v2 fixed\n"}),
                             "self_check": reviewer(APPROVE),
                             "review": [reviewer(changes("The Coin is wrong.")), reviewer(APPROVE)]})
        result = self.worker(runner, provider="agy", review=True).run()
        # The reviewer's findings go to a fix, which goes through the self check again.
        self.assertEqual([c.role for c in runner.calls],
                         ["build", "self_check", "review", "fix", "self_check", "review"])
        self.assertEqual(result["status"], "approved")
        self.assertEqual(len(result["cycles"]), 2)

    def test_a_revision_checks_itself_too(self):
        runner = FakeRunner({"revise": builder({"src/game.txt": "v2\n"}),
                             "self_check": reviewer(APPROVE)})
        result = self.worker(runner, action="revise").run()
        self.assertEqual([c.role for c in runner.calls], ["revise", "self_check"])
        self.assertEqual(result["status"], "built")

    def test_a_self_checked_build_waits_for_a_review_run(self):
        h = Harness(self, env=secrets(), machine=("devin",), at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD, EASY))
        h.ctx.store.update(lambda s: state_item(s, 12).update(planned_at=iso(DAY)))
        planned, result = h.night(FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                                              "self_check": reviewer(APPROVE)}))
        self.assertEqual((planned["provider"], result["status"]), ("devin", "built"))
        pull = h.gh.list_pulls(head="bot/issue-12")[0]
        pr = int(pull["number"])
        self.assertFalse(pull["draft"])
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_CROSS, EASY})
        self.assertEqual(h.gh.auto_merge, {})
        votes = h.ctx.store.load()["items"][str(pr)]["votes"]
        self.assertEqual((votes["builder"], votes["approvals"]), ("cognition", []))
        comment = h.gh.bot_comments(12)[-1]
        self.assertIn("It checked itself 1 time(s), the last one clean; that is not a review.",
                      comment)
        self.assertIn("No model has reviewed it yet", comment)
        self.assertIn("No subscription that could give that review is set up", comment)


class ConfigTests(unittest.TestCase):
    def test_max_self_check_rounds(self):
        self.assertEqual(make_config().max_self_check_rounds, 3)
        self.assertEqual(make_config(max_self_check_rounds=5).max_self_check_rounds, 5)
        with self.assertRaises(ConfigError):
            make_config(max_self_check_rounds=0)

    def test_status_and_providers_show_tiers_and_self_check(self):
        from harness.status import report
        gh = FakeGitHub()
        text = report(ctx_for(gh, machine=ALL_MACHINE))
        self.assertIn("`devin` (devin: `swe-2-max` weak, self-check;", text)
        self.assertIn("`claude-3` (claude: `opus` strong, `sonnet` medium;", text)
        # `harness providers` (and `window`, the same command), as bot selftest runs it.
        import contextlib
        import io
        from harness.__main__ import cmd_providers
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            self.assertEqual(cmd_providers(make_config(env=secrets(*providers.SECRETS),
                                                       machine=ALL_MACHINE),
                                           argparse.Namespace(offline=True)), 0)
        printed = out.getvalue()
        self.assertIn("swe-2-max weak self-check", printed)
        self.assertIn("weak tier, tried in this order:", printed)
        self.assertIn("swe-2-max (self-check)", printed)

    def test_peek_shows_the_difficulty_and_each_roles_tier(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh)
        queue(gh, ctx, 3, EASY, planned=False)
        look = plan_mod.peek(ctx)
        self.assertEqual(look.reason, "#3 (easy) is queued to build, for `claude-3`: plan on "
                         "`claude-3` (claude, `opus`, strong); build on `claude-3` (claude, "
                         "`sonnet`, medium); review on `claude-3` (claude, `opus`, strong)")


if __name__ == "__main__":
    unittest.main()
