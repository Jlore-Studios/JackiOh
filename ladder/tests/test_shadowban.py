"""Phase 2 shadowban tests (issue #55): the definition, case by case."""

from __future__ import annotations

from audit.shadowban import audit_candidate, compute_shadowban, resolve_agent_builds_decks
from common import load_config

POOL = ["card-a", "card-b", "card-c"]


def _log(n_opp=0, n_plays=0, decks=None, built_by_agent=False):
    decisions = [{"legal_plays": ["card-a"], "played": "card-a" if i < n_plays else "other"} for i in range(n_opp)]
    return {"decisions": decisions, "decks": decks or {}, "built_by_agent": built_by_agent}


def test_rarely_used_card_is_shadowbanned():
    out = compute_shadowban([_log(20, 0)], POOL, 20, 0.05, False)
    assert out["shadowbanned"] == ["card-a"]
    assert out["stats"]["card-a"] == {"opportunities": 20, "plays": 0, "use_rate": 0.0}


def test_use_rate_at_the_line_is_not_shadowbanned():
    out = compute_shadowban([_log(20, 1)], POOL, 20, 0.05, False)
    assert out["shadowbanned"] == []


def test_unobserved_with_fixed_decks_shadowbanned_when_agent_builds():
    fixed = compute_shadowban([_log(0, 0)], POOL, 20, 0.05, False)
    assert fixed["shadowbanned"] == []
    assert set(fixed["unobserved"]) == set(POOL)
    builds = compute_shadowban([_log(0, 0)], POOL, 20, 0.05, True)
    assert set(builds["shadowbanned"]) == set(POOL)
    assert builds["unobserved"] == []


def test_never_drafted_is_shadowbanned_only_when_agent_builds():
    logs = [_log(20, 20, decks={"p1": ["card-a"], "p2": ["card-a"]}, built_by_agent=False)]
    fixed = compute_shadowban(logs, POOL, 20, 0.05, False)
    assert "card-b" not in fixed["shadowbanned"]
    assert "card-b" in fixed["unobserved"]
    logs[0]["built_by_agent"] = True
    builds = compute_shadowban(logs, POOL, 20, 0.05, True)
    assert "card-b" in builds["shadowbanned"]


def test_auto_reads_built_by_agent_from_logs():
    assert resolve_agent_builds_decks("auto", [_log(built_by_agent=True)]) is True
    assert resolve_agent_builds_decks("auto", [_log(built_by_agent=False)]) is False
    assert resolve_agent_builds_decks(True, [_log()]) is True
    assert resolve_agent_builds_decks(False, [_log(built_by_agent=True)]) is False


def test_audit_reads_thresholds_from_config():
    config = load_config()
    assert config["shadowban"]["min_opportunities"] == 20
    assert config["shadowban"]["max_use_rate"] == 0.05
    assert config["shadowban"]["agent_builds_decks"] == "auto"
    pool = [f"card-{i}" for i in range(10)]
    logs = [
        {
            "decisions": [{"legal_plays": ["card-0"], "played": "other"} for _ in range(20)],
            "decks": {},
            "built_by_agent": False,
        }
    ]
    out = audit_candidate(logs, pool, config)
    assert out["shadowbanned"] == ["card-0"]
