"""The `status` report: halt, which subscriptions are running what, each subscription, and what
is queued."""

from __future__ import annotations

from datetime import datetime
from typing import Any, Callable

from harness import providers as providers_mod
from harness import review_rule
from harness.clock import human_delta, parse_iso
from harness.config import (LABEL_BLOCKED, LABEL_BUILD, LABEL_CROSS, LABEL_HUMAN, LABEL_NEEDS_PLAN,
                            LABEL_PR, LABEL_REVISE, LABEL_SUGGESTION, LABEL_WORKING, SLASH, TITLE)
from harness.context import Context
from harness.plan import run_status, working_threads
from harness.providers import Provider

#: What a run is doing to an item, by the item's kind (`queue.KIND_ORDER`).
DOING = {"plan": "planning", "build": "building", "revise": "revising",
         "review": "reviewing"}


def machine_text(pool: Any, live: dict[int, str]) -> str:
    """How many runs are on the bot's machine: "2 of 6 on the night box"."""
    night = sum(1 for p in live.values() if pool.on_machine(p))
    return f"{night} of {pool.machine_parallel} on the night box"


def _numbers(items: list[dict[str, Any]]) -> str:
    if not items:
        return "none"
    return ", ".join(f"#{i['number']}" for i in items)


def cap_text(ctx: Context, provider: Provider, window: str) -> str:
    """The cap on a usage window now: "cap 90%" or "no cap", and for a subscription whose cap
    changes with its hours (`off_hours`) the other one too: "cap 50% now, 70% in its hours"."""
    cfg = ctx.cfg
    now = ctx.now()
    cap = provider.caps_at(now, cfg.timezone).get(window)
    text = "no cap" if cap is None else f"cap {cap:.0%}"
    hours = provider.schedule.window(cfg.timezone)
    if hours is None or window not in provider.off_hours:
        return text
    own = provider.limits.stops.get(window)
    if hours.is_open(now):
        outside = min(1.0 if own is None else own, provider.off_hours[window])
        return f"{text} now, {outside:.0%} outside its hours"
    return f"{text} now, " + ("none" if own is None else f"{own:.0%}") + " in its hours"


def window_notes(ctx: Context, provider: Provider, entry: dict[str, Any], window: str,
                 when: Callable[[datetime], str] | None = None) -> list[str]:
    """What a reader needs beside a usage window's reading: its cap now (`cap_text`), the line a
    build, a revision or a plan must start under (`start_headroom`), the minutes it has spent of
    a budget, and when the window resets (`when` writes the time: "in 3h 10m" by default)."""
    now = ctx.now()
    when = when or (lambda at: f"in {human_delta(at - now)}")
    cap = provider.caps_at(now, ctx.cfg.timezone).get(window)
    budget = provider.limits.budgets.get(window)
    usage = entry.get("usage") if isinstance(entry.get("usage"), dict) else {}
    reading = usage.get(window)
    notes = []
    if cap is not None or (budget is None and isinstance(reading, dict)):
        notes.append(cap_text(ctx, provider, window))
    if cap is not None:
        start = providers_mod.start_under(provider, window, cap)
        if start < cap:
            notes.append(f"builds and plans start under {start:.0%}")
    if budget is not None:
        since = now - providers_mod.WINDOWS[window]
        notes.append(f"{providers_mod.minutes_spent(entry, since):.0f} of {budget} min")
    if isinstance(reading, dict):
        observed = parse_iso(usage.get("observed_at"))
        resets = parse_iso(reading.get("resets_at")) or (
            observed + providers_mod.WINDOWS[window] if observed else None)
        if resets is not None and resets > now:
            notes.append(f"resets {when(resets)}")
    return notes


def _usage_text(provider: Provider, entry: dict[str, Any], ctx: Context) -> str:
    usage = entry.get("usage") or {}
    parts = []
    noted = set()  # the windows whose notes say their minutes
    for window, label in (("five_hour", "5-hour"), ("seven_day", "7-day")):
        reading = usage.get(window) if isinstance(usage, dict) else None
        if not isinstance(reading, dict):
            continue
        resets = parse_iso(reading.get("resets_at"))
        if resets is not None and resets <= ctx.now():
            parts.append(f"{label}: reset since the reading")
            continue
        text = f"{label} {float(reading.get('utilization', 0)):.0%}"
        notes = window_notes(ctx, provider, entry, window)
        noted.add(window)
        if notes:
            text += f" ({'; '.join(notes)})"
        parts.append(text)
    for window, budget in provider.limits.budgets.items():
        if window in noted:
            continue
        since = ctx.now() - providers_mod.WINDOWS[window]
        spent = providers_mod.minutes_spent(entry, since)
        parts.append(f"{providers_mod.WINDOW_NAMES[window]} {spent:.0f}/{budget} min")
    observed = parse_iso(usage.get("observed_at")) if isinstance(usage, dict) else None
    if observed and parts:
        parts[-1] += f" (read {human_delta(ctx.now() - observed)} ago)"
    return "; ".join(parts) or "no reading yet"


