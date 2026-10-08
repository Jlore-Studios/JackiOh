"""Records a test writes by hand: `record(id, p1, p2)` is a game, `seat(deck, …)` one side of it."""

import json
from pathlib import Path


def write_records(path: Path, records: list[dict]) -> Path:
    path.write_text("".join(json.dumps(record) + "\n" for record in records), encoding="utf-8")
    return path


def seat(deck, opening=(), drawn=(), played=(), played_turns=None):
    summary = {"deck": list(deck), "opening": list(opening), "drawn": list(drawn), "played": list(played)}
    if played_turns is not None:
        summary["playedTurns"] = list(played_turns)
    return summary


def record(id, p1, p2, winner="p1", source="live", mode="bo1", patch="v0.1.1", pilots=("human", "human")):
    return {
        "id": id,
        "source": source,
        "mode": mode,
        "patch": patch,
        "pilots": {"p1": pilots[0], "p2": pilots[1]},
        "game": {
            "first": "p1",
            "winner": winner,
            "reason": "turn-cap" if winner == "draw" else "hero-death",
            "turns": 9,
            "seats": {"p1": p1, "p2": p2},
        },
    }
