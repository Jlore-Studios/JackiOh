"""The first job of a night run: decide what this run does, and claim it.

It spends nothing. In order: the two kill switches, the window (lifted by `--force`), the usage
stop, housekeeping (items a dead run left `bot:working`, bot PRs that conflict with `main`), then
one item from the queue, or a suggestion survey when the queue is empty and suggestions are due.
"""

from __future__ import annotations

from datetime import timedelta
from typing import Any

from harness import threads
from harness.clock import iso, parse_iso
from harness.config import (LABEL_BLOCKED, LABEL_BUILD, LABEL_PR, LABEL_PR_OPEN, LABEL_REVISE,
                            LABEL_SUGGESTION, LABEL_WORKING)
from harness.context import Context
from harness.errors import GitHubError
from harness.prompts import data
from harness.queue import (branch_for_issue, candidates, label_names, open_pull_for_branch,
                           set_state_label)
from harness.state import item as state_item
from harness.state import usage_refusal

MODES = ("auto", "build", "revise", "suggest")
#: After a failure that was not an item's fault, unforced runs wait this long before trying again.
INFRA_BACKOFF = timedelta(minutes=50)
#: A run asked for and never taken up is dropped after this long.
RUN_REQUEST_TTL = timedelta(hours=12)
CI_LOG_JOBS = 4


def run_link(cfg: Any) -> str:
    """` ([run](url))` for a run in Actions, and nothing for one started by hand."""
    return f" ([run]({cfg.run_url}))" if cfg.run_url else ""


def nothing(reason: str) -> dict[str, Any]:
    return {"action": "none", "reason": reason}


def stops(ctx: Context, state: dict[str, Any], force: bool) -> str | None:
    """Why no run may start now, whatever is queued, or None. `make` and `peek` share it."""
    cfg = ctx.cfg
    now = ctx.now()
    if ctx.repo_halted():
        return "halted by .harness/HALT on main"
    if state.get("halted"):
        return "halted by /harness halt"
    refusal = usage_refusal(state, dict(cfg.usage_stop), now)
    if refusal:
        return f"usage stop: {refusal}"
    if cfg.claude_ready_known and not cfg.claude_token_present:
        return "the CLAUDE_CODE_OAUTH_TOKEN secret is not set, so no model can run"
    last_infra = parse_iso((state.get("last_infra") or {}).get("at"))
    if not force and last_infra is not None and now - last_infra < INFRA_BACKOFF:
        return (f"backing off after a failure outside any item at {iso(last_infra)}: "
                f"{(state.get('last_infra') or {}).get('reason', '')[:200]}")
    return None


def peek(ctx: Context, *, force: bool = False, item: int | None = None,
         mode: str = "auto") -> tuple[bool, str, bool]:
    """Whether a run would find work, why, and whether that work was forced. Changes nothing.

    The gate asks this before it spends anything on reading the subscription's usage."""
    if mode not in MODES:
        return False, f"unknown mode {mode!r}", False
    state = ctx.store.load()
    asked = run_request(ctx, state)
    if asked is not None:
        force = True
        item = item if item is not None else asked.get("item")
    stop = stops(ctx, state, force)
    if stop:
        return False, stop, False
    only_forced = not ctx.window.is_open(ctx.now()) and not force
    queue = candidates(ctx, state)
    if only_forced:
        queue = [c for c in queue if c.forced]
    if item is not None:
        queue = [c for c in queue if c.number == int(item)]
    if mode in ("build", "revise"):
        queue = [c for c in queue if c.kind == mode]
    if mode != "suggest" and queue:
        first = queue[0]
        return True, f"#{first.number} is queued to {first.kind}", bool(force or first.forced)
    if only_forced:
        tidy = housekeeping_due(ctx, state, forced_only=True)
        if tidy:
            return True, tidy, True
        return False, f"outside the night window ({ctx.window.describe()})", False
    if item is not None and mode != "suggest":
        return False, f"#{item} is not queued", False
    tidy = housekeeping_due(ctx, state)
    if tidy:
        return True, tidy, force
    if mode in ("auto", "suggest") and suggestions_due(ctx, state, force=mode == "suggest"):
        return True, "a suggestion survey is due", force
    return False, "nothing is queued", False


