"""The adversarial review of the subscriptions change, each finding as the scenario it found."""

from __future__ import annotations

import json
import unittest

from harness import plan as plan_mod
from harness import providers, vault
from harness.config import (LABEL_BLOCKED, LABEL_BUILD, LABEL_CROSS, LABEL_PR, LABEL_REVISE,
                            LABEL_WORKING)
from harness.deliver import Deliverer
from harness.errors import ConfigError, GitHubError
from harness.runner import FakeRunner
from harness.state import item as state_item
from harness.work import Worker

from tests.fakes import FakeGitHub, push_branch
from tests.support import DAY, MACHINE, NIGHT, make_config, make_ctx, with_lanes
from tests.test_cross import ALL, timed_builder
from tests.test_flow import Harness
from tests.test_providers import raw_providers, secrets
from tests.test_work import APPROVE, builder, changes, reviewer


def medium_pr(test, h):
    """A pull request a medium model (Muse, the first of them) built and approved by day, waiting
    for its review run."""
    h.gh.add_issue(12, "Make the rules v2", labels=(LABEL_BUILD,))
    h.night(FakeRunner({"build": timed_builder({"src/game.txt": "rules v2\n"}),
                        "review": reviewer(APPROVE)}))
    pr = int(h.gh.list_pulls(head="bot/issue-12")[0]["number"])
    test.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_CROSS})
    return pr


def empty_out(h, name):
    out = h.root / name
    out.mkdir()
    return out


class AutoMergeTests(unittest.TestCase):
    def test_a_revision_by_another_model_turns_opus_auto_merge_off(self):
        h = Harness(self, env=ALL, machine=MACHINE)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        h.night(FakeRunner({"build": builder({"src/game.txt": "v2\n"}), "review": reviewer(APPROVE)}))
        pr = int(h.gh.list_pulls(head="bot/issue-12")[0]["number"])
        self.assertIn(f"PR_{pr}", h.gh.auto_merge)  # Opus: auto-merge on
        # A revision is queued by a path that leaves auto-merge alone, and Muse does it by day:
        # its medium approval alone is not enough, so the strong approval of the old head is gone.
        h.gh.threads[pr]["labels"].append({"name": LABEL_REVISE})
        h.ctx.clock_fn.at = DAY
        planned, result = h.night(FakeRunner({"revise": builder({"src/game.txt": "v2.1\n"}),
                                              "review": reviewer(APPROVE)}))
        self.assertEqual((planned["provider"], result["status"]), ("muse", "approved"))
        self.assertNotIn(f"PR_{pr}", h.gh.auto_merge)
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_CROSS})

    def test_auto_merge_is_pinned_to_the_approved_commit(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        pr = medium_pr(self, h)
        h.night(FakeRunner({"review": reviewer(APPROVE)}))
        votes = h.ctx.store.load()["items"][str(pr)]["votes"]
        self.assertEqual(h.gh.auto_merge_heads[f"PR_{pr}"], votes["sha"])

    def test_a_rejection_stands_until_its_model_approves_or_the_head_moves(self):
        h = Harness(self, env=ALL, machine=MACHINE)
        d = Deliverer(h.ctx, {"action": "review", "provider": "agy"}, empty_out(h, "o"),
                      h.deliver_repo)
        medium = {"gpt": "medium", "muse": "medium", "gemini": "medium"}
        self.assertTrue(d._rule_met({"approvals": ["gpt", "muse"], "tiers": medium}))
        self.assertFalse(d._rule_met({"approvals": ["gpt", "muse"], "tiers": medium,
                                      "rejections": ["gemini"]}))  # votes are by family
        self.assertTrue(d._rule_met({"approvals": ["gpt", "gemini"], "tiers": medium,
                                     "rejections": []}))
        self.assertTrue(d._rule_met({"approvals": ["claude"], "tiers": {"claude": "strong"}}))

    def test_only_the_bots_own_pull_requests_get_a_second_review(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        h.gh.add_pull(30, "feature/someone", labels=(LABEL_CROSS,))
        self.assertNotEqual(plan_mod.make(h.ctx).get("number"), 30)

    def test_with_only_its_builder_set_up_it_waits_for_that_models_second_review(self):
        h = Harness(self, env=secrets(), machine=("gpt",), at=DAY)
        h.gh.add_issue(12, "Make the rules v2", labels=(LABEL_BUILD,))
        h.night(FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                            "review": reviewer(APPROVE)}))
        comment = h.gh.bot_comments(12)[-1]
        self.assertIn("`gpt` (medium) approved it, so it waits for a strong or medium model's "
                      "review (the same medium model may review it again)", comment)
        self.assertNotIn("No subscription that could give that review is set up", comment)


