"""Phase 1 arena tests (issue #55): seeded reproducible games, seat alternation,
forfeits, and the legal-set invariant."""

from __future__ import annotations

import json

import pytest

from agents.random.agent import RandomAgent
from arena.bridge import Bridge
from arena.runner import play_game, play_series
from arena.types import Action, Observation

SEEDS_10 = [f"test-arena-{i}" for i in range(10)]


@pytest.fixture(scope="module")
def bridge():
    with Bridge() as b:
        yield b


def _summary(log):
    return (
        log["winner"],
        log["reason"],
        log["hash"],
        [(d["seat"], d["action_type"], d["played"]) for d in log["decisions"]],
    )


def test_random_vs_random_plays_ten_seeded_games(bridge):
    report = play_series(bridge, RandomAgent("cand"), RandomAgent("opp"), SEEDS_10)
    assert report["games"] == 10
    for log in report["logs"]:
        assert log["winner"] in ("p1", "p2", "draw")
        assert log["forfeit"] is None
        assert log["hash"] is not None
        assert len(log["decisions"]) > 0


def test_same_seeds_give_same_logs(bridge):
    first = play_series(bridge, RandomAgent("cand"), RandomAgent("opp"), SEEDS_10)
    second = play_series(bridge, RandomAgent("cand"), RandomAgent("opp"), SEEDS_10)
    assert [_summary(log) for log in first["logs"]] == [_summary(log) for log in second["logs"]]


def test_seats_alternate_fifty_fifty(bridge):
    report = play_series(bridge, RandomAgent("cand"), RandomAgent("opp"), SEEDS_10)
    seats = [log["candidate_seat"] for log in report["logs"]]
    assert seats == ["p1" if i % 2 == 0 else "p2" for i in range(10)]


class IllegalAgent:
    def act(self, obs: Observation, legal: list[Action]) -> Action:
        return {"type": "nope-not-a-move"}


def test_illegal_action_forfeits(bridge):
    log = play_game(bridge, {"p1": IllegalAgent(), "p2": RandomAgent("opp")}, "test-forfeit")
    assert log["winner"] == "p2"
    assert log["forfeit"] is not None
    assert log["forfeit"]["seat"] == "p1"
    assert log["forfeit"]["reason"] == "illegal"


class ThrowingAgent:
    def act(self, obs: Observation, legal: list[Action]) -> Action:
        raise ValueError("boom")


def test_throwing_agent_forfeits(bridge):
    log = play_game(bridge, {"p1": RandomAgent("ok"), "p2": ThrowingAgent()}, "test-throw")
    assert log["winner"] == "p1"
    assert log["forfeit"]["reason"] == "threw"


class RecordingAgent(RandomAgent):
    def __init__(self, seed):
        super().__init__(seed)
        self.seen: list[tuple[list[Action], Action]] = []

    def act(self, obs: Observation, legal: list[Action]) -> Action:
        choice = super().act(obs, legal)
        self.seen.append((list(legal), choice))
        return choice


def test_every_logged_choice_was_in_its_legal_set(bridge):
    rec1, rec2 = RecordingAgent("r1"), RecordingAgent("r2")
    log = play_game(bridge, {"p1": rec1, "p2": rec2}, "test-legal")
    total = len(rec1.seen) + len(rec2.seen)
    assert total == len(log["decisions"]) > 0
    for legal, choice in rec1.seen + rec2.seen:
        canon = {json.dumps(a, sort_keys=True) for a in legal}
        assert json.dumps(choice, sort_keys=True) in canon
