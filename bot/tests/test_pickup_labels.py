"""Labels that steer pickup: the priority tiers (#90), `human` (#96) and the difficulty labels."""

from __future__ import annotations

import dataclasses
import unittest
from typing import Any, Iterator

from harness import plan as plan_mod, providers
from harness import queue as queue_mod
from harness.config import LABEL_BUILD, LABEL_CROSS, LABEL_PR, LABEL_REVISE, LABELS
from harness.runner import FakeRunner
from harness.state import item as state_item

from tests.fakes import FakeGitHub
from tests.support import ALL_MACHINE, DAY, MACHINE, NIGHT, make_config, make_ctx
from tests.test_flow import Harness
from tests.test_work import APPROVE, builder, changes, reviewer

ALL = {"HARNESS_SECRETS_SET": " ".join(providers.SECRETS)}
HIGH, MEDIUM, LOW = "priority:high", "priority:medium", "priority:low"


def ctx_for(gh: FakeGitHub, at=NIGHT, only: providers.Provider | None = None):
    """Every subscription switched on with its login, or `only` that one."""
    cfg = make_config(env=ALL, machine=MACHINE)
    if only is not None:
        pool = dataclasses.replace(cfg.pool, priority=(only.id,),
                                   providers={**cfg.pool.providers, only.id: only})
        cfg = dataclasses.replace(cfg, pool=pool)
    return make_ctx(gh, at=at, cfg=cfg)


def queued(ctx, number: int, at: str, **fields: Any) -> None:
    ctx.store.update(lambda s: state_item(s, number).update(queued_at=at, **fields))


def picks(ctx, gh: FakeGitHub, **plan_args: Any) -> list[int]:
    """What successive runs claim, each run's item closed before the next one plans."""
    taken = []
    while True:
        planned = plan_mod.make(ctx, **plan_args)
        if planned["action"] not in ("plan", "build", "revise", "review"):
            return taken
        taken.append(planned["number"])
        gh.threads[planned["number"]]["state"] = "closed"


def every_model() -> Iterator[providers.Provider]:
    """Each committed subscription."""
    yield from make_config(env=ALL, machine=ALL_MACHINE).pool.ordered()


