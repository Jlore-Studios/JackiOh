"""GitHub events, handled at once with no model call: commands, labels, assignment, CI results.

Runs in `bot-commands.yml`. Commands are acknowledged with a reaction and answered in one reply.
Model work is only ever queued here; it happens in the night run, or at once when an operator adds
`--force`.
"""

from __future__ import annotations

from typing import Any

from harness import commands
from harness.clock import iso
from harness.commands import Command
from harness.config import (LABEL_BLOCKED, LABEL_BUILD, LABEL_PR, LABEL_PR_OPEN, LABEL_REVISE,
                            LABEL_WORKING, MARKER)
from harness.context import Context
from harness.errors import GitHubError
from harness.queue import label_names, queue_build, queue_revise, set_state_label, stop
from harness.state import item as state_item
from harness import threads
from harness.status import report
from harness.trust import LEVEL_NAMES

ACTIONS_BOT = "github-actions[bot]"


#: Events handled even when the bot sent them: CI runs its own pushes started, and pull requests
#: auto-merge closed (GitHub credits the merge to whoever turned auto-merge on).
FROM_THE_BOT_TOO = {("workflow_run", "completed"), ("pull_request_target", "closed"),
                    ("pull_request", "closed")}


def handle(ctx: Context, name: str, payload: dict[str, Any]) -> list[str]:
    sender = str((payload.get("sender") or {}).get("login", ""))
    action = payload.get("action")
    if sender.lower() in (ctx.cfg.bot_login.lower(), ACTIONS_BOT) and (
            (name, action) not in FROM_THE_BOT_TOO):
        return ["ignored: the bot's own event"]
    if name == "issue_comment" and action == "created":
        return on_comment(ctx, payload)
    if name == "pull_request_review" and action == "submitted":
        return on_review(ctx, payload)
    if name == "issues" and action in ("labeled", "assigned"):
        return on_issue_change(ctx, payload, is_pr=False)
    if name in ("pull_request", "pull_request_target"):
        if action in ("labeled", "assigned"):
            return on_issue_change(ctx, payload, is_pr=True)
        if action == "closed":
            return on_pull_closed(ctx, payload)
    if name == "workflow_run" and action == "completed":
        return on_ci(ctx, payload)
    return [f"ignored: {name}.{action}"]


def _is_own(body: str) -> bool:
    """True for a comment the bot wrote (a person quoting it keeps the marker behind `>`)."""
    lines = [line for line in str(body or "").splitlines() if MARKER in line]
    return any(not line.lstrip().startswith(">") for line in lines)


def _level(ctx: Context, user: dict[str, Any], association: str | None) -> int:
    return ctx.trust.level(str(user.get("login", "")), user.get("id"), association)


def on_comment(ctx: Context, payload: dict[str, Any]) -> list[str]:
    comment = payload.get("comment") or {}
    body = str(comment.get("body") or "")
    if _is_own(body):
        return ["ignored: the bot's own comment"]
    found = commands.parse(body, ctx.cfg.bot_login)
    if not found:
        return ["no command"]
    thread = payload.get("issue") or {}
    user = comment.get("user") or {}
    level = _level(ctx, user, comment.get("author_association"))
    if level <= 0:
        return [f"ignored: @{user.get('login')} is not on the trust list"]
    ctx.gh.react(int(comment["id"]), "eyes")
    replies = run_commands(ctx, found, thread, user, level)
    login = user.get("login", "")
    ctx.gh.create_comment(int(thread["number"]), f"@{login}\n\n" + "\n\n".join(replies))
    ctx.gh.react(int(comment["id"]), "rocket")
    return replies


def run_commands(ctx: Context, found: list[Command], thread: dict[str, Any], user: dict[str, Any],
                 level: int) -> list[str]:
    replies: list[str] = []
    for command in found:
        head = f"> {command.line}\n\n" if len(found) > 1 else ""
        if command.verb.startswith("unknown:"):
            word = command.verb.split(":", 1)[1]
            replies.append(head + f"I do not know `{word}`. "
                           + commands.HELP.format(bot=ctx.cfg.bot_login))
            continue
        if level < command.level:
            need = LEVEL_NAMES.get(command.level, str(command.level))
            have = LEVEL_NAMES.get(level, str(level))
            flag = " with `--force`" if command.force else ""
            replies.append(head + f"`{command.verb}`{flag} needs the {need} level; you have "
                           f"{have}. Nothing was done.")
            continue
        try:
            replies.append(head + execute(ctx, command, thread, user))
        except GitHubError as exc:
            replies.append(head + f"That failed: {exc}")
    return replies


