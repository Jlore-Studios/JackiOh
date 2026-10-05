"""Ladder agent contract (issue #55, phase 1).

All agents implement one contract: ``act(obs, legal) -> Action`` where ``legal``
is computed by the engine. An action not in ``legal`` forfeits the game.
Agents must be deterministic given a seed.

``Observation`` and ``Action`` are the bridge's JSON: ``observe`` returns the
engine's ``viewFor`` for the seat (so an agent never sees hidden cards), and an
action is the engine's own ``ActionBody`` JSON, passed through unchanged.
"""

from __future__ import annotations

from typing import Any, Protocol

Observation = dict[str, Any]
Action = dict[str, Any]


class Agent(Protocol):
    """One decision: pick an element of ``legal`` from ``obs``."""

    def act(self, obs: Observation, legal: list[Action]) -> Action:
        ...
