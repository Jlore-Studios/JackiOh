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
from tests.support import ALL_MACHINE, DAY, MACHINE, NIGHT, make_config, make_ctx, with_weak
from tests.test_cross import ALL
from tests.test_flow import Harness
from tests.test_providers import raw_providers, secrets
from tests.test_work import APPROVE, GATES, builder, changes, reviewer

EASY, MEDIUM, HARD = "difficulty:easy", "difficulty:medium", "difficulty:hard"
CLAUDE_SECRETS = (("claude-1", "CLAUDE_CODE_OAUTH_TOKEN"), ("claude-2", "CLAUDE_CODE_OAUTH_TOKEN_2"),
                  ("claude-3", "CLAUDE_CODE_OAUTH_TOKEN_3"), ("claude-4", "CLAUDE_CODE_OAUTH_TOKEN_4"),
                  ("claude-5", "CLAUDE_CODE_OAUTH_TOKEN_5"), ("claude-6", "CLAUDE_CODE_OAUTH_TOKEN_6"),
                  ("claude-7", "CLAUDE_CODE_OAUTH_TOKEN_7"), ("claude-8", "CLAUDE_CODE_OAUTH_TOKEN_8"))


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
        """With the weaker models off, an easy item goes to claude-3's Sonnet, medium since
        #317 part 5."""
        gh = FakeGitHub()
        ctx = ctx_for(gh, env=secrets("CLAUDE_CODE_OAUTH_TOKEN_3"), machine=())
        queue(gh, ctx, 3, EASY, planned=False)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], planned["provider"], planned["difficulty"]),
                         ("build", "claude-3", "easy"))
        self.assertEqual(seats(planned), {"plan": ("claude-3", "opus", "strong"),
                                          "build": ("claude-3", "sonnet", "medium"),
                                          "review": ("claude-3", "opus", "strong")})
        self.assertEqual(planned["routing"], [
            "#3: stepped up from weak to medium to build it: no weak-tier model "
            "(lite-1) is free now"])

    def test_a_medium_item_on_a_claude_account_is_built_with_sonnet(self):
        """Sonnet at xhigh is medium (#317 part 5), so claude-3 builds a medium item with it and
        keeps Opus for planning, reviews and hard items."""
        gh = FakeGitHub()
        ctx = ctx_for(gh, env=secrets("CLAUDE_CODE_OAUTH_TOKEN_3"), machine=())
        queue(gh, ctx, 3)
        planned = plan_mod.make(ctx)
        self.assertEqual(seats(planned)["build"], ("claude-3", "sonnet", "medium"))
        self.assertEqual(planned["routing"], [])

    def test_a_hard_item_takes_a_strong_model_only(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh)
        # Room past the Claude accounts' thirteen runs, so the medium models are free to refuse it.
        ctx = make_ctx(gh, at=NIGHT, cfg=dataclasses.replace(
            ctx.cfg, pool=dataclasses.replace(ctx.cfg.pool, max_parallel=20)))
        busy(gh, ctx, ("claude-3", 50), ("claude-3", 54), ("claude-7", 72), ("claude-7", 73),
             ("claude-8", 74), ("claude-8", 75), ("claude-1", 51),
             ("claude-1", 55), ("claude-4", 52), ("claude-6", 71), ("claude-5", 70),
             ("claude-2", 53), ("claude-2", 56))
        queue(gh, ctx, 3, HARD)
        planned = plan_mod.make(ctx)
        self.assertEqual(planned["action"], "none")  # agy and muse are free and medium, gpt weak
        self.assertIn("no subscription can take the queue now", planned["reason"])

    def test_the_usage_order(self):
        """For easy items the lanes fill in tier order (#695): the weak model first while it
        has a lane, the weak one; then the medium models, Muse on its four lanes (#1273), agy on its
        one and Codex on its one;
        then the Sonnet seats in usage order, claude-3, claude-7, claude-8, claude-1, claude-4,
        claude-6 and claude-5 once each, and only then claude-3's second lane (two lanes, uncapped)
        and claude-8's second (two lanes): with every account's room level, the load
        balancer gives the next run to one holding none (`plan.balance`); and claude-2 builds only
        after all of them (`build_last`). Past that every lane is held, and the rest wait."""
        gh = FakeGitHub()
        ctx = ctx_for(gh, machine=ALL_MACHINE)
        for number in range(3, 21):
            queue(gh, ctx, number, EASY)
            ctx.store.update(lambda s, n=number: state_item(s, n).update(
                queued_at=f"2026-09-{n + 10:02d}T00:00:00Z"))
        ctx = make_ctx(gh, at=NIGHT, cfg=dataclasses.replace(
            ctx.cfg, pool=dataclasses.replace(ctx.cfg.pool, max_parallel=20, machine_parallel=20,
                                              providers={**ctx.cfg.pool.providers, "lite":
                                                         dataclasses.replace(ctx.cfg.pool.get(
                                                             "lite"), lanes=1)})))
        order = []
        for _ in range(18):
            planned = plan_mod.make(ctx)
            if planned["action"] == "none":
                break
            gh.runs[str(planned["number"])] = {"status": "in_progress"}
            ctx.store.update(lambda s, n=planned["number"]: state_item(s, n).update(
                run_id=str(n)))
            order.append((planned["provider"], seats(planned)["build"][1]))
        self.assertEqual(order, [("lite", "lite-1"),
                                 *[("muse", "muse-spark-1.3-contributor")] * 4,
                                 ("agy", "gemini-3.8-flash-high"), ("gpt", "gpt-5.6-sol"),
                                 ("claude-3", "sonnet"), ("claude-7", "sonnet"),
                                 ("claude-8", "sonnet"), ("claude-1", "sonnet"),
                                 ("claude-4", "sonnet"), ("claude-6", "sonnet"),
                                 ("claude-5", "sonnet"), ("claude-3", "sonnet"),
                                 ("claude-8", "sonnet"), ("claude-2", "sonnet")])

    def test_a_sonnet_stand_in_builds_only_easy_items_the_weak_one_cannot_take(self):
        """A seat marked `takes_over` stands in for another subscription: with the weak one free it
        builds nothing, and it never builds a medium item, which its account builds with Opus.
        No committed seat stands in now (every Claude account switches between its two models),
        so the test marks claude-4's Sonnet itself."""
        def stand_in(gh, machine):
            raw = with_weak(raw_providers())
            for name, provider in raw["providers"].items():
                if provider.get("login") == "machine":
                    provider["enabled"] = name in machine
                if provider.get("cli") == "claude":
                    provider["schedule"] = {"mode": "window", "start": "21:00", "end": "07:00"}
                    provider.pop("off_hours", None)
            raw["plan_lanes"] = 0
            raw["providers"]["claude-4"]["extra_models"][0]["takes_over"] = "lite"
            cfg = make_config(env=secrets(*providers.SECRETS), machine=machine)
            return make_ctx(gh, at=NIGHT, cfg=dataclasses.replace(
                cfg, pool=dataclasses.replace(providers.parse(raw), max_parallel=20,
                                              machine_parallel=20)))
        gh = FakeGitHub()
        ctx = stand_in(gh, ("lite",))
        busy(gh, ctx, ("claude-3", 50), ("claude-3", 59), ("claude-7", 57), ("claude-7", 67),
             ("claude-1", 51),
             ("claude-1", 58))
        queue(gh, ctx, 3, EASY)
        self.assertEqual(seats(plan_mod.make(ctx))["build"], ("lite", "lite-1", "weak"))
        gh = FakeGitHub()
        ctx = stand_in(gh, ("lite",))
        busy(gh, ctx, ("claude-3", 50), ("claude-3", 59), ("claude-7", 57), ("claude-7", 67),
             ("claude-8", 68), ("claude-8", 75), ("claude-1", 51),
             ("claude-1", 58),
             *[("lite", n) for n in range(60, 66)])
        queue(gh, ctx, 3, EASY)
        self.assertEqual(seats(plan_mod.make(ctx))["build"], ("claude-4", "sonnet", "medium"))
        gh = FakeGitHub()
        ctx = stand_in(gh, ())
        busy(gh, ctx, ("claude-3", 50), ("claude-3", 59), ("claude-7", 57), ("claude-7", 67),
             ("claude-8", 68), ("claude-8", 75), ("claude-1", 51),
             ("claude-1", 58), ("claude-6", 71), ("claude-5", 70),
             ("claude-2", 53), ("claude-2", 56))
        queue(gh, ctx, 3, MEDIUM)
        self.assertEqual(seats(plan_mod.make(ctx))["build"], ("claude-4", "opus", "strong"))
        # A stand-in never reviews: the review rule counts the run's reviewer, Opus.
        self.assertEqual(ctx.cfg.pool.best_seat(ctx.cfg.pool.get("claude-4"), "medium").model,
                         "opus")

    def test_every_claude_account_builds_a_medium_item_on_sonnet(self):
        """Each Claude account runs Opus and Sonnet both: the router puts a medium item on its
        weakest seat that meets the tier, Sonnet, and keeps Opus for the strong roles."""
        for account in ("claude-1", "claude-2", "claude-3", "claude-4", "claude-5", "claude-6",
                        "claude-7", "claude-8"):
            with self.subTest(account=account):
                gh = FakeGitHub()
                ctx = ctx_for(gh, env=secrets(dict(CLAUDE_SECRETS)[account]), machine=())
                queue(gh, ctx, 3, MEDIUM)
                self.assertEqual(seats(plan_mod.make(ctx))["build"], (account, "sonnet", "medium"))
                gh = FakeGitHub()
                ctx = ctx_for(gh, env=secrets(dict(CLAUDE_SECRETS)[account]), machine=())
                queue(gh, ctx, 3, HARD)
                self.assertEqual(seats(plan_mod.make(ctx))["build"], (account, "opus", "strong"))

    def test_sonnet_only_while_claude_1_or_claude_3_is_open(self):
        # claude-3, claude-7 and claude-8 full (claude-3's two lanes, claude-7's one, claude-8's
        # three),
        # claude-1 open and the weak one full: claude-1 builds the easy item with Sonnet.
        gh = FakeGitHub()
        ctx = ctx_for(gh, machine=("lite",))
        busy(gh, ctx, ("claude-3", 50), ("claude-3", 59), ("claude-7", 72), ("claude-7", 73),
             ("claude-8", 74), ("claude-8", 75),
             *[("lite", n) for n in range(60, 66)])
        queue(gh, ctx, 3, EASY)
        self.assertEqual(seats(plan_mod.make(ctx))["build"], ("claude-1", "sonnet", "medium"))
        # Neither open and the medium models busy: the weak one builds it, the default, while
        # claude-2 (build_last, kept for planning and review) stays free.
        gh = FakeGitHub()
        ctx = ctx_for(gh, machine=ALL_MACHINE)
        busy(gh, ctx, ("claude-3", 50), ("claude-3", 63), ("claude-7", 72), ("claude-7", 73),
             ("claude-8", 74), ("claude-8", 75), ("claude-1", 51),
             ("claude-1", 64), ("claude-4", 52), ("claude-6", 71), ("claude-5", 70), ("agy", 53))
        ctx = make_ctx(gh, at=NIGHT, cfg=dataclasses.replace(
            ctx.cfg, pool=dataclasses.replace(ctx.cfg.pool, max_parallel=20, machine_parallel=20)))
        busy(gh, ctx, ("muse", 54), ("muse", 62), ("muse", 66), ("muse", 67),  # Muse has four lanes
             ("gpt", 55))
        queue(gh, ctx, 3, EASY)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["provider"], seats(planned)["build"][1]), ("lite", "lite-1"))
        # Only with all of the weak one's lanes taken does claude-2 build it, on its Sonnet seat:
        # its Opus stays for planning and review.
        gh.threads[3]["state"] = "open"
        gh.threads[3]["labels"] = [{"name": LABEL_BUILD}, {"name": EASY}]
        busy(gh, ctx, *[("lite", n) for n in range(56, 62)])
        self.assertEqual(seats(plan_mod.make(ctx))["build"], ("claude-2", "sonnet", "medium"))

    def test_claude_2_builds_a_medium_item_the_weak_one_may_not(self):
        """The weak one is weak, so a medium item passes it by: with only claude-2 and the weak one
        free, claude-2 builds it, on Sonnet."""
        gh = FakeGitHub()
        ctx = ctx_for(gh, machine=ALL_MACHINE)
        ctx = make_ctx(gh, at=NIGHT, cfg=dataclasses.replace(
            ctx.cfg, pool=dataclasses.replace(ctx.cfg.pool, max_parallel=20, machine_parallel=20)))
        busy(gh, ctx, ("claude-3", 50), ("claude-3", 57), ("claude-7", 72), ("claude-7", 73),
             ("claude-8", 74), ("claude-8", 75), ("claude-1", 51),
             ("claude-1", 58), ("claude-4", 52), ("claude-6", 71), ("claude-5", 70), ("agy", 53),
             ("muse", 54), ("muse", 56), ("muse", 60), ("muse", 61), ("gpt", 55))
        queue(gh, ctx, 3, MEDIUM)
        self.assertEqual(seats(plan_mod.make(ctx))["build"], ("claude-2", "sonnet", "medium"))

    def test_with_no_model_of_its_tier_free_it_steps_up_and_says_why(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY, machine=("agy",))  # by day: no Claude, and the weak one off
        queue(gh, ctx, 3, EASY)
        planned = plan_mod.make(ctx)
        self.assertEqual(seats(planned)["build"], ("agy", "gemini-3.8-flash-high", "medium"))
        self.assertEqual(planned["routing"], [
            "#3: stepped up from weak to medium to build it: no weak-tier model "
            "(lite-1) is free now"])
        self.assertIn("stepped up from weak to medium", planned["assignment"] + " ".join(
            planned["routing"]))
        # Medium with only a strong model free steps up to strong: claude-2 by day, whose Sonnet
        # is switched off for the test, so only its Opus is left.
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY, env=secrets("CLAUDE_CODE_OAUTH_TOKEN_2"), machine=(),
                      committed_hours=True)
        ctx = make_ctx(gh, at=DAY, cfg=dataclasses.replace(ctx.cfg, pool=dataclasses.replace(
            ctx.cfg.pool, providers={**ctx.cfg.pool.providers, "claude-2": dataclasses.replace(
                ctx.cfg.pool.get("claude-2"), extra_models=())})))
        queue(gh, ctx, 3)
        planned = plan_mod.make(ctx)
        self.assertEqual(seats(planned)["build"], ("claude-2", "opus", "strong"))
        self.assertIn("stepped up from medium to strong", planned["routing"][0])

    def test_a_persons_pull_request_is_revised_only_with_a_reviewer_in_the_run(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY, env=secrets(), machine=("lite",))
        gh.add_pull(9, "feature/someone", labels=("bot:revise", EASY))
        # Only the weak one is free, and it cannot review: a person's PR gets no review run, so it
        # waits.
        self.assertEqual(plan_mod.make(ctx)["action"], "none")
        # The bot's own pull request does get a review run, so the weak one may revise it.
        gh.add_pull(10, "bot/issue-1", labels=(LABEL_PR, "bot:revise", EASY))
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["number"], planned["provider"]), (10, "lite"))

    def test_revisions_follow_the_building_rule(self):
        # The weak one, weak and `easy_first`, revises an easy pull request first; without it,
        # the lane's weakest medium seat does: Muse (#695).
        for machine, seat in ((("lite", "muse"), ("lite", "lite-1", "weak")),
                              (MACHINE, ("muse", "muse-spark-1.3-contributor", "medium"))):
            gh = FakeGitHub()
            ctx = ctx_for(gh, machine=machine)
            gh.add_pull(9, "bot/issue-1", labels=(LABEL_PR, "bot:revise", EASY))
            planned = plan_mod.make(ctx)
            self.assertEqual((planned["action"], seats(planned)["build"]), ("revise", seat))
            self.assertIsNone(seats(planned)["plan"])  # a revision is not planned again


