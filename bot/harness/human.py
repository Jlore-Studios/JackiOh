"""Work only a person can do (#793): the `human required` label.

The bot may not change `bot/`, `.harness/`, `.squishy/` or `.github/` (the path guard puts such a
change back, and deliver refuses to push one), holds no secret it was not given, and has no
dashboard: GitHub's settings and secrets, Vercel, Render, Supabase and Cloudflare are a person's.
No run can finish an item that needs one of those, and asking the same question run after run
spends a run each time (#760 spent two). So the bot labels it `human required`, names the people
(`triage.HUMANS`) in a comment that says what a person must do (`ask`), and leaves it alone while
the label is on (`queue.is_human`). A person who has done their part takes the label off and
queues it again; one who builds it all closes it.

What says an item needs a person:

- its plan's **Files to touch** table lists only paths the bot may not change (`plan_needs`);
- its planner says so, in a `**Human required:**` line before the plan's **Goal** (`plan_needs`);
- its builder or reviser stopped with `"human_required": true` (`verdicts.build_report`);
- the path guard put the whole of a build's change back (`work.Worker._only_forbidden`).
"""

from __future__ import annotations

import re
from typing import Iterable

from harness import easy, triage
from harness.config import LABEL_HUMAN_REQUIRED, SLASH
from harness.git import matches

#: A planner's own word that only a person can do the task: `**Human required:** <what>`.
_SAID = re.compile(r"(?im)^[\s>*_]*human required[*_]*\s*:[*_]*\s*(.*)$")
#: The plan's **Goal** heading: the planner's line goes before it (`bot/prompts/plan.md`).
_GOAL = re.compile(r"(?im)^\s*(?:#+\s*)?(?:\d+\.\s*)?\**\s*goal\b")
#: How many lines open a plan with no **Goal** heading, where the planner's line may stand.
_HEAD_LINES = 5
#: What a planner writes on the line when no person is needed after all.
_NOBODY = {"no", "none", "n/a", "na", "nothing", "false", "not needed", "not required", "-"}


def plan_needs(plan: str, forbidden: Iterable[str]) -> str:
    """What a person must do before the bot can build any of `plan`, or "" when the bot can.

    The planner's `**Human required:**` line before **Goal** says it in its own words; a plan
    whose **Files to touch** table lists only paths in `forbidden` says it by itself. A plan that
    needs a person for one part only plans the rest and names that part under **Open
    questions**, and is built."""
    text = str(plan or "")
    goal = _GOAL.search(text)
    head = text[:goal.start()] if goal else "\n".join(text.splitlines()[:_HEAD_LINES])
    said = _SAID.search(head)
    words = " ".join(said.group(1).split()) if said else ""
    if said and words.strip(" .*_").lower() not in _NOBODY:
        return words or "Its planner says only a person can do it."
    paths = easy.plan_paths(text)
    if paths and all(matches(path, forbidden) for path in paths):
        listed = ", ".join(f"`{path}`" for path in paths[:8])
        return (f"Every file its plan touches ({listed}) is in a path I may not change, such as "
                "`bot/`, `.harness/` and `.github/`, so a person has to make the change, from "
                "the plan in the description.")
    return ""


def people() -> str:
    """The people a `human required` item is handed to, as mentions."""
    return " ".join(f"@{login}" for login in triage.HUMANS)


def ask(question: str, bot_login: str, *, what: str = "build this", kept: str = "",
        again: str = "") -> str:
    """The comment that hands an item to a person: it names them, says what they must do, and
    how to give the item back. `what` is what the bot cannot do; `kept` notes work it kept;
    `again` is how to queue it again (a build, by default)."""
    again = again or f"`{SLASH} build` (or `@{bot_login} <notes>`) queues the rest"
    return (f"{people()} this needs a person: I cannot {what} myself.{kept}\n\n"
            f"{question.strip() or 'It needs something only a person can do.'}\n\n"
            f"I labelled it `{LABEL_HUMAN_REQUIRED}`, and I leave it alone while that label is "
            f"on. Once a person has done their part, take the label off and {again}; if you "
            "build it all yourselves, close it.")
