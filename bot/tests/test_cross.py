"""Several subscriptions end to end: the review rule (one strong approval, or two medium ones, the
same model twice included, or for an easy item a weak and a medium one), review runs,
`difficulty:hard`, handoff and the vault, through plan, work and deliver against a real repository
and the fake GitHub."""

from __future__ import annotations

import dataclasses
import json
import unittest

from harness import plan as plan_mod
from harness import providers, vault
from harness.config import LABEL_BLOCKED, LABEL_BUILD, LABEL_CROSS, LABEL_PR, LABEL_REVISE
from harness.deliver import Deliverer
from harness.runner import FakeRunner, RunResult
from harness.state import item as state_item
from harness.work import NOTES_FILE

from tests.fakes import FakeGitHub, git, push_branch
from tests.support import DAY, MACHINE, make_config, make_ctx, test_pool
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


def next_run(h, run_id):
    """The next night is another workflow run: its votes count apart from the last one's."""
    h.ctx.cfg = h.cfg = dataclasses.replace(h.cfg, run_id=run_id)


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
                                 "tiers": {"gemini": "medium"},
                                 "reviews": [{"family": "gemini", "tier": "medium", "run": "777"}]})
        self.assertIn("`gemini` (medium) approved it, so it waits for a strong or medium model's "
                      "review (the same medium model may review it again)",
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
        self.assertEqual([r["family"] for r in votes["reviews"]], ["gemini", "muse"])

    def test_two_reviews_by_the_same_medium_model_merge_it(self):
        """agy builds and its own reviewer approves; with no other subscription set up, a review
        run by agy again is the second medium approval."""
        h = Harness(self, env={"HARNESS_SECRETS_SET": ""}, machine=("agy",), at=DAY)
        pr, _ = self.build_on_agy(h)
        self.assertEqual(h.gh.auto_merge, {})
        self.assertNotIn("No subscription that could give that review is set up",
                         h.gh.bot_comments(12)[-1])
        next_run(h, "778")
        planned, review = h.night(FakeRunner({"review": reviewer(APPROVE)}))
        self.assertEqual((planned["action"], planned["provider"]), ("review", "agy"))
        self.assertEqual(planned["approved"], ["gemini"])
        self.assertEqual(review["verdict"], "approve")
        self.assertIn(f"PR_{pr}", h.gh.auto_merge)
        votes = h.ctx.store.load()["items"][str(pr)]["votes"]
        self.assertEqual([(r["family"], r["tier"], r["run"]) for r in votes["reviews"]],
                         [("gemini", "medium", "777"), ("gemini", "medium", "778")])

    def test_a_deliver_job_run_again_does_not_count_its_review_twice(self):
        h = Harness(self, env={"HARNESS_SECRETS_SET": ""}, machine=("agy",), at=DAY)
        pr, result = self.build_on_agy(h)
        d = Deliverer(h.ctx, {"action": "review", "provider": "agy"}, h.root / "unused",
                      h.deliver_repo)
        d.review_seat = h.cfg.pool.seat("agy")
        votes = d._vote(pr, result["head"], approve=True, builder=False)  # run 777 again
        self.assertEqual(len(votes["reviews"]), 1)
        self.assertFalse(d._rule_met(votes))

    def test_an_easy_item_ships_on_a_weak_and_a_medium_approval(self):
        """Devin builds an easy item and checks itself, which is no review; agy, the only medium
        model, approves it in a review run; then Devin reviews it in a run of its own, and a weak
        approval with a medium one is enough for an easy item."""
        h = Harness(self, env={"HARNESS_SECRETS_SET": ""}, machine=("devin", "agy"), at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD, "difficulty:easy"),
                       body="<!-- jackioh-bot:plan -->\n## Plan\n1. Edit src/game.txt.\n"
                            "<!-- /jackioh-bot:plan -->")
        planned, result = h.night(FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                                              "self_check": reviewer(APPROVE)}))
        self.assertEqual((planned["provider"], result["status"]), ("devin", "built"))
        pr = int(h.gh.list_pulls(head="bot/issue-12")[0]["number"])
        self.assertIn("one strong model's review (Opus), two medium ones (the same model may give "
                      "both), or a weak and a medium one", h.gh.bot_comments(12)[-1])
        # A medium model is free, so it reviews first.
        next_run(h, "778")
        planned, review = h.night(FakeRunner({"review": reviewer(APPROVE)}))
        self.assertEqual((planned["provider"], planned["seats"]["review"]["tier"]),
                         ("agy", "medium"))
        self.assertEqual(h.gh.auto_merge, {})
        self.assertIn("one more review, by a strong, medium or weak model",
                      h.gh.bot_comments(pr)[-1])
        # agy is no longer free: Devin, weak, gives the second review, and that is enough.
        next_run(h, "779")
        h.ctx.cfg = h.cfg = dataclasses.replace(h.cfg, pool=test_pool(("devin",)))
        planned, review = h.night(FakeRunner({"review": reviewer(APPROVE)}))
        self.assertEqual((planned["action"], planned["provider"]), ("review", "devin"))
        self.assertEqual(planned["seats"]["review"]["tier"], "weak")
        self.assertEqual(review["verdict"], "approve")
        self.assertIn(f"PR_{pr}", h.gh.auto_merge)
        votes = h.ctx.store.load()["items"][str(pr)]["votes"]
        self.assertEqual([(r["family"], r["tier"]) for r in votes["reviews"]],
                         [("gemini", "medium"), ("cognition", "weak")])

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

    def test_a_hard_item_is_planned_and_built_by_opus(self):
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


