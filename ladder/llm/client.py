"""The LLM adapter layer (issue #55, phase 3): one interface, four providers.

The provider is configured through the environment only — repo variables and
secrets in CI, ``LLM_PROVIDER`` / ``LLM_MODEL`` / ``LLM_API_KEY`` locally — so
switching providers needs no code change. A missing or invalid configuration
fails loudly, naming the variable; it never fails silently. Only ``mock``
runs without a key.
"""

from __future__ import annotations

import os
from typing import Protocol

from .providers import anthropic, google, mock, openai
from .providers.base import ProviderError

PROVIDERS = {
    anthropic.PROVIDER_NAME: anthropic,
    openai.PROVIDER_NAME: openai,
    google.PROVIDER_NAME: google,
    mock.PROVIDER_NAME: mock,
}

# Providers that need no credential (tests and CI run keyless).
KEYLESS_PROVIDERS = frozenset({mock.PROVIDER_NAME})


class LLMClient(Protocol):
    """Single interface every provider serves."""

    def complete(self, system: str, messages: list[dict], max_tokens: int) -> str:
        ...


class ConfiguredClient:
    """An LLMClient bound to one provider, model and credential."""

    def __init__(self, provider: str, model: str, api_key: str = "") -> None:
        module = PROVIDERS.get(provider)
        if module is None:
            raise ValueError(
                f"unknown LLM_PROVIDER {provider!r}: want one of {sorted(PROVIDERS)}"
            )
        if provider not in KEYLESS_PROVIDERS and not api_key:
            raise ValueError("missing LLM_API_KEY for provider {provider!r}".format(provider=provider))
        self.provider = provider
        self.model = model
        self._api_key = api_key
        self._module = module

    def complete(self, system: str, messages: list[dict], max_tokens: int) -> str:
        return self._module.complete(self.model, self._api_key, system, messages, max_tokens)


def from_env(env: dict[str, str] | None = None) -> ConfiguredClient:
    """Build the client from LLM_PROVIDER / LLM_MODEL / LLM_API_KEY.

    Raises ValueError naming the missing or invalid variable.
    """
    env = env if env is not None else dict(os.environ)
    provider = (env.get("LLM_PROVIDER") or "").strip()
    if not provider:
        raise ValueError("missing LLM_PROVIDER: set it to one of anthropic, openai, google, mock")
    if provider not in PROVIDERS:
        raise ValueError(f"invalid LLM_PROVIDER {provider!r}: want one of {sorted(PROVIDERS)}")
    model = (env.get("LLM_MODEL") or "").strip()
    if not model:
        raise ValueError("missing LLM_MODEL: set it to the model id passed to the provider")
    api_key = env.get("LLM_API_KEY") or ""
    return ConfiguredClient(provider, model, api_key)


__all__ = ["ConfiguredClient", "KEYLESS_PROVIDERS", "LLMClient", "PROVIDERS", "ProviderError", "from_env"]
