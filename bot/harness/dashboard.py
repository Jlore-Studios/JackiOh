"""The pinned status issue: one issue, pinned to the top of the issue list, that every sweep (every
ten minutes, `bot-commands.yml`) rewrites with what the night bot is doing.

It shows the same facts as `/harness status` (`status.py`), drawn for a glance: a timeline of the
runs going now, the lanes as boxes (one per Claude account, one per slot on the machine), each
subscription's usage as bars, the queue and the last runs, and the full status text folded
underneath. Times are clock times, which stay true between rewrites; hovering one shows how long
it had been running when the issue was written. Editing an issue's body notifies nobody, so the
rewrites stay quiet. The issue's number is kept in the state file; when it is missing or the
issue was closed, an open issue with the title and the marker, opened by the bot or someone on
the trust list, is adopted, or a new one is opened and pinned.

Squishy (#60) has a section of its own in the same issue, apart from the night bot's: what it is
working on, its account, its queue and its trees. Squishy's own process draws it (`section`), and
the night bot's status loop runs that process each tick and puts the section in (`update`'s
`extra`), so each bot's facts come from its own state and labels.
"""

from __future__ import annotations

import html
from datetime import datetime, timedelta
from typing import Any

from harness import disk as disk_mod
from harness import memory as memory_mod
from harness import plan as plan_mod
from harness import providers as providers_mod
from harness import status as status_mod
from harness.clock import human_delta, parse_iso, zone
from harness.config import (LABEL_BUILD, LABEL_CROSS, LABEL_NEEDS_PLAN, LABEL_PLANNED, LABEL_REVISE,
                            LABEL_TREE, MODES, NIGHT_WORKFLOW, SLASH)
from harness.config import TITLE as BOT_TITLE
from harness.context import Context
from harness.errors import GitHubError
from harness.queue import difficulty_of, label_names, labelled_difficulty, mode_of

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
    training = plan_mod.training_ids(pool)
    rows: dict[str, list[str]] = {"hosted": [], "night": [], "training": []}
    earliest = now
    for number, provider_id in _by_start(ctx, state, live):
        doing, item, since = _doing(state, number)
        start = _local(ctx, since) if since else now
        earliest = min(earliest, start)
        end = max(now, start + timedelta(minutes=1))  # a run that just started still shows
        took = human_delta(now - start)
        name = (f"{provider_id} · {doing} {item.lstrip('#')} · "
                f"{'just started' if took == 'now' else took}")
        where = ("training" if provider_id in training
                 else "night" if pool.on_machine(provider_id) else "hosted")
        rows[where].append(
            f"    {name.replace(':', ' ')} :active, {start.strftime(fmt)}, {end.strftime(fmt)}")
    lines = ["```mermaid", "gantt",
             f"    title Runs going now, as of {now.strftime('%H:%M')} (Central time)",
             "    dateFormat YYYY-MM-DD HH:mm", "    axisFormat %H:%M",
             f"    tickInterval {_tick(now - earliest)}", "    todayMarker off"]
    night_name = "The night box" if training else "The machine"
    for where, section in (("hosted", "GitHub's runners"), ("night", night_name),
                           ("training", "The training box")):
        if rows[where]:
            lines += [f"    section {section}"] + rows[where]
    return lines + ["```"]


#: The boxes' colours: they carry their own text colour, so they read in GitHub's light and dark
#: themes alike.
BOX_STYLES = (
    "    classDef busy fill:#1f883d,stroke:#116329,color:#ffffff",
    "    classDef open fill:#ddf4ff,stroke:#54aeff,color:#0a3069",
    "    classDef paused fill:#fff8c5,stroke:#d4a72c,color:#4d2d00",
    "    classDef closed fill:#eaeef2,stroke:#8c959f,color:#24292f",
    "    classDef broken fill:#ffebe9,stroke:#cf222e,color:#82071e",
    "    classDef off fill:#f6f8fa,stroke:#d0d7de,color:#6e7781,stroke-dasharray:4 3",
    "    classDef free fill:#ffffff,stroke:#d0d7de,color:#6e7781,stroke-dasharray:4 3",
)
LEGEND = ("🟢 working · ⚪ open, waiting for work · ⏸️ at a usage cap · 🌙 outside its hours · "
          "⛔ its last run could not work (a login or the CLI) · ⚫ off or suspended · "
          "▫️ free slot")


