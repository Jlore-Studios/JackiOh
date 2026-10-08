"""The notebooks run, on the committed fixtures, and are committed without their outputs.

A notebook is never the source of a number (R377's arithmetic is the Rust's, and `test_card_stats.py` holds the
loader to it), so what is held here is that each one still runs to the end when the data changes shape: it reads
the fixtures its first cell defaults to, draws its figures, and raises nothing. Each runs again with the minimum
sample lowered, because the fixtures are 80 games and the server's minimum is 20 games per card, which leaves the
comparisons nothing to compare; and the sweep's with a sweep that has flags, which the committed one does not.
"""

import json
from pathlib import Path

import nbformat
import pytest
from helpers import record, seat, write_records
from nbconvert.preprocessors import ExecutePreprocessor

ANALYSIS = Path(__file__).resolve().parents[1]
NOTEBOOKS = sorted((ANALYSIS / "notebooks").glob("*.ipynb"))
TIMEOUT_SECONDS = 300
LOW_MINIMUM = {"JACKIOH_MIN_SAMPLE": "3"}
INPUTS = ("JACKIOH_RECORDS", "JACKIOH_CATALOG", "JACKIOH_SWEEP", "JACKIOH_REPO", "JACKIOH_PATCH", "JACKIOH_MIN_SAMPLE", "JACKIOH_SOURCE")


def sweep_line(
    card, tier, flags=(), games=8, drawn=4, affordable=6, plays=3, errors=0, timeouts=0, delta=10.0, unswept=False
):
    return {
        "defId": card,
        "games": games,
        "drawnGames": drawn,
        "affordableTurns": affordable,
        "plays": plays,
        "errors": errors,
        "timeouts": timeouts,
        "evalDeltaSum": delta * plays,
        "evalDeltaCount": plays,
        "tier": tier,
        "flags": list(flags),
        "unswept": unswept,
    }


@pytest.fixture
def flagged_sweep(tmp_path):
    """Pass-1 lines with each flag, a card never affordable, a card the catalog lacks and a pass-2 line."""
    lines = [
        sweep_line("core-002", "easy", ["error"], errors=1),
        sweep_line("core-002", "hard", ["timeout"], timeouts=2),
        sweep_line("core-011", "easy", ["neverPlayed"], affordable=5, plays=0),
        sweep_line("core-011", "hard"),
        sweep_line("classic-020", "easy", ["selfHarm"], plays=5, delta=-60.0),
        sweep_line("classic-020", "hard", ["selfHarm", "timeout"], plays=4, delta=-45.5, timeouts=1),
        sweep_line("classicplus-005", "easy", drawn=0, affordable=0, plays=0, unswept=True),
        sweep_line("classicplus-005", "hard", drawn=0, affordable=0, plays=0, unswept=True),
        sweep_line("gone-001", "easy"),
        {"forced": "core-002", "tier": "easy", "games": 24, "cards": [], "suspects": []},
    ]
    path = tmp_path / "flagged.jsonl"
    path.write_text("".join(json.dumps(line) + "\n" for line in lines), encoding="utf-8")
    return path


def outputs_of(notebook):
    return [output for cell in notebook.cells if cell.cell_type == "code" for output in cell.outputs]


def run(path, monkeypatch, env=None):
    """Execute a notebook where it lives, with `env` in the kernel's environment; returns the executed notebook."""
    for name in INPUTS:  # whatever the developer has set for a real run is not what a test reads
        monkeypatch.delenv(name, raising=False)
    for name, value in (env or {}).items():
        monkeypatch.setenv(name, str(value))
    notebook = nbformat.read(path, as_version=4)
    executor = ExecutePreprocessor(timeout=TIMEOUT_SECONDS, kernel_name="python3")
    executor.preprocess(notebook, {"metadata": {"path": str(path.parent)}})
    return notebook


def test_the_three_notebooks_are_there():
    assert [path.name for path in NOTEBOOKS] == [
        "01-card-win-rates.ipynb",
        "02-decks-and-play.ipynb",
        "03-ai-sweep-and-generations.ipynb",
    ]


