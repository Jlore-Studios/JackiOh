"""GitHub events, handled at once with no model call: commands, labels, assignment, CI results.

Runs in `bot-commands.yml`. Commands are acknowledged with a reaction and answered in one reply.
Model work is only ever queued here; it happens in the night run, or at once when an operator adds
`--force`.
"""

from __future__ import annotations

from typing import Any

from harness import asks, commands
from datetime import timedelta

from harness.asks import Ask
from harness.clock import iso, parse_iso
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
#: CI conclusions that leave a bot pull request unable to merge by itself.
CI_FAILED = ("failure", "timed_out", "cancelled", "startup_failure")


def ci_workflows(ctx: Context) -> tuple[str, ...]:
    """The workflows whose checks a bot pull request must pass: CI and the bot's own selftest."""
    return (ctx.cfg.ci_workflow, "bot selftest")


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
    if name == "issue_comment" and action in ("created", "edited"):
        return on_comment(ctx, payload)
    if name == "pull_request_review_comment" and action in ("created", "edited"):
        return on_comment(ctx, payload, review_comment=True)
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


#: The reaction the bot leaves once it has answered a comment. The sweep treats a command without
#: it as never answered; nobody but the bot's own logins can leave it as the bot. The reactions
#: for the later stages of model work are in `asks`.
ANSWERED = asks.ANSWERED


def answered(ctx: Context, comment_id: int, *, review_comment: bool = False) -> bool:
    """True when the bot has already answered this comment (its rocket reaction is there)."""
    logins = {ctx.cfg.bot_login.lower(), ACTIONS_BOT}
    for reaction in ctx.gh.reactions(comment_id, review_comment=review_comment):
        who = str((reaction.get("user") or {}).get("login", "")).lower()
        if reaction.get("content") == ANSWERED and who in logins:
            return True
    return False


NUDGE = ("I saw my name, but no request I could act on. Start a line with "
         "`@{bot} <what you want>` or `/harness build`; `/harness help` lists the rest.")


#: How long a claim is kept: longer than the sweep looks back.
CLAIM_DAYS = 5


def claim(ctx: Context, key: str, lines: list[str]) -> list[str]:
    """Record, compare-and-set, that these lines of one comment or review are being answered.

    Returns the lines nobody claimed before, so an event handler and the sweep, or a comment and
    its edit, never act on the same line twice. A line is claimed before it runs: a handler that
    dies half-way leaves its 👀 and no reply rather than running a command a second time."""
    now = ctx.now()
    fresh: list[str] = []

    def change(state: dict[str, Any]) -> None:
        handled = state.setdefault("handled", {})
        for old in [k for k, v in handled.items()
                    if (parse_iso(v.get("at")) or now) < now - timedelta(days=CLAIM_DAYS)]:
            del handled[old]
        before = handled.get(key, {}).get("lines", [])
        fresh[:] = [line for line in lines if line not in before]
        if fresh:
            handled[key] = {"at": iso(now), "lines": sorted(set(before) | set(lines))}

    ctx.store.update(change, f"claim {key}")
    return fresh


def claimed(ctx: Context, key: str) -> bool:
    return key in (ctx.store.load().get("handled") or {})


def _reply(ctx: Context, number: int, body: str) -> None:
    try:
        ctx.gh.create_comment(number, body)
    except GitHubError:
        ctx.gh.create_comment(number, body)  # once more; a second failure is the job's error


