"""The pinned "Night bot statistics" issue: everything the night bot has done, rewritten every two
hours by the `bot-status` loop (`dashboard --stats`).

Where the status issue (`dashboard.py`) says what the bot is doing now, this one keeps the record:
which subscriptions and models get work done, how many runs, pull requests, merges, commits and
closed issues, what runs end in, and how much model time each subscription spends, with Mermaid
charts. Everything is read from GitHub and the state file each time, so it holds nothing of its
own but the issue's number and when it was last written:

- **Runs and outcomes** come from the bot's own comments. A run's starting comment ("Starting work
  on this now …, on `claude-1` (claude, `opus`, strong)") names its subscription and model, and
  every later comment carrying the same run link is joined to it.
- **Pull requests, merges, commits and lines** come from the bot's pull requests; the closed
  issues from their `Closes #n` lines; commits on the default branch from the commit list.
- **Model minutes** come from each subscription's `spent` record in the state file (its last
  `providers.SPENT_KEEP` runs), so the chart says over how many runs.
"""

from __future__ import annotations

import re
import statistics
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from typing import Any, Mapping

from harness.clock import human_delta, iso, parse_iso, zone
from harness.config import NIGHT_WORKFLOW
from harness.context import Context
from harness.errors import GitHubError

TITLE = "Night bot statistics"
MARKER = "<!-- jackioh-bot:statistics -->"
LABELS = ("night bot",)
#: How often the loop rewrites the issue.
STATS_EVERY = timedelta(hours=2)
#: How much of each list the issue shows.
RECENT_MERGES = 12
DAYS_SHOWN = 14
LIMIT_COMMENTS = 5000
#: A pull request opened before the bot's comments named its builder.
UNRECORDED = "not recorded"
#: Before the bot's first comment: `list_repo_comments` takes a "since".
EVER = "2000-01-01T00:00:00Z"
LIMIT_PULLS = 500
LIMIT_RUNS = 1000

RUN_LINK = re.compile(r"actions/runs/(\d+)")
#: "`claude-1` (claude, `opus`, strong)", "`gpt` (codex, gpt-5.6-terra)": a subscription, its CLI
#: and the model; the first one a comment names is the run's builder (or planner, or reviewer).
SEAT = re.compile(r"`([a-z][a-z0-9-]*)` \(([a-z]+), `?([A-Za-z0-9][A-Za-z0-9.\-]*)`?")
OPENED = re.compile(r"\bOpened #(\d+)")
CLOSES = re.compile(r"(?i)\b(?:closes|fixes|resolves) #(\d+)")

#: What a comment says happened, tried in this order: (kind, words that mark it).
OUTCOMES: tuple[tuple[str, tuple[str, ...]], ...] = (
    ("start", ("Starting work on this now", "Starting a revision now", "Starting a review now",
               "Planning this now")),
    ("opened", ("Opened #",)),
    ("revised", ("Revision pushed",)),
    ("planned", ("Planned on",)),
    ("approved", ("approved it. Auto-merge", "approved it, so")),
    ("paused", ("Paused:", "was cut short", "reached its usage limit", "stops mid-call")),
    ("not approved", ("did not pass review after", "still had blocking findings")),
    ("infra", ("could not work on this", "died on", "left no result")),
    ("failed", ("This run failed", "failed 3 times", "second review failed")),
    ("queued", ("Queued",)),
)
ACTIONS = (("Starting work on this now", "build"), ("Starting a revision now", "revise"),
           ("Starting a review now", "review"), ("Planning this now", "plan"))
#: The outcomes the charts count as work done, and the ones that are not.
DONE = ("opened", "revised", "planned", "approved")


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


@dataclass
class Facts:
    """Everything the issue shows, gathered once."""

    events: list[Event] = field(default_factory=list)
    pulls: list[dict[str, Any]] = field(default_factory=list)
    commits_on_main: int = 0
    runs: Counter = field(default_factory=Counter)
    minutes: dict[str, tuple[float, int]] = field(default_factory=dict)


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
    return Event(at=str(comment.get("created_at") or ""), number=number, kind=kind,
                 run=run.group(1) if run else "", provider=seat.group(1) if seat else "",
                 model=seat.group(3) if seat else "", action=action,
                 pr=int(opened.group(1)) if opened else None)


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
        facts.runs[str(run.get("conclusion") or run.get("status") or "?")] += 1
    providers = (ctx.store.load().get("providers") or {})
    for provider_id, record in providers.items():
        spent = (record or {}).get("spent") or []
        total = sum(float(entry.get("minutes") or 0) for entry in spent)
        facts.minutes[provider_id] = (total, len(spent))
    return facts


# ------------------------------------------------------------------ the issue

