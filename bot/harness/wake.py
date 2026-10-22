"""The morning start (`.github/workflows/claude-wake.yml`): at 06:00 America/Chicago each Claude
subscription whose 5-hour session is not running gets one turn of the smallest model at low
effort, so its session starts then and resets five hours later, rather than at whatever hour the
first run of the day lands. A session the state file shows running is left alone, and so is
every subscription of a bot that is halted, since the turn is a model call.

GitHub starts this repository's schedules late and drops some, so the workflow fires several
times in the hours before and sleeps until `WAKE_AT` (`until_wake`); a run that starts after it,
within `LATE`, wakes at once. Each subscription's turn runs in a job holding that one secret.
"""

from __future__ import annotations

from datetime import datetime, time, timedelta
from typing import Any

from harness import clock
from harness import providers as providers_mod
from harness.config import Config
from harness.runner import ping_usage

ZONE = "America/Chicago"
WAKE_AT = time(6, 0)
#: How long before `WAKE_AT` a run may start and sleep until it, and how long after it a run
#: that GitHub started late still wakes the sessions.
EARLY = timedelta(hours=4, minutes=30)
LATE = timedelta(hours=2)
MODEL = "claude-haiku-5-5"
EFFORT = "low"
WORKFLOW = "claude-wake.yml"


def wake_time(now: datetime) -> datetime:
    """Today's `WAKE_AT` in `ZONE`, the local day being `now`'s there."""
    local = now.astimezone(clock.zone(ZONE))
    return datetime(local.year, local.month, local.day, WAKE_AT.hour, WAKE_AT.minute,
                    tzinfo=clock.zone(ZONE))


def until_wake(now: datetime) -> timedelta | None:
    """How long a run started at `now` sleeps before it wakes the sessions: zero once the hour
    has come, None outside the window around it (nothing to do)."""
    at = wake_time(now)
    if at - EARLY <= now < at:
        return at - now
    if at <= now < at + LATE:
        return timedelta(0)
    return None


def already_woken(runs: list[dict[str, Any]], now: datetime) -> bool:
    """Whether a run of `WORKFLOW` already finished green since today's wake time: the
    schedule's later firings queue behind the first, and find the sessions started."""
    since = wake_time(now)
    for run in runs:
        ended = clock.parse_iso(run.get("updated_at"))
        if run.get("conclusion") == "success" and ended is not None and ended >= since:
            return True
    return False


def _running_until(entry: dict[str, Any], now: datetime) -> datetime | None:
    usage = entry.get("usage") if isinstance(entry.get("usage"), dict) else {}
    reading = usage.get("five_hour") if isinstance(usage.get("five_hour"), dict) else {}
    resets = clock.parse_iso(reading.get("resets_at"))
    return resets if resets is not None and resets > now else None


def accounts(cfg: Config, state: dict[str, Any], now: datetime,
             home: str) -> tuple[list[dict[str, str]], list[str]]:
    """The Claude subscriptions to wake, as the workflow's matrix (`id`, `secret`, `home`), and a
    line for each one left alone and why."""
    found: list[dict[str, str]] = []
    notes: list[str] = []
    for provider in cfg.pool.ordered():
        if provider.cli != "claude" or provider.login != "secret":
            continue
        if not provider.enabled:
            notes.append(f"`{provider.id}`: turned off")
            continue
        if cfg.secrets.has(provider.secret) is False:
            notes.append(f"`{provider.id}`: no `{provider.secret}` secret")
            continue
        if providers_mod.suspension(state, provider.id) is not None:
            notes.append(f"`{provider.id}`: suspended")
            continue
        running = _running_until(providers_mod.peek_record(state, provider.id), now)
        if running is not None:
            notes.append(f"`{provider.id}`: its session is running until {clock.iso(running)}")
            continue
        found.append({"id": provider.id, "secret": provider.secret, "home": home})
    return found, notes


def to_wake(ctx: Any, home: str) -> tuple[list[dict[str, str]], list[str]]:
    """`accounts` for this bot now, or none while it is halted (`HALT` or `/harness halt`)."""
    state = ctx.store.load()
    if ctx.repo_halted() or state.get("halted"):
        return [], ["halted: a wake is a model call"]
    return accounts(ctx.cfg, state, ctx.now(), home)


def wake(cfg: Config, provider_id: str) -> tuple[bool, str]:
    """One turn of `MODEL` at `EFFORT` on `provider_id`'s subscription, signed in with the job's
    `CLAUDE_CODE_OAUTH_TOKEN`; whether the CLI gave a usage reading, and what it said."""
    provider = cfg.pool.get(provider_id)
    if provider is None or provider.cli != "claude":
        return False, f"`{provider_id}` is not a Claude subscription here"
    usage = ping_usage(cfg.claude_bin, MODEL, effort=EFFORT)
    if not usage:
        return False, (f"`{provider_id}`: the CLI gave no usage reading, so the session may not "
                       "have started")
    reading = usage.get("five_hour") if isinstance(usage.get("five_hour"), dict) else {}
    resets = reading.get("resets_at") or "an unknown time"
    return True, f"`{provider_id}`: the 5-hour session is running, resetting at {resets}"