class PlannerTests(unittest.TestCase):
    def test_a_strong_planner_plans_on_the_planning_lane(self):
        """The lane's weakest planner meeting the floor plans it (#695): Muse plans an unrated
        item, while a hard one still takes a strong planner (claude-2, kept for planning)."""
        gh = FakeGitHub()
        ctx = make_ctx(gh, at=DAY, cfg=make_config(env=secrets("CLAUDE_CODE_OAUTH_TOKEN_2"),
                                                   machine=MACHINE, committed_hours=True,
                                                   plan_lanes=None))
        queue(gh, ctx, 3, planned=False)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], planned["provider"]), ("plan", "muse"))
        self.assertEqual(seats(planned), {"plan": ("muse", "muse-spark-1.3-contributor",
                                                   "medium"),
                                          "build": None, "review": None})
        self.assertIn("needs a plan; `muse`", planned["routing"][0])
        queue(gh, ctx, 4, HARD, planned=False)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], planned["provider"]), ("plan", "claude-2"))
        self.assertEqual(seats(planned)["plan"], ("claude-2", "opus", "strong"))
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

    def test_a_medium_model_plans_a_rated_medium_item_in_its_run(self):
        """A rated medium item is planned by a medium model (#695): with none free on the lane,
        a medium builder plans it in its own run, as it plans an easy one."""
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY)
        queue(gh, ctx, 3, MEDIUM, planned=False)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], seats(planned)["plan"][2]), ("build", "medium"))
        queue(gh, ctx, 4, EASY, planned=False)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["number"], seats(planned)["plan"][2]), (4, "medium"))

    def test_with_no_planner_free_nothing_is_built(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY, env=secrets(), machine=("lite",))
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
                         ("plan", "muse", "planned"))
        self.assertEqual([(c.role, c.model, c.read_only) for c in planner.calls],
                         [("plan", "muse-spark-1.3-contributor", True)])
        record = h.ctx.store.load()["items"]["12"]
        self.assertEqual(record["handoff"]["kind"], "plan")
        self.assertIn("Rules v2", record["handoff"]["notes"])
        self.assertTrue(record["planned_at"])
        self.assertEqual(record["planned_tier"], "medium")
        self.assertEqual(record["planned_by_id"], "muse")
        # Out of the Needs plan stage; no rating line in the plan, so it is labelled medium (#735).
        self.assertEqual(h.gh.label_names(12), {LABEL_BUILD, "bot:planned", "difficulty:medium"})
        self.assertIn("Planned on `muse` (muse, `muse-spark-1.3-contributor`, medium)",
                      h.gh.bot_comments(12)[-1])
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
        # One lane, one owner (#725): Muse keeps the medium item it planned. Its own
        # review approves, but the rule still waits for a Strong seat (`bot:cross-review`).
        self.assertEqual((planned["action"], planned["provider"], result["status"]),
                         ("build", "muse", "approved"))
        self.assertEqual([c.role for c in runner.calls], ["build", "review"])
        pulls = h.gh.list_pulls(head="bot/issue-12")
        self.assertEqual(len(pulls), 1)
        self.assertIn("bot:cross-review", h.gh.label_names(int(pulls[0]["number"])))
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
        # Room past the cap on GitHub's runners (`max_parallel`, #1455): the usage order is this
        # test's, the cap its own.
        ctx = make_ctx(gh, at=NIGHT, cfg=dataclasses.replace(
            ctx.cfg, pool=dataclasses.replace(ctx.cfg.pool, max_parallel=20)))
        self.pull(gh, ctx)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], seats(planned)["review"]),
                         ("review", ("claude-3", "opus", "strong")))
        busy(gh, ctx, ("claude-3", 50), ("claude-3", 57), ("claude-7", 72), ("claude-7", 73),
             ("claude-8", 74), ("claude-8", 75), ("claude-1", 51),
             ("claude-1", 58))
        gh.threads[9]["labels"] = [{"name": LABEL_PR}, {"name": LABEL_CROSS}]
        self.assertEqual(seats(plan_mod.make(ctx))["review"][0], "claude-4")

    def test_a_medium_item_waits_for_its_strong_final(self):
        """A medium item's final audit is Strong-only (#725): with no strong seat free, no
        review run goes out; with one free, Opus audits."""
        votes = {"sha": "h1", "builder": "muse", "approvals": ["muse"], "tiers": {"muse": "medium"}}
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY)  # no Claude account by day: nothing may review it
        self.pull(gh, ctx, votes=votes)
        self.assertEqual(plan_mod.make(ctx)["action"], "none")
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY, committed_hours=True)  # claude-3 all day
        self.pull(gh, ctx, votes=votes)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], seats(planned)["review"]),
                         ("review", ("claude-3", "opus", "strong")))
        self.assertEqual(planned["approved"], ["muse"])

    def test_a_hard_pull_request_waits_for_strong(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY, committed_hours=True)  # claude-3 all day
        self.pull(gh, ctx, labels=(HARD,))
        self.assertEqual(seats(plan_mod.make(ctx))["review"], ("claude-3", "opus", "strong"))
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY)  # no strong seat by day: a hard item waits for Opus
        self.pull(gh, ctx, labels=(HARD,))
        self.assertEqual(plan_mod.make(ctx)["action"], "none")

    def test_no_weak_review_runs_go_out(self):
        """No weak review runs go out at any difficulty (#725): weak builders review themselves
        in fresh sessions, and a weak approval never ships."""
        def review_with_the_weak_one_only(labels, votes=None):
            gh = FakeGitHub()
            ctx = ctx_for(gh, at=DAY, env=secrets(), machine=("lite",))
            self.pull(gh, ctx, labels=labels, votes=votes)
            return plan_mod.make(ctx)

        self.assertEqual(review_with_the_weak_one_only((EASY,))["action"], "none")
        self.assertEqual(review_with_the_weak_one_only(())["action"], "none")
        self.assertEqual(review_with_the_weak_one_only((HARD,))["action"], "none")
        # An easy item takes the weakest seat that ships it: medium, not weak.
        gh = FakeGitHub()
        ctx = ctx_for(gh, at=DAY, env=secrets(), machine=("lite", "agy"))
        self.pull(gh, ctx, labels=(EASY,))
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], seats(planned)["review"]),
                         ("review", ("agy", "gemini-3.8-flash-high", "medium")))

    def test_whether_the_reviewers_set_up_could_still_ship_it(self):
        from harness import review_rule
        # A strong reviewer ships anything; a medium one ships only an easy item (#725).
        # Anything weaker never ships, whatever is set up.
        self.assertTrue(review_rule.reachable([], {"strong"}, "hard"))
        self.assertTrue(review_rule.reachable(["medium"], {"strong"}, "easy"))
        self.assertFalse(review_rule.reachable([], {"medium"}, "hard"))
        self.assertFalse(review_rule.reachable([], {"medium"}, "medium"))
        self.assertTrue(review_rule.reachable([], {"medium"}, "easy"))
        self.assertTrue(review_rule.reachable(["medium"], {"weak"}, "easy"))  # already met
        self.assertFalse(review_rule.reachable(["medium"], {"weak"}, "medium"))
        self.assertFalse(review_rule.reachable([], {"weak"}, "easy"))  # weak alone never ships it
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

        strong, gemini, muse, lite = (("claude", "strong"), ("gemini", "medium"),
                                       ("muse", "medium"), ("lite", "weak"))
        for difficulty in ("medium", "hard"):
            # One strong approval ships; anything weaker, even two medium ones or a weak and
            # a medium one, does not (#725 keeps Strong-only for Medium/Hard finals).
            self.assertTrue(rule(reviews(strong), difficulty))
            self.assertFalse(rule(reviews(gemini, muse), difficulty))
            self.assertFalse(rule(reviews(muse, muse), difficulty))
            self.assertFalse(rule(reviews(gemini), difficulty))
            self.assertFalse(rule(reviews(lite, lite), difficulty))
            self.assertFalse(rule(reviews(lite, gemini), difficulty))
        # An easy item ships on its Medium final audit (#725); a weak approval never does.
        self.assertTrue(rule(reviews(strong), "easy"))
        self.assertTrue(rule(reviews(gemini), "easy"))
        self.assertTrue(rule(reviews(gemini, muse), "easy"))
        self.assertTrue(rule(reviews(muse, muse), "easy"))
        self.assertTrue(rule(reviews(lite, gemini), "easy"))
        self.assertFalse(rule(reviews(lite, lite), "easy"))
        self.assertFalse(rule(reviews(lite), "easy"))
        # Votes kept before reviews were counted one by one: one per family.
        self.assertFalse(rule({"approvals": ["gemini", "muse"],
                               "tiers": {"gemini": "medium", "muse": "medium"}}, "hard"))
        self.assertTrue(rule({"approvals": ["gemini", "lite"],
                              "tiers": {"gemini": "medium", "lite": "weak"}}, "easy"))
        self.assertFalse(rule({"approvals": ["gemini", "lite"],
                               "tiers": {"gemini": "medium", "lite": "weak"}}))
        # A rejection stands until its model approves.
        self.assertFalse(rule({"approvals": ["claude"], "rejections": ["gemini"],
                               "tiers": {"claude": "strong"}}))
        # A vote kept before tiers counts at its family's strongest tier.
        self.assertTrue(rule({"approvals": ["claude"]}))
        self.assertFalse(rule({"approvals": ["gpt"]}))