def _pie(title: str, counts: Mapping[str, float]) -> list[str]:
    rows = [(name, value) for name, value in counts.items() if value > 0]
    if not rows:
        return [f"_{title}: nothing yet._"]
    return (["```mermaid", f"pie showData title {title}"]
            + [f'    "{name}" : {round(value, 1)}' for name, value in rows] + ["```"])


def _bars(title: str, axis: str, counts: Mapping[str, float], *, zeros: bool = False) -> list[str]:
    rows = [(name, value) for name, value in counts.items() if zeros or value > 0]
    if not any(value > 0 for _, value in rows):
        return [f"_{title}: nothing yet._"]
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


def render(ctx: Context, facts: Facts) -> str:
    now = ctx.now()
    events = facts.events
    starts = [e for e in events if e.kind == "start"]
    builder_of: dict[int, str] = {}
    for event in events:
        if event.kind == "opened" and event.pr is not None and event.provider:
            builder_of.setdefault(event.pr, event.provider)
    pulls = facts.pulls
    merged = [p for p in pulls if p.get("merged_at")]
    open_now = [p for p in pulls if p.get("state") == "open"]
    closed_unmerged = [p for p in pulls if p.get("state") == "closed" and not p.get("merged_at")]
    closed_issues = {int(n) for p in merged for n in CLOSES.findall(str(p.get("body") or ""))}
    branch_commits = sum(int(p.get("commits") or 0) for p in pulls)
    added = sum(int(p.get("additions") or 0) for p in merged)
    removed = sum(int(p.get("deletions") or 0) for p in merged)
    to_merge = [parse_iso(p["merged_at"]) - parse_iso(p["created_at"]) for p in merged
                if parse_iso(p.get("merged_at")) and parse_iso(p.get("created_at"))]
    median = human_delta(statistics.median(to_merge)) if to_merge else "—"
    first = parse_iso(events[0].at) if events else None
    nights = sum(facts.runs.values())

    by_provider: dict[str, Counter] = defaultdict(Counter)
    models: dict[str, Counter] = defaultdict(Counter)
    for event in events:
        if event.provider:
            by_provider[event.provider][event.kind] += 1
            if event.kind == "start":
                by_provider[event.provider][f"start:{event.action}"] += 1
                models[event.provider][event.model] += 1
    opened_by = Counter(builder_of.get(int(p["number"]), UNRECORDED) for p in pulls)
    merged_by = Counter(builder_of.get(int(p["number"]), UNRECORDED) for p in merged)
    order = sorted(set(by_provider) | set(facts.minutes),
                   key=lambda pid: (-by_provider[pid]["start"], pid))

    lines = [f"# 📊 {TITLE}", "",
             f"_Updated {_local(ctx, now).strftime('%Y-%m-%d %H:%M %Z')} by `bot-status`, every two "
             "hours. What the bot is doing right now is in the pinned **Night bot status** issue._",
             "", "## At a glance", "",
             "| | |", "|---|---|",
             f"| Active since | {_local(ctx, first).strftime('%Y-%m-%d') if first else '—'} |",
             f"| Runs started (plan, build, revise, review) | {len(starts)} |",
             f"| `{NIGHT_WORKFLOW}` workflow runs | {nights} "
             f"({', '.join(f'{k} {v}' for k, v in facts.runs.most_common())}) |",
             f"| Pull requests opened | {len(pulls)} |",
             f"| Merged | {len(merged)} ({_pct(len(merged), len(pulls))}) |",
             f"| Open now | {len(open_now)} |",
             f"| Closed without merging | {len(closed_unmerged)} |",
             f"| Issues closed by its pull requests | {len(closed_issues)} |",
             f"| Commits on its branches | {branch_commits} |",
             f"| Commits on `main` by the bot | {facts.commits_on_main} |",
             f"| Lines merged | +{added} / −{removed} |",
             f"| Median time from pull request to merge | {median} |",
             f"| Requests answered (`Queued …`) | {sum(1 for e in events if e.kind == 'queued')} |",
             ""]

    lines += ["## Who gets work done", ""]
    lines += ["| Subscription | Models | Runs | Plans | Builds | Revisions | Reviews | PRs opened "
              "| Merged | Paused (usage) | Failed | Could not run | Model time |",
              "|---|---|---|---|---|---|---|---|---|---|---|---|---|"]
    for pid in order:
        c = by_provider[pid]
        mins, runs = facts.minutes.get(pid, (0.0, 0))
        model_list = ", ".join(f"`{m}`" for m, _ in models[pid].most_common(3)) or "—"
        lines.append(
            f"| `{pid}` | {model_list} | {c['start']} | {c['start:plan']} | {c['start:build']} "
            f"| {c['start:revise']} | {c['start:review']} | {opened_by.get(pid, 0)} "
            f"| {merged_by.get(pid, 0)} | {c['paused']} | {c['failed'] + c['not approved']} "
            f"| {c['infra']} | {mins / 60:.1f} h over its last {runs} runs |")
    lines += ["", "Runs, outcomes and who built each pull request come from the bot's comments; "
              "model time from the state file's record of each subscription's last runs.", ""]

    lines += _pie("Pull requests opened, by builder", opened_by) + [""]
    lines += _pie("Pull requests merged, by builder", merged_by) + [""]
    lines += _bars("Runs started, by subscription", "runs",
                   {pid: by_provider[pid]["start"] for pid in order}) + [""]
    lines += _bars("Model hours, by subscription", "hours",
                   {pid: facts.minutes.get(pid, (0.0, 0))[0] / 60 for pid in order}) + [""]
    outcomes = Counter(e.kind for e in events if e.kind not in ("start", "queued", "other"))
    lines += ["## What runs end in", ""] + _pie("Run outcomes", outcomes) + [""]
    done = sum(outcomes[k] for k in DONE)
    lines += [f"{done} of {sum(outcomes.values())} outcomes delivered something "
              f"({_pct(done, sum(outcomes.values()))}): a pull request, a revision, a plan or an "
              "approval. The rest paused on a usage limit, failed review, failed outright, or "
              "could not run.", ""]

    days = [(_local(ctx, now) - timedelta(days=i)).strftime("%m-%d")
            for i in range(DAYS_SHOWN - 1, -1, -1)]
    per_day = Counter(_local(ctx, parse_iso(e.at)).strftime("%m-%d") for e in starts
                      if parse_iso(e.at))
    merged_day = Counter(_local(ctx, parse_iso(p["merged_at"])).strftime("%m-%d") for p in merged)
    lines += ["## Over time", ""]
    lines += _line(f"Runs started per day (last {DAYS_SHOWN} days)", "runs", days,
                   [per_day.get(d, 0) for d in days]) + [""]
    lines += _bars(f"Pull requests merged per day (last {DAYS_SHOWN} days)", "merged",
                   {d: merged_day.get(d, 0) for d in days}, zeros=True) + [""]

    recent = sorted(merged, key=lambda p: str(p.get("merged_at")), reverse=True)[:RECENT_MERGES]
    lines += ["## Latest merges", ""]
    if recent:
        lines += ["| Pull request | Built on | Open for | Lines |", "|---|---|---|---|"]
        for p in recent:
            took = parse_iso(p["merged_at"]) - parse_iso(p["created_at"])
            title = str(p.get("title") or "").replace("|", "\\|")[:70]
            built = builder_of.get(int(p["number"]))
            lines.append(f"| #{p['number']} {title} | {f'`{built}`' if built else UNRECORDED} "
                         f"| {human_delta(took)} | +{p.get('additions', 0)} / −{p.get('deletions', 0)} |")
    else:
        lines.append("Nothing merged yet.")
    lines += ["", MARKER]
    return "\n".join(lines)