@pytest.mark.parametrize("path", NOTEBOOKS, ids=lambda path: path.stem)
def test_a_committed_notebook_holds_no_outputs_and_no_execution_counts(path):
    notebook = nbformat.read(path, as_version=4)
    nbformat.validate(notebook)
    code_cells = [cell for cell in notebook.cells if cell.cell_type == "code"]
    assert code_cells, f"{path.name} has no code"
    for number, cell in enumerate(notebook.cells):
        if cell.cell_type != "code":
            continue
        assert not cell.outputs, (
            f"{path.name}: cell {number} holds outputs (jupyter nbconvert --clear-output --inplace)"
        )
        assert cell.execution_count is None, f"{path.name}: cell {number} holds an execution count"


@pytest.mark.parametrize("path", NOTEBOOKS, ids=lambda path: path.stem)
def test_runs_on_the_fixtures_and_draws_its_figures(path, monkeypatch):
    executed = run(path, monkeypatch)
    outputs = outputs_of(executed)
    assert not [output for output in outputs if output.output_type == "error"]
    assert any(output.output_type == "display_data" and "image/png" in output.data for output in outputs), (
        f"{path.name} drew no figure"
    )


@pytest.mark.parametrize("path", NOTEBOOKS[:2], ids=lambda path: path.stem)
def test_runs_with_the_minimum_sample_lowered_so_that_the_comparisons_have_cards(path, monkeypatch):
    executed = run(path, monkeypatch, LOW_MINIMUM)
    outputs = outputs_of(executed)
    assert not [output for output in outputs if output.output_type == "error"]
    figures = [output for output in outputs if output.output_type == "display_data" and "image/png" in output.data]
    # The card win rates draw its rankings and both comparisons; the decks and play, its curve, turns and games.
    assert len(figures) >= (5 if path.name.startswith("01") else 4), f"{path.name} drew {len(figures)} figures"


def test_the_sweep_notebook_runs_on_a_sweep_with_flags(monkeypatch, flagged_sweep):
    path = NOTEBOOKS[2]
    executed = run(path, monkeypatch, {"JACKIOH_SWEEP": flagged_sweep})
    outputs = outputs_of(executed)
    assert not [output for output in outputs if output.output_type == "error"]
    text = "".join(output.text for output in outputs if output.output_type == "stream")
    assert "9 lines: 5 cards at 2 tiers" in text
    assert "5 flagged, 2 unswept" in text
    assert "Never affordable at any swept tier (no evidence either way): Guy Att" in text


def test_the_win_rates_notebook_says_what_it_cannot_compare_with_one_patch_and_no_live_games(monkeypatch):
    only_dev = ANALYSIS / "fixtures" / "dev-40.jsonl"
    executed = run(NOTEBOOKS[0], monkeypatch, {"JACKIOH_RECORDS": only_dev, "JACKIOH_SOURCE": "dev"})
    text = "".join(output.text for output in outputs_of(executed) if output.output_type == "stream")
    assert "The records hold no patch before v0.3.1" in text
    assert "0 live games" in text


@pytest.mark.parametrize("path", NOTEBOOKS[:2], ids=lambda path: path.stem)
def test_the_record_notebooks_run_on_records_with_no_turns_and_cards_the_catalog_lacks(path, monkeypatch, tmp_path):
    games = [
        record(
            "g1", seat(["a", "b"], ["a"], ["b"], ["a"]), seat(["a", "c"], ["c"], ["a"], []), mode="random", source="dev"
        ),
        record("g2", seat(["a", "c"], ["c"], [], ["c"]), seat(["b"], [], ["b"], []), winner="draw", source="dev"),
    ]
    bare = write_records(tmp_path / "bare.jsonl", games)
    executed = run(path, monkeypatch, {"JACKIOH_RECORDS": bare, "JACKIOH_SOURCE": "dev"})
    outputs = outputs_of(executed)
    assert not [output for output in outputs if output.output_type == "error"]
    text = "".join(output.text for output in outputs if output.output_type == "stream")
    if path.name.startswith("02"):
        assert "No record in these files says which turn a card was played on" in text
    else:
        assert "The records hold no patch before v0.1.1" in text