def execute(ctx: Context, command: Command, thread: dict[str, Any], user: dict[str, Any]) -> str:
    number = int(thread.get("number") or 0)
    is_pr = "pull_request" in thread
    by = str(user.get("login", ""))
    verb = command.verb
    if verb == "status":
        return report(ctx) + "\n\n" + commands.HELP.format(bot=ctx.cfg.bot_login)
    if verb == "halt":
        reason = command.args or "no reason given"
        ctx.store.update(lambda s: s.update(halted=True, halt={
            "by": by, "at": iso(ctx.now()), "reason": reason}), "halt")
        return ("Halted. No new model work starts until `/harness start`; a run already going "
                "stops at its next checkpoint and keeps its work.")
    if verb == "start":
        ctx.store.update(lambda s: s.update(halted=False, halt={}), "start")
        extra = ""
        if ctx.repo_halted():
            extra = " `.harness/HALT` is still on `main`, though, and it wins until it is deleted."
        if command.force:
            ctx.dispatch(force=True)
            return "Started, and a run is starting now." + extra
        return "Started. The bot works in the next night window." + extra
    if verb == "run":
        target = command.args.lstrip("#")
        item = int(target) if target.isdigit() else None
        ctx.dispatch(force=True, item=item)
        which = f" for #{item}" if item else ""
        return f"A run{which} is starting now, outside the window if need be."
    if verb == "suggest":
        ctx.store.update(lambda s: s["suggest"].update(requested=True), "suggest")
        if command.force:
            ctx.dispatch(force=True, mode="suggest")
            return "A suggestion survey is starting now."
        return "I will survey for suggestions the next time the queue is empty in the window."
    if not number:
        return f"`{verb}` needs an issue or a pull request to act on."
    if verb == "stop":
        return stop(ctx, number, by=by)
    notes_line = ""
    if command.args:
        notes_line = " I will read your notes with the rest of the thread."
    if verb == "build" or (verb == "request" and not is_pr):
        if is_pr:
            return queue_revise(ctx, number, by=by, force=command.force) + notes_line
        return queue_build(ctx, number, by=by, force=command.force) + notes_line
    if verb in ("revise", "request"):
        if not is_pr:
            return queue_build(ctx, number, by=by, force=command.force) + notes_line
        return queue_revise(ctx, number, by=by, force=command.force) + notes_line
    return f"`{verb}` is not something I can do here."


def on_review(ctx: Context, payload: dict[str, Any]) -> list[str]:
    review = payload.get("review") or {}
    pull = payload.get("pull_request") or {}
    user = review.get("user") or {}
    level = _level(ctx, user, review.get("author_association"))
    if level <= 0:
        return [f"ignored: @{user.get('login')} is not on the trust list"]
    body = str(review.get("body") or "")
    found = commands.parse(body, ctx.cfg.bot_login)
    thread = {"number": pull.get("number"), "pull_request": {}}
    if found:
        replies = run_commands(ctx, found, thread, user, level)
        ctx.gh.create_comment(int(pull["number"]), f"@{user.get('login')}\n\n" + "\n\n".join(replies))
        return replies
    names = label_names(pull)
    if LABEL_PR in names and str(review.get("state", "")).lower() == "changes_requested" and level >= 2:
        reply = queue_revise(ctx, int(pull["number"]), by=str(user.get("login")))
        ctx.gh.create_comment(int(pull["number"]), f"@{user.get('login')}\n\n{reply}")
        return [reply]
    return ["no command in the review"]


def on_issue_change(ctx: Context, payload: dict[str, Any], *, is_pr: bool) -> list[str]:
    thread = payload.get("pull_request") if is_pr else payload.get("issue")
    thread = thread or {}
    sender = payload.get("sender") or {}
    # Only people with triage access can label or assign, so GitHub has vouched for the sender.
    level = ctx.trust.level(str(sender.get("login", "")), sender.get("id"), "COLLABORATOR")
    number = int(thread.get("number") or 0)
    action = payload.get("action")
    wanted = False
    if action == "labeled":
        name = str((payload.get("label") or {}).get("name", ""))
        wanted = (name == LABEL_BUILD and not is_pr) or (name == LABEL_REVISE and is_pr)
        if not wanted:
            return [f"ignored: label {name}"]
    elif action == "assigned":
        assignee = str((payload.get("assignee") or {}).get("login", ""))
        wanted = assignee.lower() == ctx.cfg.bot_login.lower()
        if not wanted:
            return [f"ignored: assigned to {assignee}"]
    if level < 2:
        if action == "labeled":
            ctx.gh.remove_label(number, LABEL_REVISE if is_pr else LABEL_BUILD)
        return [f"ignored: @{sender.get('login')} is below the maintainer level"]
    by = str(sender.get("login", ""))
    if is_pr:
        reply = queue_revise(ctx, number, by=by, label_present=action == "labeled")
    else:
        reply = queue_build(ctx, number, by=by, label_present=action == "labeled")
    ctx.gh.create_comment(number, f"@{by} {reply}")
    return [reply]


