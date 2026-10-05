"""OpenAI provider: ``POST /v1/chat/completions`` over stdlib http (no SDK)."""

from __future__ import annotations

from .base import ProviderError, post_json

PROVIDER_NAME = "openai"
API_URL = "https://api.openai.com/v1/chat/completions"


def complete(model: str, api_key: str, system: str, messages: list[dict], max_tokens: int) -> str:
    full = [{"role": "system", "content": system}, *messages]
    payload = post_json(
        API_URL,
        {"Authorization": f"Bearer {api_key}"},
        {"model": model, "max_tokens": max_tokens, "messages": full},
    )
    choices = payload.get("choices")
    if not isinstance(choices, list) or not choices:
        raise ProviderError(f"empty reply: {str(payload)[:300]}")
    first = choices[0]
    content = first.get("message", {}).get("content") if isinstance(first, dict) else None
    if not isinstance(content, str) or not content:
        raise ProviderError(f"empty reply: {str(payload)[:300]}")
    return content
