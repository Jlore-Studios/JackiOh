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

`load_sweep` reads a shadow-ban sweep's pass-1 lines (`cargo jackioh sweep --json`) and `load_generations` the
AI's generations (`crates/ai/generation.json` and `training/history/*.jsonl`); neither is a record file.

Only the fields named below are read; a field a record grows later is ignored. A record id is unique among
records (R376), so `load` refuses a second record with an id it has read: files that overlap are the caller's
to trim, where `stats report` would count the game twice.
"""

from __future__ import annotations

import json
import re
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
    "load_generations",
    "load_sweep",
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


SWEEP_COLUMNS = {
    "card": "str",
    "tier": "str",
    "games": "int64",
    "drawn_games": "int64",
    "affordable_turns": "int64",
    "plays": "int64",
    "errors": "int64",
    "timeouts": "int64",
    "eval_delta_sum": "float64",
    "eval_delta_count": "int64",
    "eval_delta_mean": "float64",
    "flags": "object",
    "unswept": "bool",
}


def _sweep_row(line: dict) -> dict:
    count = line["evalDeltaCount"]
    return {
        "card": line["defId"],
        "tier": line["tier"],
        "games": line["games"],
        "drawn_games": line["drawnGames"],
        "affordable_turns": line["affordableTurns"],
        "plays": line["plays"],
        "errors": line["errors"],
        "timeouts": line["timeouts"],
        "eval_delta_sum": line["evalDeltaSum"],
        "eval_delta_count": count,
        # The mean change of the AI's evaluation per play; no plays, no mean (not a mean of zero).
        "eval_delta_mean": line["evalDeltaSum"] / count if count > 0 else float("nan"),
        "flags": list(line["flags"]),
        "unswept": line["unswept"],
    }


def load_sweep(paths: str | Path | Iterable[str | Path]) -> pd.DataFrame:
    """A sweep's pass-1 lines (`cargo jackioh sweep --json`) as one row per card and tier.

    Columns: card, tier, games, drawn_games, affordable_turns, plays, errors, timeouts, eval_delta_sum,
    eval_delta_count, eval_delta_mean (NaN with no plays), flags (a list of `error`, `timeout`, `neverPlayed`
    and `selfHarm`, as the sweep wrote them) and unswept (never once affordable in hand: no evidence either
    way). A pass-2 line (`--pass2`'s, which names a `forced` card) is skipped, so the files of a whole sweep
    may be named together. A blank line is skipped; a line that is not a result, or a card and tier an earlier
    line holds, raises a ValueError naming its file and line.
    """
    if isinstance(paths, (str, Path)):
        paths = [paths]
    rows: list[dict] = []
    where: dict[tuple[str, str], str] = {}
    for path in paths:
        with open(path, encoding="utf-8") as lines:
            for number, line in enumerate(lines, start=1):
                if not line.strip():
                    continue
                here = f"{path}: line {number}"
                try:
                    parsed = json.loads(line)
                    if "forced" in parsed and "defId" not in parsed:
                        continue
                    row = _sweep_row(parsed)
                except (KeyError, TypeError, ValueError) as error:
                    raise ValueError(f"{here}: not a sweep result ({error!r})") from error
                key = (row["card"], row["tier"])
                if key in where:
                    raise ValueError(f"{here}: {key[0]} at {key[1]} is also at {where[key]}")
                where[key] = here
                rows.append(row)
    return pd.DataFrame(rows, columns=list(SWEEP_COLUMNS)).astype(SWEEP_COLUMNS)


GENERATION_COLUMNS = {
    "generation": "int64",
    "lane": "str",
    "date": "str",
    "parent": "str",
    "vs_random_wins": "Int64",
    "vs_random_games": "Int64",
    "vs_parent_wins": "Int64",
    "vs_parent_games": "Int64",
    "shadow_ban": "Int64",
    "parent_shadow_ban": "Int64",
}

_OUT_OF = re.compile(r"(\d+)/(\d+)")


def _wins_of_games(value: str | None) -> tuple[int | None, int | None]:
    """A gate's "93/100" as (93, 100); null as (None, None)."""
    if value is None:
        return None, None
    found = _OUT_OF.fullmatch(value)
    if found is None:
        raise ValueError(f"{value!r} is not wins/games")
    return int(found[1]), int(found[2])


def _generation_row(line: dict) -> dict:
    vs_random = _wins_of_games(line["vsRandom"])
    vs_parent = _wins_of_games(line.get("vsParent"))
    return {
        "generation": line["generation"],
        "lane": line["lane"],
        "date": line["date"],
        "parent": line["parent"],
        "vs_random_wins": vs_random[0],
        "vs_random_games": vs_random[1],
        "vs_parent_wins": vs_parent[0],
        "vs_parent_games": vs_parent[1],
        "shadow_ban": line.get("shadowBan"),
        "parent_shadow_ban": line.get("parentShadowBan"),
    }


def load_generations(repo_root: str | Path) -> pd.DataFrame:
    """The AI's generations, oldest first: `crates/ai/generation.json` and `training/history/*.jsonl` under `repo_root`.

    One row per generation: generation, lane, date, parent (the commit it was built from, None for the
    first), vs_random_wins and vs_random_games, vs_parent_wins and vs_parent_games (NA for a generation with
    no parent) and shadow_ban and parent_shadow_ban (cards kept out of the AI's decks, R186; NA where the
    line does not say). Generations count across both lanes, so a number is one line: the history's line
    for a generation wins over `generation.json`'s, which holds the newest line again (and generation 0,
    which no history holds). A history directory that does not exist is no history; two history lines for
    one generation, or a line that cannot be read, raise a ValueError naming its file and line.
    """
    root = Path(repo_root)
    current_path = root / "crates" / "ai" / "generation.json"
    try:
        with open(current_path, encoding="utf-8") as file:
            current = _generation_row(json.load(file))
    except (KeyError, TypeError, ValueError) as error:
        raise ValueError(f"{current_path}: not a generation ({error!r})") from error
    rows: dict[int, dict] = {}
    where: dict[int, str] = {}
    for path in sorted((root / "training" / "history").glob("*.jsonl")):
        with open(path, encoding="utf-8") as lines:
            for number, line in enumerate(lines, start=1):
                if not line.strip():
                    continue
                here = f"{path}: line {number}"
                try:
                    row = _generation_row(json.loads(line))
                except (KeyError, TypeError, ValueError) as error:
                    raise ValueError(f"{here}: not a generation ({error!r})") from error
                number_of = row["generation"]
                if number_of in where:
                    raise ValueError(f"{here}: generation {number_of} is also at {where[number_of]}")
                where[number_of] = here
                rows[number_of] = row
    rows.setdefault(current["generation"], current)
    ordered = [rows[number] for number in sorted(rows)]
    return pd.DataFrame(ordered, columns=list(GENERATION_COLUMNS)).astype(GENERATION_COLUMNS)


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
