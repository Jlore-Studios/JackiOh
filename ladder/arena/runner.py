"""Seeded headless game runner (issue #55, phase 1).

``play_game`` plays one game through the bridge: decks are dealt from the
bridge's ``deck`` command on seed-derived streams (fixed decks, never built by
the agent), seats play in engine order via the bridge's ``next`` reply, and
every decision is logged with its legal set, the choice, and the acting
specialist if the agent names one (multi-context agents set
``last_specialist`` when they act; the arena only reads it).

``play_series`` plays N seeded games with seats alternated 50/50, using one
fresh seed set per generation for every opponent.

An agent that returns an action outside its ``legal`` set — or whose ``act``
raises — forfeits that game. A bridge failure raises ``BridgeError`` instead:
that is an arena crash, never a forfeit.
"""

from __future__ import annotations

import json
from typing import Any

from .bridge import Bridge, BridgeError
from .types import Action, Agent, Observation

# The engine's DECK_SIZE (packages/engine/src/config.ts): ladder games are ordinary games.
LADDER_DECK_SIZE = 20

# Safety bound on decisions in one game; the engine's turn cap ends real games first.
MAX_ACTIONS_PER_GAME = 3000

Decision = dict[str, Any]
GameLog = dict[str, Any]


def _canonical(action: Action) -> str:
    return json.dumps(action, sort_keys=True)


def _def_of(obs: Observation, instance_id: str) -> str | None:
    # Card views (``cardView``) key hand and graveyard cards by ``instanceId``
    # with the definition in ``defId``. Legal ``play`` actions name cards from
    # either zone (graveyard casts reuse the same action), so both are searched.
    you = obs.get("you", {})
    for zone in ("hand", "graveyard"):
        cards = you.get(zone, [])
        if not isinstance(cards, list):
            continue
        for card in cards:
            if isinstance(card, dict) and card.get("instanceId") == instance_id:
                def_id = card.get("defId")
                if isinstance(def_id, str):
                    return def_id
    return None


def _legal_play_defids(legal: list[Action], obs: Observation) -> list[str]:
    out: list[str] = []
    for action in legal:
        if action.get("type") == "play" and isinstance(action.get("instanceId"), str):
            def_id = _def_of(obs, action["instanceId"])
            if def_id is not None and def_id not in out:
                out.append(def_id)
    return sorted(out)


def _opponent(seat: str) -> str:
    return "p2" if seat == "p1" else "p1"


def play_game(
    bridge: Bridge,
    agents: dict[str, Agent],
    seed: str,
    game_index: int = 0,
    max_actions: int = MAX_ACTIONS_PER_GAME,
) -> GameLog:
    """Play one game; ``agents`` maps each seat to its agent."""
    game_id = f"g{game_index}-{seed}"
    decks = {}
    for seat in ("p1", "p2"):
        reply = bridge.call({"cmd": "deck", "seed": f"{seed}:{seat}-deck", "size": LADDER_DECK_SIZE})
        deck = reply.get("deck")
        if not isinstance(deck, list):
            raise BridgeError("bridge deck reply has no deck list")
        decks[seat] = deck
    bridge.call({"cmd": "new_game", "gameId": game_id, "seed": seed, "decks": [decks["p1"], decks["p2"]]})
    decisions: list[Decision] = []
    seat = "p1"
    result: dict[str, Any] | None = None
    forfeit: dict[str, Any] | None = None
    actions = 0
    game_hash: str | None = None
    try:
        while actions < max_actions:
            legal_reply = bridge.call({"cmd": "legal", "gameId": game_id, "seat": seat})
            legal = legal_reply.get("legal")
            if not isinstance(legal, list):
                raise BridgeError("bridge legal reply has no legal list")
            obs_reply = bridge.call({"cmd": "observe", "gameId": game_id, "seat": seat})
            obs = obs_reply.get("view")
            if not isinstance(obs, dict):
                raise BridgeError("bridge observe reply has no view")
            if obs.get("result") is not None:
                result = obs["result"]
                break
            if not legal:
                raise BridgeError(f"game live but {seat} has no legal actions")
            agent = agents[seat]
            try:
                choice = agent.act(obs, legal)
            except Exception as exc:
                forfeit = {"seat": seat, "reason": "threw", "error": str(exc)[:500]}
                result = {"winner": _opponent(seat), "reason": "forfeit", "forfeit": forfeit}
                break
            if not isinstance(choice, dict) or _canonical(choice) not in {_canonical(a) for a in legal}:
                forfeit = {"seat": seat, "reason": "illegal", "action": choice}
                result = {"winner": _opponent(seat), "reason": "forfeit", "forfeit": forfeit}
                break
            played = _def_of(obs, choice["instanceId"]) if choice.get("type") == "play" else None
            specialist = getattr(agent, "last_specialist", None)
            decisions.append(
                {
                    "seat": seat,
                    "legal_plays": _legal_play_defids(legal, obs),
                    "played": played,
                    "action_type": choice.get("type"),
                    "action": choice,
                    "specialist": specialist if isinstance(specialist, str) else None,
                }
            )
            act_reply = bridge.call({"cmd": "act", "gameId": game_id, "seat": seat, "action": choice})
            actions += 1
            if act_reply.get("result") is not None:
                result = act_reply["result"]
                break
            nxt = act_reply.get("next")
            seat = nxt if nxt in ("p1", "p2") else _opponent(seat)
        else:
            raise BridgeError(f"game {seed} exceeded {max_actions} actions")
        if result is None:
            raise BridgeError(f"game {seed} ended with no result")
        try:
            final = bridge.call({"cmd": "result", "gameId": game_id})
            if isinstance(final.get("hash"), str):
                game_hash = final["hash"]
        except BridgeError:
            pass
    finally:
        try:
            bridge.call({"cmd": "close", "gameId": game_id})
        except BridgeError:
            pass
    if result is None:
        raise BridgeError(f"game {seed} ended with no result")
    return {
        "seed": seed,
        "winner": result.get("winner"),
        "reason": result.get("reason"),
        "forfeit": forfeit,
        "decisions": decisions,
        "actions": actions,
        "decks": decks,
        "built_by_agent": False,
        "hash": game_hash,
    }


def play_series(
    bridge: Bridge,
    candidate: Agent,
    opponent: Agent,
    seeds: list[str],
    max_actions: int = MAX_ACTIONS_PER_GAME,
) -> dict[str, Any]:
    """Play one game per seed; the candidate sits p1 on even games, p2 on odd ones."""
    logs: list[GameLog] = []
    wins = 0
    for i, seed in enumerate(seeds):
        candidate_seat = "p1" if i % 2 == 0 else "p2"
        agents = {candidate_seat: candidate, _opponent(candidate_seat): opponent}
        log = play_game(bridge, agents, seed, game_index=i, max_actions=max_actions)
        log["candidate_seat"] = candidate_seat
        logs.append(log)
        if log["winner"] == candidate_seat:
            wins += 1
    return {"wins": wins, "games": len(seeds), "logs": logs}
