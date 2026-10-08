"""GitHub events, handled at once with no model call: commands, labels, assignment, CI results.

Runs in `bot-commands.yml`. Commands are acknowledged with a reaction and answered in one reply.
Model work is only ever queued here; it happens in the night run, or at once when an operator adds
`--force`.
"""

from __future__ import annotations

import re
from typing import Any

from datetime import timedelta

from harness import asks, commands, stepup
from harness import providers as providers_mod

from harness.asks import Ask
from harness.clock import iso, parse_iso
from harness.commands import Command
from harness.config import (IDENTITY, LABEL_BLOCKED, LABEL_BUILD, LABEL_PR, LABEL_PR_OPEN,
                            LABEL_REVISE, LABEL_TREE, LABEL_WORKING, MARKER, MODES, MODE_LABELS,
                            OTHERS, SLASH)
from harness.context import Context
from harness.errors import GitHubError
from harness.providers import Provider
from harness.queue import (label_names, queue_build, queue_review, queue_revise, set_state_label,
                           stop, wip_branch)
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


#: What one bot may ask of another in a comment (#60): Squishy stops the sub-issues of a tree it
#: split that are queued for the night bot.
OTHER_BOT_VERBS = ("stop",)


def from_other_bot(user: dict[str, Any]) -> bool:
    """The comment's author is another bot in the repository (`config.OTHERS`)."""
    login = str(user.get("login") or "").lower()
    return any(other.login.lower() == login for other in OTHERS)


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
         "`@{bot} <what you want>` or `" + SLASH + " build`; `" + SLASH + " help` lists the rest.")


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
    if from_other_bot(comment.get("user") or {}):
        # The bots name each other in their replies (#60): only a bare `stop`, which Squishy
        # sends to stop a tree it split, is acted on, and nothing is answered with a hint, so
        # two bots never talk to each other for ever.
        found = [c for c in found if c.verb in OTHER_BOT_VERBS and not c.args]
        nudge = False
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
                           "request. " + commands.pointer(ctx.cfg.bot_login))
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
        return report(ctx) + "\n\n" + commands.pointer(ctx.cfg.bot_login)
    if verb == "help":
        return commands.help_text(ctx.cfg.bot_login, command.args)
    if verb == "halt":
        reason = command.args or "no reason given"
        ctx.store.update(lambda s: s.update(halted=True, halt={
            "by": by, "at": iso(ctx.now()), "reason": reason}), "halt")
        return (f"Halted. No new model work starts until `{SLASH} start`; a run already going "
                "stops at its next checkpoint and keeps its work.")
    if verb == "start":
        # `resume <subscription>` (or `start <subscription>`) lifts that one's suspension; any
        # other words after `start` are a note, and it lifts the halt as it always did.
        if command.args and (_said_resume(command)
                             or _subscription(ctx, command.args.split()[0])[0] is not None):
            return _resume(ctx, command)
        ctx.store.update(lambda s: s.update(halted=False, halt={}), "start")
        extra = ""
        if ctx.repo_halted():
            extra = " `.harness/HALT` is still on `main`, though, and it wins until it is deleted."
        if command.force:
            return "Started. " + _run_now(ctx, None) + extra
        return "Started. The bot takes work again as soon as a subscription is free." + extra
    if verb == "suspend":
        word, _, rest = command.args.partition(" ")
        if not word:
            return (f"`suspend` needs a subscription, as in `{SLASH} suspend claude-3 using it "
                    "myself`. Nothing was done.")
        provider, problem = _subscription(ctx, word)
        if provider is None:
            return problem
        held = {"by": by, "at": iso(ctx.now()), "reason": rest.strip()}
        ctx.store.update(lambda s: providers_mod.record(s, provider.id).update(suspended=held),
                         f"suspend {provider.id}")
        return (f"Suspended `{provider.id}`. No new work starts on it until "
                f"`{SLASH} resume {provider.id}`; a run already going on it stops at its next "
                "checkpoint, keeps its work and goes back to the queue for another subscription.")
    if verb == "run":
        target = command.args.lstrip("#")
        return _run_now(ctx, int(target) if target.isdigit() else None)
    if verb in ("suggest", "oneshot", "split", "fullsend") and not commands.offered(verb):
        return _not_mine(verb)
    if verb == "suggest":
        def wanted(state: dict[str, Any]) -> None:
            state["suggest"].update(requested=True)
            asks.note(state["suggest"], ask)
        ctx.store.update(wanted, "suggest")
        if command.force:
            ctx.dispatch(force=True, mode="suggest")
            return "A suggestion survey is starting now."
        return ("I will survey for suggestions the next time the queue is empty and a "
                "subscription is free.")
    if not number:
        return f"`{verb}` needs an issue or a pull request to act on."
    if verb == "stop":
        reply = stop(ctx, number, by=by)
        if LABEL_TREE in label_names(ctx.gh.get_issue(number)):
            reply += " " + stop_tree(ctx, number, by=by)
        return reply
    if verb == "rebuild":
        return stepup.rebuild_request(ctx, number, by)
    if verb == "review":
        if not is_pr:
            return ("`review` asks for a review run of one of my pull requests; on an issue, "
                    f"`{SLASH} build` queues it.")
        first, _, rest = command.args.partition(" ")
        floor = first.lower() if first.lower() in ("strong", "medium") else ""
        notes = rest.strip() if floor else command.args.strip()
        return queue_review(ctx, number, by=by, floor=floor, notes=notes, ask=ask)
    notes_line = ""
    if command.args:
        notes_line = " I will read your notes with the rest of the thread."
    if verb in ("oneshot", "split", "fullsend"):
        if is_pr:
            return f"`{verb}` works on an issue; on a pull request, ask for a revision."
        first, _, rest = command.args.partition(" ")
        mode = verb
        if verb == "split" and first.lower() == "bot":
            mode = "split-bot"
            notes_line = " I will read your notes with the rest of the thread." if rest else ""
        if mode not in MODES:
            return _not_mine(mode)
        return queue_build(ctx, number, by=by, force=command.force, ask=ask,
                           mode=mode) + notes_line
    if verb in ("build", "revise", "request"):
        if is_pr:
            return queue_revise(ctx, number, by=by, force=command.force, ask=ask) + notes_line
        # An explicit `build` is a plain build: a mode the issue was queued in comes off.
        mode = "" if verb == "build" and MODES else None
        return queue_build(ctx, number, by=by, force=command.force, ask=ask,
                           mode=mode) + notes_line
    return f"`{verb}` is not something I can do here."