def run_request(ctx: Context, state: dict[str, Any]) -> dict[str, Any] | None:
    """A `/harness run` (or `start --force`) not yet taken up, if one is fresh enough."""
    asked = state.get("run_requested")
    at = parse_iso((asked or {}).get("at"))
    if not isinstance(asked, dict) or at is None or ctx.now() - at > RUN_REQUEST_TTL:
        return None
    return asked


def housekeeping_due(ctx: Context, state: dict[str, Any], *,
                     forced_only: bool = False) -> str | None:
    """What `housekeeping` would requeue, read without changing anything, or None."""
    for thread in ctx.gh.list_issues(labels=LABEL_WORKING):
        if forced_only and not state["items"].get(str(thread["number"]), {}).get("forced"):
            continue
        run_id = str(state["items"].get(str(thread["number"]), {}).get("run_id") or "")
        try:
            alive = bool(run_id) and ctx.gh.get_run(run_id).get("status") in (
                "queued", "in_progress", "waiting")
        except GitHubError:
            alive = False
        if not alive:
            return f"#{thread['number']} was left working by a run that ended"
    if forced_only:
        return None
    for thread in ctx.gh.list_issues(labels=LABEL_PR):
        names = label_names(thread)
        if "pull_request" not in thread or names & {LABEL_REVISE, LABEL_WORKING, LABEL_BLOCKED}:
            continue
        record = state["items"].get(str(thread["number"]), {})
        if record.get("stop_requested"):
            continue
        if ctx.gh.get_pull(int(thread["number"])).get("mergeable_state") == "dirty":
            return f"#{thread['number']} conflicts with main"
    return None


def make(ctx: Context, *, force: bool = False, item: int | None = None, mode: str = "auto") -> dict:
    """Decide what this run does. Outside the window only forced work runs: the `force` of this
    dispatch, or an item an operator queued with `--force`, which outlives the run it started."""
    cfg = ctx.cfg
    now = ctx.now()
    if mode not in MODES:
        return nothing(f"unknown mode {mode!r}")
    state = ctx.store.load()
    asked = run_request(ctx, state)
    if asked is not None:
        # Taken up here, whatever this run finds: one request, one run.
        ctx.store.update(lambda s: s.update(run_requested=None), "run request taken")
        force = True
        item = item if item is not None else asked.get("item")
    stop = stops(ctx, state, force)
    if stop:
        return nothing(stop)
    in_window = ctx.window.is_open(now)
    only_forced = not in_window and not force
    if only_forced and not any(c.forced for c in candidates(ctx, state)):
        return nothing(f"outside the night window ({ctx.window.describe()})")
    notes = housekeeping(ctx, state)
    queue = candidates(ctx, ctx.store.load())
    if only_forced:
        queue = [c for c in queue if c.forced]
    if item is not None:
        queue = [c for c in queue if c.number == int(item)]
    if mode in ("build", "revise"):
        queue = [c for c in queue if c.kind == mode]
    if mode != "suggest":
        for candidate in queue:
            planned = claim(ctx, candidate.number, candidate.kind)
            if planned is not None:
                planned["housekeeping"] = notes
                planned["forced"] = bool(force or candidate.forced)
                return planned
    if item is not None and mode != "suggest":
        return nothing(f"#{item} is not queued (or has failed {cfg.max_failures} times)")
    if mode in ("auto", "suggest") and not only_forced:
        planned = suggestion_plan(ctx, force=mode == "suggest")
        if planned is not None:
            return planned
    return {**nothing("nothing is queued"), "housekeeping": notes}


