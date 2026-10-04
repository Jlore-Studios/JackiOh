"""The subscriptions the bot can spend: `.harness/providers.json`, and whether each is usable now.

A *provider* is one subscription behind one CLI: a Claude account (`claude`), ChatGPT through
the Codex CLI (`codex`), Google through the Antigravity CLI (`agy`), Meta through Muse Code
(`muse`), or Cognition through the Devin CLI (`devin`). Each has its own hours (`schedule`), its own limits (`limits`), the runner its model job
runs on (`runs_on`: GitHub's `ubuntu-latest`, or its own runner on the bot's machine,
`night-vm-<id>`, which runs as a Linux user of its own; `bot/machine/README.md`), and its login
(`login`): a `secret` the workflows hand to that model job and to nothing else, or a login made
once on the machine in that user's home (`machine`), which never leaves it. A run uses exactly one
provider from start to finish; `plan` chooses it and records it on the item it claims, so a
provider works on one item at a time and at most `max_parallel` items run at once.

Every model a provider can run is a *seat* with a *tier* (`weak`, `medium` or `strong`): its main
`model` with its `tier`, and any `extra_models` (claude-3 and claude-1 run Sonnet, weak, as well as
Opus, strong). The tier attaches to the model a role runs on, not to the subscription. The
top-level `tiers` map lists each tier's models in the order the router tries them, after the
subscriptions' own order (`priority`, the usage order). A provider with `self_check` (Devin)
checks its own builds before any review.

Whether a provider may start a run now is `availability()`; its reading of the state file is
`state["providers"][<id>]` (usage readings, a refusal's reset time, minutes spent), which
`deliver` keeps up to date after every run.
"""

from __future__ import annotations

import json
import re
import dataclasses
from dataclasses import dataclass, field
from datetime import date, datetime, timedelta
from pathlib import Path
from typing import Any, Mapping

from harness import clock
from harness.clock import human_delta, iso, parse_iso, zone
from harness.errors import ConfigError

PROVIDERS_PATH = Path(".harness") / "providers.json"

CLIS = ("claude", "codex", "agy", "muse", "devin")
LOGINS = ("secret", "machine")
#: The CLIs that can be logged in from a secret on a fresh runner (`logins.py`). agy and Devin
#: keep their logins only on the machine.
SECRET_CLIS = ("claude", "codex", "muse")
_LABEL = re.compile(r"^[A-Za-z0-9._-]+$")
#: Labels of GitHub's own runners: a fresh virtual machine per job, with no login on it.
HOSTED_PREFIXES = ("ubuntu-", "windows-", "macos-")


def hosted(label: str) -> bool:
    """Whether `label` is one of GitHub's own runners rather than the bot's machine."""
    return label.startswith(HOSTED_PREFIXES)

ROLES = ("plan", "build", "fix", "revise", "review", "suggest")

#: The tiers, weakest first. A difficulty needs at least `MIN_TIER[difficulty]` to build it.
TIERS = ("weak", "medium", "strong")
TIER_RANK = {name: rank for rank, name in enumerate(TIERS)}

#: The secrets a provider may name. The workflows can only hand a run a secret they list, so this
#: is the same list as `bot-night.yml`'s (a test checks), and nothing else: never the bot's
#: GitHub token.
SECRETS: tuple[str, ...] = (
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN_2",
    "CLAUDE_CODE_OAUTH_TOKEN_3",
    "CLAUDE_CODE_OAUTH_TOKEN_4",
    "CLAUDE_CODE_OAUTH_TOKEN_5",
    "CODEX_AUTH_JSON",
    "MUSE_AUTH",
)

#: After a run on a provider could not work (its login refused, its CLI would not install or
#: start), unforced runs leave that provider alone this long. The others carry on. Each failure in
#: a row waits longer (the last wait repeats), so a dead login stops costing a run every hour; the
#: streak ends when a run on the provider gets a model call through (`clear_infra`).
INFRA_BACKOFFS = (timedelta(minutes=50), timedelta(hours=2), timedelta(hours=8))
INFRA_BACKOFF = INFRA_BACKOFFS[0]
#: After this many failures in a row, `deliver` opens an issue asking a person to fix it.
INFRA_ASK_AFTER = 3

#: Usage windows, and how long each lasts when a reading carries no reset time of its own.
WINDOWS = {"five_hour": timedelta(hours=5), "seven_day": timedelta(days=7)}
WINDOW_NAMES = {"five_hour": "5-hour", "seven_day": "7-day"}


