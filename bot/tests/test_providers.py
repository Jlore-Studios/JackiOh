"""Subscriptions: `.harness/providers.json`, availability, and which one takes which item."""

from __future__ import annotations

import json
import unittest
from datetime import timedelta

from harness import clock, plan as plan_mod, providers
from harness.config import LABEL_BUILD, LABEL_CROSS, LABEL_PR, LABEL_WORKING
from harness.errors import ConfigError
from harness.providers import Secrets
from harness.state import item as state_item

from tests.fakes import FakeGitHub
from tests.support import DAY, NIGHT, ROOT, make_config, make_ctx

ALL = " ".join(providers.SECRETS)


def raw_providers() -> dict:
    return json.loads((ROOT / providers.PROVIDERS_PATH).read_text(encoding="utf-8"))


def pool_with(**changes) -> providers.Pool:
    raw = raw_providers()
    for name, fields in changes.items():
        raw["providers"][name].update(fields)
    return providers.parse(raw)


def secrets(*names: str) -> dict:
    return {"HARNESS_SECRETS_SET": " ".join(names)}


class ParseTests(unittest.TestCase):
    def test_the_committed_file(self):
        pool = providers.load(ROOT)
        self.assertEqual(pool.priority[0], "claude-1")
        self.assertEqual(pool.max_parallel, 3)
        self.assertEqual({p.cli for p in pool.ordered()}, set(providers.CLIS))
        self.assertEqual(len([p for p in pool.ordered() if p.cli == "claude"]), 4)
        first = pool.get("claude-1")
        self.assertEqual(first.secret, "CLAUDE_CODE_OAUTH_TOKEN")
        self.assertTrue(first.self_review and first.difficult and first.quiet_check)
        for provider in pool.ordered():
            self.assertEqual(provider.self_review, provider.cli == "claude", provider.id)
            self.assertEqual(provider.difficult, provider.cli == "claude", provider.id)

    def test_mistakes_are_errors(self):
        def broken(change) -> None:
            raw = raw_providers()
            change(raw)
            with self.assertRaises(ConfigError):
                providers.parse(raw)
        broken(lambda r: r["providers"]["gpt"].update(surprise=1))
        broken(lambda r: r["providers"]["gpt"].update(cli="copilot"))
        broken(lambda r: r["providers"]["gpt"].update(secret="BOT_GITHUB_TOKEN"))
        broken(lambda r: r["providers"]["gpt"].update(secret="CLAUDE_CODE_OAUTH_TOKEN"))
        broken(lambda r: r["providers"]["gpt"].update(limits={"mode": "caps", "five_hour": 98}))
        broken(lambda r: r["providers"]["gpt"].update(limits={"mode": "caps"}))
        broken(lambda r: r["providers"]["gpt"].update(schedule={"mode": "window", "start": "9"}))
        broken(lambda r: r["providers"]["gpt"].update(roles=["build", "dance"]))
        broken(lambda r: r.update(priority=["claude-1"]))
        broken(lambda r: r.update(max_parallel=0))


