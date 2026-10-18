"""Triage (`.github/workflows/triage.yml`, #664): what a person's new issue or pull request gets
without anyone asking, from fixed rules and no model, in one job on GitHub's runner.

- An issue a trusted person opens gets the answers of the form it came from (`form_labels`: its
  difficulty, its priority and who does it, `bot:build` or `human`), the type label its title's
  prefix names when it has none (`type_label`), and an issue type (`fallback_type`). The night bot
  holds a build that waits for another issue by itself (`queue.waits_for`: a "Blocked by #n" line,
  the earlier parts of a patch), so triage links nothing.
- An issue a person labels `human` (or `human required`, #793), or opens with it, goes to people:
  both are assigned, the bot is unassigned, and the bot's queue labels come off
  (`hand_to_people`). The bot hands an issue over itself the same way when it finds it cannot do
  it (`deliver.Deliverer._needs_person`).
- A pull request a trusted person opens gets the type label its title's prefix names.

It adds, never removes, but for that last: a label, an assignee or a type a person set stays. A
person can call it on any thread (`workflow_dispatch` with its number), which triages it as if it
had just been opened, its form's answers aside. Any failure skips quietly.

Handing an issue to the bot is `bot:build` (or assigning the bot, or `/harness build`), and that is
also how a person approves one of the bot's suggestions; handing it to people is `human`.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Any, Mapping

from harness import config as config_mod
from harness.errors import GitHubError
from harness.trust import TRUSTED_ASSOCIATIONS, Trust

#: The people a human task goes to.
HUMANS: tuple[str, ...] = ("MaxGoetzmann", "jgoetzmann")
#: The type labels (`docs/issues-and-patches.md`): the only ones a pull request gets.
TYPE_LABELS: tuple[str, ...] = config_mod.TYPE_LABELS
HUMAN_LABEL = config_mod.LABEL_HUMAN
#: The labels that hand an issue to people: a person's `human`, and `human required` (#793), the
#: bot's own hand-over, which a person may put on too.
HUMAN_LABELS = frozenset({config_mod.LABEL_HUMAN, config_mod.LABEL_HUMAN_REQUIRED})
#: The bot's queue labels `human` takes off: people do the issue now. A run that holds it
#: (`bot:working`) or a pull request it opened keep theirs.
QUEUED = frozenset({config_mod.LABEL_BUILD, config_mod.LABEL_NEEDS_PLAN})
#: The type label each title prefix names (`docs/issues-and-patches.md`, Titles).
PREFIX_TYPES: tuple[tuple[re.Pattern[str], str], ...] = (
    (re.compile(r"^Patch v\d", re.I), "patch"),
    (re.compile(r"^v\d+\.\d+\.0\b"), "major version"),
    (re.compile(r"^Night bot:", re.I), "night bot"),
    (re.compile(r"^(?:Architecture|CI):", re.I), "architecture"),
)
#: A title that already follows the convention.
CONVENTION = re.compile(
    r"^(?:Patch v\d+\.\d+\.(?:\d+|X|Y)[a-z]?(?: \(part \d+ of \d+\))?"
    r"|v\d+\.\d+\.0(?: \(part \d+ of \d+\))?"
    r"|Night bot|CI|Architecture): \S")
#: The titles tools give the pull requests they open, which the title check passes as they are
#: (`pull_title_ok`): `patches ship: v0.3.1` (patches-ship.yml; several versions joined by ", "),
#: `Promote main to production: 28 commit(s) up to 6d44be2` (scripts/promote-production.sh, into
#: `production`) and a training lane's promotion, `AI gen 8 (improve): …` (training/improve.md, learn.yml and
#: unban.md). The bots' own pull requests take a title that follows the convention (`pull_title`).
AUTOMATED_PULL = re.compile(
    r"^(?:patches ship: v\d+\.\d+\.\d+[a-z]?(?:, v\d+\.\d+\.\d+[a-z]?)*$"
    r"|Promote main to production: \d+ commit\(s\) up to [0-9a-f]{7,40}$"
    r"|AI gen \d+ \((?:improve|unban|learn)\): \S)")
#: GitHub's Revert button titles its pull request `Revert "<the reverted pull request's title>"`.
REVERT = re.compile(r'^Revert "(.+)"$')
#: The organisation's issue types (Settings → Planning → Issue types), used when the token cannot
#: read them: an issue gets one; a pull request has none.
DEFAULT_ISSUE_TYPES: dict[str, str] = {
    "Task": "A specific piece of work",
    "Bug": "An unexpected problem or behavior",
    "Feature": "A request, idea, or new functionality",
}


@dataclass
class Plan:
    """What `apply` does to a thread."""

    labels: list[str] = field(default_factory=list)
    assignees: list[str] = field(default_factory=list)
    #: The issue's type (Task, Bug, Feature), only when it has none.
    issue_type: str = ""
    #: Assignees `human` moves off it: the bot.
    unassign: list[str] = field(default_factory=list)
    #: The bot's queue labels `human` takes off.
    remove: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)

    def empty(self) -> bool:
        return not (self.labels or self.assignees or self.issue_type or self.unassign
                    or self.remove)


# ------------------------------------------------------------------ titles

def thread_of(payload: Mapping[str, Any]) -> tuple[dict[str, Any], bool]:
    """The issue or pull request an event is about, and whether it is a pull request."""
    if payload.get("pull_request"):
        return dict(payload["pull_request"]), True
    return dict(payload.get("issue") or {}), "pull_request" in (payload.get("issue") or {})


def follows_convention(title: str) -> bool:
    return bool(CONVENTION.match(title.strip()))


def pull_title_ok(title: str) -> bool:
    """Whether a pull request's title may become the squash commit's subject (the title check,
    `.github/workflows/pr-title.yml`, #187): it follows the convention, it is one a tool gives its
    own pull requests (`AUTOMATED_PULL`), or it reverts a pull request whose title passes."""
    text = title.strip()
    revert = REVERT.match(text)
    if revert:
        return pull_title_ok(revert.group(1))
    return bool(CONVENTION.match(text) or AUTOMATED_PULL.match(text))


def pull_title(chosen: str, issue: str) -> str:
    """The title of the pull request a bot opens for an issue: its builder's when that passes the
    title check, else the issue's when that does, else the builder's as it is, which the check
    then flags for a person to retitle."""
    for title in (chosen, issue):
        if title.strip() and pull_title_ok(title):
            return " ".join(title.split())
    return chosen.strip() or issue.strip()


def type_label(title: str) -> str:
    """The type label a title's prefix names, or "" for a title with no prefix."""
    text = str(title or "").strip()
    return next((label for pattern, label in PREFIX_TYPES if pattern.match(text)), "")


#: An issue form's answers in the body GitHub writes: `### <field label>`, a blank line, the answer.
_FORM_ANSWER = re.compile(r"^### (.+?)[ \t]*\n+(.*?)\s*(?=^### |\Z)", re.M | re.S)
_PRIORITIES = (config_mod.LABEL_PRIORITY_HIGH, config_mod.LABEL_PRIORITY_MEDIUM,
               config_mod.LABEL_PRIORITY_LOW)
#: Who does it, as each form's dropdown spells it: the label in its brackets.
_WHO = (config_mod.LABEL_BUILD, HUMAN_LABEL)


def form_labels(body: str) -> list[str]:
    """The labels an issue form's dropdowns ask for (`.github/ISSUE_TEMPLATE/`), which triage puts
    on when a trusted person opens the issue, as if they had set them by hand: "Difficulty",
    "Priority", and who does it, `bot:build` or `human`. "Not sure", "No priority" and "Decide
    later" ask for nothing; neither does a body no form wrote."""
    answers = {m.group(1).strip().lower(): m.group(2).strip().lower()
               for m in _FORM_ANSWER.finditer((body or "").replace("\r\n", "\n"))}
    labels = []
    difficulty = next((v for k, v in answers.items() if k.startswith("difficulty")), "")
    labels += [name for name, level in config_mod.DIFFICULTY_LABELS.items()
               if difficulty.startswith(level)][:1]
    labels += [name for name in _PRIORITIES if answers.get("priority") == name.split(":")[1]]
    labels += [name for name in _WHO if f"({name})" in answers.get("who does it", "")][:1]
    return labels


# ------------------------------------------------------------------ issue types

_BUG_WORDS = re.compile(
    r"(?i)\b(bug|fails?|failing|failed|broken|breaks|crash(?:es)?|wrong|lag(?:s|gy)?|regression"
    r"|errors?|not working|doesn'?t work|stuck|stall(?:s|ing)?|hangs?)\b")
_FEATURE_WORDS = re.compile(
    r"(?i)\b(new|adds?|support|page|mode|screen|easter egg|leaderboard|emotes?|feature)\b")
_TASK_PREFIXES = ("architecture:", "refactor:", "ci:", "night bot:", "tests:", "docs:")


def fallback_type(issue: Mapping[str, Any], types: Mapping[str, str]) -> str:
    """An issue's type from its title and labels (triage, and the sweep for the rest): a failure
    is a Bug; tooling, CI, the bot and trackers are a Task; something new for players or the team
    is a Feature; anything else a Task. "" when the organisation has none of those."""
    title = str(issue.get("title") or "")
    names = {str(label.get("name")).lower() for label in issue.get("labels") or []}
    if _BUG_WORDS.search(title):
        wanted = "Bug"
    elif title.lower().startswith(_TASK_PREFIXES) or names & {"architecture", "night bot"}:
        wanted = "Task"
    elif _FEATURE_WORDS.search(title):
        wanted = "Feature"
    else:
        wanted = "Task"
    named = {name.lower(): name for name in types}
    return named.get(wanted.lower(), "")


def issue_types(gh: Any) -> dict[str, str]:
    """The organisation's issue types and their descriptions, or the defaults when there are none
    or the token cannot read them."""
    try:
        found = {str(t.get("name")): str(t.get("description") or "")
                 for t in gh.list_issue_types() or [] if t.get("name")}
    except (GitHubError, AttributeError):
        found = {}
    return found or dict(DEFAULT_ISSUE_TYPES)


# ------------------------------------------------------------------ the gate

def _is_bot(login: str, bot_login: str) -> bool:
    return login.lower() == bot_login.lower() or login.endswith("[bot]")


def gate(payload: Mapping[str, Any], trust: Trust, bot_login: str, *,
         asked: bool = False) -> tuple[str, str]:
    """What this event asks of triage, and why: "open" for a thread just opened (or one a person
    called triage on, `asked`), "human" for an issue a person labelled `human`, or "" to skip. A
    new thread goes only when a trusted person opened it; the bot and apps label their own."""
    thread, is_pr = thread_of(payload)
    if not thread.get("number"):
        return "", "no issue or pull request in the event"
    if asked:
        return "open", f"#{thread['number']}: a person called triage on it"
    if payload.get("action") == "labeled":
        name = str((payload.get("label") or {}).get("name") or "")
        sender = str((payload.get("sender") or {}).get("login") or "")
        if is_pr or name.lower() not in HUMAN_LABELS:
            return "", f"label {name} asks nothing of triage"
        if not sender or _is_bot(sender, bot_login):
            return "", f"labelled by {sender or 'nobody known'}, which labels its own"
        return "human", f"#{thread['number']}: @{sender} labelled it {name.lower()}"
    if payload.get("action") != "opened":
        return "", f"{payload.get('action')} asks nothing of triage"
    user = thread.get("user") or {}
    login = str(user.get("login") or "")
    if _is_bot(login, bot_login):
        return "", f"opened by {login}, which labels its own"
    association = str(thread.get("author_association") or "").upper()
    if association not in TRUSTED_ASSOCIATIONS and trust.level(
            login, user.get("id"), association) < 1:
        return "", f"@{login} is not trusted ({association or 'no association'})"
    return "open", f"#{thread['number']} by @{login}"


# ------------------------------------------------------------------ decide and apply

def _names(thread: Mapping[str, Any]) -> list[str]:
    return [str(label.get("name")) for label in thread.get("labels") or []]


def hand_to_people(plan: Plan, thread: Mapping[str, Any], bot_login: str) -> None:
    """`human` on an issue: both people take it, and the bot lets go of it."""
    assigned = {str(a.get("login") or "").lower() for a in thread.get("assignees") or []}
    plan.assignees += [h for h in HUMANS if h.lower() not in assigned and h not in plan.assignees]
    if bot_login.lower() in assigned:
        plan.unassign = [bot_login]
    plan.remove = [name for name in _names(thread) if name.lower() in QUEUED]


def decide(step: str, thread: Mapping[str, Any], is_pr: bool, bot_login: str, *,
           types: Mapping[str, str] | None = None, form: bool = True) -> Plan:
    """What `step` (`gate`'s answer) changes on `thread`, read afresh. `form`: take the form's
    answers too (an issue just opened; not one a person called triage on again)."""
    plan = Plan()
    if thread.get("state") not in (None, "open"):
        plan.notes.append("closed meanwhile")
        return plan
    present = _names(thread)
    lowered = {name.lower() for name in present}
    if step == "human":
        if lowered & HUMAN_LABELS:
            hand_to_people(plan, thread, bot_login)
        else:
            plan.notes.append("`human` came off again, so it stays as it is")
        return plan
    if form and not is_pr:
        plan.labels += [name for name in form_labels(str(thread.get("body") or ""))
                        if name.lower() not in lowered]
    kind = type_label(str(thread.get("title") or ""))
    if kind and not lowered & set(TYPE_LABELS) and kind not in plan.labels:
        plan.labels.append(kind)
    if is_pr:
        return plan
    if not thread.get("type"):  # a person's type stays
        labelled = [*(thread.get("labels") or []), *({"name": n} for n in plan.labels)]
        plan.issue_type = fallback_type({**thread, "labels": labelled},
                                        types or DEFAULT_ISSUE_TYPES)
    if lowered & HUMAN_LABELS or HUMAN_LABEL in plan.labels:
        hand_to_people(plan, thread, bot_login)
    return plan


def apply(gh: Any, number: int, plan: Plan) -> list[str]:
    """Make `plan`'s changes; each one on its own, so one refusal leaves the rest done."""
    done: list[str] = []
    steps = []
    if plan.labels:
        steps.append((f"labelled {', '.join(plan.labels)}",
                      lambda: gh.add_labels(number, plan.labels)))
    if plan.assignees:
        steps.append((f"assigned {', '.join('@' + a for a in plan.assignees)}",
                      lambda: gh.add_assignees(number, plan.assignees)))
    if plan.unassign:
        steps.append((f"unassigned {', '.join('@' + a for a in plan.unassign)}",
                      lambda: gh.remove_assignees(number, plan.unassign)))
    if plan.issue_type:
        steps.append((f"typed it {plan.issue_type}", lambda: _set_type(gh, number, plan.issue_type)))
    for name in plan.remove:
        steps.append((f"took `{name}` off", lambda n=name: gh.remove_label(number, n)))
    for what, step in steps:
        try:
            step()
            done.append(what)
        except GitHubError as exc:
            done.append(f"could not do this: {what} ({exc.status})")
        except ValueError as exc:
            done.append(f"could not do this: {what} ({exc})")
    return done


def _set_type(gh: Any, number: int, name: str) -> None:
    """Set the issue's type. GitHub drops a type it will not take without an error, so read the
    answer back."""
    answer = gh.update_issue(number, type=name) or {}
    got = answer.get("type")
    got = got.get("name") if isinstance(got, Mapping) else got
    if str(got or "").lower() != name.lower():
        raise ValueError("GitHub did not keep it")
