"""Muse fills gpt's and agy's lanes on the night box while they cannot start work (`fill_in` in
providers.json): Muse's own two lanes are on the Muse box, and each box runs two jobs at most."""

from __future__ import annotations

import copy
import unittest

from harness import controller, memory, plan as plan_mod, providers, waiting
from harness.config import LABEL_BUILD, LABEL_WORKING
from harness.errors import ConfigError
from harness.state import item as state_item

from tests.fakes import FakeGitHub
from tests.support import DAY, MACHINE, make_config, make_ctx, raw_providers

FILL = "night-vm-muse-fill"


def ctx_for(gh, machine=MACHINE):
    return make_ctx(gh, at=DAY, cfg=make_config(
        env={"HARNESS_SECRETS_SET": " ".join(providers.SECRETS)}, machine=machine))


def refused(ctx, provider_id):
    """`provider_id` refused a call a minute ago and waits two hours: it cannot start work."""
    ctx.store.update(lambda s: providers.note_usage(s, provider_id, None, "+PT2H", DAY))


def take_all(gh, ctx, limit=8):
    """Plan until nothing more starts; each claim's run stays alive. (number, provider, runs_on)
    for each, and the last plan's reason."""
    taken, planned = [], {}
    for _ in range(limit):
        planned = plan_mod.make(ctx)
        if planned["action"] == "none":
            break
        gh.runs[str(planned["number"])] = {"status": "in_progress"}
        ctx.store.update(lambda s, n=planned["number"]: state_item(s, n).update(run_id=str(n)))
        taken.append((planned["number"], planned["provider"], planned["runs_on"]))
    return taken, planned.get("reason", "")


class FillInTests(unittest.TestCase):
    def setUp(self):
        self.gh = FakeGitHub()
        for n in (3, 4, 5, 6, 7):
            self.gh.add_issue(n, labels=(LABEL_BUILD,))

    def test_with_gpt_and_agy_free_each_keeps_its_lane(self):
        ctx = ctx_for(self.gh)
        taken, reason = take_all(self.gh, ctx)
        self.assertEqual(taken, [(3, "muse", "night-vm-muse"), (4, "muse", "night-vm-muse"),
                                 (5, "agy", "night-vm-agy"), (6, "gpt", "night-vm-gpt")])
        self.assertIn("`muse` is busy", reason)

    def test_muse_fills_the_lane_of_one_that_cannot_start(self):
        ctx = ctx_for(self.gh)
        refused(ctx, "gpt")
        taken, _ = take_all(self.gh, ctx)
        # Muse, first in the tier order, takes gpt's lane before agy takes its own.
        self.assertEqual(taken, [(3, "muse", "night-vm-muse"), (4, "muse", "night-vm-muse"),
                                 (5, "muse", FILL), (6, "agy", "night-vm-agy")])
        state = ctx.store.load()
        self.assertEqual(state["items"]["5"]["runs_on"], FILL)
        lanes = plan_mod.read_lanes(ctx, state)
        self.assertEqual((lanes.filling, lanes.count("muse")), ({5}, 3))
        self.assertTrue(lanes.full(ctx.cfg.pool.get("muse")))
        # gpt's reset comes while Muse holds its lane: gpt waits for it, so the night box never
        # runs more than two jobs.
        ctx.store.update(lambda s: s["providers"]["gpt"].pop("refused_until"))
        again = plan_mod.make(ctx)
        self.assertEqual(again["action"], "none")
        self.assertIn("`gpt` waits for its lane on its box, which `muse` fills now",
                      again["reason"])
        self.assertTrue(plan_mod.fill_in_full(ctx.cfg.pool, ctx.cfg.pool.get("gpt"),
                                              plan_mod.read_lanes(ctx, ctx.store.load())))
        explained = waiting.explain(ctx, ctx.store.load(), {
            n: p for n, p, _ in taken})
        self.assertEqual(explained.now["gpt"].kind, "full")
        self.assertIn("`muse` fills it", explained.now["gpt"].text)
        # Once Muse's run there ends, the lane is gpt's again.
        self.gh.runs["5"] = {"status": "completed"}
        self.gh.add_issue(8, labels=(LABEL_BUILD,))
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["provider"], planned["runs_on"]), ("gpt", "night-vm-gpt"))

    def test_with_both_off_muse_takes_both_lanes_and_no_more(self):
        ctx = ctx_for(self.gh, machine=("muse",))
        taken, reason = take_all(self.gh, ctx)
        self.assertEqual([r for _, _, r in taken],
                         ["night-vm-muse", "night-vm-muse", FILL, FILL])
        self.assertIn("`muse` is busy", reason)

    def test_a_planning_run_of_gpt_or_agy_holds_a_shared_lane(self):
        ctx = ctx_for(self.gh)
        pool = ctx.cfg.pool
        lanes = plan_mod.Lanes(8, machine_limit=6, machine=frozenset(MACHINE))
        lanes = lanes.with_run(40, "gpt", "plan").with_run(41, "muse", "build", True)
        self.assertTrue(plan_mod.fill_in_full(pool, pool.get("agy"), lanes))
        self.assertFalse(plan_mod.fill_in_full(pool, pool.get("muse"), lanes))
        self.assertEqual((lanes.filling_by("muse"), lanes.all_by("gpt")), (1, 1))
        # Muse's fill-in takes none of its own lanes.
        self.assertFalse(lanes.full(pool.get("muse")))

    def test_the_survey_never_fills_a_lane(self):
        ctx = ctx_for(self.gh, machine=("muse",))
        lanes = plan_mod.Lanes(8, {40: "muse", 41: "muse"}, machine_limit=6,
                               machine=frozenset(MACHINE))
        state = ctx.store.load()
        muse = ctx.cfg.pool.get("muse")
        self.assertEqual(plan_mod.runner_for(ctx, state, muse, lanes, "build"), FILL)
        self.assertEqual(plan_mod.runner_for(ctx, state, muse, lanes, "suggest"), "")