def _record(state: dict[str, Any], number: int) -> dict[str, Any]:
    """The state record of a lane: an item's, or the suggestion survey's (number 0)."""
    if number == 0:
        return state.get("suggest") or {}
    return state["items"].get(str(number), {})


record_of = _record


def lanes_now(ctx: Context, state: dict[str, Any]) -> tuple[dict[int, str], dict[int, str]]:
    """The lanes the state file says are held (item number, 0 for a survey -> provider), and those
    of them whose run GitHub has not seen end."""
    held: dict[int, str] = {}
    for issue in working_threads(ctx):  # closed ones too: a run goes on until it ends
        record = state["items"].get(str(issue["number"]), {})
        held[int(issue["number"])] = str(record.get("provider") or providers_mod.LEGACY_PROVIDER)
    survey = state.get("suggest") or {}
    if survey.get("provider"):
        held[0] = str(survey["provider"])
    return held, live_lanes(ctx, state, held)


def live_lanes(ctx: Context, state: dict[str, Any], held: dict[int, str]) -> dict[int, str]:
    """`held` without the runs GitHub says have ended (the next plan requeues their items). A run
    GitHub cannot read counts as still going, as it does for the plan (`plan.read_lanes`)."""
    return {number: provider for number, provider in held.items()
            if run_status(ctx, _record(state, number).get("run_id")) != "dead"}


def running_lines(ctx: Context, state: dict[str, Any], live: dict[int, str]) -> list[str]:
    """Which subscriptions are running now: on what, for how long, and in which run."""
    cfg = ctx.cfg
    lanes = cfg.pool.max_parallel

    def planning_run(number: int) -> bool:
        return bool(number) and _record(state, number).get("action") == "plan"
    planning = sum(1 for number in live if planning_run(number))
    # GitHub's runners and the machine have lanes of their own (`plan.Lanes`).
    hosted = sum(1 for number, provider_id in live.items()
                 if not planning_run(number) and not cfg.pool.on_machine(provider_id))
    free = max(0, lanes - hosted)
    planning_text = (f"; {planning} of {cfg.pool.plan_lanes} planning lanes"
                     if cfg.pool.plan_lanes else "")
    if not live:
        return [f"- Running now: nothing ({free} of {lanes} lanes free on GitHub's runners"
                + (f", {cfg.pool.machine_parallel} on the night box"
                   if cfg.pool.machine_parallel else "")
                + (f", and {cfg.pool.plan_lanes} for planning" if cfg.pool.plan_lanes else "")
                + ")."]
    order = {provider_id: i for i, provider_id in enumerate(cfg.pool.priority)}
    lines = [f"- **Running now** ({hosted} of {lanes} lanes on GitHub's runners, {free} free; "
             f"{machine_text(cfg.pool, live)}{planning_text}):"]
    for number, provider_id in sorted(live.items(),
                                      key=lambda kv: (order.get(kv[1], len(order)), kv[0])):
        record = _record(state, number)
        if number:
            doing = "plan" if record.get("action") == "plan" else str(record.get("kind"))
            what = f"{DOING.get(doing, 'working on')} #{number}"
            since = parse_iso(record.get("started_at"))
        else:
            what = "a suggestion survey"
            since = parse_iso(record.get("last_run"))
        provider = cfg.pool.get(provider_id)
        name = f"`{provider_id}`" + (f" ({provider.cli}, `{provider.model}`)" if provider else "")
        detail = []
        if since is not None:
            elapsed = human_delta(ctx.now() - since)
            detail.append("just started" if elapsed == "now" else f"for {elapsed}")
        if record.get("run_id"):
            detail.append(f"[run]({cfg.server_url}/{cfg.repo}/actions/runs/{record['run_id']})")
        lines.append(f"  - {name}: {what}" + (f", {', '.join(detail)}" if detail else "") + ".")
    return lines


