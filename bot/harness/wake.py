"""The wake (`.github/workflows/claude-wake.yml`): at `WAKE_AT` America/Chicago and every `EVERY`
around it, each Claude subscription whose 5-hour session is not running gets one turn of the
smallest model at low effort, so its session starts then and resets five hours later, rather than
at whatever hour its next run lands. A session the state file shows running is left alone, and so
is one whose usage limit refused a call or has a window full until later, and every subscription
of a bot that is halted, since the turn is a model call. A ping its limit refuses is reported, not
failed: the next wake time tries it again.

GitHub starts this repository's schedules late and drops some, so the workflow fires every hour
and a run within `EARLY` before a wake time sleeps until it (`wake_time`); one that starts after
it, within `LATE`, wakes at once. Each subscription's turn runs in a job holding that one secret.
"""

from __future__ import annotations

from datetime import datetime, time, timedelta
from typing import Any

from harness import clock
from harness import providers as providers_mod
from harness.config import Config
from harness.runner import RATE_LIMIT_WORDS, ping

ZONE = "America/Chicago"
#: The wake times: `WAKE_AT` in `ZONE` and every `EVERY` around it, so 02:00, 06:00, 10:00,
#: 14:00, 18:00 and 22:00. `EVERY` divides a day, and `EARLY` and `LATE` add up to no more than
#: it, so each run is for at most one wake time.
WAKE_AT = time(6, 0)
EVERY = timedelta(hours=4)
#: How long before a wake time a run may start and sleep until it, and how long after it a run
#: that GitHub started late still wakes the sessions.
EARLY = timedelta(hours=1)
LATE = timedelta(hours=2)
MODEL = "claude-haiku-5-5"
EFFORT = "low"
WORKFLOW = "claude-wake.yml"


def wake_time(now: datetime) -> datetime | None:
    """The wake time a run started at `now` is for: the one at most `EARLY` ahead of it or
    `LATE` behind it, None between them (nothing to do)."""
    zone = clock.zone(ZONE)
    today = now.astimezone(zone).date()
    for day in (-1, 0, 1):
        first = datetime.combine(today + timedelta(days=day), WAKE_AT, tzinfo=zone)
        for step in range(timedelta(days=1) // EVERY):
            at = first + step * EVERY
            if at - EARLY <= now < at + LATE:
                return at
    return None


def already_woken(runs: list[dict[str, Any]], now: datetime) -> bool:
    """Whether a run of `WORKFLOW` already finished green since the wake time `now` is for: the
    schedule's later firings queue behind the first, and find the sessions started."""
    since = wake_time(now)
    if since is None:
        return False
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


def _refused_until(entry: dict[str, Any], now: datetime) -> datetime | None:
    """Until when the subscription's limit would refuse a ping: the state file says it refused
    a call, or has a window (the week's, say) full, that has not reset by `now`."""
    usage = entry.get("usage") if isinstance(entry.get("usage"), dict) else {}
    times = [clock.parse_iso(entry.get("refused_until"))]
    times += [clock.parse_iso(w.get("resets_at")) for w in usage.values()
              if isinstance(w, dict) and isinstance(w.get("utilization"), (int, float))
              and w["utilization"] >= 1.0]
    later = [t for t in times if t is not None and t > now]
    return max(later) if later else None


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
        entry = providers_mod.peek_record(state, provider.id)
        running = _running_until(entry, now)
        if running is not None:
            notes.append(f"`{provider.id}`: its session is running until {clock.iso(running)}")
            continue
        refused = _refused_until(entry, now)
        if refused is not None:
            notes.append(f"`{provider.id}`: its usage limit refuses calls until "
                         f"{clock.iso(refused)}")
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
    `CLAUDE_CODE_OAUTH_TOKEN`; whether it started the session or its usage limit refused it (no
    failure: there is nothing to start until the limit resets), and what it said."""
    provider = cfg.pool.get(provider_id)
    if provider is None or provider.cli != "claude":
        return False, f"`{provider_id}` is not a Claude subscription here"
    result, usage = ping(cfg.claude_bin, MODEL, effort=EFFORT)
    text = result.get("result") if isinstance(result, dict) else None
    limited = isinstance(result, dict) and bool(result.get("is_error")) and bool(
        RATE_LIMIT_WORDS.search(str(text or "")))
    if (usage or {}).get("status") == "rejected" or limited:
        resets = (usage or {}).get("rejected_resets_at") or "its reset"
        return True, (f"`{provider_id}`: its usage limit refused the ping, so it is left until "
                      f"{resets}; the next wake time tries it again")
    if not usage:
        return False, (f"`{provider_id}`: the CLI gave no usage reading, so the session may not "
                       "have started")
    reading = usage.get("five_hour") if isinstance(usage.get("five_hour"), dict) else {}
    resets = reading.get("resets_at") or "an unknown time"
    return True, f"`{provider_id}`: the 5-hour session is running, resetting at {resets}"
