"""The statistics in the pinned status issue (#148): what the night bot has done over the last six
hours, the last day, the last week and all time, counted afresh every 30 minutes by the
`bot-status` loop (`dashboard --stats`) and kept in the issue between counts (`kept`).

Where the rest of the issue says what the bot is doing now, this block keeps the record. It opens
with one table that sets the four windows side by side and charts of the last two weeks, then
folds each window into a section of its own: what each subscription did in it (plans, builds,
revisions, reviews, pull requests opened and merged, lines merged, pauses, failures, model time,
tokens and cost) with bar charts of runs and lines by subscription; then every pull request
merged, newest first, with who planned, built, revised and approved it. The charts are bars and
lines, never pies. Everything is read from GitHub and the state file each count, so it holds
nothing of its own but when it last counted:

- **Runs and outcomes** come from the bot's own comments. A run's starting comment ("Starting work
  on this now …, on `claude-1` (claude, `opus`, strong)") names its subscription and model, and
  every later comment carrying the same run link is joined to it.
- **Who did what to a pull request**: its builder from the "Opened #n, built on …" comment on its
  issue (for one opened before that comment named a builder, the last build started on its issue
  before it was opened); its planner from "Planned on …" on its issue; its revisers from
  "Revision pushed … on …" on it; its approvers from the seats a comment says approved it.
- **Pull requests, merges, commits and lines** come from the bot's pull requests; the closed
  issues from their `Closes #n` lines; commits on the default branch from the commit list.
- **Model minutes** come from each subscription's `spent` record in the state file. Every entry is
  dated, so a window sums the entries in it; all time is its last `providers.SPENT_KEEP` runs.
- **Model tokens and cost** come from each subscription's `tokens_spent` record beside it
  (#695): per run its input, output and cached tokens and its USD cost, summed the same way.

Squishy (#60) has a folded section of its own at the end, apart from the night bot's record: the
same four windows from its own comments, pull requests, runs and state, and its modes (one-shot
builds, splits, the sub-issues they opened, the trees closed). Squishy's own process draws it
(`bot_section`), and the night bot's loop runs that process at each count and puts it in
(`block`'s `extra`).

This used to be an issue of its own, "Night bot statistics" (#190); it is now part of the status
issue, so one pinned issue says what the bot is doing and what it has done.
"""

from __future__ import annotations

import re
import statistics
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from typing import Any, Callable, Mapping

from harness.clock import human_delta, iso, parse_iso, zone
from harness.config import BRANCH_PREFIX, MODES, NIGHT_WORKFLOW, SLASH
from harness.config import TITLE as BOT_TITLE
from harness.context import Context
from harness.errors import GitHubError

#: Where the block starts and ends in the status issue, so a rewrite between counts keeps it.
BLOCK_START = "<!-- jackioh-bot:statistics -->"
BLOCK_END = "<!-- /jackioh-bot:statistics -->"
#: How often the loop counts afresh: often enough that the six-hour window is current. (A count
#: reads every bot comment and pull request, a few hundred calls, so not at every rewrite.)
STATS_EVERY = timedelta(minutes=30)
#: The four windows the issue reports, newest first; None is all time.
WINDOWS: tuple[tuple[str, timedelta | None], ...] = (
    ("Last 6 hours", timedelta(hours=6)),
    ("Last 24 hours", timedelta(hours=24)),
    ("Last 7 days", timedelta(days=7)),
    ("All time", None),
)
#: How many merged pull requests the block lists, newest first; the status issue lists fewer when
#: its body would pass GitHub's limit (`dashboard.fit`).
MERGES_SHOWN = 40
TITLE_SHOWN = 70
DAYS_SHOWN = 14
LIMIT_COMMENTS = 5000
#: A pull request whose builder no comment names.
UNRECORDED = "not recorded"
#: Before the bot's first comment: `list_repo_comments` takes a "since".
EVER = "2000-01-01T00:00:00Z"
LIMIT_PULLS = 500
LIMIT_RUNS = 1000