@dataclass(frozen=True)
class Schedule:
    """When a provider may start work: `always`, or a daily `window` in the bot's time zone."""

    mode: str
    start: str = ""
    end: str = ""

    def window(self, zone_name: str) -> clock.Window | None:
        return clock.Window.of(zone_name, self.start, self.end) if self.mode == "window" else None

    def is_open(self, zone_name: str, at: datetime) -> bool:
        window = self.window(zone_name)
        return window is None or window.is_open(at)

    def describe(self, zone_name: str) -> str:
        window = self.window(zone_name)
        return "any time" if window is None else window.describe()


@dataclass(frozen=True)
class Limits:
    """`caps`: no new run past these fractions of the 5-hour or 7-day allowance, or past these
    minutes of model time in a rolling 5 hours or 7 days (for a CLI that reports no usage).
    `none`: use whatever there is; a refusal parks the provider until the limit resets."""

    mode: str
    five_hour: float | None = None
    seven_day: float | None = None
    five_hour_minutes: int | None = None
    seven_day_minutes: int | None = None
    #: How far under a cap a build or revision must be to start (`start_headroom` in
    #: providers.json): one that starts at 39% under a 40% cap is cut off almost at once.
    headroom: Mapping[str, float] = field(default_factory=dict)

    @property
    def stops(self) -> dict[str, float]:
        if self.mode != "caps":
            return {}
        return {k: v for k, v in (("five_hour", self.five_hour), ("seven_day", self.seven_day))
                if v is not None}

    @property
    def budgets(self) -> dict[str, int]:
        if self.mode != "caps":
            return {}
        return {k: v for k, v in (("five_hour", self.five_hour_minutes),
                                  ("seven_day", self.seven_day_minutes)) if v is not None}


@dataclass(frozen=True)
class Provider:
    id: str
    enabled: bool
    cli: str
    #: The model family, for the review rule: two medium reviews must come from two families.
    family: str
    model: str
    effort: str
    secret: str
    schedule: Schedule
    limits: Limits
    #: The tier of `model`: `weak`, `medium` or `strong`.
    tier: str
    #: Wait until nobody else is spending the subscription before an unforced run (`quiet.py`).
    quiet_check: bool
    roles: tuple[str, ...]
    #: Extra, non-secret environment for the CLI.
    env: Mapping[str, str] = field(default_factory=dict)
    #: `secret` (handed over by the workflow) or `machine` (logged in on the runner already).
    login: str = "secret"
    #: The runner label its model job runs on.
    runs_on: str = "ubuntu-latest"
    #: The first day (in the bot's time zone) it takes no new work, and why: a free offer that
    #: ends, say. On that day the planner also opens one issue asking a person to decide.
    off_from: date | None = None
    off_reason: str = ""
    #: How many items it may work on at once. On the machine each needs a runner of its own
    #: with its `runs_on` label (`register-runners.sh` registers that many).
    lanes: int = 1
    #: Its builds check themselves before any review (`work.py`'s self-check loop).
    self_check: bool = False
    #: It builds, fixes and revises only after every other subscription that may (claude-2, kept
    #: for planning and review); it plans and reviews in its `priority` place.
    build_last: bool = False
    #: It builds and revises easy items ahead of every other subscription while it has a free lane
    #: (Devin: it may build nothing harder, so the stronger models are kept for what only they can).
    easy_first: bool = False
    #: Other models the subscription can run for a role, each with its own tier.
    extra_models: tuple["ExtraModel", ...] = ()
    #: Caps outside its `schedule` window (`{"five_hour": 0.4}`): it may work then too, but only
    #: under these, and a run that goes past them there stops. No entry: it works in its hours only.
    off_hours: Mapping[str, float] = field(default_factory=dict)

    def describe(self) -> str:
        return f"`{self.id}` ({self.cli}, {self.model})"

    def hours(self, zone_name: str) -> str:
        """Its hours, and how far it may go outside them."""
        text = self.schedule.describe(zone_name)
        if self.off_hours and self.schedule.mode == "window":
            caps = " and ".join(f"{share:.0%} of {WINDOW_NAMES[window]}"
                                for window, share in self.off_hours.items())
            text += f", outside them up to {caps}"
        return text

    def caps_at(self, at: datetime, zone_name: str | None) -> dict[str, float]:
        """The usage caps that hold at `at`: its own, and outside its window the tighter of
        those and its `off_hours` ones."""
        caps = dict(self.limits.stops)
        if (self.off_hours and zone_name is not None
                and not self.schedule.is_open(zone_name, at)):
            for window, share in self.off_hours.items():
                caps[window] = min(caps.get(window, 1.0), share)
        return caps


@dataclass(frozen=True)
class ExtraModel:
    model: str
    effort: str
    tier: str


@dataclass(frozen=True)
class TierEntry:
    """One model in a tier's preference order (`tiers` in providers.json)."""

    model: str


