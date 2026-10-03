"""Several subscriptions end to end: the review rule (one strong approval, or two medium ones of
different families), review runs, `difficulty:hard`, handoff and the vault, through plan, work
and deliver against a real repository and the fake GitHub."""

from __future__ import annotations

import json
import unittest

from harness import plan as plan_mod
from harness import providers, vault
from harness.config import LABEL_BLOCKED, LABEL_BUILD, LABEL_CROSS, LABEL_PR, LABEL_REVISE
from harness.deliver import Deliverer
from harness.runner import FakeRunner, RunResult
from harness.work import NOTES_FILE

from tests.fakes import git, push_branch
from tests.support import DAY, MACHINE
from tests.test_flow import Harness
from tests.test_work import APPROVE, builder, changes, reviewer

ALL = {"HARNESS_SECRETS_SET": " ".join(providers.SECRETS)}


def timed_builder(edits, minutes=10.0):
    inner = builder(edits)
    def handler(request):
        result = inner(request)
        result.duration_s = minutes * 60
        return result
    return handler


class ReviewRuleTests(unittest.TestCase):
    def build_on_agy(self, h):
        """By day the Claude accounts are closed: agy, first of the medium models, plans, builds
        and reviews it in one run, and its own approval is one medium vote."""
        h.gh.add_issue(12, "Make the rules v2", labels=(LABEL_BUILD,))
        runner = FakeRunner({"build": timed_builder({"src/game.txt": "rules v2\n"}),
                             "review": reviewer(APPROVE)})
        planned, result = h.night(runner)
        self.assertEqual((planned["provider"], result["status"]), ("agy", "approved"))
        self.assertEqual([c.role for c in runner.calls], ["plan", "build", "review"])
        self.assertEqual({c.model for c in runner.calls}, {"gemini-3.8-flash-high"})
        pr = int(h.gh.list_pulls(head="bot/issue-12")[0]["number"])
        return pr, result

    def test_two_medium_reviews_of_different_families_merge_it(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        pr, result = self.build_on_agy(h)
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_CROSS})
        self.assertEqual(h.gh.auto_merge, {})
        votes = h.ctx.store.load()["items"][str(pr)]["votes"]
        self.assertEqual(votes, {"sha": result["head"], "builder": "gemini",
                                 "approvals": ["gemini"], "rejections": [],
                                 "tiers": {"gemini": "medium"}})
        self.assertIn("`gemini` (medium) approved it, so it waits for one strong model's review "
                      "(Opus), or a medium one from a family that has not approved it yet",
                      h.gh.bot_comments(12)[-1])
        self.assertEqual(h.ctx.store.load()["providers"]["agy"]["spent"][-1]["minutes"], 10.0)
        # No strong model is free by day: the review run goes to another medium family.
        planned, review = h.night(FakeRunner({"review": reviewer(APPROVE)}))
        self.assertEqual((planned["action"], planned["provider"]), ("review", "muse"))
        self.assertEqual(planned["seats"]["review"]["tier"], "medium")
        self.assertEqual((review["status"], review["verdict"]), ("reviewed", "approve"))
        self.assertIn(f"PR_{pr}", h.gh.auto_merge)
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR})
        votes = h.ctx.store.load()["items"][str(pr)]["votes"]
        self.assertEqual((votes["approvals"], votes["tiers"]),
                         (["gemini", "muse"], {"gemini": "medium", "muse": "medium"}))

    def test_one_strong_review_is_enough(self):
        h = Harness(self, env=ALL, machine=MACHINE)  # by night: the Claude accounts are open
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                             "review": reviewer(APPROVE)})
        planned, result = h.night(runner)
        self.assertEqual((planned["provider"], result["status"]), ("claude-3", "approved"))
        self.assertEqual([(c.role, c.model) for c in runner.calls],
                         [("plan", "opus"), ("build", "opus"), ("review", "opus")])
        pr = int(h.gh.list_pulls(head="bot/issue-12")[0]["number"])
        self.assertIn(f"PR_{pr}", h.gh.auto_merge)
        self.assertEqual(h.ctx.store.load()["items"][str(pr)]["votes"]["tiers"],
                         {"claude": "strong"})

    def test_a_review_run_prefers_a_strong_model(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        pr, _ = self.build_on_agy(h)
        h.committed_hours()  # claude-3 works all day: a strong model is free
        planned, review = h.night(FakeRunner({"review": reviewer(APPROVE)}))
        self.assertEqual((planned["action"], planned["provider"]), ("review", "claude-3"))
        self.assertEqual(planned["seats"]["review"]["tier"], "strong")
        self.assertEqual(review["verdict"], "approve")
        self.assertIn(f"PR_{pr}", h.gh.auto_merge)  # one strong approval is enough

    def test_findings_send_it_back_and_it_stops_after_three(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        pr, _ = self.build_on_agy(h)
        h.night(FakeRunner({"review": reviewer(changes("It skips the replay check."))}))
        record = h.ctx.store.load()["items"][str(pr)]
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_REVISE})
        self.assertEqual((record["source"], record["cross_rounds"]), ("cross-review", 1))
        # The revision goes to the builder the building rule picks, reads the findings, and
        # goes back for another review.
        runner = FakeRunner({"revise": builder({"src/game.txt": "rules v2 + replay\n"}),
                             "review": reviewer(APPROVE)})
        planned, result = h.night(runner)
        self.assertEqual((planned["action"], planned["provider"], result["status"]),
                         ("revise", "agy", "approved"))
        self.assertIn("It skips the replay check.", runner.calls[0].prompt)
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_CROSS})
        self.assertEqual(h.gh.auto_merge, {})
        for _ in range(2):
            h.night(FakeRunner({"review": reviewer(changes("Still wrong."))}))
            if LABEL_REVISE in h.gh.label_names(pr):
                h.night(FakeRunner({"revise": builder({"src/game.txt": "again\n"}),
                                    "review": reviewer(APPROVE)}))
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_BLOCKED})
        self.assertIn("It needs a person", h.gh.bot_comments(pr)[-1])

    def test_a_review_of_a_head_that_moved_does_not_count(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        pr, _ = self.build_on_agy(h)
        def moved(request):
            # Someone pushes to the branch while the reviewer reads it.
            push_branch(h.origin, h.root, "bot/issue-12", {"src/game.txt": "someone else\n"},
                        base="bot/issue-12")
            return reviewer(APPROVE)(request)
        h.night(FakeRunner({"review": moved}))
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_CROSS})
        self.assertEqual(h.gh.auto_merge, {})
        self.assertIn("does not count", h.gh.bot_comments(pr)[-1])

    def test_a_hard_item_is_planned_built_and_reviewed_by_opus_only(self):
        h = Harness(self, env=ALL, machine=MACHINE)
        h.gh.add_issue(12, labels=(LABEL_BUILD, "difficulty:hard"))
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                             "review": reviewer(APPROVE)})
        planned, result = h.night(runner)
        self.assertEqual((planned["provider"], planned["difficulty"], result["status"]),
                         ("claude-3", "hard", "approved"))
        self.assertEqual({c.model for c in runner.calls}, {"opus"})
        pr = int(h.gh.list_pulls(head="bot/issue-12")[0]["number"])
        self.assertIn(f"PR_{pr}", h.gh.auto_merge)
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, "difficulty:hard"})
        self.assertEqual(h.ctx.store.load()["items"][str(pr)]["difficulty"], "hard")