RUN_LINK = re.compile(r"actions/runs/(\d+)")
#: "`claude-1` (claude, `opus`, strong)", "`gpt` (codex, gpt-5.6-terra)": a subscription, its CLI
#: and the model; the first one a comment names is the run's builder (or planner, or reviewer).
SEAT = re.compile(r"`([a-z][a-z0-9-]*)` \(([a-z]+), `?([A-Za-z0-9][A-Za-z0-9.\-]*)`?")
#: A seat the comment says approved the change: "Its adversarial reviewer, `claude-2` (claude,
#: `opus`, strong), approved it", "`claude-2` (…) reviewed it adversarially and approved it",
#: "Second review (…): `claude-2` (…) approved it".
APPROVED_BY = re.compile(r"`([a-z][a-z0-9-]*)` \([a-z]+, `?[A-Za-z0-9][A-Za-z0-9.\-]*`?"
                         r"(?:, [a-z]+)?\),? (?:reviewed it adversarially and )?approved it")
OPENED = re.compile(r"\bOpened #(\d+)")
#: A split's checklist: "Split #40 into 3 sub-issue(s) (…)".
SPLIT_INTO = re.compile(r"\bSplit #\d+ into (\d+) sub-issue")
CLOSES = re.compile(r"(?i)\b(?:closes|fixes|resolves) #(\d+)")
BRANCH_ISSUE = re.compile(r"^" + re.escape(BRANCH_PREFIX) + r"issue-(\d+)$")

#: What a comment says happened, tried in this order: (kind, words that mark it).
OUTCOMES: tuple[tuple[str, tuple[str, ...]], ...] = (
    ("start", ("Starting work on this now", "Starting a revision now", "Starting a review now",
               "Planning this now", "Starting a one-shot build", "Splitting this now")),
    ("opened", ("Opened #",)),
    ("revised", ("Revision pushed",)),
    ("planned", ("Planned on",)),
    # Squishy's splits (#60): the checklist of a split, and a tree whose close-out found it done.
    ("split", ("sub-issue(s) (",)),
    ("closed tree", ("end state holds on",)),
    ("approved", ("approved it. Auto-merge", "approved it, so")),
    ("paused", ("Paused:", "was cut short", "reached its usage limit", "stops mid-call")),
    ("not approved", ("did not pass review after", "still had blocking findings")),
    ("infra", ("could not work on this", "died on", "left no result")),
    ("failed", ("This run failed", "failed 3 times", "second review failed")),
    ("queued", ("Queued",)),
)
ACTIONS = (("Starting work on this now", "build"), ("Starting a revision now", "revise"),
           ("Starting a review now", "review"), ("Planning this now", "plan"),
           ("Starting a one-shot build", "oneshot"), ("Splitting this now", "split"))
#: The outcomes that count as work done, and the ones that are not.
DONE = ("opened", "revised", "planned", "approved", "split", "closed tree")
NOT_DONE = ("paused", "not approved", "failed", "infra")


@dataclass
class Event:
    at: str
    number: int
    kind: str
    run: str = ""
    provider: str = ""
    model: str = ""
    action: str = ""
    pr: int | None = None
    approvers: tuple[str, ...] = ()
    #: For a split: how many sub-issues it opened.
    count: int = 0


@dataclass
class Facts:
    """Everything the issue shows, gathered once."""

    events: list[Event] = field(default_factory=list)
    pulls: list[dict[str, Any]] = field(default_factory=list)
    commits_on_main: int = 0
    #: `bot-night.yml`'s runs: (when it started, how it ended).
    runs: list[tuple[str, str]] = field(default_factory=list)
    #: Each subscription's dated model minutes: (when, minutes).
    spent: dict[str, list[tuple[str, float]]] = field(default_factory=dict)
    #: Each subscription's dated token counts and cost: (when, input, output, cache, USD).
    counted: dict[str, list[tuple[str, int, int, int, float]]] = field(default_factory=dict)


def classify(body: str) -> str:
    for kind, words in OUTCOMES:
        if any(word in body for word in words):
            return kind
    return "other"


def parse(comment: Mapping[str, Any]) -> Event:
    """One bot comment as an event: what happened, in which run, on which subscription."""
    body = str(comment.get("body") or "")
    number = int(str(comment.get("issue_url") or "0").rstrip("/").rsplit("/", 1)[-1] or 0)
    kind = classify(body)
    run = RUN_LINK.search(body)
    seat = SEAT.search(body)
    opened = OPENED.search(body)
    action = next((name for words, name in ACTIONS if words in body), "")
    approvers = tuple(dict.fromkeys(m.group(1) for m in APPROVED_BY.finditer(body)))
    split = SPLIT_INTO.search(body)
    return Event(at=str(comment.get("created_at") or ""), number=number, kind=kind,
                 run=run.group(1) if run else "", provider=seat.group(1) if seat else "",
                 model=seat.group(3) if seat else "", action=action,
                 pr=int(opened.group(1)) if opened else None, approvers=approvers,
                 count=int(split.group(1)) if split else 0)


