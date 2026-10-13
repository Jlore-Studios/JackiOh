"""Suggestions that came to nothing (#797), kept so that no survey offers one again.

A survey sees only the latest issues (`plan.suggestion_plan`), so a suggestion rejected weeks ago
falls out of its view and comes back. This log does not: it lives in the state file
(`suggest.declined`), newest last, and every survey reads all of it. A suggestion goes in when:

- a person rejects it with a `reject` comment (`events.answer_vote`);
- a person closes it unbuilt (`sync`, read from GitHub at each survey);
- it was approved, and the bot then handed it to people as work only a person can do
  (`deliver.Deliverer._needs_person`) or failed it for good (`deliver.Deliverer._fail`).

A suggestion the survey offers again anyway, by a title already here, is not opened
(`deliver.Deliverer._suggestions`).
"""

from __future__ import annotations

import re
from typing import Any, Iterable

#: The most entries kept, oldest dropped first: the survey's prompt carries every one.
LIMIT = 200


def _key(title: str) -> str:
    """A title as the log compares it: lowercased, its words alone."""
    return " ".join(re.findall(r"[a-z0-9]+", str(title or "").lower()))


def entries(state: dict[str, Any]) -> list[dict[str, Any]]:
    found = (state.get("suggest") or {}).get("declined")
    return [e for e in found if isinstance(e, dict)] if isinstance(found, list) else []


def note(state: dict[str, Any], number: int, title: str, why: str, at: str) -> bool:
    """Log suggestion `number` as come to nothing, once: True when it was not logged yet."""
    suggest = state.setdefault("suggest", {})
    log = entries(state)
    if any(int(e.get("number") or 0) == int(number) for e in log):
        return False
    log.append({"number": int(number), "title": " ".join(str(title or "").split())[:200],
                "why": " ".join(str(why or "").split())[:300], "at": at})
    suggest["declined"] = log[-LIMIT:]
    return True


def sync(state: dict[str, Any], closed: Iterable[dict[str, Any]], at: str) -> list[int]:
    """Log every closed suggestion in `closed` (issues labelled `bot:suggestion`) that was closed
    unbuilt: a suggestion that is built loses the label as it is queued, so one still carrying it
    when closed never was. Returns the numbers newly logged."""
    added = []
    for issue in closed:
        if "pull_request" in issue or issue.get("state") != "closed":
            continue
        reason = str(issue.get("state_reason") or "closed")
        if note(state, int(issue["number"]), str(issue.get("title") or ""),
                f"closed unbuilt by a person ({reason.replace('_', ' ')})",
                str(issue.get("closed_at") or at)):
            added.append(int(issue["number"]))
    return added


def known(state: dict[str, Any], title: str) -> bool:
    """A suggestion of this title came to nothing before."""
    key = _key(title)
    return bool(key) and any(_key(str(e.get("title") or "")) == key for e in entries(state))


def lines(state: dict[str, Any]) -> str:
    """The log as the survey reads it, one suggestion a line, or "(none)"."""
    found = [f"- #{e.get('number')} {e.get('title')}: {e.get('why')}" for e in entries(state)]
    return "\n".join(found) or "(none)"
