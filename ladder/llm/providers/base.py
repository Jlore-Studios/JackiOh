"""LLM provider interface (issue #55, phase 3).

Provider-specific code lives in ``ladder/llm/providers/`` only: each provider
module exposes ``PROVIDER_NAME`` and ``complete(model, api_key, system,
messages, max_tokens)``. Nothing else may import a provider SDK — and none is
needed, since every provider here speaks its REST API over the standard
library. ``ProviderError`` is what a transport or API failure looks like.
"""

from __future__ import annotations

import json
import urllib.error
import urllib.request

# Seconds to wait for one model call before the generation stage fails loudly.
REQUEST_TIMEOUT_S = 120


class ProviderError(RuntimeError):
    pass


def post_json(url: str, headers: dict[str, str], body: dict, timeout: float = REQUEST_TIMEOUT_S) -> dict:
    """POST a JSON body and return the decoded JSON object, or raise ProviderError."""
    data = json.dumps(body).encode("utf-8")
    request = urllib.request.Request(url, data=data, headers={**headers, "Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            payload = json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode("utf-8", "replace")[:500]
        raise ProviderError(f"HTTP {exc.code}: {detail}") from exc
    except OSError as exc:
        raise ProviderError(f"request failed: {exc}") from exc
    if not isinstance(payload, dict):
        raise ProviderError("provider replied non-object JSON")
    return payload


def text_parts(parts: object) -> str:
    """Join a response's text parts; non-text parts contribute nothing."""
    if not isinstance(parts, list):
        return ""
    out = []
    for part in parts:
        if isinstance(part, dict) and isinstance(part.get("text"), str):
            out.append(part["text"])
    return "".join(out)
