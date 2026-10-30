"""An item's plan, kept in its issue's description (the Needs plan stage).

A strong model plans every item before it is built (`plan.py`), and the plan goes into a marked
**Plan** section at the end of the issue's description, so a person sees it where the task is and
can correct it there: the builder starts from that section, whatever it says by then. The section
carries no bot marker (`config.MARKER`), which would make the whole description read as the bot's.
"""

from __future__ import annotations

import re

START = "<!-- jackioh-bot:plan -->"
END = "<!-- /jackioh-bot:plan -->"
#: GitHub refuses a description over 65,536 characters; the plan gives way, never the task.
BODY_LIMIT = 65_000
#: What the plan prompt asks a planner to stay under (`bot/prompts/plan.md`, #208).
PLAN_WORDS = 2_500
#: How much of a planner's answer becomes the plan: its beginning, where the goal and the first
#: steps are, never its end alone (`work.py`), and how much a comment carrying a plan the
#: description could not take shows (`deliver.py`). Room over PLAN_WORDS, so a plan that runs a
#: little long is kept whole, and well under BODY_LIMIT and `threads.MAX_CHARS`, so the task and
#: its comments keep room beside it. #307's plan was cut at the old 20,000.
PLAN_CHARS = 30_000
_SECTION = re.compile(re.escape(START) + r".*?(?:" + re.escape(END) + r"|\Z)", re.S)
_AROUND = re.compile(r"\s*" + _SECTION.pattern + r"\s*", re.S)
_HEADER = re.compile(r"\A\s*## Plan\s*\n+(?:_[^\n]*_\s*\n+)?")


def without_plan(body: str | None) -> str:
    """The description with its Plan section taken out."""
    return _AROUND.sub("\n\n", str(body or "")).strip()


def plan_of(body: str | None) -> str:
    """The Plan section's text (a person's edits included), without its heading, or ""."""
    match = _SECTION.search(str(body or ""))
    if not match:
        return ""
    text = match.group(0)[len(START):]
    if text.endswith(END):
        text = text[:-len(END)]
    return _HEADER.sub("", text).strip()


def with_plan(body: str | None, plan: str, who: str, link: str = "") -> str:
    """The description with `plan` as its Plan section, in place of any earlier one."""
    task = without_plan(body)
    note = (f"_Written by {who}{f' ({link})' if link else ''} before anyone built this. The night "
            "bot's builder starts from this section: edit it here to change the plan._")
    head = f"{task}\n\n{START}\n## Plan\n\n{note}\n\n"
    room = max(0, BODY_LIMIT - len(head) - len(END) - 40)
    text = plan.strip()
    if len(text) > room:
        text = text[:room].rstrip() + "\n\n…(cut to fit the description)"
    return f"{head}{text}\n{END}\n".lstrip("\n")
