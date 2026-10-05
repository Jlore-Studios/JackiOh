"""Google provider: ``generateContent`` over stdlib http (no SDK).

Roles map ``assistant`` to ``model``; a leading system prompt travels as
``system_instruction``.
"""

from __future__ import annotations

import urllib.parse

from .base import ProviderError, post_json, text_parts

PROVIDER_NAME = "google"


def _role(role: object) -> str:
    return "model" if role == "assistant" else "user"


def complete(model: str, api_key: str, system: str, messages: list[dict], max_tokens: int) -> str:
    query = urllib.parse.urlencode({"key": api_key})
    url = f"https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?{query}"
    contents = [
        {"role": _role(m.get("role")), "parts": [{"text": m.get("content", "")}]}
        for m in messages
    ]
    payload = post_json(
        url,
        {},
        {
            "system_instruction": {"parts": [{"text": system}]},
            "contents": contents,
            "generationConfig": {"maxOutputTokens": max_tokens},
        },
    )
    candidates = payload.get("candidates")
    if not isinstance(candidates, list) or not candidates:
        raise ProviderError(f"empty reply: {str(payload)[:300]}")
    first = candidates[0]
    content = first.get("content", {}) if isinstance(first, dict) else {}
    text = text_parts(content.get("parts")) if isinstance(content, dict) else ""
    if not text:
        raise ProviderError(f"empty reply: {str(payload)[:300]}")
    return text
