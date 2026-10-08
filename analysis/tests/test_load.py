"""The loader: record files into the `games`, `seats` and `seat_cards` tables."""

import json
from pathlib import Path

import pandas as pd
import pytest

from helpers import record, seat, write_records
from jackioh_records import load

FIXTURES = Path(__file__).resolve().parents[1] / "fixtures"


def two_games():
    return [
        record(
            "g1",
            seat(["a", "b", "b", "c"], ["a"], ["b"], ["a", "b", "a"], [2, 3, 5]),
            seat(["a", "z"], ["core-t-coin", "z"], [], []),
            pilots=("human", "ai"),
        ),
        record("g2", seat(["a"]), seat(["a"]), winner="draw", source="dev", mode="random", patch="v0.2.5"),
    ]


def test_one_row_per_game_seat_and_distinct_card_in_each_deck(tmp_path):
    records = load(write_records(tmp_path / "two.jsonl", two_games()))

    assert list(records.games["id"]) == ["g1", "g2"]
    assert records.games.set_index("id").loc["g2"].to_dict() == {
        "source": "dev",
        "mode": "random",
        "patch": "v0.2.5",
        "first": "p1",
        "winner": "draw",
        "reason": "turn-cap",
        "turns": 9,
    }
    assert records.games["turns"].dtype == "int64"

    seats = records.seats
    assert [(row.game, row.seat) for row in seats.itertuples()] == [
        ("g1", "p1"),
        ("g1", "p2"),
        ("g2", "p1"),
        ("g2", "p2"),
    ]
    assert list(seats["pilot"]) == ["human", "ai", "human", "human"]
    assert list(seats["went_first"]) == [True, False, True, False]
    assert list(seats["result"]) == ["win", "loss", "draw", "draw"]

    cards = records.seat_cards
    # g1's p1 deck names b twice and is one row: a, b, c; g1's p2 holds a and z; g2 holds a twice over.
    assert [(row.game, row.seat, row.card) for row in cards.itertuples()] == [
        ("g1", "p1", "a"),
        ("g1", "p1", "b"),
        ("g1", "p1", "c"),
        ("g1", "p2", "a"),
        ("g1", "p2", "z"),
        ("g2", "p1", "a"),
        ("g2", "p2", "a"),
    ]
    g1_p1 = cards[(cards["game"] == "g1") & (cards["seat"] == "p1")].set_index("card")
    assert list(g1_p1["in_opening"]) == [True, False, False]
    assert list(g1_p1["drawn"]) == [False, True, False]
    assert list(g1_p1["played"]) == [True, True, False]
    # The turn a card was first played on; playedTurns runs beside played, and a card never played has none.
    assert g1_p1["first_played_turn"].dtype == pd.Int64Dtype()
    assert g1_p1["first_played_turn"].iloc[0] == 2
    assert g1_p1["first_played_turn"].iloc[1] == 3
    assert pd.isna(g1_p1["first_played_turn"].iloc[2])
    # The Coin is in an opening hand and in no deck, so it has no row.
    assert "core-t-coin" not in set(cards["card"])
    # A record with no playedTurns (a file older than the field) has no first turn, and is not an error.
    assert cards[cards["game"] == "g2"]["first_played_turn"].isna().all()


def test_reads_several_files_in_the_order_named_skips_blank_lines_and_ignores_fields_it_does_not_know(tmp_path):
    first, second = two_games()
    second["somethingNew"] = {"added": "later"}
    second["game"]["seats"]["p1"]["alsoNew"] = [1]
    a = tmp_path / "a.jsonl"
    b = tmp_path / "b.jsonl"
    a.write_text(json.dumps(first) + "\n\n", encoding="utf-8")
    b.write_text("\n" + json.dumps(second), encoding="utf-8")

    assert list(load([b, a]).games["id"]) == ["g2", "g1"]
    assert list(load([str(a), str(b)]).games["id"]) == ["g1", "g2"]
    assert list(load(a).games["id"]) == ["g1"]


def test_an_empty_input_is_three_empty_tables_with_their_columns(tmp_path):
    empty = tmp_path / "empty.jsonl"
    empty.write_text("", encoding="utf-8")
    records = load(empty)
    assert records.games.empty and records.seats.empty and records.seat_cards.empty
    assert "first_played_turn" in records.seat_cards.columns
    assert list(load([]).games.columns) == list(records.games.columns)


def test_names_the_file_and_line_it_cannot_read(tmp_path):
    good = json.dumps(two_games()[0])
    garbled = tmp_path / "garbled.jsonl"
    garbled.write_text(good + "\nnot json\n", encoding="utf-8")
    with pytest.raises(ValueError, match=r"garbled\.jsonl: line 2: not a game record"):
        load(garbled)
    partial = tmp_path / "partial.jsonl"
    partial.write_text(good + '\n{"id": "x"}\n', encoding="utf-8")
    with pytest.raises(ValueError, match=r"partial\.jsonl: line 2: not a game record"):
        load(partial)
    with pytest.raises(FileNotFoundError):
        load(tmp_path / "missing.jsonl")


def test_refuses_a_record_id_it_has_read_where_it_would_count_the_game_twice(tmp_path):
    line = json.dumps(two_games()[0])
    a = tmp_path / "a.jsonl"
    b = tmp_path / "b.jsonl"
    a.write_text(line + "\n", encoding="utf-8")
    b.write_text(line + "\n", encoding="utf-8")
    with pytest.raises(ValueError, match=r"b\.jsonl: line 1: record 'g1' is also at .*a\.jsonl: line 1"):
        load([a, b])
    with pytest.raises(ValueError, match="line 2: record 'g1' is also at .*line 1"):
        a.write_text(line + "\n" + line + "\n", encoding="utf-8")
        load(a)


def test_loads_the_fixture_whole():
    records = load(FIXTURES / "dev-40.jsonl")
    assert len(records.games) == 40
    assert len(records.seats) == 80
    assert set(records.seat_cards["game"]) == set(records.games["id"])
    assert records.games["id"].is_unique
    assert records.games["id"].str.startswith("dev:").all()