def joined(events: list[Event]) -> list[Event]:
    """Each event with its run's subscription and model when it names none itself."""
    seats: dict[str, tuple[str, str]] = {}
    for event in events:
        if event.kind == "start" and event.run and event.provider:
            seats.setdefault(event.run, (event.provider, event.model))
    for event in events:
        if not event.provider and event.run in seats:
            event.provider, event.model = seats[event.run]
    return events


# ------------------------------------------------------------------ gathering

def collect(ctx: Context) -> Facts:
    gh, bot = ctx.gh, ctx.cfg.bot_login.lower()
    facts = Facts()
    comments = [c for c in gh.list_repo_comments(EVER, limit=LIMIT_COMMENTS)
                if str((c.get("user") or {}).get("login") or "").lower() == bot]
    facts.events = joined(sorted((parse(c) for c in comments), key=lambda e: e.at))
    for pull in gh.list_pulls(state="all", limit=LIMIT_PULLS):
        if str((pull.get("user") or {}).get("login") or "").lower() != bot:
            continue
        try:
            detail = gh.get_pull(int(pull["number"]))
        except GitHubError:
            detail = pull
        facts.pulls.append({**pull, **detail})
    try:
        facts.commits_on_main = len(gh.list_commits(author=ctx.cfg.bot_login, limit=LIMIT_RUNS))
    except GitHubError:
        facts.commits_on_main = 0
    for run in gh.list_runs(NIGHT_WORKFLOW, limit=LIMIT_RUNS):
        facts.runs.append((str(run.get("created_at") or ""),
                           str(run.get("conclusion") or run.get("status") or "?")))
    providers = (ctx.store.load().get("providers") or {})
    for provider_id, record in providers.items():
        facts.spent[provider_id] = [(str(entry.get("at") or ""), float(entry.get("minutes") or 0))
                                    for entry in (record or {}).get("spent") or []]
        facts.counted[provider_id] = [
            (str(entry.get("at") or ""), int(entry.get("input") or 0),
             int(entry.get("output") or 0), int(entry.get("cache") or 0),
             float(entry.get("cost") or 0))
            for entry in (record or {}).get("tokens_spent") or []]
    return facts


# ------------------------------------------------------------------ who did what

@dataclass
class Credits:
    """Who planned, built, revised and approved each of the bot's pull requests."""

    built: dict[int, str] = field(default_factory=dict)
    planned: dict[int, list[str]] = field(default_factory=dict)
    revised: dict[int, list[str]] = field(default_factory=dict)
    approved: dict[int, list[str]] = field(default_factory=dict)

    def builder(self, pull: Mapping[str, Any]) -> str:
        return self.built.get(int(pull["number"]), UNRECORDED)


def issue_of(pull: Mapping[str, Any]) -> int | None:
    """The issue a pull request builds: its branch `bot/issue-<n>`, or its first `Closes #n`."""
    branch = BRANCH_ISSUE.match(str((pull.get("head") or {}).get("ref") or ""))
    if branch:
        return int(branch.group(1))
    closes = CLOSES.search(str(pull.get("body") or ""))
    return int(closes.group(1)) if closes else None


def credits(events: list[Event], pulls: list[dict[str, Any]]) -> Credits:
    out = Credits()
    for event in events:
        if event.kind == "opened" and event.pr is not None and event.provider:
            out.built.setdefault(event.pr, event.provider)

    def add(table: dict[int, list[str]], number: int, provider: str) -> None:
        if provider and provider not in table.setdefault(number, []):
            table[number].append(provider)

    for pull in pulls:
        number, issue = int(pull["number"]), issue_of(pull)
        opened_at = str(pull.get("created_at") or "")
        if number not in out.built and issue is not None:
            builds = [e for e in events if e.number == issue and e.kind == "start"
                      and e.action == "build" and e.provider and e.at <= opened_at]
            if builds:
                out.built[number] = builds[-1].provider
        for event in events:
            if event.kind == "planned" and issue is not None and event.number == issue:
                add(out.planned, number, event.provider)
            if event.kind == "revised" and event.number == number:
                add(out.revised, number, event.provider)
            if event.approvers and (event.number == number
                                    or (event.kind == "opened" and event.pr == number)):
                for approver in event.approvers:
                    add(out.approved, number, approver)
    return out