def _not_mine(verb: str) -> str:
    """The reply to a verb this bot does not have, naming the bot that does (#60)."""
    if verb == "suggest":
        return f"{IDENTITY.title} makes no suggestions."
    names = ", ".join(f"{o.name} (@{o.login})" for o in OTHERS)
    hint = f" {names} may: ask there." if names else ""
    return f"I have no `{verb}` mode.{hint}"


def stop_tree(ctx: Context, parent: int, *, by: str) -> str:
    """`stop` on a parent the bot split (#60): its open sub-issues leave the queue too. Its own
    are stopped here; one queued for another bot gets that bot's own `stop` command, so it stops
    its work the way it always does."""
    stopped: list[str] = []
    try:
        children = ctx.gh.list_sub_issues(parent)
    except GitHubError as exc:
        return f"Its sub-issues could not be read ({str(exc)[:200]}), so they are still queued."
    for child in children:
        if child.get("state") != "open":
            continue
        number = int(child["number"])
        other = None
        names = {n.lower() for n in label_names(child)}
        for candidate in OTHERS:
            if any(n.startswith(candidate.label_prefix) for n in names):
                other = candidate
        if other is None:
            stop(ctx, number, by=by)
        else:
            ctx.gh.create_comment(number, f"@{other.login} stop\n\n{by} stopped the tree this "
                                  f"belongs to (#{parent}).")
        stopped.append(f"#{number}")
    if not stopped:
        return "It has no open sub-issues."
    return f"Its open sub-issues leave the queue too: {', '.join(stopped)}."


def _subscription(ctx: Context, word: str) -> tuple[Provider | None, str]:
    """The subscription a `suspend` or `resume` names, or None and the reply saying why not."""
    name = word.strip("`:,.!?").lower()
    provider = ctx.cfg.pool.get(name)
    if provider is not None:
        return provider, ""
    known = ", ".join(f"`{p.id}`" for p in ctx.cfg.pool.ordered())
    return None, f"`{name}` is not a subscription; I know {known}. Nothing was done."


def _said_resume(command: Command) -> bool:
    """The line's verb was `resume`, the alias that always names a subscription."""
    return re.search(r"(?:^|[\s/])resume\b", command.line.lower()) is not None