class AvailabilityTests(unittest.TestCase):
    zone = "America/Chicago"

    def why(self, provider_id, state=None, at=NIGHT, env=None, forced=False, pool=None):
        pool = pool or providers.load(ROOT)
        return providers.availability(pool.get(provider_id), state or {"providers": {}}, at,
                                      self.zone, Secrets.of(env or secrets(*providers.SECRETS)),
                                      forced=forced)

    def test_hours_secrets_and_the_switch(self):
        self.assertIsNone(self.why("claude-1"))
        self.assertIn("outside its hours (21:00–07:00", self.why("claude-1", at=DAY))
        self.assertIsNone(self.why("claude-1", at=DAY, forced=True))
        self.assertIsNone(self.why("gpt", at=DAY))
        self.assertIn("`CODEX_AUTH_JSON` is not set", self.why("gpt", env=secrets("MUSE_AUTH")))
        off = pool_with(gpt={"enabled": False})
        self.assertIn("switched off", self.why("gpt", pool=off))
        # Run by hand, nothing says which secrets the workflow has: nothing is ruled out.
        self.assertIsNone(providers.availability(
            providers.load(ROOT).get("gpt"), {}, DAY, self.zone, Secrets.of({})))

    def test_caps_refusals_and_minutes(self):
        later = clock.iso(NIGHT + timedelta(hours=2))
        state = {"providers": {"claude-2": {"usage": {"seven_day": {"utilization": 0.95,
                                                                     "resets_at": later}}}}}
        self.assertIn("7-day usage is 95%", self.why("claude-2", state))
        self.assertIsNone(self.why("claude-2", state, at=NIGHT + timedelta(hours=3)))
        # `none` has no caps: only a refusal stops it, until its reset.
        state = {"providers": {"gpt": {"usage": {"seven_day": {"utilization": 0.99,
                                                                "resets_at": later}}}}}
        self.assertIsNone(self.why("gpt", state))
        s: dict = {"providers": {}}
        providers.note_usage(s, "gpt", None, "+PT30M", NIGHT)
        self.assertIn("refused a call", self.why("gpt", s))
        self.assertIsNone(self.why("gpt", s, at=NIGHT + timedelta(minutes=31)))
        budget = pool_with(muse={"limits": {"mode": "caps", "seven_day_minutes": 60}})
        s = {"providers": {}}
        providers.note_usage(s, "muse", None, None, NIGHT - timedelta(days=2), minutes=40)
        providers.note_usage(s, "muse", None, None, NIGHT - timedelta(hours=1), minutes=25)
        self.assertIn("spent 65 of its 60 minutes", self.why("muse", s, pool=budget))
        providers.note_usage(s, "muse", None, None, NIGHT - timedelta(days=9), minutes=999)
        self.assertIn("spent 65", self.why("muse", s, pool=budget))  # a week-old run has gone

    def test_the_old_top_level_readings_belong_to_the_first_claude_account(self):
        later = clock.iso(NIGHT + timedelta(hours=2))
        state = {"usage": {"five_hour": {"utilization": 0.99, "resets_at": later}},
                 "providers": {}}
        self.assertIn("5-hour usage is 99%", self.why("claude-1", state))
        self.assertIsNone(self.why("claude-2", state))
        providers.note_usage(state, "claude-1", {"five_hour": {"utilization": 0.1,
                                                               "resets_at": later}}, None, NIGHT)
        self.assertIsNone(self.why("claude-1", state))


def ctx_for(gh, at=NIGHT, env=None):
    return make_ctx(gh, at=at, cfg=make_config(env=env or secrets(*providers.SECRETS)))