def housekeeping(ctx: Context, state: dict[str, Any]) -> list[str]:
    """Requeue items a dead run left working; queue a revision for conflicted bot PRs."""
    notes: list[str] = []
    for thread in ctx.gh.list_issues(labels=LABEL_WORKING):
        number = int(thread["number"])
        record = state["items"].get(str(number), {})
        run_id = str(record.get("run_id") or "")
        if run_id and run_id == ctx.cfg.run_id:
            continue
        alive = False
        if run_id:
            try:
                alive = ctx.gh.get_run(run_id).get("status") in ("queued", "in_progress", "waiting")
            except GitHubError:
                alive = False
        if alive:
            continue
        wanted = LABEL_REVISE if "pull_request" in thread else LABEL_BUILD
        set_state_label(ctx, number, label_names(thread), wanted)
        ctx.gh.create_comment(number, "The run that was working on this ended without "
                              "finishing; it is back in the queue.")
        notes.append(f"requeued #{number} from a dead run")
    for thread in ctx.gh.list_issues(labels=LABEL_PR):
        if "pull_request" not in thread:
            continue
        names = label_names(thread)
        if names & {LABEL_REVISE, LABEL_WORKING, LABEL_BLOCKED}:
            continue
        pull = ctx.gh.get_pull(int(thread["number"]))
        record = state["items"].get(str(pull["number"]), {})
        if pull.get("mergeable_state") == "dirty" and not record.get("stop_requested"):
            number = int(pull["number"])
            set_state_label(ctx, number, names, LABEL_REVISE)
            ctx.store.update(lambda s, n=number: state_item(s, n).update(
                kind="revise", source="conflict", queued_at=iso(ctx.now()), failures=0,
                stop_requested=False), f"conflict #{number}")
            notes.append(f"queued #{number}: it conflicts with main")
    for thread in ctx.gh.list_issues(labels=LABEL_PR_OPEN):
        number = int(thread["number"])
        if open_pull_for_branch(ctx, branch_for_issue(number)) is None:
            ctx.gh.remove_label(number, LABEL_PR_OPEN)
            notes.append(f"#{number} has no open bot PR any more")
    return notes


def claim(ctx: Context, number: int, kind: str) -> dict[str, Any] | None:
    """Build the plan for one queued thread and mark it `bot:working`."""
    cfg = ctx.cfg
    thread = ctx.gh.get_issue(number)
    names = label_names(thread)
    if thread.get("state") != "open":
        set_state_label(ctx, number, names, None)
        return None
    state = ctx.store.load()
    record = state["items"].get(str(number), {})
    if kind == "build":
        planned: dict[str, Any] = {
            "action": "build",
            "number": number,
            "title": thread.get("title", ""),
            "branch": branch_for_issue(number),
            "thread": threads.issue_thread(ctx.gh, ctx.trust, number, cfg.bot_login),
            "previous_findings": record.get("last_findings") or [],
            "previous_question": record.get("question") or "",
        }
        message = (f"Starting work on this now{run_link(cfg)}. I build it, run the "
                   f"repository's checks, and have an adversarial reviewer read the change, up to "
                   f"{cfg.max_review_cycles} rounds. Only an approved change becomes a pull request.")
    else:
        pull = ctx.gh.get_pull(number)
        head_repo = ((pull.get("head") or {}).get("repo") or {}).get("full_name")
        if head_repo != cfg.repo:
            set_state_label(ctx, number, names, None)
            ctx.gh.create_comment(number, "This pull request comes from a fork, so I cannot push "
                                  "to it. Taking it out of the queue.")
            return None
        source = str(record.get("source") or "request")
        issue_number = threads.linked_issue(pull)
        feedback = threads.pull_feedback(ctx.gh, ctx.trust, number, cfg.bot_login,
                                         record.get("feedback_since"), issue=issue_number)
        if source == "ci":
            feedback += "\n\n" + ci_logs(ctx, record.get("ci_run_id"))
        issue_text = ""
        if issue_number:
            try:
                issue_text = threads.issue_thread(ctx.gh, ctx.trust, issue_number, cfg.bot_login)
            except GitHubError:
                issue_number = None
        pull_text = threads.pull_text(pull)
        planned = {
            "action": "revise",
            "number": number,
            "title": pull.get("title", ""),
            "branch": (pull.get("head") or {}).get("ref", ""),
            "source": source,
            "pull": pull_text,
            "issue": issue_text,
            "feedback": feedback,
            "thread": "\n\n".join(p for p in (pull_text, issue_text, feedback) if p),
            "bot_pr": LABEL_PR in names,
            "issue_number": issue_number,
        }
        message = (f"Starting a revision now{run_link(cfg)}, because of: {source}. It "
                   "goes through the same checks and adversarial review before I push it.")
    set_state_label(ctx, number, names, LABEL_WORKING)
    def change(state: dict[str, Any]) -> None:
        entry = state_item(state, number)
        entry.update(run_id=cfg.run_id, started_at=iso(ctx.now()), kind=kind,
                     stop_requested=False, pending_request=False)
        state["last_run"] = {"at": iso(ctx.now()), "url": cfg.run_url, "what": f"{kind} #{number}"}
    ctx.store.update(change, f"claim #{number}")
    ctx.gh.create_comment(number, message)
    return planned


