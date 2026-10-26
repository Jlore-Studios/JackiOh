"""The pinned status issue (#148): one issue, pinned to the top of the issue list, that the
`bot-status` loop rewrites every five minutes with what the night bot is doing and what it has
done.

It opens with a headline (what runs, what waits and on what, which subscriptions can start work),
then draws the same facts as `/harness status` (`status.py`) for a glance: what each lane is doing
and a timeline of the runs going now; every subscription as a box with what it is doing or why it
is not, and its usage, always; the queue, each item with the tier its difficulty needs and what it
waits for (`waiting.py`, the planner's own rules on the lanes held now: never "free" for a
subscription the planner would give no work, never "starts next" for an item it would not start);
each subscription's usage as bars and charts against its caps; the last runs; the statistics
(`stats.py`: the last six hours, day, week and all time, counted every 30 minutes and kept in the
issue between counts, with which set's cards win most, `cardsets.py`, read from production once a
day); and the full status text folded underneath. Times are clock times, which
stay true between rewrites; hovering one shows how long it had been running when the issue was
written. Editing an issue's body notifies nobody, so the rewrites stay quiet. The issue's number
is kept in the state file; when it is missing or the issue was closed, an open issue with the
title and the marker, opened by the bot or someone on the trust list, is adopted, or a new one is
opened and pinned.

Every runner has a section too, low in the issue (`runners_section`): each job a runner holds now,
the step it is on and what it is for, and the jobs queued for one, as boxes the way the
subscriptions are drawn and as a table, each folding away. The owner's PC's group carries its load
and the gather workflow, from a heartbeat its supervisor writes to a gist (`read_runners`). Between
rewrites the loop redraws that section alone every two minutes (`refresh_runners`). What each runner
and each workflow is for changes only with the pool or the workflows, so it is a comment on the issue
(`explain`), which the section links.

Squishy (#60) has a section of its own in the same issue, apart from the night bot's: what it is
working on, its account, its queue and its trees. Squishy's own process draws it (`section`), and
the night bot's status loop runs that process each tick and puts the section in (`update`'s
`extra`), so each bot's facts come from its own state and labels. Its record goes in the
statistics, folded, drawn the same way (`stats.bot_section`).
"""

from __future__ import annotations

import hashlib
import html
import json
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from typing import Any, Callable

from harness import cardsets as cardsets_mod
from harness import disk as disk_mod
from harness import memory as memory_mod
from harness import plan as plan_mod
from harness import providers as providers_mod
from harness import stats as stats_mod
from harness import status as status_mod
from harness import waiting as waiting_mod
from harness.clock import human_delta, parse_iso, zone
from harness.config import LABEL_BLOCKED, LABEL_TREE, MODES, NIGHT_WORKFLOW, SLASH
from harness.config import TITLE as BOT_TITLE
from harness.context import Context
from harness.errors import GitHubError
from harness.queue import label_names, mode_of

TITLE = "Night bot status"
MARKER = "<!-- jackioh-bot:dashboard -->"
LABELS = ("night bot",)
#: How often the `bot-status` loop rewrites the issue (`EVERY_SECONDS` in bot-status.yml).
EVERY_MINUTES = 5
#: GitHub refuses an issue body over 65,536 characters; the issue keeps under this (`fit`).
MAX_BODY = 65_000
#: How much of each list the issue shows.
QUEUE_ROWS = 15
RUN_ROWS = 8
BAR_CELLS = 10
BOX_CELLS = 5
RESULT = {"success": "✅", "failure": "❌", "cancelled": "⏹️", "skipped": "⏭️",
          "timed_out": "⌛", "startup_failure": "❌"}
