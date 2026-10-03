"""Subscriptions: `.harness/providers.json`, availability, and which one takes which item."""

from __future__ import annotations

import json
import unittest
from datetime import datetime, timedelta, timezone

from harness import clock, plan as plan_mod, providers
from harness.config import LABEL_BUILD, LABEL_CROSS, LABEL_PR, LABEL_WORKING
from harness.errors import ConfigError
from harness.providers import Secrets
from harness.state import item as state_item

from tests.fakes import FakeGitHub
from tests.support import ALL_MACHINE, DAY, MACHINE, NIGHT, ROOT, make_config, make_ctx, test_pool

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
        self.assertEqual((pool.max_parallel, pool.machine_parallel), (7, 3))
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

    def test_logins_and_runners(self):
        pool = providers.load(ROOT)
        for provider in pool.ordered():
            # The Claude accounts run on GitHub's runners; every other subscription has a runner
            # of its own on the machine (bot/machine/README.md).
            claude = provider.cli == "claude"
            self.assertEqual(provider.runs_on,
                             "ubuntu-latest" if claude else f"night-vm-{provider.id}")
            self.assertEqual(provider.login, "secret" if claude else "machine")
            self.assertEqual(pool.on_machine(provider.id), not claude)
        self.assertEqual(pool.get("gpt").secret, "")

        def broken(change, words) -> None:
            raw = raw_providers()
            change(raw)
            with self.assertRaisesRegex(ConfigError, words):
                providers.parse(raw)
        broken(lambda r: r["providers"]["gpt"].update(secret="CODEX_AUTH_JSON"), "takes no secret")
        broken(lambda r: r["providers"]["gpt"].update(runs_on="ubuntu-latest"), "its runner")
        broken(lambda r: r["providers"]["gpt"].update(runs_on="night vm"), "not a runner label")
        broken(lambda r: r["providers"]["gpt"].update(login="keyring"), "not one of")
        broken(lambda r: r["providers"]["agy"].update(login="secret", secret="MUSE_AUTH",
                                                      runs_on="ubuntu-latest"), "on the machine only")
        broken(lambda r: r["providers"]["muse"].update(runs_on="night-vm-gpt"), "share the runner")
        broken(lambda r: r["providers"]["claude-2"].update(runs_on="night-vm-gpt"),
               "share the runner")
        broken(lambda r: r["providers"]["devin"].update(lanes=0), "at least 1")
        broken(lambda r: r["providers"]["devin"].update(lanes="two"), "not a number")
        broken(lambda r: r.update(machine_parallel=8), "machine_parallel")
        # A secret login may still run on GitHub's runners, and those are shared by design.
        raw = raw_providers()
        for name in ("claude-1", "claude-2"):
            raw["providers"][name]["runs_on"] = "ubuntu-latest"
        raw["providers"]["gpt"].update(login="secret", secret="CODEX_AUTH_JSON",
                                       runs_on="ubuntu-latest")
        self.assertEqual(providers.parse(raw).get("gpt").secret, "CODEX_AUTH_JSON")


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
        self.assertIn("`CLAUDE_CODE_OAUTH_TOKEN_2` is not set",
                      self.why("claude-2", env=secrets("CLAUDE_CODE_OAUTH_TOKEN")))
        # A login on the machine has no secret to be missing.
        self.assertIsNone(self.why("gpt", env=secrets()))
        keyed = pool_with(gpt={"login": "secret", "secret": "CODEX_AUTH_JSON",
                               "runs_on": "ubuntu-latest"})
        self.assertIn("`CODEX_AUTH_JSON` is not set",
                      self.why("gpt", env=secrets("MUSE_AUTH"), pool=keyed))
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
        # --force lifts a subscription's hours, never its caps.
        self.assertIn("7-day usage is 95%", self.why("claude-2", state, at=DAY, forced=True))
        # claude-2 stops at 90% of its 5-hour session too; claude-1 goes on to 98%.
        session = {"five_hour": {"utilization": 0.92, "resets_at": later}}
        self.assertIn("5-hour usage is 92%",
                      self.why("claude-2", {"providers": {"claude-2": {"usage": session}}}))
        self.assertIsNone(self.why("claude-1", {"providers": {"claude-1": {"usage": session}}}))
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


