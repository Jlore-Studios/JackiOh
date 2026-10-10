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
        # The usage order: claude-3 (no caps), claude-7 and claude-1 first, then claude-4,
        # claude-6 and claude-5 last of the Claude accounts (each under its caps), then the medium models, Muse first (#317 part 9), then claude-2, kept for
        # planning and review, and Devin last. (The AI's training lanes run outside the
        # harness: no subscription of theirs.)
        self.assertEqual(pool.priority, ("claude-3", "claude-7", "claude-1", "claude-4",
                                         "claude-6", "claude-5", "muse", "agy", "gpt",
                                         "claude-2", "devin"))
        self.assertEqual((pool.max_parallel, pool.machine_parallel), (11, 6))
        self.assertEqual({p.cli for p in pool.ordered()}, set(providers.CLIS))
        self.assertEqual(len([p for p in pool.ordered() if p.cli == "claude"]), 7)
        first = pool.get("claude-1")
        self.assertEqual(first.secret, "CLAUDE_CODE_OAUTH_TOKEN")
        # No subscription waits for quiet any more: the bot spends claude-1 like the rest.
        self.assertFalse(any(p.quiet_check for p in pool.ordered()))
        seats = {p.id: [(s.model, s.tier, s.self_check) for s in pool.seats(p)]
                 for p in pool.ordered()}
        self.assertEqual(seats, {
            "claude-3": [("opus", "strong", False), ("sonnet", "medium", False)],
            "claude-1": [("opus", "strong", False), ("sonnet", "medium", False)],
            "claude-4": [("opus", "strong", False), ("sonnet", "medium", False)],
            "claude-5": [("opus", "strong", False), ("sonnet", "medium", False)],
            "claude-6": [("opus", "strong", False), ("sonnet", "medium", False)],
            "claude-7": [("opus", "strong", False), ("sonnet", "medium", False)],
            "agy": [("gemini-3.8-flash-high", "medium", False)],
            "muse": [("muse-spark-1.3-contributor", "medium", False)],
            "gpt": [("gpt-5.6-terra", "medium", False)],
            "claude-2": [("opus", "strong", False), ("sonnet", "medium", False)],
            "devin": [("swe-2-max", "weak", True)],
        })
        self.assertEqual([e.model for e in pool.tiers["weak"]], ["swe-2-max"])
        self.assertEqual([e.model for e in pool.tiers["medium"]],
                         ["muse-spark-1.3-contributor", "gemini-3.8-flash-high", "sonnet",
                          "gpt-5.6-terra"])
        # Every Claude account switches between its Opus and its Sonnet as the work needs: no
        # seat on them only stands in for another subscription.
        self.assertEqual({p.id: [s.takes_over for s in pool.seats(p)]
                          for p in pool.ordered() if p.cli == "claude"},
                         {name: ["", ""] for name in ("claude-1", "claude-2", "claude-3",
                                                      "claude-4", "claude-5", "claude-6",
                                                      "claude-7")})
        self.assertEqual([e.model for e in pool.tiers["strong"]], ["opus"])
        for provider in pool.ordered():
            self.assertFalse(hasattr(provider, "difficult") or hasattr(provider, "self_review"))
        self.assertNotIn("difficult_label", raw_providers())

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
        broken(lambda r: r["providers"]["gpt"].update(only_labels=["training"]))  # retired
        broken(lambda r: r["providers"]["gpt"].update(schedule={"mode": "window", "start": "9"}))
        broken(lambda r: r["providers"]["gpt"].update(roles=["build", "dance"]))
        broken(lambda r: r.update(priority=["claude-1"]))
        broken(lambda r: r.update(max_parallel=0))

    def test_tier_mistakes_are_errors(self):
        def broken(change, words) -> None:
            raw = raw_providers()
            change(raw)
            with self.assertRaisesRegex(ConfigError, words):
                providers.parse(raw)
        broken(lambda r: r["providers"]["gpt"].update(tier="genius"), "not one of weak")
        broken(lambda r: r["providers"]["gpt"].pop("tier"), "missing tier")
        broken(lambda r: r["providers"]["gpt"].update(tier="strong"), "tiers lists it under medium")
        broken(lambda r: r["providers"]["gpt"].update(model="gpt-9"), "does not list it")
        broken(lambda r: r["tiers"]["weak"].append({"model": "opus"}), "more than one place")
        broken(lambda r: r["tiers"].update(huge=[]), "unknown tiers")
        broken(lambda r: r.pop("tiers"), "expected an object")
        broken(lambda r: r["providers"]["claude-2"].update(
            extra_models=[{"model": "sonnet", "tier": "weak"}]), "tiers lists it under medium")
        broken(lambda r: r["providers"]["claude-2"].update(
            extra_models=[{"model": "sonnet", "tier": "medium", "takes_over": "nobody"}]),
            "which is not another provider")
        broken(lambda r: r["providers"]["claude-2"].update(extra_models=[{"tier": "weak"}]),
               "needs a model")
        broken(lambda r: r["providers"]["gpt"].update(difficult=True), "unknown keys difficult")
        broken(lambda r: r["providers"]["gpt"].update(self_review=True), "unknown keys self_review")

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
        broken(lambda r: r.update(machine_parallel=-1), "machine_parallel")
        # The machine's slots are its own, apart from `max_parallel`: it may have more.
        self.assertEqual(providers.parse({**raw_providers(), "max_parallel": 2}).machine_parallel,
                         6)
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
        # claude-4 works any hour: up to 70% in its 03:00–15:00 window, under 50% outside it.
        self.assertIsNone(self.why("claude-4"))
        self.assertIsNone(self.why("claude-4", at=DAY))
        # A window with no `off_hours` shuts outside its hours, unless forced.
        raw = raw_providers()
        del raw["providers"]["claude-4"]["off_hours"]
        windowed = providers.parse(raw)
        self.assertIn("outside its hours (03:00–15:00", self.why("claude-4", pool=windowed))
        self.assertIsNone(self.why("claude-4", forced=True, pool=windowed))
        self.assertIsNone(self.why("claude-4", at=DAY, pool=windowed))
        self.assertIsNone(self.why("claude-1", at=DAY))  # any hour, under its caps
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
        # claude-2 stops at 90% of its 5-hour session too; claude-1 goes on to 100%.
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
        state = {"usage": {"five_hour": {"utilization": 1.0, "resets_at": later}},
                 "providers": {}}
        self.assertIn("5-hour usage is 100%", self.why("claude-1", state))
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
                         (3, "muse", "muse", "", "machine", "night-vm-muse"))
        gh2 = FakeGitHub()
        gh2.add_issue(3, labels=(LABEL_BUILD,))
        planned = plan_mod.make(ctx_for(gh2, at=DAY, machine=("gpt",)))
        self.assertEqual((planned["provider"], planned["runs_on"]), ("gpt", "night-vm-gpt"))
        self.assertEqual(ctx_for(gh2).store.load()["items"]["3"]["provider"], "gpt")

    def test_devin_takes_easy_items_first_and_nothing_harder(self):
        """Devin (SWE-2) logs in on the machine, works any hour with no caps, is weak and checks
        its own builds. It is last in the usage order, but an easy item goes to it first while it
        has a free lane (`easy_first`), planned by a free medium or strong model on its own run."""
        pool = providers.load(ROOT)
        devin = pool.get("devin")
        self.assertEqual((devin.cli, devin.family, devin.login, devin.runs_on),
                         ("devin", "cognition", "machine", "night-vm-devin"))
        self.assertEqual((devin.schedule.mode, devin.limits.mode), ("always", "none"))
        self.assertEqual((devin.tier, devin.self_check, devin.easy_first), ("weak", True, True))
        self.assertEqual(pool.priority[-1], "devin")
        # Unplanned, Devin cannot take it: it builds only from a plan. By day no strong model is
        # free (the Claude accounts keep to the night here), so Muse plans it in its own run and
        # builds it.
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD, "difficulty:easy"))
        planned = plan_mod.make(ctx_for(gh, at=DAY, machine=ALL_MACHINE))
        self.assertEqual((planned["action"], planned["provider"]), ("build", "muse"))
        self.assertIn("#3 needs a plan", planned["housekeeping"])
        # A medium item passes Devin by.
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        planned = plan_mod.make(ctx_for(gh, at=DAY, machine=ALL_MACHINE))
        self.assertEqual((planned["action"], planned["provider"]), ("build", "muse"))
        # With nothing else free, Devin builds an easy item another model already planned.
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD, "difficulty:easy"))
        ctx = ctx_for(gh, at=DAY, machine=("devin",))
        self.assertEqual(plan_mod.make(ctx)["action"], "none")  # no planner is free
        # A medium model's plan is enough for an easy item (#317 part 6), and Devin builds it.
        ctx.store.update(lambda s: state_item(s, 3).update(planned_at=clock.iso(DAY),
                                                            planned_tier="medium"))
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], planned["provider"]), ("build", "devin"))
        self.assertEqual(planned["seats"]["self_check"], True)
        self.assertIsNone(planned["seats"]["review"])  # weak reviews only in a review run

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

    def test_claude_1_works_outside_its_hours_up_to_40_percent(self):
        """claude-1's window is 21:00–07:00 under a 100% cap on its 5-hour session with no
        weekly cap; outside it, it still works while its 5-hour usage is under 40%
        (`off_hours`), and a run there stops past 40%."""
        claude_1 = providers.load(ROOT).get("claude-1")
        self.assertEqual(claude_1.hours("America/Chicago"),
                         "21:00–07:00 America/Chicago, outside them up to 40% of 5-hour")
        later = clock.iso(DAY + timedelta(hours=12))  # past both checks below
        everyone = Secrets.of(secrets(*providers.SECRETS))

        def why(at, used):
            entry = {"usage": {"five_hour": {"utilization": used, "resets_at": later}}}
            state = {"providers": {"claude-1": entry}}
            return providers.availability(claude_1, state, at, "America/Chicago", everyone)

        night = DAY + timedelta(hours=10)  # 22:00 CDT, inside the window
        self.assertIsNone(why(DAY, 0.3))
        self.assertIn("5-hour usage is 45%, at or over its 40% cap outside its hours",
                      why(DAY, 0.45))
        self.assertIsNone(why(night, 0.45))
        self.assertIsNone(why(night, 0.99))  # it may spend its whole session in its hours
        self.assertIn("at or over its 100% cap; ", why(night, 1.0))
        # The mid-run stop holds the same caps.
        entry = {"usage": {"five_hour": {"utilization": 0.41, "resets_at": later}}}
        self.assertIsNotNone(providers.refusal(claude_1, entry, DAY, "America/Chicago"))
        self.assertIsNone(providers.refusal(claude_1, entry, night, "America/Chicago"))
        raw = raw_providers()
        raw["providers"]["claude-1"]["off_hours"] = {"five_hours": 0.4}
        with self.assertRaises(ConfigError):
            providers.parse(raw)

    def test_claude_4_works_any_hour_under_half_and_up_to_70_percent_from_3_to_15(self):
        """claude-4's window is 03:00–15:00 Central under a 70% cap on its 5-hour session with
        no weekly cap; outside it, it still works while that session is under 50%
        (`off_hours`)."""
        claude_4 = providers.load(ROOT).get("claude-4")
        self.assertEqual((claude_4.secret, claude_4.schedule.mode,
                          dict(claude_4.limits.stops), dict(claude_4.off_hours)),
                         ("CLAUDE_CODE_OAUTH_TOKEN_4", "window",
                          {"five_hour": 0.7}, {"five_hour": 0.5}))
        self.assertEqual(claude_4.hours("America/Chicago"),
                         "03:00–15:00 America/Chicago, outside them up to 50% of 5-hour")
        later = clock.iso(DAY + timedelta(days=1))  # past every check below
        everyone = Secrets.of(secrets(*providers.SECRETS))

        def why(at, five_hour, seven_day=0.1, starting=False):
            entry = {"usage": {"five_hour": {"utilization": five_hour, "resets_at": later},
                               "seven_day": {"utilization": seven_day, "resets_at": later}}}
            return providers.availability(claude_4, {"providers": {"claude-4": entry}}, at,
                                          "America/Chicago", everyone, starting=starting)

        morning = DAY - timedelta(hours=2)  # 10:00 CDT, inside the window
        evening = DAY + timedelta(hours=5)  # 17:00 CDT, outside it
        night = DAY + timedelta(hours=10)  # 22:00 CDT, outside it
        self.assertIsNone(why(morning, 0.6))
        # No weekly cap: a nearly spent week holds nothing back, in or out of the window.
        self.assertIsNone(why(morning, 0.1, seven_day=0.99))
        self.assertIn("5-hour usage is 72%, at or over its 70% cap; ", why(morning, 0.72))
        for outside in (evening, night):
            self.assertIsNone(why(outside, 0.3))
            self.assertIsNone(why(outside, 0.3, seven_day=0.99))
            self.assertIn("5-hour usage is 55%, at or over its 50% cap outside its hours",
                          why(outside, 0.55))
        # A build starts only `start_headroom` under the cap that holds then.
        self.assertIsNone(why(morning, 0.5, starting=True))
        self.assertIn("too close to its 70% cap", why(morning, 0.67, starting=True))
        self.assertIn("too close to its 50% cap outside its hours",
                      why(evening, 0.47, starting=True))
        # A run going on stops at the cap that holds when it is checked.
        entry = {"usage": {"five_hour": {"utilization": 0.55, "resets_at": later}}}
        self.assertIsNone(providers.refusal(claude_4, entry, morning, "America/Chicago"))
        self.assertIsNotNone(providers.refusal(claude_4, entry, evening, "America/Chicago"))

    def test_claude_6_matches_claude_4_with_no_weekly_cap(self):
        """claude-6 runs under the same rules as claude-4: the 03:00–15:00 window up to 70% of
        its 5-hour session, under 50% outside it, and no weekly cap."""
        pool = providers.load(ROOT)
        claude_6 = pool.get("claude-6")
        claude_4 = pool.get("claude-4")
        self.assertEqual(claude_6.secret, "CLAUDE_CODE_OAUTH_TOKEN_6")
        self.assertEqual((claude_6.schedule, dict(claude_6.limits.stops),
                          dict(claude_6.off_hours)),
                         (claude_4.schedule, dict(claude_4.limits.stops),
                          dict(claude_4.off_hours)))
        self.assertEqual(claude_6.hours("America/Chicago"),
                         claude_4.hours("America/Chicago"))
        later = clock.iso(DAY + timedelta(days=1))
        everyone = Secrets.of(secrets(*providers.SECRETS))

        def why(at, five_hour, seven_day=0.1):
            entry = {"usage": {"five_hour": {"utilization": five_hour, "resets_at": later},
                               "seven_day": {"utilization": seven_day, "resets_at": later}}}
            return providers.availability(claude_6, {"providers": {"claude-6": entry}}, at,
                                          "America/Chicago", everyone)

        morning = DAY - timedelta(hours=2)  # 10:00 CDT, inside the window
        evening = DAY + timedelta(hours=5)  # 17:00 CDT, outside it
        self.assertIsNone(why(morning, 0.6, seven_day=0.99))
        self.assertIn("5-hour usage is 72%, at or over its 70% cap; ", why(morning, 0.72))
        self.assertIsNone(why(evening, 0.3, seven_day=0.99))
        self.assertIn("5-hour usage is 55%, at or over its 50% cap outside its hours",
                      why(evening, 0.55))

    def test_claude_7_works_any_hour_under_half_and_up_to_70_percent_from_1_to_10(self):
        """claude-7's window is 01:00–10:00 Central under a 70% cap on its 5-hour session with
        no weekly cap; from 10:00 to 01:00 it still works while that session is under 50%
        (`off_hours`)."""
        claude_7 = providers.load(ROOT).get("claude-7")
        self.assertEqual((claude_7.secret, claude_7.schedule.mode,
                          dict(claude_7.limits.stops), dict(claude_7.off_hours)),
                         ("CLAUDE_CODE_OAUTH_TOKEN_7", "window",
                          {"five_hour": 0.7}, {"five_hour": 0.5}))
        self.assertEqual(claude_7.hours("America/Chicago"),
                         "01:00–10:00 America/Chicago, outside them up to 50% of 5-hour")
        later = clock.iso(DAY + timedelta(days=1))  # past every check below
        everyone = Secrets.of(secrets(*providers.SECRETS))

        def why(at, five_hour, seven_day=0.1, starting=False):
            entry = {"usage": {"five_hour": {"utilization": five_hour, "resets_at": later},
                               "seven_day": {"utilization": seven_day, "resets_at": later}}}
            return providers.availability(claude_7, {"providers": {"claude-7": entry}}, at,
                                          "America/Chicago", everyone, starting=starting)

        early = DAY - timedelta(hours=10, minutes=30)  # 01:30 CDT, inside the window
        morning = DAY - timedelta(hours=2, minutes=30)  # 09:30 CDT, inside it
        late_morning = DAY - timedelta(hours=2)  # 10:00 CDT, the window has closed
        midnight = DAY + timedelta(hours=12, minutes=30)  # 00:30 CDT, outside it
        for inside in (early, morning):
            self.assertIsNone(why(inside, 0.6))
            # No weekly cap: a nearly spent week holds nothing back, in or out of the window.
            self.assertIsNone(why(inside, 0.1, seven_day=0.99))
            self.assertIn("5-hour usage is 72%, at or over its 70% cap; ", why(inside, 0.72))
        for outside in (late_morning, DAY, midnight):
            self.assertIsNone(why(outside, 0.3))
            self.assertIsNone(why(outside, 0.3, seven_day=0.99))
            self.assertIn("5-hour usage is 55%, at or over its 50% cap outside its hours",
                          why(outside, 0.55))
        # A build starts only `start_headroom` under the cap that holds then.
        self.assertIsNone(why(morning, 0.5, starting=True))
        self.assertIn("too close to its 70% cap", why(morning, 0.67, starting=True))
        self.assertIn("too close to its 50% cap outside its hours",
                      why(DAY, 0.47, starting=True))
        # A run going on stops at the cap that holds when it is checked.
        entry = {"usage": {"five_hour": {"utilization": 0.55, "resets_at": later}}}
        self.assertIsNone(providers.refusal(claude_7, entry, morning, "America/Chicago"))
        self.assertIsNotNone(providers.refusal(claude_7, entry, DAY, "America/Chicago"))
        # Without its secret it takes nothing.
        self.assertIn("CLAUDE_CODE_OAUTH_TOKEN_7", providers.availability(
            claude_7, {}, NIGHT, "America/Chicago", Secrets.of(secrets("CLAUDE_CODE_OAUTH_TOKEN"))))

    def test_claude_5_works_any_hour_under_40_percent_of_its_session_and_60_of_its_week(self):
        claude_5 = providers.load(ROOT).get("claude-5")
        self.assertEqual((claude_5.secret, claude_5.schedule.mode, dict(claude_5.limits.stops)),
                         ("CLAUDE_CODE_OAUTH_TOKEN_5", "always",
                          {"five_hour": 0.4, "seven_day": 0.6}))
        later = clock.iso(DAY + timedelta(days=1))
        everyone = Secrets.of(secrets(*providers.SECRETS))

        def why(at, five_hour, seven_day=0.1, starting=False):
            entry = {"usage": {"five_hour": {"utilization": five_hour, "resets_at": later},
                               "seven_day": {"utilization": seven_day, "resets_at": later}}}
            return providers.availability(claude_5, {"providers": {"claude-5": entry}}, at,
                                          "America/Chicago", everyone, starting=starting)

        for at in (DAY, NIGHT):  # the same caps by day and by night
            self.assertIsNone(why(at, 0.35))
            self.assertIn("5-hour usage is 41%, at or over its 40% cap; ", why(at, 0.41))
            self.assertIsNone(why(at, 0.1, seven_day=0.55))
            self.assertIn("7-day usage is 61%, at or over its 60% cap; ",
                          why(at, 0.1, seven_day=0.61))
        # A build starts only `start_headroom` under the caps: under 35% and 55%.
        self.assertIsNone(why(DAY, 0.3, starting=True))
        self.assertIn("too close to its 40% cap", why(DAY, 0.37, starting=True))
        self.assertIn("too close to its 60% cap", why(DAY, 0.1, seven_day=0.57, starting=True))

    def test_the_medium_models_may_spend_their_whole_week(self):
        pool = providers.load(ROOT)
        self.assertEqual({pid: pool.get(pid).limits.stops.get("seven_day")
                          for pid in ("gpt", "agy", "muse")},
                         {"gpt": 1.0, "agy": 1.0, "muse": 1.0})

    def test_claude_2_and_3_work_any_hour(self):
        """The committed hours: claude-1, claude-2 and claude-3 run all day, claude-1 under its
        100% cap with no weekly cap, claude-2 under 90% and claude-3 with none; claude-4 and
        claude-6 run all day too, up to 70% of their 5-hour session from 03:00 to 15:00 and
        under 50% otherwise, claude-7 the same from 01:00 to 10:00, all three with no weekly
        cap, and claude-5 under 40%/60% (their own tests are above). By day claude-3, claude-7,
        claude-1, claude-4, claude-6 and then claude-5
        take Opus's
        work first, and claude-2, kept back, plans for the medium models."""
        pool = providers.load(ROOT)
        hours = {p.id: (p.schedule.mode, p.limits.mode) for p in pool.ordered() if p.cli == "claude"}
        self.assertEqual(hours, {"claude-1": ("window", "caps"), "claude-2": ("always", "caps"),
                                 "claude-3": ("always", "none"), "claude-4": ("window", "caps"),
                                 "claude-5": ("always", "caps"),
                                 "claude-6": ("window", "caps"),
                                 "claude-7": ("window", "caps")})
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        gh.add_issue(4, labels=(LABEL_BUILD, "difficulty:hard"))
        ctx = ctx_for(gh, at=DAY, committed_hours=True)
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["number"], planned["provider"], planned["action"]),
                         (4, "claude-3", "build"))
        gh.runs["1"] = {"status": "in_progress"}
        ctx.store.update(lambda s: state_item(s, 4).update(run_id="1"))
        planned = plan_mod.make(ctx)
        # claude-3's second lane takes the next one too.
        self.assertEqual((planned["number"], planned["provider"], planned["action"]),
                         (3, "claude-3", "build"))
        gh.runs["2"] = {"status": "in_progress"}
        ctx.store.update(lambda s: state_item(s, 3).update(run_id="2"))
        gh.add_issue(11, labels=(LABEL_BUILD,))
        planned = plan_mod.make(ctx)
        # claude-7, next in the usage order, takes the next (under 50% outside 01:00–10:00).
        self.assertEqual((planned["number"], planned["provider"], planned["action"]),
                         (11, "claude-7", "build"))
        gh.runs["11"] = {"status": "in_progress"}
        ctx.store.update(lambda s: state_item(s, 11).update(run_id="11"))
        gh.add_issue(12, labels=(LABEL_BUILD,))
        planned = plan_mod.make(ctx)
        # claude-7's second lane takes the next one too.
        self.assertEqual((planned["number"], planned["provider"], planned["action"]),
                         (12, "claude-7", "build"))
        gh.runs["12"] = {"status": "in_progress"}
        ctx.store.update(lambda s: state_item(s, 12).update(run_id="12"))
        gh.add_issue(5, labels=(LABEL_BUILD,))
        planned = plan_mod.make(ctx)
        # claude-1 works by day too, and plans its own build.
        self.assertEqual((planned["number"], planned["provider"], planned["action"]),
                         (5, "claude-1", "build"))
        gh.runs["3"] = {"status": "in_progress"}
        ctx.store.update(lambda s: state_item(s, 5).update(run_id="3"))
        gh.add_issue(6, labels=(LABEL_BUILD,))
        planned = plan_mod.make(ctx)
        # claude-1's second lane takes the next one too.
        self.assertEqual((planned["number"], planned["provider"], planned["action"]),
                         (6, "claude-1", "build"))
        gh.runs["4"] = {"status": "in_progress"}
        ctx.store.update(lambda s: state_item(s, 6).update(run_id="4"))
        gh.add_issue(7, labels=(LABEL_BUILD,))
        planned = plan_mod.make(ctx)
        # claude-4 works by day too (under 70% until 15:00), next in the usage order.
        self.assertEqual((planned["number"], planned["provider"], planned["action"]),
                         (7, "claude-4", "build"))
        gh.runs["5"] = {"status": "in_progress"}
        ctx.store.update(lambda s: state_item(s, 7).update(run_id="5"))
        gh.add_issue(8, labels=(LABEL_BUILD,))
        planned = plan_mod.make(ctx)
        # claude-6, under the same rules as claude-4, comes next.
        self.assertEqual((planned["number"], planned["provider"], planned["action"]),
                         (8, "claude-6", "build"))
        gh.runs["6"] = {"status": "in_progress"}
        ctx.store.update(lambda s: state_item(s, 8).update(run_id="6"))
        gh.add_issue(9, labels=(LABEL_BUILD,))
        planned = plan_mod.make(ctx)
        # claude-5, last of the Claude accounts, comes after claude-6.
        self.assertEqual((planned["number"], planned["provider"], planned["action"]),
                         (9, "claude-5", "build"))
        gh.runs["7"] = {"status": "in_progress"}
        ctx.store.update(lambda s: state_item(s, 9).update(run_id="7"))
        gh.add_issue(10, labels=(LABEL_BUILD,))
        planned = plan_mod.make(ctx)
        # #10 builds on Muse; with the planning lane off here (test_needs_plan.py has it), it
        # plans in its own run, on its medium model.
        self.assertEqual((planned["number"], planned["provider"], planned["action"]),
                         (10, "muse", "build"))
        self.assertEqual(planned["seats"]["plan"]["tier"], "medium")
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

    def test_hard_work_waits_for_a_strong_model_and_comes_first(self):
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        gh.add_issue(4, labels=(LABEL_BUILD, "difficulty:hard"))
        planned = plan_mod.make(ctx_for(gh))
        self.assertEqual((planned["number"], planned["provider"]), (4, "claude-3"))
        gh = FakeGitHub()
        gh.add_issue(4, labels=(LABEL_BUILD, "difficulty:hard"))
        planned = plan_mod.make(ctx_for(gh, at=DAY))
        self.assertEqual(planned["action"], "none")
        self.assertIn("outside its hours", planned["reason"])
        self.assertEqual(plan_mod.make(ctx_for(gh, at=DAY), force=True)["provider"], "claude-3")

    def test_each_subscription_takes_as_many_items_as_its_lanes(self):
        gh = FakeGitHub()
        for n in (3, 4, 5, 6, 7, 8, 9):
            gh.add_issue(n, labels=(LABEL_BUILD,))
        ctx = ctx_for(gh, at=DAY)
        taken = []
        for _ in range(7):
            planned = plan_mod.make(ctx)
            if planned["action"] == "none":
                break
            gh.runs[str(len(taken))] = {"status": "in_progress"}
            ctx.store.update(lambda s, n=planned["number"], r=str(len(taken)): state_item(
                s, n).update(run_id=r))
            taken.append((planned["number"], planned["provider"]))
        # Muse, first of the medium models, has four lanes, so it takes four items before agy
        # and gpt, next in the usage order.
        self.assertEqual(taken, [(3, "muse"), (4, "muse"), (5, "muse"), (6, "muse"), (7, "agy"),
                                 (8, "gpt")])
        # By day the Claude accounts are closed, and each machine subscription holds its lanes.
        self.assertIn("`muse` is busy", planned["reason"])
        # Each claim with work and a lane left started the next run.
        self.assertEqual(len(gh.dispatches), 5)

    def test_with_the_machine_subscriptions_full_github_takes_the_claude_accounts(self):
        """Each machine subscription holds as many runs as its lanes (Muse four); a Claude account
        on GitHub's runners still takes work, up to `max_parallel` in all."""
        gh = FakeGitHub()
        for n in (3, 4, 5, 6):
            gh.add_issue(n, labels=(LABEL_BUILD,))
        ctx = ctx_for(gh, at=NIGHT, env=secrets("CLAUDE_CODE_OAUTH_TOKEN_2"), machine=MACHINE)
        for n, provider in ((40, "gpt"), (41, "agy"), (42, "muse"), (43, "muse"), (44, "muse"),
                            (45, "muse")):
            gh.add_issue(n, labels=(LABEL_WORKING,))
            gh.runs[str(n)] = {"status": "in_progress"}
            ctx.store.update(lambda s, n=n, p=provider: state_item(s, n).update(run_id=str(n),
                                                                             provider=p))
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["provider"], planned["runs_on"]), ("claude-2", "ubuntu-latest"))
        # With two lanes the Claude account takes a second item too; only when both lanes
        # are held does the next item wait.
        gh.runs["1"] = {"status": "in_progress"}
        ctx.store.update(lambda s: state_item(s, planned["number"]).update(run_id="1"))
        again = plan_mod.make(ctx)
        self.assertEqual((again["action"], again["provider"]), ("build", "claude-2"))
        gh.runs["2"] = {"status": "in_progress"}
        ctx.store.update(lambda s: state_item(s, again["number"]).update(run_id="2"))
        third = plan_mod.make(ctx)
        self.assertEqual(third["action"], "none")
        self.assertIn("`claude-2` is busy", third["reason"])

    def test_devin_can_fill_the_machine(self):
        """`lanes: 6`: Devin takes more items while it holds some (six runners carry its
        label); a seventh waits on its lanes. The night box's limit of six counts every
        subscription's runs, so with another one there Devin gets fewer."""
        gh = FakeGitHub()
        items = (3, 4, 5, 6, 7, 8, 9)
        for n in items:
            gh.add_issue(n, labels=(LABEL_BUILD, "difficulty:easy"))  # Devin is weak
        ctx = ctx_for(gh, at=DAY, env=secrets(), machine=("devin",))
        ctx.store.update(lambda s: [state_item(s, n).update(planned_at=clock.iso(DAY))
                                    for n in items])
        self.assertEqual((ctx.cfg.pool.get("devin").lanes, ctx.cfg.pool.machine_parallel), (6, 6))
        taken = []
        for _ in range(7):
            planned = plan_mod.make(ctx)
            if planned["action"] == "none":
                break
            gh.runs[str(planned["number"])] = {"status": "in_progress"}
            ctx.store.update(lambda s, n=planned["number"]: state_item(s, n).update(
                run_id=str(n)))
            taken.append((planned["number"], planned["provider"]))
        self.assertEqual(taken, [(n, "devin") for n in items[:6]])
        self.assertIn("`devin` is busy", planned["reason"])
        lanes = plan_mod.read_lanes(ctx, ctx.store.load())
        self.assertEqual((lanes.count("devin"), lanes.on_machine(ctx.cfg.pool)), (6, 6))
        # With GPT holding a machine run, Devin takes five and waits for room on the box.
        gh = FakeGitHub()
        for n in items:
            gh.add_issue(n, labels=(LABEL_BUILD, "difficulty:easy"))
        ctx = ctx_for(gh, at=DAY, env=secrets(), machine=("devin", "gpt"))
        ctx.store.update(lambda s: [state_item(s, n).update(planned_at=clock.iso(DAY))
                                    for n in items])
        gh.add_issue(40, labels=(LABEL_WORKING,))
        gh.runs["40"] = {"status": "in_progress"}
        ctx.store.update(lambda s: state_item(s, 40).update(run_id="40", provider="gpt"))
        devins = 0
        for _ in range(7):
            planned = plan_mod.make(ctx)
            if planned["action"] == "none":
                break
            gh.runs[str(planned["number"])] = {"status": "in_progress"}
            ctx.store.update(lambda s, n=planned["number"]: state_item(s, n).update(
                run_id=str(n)))
            devins += planned["provider"] == "devin"
        self.assertEqual(devins, 5)
        self.assertIn("`devin` waits for room on the machine (6 at once)", planned["reason"])

    def test_a_second_review_goes_to_another_model_family(self):
        gh = FakeGitHub()
        gh.add_pull(9, "bot/issue-5", labels=(LABEL_PR, LABEL_CROSS), sha="h1")
        ctx = ctx_for(gh, at=DAY)
        ctx.store.update(lambda s: state_item(s, 9).update(
            votes={"sha": "h1", "builder": "gpt", "approvals": ["gpt"]}))
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], planned["provider"], planned["head"], planned["builder"]),
                         ("review", "muse", "h1", "gpt"))
        self.assertEqual(gh.label_names(9), {LABEL_PR, LABEL_WORKING})

    def test_the_shared_subscription_takes_work_without_a_wait(self):
        """claude-1 is spent like any other subscription now: no gate wait for quiet, so the
        gate's word changes nothing."""
        env = secrets("CLAUDE_CODE_OAUTH_TOKEN")
        for quiet_ok in ("", "claude-1"):
            gh = FakeGitHub()
            gh.add_issue(3, labels=(LABEL_BUILD,))
            planned = plan_mod.make(ctx_for(gh, env=env, machine=()), quiet_ok=quiet_ok)
            self.assertEqual((planned["action"], planned["provider"]), ("build", "claude-1"))


