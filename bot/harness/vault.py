"""Refreshed logins, kept encrypted between runs.

Some CLIs rotate their login as they use it: Codex replaces its refresh token on every refresh,
so the `auth.json` pasted into a secret stops working after one run unless the refreshed file is
kept. Runners are thrown away, and the state branch is public, so the refreshed files are
encrypted before they leave the model job, and `deliver` (which holds the bot's token but no key)
stores the ciphertext on `bot-state` as `vault/<provider>.enc`. The next run's `plan` passes it
back, and the model job decrypts it with a key derived from that provider's own secret. A run
can therefore open only its own provider's vault, never another's, and a new secret (a fresh
login pasted in) makes the old vault unreadable, so the fresh login wins.

The cipher is AES-256-CBC through the `openssl` command (the standard library has none), with a
SHA-256 HMAC over the ciphertext so a tampered or truncated vault is refused, not half-used.
"""

from __future__ import annotations

import base64
import hashlib
import hmac
import json
import os
import subprocess
from typing import Any, Mapping

VERSION = "v1"
#: A vault larger than this is refused rather than stored or opened.
MAX_BYTES = 200_000
PBKDF2_ITER = "200000"


def _keys(secret: str) -> tuple[str, bytes]:
    root = hashlib.sha256(b"jackioh-bot-vault\0" + secret.encode("utf-8")).digest()
    cipher = hashlib.sha256(b"cipher\0" + root).hexdigest()
    mac = hashlib.sha256(b"mac\0" + root).digest()
    return cipher, mac


def _openssl(args: list[str], data: bytes, password: str) -> bytes:
    env = {"PATH": os.environ.get("PATH", "/usr/bin:/bin"), "VAULT_PASS": password}
    proc = subprocess.run(["openssl", "enc", "-aes-256-cbc", "-pbkdf2", "-iter", PBKDF2_ITER,
                           "-md", "sha256", "-pass", "env:VAULT_PASS", *args],
                          input=data, capture_output=True, env=env, check=False, timeout=60)
    if proc.returncode != 0:
        raise ValueError(f"openssl failed: {proc.stderr.decode('utf-8', 'replace')[:200]}")
    return proc.stdout


def seal(payload: Mapping[str, Any], secret: str) -> str:
    """`payload` (JSON-able) encrypted under a key from `secret`, as one line of text."""
    plain = json.dumps(payload, sort_keys=True).encode("utf-8")
    cipher_key, mac_key = _keys(secret)
    body = _openssl(["-e", "-salt"], plain, cipher_key)
    tag = hmac.new(mac_key, body, hashlib.sha256).hexdigest()
    text = f"{VERSION}:{tag}:{base64.b64encode(body).decode('ascii')}"
    if len(text) > MAX_BYTES:
        raise ValueError(f"the vault is {len(text)} bytes, over {MAX_BYTES}")
    return text


def open_(text: str, secret: str) -> dict[str, Any] | None:
    """The payload sealed in `text`, or None when it is missing, tampered with, or sealed under
    another secret."""
    if not text or len(text) > MAX_BYTES or not secret:
        return None
    try:
        version, tag, encoded = text.strip().split(":", 2)
        if version != VERSION:
            return None
        body = base64.b64decode(encoded, validate=True)
        cipher_key, mac_key = _keys(secret)
        if not hmac.compare_digest(tag, hmac.new(mac_key, body, hashlib.sha256).hexdigest()):
            return None
        payload = json.loads(_openssl(["-d"], body, cipher_key).decode("utf-8"))
    except (ValueError, OSError, subprocess.SubprocessError):
        return None
    return payload if isinstance(payload, dict) else None


def looks_sealed(text: str) -> bool:
    """A cheap check `deliver` makes before storing a vault it cannot open: the right shape and
    size, so a model cannot park arbitrary text on the state branch."""
    if not text or len(text) > MAX_BYTES:
        return False
    parts = text.strip().split(":", 2)
    if len(parts) != 3 or parts[0] != VERSION or len(parts[1]) != 64:
        return False
    try:
        int(parts[1], 16)
        base64.b64decode(parts[2], validate=True)
    except ValueError:
        return False
    return True
