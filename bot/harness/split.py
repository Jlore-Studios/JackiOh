"""A split (#60): an issue broken into sub-issues, each small enough for one run.

Squishy's `split` and `split-bot` modes run one model session that reads the issue and the code and
answers with the tree as data, in a `<!-- split: {...} -->` header (`work.Worker._split`). The model
writes no issue itself: the deliver job, which holds the token and runs no model, checks the tree
here and creates the sub-issues (`deliver.Deliverer._split_out`): GitHub sub-issues of the parent,
with GitHub's own "blocked by" links between them, each queued for whoever builds it (Squishy for
`split`, the night bot for `split-bot`). Each one's plan goes into its description as the
**Plan** section (`issueplan.py`), which both bots build from without planning again.

The same session closes a tree out: when every sub-issue has closed, the parent is split again
with its sub-issues listed, and the answer is `"done": true` when the parent's end state holds on
`main`, or the sub-issues still missing.

Both bots' `fullsend` mode (#505) splits the same way, into parts that own their files
(`FULLSEND_NOTES`), but each part lands on the parent's own branch (`onto`) instead of `main`,
with no pull request, checks or review of its own (`work.Worker._part`, `deliver.Deliverer.
_land_part`); once every part has closed, the parent's reconcile builds that branch into one pull
request into `main` (`prompts/reconcile.md`).
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass, field
from typing import Any

from harness.verdicts import header

#: The most sub-issues one split may open: a tree bigger than this is a project to split in stages.
MAX_ISSUES = 20
TITLE_CHARS = 200
BODY_CHARS = 20_000
PLAN_CHARS = 30_000
_KEY = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_.-]{0,40}$")

#: What a fullsend split is told beyond a split's rules (#505, `prompts/split.md`'s `$fullsend`):
#: the parts land on one branch and are reconciled there, as `.claude/skills/fullsend/` shatters a
#: build into slices (its Phase 1).
FULLSEND_NOTES = """## This is a fullsend split

A person asked for this issue to be built with **fullsend** (`.claude/skills/fullsend/SKILL.md`),
and the rules below change for it in four ways.

- **One branch, not `main`.** Each part lands on `{branch}`, the issue's own branch, not on
  `main`, with no pull request, checks or review of its own. A part need not pass the checks on
  its own: once every part has closed, one run reconciles the branch, makes every check green and
  opens one pull request into `main`.
- **Vertical slices that own their files.** Cut the issue as the skill's Phase 1 shatters a build:
  each part owns its files outright, top to bottom, and no file is in two parts, so the parts with
  no `blocked_by` are built side by side. Name each part's files in its body and its plan.
- **`blocked_by` only where one part reads another's code.** A part that only needs another to
  exist (a type, a function it calls) can follow the interface the issue and your plan fix, and
  needs no link.
- **No "make it green" part.** A part may test its own code, but the tests that need several
  parts, and every fix that makes the checks pass, are the reconcile's: a part that only fixes the
  others would wait for all of them.
