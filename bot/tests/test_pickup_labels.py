"""Labels that steer pickup: the priority tiers (#90), and `human` and `shitter` (#96)."""

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
from tests.support import DAY, MACHINE, NIGHT, make_config, make_ctx
from tests.test_flow import Harness
from tests.test_work import APPROVE, builder, reviewer

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
        if planned["action"] not in ("build", "revise", "review"):
            return taken
        taken.append(planned["number"])
        gh.threads[planned["number"]]["state"] = "closed"


def every_model() -> Iterator[providers.Provider]:
    """Each committed subscription, and Astra and an unknown model in `gpt`'s place."""
    pool = make_config(env=ALL, machine=MACHINE).pool
    yield from pool.ordered()
    yield dataclasses.replace(pool.get("gpt"), model="gpt-5.6-astra")
    yield dataclasses.replace(pool.get("gpt"), model="mystery-model-9")


# ------------------------------------------------------------------ before these labels

def pairs_before(ctx, state, queue, lanes, *, force, quiet_ok):
    """`plan.pairs` as it was before the priority tiers and `shitter`, word for word."""
    order = sorted(queue, key=lambda c: (not (force or c.forced), not c.difficult,
                                         queue_mod.KIND_ORDER[c.kind], c.queued_at, c.number))
    for candidate in order:
        forced = force or candidate.forced
        for provider in ctx.cfg.pool.ordered():
            if candidate.difficult and not provider.difficult:
                continue
            if candidate.kind == "review" and provider.family == candidate.builder:
                continue
            if plan_mod._usable(ctx, state, provider, lanes, plan_mod.ROLE_OF[candidate.kind],
                                forced=forced, quiet_ok=quiet_ok):
                yield candidate, provider
                break


def a_mixed_queue(gh: FakeGitHub, ctx) -> None:
    """Forced, `difficult`, second reviews, revisions and builds of several ages, none of them
    with a priority, `human` or `shitter` label."""
    for number in (3, 4, 5, 6):
        gh.add_issue(number, labels=(LABEL_BUILD,))
    gh.add_issue(7, labels=(LABEL_BUILD, "difficult"))
    gh.add_issue(8, labels=(LABEL_BUILD, "an-unrelated-label", "priority:urgent"))
    gh.add_pull(9, "bot/issue-1", labels=(LABEL_PR, LABEL_REVISE))
    gh.add_pull(10, "bot/issue-2", labels=(LABEL_PR, LABEL_CROSS))
    gh.add_pull(11, "bot/issue-0", labels=(LABEL_PR, LABEL_CROSS))
    for number, at in ((3, "2026-09-25"), (4, "2026-09-21"), (5, "2026-09-23"), (6, "2026-09-23"),
                       (7, "2026-09-28"), (8, "2026-09-20"), (9, "2026-09-27"),
                       (10, "2026-09-26"), (11, "2026-09-24")):
        queued(ctx, number, f"{at}T00:00:00Z")
    queued(ctx, 6, "2026-09-23T00:00:00Z", forced=True)
    ctx.store.update(lambda s: state_item(s, 10).update(votes={"builder": "gpt"}))
    ctx.store.update(lambda s: state_item(s, 11).update(votes={"builder": "claude-opus"}))


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

    def test_with_no_priority_labels_anywhere_the_order_is_the_one_before(self):
        for at in (NIGHT, DAY):
            for force in (False, True):
                gh = FakeGitHub()
                ctx = ctx_for(gh, at=at)
                a_mixed_queue(gh, ctx)
                state = ctx.store.load()
                queue = queue_mod.candidates(ctx, state)
                lanes = plan_mod.Lanes(3)
                now = [(c.number, p.id) for c, p in plan_mod.pairs(
                    ctx, state, queue, lanes, force=force, quiet_ok=plan_mod.ANY_QUIET)]
                before = [(c.number, p.id) for c, p in pairs_before(
                    ctx, state, queue, lanes, force=force, quiet_ok=plan_mod.ANY_QUIET)]
                self.assertEqual(now, before, (at, force))
                self.assertEqual(len(now), 9 if at == NIGHT or force else 8)
        # The queue itself is listed in the same order as before.
        self.assertEqual(queue, sorted(queue, key=lambda c: (
            not c.forced, queue_mod.KIND_ORDER[c.kind], c.queued_at, c.number)))
        self.assertEqual([c.number for c in queue], [6, 11, 10, 9, 8, 4, 5, 3, 7])


