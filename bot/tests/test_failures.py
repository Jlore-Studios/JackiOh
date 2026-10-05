"""Why a run's rounds failed (bot/harness/failures.py), and the labels that go with it: `bot:stuck`
on a run that used every round, `bot:planned` on an item whose plan is in its description."""

from __future__ import annotations

import unittest

from harness import failures
from harness import plan as plan_mod
from harness.clock import iso
from harness.config import LABEL_NEEDS_PLAN, LABEL_PLANNED
from harness.state import item as state_item

from tests.fakes import FakeGitHub
from tests.support import NIGHT
from tests.test_difficulty import EASY, busy, queue
from tests.test_needs_plan import lane_ctx


def cycle(n, *, ok=True, error=None, red=(), review="blocked", claims=("It breaks replay.",)):
    gates = [{"name": "typecheck", "ok": "typecheck" not in red, "exit_code": 2 if "typecheck" in red
              else 0, "pre_existing": False, "skipped": ""},
             {"name": "unit", "ok": False, "exit_code": 1, "pre_existing": True, "skipped": ""}]
    if review == "unreadable":
        verdict = {"verdict": "unreadable", "readable": False, "findings": []}
    elif review == "blocked":
        verdict = {"verdict": "changes", "readable": True,
                   "findings": [{"severity": "blocking", "where": "src/a.ts", "claim": c}
                                for c in claims]}
    else:
        verdict = None
    entry = {"n": n, "builder": {"role": "build" if n == 1 else "fix", "ok": ok, "error": error,
                                 "minutes": 12.0, "model": "opus", "timed_out": False},
             "gates": gates}
    if verdict is not None:
        entry["review"] = verdict
    return entry


class WhyTests(unittest.TestCase):
    def test_every_round_used_says_why_round_by_round(self):
        cycles = [cycle(1, red=("typecheck",)), cycle(2), cycle(3, claims=("It breaks replay.",
                                                                         "R12 has no test."))]
        self.assertTrue(failures.stuck(cycles, 3))
        text = failures.why(cycles, 3, "no approval after 3 review cycles")
        self.assertIn("### Why it failed 3 times", text)
        self.assertIn("**The reviewer blocked 3 of 3 reviewed round(s)**", text)
        self.assertIn("**Checks were red in 1 round(s):** `typecheck` ×1", text)
        self.assertNotIn("`unit`", text.split("<details>")[0])  # red on main too: not this change's
        self.assertIn("`src/a.ts`: It breaks replay. (×3)", text)
        self.assertIn("| 1 | build on `opus`, 12.0 min | red: `typecheck` (exit 2) | blocked", text)
        self.assertIn("**Builder time:** 36 min over 3 round(s).", text)

    def test_a_run_that_stopped_sooner_says_its_real_count_and_reason(self):
        cycles = [cycle(1, ok=False, error="Individual quota reached.", review="unreadable")]
        self.assertFalse(failures.stuck(cycles, 10))
        text = failures.why(cycles, 10, "the reviewer's answer could not be read twice in a row")
        self.assertIn("### Why it stopped after 1 of 10 round(s)", text)
        self.assertIn("its answer could not be read in 1", text)
        self.assertIn("**The builder's session failed in 1 round(s)**, most often: Individual "
                      "quota reached.", text)
        self.assertIn("**review unreadable**", text)

    def test_nothing_recorded_still_reads(self):
        self.assertIn("### Why it stopped after 0 of 10 round(s)", failures.why(None, 10))
        self.assertFalse(failures.stuck([], 0))


class PlannedLabelTests(unittest.TestCase):
    def test_a_planned_item_carries_bot_planned_and_loses_it_if_it_needs_a_plan_again(self):
        gh = FakeGitHub()
        ctx = lane_ctx(gh)
        busy(gh, ctx, *[(p, 50 + i) for i, p in enumerate(
            ("claude-3", "claude-1", "claude-4", "claude-2"))])
        queue(gh, ctx, 3)                                      # planned (a strong model's, legacy)
        queue(gh, ctx, 4, "difficulty:medium")
        ctx.store.update(lambda s: state_item(s, 4).update(planned_at=iso(NIGHT),
                                                            planned_tier="medium"))
        gh.add_labels(4, [LABEL_PLANNED])                      # a medium item needing a strong plan
        plan_mod.make(ctx)
        self.assertIn(LABEL_PLANNED, gh.label_names(3))
        self.assertNotIn(LABEL_NEEDS_PLAN, gh.label_names(3))
        self.assertIn(LABEL_NEEDS_PLAN, gh.label_names(4))
        self.assertNotIn(LABEL_PLANNED, gh.label_names(4))


if __name__ == "__main__":
    unittest.main()