def resolve(extra: dict[str, str] | None = None):
    """A builder that resolves the conflict `main` left in src/game.txt, and writes `extra`."""
    def handler(request):
        target = request.cwd / "src" / "game.txt"
        assert "<<<<<<<" in target.read_text(), target.read_text()
        target.write_text("rules v2, with main's\n")
        for name, text in (extra or {}).items():
            (request.cwd / name).parent.mkdir(parents=True, exist_ok=True)
            (request.cwd / name).write_text(text)
        return RunResult(True, DONE)
    return handler


class ConflictCarryTests(unittest.TestCase):
    """A change the review rule cleared, then left with conflicts by `main`: the revision that
    resolves them ships on its own run's adversarial review, with no review run, when it started
    from the cleared commit and changed nothing but the conflicted files."""

    def conflicted(self, h, pr):
        """`main` moves on and the branch no longer merges."""
        push_branch(h.origin, h.root, "main", {"src/game.txt": "rules from main\n"})
        h.gh.threads[pr]["mergeable_state"] = "dirty"

    def cleared_pr(self, h):
        """agy builds and approves it, muse approves it in a review run: the rule is met, and
        then `main` conflicts with it."""
        pr, built = ReviewRuleTests.build_on_agy(self, h)
        next_run(h, "778")
        h.night(FakeRunner({"review": reviewer(APPROVE)}))
        self.assertIn(f"PR_{pr}", h.gh.auto_merge)
        record = h.ctx.store.load()["items"][str(pr)]
        self.assertEqual(record["cleared"]["sha"], built["head"])
        self.assertEqual(record["cleared"]["by"], "`gemini` (medium), `muse` (medium)")
        self.conflicted(h, pr)
        next_run(h, "779")
        return pr, built["head"]

    def test_a_conflict_on_a_cleared_change_ships_on_its_own_review(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        pr, cleared_sha = self.cleared_pr(h)
        runner = FakeRunner({"revise": resolve(), "review": reviewer(APPROVE)})
        planned, result = h.night(runner)
        self.assertEqual((planned["action"], planned["source"], planned["provider"]),
                         ("revise", "conflict", "agy"))
        self.assertEqual((result["status"], planned["cleared"]["sha"]), ("approved", cleared_sha))
        self.assertIn("it merges without another review run", h.gh.bot_comments(pr)[-2])
        self.assertEqual([c.role for c in runner.calls], ["revise", "review"])
        self.assertIn("Resolve the conflicts and change nothing else", runner.calls[0].prompt)
        self.assertIn("yours is the only review of the resolution", runner.calls[1].prompt)
        self.assertIn("resolved the conflicts in `src/game.txt`", runner.calls[1].prompt)
        # No review run: auto-merge is back on, pinned to the resolved head.
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR})
        self.assertEqual(h.gh.auto_merge_heads[f"PR_{pr}"], result["head"])
        record = h.ctx.store.load()["items"][str(pr)]
        self.assertEqual([r["family"] for r in record["votes"]["reviews"]], ["gemini"])
        carried = record["votes"]["carried"]
        self.assertEqual((carried["from"], carried["conflicts"]), (cleared_sha, ["src/game.txt"]))
        # The resolved head is cleared in turn, so the next conflict carries again.
        self.assertEqual((record["cleared"]["sha"], record["cleared"]["carried_from"]),
                         (result["head"], cleared_sha))
        self.assertEqual((record["cleared"]["by"], record["cleared"]["resolved_by"]),
                         ("`gemini` (medium), `muse` (medium)", "`gemini` (medium)"))
        comment = h.gh.bot_comments(pr)[-1]
        self.assertIn(f"The reviews had cleared this change at `{cleared_sha[:12]}` (`gemini` "
                      "(medium), `muse` (medium)). This revision only merged `main` and resolved "
                      "the conflicts in `src/game.txt`, and its own reviewer approved that, so the "
                      "clearance carries over and no review run is needed. Auto-merge is on",
                      comment)

    def test_a_resolution_that_changes_more_goes_back_to_review(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        pr, cleared_sha = self.cleared_pr(h)
        planned, result = h.night(FakeRunner({"revise": resolve({"src/extra.txt": "more\n"}),
                                              "review": reviewer(APPROVE)}))
        self.assertEqual(result["status"], "approved")
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_CROSS})
        self.assertEqual(h.gh.auto_merge, {})
        record = h.ctx.store.load()["items"][str(pr)]
        self.assertNotIn("carried", record["votes"])
        self.assertEqual(record["cleared"]["sha"], cleared_sha)
        self.assertIn("but that does not carry over: it changed more than the conflicts "
                      "(`src/extra.txt`). `gemini` (medium) approved it, so it waits for a strong "
                      "or medium model's review", h.gh.bot_comments(pr)[-1])

    def test_a_branch_that_moved_after_the_reviews_does_not_carry(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        pr, _ = self.cleared_pr(h)
        # Someone pushed to the branch after the reviews cleared it; nobody reviewed that.
        push_branch(h.origin, h.root, "bot/issue-12", {"src/other.txt": "unreviewed\n"},
                    base="bot/issue-12")
        runner = FakeRunner({"revise": resolve(), "review": reviewer(APPROVE)})
        h.night(runner)
        self.assertNotIn("change nothing else", runner.calls[0].prompt)
        self.assertNotIn("yours is the only review", runner.calls[1].prompt)
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_CROSS})
        self.assertIn("but that does not carry over: the branch had moved on from the commit they "
                      "cleared", h.gh.bot_comments(pr)[-1])

    def test_a_change_short_of_the_rule_has_nothing_to_carry(self):
        """One medium approval of two: the change was never cleared, so its resolution waits for
        the review run like any revision."""
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        pr, _ = ReviewRuleTests.build_on_agy(self, h)
        self.assertNotIn("cleared", h.ctx.store.load()["items"][str(pr)])
        self.conflicted(h, pr)
        next_run(h, "778")
        runner = FakeRunner({"revise": resolve(), "review": reviewer(APPROVE)})
        planned, result = h.night(runner)
        self.assertEqual((planned["source"], result["status"]), ("conflict", "approved"))
        self.assertNotIn("cleared", planned)
        self.assertNotIn("yours is the only review", runner.calls[1].prompt)
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_CROSS})
        self.assertEqual(h.gh.auto_merge, {})
        self.assertNotIn("cleared this change", h.gh.bot_comments(pr)[-1])

    def test_a_builder_that_reviews_in_its_own_run_goes_first(self):
        """Devin takes an easy revision first, but nothing reviews in its run, so a conflict on a
        cleared change goes to agy, whose own reviewer can carry the clearance."""
        gh = FakeGitHub()
        ctx = make_ctx(gh, at=DAY, cfg=make_config(env={"HARNESS_SECRETS_SET": ""},
                                                   machine=("devin", "agy")))
        gh.add_pull(9, "bot/issue-2", labels=(LABEL_PR, LABEL_REVISE, "difficulty:easy"))
        ctx.store.update(lambda s: state_item(s, 9).update(kind="revise", source="conflict"))
        self.assertEqual(plan_mod.peek(ctx).provider, "devin")
        ctx.store.update(lambda s: state_item(s, 9).update(
            cleared={"sha": "abc", "by": "`claude` (strong)"}))
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["provider"], planned["cleared"]["sha"]), ("agy", "abc"))
        self.assertIn("it merges without another review run", gh.bot_comments(9)[-1])


if __name__ == "__main__":
    unittest.main()
