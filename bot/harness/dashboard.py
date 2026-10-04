"""The pinned status issue: one issue, pinned to the top of the issue list, that every sweep (every
ten minutes, `bot-commands.yml`) rewrites with what the night bot is doing.

It shows the same facts as `/harness status` (`status.py`), drawn for a glance: a timeline of the
runs going now, the lanes in use, each subscription's usage as bars, the queue and the last runs,
and the full status text folded underneath. Editing an issue's body notifies nobody, so the
rewrites stay quiet. The issue's number is kept in the state file; when it is missing or the
issue was closed, an open issue with the title and the marker, opened by the bot or someone on
the trust list, is adopted, or a new one is opened and pinned.
"""

from __future__ import annotations

from datetime import datetime
from typing import Any

from harness import providers as providers_mod
from harness import status as status_mod
from harness.clock import human_delta, parse_iso, zone
from harness.config import (LABEL_BUILD, LABEL_CROSS, LABEL_NEEDS_PLAN, LABEL_PLANNED, LABEL_REVISE,
                            NIGHT_WORKFLOW)
from harness.context import Context
from harness.errors import GitHubError
from harness.queue import difficulty_of, label_names

TITLE = "Night bot status"
MARKER = "<!-- jackioh-bot:dashboard -->"
LABELS = ("night bot",)
#: How much of each list the issue shows.
QUEUE_ROWS = 15
RUN_ROWS = 8
BAR_CELLS = 10
RESULT = {"success": "✅", "failure": "❌", "cancelled": "⏹️", "skipped": "⏭️",
          "timed_out": "⌛", "startup_failure": "❌"}


def _local(ctx: Context, at: datetime) -> datetime:
    return at.astimezone(zone(ctx.cfg.timezone))


def _cell(text: Any, limit: int = 70) -> str:
    """Text for a table cell: one line, no pipes, clipped."""
    flat = " ".join(str(text or "").split()).replace("|", "\\|")
    return flat if len(flat) <= limit else flat[: limit - 1] + "…"


def bar(fraction: float | None) -> str:
    """`▰▰▰▱▱▱▱▱▱▱ 30%`, or a dash with no reading."""
    if fraction is None:
        return "—"
    fraction = min(max(float(fraction), 0.0), 1.0)
    full = round(fraction * BAR_CELLS)
    return "▰" * full + "▱" * (BAR_CELLS - full) + f" {fraction:.0%}"


def _reading(ctx: Context, entry: dict[str, Any], window: str) -> float | None:
    usage = entry.get("usage") if isinstance(entry.get("usage"), dict) else {}
    reading = usage.get(window) if isinstance(usage, dict) else None
    if not isinstance(reading, dict):
        return None
    resets = parse_iso(reading.get("resets_at"))
    if resets is not None and resets <= ctx.now():
        return 0.0  # the window reset since the reading
    try:
        return float(reading.get("utilization", 0))
    except (TypeError, ValueError):
        return None


def _what(state: dict[str, Any], number: int) -> tuple[str, datetime | None]:
    record = status_mod.record_of(state, number)
    if not number:
        return "survey", parse_iso(record.get("last_run"))
    doing = "plan" if record.get("action") == "plan" else str(record.get("kind") or "work")
    return f"item {number} {doing}", parse_iso(record.get("started_at"))


def running_table(ctx: Context, state: dict[str, Any], live: dict[int, str],
                  issues: list[dict[str, Any]]) -> list[str]:
    """What each lane is doing now, with a link to the run doing it."""
    if not live:
        return []
    cfg = ctx.cfg
    titles = {int(issue["number"]): issue.get("title") for issue in issues}
    order = {provider_id: i for i, provider_id in enumerate(cfg.pool.priority)}
    rows = []
    for number, provider_id in sorted(live.items(),
                                      key=lambda kv: (order.get(kv[1], len(order)), kv[0])):
        record = status_mod.record_of(state, number)
        if number:
            kind = "plan" if record.get("action") == "plan" else str(record.get("kind"))
            doing = status_mod.DOING.get(kind, "working on")
            item, since = f"#{number}", parse_iso(record.get("started_at"))
        else:
            doing, item, since = "suggestion survey", "—", parse_iso(record.get("last_run"))
        where = "machine" if cfg.pool.on_machine(provider_id) else "GitHub"
        provider = cfg.pool.get(provider_id)
        model = f" `{provider.model}`" if provider else ""
        elapsed = human_delta(ctx.now() - since) if since else ""
        run_id = record.get("run_id")
        run = (f"[run {run_id}]({cfg.server_url}/{cfg.repo}/actions/runs/{run_id})"
               if run_id else "—")
        rows.append(f"| {item} | {_cell(titles.get(number, ''), 50)} | {doing} "
                    f"| `{provider_id}`{model} ({where}) "
                    f"| {'just started' if elapsed in ('', 'now') else elapsed} | {run} |")
    return (["| Item | Title | Doing | Subscription | For | Run |",
             "|---|---|---|---|---|---|"] + rows)


