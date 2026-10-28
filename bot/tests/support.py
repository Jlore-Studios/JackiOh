"""Shared setup: a Config for tests and a Context around a FakeGitHub."""

from __future__ import annotations

import dataclasses
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from harness import config as config_mod
from harness import providers as providers_mod
from harness.context import Context
from harness.state import StateStore
from harness.trust import Trust

from tests.fakes import FakeGitHub

ROOT = config_mod.REPO_ROOT
# 2026-09-30 03:00 UTC is 22:00 CDT on the 29th: inside the night window.
NIGHT = datetime(2026, 9, 30, 3, 0, tzinfo=timezone.utc)
# 2026-09-29 17:00 UTC is 12:00 CDT: outside it.
DAY = datetime(2026, 9, 29, 17, 0, tzinfo=timezone.utc)

#: Only the first Claude account's secret is set, as before there were several subscriptions;
#: tests of the others set `HARNESS_SECRETS_SET` themselves.
TEST_ENV = {"GITHUB_RUN_ID": "777", "GITHUB_REPOSITORY": "jgoetzmann/JackiOh",
            "HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN"}


def raw_config() -> dict[str, Any]:
    return json.loads((ROOT / config_mod.CONFIG_PATH).read_text(encoding="utf-8"))


def raw_providers() -> dict[str, Any]:
    return json.loads((ROOT / providers_mod.PROVIDERS_PATH).read_text(encoding="utf-8"))


#: A weak subscription for the tests alone (#2129): no committed one is weak any more, so the weak
#: tier's rules (weak first for an easy item, `easy_first`, `self_check`, the easy rule) keep one
#: to hold them to. It is what the weak seat was: on the machine, any hour, no caps, six lanes,
#: last in the usage order, its builds checking themselves.
WEAK = "lite"
WEAK_MODEL = "lite-1"
WEAK_PROVIDER: dict[str, Any] = {
    "note": "The tests' weak subscription (tests/support.py).", "enabled": True, "cli": "codex",
    "family": "lite", "model": WEAK_MODEL, "effort": "", "tier": "weak", "self_check": True,
    "easy_first": True, "login": "machine", "runs_on": f"night-vm-{WEAK}", "lanes": 6,
    "schedule": {"mode": "always"}, "limits": {"mode": "none"}, "quiet_check": False}


def with_weak(raw: dict[str, Any]) -> dict[str, Any]:
    """`raw` (providers.json's JSON) with the tests' weak subscription (`WEAK`), last in the usage
    order, its model the weak tier's."""
    raw["providers"][WEAK] = json.loads(json.dumps(WEAK_PROVIDER))
    raw["priority"] = [*raw["priority"], WEAK]
    raw.setdefault("tiers", {})["weak"] = [{"model": WEAK_MODEL}]
    return raw


#: The machine subscriptions the tests switch on by default: no secret switches them on or off.
#: The weak one (`WEAK`, behind them in `priority`) is named by the tests about it, so the tests
#: of the others stay about them.
MACHINE: tuple[str, ...] = ("gpt", "agy", "muse")
#: Every subscription logged in on the machine, the weak one's included.
ALL_MACHINE: tuple[str, ...] = tuple(name for name, p in with_weak(raw_providers())["providers"]
                                     .items() if p.get("login") == "machine")


#: The hours every Claude account has in tests unless a test asks for the committed ones.
NIGHT_WINDOW = {"mode": "window", "start": "21:00", "end": "07:00"}


def test_pool(machine: tuple[str, ...] = (), committed_hours: bool = False,
              plan_lanes: int | None = 0) -> providers_mod.Pool:
    """The committed subscriptions and the tests' weak one (`WEAK`), with the machine's
    (`"login": "machine"`) switched off
    unless named in `machine`, and on when named. A machine login has no secret to be missing, so it counts as set
    up in every run; tests start, as before the machine, with only the subscriptions whose
    secrets `HARNESS_SECRETS_SET` names.

    Every Claude account keeps to the night window, whatever hours the owner commits for it
    (`claude-3` runs all day), so a test of the other subscriptions by day stays about them;
    `committed_hours` keeps the committed ones.

    The planning lane (`plan_lanes`) is off, so a test of building finds the builder planning in
    its own run as before; a test of the Needs plan stage turns it on (`plan_lanes=None` keeps the
    committed number)."""
    raw = with_weak(raw_providers())
    if plan_lanes is not None:
        raw["plan_lanes"] = plan_lanes
    for name, provider in raw["providers"].items():
        if provider.get("login") == "machine":
            # Named ones are on whatever the owner commits, so a test of a machine subscription
            # stays about it.
            provider["enabled"] = name in machine
        if provider.get("cli") == "claude" and not committed_hours:
            provider["schedule"] = dict(NIGHT_WINDOW)
            provider.pop("off_hours", None)
    return providers_mod.parse(raw)


def with_lanes(cfg: Any, max_parallel: int, machine_parallel: int | None = None) -> Any:
    """`cfg` with other run limits, for a test about lanes filling up."""
    machine = max_parallel if machine_parallel is None else machine_parallel
    pool = dataclasses.replace(cfg.pool, max_parallel=max_parallel, machine_parallel=machine)
    return dataclasses.replace(cfg, pool=pool)


def secret_login(provider: providers_mod.Provider, secret: str) -> providers_mod.Provider:
    """`provider` logged in from a secret on GitHub's runners instead of on the machine."""
    return dataclasses.replace(provider, login="secret", secret=secret, runs_on="ubuntu-latest")


def make_config(root: Path | None = None, env: dict[str, str] | None = None,
                machine: tuple[str, ...] = (), committed_hours: bool = False,
                plan_lanes: int | None = 0, **overrides: Any):
    raw = raw_config()
    raw.update(overrides)
    return config_mod.parse(raw, root or ROOT, {**TEST_ENV, **(env or {})},
                            pool=test_pool(machine, committed_hours, plan_lanes))


class Clock:
    def __init__(self, at: datetime = NIGHT) -> None:
        self.at = at

    def __call__(self) -> datetime:
        return self.at


def make_ctx(gh: FakeGitHub | None = None, *, at: datetime = NIGHT, cfg=None,
             trust_text: str | None = None) -> Context:
    gh = gh or FakeGitHub()
    cfg = cfg or make_config()
    trust = Trust.parse(trust_text) if trust_text is not None else Trust.load(ROOT / config_mod.TRUST_PATH)
    return Context(cfg=cfg, gh=gh, store=StateStore(gh), trust=trust, clock_fn=Clock(at))


def idle_for(ctx: Context, minutes: int = 31) -> None:
    """A free lane has had nothing to start for `minutes` (#797): a survey is due once that is
    `suggestions.idle_minutes` (30) or more."""
    from datetime import timedelta
    from harness.clock import iso
    ctx.store.update(lambda s: s["suggest"].update(
        idle_since=iso(ctx.now() - timedelta(minutes=minutes))))
