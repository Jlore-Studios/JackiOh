"""Comment commands: `/harness <verb> [args]`, `/harness-<verb>` and `@<bot> <verb or request>`.

A command is a line of its own. Lines inside fenced code blocks and quoted lines (`> ...`) are
never commands, so quoting the bot back at it cannot re-run anything. At most `MAX_COMMANDS` are
read from one comment, top to bottom.

Both prefixes read the rest of the line the same way: a verb (or an alias) runs; a single word
that is a near miss of a verb runs nothing and gets a "did you mean"; anything else is a request:
on an issue it asks for a build, on a pull request for a revision, with those words as the notes.
The one difference is plain English: after `@<bot>`, a control verb followed by words that do not
fit it is a request ("@bot start with option A"), unless a colon marks the words as the verb's
own ("@bot halt: away this week").
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from harness.trust import LEVEL_NAMES

VERBS: tuple[str, ...] = ("build", "revise", "stop", "status", "help", "halt", "start", "suggest",
                          "run")

ALIASES: dict[str, str] = {
    "work": "build",
    "fix": "revise",
    "update": "revise",
    "resume": "start",
    "unhalt": "start",
    "go": "run",
    "suggestions": "suggest",
}

#: The level each verb needs. `request` is naming the bot with free text; `typo` is a near miss
#: of a verb, which only gets a reply.
LEVELS: dict[str, int] = {
    "status": 1,
    "help": 1,
    "typo": 1,
    "build": 2,
    "revise": 2,
    "stop": 2,
    "suggest": 2,
    "request": 2,
    "halt": 3,
    "start": 3,
    "run": 3,
}

#: `--force` lifts the subscriptions' hours and nothing else (their usage caps hold). Operator only.
FORCE_FLAG = "--force"
FORCE_LEVEL = 3
MAX_COMMANDS = 10

_FENCE = re.compile(r"^\s*(```|~~~)")
_SLASH = re.compile(r"^\s*(?:@[\w-]+\s+)*/harness(?:-|[ \t]+|$)(.*)$", re.IGNORECASE)
_ISSUE_REF = re.compile(r"^#\d+$")


@dataclass(frozen=True)
class Command:
    verb: str
    args: str
    force: bool
    line: str
    #: For a `typo`: the verb or alias the word was probably meant to be.
    meant: str = ""

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


def _distance(a: str, b: str) -> int:
    """Edits from `a` to `b`, a swap of two neighbouring letters counting as one."""
    rows = [list(range(len(b) + 1))]
    for i in range(1, len(a) + 1):
        row = [i] + [0] * len(b)
        for j in range(1, len(b) + 1):
            cost = 0 if a[i - 1] == b[j - 1] else 1
            row[j] = min(rows[-1][j] + 1, row[j - 1] + 1, rows[-1][j - 1] + cost)
            if i > 1 and j > 1 and a[i - 1] == b[j - 2] and a[i - 2] == b[j - 1]:
                row[j] = min(row[j], rows[-2][j - 2] + 1)
        rows.append(row)
    return rows[-1][-1]


def misspelt(args: str) -> str | None:
    """The verb or alias a request was probably meant to be, or None.

    Only a request of one word (an `#n` aside) counts: "stauts" is a typo, but "do the thing"
    is plain words even though "do" is one letter from "go". A long word may be two edits off,
    a short one only one."""
    words = [w for w in args.split() if not _ISSUE_REF.match(w)]
    if len(words) != 1:
        return None
    word = words[0].lower().strip(":,.!?")
    if len(word) < 2 or _canonical(word):
        return None
    limit = 1 if len(word) <= 4 else 2
    best: tuple[int, str] | None = None
    for name in (*VERBS, *ALIASES):
        gap = _distance(word, name)
        if gap <= limit and (best is None or gap < best[0]):
            best = (gap, name)
    return best[1] if best else None


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
#: lift a halt). A colon right after the verb ("@bot halt: away this week") says the words are the
#: verb's own. `/harness <verb>` is always a command.
_MENTION_ARGS: dict[str, re.Pattern[str]] = {
    "status": re.compile(r"^$"),
    "help": re.compile(r"^\S*$"),
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


def _read(rest: str, line: str, *, plain_words: bool) -> Command:
    """The command in what follows a prefix. `plain_words` is the `@bot` form."""
    first, _, tail = rest.partition(" ")
    verb = _canonical(first) if first else None
    args, force = _strip_force(tail)
    if verb is not None and plain_words and not first.endswith(":"):
        shape = _MENTION_ARGS.get(verb)
        if shape is not None and not shape.match(args):
            verb = None
    if verb is not None:
        return Command(verb, args, force, line.strip())
    args, force = _strip_force(rest)
    meant = misspelt(args)
    if meant:
        return Command("typo", args, force, line.strip(), meant)
    return Command("request", args, force, line.strip())


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
            found.append(_read(slash.group(1).strip(), line, plain_words=False))
            continue
        named = mention.match(line)
        if not named:
            continue
        rest = named.group(1).strip()
        if rest.lower().startswith("/harness"):
            continue  # `@bot /harness verb` is matched by _SLASH above
        found.append(_read(rest, line, plain_words=True))
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
**Commands.** Write one per line, starting with `/harness` or `@{bot}`; the two work the same way.

| Verb | What it does | Where | Level |
|---|---|---|---|
| `build [notes]` | queue this issue for the bot | issue | 2 |
| `revise <notes>` | queue a revision of this PR with your notes | pull request | 2 |
| `stop` | take this issue or PR out of the queue and stop work on it | issue or PR | 2 |
| `suggest` | ask for improvement suggestions when the queue is empty | anywhere | 2 |
| `status` | halt state, each subscription, the queue | anywhere | 1 |
| `help [verb]` | this list, or one command in detail | anywhere | 1 |
| `halt [reason]` | stop all model work until `start` | anywhere | 3 |
| `start` | lift a halt | anywhere | 3 |
| `run [#n]` | start a run now, outside a subscription's hours if need be | anywhere | 3 |

Anything else after `/harness` or `@{bot}` is a request: a build on an issue, a revision on a PR, with your words as the notes. A single word that looks like a misspelt verb (`stauts`) runs nothing; I ask what you meant.
After `@{bot}`, a control verb followed by more words reads as plain English, so `@{bot} stop using the old sprite` is a request. Write the verb alone, or with a colon (`@{bot} halt: away this week`), for the command.
Add `--force` (level 3) to `build`, `revise` or `suggest` to start now, outside a subscription's hours if need be. Label an issue `difficulty:easy`, `difficulty:medium` (the default) or `difficulty:hard` to set the weakest model tier that may build it; `difficulty:hard` keeps it for Opus.
Labels do the same as the verbs: `bot:build` on an issue, `bot:revise` on a PR; assigning @{bot} queues the thread.
On your comment: 👀 seen · 👍 a model will read it · 🚀 answered · ❤️ a run has it · 🎉 done · 😕 it ended without an answer.
"""