def timeline(ctx: Context, state: dict[str, Any], live: dict[int, str]) -> list[str]:
    """A Mermaid timeline of the runs going now, one section per subscription."""
    if not live:
        return ["Nothing is running right now."]
    now = _local(ctx, ctx.now())
    fmt = "%Y-%m-%d %H:%M"
    lines = ["```mermaid", "gantt", "    title Running now (Central time)",
             "    dateFormat YYYY-MM-DD HH:mm", "    axisFormat %H:%M"]
    order = {provider_id: i for i, provider_id in enumerate(ctx.cfg.pool.priority)}
    section = None
    for number, provider_id in sorted(live.items(),
                                      key=lambda kv: (order.get(kv[1], len(order)), kv[0])):
        what, since = _what(state, number)
        start = _local(ctx, since) if since else now
        if provider_id != section:  # one section per subscription, its lanes under it
            where = "machine" if ctx.cfg.pool.on_machine(provider_id) else "GitHub"
            lines.append(f"    section {provider_id} ({where})")
            section = provider_id
        lines.append(f"    {what} :active, {start.strftime(fmt)}, {now.strftime(fmt)}")
    lines.append("```")
    return lines


def lanes_chart(ctx: Context, live: dict[int, str]) -> list[str]:
    """A Mermaid pie of the lanes: planning, on the machine, on GitHub's runners, free."""
    pool = ctx.cfg.pool
    state = ctx.store.load()
    planning = sum(1 for number in live if number
                   and state["items"].get(str(number), {}).get("action") == "plan")
    building = {n: p for n, p in live.items()
                if not (n and state["items"].get(str(n), {}).get("action") == "plan")}
    machine = sum(1 for provider_id in building.values() if pool.on_machine(provider_id))
    slices = [("Planning", planning), ("On the machine", machine),
              ("On GitHub's runners", len(building) - machine),
              ("Free", max(0, pool.max_parallel - len(building))
               + max(0, pool.plan_lanes - planning))]
    planning_lanes = f", and {pool.plan_lanes} for planning" if pool.plan_lanes else ""
    return (["```mermaid", f"pie showData title Lanes ({pool.max_parallel}, at most "
             f"{pool.machine_parallel} on the machine{planning_lanes})"]
            + [f'    "{name}" : {count}' for name, count in slices if count > 0] + ["```"])


def subscription_table(ctx: Context, state: dict[str, Any], live: dict[int, str]) -> list[str]:
    cfg = ctx.cfg
    busy: dict[str, list[int]] = {}
    for number, provider_id in live.items():
        busy.setdefault(provider_id, []).append(number)
    lines = ["| Subscription | Models (tier) | Hours | Now | 5-hour | 7-day |",
             "|---|---|---|---|---|---|"]
    for provider in cfg.pool.ordered():
        entry = providers_mod.peek_record(state, provider.id)
        if provider.id in busy:
            now = "🟢 " + ", ".join(f"#{n}" if n else "survey" for n in sorted(busy[provider.id]))
        else:
            reason = providers_mod.availability(provider, state, ctx.now(), cfg.timezone,
                                                cfg.secrets)
            now = "⚪ free" if reason is None else "⏸️ " + _cell(reason, 60)
        seats = ", ".join(f"`{seat.model}` {seat.tier}" for seat in cfg.pool.seats(provider))
        lines.append(f"| `{provider.id}` | {seats} | {_cell(provider.hours(cfg.timezone))} "
                     f"| {now} | {bar(_reading(ctx, entry, 'five_hour'))} "
                     f"| {bar(_reading(ctx, entry, 'seven_day'))} |")
    return lines


def queue_table(issues: list[dict[str, Any]]) -> list[str]:
    rows = []
    # The Needs plan stage first: the planning lane takes those before anything else.
    issues = sorted(issues, key=lambda issue: LABEL_NEEDS_PLAN not in label_names(issue))
    for issue in issues:
        names = label_names(issue)
        is_pr = "pull_request" in issue
        if LABEL_BUILD in names and not is_pr:
            kind = ("**needs plan**" if LABEL_NEEDS_PLAN in names
                    else "build (planned)" if LABEL_PLANNED in names else "build")
        elif LABEL_REVISE in names and is_pr:
            kind = "revise"
        elif LABEL_CROSS in names and is_pr:
            kind = "review"
        else:
            continue
        priority = next((name.split(":", 1)[1] for name in sorted(names)
                         if name.lower().startswith("priority:")), "—")
        rows.append(f"| #{issue['number']} | {kind} | {difficulty_of(names)} | {priority} "
                    f"| {_cell(issue.get('title'))} |")
    if not rows:
        return ["Nothing is queued."]
    more = len(rows) - QUEUE_ROWS
    return (["| Item | Kind | Difficulty | Priority | Title |", "|---|---|---|---|---|"]
            + rows[:QUEUE_ROWS] + ([f"\n…and {more} more."] if more > 0 else []))