@dataclass(frozen=True)
class Seat:
    """One model one subscription can run, and its tier: what a role is given to run on."""

    provider: Provider
    model: str
    effort: str
    tier: str
    self_check: bool
    #: Its place in its tier's preference order: the router tries the lowest first.
    rank: int

    @property
    def family(self) -> str:
        return self.provider.family

    def describe(self) -> str:
        return f"`{self.provider.id}` ({self.provider.cli}, `{self.model}`, {self.tier})"

    def to_dict(self) -> dict[str, Any]:
        return {"provider": self.provider.id, "model": self.model, "effort": self.effort,
                "tier": self.tier, "self_check": self.self_check}


def tier_at_least(tier: str, floor: str) -> bool:
    return TIER_RANK[tier] >= TIER_RANK[floor]


@dataclass(frozen=True)
class Pool:
    max_parallel: int
    priority: tuple[str, ...]
    providers: Mapping[str, Provider]
    #: How many of those runs may be on the bot's machine at once (`runs_on` not GitHub's): its
    #: two vCPUs run every machine job's checks, while each of GitHub's runners has its own four.
    machine_parallel: int = 0
    #: How many planning runs may go at once on top of `max_parallel` (the planning lane): a
    #: strong model plans the Needs plan stage there while the build lanes are full.
    plan_lanes: int = 0
    #: Each tier's models, in the order the router tries them.
    tiers: Mapping[str, tuple[TierEntry, ...]] = field(default_factory=dict)

    def on_machine(self, provider_id: str) -> bool:
        provider = self.get(provider_id)
        return provider is not None and not hosted(provider.runs_on)

    def ordered(self) -> list[Provider]:
        """Every provider, in `priority` order."""
        return [self.providers[name] for name in self.priority]

    def get(self, provider_id: str | None) -> Provider | None:
        return self.providers.get(str(provider_id or ""))

    def _entry(self, tier: str, model: str) -> tuple[int, TierEntry | None]:
        for rank, entry in enumerate(self.tiers.get(tier, ())):
            if entry.model == model:
                return rank, entry
        return len(self.tiers.get(tier, ())), None

    def seats(self, provider: Provider) -> list[Seat]:
        """The models `provider` can run, its main one first."""
        found = []
        for model, effort, tier, own_check in (
                (provider.model, provider.effort, provider.tier, provider.self_check),
                *((m.model, m.effort, m.tier, False) for m in provider.extra_models)):
            rank, _ = self._entry(tier, model)
            found.append(Seat(provider, model, effort, tier, own_check, rank))
        return found

    def seat(self, provider_id: str | None, model: str | None = None) -> Seat | None:
        """`provider_id`'s seat for `model`, or its main one; None when it has no such model."""
        provider = self.get(provider_id)
        if provider is None:
            return None
        for found in self.seats(provider):
            if model in (None, "", found.model):
                return found
        return None

    def best_seat(self, provider: Provider, floor: str = "weak") -> Seat | None:
        """`provider`'s strongest seat at `floor` or above, or None."""
        seats = [s for s in self.seats(provider) if tier_at_least(s.tier, floor)]
        return max(seats, key=lambda s: (TIER_RANK[s.tier], -s.rank), default=None)

    def family_tier(self, family: str) -> str:
        """The strongest tier any seat of `family` has: how a vote with no tier recorded counts."""
        tiers = [s.tier for p in self.ordered() if p.family == family for s in self.seats(p)]
        return max(tiers, key=TIER_RANK.__getitem__, default="weak")


def _fraction(value: Any, where: str) -> float | None:
    if value is None:
        return None
    number = float(value)
    if not 0.0 < number <= 1.0:
        raise ConfigError(f"{where}: {value} is outside (0, 1]")
    return number


def _schedule(raw: Any, where: str) -> Schedule:
    if not isinstance(raw, Mapping):
        raise ConfigError(f"{where}: expected an object")
    mode = str(raw.get("mode", ""))
    if mode == "always":
        return Schedule("always")
    if mode == "window":
        try:
            start, end = str(raw["start"]), str(raw["end"])
            clock.parse_hhmm(start), clock.parse_hhmm(end)
        except (KeyError, ValueError) as exc:
            raise ConfigError(f"{where}: a window needs start and end as HH:MM ({exc})") from exc
        return Schedule("window", start, end)
    raise ConfigError(f"{where}.mode: {mode!r} is not always or window")


