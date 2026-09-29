"""Durable state: one JSON file on the unprotected `bot-state` branch.

Written through the Contents API with the blob sha as a compare-and-set, so the event workflow
and a night run can both update it without a shared lock: a write that lost the race re-reads
and re-applies its change.
"""

from __future__ import annotations

import copy
import json
from datetime import datetime, timedelta
from typing import Any, Callable

from harness.clock import iso, parse_iso
from harness.config import STATE_BRANCH, STATE_FILE
from harness.errors import GitHubError, StateConflict

MAX_ATTEMPTS = 6
WINDOW_NAMES = {"five_hour": "5-hour", "seven_day": "7-day"}


def default_state() -> dict[str, Any]:
    return {
        "version": 1,
        "halted": False,
        "halt": {},
        "usage": None,
        "rate_limited_until": None,
        "items": {},
        "suggest": {"last_run": None, "requested": False},
        "last_run": None,
    }


def normalise(data: Any) -> dict[str, Any]:
    """`data` with every key `default_state` has, so readers never meet a missing key."""
    state = default_state()
    if isinstance(data, dict):
        for key, value in data.items():
            state[key] = value
    if not isinstance(state.get("items"), dict):
        state["items"] = {}
    if not isinstance(state.get("suggest"), dict):
        state["suggest"] = {"last_run": None, "requested": False}
    return state


class StateStore:
    """Reads and updates the state file on a branch through a `GitHub` client."""

    def __init__(self, gh: Any, branch: str = STATE_BRANCH, path: str = STATE_FILE) -> None:
        self.gh = gh
        self.branch = branch
        self.path = path

    def load(self) -> dict[str, Any]:
        text, _ = self.gh.get_file(self.path, self.branch)
        if text is None:
            return default_state()
        try:
            return normalise(json.loads(text))
        except ValueError:
            return default_state()

    def ensure(self) -> bool:
        """Create the branch with an empty state when it does not exist. True when created."""
        if self.gh.branch_sha(self.branch) is not None:
            return False
        self.gh.create_orphan_branch(
            self.branch, self.path, _dump(default_state()), "state: start [skip ci]"
        )
        return True

    def update(self, change: Callable[[dict[str, Any]], Any], message: str = "state") -> dict:
        """Apply `change` (which mutates its argument) and write the result. Returns the state."""
        for _ in range(MAX_ATTEMPTS):
            text, sha = self.gh.get_file(self.path, self.branch)
            before = default_state() if text is None else normalise(_loads(text))
            after = copy.deepcopy(before)
            change(after)
            if after == before and text is not None:
                return after
            if text is None and self.gh.branch_sha(self.branch) is None:
                self.ensure()
                continue
            try:
                self.gh.put_file(
                    self.path, _dump(after), branch=self.branch, sha=sha,
                    message=f"{message} [skip ci]",
                )
                return after
            except GitHubError as exc:
                if exc.status in (409, 422):
                    continue
                raise
        raise StateConflict(f"{self.path} on {self.branch} kept changing; gave up after "
                            f"{MAX_ATTEMPTS} attempts")


def _loads(text: str) -> Any:
    try:
        return json.loads(text)
    except ValueError:
        return {}


def _dump(state: dict[str, Any]) -> str:
    return json.dumps(state, indent=2, sort_keys=True) + "\n"


# ---------------------------------------------------------------------- helpers on a state dict


def item(state: dict[str, Any], number: int) -> dict[str, Any]:
    """The record for one issue or PR, created empty when missing."""
    return state["items"].setdefault(str(int(number)), {})


def usage_refusal(state: dict[str, Any], stops: dict[str, float], at: datetime) -> str | None:
    """Why the last usage reading says to start no model call now, or None."""
    until = parse_iso(state.get("rate_limited_until"))
    if until is not None and until > at:
        return f"the subscription refused a call; it resets at {iso(until)}"
    usage = state.get("usage") or {}
    for window, stop in stops.items():
        reading = usage.get(window) if isinstance(usage, dict) else None
        if not isinstance(reading, dict):
            continue
        resets = parse_iso(reading.get("resets_at"))
        if resets is not None and resets <= at:
            continue  # that window has reset since the reading
        utilization = reading.get("utilization")
        if isinstance(utilization, (int, float)) and utilization >= stop:
            when = f"; it resets at {iso(resets)}" if resets else ""
            name = WINDOW_NAMES.get(window, window)
            return f"{name} usage is {utilization:.0%}, at or over the {stop:.0%} stop{when}"
    return None


def record_usage(state: dict[str, Any], usage: dict | None, reset_at: str | None,
                 at: datetime) -> None:
    """Keep the newest usage reading and any refusal."""
    if isinstance(usage, dict) and usage:
        state["usage"] = {**usage, "observed_at": iso(at)}
    if reset_at:
        resets = parse_iso(reset_at)
        if resets is None and reset_at.startswith("+PT"):
            resets = at + _duration(reset_at)
        if resets is not None:
            state["rate_limited_until"] = iso(resets)


def _duration(text: str) -> timedelta:
    amount = int("".join(ch for ch in text if ch.isdigit()) or "30")
    return timedelta(hours=amount) if text.upper().endswith("H") else timedelta(minutes=amount)