"""
#: What a part's builder reads first (#505, `work.Worker._first_prompt`).
PART_NOTE = ("This is part of fullsend tree #{parent}. Your change lands on `{onto}`, which holds "
             "the parts built before yours, not on `main`, and the harness runs no checks and no "
             "review on it. Change only the files your plan names. A check that is red because "
             "another part is missing is expected: a later run reconciles the parts and makes "
             "every check green.\n\n")
_FENCE = re.compile(r"```json\s*\n(.*?)\n```", re.S)


@dataclass
class SubIssue:
    key: str
    title: str
    body: str
    plan: str = ""
    difficulty: str = ""
    labels: tuple[str, ...] = ()
    #: The keys of the sub-issues of this split that must close first, and the numbers of issues
    #: already open that must (a close-out may build on what is there).
    blocked_by: tuple[str, ...] = ()
    blocked_by_issues: tuple[int, ...] = ()

    def to_dict(self) -> dict[str, Any]:
        return {"key": self.key, "title": self.title, "body": self.body, "plan": self.plan,
                "difficulty": self.difficulty, "labels": list(self.labels),
                "blocked_by": list(self.blocked_by),
                "blocked_by_issues": list(self.blocked_by_issues)}


@dataclass
class Tree:
    done: bool
    summary: str
    issues: list[SubIssue] = field(default_factory=list)
    #: What the tree could not be read as, one line each: the answer is refused when non-empty.
    problems: list[str] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        return not self.problems and (self.done or bool(self.issues))

    def to_dict(self) -> dict[str, Any]:
        return {"done": self.done, "summary": self.summary,
                "issues": [i.to_dict() for i in self.issues], "problems": list(self.problems)}


def _raw(text: str) -> Any:
    value, _ = header(text, "split")
    if value is not None:
        return value
    found = _FENCE.findall(str(text or ""))
    for block in reversed(found):
        try:
            return json.loads(block)
        except ValueError:
            continue
    return None


def _ints(raw: Any) -> tuple[int, ...]:
    found = []
    for entry in raw or ():
        match = re.fullmatch(r"#?(\d{1,7})", str(entry).strip())
        if match:
            found.append(int(match.group(1)))
    return tuple(found)


def parse(text: str, difficulties: tuple[str, ...] = ("easy", "medium", "hard")) -> Tree:
    """The tree a split session answered with. Problems say what is wrong with it; a tree with
    any is not used, so a half-read answer never opens half a project."""
    return from_dict(_raw(text), difficulties)


def from_dict(raw: Any, difficulties: tuple[str, ...] = ("easy", "medium", "hard")) -> Tree:
    """A tree from its JSON: the model's answer, or the work job's result, which the deliver job
    checks again rather than trust."""
    if not isinstance(raw, dict):
        return Tree(False, "", problems=["no `<!-- split: {...} -->` header with a JSON object"])
    done = bool(raw.get("done"))
    summary = str(raw.get("summary") or "").strip()[:4000]
    entries = raw.get("issues") or []
    if not isinstance(entries, list):
        return Tree(done, summary, problems=["`issues` is not a list"])
    tree = Tree(done, summary)
    if len(entries) > MAX_ISSUES:
        tree.problems.append(f"{len(entries)} sub-issues; at most {MAX_ISSUES} at once")
    seen: set[str] = set()
    for i, entry in enumerate(entries[:MAX_ISSUES]):
        where = f"issues[{i}]"
        if not isinstance(entry, dict):
            tree.problems.append(f"{where} is not an object")
            continue
        key = str(entry.get("key") or f"s{i + 1}").strip()
        if not _KEY.match(key) or key in seen:
            tree.problems.append(f"{where}: key {key!r} is not a short unique name")
            continue
        seen.add(key)
        title = " ".join(str(entry.get("title") or "").split())[:TITLE_CHARS]
        body = str(entry.get("body") or "").strip()[:BODY_CHARS]
        if not title or not body:
            tree.problems.append(f"{where} ({key}): needs a title and a body")
            continue
        difficulty = str(entry.get("difficulty") or "").strip().lower()
        if difficulty and difficulty not in difficulties:
            difficulty = ""
        labels = tuple(dict.fromkeys(str(label).strip() for label in entry.get("labels") or ()
                                     if str(label).strip()))
        tree.issues.append(SubIssue(
            key, title, body, plan=str(entry.get("plan") or "").strip()[:PLAN_CHARS],
            difficulty=difficulty, labels=labels[:6],
            blocked_by=tuple(str(k).strip() for k in entry.get("blocked_by") or ()
                             if isinstance(k, str) and not re.fullmatch(r"#?\d+", k.strip())),
            blocked_by_issues=_ints([*(k for k in entry.get("blocked_by") or ()
                                       if re.fullmatch(r"#?\d+", str(k).strip())),
                                     *(entry.get("blocked_by_issues") or ())])))
    keys = {issue.key for issue in tree.issues}
    for issue in tree.issues:
        unknown = [k for k in issue.blocked_by if k not in keys]
        if unknown:
            tree.problems.append(f"{issue.key}: blocked by unknown key(s) {', '.join(unknown)}")
    if not tree.problems and _cycle(tree.issues):
        tree.problems.append("the blocked-by links go round in a circle")
    if done and tree.issues:
        tree.problems.append("`done` is true but it lists sub-issues to open")
    if not done and not tree.issues and not tree.problems:
        tree.problems.append("no sub-issues, and `done` is not true")
    return tree


def _cycle(issues: list[SubIssue]) -> bool:
    after = {issue.key: set(issue.blocked_by) for issue in issues}
    state: dict[str, int] = {}

    def visit(key: str) -> bool:
        if state.get(key) == 1:
            return True
        if state.get(key) == 2:
            return False
        state[key] = 1
        if any(visit(k) for k in after.get(key, ())):
            return True
        state[key] = 2
        return False

    return any(visit(key) for key in after)


def ordered(issues: list[SubIssue]) -> list[SubIssue]:
    """The sub-issues with every one after those it waits for (the tree has no cycle), in the
    model's order otherwise: issues are numbered in the order they are opened, so the queue's
    oldest-first order follows the dependencies."""
    placed: list[SubIssue] = []
    done: set[str] = set()
    rest = list(issues)
    while rest:
        for issue in rest:
            if set(issue.blocked_by) <= done:
                placed.append(issue)
                done.add(issue.key)
                rest.remove(issue)
                break
        else:  # a cycle `parse` should have refused: keep the rest as given
            placed += rest
            break
    return placed


def body(issue: SubIssue, parent: int, who: str, waits: list[int], onto: str = "") -> str:
    """A sub-issue's description, its plan aside (`issueplan.with_plan` adds that). The issues it
    waits for are named in a "Blocked by" line too, which the queue reads (`queue.waits_for`) even
    if GitHub's own link could not be made. A fullsend part (#505) says where it lands."""
    blocked = f"\n\nBlocked by {', '.join(f'#{n}' for n in waits)}." if waits else ""
    lands = f"\n\nIts build lands on `{onto}`, not `main` (fullsend)." if onto else ""
    return f"{issue.body}{blocked}{lands}\n\nPart of #{parent}, split by {who}."


def checklist(parent: int, made: list[tuple[SubIssue, int]], *, builder: str, summary: str,
              link: str, onto: str = "") -> str:
    """The comment on the parent after a split: the tree, in build order, and how it goes on."""
    lines = [f"Split #{parent} into {len(made)} sub-issue(s) ({link}), queued for {builder} in "
             "this order; each waits for the ones it is blocked by:", ""]
    numbers = {issue.key: number for issue, number in made}
    for issue, number in made:
        waits = [f"#{numbers[k]}" for k in issue.blocked_by if k in numbers]
        waits += [f"#{n}" for n in issue.blocked_by_issues]
        after = f" (after {', '.join(waits)})" if waits else ""
        lines.append(f"- [ ] #{number}{after}")
    if onto:
        lines += ["", f"This is a fullsend tree: each part lands on `{onto}`, not on `main`, with "
                  "no pull request of its own. Once every part has closed, one run reconciles them "
                  "on that branch, makes every check green and opens one pull request into `main`."]
    if summary:
        lines += ["", summary]
    return "\n".join(lines)