class PriorityTests(unittest.TestCase):
    """#90: `priority:high`, `priority:medium`, none, then `priority:low`."""

    def test_the_tier_of_a_set_of_labels(self):
        tier = queue_mod.priority_tier
        self.assertEqual(tier({HIGH}), 0)
        self.assertEqual(tier({MEDIUM, "bot:build"}), 1)
        self.assertEqual(tier(set()), 2)
        self.assertEqual(tier({LOW}), 3)
        # The highest label wins, names match whatever their case, and any other `priority:*`
        # label is no priority at all.
        self.assertEqual(tier({HIGH, LOW}), 0)
        self.assertEqual(tier({MEDIUM, LOW}), 1)
        self.assertEqual(tier({"Priority:HIGH"}), 0)
        self.assertEqual(tier({"priority:urgent", "priority: high", "priority:highest"}), 2)
        self.assertEqual(tier({"priority:urgent", LOW}), 3)

    def test_a_mix_of_tiers_is_picked_high_medium_none_low(self):
        gh = FakeGitHub()
        ctx = make_ctx(gh)
        # Oldest first, so without the labels they would go the other way round.
        for number, label in ((3, LOW), (4, None), (5, MEDIUM), (6, HIGH)):
            gh.add_issue(number, labels=(LABEL_BUILD,) + ((label,) if label else ()))
            queued(ctx, number, f"2026-09-2{number}T00:00:00Z")
        self.assertEqual(picks(ctx, gh), [6, 5, 4, 3])

    def test_the_chosen_items_tier_is_in_the_plan(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD, MEDIUM))
        gh.add_issue(4, labels=(LABEL_BUILD,))
        ctx = make_ctx(gh)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["number"], planned["priority"]), (3, "medium"))
        gh.threads[3]["state"] = "closed"
        self.assertEqual(plan_mod.make(ctx)["priority"], "none")

    def test_within_a_tier_the_order_is_the_one_before(self):
        gh = FakeGitHub()
        ctx = make_ctx(gh)
        gh.add_issue(3, labels=(LABEL_BUILD, HIGH))
        gh.add_issue(4, labels=(LABEL_BUILD, HIGH))
        gh.add_issue(5, labels=(LABEL_BUILD, HIGH))
        gh.add_pull(9, "bot/issue-1", labels=(LABEL_PR, LABEL_REVISE, HIGH))
        gh.add_issue(6, labels=(LABEL_BUILD,))
        for number, at in ((3, "2026-09-22"), (4, "2026-09-21"), (5, "2026-09-23"),
                           (9, "2026-09-25"), (6, "2026-09-20")):
            queued(ctx, number, f"{at}T00:00:00Z")
        # Revisions before builds, then the oldest build first, as before; the unlabelled one,
        # oldest of all, after every `priority:high` one.
        self.assertEqual(picks(ctx, gh), [9, 4, 3, 5, 6])

    def test_high_and_low_together_count_as_high(self):
        gh = FakeGitHub()
        ctx = make_ctx(gh)
        gh.add_issue(3, labels=(LABEL_BUILD, MEDIUM))
        gh.add_issue(4, labels=(LABEL_BUILD, LOW, HIGH))
        queued(ctx, 3, "2026-09-20T00:00:00Z")
        queued(ctx, 4, "2026-09-28T00:00:00Z")
        self.assertEqual(picks(ctx, gh), [4, 3])

    def test_a_forced_item_still_comes_first(self):
        gh = FakeGitHub()
        ctx = make_ctx(gh)
        gh.add_issue(3, labels=(LABEL_BUILD, HIGH))
        gh.add_issue(4, labels=(LABEL_BUILD, LOW))
        queued(ctx, 4, "2026-09-28T00:00:00Z", forced=True)
        self.assertEqual(picks(ctx, gh), [4, 3])

    def test_a_label_changed_between_runs_counts_at_the_next_pickup(self):
        gh = FakeGitHub()
        ctx = make_ctx(gh)
        for number in (3, 4, 5):
            gh.add_issue(number, labels=(LABEL_BUILD,))
            queued(ctx, number, f"2026-09-2{number}T00:00:00Z")
        self.assertEqual(plan_mod.make(ctx)["number"], 3)
        gh.threads[3]["state"] = "closed"
        gh.add_labels(5, [HIGH])
        self.assertEqual(plan_mod.make(ctx)["number"], 5)

    def test_with_no_priority_labels_the_order_is_forced_reviews_revisions_harder_builds(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh)
        for number in (3, 4, 5, 6):
            gh.add_issue(number, labels=(LABEL_BUILD,))
        gh.add_issue(7, labels=(LABEL_BUILD, "difficulty:hard"))
        gh.add_issue(8, labels=(LABEL_BUILD, "an-unrelated-label", "priority:urgent"))
        gh.add_pull(9, "bot/issue-1", labels=(LABEL_PR, LABEL_REVISE))
        gh.add_pull(10, "bot/issue-2", labels=(LABEL_PR, LABEL_CROSS))
        for number, at in ((3, "2026-09-25"), (4, "2026-09-21"), (5, "2026-09-23"),
                           (6, "2026-09-23"), (7, "2026-09-28"), (8, "2026-09-20"),
                           (9, "2026-09-27"), (10, "2026-09-26")):
            queued(ctx, number, f"{at}T00:00:00Z")
        queued(ctx, 6, "2026-09-23T00:00:00Z", forced=True)
        state = ctx.store.load()
        queue = queue_mod.candidates(ctx, state)
        order = [c.number for c, _ in plan_mod.pairs(ctx, state, queue, plan_mod.Lanes(3),
                                                     force=False, quiet_ok=plan_mod.ANY_QUIET)]
        # Forced first, then the review and the revision (work already begun), then the builds:
        # the hard one first (only the strongest can take it), the rest oldest first.
        self.assertEqual(order, [6, 10, 9, 7, 8, 4, 5, 3])