def on_comment(ctx: Context, payload: dict[str, Any], *, review_comment: bool = False) -> list[str]:
    """Answer the commands in a comment, a line comment on a diff, or an edit of either."""
    comment = payload.get("comment") or {}
    body = str(comment.get("body") or "")
    if _is_own(body):
        return ["ignored: the bot's own comment"]
    found = commands.parse(body, ctx.cfg.bot_login)
    nudge = not found and commands.names_the_bot(body, ctx.cfg.bot_login)
    if not found and not nudge:
        return ["no command"]
    if review_comment:
        pull = payload.get("pull_request") or {}
        thread = {"number": pull.get("number"), "pull_request": {}}
    else:
        thread = payload.get("issue") or {}
    user = comment.get("user") or {}
    if payload.get("action") == "edited":
        editor = payload.get("sender") or {}
        if editor.get("id") is not None and editor.get("id") != user.get("id"):
            return ["ignored: an edit by someone other than the comment's author"]
    level = _level(ctx, user, comment.get("author_association"))
    if level <= 0:
        return [f"ignored: @{user.get('login')} is not on the trust list"]
    comment_id = int(comment["id"])
    key = f"{'rc' if review_comment else 'c'}:{comment_id}"
    lines = [c.line for c in found] or ["(named the bot)"]
    fresh = claim(ctx, key, lines)
    if not fresh:
        return ["ignored: already answered"]
    ctx.gh.react(comment_id, asks.SEEN, review_comment=review_comment)
    ask = Ask(asks.key(comment_id, review_comment=review_comment))
    if nudge:
        replies = [NUDGE.format(bot=ctx.cfg.bot_login)]
    else:
        replies = run_commands(ctx, [c for c in found if c.line in fresh], thread, user, level,
                               ask=ask)
    if ask.recorded:
        ctx.gh.react(comment_id, asks.QUEUED, review_comment=review_comment)
    login = user.get("login", "")
    _reply(ctx, int(thread["number"]), f"@{login}\n\n" + "\n\n".join(replies))
    ctx.gh.react(comment_id, ANSWERED, review_comment=review_comment)
    return replies


def run_commands(ctx: Context, found: list[Command], thread: dict[str, Any], user: dict[str, Any],
                 level: int, *, ask: Ask | None = None) -> list[str]:
    """Run each command and return its reply. `ask` is the comment they came from: whatever
    queues model work records it, so the later stages can react to it (`asks`)."""
    replies: list[str] = []
    for command in found:
        head = f"> {command.line}\n\n" if len(found) > 1 else ""
        if command.verb == "typo":
            word = command.args.split()[0] if command.args.split() else command.args
            replies.append(head + f"`{word}` is not a command; did you mean `{command.meant}`? "
                           "I did nothing. Say it again spelt that way, or with more words for a "
                           "request. " + commands.POINTER.format(bot=ctx.cfg.bot_login))
            continue
        if level < command.level:
            need = LEVEL_NAMES.get(command.level, str(command.level))
            have = LEVEL_NAMES.get(level, str(level))
            flag = " with `--force`" if command.force else ""
            replies.append(head + f"`{command.verb}`{flag} needs the {need} level; you have "
                           f"{have}. Nothing was done.")
            continue
        try:
            replies.append(head + execute(ctx, command, thread, user, ask))
        except Exception as exc:  # noqa: BLE001 - a failure is answered, never dropped in silence
            replies.append(head + f"That failed ({type(exc).__name__}: {str(exc)[:300]}). "
                           "Nothing is lost by saying it again.")
    return replies


def execute(ctx: Context, command: Command, thread: dict[str, Any], user: dict[str, Any],
            ask: Ask | None = None) -> str:
    number = int(thread.get("number") or 0)
    is_pr = "pull_request" in thread
    by = str(user.get("login", ""))
    verb = command.verb
    if verb == "status":
        return report(ctx) + "\n\n" + commands.POINTER.format(bot=ctx.cfg.bot_login)
    if verb == "help":
        return commands.help_text(ctx.cfg.bot_login, command.args)
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
            return "Started. " + _run_now(ctx, None) + extra
        return "Started. The bot works in the next night window." + extra
    if verb == "run":
        target = command.args.lstrip("#")
        return _run_now(ctx, int(target) if target.isdigit() else None)
    if verb == "suggest":
        def wanted(state: dict[str, Any]) -> None:
            state["suggest"].update(requested=True)
            asks.note(state["suggest"], ask)
        ctx.store.update(wanted, "suggest")
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
    if verb in ("build", "revise", "request"):
        if is_pr:
            return queue_revise(ctx, number, by=by, force=command.force, ask=ask) + notes_line
        return queue_build(ctx, number, by=by, force=command.force, ask=ask) + notes_line
    return f"`{verb}` is not something I can do here."