def ci_logs(ctx: Context, run_id: Any) -> str:
    """The tails of the failed jobs of one CI run, fenced as data."""
    if not run_id:
        return data("(no CI run was recorded)", "CI failure")
    try:
        jobs = [j for j in ctx.gh.list_jobs(run_id)
                if j.get("conclusion") in ("failure", "timed_out", "cancelled")]
    except GitHubError as exc:
        return data(f"(could not read the CI run: {exc})", "CI failure")
    parts = []
    for job in jobs[:CI_LOG_JOBS]:
        failed = [s.get("name") for s in job.get("steps", []) if s.get("conclusion") == "failure"]
        parts.append(f"## Job: {job.get('name')} ({job.get('conclusion')})\n"
                     f"Failed steps: {', '.join(str(f) for f in failed) or '?'}\n\n"
                     f"{ctx.gh.job_log(job.get('id'))}")
    return data("\n\n".join(parts) or "(no failed job found)", f"CI run {run_id}")


def open_suggestion_count(ctx: Context) -> int:
    return len([i for i in ctx.gh.list_issues(labels=LABEL_SUGGESTION) if "pull_request" not in i])


def suggestions_due(ctx: Context, state: dict[str, Any], *, force: bool = False) -> int:
    """How many suggestions a survey may open now: 0 when none is due or the cap is full."""
    cfg = ctx.cfg
    if not cfg.suggestions_enabled:
        return 0
    suggest = state.get("suggest") or {}
    last = parse_iso(suggest.get("last_run"))
    due = force or bool(suggest.get("requested")) or last is None or (
        ctx.now() - last >= timedelta(hours=cfg.suggestions_min_interval_hours))
    if not due:
        return 0
    return max(0, cfg.suggestions_max_open - open_suggestion_count(ctx))


def suggestion_plan(ctx: Context, *, force: bool) -> dict[str, Any] | None:
    cfg = ctx.cfg
    count = suggestions_due(ctx, ctx.store.load(), force=force)
    if count <= 0:
        return None
    lines = []
    for issue in ctx.gh.list_issues(labels="", state="all", limit=150):
        kind = "PR" if "pull_request" in issue else "issue"
        tags = ", ".join(sorted(label_names(issue)))
        lines.append(f"- {kind} #{issue['number']} [{issue.get('state')}] {issue.get('title', '')}"
                     + (f" ({tags})" if tags else ""))
    previous = ctx.store.load().get("suggest") or {}
    was_requested = bool(previous.get("requested"))

    def change(s: dict[str, Any]) -> None:
        s["suggest"] = {"last_run": iso(ctx.now()), "requested": False}
        s["last_run"] = {"at": iso(ctx.now()), "url": cfg.run_url, "what": "suggestions"}
    ctx.store.update(change, "claim suggestions")
    return {
        "action": "suggest",
        "count": count,
        "existing": data("\n".join(lines) or "(none)", "Existing issues and pull requests"),
        "forced": bool(force),
        "was_requested": was_requested,
        "previous_last_run": previous.get("last_run"),
    }
