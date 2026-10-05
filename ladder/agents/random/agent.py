"""The random bot (issue #55, phase 1): the reference implementation behind the
``Agent`` contract. Picks uniformly from ``legal`` with its own seeded
``random.Random``, so games are reproducible from the seed alone.

It never takes ``concede``, ``offerDraw`` or ``answerDraw``: those end or
spoiler a game rather than play it, and the engine's own random policy skips
exactly these three (``AI_SKIPPED_ACTIONS``, SPEC §10.7, R84). A bot that
resigns the coin-flip turns when ``legal`` is ``{endTurn, concede}`` would be
no baseline at all. See DECISIONS.md.
"""

from __future__ import annotations

import random

from arena.types import Action, Observation

# The engine's AI_SKIPPED_ACTIONS (packages/engine/src/subsystems/aiPolicy.ts).
SKIPPED_ACTION_TYPES = frozenset({"concede", "offerDraw", "answerDraw"})


class RandomAgent:
    """Reference agent: uniform choice over playable ``legal``, deterministic in ``seed``."""

    def __init__(self, seed: str | int = "random") -> None:
        self._rng = random.Random(seed)
        self.last_specialist: str | None = None

    def act(self, obs: Observation, legal: list[Action]) -> Action:
        _ = obs
        self.last_specialist = None
        playable = [a for a in legal if a.get("type") not in SKIPPED_ACTION_TYPES]
        return self._rng.choice(playable or legal)