# ------------------------------------------------------------------ one window

@dataclass
class Window:
    """Everything that happened from `start` to now (`start` None: all time)."""

    name: str
    start: datetime | None
    events: list[Event]
    opened: list[dict[str, Any]]
    merged: list[dict[str, Any]]
    runs: list[tuple[str, str]]
    minutes: dict[str, float]
    #: Per subscription: (input tokens, output tokens, cached tokens) and USD cost (#695).
    tokens: dict[str, tuple[int, int, int]]
    cost: dict[str, float]

    @property
    def starts(self) -> list[Event]:
        return [e for e in self.events if e.kind == "start"]

    @property
    def outcomes(self) -> Counter:
        return Counter(e.kind for e in self.events if e.kind in DONE + NOT_DONE)


def _within(at: str | None, start: datetime | None) -> bool:
    """`at` falls in a window from `start`; an undated thing counts only toward all time."""
    if start is None:
        return True
    when = parse_iso(at)
    return when is not None and when >= start


def window(name: str, span: timedelta | None, now: datetime, facts: Facts) -> Window:
    start = None if span is None else now - span
    return Window(
        name=name, start=start,
        events=[e for e in facts.events if _within(e.at, start)],
        opened=[p for p in facts.pulls if _within(p.get("created_at"), start)],
        merged=sorted((p for p in facts.pulls
                       if p.get("merged_at") and _within(p.get("merged_at"), start)),
                      key=lambda p: str(p.get("merged_at")), reverse=True),
        runs=[r for r in facts.runs if _within(r[0], start)],
        minutes={pid: sum(m for at, m in entries if _within(at, start))
                 for pid, entries in facts.spent.items()},
        tokens={pid: (sum(i for at, i, _o, _c, _d in entries if _within(at, start)),
                      sum(o for at, _i, o, _c, _d in entries if _within(at, start)),
                      sum(c for at, _i, _o, c, _d in entries if _within(at, start)))
                for pid, entries in facts.counted.items()},
        cost={pid: sum(d for at, _i, _o, _c, d in entries if _within(at, start))
              for pid, entries in facts.counted.items()},
    )


# ------------------------------------------------------------------ the issue

def _bars(title: str, axis: str, counts: Mapping[str, float], *, zeros: bool = False,
          empty: str = "nothing yet") -> list[str]:
    rows = [(name, value) for name, value in counts.items() if zeros or value > 0]
    if not any(value > 0 for _, value in rows):
        return [f"_{title}: {empty}._"]
    names = ", ".join(f'"{name}"' for name, _ in rows)
    values = ", ".join(str(round(value, 1)) for _, value in rows)
    return ["```mermaid", "xychart-beta", f'    title "{title}"', f"    x-axis [{names}]",
            f'    y-axis "{axis}"', f"    bar [{values}]", "```"]


def _line(title: str, axis: str, days: list[str], values: list[int]) -> list[str]:
    if not any(values):
        return [f"_{title}: nothing yet._"]
    return ["```mermaid", "xychart-beta", f'    title "{title}"',
            f"    x-axis [{', '.join(repr(day) for day in days)}]".replace("'", '"'),
            f'    y-axis "{axis}"', f"    line [{', '.join(str(v) for v in values)}]", "```"]


def _local(ctx: Context, at: datetime) -> datetime:
    return at.astimezone(zone(ctx.cfg.timezone))


def _pct(part: float, whole: float) -> str:
    return f"{100 * part / whole:.0f}%" if whole else "—"


def _lines(pulls: list[dict[str, Any]]) -> str:
    added = sum(int(p.get("additions") or 0) for p in pulls)
    removed = sum(int(p.get("deletions") or 0) for p in pulls)
    return f"+{added:,} / −{removed:,}"


def _total(pulls: list[dict[str, Any]], key: str) -> int:
    return sum(int(p.get(key) or 0) for p in pulls)


def _median_to_merge(merged: list[dict[str, Any]]) -> str:
    took = [parse_iso(p["merged_at"]) - parse_iso(p["created_at"]) for p in merged
            if parse_iso(p.get("merged_at")) and parse_iso(p.get("created_at"))]
    return human_delta(statistics.median(took)) if took else "—"


