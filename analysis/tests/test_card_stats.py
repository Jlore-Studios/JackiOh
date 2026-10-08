"""R377's card win rates: the loader's figures against the Rust's (`jackioh_engine::card_stats`), and the
sweep and generation loaders.

The fixture is a development run (`cargo jackioh stats --games 40`) and its report is what
`cargo jackioh stats report` printed over it, so these tests hold the notebooks' arithmetic to the engine's.
The hand-written records are the engine's own R377 test, counted on the fingers.
"""

import json
import math
from pathlib import Path

import pandas as pd
import pytest
from helpers import record, seat, write_records
from jackioh_records import BREAKDOWNS, card_stats, load, load_catalog, load_generations, load_sweep

FIXTURES = Path(__file__).resolve().parents[1] / "fixtures"
RATE_TOLERANCE = 1e-9

# The report names its breakdowns the way the wire does.
WIRE_NAMES = {
    "in_deck": "inDeck",
    "opening_hand": "openingHand",
    "going_first": "goingFirst",
    "going_second": "goingSecond",
    "played": "played",
    "drawn_not_played": "drawnNotPlayed",
}


def tally(stats, card, breakdown):
    row = stats[stats["card"] == card].iloc[0]
    return (
        int(row[f"{breakdown}_games"]),
        int(row[f"{breakdown}_wins"]),
        int(row[f"{breakdown}_draws"]),
    )


def assert_matches_report(stats, report):
    """Every figure of the Rust report: integers exactly, rates to RATE_TOLERANCE."""
    assert stats.attrs["games"] == report["games"]
    assert stats.attrs["decks"] == report["decks"]
    assert list(stats["card"]) == [row["card"] for row in report["cards"]]
    for row, expected in zip(stats.to_dict("records"), report["cards"], strict=True):
        rates = {}
        for breakdown in BREAKDOWNS:
            want = expected[WIRE_NAMES[breakdown]]
            assert row[f"{breakdown}_games"] == want["games"], (row["card"], breakdown)
            assert row[f"{breakdown}_wins"] == want["wins"], (row["card"], breakdown)
            assert row[f"{breakdown}_draws"] == want["draws"], (row["card"], breakdown)
            if want["games"] == 0:
                assert math.isnan(row[f"{breakdown}_rate"]), (row["card"], breakdown)
                rates[breakdown] = None
            else:
                rates[breakdown] = want["wins"] / want["games"]
                assert row[f"{breakdown}_rate"] == pytest.approx(rates[breakdown], abs=RATE_TOLERANCE)
        # The played delta, in percentage points, shows nothing while either side has no games.
        if rates["played"] is None or rates["drawn_not_played"] is None:
            assert math.isnan(row["played_delta"]), row["card"]
        else:
            delta = 100 * (rates["played"] - rates["drawn_not_played"])
            assert row["played_delta"] == pytest.approx(delta, abs=RATE_TOLERANCE), row["card"]


def test_card_stats_equals_the_rust_report_on_the_fixture():
    report = json.loads((FIXTURES / "dev-40.report.json").read_text(encoding="utf-8"))
    assert report["filter"]["source"] == "all"
    assert report["games"] == 40 and report["cards"]
    stats = card_stats(load(FIXTURES / "dev-40.jsonl"), source="all")
    assert_matches_report(stats, report)


def test_card_stats_reads_live_games_unless_a_development_run_is_asked_for_by_name():
    records = load(FIXTURES / "dev-40.jsonl")
    assert set(records.games["source"]) == {"dev"}
    live = card_stats(records)
    assert live.attrs["games"] == 0 and live.attrs["decks"] == 0
    assert live.empty
    assert card_stats(records, source="dev").attrs["games"] == 40
    # Both seats of a development game are the AI's, and the run files every game under one patch.
    patch = records.games["patch"].iloc[0]
    everything = card_stats(records, source="all")
    for narrowed in (
        card_stats(records, source="dev"),
        card_stats(records, source="all", pilot="ai"),
        card_stats(records, source="all", mode="random", patch=patch),
    ):
        assert narrowed.equals(everything)
    assert card_stats(records, source="all", pilot="human").empty
    assert card_stats(records, source="all", mode="bo1").empty
    assert card_stats(records, source="all", patch="v0.0.0").empty


