"""Phase 3 LLM tests (issue #55): the mock provider, loud config failures, and
provider confinement (provider-specific code stays in ``providers/``)."""

from __future__ import annotations

from pathlib import Path

import pytest

from llm.client import ConfiguredClient, from_env
from llm.providers import mock

LADDER_ROOT = Path(__file__).resolve().parent.parent


def test_mock_returns_a_fixed_proposal_without_credentials():
    client = ConfiguredClient("mock", "mock-model")
    first = client.complete("system", [{"role": "user", "content": "propose"}], 100)
    second = client.complete("system", [{"role": "user", "content": "propose"}], 100)
    assert first == second
    assert "agent:" in first


def test_mock_module_needs_no_key():
    assert mock.complete("any-model", "", "system", [], 10) == mock.MOCK_PROPOSAL


def test_missing_provider_fails_loudly():
    with pytest.raises(ValueError, match="LLM_PROVIDER"):
        from_env({})


def test_invalid_provider_fails_loudly():
    with pytest.raises(ValueError, match="LLM_PROVIDER"):
        from_env({"LLM_PROVIDER": "skynet", "LLM_MODEL": "x"})


def test_missing_model_fails_loudly():
    with pytest.raises(ValueError, match="LLM_MODEL"):
        from_env({"LLM_PROVIDER": "mock"})


def test_missing_key_fails_loudly_for_keyed_providers():
    with pytest.raises(ValueError, match="LLM_API_KEY"):
        from_env({"LLM_PROVIDER": "anthropic", "LLM_MODEL": "x"})
    client = from_env({"LLM_PROVIDER": "mock", "LLM_MODEL": "mock-model"})
    assert client.provider == "mock"


def test_provider_specific_code_stays_in_providers():
    """Nothing outside ladder/llm/providers may touch provider transports.

    Our own adapter (``from .providers import ...``) is fine; third-party
    SDKs and the http transport are not.
    """
    banned = ("urllib", "anthropic", "openai", "httpx", "requests")
    offenders = []
    for path in LADDER_ROOT.rglob("*.py"):
        if "llm/providers" in path.as_posix():
            continue
        for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.split("#")[0]
            if "providers" in stripped:
                continue
            hits = [b for b in banned if f"import {b}" in stripped or f"from {b}" in stripped]
            if hits:
                offenders.append(f"{path.name}:{lineno}: {hits}")
    assert offenders == []