class PeekTests(unittest.TestCase):
    def test_the_gate_never_waits_for_quiet(self):
        """No subscription waits for quiet, so peek never names one: not with every secret,
        not with only the shared subscription's, not by day."""
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        look = plan_mod.peek(ctx_for(gh))
        self.assertEqual((look.work, look.provider, look.quiet_provider), (True, "claude-3", ""))
        look = plan_mod.peek(ctx_for(gh, env=secrets("CLAUDE_CODE_OAUTH_TOKEN"), machine=("gpt",)))
        self.assertEqual((look.work, look.provider, look.quiet_provider, look.quiet_secret,
                          look.fallback),
                         (True, "claude-1", "", "", False))
        look = plan_mod.peek(ctx_for(gh, at=DAY))
        self.assertEqual((look.provider, look.quiet_provider), ("muse", ""))

    def test_runs_never_wait_for_quiet(self):
        """Even a run sitting at the quiet step holds nothing back: with no subscription
        waiting, a second run goes ahead on the shared one too."""
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        gh.runs["50"] = {"id": 50, "status": "in_progress"}
        gh.jobs["50"] = [{"steps": [{"name": plan_mod.QUIET_STEP, "status": "in_progress"}]}]
        look = plan_mod.peek(ctx_for(gh))
        self.assertEqual((look.work, look.provider, look.quiet_provider), (True, "claude-3", ""))
        look = plan_mod.peek(ctx_for(gh, env=secrets("CLAUDE_CODE_OAUTH_TOKEN"), machine=()))
        self.assertEqual((look.work, look.provider, look.quiet_provider), (True, "claude-1", ""))


