"""Anthropic provider: ``POST /v1/messages`` over stdlib http (no SDK)."""

from __future__ import annotations

from .base import ProviderError, post_json, text_parts

PROVIDER_NAME = "anthropic"
API_URL = "https://api.anthropic.com/v1/messages"
API_VERSION = "2023-06-01"


def complete(model: str, api_key: str, system: str, messages: list[dict], max_tokens: int) -> str:
    payload = post_json(
        API_URL,
        {"x-api-key": api_key, "anthropic-version": API_VERSION},
        {"model": model, "max_tokens": max_tokens, "system": system, "messages": messages},
    )
    text = text_parts(payload.get("content"))
    if not text:
        raise ProviderError(f"empty reply: {str(payload)[:300]}")
    return text
