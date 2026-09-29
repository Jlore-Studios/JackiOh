"""Who may command the bot, read from `.harness/trust.txt`. Fails closed.

Each line is `<login> <level> [id:<numeric GitHub user id>]`. A listed login is honoured only
when GitHub also vouches for it: either the id matches the account that acted, or GitHub reports
that account as OWNER, MEMBER or COLLABORATOR of the repository. Levels: 3 operator,
2 maintainer, 1 asker, 0 nobody.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path

TRUSTED_ASSOCIATIONS = frozenset({"OWNER", "MEMBER", "COLLABORATOR"})
MAX_LEVEL = 3
LEVEL_NAMES = {0: "nobody", 1: "asker", 2: "maintainer", 3: "operator"}

_LOGIN = re.compile(r"^[A-Za-z0-9-]{1,39}$")
_ID = re.compile(r"^id:([1-9][0-9]{0,19})$", re.IGNORECASE)


@dataclass(frozen=True)
class Entry:
    login: str
    level: int
    user_id: int | None


@dataclass(frozen=True)
class Trust:
    entries: dict[str, Entry] = field(default_factory=dict)
    problems: tuple[str, ...] = ()

    @classmethod
    def parse(cls, text: str) -> "Trust":
        entries: dict[str, Entry] = {}
        problems: list[str] = []
        for number, raw in enumerate(text.splitlines(), start=1):
            line = raw.split("#", 1)[0].strip()
            if not line:
                continue
            parts = line.split()
            login = parts[0].lstrip("@")
            if not _LOGIN.match(login):
                problems.append(f"line {number}: {login!r} is not a GitHub login")
                continue
            level = 1
            user_id: int | None = None
            bad = False
            for token in parts[1:]:
                if token.isascii() and token.isdigit():
                    level = int(token)
                    if level > MAX_LEVEL:
                        problems.append(f"line {number}: level {level} is above {MAX_LEVEL}")
                        bad = True
                    continue
                match = _ID.match(token)
                if match:
                    user_id = int(match.group(1))
                    continue
                problems.append(f"line {number}: cannot read {token!r}")
                bad = True
            if not bad:
                entries[login.lower()] = Entry(login, level, user_id)
        return cls(entries, tuple(problems))

    @classmethod
    def load(cls, path: Path) -> "Trust":
        try:
            return cls.parse(Path(path).read_text(encoding="utf-8"))
        except FileNotFoundError:
            return cls({}, (f"{path} does not exist; nobody may command the bot",))

    def level(self, login: str, user_id: int | None = None, association: str | None = None) -> int:
        """The level GitHub and the file together grant this account. 0 when either refuses."""
        entry = self.entries.get(str(login or "").lstrip("@").lower())
        if entry is None:
            return 0
        if entry.user_id is not None:
            # A pinned line counts for that account only; no id to compare is a refusal.
            return entry.level if user_id is not None and int(user_id) == entry.user_id else 0
        if str(association or "").upper() in TRUSTED_ASSOCIATIONS:
            return entry.level
        return 0

    def logins(self) -> list[str]:
        return sorted(e.login for e in self.entries.values())