def ctx_for(gh, at=NIGHT, env=None, machine=MACHINE, committed_hours=False):
    return make_ctx(gh, at=at, cfg=make_config(env=env or secrets(*providers.SECRETS),
                                               machine=machine, committed_hours=committed_hours))


class MatchingTests(unittest.TestCase):
    """`plan.make` with several subscriptions: who takes what."""

    def test_by_day_the_first_free_subscription_with_its_secret_builds(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        planned = plan_mod.make(ctx_for(gh, at=DAY))
        self.assertEqual((planned["number"], planned["provider"], planned["cli"], planned["secret"],
                          planned["login"], planned["runs_on"]),
                         (3, "gpt", "codex", "", "machine", "night-vm-gpt"))
        gh2 = FakeGitHub()
        gh2.add_issue(3, labels=(LABEL_BUILD,))
        planned = plan_mod.make(ctx_for(gh2, at=DAY, machine=("agy",)))
        self.assertEqual((planned["provider"], planned["runs_on"]), ("agy", "night-vm-agy"))
        self.assertEqual(ctx_for(gh2).store.load()["items"]["3"]["provider"], "agy")

    def test_devin_takes_the_first_lane_after_the_claude_accounts(self):
        """Devin (SWE-2, free on the CLI until 2026-10-16) logs in on the machine, works any
        hour with no caps, is low-tier, and comes right after the Claude accounts in `priority`:
        by day, with them closed, it takes work ahead of GPT, agy and Muse."""
        pool = providers.load(ROOT)
        devin = pool.get("devin")
        self.assertEqual((devin.cli, devin.family, devin.login, devin.runs_on),
                         ("devin", "cognition", "machine", "night-vm-devin"))
        self.assertEqual((devin.schedule.mode, devin.limits.mode), ("always", "none"))
        self.assertEqual(providers.model_tier(devin.model), providers.LOW_TIER)
        self.assertEqual(pool.priority.index("devin"), pool.priority.index("claude-4") + 1)
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        planned = plan_mod.make(ctx_for(gh, at=DAY, machine=ALL_MACHINE))
        self.assertEqual((planned["provider"], planned["cli"], planned["runs_on"]),
                         ("devin", "devin", "night-vm-devin"))

    def test_devin_is_switched_off_from_october_15(self):
        """`off_from`: from that day (Central time) Devin takes no new work, forced or not, and
        the planner opens one issue asking a person what it should do now."""
        pool = providers.load(ROOT)
        devin = pool.get("devin")
        self.assertEqual(str(devin.off_from), "2026-10-15")
        self.assertIn("free on Devin's CLI only through 2026-10-16", devin.off_reason)
        everyone = Secrets.of(secrets(*providers.SECRETS))
        late_on_the_14th = datetime(2026, 10, 15, 4, 59, tzinfo=timezone.utc)  # 23:59 CDT
        self.assertIsNone(providers.availability(devin, {}, late_on_the_14th, "America/Chicago",
                                                 everyone))
        on_the_15th = datetime(2026, 10, 15, 5, 0, tzinfo=timezone.utc)
        for forced in (False, True):
            self.assertIn("switched off from 2026-10-15", providers.availability(
                devin, {}, on_the_15th, "America/Chicago", everyone, forced=forced))
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        ctx = make_ctx(gh, at=on_the_15th + timedelta(hours=12),
                       cfg=make_config(env=secrets(*providers.SECRETS), machine=ALL_MACHINE))
        planned = plan_mod.make(ctx)
        self.assertNotEqual(planned.get("provider"), "devin")
        opened = [n for n, t in gh.threads.items() if "`devin` is switched off" in t["title"]]
        self.assertEqual(len(opened), 1)
        body = gh.threads[opened[0]]["body"]
        self.assertIn("off_from: 2026-10-15", body)
        self.assertIn("free on Devin's CLI only through 2026-10-16", body)
        self.assertEqual(gh.label_names(opened[0]), {"night bot"})
        plan_mod.make(ctx)  # once only
        self.assertEqual(len([t for t in gh.threads.values()
                              if "`devin` is switched off" in t["title"]]), 1)
        raw = raw_providers()
        raw["providers"]["devin"]["off_from"] = "mid-October"
        with self.assertRaises(ConfigError):
            providers.parse(raw)

    def test_claude_2_and_3_work_any_hour(self):
        """The committed hours: claude-2 and claude-3 run all day, claude-2 under its 90% caps
        and claude-3 with none, so by day they take Opus's work ahead of the other models,
        `difficult` first; claude-1 and claude-4 wait for the night."""
        pool = providers.load(ROOT)
        hours = {p.id: (p.schedule.mode, p.limits.mode) for p in pool.ordered() if p.cli == "claude"}
        self.assertEqual(hours, {"claude-1": ("window", "caps"), "claude-2": ("always", "caps"),
                                 "claude-3": ("always", "none"), "claude-4": ("window", "caps")})
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        gh.add_issue(4, labels=(LABEL_BUILD, "difficult"))
        ctx = ctx_for(gh, at=DAY, committed_hours=True)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["number"], planned["provider"]), (4, "claude-2"))
        gh.runs["1"] = {"status": "in_progress"}
        ctx.store.update(lambda s: state_item(s, 4).update(run_id="1"))
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["number"], planned["provider"]), (3, "claude-3"))
        # Past 90% claude-2 is held; claude-3's readings never stop it, only a refusal does.
        later = clock.iso(DAY + timedelta(days=2))
        full = {"five_hour": {"utilization": 0.99, "resets_at": later},
                "seven_day": {"utilization": 0.99, "resets_at": later}}
        state = {"providers": {"claude-2": {"usage": full}, "claude-3": {"usage": full}}}
        everyone = Secrets.of(secrets(*providers.SECRETS))
        self.assertIn("usage is 99%", providers.availability(
            pool.get("claude-2"), state, DAY, "America/Chicago", everyone))
        self.assertIsNone(providers.availability(
            pool.get("claude-3"), state, DAY, "America/Chicago", everyone))

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
        self.assertEqual(taken, [(3, "gpt"), (4, "agy"), (5, "muse")])
        # By day the Claude accounts are closed, and each machine subscription holds its lane.
        self.assertIn("`muse` is busy", planned["reason"])
        # Each claim with work and a lane left started the next run.
        self.assertEqual(len(gh.dispatches), 2)

    def test_the_machine_holds_three_and_github_takes_the_claude_accounts(self):
        """Machine runs stop at `machine_parallel`; a Claude account on GitHub's runners still
        takes work, up to `max_parallel` in all."""
        gh = FakeGitHub()
        for n in (3, 4, 5, 6):
            gh.add_issue(n, labels=(LABEL_BUILD,))
        ctx = ctx_for(gh, at=NIGHT, env=secrets("CLAUDE_CODE_OAUTH_TOKEN_2"), machine=MACHINE)
        for n, provider in ((40, "gpt"), (41, "agy"), (42, "muse")):
            gh.add_issue(n, labels=(LABEL_WORKING,))
            gh.runs[str(n)] = {"status": "in_progress"}
            ctx.store.update(lambda s, n=n, p=provider: state_item(s, n).update(run_id=str(n),
                                                                             provider=p))
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["provider"], planned["runs_on"]), ("claude-2", "ubuntu-latest"))
        # With the Claude account busy too, the next item waits for the machine.
        gh.runs["1"] = {"status": "in_progress"}
        ctx.store.update(lambda s: state_item(s, planned["number"]).update(run_id="1"))
        again = plan_mod.make(ctx)
        self.assertEqual(again["action"], "none")
        self.assertIn("`claude-2` is busy", again["reason"])

    def test_devin_works_on_two_items_at_once(self):
        """`lanes: 2`: Devin takes a second item while it holds one (two runners carry its
        label), and a third waits; the machine's limit still counts both."""
        gh = FakeGitHub()
        for n in (3, 4, 5):
            gh.add_issue(n, labels=(LABEL_BUILD,))
        ctx = ctx_for(gh, at=DAY, env=secrets(), machine=("devin",))
        self.assertEqual(ctx.cfg.pool.get("devin").lanes, 2)
        taken = []
        for _ in range(3):
            planned = plan_mod.make(ctx)
            if planned["action"] == "none":
                break
            gh.runs[str(planned["number"])] = {"status": "in_progress"}
            ctx.store.update(lambda s, n=planned["number"]: state_item(s, n).update(
                run_id=str(n)))
            taken.append((planned["number"], planned["provider"]))
        self.assertEqual(taken, [(3, "devin"), (4, "devin")])
        self.assertIn("`devin` is busy", planned["reason"])
        lanes = plan_mod.read_lanes(ctx, ctx.store.load())
        self.assertEqual((lanes.count("devin"), lanes.on_machine(ctx.cfg.pool)), (2, 2))

    def test_a_second_review_goes_to_another_model_family(self):
        gh = FakeGitHub()
        gh.add_pull(9, "bot/issue-5", labels=(LABEL_PR, LABEL_CROSS), sha="h1")
        ctx = ctx_for(gh, at=DAY)
        ctx.store.update(lambda s: state_item(s, 9).update(
            votes={"sha": "h1", "builder": "gpt", "approvals": ["gpt"]}))
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], planned["provider"], planned["head"], planned["builder"]),
                         ("review", "agy", "h1", "gpt"))
        self.assertEqual(gh.label_names(9), {LABEL_PR, LABEL_WORKING})

    def test_the_shared_subscription_needs_the_gates_word(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        env = secrets("CLAUDE_CODE_OAUTH_TOKEN")
        self.assertIn("waits for its owner to be quiet",
                      plan_mod.make(ctx_for(gh, env=env, machine=()), quiet_ok="")["reason"])
        self.assertEqual(plan_mod.make(ctx_for(gh, env=env, machine=()),
                                       quiet_ok="claude-1")["provider"],
                         "claude-1")


class PeekTests(unittest.TestCase):
    def test_the_gate_waits_for_quiet_only_when_the_shared_one_is_best(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        # Another Claude account can take it now: no wait for the shared one.
        look = plan_mod.peek(ctx_for(gh))
        self.assertEqual((look.work, look.provider, look.quiet_provider), (True, "claude-2", ""))
        # Opus is still worth the wait over another model, which stays the fallback.
        look = plan_mod.peek(ctx_for(gh, env=secrets("CLAUDE_CODE_OAUTH_TOKEN"), machine=("gpt",)))
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
        look = plan_mod.peek(ctx_for(gh, env=secrets("CLAUDE_CODE_OAUTH_TOKEN"), machine=()))
        self.assertFalse(look.work)
        self.assertIn("already waits for `claude-1`", look.reason)


class WhenTests(unittest.TestCase):
    def test_when_a_queued_item_runs(self):
        only_claude = Secrets.of(secrets("CLAUDE_CODE_OAUTH_TOKEN"))
        text = providers.when_free(test_pool(), {}, DAY, "America/Chicago", only_claude)
        self.assertIn("when `claude-1` opens (21:00–07:00", text)
        text = providers.when_free(providers.load(ROOT), {}, DAY, "America/Chicago", only_claude)
        self.assertIn("`gpt`, `agy`, `muse` can take it now", text)


if __name__ == "__main__":
    unittest.main()
