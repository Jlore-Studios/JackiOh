"""Secret redaction. Everything the harness writes to GitHub or to an artifact passes through it."""

from __future__ import annotations

import re

from harness import config

REDACTION = "[REDACTED]"

_PATTERNS: tuple[re.Pattern[str], ...] = (
    re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----.*?-----END [A-Z ]*PRIVATE KEY-----", re.S),
    re.compile(r"sk-ant-[A-Za-z0-9_\-]{20,}"),
    re.compile(r"github_pat_[A-Za-z0-9_]{40,}"),
    re.compile(r"gh[pousr]_[A-Za-z0-9]{30,}"),
    re.compile(r"AKIA[0-9A-Z]{16}"),
    re.compile(r"(?i)\bbearer\s+[A-Za-z0-9._\-]{16,}"),
    re.compile(r"(?i)\bbasic\s+[A-Za-z0-9+/=]{24,}"),
)


def redact(text: str, extra: list[str] | None = None) -> str:
    """`text` with every known token shape and every live secret value replaced."""
    if not text:
        return text
    out = text
    for pattern in _PATTERNS:
        out = pattern.sub(REDACTION, out)
    values = list(config.secret_values()) + list(extra or [])
    for value in sorted(values, key=len, reverse=True):
        if value and value in out:
            out = out.replace(value, REDACTION)
    return out


def redact_json(obj: object) -> object:
    """Every string leaf of nested dicts and lists, redacted."""
    if isinstance(obj, str):
        return redact(obj)
    if isinstance(obj, dict):
        return {key: redact_json(value) for key, value in obj.items()}
    if isinstance(obj, (list, tuple)):
        return [redact_json(value) for value in obj]
    return obj