class HumanTests(unittest.TestCase):
    """#96: no model takes `human`, whatever its tier."""

    def test_human_is_never_selected_whatever_the_model(self):
        # devin-train takes only `training` items, so it takes nothing here by design.
        general = [p for p in every_model() if not p.only_labels]
        self.assertTrue(any(p.id == "devin-train" for p in every_model()))
        for provider in general:
            gh = FakeGitHub()
            ctx = ctx_for(gh, only=provider)
            gh.add_issue(5, labels=(LABEL_BUILD, "human", HIGH, "difficulty:easy"))
            gh.add_issue(6, labels=(LABEL_BUILD, "difficulty:easy"))
            ctx.store.update(lambda s: state_item(s, 6).update(planned_at="2026-09-20T00:00:00Z"))
            queued(ctx, 5, "2026-09-20T00:00:00Z", forced=True)
            planned = plan_mod.make(ctx, force=True)
            self.assertEqual(planned["number"], 6, provider.id)
            self.assertIn("#5 skipped: labelled `human`, so no model takes it", planned["skipped"][0])
            gh.threads[6]["state"] = "closed"
            planned = plan_mod.make(ctx, force=True, item=5)
            self.assertEqual(planned["action"], "none", provider.id)
            self.assertIn("#5 is not queued", planned["reason"])

    def test_a_human_pull_request_is_not_revised_either(self):
        gh = FakeGitHub()
        gh.add_pull(9, "bot/issue-1", labels=(LABEL_PR, LABEL_REVISE, "Human"))
        gh.add_issue(3, labels=(LABEL_BUILD,))
        ctx = ctx_for(gh)
        self.assertEqual(picks(ctx, gh), [3])
        self.assertIn("#9 skipped: labelled `human`", plan_mod.peek(ctx).skipped[0])

    def test_human_is_never_queued_planned_labelled_or_built(self):
        """`human`: people do it. A request to build it gets a reply and nothing else, and it is
        never a candidate, nor given a stage label, nor revised when it conflicts."""
        gh = FakeGitHub()
        ctx = make_ctx(gh, at=NIGHT, cfg=make_config(env=ALL, machine=MACHINE, plan_lanes=None))
        gh.add_issue(5, labels=(LABEL_BUILD, "Human", HIGH))
        gh.add_issue(6, labels=("human",))
        gh.add_issue(3, labels=(LABEL_BUILD,))
        reply = queue_mod.queue_build(ctx, 6, by="MaxGoetzmann")
        self.assertIn("#6 is labelled `human`: people do it, so I leave it alone", reply)
        self.assertEqual(gh.label_names(6), {"human"})
        planned = plan_mod.make(ctx, force=True)
        self.assertEqual(planned["number"], 3)
        self.assertIn("#5 skipped: labelled `human`", planned["skipped"][0])
        self.assertEqual(gh.label_names(5), {LABEL_BUILD, "Human", HIGH})  # no bot:needs-plan
        pull = gh.add_pull(9, "bot/issue-1", labels=(LABEL_PR, "human"))
        pull["mergeable_state"] = "dirty"
        plan_mod.housekeeping(ctx, ctx.store.load())
        self.assertNotIn(LABEL_REVISE, gh.label_names(9))
        self.assertIn("labelled `human`", queue_mod.queue_revise(ctx, 9, by="MaxGoetzmann"))

    def test_the_reply_says_what_the_label_changes(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh)
        gh.add_issue(5, labels=("human",))
        gh.add_issue(7, labels=("Difficulty:Hard",))
        gh.add_issue(8)
        self.assertIn("#5 is labelled `human`: people do it, so I leave it alone",
                      queue_mod.queue_build(ctx, 5, by="MaxGoetzmann"))
        self.assertIn("labelled `difficulty:hard`, so only Opus plans, builds and reviews it",
                      queue_mod.queue_build(ctx, 7, by="MaxGoetzmann"))
        self.assertNotIn("labelled", queue_mod.queue_build(ctx, 8, by="MaxGoetzmann"))


class LabelsTests(unittest.TestCase):
    def test_setup_creates_the_new_labels(self):
        self.assertEqual(LABELS["human"][1],
                         "People do this: the night bot never queues, plans, builds or labels it")
        self.assertNotIn("shitter", LABELS)
        self.assertNotIn("difficult", LABELS)
        self.assertEqual({name: LABELS[name][0] for name in (HIGH, MEDIUM, LOW)},
                         {HIGH: "d73a4a", MEDIUM: "fbca04", LOW: "0e8a16"})
        gh = FakeGitHub()
        created = [name for name, (color, text) in LABELS.items()
                   if gh.ensure_label(name, color, text)]
        self.assertTrue({"human", "difficulty:easy", "difficulty:medium", "difficulty:hard",
                         HIGH, MEDIUM, LOW} <= set(created))
        self.assertFalse(any(gh.ensure_label(name, color, text)
                             for name, (color, text) in LABELS.items()))

    def test_the_pull_request_keeps_the_issues_difficulty_and_tier(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD, "difficulty:easy", "Priority:High"))
        planned, result = h.night(FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                                              "review": reviewer(APPROVE)}))
        self.assertEqual((planned["provider"], planned["priority"], planned["difficulty"],
                          result["status"]), ("muse", "high", "easy", "approved"))
        pr = int(h.gh.list_pulls(head="bot/issue-12")[0]["number"])
        self.assertEqual(h.gh.label_names(pr),
                         {LABEL_PR, LABEL_CROSS, "difficulty:easy", "Priority:High"})
        self.assertEqual(h.ctx.store.load()["items"][str(pr)]["difficulty"], "easy")

    def test_a_draft_pull_request_keeps_them_too(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD, "difficulty:hard", LOW))
        planned, result = h.night(FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                                              "fix": builder({"src/game.txt": "v3\n"}),
                                              "review": reviewer(changes("Still wrong."))}),
                                  force=True)
        self.assertEqual((planned["provider"], result["status"]), ("claude-3", "not_approved"))
        pull = h.gh.list_pulls(head="bot/issue-12")[0]
        self.assertTrue(pull["draft"])
        self.assertEqual(h.gh.label_names(int(pull["number"])),
                         {LABEL_PR, "bot:blocked", "bot:stuck", "difficulty:hard", LOW})


if __name__ == "__main__":
    unittest.main()