class CheckTests(unittest.TestCase):
    def test_a_dispatch_may_name_the_fill_in_runners_of_its_own_subscription_only(self):
        ctx = ctx_for(FakeGitHub())
        given = {"cli": "muse", "secret": "", "shared": "false"}
        planned = {"provider": "muse", "runs_on": FILL, "cli": "muse"}
        self.assertEqual(controller.check(ctx, planned, {**given, "runs_on": FILL}), [])
        self.assertTrue(controller.check(ctx, planned, {**given, "runs_on": "night-vm-muse"}))
        gpt = {"provider": "gpt", "runs_on": FILL, "cli": "codex"}
        self.assertTrue(controller.check(ctx, gpt, {**given, "cli": "codex", "runs_on": FILL}))


class ParseTests(unittest.TestCase):
    def raw(self, **fill):
        raw = copy.deepcopy(raw_providers())
        raw["providers"]["muse"]["fill_in"] = fill
        return raw

    def test_the_committed_fill_in(self):
        pool = providers.load(providers.Path(__file__).resolve().parents[2])
        muse = pool.get("muse")
        self.assertEqual((muse.runs_on, muse.lanes, muse.fill_in),
                         ("night-vm-muse", 2, providers.FillIn(FILL, ("gpt", "agy"))))
        self.assertEqual((pool.filler_of("gpt").id, pool.filler_of("agy").id),
                         ("muse", "muse"))
        self.assertIsNone(pool.filler_of("devin"))

    def test_it_refuses_a_bad_fill_in(self):
        for fill, message in (({"runs_on": FILL}, "needs runs_on and for"),
                              ({"runs_on": FILL, "for": ["nobody"]}, "not another subscription"),
                              ({"runs_on": FILL, "for": ["muse"]}, "not another subscription"),
                              ({"runs_on": "night-vm-gpt", "for": ["gpt"]}, "runners of its own"),
                              ({"runs_on": "ubuntu-latest", "for": ["gpt"]}, "on the machine"),
                              ({"runs_on": FILL, "for": ["gpt"], "lanes": 2}, "unknown keys")):
            with self.assertRaises(ConfigError, msg=fill) as caught:
                providers.parse(self.raw(**fill))
            self.assertIn(message, str(caught.exception))

    def test_one_moved_to_githubs_runners_has_no_lane_to_lend(self):
        raw = copy.deepcopy(raw_providers())
        raw["providers"]["gpt"].update(login="secret", secret="CODEX_AUTH_JSON",
                                       runs_on="ubuntu-latest")
        pool = providers.parse(raw)
        self.assertEqual(pool.filled_by(pool.get("muse")), ["agy"])
        self.assertIsNone(pool.filler_of("gpt"))


class BoxTests(unittest.TestCase):
    def test_a_fill_in_runner_is_on_the_night_box(self):
        self.assertEqual(memory.box_of("night-vm-muse-fill-2", FILL), "")
        self.assertEqual(memory.box_of("night-vm-muse-2-b2", "night-vm-muse"), "b2")


if __name__ == "__main__":
    unittest.main()
