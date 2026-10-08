"""JackiOh's game records as pandas tables (SPEC §9.11, R376-R378).

A record file holds one GameRecord per line, as `jackioh-server stats-export`, `cargo jackioh stats --out`
and the training lanes write it. `load` turns files into three tables, and `card_stats` computes R377's
card win rates over them the way `jackioh_engine::card_stats` does (`cargo jackioh stats report` prints
the Rust's figures for the same files).

    records = load(["live.jsonl", "dev.jsonl"])
    records.games       one row per record
    records.seats       one row per game and seat
    records.seat_cards  one row per game, seat and distinct card in that seat's deck
    card_stats(records, source="all", mode="random")

Only the fields named below are read; a field a record grows later is ignored. A record id is unique among
records (R376), so `load` refuses a second record with an id it has read: files that overlap are the caller's
to trim, where `stats report` would count the game twice.
"""

from __future__ import annotations

import json
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path

import pandas as pd

__all__ = [
    "BREAKDOWNS",
    "CATALOG_PATH",
    "Records",
    "card_stats",
    "load",
    "load_catalog",
]

SEATS = ("p1", "p2")
SOURCES = ("live", "dev")
SOURCE_FILTERS = ("live", "dev", "all")
MODES = ("bo1", "bo3", "random")
PILOT_FILTERS = ("human", "ai", "unified")

# R377's breakdowns, in column order.
BREAKDOWNS = ("in_deck", "opening_hand", "going_first", "going_second", "played", "drawn_not_played")

CATALOG_PATH = Path(__file__).resolve().parents[2] / "crates" / "cards" / "catalog.json"

# What a played delta and a rate are measured in: R377's "percentage points".
POINTS = 100.0

GAMES_COLUMNS = {
    "id": "str",
    "source": "str",
    "mode": "str",
    "patch": "str",
    "first": "str",
    "winner": "str",
    "reason": "str",
    "turns": "int64",
}
SEATS_COLUMNS = {
    "game": "str",
    "seat": "str",
    "pilot": "str",
    "went_first": "bool",
    "result": "str",
}
SEAT_CARDS_COLUMNS = {
    "game": "str",
    "seat": "str",
    "card": "str",
    "in_opening": "bool",
    "drawn": "bool",
    "played": "bool",
    "first_played_turn": "Int64",
}


@dataclass(frozen=True)
class Records:
    """The three tables a set of record files makes. `game` joins `seats` and `seat_cards` to `games.id`."""

    games: pd.DataFrame
    seats: pd.DataFrame
    seat_cards: pd.DataFrame


def _table(rows: list[dict], columns: dict[str, str]) -> pd.DataFrame:
    return pd.DataFrame(rows, columns=list(columns)).astype(columns)


def _result(winner: str, seat: str) -> str:
    """A seat's outcome: a draw is a game not won (R377)."""
    if winner == "draw":
        return "draw"
    return "win" if winner == seat else "loss"


def _first_turns(played: list[str], turns: list[int] | None) -> dict[str, int]:
    """The turn each card was first played on; `playedTurns` runs beside `played`, and older files lack it."""
    first: dict[str, int] = {}
    if turns is None:
        return first
    for card, turn in zip(played, turns, strict=True):
        if card not in first or turn < first[card]:
            first[card] = turn
    return first


def _read_record(record: dict, games: list[dict], seats: list[dict], seat_cards: list[dict]) -> None:
    game = record["game"]
    game_id = record["id"]
    winner = game["winner"]
    games.append(
        {
            "id": game_id,
            "source": record["source"],
            "mode": record["mode"],
            "patch": record["patch"],
            "first": game["first"],
            "winner": winner,
            "reason": game["reason"],
            "turns": game["turns"],
        }
    )
    for seat in SEATS:
        summary = game["seats"][seat]
        seats.append(
            {
                "game": game_id,
                "seat": seat,
                "pilot": record["pilots"][seat],
                "went_first": game["first"] == seat,
                "result": _result(winner, seat),
            }
        )
        opening = set(summary["opening"])
        drawn = set(summary["drawn"])
        played = set(summary["played"])
        first_played = _first_turns(summary["played"], summary.get("playedTurns"))
        # Each deck once per game, in the order the deck first names a card (R377).
        for card in dict.fromkeys(summary["deck"]):
            seat_cards.append(
                {
                    "game": game_id,
                    "seat": seat,
                    "card": card,
                    "in_opening": card in opening,
                    "drawn": card in drawn,
                    "played": card in played,
                    "first_played_turn": first_played.get(card),
                }
            )


