"""One fit end to end (#1009), on the committed fixture: the report, its decision and the model."""

import json
from pathlib import Path
from types import SimpleNamespace

import numpy as np
import torch

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
    assert all((tmp_path / f"model-seed{seed}.pt").exists() for seed in (1, 2))
    assert [s["validation"] > 0 for s in report["seeds"]] == [True, True]


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


def test_each_relabel_refits_on_the_targets_the_fit_before_it_gives(tmp_path, monkeypatch):
    report = F.run([str(FIXTURE)], tmp_path, ROOT, seeds=(1,), curve=(), td_lambda=0.3, td_relabels=2, log=lambda _: None)
    assert (report["tdLambda"], report["tdRelabels"]) == (0.3, 2)
    assert report["models"]["c"]["name"].endswith("on TD(0.3) targets, relabelled 2 times")

    # The loop itself, on stand-ins: fit k's network predicts k/10 everywhere.
    game, seat, step = np.array([0, 0, 0]), np.array([0, 0, 0]), np.array([1, 2, 3])
    outcome = np.array([1.0, 1.0, 1.0])
    positions = SimpleNamespace(game=game, seat=seat, step=step, outcome=outcome)
    data = SimpleNamespace(outcome=torch.tensor(outcome, dtype=torch.float32))
    made, fits = [], []
    monkeypatch.setattr(F, "replace", lambda d, outcome: SimpleNamespace(outcome=outcome))
    monkeypatch.setattr(F, "train", lambda net, d, *a, **k: fits.append(d.outcome.clone()) or {"validation": 0.0})
    monkeypatch.setattr(F, "predict", lambda net, d: np.full(3, net.k / 10))

    def make():
        made.append(SimpleNamespace(k=len(made) + 1))
        return made[-1]

    net, _ = F.fit_network(make, data, positions, np.arange(3), np.arange(0), seed=1, lam=0.3, relabels=2, log=lambda _: None)
    assert net is made[-1] and len(made) == len(fits) == 3
    assert fits[0].tolist() == [1.0, 1.0, 1.0]
    for k in (1, 2):
        expected = F.td_targets(game, seat, step, outcome, np.full(3, k / 10), 0.3)
        assert np.allclose(fits[k].numpy(), expected)