def _run_now(ctx: Context, item: int | None) -> str:
    """Ask for a run now. The request is written down first, so a run that GitHub replaces or
    refuses to start is made up by the next hourly one."""
    ctx.store.update(lambda s: s.update(run_requested={"at": iso(ctx.now()), "item": item}),
                     "run requested")
    which = f" for #{item}" if item else ""
    try:
        ctx.dispatch(force=True, item=item)
    except GitHubError as exc:
        return (f"Starting a run{which} failed ({str(exc)[:200]}); the next hourly run does it "
                "instead.")
    return f"A run{which} is starting now, outside the window if need be."


def on_review(ctx: Context, payload: dict[str, Any]) -> list[str]:
    """A review: its body's commands, a hint when it names the bot, or a revision when a trusted
    person asks a bot pull request for changes. Claimed like a comment, so it runs once."""
    review = payload.get("review") or {}
    pull = payload.get("pull_request") or {}
    user = review.get("user") or {}
    level = _level(ctx, user, review.get("author_association"))
    if level <= 0:
        return [f"ignored: @{user.get('login')} is not on the trust list"]
    body = str(review.get("body") or "")
    found = commands.parse(body, ctx.cfg.bot_login)
    changes = (LABEL_PR in label_names(pull) and level >= 2
               and str(review.get("state", "")).lower() == "changes_requested")
    nudge = not found and not changes and commands.names_the_bot(body, ctx.cfg.bot_login)
    if not (found or changes or nudge):
        return ["no command in the review"]
    lines = [c.line for c in found] or ["(changes requested)" if changes else "(named the bot)"]
    if not claim(ctx, f"r:{review.get('id')}", lines):
        return ["ignored: already answered"]
    number = int(pull["number"])
    login = user.get("login", "")
    if found:
        replies = run_commands(ctx, found, {"number": number, "pull_request": {}}, user, level)
    elif changes:
        replies = [queue_revise(ctx, number, by=str(login))]
    else:
        replies = [NUDGE.format(bot=ctx.cfg.bot_login)]
    _reply(ctx, number, f"@{login}\n\n" + "\n\n".join(replies))
    return replies


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
        # Either queue label works on either kind of thread: an issue builds, a PR revises.
        wanted = name in (LABEL_BUILD, LABEL_REVISE)
        if not wanted:
            return [f"ignored: label {name}"]
    elif action == "assigned":
        assignee = str((payload.get("assignee") or {}).get("login", ""))
        wanted = assignee.lower() == ctx.cfg.bot_login.lower()
        if not wanted:
            return [f"ignored: assigned to {assignee}"]
    if level < 2:
        if action == "labeled":
            ctx.gh.remove_label(number, str((payload.get("label") or {}).get("name", "")))
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
    # "Closes #N" in the body should do this, but GitHub doesn't always: #43 merged and left #39 open.
    if ((pull.get("base") or {}).get("ref") == ctx.cfg.default_branch
            and ctx.gh.get_issue(issue).get("state") == "open"):
        ctx.gh.update_issue(issue, state="closed", state_reason="completed")
        return [f"#{number} merged; closed #{issue}"]
    return [f"#{number} merged; #{issue} closes with it"]


def on_ci(ctx: Context, payload: dict[str, Any]) -> list[str]:
    run = payload.get("workflow_run") or {}
    if run.get("name") not in ci_workflows(ctx):
        return [f"ignored: workflow {run.get('name')}"]
    if run.get("conclusion") not in CI_FAILED:
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
            ctx.act.rerun_failed_jobs(run["id"])
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