def runs_table(ctx: Context) -> list[str]:
    try:
        runs = ctx.gh.list_runs(NIGHT_WORKFLOW, limit=RUN_ROWS)
    except GitHubError:
        return ["The last runs could not be read."]
    rows = []
    for run in runs[:RUN_ROWS]:
        at = parse_iso(run.get("created_at"))
        when = _local(ctx, at).strftime("%a %H:%M") if at else "?"
        result = (RESULT.get(str(run.get("conclusion")), str(run.get("conclusion")))
                  if run.get("status") == "completed" else "🔄 running")
        link = f"[{run.get('run_number') or 'run'}]({run['html_url']})" if run.get("html_url") \
            else str(run.get("id", ""))
        rows.append(f"| {when} | {_cell(run.get('event'), 20)} | {result} | {link} |")
    if not rows:
        return ["No runs yet."]
    return ["| Started (Central) | Trigger | Result | Run |", "|---|---|---|---|"] + rows


def render(ctx: Context) -> str:
    state = ctx.store.load()
    issues = ctx.gh.list_issues(labels="")
    held, live = status_mod.lanes_now(ctx, state)
    updated = _local(ctx, ctx.now()).strftime("%Y-%m-%d %H:%M %Z")
    halted = state.get("halted") or ctx.repo_halted()
    lines = [f"# 🤖 {TITLE}", "",
             f"{'🛑 **Halted.** ' if halted else ''}_Updated {updated} by `bot-status`, "
             "which rewrites this issue every ten minutes. `/harness status` gives the same facts "
             "on demand._", "",
             "## What it is working on", ""]
    running = running_table(ctx, state, live, issues)
    lines += (running + [""] if running else [])
    lines += timeline(ctx, state, live) + [""] + lanes_chart(ctx, live) + [""]
    lines += ["## Subscriptions", ""] + subscription_table(ctx, state, live) + [""]
    lines += ["## Queue", ""] + queue_table(issues) + [""]
    lines += ["## Last night-bot runs", ""] + runs_table(ctx) + [""]
    lines += ["<details><summary>The full status</summary>", "",
              status_mod.report(ctx), "", "</details>", "", MARKER]
    return "\n".join(lines)


def _trusted(ctx: Context, issue: dict[str, Any]) -> bool:
    user = issue.get("user") or {}
    login = str(user.get("login") or "")
    return (login.lower() == ctx.cfg.bot_login.lower()
            or ctx.trust.level(login, user.get("id"), issue.get("author_association")) >= 1)


def find(ctx: Context, state: dict[str, Any]) -> dict[str, Any] | None:
    """The open status issue: the one the state file names, or one to adopt."""
    number = (state.get("dashboard") or {}).get("issue")
    if number:
        try:
            issue = ctx.gh.get_issue(int(number))
            if issue.get("state") == "open" and "pull_request" not in issue:
                return issue
        except GitHubError:
            pass
    for issue in ctx.gh.list_issues(labels=""):
        if ("pull_request" not in issue and issue.get("title") == TITLE
                and MARKER in str(issue.get("body") or "") and _trusted(ctx, issue)):
            return issue
    return None


def update(ctx: Context) -> str:
    """Rewrite the status issue, opening and pinning it first if there is none."""
    body = render(ctx)
    state = ctx.store.load()
    issue = find(ctx, state)
    if issue is None:
        issue = ctx.gh.create_issue(TITLE, body, LABELS)
        number = int(issue["number"])
        note = f"opened #{number}"
        try:
            ctx.gh.pin_issue(str(issue.get("node_id") or ""))
            note += " and pinned it"
        except GitHubError as exc:
            note += f"; could not pin it ({exc.status}): pin it by hand"
    else:
        number = int(issue["number"])
        ctx.gh.update_issue(number, body=body)
        note = f"rewrote #{number}"
    if (state.get("dashboard") or {}).get("issue") != number:
        ctx.store.update(lambda s: s.update(dashboard={"issue": number}), "dashboard issue")
    return note