def _tokens(w: Window) -> str:
    """A window's token counts as `in / out / cached`, compact (#695)."""
    tin = sum(t[0] for t in w.tokens.values())
    tout = sum(t[1] for t in w.tokens.values())
    cached = sum(t[2] for t in w.tokens.values())
    if not tin and not tout and not cached:
        return "—"
    return f"{tin:,} / {tout:,} / {cached:,}"


def _title(pull: Mapping[str, Any]) -> str:
    title = str(pull.get("title") or "").replace("|", "\\|")
    return title if len(title) <= TITLE_SHOWN else title[:TITLE_SHOWN].rstrip() + "…"


def _who(providers: list[str]) -> str:
    return ", ".join(f"`{p}`" for p in providers) or "—"


def summary(windows: list[Window]) -> list[str]:
    """One row per measure, one column per window."""
    rows: list[tuple[str, Callable[[Window], str]]] = [
        ("Runs started", lambda w: str(len(w.starts))),
        ("… plans · builds · revisions · reviews",
         lambda w: " · ".join(str(sum(1 for e in w.starts if e.action == a))
                              for a in ("plan", "build", "revise", "review"))),
        ("Outcomes that delivered something",
         lambda w: f"{sum(w.outcomes[k] for k in DONE)} of {sum(w.outcomes.values())} "
                   f"({_pct(sum(w.outcomes[k] for k in DONE), sum(w.outcomes.values()))})"),
        ("Paused on a usage limit", lambda w: str(w.outcomes["paused"])),
        ("Failed review or failed",
         lambda w: str(w.outcomes["not approved"] + w.outcomes["failed"])),
        ("Could not run", lambda w: str(w.outcomes["infra"])),
        ("Pull requests opened", lambda w: str(len(w.opened))),
        ("Pull requests merged", lambda w: str(len(w.merged))),
        ("Issues closed by them",
         lambda w: str(len({int(n) for p in w.merged
                            for n in CLOSES.findall(str(p.get("body") or ""))}))),
        ("Lines merged (added / removed)", lambda w: _lines(w.merged)),
        ("Files changed in them", lambda w: f"{_total(w.merged, 'changed_files'):,}"),
        ("Commits in them", lambda w: f"{_total(w.merged, 'commits'):,}"),
        ("Median time from opening to merge", lambda w: _median_to_merge(w.merged)),
        ("Model hours", lambda w: f"{sum(w.minutes.values()) / 60:.1f}"),
        ("Model tokens (in / out / cached)", _tokens),
        ("Model cost (USD)", lambda w: f"${sum(w.cost.values()):.2f}"),
        (f"`{NIGHT_WORKFLOW}` workflow runs", lambda w: str(len(w.runs))),
        ("Requests answered (`Queued …`)",
         lambda w: str(sum(1 for e in w.events if e.kind == "queued"))),
    ]
    lines = ["| | " + " | ".join(w.name for w in windows) + " |",
             "|---|" + "---|" * len(windows)]
    lines += [f"| {label} | " + " | ".join(cell(w) for w in windows) + " |" for label, cell in rows]
    return lines


def by_subscription(w: Window, who: Credits) -> dict[str, Counter]:
    counts: dict[str, Counter] = defaultdict(Counter)
    for event in w.events:
        if not event.provider:
            continue
        c = counts[event.provider]
        if event.kind == "start":
            c["start"] += 1
            c[f"start:{event.action}"] += 1
        elif event.kind in DONE + NOT_DONE:
            c[event.kind] += 1
    # Pull requests count under keys of their own: "opened" is also an outcome (a comment).
    for pull in w.opened:
        counts[who.builder(pull)]["pr:opened"] += 1
    for pull in w.merged:
        c = counts[who.builder(pull)]
        c["pr:merged"] += 1
        c["added"] += int(pull.get("additions") or 0)
        c["removed"] += int(pull.get("deletions") or 0)
    return counts


def _order(w: Window, counts: dict[str, Counter]) -> list[str]:
    """The subscriptions with anything in the window: the busiest first, the unrecorded last."""
    return sorted((pid for pid in set(counts) | {p for p, m in w.minutes.items() if m > 0}
                   | {p for p, t in w.tokens.items() if t != (0, 0, 0)}
                   | {p for p, c in w.cost.items() if c > 0}
                   if counts[pid] or w.minutes.get(pid) or w.tokens.get(pid, (0, 0, 0))
                   != (0, 0, 0) or w.cost.get(pid, 0.0) > 0),
                  key=lambda pid: (pid == UNRECORDED, -counts[pid]["start"],
                                   -counts[pid]["pr:merged"], pid))


