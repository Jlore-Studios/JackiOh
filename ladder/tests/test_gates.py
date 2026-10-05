"""Phase 2 gate tests (issue #55): 0, 1 and 2 champions, config thresholds,
and the zero-shadowban exception."""

from __future__ import annotations

from audit.gates import evaluate_gates
from common import load_config

import pytest

POOL = 100


@pytest.fixture(scope="module")
def config():
    return load_config()


def test_thresholds_come_from_config(config):
    assert config["games_per_opponent"] == 100
    assert config["win_threshold_vs_champion"] == 75
    assert config["win_threshold_vs_random"] == 90
    assert config["max_shadowban_fraction"] == 0.10


def test_no_champions_random_and_shadowban_decide(config):
    ok = evaluate_gates([], 90, 5, [], POOL, config)
    assert ok == {"promoted": True, "failures": []}
    short = evaluate_gates([], 89, 5, [], POOL, config)
    assert short["promoted"] is False
    assert any("random" in f for f in short["failures"])


def test_one_champion_threshold(config):
    assert evaluate_gates([75], 90, 0, [5], POOL, config)["promoted"] is True
    out = evaluate_gates([74], 90, 0, [5], POOL, config)
    assert out["promoted"] is False
    assert any("champion-1" in f for f in out["failures"])


def test_two_champions_both_must_pass(config):
    assert evaluate_gates([80, 75], 95, 0, [4, 6], POOL, config)["promoted"] is True
    out = evaluate_gates([80, 74], 95, 0, [4, 6], POOL, config)
    assert out["promoted"] is False
    assert any("champion-2" in f for f in out["failures"])


def test_shadowban_fraction_gate(config):
    assert evaluate_gates([], 100, 10, [], POOL, config)["promoted"] is True
    out = evaluate_gates([], 100, 11, [], POOL, config)
    assert out["promoted"] is False
    assert any("shadowban" in f for f in out["failures"])


def test_strictly_fewer_than_champion_1(config):
    assert evaluate_gates([80], 95, 2, [3], POOL, config)["promoted"] is True
    out = evaluate_gates([80], 95, 3, [3], POOL, config)
    assert out["promoted"] is False
    assert any("shadowban-improvement" in f for f in out["failures"])


def test_zero_shadowban_exception(config):
    assert evaluate_gates([80], 95, 0, [0], POOL, config)["promoted"] is True
    out = evaluate_gates([80], 95, 1, [0], POOL, config)
    assert out["promoted"] is False
    assert any("shadowban-parity" in f for f in out["failures"])


def test_all_gates_reported_together(config):
    out = evaluate_gates([10], 10, 50, [0], POOL, config)
    assert out["promoted"] is False
    assert len(out["failures"]) == 4