class MatchingTests(unittest.TestCase):
    """`plan.make` with several subscriptions: who takes what."""

    def test_by_day_the_first_free_subscription_with_its_secret_builds(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        planned = plan_mod.make(ctx_for(gh, at=DAY))
        self.assertEqual((planned["number"], planned["provider"], planned["cli"], planned["secret"]),
                         (3, "gpt", "codex", "CODEX_AUTH_JSON"))
        gh2 = FakeGitHub()
        gh2.add_issue(3, labels=(LABEL_BUILD,))
        planned = plan_mod.make(ctx_for(gh2, at=DAY, env=secrets("GEMINI_OAUTH_CREDS")))
        self.assertEqual(planned["provider"], "gemini")
        self.assertEqual(ctx_for(gh2).store.load()["items"]["3"]["provider"], "gemini")

    def test_difficult_work_waits_for_opus_and_opus_takes_it_first(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        gh.add_issue(4, labels=(LABEL_BUILD, "difficult"))
        planned = plan_mod.make(ctx_for(gh))
        self.assertEqual((planned["number"], planned["provider"]), (4, "claude-1"))
        gh = FakeGitHub()
        gh.add_issue(4, labels=(LABEL_BUILD, "difficult"))
        planned = plan_mod.make(ctx_for(gh, at=DAY))
        self.assertEqual(planned["action"], "none")
        self.assertIn("outside its hours", planned["reason"])
        self.assertEqual(plan_mod.make(ctx_for(gh, at=DAY), force=True)["provider"], "claude-1")

    def test_one_item_per_subscription_and_at_most_three_lanes(self):
        gh = FakeGitHub()
        for n in (3, 4, 5, 6):
            gh.add_issue(n, labels=(LABEL_BUILD,))
        ctx = ctx_for(gh, at=DAY)
        taken = []
        for _ in range(4):
            planned = plan_mod.make(ctx)
            if planned["action"] == "none":
                break
            gh.runs[str(len(taken))] = {"status": "in_progress"}
            ctx.store.update(lambda s, n=planned["number"], r=str(len(taken)): state_item(
                s, n).update(run_id=r))
            taken.append((planned["number"], planned["provider"]))
        self.assertEqual(taken, [(3, "gpt"), (4, "gemini"), (5, "muse")])
        self.assertIn("every lane is busy", planned["reason"])
        # Each claim with work and a lane left started the next run.
        self.assertEqual(len(gh.dispatches), 2)

    def test_a_second_review_goes_to_another_model_family(self):
        gh = FakeGitHub()
        gh.add_pull(9, "bot/issue-5", labels=(LABEL_PR, LABEL_CROSS), sha="h1")
        ctx = ctx_for(gh, at=DAY)
        ctx.store.update(lambda s: state_item(s, 9).update(
            votes={"sha": "h1", "builder": "gpt", "approvals": ["gpt"]}))
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], planned["provider"], planned["head"], planned["builder"]),
                         ("review", "gemini", "h1", "gpt"))
        self.assertEqual(gh.label_names(9), {LABEL_PR, LABEL_WORKING})

    def test_the_shared_subscription_needs_the_gates_word(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        env = secrets("CLAUDE_CODE_OAUTH_TOKEN")
        self.assertIn("waits for its owner to be quiet",
                      plan_mod.make(ctx_for(gh, env=env), quiet_ok="")["reason"])
        self.assertEqual(plan_mod.make(ctx_for(gh, env=env), quiet_ok="claude-1")["provider"],
                         "claude-1")


class PeekTests(unittest.TestCase):
    def test_the_gate_waits_for_quiet_only_when_the_shared_one_is_best(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        look = plan_mod.peek(ctx_for(gh))
        self.assertEqual((look.work, look.provider, look.quiet_provider, look.quiet_secret,
                          look.fallback),
                         (True, "claude-1", "claude-1", "CLAUDE_CODE_OAUTH_TOKEN", True))
        look = plan_mod.peek(ctx_for(gh, at=DAY))
        self.assertEqual((look.provider, look.quiet_provider), ("gpt", ""))

    def test_a_second_run_does_not_wait_for_quiet_too(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        gh.runs["50"] = {"id": 50, "status": "in_progress"}
        gh.jobs["50"] = [{"steps": [{"name": plan_mod.QUIET_STEP, "status": "in_progress"}]}]
        look = plan_mod.peek(ctx_for(gh))
        self.assertEqual((look.work, look.provider, look.quiet_provider), (True, "claude-2", ""))
        look = plan_mod.peek(ctx_for(gh, env=secrets("CLAUDE_CODE_OAUTH_TOKEN")))
        self.assertFalse(look.work)
        self.assertIn("already waits for `claude-1`", look.reason)


class WhenTests(unittest.TestCase):
    def test_when_a_queued_item_runs(self):
        pool = providers.load(ROOT)
        text = providers.when_free(pool, {}, DAY, "America/Chicago",
                                   Secrets.of(secrets("CLAUDE_CODE_OAUTH_TOKEN")))
        self.assertIn("when `claude-1` opens (21:00–07:00", text)
        text = providers.when_free(pool, {}, DAY, "America/Chicago",
                                   Secrets.of(secrets("CODEX_AUTH_JSON")))
        self.assertIn("`gpt` can take it now", text)


if __name__ == "__main__":
    unittest.main()