def _headroom(raw: Any, where: str) -> dict[str, float]:
    if raw in (None, {}):
        return {}
    if not isinstance(raw, Mapping):
        raise ConfigError(f"{where}: expected an object such as {{\"five_hour\": 0.15}}")
    unknown = sorted(set(raw) - set(WINDOWS))
    if unknown:
        raise ConfigError(f"{where}: unknown windows {', '.join(unknown)}")
    return {str(window): _fraction(share, f"{where}.{window}") for window, share in raw.items()}


def _limits(raw: Any, where: str) -> Limits:
    if not isinstance(raw, Mapping):
        raise ConfigError(f"{where}: expected an object")
    mode = str(raw.get("mode", ""))
    if mode == "none":
        return Limits("none")
    if mode != "caps":
        raise ConfigError(f"{where}.mode: {mode!r} is not caps or none")
    def minutes(key: str) -> int | None:
        value = raw.get(key)
        if value is None:
            return None
        if int(value) < 1:
            raise ConfigError(f"{where}.{key}: must be at least 1")
        return int(value)
    limits = Limits("caps", _fraction(raw.get("five_hour"), f"{where}.five_hour"),
                    _fraction(raw.get("seven_day"), f"{where}.seven_day"),
                    minutes("five_hour_minutes"), minutes("seven_day_minutes"))
    if not limits.stops and not limits.budgets:
        raise ConfigError(f"{where}: caps needs at least one of five_hour, seven_day, "
                          "five_hour_minutes, seven_day_minutes")
    return limits


_PROVIDER_KEYS = {"enabled", "cli", "family", "model", "effort", "tier", "secret", "schedule",
                  "limits", "quiet_check", "roles", "env", "note", "login", "runs_on",
                  "self_check", "extra_models", "off_from", "off_reason",
                  "lanes", "build_last", "easy_first", "off_hours"}


def _tier(value: Any, where: str) -> str:
    tier = str(value or "")
    if tier not in TIERS:
        raise ConfigError(f"{where}: {tier!r} is not one of {', '.join(TIERS)}")
    return tier


def _extra_models(raw: Any, effort: str, where: str) -> tuple[ExtraModel, ...]:
    if raw is None:
        return ()
    if not isinstance(raw, list):
        raise ConfigError(f"{where}: expected a list")
    found = []
    for i, entry in enumerate(raw):
        if not isinstance(entry, Mapping) or not entry.get("model"):
            raise ConfigError(f"{where}[{i}]: needs a model and a tier")
        unknown = sorted(set(entry) - {"model", "effort", "tier", "note"})
        if unknown:
            raise ConfigError(f"{where}[{i}]: unknown keys {', '.join(unknown)}")
        found.append(ExtraModel(str(entry["model"]), str(entry.get("effort", effort)),
                                _tier(entry.get("tier"), f"{where}[{i}].tier")))
    return tuple(found)


def _tiers(raw: Any) -> dict[str, tuple[TierEntry, ...]]:
    where = f"{PROVIDERS_PATH}: tiers"
    if not isinstance(raw, Mapping):
        raise ConfigError(f"{where}: expected an object with {', '.join(TIERS)}")
    unknown = sorted(set(raw) - set(TIERS))
    if unknown:
        raise ConfigError(f"{where}: unknown tiers {', '.join(unknown)}")
    found: dict[str, tuple[TierEntry, ...]] = {}
    for tier in TIERS:
        entries = []
        for i, entry in enumerate(raw.get(tier) or []):
            if isinstance(entry, str):
                entry = {"model": entry}
            if not isinstance(entry, Mapping) or not entry.get("model"):
                raise ConfigError(f"{where}.{tier}[{i}]: needs a model")
            extra = sorted(set(entry) - {"model", "note"})
            if extra:
                raise ConfigError(f"{where}.{tier}[{i}]: unknown keys {', '.join(extra)}")
            entries.append(TierEntry(str(entry["model"])))
        found[tier] = tuple(entries)
    models = [e.model for entries in found.values() for e in entries]
    twice = sorted({m for m in models if models.count(m) > 1})
    if twice:
        raise ConfigError(f"{where}: {', '.join(twice)} is in more than one place; a model has "
                          "one tier")
    return found


