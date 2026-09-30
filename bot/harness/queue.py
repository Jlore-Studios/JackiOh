"""The work queue, kept as labels on issues and pull requests plus a record in the state file.

`bot:build` on an issue and `bot:revise` on a pull request mean queued; `bot:working` means a
run holds it; `bot:blocked` means it waits for a person. Only one `bot:` state label is on a
thread at a time, apart from `bot:pr` and `bot:pr-open`, which say what a thread is.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from harness.clock import human_delta, iso
from harness.config import (LABEL_BLOCKED, LABEL_BUILD, LABEL_PR, LABEL_PR_OPEN, LABEL_REVISE,
                            LABEL_WORKING)
from harness.context import Context
from harness.errors import GitHubError
from harness.state import item as state_item

STATE_LABELS = (LABEL_BUILD, LABEL_REVISE, LABEL_WORKING, LABEL_BLOCKED)


def label_names(thread: dict[str, Any]) -> set[str]:
    return {str(label.get("name")) for label in thread.get("labels", []) if isinstance(label, dict)}


def set_state_label(ctx: Context, number: int, current: set[str], wanted: str | None) -> None:
    """Leave exactly `wanted` (or none) of the queue's state labels on the thread."""
    for name in STATE_LABELS:
        if name in current and name != wanted:
            ctx.gh.remove_label(number, name)
    if wanted and wanted not in current:
        ctx.gh.add_labels(number, [wanted])


def branch_for_issue(number: int) -> str:
    return f"bot/issue-{int(number)}"


def open_pull_for_branch(ctx: Context, branch: str) -> dict[str, Any] | None:
    pulls = ctx.gh.list_pulls(state="open", head=branch)
    return pulls[0] if pulls else None


def _when(ctx: Context) -> str:
    window = ctx.window
    now = ctx.now()
    if window.is_open(now):
        return f"in the current night window ({window.describe()})"
    return f"when the night window opens ({window.describe()}, in {human_delta(window.next_open(now) - now)})"


def _halt_note(ctx: Context, state: dict[str, Any]) -> str:
    return " The bot is halted, though: `/harness start` resumes it." if state.get("halted") else ""


def queue_build(ctx: Context, number: int, *, by: str, force: bool = False,
                label_present: bool = False) -> str:
    """Queue an issue for building. Returns the reply line."""
    issue = ctx.gh.get_issue(number)
    if "pull_request" in issue:
        return queue_revise(ctx, number, by=by, force=force, label_present=label_present)
    if issue.get("state") != "open":
        return f"#{number} is closed, so there is nothing to build. Reopen it first."
    names = label_names(issue)
    if LABEL_WORKING in names:
        _pending(ctx, number, by)
        return (f"I am working on #{number} right now. When this run ends I go round once more "
                "with your comment.")
    if LABEL_PR_OPEN in names:
        pull = open_pull_for_branch(ctx, branch_for_issue(number))
        if pull is not None:
            set_state_label(ctx, number, names, None)
            return queue_revise(ctx, int(pull["number"]), by=by, force=force)
    set_state_label(ctx, number, names, LABEL_BUILD)
    state = ctx.store.update(lambda s: _queued(s, number, "build", by, force, ctx),
                             f"queue #{number}")
    if force:
        return _start_now(ctx, number, "build", f"Queued #{number}") + _halt_note(ctx, state)
    return f"Queued #{number}; I will build it {_when(ctx)}.{_halt_note(ctx, state)}"


def queue_revise(ctx: Context, number: int, *, by: str, force: bool = False, source: str = "request",
                 label_present: bool = False, extra: dict[str, Any] | None = None) -> str:
    """Queue a pull request for a revision. Returns the reply line."""
    pull = ctx.gh.get_pull(number)
    if pull.get("state") != "open":
        return f"#{number} is not open, so I will not change it."
    head_repo = ((pull.get("head") or {}).get("repo") or {}).get("full_name")
    if head_repo != ctx.cfg.repo:
        return f"#{number} comes from a fork; I can only push to branches in {ctx.cfg.repo}."
    names = label_names(pull)
    if LABEL_WORKING in names:
        _pending(ctx, number, by)
        return (f"I am revising #{number} right now. When this run ends I go round once more "
                "with your comment.")
    set_state_label(ctx, number, names, LABEL_REVISE)
    if LABEL_PR in names and pull.get("auto_merge"):
        try:
            ctx.gh.disable_auto_merge(pull["node_id"])
        except GitHubError:
            pass
    def change(state: dict[str, Any]) -> None:
        _queued(state, number, "revise", by, force, ctx)
        record = state_item(state, number)
        record["source"] = source
        record.update(extra or {})
    state = ctx.store.update(change, f"queue revise #{number}")
    held = " Auto-merge is off until the revision lands." if LABEL_PR in names else ""
    if force:
        return (_start_now(ctx, number, "revise", f"Queued a revision of #{number}") + held
                + _halt_note(ctx, state))
    return f"Queued a revision of #{number}; I will do it {_when(ctx)}.{held}{_halt_note(ctx, state)}"