def load(paths: str | Path | Iterable[str | Path]) -> Records:
    """Every record of every file, in the order the files are named and the lines written.

    A blank line is skipped. A line that is not a record, or whose id an earlier line holds, raises a
    ValueError naming its file and line.
    """
    if isinstance(paths, (str, Path)):
        paths = [paths]
    games: list[dict] = []
    seats: list[dict] = []
    seat_cards: list[dict] = []
    where: dict[str, str] = {}
    for path in paths:
        with open(path, encoding="utf-8") as lines:
            for number, line in enumerate(lines, start=1):
                if not line.strip():
                    continue
                here = f"{path}: line {number}"
                try:
                    record = json.loads(line)
                    game_id = record["id"]
                    _read_record(record, games, seats, seat_cards)
                except (KeyError, TypeError, ValueError) as error:
                    raise ValueError(f"{here}: not a game record ({error!r})") from error
                if game_id in where:
                    raise ValueError(f"{here}: record {game_id!r} is also at {where[game_id]}")
                where[game_id] = here
    return Records(
        games=_table(games, GAMES_COLUMNS),
        seats=_table(seats, SEATS_COLUMNS),
        seat_cards=_table(seat_cards, SEAT_CARDS_COLUMNS),
    )


def load_catalog(path: str | Path = CATALOG_PATH) -> pd.DataFrame:
    """The catalog (`crates/cards/catalog.json`) as one row per card, indexed by id.

    Columns: name, set, rarity, cost, type, tags (a tuple of strings). Tokens are in it, with rarity "Token".
    """
    with open(path, encoding="utf-8") as file:
        catalog = json.load(file)
    rows = [
        {
            "id": card["id"],
            "name": card["name"],
            "set": card["set"],
            "rarity": card["rarity"],
            "cost": card["cost"],
            "type": card["type"],
            "tags": tuple(card["tags"]),
        }
        for card in catalog.values()
    ]
    return pd.DataFrame(rows, columns=["id", "name", "set", "rarity", "cost", "type", "tags"]).set_index("id")


def _one_of(name: str, value: str | None, allowed: tuple[str, ...]) -> None:
    if value is not None and value not in allowed:
        raise ValueError(f"{name} must be one of {', '.join(allowed)} (got {value!r})")


def card_stats(
    records: Records,
    source: str = "live",
    mode: str | None = None,
    patch: str | None = None,
    pilot: str = "unified",
) -> pd.DataFrame:
    """R377: every card's breakdowns over the records the arguments read, one row per card by catalog id.

    The arguments are `stats-cards`' flags and defaults (R378: live games unless `source` names `dev` or
    `all`; `mode` and `patch` None read every mode and patch; `pilot` "unified" counts every seat). A game
    is one deck in one game: a game in which both decks held a card counts once for each, and a draw is a
    game not won. For each breakdown in `BREAKDOWNS` the frame has `<breakdown>_games`, `_wins`, `_draws`
    and `_rate` (wins over games, NaN with no games); `played_delta` is the played win rate minus the
    drawn-but-not-played win rate in percentage points, NaN while either has no games. The frame's
    `attrs` hold `games` (records read with at least one seat counted) and `decks` (seats counted).
    """
    _one_of("source", source, SOURCE_FILTERS)
    _one_of("mode", mode, MODES)
    _one_of("pilot", pilot, PILOT_FILTERS)

    games = records.games
    read = games["source"].isin(SOURCES if source == "all" else (source,))
    if mode is not None:
        read &= games["mode"] == mode
    if patch is not None:
        read &= games["patch"] == patch

    seats = records.seats[records.seats["game"].isin(games.loc[read, "id"])]
    if pilot != "unified":
        seats = seats[seats["pilot"] == pilot]

    counted = seats.merge(records.seat_cards, on=["game", "seat"], how="inner")
    counted["win"] = (counted["result"] == "win").astype("int64")
    counted["draw"] = (counted["result"] == "draw").astype("int64")
    in_hand = counted["in_opening"] | counted["drawn"]
    masks = {
        "in_deck": pd.Series(True, index=counted.index),
        "opening_hand": counted["in_opening"],
        "going_first": counted["went_first"],
        "going_second": ~counted["went_first"],
        "played": counted["played"],
        "drawn_not_played": in_hand & ~counted["played"],
    }

    cards = pd.Index(sorted(counted["card"].unique()), name="card", dtype="str")
    stats = pd.DataFrame(index=cards)
    for breakdown in BREAKDOWNS:
        held = counted[masks[breakdown]].groupby("card")
        tally = pd.DataFrame(
            {
                f"{breakdown}_games": held["card"].size(),
                f"{breakdown}_wins": held["win"].sum(),
                f"{breakdown}_draws": held["draw"].sum(),
            }
        ).reindex(cards, fill_value=0)
        tally = tally.astype("int64")
        tally[f"{breakdown}_rate"] = tally[f"{breakdown}_wins"] / tally[f"{breakdown}_games"].where(
            tally[f"{breakdown}_games"] > 0
        )
        stats = stats.join(tally)
    stats["played_delta"] = POINTS * (stats["played_rate"] - stats["drawn_not_played_rate"])
    stats = stats.reset_index()
    stats.attrs["games"] = int(seats["game"].nunique())
    stats.attrs["decks"] = len(seats)
    stats.attrs["filter"] = {"source": source, "mode": mode, "patch": patch, "pilot": pilot}
    return stats