def _provider(name: str, raw: Any) -> Provider:
    where = f"providers.{name}"
    if not isinstance(raw, Mapping):
        raise ConfigError(f"{where}: expected an object")
    unknown = sorted(set(raw) - _PROVIDER_KEYS)
    if unknown:
        raise ConfigError(f"{where}: unknown keys {', '.join(unknown)}")
    for key in ("cli", "family", "model", "tier", "schedule", "limits"):
        if key not in raw:
            raise ConfigError(f"{where}: missing {key}")
    cli = str(raw["cli"])
    if cli not in CLIS:
        raise ConfigError(f"{where}.cli: {cli!r} is not one of {', '.join(CLIS)}")
    login = str(raw.get("login", "secret"))
    if login not in LOGINS:
        raise ConfigError(f"{where}.login: {login!r} is not one of {', '.join(LOGINS)}")
    secret = str(raw.get("secret") or "")
    if login == "secret":
        if cli not in SECRET_CLIS:
            raise ConfigError(f"{where}: the {cli} CLI logs in on the machine only "
                              '("login": "machine")')
        if secret not in SECRETS:
            raise ConfigError(f"{where}.secret: {secret!r} is not one the workflows hand over "
                              f"({', '.join(SECRETS)})")
    elif secret:
        raise ConfigError(f"{where}.secret: a machine login takes no secret")
    runs_on = str(raw.get("runs_on", "ubuntu-latest"))
    if not _LABEL.match(runs_on):
        raise ConfigError(f"{where}.runs_on: {runs_on!r} is not a runner label")
    if login == "machine" and hosted(runs_on):
        raise ConfigError(f"{where}: a machine login needs runs_on to be its runner on the machine "
                          f"(night-vm-{name}), not GitHub's {runs_on}")
    roles = tuple(str(r) for r in raw.get("roles", ROLES))
    bad = [r for r in roles if r not in ROLES]
    if bad:
        raise ConfigError(f"{where}.roles: unknown {', '.join(bad)}")
    env = raw.get("env") or {}
    if not isinstance(env, Mapping):
        raise ConfigError(f"{where}.env: expected an object")
    if raw.get("quiet_check") and cli != "claude":
        raise ConfigError(f"{where}.quiet_check: only a Claude account can be checked for quiet "
                          "(the check reads Claude's usage)")
    return Provider(
        id=name,
        enabled=bool(raw.get("enabled", True)),
        cli=cli,
        family=str(raw["family"]),
        model=str(raw["model"]),
        effort=str(raw.get("effort", "")),
        tier=_tier(raw["tier"], f"{where}.tier"),
        secret=secret,
        schedule=_schedule(raw["schedule"], f"{where}.schedule"),
        limits=_limits(raw["limits"], f"{where}.limits"),
        quiet_check=bool(raw.get("quiet_check", False)),
        roles=roles,
        env={str(k): str(v) for k, v in env.items()},
        login=login,
        runs_on=runs_on,
        off_from=_off_from(raw.get("off_from"), f"{where}.off_from"),
        off_reason=str(raw.get("off_reason") or ""),
        lanes=_lanes(raw.get("lanes", 1), f"{where}.lanes"),
        self_check=bool(raw.get("self_check", False)),
        build_last=bool(raw.get("build_last", False)),
        easy_first=bool(raw.get("easy_first", False)),
        off_hours=_off_hours(raw.get("off_hours"), f"{where}.off_hours"),
        extra_models=_extra_models(raw.get("extra_models"), str(raw.get("effort", "")),
                                   f"{where}.extra_models"),
    )


def parse(raw: Any) -> Pool:
    """A `Pool` from the JSON of `.harness/providers.json`. Raises `ConfigError`."""
    if not isinstance(raw, Mapping):
        raise ConfigError(f"{PROVIDERS_PATH} must hold a JSON object")
    providers = {str(k): _provider(str(k), v) for k, v in dict(raw.get("providers") or {}).items()}
    headroom = _headroom(raw.get("start_headroom"), f"{PROVIDERS_PATH}.start_headroom")
    providers = {k: dataclasses.replace(p, limits=dataclasses.replace(p.limits, headroom=headroom))
                 if p.limits.stops else p for k, p in providers.items()}
    if not providers:
        raise ConfigError(f"{PROVIDERS_PATH}: at least one provider is required")
    secrets = [p.secret for p in providers.values() if p.login == "secret"]
    clash = sorted({s for s in secrets if secrets.count(s) > 1})
    if clash:
        raise ConfigError(f"{PROVIDERS_PATH}: two providers share the secret {', '.join(clash)}")
    # A runner on the machine is one Linux user with one home: a machine login lives there, and a
    # second provider on the same runner would run in the first one's home, with its login.
    for p in providers.values():
        shared = sorted(q.id for q in providers.values() if q.id != p.id and q.runs_on == p.runs_on)
        if shared and not hosted(p.runs_on):
            raise ConfigError(f"{PROVIDERS_PATH}: {p.id} and {', '.join(shared)} share the runner "
                              f"{p.runs_on}; each provider on the machine has its own")
    priority = tuple(str(p) for p in raw.get("priority") or providers)
    missing = [p for p in providers if p not in priority]
    extra = [p for p in priority if p not in providers]
    if missing or extra or len(set(priority)) != len(priority):
        raise ConfigError(f"{PROVIDERS_PATH}: priority must name every provider once "
                          f"(missing {missing}, unknown {extra})")
    lanes = int(raw.get("max_parallel", 1))
    if lanes < 1:
        raise ConfigError(f"{PROVIDERS_PATH}: max_parallel must be at least 1")
    tiers = _tiers(raw.get("tiers"))
    machine = int(raw.get("machine_parallel", lanes))
    if not 0 <= machine <= lanes:
        raise ConfigError(f"{PROVIDERS_PATH}: machine_parallel must be from 0 to max_parallel")
    plan_lanes = int(raw.get("plan_lanes", 0))
    if plan_lanes < 0:
        raise ConfigError(f"{PROVIDERS_PATH}: plan_lanes must be 0 or more")
    pool = Pool(lanes, priority, providers, machine_parallel=machine, plan_lanes=plan_lanes,
                tiers=tiers)
    # Every model a provider runs has its place in its own tier's order, so the router always
    # knows which to try first, and a model never has two tiers.
    for provider in providers.values():
        for seat in pool.seats(provider):
            placed = [t for t, entries in tiers.items() if any(e.model == seat.model
                                                                 for e in entries)]
            if placed != [seat.tier]:
                raise ConfigError(
                    f"{PROVIDERS_PATH}: {provider.id} runs {seat.model} as {seat.tier}, but "
                    + (f"tiers lists it under {', '.join(placed)}" if placed
                       else f"tiers.{seat.tier} does not list it"))
    return pool


