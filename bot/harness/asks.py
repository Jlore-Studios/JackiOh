"""Reactions that follow a request through the queue, so a person sees where it has got to.

A comment that asks for model work (a build, a revision, a request in plain words, a suggestion
survey, or notes left while a run holds the thread) is recorded as an *ask* on the state record
it waits on: the thread's item, or `state["suggest"]`. An ask is the key its claim uses, `c:<id>`
for a conversation comment and `rc:<id>` for a line comment on a diff. A review cannot take a
reaction, so a review's request is not tracked. Each stage reacts to the ask:

| Reaction | Meaning | Left by |
|---|---|---|
| 👀 `eyes` | seen | events, before acting |
| 👍 `+1` | a model will read it: queued, or noted for the next pass | events |
| 🚀 `rocket` | answered; the sweep reads it as "handled" | events, after replying |
| ❤️ `heart` | a run has it: a model is reading it now | plan, claiming the thread |
| 🎉 `hooray` | done: the run that read it finished with an answer | deliver |
| 😕 `confused` | it ended without an answer: blocked, stopped or given up on | deliver, stop |

An ask waits in `asks` until a run claims the thread, which moves it to `taken_asks`. When the
run ends, each taken ask gets 🎉 or 😕, or goes back to waiting if the item is queued again. An
ask that arrives while a run holds the thread waits for the next pass. The helpers that change a
record are pure, for use inside `StateStore.update`; `react` is called once the write is done.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Iterable

SEEN = "eyes"
QUEUED = "+1"
ANSWERED = "rocket"
WORKING = "heart"
DONE = "hooray"
NO_ANSWER = "confused"

#: The most asks one record keeps; the oldest go first.
MAX_ASKS = 20


@dataclass
class Ask:
    """The comment a request came from. `recorded` says whether anything queued it."""

    key: str
    recorded: bool = False


def key(comment_id: int, *, review_comment: bool = False) -> str:
    return f"{'rc' if review_comment else 'c'}:{int(comment_id)}"


def _merge(*groups: Iterable[str] | None) -> list[str]:
    merged: list[str] = []
    for group in groups:
        for entry in group or []:
            if entry not in merged:
                merged.append(entry)
    return merged[-MAX_ASKS:]


def note(holder: dict[str, Any], ask: Ask | None) -> None:
    """Record `ask` as waiting on `holder`."""
    if ask is None:
        return
    holder["asks"] = _merge(holder.get("asks"), [ask.key])
    ask.recorded = True


def add(holder: dict[str, Any], keys: Iterable[str]) -> None:
    holder["asks"] = _merge(holder.get("asks"), keys)


def take(holder: dict[str, Any]) -> list[str]:
    """A run has claimed the holder: its waiting asks become taken. Returns those that moved."""
    moved = list(holder.get("asks") or [])
    holder["taken_asks"] = _merge(holder.get("taken_asks"), moved)
    holder["asks"] = []
    return moved


def give_back(holder: dict[str, Any]) -> None:
    """The run ended without an answer and the item is queued again: its asks wait once more."""
    holder["asks"] = _merge(holder.get("taken_asks"), holder.get("asks"))
    holder["taken_asks"] = []


def pop_taken(holder: dict[str, Any]) -> list[str]:
    taken = list(holder.get("taken_asks") or [])
    holder["taken_asks"] = []
    return taken


def pop_waiting(holder: dict[str, Any]) -> list[str]:
    waiting = list(holder.get("asks") or [])
    holder["asks"] = []
    return waiting


def react(gh: Any, keys: Iterable[str], content: str) -> None:
    """Leave `content` on every ask in `keys`. A reaction is a courtesy: `gh.react` swallows its
    own failures."""
    for entry in keys:
        kind, _, raw = str(entry).partition(":")
        if kind in ("c", "rc") and raw.isdigit():
            gh.react(int(raw), content, review_comment=kind == "rc")