def on_pull_closed(ctx: Context, payload: dict[str, Any]) -> list[str]:
    pull = payload.get("pull_request") or {}
    if LABEL_PR not in label_names(pull):
        return ["ignored: not a bot pull request"]
    issue = threads.linked_issue(pull)
    number = int(pull["number"])
    ctx.store.update(lambda s: state_item(s, number).update(closed_at=iso(ctx.now()),
                                                            merged=bool(pull.get("merged"))),
                     f"closed #{number}")
    if issue is None:
        return ["closed; no linked issue"]
    ctx.gh.remove_label(issue, LABEL_PR_OPEN)
    if not pull.get("merged"):
        ctx.gh.create_comment(issue, f"#{number} was closed without merging. Comment "
                              f"`/harness build` to have me try again.")
        return [f"#{number} closed unmerged"]
    return [f"#{number} merged; #{issue} closes with it"]


def on_ci(ctx: Context, payload: dict[str, Any]) -> list[str]:
    run = payload.get("workflow_run") or {}
    if run.get("name") != ctx.cfg.ci_workflow:
        return [f"ignored: workflow {run.get('name')}"]
    if run.get("conclusion") not in ("failure", "timed_out"):
        return [f"ignored: conclusion {run.get('conclusion')}"]
    branch = str(run.get("head_branch") or "")
    numbers = [int(p["number"]) for p in run.get("pull_requests") or [] if p.get("number")]
    if not numbers and branch:
        numbers = [int(p["number"]) for p in ctx.gh.list_pulls(state="open", head=branch)]
    out: list[str] = []
    for number in numbers:
        pull = ctx.gh.get_pull(number)
        if pull.get("state") != "open" or (pull.get("head") or {}).get("sha") != run.get("head_sha"):
            out.append(f"#{number}: stale run")
            continue
        names = label_names(pull)
        if LABEL_PR not in names:
            out.append(f"#{number}: not a bot pull request")
            continue
        record = ctx.store.load()["items"].get(str(number), {})
        if names & {LABEL_BLOCKED, LABEL_REVISE, LABEL_WORKING} or record.get("stop_requested"):
            out.append(f"#{number}: blocked, stopped or already queued; left alone")
            continue
        sha = str(run.get("head_sha"))
        reruns = dict(record.get("ci_reruns") or {})
        if int(reruns.get(sha, 0)) < ctx.cfg.ci_reruns:
            ctx.gh.rerun_failed_jobs(run["id"])
            reruns[sha] = int(reruns.get(sha, 0)) + 1
            ctx.store.update(lambda s: state_item(s, number).update(ci_reruns=reruns),
                             f"ci rerun #{number}")
            out.append(f"#{number}: re-ran the failed jobs of run {run['id']}")
            continue
        fixes = int(record.get("ci_fixes", 0))
        link = f"[run]({run.get('html_url')})"
        if fixes >= ctx.cfg.max_failures:
            set_state_label(ctx, number, names, LABEL_BLOCKED)
            if pull.get("auto_merge"):
                try:
                    ctx.gh.disable_auto_merge(pull["node_id"])
                except GitHubError:
                    pass
            ctx.gh.create_comment(number, f"CI failed again ({link}) after {fixes} tries at "
                                  "fixing it, so I stopped. It needs a person.")
            out.append(f"#{number}: blocked after {fixes} CI fixes")
            continue
        reply = queue_revise(ctx, number, by=ctx.cfg.bot_login, source="ci",
                             extra={"ci_run_id": run["id"], "ci_sha": sha, "ci_fixes": fixes + 1})
        ctx.gh.create_comment(number, f"CI failed again ({link}). {reply}")
        out.append(f"#{number}: queued a CI fix")
    return out or ["no open pull request for this run"]