def _account(ctx: Context, state: dict[str, Any], provider: Any) -> tuple[str, str]:
    """A subscription with no run going: its box's style and why, in a line or two. The checks go
    in the order `providers.availability` makes them."""
    cfg = ctx.cfg
    now = ctx.now()
    if providers_mod.availability(provider, state, now, cfg.timezone, cfg.secrets) is None:
        return "open", "⚪ open"
    if not provider.enabled or providers_mod.switched_off_by_date(provider, now, cfg.timezone):
        return "off", "⚫ switched off"
    if providers_mod.suspension(state, provider.id) is not None:
        return "off", "⚫ suspended"
    if provider.login == "secret" and cfg.secrets.has(provider.secret) is False:
        return "off", "⚫ off: no secret set"
    window = provider.schedule.window(cfg.timezone)
    if not provider.off_hours and window is not None and not window.is_open(now):
        return "closed", f"🌙 opens {_clock(ctx, window.next_open(now))}"
    entry = providers_mod.peek_record(state, provider.id)
    infra = entry.get("infra") if isinstance(entry.get("infra"), dict) else {}
    failed = parse_iso(infra.get("at"))
    if failed is not None and now - failed < providers_mod.infra_backoff(infra):
        return "broken", ("⛔ last run could not work<br/>tries again "
                          f"{_clock(ctx, failed + providers_mod.infra_backoff(infra))}")
    return "paused", "⏸️ " + _cap_reached(ctx, provider, entry)


def _cap_reached(ctx: Context, provider: Any, entry: dict[str, Any]) -> str:
    """Which limit holds a paused subscription, and until when."""
    now = ctx.now()
    until = parse_iso(entry.get("refused_until"))
    if until is not None and until > now:
        return f"refused a call<br/>until {_clock(ctx, until)}"
    usage = entry.get("usage") if isinstance(entry.get("usage"), dict) else {}
    observed = parse_iso(usage.get("observed_at"))
    for window, cap in provider.caps_at(now, ctx.cfg.timezone).items():
        used = _reading(ctx, entry, window)
        if used is None or used < cap:
            continue
        reading = usage.get(window) if isinstance(usage.get(window), dict) else {}
        resets = parse_iso(reading.get("resets_at")) or (
            observed + providers_mod.WINDOWS[window] if observed else None)
        until_text = f"<br/>resets {_clock(ctx, resets)}" if resets else ""
        return f"{providers_mod.WINDOW_NAMES[window]} {used:.0%}, cap {cap:.0%}{until_text}"
    return "at its limit"