class HandoffTests(unittest.TestCase):
    def test_the_next_agent_gets_the_notes_and_the_branch(self):
        h = Harness(self, env=ALL, machine=MACHINE)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))

        def limited(request):
            return RunResult(False, "", 1, error="hit your limit", reset_at="2026-09-30T04:00:00Z")

        first = FakeRunner({"build": builder({"src/game.txt": "half\n",
                                              NOTES_FILE: "plan: rules v2\nnext: the Coin\n"}),
                            "review": limited})
        planned, result = h.night(first)
        self.assertEqual((planned["provider"], result["status"]), ("claude-3", "interrupted"))
        record = h.ctx.store.load()["items"]["12"]
        self.assertIn("next: the Coin", record["handoff"]["notes"])
        self.assertEqual(record["handoff"]["provider"], "claude-3")
        self.assertTrue(record["planned_at"])  # planned in that run: not planned again
        # The notes never reach the branch.
        self.assertNotIn(NOTES_FILE, git(h.origin, "ls-tree", "-r", "--name-only", result["head"]))
        # claude-3 is parked until its reset; the next run is another account, told what happened.
        second = FakeRunner({"build": builder({"src/game.txt": "whole\n"}),
                             "review": reviewer(APPROVE)})
        planned, result = h.night(second)
        self.assertEqual((planned["provider"], result["status"]), ("claude-1", "approved"))
        self.assertEqual([c.role for c in second.calls], ["build", "review"])
        prompt = second.calls[0].prompt
        self.assertIn("## Picking up from another agent", prompt)
        self.assertIn("next: the Coin", prompt)
        self.assertNotIn("handoff", h.ctx.store.load()["items"]["12"])


class VaultDeliveryTests(unittest.TestCase):
    def test_deliver_keeps_a_sealed_login_and_refuses_anything_else(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned = plan_mod.make(h.ctx)
        self.assertEqual(planned["provider"], "agy")
        out = h.root / "out-vault"
        out.mkdir()
        (out / "result.json").write_text(json.dumps({"status": "failed", "reason": "boom"}))
        sealed = vault.seal({"provider": "agy", "files": {"auth.json": "{}"}}, "the-secret")
        (out / "vault.enc").write_text(sealed)
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        text, _ = h.gh.get_file("vault/agy.enc", "bot-state")
        self.assertEqual(text.strip(), sealed)
        # The next plan hands it to the model job, which alone can open it.
        planned = plan_mod.make(h.ctx)
        self.assertEqual(planned["vault"].strip(), sealed)
        (out / "vault.enc").write_text("not a vault")
        log = Deliverer(h.ctx, planned, out, h.deliver_repo).run()["log"]
        self.assertIn("ignored a vault that is not sealed", log)


if __name__ == "__main__":
    unittest.main()
