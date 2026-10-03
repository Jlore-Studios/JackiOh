"""The `status` report: halt, which subscriptions are running what, each subscription, and what
is queued."""

from __future__ import annotations

from typing import Any

from harness import providers as providers_mod
from harness.clock import human_delta, parse_iso
from harness.config import (LABEL_BLOCKED, LABEL_BUILD, LABEL_CROSS, LABEL_PR, LABEL_REVISE,
                            LABEL_SUGGESTION, LABEL_WORKING)
from harness.context import Context
from harness.plan import run_status, working_threads
from harness.providers import Provider

#: What a run is doing to an item, by the item's kind (`queue.KIND_ORDER`).
DOING = {"build": "building", "revise": "revising", "review": "giving a second review to"}


def _numbers(items: list[dict[str, Any]]) -> str:
    if not items:
        return "none"
    return ", ".join(f"#{i['number']}" for i in items)


def _usage_text(provider: Provider, entry: dict[str, Any], ctx: Context) -> str:
    usage = entry.get("usage") or {}
    parts = []
    for window, label in (("five_hour", "5-hour"), ("seven_day", "7-day")):
        reading = usage.get(window) if isinstance(usage, dict) else None
        if not isinstance(reading, dict):
            continue
        resets = parse_iso(reading.get("resets_at"))
        if resets is not None and resets <= ctx.now():
            parts.append(f"{label}: reset since the reading")
            continue
        text = f"{label} {float(reading.get('utilization', 0)):.0%}"
        cap = provider.limits.stops.get(window)
        if cap is not None:
            text += f" (cap {cap:.0%})"
        parts.append(text)
    for window, budget in provider.limits.budgets.items():
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


def live_lanes(ctx: Context, state: dict[str, Any], held: dict[int, str]) -> dict[int, str]:
    """`held` without the runs GitHub says have ended (the next plan requeues their items). A run
    GitHub cannot read counts as still going, as it does for the plan (`plan.read_lanes`)."""
    return {number: provider for number, provider in held.items()
            if run_status(ctx, _record(state, number).get("run_id")) != "dead"}


def running_lines(ctx: Context, state: dict[str, Any], live: dict[int, str]) -> list[str]:
    """Which subscriptions are running now: on what, for how long, and in which run."""
    cfg = ctx.cfg
    lanes = cfg.pool.max_parallel
    free = max(0, lanes - len(live))
    if not live:
        return [f"- Running now: nothing ({free} of {lanes} lanes free)."]
    order = {provider_id: i for i, provider_id in enumerate(cfg.pool.priority)}
    on_machine = sum(1 for provider_id in live.values() if cfg.pool.on_machine(provider_id))
    lines = [f"- **Running now** ({len(live)} of {lanes} lanes, {free} free; {on_machine} of "
             f"{cfg.pool.machine_parallel} on the machine, the rest on GitHub's runners):"]
    for number, provider_id in sorted(live.items(),
                                      key=lambda kv: (order.get(kv[1], len(order)), kv[0])):
        record = _record(state, number)
        if number:
            what = f"{DOING.get(str(record.get('kind')), 'working on')} #{number}"
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
        reason = providers_mod.availability(provider, state, ctx.now(), cfg.timezone, cfg.secrets)
        if provider.id in busy:
            number = busy[provider.id]
            now_doing = f"**working on #{number}**" if number else "**running a survey**"
        elif reason is None:
            now_doing = "free"
        else:
            now_doing = reason
        lines.append(f"  - `{provider.id}` ({provider.cli}, `{provider.model}`, "
                     f"{provider.schedule.describe(cfg.timezone)}): {now_doing}. "
                     f"Usage: {_usage_text(provider, entry, ctx)}.")
    return lines


def report(ctx: Context) -> str:
    cfg = ctx.cfg
    state = ctx.store.load()
    lines = ["**Night bot status**", ""]
    halt = state.get("halt") or {}
    if state.get("halted"):
        by = f" by @{halt.get('by')}" if halt.get("by") else ""
        why = f": {halt.get('reason')}" if halt.get("reason") else ""
        lines.append(f"- **Halted**{by}{why}. `/harness start` resumes it.")
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

    held: dict[int, str] = {}
    for issue in working_threads(ctx):  # closed ones too: a run goes on until it ends
        record = state["items"].get(str(issue["number"]), {})
        held[int(issue["number"])] = str(record.get("provider") or providers_mod.LEGACY_PROVIDER)
    survey = state.get("suggest") or {}
    if survey.get("provider"):
        held[0] = str(survey["provider"])
    live = live_lanes(ctx, state, held)
    lines += running_lines(ctx, state, live)
    lines.append(f"- Subscriptions (at most {cfg.pool.max_parallel} at once, "
                 f"{cfg.pool.machine_parallel} of them on the machine):")
    lines += provider_lines(ctx, state, live)
    ended = ", ".join(f"#{n}" for n in sorted(held) if n and n not in live)
    lines.append(f"- Working on: {_numbers(labelled(LABEL_WORKING))}"
                 + (f" (no run is going for {ended} any more; the next plan requeues it)"
                    if ended else "") + ".")
    lines.append(f"- Queued to build: {_numbers(labelled(LABEL_BUILD, prs=False))}; "
                 f"to revise: {_numbers(labelled(LABEL_REVISE, prs=True))}; "
                 f"for a second review: {_numbers(labelled(LABEL_CROSS, prs=True))}.")
    lines.append(f"- Waiting for a person: {_numbers(labelled(LABEL_BLOCKED))}.")
    lines.append(f"- Open bot pull requests: {_numbers(labelled(LABEL_PR, prs=True))}.")
    suggestions = labelled(LABEL_SUGGESTION, prs=False)
    lines.append(f"- Suggestions open: {len(suggestions)} of {cfg.suggestions_max_open}"
                 f"{' (asked for more)' if state['suggest'].get('requested') else ''}.")
    last = state.get("last_run") or {}
    if last.get("url"):
        lines.append(f"- Last run: [{last.get('what', 'run')}]({last['url']}) at {last.get('at', '?')}.")
    lines.append(f"- Up to {cfg.max_review_cycles} build and review rounds per item. A change "
                 "merges on its builder's model's approval when that model is enough by itself "
                 "(Opus), else after a second model approves it too; auto-merge "
                 f"{'on' if cfg.auto_merge else 'off'}.")
    return "\n".join(lines)
