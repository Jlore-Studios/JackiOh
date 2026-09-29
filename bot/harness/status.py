"""The `status` report: halt, window, usage, what is running and what is queued."""

from __future__ import annotations

from typing import Any

from harness import clock
from harness.clock import human_delta, parse_iso
from harness.config import (LABEL_BLOCKED, LABEL_BUILD, LABEL_PR, LABEL_REVISE, LABEL_SUGGESTION,
                            LABEL_WORKING)
from harness.context import Context
from harness.state import usage_refusal


def _numbers(items: list[dict[str, Any]]) -> str:
    if not items:
        return "none"
    return ", ".join(f"#{i['number']}" for i in items)


def _usage_line(state: dict[str, Any], ctx: Context) -> str:
    usage = state.get("usage") or {}
    if not usage:
        return "no reading yet (the first model call records one)"
    parts = []
    for window, label in (("five_hour", "5-hour"), ("seven_day", "7-day")):
        reading = usage.get(window)
        if not isinstance(reading, dict):
            continue
        stop = ctx.cfg.usage_stop.get(window)
        resets = parse_iso(reading.get("resets_at"))
        if resets is not None and resets <= ctx.now():
            parts.append(f"{label}: reset since the reading")
            continue
        text = f"{label} {float(reading.get('utilization', 0)):.0%}"
        if stop is not None:
            text += f" (stops at {stop:.0%})"
        if resets is not None:
            text += f", resets in {human_delta(resets - ctx.now())}"
        parts.append(text)
    observed = parse_iso(usage.get("observed_at"))
    age = f" (read {human_delta(ctx.now() - observed)} ago)" if observed else ""
    return "; ".join(parts) + age if parts else "no reading yet"


def report(ctx: Context) -> str:
    cfg = ctx.cfg
    now = ctx.now()
    state = ctx.store.load()
    window = ctx.window
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
    local = now.astimezone(clock.zone(cfg.timezone))
    if window.is_open(now):
        closes = window.closes_at(now)
        lines.append(f"- Window {window.describe()}: **open**, closes in "
                     f"{human_delta(closes - now) if closes else '?'} (it is {local:%H:%M} there).")
    else:
        opens = window.next_open(now)
        lines.append(f"- Window {window.describe()}: closed, opens in {human_delta(opens - now)} "
                     f"(it is {local:%H:%M} there).")
    lines.append(f"- Usage: {_usage_line(state, ctx)}.")
    refusal = usage_refusal(state, dict(cfg.usage_stop), now)
    if refusal:
        lines.append(f"- **Paused for usage**: {refusal}.")
    issues = ctx.gh.list_issues(labels="")
    def labelled(name: str, prs: bool | None = None) -> list[dict[str, Any]]:
        found = []
        for issue in issues:
            names = {label.get("name") for label in issue.get("labels", [])}
            is_pr = "pull_request" in issue
            if name in names and (prs is None or prs == is_pr):
                found.append(issue)
        return found
    lines.append(f"- Working on: {_numbers(labelled(LABEL_WORKING))}.")
    lines.append(f"- Queued to build: {_numbers(labelled(LABEL_BUILD, prs=False))}; "
                 f"to revise: {_numbers(labelled(LABEL_REVISE, prs=True))}.")
    lines.append(f"- Waiting for a person: {_numbers(labelled(LABEL_BLOCKED))}.")
    lines.append(f"- Open bot pull requests: {_numbers(labelled(LABEL_PR, prs=True))}.")
    suggestions = labelled(LABEL_SUGGESTION, prs=False)
    lines.append(f"- Suggestions open: {len(suggestions)} of {cfg.suggestions_max_open}"
                 f"{' (asked for more)' if state['suggest'].get('requested') else ''}.")
    last = state.get("last_run") or {}
    if last.get("url"):
        lines.append(f"- Last run: [{last.get('what', 'run')}]({last['url']}) at {last.get('at', '?')}.")
    lines.append(f"- Model: `{cfg.model}` at `{cfg.effort}` effort; up to {cfg.max_review_cycles} "
                 "build and review rounds per item; auto-merge "
                 f"{'on' if cfg.auto_merge else 'off'}.")
    return "\n".join(lines)
