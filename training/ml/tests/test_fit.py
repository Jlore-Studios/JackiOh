"""One fit end to end (#1009), on the committed fixture: the report, its decision and the model."""

import json
from pathlib import Path

import numpy as np

from jackioh_ml import fit as F

FIXTURE = Path(__file__).parent / "fixtures" / "positions.jsonl"
ROOT = Path(__file__).resolve().parents[3]


def test_one_fit_end_to_end(tmp_path):
    report = F.run([str(FIXTURE)], tmp_path, ROOT, seeds=(1, 2), curve=(5, 10), log=lambda _: None)
    assert report["decision"]["verdict"] in {"go", "more data", "stop"}
    for key in ("a", "b", "c"):
        loss = report["models"][key]["heldOut"]["all"]["logLoss"]
        assert 0 < loss < 2, (key, loss)
    assert [p["games"] for p in report["curve"]][:1] == [5]
    assert (tmp_path / "report.md").read_text(encoding="utf-8").startswith("# The learned evaluation's offline fit")
    assert json.loads((tmp_path / "report.json").read_text(encoding="utf-8"))["featuresVersion"] == report["featuresVersion"]
    assert (tmp_path / "model.pt").exists()


def test_the_metrics():
    y = np.array([1.0, 0.0, 1.0, 0.0])
    assert F.auc(np.array([0.9, 0.1, 0.8, 0.2]), y) == 1.0
    assert F.auc(np.array([0.1, 0.9, 0.2, 0.8]), y) == 0.0
    assert F.auc(np.array([0.5, 0.5, 0.5, 0.5]), y) == 0.5
    assert abs(F.log_loss(np.array([0.5] * 4), y) - np.log(2)) < 1e-9


def test_td_targets_run_along_each_game_and_seat_in_step_order():
    # Two games; game 0's seats alternate. Rows are out of order on purpose.
    game = np.array([0, 0, 0, 0, 1, 0])
    seat = np.array([0, 1, 0, 1, 0, 0])
    step = np.array([1, 2, 3, 4, 1, 5])
    outcome = np.array([1.0, 0.0, 1.0, 0.0, 0.5, 1.0])
    p = np.array([0.6, 0.4, 0.7, 0.3, 0.5, 0.8])
    t = F.td_targets(game, seat, step, outcome, p, 0.5)
    # Seat 0 of game 0: steps 1, 3, 5 (rows 0, 2, 5). The last keeps its outcome.
    assert t[5] == 1.0
    assert abs(t[2] - (0.5 * 0.8 + 0.5 * 1.0)) < 1e-12
    assert abs(t[0] - (0.5 * 0.7 + 0.5 * t[2])) < 1e-12
    # Seat 1: steps 2, 4 (rows 1, 3).
    assert t[3] == 0.0
    assert abs(t[1] - (0.5 * 0.3 + 0.5 * 0.0)) < 1e-12
    # A game of one row keeps its outcome, and lambda 1 is the outcome everywhere.
    assert t[4] == 0.5
    assert np.array_equal(F.td_targets(game, seat, step, outcome, p, 1.0), outcome)


def test_lambda_one_fits_the_outcomes_alone(tmp_path):
    report = F.run([str(FIXTURE)], tmp_path, ROOT, seeds=(1,), curve=(5,), td_lambda=1.0, log=lambda _: None)
    assert report["tdLambda"] == 1.0
    assert report["models"]["c"]["name"].endswith("on the outcomes")