def test_every_card_of_the_fixture_is_in_the_catalog():
    catalog = load_catalog()
    stats = card_stats(load(FIXTURES / "dev-40.jsonl"), source="all")
    assert set(stats["card"]) <= set(catalog.index)
    core_001 = catalog.loc["core-001"]
    assert (core_001["name"], core_001["set"], core_001["rarity"], core_001["type"]) == (
        "Big D-fender",
        "Core",
        "Common",
        "Unit",
    )
    assert core_001["cost"] == 2 and core_001["tags"] == ("Human",)


def test_counts_each_deck_once_per_game_a_mirror_once_per_deck_and_a_draw_as_a_game_not_won(tmp_path):
    path = write_records(
        tmp_path / "hand.jsonl",
        [
            # p1 wins going first. p1 opens with a, draws b and plays a; c stays in its library.
            record(
                "g1",
                seat(["a", "b", "c"], ["a"], ["b"], ["a"]),
                # p2 holds a too: the mirror counts it once more, as a loss going second.
                seat(["a", "d"], ["core-t-coin", "d"], ["a"], ["d"]),
            ),
            # p2 wins going second with a in its opening hand, played twice (still one game).
            record(
                "g2",
                seat(["b", "c"], ["b", "c"], [], ["c"]),
                seat(["a", "b"], ["a"], [], ["a", "a"]),
                winner="p2",
            ),
            # A draw: both decks hold b, drawn and never played.
            record("g3", seat(["b"], [], ["b"], []), seat(["b"], ["b"], [], []), winner="draw"),
        ],
    )
    stats = card_stats(load(path))
    assert (stats.attrs["games"], stats.attrs["decks"]) == (3, 6)
    assert list(stats["card"]) == ["a", "b", "c", "d"]

    # (games, wins, draws)
    assert tally(stats, "a", "in_deck") == (3, 2, 0)
    assert tally(stats, "a", "opening_hand") == (2, 2, 0)
    assert tally(stats, "a", "going_first") == (1, 1, 0)
    assert tally(stats, "a", "going_second") == (2, 1, 0)
    assert tally(stats, "a", "played") == (2, 2, 0)
    assert tally(stats, "a", "drawn_not_played") == (1, 0, 0)

    assert tally(stats, "b", "in_deck") == (5, 2, 2)
    assert tally(stats, "b", "opening_hand") == (2, 0, 1)
    assert tally(stats, "b", "played") == (0, 0, 0)
    # g1 p1 (drawn), g2 p1 (opening), g3 both: never played. g2 p2 held b and never drew it.
    assert tally(stats, "b", "drawn_not_played") == (4, 1, 2)

    # c never reached g1's hand, so it is in neither side of the delta there.
    assert tally(stats, "c", "in_deck")[0] == 2
    assert tally(stats, "c", "played") == (1, 0, 0)
    assert tally(stats, "c", "drawn_not_played") == (0, 0, 0)

    # The Coin was in an opening hand and is in no deck, so it has no entry.
    assert "core-t-coin" not in set(stats["card"])

    # a: played 2 of 2 won, drawn and not played 0 of 1; no rate where there are no games, and no delta.
    row = stats.set_index("card")
    assert row.loc["a", "played_delta"] == pytest.approx(100.0)
    assert math.isnan(row.loc["b", "played_rate"]) and math.isnan(row.loc["b", "played_delta"])
    assert math.isnan(row.loc["c", "drawn_not_played_rate"]) and math.isnan(row.loc["c", "played_delta"])
    assert row.loc["b", "in_deck_rate"] == pytest.approx(2 / 5)