def section(ctx: Context, w: Window, who: Credits, facts: Facts) -> list[str]:
    """One window, folded: what each subscription did in it, its tokens and cost, and bar charts of
    runs and lines by subscription (all time adds model hours and what runs ended in)."""
    counts = by_subscription(w, who)
    order = _order(w, counts)
    has_tokens = any(tok != (0, 0, 0) or w.cost.get(pid, 0.0) > 0
                     for pid, tok in w.tokens.items())
    active = bool(w.events or w.opened or w.merged)
    if not active and not has_tokens:
        return [f"<details><summary><b>{w.name}</b>: nothing happened</summary>", "",
                "Nothing happened.", "", "</details>", ""]
    runs = sum(counts[pid]["start"] for pid in order)
    summary = (f"<b>{w.name}</b>: {runs} runs, {len(w.merged)} pull requests merged, "
               f"{_lines(w.merged)} lines")
    lines = [f"<details><summary>{summary}</summary>", ""]
    if active:
        lines += ["| Subscription | Models | Runs | Plans | Builds | Revisions | Reviews "
                  "| Outcomes delivered | PRs opened | Merged | Lines merged | Paused | Failed "
                  "| Could not run | Model time |", "|---|" + "---|" * 14]
        models: dict[str, Counter] = defaultdict(Counter)
        for event in w.starts:
            if event.provider:
                models[event.provider][event.model] += 1
        for pid in order:
            c = counts[pid]
            ended = sum(c[k] for k in DONE + NOT_DONE)
            done = sum(c[k] for k in DONE)
            model_list = ", ".join(f"`{m}`" for m, _ in models[pid].most_common(3)) or "—"
            mins = w.minutes.get(pid, 0.0)
            kept = len(facts.spent.get(pid, []))
            time = f"{mins / 60:.1f} h"
            if w.start is None and kept:
                time += f" (its last {kept} runs)"
            name = f"`{pid}`" if pid != UNRECORDED else f"_{UNRECORDED}_"
            lines.append(
                f"| {name} | {model_list} | {c['start']} | {c['start:plan']} | {c['start:build']} "
                f"| {c['start:revise']} | {c['start:review']} | {done} of {ended} "
                f"| {c['pr:opened']} | {c['pr:merged']} | +{c['added']:,} / −{c['removed']:,} "
                f"| {c['paused']} "
                f"| {c['failed'] + c['not approved']} | {c['infra']} | {time} |")
        lines.append("")
    rows = [(pid, w.tokens.get(pid, (0, 0, 0)), w.cost.get(pid, 0.0)) for pid in order]
    rows = [(pid, tok, cost) for pid, tok, cost in rows if tok != (0, 0, 0) or cost > 0]
    lines += ["**Tokens and cost**", ""]
    if not rows:
        lines += ["No call reported token counts in this window.", ""]
    else:
        lines += ["| Subscription | Input tokens | Output tokens | Cached tokens | Cost (USD) |",
                  "|---|---|---|---|---|"]
        for pid, (tin, tout, cached), cost in rows:
            name = f"`{pid}`" if pid != UNRECORDED else f"_{UNRECORDED}_"
            lines.append(f"| {name} | {tin:,} | {tout:,} | {cached:,} | ${cost:.2f} |")
        lines.append("")
    if active:
        lines += _bars(f"Runs started, by subscription ({w.name.lower()})", "runs",
                       {pid: counts[pid]["start"] for pid in order}, empty="none") + [""]
        lines += _bars(f"Lines added in merged pull requests, by builder ({w.name.lower()})",
                       "lines", {pid: counts[pid]["added"] for pid in order}, empty="none") + [""]
        if w.start is None:
            lines += _bars("Model hours, by subscription (all time)", "hours",
                           {pid: w.minutes.get(pid, 0.0) / 60 for pid in order}) + [""]
            outcomes = w.outcomes
            lines += _bars("What runs ended in (all time)", "runs",
                           {kind: outcomes[kind] for kind in DONE + NOT_DONE}, zeros=True) + [""]
    return lines + ["</details>", ""]