def load(root: Path) -> Pool:
    path = Path(root) / PROVIDERS_PATH
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise ConfigError(f"{path} does not exist") from exc
    except ValueError as exc:
        raise ConfigError(f"{path} is not valid JSON: {exc}") from exc
    return parse(raw)


# ---------------------------------------------------------------------- the state of each one


#: The first Claude account inherits the readings the state file kept before there were several
#: subscriptions (`usage`, `rate_limited_until` at the top level).
LEGACY_PROVIDER = "claude-1"
#: How many past runs' minutes a provider keeps, for `*_minutes` budgets.
SPENT_KEEP = 200


def record(state: dict[str, Any], provider_id: str) -> dict[str, Any]:
    """The state file's record for one provider, created empty when missing."""
    providers = state.setdefault("providers", {})
    entry = providers.setdefault(provider_id, {})
    if provider_id == LEGACY_PROVIDER and "usage" not in entry and state.get("usage"):
        entry["usage"] = state.get("usage")
    if (provider_id == LEGACY_PROVIDER and "refused_until" not in entry
            and state.get("rate_limited_until")):
        entry["refused_until"] = state.get("rate_limited_until")
    return entry


def peek_record(state: dict[str, Any], provider_id: str) -> dict[str, Any]:
    """`record` without creating anything."""
    entry = dict((state.get("providers") or {}).get(provider_id) or {})
    if provider_id == LEGACY_PROVIDER:
        entry.setdefault("usage", state.get("usage"))
        entry.setdefault("refused_until", state.get("rate_limited_until"))
    return entry


def note_usage(state: dict[str, Any], provider_id: str, usage: dict | None, reset_at: str | None,
               at: datetime, minutes: float = 0.0) -> None:
    """Keep a run's newest usage reading, any refusal, and the minutes it spent."""
    entry = record(state, provider_id)
    if isinstance(usage, dict) and usage:
        entry["usage"] = {**usage, "observed_at": iso(at)}
    if reset_at:
        resets = parse_iso(reset_at)
        if resets is None and str(reset_at).startswith("+PT"):
            resets = at + _duration(str(reset_at))
            from harness.runner import DEFAULT_PARK  # runner imports config, which imports this
            if str(reset_at) == DEFAULT_PARK:
                # The refusal named no reset: a window the last reading had nearly full is the
                # likely cause, so wait for it (gpt's week at 89% was retried every hour).
                held = near_full_reset(entry.get("usage"), at)
                if held is not None and held > resets:
                    resets = held
        if resets is not None:
            entry["refused_until"] = iso(resets)
    if minutes > 0:
        spent = list(entry.get("spent") or [])
        spent.append({"at": iso(at), "minutes": round(float(minutes), 1)})
        entry["spent"] = spent[-SPENT_KEEP:]


#: A window at least this full in the last reading is taken to be why a refusal came.
NEAR_FULL = 0.85


def near_full_reset(usage: Any, at: datetime) -> datetime | None:
    """When the latest-resetting window that the reading had nearly full resets, if any."""
    if not isinstance(usage, Mapping):
        return None
    found = []
    for window in WINDOWS:
        reading = usage.get(window)
        if not isinstance(reading, Mapping):
            continue
        resets = parse_iso(reading.get("resets_at"))
        utilization = reading.get("utilization")
        if (resets is not None and resets > at and isinstance(utilization, (int, float))
                and utilization >= NEAR_FULL):
            found.append(resets)
    return max(found, default=None)