def test_filters_by_source_mode_patch_and_pilot_and_counts_only_games_with_a_seat_it_counts(tmp_path):
    def simple(id, **fields):
        return record(id, seat(["a", "b", "c"]), seat(["a", "b", "c"]), **fields)

    path = write_records(
        tmp_path / "filters.jsonl",
        [
            simple("1", mode="bo1", patch="v0.1.1"),
            simple("2", mode="random", patch="v0.1.1"),
            simple("3", mode="bo3", patch="v0.2.5"),
            # One human seat, one AI seat; the AI's deck holds only z.
            record(
                "4",
                seat(["a", "b", "c"]),
                seat(["z"]),
                winner="p2",
                mode="random",
                patch="v0.2.5",
                pilots=("human", "ai"),
            ),
            record(
                "dev:v0.2.5:dev:1",
                seat(["a"]),
                seat(["a"]),
                source="dev",
                mode="random",
                patch="v0.2.5",
                pilots=("ai", "ai"),
            ),
        ],
    )
    records = load(path)

    def games(**filters):
        return card_stats(records, **filters).attrs["games"]

    assert games() == 4
    assert games(mode="random") == 2
    assert games(patch="v0.2.5") == 2
    assert games(mode="random", patch="v0.2.5") == 1
    assert games(mode="bo1", patch="v0.2.5") == 0
    assert games(source="dev") == 1
    assert games(source="all") == 5
    assert games(source="all", mode="random", patch="v0.2.5") == 2

    human = card_stats(records, pilot="human")
    assert human.attrs["decks"] == 7
    assert list(human["card"]) == ["a", "b", "c"]
    ai = card_stats(records, pilot="ai")
    assert (ai.attrs["games"], ai.attrs["decks"]) == (1, 1)
    assert list(ai["card"]) == ["z"]
    assert tally(ai, "z", "in_deck") == (1, 1, 0)
    unified = card_stats(records)
    assert unified.attrs["decks"] == 8
    assert tally(unified, "a", "in_deck")[0] == 7


def test_refuses_a_filter_it_does_not_know_rather_than_reading_something_else(tmp_path):
    records = load(write_records(tmp_path / "one.jsonl", [record("1", seat(["a"]), seat(["b"]))]))
    for bad in ({"source": "practice"}, {"mode": "bo5"}, {"pilot": "robot"}):
        with pytest.raises(ValueError, match="must be one of"):
            card_stats(records, **bad)


# The loaders for the sweep and the AI's generations (the third notebook's inputs).

REPO_ROOT = Path(__file__).resolve().parents[2]


def sweep_line(card, tier, **fields):
    line = {
        "defId": card,
        "games": 8,
        "drawnGames": 4,
        "affordableTurns": 5,
        "plays": 4,
        "errors": 0,
        "timeouts": 0,
        "evalDeltaSum": 20.0,
        "evalDeltaCount": 4,
        "tier": tier,
        "flags": [],
        "unswept": False,
    }
    line.update(fields)
    return line


def write_lines(path, lines):
    path.write_text("".join(json.dumps(line) + "\n" for line in lines), encoding="utf-8")
    return path


def test_load_sweep_reads_the_fixtures_lines_one_row_per_card_and_tier():
    sweep = load_sweep(FIXTURES / "sweep-small.jsonl")
    lines = [json.loads(line) for line in (FIXTURES / "sweep-small.jsonl").read_text(encoding="utf-8").splitlines()]

    assert [(row.card, row.tier) for row in sweep.itertuples()] == [(line["defId"], line["tier"]) for line in lines]
    assert len(sweep) == 8 and set(sweep["tier"]) == {"easy", "hard"}
    for row, line in zip(sweep.to_dict("records"), lines, strict=True):
        assert row["games"] == line["games"]
        assert row["drawn_games"] == line["drawnGames"]
        assert row["affordable_turns"] == line["affordableTurns"]
        assert row["plays"] == line["plays"]
        assert row["errors"] == line["errors"] and row["timeouts"] == line["timeouts"]
        assert row["eval_delta_sum"] == line["evalDeltaSum"] and row["eval_delta_count"] == line["evalDeltaCount"]
        assert row["eval_delta_mean"] == pytest.approx(line["evalDeltaSum"] / line["evalDeltaCount"])
        assert row["flags"] == line["flags"] and row["unswept"] == line["unswept"]
    assert sweep["games"].dtype == "int64" and sweep["eval_delta_mean"].dtype == "float64"
    assert sweep["unswept"].dtype == "bool"
    assert set(sweep["card"]) <= set(load_catalog().index)


