"""#317 part 1: a model call that fails in seconds is the subscription's failure, not the item's.

Devin's CLI failed every call in seconds from 2026-10-05 07:30Z. Each failed builder call was
committed as a pass, checked, self-checked and delivered as "Revision done", and each failed review
call was read as an unreadable answer, so the items were charged and blocked while the broken
subscription kept getting work.
"""

from __future__ import annotations

import unittest

from harness.config import LABEL_BUILD, LABEL_CROSS, LABEL_PR
from harness.runner import FakeRunner, RunResult, stderr_error

from tests.support import DAY, MACHINE
from tests import test_cross
from tests.test_cross import ALL
from tests.test_flow import Harness
from tests.test_work import APPROVE, builder, reviewer


def failing(seconds: float, error: str = "boom", text: str = ""):
    def handler(request):
        return RunResult(False, text, 1, duration_s=seconds, error=error)
    return handler


class InstantFailureTests(unittest.TestCase):
    def test_a_call_that_fails_in_seconds_backs_the_subscription_off_and_spares_the_item(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned, result = h.night(FakeRunner({"plan": failing(2, "}")}))
        provider = planned["provider"]
        self.assertEqual(result["status"], "infra")
        self.assertIn("CLI failed at once: }", result["reason"])
        state = h.ctx.store.load()
        self.assertEqual(state["providers"][provider]["infra"]["streak"], 1)
        self.assertEqual(int(state["items"]["12"].get("failures", 0)), 0)
        self.assertEqual(h.gh.label_names(12) & {LABEL_BUILD}, {LABEL_BUILD})
        self.assertIn("That is not this item's fault", h.gh.bot_comments(12)[-1])
        # The subscription is left alone: the next run goes elsewhere.
        planned, _ = h.night(FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                                         "review": reviewer(APPROVE)}))
        self.assertNotEqual(planned["provider"], provider)

    def test_three_in_a_row_ask_a_person(self):
        h = Harness(self, env={"HARNESS_SECRETS_SET": ""}, machine=("muse",), at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        for _ in range(3):
            h.night(FakeRunner({"plan": failing(3, "Error: model unavailable")}), force=True)
        issues = [t for t in h.gh.threads.values()
                  if "could not work 3 times in a row" in str(t.get("title"))]
        self.assertEqual(len(issues), 1)
        self.assertIn("model unavailable", issues[0]["body"])

    def test_a_builder_that_fails_after_working_a_while_but_changed_nothing_fails_the_run(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        _, result = h.night(FakeRunner({"build": failing(600, "the session crashed")}))
        self.assertEqual(result["status"], "failed")
        self.assertIn("the builder's session failed and changed nothing: the session crashed",
                      result["reason"])
        self.assertEqual(result["cycles"][0]["builder"]["changed"], False)
        self.assertIsNone(h.origin_sha("bot/issue-12"))  # no "pass 1" commit, nothing pushed
        self.assertEqual(int(h.ctx.store.load()["items"]["12"]["failures"]), 1)

    def test_a_builder_that_failed_after_doing_work_keeps_it(self):
        """Claude's turn limit ends a long session `ok: false`: its work is still a pass."""
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        inner = builder({"src/game.txt": "v2\n"})

        def turn_limit(request):
            done = inner(request)
            return RunResult(False, done.text, 1, duration_s=5400, error="error_max_turns")
        _, result = h.night(FakeRunner({"build": turn_limit, "review": reviewer(APPROVE)}))
        self.assertEqual(result["status"], "approved")
        self.assertEqual(result["cycles"][0]["builder"]["changed"], True)

    def test_a_review_call_that_fails_names_its_error(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        pr, _ = test_cross.ReviewRuleTests.build_on_medium(self, h)
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_CROSS})
        _, review = h.night(FakeRunner({"review": failing(900, "overloaded")}))
        self.assertEqual(review["status"], "failed")
        self.assertEqual(review["reason"], "the reviewer's call failed: overloaded")
        self.assertIn("the reviewer's call failed: overloaded", h.gh.bot_comments(pr)[-1])
        self.assertNotIn("could not be read", h.gh.bot_comments(pr)[-1])


class StderrTests(unittest.TestCase):
    def test_a_json_error_body_gives_its_message(self):
        stderr = ("Welcome\n{\n  \"error\": {\n    \"type\": \"rate\",\n"
                  "    \"message\": \"Your daily allowance is spent\"\n  }\n}\n")
        self.assertEqual(stderr_error(stderr, "x"), "Your daily allowance is spent")

    def test_otherwise_the_last_lines(self):
        stderr = "\n".join(f"line {n}" for n in range(30))
        found = stderr_error(stderr, "x")
        self.assertTrue(found.startswith("line 10\n") and found.endswith("line 29"))
        self.assertEqual(stderr_error("  \n", "devin exited 1"), "devin exited 1")


if __name__ == "__main__":
    unittest.main()