def _start_now(ctx: Context, number: int, mode: str, queued: str) -> str:
    """Start a forced run. If GitHub refuses, the item is still queued and still forced, and the
    next hourly run starts it, inside the window or not."""
    try:
        ctx.dispatch(item=number, force=True, mode=mode)
    except GitHubError as exc:
        return (f"{queued}, but starting a run now failed ({str(exc)[:200]}). It stays forced, so "
                "the next hourly run starts it.")
    return f"{queued} and started a run now (`--force`)."


def _queued(state: dict[str, Any], number: int, kind: str, by: str, force: bool, ctx: Context) -> None:
    """A fresh request: it clears a stop, the failure and interruption counts and any pending
    note, and keeps `ci_fixes`, which only a person's `forget` clears."""
    record = state_item(state, number)
    record.update(kind=kind, queued_at=iso(ctx.now()), requested_by=by, forced=bool(force),
                  stop_requested=False, failures=0, interruptions=0, pending_request=False)


def _pending(ctx: Context, number: int, by: str) -> None:
    """Remember a request that arrived while a run held the thread; deliver requeues it."""
    ctx.store.update(lambda s: state_item(s, number).update(
        pending_request=True, pending_by=by, pending_at=iso(ctx.now())), f"pending #{number}")


def stop(ctx: Context, number: int, *, by: str) -> str:
    """Take a thread out of the queue. A run that holds it keeps `bot:working` until it has
    stopped, so a request made in the meantime waits for that run instead of racing it."""
    thread = ctx.gh.get_issue(number)
    names = label_names(thread)
    working = LABEL_WORKING in names
    set_state_label(ctx, number, names, LABEL_WORKING if working else None)
    if "pull_request" in thread and LABEL_PR in names:
        try:
            pull = ctx.gh.get_pull(number)
            if pull.get("auto_merge"):
                ctx.gh.disable_auto_merge(pull["node_id"])
        except GitHubError:
            pass
    def change(state: dict[str, Any]) -> None:
        record = state_item(state, number)
        record.update(stop_requested=True, stopped_by=by, stopped_at=iso(ctx.now()), forced=False,
                      pending_request=False)
    ctx.store.update(change, f"stop #{number}")
    if working:
        return f"Stopping work on #{number}; the run gives up at its next checkpoint."
    return f"#{number} is out of the queue."


@dataclass
class Candidate:
    number: int
    kind: str  # "build" | "revise"
    title: str
    forced: bool
    queued_at: str


def candidates(ctx: Context, state: dict[str, Any]) -> list[Candidate]:
    """Queued threads: forced requests first, then revisions, then oldest first.

    Read by label, so no number of open threads hides one. Either queue label queues either kind
    of thread: an issue builds and a pull request revises. A queue label is the request, so a
    thread that failed before and was labelled again is taken again."""
    found: dict[int, Candidate] = {}
    for label in (LABEL_BUILD, LABEL_REVISE):
        for thread in ctx.gh.list_issues(labels=label):
            number = int(thread["number"])
            if LABEL_WORKING in label_names(thread) or number in found:
                continue
            record = state["items"].get(str(number), {})
            found[number] = Candidate(
                number, "revise" if "pull_request" in thread else "build",
                str(thread.get("title", "")), bool(record.get("forced")),
                str(record.get("queued_at") or thread.get("created_at") or ""))
    return sorted(found.values(), key=lambda c: (not c.forced, c.kind != "revise", c.queued_at,
                                                 c.number))

