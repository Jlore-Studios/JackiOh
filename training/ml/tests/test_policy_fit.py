"""The policy prior's fit (#1171): it learns which keys the played moves hold, beats the heuristic order
on held-out decisions when the data says it should, writes the table the crate compiles in, and
refuses a file without labels."""

import json
import math

import numpy as np
import pytest

from jackioh_ml import policy_fit as P

HEADER = {"featuresVersion": 1, "every": "decision", "denseNames": [], "buildCatalog": "test", "policyKeysVersion": 1}


def write_positions(path, header=HEADER):
    """100 games of 10 decisions, each 10 moves and End Turn. The played move, at a place that walks
    through the heuristic order, plays `good`; every other move plays a filler of its own."""
    lines = [json.dumps(header)]
    for g in range(100):
        for d in range(10):
            p = (g + d) % 10
            candidates = []
            for i in range(10):
                card = "good" if i == p else f"filler{i}"
                candidates.append(sorted([f"card:play:{card}", "kind:play", f"rank:{i}"]))
            candidates.append(["kind:endTurn"])
            lines.append(json.dumps({"game": f"g{g}", "policy": {"candidates": candidates, "chosen": p}}))
    lines.append(json.dumps({"game": "g0", "policy": {"candidates": [["kind:endTurn"]], "chosen": None}}))
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return path


def test_the_prior_learns_the_played_card_and_writes_the_crate_s_table(tmp_path):
    positions = write_positions(tmp_path / "positions.jsonl")
    report = P.run([str(positions)], tmp_path / "fit", log=lambda _: None)
    table = json.loads((tmp_path / "fit" / "weights.json").read_text(encoding="utf-8"))
    assert table["keysVersion"] == 1 and table["catalog"] == "test"
    assert table["keys"] == sorted(set(table["keys"]))
    assert len(table["weights"]) == len(table["keys"])
    assert all(math.isfinite(w) for w in table["weights"])
    weights = dict(zip(table["keys"], table["weights"]))
    assert max(weights, key=weights.get) == "card:play:good", weights
    assert report["prior"]["within6"] > report["heuristic"]["within6"]
    assert report["prior"]["top1"] > report["heuristic"]["top1"]
    assert report["prior"]["logLoss"] < report["prior"]["uniformLogLoss"]
    assert report["decision"]["go"]
    assert report["skipped"] == 1
    assert report["games"]["train"] + report["games"]["heldOut"] == 100
    assert (tmp_path / "fit" / "report.md").read_text(encoding="utf-8").startswith("# The policy prior's offline fit")


def test_a_file_without_policy_labels_is_refused(tmp_path):
    header = {key: value for key, value in HEADER.items() if key != "policyKeysVersion"}
    positions = write_positions(tmp_path / "positions.jsonl", header)
    with pytest.raises(ValueError):
        P.load_decisions([positions])
    other = write_positions(tmp_path / "other.jsonl", {**HEADER, "policyKeysVersion": 2})
    with pytest.raises(ValueError):
        P.load_decisions([write_positions(tmp_path / "first.jsonl"), other])


def test_a_refit_gives_the_same_weights(tmp_path):
    positions = write_positions(tmp_path / "positions.jsonl")
    dec = P.load_decisions([positions])
    train, _ = P.split(dec)
    vocab = P.vocabulary(dec, train)
    first = P.fit(dec, train, vocab)
    assert np.array_equal(first, P.fit(dec, train, vocab))


def test_the_ranks_sort_by_the_prior_with_ties_in_their_own_order(tmp_path):
    positions = write_positions(tmp_path / "positions.jsonl")
    dec = P.load_decisions([positions])
    rows = np.arange(len(dec))
    heuristic = P.ranks(dec, rows)
    # The played move walks through the ten places: one in ten is first, six in ten within six.
    assert heuristic["decisions"] == 1000 and heuristic["endTurn"] == 0
    assert heuristic["top1"] == pytest.approx(0.1)
    assert heuristic["within6"] == pytest.approx(0.6)
    assert heuristic["meanRank"] == pytest.approx(5.5)
    # A prior of no keys ties every move: the order is the heuristic's.
    assert P.ranks(dec, rows, {}) == heuristic
    assert P.ranks(dec, rows, {"card:play:good": 1.0})["top1"] == 1.0
    assert P.ranks(dec, rows, {"card:play:good": -1.0})["meanRank"] == 10.0