#: `help <verb>`: what it takes, what it does, and an example.
VERB_HELP: dict[str, tuple[str, str, str]] = {
    "build": ("build [notes]", "Queue this issue for the bot; your notes join the request. On a "
              "pull request it is the same as `revise`.", "/harness build make the Coin spin too"),
    "revise": ("revise <notes>", "Queue a revision of this pull request with your notes. "
               "Auto-merge stays off until the revision lands.", "@{bot} revise rename the helper"),
    "stop": ("stop", "Take this issue or pull request out of the queue. A run working on it gives "
             "up at its next checkpoint and keeps what it has.", "/harness stop"),
    "suggest": ("suggest", "Ask for a suggestion survey the next time the queue is empty.",
                "@{bot} suggest"),
    "status": ("status", "Halt state, which subscriptions are running what right now, each "
               "subscription (its hours, usage, and what it is doing), and what is queued.",
               "@{bot} status"),
    "help": ("help [verb]", "The list of commands, or one of them in detail.",
             "/harness help build"),
    "halt": ("halt [reason]", "Stop all model work until `start`; a run already going stops at "
             "its next checkpoint. After `@{bot}`, a colon after the verb gives the reason.",
             "@{bot} halt: away this week"),
    "start": ("start", "Lift a halt. `start --force` also starts a run now.",
              "/harness start --force"),
    "run": ("run [#n]", "Start a run now, outside a subscription's hours if need be, for one item or "
            "whatever is next in the queue.", "@{bot} run #12"),
}

POINTER = "`/harness help` (or `@{bot} help`) lists the commands."


def help_text(bot: str, topic: str = "") -> str:
    """The reply to `help`, or to `help <verb>`."""
    words = topic.split()
    word = words[0] if words else ""
    if not word:
        return HELP.format(bot=bot)
    verb = _canonical(word)
    if verb is None:
        return f"I do not know `{word}`. " + HELP.format(bot=bot)
    usage, what, example = VERB_HELP[verb]
    level = LEVELS[verb]
    aliases = [name for name, target in ALIASES.items() if target == verb]
    also = f"; also written {', '.join(f'`{a}`' for a in aliases)}" if aliases else ""
    return (f"**`{usage}`** (level {level}, {LEVEL_NAMES.get(level, level)}{also})\n\n"
            f"{what.format(bot=bot)}\n\nFor example: `{example.format(bot=bot)}`. "
            + POINTER.format(bot=bot))
