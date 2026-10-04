"""The shadowban audit (issue #55, phase 2): exactly the issue's definition.

Across all gauntlet games for a candidate, for each card in the pool:

- ``opportunities`` is the number of decisions where playing that card was in
  ``legal`` (the decision log's ``legal_plays``).
- ``plays`` is the number of times the agent chose it (the log's ``played``).
- A card is shadowbanned when ``opportunities >= min_opportunities`` and
  ``plays / opportunities < max_use_rate``.
- A card below ``min_opportunities`` is ``unobserved`` with fixed decks, and
  shadowbanned when the agent builds decks (else it could dodge the metric by
  never drafting awkward cards).
- When the agent builds decks, a pool card drafted in no gauntlet game is also
  shadowbanned.

All thresholds come from ``ladder/config.yaml``; nothing is hardcoded.
``agent_builds_decks: auto`` reads the arena logs' ``built_by_agent`` flag.
"""

from __future__ import annotations

from typing import Any

GameLog = dict[str, Any]


def resolve_agent_builds_decks(setting: bool | str, logs: list[GameLog]) -> bool:
    """Resolve ``auto`` from the arena logs; ``true``/``false`` override."""
    if setting is True:
        return True
    if setting is False:
        return False
    if setting == "auto":
        return any(bool(log.get("built_by_agent")) for log in logs)
    raise ValueError(f"agent_builds_decks must be true, false or auto, not {setting!r}")


def compute_shadowban(
    logs: list[GameLog],
    card_pool: list[str],
    min_opportunities: int,
    max_use_rate: float,
    agent_builds_decks: bool,
) -> dict[str, Any]:
    """Audit one candidate's gauntlet logs against the card pool."""
    opportunities: dict[str, int] = {card: 0 for card in card_pool}
    plays: dict[str, int] = {card: 0 for card in card_pool}
    drafted: set[str] = set()
    for log in logs:
        decks = log.get("decks", {})
        if isinstance(decks, dict):
            for deck in decks.values():
                if isinstance(deck, list):
                    drafted.update(c for c in deck if isinstance(c, str))
        for decision in log.get("decisions", []):
            seen = set()
            for card in decision.get("legal_plays", []):
                if card in opportunities and card not in seen:
                    opportunities[card] += 1
                    seen.add(card)
            played = decision.get("played")
            if played in plays:
                plays[played] += 1

    shadowbanned: list[str] = []
    unobserved: list[str] = []
    stats: dict[str, dict[str, int | float]] = {}
    for card in card_pool:
        opp = opportunities[card]
        used = plays[card]
        rate = (used / opp) if opp else 0.0
        stats[card] = {"opportunities": opp, "plays": used, "use_rate": rate}
        if opp >= min_opportunities:
            if rate < max_use_rate:
                shadowbanned.append(card)
        elif agent_builds_decks:
            shadowbanned.append(card)
        else:
            unobserved.append(card)

    if agent_builds_decks:
        for card in card_pool:
            if card not in drafted and card not in shadowbanned:
                shadowbanned.append(card)

    return {
        "shadowbanned": sorted(shadowbanned),
        "unobserved": sorted(unobserved),
        "stats": stats,
        "agent_builds_decks": agent_builds_decks,
    }


def audit_candidate(
    logs: list[GameLog],
    card_pool: list[str],
    config: dict[str, Any],
) -> dict[str, Any]:
    """The config-driven entry point: thresholds from ``config['shadowban']``."""
    shadowban = config.get("shadowban", {})
    return compute_shadowban(
        logs,
        card_pool,
        min_opportunities=shadowban.get("min_opportunities", 20),
        max_use_rate=shadowban.get("max_use_rate", 0.05),
        agent_builds_decks=resolve_agent_builds_decks(
            shadowban.get("agent_builds_decks", "auto"), logs
        ),
    )