def lanes_boxes(ctx: Context, state: dict[str, Any], live: dict[int, str]) -> list[str]:
    """The lanes as boxes: one per subscription on GitHub's runners (the Claude accounts), green
    with what it runs or saying why it is idle, and one per slot on the machine, with the run in
    it or empty."""
    pool = ctx.cfg.pool
    runs: dict[str, list[int]] = {}
    for number, provider_id in _by_start(ctx, state, live):
        runs.setdefault(provider_id, []).append(number)

    def run_line(number: int) -> str:
        doing, item, since = _doing(state, number)
        return f"{doing} {item}" + (f"<br/>since {_clock(ctx, since)}" if since else "")

    # By name (claude-1 to claude-6), so each account keeps its place from one rewrite to the next.
    hosted = sorted((p for p in pool.ordered() if not pool.on_machine(p.id)), key=lambda p: p.id)
    boxes: list[str] = []
    busy_hosted = 0
    for i, provider in enumerate(hosted):
        if runs.get(provider.id):
            busy_hosted += 1
            text, style = "<br/>".join(f"🟢 {run_line(n)}" for n in runs[provider.id]), "busy"
        else:
            style, text = _account(ctx, state, provider)
        boxes.append(f'        h{i}["<b>{provider.id}</b><br/>{text}"]:::{style}')
    # The night box and the training box apart (#317 part 11): devin-train's slot is on a box of
    # its own, which the night box's subscriptions cannot take.
    training = plan_mod.training_ids(pool)
    machine = [(n, p) for p, numbers in runs.items() if pool.on_machine(p) and p not in training
               for n in numbers]
    machine.sort(key=lambda pair: (_doing(state, pair[0])[2] or ctx.now(), pair[0]))
    night_slots = status_mod.night_slots(pool) if training else pool.machine_parallel
    slots = max(night_slots, len(machine))
    slot_boxes = []
    for i in range(slots):
        if i < len(machine):
            number, provider_id = machine[i]
            slot_boxes.append(f'        m{i}["<b>{provider_id}</b><br/>{run_line(number)}"]:::busy')
        else:
            slot_boxes.append(f'        m{i}["free"]:::free')
    train_boxes = []
    for provider_id in sorted(training):
        provider = pool.get(provider_id)
        held = runs.get(provider_id, [])
        for lane in range(max(provider.lanes, len(held))):
            name = f"t{len(train_boxes)}"
            if lane < len(held):
                train_boxes.append(f'        {name}["<b>{provider_id}</b><br/>'
                                   f'{run_line(held[lane])}"]:::busy')
            else:
                style, text = _account(ctx, state, provider)
                style, text = ("free", "free") if style == "open" else (style, text)
                train_boxes.append(f'        {name}["<b>{provider_id}</b><br/>{text}"]:::{style}')
    claude = all(p.cli == "claude" for p in hosted)
    hosted_title = ("Claude accounts, on GitHub's runners" if claude and hosted
                    else "GitHub's runners")
    if claude and len(hosted) == 1 and not pool.machine_parallel:
        hosted_title = f"{BOT_TITLE}'s Claude account, on GitHub's runners"
    # A pool that never runs on the machine (Squishy's, #60) draws no machine box.
    machine_box = bool(pool.machine_parallel or machine)
    lines = ["```mermaid", "flowchart TB"]
    if hosted:
        lines += [f'    subgraph hosted["{hosted_title}: {busy_hosted} of {len(hosted)} working"]',
                  "        direction LR", *boxes,
                  *(["        " + " ~~~ ".join(f"h{i}" for i in range(len(hosted)))]
                    if len(hosted) > 1 else []), "    end"]
    night_title = "The night box" if training else "The machine"
    if machine_box:
        lines += [f'    subgraph machine["{night_title}: {len(machine)} of {night_slots} '
                  'slots in use"]', "        direction LR", *slot_boxes,
                  "        " + " ~~~ ".join(f"m{i}" for i in range(slots)), "    end"]
    if train_boxes:
        lines += ['    subgraph training["The training box: ladder training items only"]',
                  "        direction LR", *train_boxes]
        if len(train_boxes) > 1:
            lines.append("        " + " ~~~ ".join(f"t{i}" for i in range(len(train_boxes))))
        lines.append("    end")
    if hosted and machine_box:
        lines.append("    hosted ~~~ machine")
    if train_boxes:
        lines.append("    machine ~~~ training")
    lines += [*BOX_STYLES, "```"]
    planning = sum(1 for n in live if n and status_mod.record_of(state, n).get("action") == "plan")
    building = len(live) - planning
    plan_note = f"; planning {planning} of {pool.plan_lanes}" if pool.plan_lanes else ""
    if train_boxes:
        plan_note += (". The training box runs only items labelled `training` "
                      f"({', '.join(f'`{p}`' for p in sorted(training))}), off the night box")
    return lines + ["", f"<sub>{LEGEND}</sub>", "",
                    f"<sub>Lanes: {building} of {pool.max_parallel} in use{plan_note}.</sub>"]


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
                    else MODE_KIND.get(mode_of(names), "") if mode_of(names)
                    else "build (planned)" if LABEL_PLANNED in names else "build")
        elif LABEL_REVISE in names and is_pr:
            kind = "revise"
        elif LABEL_CROSS in names and is_pr:
            kind = "review"
        else:
            continue
        priority = next((name.split(":", 1)[1] for name in sorted(names)
                         if name.lower().startswith("priority:")), "—")
        rated = difficulty_of(names) if (is_pr or labelled_difficulty(names)) else "unrated"
        rows.append(f"| #{issue['number']} | {kind} | {rated} | {priority} "
                    f"| {_cell(issue.get('title'))} |")
    if not rows:
        return ["Nothing is queued."]
    more = len(rows) - QUEUE_ROWS
    return (["| Item | Kind | Difficulty | Priority | Title |", "|---|---|---|---|---|"]
            + rows[:QUEUE_ROWS] + ([f"\n…and {more} more."] if more > 0 else []))