class SelfCheckTests(unittest.TestCase):
    """A builder whose seat has `self_check` (the weak one) checks its own change before any
    review."""

    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        self.origin, self.clone = make_origin(self.root)
        self.out = self.root / "out"

    def worker(self, runner, *, provider="lite", review=None, rounds=3, action="build"):
        raw = with_weak(raw_providers())
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
                         [("build", "lite-1", False), ("self_check", "lite-1", True)])
        self.assertIn("This is a self check, not a review", runner.calls[1].prompt)
        # The weak one has no seat that may review: the change goes to a review run, never approved.
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
        h = Harness(self, env=secrets(), machine=("lite",), at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD, EASY))
        h.ctx.store.update(lambda s: state_item(s, 12).update(planned_at=iso(DAY)))
        planned, result = h.night(FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                                              "self_check": reviewer(APPROVE)}))
        self.assertEqual((planned["provider"], result["status"]), ("lite", "built"))
        pull = h.gh.list_pulls(head="bot/issue-12")[0]
        pr = int(pull["number"])
        self.assertFalse(pull["draft"])
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_CROSS, EASY})
        self.assertEqual(h.gh.auto_merge, {})
        votes = h.ctx.store.load()["items"][str(pr)]["votes"]
        self.assertEqual((votes["builder"], votes["approvals"]), ("lite", []))
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
        self.assertIn("`lite` (codex: `lite-1` weak, self-check;", text)
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
        self.assertIn("lite-1 weak self-check", printed)
        self.assertIn("weak tier, tried in this order:", printed)
        self.assertIn("lite-1 (self-check)", printed)

    def test_peek_shows_the_difficulty_and_each_roles_tier(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh)
        queue(gh, ctx, 3, EASY, planned=False)
        look = plan_mod.peek(ctx)
        self.assertEqual(look.reason, "#3 (easy) is queued to build, for `muse`: plan on "
                         "`muse` (muse, `muse-spark-1.3-contributor`, medium); build on `muse` "
                         "(muse, `muse-spark-1.3-contributor`, medium); review on `muse` (muse, "
                         "`muse-spark-1.3-contributor`, medium)")


if __name__ == "__main__":
    unittest.main()