class SecondReviewTests(unittest.TestCase):
    def test_a_comment_during_the_review_gets_its_revision_before_any_merge(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        pr = medium_pr(self, h)

        def commented(request):
            h.ctx.store.update(lambda s: state_item(s, pr).update(pending_request=True))
            return reviewer(APPROVE)(request)

        h.night(FakeRunner({"review": commented}))
        self.assertEqual(h.gh.auto_merge, {})
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_REVISE})
        self.assertFalse(h.ctx.store.load()["items"][str(pr)]["pending_request"])

    def test_a_review_whose_run_dies_backs_off_its_subscription_and_is_counted(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        pr = medium_pr(self, h)
        seen = []
        for attempt in (1, 2):
            planned = plan_mod.make(h.ctx)
            self.assertEqual((planned["action"], planned["number"]), ("review", pr))
            seen.append(planned["provider"])
            Deliverer(h.ctx, planned, empty_out(h, f"died-{attempt}"), h.deliver_repo).run()
        self.assertEqual(seen, ["agy", "gpt"])  # each death backs its subscription off
        self.assertEqual(h.gh.label_names(pr), {LABEL_PR, LABEL_BLOCKED})
        self.assertIn("died 2 times", h.gh.bot_comments(pr)[-1])

    def test_a_second_review_runs_none_of_the_builders_code(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        log = h.root / "installs.log"
        h.cfg = make_config(env={"GITHUB_SERVER_URL": f"file://{h.root / 'remote'}",
                                 "BOT_GITHUB_TOKEN": "t" * 20, **ALL},
                            machine=MACHINE,
                            gates=[{"name": "rules exist", "run": "true", "timeout_minutes": 1}],
                            install={"run": f"pwd >> {log}", "timeout_minutes": 1},
                            max_review_cycles=2)
        h.ctx = make_ctx(h.gh, cfg=h.cfg, at=DAY)
        pr = medium_pr(self, h)
        builds = log.read_text().count("\n")
        self.assertGreaterEqual(builds, 1)  # the build did install, so the log works
        planned, review = h.night(FakeRunner({"review": reviewer(APPROVE)}))
        self.assertEqual((planned["action"], review["status"]), ("review", "reviewed"))
        self.assertEqual(log.read_text().count("\n"), builds)
        self.assertNotIn(f"item-{pr}", log.read_text())


class InfraTests(unittest.TestCase):
    def test_a_broken_subscription_backs_off_alone(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned = plan_mod.make(h.ctx)
        self.assertEqual(planned["provider"], "muse")
        out = empty_out(h, "infra")
        (out / "result.json").write_text(json.dumps({"status": "infra",
                                                     "reason": "agy's login is not a login"}))
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        planned = plan_mod.make(h.ctx)
        self.assertEqual((planned["number"], planned["provider"]), (12, "agy"))

    def test_a_run_github_cannot_read_keeps_its_lane_and_its_item(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_WORKING,))
        gh.add_issue(4, labels=(LABEL_BUILD,))

        def flaky(run_id):
            raise GitHubError("server error", 502)

        gh.get_run = flaky
        ctx = make_ctx(gh, at=DAY, cfg=make_config(env=secrets(), machine=("gpt",)))
        ctx.store.update(lambda s: state_item(s, 3).update(run_id="9", provider="gpt"))
        planned = plan_mod.make(ctx)
        self.assertEqual(gh.label_names(3), {LABEL_WORKING})  # not requeued
        self.assertEqual(planned["action"], "none")  # gpt, its only subscription, is held

    def test_a_run_request_is_kept_when_every_lane_is_busy(self):
        gh = FakeGitHub()
        gh.add_issue(4, labels=(LABEL_BUILD,))
        # The machine's three slots and GitHub's three lanes, each held apart from the other.
        held = ((5, "gpt"), (6, "agy"), (7, "muse"), (8, "claude-4"), (9, "claude-5"),
                (10, "claude-6"))
        for n, _ in held:
            gh.add_issue(n, labels=(LABEL_WORKING,))
            gh.runs[str(n)] = {"status": "in_progress"}
        ctx = make_ctx(gh, at=DAY, cfg=with_lanes(make_config(env=ALL, machine=MACHINE), 3))
        ctx.store.update(lambda s: (s.update(run_requested={"at": "2026-09-29T16:59:00Z",
                                                            "item": None}),
                                    *[state_item(s, n).update(run_id=str(n), provider=p)
                                      for n, p in held]))
        self.assertIn("every lane is busy", plan_mod.make(ctx)["reason"])
        self.assertIsNotNone(ctx.store.load()["run_requested"])

    def test_github_s_runners_and_the_machine_have_lanes_of_their_own(self):
        """A full machine leaves GitHub's lanes to the Claude accounts, and Claude runs filling
        GitHub's lanes leave the machine's slots to its subscriptions."""
        for held, on_machine in ((((5, "gpt"), (6, "agy"), (7, "muse")), False),
                                 (((5, "claude-4"), (6, "claude-5"), (7, "claude-6")), True)):
            with self.subTest(held=[p for _, p in held]):
                gh = FakeGitHub()
                gh.add_issue(4, labels=(LABEL_BUILD,))
                for n, _ in held:
                    gh.add_issue(n, labels=(LABEL_WORKING,))
                    gh.runs[str(n)] = {"status": "in_progress"}
                ctx = make_ctx(gh, at=NIGHT,
                               cfg=with_lanes(make_config(env=ALL, machine=MACHINE), 3))
                ctx.store.update(lambda s: [state_item(s, n).update(run_id=str(n), provider=p)
                                            for n, p in held])
                planned = plan_mod.make(ctx)
                self.assertEqual((planned["action"], planned["number"]), ("build", 4))
                self.assertEqual(ctx.cfg.pool.on_machine(planned["provider"]), on_machine)


class VaultAndLoginTests(unittest.TestCase):
    def test_a_vault_write_that_races_another_writer_is_tried_again(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned = plan_mod.make(h.ctx)
        out = empty_out(h, "vault")
        (out / "result.json").write_text(json.dumps({"status": "failed", "reason": "boom"}))
        sealed = vault.seal({"provider": "muse", "files": {"auth.json": "{}"}}, "the-secret")
        (out / "vault.enc").write_text(sealed)
        h.gh.branches.add("bot-state")
        h.gh.conflicts_to_inject = 1
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        text, _ = h.gh.get_file("vault/muse.enc", "bot-state")
        self.assertEqual(text.strip(), sealed)

    def test_every_model_call_hands_a_refreshed_login_on_at_once(self):
        calls = []
        h = Harness(self, env=ALL, machine=MACHINE)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned = plan_mod.make(h.ctx)
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}), "review": reviewer(APPROVE)})
        Worker(h.cfg, planned, runner, h.clone, h.root / "work", h.root / "out-calls",
               after_call=lambda: calls.append(1)).run()
        self.assertEqual(len(calls), len(runner.calls))

    def test_only_a_claude_account_can_wait_for_quiet(self):
        raw = raw_providers()
        raw["providers"]["gpt"]["quiet_check"] = True
        with self.assertRaises(ConfigError):
            providers.parse(raw)


if __name__ == "__main__":
    unittest.main()
