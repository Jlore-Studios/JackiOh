"""The difficulty lanes: each role goes to the lane's weakest seat that meets its tier, and
the builder is never the planner (#695)."""

from __future__ import annotations

import unittest

from harness import plan as plan_mod

from tests.support import make_config


def who(pool, providers, difficulty, **kwargs):
    seat, note = plan_mod.builder_seat(pool, providers, difficulty, **kwargs)
    return ((seat.provider.id, seat.tier), note) if seat else (None, note)


class LaneTests(unittest.TestCase):
    def test_a_medium_build_stays_with_its_planner(self):
        """One lane, one owner (#725): a medium item stays with the subscription that planned
        it when that one can build it."""
        pool = make_config().pool
        claude = pool.get("claude-3")
        muse, agy = pool.get("muse"), pool.get("agy")
        seat, note = plan_mod.builder_seat(pool, [claude, muse, agy], "medium",
                                           avoid="muse")
        self.assertEqual(seat.provider.id, "muse")
        self.assertEqual(seat.tier, "medium")
        self.assertIn("one lane, one owner", note)

    def test_a_build_keeps_its_planner_with_a_note(self):
        pool = make_config().pool
        muse = pool.get("muse")
        seat, note = plan_mod.builder_seat(pool, [muse], "medium", avoid="muse")
        self.assertEqual(seat.provider.id, "muse")
        self.assertIn("its planner `muse` keeps it", note)

    def test_a_busy_planner_loses_it_to_the_lane(self):
        """The owner keeps it only when it can build it (#725): with the planner not free,
        the lane's tier order picks."""
        pool = make_config().pool
        claude = pool.get("claude-3")
        muse, agy = pool.get("muse"), pool.get("agy")
        seat, _note = plan_mod.builder_seat(pool, [claude, agy], "medium",
                                            avoid="muse")
        self.assertEqual(seat.provider.id, "agy")
        self.assertEqual(seat.tier, "medium")

    def test_no_planner_means_no_avoidance(self):
        pool = make_config().pool
        claude, muse = pool.get("claude-3"), pool.get("muse")
        seat, note = plan_mod.builder_seat(pool, [claude, muse], "medium")
        self.assertEqual((seat.provider.id, seat.tier), ("muse", "medium"))
        self.assertEqual(note, "")

    def test_an_easy_build_still_avoids_its_planner(self):
        """An easy item goes weak-first even when its medium planner is free (#725): the
        planner must not spend its lane on weak work."""
        pool = make_config(machine=("muse", "devin")).pool
        muse, devin = pool.get("muse"), pool.get("devin")
        seat, _note = plan_mod.builder_seat(pool, [muse, devin], "easy", avoid="muse")
        self.assertEqual(seat.provider.id, "devin")

    def test_a_revision_answering_an_audit_stays_at_its_tier(self):
        """A revision that answers a final audit stays at the auditor's tier or above (#725),
        so the auditor fixes and pushes it."""
        pool = make_config().pool
        claude = pool.get("claude-3")
        muse, agy = pool.get("muse"), pool.get("agy")
        seat, _note = plan_mod.builder_seat(pool, [muse, agy, claude], "medium",
                                            audit_tier="strong")
        self.assertEqual((seat.provider.id, seat.tier), ("claude-3", "strong"))
        seat, _note = plan_mod.builder_seat(pool, [muse, agy], "easy", audit_tier="medium")
        self.assertEqual(seat.tier, "medium")

    def test_an_easy_build_tries_gpt_before_devin(self):
        pool = make_config(machine=("muse", "devin")).pool
        gpt, devin = pool.get("gpt"), pool.get("devin")
        seat, _note = plan_mod.builder_seat(pool, [devin, gpt], "easy")
        self.assertEqual(seat.provider.id, "gpt")

    def test_a_hard_build_takes_a_strong_seat(self):
        pool = make_config().pool
        claude, muse = pool.get("claude-3"), pool.get("muse")
        seat, _note = plan_mod.builder_seat(pool, [muse, claude], "hard")
        self.assertEqual((seat.provider.id, seat.tier), ("claude-3", "strong"))


if __name__ == "__main__":
    unittest.main()
