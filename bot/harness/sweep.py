"""The sweep: find every request the event handler never answered, and answer it now.

Events are delivered once, and a handler can fail before it acts (GitHub is down, a write
races, a bug). The labels are the queue and survive all of that, but a comment, a review, an
assignment or a failed CI run lives only in its event. So every ten minutes, and on demand, this
reads recent history back from GitHub itself and answers, through the same code the event would
have used:

- a trusted comment, diff comment or review that carries a command or names the bot, and that
  nobody has claimed (`events.claim`);
- a trusted review asking a bot pull request for changes, likewise unclaimed;
- an open issue or PR assigned to the bot that nothing ever queued;
- a failed CI run on the head of a bot PR that nothing reran or queued a fix for;
- last, a night run that should be going and is not: GitHub drops scheduled runs, sometimes a
  whole night of `bot-night`'s hourly ones, so when the gate's own question (`plan.peek`) finds
  work for a free lane and no `bot-night` run is on its way to take it, the sweep starts one.

It looks back to the later of `LOOKBACK` and the first sweep ever (so the first one replays
nothing), leaves anything younger than `SETTLE` to a handler that may still be running, and
skips an edited comment, whose editor it cannot see: an author's edit arrives as its own event.
One failure never stops the rest.
"""

from __future__ import annotations

import re
from datetime import datetime, timedelta
from typing import Any, Callable

from harness import commands, events
from harness import plan as plan_mod
from harness.clock import iso, parse_iso
from harness.config import (LABEL_BLOCKED, LABEL_BUILD, LABEL_PR, LABEL_PR_OPEN, LABEL_REVISE,
                            LABEL_WORKING, NIGHT_WORKFLOW)
from harness.context import Context
from harness.queue import label_names, queue_build, queue_revise

LOOKBACK = timedelta(days=3)
#: Anything younger than this may still be in its handler's hands.
SETTLE = timedelta(minutes=10)
#: An edit shows as `updated_at` later than `created_at` by more than this.
EDITED = timedelta(seconds=5)
QUEUE_LABELS = {LABEL_BUILD, LABEL_REVISE, LABEL_WORKING, LABEL_BLOCKED}
#: A `bot-night` run in one of these states is going, or about to.
LIVE_RUN = ("queued", "in_progress", "waiting", "pending", "requested")

_NUMBER = re.compile(r"/(?:issues|pulls)/(\d+)$")


def sweep(ctx: Context) -> list[str]:
    state = ctx.store.load()
    first = parse_iso((state.get("last_sweep") or {}).get("since"))
    now = ctx.now()
    if first is None:
        ctx.store.update(lambda s: s.update(last_sweep={"at": iso(now), "since": iso(now),
                                                        "notes": ["the first sweep: from now on"]}),
                         "sweep")
        return ["the first sweep: requests from now on are swept"]
    since = max(first, now - LOOKBACK)
    notes: list[str] = []
    for part in (_comments, _reviews, _assignments, _failed_ci, _night_run):
        try:
            notes += part(ctx, since)
        except Exception as exc:  # noqa: BLE001 - one part failing never stops the rest
            notes.append(f"{part.__name__.strip('_')}: could not finish ({type(exc).__name__}: "
                         f"{str(exc)[:200]})")
    ctx.store.update(lambda s: s.update(last_sweep={"at": iso(now), "since": iso(first),
                                                    "notes": notes[-20:]}), "sweep")
    return notes


def _each(items: list[Any], act: Callable[[Any], str | None], notes: list[str]) -> None:
    """Run `act` on every item; a failure is noted and the next item still runs."""
    for entry in items:
        try:
            note = act(entry)
        except Exception as exc:  # noqa: BLE001
            note = f"skipped one ({type(exc).__name__}: {str(exc)[:200]})"
        if note:
            notes.append(note)


def _number(url: str) -> int | None:
    match = _NUMBER.search(str(url or ""))
    return int(match.group(1)) if match else None


def _settled(ctx: Context, stamp: Any, since: datetime) -> bool:
    at = parse_iso(stamp)
    return at is not None and since <= at <= ctx.now() - SETTLE


def _trusted(ctx: Context, entry: dict[str, Any], level: int = 1) -> bool:
    user = entry.get("user") or {}
    return ctx.trust.level(str(user.get("login", "")), user.get("id"),
                           entry.get("author_association")) >= level


def _asks(ctx: Context, body: str) -> bool:
    return bool(commands.parse(body, ctx.cfg.bot_login)
                or commands.names_the_bot(body, ctx.cfg.bot_login))


