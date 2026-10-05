"""Phase 3 proposer tests (issue #55): lenient parsing, strict validation,
retries with feedback, and the context cap. The mock provider only, no network."""

from __future__ import annotations

import json
from pathlib import Path

import pytest
import yaml

from llm.providers import mock
from proposer.propose import (
    ProposalError,
    append_history,
    build_context,
    history_entry,
    load_schema,
    parse_proposal,
    propose,
)

SCHEMA = load_schema(Path(__file__).resolve().parent.parent / "schemas" / "spec.schema.json")


def test_mock_proposal_parses_and_validates():
    spec, files = parse_proposal(mock.MOCK_PROPOSAL, SCHEMA)
    assert spec["name"] == "mock-contender"
    assert spec["agent"]["kind"] == "rl"
    assert set(files) == {"agent.py"}


def test_prose_wrapped_answers_parse():
    text = (
        "Here is my contender for this generation. I chose PPO with masking.\n\n"
        + mock.MOCK_PROPOSAL
        + "\nGood luck in the gauntlet!"
    )
    spec, files = parse_proposal(text, SCHEMA)
    assert spec["name"] == "mock-contender"
    assert "agent.py" in files


def test_invalid_spec_rejected():
    with pytest.raises(ProposalError):
        parse_proposal("```yaml\nname: broken\n```\n", SCHEMA)


class ScriptedClient:
    def __init__(self, replies):
        self.replies = list(replies)
        self.calls = []

    def complete(self, system, messages, max_tokens):
        self.calls.append([dict(m) for m in messages])
        return self.replies[min(len(self.calls) - 1, len(self.replies) - 1)]


def test_invalid_spec_retries_with_feedback_then_fails():
    client = ScriptedClient(["not yaml at all {{{"] * 5)
    with pytest.raises(ProposalError):
        propose(client, "system", "context", SCHEMA, 100, 3)
    assert len(client.calls) == 4  # the try plus 3 retries
    for messages in client.calls[1:]:
        assert messages[-1]["role"] == "user"
        assert "failed validation" in messages[-1]["content"]


def test_retry_succeeds_after_feedback():
    client = ScriptedClient(["garbage {{{", mock.MOCK_PROPOSAL])
    out = propose(client, "system", "context", SCHEMA, 100, 3)
    assert out["spec"]["name"] == "mock-contender"
    assert out["attempts"] == 2


def test_context_cap_drops_history_first():
    history = [f"gen-{i}: " + "x" * 500 for i in range(5)]
    cards = ["card-a", "card-b"]
    out = build_context("iface", "schema", "champs", history, cards, [], max_chars=1200)
    assert 0 < out["dropped_history"] < 5  # oldest dropped, newest kept
    assert out["trimmed_cards"] is False
    assert "gen-4" in out["context"]
    assert "gen-0" not in out["context"]
    assert "card-a\ncard-b" in out["context"]


def test_card_list_trims_to_referenced_plus_flagged():
    history: list[str] = []
    cards = [f"card-{i}" for i in range(50)]
    out = build_context("i", "s", "c", history, cards, ["card-49"], max_chars=120)
    assert out["trimmed_cards"] is True
    assert "card-49" in out["context"]
    assert "card-0" not in out["context"].split("# Cards\n")[1].splitlines()


def test_history_entry_records_provider_model_and_reasons(tmp_path):
    spec = yaml.safe_load(mock.MOCK_SPEC_YAML)
    entry = history_entry("mock", "mock-model", spec, {"wins": 3}, False, ["gates missed"])
    assert entry["provider"] == "mock"
    assert entry["model"] == "mock-model"
    assert entry["spec"]["name"] == "mock-contender"
    assert entry["failure_reasons"] == ["gates missed"]
    path = tmp_path / "proposals.jsonl"
    append_history(path, entry)
    assert json.loads(path.read_text().splitlines()[0])["provider"] == "mock"
