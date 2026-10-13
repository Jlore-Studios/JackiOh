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
issue between counts); and the full status text folded underneath. Times are clock times, which
stay true between rewrites; hovering one shows how long it had been running when the issue was
written. Editing an issue's body notifies nobody, so the rewrites stay quiet. The issue's number
is kept in the state file; when it is missing or the issue was closed, an open issue with the
title and the marker, opened by the bot or someone on the trust list, is adopted, or a new one is
opened and pinned.

Squishy (#60) has a section of its own in the same issue, apart from the night bot's: what it is
working on, its account, its queue and its trees. Squishy's own process draws it (`section`), and
the night bot's status loop runs that process each tick and puts the section in (`update`'s
`extra`), so each bot's facts come from its own state and labels. Its record goes in the
statistics, folded, drawn the same way (`stats.bot_section`).
"""

from __future__ import annotations

import html
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import Any, Callable

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
    return f"{ctx.cfg.server_url}/{ctx.cfg.repo}/actions/runs/{run_id}" if run_id else ""


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
    # By name (claude-1 to claude-7), so each keeps its place from one rewrite to the next.
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
    # A pool that never runs on the machine (Squishy's, #60) draws no machine box. (The AI's
    # training lanes run outside the harness, on a box of their own, so they have none here.)
    if machine and pool.machine_parallel:
        lines += [f'    subgraph machine["The night box: {on_machine} of {pool.machine_parallel} '
                  'slots in use"]', "        direction LR",
                  *(_box(ctx, state, p, now[p.id], runs.get(p.id, []), f"m{i}")
                    for i, p in enumerate(machine)),
                  *(["        " + " ~~~ ".join(f"m{i}" for i in range(len(machine)))]
                    if len(machine) > 1 else []), "    end"]
        if hosted:
            lines.append("    hosted ~~~ machine")
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


@dataclass
class View:
    """What both bots' views are drawn from, read from GitHub once per rewrite: the state, the
    open threads, the live lanes, why nothing may start (halted) if so, the queue and the
    subscriptions explained, the last runs and the full status text."""

    state: dict[str, Any]
    issues: list[dict[str, Any]]
    live: dict[int, str]
    halted: str
    explained: waiting_mod.Explained
    runs: list[str]
    report: str


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
    lines = [f"# 🤖 {TITLE}", ""]
    if halted:
        lines += ["> [!WARNING]", f"> 🛑 **Halted**: {halted}. Nothing starts until it is lifted.",
                  ""]
    lines += [f"> {headline(ctx, live, explained)}", "",
              f"<sub>Updated {updated} by `bot-status`, which rewrites this issue every "
              f"{EVERY_MINUTES} minutes; the statistics are counted every "
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
    if statistics:
        lines += [statistics.strip(), ""]
    for part in extra:
        lines += ["---", "", part.strip(), ""]
    lines += ["<details><summary>The full status</summary>", "",
              view.report, "", "</details>", "", MARKER]
    return "\n".join(lines)


def fit(ctx: Context, view: View, extra: tuple[str, ...], draw: Callable[[int], str]) -> str:
    """The body, listing fewer merged pull requests in the statistics (`draw(shown)`) while it
    would pass GitHub's limit on an issue's body, and leaving the statistics out last of all."""
    for shown in SHOWN_STEPS:
        body = render(ctx, extra, draw(shown), view)
        if len(body) <= MAX_BODY:
            return body
    return render(ctx, extra, "", view)


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
           stats_extra: Callable[[], tuple[str, ...]] = tuple, force_stats: bool = False) -> str:
    """Rewrite the status issue, opening and pinning it first if there is none. `extra` are the
    other bots' sections (`section`), drawn by their own processes.

    With `stats`, the issue carries the statistics (`stats.block`): counted afresh when
    `STATS_EVERY` has passed (or `force_stats`, or the issue has none yet, or the kept block no
    longer fits), with `stats_extra` drawing the other bots' records then; otherwise the block
    the issue holds is kept as it is, so a rewrite costs a few calls, not a count's hundreds."""
    view = look(ctx)
    state = view.state
    issue = find(ctx, state)
    kept = stats_mod.kept(issue.get("body")) if (stats and issue is not None) else None
    note = ""
    if stats and (force_stats or kept is None or stats_mod.due(ctx, state)):
        body, note = _counted(ctx, view, extra, stats_extra)
    elif kept is not None:
        body = render(ctx, extra, kept, view)
        if len(body) > MAX_BODY:
            body, note = _counted(ctx, view, extra, stats_extra)
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
    else:
        number = int(issue["number"])
        ctx.gh.update_issue(number, body=body)
        done = f"rewrote #{number}"
    if (state.get("dashboard") or {}).get("issue") != number:
        ctx.store.update(lambda s: s.update(dashboard={"issue": number}), "dashboard issue")
    return done + note


def _counted(ctx: Context, view: View, extra: tuple[str, ...],
             stats_extra: Callable[[], tuple[str, ...]]) -> tuple[str, str]:
    """The body with the statistics counted afresh, and the note that says so."""
    facts = stats_mod.collect(ctx)
    others = stats_extra()
    body = fit(ctx, view, extra,
               lambda shown: stats_mod.block(ctx, facts, shown=shown, extra=others))
    stats_mod.counted(ctx)
    return body, ", with the statistics counted afresh"