def _comments(ctx: Context, since: datetime) -> list[str]:
    notes: list[str] = []
    for review_comment, found in (
            (False, ctx.gh.list_repo_comments(iso(since))),
            (True, ctx.gh.list_repo_review_comments(iso(since)))):
        def act(comment: dict[str, Any], review_comment: bool = review_comment) -> str | None:
            created = parse_iso(comment.get("created_at"))
            updated = parse_iso(comment.get("updated_at")) or created
            if not _settled(ctx, comment.get("created_at"), since):
                return None
            if created is None or updated is None or updated - created > EDITED:
                return None
            login = str((comment.get("user") or {}).get("login", "")).lower()
            body = str(comment.get("body") or "")
            if login in (ctx.cfg.bot_login.lower(), events.ACTIONS_BOT) or events._is_own(body):
                return None
            if not _asks(ctx, body) or not _trusted(ctx, comment):
                return None
            key = f"{'rc' if review_comment else 'c'}:{comment['id']}"
            if events.claimed(ctx, key) or events.answered(ctx, int(comment["id"]),
                                                           review_comment=review_comment):
                return None
            if review_comment:
                number = _number(comment.get("pull_request_url", ""))
                if number is None:
                    return None
                payload = {"action": "created", "comment": comment,
                           "pull_request": ctx.gh.get_pull(number)}
            else:
                number = _number(comment.get("issue_url", ""))
                if number is None:
                    return None
                payload = {"action": "created", "comment": comment,
                           "issue": ctx.gh.get_issue(number)}
            events.on_comment(ctx, payload, review_comment=review_comment)
            return f"answered a comment on #{number} that was never answered"
        _each(found, act, notes)
    return notes


def _reviews(ctx: Context, since: datetime) -> list[str]:
    notes: list[str] = []
    pulls = [t for t in ctx.gh.list_issues(labels="") if "pull_request" in t]

    def act(thread: dict[str, Any]) -> str | None:
        number = int(thread["number"])
        is_bot_pr = LABEL_PR in label_names(thread)
        answered = []
        for review in ctx.gh.list_reviews(number):
            if not _settled(ctx, review.get("submitted_at"), since) or not _trusted(ctx, review):
                continue
            changes = is_bot_pr and str(review.get("state", "")).upper() == "CHANGES_REQUESTED"
            if not (changes or _asks(ctx, str(review.get("body") or ""))):
                continue
            if events.claimed(ctx, f"r:{review.get('id')}"):
                continue
            payload = {"action": "submitted", "review": review, "pull_request": ctx.gh.get_pull(number)}
            out = events.on_review(ctx, payload)
            if out and not out[0].startswith(("ignored", "no command")):
                answered.append(str(review.get("id")))
        return f"answered {len(answered)} review(s) on #{number} never answered" if answered else None

    _each(pulls, act, notes)
    return notes


def _assignments(ctx: Context, since: datetime) -> list[str]:
    state = ctx.store.load()
    notes: list[str] = []

    def act(thread: dict[str, Any]) -> str | None:
        number = int(thread["number"])
        record = state["items"].get(str(number), {})
        if label_names(thread) & (QUEUE_LABELS | {LABEL_PR_OPEN}) or record.get("queued_at") or \
                record.get("stop_requested"):
            return None
        if not _settled(ctx, thread.get("updated_at") or iso(since), since - LOOKBACK):
            return None
        by = "an assignment"
        reply = queue_revise(ctx, number, by=by) if "pull_request" in thread else \
            queue_build(ctx, number, by=by)
        ctx.gh.create_comment(number, f"I was assigned here but never queued it. {reply}")
        return f"queued #{number} from an assignment nothing answered"

    _each(ctx.gh.list_issues(assignee=ctx.cfg.bot_login), act, notes)
    return notes


def _failed_ci(ctx: Context, since: datetime) -> list[str]:
    state = ctx.store.load()
    notes: list[str] = []
    pulls = [t for t in ctx.gh.list_issues(labels=LABEL_PR) if "pull_request" in t]

    def act(thread: dict[str, Any]) -> str | None:
        number = int(thread["number"])
        if label_names(thread) & QUEUE_LABELS or state["items"].get(str(number), {}).get(
                "stop_requested"):
            return None
        sha = (ctx.gh.get_pull(number).get("head") or {}).get("sha")
        if not sha:
            return None
        workflows = events.ci_workflows(ctx)
        runs = [r for r in ctx.gh.runs_for_sha(sha) if r.get("name") in workflows]
        failed = [r for r in runs if r.get("status") == "completed"
                  and r.get("conclusion") in events.CI_FAILED
                  and _settled(ctx, r.get("updated_at") or r.get("created_at"), since - LOOKBACK)]
        if not failed:
            return None
        latest = max(failed, key=lambda r: (str(r.get("created_at", "")), int(r.get("run_attempt") or 1)))
        out = events.on_ci(ctx, {"action": "completed", "workflow_run": latest})
        acted = [line for line in out if "re-ran" in line or "queued" in line or "blocked" in line]
        return f"CI on #{number}: {'; '.join(acted)}" if acted else None

    _each(pulls, act, notes)
    return notes


def _night_run(ctx: Context, since: datetime) -> list[str]:
    """Start a night run when a lane and a subscription are free for work and no run is on its
    way to take it. Runs after the other parts, so a request they just answered counts. A run
    that is going but holds no lane yet (still in its gate or plan, or delivering) is on its way,
    so the sweep waits for it rather than piling more runs behind it."""
    look = plan_mod.peek(ctx)
    if not look.work:
        return []
    live = [r for r in ctx.gh.list_runs(NIGHT_WORKFLOW, limit=20) if r.get("status") in LIVE_RUN]
    # A run waiting for the shared subscription to be quiet may wait two hours; it is not on
    # its way to the other subscriptions' work, which `peek` then offers instead.
    waiting = 1 if plan_mod.quiet_waiting(ctx) else 0
    if len(live) - waiting > look.held:
        return []
    ctx.dispatch()
    return [f"started a night run, as a lane was free: {look.reason}"]
