"""Several subscriptions end to end: the two-model rule, second reviews, `difficult`, handoff and
the vault, through plan, work and deliver against a real repository and the fake GitHub."""

from __future__ import annotations

import json
import unittest

from harness import providers, vault
from harness.config import LABEL_BLOCKED, LABEL_BUILD, LABEL_CROSS, LABEL_PR, LABEL_REVISE
from harness.deliver import Deliverer
from harness.runner import FakeRunner, RunResult
from harness.work import NOTES_FILE

from tests.fakes import git
from tests.support import DAY, NIGHT
from tests.test_flow import Harness
from tests.test_work import APPROVE, DONE, builder, changes, reviewer

ALL = {"HARNESS_SECRETS_SET": " ".join(providers.SECRETS)}


def timed_builder(edits, minutes=10.0):
    inner = builder(edits)
    def handler(request):
        result = inner(request)
        result.duration_s = minutes * 60
        return result
    return handler


class TwoModelTests(unittest.TestCase):
    def build_on_gpt(self, h):
        h.gh.add_issue(12, "Make the rules v2", labels=(LABEL_BUILD,))
        runner = FakeRunner({"build": timed_builder({"src/game.txt": "rules v2\n"}),
                             "review": reviewer(APPROVE)})
        planned, result = h.night(runner)
        self.assertEqual((planned["provider"], result["status"]), ("gpt", "approved"))
        self.assertEqual(runner.calls[0].model, "gpt-6.1-sol")
        pr = int(h.gh.list_pulls(head="bot/issue-12")[0]["number"])
        return pr, result

    def test_a_change_built_by_another_model_waits_for_a_second_review(self):
        h = Harness(self, env=ALL, at=DAY)
        pr, result = self.build_on_gpt(h)
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_CROSS})
        self.assertEqual(h.gh.auto_merge, {})
        votes = h.ctx.store.load()["items"][str(pr)]["votes"]
        self.assertEqual(votes, {"sha": result["head"], "builder": "gpt", "approvals": ["gpt"],
                                 "rejections": []})
        self.assertIn("waits for a second model's review", h.gh.bot_comments(12)[-1])
        spent = h.ctx.store.load()["providers"]["gpt"]["spent"]
        self.assertEqual(spent[-1]["minutes"], 10.0)
        # The second review goes to another family, and its approval turns auto-merge on.
        planned, review = h.night(FakeRunner({"review": reviewer(APPROVE)}))
        self.assertEqual((planned["action"], planned["provider"]), ("review", "gemini"))
        self.assertEqual((review["status"], review["verdict"]), ("reviewed", "approve"))
        self.assertIn(f"PR_{pr}", h.gh.auto_merge)
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR})
        self.assertEqual(h.ctx.store.load()["items"][str(pr)]["votes"]["approvals"],
                         ["gpt", "gemini"])
        self.assertIn("Second review", h.gh.bot_comments(pr)[-1])

    def test_a_second_review_with_findings_sends_it_back_and_stops_after_three(self):
        h = Harness(self, env=ALL, at=DAY)
        pr, _ = self.build_on_gpt(h)
        h.night(FakeRunner({"review": reviewer(changes("It skips the replay check."))}))
        record = h.ctx.store.load()["items"][str(pr)]
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_REVISE})
        self.assertEqual((record["source"], record["cross_rounds"]), ("cross-review", 1))
        # The revision reads the second review's findings, and goes back for another review.
        runner = FakeRunner({"revise": builder({"src/game.txt": "rules v2 + replay\n"}),
                             "review": reviewer(APPROVE)})
        planned, result = h.night(runner)
        self.assertEqual((planned["action"], result["status"]), ("revise", "approved"))
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
        h = Harness(self, env=ALL, at=DAY)
        pr, _ = self.build_on_gpt(h)
        from tests.fakes import push_branch
        def moved(request):
            # Someone pushes to the branch while the second reviewer reads it.
            push_branch(h.origin, h.root, "bot/issue-12", {"src/game.txt": "someone else\n"},
                        base="bot/issue-12")
            return reviewer(APPROVE)(request)
        h.night(FakeRunner({"review": moved}))
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_CROSS})
        self.assertEqual(h.gh.auto_merge, {})
        self.assertIn("does not count", h.gh.bot_comments(pr)[-1])

    def test_opus_needs_no_second_model_and_difficult_stays_with_it(self):
        h = Harness(self, env=ALL)
        h.gh.add_issue(12, labels=(LABEL_BUILD, "difficult"))
        planned, result = h.night(FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                                              "review": reviewer(APPROVE)}))
        self.assertEqual((planned["provider"], result["status"]), ("claude-1", "approved"))
        pr = int(h.gh.list_pulls(head="bot/issue-12")[0]["number"])
        self.assertIn(f"PR_{pr}", h.gh.auto_merge)
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, "difficult"})
        self.assertTrue(h.ctx.store.load()["items"][str(pr)]["difficult"])


class HandoffTests(unittest.TestCase):
    def test_the_next_agent_gets_the_notes_and_the_branch(self):
        h = Harness(self, env=ALL)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))

        def limited(request):
            return RunResult(False, "", 1, error="hit your limit", reset_at="2026-09-30T04:00:00Z")

        first = FakeRunner({"build": builder({"src/game.txt": "half\n",
                                              NOTES_FILE: "plan: rules v2\nnext: the Coin\n"}),
                            "review": limited})
        planned, result = h.night(first)
        self.assertEqual((planned["provider"], result["status"]), ("claude-1", "interrupted"))
        record = h.ctx.store.load()["items"]["12"]
        self.assertIn("next: the Coin", record["handoff"]["notes"])
        self.assertEqual(record["handoff"]["provider"], "claude-1")
        # The notes never reach the branch.
        self.assertNotIn(NOTES_FILE, git(h.origin, "ls-tree", "-r", "--name-only", result["head"]))
        # claude-1 is parked until its reset; the next run is another account, told what happened.
        second = FakeRunner({"build": builder({"src/game.txt": "whole\n"}),
                             "review": reviewer(APPROVE)})
        planned, result = h.night(second)
        self.assertEqual((planned["provider"], result["status"]), ("claude-2", "approved"))
        prompt = second.calls[0].prompt
        self.assertIn("## Picking up from another agent", prompt)
        self.assertIn("next: the Coin", prompt)
        self.assertNotIn("handoff", h.ctx.store.load()["items"]["12"])


class VaultDeliveryTests(unittest.TestCase):
    def test_deliver_keeps_a_sealed_login_and_refuses_anything_else(self):
        h = Harness(self, env=ALL, at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        from harness import plan as plan_mod
        planned = plan_mod.make(h.ctx)
        self.assertEqual(planned["provider"], "gpt")
        out = h.root / "out-vault"
        out.mkdir()
        (out / "result.json").write_text(json.dumps({"status": "failed", "reason": "boom"}))
        sealed = vault.seal({"provider": "gpt", "files": {"auth.json": "{}"}}, "the-secret")
        (out / "vault.enc").write_text(sealed)
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        text, _ = h.gh.get_file("vault/gpt.enc", "bot-state")
        self.assertEqual(text.strip(), sealed)
        # The next plan hands it to the model job, which alone can open it.
        planned = plan_mod.make(h.ctx)
        self.assertEqual(planned["vault"].strip(), sealed)
        (out / "vault.enc").write_text("not a vault")
        log = Deliverer(h.ctx, planned, out, h.deliver_repo).run()["log"]
        self.assertIn("ignored a vault that is not sealed", log)


if __name__ == "__main__":
    unittest.main()
