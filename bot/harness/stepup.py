"""An item's own failures, and what three of them lead to (#317 parts 4 and 8).

A strike is a run that failed for the item's own sake: a run that failed, a build or revision its
reviewer would not approve, a review run that rejected its head, CI still red after its fixes, a
run that died. Infra, a usage pause, a halt or a stop is no strike. Strikes count on the issue (a
bot pull request's count on the issue it closes), and nothing a person asks resets them: only a
head that meets the review rule (`clear`), or a step up, does.

At `STEP_UP_AFTER` (3) strikes the bot raises the item's difficulty one step, easy to medium to
hard, so a stronger model plans and builds it, and rebuilds its open pull request from `main`
rather than revising it again (`rebuild`): #141, #203 and #214 were revised for days, Devin
committing conflict markers each time, until a person closed them. A difficulty a person set is
never replaced: the bot asks instead, and so it does at hard, where there is no step left.

`/harness rebuild` rebuilds at once, at the same difficulty.
"""

from __future__ import annotations

from typing import Any

from harness import threads
from harness.clock import iso
from harness.config import (DIFFICULTIES, DIFFICULTY_LABELS, LABEL_BLOCKED, LABEL_BUILD, LABEL_PR,
                            LABEL_PR_OPEN, LABEL_REVISE, STEP_UP_AFTER)
from harness.context import Context
from harness.errors import GitHubError
from harness.queue import (branch_for_issue, carried_difficulty, difficulty_of, is_human,
                           label_names, open_pull_for_branch, rating_source, set_state_label,
                           wip_branch)
from harness.state import item as state_item

#: How many strike reasons an item keeps, for the comment that says why it stepped up.
STRIKE_LOG = 10


def _try(action: Any) -> None:
    try:
        action()
    except GitHubError:
        pass


def pair(ctx: Context, number: int) -> tuple[int, int | None]:
    """`(the thread strikes count on, its open bot pull request or None)` for an issue or a pull
    request: a bot pull request's issue (its record's `issue`, else the one it says it closes),
    or the pull request itself when it closes none."""
    thread = ctx.gh.get_issue(number)
    if "pull_request" not in thread:
        pull = open_pull_for_branch(ctx, branch_for_issue(number))
        return number, (int(pull["number"]) if pull else None)
    record = ctx.store.load()["items"].get(str(number), {})
    issue = int(record.get("issue") or 0) or threads.linked_issue(thread) or 0
    open_pr = number if thread.get("state") == "open" else None
    return (issue or number), open_pr


def strike(ctx: Context, number: int, why: str, *, link: str = "") -> bool:
    """Count one failure of the item's own. At `STEP_UP_AFTER` it steps the item up (`step_up`)
    and returns True: the caller's own ending (a block, a requeue) is then replaced by it."""
    target, pr = pair(ctx, number)

    def change(state: dict[str, Any]) -> None:
        entry = state_item(state, target)
        entry["strikes"] = int(entry.get("strikes") or 0) + 1
        entry["strike_log"] = [*(entry.get("strike_log") or []), str(why)[:200]][-STRIKE_LOG:]
    state = ctx.store.update(change, f"strike #{target}")
    entry = state_item(state, target)
    count = int(entry.get("strikes") or 0)
    if count < STEP_UP_AFTER:
        return False
    step_up(ctx, target, pr, reasons=list(entry.get("strike_log") or [])[-count:], link=link)
    return True


def clear(ctx: Context, number: int) -> None:
    """A head met the review rule: the item's strikes are over."""
    target, _ = pair(ctx, number)
    ctx.store.update(lambda s: state_item(s, target).update(strikes=0, strike_log=[]),
                     f"strikes #{target} cleared")


def _reasons(reasons: list[str]) -> str:
    return "; ".join(dict.fromkeys(r for r in reasons if r)) or "no reason recorded"


def step_up(ctx: Context, target: int, pr: int | None, *, reasons: list[str],
            link: str = "") -> str:
    """Raise `target`'s difficulty one step and build it again (`rebuild` its open pull request),
    or block it for a person when a person set its difficulty or it is hard already. Returns what
    it did, in words."""
    thread = ctx.gh.get_issue(target)
    names = label_names(thread)
    record = ctx.store.load()["items"].get(str(target), {})
    current = difficulty_of(names, carried_difficulty(record))
    ctx.store.update(lambda s: state_item(s, target).update(strikes=0, strike_log=[]),
                     f"step up #{target}")
    why = _reasons(reasons)
    where = f" ({link})" if link else ""
    if current == DIFFICULTIES[-1] or rating_source(names, record) == "person":
        for number in {target, *([pr] if pr else [])}:
            _try(lambda n=number: set_state_label(ctx, n, label_names(ctx.gh.get_issue(n)),
                                                   LABEL_BLOCKED))
        if current == DIFFICULTIES[-1]:
            ask = ("It is `difficulty:hard` already, so there is no stronger model to hand it to: "
                   "it needs a person. Split it, or answer what blocks it, then `/harness build`.")
        else:
            nxt = DIFFICULTIES[DIFFICULTIES.index(current) + 1]
            ask = (f"A person set `difficulty:{current}`, so I do not change it: should it be "
                   f"`difficulty:{nxt}`? Relabel it if so, then `/harness build` (or "
                   "`/harness rebuild` on its pull request) tries again.")
        _try(lambda: ctx.gh.create_comment(target, f"This failed {STEP_UP_AFTER} times{where}: "
                                           f"{why}. {ask}"))
        return f"blocked #{target}"
    nxt = DIFFICULTIES[DIFFICULTIES.index(current) + 1]
    wanted = f"difficulty:{nxt}"
    for number in {target, *([pr] if pr else [])}:
        labels = label_names(ctx.gh.get_issue(number))
        for name in labels:
            if name.lower() in DIFFICULTY_LABELS and name != wanted:
                _try(lambda n=number, m=name: ctx.gh.remove_label(n, m))
        if wanted not in labels:
            _try(lambda n=number: ctx.gh.add_labels(n, [wanted]))
    now = iso(ctx.now())
    ctx.store.update(lambda s: state_item(s, target).update(difficulty_by={
        "difficulty": nxt, "provider": "step-up", "tier": "", "at": now,
        "why": f"failed {STEP_UP_AFTER} times at difficulty:{current}"}), f"raised #{target}")
    if pr:
        ctx.store.update(lambda s: state_item(s, pr).update(difficulty=nxt), f"raised #{pr}")
    said = (f"This failed {STEP_UP_AFTER} times at `difficulty:{current}`{where}: {why}. Raised to "
            f"`{wanted}`, so a model of that tier plans and builds it")
    if pr and target != pr:
        rebuild(ctx, target, pr, why=f"it failed {STEP_UP_AFTER} times at "
                f"`difficulty:{current}` ({why}); it was raised to `{wanted}`", said=said)
        return f"raised #{target} to {nxt} and rebuilt #{pr}"
    if "pull_request" in thread:
        # A pull request that closes no issue: revise it again, at the new difficulty.
        _try(lambda: set_state_label(ctx, target, label_names(ctx.gh.get_issue(target)),
                                     LABEL_REVISE))
    else:
        _try(lambda: set_state_label(ctx, target, label_names(ctx.gh.get_issue(target)),
                                     LABEL_BUILD))
        ctx.store.update(lambda s: state_item(s, target).update(
            kind="build", queued_at=now, failures=0, interruptions=0, died=0), f"requeue #{target}")
    _try(lambda: ctx.gh.create_comment(target, f"{said}; it is queued again."))
    return f"raised #{target} to {nxt}"


