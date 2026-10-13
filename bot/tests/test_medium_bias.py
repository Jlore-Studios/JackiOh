"""#695: classification biases toward medium, while weak-tier handling is unchanged.

Borderline work (close to the easy limits, or a rating whose `why` admits doubt) now
rates `difficulty:medium`; clear-easy work still rates easy. The weak tier still builds
only easy items (`MIN_TIER`, `easy_first`) and the review rule still counts a weak
approval only toward an easy item (`review_rule.py`, untouched).
"""

from __future__ import annotations

import dataclasses
import unittest

from harness import easy as easy_mod
from harness import review_rule
from harness.config import MIN_TIER

from tests.support import make_config
from tests.test_rating import EASY, MEDIUM, day_harness, planner, rated_plan


def table(files) -> str:
    rows = "\n".join(f"| `{path}` | change | the change |" for path in files)
    return ("## 2. Files to touch\n\n| Path | new/change | What |\n|---|---|---|\n"
            f"{rows}\n")


class BorderlineBreachTests(unittest.TestCase):
    def cfg(self):
        return make_config()

    def test_a_small_plan_and_change_hold(self):
        cfg = self.cfg()
        self.assertEqual(easy_mod.plan_breach(table(("src/game.txt",)), cfg.easy,
                                              cfg.review_paths, cfg.forbidden_paths), "")
        self.assertEqual(easy_mod.change_breach({"src/game.txt": 10}, cfg.easy), "")

    def test_a_plan_close_to_the_file_limit_is_medium(self):
        cfg = self.cfg()
        breach = easy_mod.plan_breach(table(tuple(f"src/f{n}.txt" for n in range(9))),
                                      cfg.easy, cfg.review_paths, cfg.forbidden_paths)
        self.assertIn("close to the limit of 10", breach)
        self.assertIn("borderline easy is medium", breach)
        # Room to spare stays easy, and going over keeps its old message.
        self.assertEqual(easy_mod.plan_breach(table(tuple(f"src/f{n}.txt" for n in range(8))),
                                              cfg.easy, cfg.review_paths,
                                              cfg.forbidden_paths), "")
        self.assertIn("over 10", easy_mod.plan_breach(
            table(tuple(f"src/f{n}.txt" for n in range(11))), cfg.easy,
            cfg.review_paths, cfg.forbidden_paths))

    def test_a_change_close_to_a_limit_is_medium(self):
        cfg = self.cfg()
        self.assertIn("borderline easy is medium",
                      easy_mod.change_breach({f"src/f{n}.txt": 1 for n in range(9)},
                                             cfg.easy))
        self.assertIn("close to the limit of 400",
                      easy_mod.change_breach({"src/big.txt": 350}, cfg.easy))
        self.assertIn("over 10", easy_mod.change_breach(
            {f"src/f{n}.txt": 1 for n in range(11)}, cfg.easy))

    def test_tiny_limits_have_no_borderline_zone(self):
        """A `max_files: 1` lane (fullsend parts) still builds a one-file change as easy."""
        cfg = self.cfg()
        tiny = dataclasses.replace(cfg.easy, max_files=1)
        self.assertEqual(easy_mod.plan_breach(table(("src/client.txt",)), tiny,
                                              cfg.review_paths, cfg.forbidden_paths), "")
        self.assertEqual(easy_mod.change_breach({"src/client.txt": 1}, tiny), "")

    def test_an_unsure_why_is_doubt_and_a_confident_one_is_not(self):
        for why in ("torn between easy and medium",
                    "borderline easy, but the files fit",
                    "not sure about the shared surface",
                    "close to the limit on files",
                    "could go either way"):
            self.assertTrue(easy_mod.uncertain_why(why), why)
        for why in ("one file, one test", "a single copy-paste", ""):
            self.assertFalse(easy_mod.uncertain_why(why), why)


class RatingBiasTests(unittest.TestCase):
    def test_a_clear_easy_item_still_rates_easy(self):
        h = day_harness(self)
        h.gh.add_issue(12, labels=("bot:build",), body="Make the rules v2.")
        runner = planner(rated_plan("easy"))
        h.night(runner)
        self.assertIn(EASY, h.gh.label_names(12))
        self.assertIn("Rated `difficulty:easy`", h.gh.bot_comments(12)[-1])

    def test_a_plan_close_to_the_limit_rates_medium(self):
        h = day_harness(self)
        h.gh.add_issue(12, labels=("bot:build",), body="Make the rules v2.")
        h.night(planner(rated_plan("easy", files=[f"src/f{n}.txt" for n in range(9)])))
        self.assertIn(MEDIUM, h.gh.label_names(12))
        self.assertIn("borderline easy is medium", h.gh.bot_comments(12)[-1])

    def test_an_unsure_rating_rates_medium(self):
        h = day_harness(self)
        h.gh.add_issue(12, labels=("bot:build",), body="Make the rules v2.")
        h.night(planner(rated_plan("easy", why="torn between easy and medium")))
        self.assertIn(MEDIUM, h.gh.label_names(12))
        self.assertIn("was unsure", h.gh.bot_comments(12)[-1])

    def test_the_planner_is_told_to_prefer_medium_when_in_doubt(self):
        h = day_harness(self)
        h.gh.add_issue(12, labels=("bot:build",), body="Make the rules v2.")
        runner = planner(rated_plan("easy"))
        h.night(runner)
        self.assertIn("When in doubt", runner.calls[0].prompt)
        self.assertIn("At most 10 files changed", runner.calls[0].prompt)

    def test_the_rule_keeps_the_tiebreaker_and_names_borderline(self):
        self.assertIn("choose the higher", easy_mod.RULE)
        self.assertIn("borderline easy is **medium**", easy_mod.RULE)
        self.assertIn("room to spare", easy_mod.RULE)


class WeakTierUntouchedTests(unittest.TestCase):
    def test_weak_builds_only_easy(self):
        self.assertEqual(MIN_TIER, {"easy": "weak", "medium": "medium", "hard": "strong"})

    def test_a_weak_approval_never_ships(self):
        # A weak approval is a finding round, not a vote (#725): no weak review runs go out,
        # and it never ships anything at any difficulty.
        self.assertTrue(review_rule.met(["weak", "medium"], "easy"))  # the medium ships it
        self.assertFalse(review_rule.met(["weak", "medium"], "medium"))
        self.assertFalse(review_rule.met(["weak", "medium"], "hard"))
        self.assertFalse(review_rule.helps([], "weak", "easy"))
        self.assertFalse(review_rule.helps([], "weak", "medium"))
        self.assertFalse(review_rule.reachable([], {"weak"}, "easy"))
        self.assertTrue(review_rule.reachable(["medium"], {"weak"}, "easy"))  # already met
        # Only a Strong approval ships medium and hard; a medium one ships easy.
        self.assertTrue(review_rule.met(["strong"], "hard"))
        self.assertFalse(review_rule.met(["medium", "medium"], "hard"))
        self.assertFalse(review_rule.met(["weak"], "easy"))
        self.assertTrue(review_rule.met(["medium"], "easy"))


if __name__ == "__main__":
    unittest.main()
