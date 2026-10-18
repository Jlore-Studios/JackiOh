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