def test_the_sweep_notebook_runs_on_an_empty_sweep_and_a_repository_with_no_promotions(monkeypatch, tmp_path):
    empty = tmp_path / "empty.jsonl"
    empty.write_text("", encoding="utf-8")
    (tmp_path / "crates" / "ai").mkdir(parents=True)
    zero = {
        "generation": 0,
        "lane": "port",
        "parent": None,
        "vsRandom": "94/100",
        "vsParent": None,
        "date": "2026-10-07",
    }
    (tmp_path / "crates" / "ai" / "generation.json").write_text(json.dumps(zero), encoding="utf-8")
    executed = run(NOTEBOOKS[2], monkeypatch, {"JACKIOH_SWEEP": empty, "JACKIOH_REPO": tmp_path})
    outputs = outputs_of(executed)
    assert not [output for output in outputs if output.output_type == "error"]
    text = "".join(output.text for output in outputs if output.output_type == "stream")
    assert "0 lines: 0 cards" in text and "1 generations, 0 to 0" in text


def streams(executed):
    return "".join(output.text for output in outputs_of(executed) if output.output_type == "stream")


@pytest.mark.parametrize("path", NOTEBOOKS[:2], ids=lambda path: path.stem)
def test_the_record_notebooks_read_live_games_unless_asked_for_more(path, monkeypatch):
    """R378: live data is the default and development data is never mixed in unless asked for."""
    assert "live games of patch v0.3.1" in streams(run(path, monkeypatch))
    assert "all games of patch v0.3.1" in streams(run(path, monkeypatch, {"JACKIOH_SOURCE": "all"}))
    assert "dev games of patch v0.3.1" in streams(run(path, monkeypatch, {"JACKIOH_SOURCE": "dev"}))


@pytest.mark.parametrize("path", NOTEBOOKS[:2], ids=lambda path: path.stem)
def test_the_record_notebooks_say_when_there_is_no_live_game_to_read(path, monkeypatch):
    with pytest.raises(Exception, match="No live game in the records"):
        run(path, monkeypatch, {"JACKIOH_RECORDS": ANALYSIS / "fixtures" / "dev-40.jsonl"})


def test_the_newest_patch_follows_the_revision_letters(monkeypatch, tmp_path):
    """v0.2.10 < v0.2.10b < v0.2.10d, whatever order the records name them in."""
    games = [
        record("g1", seat(["a"], ["a"], [], ["a"]), seat(["b"], ["b"], [], ["b"]), patch=patch)
        for patch in ("v0.2.10d", "v0.2.10", "v0.2.9", "v0.2.10b")
    ]
    games = [{**game, "id": f"live:{n}"} for n, game in enumerate(games)]
    path = write_records(tmp_path / "patches.jsonl", games)
    text = streams(run(NOTEBOOKS[0], monkeypatch, {"JACKIOH_RECORDS": path}))
    assert "reading live games of patch v0.2.10d (the one before it: v0.2.10b)" in text


@pytest.mark.parametrize("path", NOTEBOOKS, ids=lambda path: path.stem)
def test_a_relative_path_is_read_from_the_repository_root(path, monkeypatch):
    """The README's commands name files relative to the repository root, though the kernel runs in notebooks/."""
    root = ANALYSIS.parent
    env = {
        "JACKIOH_RECORDS": "analysis/fixtures/dev-40.jsonl,analysis/fixtures/live-40.jsonl",
        "JACKIOH_SWEEP": "analysis/fixtures/sweep-small.jsonl",
        "JACKIOH_REPO": "analysis/fixtures/repo",
        "JACKIOH_CATALOG": "crates/cards/catalog.json",
    }
    assert all((root / value).is_file() or (root / value).is_dir() for value in env.values() if "," not in value)
    executed = run(path, monkeypatch, env)
    assert not [output for output in outputs_of(executed) if output.output_type == "error"]
