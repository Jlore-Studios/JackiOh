"""Comment commands: `/harness <verb> [args]`, `/harness-<verb>` and `@<bot> <verb or request>`.

A command is a line of its own. Lines inside fenced code blocks and quoted lines (`> ...`) are
never commands, so quoting the bot back at it cannot re-run anything. At most `MAX_COMMANDS` are
read from one comment, top to bottom. Naming the bot followed by words that are not a verb is a
request: on an issue it asks for a build, on a pull request for a revision, with those words as
the notes.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

VERBS: tuple[str, ...] = ("build", "revise", "stop", "status", "halt", "start", "suggest", "run")

ALIASES: dict[str, str] = {
    "work": "build",
    "fix": "revise",
    "update": "revise",
    "help": "status",
    "resume": "start",
    "unhalt": "start",
    "go": "run",
    "suggestions": "suggest",
}

#: The level each verb needs. `request` is naming the bot with free text.
LEVELS: dict[str, int] = {
    "status": 1,
    "build": 2,
    "revise": 2,
    "stop": 2,
    "suggest": 2,
    "request": 2,
    "halt": 3,
    "start": 3,
    "run": 3,
}

#: `--force` lifts the calendar (the night window) and nothing else. Operator only.
FORCE_FLAG = "--force"
FORCE_LEVEL = 3
MAX_COMMANDS = 10

_FENCE = re.compile(r"^\s*(```|~~~)")
_SLASH = re.compile(r"^\s*(?:@[\w-]+\s+)*/harness[ \t-]+([A-Za-z]+)\b(.*)$", re.IGNORECASE)


@dataclass(frozen=True)
class Command:
    verb: str
    args: str
    force: bool
    line: str

    @property
    def level(self) -> int:
        need = LEVELS.get(self.verb, 3)
        return max(need, FORCE_LEVEL) if self.force else need


def _strip_force(args: str) -> tuple[str, bool]:
    tokens = args.split()
    force = any(t.lower() == FORCE_FLAG for t in tokens)
    kept = " ".join(t for t in tokens if t.lower() != FORCE_FLAG)
    return kept.strip(), force


def _canonical(word: str) -> str | None:
    word = word.lower().strip(":,.!?")
    word = ALIASES.get(word, word)
    return word if word in VERBS else None


def command_lines(body: str) -> list[str]:
    """The lines of `body` that may carry a command: outside fences, not quoted."""
    lines: list[str] = []
    fenced = False
    for line in str(body or "").splitlines():
        if _FENCE.match(line):
            fenced = not fenced
            continue
        if fenced or line.lstrip().startswith(">"):
            continue
        lines.append(line)
    return lines


#: In the `@bot <verb>` form, a control verb counts only when what follows fits it; otherwise
#: the line is a request in plain words ("@bot start with option A" asks for a build, it does not
#: lift a halt). `/harness <verb>` is always a command.
_MENTION_ARGS: dict[str, re.Pattern[str]] = {
    "status": re.compile(r"^$"),
    "stop": re.compile(r"^$"),
    "halt": re.compile(r"^$"),
    "start": re.compile(r"^$"),
    "suggest": re.compile(r"^$"),
    "run": re.compile(r"^(#?\d+)?$"),
}
_LIST_MARK = re.compile(r"^\s*(?:[-*+]|\d+[.)])\s+")
_WRAP = "`*_~"


def _unmark(line: str) -> str:
    """A line without the markdown a person may wrap a command in: a list marker, and backticks
    or emphasis around the whole command."""
    text = _LIST_MARK.sub("", line).strip()
    while text and text[0] in _WRAP and text[-1] in _WRAP:
        text = text[1:-1].strip()
    return text.lstrip(_WRAP).rstrip(_WRAP)


def parse(body: str, bot_login: str) -> list[Command]:
    """Every command in a comment, in order, at most `MAX_COMMANDS`."""
    mention = re.compile(
        r"^\s*@" + re.escape(bot_login.lstrip("@")) + r"\b[:,]?\s*(.*)$", re.IGNORECASE
    )
    found: list[Command] = []
    for raw in command_lines(body):
        if len(found) >= MAX_COMMANDS:
            break
        line = _unmark(raw)
        slash = _SLASH.match(line)
        if slash:
            verb = _canonical(slash.group(1))
            args, force = _strip_force(slash.group(2))
            found.append(Command(verb or f"unknown:{slash.group(1).lower()}", args, force, line.strip()))
            continue
        named = mention.match(line)
        if not named:
            continue
        rest = named.group(1).strip()
        if rest.lower().startswith("/harness"):
            continue  # `@bot /harness verb` is matched by _SLASH above
        first, _, tail = rest.partition(" ")
        verb = _canonical(first) if first else None
        args, force = _strip_force(tail)
        shape = _MENTION_ARGS.get(verb or "")
        if verb is not None and (shape is None or shape.match(args)):
            found.append(Command(verb, args, force, line.strip()))
        else:
            args, force = _strip_force(rest)
            found.append(Command("request", args, force, line.strip()))
    return found


def mentions(body: str, bot_login: str) -> bool:
    """True when a command line of `body` names the bot or says `/harness`."""
    return bool(parse(body, bot_login))


def names_the_bot(body: str, bot_login: str) -> bool:
    """True when the bot's handle, or `/harness`, appears anywhere outside quotes and code, even
    mid-sentence: a request that did not parse is answered with a hint, never ignored."""
    handle = re.compile(r"(?<![\w-])@" + re.escape(bot_login.lstrip("@")) + r"(?![\w-])",
                        re.IGNORECASE)
    slash = re.compile(r"(?<![\w/])/harness\b", re.IGNORECASE)
    return any(handle.search(line) or slash.search(line) for line in command_lines(body))


HELP = """\
**Commands** — write one per line, as `/harness <verb>` or `@{bot} <verb>`:

| Verb | What it does | Where | Level |
|---|---|---|---|
| `build [notes]` | queue this issue for the night window | issue | 2 |
| `revise <notes>` | queue a revision of this PR with your notes | pull request | 2 |
| `stop` | take this issue or PR out of the queue and stop work on it | issue or PR | 2 |
| `suggest` | ask for improvement suggestions when the queue is empty | anywhere | 2 |
| `status` | halt state, window, usage, queue | anywhere | 1 |
| `halt` | stop all model work until `start` | anywhere | 3 |
| `start` | lift a halt | anywhere | 3 |
| `run` | start a night run now, outside the window | anywhere | 3 |

`@{bot} <anything else>` asks for a build on an issue, or a revision on a PR, with your words as the notes.
Add `--force` (level 3) to `build`, `revise` or `suggest` to start now instead of in the window.
Labels do the same as the verbs: `bot:build` on an issue, `bot:revise` on a PR; assigning @{bot} queues the thread.
"""