def rebuild(ctx: Context, issue: int, pr: int, *, why: str, said: str = "") -> str:
    """Close bot pull request `pr`, keep its branch as `bot/old/issue-<n>-<date>`, and queue
    `issue` to build again from `main`, its plan and last findings kept for the new build, which is
    told about the old pull request (#317 part 4). Returns the line for the comment."""
    pull = ctx.gh.get_pull(pr)
    head = pull.get("head") or {}
    branch, sha = str(head.get("ref") or ""), str(head.get("sha") or "")
    ctx.store.update(lambda s: state_item(s, pr).update(rebuilt=True), f"rebuild #{pr}")
    if pull.get("state") == "open":
        if pull.get("auto_merge") and pull.get("node_id"):
            _try(lambda: ctx.gh.disable_auto_merge(pull["node_id"]))
        _try(lambda: set_state_label(ctx, pr, label_names(pull), None))
        _try(lambda: ctx.gh.update_pull(pr, state="closed"))
    old = ""
    if branch == branch_for_issue(issue) and sha:
        old = f"bot/old/issue-{issue}-{ctx.now().strftime('%Y%m%d-%H%M')}"
        try:
            ctx.gh.create_branch(old, sha)
            ctx.gh.delete_branch(branch)
        except GitHubError:
            old = ""
    _try(lambda: ctx.gh.delete_branch(wip_branch(pr)))
    now = iso(ctx.now())

    def change(state: dict[str, Any]) -> None:
        entry = state_item(state, issue)
        for key in ("pr", "handoff", "wip"):
            entry.pop(key, None)
        entry.update(kind="build", queued_at=now, failures=0, interruptions=0, died=0, strikes=0,
                     strike_log=[], stop_requested=False, previous_pr=pr, previous_branch=old,
                     previous_why=why[:500])
        state_item(state, pr).pop("wip", None)
    ctx.store.update(change, f"rebuild #{issue}")
    labels = label_names(ctx.gh.get_issue(issue))
    if LABEL_PR_OPEN in labels:
        _try(lambda: ctx.gh.remove_label(issue, LABEL_PR_OPEN))
    _try(lambda: set_state_label(ctx, issue, label_names(ctx.gh.get_issue(issue)), LABEL_BUILD))
    kept = f" Its branch is kept as `{old}`." if old else ""
    _try(lambda: ctx.gh.create_comment(pr, f"Closed to build #{issue} again from `main`: {why}."
                                       f"{kept}"))
    lead = f"{said}. " if said else ""
    line = (f"{lead}#{pr} is closed and #{issue} is queued to build again from `main`.{kept} The "
            "new build is told what went wrong with it.")
    _try(lambda: ctx.gh.create_comment(issue, line))
    return line


def rebuild_request(ctx: Context, number: int, by: str) -> str:
    """`/harness rebuild` on a bot pull request or its issue: rebuild at once, at the same
    difficulty. Returns the reply."""
    thread = ctx.gh.get_issue(number)
    if is_human(label_names(thread)):
        return (f"#{number} is labelled `human`: people do it, so I leave it alone. Take that "
                "label off to hand it to me.")
    if "pull_request" in thread:
        if LABEL_PR not in label_names(thread):
            return f"#{number} is not a pull request I opened, so I will not close it."
        issue, _ = pair(ctx, number)
        if issue == number:
            return f"#{number} closes no issue, so there is nothing to build again from `main`."
        pr = number
    else:
        issue, pr = number, None
        found = open_pull_for_branch(ctx, branch_for_issue(number))
        pr = int(found["number"]) if found else None
        if pr is None:
            return (f"#{number} has no open pull request of mine to rebuild; `/harness build` "
                    "queues it.")
    rebuild(ctx, issue, pr, why=f"@{by} asked for it (`/harness rebuild`)")
    return f"Closed #{pr} and queued #{issue} to build again from `main`."