def test_load_sweep_keeps_the_flags_as_a_list_and_no_plays_as_no_mean(tmp_path):
    path = write_lines(
        tmp_path / "flags.jsonl",
        [
            sweep_line("a", "easy", flags=["error", "selfHarm"], errors=2, evalDeltaSum=-200.0),
            sweep_line("a", "hard"),
            sweep_line("b", "easy", flags=["neverPlayed"], plays=0, evalDeltaSum=0.0, evalDeltaCount=0),
            sweep_line("c", "easy", affordableTurns=0, plays=0, evalDeltaSum=0.0, evalDeltaCount=0, unswept=True),
        ],
    )
    sweep = load_sweep(path).set_index(["card", "tier"])
    assert sweep.loc[("a", "easy"), "flags"] == ["error", "selfHarm"]
    assert sweep.loc[("a", "easy"), "eval_delta_mean"] == -50.0
    assert sweep.loc[("a", "hard"), "flags"] == []
    # A card with no plays has no mean (the Rust divides by one there; a table must not show a mean of zero).
    assert math.isnan(sweep.loc[("b", "easy"), "eval_delta_mean"])
    assert sweep.loc[("c", "easy"), "unswept"]


def test_load_sweep_reads_several_files_skips_pass_2_lines_and_ignores_fields_it_does_not_know(tmp_path):
    first = write_lines(tmp_path / "a.jsonl", [sweep_line("a", "easy")])
    pass_2 = {"forced": "a", "tier": "easy", "games": 24, "cards": [], "suspects": []}
    second = write_lines(tmp_path / "b.jsonl", [sweep_line("b", "hard", somethingNew=[1]), pass_2])
    sweep = load_sweep([second, str(first)])
    assert [(row.card, row.tier) for row in sweep.itertuples()] == [("b", "hard"), ("a", "easy")]
    assert list(load_sweep(first)["card"]) == ["a"]
    # No lines is an empty table with its columns, not an error.
    empty = tmp_path / "empty.jsonl"
    empty.write_text("\n", encoding="utf-8")
    assert load_sweep(empty).empty and "eval_delta_mean" in load_sweep(empty).columns


def test_load_sweep_names_the_file_and_line_it_cannot_read_and_a_card_and_tier_it_has_read(tmp_path):
    good = json.dumps(sweep_line("a", "easy"))
    garbled = tmp_path / "garbled.jsonl"
    garbled.write_text(good + "\nnot json\n", encoding="utf-8")
    with pytest.raises(ValueError, match=r"garbled\.jsonl: line 2: not a sweep result"):
        load_sweep(garbled)
    partial = tmp_path / "partial.jsonl"
    partial.write_text(good + '\n{"defId": "b"}\n', encoding="utf-8")
    with pytest.raises(ValueError, match=r"partial\.jsonl: line 2: not a sweep result"):
        load_sweep(partial)
    # A slice cut short and run again writes its finished cards twice; the table would count them twice.
    again = write_lines(
        tmp_path / "again.jsonl", [sweep_line("a", "easy"), sweep_line("a", "hard"), sweep_line("a", "easy")]
    )
    with pytest.raises(ValueError, match=r"again\.jsonl: line 3: a at easy is also at .*again\.jsonl: line 1"):
        load_sweep(again)


def test_load_generations_reads_the_fixture_repository_oldest_first():
    generations = load_generations(FIXTURES / "repo")
    assert list(generations["generation"]) == [1, 2, 3]
    assert list(generations["lane"]) == ["improve", "improve", "unban"]
    assert list(generations["vs_random_wins"]) == [91, 93, 93]
    assert list(generations["vs_random_games"]) == [100, 100, 100]
    assert list(generations["vs_parent_wins"]) == [86, 88, 77]
    assert list(generations["shadow_ban"]) == [11, 11, 9]
    assert list(generations["parent_shadow_ban"]) == [11, 11, 11]
    assert generations["generation"].dtype == "int64" and generations["vs_parent_wins"].dtype == pd.Int64Dtype()
    # generation.json holds generation 3 again, and the history's line for it is the one read.
    assert generations["parent"].iloc[2] == "5b1c0de"