def note_infra(state: dict[str, Any], provider_id: str, reason: str, at: datetime, *,
               escalate: bool = True) -> int:
    """A run on this provider could not work: leave it alone for its backoff (`infra_backoff`).
    Returns the failures in a row. `escalate` False (a failure that was not the provider's, such
    as an install that fails on untouched main) keeps the streak where it was."""
    entry = record(state, provider_id)
    previous = entry.get("infra") if isinstance(entry.get("infra"), Mapping) else {}
    streak = _streak(previous)
    if escalate or not previous:
        streak = streak + 1 if previous else 1
    entry["infra"] = {**{k: previous[k] for k in ("issue",) if previous.get(k)},
                      "at": iso(at), "reason": str(reason)[:500], "streak": streak}
    return streak


def clear_infra(state: dict[str, Any], provider_id: str) -> int | None:
    """A run on this provider got a model call through: its failure streak is over. Returns the
    issue `deliver` opened about it, if any, to close."""
    entry = (state.get("providers") or {}).get(provider_id)
    if not isinstance(entry, dict) or not isinstance(entry.get("infra"), Mapping):
        return None
    issue = entry.pop("infra").get("issue")
    return int(issue) if issue else None


def _streak(infra: Mapping[str, Any]) -> int:
    try:
        return max(1, int(infra.get("streak") or 1)) if infra else 0
    except (TypeError, ValueError):
        return 1


def infra_backoff(infra: Mapping[str, Any]) -> timedelta:
    """How long a provider is left alone after its last failure, by the failures in a row."""
    return INFRA_BACKOFFS[min(max(_streak(infra), 1), len(INFRA_BACKOFFS)) - 1]


def _duration(text: str) -> timedelta:
    """`+PT90M`, `+PT2H`, `+PT1H30M`: how long a relative refusal lasts (30 minutes if unread)."""
    match = re.fullmatch(r"\+?PT(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?", text.strip().upper())
    if not match or not any(match.groups()):
        return timedelta(minutes=30)
    hours, minutes, seconds = (int(g or 0) for g in match.groups())
    return timedelta(hours=hours, minutes=minutes, seconds=seconds)


def refusal(provider: Provider, entry: Mapping[str, Any], at: datetime,
            zone_name: str | None = None, *, starting: bool = False) -> str | None:
    """Why this provider's usage says to start nothing now, or None. With the bot's time zone,
    outside its window its `off_hours` caps hold too. `starting` (a build or a revision about to
    start) leaves `start_headroom` under each cap; a run going on stops at the cap itself."""
    until = parse_iso(entry.get("refused_until"))
    if until is not None and until > at:
        return f"it refused a call; its limit resets at {iso(until)}"
    usage = entry.get("usage") or {}
    observed = parse_iso(usage.get("observed_at")) if isinstance(usage, dict) else None
    caps = provider.caps_at(at, zone_name)
    outside = caps != dict(provider.limits.stops)
    room = provider.limits.headroom if starting else {}
    for window, cap in caps.items():
        stop = max(0.0, cap - room.get(window, 0.0))
        reading = usage.get(window) if isinstance(usage, dict) else None
        if not isinstance(reading, dict):
            continue
        resets = parse_iso(reading.get("resets_at"))
        if resets is None and observed is not None:
            resets = observed + WINDOWS[window]
        if resets is None or resets <= at:
            continue  # that window has reset since the reading, or cannot be dated
        utilization = reading.get("utilization")
        if isinstance(utilization, (int, float)) and utilization >= stop:
            where = " outside its hours" if outside else ""
            if stop < cap:
                return (f"{WINDOW_NAMES[window]} usage is {utilization:.0%}, too close to its "
                        f"{cap:.0%} cap{where} to start a build (it starts under {stop:.0%}); "
                        f"it resets at {iso(resets)}")
            return (f"{WINDOW_NAMES[window]} usage is {utilization:.0%}, at or over its "
                    f"{stop:.0%} cap{where}; it resets at {iso(resets)}")
    for window, budget in provider.limits.budgets.items():
        used = minutes_spent(entry, at - WINDOWS[window])
        if used >= budget:
            return f"it has spent {used:.0f} of its {budget} minutes in the {WINDOW_NAMES[window]} window"
    return None


def minutes_spent(entry: Mapping[str, Any], since: datetime) -> float:
    total = 0.0
    for run in entry.get("spent") or []:
        at = parse_iso(run.get("at")) if isinstance(run, Mapping) else None
        if at is not None and at >= since:
            total += float(run.get("minutes") or 0)
    return total