def over_time(ctx: Context, w: Window) -> list[str]:
    """The last two weeks day by day: runs started, pull requests merged and lines added."""
    now = ctx.now()
    days = [(_local(ctx, now) - timedelta(days=i)).strftime("%m-%d")
            for i in range(DAYS_SHOWN - 1, -1, -1)]

    def day(at: str) -> str:
        when = parse_iso(at)
        return _local(ctx, when).strftime("%m-%d") if when else ""

    per_day = Counter(day(e.at) for e in w.starts)
    merged_day = Counter(day(p["merged_at"]) for p in w.merged)
    added_day: Counter = Counter()
    for p in w.merged:
        added_day[day(p["merged_at"])] += int(p.get("additions") or 0)
    lines = _line(f"Runs started per day (last {DAYS_SHOWN} days)", "runs", days,
                  [per_day.get(d, 0) for d in days]) + [""]
    lines += _bars(f"Pull requests merged per day (last {DAYS_SHOWN} days)", "merged",
                   {d: merged_day.get(d, 0) for d in days}, zeros=True) + [""]
    lines += _bars(f"Lines added per day in merged pull requests (last {DAYS_SHOWN} days)",
                   "lines", {d: added_day.get(d, 0) for d in days}, zeros=True) + [""]
    return lines


def merged_table(ctx: Context, merged: list[dict[str, Any]], who: Credits, *,
                 shown: int) -> list[str]:
    """Every pull request merged, newest first, with who planned, built, revised and approved it,
    folded."""
    if not merged:
        return ["_No pull request has merged yet._", ""]
    head = (f"<b>Pull requests merged</b>: the newest {min(shown, len(merged))} of "
            f"{len(merged)}, with who planned, built, revised and approved each")
    if not shown:
        return [f"_{len(merged)} pull requests merged; the list is left out to keep this issue "
                "under GitHub's size limit._", ""]
    lines = [f"<details><summary>{head}</summary>", "",
             "| Pull request | Merged | Planned | Built | Revised | Approved | Open for | Lines "
             "| Files |", "|---|---|---|---|---|---|---|---|---|"]
    for p in merged[:shown]:
        number = int(p["number"])
        took = parse_iso(p["merged_at"]) - parse_iso(p["created_at"])
        built = who.builder(p)
        when = _local(ctx, parse_iso(p["merged_at"])).strftime("%a %m-%d %H:%M")
        lines.append(
            f"| #{number} {_title(p)} | {when} | {_who(who.planned.get(number, []))} "
            f"| {f'`{built}`' if built != UNRECORDED else UNRECORDED} "
            f"| {_who(who.revised.get(number, []))} | {_who(who.approved.get(number, []))} "
            f"| {human_delta(took)} | {_lines([p])} | {int(p.get('changed_files') or 0)} |")
    if len(merged) > shown:
        lines.append(f"\n_… and {len(merged) - shown} older ones._")
    return lines + ["", "</details>", ""]


def modes(windows: list[Window]) -> list[str]:
    """Squishy's modes (#60), one row each, one column per window."""
    rows: list[tuple[str, Callable[[Window], str]]] = [
        ("One-shot builds started",
         lambda w: str(sum(1 for e in w.starts if e.action == "oneshot"))),
        ("Splits run", lambda w: str(sum(1 for e in w.starts if e.action == "split"))),
        ("Sub-issues they opened", lambda w: str(sum(e.count for e in w.events
                                                     if e.kind == "split"))),
        ("Trees closed, their end state holding",
         lambda w: str(sum(1 for e in w.events if e.kind == "closed tree"))),
    ]
    lines = ["| | " + " | ".join(w.name for w in windows) + " |",
             "|---|" + "---|" * len(windows)]
    lines += [f"| {label} | " + " | ".join(cell(w) for w in windows) + " |" for label, cell in rows]
    return lines


SECTION_START = "<!-- jackioh-bot:statistics-section -->"
SECTION_END = "<!-- /jackioh-bot:statistics-section -->"
#: How many of its merged pull requests a bot's section lists.
SECTION_MERGES = 10