def _resume(ctx: Context, command: Command) -> str:
    """`start <subscription>` (`resume claude-3`): lift that one's suspension, and no halt."""
    provider, problem = _subscription(ctx, command.args.split()[0])
    if provider is None:
        return problem + f" `{SLASH} start` alone lifts a halt."
    if providers_mod.suspension(ctx.store.load(), provider.id) is None:
        return f"`{provider.id}` is not suspended, so there was nothing to lift."
    ctx.store.update(lambda s: providers_mod.record(s, provider.id).pop("suspended", None),
                     f"resume {provider.id}")
    reply = f"Resumed `{provider.id}`: the bot spends it again as soon as it is free."
    if ctx.store.load().get("halted"):
        reply += f" The bot is still halted, though: `{SLASH} start` lifts that."
    if command.force:
        reply += " " + _run_now(ctx, None)
    return reply


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
    return f"A run{which} is starting now, outside a subscription's hours if need be."


def on_review(ctx: Context, payload: dict[str, Any]) -> list[str]:
    """A review: its body's commands, a hint when it names the bot, or a revision when a trusted
    person asks a bot pull request for changes. Claimed like a comment, so it runs once."""
    review = payload.get("review") or {}
    pull = payload.get("pull_request") or {}
    user = review.get("user") or {}
    level = _level(ctx, user, review.get("author_association"))
    if level <= 0:
        return [f"ignored: @{user.get('login')} is not on the trust list"]
    if from_other_bot(user):
        return ["ignored: another bot's review"]
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
    mode: str | None = None
    if action == "labeled":
        name = str((payload.get("label") or {}).get("name", ""))
        # Either queue label works on either kind of thread: an issue builds, a PR revises. A
        # mode's label (#60) queues the issue in that mode.
        wanted = name in (LABEL_BUILD, LABEL_REVISE) or (name in MODE_LABELS and not is_pr)
        if not wanted:
            return [f"ignored: label {name}"]
        mode = MODE_LABELS.get(name)
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
        reply = queue_build(ctx, number, by=by, label_present=action == "labeled", mode=mode)
    ctx.gh.create_comment(number, f"@{by} {reply}")
    return [reply]


def _drop_wip(ctx: Context, number: int) -> None:
    """A closed or merged bot pull request's unfinished revision (`bot/wip/<pr>`, #313) is done
    with: delete the branch and drop the record's `wip`."""
    if not ctx.store.load()["items"].get(str(number), {}).get("wip"):
        return
    try:
        ctx.gh.delete_branch(wip_branch(number))
    except GitHubError:
        pass  # gone already
    ctx.store.update(lambda s: state_item(s, number).pop("wip", None), f"wip #{number} closed")


def on_pull_closed(ctx: Context, payload: dict[str, Any]) -> list[str]:
    pull = payload.get("pull_request") or {}
    if LABEL_PR not in label_names(pull):
        return ["ignored: not a bot pull request"]
    issue = threads.linked_issue(pull)
    number = int(pull["number"])
    ctx.store.update(lambda s: state_item(s, number).update(closed_at=iso(ctx.now()),
                                                            merged=bool(pull.get("merged"))),
                     f"closed #{number}")
    _drop_wip(ctx, number)
    if pull.get("merged"):
        try:
            stepup.clear(ctx, number)  # merged: approved and green, so its strikes are over
        except GitHubError:
            pass
    if issue is None:
        return ["closed; no linked issue"]
    ctx.gh.remove_label(issue, LABEL_PR_OPEN)
    if ctx.store.load()["items"].get(str(number), {}).get("rebuilt"):
        return [f"#{number} closed to build #{issue} again"]  # `stepup.rebuild` said why
    if not pull.get("merged"):
        ctx.gh.create_comment(issue, f"#{number} was closed without merging. Comment "
                              f"`{SLASH} build` to have me try again.")
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
        if fixes and str(record.get("ci_fixed_head") or "") == sha:
            # Every CI fix that left it red is a strike of its own (#316), not only the last one:
            # this is the head the last CI fix pushed (`deliver._revise`). A head some other
            # revision made is no strike, whatever fixes came before.
            try:
                stepped = stepup.strike(ctx, number, f"CI still failed after {fixes} fix(es)",
                                        link=link)
            except GitHubError:
                stepped = False
            if stepped:
                out.append(f"#{number}: stepped up after {fixes} CI fixes")
                continue
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