def test_load_generations_reads_generation_zero_off_generation_json_when_no_history_holds_it(tmp_path):
    (tmp_path / "crates" / "ai").mkdir(parents=True)
    zero = {
        "generation": 0,
        "lane": "port",
        "parent": None,
        "vsRandom": "94/100",
        "vsParent": None,
        "shadowBan": 11,
        "date": "2026-10-07",
    }
    (tmp_path / "crates" / "ai" / "generation.json").write_text(json.dumps(zero), encoding="utf-8")
    generations = load_generations(tmp_path)  # no training/history at all: no promotions yet
    assert len(generations) == 1
    row = generations.iloc[0]
    assert (row["generation"], row["lane"], row["vs_random_wins"], row["vs_random_games"]) == (0, "port", 94, 100)
    assert pd.isna(row["vs_parent_wins"]) and pd.isna(row["vs_parent_games"]) and pd.isna(row["parent_shadow_ban"])
    assert row["shadow_ban"] == 11

    # Promotions append to the lanes' files, and the newest is in generation.json too: one row, not two.
    history = tmp_path / "training" / "history"
    history.mkdir(parents=True)
    promotion = {
        **zero,
        "generation": 1,
        "lane": "improve",
        "parent": "abc",
        "vsParent": "87/100",
        "parentShadowBan": 11,
    }
    (tmp_path / "crates" / "ai" / "generation.json").write_text(json.dumps(promotion), encoding="utf-8")
    write_lines(history / "improve.jsonl", [promotion])
    write_lines(history / "unban.jsonl", [])
    assert list(load_generations(tmp_path)["generation"]) == [1]


def test_load_generations_names_the_file_and_line_it_cannot_read(tmp_path):
    (tmp_path / "crates" / "ai").mkdir(parents=True)
    history = tmp_path / "training" / "history"
    history.mkdir(parents=True)
    zero = {"generation": 0, "lane": "port", "parent": None, "vsRandom": "94/100", "vsParent": None, "date": "d"}
    generation_json = tmp_path / "crates" / "ai" / "generation.json"
    generation_json.write_text(json.dumps(zero), encoding="utf-8")
    one = {**zero, "generation": 1, "lane": "improve", "parent": "abc", "vsParent": "87/100"}

    write_lines(history / "improve.jsonl", [one, {**one, "lane": "unban"}])
    with pytest.raises(ValueError, match=r"improve\.jsonl: line 2: generation 1 is also at .*improve\.jsonl: line 1"):
        load_generations(tmp_path)
    write_lines(history / "improve.jsonl", [one, {**one, "generation": 2, "vsRandom": "lots"}])
    with pytest.raises(ValueError, match=r"improve\.jsonl: line 2: not a generation"):
        load_generations(tmp_path)
    (history / "improve.jsonl").write_text("[]\n", encoding="utf-8")
    with pytest.raises(ValueError, match=r"improve\.jsonl: line 1: not a generation"):
        load_generations(tmp_path)
    (history / "improve.jsonl").unlink()
    generation_json.write_text('{"generation": 0}', encoding="utf-8")
    with pytest.raises(ValueError, match=r"generation\.json: not a generation"):
        load_generations(tmp_path)


def test_load_generations_reads_this_repository_and_finds_the_generation_main_holds():
    main = json.loads((REPO_ROOT / "crates" / "ai" / "generation.json").read_text(encoding="utf-8"))
    generations = load_generations(REPO_ROOT)
    row = generations[generations["generation"] == main["generation"]].iloc[0]
    assert row["lane"] == main["lane"]
    assert f"{row['vs_random_wins']}/{row['vs_random_games']}" == main["vsRandom"]
    assert generations["generation"].is_monotonic_increasing and generations["generation"].is_unique