#: A queued issue's kind in a bot with modes (#60).
MODE_KIND = {"oneshot": "one-shot (fullsend)", "split": "split", "split-bot": "split for the night bot"}


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


def section(ctx: Context) -> str:
    """This bot's section of the shared status issue (#60): Squishy's, drawn by its own process
    and put into the night bot's issue apart from the night bot's own facts."""
    state = ctx.store.load()
    issues = ctx.gh.list_issues(labels="")
    _, live = status_mod.lanes_now(ctx, state)
    halted = state.get("halted") or ctx.repo_halted()
    lines = [SECTION_START, f"## 🫧 {BOT_TITLE}", "",
             f"{'🛑 **Halted.** ' if halted else ''}_@{ctx.cfg.bot_login}, on its own Claude "
             f"account and GitHub's runners. `{SLASH} status` gives the same facts on demand._",
             "", "### What it is working on", ""]
    # The night bot's two charts, for this bot's runs and lanes: the timeline, then the boxes.
    running = running_table(ctx, state, live, issues)
    lines += (running + [""]) if running else []
    lines += timeline(ctx, state, live) + [""] + lanes_boxes(ctx, state, live) + [""]
    lines += ["### Its account", ""] + subscription_table(ctx, state, live) + [""]
    lines += ["### Its queue", ""] + queue_table(issues) + [""]
    if MODES:
        lines += ["### Its trees", ""] + trees_table(ctx, state) + [""]
    lines += [f"### Last {BOT_TITLE} runs", ""] + runs_table(ctx) + [""]
    lines += [f"<details><summary>{BOT_TITLE}'s full status</summary>", "",
              status_mod.report(ctx), "", "</details>", "", SECTION_END]
    return "\n".join(lines)


SECTION_START = "<!-- jackioh-bot:section -->"
SECTION_END = "<!-- /jackioh-bot:section -->"


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


def render(ctx: Context, extra: tuple[str, ...] = ()) -> str:
    """The whole issue: the night bot's facts, then each other bot's section (`extra`), then the
    full status folded."""
    state = ctx.store.load()
    issues = ctx.gh.list_issues(labels="")
    held, live = status_mod.lanes_now(ctx, state)
    updated = _local(ctx, ctx.now()).strftime("%Y-%m-%d %H:%M %Z")
    halted = state.get("halted") or ctx.repo_halted()
    lines = [f"# 🤖 {TITLE}", "",
             f"{'🛑 **Halted.** ' if halted else ''}_Updated {updated} by `bot-status`, "
             "which rewrites this issue every ten minutes. Times are Central; hover one to see how "
             "long it ran. `/harness status` gives the same facts on demand._", "",
             "## What it is working on", ""]
    running = running_table(ctx, state, live, issues)
    lines += (running + [""] if running else [])
    lines += timeline(ctx, state, live) + [""] + lanes_boxes(ctx, state, live) + [""]
    lines += ["## Subscriptions", ""] + subscription_table(ctx, state, live) + [""]
    disk_line = disk_mod.line(state)
    lines += [disk_line, ""] if disk_line else []
    memory_line = memory_mod.line(state, ctx.now())
    lines += [memory_line, ""] if memory_line else []
    lines += ["## Queue", ""] + queue_table(issues) + [""]
    lines += ["## Last night-bot runs", ""] + runs_table(ctx) + [""]
    for part in extra:
        lines += ["---", "", part.strip(), ""]
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


def update(ctx: Context, extra: tuple[str, ...] = ()) -> str:
    """Rewrite the status issue, opening and pinning it first if there is none. `extra` are the
    other bots' sections (`section`), drawn by their own processes."""
    body = render(ctx, extra)
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