class HumanAndShitterTests(unittest.TestCase):
    """#96: no model takes `human`; only a low-tier one takes `shitter`."""

    def test_the_tier_of_a_model(self):
        tier = providers.model_tier
        for name in ("opus", "OPUS", "claude-opus-4-1", "Claude Opus 5.5", "claude-opus",
                     "gpt-5.6-astra", "OpenAI Astra", "astra-2"):
            self.assertEqual(tier(name), providers.HIGH_TIER, name)
        for name in ("gpt-5.6-terra", "gemini-3.8-flash-high", "muse-spark-1.3-contributor",
                     "sonnet", "claude-haiku-4-5", "", "mystery-model-9", "astral",
                     "opuscule", "pastra"):
            self.assertEqual(tier(name), providers.LOW_TIER, name)

    def test_the_committed_subscriptions_tiers(self):
        pool = make_config(env=ALL, machine=MACHINE).pool
        self.assertEqual({p.id for p in pool.ordered()
                          if providers.model_tier(p.model) == providers.HIGH_TIER},
                         {"claude-1", "claude-2", "claude-3", "claude-4"})

    def test_human_is_never_selected_whatever_the_model(self):
        for provider in every_model():
            gh = FakeGitHub()
            ctx = ctx_for(gh, only=provider)
            gh.add_issue(5, labels=(LABEL_BUILD, "human", HIGH))
            gh.add_issue(6, labels=(LABEL_BUILD,))
            queued(ctx, 5, "2026-09-20T00:00:00Z", forced=True)
            planned = plan_mod.make(ctx, force=True)
            self.assertEqual(planned["number"], 6, provider.model)
            self.assertIn("#5 skipped: labelled `human`, so no model takes it", planned["skipped"][0])
            gh.threads[6]["state"] = "closed"
            planned = plan_mod.make(ctx, force=True, item=5)
            self.assertEqual(planned["action"], "none", provider.model)
            self.assertIn("#5 is not queued", planned["reason"])

    def test_a_human_pull_request_is_not_revised_either(self):
        gh = FakeGitHub()
        gh.add_pull(9, "bot/issue-1", labels=(LABEL_PR, LABEL_REVISE, "Human"))
        gh.add_issue(3, labels=(LABEL_BUILD,))
        ctx = ctx_for(gh)
        self.assertEqual(picks(ctx, gh), [3])
        self.assertIn("#9 skipped: labelled `human`", plan_mod.peek(ctx).skipped[0])

    def test_shitter_is_skipped_by_opus_and_astra(self):
        for provider in every_model():
            if providers.model_tier(provider.model) != providers.HIGH_TIER:
                continue
            gh = FakeGitHub()
            ctx = ctx_for(gh, only=provider)
            gh.add_issue(7, labels=(LABEL_BUILD, "shitter"))
            planned = plan_mod.make(ctx, force=True)
            self.assertNotEqual(planned.get("number"), 7, provider.model)
            self.assertEqual(planned["skipped"], [
                f"#7 skipped on `{provider.id}`: labelled `shitter`, and {provider.model} is "
                "high-tier"])
            look = plan_mod.peek(ctx, force=True, mode="build")
            self.assertFalse(look.work)
            self.assertEqual(look.skipped, planned["skipped"])

    def test_shitter_is_selectable_by_a_low_tier_model(self):
        for provider in every_model():
            if providers.model_tier(provider.model) != providers.LOW_TIER:
                continue
            gh = FakeGitHub()
            ctx = ctx_for(gh, only=provider)
            gh.add_issue(7, labels=(LABEL_BUILD, "Shitter"))
            planned = plan_mod.make(ctx, force=True)
            self.assertEqual((planned["number"], planned["provider"]), (7, provider.id),
                             provider.model)
            self.assertEqual(planned["skipped"], [])

    def test_an_unrecognised_model_is_low_tier(self):
        unknown = list(every_model())[-1]
        self.assertEqual(unknown.model, "mystery-model-9")
        self.assertEqual(providers.model_tier(unknown.model), providers.LOW_TIER)
        gh = FakeGitHub()
        gh.add_issue(7, labels=(LABEL_BUILD, "shitter"))
        self.assertEqual(plan_mod.make(ctx_for(gh, only=unknown), force=True)["number"], 7)

    def test_by_night_opus_passes_shitter_on_to_the_next_free_model(self):
        gh = FakeGitHub()
        gh.add_issue(7, labels=(LABEL_BUILD, "shitter"))
        planned = plan_mod.make(ctx_for(gh))
        self.assertEqual((planned["number"], planned["provider"]), (7, "gpt"))
        self.assertEqual([line.split(":")[0] for line in planned["skipped"]],
                         [f"#7 skipped on `claude-{n}`" for n in (1, 2, 3, 4)])

    def test_a_second_review_of_a_shitter_pull_request_goes_to_a_low_tier_model(self):
        gh = FakeGitHub()
        gh.add_pull(10, "bot/issue-2", labels=(LABEL_PR, LABEL_CROSS, "shitter"))
        ctx = ctx_for(gh)
        ctx.store.update(lambda s: state_item(s, 10).update(votes={"builder": "gpt"}))
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], planned["provider"]), ("review", "agy"))

    def test_human_and_shitter_together_are_never_selected(self):
        for provider in every_model():
            gh = FakeGitHub()
            ctx = ctx_for(gh, only=provider)
            gh.add_issue(5, labels=(LABEL_BUILD, "human", "shitter"))
            planned = plan_mod.make(ctx, force=True, item=5)
            self.assertEqual(planned["action"], "none", provider.model)
            self.assertEqual(len(planned["skipped"]), 1)
            self.assertIn("#5 skipped: labelled `human`", planned["skipped"][0])

    def test_difficult_and_shitter_together_have_no_model(self):
        gh = FakeGitHub()
        gh.add_issue(7, labels=(LABEL_BUILD, "difficult", "shitter"))
        planned = plan_mod.make(ctx_for(gh), force=True)
        self.assertNotEqual(planned.get("number"), 7)

    def test_with_neither_label_anywhere_selection_is_the_one_before(self):
        # The same mixed queue as the priority test, every pairing compared, by night and by
        # day, forced or not: no item is skipped and nothing moves.
        for at in (NIGHT, DAY):
            gh = FakeGitHub()
            ctx = ctx_for(gh, at=at)
            a_mixed_queue(gh, ctx)
            state = ctx.store.load()
            skipped: list[str] = []
            queue = queue_mod.candidates(ctx, state, skipped)
            now = list(plan_mod.pairs(ctx, state, queue, plan_mod.Lanes(3), force=False,
                                      quiet_ok="", skipped=skipped))
            before = list(pairs_before(ctx, state, queue, plan_mod.Lanes(3), force=False,
                                       quiet_ok=""))
            self.assertEqual([(c.number, p.id) for c, p in now],
                             [(c.number, p.id) for c, p in before])
            self.assertEqual(skipped, [])

    def test_the_reply_says_what_the_label_changes(self):
        gh = FakeGitHub()
        ctx = ctx_for(gh)
        gh.add_issue(5, labels=("human",))
        gh.add_issue(7, labels=("shitter",))
        gh.add_issue(8)
        self.assertIn("labelled `human`, though, so no model takes it",
                      queue_mod.queue_build(ctx, 5, by="MaxGoetzmann"))
        self.assertIn("labelled `shitter`, so only a low-tier model takes it",
                      queue_mod.queue_build(ctx, 7, by="MaxGoetzmann"))
        self.assertNotIn("labelled", queue_mod.queue_build(ctx, 8, by="MaxGoetzmann"))