@dataclass(frozen=True)
class Secrets:
    """Which provider secrets the workflow has. `known` is False outside a workflow (an operator
    running the harness by hand), where nothing can be told and nothing is ruled out."""

    present: frozenset[str]
    known: bool

    @classmethod
    def of(cls, env: Mapping[str, str]) -> "Secrets":
        if "HARNESS_SECRETS_SET" in env:
            return cls(frozenset(env["HARNESS_SECRETS_SET"].split()), True)
        return cls(frozenset(k for k in SECRETS if env.get(k)), False)

    def has(self, name: str) -> bool | None:
        if name in self.present:
            return True
        return False if self.known else None


def _off_hours(raw: Any, where: str) -> dict[str, float]:
    if raw in (None, {}):
        return {}
    if not isinstance(raw, Mapping):
        raise ConfigError(f"{where}: expected an object such as {{\"five_hour\": 0.4}}")
    unknown = sorted(set(raw) - set(WINDOWS))
    if unknown:
        raise ConfigError(f"{where}: unknown windows {', '.join(unknown)}")
    return {str(window): _fraction(share, f"{where}.{window}") for window, share in raw.items()}


def _lanes(raw: Any, where: str) -> int:
    try:
        lanes = int(raw)
    except (TypeError, ValueError) as exc:
        raise ConfigError(f"{where}: {raw!r} is not a number") from exc
    if lanes < 1:
        raise ConfigError(f"{where}: must be at least 1")
    return lanes


def _off_from(raw: Any, where: str) -> date | None:
    if raw in (None, ""):
        return None
    try:
        return date.fromisoformat(str(raw))
    except ValueError as exc:
        raise ConfigError(f"{where}: {raw!r} is not a date (YYYY-MM-DD)") from exc


def switched_off_by_date(provider: Provider, at: datetime, zone_name: str) -> bool:
    """Whether `provider`'s `off_from` day has come, in the bot's time zone."""
    return provider.off_from is not None and at.astimezone(zone(zone_name)).date() >= provider.off_from


def availability(provider: Provider, state: dict[str, Any], at: datetime, zone_name: str,
                 secrets: Secrets, *, forced: bool = False, starting: bool = False) -> str | None:
    """Why `provider` may not start a run now, or None when it may. Busy-ness is the caller's."""
    if not provider.enabled:
        return "switched off in providers.json"
    if switched_off_by_date(provider, at, zone_name):
        return f"switched off from {provider.off_from} (`off_from` in providers.json)"
    if provider.login == "secret" and secrets.has(provider.secret) is False:
        return f"its secret `{provider.secret}` is not set"
    if not forced and not provider.off_hours and not provider.schedule.is_open(zone_name, at):
        window = provider.schedule.window(zone_name)
        opens = window.next_open(at) if window else at
        return f"outside its hours ({provider.schedule.describe(zone_name)}; opens in {human_delta(opens - at)})"
    entry = peek_record(state, provider.id)
    infra = entry.get("infra") if isinstance(entry.get("infra"), Mapping) else {}
    failed = parse_iso(infra.get("at"))
    backoff = infra_backoff(infra)
    if not forced and failed is not None and at - failed < backoff:
        streak = _streak(infra)
        times = f", {streak} times in a row" if streak > 1 else ""
        return (f"its last run could not work{times} ({str(infra.get('reason') or '')[:120]}); "
                f"it is left alone until {iso(failed + backoff)}")
    return refusal(provider, entry, at, zone_name, starting=starting)


def when_free(pool: Pool, state: dict[str, Any], at: datetime, zone_name: str,
              secrets: Secrets) -> str:
    """When a queued item can expect a run: now, or when the soonest subscription opens."""
    now_free = [p.id for p in pool.ordered()
                if availability(p, state, at, zone_name, secrets) is None]
    if now_free:
        return f"in the next run ({', '.join(f'`{p}`' for p in now_free)} can take it now)"
    soonest: tuple[datetime, Provider] | None = None
    for provider in pool.ordered():
        window = provider.schedule.window(zone_name)
        if window is None or availability(provider, state, at, zone_name, secrets,
                                          forced=True) is not None:
            continue
        opens = window.next_open(at)
        if soonest is None or opens < soonest[0]:
            soonest = (opens, provider)
    if soonest is not None:
        opens, provider = soonest
        return (f"when `{provider.id}` opens ({provider.schedule.describe(zone_name)}, in "
                f"{human_delta(opens - at)})")
    return "when a subscription is free again (`/harness status` says why none is now)"
