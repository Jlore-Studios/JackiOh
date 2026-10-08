"""Which bot this process is (#60): the night bot, or Squishy, from the one harness.

Both bots run this code. Each workflow names its bot's home directory in `HARNESS_HOME`:
`.harness` (the default) for the night bot, `.squishy` for Squishy. The home holds that bot's
`config.json` and `providers.json`, and the config's `identity` section says what sets the bot
apart from the other: its labels' prefix, its slash command, its branches, its state and journal
branches, its workflow, the marker on everything it writes, and the modes it has beyond building.
A config without the section is the night bot's, as it always was.

The rest of the harness reads these through `config.py`'s constants (`LABEL_BUILD`, `SLASH`,
`STATE_BRANCH`, …), which are fixed when the process starts: one process is one bot. Code that
needs the other bot (the night bot's status issue carries Squishy's section, and its status loop
sweeps for Squishy too) starts a second process with the other home (`__main__._companion`).

This module and `config.py` are the only ones that read `os.environ`.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Mapping

#: The repository root: `bot/harness/identity.py` sits two directories below it.
REPO_ROOT = Path(__file__).resolve().parents[2]

#: The modes a bot may have beyond a plain build (#60): `oneshot` builds an issue in one run with
#: fullsend, `split` breaks it into sub-issues the bot builds itself, `split-bot` into sub-issues
#: for the other bot, and `fullsend` (#505) into parts the bot lands on one branch of the issue's
#: own, which a last run then reconciles into one pull request.
MODES = ("oneshot", "split", "split-bot", "fullsend")


@dataclass(frozen=True)
class Other:
    """Another bot in the same repository: the issues it owns this one leaves alone (its queue
    labels on them, or assigned to its account), and, with `home`, the status section and sweep
    the night bot's status loop runs for it."""

    name: str
    login: str
    label_prefix: str
    home: str = ""
    #: The environment variable that holds its GitHub token, for a process started as it
    #: (`__main__._companion`).
    token_env: str = ""


@dataclass(frozen=True)
class Identity:
    #: How it names itself in a sentence ("the night bot", "Squishy") and at the head of a title.
    name: str = "the night bot"
    title: str = "Night bot"
    #: The slash command, without its slash: `/harness build`, `/squishy build`.
    command: str = "harness"
    label_prefix: str = "bot:"
    branch_prefix: str = "bot/"
    state_branch: str = "bot-state"
    journal_branch: str = "bot-journal"
    #: The workflow a run is (`bot-night.yml`): dispatched to start one, read to list the runs.
    workflow: str = "bot-night.yml"
    marker: str = "<!-- jackioh-bot -->"
    #: Where the trust list is: the bots share one.
    trust: str = ".harness/trust.txt"
    modes: tuple[str, ...] = ()
    others: tuple[Other, ...] = field(default_factory=tuple)
    #: Whether it makes suggestions (the config's `suggestions.enabled`): Squishy does not, so it
    #: has no labels for them.
    suggestions: bool = True

    @property
    def slash(self) -> str:
        return f"/{self.command}"

    @property
    def commit_prefix(self) -> str:
        """What its commits' messages start with: `bot: build pass 1 for #85`."""
        return self.label_prefix.rstrip(":")


def home() -> str:
    """The bot's home directory, relative to the repository root."""
    found = os.environ.get("HARNESS_HOME", "").strip().strip("/")
    return found or ".harness"


def _other(raw: Any, where: str) -> Other:
    if not isinstance(raw, Mapping):
        raise ValueError(f"{where}: expected an object")
    try:
        return Other(str(raw["name"]), str(raw["login"]), str(raw["label_prefix"]),
                     home=str(raw.get("home") or ""), token_env=str(raw.get("token_env") or ""))
    except KeyError as exc:
        raise ValueError(f"{where}: needs name, login and label_prefix ({exc})") from exc


KEYS = ("name", "title", "command", "label_prefix", "branch_prefix", "state_branch",
        "journal_branch", "workflow", "marker", "trust", "modes", "others")


def parse(raw: Any, *, suggestions: bool = True) -> Identity:
    """An `Identity` from a config's `identity` section (None or {} is the night bot's)."""
    if raw is None:
        return Identity(suggestions=suggestions)
    if not isinstance(raw, Mapping):
        raise ValueError("identity: expected an object")
    unknown = sorted(set(raw) - set(KEYS))
    if unknown:
        raise ValueError(f"identity: unknown keys {', '.join(unknown)}")
    modes = tuple(str(m) for m in raw.get("modes") or ())
    bad = [m for m in modes if m not in MODES]
    if bad:
        raise ValueError(f"identity.modes: {', '.join(bad)} not one of {', '.join(MODES)}")
    fields = {k: str(raw[k]) for k in KEYS if k in raw and k not in ("modes", "others")}
    others = tuple(_other(o, f"identity.others[{i}]") for i, o in enumerate(raw.get("others") or ()))
    found = Identity(**fields, modes=modes, others=others, suggestions=suggestions)
    if not found.label_prefix.endswith(":") or not found.branch_prefix.endswith("/"):
        raise ValueError("identity: label_prefix ends with `:` and branch_prefix with `/`")
    return found


def load(root: Path | None = None, where: str | None = None) -> Identity:
    """The identity in `<root>/<home>/config.json`; the night bot's when the file or the section
    is missing (a test's temporary root, say). A section that is there but wrong is an error."""
    path = Path(root or REPO_ROOT) / (where or home()) / "config.json"
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return Identity()
    if not isinstance(raw, dict):
        return Identity()
    wanted = raw.get("suggestions") if isinstance(raw.get("suggestions"), dict) else {}
    return parse(raw.get("identity"), suggestions=bool(wanted.get("enabled", True)))


HOME = home()
CURRENT = load()
