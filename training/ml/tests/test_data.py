"""The loader (#1009): the feature length comes from the data, a FEATURES_VERSION other than the
files' is refused, and the hold-out split keeps each game whole."""

import json
from pathlib import Path

import numpy as np
import pytest

from jackioh_ml import data as D

FIXTURE = Path(__file__).parent / "fixtures" / "positions.jsonl"


def test_the_fixture_loads_with_its_own_feature_length():
    positions = D.load([FIXTURE])
    header = json.loads(FIXTURE.read_text(encoding="utf-8").splitlines()[0])
    assert positions.features_version == header["featuresVersion"]
    assert positions.dense.shape == (len(positions), len(header["denseNames"]))
    assert len(positions) == len(FIXTURE.read_text(encoding="utf-8").splitlines()) - 1
    assert set(np.unique(positions.outcome)) <= {0.0, 0.5, 1.0}
    assert len(positions.sparse_ptr) == len(positions) + 1
    assert positions.sparse_ptr[-1] == len(positions.sparse_idx) == len(positions.sparse_val)


def test_another_features_version_is_refused(tmp_path):
    with pytest.raises(D.FeatureMismatch):
        D.load([FIXTURE], expect_version=D.load([FIXTURE]).features_version + 1)
    lines = FIXTURE.read_text(encoding="utf-8").splitlines()
    header = json.loads(lines[0])
    header["featuresVersion"] += 1
    other = tmp_path / "other.jsonl"
    other.write_text("\n".join([json.dumps(header)] + lines[1:3]) + "\n", encoding="utf-8")
    with pytest.raises(D.FeatureMismatch):
        D.load([FIXTURE, other])


def test_a_row_of_another_length_is_refused(tmp_path):
    lines = FIXTURE.read_text(encoding="utf-8").splitlines()
    row = json.loads(lines[1])
    row["dense"] = row["dense"][:-1]
    bad = tmp_path / "bad.jsonl"
    bad.write_text(lines[0] + "\n" + json.dumps(row) + "\n", encoding="utf-8")
    with pytest.raises(D.FeatureMismatch):
        D.load([bad])


def test_the_hold_out_keeps_every_game_whole_and_is_stable():
    positions = D.load([FIXTURE])
    fit, held = D.split_by_game(positions, 0.3)
    assert len(fit) + len(held) == len(positions)
    assert not set(positions.game[fit]) & set(positions.game[held])
    again_fit, again_held = D.split_by_game(D.load([FIXTURE]), 0.3)
    assert (fit == again_fit).all() and (held == again_held).all()


def test_take_keeps_each_rows_sparse_entries():
    positions = D.load([FIXTURE])
    rows = np.array([5, 0, 17])
    taken = positions.take(rows)
    for new, old in enumerate(rows):
        a = positions.sparse_idx[positions.sparse_ptr[old] : positions.sparse_ptr[old + 1]]
        b = taken.sparse_idx[taken.sparse_ptr[new] : taken.sparse_ptr[new + 1]]
        assert (a == b).all()
    assert (taken.dense == positions.dense[rows]).all()


def test_the_vocabulary_keeps_frequent_keys_and_maps_the_rest_to_nothing():
    positions = D.load([FIXTURE])
    rows = np.arange(len(positions))
    every = D.Vocabulary.build(positions, rows, 1)
    assert set(every.keys) == set(positions.keys)
    frequent = D.Vocabulary.build(positions, rows, 50)
    assert set(frequent.keys) < set(every.keys)
    remap = frequent.remap(positions)
    assert (remap >= 0).sum() == len(frequent.keys)


def test_first_games_nest():
    positions = D.load([FIXTURE])
    rows = np.arange(len(positions))
    small = set(D.first_games(positions, rows, 5))
    large = set(D.first_games(positions, rows, 10))
    assert small < large
    assert len(D.games_of(positions, np.array(sorted(small)))) == 5