#: 2026-10-09 06:00 UTC is 01:00 CDT: outside claude-4's and claude-6's 03:00–15:00 window.
EARLY = datetime(2026, 10, 9, 6, 0, tzinfo=timezone.utc)


def too_close_to_start() -> dict:
    """The night claude-4 and claude-6 sat idle with work queued: outside their hours, under
    their 50% cap there, but not 5 points under it (`start_headroom`); every other subscription
    refused until later."""
    later = clock.iso(EARLY + timedelta(hours=6))
    state: dict = {"providers": {pid: {"refused_until": later}
                                 for pid in raw_providers()["providers"]}}
    state["providers"]["claude-4"] = {"usage": {
        "five_hour": {"utilization": 0.49, "resets_at": "2026-10-09T09:10:00Z"},
        "observed_at": "2026-10-09T04:37:07Z"}}
    state["providers"]["claude-6"] = {"usage": {
        "five_hour": {"utilization": 0.45, "resets_at": "2026-10-09T07:50:00Z"},
        "observed_at": "2026-10-09T03:34:44Z"}}
    return state


class StartTests(unittest.TestCase):
    """A subscription under its cap but too close to it to start a build, a revision or a plan
    says so, says it still takes reviews, and says when it can start one again."""

    def setUp(self):
        self.pool = providers.load(ROOT)
        self.state = too_close_to_start()
        self.everyone = Secrets.of(secrets(*providers.SECRETS))

    def test_too_close_to_start_says_reviews_only_and_when_it_can(self):
        for pid, frees, wait in (("claude-4", "2026-10-09T08:00:00Z", "2h 00m"),
                                 ("claude-6", "2026-10-09T07:50:00Z", "1h 50m")):
            provider = self.pool.get(pid)
            # A review goes up to the cap, so it may start one.
            self.assertIsNone(providers.availability(provider, self.state, EARLY,
                                                     "America/Chicago", self.everyone))
            why = providers.start_reason(provider, self.state, EARLY, "America/Chicago",
                                         self.everyone)
            self.assertIn("too close to its 50% cap outside its hours to start a long run "
                          "(it starts under 45%)", why)
            self.assertIn(f"; it takes reviews only; it can start a build or a plan again in "
                          f"{wait}", why)
            entry = providers.peek_record(self.state, pid)
            # claude-4 frees when its window opens (70% cap, starts under 65%), before its reset;
            # claude-6 when its session resets, before its window opens.
            self.assertEqual(clock.iso(providers.frees_at(provider, entry, EARLY,
                                                          "America/Chicago")), frees)
        # Something no reset lifts gets no time.
        held = {"providers": {"claude-4": {"infra": {"at": clock.iso(EARLY), "reason": "401"}}}}
        why = providers.start_reason(self.pool.get("claude-4"), held, EARLY, "America/Chicago",
                                     self.everyone)
        self.assertIn("could not work", why)
        self.assertNotIn("again in", why)

    def test_a_queued_item_and_the_planner_say_the_same(self):
        text = providers.when_free(test_pool(committed_hours=True), self.state, EARLY,
                                   "America/Chicago", self.everyone)
        self.assertEqual(text, "when `claude-6` has room under its caps again (in 1h 50m)")
        gh = FakeGitHub()
        gh.add_issue(3, labels=(LABEL_BUILD,))
        ctx = ctx_for(gh, at=EARLY, machine=(), committed_hours=True)
        ctx.store.update(lambda s: s["providers"].update(self.state["providers"]), "seed")
        planned = plan_mod.make(ctx)
        self.assertEqual(planned["action"], "none")
        self.assertIn("`claude-4` 5-hour usage is 49%, too close to its 50% cap", planned["reason"])
        self.assertIn("it can start a build or a plan again in 2h 00m", planned["reason"])
        self.assertNotIn("is free", planned["reason"])


class WhenTests(unittest.TestCase):
    def test_when_a_queued_item_runs(self):
        only_claude = Secrets.of(secrets("CLAUDE_CODE_OAUTH_TOKEN"))
        text = providers.when_free(test_pool(), {}, DAY, "America/Chicago", only_claude)
        self.assertIn("when `claude-1` opens (21:00–07:00", text)
        text = providers.when_free(providers.load(ROOT), {}, DAY, "America/Chicago", only_claude)
        self.assertIn("`muse`, `agy`, `gpt`, `devin` can take it now", text)


if __name__ == "__main__":
    unittest.main()