class LabelsTests(unittest.TestCase):
    def test_setup_creates_the_new_labels(self):
        self.assertEqual(LABELS["human"][1], "A human will do this. Night bot skips it.")
        self.assertEqual(LABELS["shitter"][1],
                         "Low-tier models only (anything except OpenAI Astra or Claude Opus).")
        self.assertEqual({name: LABELS[name][0] for name in (HIGH, MEDIUM, LOW)},
                         {HIGH: "d73a4a", MEDIUM: "fbca04", LOW: "0e8a16"})
        gh = FakeGitHub()
        created = [name for name, (color, text) in LABELS.items()
                   if gh.ensure_label(name, color, text)]
        self.assertTrue({"human", "shitter", HIGH, MEDIUM, LOW} <= set(created))
        self.assertFalse(any(gh.ensure_label(name, color, text)
                             for name, (color, text) in LABELS.items()))

    def test_the_pull_request_keeps_the_issues_tier_and_shitter(self):
        h = Harness(self, env=ALL, machine=MACHINE)
        h.gh.add_issue(12, labels=(LABEL_BUILD, "shitter", "Priority:High"))
        planned, result = h.night(FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                                              "review": reviewer(APPROVE)}))
        self.assertEqual((planned["provider"], planned["priority"], result["status"]),
                         ("gpt", "high", "approved"))
        pr = int(h.gh.list_pulls(head="bot/issue-12")[0]["number"])
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_CROSS, "shitter", "Priority:High"})
        # Opus may not give the second review either.
        planned, _ = h.night(FakeRunner({"review": reviewer(APPROVE)}))
        self.assertEqual((planned["action"], planned["number"], planned["provider"]),
                         ("review", pr, "agy"))


if __name__ == "__main__":
    unittest.main()
