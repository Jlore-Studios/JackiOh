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

Whether a provider may start a run now is `availability()`; its reading of the state file is
`state["providers"][<id>]` (usage readings, a refusal's reset time, minutes spent), which
`deliver` keeps up to date after every run.
"""

from __future__ import annotations

import json
import re
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

ROLES = ("build", "fix", "revise", "review", "suggest")

#: The secrets a provider may name. The workflows can only hand a run a secret they list, so this
#: is the same list as `bot-night.yml`'s (a test checks), and nothing else: never the bot's
#: GitHub token.
SECRETS: tuple[str, ...] = (
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN_2",
    "CLAUDE_CODE_OAUTH_TOKEN_3",
    "CLAUDE_CODE_OAUTH_TOKEN_4",
    "CODEX_AUTH_JSON",
    "MUSE_AUTH",
)

#: After a run on a provider could not work (its login refused, its CLI would not install or
#: start), unforced runs leave that provider alone this long. The others carry on.
INFRA_BACKOFF = timedelta(minutes=50)

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
    #: The model family, for the review rule: a second review must come from another family.
    family: str
    model: str
    effort: str
    secret: str
    schedule: Schedule
    limits: Limits
    #: Its own approval is enough to merge (Opus); otherwise a second model must approve too.
    self_review: bool
    #: May take issues labelled `difficult`, and takes them before anything else in their
    #: priority tier.
    difficult: bool
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

    def describe(self) -> str:
        return f"`{self.id}` ({self.cli}, {self.model})"


HIGH_TIER = "high"
LOW_TIER = "low"
_HIGH_TIER_FAMILIES = re.compile(r"astra|opus", re.IGNORECASE)


def model_tier(model: str) -> str:
    """`high` for OpenAI's Astra and Claude Opus, any version: the family name anywhere in the
    model's name, whatever its case (`opus`, `opusplan`, `claude-opus-4-1`, `gpt-5.6-astra`);
    `low` for every other model, an unknown or empty name included. Only `shitter` asks (#96)."""
    return HIGH_TIER if _HIGH_TIER_FAMILIES.search(model or "") else LOW_TIER


@dataclass(frozen=True)
class Pool:
    max_parallel: int
    priority: tuple[str, ...]
    providers: Mapping[str, Provider]
    difficult_label: str

    def ordered(self) -> list[Provider]:
        """Every provider, in `priority` order."""
        return [self.providers[name] for name in self.priority]

    def get(self, provider_id: str | None) -> Provider | None:
        return self.providers.get(str(provider_id or ""))


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


_PROVIDER_KEYS = {"enabled", "cli", "family", "model", "effort", "secret", "schedule", "limits",
                  "self_review", "difficult", "quiet_check", "roles", "env", "note", "login",
                  "runs_on", "off_from", "off_reason"}


def _provider(name: str, raw: Any) -> Provider:
    where = f"providers.{name}"
    if not isinstance(raw, Mapping):
        raise ConfigError(f"{where}: expected an object")
    unknown = sorted(set(raw) - _PROVIDER_KEYS)
    if unknown:
        raise ConfigError(f"{where}: unknown keys {', '.join(unknown)}")
    for key in ("cli", "family", "model", "schedule", "limits"):
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
        secret=secret,
        schedule=_schedule(raw["schedule"], f"{where}.schedule"),
        limits=_limits(raw["limits"], f"{where}.limits"),
        self_review=bool(raw.get("self_review", False)),
        difficult=bool(raw.get("difficult", False)),
        quiet_check=bool(raw.get("quiet_check", False)),
        roles=roles,
        env={str(k): str(v) for k, v in env.items()},
        login=login,
        runs_on=runs_on,
        off_from=_off_from(raw.get("off_from"), f"{where}.off_from"),
        off_reason=str(raw.get("off_reason") or ""),
    )


def parse(raw: Any) -> Pool:
    """A `Pool` from the JSON of `.harness/providers.json`. Raises `ConfigError`."""
    if not isinstance(raw, Mapping):
        raise ConfigError(f"{PROVIDERS_PATH} must hold a JSON object")
    providers = {str(k): _provider(str(k), v) for k, v in dict(raw.get("providers") or {}).items()}
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
    return Pool(lanes, priority, providers, str(raw.get("difficult_label", "difficult")))


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
        if resets is not None:
            entry["refused_until"] = iso(resets)
    if minutes > 0:
        spent = list(entry.get("spent") or [])
        spent.append({"at": iso(at), "minutes": round(float(minutes), 1)})
        entry["spent"] = spent[-SPENT_KEEP:]


def note_infra(state: dict[str, Any], provider_id: str, reason: str, at: datetime) -> None:
    """A run on this provider could not work: leave it alone for `INFRA_BACKOFF`."""
    entry = record(state, provider_id)
    entry["infra"] = {"at": iso(at), "reason": str(reason)[:500]}


def _duration(text: str) -> timedelta:
    amount = int("".join(ch for ch in text if ch.isdigit()) or "30")
    return timedelta(hours=amount) if text.upper().endswith("H") else timedelta(minutes=amount)


def refusal(provider: Provider, entry: Mapping[str, Any], at: datetime) -> str | None:
    """Why this provider's usage says to start nothing now, or None."""
    until = parse_iso(entry.get("refused_until"))
    if until is not None and until > at:
        return f"it refused a call; its limit resets at {iso(until)}"
    usage = entry.get("usage") or {}
    observed = parse_iso(usage.get("observed_at")) if isinstance(usage, dict) else None
    for window, stop in provider.limits.stops.items():
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
            return (f"{WINDOW_NAMES[window]} usage is {utilization:.0%}, at or over its "
                    f"{stop:.0%} cap; it resets at {iso(resets)}")
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
                 secrets: Secrets, *, forced: bool = False) -> str | None:
    """Why `provider` may not start a run now, or None when it may. Busy-ness is the caller's."""
    if not provider.enabled:
        return "switched off in providers.json"
    if switched_off_by_date(provider, at, zone_name):
        return f"switched off from {provider.off_from} (`off_from` in providers.json)"
    if provider.login == "secret" and secrets.has(provider.secret) is False:
        return f"its secret `{provider.secret}` is not set"
    if not forced and not provider.schedule.is_open(zone_name, at):
        window = provider.schedule.window(zone_name)
        opens = window.next_open(at) if window else at
        return f"outside its hours ({provider.schedule.describe(zone_name)}; opens in {human_delta(opens - at)})"
    entry = peek_record(state, provider.id)
    infra = entry.get("infra") if isinstance(entry.get("infra"), Mapping) else {}
    failed = parse_iso(infra.get("at"))
    if not forced and failed is not None and at - failed < INFRA_BACKOFF:
        return (f"its last run could not work ({str(infra.get('reason') or '')[:120]}); it is "
                f"left alone until {iso(failed + INFRA_BACKOFF)}")
    return refusal(provider, entry, at)


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