def find(ctx: Context, state: Mapping[str, Any]) -> dict[str, Any] | None:
    number = (state.get("statistics") or {}).get("issue")
    if number:
        try:
            issue = ctx.gh.get_issue(int(number))
            if issue.get("state") == "open" and "pull_request" not in issue:
                return issue
        except GitHubError:
            pass
    for issue in ctx.gh.list_issues(labels=""):
        if ("pull_request" not in issue and issue.get("title") == TITLE
                and MARKER in str(issue.get("body") or "")):
            return issue
    return None


def due(ctx: Context, state: Mapping[str, Any]) -> bool:
    last = parse_iso((state.get("statistics") or {}).get("at"))
    return last is None or ctx.now() - last >= STATS_EVERY


def update(ctx: Context, *, force: bool = False) -> str:
    """Rewrite the statistics issue when two hours have passed (or `force`), opening and pinning
    it first if there is none."""
    state = ctx.store.load()
    if not force and not due(ctx, state):
        return "statistics: not due"
    body = render(ctx, collect(ctx))
    issue = find(ctx, state)
    if issue is None:
        issue = ctx.gh.create_issue(TITLE, body, LABELS)
        number = int(issue["number"])
        note = f"statistics: opened #{number}"
        try:
            ctx.gh.pin_issue(str(issue.get("node_id") or ""))
            note += " and pinned it"
        except GitHubError as exc:
            note += f"; could not pin it ({exc.status}): pin it by hand"
    else:
        number = int(issue["number"])
        ctx.gh.update_issue(number, body=body)
        note = f"statistics: rewrote #{number}"
    at = iso(ctx.now())
    ctx.store.update(lambda s: s.update(statistics={"issue": number, "at": at}), "statistics")
    return note
