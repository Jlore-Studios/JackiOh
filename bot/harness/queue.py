"""The work queue, kept as labels on issues and pull requests plus a record in the state file.

`bot:build` on an issue and `bot:revise` on a pull request mean queued; `bot:cross-review` on a
bot pull request means it waits for a review run; `bot:working` means a run holds it;
`bot:blocked` means it waits for a person. Only one `bot:` state label is on a thread at a time,
apart from `bot:pr` and `bot:pr-open`, which say what a thread is.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Any

from harness import asks, issueplan, review_rule
from harness import providers as providers_mod
from harness.asks import Ask
from harness.clock import iso
from harness.config import (DEFAULT_DIFFICULTY, DIFFICULTIES, DIFFICULTY_LABELS, LABEL_BLOCKED,
                            LABEL_BUILD, LABEL_CROSS, LABEL_HUMAN, LABEL_PR, LABEL_PR_OPEN,
                            LABEL_PRIORITY_HIGH, LABEL_PRIORITY_LOW, LABEL_PRIORITY_MEDIUM,
                            LABEL_READY, LABEL_REVISE, LABEL_WORKING, PLAN_FLOOR,
                            UNRATED_PLAN_FLOOR)
from harness.context import Context
from harness.errors import GitHubError
from harness.state import item as state_item

STATE_LABELS = (LABEL_BUILD, LABEL_REVISE, LABEL_CROSS, LABEL_WORKING, LABEL_BLOCKED)


def label_names(thread: dict[str, Any]) -> set[str]:
    return {str(label.get("name")) for label in thread.get("labels", []) if isinstance(label, dict)}


def set_state_label(ctx: Context, number: int, current: set[str], wanted: str | None) -> None:
    """Leave exactly `wanted` (or none) of the queue's state labels on the thread. A pull request
    back in the queue is no longer ready for a person to merge, so `ready for merge` comes off."""
    for name in STATE_LABELS:
        if name in current and name != wanted:
            ctx.gh.remove_label(number, name)
    if wanted and LABEL_READY in current:
        ctx.gh.remove_label(number, LABEL_READY)
    if wanted and wanted not in current:
        ctx.gh.add_labels(number, [wanted])


def branch_for_issue(number: int) -> str:
    return f"bot/issue-{int(number)}"


def open_pull_for_branch(ctx: Context, branch: str) -> dict[str, Any] | None:
    pulls = ctx.gh.list_pulls(state="open", head=branch)
    return pulls[0] if pulls else None


def _when(ctx: Context, state: dict[str, Any]) -> str:
    return providers_mod.when_free(ctx.cfg.pool, state, ctx.now(), ctx.cfg.timezone,
                                   ctx.cfg.secrets)


def _halt_note(ctx: Context, state: dict[str, Any]) -> str:
    return " The bot is halted, though: `/harness start` resumes it." if state.get("halted") else ""


def is_human(names: set[str]) -> bool:
    """Labelled `human` (whatever its case): people do it, and the bot leaves it alone."""
    return LABEL_HUMAN in {name.lower() for name in names}


def human_reply(number: int) -> str:
    return (f"#{number} is labelled `{LABEL_HUMAN}`: people do it, so I leave it alone. Take that "
            "label off to hand it to me.")


def _label_note(names: set[str]) -> str:
    """What `difficulty:hard` on the thread changes about who takes it (`human` refuses earlier)."""
    if difficulty_of(names) == "hard":
        return " It is labelled `difficulty:hard`, so only Opus plans, builds and reviews it."
    return ""


def difficulty_of(names: set[str], carried: str = "") -> str:
    """The thread's difficulty: the hardest `difficulty:*` label on it (whatever its case) or
    `carried` (what its issue had, kept on a bot PR's record, or a floor the bot raised it to),
    and `medium` with neither."""
    found = [DIFFICULTY_LABELS[name.lower()] for name in names
             if name.lower() in DIFFICULTY_LABELS]
    if carried in DIFFICULTIES:
        found.append(carried)
    return max(found, key=DIFFICULTIES.index, default=DEFAULT_DIFFICULTY)


def labelled_difficulty(names: set[str]) -> str:
    """The hardest `difficulty:*` label on the thread, or "" for none."""
    found = [DIFFICULTY_LABELS[name.lower()] for name in names
             if name.lower() in DIFFICULTY_LABELS]
    return max(found, key=DIFFICULTIES.index, default="")


def carried_difficulty(record: dict[str, Any]) -> str:
    """What the state file carries for a thread's difficulty: a bot pull request's issue's
    (`difficulty`), and a floor the bot raised it to (`difficulty_floor`: the easy rule broken,
    or three failures under a person's label, #317 part 8), the harder of the two."""
    found = [str(record.get(key) or "") for key in ("difficulty", "difficulty_floor")]
    found = [d for d in found if d in DIFFICULTIES]
    return max(found, key=DIFFICULTIES.index, default="")


def rated(names: set[str], record: dict[str, Any]) -> bool:
    """Someone has said how hard it is: a `difficulty:*` label, the bot's own rating
    (`difficulty_by`), or a difficulty its record carries. An unrated item counts as medium for
    routing, and a medium or strong model rates it when it plans it (#317 part 8)."""
    return bool(labelled_difficulty(names) or record.get("difficulty_by")
                or carried_difficulty(record))


def queue_build(ctx: Context, number: int, *, by: str, force: bool = False,
                label_present: bool = False, ask: Ask | None = None) -> str:
    """Queue an issue for building. Returns the reply line. `ask` is the comment that asked, kept
    on the record that ends up queued so later stages can react to it (`asks`)."""
    issue = ctx.gh.get_issue(number)
    if "pull_request" in issue:
        return queue_revise(ctx, number, by=by, force=force, label_present=label_present, ask=ask)
    if issue.get("state") != "open":
        return f"#{number} is closed, so there is nothing to build. Reopen it first."
    names = label_names(issue)
    if is_human(names):
        return human_reply(number)
    if LABEL_WORKING in names:
        _pending(ctx, number, by, ask)
        return (f"I am working on #{number} right now. When this run ends I go round once more "
                "with your comment.")
    if LABEL_PR_OPEN in names:
        pull = open_pull_for_branch(ctx, branch_for_issue(number))
        if pull is not None:
            set_state_label(ctx, number, names, None)
            return queue_revise(ctx, int(pull["number"]), by=by, force=force, ask=ask)
    set_state_label(ctx, number, names, LABEL_BUILD)
    state = ctx.store.update(lambda s: _queued(s, number, "build", by, force, ctx, ask),
                             f"queue #{number}")
    if force:
        return (_start_now(ctx, number, "build", f"Queued #{number}") + _halt_note(ctx, state)
                + _label_note(names))
    return (f"Queued #{number}; I will build it {_when(ctx, state)}.{_halt_note(ctx, state)}"
            f"{_label_note(names)}")


def queue_revise(ctx: Context, number: int, *, by: str, force: bool = False, source: str = "request",
                 label_present: bool = False, extra: dict[str, Any] | None = None,
                 ask: Ask | None = None) -> str:
    """Queue a pull request for a revision. Returns the reply line."""
    pull = ctx.gh.get_pull(number)
    if pull.get("state") != "open":
        return f"#{number} is not open, so I will not change it."
    if is_human(label_names(pull)):
        return human_reply(number)
    head_repo = ((pull.get("head") or {}).get("repo") or {}).get("full_name")
    if head_repo != ctx.cfg.repo:
        return f"#{number} comes from a fork; I can only push to branches in {ctx.cfg.repo}."
    names = label_names(pull)
    if LABEL_WORKING in names:
        _pending(ctx, number, by, ask)
        return (f"I am revising #{number} right now. When this run ends I go round once more "
                "with your comment.")
    set_state_label(ctx, number, names, LABEL_REVISE)
    if LABEL_PR in names and pull.get("auto_merge"):
        try:
            ctx.gh.disable_auto_merge(pull["node_id"])
        except GitHubError:
            pass
    def change(state: dict[str, Any]) -> None:
        _queued(state, number, "revise", by, force, ctx, ask)
        record = state_item(state, number)
        record["source"] = source
        record.update(extra or {})
    state = ctx.store.update(change, f"queue revise #{number}")
    held = " Auto-merge is off until the revision lands." if LABEL_PR in names else ""
    if force:
        return (_start_now(ctx, number, "revise", f"Queued a revision of #{number}") + held
                + _halt_note(ctx, state) + _label_note(names))
    return (f"Queued a revision of #{number}; I will do it {_when(ctx, state)}.{held}"
            f"{_halt_note(ctx, state)}{_label_note(names)}")


def _start_now(ctx: Context, number: int, mode: str, queued: str) -> str:
    """Start a forced run. If GitHub refuses, the item is still queued and still forced, and the
    next hourly run starts it, inside a subscription's hours or not."""
    try:
        ctx.dispatch(item=number, force=True, mode=mode)
    except GitHubError as exc:
        return (f"{queued}, but starting a run now failed ({str(exc)[:200]}). It stays forced, so "
                "the next hourly run starts it.")
    return f"{queued} and started a run now (`--force`)."


def _queued(state: dict[str, Any], number: int, kind: str, by: str, force: bool, ctx: Context,
            ask: Ask | None = None) -> None:
    """A fresh request: it clears a stop, the failure and interruption counts and any pending
    note, and keeps `ci_fixes`, which only a person's `forget` clears."""
    record = state_item(state, number)
    record.update(kind=kind, queued_at=iso(ctx.now()), requested_by=by, forced=bool(force),
                  stop_requested=False, failures=0, interruptions=0, pending_request=False)
    asks.note(record, ask)


def _pending(ctx: Context, number: int, by: str, ask: Ask | None = None) -> None:
    """Remember a request that arrived while a run held the thread; deliver requeues it."""
    def change(state: dict[str, Any]) -> None:
        record = state_item(state, number)
        record.update(pending_request=True, pending_by=by, pending_at=iso(ctx.now()))
        asks.note(record, ask)
    ctx.store.update(change, f"pending #{number}")


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
    dropped: list[str] = []
    def change(state: dict[str, Any]) -> None:
        record = state_item(state, number)
        record.update(stop_requested=True, stopped_by=by, stopped_at=iso(ctx.now()), forced=False,
                      pending_request=False)
        # Asks still waiting will never be answered; a run's own are settled when it stops.
        dropped[:] = asks.pop_waiting(record)
    ctx.store.update(change, f"stop #{number}")
    asks.react(ctx.gh, dropped, asks.NO_ANSWER)
    if working:
        return f"Stopping work on #{number}; the run gives up at its next checkpoint."
    return f"#{number} is out of the queue."


#: The pickup tiers by label (#90). A thread with none of them is in tier 2.
PRIORITY_TIERS = {LABEL_PRIORITY_HIGH: 0, LABEL_PRIORITY_MEDIUM: 1, LABEL_PRIORITY_LOW: 3}
NO_PRIORITY = 2
PRIORITY_NAMES = {0: "high", 1: "medium", NO_PRIORITY: "none", 3: "low"}


def priority_tier(names: set[str]) -> int:
    """The thread's pickup tier, the lowest picked first: `priority:high` 0, `priority:medium`
    1, none 2, `priority:low` 3. The highest label wins; names match whatever their case, and any
    other `priority:*` label counts as none."""
    return min((PRIORITY_TIERS[name.lower()] for name in names if name.lower() in PRIORITY_TIERS),
               default=NO_PRIORITY)


@dataclass
class Candidate:
    number: int
    kind: str  # "build" | "revise" | "review" (a review run on a bot PR)
    title: str
    forced: bool
    queued_at: str
    #: `easy`, `medium` or `hard` (`difficulty_of`): which models may plan, build and review it.
    difficulty: str = DEFAULT_DIFFICULTY
    #: For a review: the family of the model that built the change.
    builder: str = ""
    #: Its pickup tier (`priority_tier`), which comes before everything but `forced`.
    priority: int = NO_PRIORITY
    #: For a build: a planning session already wrote its plan (`plan.py` plans it first if not).
    planned: bool = False
    #: The tier of the model that wrote that plan ("" if none, or if not recorded).
    plan_tier: str = ""
    #: For a review: the families whose approval the head already has (for the reviewer's prompt).
    approved: tuple[str, ...] = ()
    #: For a review: the tier of each approval the head has, one per review (`review_rule`).
    approval_tiers: tuple[str, ...] = ()
    #: A pull request the bot opened (`bot:pr`): one a person opened never gets a review run, so
    #: its revision needs a reviewer in the same run.
    bot_pr: bool = False
    #: For a revision: it resolves a conflict with `main` on a change the review rule cleared
    #: (`cleared`), so the run's own reviewer can carry that clearance (`deliver._carry`).
    carries: bool = False
    #: Its labels: subscriptions with `only_labels` take only items carrying them.
    labels: tuple[str, ...] = ()
    #: Someone has rated how hard it is (`rated`); an unrated build is rated when it is planned.
    rated: bool = True
    #: A revision queued because `main` left the pull request with conflicts: only a builder
    #: with its own reviewer in the run takes it, never Devin (#317 part 2).
    conflict: bool = False


def cleared(record: dict[str, Any]) -> dict[str, Any]:
    """The last commit of a bot pull request that met the review rule, as deliver recorded it
    (`{"sha", "at", "by", ...}`), or `{}`. It is keyed by commit: a head that moved since is not
    cleared, whatever the record says."""
    found = record.get("cleared")
    return found if isinstance(found, dict) and found.get("sha") else {}


def carries(record: dict[str, Any]) -> bool:
    """A revision queued because `main` left a cleared change with conflicts."""
    return record.get("source") == "conflict" and bool(cleared(record))


def plan_of(record: dict[str, Any], thread: dict[str, Any], kind: str) -> dict[str, Any]:
    """Whether a build is planned, and by what tier: the bot's own record of its planning run,
    or else a Plan section someone put in the issue's description (`issueplan.START` … `END`),
    which counts as a strong plan: a person, or a session they ran, wrote it on purpose, and the
    bot must not plan over it."""
    if record.get("planned_at"):
        return {"planned": True, "plan_tier": str(record.get("planned_tier") or "")}
    if kind == "build" and issueplan.plan_of(thread.get("body")):
        return {"planned": True, "plan_tier": "strong"}
    return {"planned": False, "plan_tier": ""}


def plan_floor(candidate: Candidate) -> str:
    """The weakest tier whose plan this item builds from (`config.PLAN_FLOOR`): medium for an
    easy item, strong for anything harder, and medium to rate and plan an item nobody has rated
    or planned yet. One planned but never rated (a plan from before ratings, or a planner that
    gave none) counts as medium, so it needs a strong plan."""
    if not candidate.rated and not candidate.planned:
        return UNRATED_PLAN_FLOOR
    return PLAN_FLOOR[candidate.difficulty]


def plan_meets(candidate: Candidate) -> bool:
    """Its plan was written by a model of at least its plan floor (#317 part 6). A plan from
    before planners' tiers were recorded, or one a person put in the description, counts as
    strong."""
    return candidate.planned and providers_mod.tier_at_least(candidate.plan_tier or "strong",
                                                             plan_floor(candidate))


def needs_plan(candidate: Candidate) -> bool:
    """The Needs plan stage: a build with no plan yet, or one whose plan came from a model
    weaker than its difficulty's plan floor (a medium plan for a medium or hard item). Devin,
    which cannot plan, builds only from a plan that meets its floor."""
    return candidate.kind == "build" and not plan_meets(candidate)


#: The order of urgency after forced items, the priority tier and the difficulty (`pairs` in
#: plan.py puts harder items first, since only the stronger models can take them).
KIND_ORDER = {"review": 0, "revise": 1, "build": 2}


#: A line of a description that names an issue that must close first: "Blocked by #125",
#: "Depends on #12 and #14", "Do not start until #125 has merged".
_BLOCKED_BY = re.compile(
    r"(?i)\b(?:blocked by|depends on|do not start until|waits (?:on|for))[^\S\n]*:?[^\S\n]*"
    r"(#\d+(?:[^\S\n]*(?:,|\band\b|&)?[^\S\n]*#\d+)*)")
#: A multi-part patch's part (docs/issues-and-patches.md): "Patch v0.2.X (part 2 of 4): …".
#: n is the order the parts merge in, and every part of a patch carries the same m.
_PART = re.compile(r"(?i)^(.*?)\s*\(part (\d+) of (\d+)\)")


def named_blockers(body: str | None) -> set[int]:
    """The issues a description says must close first, its Plan section left out (a planner's
    notes are not the task's)."""
    found: set[int] = set()
    for match in _BLOCKED_BY.finditer(issueplan.without_plan(body)):
        found.update(int(n) for n in re.findall(r"#(\d+)", match.group(1)))
    return found


def part_of(title: str) -> tuple[tuple[str, int], int] | None:
    """`((the patch's name, m), n)` for a part's title, or None. Several patches can share a
    name such as "Patch v0.2.X", so the parts of one patch are those with its name and its m."""
    match = _PART.match(str(title or "").strip())
    if not match:
        return None
    return (" ".join(match.group(1).lower().split()), int(match.group(3))), int(match.group(2))


def waits_for(ctx: Context, threads: list[dict[str, Any]]) -> dict[int, list[int]]:
    """The issues among `threads` that wait for another to close first, with what they wait for:
    the issues a "Blocked by #n" line names, GitHub's own blocked-by links, and the earlier parts
    of the same patch (a part builds only once every part before it has closed). Only open ones
    count. GitHub is asked about open issues only when some thread names one or is a part."""
    named = {int(t["number"]): named_blockers(t.get("body")) for t in threads}
    parts = {int(t["number"]): part_of(str(t.get("title") or "")) for t in threads}
    open_numbers: set[int] = set()
    earlier: dict[int, list[int]] = {}
    if any(named.values()) or any(parts.values()):
        try:
            everything = ctx.gh.list_issues(state="open", limit=500)
        except GitHubError:
            everything = []
        open_numbers = {int(t["number"]) for t in everything}
        for number, part in parts.items():
            if part is None:
                continue
            earlier[number] = sorted(
                int(t["number"]) for t in everything if "pull_request" not in t
                and (other := part_of(str(t.get("title") or ""))) is not None
                and other[0] == part[0] and other[1] < part[1])
    waiting: dict[int, list[int]] = {}
    for thread in threads:
        number = int(thread["number"])
        blockers = {n for n in named[number] if n in open_numbers and n != number}
        blockers.update(earlier.get(number, ()))
        summary = thread.get("issue_dependencies_summary")
        if isinstance(summary, dict) and int(summary.get("blocked_by") or 0) > 0:
            try:
                blockers.update(int(b["number"]) for b in ctx.gh.blocked_by(number)
                                if b.get("state") == "open")
            except GitHubError:
                pass
        if blockers:
            waiting[number] = sorted(blockers)
    return waiting


def candidates(ctx: Context, state: dict[str, Any],
               skipped: list[str] | None = None) -> list[Candidate]:
    """Queued threads: forced requests first, then by priority tier, then second reviews,
    revisions, and the oldest builds.

    Read by label, so no number of open threads hides one, and a label changed since the last
    plan counts at this one. Either queue label queues either kind of thread: an issue builds
    and a pull request revises. A queue label is the request, so a thread that failed before and
    was labelled again is taken again. A thread labelled `human` is left out, whatever model
    would take it, and so is a build that waits for another issue (`waits_for`) unless it was
    forced, each with a line in `skipped` when the caller keeps one."""
    found: dict[int, Candidate] = {}
    human: set[int] = set()
    builds: list[dict[str, Any]] = []
    for label in (LABEL_BUILD, LABEL_REVISE, LABEL_CROSS):
        for thread in ctx.gh.list_issues(labels=label):
            number = int(thread["number"])
            names = label_names(thread)
            if LABEL_WORKING in names or number in found or number in human:
                continue
            is_pr = "pull_request" in thread
            if label == LABEL_CROSS and not (is_pr and LABEL_PR in names):
                continue  # a second review is for the bot's own pull requests only
            lowered = {name.lower() for name in names}
            if LABEL_HUMAN in lowered:
                human.add(number)
                continue
            record = state["items"].get(str(number), {})
            kind = "review" if label == LABEL_CROSS else "revise" if is_pr else "build"
            votes = record.get("votes") or {}
            found[number] = Candidate(
                number, kind, str(thread.get("title", "")), bool(record.get("forced")),
                str(record.get("queued_at") or thread.get("created_at") or ""),
                difficulty=difficulty_of(names, carried_difficulty(record)),
                builder=str(votes.get("builder") or ""),
                priority=priority_tier(names), **plan_of(record, thread, kind),
                approved=tuple(votes.get("approvals") or ()),
                approval_tiers=tuple(tier for _, tier in review_rule.approvals(
                    votes, ctx.cfg.pool.family_tier)),
                bot_pr=LABEL_PR in names,
                carries=kind == "revise" and LABEL_PR in names and carries(record),
                labels=tuple(sorted(names)), rated=kind != "build" or rated(names, record),
                conflict=kind == "revise" and record.get("source") == "conflict")
            if kind == "build" and not found[number].forced:
                builds.append(thread)
    waiting = waits_for(ctx, builds) if builds else {}
    for number in waiting:
        found.pop(number, None)
    if skipped is not None:
        skipped.extend(f"#{number} skipped: labelled `human`, so no model takes it, whatever its "
                       "tier" for number in sorted(human))
        skipped.extend(f"#{number} skipped: it waits for "
                       f"{', '.join(f'#{n}' for n in blockers)} to close first"
                       for number, blockers in sorted(waiting.items()))
    return sorted(found.values(), key=lambda c: (not c.forced, c.priority, KIND_ORDER[c.kind],
                                                 c.queued_at, c.number))

