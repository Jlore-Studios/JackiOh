"""The promotion gates (issue #55, phase 2): a candidate ships only if ALL pass.

1. Wins >= threshold against EACH current champion (0, 1 or 2 exist; only the
   checks that exist apply).
2. Wins >= threshold against the random bot.
3. Shadowbanned cards <= max fraction of the current card pool.
4. With at least one champion, the candidate's shadowban count is strictly
   less than champion 1's — except when champion 1 has 0, when the candidate
   must also have 0.

All thresholds live in ``ladder/config.yaml`` and arrive here in ``config``.
"""

from __future__ import annotations

from typing import Any


def evaluate_gates(
    wins_vs_champions: list[int],
    wins_vs_random: int,
    candidate_shadowbanned: int,
    champion_shadowbanned: list[int],
    pool_size: int,
    config: dict[str, Any],
) -> dict[str, Any]:
    """Apply every gate that exists; ``failures`` names each one missed."""
    games = config.get("games_per_opponent", 100)
    need_champion = config.get("win_threshold_vs_champion", 75)
    need_random = config.get("win_threshold_vs_random", 90)
    max_fraction = config.get("max_shadowban_fraction", 0.10)

    failures: list[str] = []
    for i, wins in enumerate(wins_vs_champions):
        if wins < need_champion:
            failures.append(
                f"champion-{i + 1}: {wins}/{games} wins, need {need_champion}"
            )
    if wins_vs_random < need_random:
        failures.append(f"random: {wins_vs_random}/{games} wins, need {need_random}")

    fraction = (candidate_shadowbanned / pool_size) if pool_size else 0.0
    if fraction > max_fraction:
        failures.append(
            f"shadowban: {candidate_shadowbanned}/{pool_size} cards "
            f"({fraction:.1%}), max {max_fraction:.0%}"
        )

    if champion_shadowbanned:
        champ1 = champion_shadowbanned[0]
        if champ1 == 0:
            if candidate_shadowbanned != 0:
                failures.append(
                    f"shadowban-parity: champion 1 has 0 shadowbans, "
                    f"candidate has {candidate_shadowbanned}"
                )
        elif not candidate_shadowbanned < champ1:
            failures.append(
                f"shadowban-improvement: candidate has {candidate_shadowbanned} "
                f"shadowbans, need strictly fewer than champion 1's {champ1}"
            )

    return {"promoted": not failures, "failures": failures}