#: How the statistics block shrinks when the issue would pass `MAX_BODY`: fewer merged pull
#: requests listed.
SHOWN_STEPS = (stats_mod.MERGES_SHOWN, stats_mod.MERGES_SHOWN // 2, 10, 0)


def _local(ctx: Context, at: datetime) -> datetime:
    return at.astimezone(zone(ctx.cfg.timezone))


def _clock(ctx: Context, at: datetime) -> str:
    """`20:27` today (Central time), `Fri 18:47` on another day."""
    local, today = _local(ctx, at), _local(ctx, ctx.now())
    return local.strftime("%H:%M" if local.date() == today.date() else "%a %H:%M")


def _hover(text: str, title: str, url: str = "") -> str:
    """`text` that shows `title` when hovered: a link with a title when there is somewhere to go,
    otherwise a span (GitHub keeps `title` on both, and strips `<abbr>`)."""
    if url:
        return f'[{text}]({url} "{title.replace(chr(34), chr(39))}")'
    return f'<span title="{html.escape(title, quote=True)}">{text}</span>'


def _running_for(ctx: Context, since: datetime) -> str:
    """How long a run had been going when this rewrite was written."""
    written = _clock(ctx, ctx.now())
    elapsed = human_delta(ctx.now() - since)
    if elapsed == "now":
        return f"just started when this was written at {written}"
    return f"running {elapsed} when this was written at {written}"


def _run_url(ctx: Context, run_id: Any) -> str:
    if not run_id or str(run_id).startswith("pending:"):
        return ""  # the status loop started it and has not found its run yet (#1455)
    return f"{ctx.cfg.server_url}/{ctx.cfg.repo}/actions/runs/{run_id}"


def _cell(text: Any, limit: int = 70) -> str:
    """Text for a table cell: one line, no pipes, clipped."""
    flat = " ".join(str(text or "").split()).replace("|", "\\|")
    return flat if len(flat) <= limit else flat[: limit - 1] + "…"


def bar(fraction: float | None, cells: int = BAR_CELLS) -> str:
    """`▰▰▰▱▱▱▱▱▱▱ 30%`, or a dash with no reading."""
    if fraction is None:
        return "—"
    fraction = min(max(float(fraction), 0.0), 1.0)
    full = round(fraction * cells)
    return "▰" * full + "▱" * (cells - full) + f" {fraction:.0%}"


def _window_reading(ctx: Context, provider: Any, entry: dict[str, Any],
                    window: str) -> tuple[float | None, datetime | None, float | None]:
    """A usage window as (used now, when it reset since the reading, what the reading said).
    Used now is the reading, 0 once the window has reset since it (as the planner counts it), or
    None with no reading; a reading from before the subscription's plan changed counts as none."""
    usage = providers_mod.current_usage(provider, entry)
    reading = usage.get(window) if isinstance(usage, dict) else None
    if not isinstance(reading, dict):
        return None, None, None
    try:
        read = float(reading.get("utilization", 0))
    except (TypeError, ValueError):
        return None, None, None
    resets = parse_iso(reading.get("resets_at"))
    if resets is not None and resets <= ctx.now():
        return 0.0, resets, read
    return read, None, read


def _doing(state: dict[str, Any], number: int) -> tuple[str, str, datetime | None]:
    """What a held lane is doing: its verb (`building`), the item (`#37`, or `survey`), and since
    when."""
    record = status_mod.record_of(state, number)
    if not number:
        return "surveying", "survey", parse_iso(record.get("last_run"))
    kind = "plan" if record.get("action") == "plan" else str(record.get("kind"))
    return (status_mod.DOING.get(kind, "working on"), f"#{number}",
            parse_iso(record.get("started_at")))


def _by_start(ctx: Context, state: dict[str, Any], live: dict[int, str]) -> list[tuple[int, str]]:
    """The live lanes, the longest-running first."""
    def started(pair: tuple[int, str]) -> datetime:
        return _doing(state, pair[0])[2] or ctx.now()
    return sorted(live.items(), key=lambda pair: (started(pair), pair[0]))


# ------------------------------------------------------------------ what is running now

def running_table(ctx: Context, state: dict[str, Any], live: dict[int, str],
                  issues: list[dict[str, Any]]) -> list[str]:
    """What each lane is doing now. The start time links to the run doing it; hovering it shows
    how long the run had been going when the issue was written."""
    if not live:
        return []
    cfg = ctx.cfg
    titles = {int(issue["number"]): issue.get("title") for issue in issues}
    rows = []
    for number, provider_id in _by_start(ctx, state, live):
        record = status_mod.record_of(state, number)
        doing, item, since = _doing(state, number)
        where = "machine" if cfg.pool.on_machine(provider_id) else "GitHub"
        provider = cfg.pool.get(provider_id)
        model = f" `{provider.model}`" if provider else ""
        url = _run_url(ctx, record.get("run_id"))
        if since is not None:
            started = _hover(_clock(ctx, since), _running_for(ctx, since), url)
        else:
            started = f"[run]({url})" if url else "—"
        rows.append(f"| {item if number else '—'} | {_cell(titles.get(number, ''), 50)} "
                    f"| {doing if number else 'suggestion survey'} "
                    f"| `{provider_id}`{model} ({where}) | {started} |")
    return (["| Item | Title | Doing | Subscription | Started (Central) |",
             "|---|---|---|---|---|"] + rows)


def _tick(span: timedelta) -> str:
    """A tick interval that puts a handful of labels on the time axis."""
    for limit, tick in ((timedelta(hours=1), "10minute"), (timedelta(hours=3), "30minute"),
                        (timedelta(hours=8), "1hour")):
        if span <= limit:
            return tick
    return "2hour"


def timeline(ctx: Context, state: dict[str, Any], live: dict[int, str]) -> list[str]:
    """A Mermaid timeline of the runs going now: GitHub's runners, then the machine, the
    longest-running first, each bar named by its subscription, what it does and for how long.
    (A gantt chart reads `#` as a comment, so items are named without it.)"""
    if not live:
        return ["Nothing is running right now."]
    pool = ctx.cfg.pool
    now = _local(ctx, ctx.now())
    fmt = "%Y-%m-%d %H:%M"
    rows: dict[str, list[str]] = {"hosted": [], "night": []}
    earliest = now
    for number, provider_id in _by_start(ctx, state, live):
        doing, item, since = _doing(state, number)
        start = _local(ctx, since) if since else now
        earliest = min(earliest, start)
        end = max(now, start + timedelta(minutes=1))  # a run that just started still shows
        took = human_delta(now - start)
        name = (f"{provider_id} · {doing} {item.lstrip('#')} · "
                f"{'just started' if took == 'now' else took}")
        where = "night" if pool.on_machine(provider_id) else "hosted"
        rows[where].append(
            f"    {name.replace(':', ' ')} :active, {start.strftime(fmt)}, {end.strftime(fmt)}")
    lines = ["```mermaid", "gantt",
             f"    title Runs going now, as of {now.strftime('%H:%M')} (Central time)",
             "    dateFormat YYYY-MM-DD HH:mm", "    axisFormat %H:%M",
             f"    tickInterval {_tick(now - earliest)}", "    todayMarker off"]
    for where, section_name in (("hosted", "GitHub's runners"), ("night", "The night box")):
        if rows[where]:
            lines += [f"    section {section_name}"] + rows[where]
    return lines + ["```"]


#: The boxes' colours: they carry their own text colour, so they read in GitHub's light and dark
#: themes alike.
BOX_STYLES = (
    "    classDef busy fill:#1f883d,stroke:#116329,color:#ffffff",
    "    classDef next fill:#0969da,stroke:#0550ae,color:#ffffff",
    "    classDef idle fill:#ddf4ff,stroke:#54aeff,color:#0a3069",
    "    classDef paused fill:#fff8c5,stroke:#d4a72c,color:#4d2d00",
    "    classDef closed fill:#eaeef2,stroke:#8c959f,color:#24292f",
    "    classDef broken fill:#ffebe9,stroke:#cf222e,color:#82071e",
    "    classDef off fill:#f6f8fa,stroke:#d0d7de,color:#6e7781,stroke-dasharray:4 3",
)
#: Each kind of `waiting.Now` as a box style.
STYLE = {"busy": "busy", "next": "next", "idle": "idle", "reviews": "paused", "full": "paused",
         "capped": "paused", "closed": "closed", "broken": "broken", "off": "off"}
LEGEND = ("🟢 working · ▶️ takes queued work at the next run · ⚪ idle: it may start work, and "
          "nothing queued is for it · ⏸️ at or near a usage cap (when it can start a build or a "
          "plan again), or no lane free where it runs · 🌙 outside its hours · ⛔ backing off "
          "after a run that could not work (a login or the CLI) · ⚫ off or suspended. Usage is "
          "the last reading (5h: the 5-hour session, 7d: the week), against its cap now.")


def _account(ctx: Context, state: dict[str, Any], provider: Any) -> tuple[str, str]:
    """A subscription with no run going, as its box's style and why it is not working, by usage
    and hours alone (`waiting.held_by`): `idle` when it may start a build, a revision or a plan.
    The queue and the lanes say more (`waiting.subscriptions`)."""
    held = waiting_mod.held_by(ctx, state, provider)
    if held is None:
        return "idle", "⚪ idle"
    return STYLE[held.kind], held.text


def _mini_usage(ctx: Context, provider: Any, entry: dict[str, Any]) -> list[str]:
    """A box's usage lines: `5h ▰▰▱▱▱ 42% · cap 40%`, one per window with a reading or a cap."""
    caps = provider.caps_at(ctx.now(), ctx.cfg.timezone)
    lines = []
    for window, short in (("five_hour", "5h"), ("seven_day", "7d")):
        used, reset, _ = _window_reading(ctx, provider, entry, window)
        cap = caps.get(window)
        if used is None and cap is None:
            continue
        cap_text = f" · cap {cap:.0%}" if cap is not None and cap < 1.0 else ""
        if used is None:
            lines.append(f"{short} not read yet{cap_text}")
        elif reset is not None:
            lines.append(f"{short} {bar(0.0, BOX_CELLS)} · reset {_clock(ctx, reset)}{cap_text}")
        else:
            lines.append(f"{short} {bar(used, BOX_CELLS)}{cap_text}")
    budgets = provider.limits.budgets
    for window, budget in budgets.items():
        spent = providers_mod.minutes_spent(entry, ctx.now() - providers_mod.WINDOWS[window])
        lines.append(f"{providers_mod.WINDOW_NAMES[window]} {spent:.0f}/{budget} min")
    return lines or ["usage not read yet"]


def _box(ctx: Context, state: dict[str, Any], provider: Any, now: waiting_mod.Now,
         runs: list[int], name: str) -> str:
    """One subscription's box: its name, what it is doing or why not, and its usage."""
    def run_line(number: int) -> str:
        doing, item, since = _doing(state, number)
        return f"🟢 {doing} {item}" + (f" · since {_clock(ctx, since)}" if since else "")

    lines = [f"<b>{provider.id}</b>"]
    if runs:
        lines += [run_line(n) for n in runs]
        style = "busy"
        queued = [n for n in now.items if n not in runs]
        if queued:
            lines.append("▶️ takes " + ", ".join(f"#{n}" for n in queued) + " next")
    else:
        lines.append(now.text)
        style = STYLE[now.kind]
    lines += _mini_usage(ctx, provider, providers_mod.peek_record(state, provider.id))
    text = "<br/>".join(lines).replace('"', "'")
    return f'        {name}["{text}"]:::{style}'


#: The AI's training lanes on the training box that keep a pull request open from `ai/<lane>`
#: (training/README.md, #1272). None since 2026-10-25: #682's audit stopped `improve` and `unban`
#: (#1677), and `gather` left the box for the owner's PC (gather.yml, training/gather.md), where the
#: runners section draws it, so no training box is drawn. A lane named here again shows its draft.
TRAINING_LANES: tuple[str, ...] = ()
#: A training lane's draft unchanged for longer than a session (`training/loop.sh`'s
#: TRAIN_SESSION_SECONDS, five hours; each session pushes to it as it starts) says the lane stopped.
TRAINING_STALE = timedelta(hours=6)


def training_boxes(ctx: Context) -> list[str]:
    """The training box's lanes as a subgraph, read from their pull requests: a draft is a Muse
    session at work (or an idle unban lane, which its title says), one ready for review is a
    promotion waiting on CI, and none is a lane between sessions or stopped. [] with no lane in
    `TRAINING_LANES`, or when the pull requests cannot be read."""
    if not TRAINING_LANES:
        return []
    try:
        pulls = {str(p["head"]["ref"]): p for p in ctx.gh.list_pulls(state="open")}
    except GitHubError:
        return []
    boxes, working = [], 0
    for i, lane in enumerate(TRAINING_LANES):
        pull = pulls.get(f"ai/{lane}")
        if pull is None:
            text, style = "no pull request open<br/>between sessions, or stopped", "closed"
        else:
            number, updated = pull["number"], parse_iso(str(pull.get("updated_at") or ""))
            since = f"<br/>updated {_clock(ctx, updated)}" if updated else ""
            if not pull.get("draft"):
                text, style = f"▶️ promotion #{number}<br/>waiting on CI{since}", "next"
            elif "): idle," in str(pull.get("title") or ""):
                text, style = f"⏸️ idle on #{number}<br/>the AI on main bans no card", "paused"
            elif updated and ctx.now() - updated > TRAINING_STALE:
                text, style = f"⛔ #{number} unchanged{since}<br/>is the lane running?", "broken"
            else:
                text, style = f"🟢 a Muse session on #{number}{since}", "busy"
                working += 1
        boxes.append(f'        t{i}["<b>{lane}</b><br/>{text}"]:::{style}')
    title = f"The training box, on Muse: {working} of {len(TRAINING_LANES)} lanes working"
    chain = (["        " + " ~~~ ".join(f"t{i}" for i in range(len(TRAINING_LANES)))]
             if len(TRAINING_LANES) > 1 else [])
    return [f'    subgraph training["{title}"]', "        direction LR", *boxes, *chain, "    end"]


def lanes_boxes(ctx: Context, state: dict[str, Any], live: dict[int, str],
                now: dict[str, waiting_mod.Now] | None = None) -> list[str]:
    """Every subscription as a box: GitHub's runners (the Claude accounts), then the night box's
    (each with the runs it has there), each saying what it is doing, or why it is not, and its
    usage."""
    pool = ctx.cfg.pool
    if now is None:
        now = waiting_mod.subscriptions(ctx, state, waiting_mod.lanes_of(ctx, state, live),
                                        waiting_mod.lanes_of(ctx, state, live), {})
    runs: dict[str, list[int]] = {}
    for number, provider_id in _by_start(ctx, state, live):
        runs.setdefault(provider_id, []).append(number)
    # By name (claude-1 to claude-8), so each keeps its place from one rewrite to the next.
    hosted = sorted((p for p in pool.ordered() if not pool.on_machine(p.id)), key=lambda p: p.id)
    machine = sorted((p for p in pool.ordered() if pool.on_machine(p.id)), key=lambda p: p.id)
    busy_hosted = sum(1 for p in hosted if runs.get(p.id))
    on_machine = sum(len(runs.get(p.id, [])) for p in machine)
    claude = all(p.cli == "claude" for p in hosted)
    hosted_title = ("Claude accounts, on GitHub's runners" if claude and hosted
                    else "GitHub's runners")
    if claude and len(hosted) == 1 and not pool.machine_parallel:
        hosted_title = f"{BOT_TITLE}'s Claude account, on GitHub's runners"
    lines = ["```mermaid", "flowchart TB"]
    if hosted:
        lines += [f'    subgraph hosted["{hosted_title}: {busy_hosted} of {len(hosted)} working"]',
                  "        direction LR",
                  *(_box(ctx, state, p, now[p.id], runs.get(p.id, []), f"h{i}")
                    for i, p in enumerate(hosted)),
                  *(["        " + " ~~~ ".join(f"h{i}" for i in range(len(hosted)))]
                    if len(hosted) > 1 else []), "    end"]
    # A pool that never runs on the machine (Squishy's, #60) draws no machine box, and no
    # training box: the AI's training lanes run outside the harness, on a box of their own, so
    # their boxes come from their pull requests (`training_boxes`).
    if machine and pool.machine_parallel:
        lines += [f'    subgraph machine["The night box: {on_machine} of {pool.machine_parallel} '
                  'slots in use"]', "        direction LR",
                  *(_box(ctx, state, p, now[p.id], runs.get(p.id, []), f"m{i}")
                    for i, p in enumerate(machine)),
                  *(["        " + " ~~~ ".join(f"m{i}" for i in range(len(machine)))]
                    if len(machine) > 1 else []), "    end"]
        if hosted:
            lines.append("    hosted ~~~ machine")
        training = training_boxes(ctx)
        lines += training + (["    machine ~~~ training"] if training else [])
    lines += [*BOX_STYLES, "```"]
    planning_runs = {n for n in live
                     if n and status_mod.record_of(state, n).get("action") == "plan"}
    # GitHub's runners have lanes of their own, apart from the night box's slots (`plan.Lanes`).
    building = sum(1 for n, p in live.items() if n not in planning_runs and not pool.on_machine(p))
    plan_note = f"; planning {len(planning_runs)} of {pool.plan_lanes}" if pool.plan_lanes else ""
    return lines + ["", f"<sub>{LEGEND}</sub>", "",
                    f"<sub>Lanes on GitHub's runners: {building} of {pool.max_parallel} in use"
                    f"{plan_note}.</sub>"]


# ------------------------------------------------------------------ the subscriptions

def _now_cell(ctx: Context, state: dict[str, Any], now: waiting_mod.Now,
              runs: list[int]) -> str:
    if runs:
        cell = "🟢 " + ", ".join(f"{_doing(state, n)[0]} #{n}" if n else "survey" for n in runs)
        queued = [n for n in now.items if n not in runs]
        return cell + ("<br/>▶️ takes " + ", ".join(f"#{n}" for n in queued) + " next"
                       if queued else "")
    return now.text


def subscription_table(ctx: Context, state: dict[str, Any], live: dict[int, str],
                       now: dict[str, waiting_mod.Now] | None = None) -> list[str]:
    cfg = ctx.cfg
    if now is None:
        lanes = waiting_mod.lanes_of(ctx, state, live)
        now = waiting_mod.subscriptions(ctx, state, lanes, lanes, {})
    busy: dict[str, list[int]] = {}
    for number, provider_id in live.items():
        busy.setdefault(provider_id, []).append(number)
    lines = ["| Subscription | Now | 5-hour session | 7-day week | Read | Models (tier) | Hours |",
             "|---|---|---|---|---|---|---|"]
    for provider in cfg.pool.ordered():
        entry = providers_mod.peek_record(state, provider.id)
        seats = ", ".join(f"`{seat.model}` {seat.tier}" for seat in cfg.pool.seats(provider))
        doing = _now_cell(ctx, state, now[provider.id], sorted(busy.get(provider.id, [])))
        lines.append(f"| `{provider.id}` | {doing} "
                     f"| {_usage_cell(ctx, provider, entry, 'five_hour')} "
                     f"| {_usage_cell(ctx, provider, entry, 'seven_day')} "
                     f"| {_read_ago(ctx, entry)} | {seats} "
                     f"| {_cell(provider.hours(cfg.timezone))} |")
    return lines


def _usage_cell(ctx: Context, provider: Any, entry: dict[str, Any], window: str) -> str:
    """A usage window's bar, and under it its cap now (and in or outside its hours), the line a
    build or a plan must start under, and when it resets (`status.window_notes`). A window that
    reset since its reading says so and what the reading was, so 0% is never a guess left
    unsaid; one with no reading says that."""
    notes = status_mod.window_notes(ctx, provider, entry, window, lambda at: _clock(ctx, at))
    used, reset, read = _window_reading(ctx, provider, entry, window)
    if used is None:
        cell = "not read yet" if (notes or window == "five_hour") else "—"
        if not notes and window == "seven_day":
            notes = ["not reported"]
    else:
        cell = bar(used)
        if reset is not None:
            notes = [n for n in notes if not n.startswith("resets")]
            notes.append(f"reset {_clock(ctx, reset)}, not read since (it read {read:.0%})")
    return cell + (f"<br/><sub>{' · '.join(notes)}</sub>" if notes else "")


def _read_ago(ctx: Context, entry: dict[str, Any]) -> str:
    """How old a subscription's usage reading is: the bars are only as fresh as that."""
    usage = entry.get("usage") if isinstance(entry.get("usage"), dict) else {}
    observed = parse_iso(usage.get("observed_at"))
    if observed is None:
        return "never"
    age = human_delta(ctx.now() - observed)
    return _hover("just now" if age == "now" else f"{age} ago", f"read at {_clock(ctx, observed)}")


#: The usage charts' colours: the cap behind in grey, the reading over it in green, so a bar
#: that reaches the grey one's top is at its cap.
USAGE_CHART = ('%%{init: {"themeVariables": {"xyChart": {"plotColorPalette": '
               '"#d0d7de, #1f883d"}}}}%%')


def usage_charts(ctx: Context, state: dict[str, Any]) -> list[str]:
    """Each subscription's 5-hour and 7-day usage as bars over its cap now, for every one with a
    reading; the ones with none are named under the chart."""
    pool = ctx.cfg.pool
    lines: list[str] = []
    for window, name in (("five_hour", "5-hour session"), ("seven_day", "7-day week")):
        names, used, caps, unread = [], [], [], []
        for provider in pool.ordered():
            entry = providers_mod.peek_record(state, provider.id)
            value, _, _ = _window_reading(ctx, provider, entry, window)
            cap = provider.caps_at(ctx.now(), ctx.cfg.timezone).get(window)
            if value is None:
                if window == "five_hour" or cap is not None:
                    unread.append(provider.id)
                continue
            names.append(provider.id)
            used.append(round(min(value, 1.0) * 100))
            caps.append(round((cap if cap is not None else 1.0) * 100))
        if not names:
            continue
        lines += ["```mermaid", USAGE_CHART, "xychart-beta",
                  f'    title "{name}: usage now (green) against its cap (grey), %"',
                  f"    x-axis [{', '.join(f'{chr(34)}{n}{chr(34)}' for n in names)}]",
                  '    y-axis "% used" 0 --> 100',
                  f"    bar [{', '.join(str(c) for c in caps)}]",
                  f"    bar [{', '.join(str(u) for u in used)}]", "```"]
        if unread:
            lines.append(f"<sub>No {name} reading yet: "
                         f"{', '.join(f'`{p}`' for p in unread)}.</sub>")
        lines.append("")
    return lines


# ------------------------------------------------------------------ the queue

def difficulty_legend(pool: Any) -> str:
    """What each difficulty may be built by, in one line above the queue."""
    names = {tier: waiting_mod.tier_names(pool, tier) for tier in ("weak", "medium", "strong")}
    return ("<sub>🟢 **easy**: any model, the weak ones first "
            f"({names['weak']}) · 🟡 **medium**, and ⚪ **unrated** until its planner rates it: "
            f"a medium or strong model ({names['medium']}, then {names['strong']}) · 🔴 **hard**: "
            f"a strong model only ({names['strong']}). A build needs a plan first, by a medium "
            "or strong model (strong for a hard one); a review run of a medium or hard change "
            "needs a strong model; a revision that answers a strong audit stays strong.</sub>")


#: What a run does to a queued item, as the queue says it.
STEP = {"plan": "plan", "build": "build", "revise": "revise", "review": "review run"}


def _starts_text(assignment: Any) -> str:
    seat = (assignment.plan if assignment.action == "plan"
            else assignment.review if assignment.action == "review" else assignment.build)
    seat = seat or assignment.plan or assignment.build or assignment.review
    if seat is None:
        return f"▶️ **starts at the next run** on `{assignment.provider.id}`"
    return f"▶️ **starts at the next run** on `{seat.provider.id}` (`{seat.model}`, {seat.tier})"


def queue_table(ctx: Context, explained: waiting_mod.Explained,
                issues: list[dict[str, Any]]) -> list[str]:
    """The queue in the planner's order: what the next run does to each item, its difficulty and
    the tier that needs, and what takes it next or what it waits for."""
    rows = explained.rows
    modes = {int(i["number"]): mode_of(label_names(i)) for i in issues}
    lines = []
    for row in rows[:QUEUE_ROWS]:
        if row.held:
            lines.append(f"| #{row.number} | {_cell(row.title, 55)} | — | — "
                         f"| ⏸️ held back: {row.waiting} | — |")
            continue
        step = STEP.get(row.step, row.step)
        if row.starts is not None and row.starts.action == "build" and row.starts.plan:
            step = "plan, then build"  # no planner free on the planning lane: its builder plans
        mode = modes.get(row.number)
        if mode and row.step == "build":
            step = MODE_KIND.get(mode, step)
        difficulty = (f"{waiting_mod.DIFFICULTY_MARK.get(row.difficulty, '')} {row.difficulty} "
                      f"→ {row.tier}" + (f"<br/><sub>{row.tier_why}</sub>" if row.tier_why else ""))
        what = _starts_text(row.starts) if row.starts is not None else row.waiting
        forced = " (forced)" if row.forced else ""
        lines.append(f"| #{row.number} | {_cell(row.title, 55)} | {step}{forced} | {difficulty} "
                     f"| {what} | {row.priority if row.priority != 'none' else '—'} |")
    if not lines:
        return ["Nothing is queued."]
    more = len(rows) - QUEUE_ROWS
    return (["| Item | Title | Next step | Difficulty → needs | Starts, or waits for | Priority |",
             "|---|---|---|---|---|---|"]
            + lines + ([f"\n…and {more} more."] if more > 0 else []))


#: A queued issue's kind in a bot with modes (#60).
MODE_KIND = {"oneshot": "one-shot (fullsend)", "split": "split", "split-bot": "split for the night bot"}


def waiting_for_people(issues: list[dict[str, Any]]) -> str:
    """The items waiting for a person (`bot:blocked`), in a line, or ""."""
    numbers = [f"#{i['number']}" for i in issues if LABEL_BLOCKED in label_names(i)]
    return f"👤 **Waiting for a person:** {', '.join(numbers)}." if numbers else ""


def headline(ctx: Context, live: dict[int, str], explained: waiting_mod.Explained) -> str:
    """One line: how many runs go, what the queue waits on, and which subscriptions can start
    work now."""
    queued = [r for r in explained.rows if not r.held]
    starting = [r for r in queued if r.starts is not None]
    waiting: dict[str, int] = {}
    for row in queued:
        if row.starts is None:
            key = (f"a {row.tier} planner" if row.step == "plan"
                   else waiting_mod.TIER_WORDS[row.tier])
            waiting[key] = waiting.get(key, 0) + 1
    held = len(explained.rows) - len(queued)
    kinds: dict[str, list[str]] = {}
    for provider_id, now in explained.now.items():
        kinds.setdefault(now.kind, []).append(provider_id)
    parts = [f"🟢 **{len(live)} running**"]
    queue = f"📋 **{len(queued)} queued**"
    details = ([f"{len(starting)} {'starts' if len(starting) == 1 else 'start'} at the next run"]
               if starting else []) + [
        f"{n} {'waits' if n == 1 else 'wait'} for {what}"
        for what, n in sorted(waiting.items(), key=lambda kv: -kv[1])]
    if details:
        queue += ": " + ", ".join(details)
    if held:
        queue += f" ({held} more held back)"
    parts.append(queue)
    ready = kinds.get("idle", []) + kinds.get("next", [])
    parts.append("🔋 " + (f"**{len(ready)} of {len(explained.now)} subscriptions can start work "
                         f"now**: {', '.join(f'`{p}`' for p in ready)}" if ready
                         else f"**none of {len(explained.now)} subscriptions can start work now**"))
    return " · ".join(parts)


# ------------------------------------------------------------------ trees, runs, Squishy

def trees_table(ctx: Context, state: dict[str, Any]) -> list[str]:
    """The trees the bot split (#60): each parent, how many of its sub-issues have closed, and
    who builds them."""
    rows = []
    for issue in ctx.gh.list_issues(labels=LABEL_TREE):
        if "pull_request" in issue:
            continue
        number = int(issue["number"])
        try:
            children = ctx.gh.list_sub_issues(number)
        except GitHubError:
            children = []
        closed = sum(1 for child in children if child.get("state") == "closed")
        record = status_mod.record_of(state, number)
        tree = record.get("tree") if isinstance(record.get("tree"), dict) else {}
        who = "the night bot" if tree.get("mode") == "split-bot" else BOT_TITLE
        done = bar(closed / len(children)) if children else "—"
        rows.append(f"| #{number} | {_cell(issue.get('title'), 50)} | {closed} of {len(children)} "
                    f"closed {done} | {who} |")
    if not rows:
        return ["No open trees."]
    return ["| Parent | Title | Sub-issues | Built by |", "|---|---|---|---|", *rows]


def runs_table(ctx: Context) -> list[str]:
    try:
        runs = ctx.gh.list_runs(NIGHT_WORKFLOW, limit=RUN_ROWS)
    except GitHubError:
        return ["The last runs could not be read."]
    rows = []
    for run in runs[:RUN_ROWS]:
        at = parse_iso(run.get("created_at"))
        result = (RESULT.get(str(run.get("conclusion")), str(run.get("conclusion")))
                  if run.get("status") == "completed" else "🔄 running")
        when = _hover(_local(ctx, at).strftime("%a %H:%M"), _took(ctx, run, at)) if at else "?"
        link = f"[{run.get('run_number') or 'run'}]({run['html_url']})" if run.get("html_url") \
            else str(run.get("id", ""))
        rows.append(f"| {when} | {_cell(run.get('event'), 20)} | {result} | {link} |")
    if not rows:
        return ["No runs yet."]
    return ["| Started (Central) | Trigger | Result | Run |", "|---|---|---|---|"] + rows


def _took(ctx: Context, run: dict[str, Any], created: datetime) -> str:
    """How long a run took, or had been going when this was written."""
    began = parse_iso(run.get("run_started_at")) or created
    if run.get("status") != "completed":
        return _running_for(ctx, began)
    ended = parse_iso(run.get("updated_at"))
    if ended is None:
        return "finished"
    took = human_delta(ended - began)
    return "took under a minute" if took == "now" else f"took {took}"


# ------------------------------------------------------------------ the learn lane

#: The learn lane (#466; training/learn.md, .github/workflows/learn.yml): the learned evaluation,
#: retrained and promoted on the owner's PC (the local runner, bot/machine/README.md) with no model
#: in the loop and no harness either, so its facts come from GitHub alone: the generation on main,
#: the lane's pull request and its runs.
LEARN_WORKFLOW = "learn.yml"
LEARN_BRANCH = "ai/learn"
#: learn.yml's schedule, "23 6 * * *": every day at 06:23 UTC. Each run starts the next as it ends
#: (its `next` job), so the schedule only starts the lane again when that chain stops.
LEARN_SCHEDULE = (6, 23)
#: learn.yml's LEARN_PR_STALE_HOURS: a lane pull request open this long is closed by the next run.
LEARN_PR_STALE = timedelta(hours=48)
LEARN_ROWS = 5


def next_learn_run(now: datetime) -> datetime:
    """When learn.yml's schedule next fires (GitHub fires schedules late; never early)."""
    hour, minute = LEARN_SCHEDULE
    at = now.astimezone(timezone.utc).replace(hour=hour, minute=minute, second=0, microsecond=0)
    return at if at > now else at + timedelta(days=1)


#: A job that ended this way with no runner was never taken: the PC was off, asleep or busy for the
#: 24 hours GitHub lets a job wait for a self-hosted runner.
LEARN_NO_RUNNER = "⏸️ the PC's runner took none of it in 24 hours"


def learn_outcome(run: dict[str, Any], jobs: list[dict[str, Any]]) -> str:
    """How a learn.yml run ended, from its jobs' and steps' conclusions (learn.yml's names)."""
    if run.get("status") != "completed":
        return "🔄 running"
    def named(items: list[dict[str, Any]], prefix: str) -> dict[str, Any] | None:
        return next((i for i in items if str(i.get("name") or "").startswith(prefix)), None)
    def untaken(job: dict[str, Any] | None) -> bool:
        return (job is not None and not job.get("runner_name")
                and job.get("conclusion") in ("failure", "cancelled"))
    retrain, plan = named(jobs, "retrain"), named(jobs, "plan")
    lost = [j for j in jobs if str(j.get("name") or "").startswith("export")
            and j.get("conclusion") not in ("success", "skipped")]
    export_failed = LEARN_NO_RUNNER if all(untaken(j) for j in lost) else "❌ the export failed"
    if untaken(plan) or untaken(retrain):
        return LEARN_NO_RUNNER
    if retrain is None or retrain.get("conclusion") == "skipped":
        if plan is not None and plan.get("conclusion") == "failure":
            return "❌ the plan failed"
        if lost:  # a run from before the retrain went on with the shards that finished
            return export_failed
        if run.get("conclusion") == "cancelled":
            return "⏹️ cancelled"
        return "⏭️ waited on its open pull request"
    steps = retrain.get("steps") or []
    # The retrain goes on with the shards that finished; it stops here only when none did.
    shards = named(steps, "the export's shards")
    if shards is not None and shards.get("conclusion") == "failure":
        return export_failed if lost else "❌ the export failed"
    fit, gate = named(steps, "retrain and screen"), named(steps, "the gate")
    deliver, moving = named(steps, "deliver"), named(steps, "main kept moving")
    waits = named(steps, "a promotion still open")
    if fit is not None and fit.get("conclusion") == "failure":
        return "❌ the retrain broke"
    if gate is None or gate.get("conclusion") == "skipped":
        return "screened out" if retrain.get("conclusion") == "success" else "❌ the retrain broke"
    if gate.get("conclusion") == "failure":
        return "❌ the gate broke"
    if deliver is not None and deliver.get("conclusion") == "success":
        return "🎉 promoted"
    if deliver is not None and deliver.get("conclusion") == "failure":
        return "❌ passed, but could not open its pull request (LEARN_LANE_TOKEN?)"
    if waits is not None and waits.get("conclusion") == "success":
        return "⏸️ passed, behind a promotion still open: kept as its artifact"
    if moving is not None and moving.get("conclusion") == "success":
        return "🔁 main kept moving: another run on the export"
    return "gated out"


def learn_lane(ctx: Context) -> list[str]:
    """The learn lane's section: the AI on main, the lane's pull request, the next scheduled run,
    and its last runs with how each ended. Each part says so when GitHub cannot be read."""
    lines = ["## 🧠 The learn lane", "",
             "<sub>`learn.yml` retrains the learned evaluation on fresh self-play on the owner's "
             "PC, one run after another (the local runner, only while the PC is idle: a run takes "
             "about 40 hours of it), files each export's games into the card statistics, and "
             "promotes the network through the gate, with no model in the loop. It closes a stuck "
             "promotion, and gates again when main moves (training/learn.md, Running "
             "unattended).</sub>", ""]
    try:
        text, _ = ctx.gh.get_file("crates/ai/generation.json", "main")
        record = json.loads(text) if text else {}
        on_main = (f"gen {record.get('generation')} ({record.get('lane')}), promoted "
                   f"{record.get('date')}, {record.get('vsParent')} against its parent"
                   if record else "no generation.json")
    except (GitHubError, ValueError):
        on_main = "could not be read"
    lines.append(f"- **The AI on main:** {on_main}.")
    try:
        pull = next((p for p in ctx.gh.list_pulls(state="open")
                     if str(p["head"]["ref"]) == LEARN_BRANCH), None)
        if pull is None:
            pulled = "none open."
        else:
            opened = parse_iso(str(pull.get("created_at") or ""))
            since = f" open since {_clock(ctx, opened)}" if opened else " open"
            link = f"[#{pull['number']}]({pull.get('html_url', '')})"
            if opened and ctx.now() - opened > LEARN_PR_STALE:
                pulled = f"⛔ {link}{since}: the next run closes it."
            else:
                pulled = f"▶️ {link}{since}: CI decides, and auto-merge lands it."
    except GitHubError:
        pulled = "could not be read."
    lines.append(f"- **Its pull request:** {pulled}")
    upcoming = _local(ctx, next_learn_run(ctx.now())).strftime("%a %b %d %H:%M")
    lines += [f"- **Next scheduled run:** {upcoming} (Central).", ""]
    try:
        runs = [r for r in ctx.gh.list_runs(LEARN_WORKFLOW, limit=20)
                if r.get("event") != "workflow_run"][:LEARN_ROWS]
    except GitHubError:
        return lines + ["Its runs could not be read."]
    rows = []
    for run in runs:
        try:
            jobs = ctx.gh.list_jobs(run["id"]) if run.get("status") == "completed" else []
            result = learn_outcome(run, jobs)
        except (GitHubError, KeyError):
            result = RESULT.get(str(run.get("conclusion")), str(run.get("conclusion")))
        at = parse_iso(run.get("created_at"))
        when = _hover(_local(ctx, at).strftime("%a %H:%M"), _took(ctx, run, at)) if at else "?"
        link = f"[{run.get('run_number') or 'run'}]({run['html_url']})" if run.get("html_url") \
            else str(run.get("id", ""))
        rows.append(f"| {when} | {_cell(run.get('event'), 20)} | {result} | {link} |")
    if not rows:
        return lines + ["No runs yet."]
    return lines + ["| Started (Central) | Trigger | Result | Run |", "|---|---|---|---|", *rows]


# ------------------------------------------------------------------ every runner

#: How often the `bot-status` loop redraws the runners section alone, between its rewrites
#: (`RUNNERS_EVERY_SECONDS` in bot-status.yml, `refresh_runners`).
RUNNERS_MINUTES = 2
RUNNERS_START = "<!-- jackioh-bot:runners -->"
RUNNERS_END = "<!-- /jackioh-bot:runners -->"
#: Marks the comment on the status issue that says what each runner is for (`explain`): it
#: changes only with the pool or the workflows, so it is kept out of the body, which is full.
EXPLAINED = "<!-- jackioh-bot:runners-explained -->"
#: GitHub's runners the organization may hold at once, its every repository's jobs together (the
#: Free plan's 20: selfplay.yml's header, #1455).
HOSTED_RUNNERS = 20
#: The runs going or queued whose jobs the section reads, the oldest first: a call each, which
#: costs no rate limit while the run's jobs are as they were at the last read (`gh._CACHED`).
RUNNER_RUNS = 40
#: How many busy runners the table lists at most; fewer when the issue has no room for them.
RUNNER_ROWS = 25
#: The owner's PC's own label (bot/machine/README.md, The local runner).
LOCAL_LABEL = "jackioh-selfplay-local"
#: What each workflow is for, by file. A workflow added without a line here fails `bot selftest`
#: (test_dashboard.RunnersTests).
WORKFLOWS = {
    "bot-cache.yml": "the night bot's Rust build cache, built from main for its jobs on GitHub's",
    "bot-commands.yml": "the night bot's ears: commands, labels, reviews, CI results; its sweep",
    "bot-night.yml": "the night bot: gate, plan, `work` (one item planned, built, checked, "
                     "reviewed), deliver",
    "bot-status.yml": "this issue's loop: rewrites it, sweeps, plans and starts night runs, "
                      "delivers them",
    "ci-duration.yml": "after CI, an issue for any job over seven minutes",
    "ci.yml": "CI: the five required checks on a pull request; everything on a push to main",
    "claude-wake.yml": "starts each Claude account's 5-hour session at 06:00 Central",
    "deploy-watch.yml": "asks Render to deploy each push to main; an issue if it never arrives",
    "gather.yml": "each new patch's dev games on the owner's PC, filed as totals (GATHER_ON_PATCH)",
    "hygiene-delete.yml": "deletes the branches a person ticked in the hygiene issue",
    "hygiene.yml": "weekly: the hygiene issues for Squishy",
    "learn.yml": "the learn lane: self-play, retrain and gate of the learned evaluation, back to back",
    "lock-conversations.yml": "locks every conversation to collaborators",
    "patches-ship.yml": "ships the pending card-patch fragments after a merge",
    "pr-title.yml": "holds a pull request's title to the convention",
    "promote-production.yml": "merges main into production once a day",
    "selfplay-shards.yml": "one round of a self-play export's shards",
    "selfplay.yml": "a self-play export, on the owner's PC unless asked otherwise",
    "squishy-commands.yml": "Squishy's ears and its sweep",
    "squishy-run.yml": "Squishy: gate, plan, `work` on its own Claude account, deliver",
    "super.yml": "the daily super run: 10,000 fuzz seeds, the full gates, every e2e spec, the db",
    "sweep.yml": "sweeps the shadow bans again when the AI's generation moves",
    "title-retcon.yml": "the title retcon, by hand only",
    "triage.yml": "labels each new issue and pull request from fixed rules",
    "voice.yml": "renders a branch's voice lines (macOS `say`, Windows SAPI), checked on Linux",
    "winrates-refresh.yml": "daily: the Tech decks' win-rate table, once a patch",
}
#: GitHub's runners and the owner's PC, and what each is for, as (label, where, what it runs);
#: the bot's machine's are drawn from the pool (`explanation`).
HOSTED_ROLES = (
    ("ubuntu-latest", "GitHub's Linux: a fresh machine each job, four vCPUs",
     "nearly every job: CI, every bot job that holds a write token (gate, plan, deliver, the "
     "sweeps, this issue's loop), the Claude accounts' and Squishy's model jobs, every other "
     "workflow"),
    ("macos-15", "GitHub's macOS", "`voice`'s macOS `say` lines"),
    ("windows-2022", "GitHub's Windows", "`voice`'s Windows SAPI lines"),
)
LOCAL_ROLE = (LOCAL_LABEL, "the owner's PC: a container a job, `pc-selfplay-<time>`, only while "
              "the PC is otherwise idle", "`learn`, every job of it, back to back, `selfplay` "
              "exports and `gather`")
BOXES_NOTE = ("The night box and the Muse box (a runner name ending `-b2`) are the bot's two AWS "
              "machines, 4 vCPUs and 16 GB each; each starts when one of its jobs queues and stops "
              "after 30 idle minutes. A lane past a subscription's first is `-2`, `-3` …. A "
              "subscription logged in on the owner's PC may take its job there too "
              "(`pc-<id>-<time>`). The PC's supervisor sends a heartbeat every two minutes (its "
              "load without its runners, and each runner's CPUs and whether it is slowed), which "
              "the PC's group in the runners section shows with the gather workflow's state. "
              "bot/machine/README.md has the rest.")


def explanation(ctx: Context) -> str:
    """The comment that says what each runner is for: GitHub's, each subscription's on the bot's
    machine (from the pool, so a subscription added there shows), and the owner's PC; then what
    each workflow is for."""
    pool = ctx.cfg.pool
    rows = [f"| `{label}` | {where} | {what} |" for label, where, what in HOSTED_ROLES]
    for provider in pool.ordered():
        if pool.on_machine(provider.id):
            lanes = f"{provider.lanes} lanes" if provider.lanes > 1 else "1 lane"
            rows.append(f"| `{provider.runs_on}` | the bot's AWS boxes, {lanes} | `bot-night`'s "
                        f"`work` for `{provider.id}`: the `{provider.cli}` CLI with "
                        f"`{provider.model}`, as the Linux user `agent-{provider.id}`, which holds "
                        "that login alone |")
            if provider.fill_in is not None:
                of = ", ".join(f"`{i}`" for i in provider.fill_in.of)
                rows.append(f"| `{provider.fill_in.runs_on}` | the bot's AWS boxes, in the lanes "
                            f"of {of} | `{provider.id}`'s `work` in those lanes while they cannot "
                            "start work (capped, off, backing off), as the same user |")
    rows.append("| `{}` | {} | {} |".format(*LOCAL_ROLE))
    flows = [f"| `{name.removesuffix('.yml')}` | {what} |" for name, what in sorted(WORKFLOWS.items())]
    return "\n".join([
        "## 🖥️ What each runner is for", "",
        "<sub>The status issue's runners section says what each runner is doing now; this says "
        "what each is for. The bot rewrites it when a runner or a workflow changes.</sub>", "",
        "| Runner | Where | What it runs |", "|---|---|---|", *rows, "", f"<sub>{BOXES_NOTE}</sub>",
        "", "### Each workflow", "", "| Workflow | What it is for |", "|---|---|", *flows, "",
        EXPLAINED])


def _explained_url(ctx: Context, state: dict[str, Any]) -> str:
    kept = state.get("runners_explained") or {}
    if not kept.get("issue") or not kept.get("id"):
        return ""
    return (f"{ctx.cfg.server_url}/{ctx.cfg.repo}/issues/{kept['issue']}"
            f"#issuecomment-{kept['id']}")


def explain(ctx: Context, number: int, state: dict[str, Any]) -> None:
    """Keep the status issue's comment on what each runner is for: written once, and again only
    when its text changes (the state file keeps its id and a digest), so a rewrite costs no call.
    `state` takes the record too, so the rewrite in hand links the comment."""
    text = explanation(ctx)
    digest = hashlib.sha256(text.encode("utf-8")).hexdigest()[:16]
    kept = state.get("runners_explained") or {}
    if kept.get("issue") == number and kept.get("digest") == digest and kept.get("id"):
        return
    comment_id = kept.get("id") if kept.get("issue") == number else None
    try:
        if comment_id:
            ctx.gh.update_comment(int(comment_id), text)
    except GitHubError as exc:
        if exc.status != 404:
            raise
        comment_id = None  # deleted by hand: written again
    if not comment_id:
        comment_id = ctx.gh.create_comment(number, text).get("id")
    record = {"issue": number, "id": comment_id, "digest": digest}
    ctx.store.update(lambda s: s.update(runners_explained=record), "runners explained")
    state["runners_explained"] = record


#: The owner's PC's heartbeat: the gist file its supervisor rewrites every `heartbeatSeconds`
#: (bot/machine/local/supervisor.ps1, `Send-Heartbeat`), and how old it may be before the issue
#: says the PC has gone quiet (several missed beats: off, asleep, or the supervisor stopped).
PC_FILE = "pc.json"
PC_QUIET = timedelta(minutes=10)
#: The gather workflow, whose one job runs on the owner's PC (training/gather.md).
GATHER_WORKFLOW = "gather.yml"
#: How many boxes a row of the runners diagram holds, and how many of GitHub's runners it draws at
#: most when the issue is short of room (`runners_section`).
BOX_ROW = 6
BOX_STEPS = (12, 6, 0)
RUNNER_LEGEND = ("🟢 in a job · ⏸️ in a job, slowed while the PC is busy · ⏳ queued for a runner · "
                 "⚪ idle, waiting for a job · 💤 no job: idle, or its box is off · ⚫ off · ❌ its "
                 "last run failed. GitHub's runners exist only while they hold a job.")


@dataclass
class Found:
    """What the runners section draws, read once: each busy job and each queued one with its run,
    how many runs were not read and whether the runs could be read; the gather workflow's last
    runs; and the owner's PC's heartbeat (None, with `pc_note` saying why, when there is none)."""

    busy: list[tuple[dict[str, Any], dict[str, Any]]]
    queued: list[tuple[dict[str, Any], dict[str, Any]]]
    unread: int = 0
    failed: bool = False
    gather: list[dict[str, Any]] = field(default_factory=list)
    pc: dict[str, Any] | None = None
    pc_note: str = ""


def read_runners(ctx: Context) -> Found:
    """The jobs of the runs going and queued, from GitHub, since it lists a repository's own
    runners only to an admin, which the bot is not; the gather workflow's last runs; and the PC's
    heartbeat. Every read is a conditional request (`gh._CACHED`)."""
    try:
        runs = (ctx.gh.list_repo_runs(status="in_progress", cached=True)
                + ctx.gh.list_repo_runs(status="queued", cached=True))
    except GitHubError:
        return Found([], [], failed=True)
    runs.sort(key=lambda run: str(run.get("created_at") or ""))
    found = Found([], [], unread=max(0, len(runs) - RUNNER_RUNS))
    for run in runs[:RUNNER_RUNS]:
        try:
            jobs = ctx.gh.list_jobs(run["id"], cached=True)
        except (GitHubError, KeyError):
            found.unread += 1
            continue
        for job in jobs:
            if job.get("status") == "in_progress":
                found.busy.append((run, job))
            elif job.get("status") == "queued":
                found.queued.append((run, job))
    found.busy.sort(key=lambda pair: (_where(pair[1])[0], str(pair[1].get("started_at") or "")))
    try:
        found.gather = [run for run in ctx.gh.list_runs(GATHER_WORKFLOW, limit=5, cached=True)
                        if _is_gather(run)]
    except GitHubError:
        pass
    found.pc, found.pc_note = _heartbeat(ctx)
    return found


def _heartbeat(ctx: Context) -> tuple[dict[str, Any] | None, str]:
    """The owner's PC's last heartbeat, from the gist the repository variable JACKIOH_PC_GIST
    names (`cfg.pc_gist`), or None and why not."""
    if not ctx.cfg.pc_gist:
        return None, "no heartbeat set up (JACKIOH_PC_GIST)"
    try:
        files = ctx.gh.get_gist(ctx.cfg.pc_gist, cached=True).get("files") or {}
        beat = json.loads(str((files.get(PC_FILE) or {}).get("content") or "{}"))
    except (GitHubError, ValueError):
        return None, "its heartbeat could not be read"
    if not isinstance(beat, dict) or parse_iso(beat.get("at")) is None:
        return None, "no heartbeat yet"
    return beat, ""


def _is_gather(run: dict[str, Any]) -> bool:
    return str(run.get("path") or "").endswith("/" + GATHER_WORKFLOW)


def _where(job: dict[str, Any]) -> tuple[int, str]:
    """Where a job runs, from its runner's name and its label, as (order, words)."""
    name, labels = str(job.get("runner_name") or ""), [str(x) for x in job.get("labels") or []]
    label = labels[0] if labels else ""
    if name.startswith("pc-") or label == LOCAL_LABEL:
        return 3, "owner's PC"
    if name.startswith("night-vm-") or label.startswith("night-vm-"):
        return (2, "Muse box") if memory_mod.box_of(name, label) else (1, "night box")
    for prefix, system in (("ubuntu", "Linux"), ("macos", "macOS"), ("windows", "Windows")):
        if label.startswith(prefix):
            return 0, system
    return (0, "GitHub's") if providers_mod.hosted(label) else (4, "unknown box")


def _step(job: dict[str, Any], limit: int = 45) -> str:
    """The step a running job is on, or the last it finished."""
    steps = job.get("steps") or []
    going = next((s for s in steps if s.get("status") == "in_progress"), None)
    if going is not None:
        return _cell(going.get("name"), limit)
    done = [s for s in steps if s.get("status") == "completed"]
    return f"after {_cell(done[-1].get('name'), limit - 6)}" if done else "setting up"


def _night_items(state: dict[str, Any]) -> dict[str, int]:
    """The night bot's runs its state file names, by run id (or `pending:<ref>`), to the item
    each works on (0: the suggestion survey)."""
    found = {str(record["run_id"]): int(number)
             for number, record in (state.get("items") or {}).items()
             if isinstance(record, dict) and record.get("run_id") and str(number).isdigit()}
    survey = state.get("suggest") or {}
    if survey.get("run_id"):
        found[str(survey["run_id"])] = 0
    return found


def _run_for(state: dict[str, Any], items: dict[str, int], run: dict[str, Any]) -> str:
    """What a run is for: the night bot's item and what it does to it, the pull request it
    checks, or its title."""
    number = items.get(str(run.get("id")))
    title = str(run.get("display_title") or "")
    if number is None:
        number = next((n for ref, n in items.items() if ref.startswith(plan_mod.PENDING)
                       and title.startswith(f"bot-night {ref[len(plan_mod.PENDING):]} ")), None)
    if number == 0:
        return "the suggestion survey"
    if number is not None:
        doing, item, _ = _doing(state, number)
        provider = status_mod.record_of(state, number).get("provider")
        return f"{doing} {item}" + (f" on `{provider}`" if provider else "")
    pulls = run.get("pull_requests") or []
    if pulls:
        return f"#{pulls[0].get('number')} {_cell(title, 36)}"
    return f"{run.get('event')}: {_cell(title, 36)}"


def _job_name(run: dict[str, Any], job: dict[str, Any]) -> str:
    """`ci · rust (test engine)`: the workflow by its file, then the job."""
    flow = str(run.get("path") or "").rsplit("/", 1)[-1].removesuffix(".yml")
    return f"{flow or _cell(run.get('name'), 24)} · {_cell(job.get('name'), 32)}"


def _text(value: Any, limit: int = 34) -> str:
    """Text for a box: one line, clipped, with nothing Mermaid or HTML would read as markup."""
    flat = " ".join(str(value or "").split())
    for old, new in (("`", ""), ('"', "'"), ("<", "‹"), (">", "›")):
        flat = flat.replace(old, new)
    return flat if len(flat) <= limit else flat[: limit - 1] + "…"


#: An invisible box, which pads a short last row of boxes so that it starts at the left: Mermaid
#: lines up rows of unequal length at their right ends.
GHOST_STYLE = "    classDef ghost fill:transparent,stroke:transparent,color:transparent"


def _boxes(sid: str, title: str, boxes: list[tuple[list[str], str]]) -> list[str]:
    """A subgraph of boxes, each (its lines, its style), `BOX_ROW` to a row, each row a chain of
    invisible links; past one row, the last is padded with invisible boxes (`GHOST_STYLE`)."""
    if len(boxes) > BOX_ROW and len(boxes) % BOX_ROW:
        boxes = boxes + [([" "], "ghost")] * (BOX_ROW - len(boxes) % BOX_ROW)
    nodes = [f"{sid}{i}" for i in range(len(boxes))]
    lines = [f'    subgraph {sid}["{_text(title, 120)}"]', "        direction LR"]
    lines += [f'        {node}["{"<br/>".join(t for t in text if t)}"]:::{style}'
              for node, (text, style) in zip(nodes, boxes)]
    lines += ["        " + " ~~~ ".join(nodes[i:i + BOX_ROW])
              for i in range(0, len(nodes), BOX_ROW) if len(nodes[i:i + BOX_ROW]) > 1]
    return lines + ["    end"]


def _fresh(ctx: Context, found: Found) -> dict[str, Any] | None:
    """The PC's heartbeat while it is recent enough to trust."""
    at = parse_iso((found.pc or {}).get("at"))
    return found.pc if at is not None and ctx.now() - at <= PC_QUIET else None


def _pc_title(ctx: Context, found: Found) -> str:
    """The PC's group: how loaded it is, from its heartbeat, and when it was heard."""
    if found.pc is None:
        return f"The owner's PC: {found.pc_note}"
    beat, at = found.pc, parse_iso(found.pc.get("at"))
    if _fresh(ctx, found) is None:
        return (f"The owner's PC: not heard since {_clock(ctx, at)}: off, asleep, or its "
                "supervisor stopped")
    try:
        load = f"CPU {float(beat.get('cpu')):.0f}%, {float(beat.get('memGB')):.1f} GB in use"
    except (TypeError, ValueError):
        load = "its load not reported"
    mood = "calm, lending itself" if beat.get("calm") else "busy with its owner's work"
    return f"The owner's PC: {mood}, {load} · heard {_clock(ctx, at)}"


def _pc_runners(beat: dict[str, Any] | None) -> list[dict[str, Any]]:
    """The heartbeat's runner containers (PowerShell 5.1 may write a list as `{"value": [...]}`)."""
    runners = (beat or {}).get("runners") or []
    if isinstance(runners, dict):
        runners = runners.get("value") or []
    return [r for r in runners if isinstance(r, dict)] if isinstance(runners, list) else []


def _pc_runner(beat: dict[str, Any] | None, name: str) -> dict[str, Any]:
    return next((r for r in _pc_runners(beat) if r.get("name") == name), {})


def _gather_box(ctx: Context, found: Found, beat: dict[str, Any] | None) -> tuple[list[str], str]:
    """The gather workflow on the owner's PC: gathering now (and how hard the PC lets it), waiting
    for the PC's runner, or how its last run ended."""
    title = "<b>gather</b>"
    going = next(((run, job) for run, job in found.busy if _is_gather(run)), None)
    if going is not None:
        _, job = going
        since = parse_iso(job.get("started_at"))
        took = human_delta(ctx.now() - since) if since else "?"
        lines = [title, "🟢 gathering", _text(_step(job)), f"for {took}"]
        runner = _pc_runner(beat, str(job.get("runner_name") or ""))
        if runner.get("throttled"):
            return lines + ["⏸️ slowed: the PC is busy"], "paused"
        if runner.get("cpus") is not None:
            lines.append(f"on {float(runner['cpus']):.1f} CPUs")
        return lines, "busy"
    waiting = next(((run, job) for run, job in found.queued if _is_gather(run)), None)
    if waiting is not None:
        since = parse_iso(waiting[1].get("created_at"))
        return [title, "⏳ waiting for the PC's runner",
                f"for {human_delta(ctx.now() - since)}" if since else ""], "next"
    done = sorted((r for r in found.gather if r.get("status") == "completed"),
                  key=lambda r: str(r.get("created_at") or ""), reverse=True)
    if not done:
        return [title, "no run yet", "it plays each patch", "as it ships"], "off"
    last, at = done[0], parse_iso(done[0].get("created_at"))
    when = _clock(ctx, at) if at else "?"
    if last.get("conclusion") == "skipped":
        return [title, "⚫ off: GATHER_ON_PATCH", f"is not true (skipped {when})"], "off"
    if last.get("conclusion") == "success":
        return [title, "⚪ not gathering", f"last played {when}"], "idle"
    return [title, f"❌ its last run: {_text(last.get('conclusion'), 12)}", when], "broken"


def runner_diagram(ctx: Context, state: dict[str, Any], found: Found, boxes: int) -> list[str]:
    """The runners as boxes, as the subscriptions are drawn: GitHub's runners (at most `boxes`
    of them), the night box's and the Muse box's, the owner's PC with its heartbeat and the gather
    workflow, then the jobs queued for a runner, by label."""
    items = _night_items(state)
    beat = _fresh(ctx, found)

    def busy_box(run: dict[str, Any], job: dict[str, Any], named: bool) -> tuple[list[str], str]:
        since = parse_iso(job.get("started_at"))
        took = human_delta(ctx.now() - since) if since else "?"
        doing = _text(_job_name(run, job))
        lines = ([f"<b>{_text(job.get('runner_name'))}</b>", f"🟢 {doing}"] if named
                 else [f"<b>{doing}</b>"])
        lines += [_text(_run_for(state, items, run)), _text(_step(job)), took]
        runner = _pc_runner(beat, str(job.get("runner_name") or ""))
        if runner.get("throttled"):
            return lines + ["⏸️ slowed: the PC is busy"], "paused"
        return lines, "busy"

    by_place: dict[int, list[tuple[dict[str, Any], dict[str, Any]]]] = {}
    for run, job in found.busy:
        by_place.setdefault(_where(job)[0], []).append((run, job))
    lines = ["```mermaid", "flowchart TB"]
    groups: list[str] = []
    hosted = by_place.get(0, [])
    if hosted:
        shown = [busy_box(run, job, False) for run, job in hosted[:boxes]]
        if len(hosted) > boxes:
            shown.append(([f"<b>… and {len(hosted) - boxes} more</b>", "not drawn: no room"],
                          "closed"))
        lines += _boxes("gh", f"GitHub's runners: {len(hosted)} busy, of the {HOSTED_RUNNERS} "
                        "the organization may hold", shown)
        groups.append("gh")
    pool = ctx.cfg.pool
    held = {str(label) for _, job in found.busy for label in job.get("labels") or []}
    asked: dict[str, int] = {}
    for _, job in found.queued:
        for label in job.get("labels") or []:
            asked[str(label)] = asked.get(str(label), 0) + 1
    if by_place.get(1):
        lines += _boxes("nb", f"The night box (AWS): {len(by_place[1])} busy",
                        [busy_box(run, job, True) for run, job in by_place[1]])
        groups.append("nb")
    if by_place.get(2):
        lines += _boxes("mb", f"The Muse box (AWS): {len(by_place[2])} busy",
                        [busy_box(run, job, True) for run, job in by_place[2]])
        groups.append("mb")
    # The machine's labels no job holds, its fill-in ones (`FillIn`) among them, apart from the
    # boxes: which box a label's runners are on is register-runners.sh's to say, not the pool's.
    labels = [label for p in pool.ordered() if pool.on_machine(p.id)
              for label in (p.runs_on, *((p.fill_in.runs_on,) if p.fill_in else ()))]
    idle = [([f"<b>{_text(label)}</b>", f"⏳ {asked[label]} queued for it",
              "its box is starting, or full"], "next") if asked.get(label)
            else ([f"<b>{_text(label)}</b>", "💤 no job", "idle, or its box is off"], "closed")
            for label in dict.fromkeys(labels) if label not in held]
    if idle:
        lines += _boxes("nj", "The AWS boxes' labels with no job", idle)
        groups.append("nj")
    pc_jobs = by_place.get(3, [])
    pc = [busy_box(run, job, True) for run, job in pc_jobs]
    with_jobs = {str(job.get("runner_name") or "") for _, job in pc_jobs}
    for runner in _pc_runners(beat):
        if str(runner.get("name") or "") in with_jobs:
            continue
        name = f"<b>{_text(runner.get('name'))}</b>"
        pc.append(([name, "🟢 in a job"], "busy") if runner.get("busy")
                  else ([name, "⚪ idle, waiting", f"for a {_text(runner.get('label'), 24)} job"],
                        "idle"))
    if not pc:
        pc.append(([f"<b>{LOCAL_LABEL}</b>", "💤 no job",
                    "the PC is busy" if beat is not None and not beat.get("calm")
                    else "idle, or the PC is off"], "closed"))
    pc.append(_gather_box(ctx, found, beat))
    lines += _boxes("pc", _pc_title(ctx, found), pc)
    groups.append("pc")
    if by_place.get(4):
        lines += _boxes("ob", f"Other runners: {len(by_place[4])} busy",
                        [busy_box(run, job, True) for run, job in by_place[4]])
        groups.append("ob")
    waiting: dict[str, list[tuple[dict[str, Any], dict[str, Any]]]] = {}
    for run, job in found.queued:
        waiting.setdefault(", ".join(str(x) for x in job.get("labels") or []) or "any",
                           []).append((run, job))
    if waiting:
        queue = []
        for label, jobs in sorted(waiting.items(), key=lambda kv: -len(kv[1])):
            run, job = min(jobs, key=lambda pair: str(pair[1].get("created_at") or ""))
            since = parse_iso(job.get("created_at"))
            queue.append(([f"<b>{_text(label)}</b>",
                           f"⏳ {len(jobs)} {'job' if len(jobs) == 1 else 'jobs'}",
                           f"the longest {human_delta(ctx.now() - since)}" if since else "",
                           _text(_job_name(run, job))], "next"))
        lines += _boxes("qu", f"Queued for a runner: {len(found.queued)} "
                        + ("job" if len(found.queued) == 1 else "jobs"), queue)
        groups.append("qu")
    if len(groups) > 1:
        lines.append("    " + " ~~~ ".join(groups))
    ghosts = [GHOST_STYLE] if any(":::ghost" in line for line in lines) else []
    return lines + [*BOX_STYLES, *ghosts, "```"]


def runners_section(ctx: Context, state: dict[str, Any], found: Found | None = None, *,
                    room: int = MAX_BODY) -> str:
    """Every runner and what it is doing, as boxes (`runner_diagram`) and as a table: each job a
    runner holds now, the step it is on, what it is for and how long it has run, linked to the
    job; and a link to the comment on what each runner is for (`explain`). Each part folds
    away. Short of `room`, the table lists fewer runners, then the boxes show fewer of GitHub's."""
    found = found if found is not None else read_runners(ctx)
    read = _clock(ctx, ctx.now())
    url = _explained_url(ctx, state)
    explained = (f"What each runner and workflow is for: [its comment]({url})." if url
                 else "What each runner and workflow is for is in a comment below.")
    head = [RUNNERS_START, "## 🖥️ Every runner", ""]
    if found.failed:
        return "\n".join(head + [f"The runs going could not be read at {read}.", "",
                                 f"<sub>{explained}</sub>", RUNNERS_END])
    busy, queued = found.busy, found.queued
    hosted, places = 0, {}
    for _, job in busy:
        order, place = _where(job)
        if order == 0:
            hosted += 1
        else:
            places[place] = places.get(place, 0) + 1
    counts = [f"{hosted} on GitHub's (of the {HOSTED_RUNNERS} the organization may hold)"]
    counts += [f"{n} on the {place}" for place, n in places.items()]
    head += [f"🟢 **{len(busy)} {'runner' if len(busy) == 1 else 'runners'} busy**: "
             + ", ".join(counts) + f" · ⏳ **{len(queued)} "
             + ("job" if len(queued) == 1 else "jobs") + f" queued** for a runner · read at {read}",
             "", f"<sub>Redrawn every {RUNNERS_MINUTES} minutes from the runs going and queued, "
             f"their jobs and the owner's PC's heartbeat. {explained}</sub>", ""]
    items = _night_items(state)
    rows = []
    for run, job in busy[:RUNNER_ROWS]:
        since = parse_iso(job.get("started_at"))
        took = human_delta(ctx.now() - since) if since else "?"
        link = str(job.get("html_url") or "")
        name = f"`{job['runner_name']}`" if job.get("runner_name") else "—"
        rows.append(f"| {name} ({_where(job)[1]}) | {_job_name(run, job)} | {_step(job)} "
                    f"| {_run_for(state, items, run)} | {f'[{took}]({link})' if link else took} |")
    tail = ([f"<sub>{found.unread} more {'run' if found.unread == 1 else 'runs'} not read.</sub>",
             ""] if found.unread else []) + [RUNNERS_END]

    def drawn(shown: int, boxes: int) -> str:
        diagram = ["<details open><summary><b>The runners, as boxes</b></summary>", "",
                   *runner_diagram(ctx, state, found, boxes), "", f"<sub>{RUNNER_LEGEND}</sub>",
                   "", "</details>", ""]
        if not busy:
            table = ["No runner holds a job right now.", ""]
        else:
            more = len(busy) - shown
            table = ["<details open><summary><b>Every busy runner's job</b>, as a table "
                     f"({len(busy)})</summary>", ""]
            table += (["| Runner | Workflow · job | Step now | For | Running |",
                       "|---|---|---|---|---|", *rows[:shown]] if shown else [])
            table += ([f"\n…and {more} more." if shown else f"{more} not listed: no room."]
                      if more > 0 else []) + ["", "</details>", ""]
        return "\n".join(head + diagram + table + tail)
    hosted_boxes = sum(1 for _, job in busy if _where(job)[0] == 0)
    steps = [(n, hosted_boxes) for n in sorted({len(rows), len(rows) // 2, min(len(rows), 5), 0},
                                                reverse=True)]
    steps += [(0, n) for n in BOX_STEPS if n < hosted_boxes]
    for shown, boxes in steps:
        section = drawn(shown, boxes)
        if len(section) <= room:
            return section
    return section


def refresh_runners(ctx: Context) -> str:
    """Redraw the status issue's runners section alone, between the loop's rewrites: the rest of
    the issue stays as the last rewrite left it, and the table lists what room it leaves."""
    state = ctx.store.load()
    number = (state.get("dashboard") or {}).get("issue")
    if not number:
        return "no status issue yet"
    issue = ctx.gh.get_issue(int(number))
    body = str(issue.get("body") or "")
    start, end = body.find(RUNNERS_START), body.find(RUNNERS_END)
    if issue.get("state") != "open" or start < 0 or end < start:
        return f"#{number} has no runners section to redraw: the next rewrite draws it"
    rest = len(body) - (end + len(RUNNERS_END) - start)
    fresh = (body[:start] + runners_section(ctx, state, room=MAX_BODY - rest)
             + body[end + len(RUNNERS_END):])
    if len(fresh) > MAX_BODY:
        return f"#{number} would pass GitHub's limit: the next rewrite makes room"
    ctx.gh.update_issue(int(number), body=fresh)
    return f"redrew #{number}'s runners"


@dataclass
class View:
    """What both bots' views are drawn from, read from GitHub once per rewrite: the state, the
    open threads, the live lanes, why nothing may start (halted) if so, the queue and the
    subscriptions explained, the last runs and the full status text. `learn` is the learn lane's
    section, and `runners` the runners' drawn from the runs `found`, read once per rewrite by
    `update` (None: `render` reads them)."""

    state: dict[str, Any]
    issues: list[dict[str, Any]]
    live: dict[int, str]
    halted: str
    explained: waiting_mod.Explained
    runs: list[str]
    report: str
    learn: list[str] | None = None
    runners: str | None = None
    found: Found | None = None


def look(ctx: Context) -> View:
    state = ctx.store.load()
    issues = ctx.gh.list_issues(labels="")
    _, live = status_mod.lanes_now(ctx, state)
    halted = plan_mod.stops(ctx, state, False) or ""
    explained = waiting_mod.explain(ctx, state, live, issues=issues, halted=halted)
    return View(state, issues, live, halted, explained, runs_table(ctx), status_mod.report(ctx))


def section(ctx: Context) -> str:
    """This bot's section of the shared status issue (#60): Squishy's, drawn by its own process
    and put into the night bot's issue apart from the night bot's own facts."""
    view = look(ctx)
    state, issues, live, halted, explained = (view.state, view.issues, view.live, view.halted,
                                              view.explained)
    lines = [SECTION_START, f"## 🫧 {BOT_TITLE}", "",
             f"{'🛑 **Halted.** ' if halted else ''}_@{ctx.cfg.bot_login}, on its own Claude "
             f"account and GitHub's runners. `{SLASH} status` gives the same facts on demand._",
             "", headline(ctx, live, explained), "", "### What it is working on", ""]
    # The night bot's two charts, for this bot's runs and lanes: the timeline, then the boxes.
    running = running_table(ctx, state, live, issues)
    lines += (running + [""]) if running else []
    lines += timeline(ctx, state, live) + [""]
    lines += lanes_boxes(ctx, state, live, explained.now) + [""]
    lines += ["### Its queue", ""] + queue_table(ctx, explained, issues) + [""]
    lines += ["### Its account", ""] + subscription_table(ctx, state, live, explained.now) + [""]
    if MODES:
        lines += ["### Its trees", ""] + trees_table(ctx, state) + [""]
    lines += [f"### Last {BOT_TITLE} runs", ""] + view.runs + [""]
    lines += [f"<details><summary>{BOT_TITLE}'s full status</summary>", "",
              view.report, "", "</details>", "", SECTION_END]
    return "\n".join(lines)


SECTION_START = "<!-- jackioh-bot:section -->"
SECTION_END = "<!-- /jackioh-bot:section -->"


# ------------------------------------------------------------------ the issue

def render(ctx: Context, extra: tuple[str, ...] = (), statistics: str = "",
           view: View | None = None) -> str:
    """The whole issue: the headline, the night bot's facts, its statistics, then each other
    bot's section (`extra`), then the full status folded."""
    view = view or look(ctx)
    state, issues, live, halted, explained = (view.state, view.issues, view.live, view.halted,
                                              view.explained)
    updated = _local(ctx, ctx.now()).strftime("%Y-%m-%d %H:%M %Z")
    # The first line, so a glance at the issue says how fresh it is.
    lines = [f"**Last updated at {updated}**", "", f"# 🤖 {TITLE}", ""]
    if halted:
        lines += ["> [!WARNING]", f"> 🛑 **Halted**: {halted}. Nothing starts until it is lifted.",
                  ""]
    lines += [f"> {headline(ctx, live, explained)}", "",
              f"<sub>`bot-status` rewrites this issue every "
              f"{EVERY_MINUTES} minutes, and its runners every {RUNNERS_MINUTES}; the statistics "
              f"are counted every "
              f"{int(stats_mod.STATS_EVERY.total_seconds() // 60)}. Times are Central; hover one "
              f"to see how long it ran. `{SLASH} status` gives the same facts on demand.</sub>", "",
              "## ⚡ Running now", ""]
    running = running_table(ctx, state, live, issues)
    lines += (running + [""] if running else [])
    lines += timeline(ctx, state, live) + [""]
    lines += lanes_boxes(ctx, state, live, explained.now) + [""]
    lines += ["## 📋 Queue", "", difficulty_legend(ctx.cfg.pool), ""]
    lines += queue_table(ctx, explained, issues) + [""]
    people = waiting_for_people(issues)
    lines += [people, ""] if people else []
    lines += ["## 🔋 Subscriptions", ""]
    lines += subscription_table(ctx, state, live, explained.now) + [""]
    lines += usage_charts(ctx, state)
    disk_line = disk_mod.line(state)
    lines += [disk_line, ""] if disk_line else []
    memory_line = memory_mod.line(state, ctx.now())
    lines += [memory_line, ""] if memory_line else []
    lines += ["## 🏃 Last night-bot runs", ""] + view.runs + [""]
    lines += (view.learn if view.learn is not None else learn_lane(ctx)) + [""]
    if statistics:
        lines += [statistics.strip(), ""]
    for part in extra:
        lines += ["---", "", part.strip(), ""]
    # Low in the issue, below everything else the bots draw: it is the longest part, and the
    # least read at a glance.
    lines += ["---", "", view.runners if view.runners is not None else runners_section(ctx, state),
              ""]
    lines += ["<details><summary>The full status</summary>", "",
              view.report, "", "</details>", "", MARKER]
    return "\n".join(lines)


def fit(ctx: Context, view: View, extra: tuple[str, ...], draw: Callable[[int], str]) -> str:
    """The body, listing fewer merged pull requests in the statistics (`draw(shown)`) while it
    would pass GitHub's limit on an issue's body, then fewer busy runners (`_room`), and leaving
    the statistics out last of all."""
    for shown in SHOWN_STEPS:
        body = render(ctx, extra, draw(shown), view)
        if len(body) <= MAX_BODY:
            return body
    body = _room(ctx, view, body, lambda: render(ctx, extra, draw(SHOWN_STEPS[-1]), view))
    return body if len(body) <= MAX_BODY else render(ctx, extra, "", view)


def _room(ctx: Context, view: View, body: str, again: Callable[[], str]) -> str:
    """`body` drawn `again` with the runners section cut to the room the rest of it leaves, when
    it would pass GitHub's limit; as it was when it would not, or has no section."""
    if len(body) <= MAX_BODY or view.runners is None:
        return body
    room = MAX_BODY - (len(body) - len(view.runners))
    view.runners = runners_section(ctx, view.state, view.found, room=room)
    return again()


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


def update(ctx: Context, extra: tuple[str, ...] = (), *, stats: bool = False,
           stats_extra: Callable[[], tuple[str, ...]] = tuple, force_stats: bool = False,
           read_cards: cardsets_mod.Fetch | None = None) -> str:
    """Rewrite the status issue, opening and pinning it first if there is none. `extra` are the
    other bots' sections (`section`), drawn by their own processes.

    With `stats`, the issue carries the statistics (`stats.block`): counted afresh when
    `STATS_EVERY` has passed (or `force_stats`, or the issue has none yet, or the kept block no
    longer fits), with `stats_extra` drawing the other bots' records then; otherwise the block
    the issue holds is kept as it is, so a rewrite costs a few calls, not a count's hundreds.
    A count reads which set's cards win most from production through `read_cards` once a day
    (`cardsets.section`), and keeps the section the issue holds between reads."""
    view = look(ctx)
    view.learn = learn_lane(ctx)
    view.found = read_runners(ctx)
    state = view.state
    issue = find(ctx, state)
    if issue is not None:
        explain(ctx, int(issue["number"]), state)
    view.runners = runners_section(ctx, state, view.found)
    body_now = issue.get("body") if (stats and issue is not None) else None
    kept = stats_mod.kept(body_now)
    note = ""

    def count() -> tuple[str, str]:
        return _counted(ctx, view, extra, stats_extra,
                        cardsets_mod.section(ctx, state, cardsets_mod.kept(body_now), read_cards))
    if stats and (force_stats or kept is None or stats_mod.due(ctx, state)):
        body, note = count()
    elif kept is not None:
        body = render(ctx, extra, kept, view)
        # More runners busy than at the last count: fewer listed, rather than a count again.
        body = _room(ctx, view, body, lambda: render(ctx, extra, kept, view))
        if len(body) > MAX_BODY:
            body, note = count()
    else:
        body = render(ctx, extra, "", view)
    if issue is None:
        issue = ctx.gh.create_issue(TITLE, body, LABELS)
        number = int(issue["number"])
        done = f"opened #{number}"
        try:
            ctx.gh.pin_issue(str(issue.get("node_id") or ""))
            done += " and pinned it"
        except GitHubError as exc:
            done += f"; could not pin it ({exc.status}): pin it by hand"
        explain(ctx, number, state)  # the next rewrite links it
    else:
        number = int(issue["number"])
        ctx.gh.update_issue(number, body=body)
        done = f"rewrote #{number}"
    if (state.get("dashboard") or {}).get("issue") != number:
        ctx.store.update(lambda s: s.update(dashboard={"issue": number}), "dashboard issue")
    return done + note


def _counted(ctx: Context, view: View, extra: tuple[str, ...],
             stats_extra: Callable[[], tuple[str, ...]], cards: str = "") -> tuple[str, str]:
    """The body with the statistics counted afresh, and the note that says so."""
    facts = stats_mod.collect(ctx)
    others = stats_extra()
    body = fit(ctx, view, extra,
               lambda shown: stats_mod.block(ctx, facts, shown=shown, extra=others, cards=cards))
    stats_mod.counted(ctx)
    return body, ", with the statistics counted afresh"