def provider_lines(ctx: Context, state: dict[str, Any], held: dict[int, str]) -> list[str]:
    cfg = ctx.cfg
    busy = {p: n for n, p in held.items()}
    lines = []
    for provider in cfg.pool.ordered():
        entry = providers_mod.peek_record(state, provider.id)
        # An idle subscription waits for a build, a revision or a plan, which start only
        # `start_headroom` under each cap (`plan._usable`).
        reason = providers_mod.start_reason(provider, state, ctx.now(), cfg.timezone, cfg.secrets)
        if provider.id in busy:
            number = busy[provider.id]
            now_doing = f"**working on #{number}**" if number else "**running a survey**"
        elif reason is None:
            now_doing = "free"
        else:
            now_doing = reason
        seats = ", ".join(f"`{seat.model}` {seat.tier}" + (", self-check" if seat.self_check
                                                         else "")
                          for seat in cfg.pool.seats(provider))
        lines.append(f"  - `{provider.id}` ({provider.cli}: {seats}; "
                     f"{provider.hours(cfg.timezone)}): {now_doing}. "
                     f"Usage: {_usage_text(provider, entry, ctx)}.")
    return lines


def report(ctx: Context) -> str:
    cfg = ctx.cfg
    state = ctx.store.load()
    lines = [f"**{TITLE} status**", ""]
    halt = state.get("halt") or {}
    if state.get("halted"):
        by = f" by @{halt.get('by')}" if halt.get("by") else ""
        why = f": {halt.get('reason')}" if halt.get("reason") else ""
        lines.append(f"- **Halted**{by}{why}. `{SLASH} start` resumes it.")
    else:
        lines.append("- Not halted.")
    if ctx.repo_halted():
        lines.append("- **`.harness/HALT` is on `main`**: nothing runs until that file is deleted.")
    issues = ctx.gh.list_issues(labels="")

    def labelled(name: str, prs: bool | None = None) -> list[dict[str, Any]]:
        found = []
        for issue in issues:
            names = {label.get("name") for label in issue.get("labels", [])}
            is_pr = "pull_request" in issue
            if name in names and (prs is None or prs == is_pr):
                found.append(issue)
        return found

    held, live = lanes_now(ctx, state)
    lines += running_lines(ctx, state, live)
    lines.append(f"- Subscriptions (at most {cfg.pool.max_parallel} at once on GitHub's runners "
                 f"and {cfg.pool.machine_parallel} on the machine):")
    lines += provider_lines(ctx, state, live)
    ended = ", ".join(f"#{n}" for n in sorted(held) if n and n not in live)
    lines.append(f"- Working on: {_numbers(labelled(LABEL_WORKING))}"
                 + (f" (no run is going for {ended} any more; the next plan requeues it)"
                    if ended else "") + ".")
    lines.append(f"- **Needs plan** (rated and planned first, into the description: an easy or "
                 f"unrated item by a medium or strong model, the rest by a strong one): "
                 f"{_numbers(labelled(LABEL_NEEDS_PLAN, prs=False))}.")
    lines.append(f"- Queued to build: {_numbers(labelled(LABEL_BUILD, prs=False))}; "
                 f"to revise: {_numbers(labelled(LABEL_REVISE, prs=True))}; "
                 f"for a review run: {_numbers(labelled(LABEL_CROSS, prs=True))}.")
    lines.append(f"- Waiting for a person: {_numbers(labelled(LABEL_BLOCKED))}.")
    lines.append(f"- People's work (`{LABEL_HUMAN}`, never touched): "
                 f"{_numbers(labelled(LABEL_HUMAN))}.")
    lines.append(f"- Open bot pull requests: {_numbers(labelled(LABEL_PR, prs=True))}.")
    suggestions = labelled(LABEL_SUGGESTION, prs=False)
    lines.append(f"- Suggestions open: {len(suggestions)} of {cfg.suggestions_max_open}"
                 f"{' (asked for more)' if state['suggest'].get('requested') else ''}.")
    last = state.get("last_run") or {}
    if last.get("url"):
        lines.append(f"- Last run: [{last.get('what', 'run')}]({last['url']}) at {last.get('at', '?')}.")
    lines.append(f"- Up to {cfg.max_review_cycles} build and review rounds per item, after a "
                 "plan on the planning lane (or by its builder when none is free), which goes "
                 "into the issue's description, by a medium or strong model for an easy or "
                 "unrated item and a strong one for the rest; the planner rates an unrated item. "
                 "An item's `difficulty:easy|medium|hard` (medium until rated) sets the weakest "
                 "tier that may build it, and three failures of its own raise it a step. A "
                 "change merges once "
                 f"{review_rule.SUMMARY} approved it. A self-checking builder "
                 f"checks itself up to {cfg.max_self_check_rounds} times first. Auto-merge "
                 f"{'on' if cfg.auto_merge else 'off'}.")
    return "\n".join(lines)