def bot_section(ctx: Context) -> str:
    """This bot's section of the night bot's statistics (#60): Squishy's record, drawn by its own
    process from its own comments, pull requests, runs and state, and folded in the status issue."""
    facts = collect(ctx)
    now = ctx.now()
    who = credits(facts.events, facts.pulls)
    windows = [window(name, span, now, facts) for name, span in WINDOWS]
    lines = [SECTION_START, f"<details><summary>🫧 <b>{BOT_TITLE}'s record</b>: "
             f"{len(windows[-1].merged)} pull requests merged</summary>", "",
             f"_What @{ctx.cfg.bot_login} has done, read the same way as the night bot's record "
             f"above. What it is doing now is in its section above, and `{SLASH} status`._", ""]
    lines += summary(windows) + [""]
    if MODES:
        lines += ["**Its modes**", ""] + modes(windows) + [""]
    merged = windows[-1].merged
    lines += [f"**Its pull requests merged** ({len(merged)})", ""]
    if not merged:
        lines += ["None yet.", ""]
    else:
        lines += ["| Pull request | Built | Approved | Open for | Lines |", "|---|---|---|---|---|"]
        for p in merged[:SECTION_MERGES]:
            number = int(p["number"])
            took = parse_iso(p["merged_at"]) - parse_iso(p["created_at"])
            built = who.builder(p)
            lines.append(f"| #{number} {_title(p)} "
                         f"| {f'`{built}`' if built != UNRECORDED else UNRECORDED} "
                         f"| {_who(who.approved.get(number, []))} | {human_delta(took)} "
                         f"| {_lines([p])} |")
        if len(merged) > SECTION_MERGES:
            lines.append(f"\n_… and {len(merged) - SECTION_MERGES} older ones._")
        lines.append("")
    return "\n".join(lines + ["</details>", "", SECTION_END])


def block(ctx: Context, facts: Facts, *, shown: int = MERGES_SHOWN,
          extra: tuple[str, ...] = ()) -> str:
    """The statistics block of the status issue, between its markers: the four windows side by
    side, the last two weeks, each window folded, the merged pull requests folded, and the other
    bots' sections (`extra`, `bot_section`) folded at the end."""
    now = ctx.now()
    who = credits(facts.events, facts.pulls)
    windows = [window(name, span, now, facts) for name, span in WINDOWS]
    pulls = facts.pulls
    first = parse_iso(facts.events[0].at) if facts.events else None
    open_now = sum(1 for p in pulls if p.get("state") == "open")
    closed_unmerged = sum(1 for p in pulls if p.get("state") == "closed" and not p.get("merged_at"))
    ended = Counter(conclusion for _, conclusion in facts.runs)
    counted = _local(ctx, now).strftime("%H:%M")
    lines = [BLOCK_START, "## 📊 What it has done", "",
             f"_Counted at {counted}, afresh every {int(STATS_EVERY.total_seconds() // 60)} "
             "minutes. Active since "
             f"{_local(ctx, first).strftime('%Y-%m-%d') if first else '—'}. Now: "
             f"{open_now} of its pull requests open, {closed_unmerged} closed without merging, "
             f"{facts.commits_on_main} commits on `main` by the bot itself. "
             f"`{NIGHT_WORKFLOW}` runs ended: "
             f"{', '.join(f'{k} {v}' for k, v in ended.most_common()) or 'none yet'}._", ""]
    lines += ["### The four windows", ""] + summary(windows) + [""]
    lines += ["### The last two weeks", ""] + over_time(ctx, windows[-1])
    lines += ["### Per window", "",
              "<sub>Runs, outcomes and who did what come from the bot's comments; lines, files "
              "and commits from its merged pull requests; model time, tokens and cost from the "
              "state file's dated record of each subscription's runs. A run counts in the window "
              "it started in, an outcome (a pull request opened, a revision pushed, a plan, an "
              "approval, or a pause, failure or run that could not start) in the window it was "
              "posted in, and a pull request in the window it was opened or merged in.</sub>", ""]
    for w in windows:
        lines += section(ctx, w, who, facts)
    lines += merged_table(ctx, windows[-1].merged, who, shown=shown)
    for part in extra:
        lines += [part.strip(), ""]
    return "\n".join(lines + [BLOCK_END])


def kept(body: str | None) -> str | None:
    """The statistics block a rewrite of the status issue keeps between counts, or None."""
    text = str(body or "")
    start, end = text.find(BLOCK_START), text.find(BLOCK_END)
    if start < 0 or end < start:
        return None
    return text[start:end + len(BLOCK_END)]


def due(ctx: Context, state: Mapping[str, Any]) -> bool:
    last = parse_iso((state.get("statistics") or {}).get("at"))
    return last is None or ctx.now() - last >= STATS_EVERY


def counted(ctx: Context) -> None:
    """Note in the state file that the statistics were counted now."""
    at = iso(ctx.now())
    ctx.store.update(lambda s: s.update(statistics={"at": at}), "statistics")
