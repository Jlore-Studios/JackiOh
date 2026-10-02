"""A provider's login, for one model job.

A **machine** login (`"login": "machine"` in providers.json) was made once, by hand, on the
bot's own machine as that provider's Linux user (`agent-<id>`, whose runner alone runs its jobs;
`bot/machine/README.md`): the CLI finds it in its usual place in that home, so `prepare` writes
nothing and only sets the CLI's environment. agy logs in only this way.

A **secret** login: the workflow hands the model job one secret, its provider's, as
`HARNESS_PROVIDER_SECRET`. `prepare` turns it into what that CLI reads, under a private directory
outside every worktree, and returns the environment the CLI needs; the CLI's own home stays
untouched.

| CLI | The secret holds | Written as |
|---|---|---|
| claude | the token `claude setup-token` prints | `CLAUDE_CODE_OAUTH_TOKEN`, with a `CLAUDE_CONFIG_DIR` of its own |
| codex | `~/.codex/auth.json` after `codex login` (ChatGPT) | `$CODEX_HOME/auth.json`, stored as a file |
| muse | `~/.config/muse/auth.json` after `muse login`, or an API key | `$XDG_CONFIG_HOME/muse/auth.json`, or `META_API_KEY` (billed per token) |

A JSON secret may be pasted as it is or base64-encoded. A login refreshed by an earlier run (the
vault, `vault.py`) wins over the pasted one when it opens under the same secret. After the job,
`seal` reads the files back; if the CLI rotated them, the new ones go into the vault.
"""

from __future__ import annotations

import base64
import binascii
import json
import os
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from harness import redact, vault
from harness.clock import iso, now as clock_now
from harness.errors import LoginError
from harness.providers import Provider

#: What a secret must look like for each CLI that takes a login file, for the error a person reads.
FILE_HINTS = {
    "codex": "the contents of ~/.codex/auth.json after `codex login` (Sign in with ChatGPT)",
    "muse": "the contents of ~/.config/muse/auth.json after `muse login`, or a META_API_KEY",
}
#: Environment each CLI gets on top of its login, either way.
CLI_ENV: dict[str, dict[str, str]] = {
    "muse": {"MUSE_NO_AUTO_UPDATE": "1"},
}


@dataclass
class Login:
    provider: str
    cli: str
    home: Path
    env: dict[str, str]
    #: The files that make the login, by name: what `seal` keeps when the CLI changes them.
    files: dict[str, Path] = field(default_factory=dict)
    initial: dict[str, str] = field(default_factory=dict, repr=False)
    secret: str = field(default="", repr=False)
    #: Where the login came from: `secret`, `vault`, or `api key`.
    source: str = "secret"


def _json_text(secret: str) -> str | None:
    """The secret as JSON text: as pasted, or base64-decoded. None when it is neither."""
    text = secret.strip()
    candidates = [text]
    try:
        candidates.append(base64.b64decode(text, validate=True).decode("utf-8"))
    except (binascii.Error, ValueError, UnicodeDecodeError):
        pass
    for candidate in candidates:
        try:
            if isinstance(json.loads(candidate), dict):
                return candidate
        except ValueError:
            continue
    return None


def _leaves(text: str) -> list[str]:
    """Every string inside a JSON document: the tokens a transcript must never show."""
    found: list[str] = []
    def walk(node: Any) -> None:
        if isinstance(node, str):
            found.append(node)
        elif isinstance(node, dict):
            for value in node.values():
                walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)
    try:
        walk(json.loads(text))
    except ValueError:
        pass
    return found


def _write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    path.write_text(text, encoding="utf-8")
    os.chmod(path, 0o600)


def prepare(provider: Provider, secret: str, vault_text: str, home: Path) -> Login:
    """Write `provider`'s login under `home`, or for a machine login, only its environment.
    Raises `LoginError` when the secret is unusable."""
    if provider.login == "machine":
        env = {**CLI_ENV.get(provider.cli, {}), **dict(provider.env)}
        return Login(provider.id, provider.cli, Path(home), env, source="machine")
    secret = (secret or "").strip()
    if not secret:
        raise LoginError(f"`{provider.id}`'s secret `{provider.secret}` is empty in this job")
    home = Path(home)
    home.mkdir(parents=True, exist_ok=True, mode=0o700)
    redact.remember([secret])
    env = dict(provider.env)
    login = Login(provider.id, provider.cli, home, env, secret=secret)
    saved: dict[str, str] = {}
    opened = vault.open_(vault_text, secret) if vault_text else None
    if opened and opened.get("provider") == provider.id and isinstance(opened.get("files"), dict):
        saved = {str(k): str(v) for k, v in opened["files"].items()}
    if provider.cli == "claude":
        config_dir = home / "claude"
        config_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
        env.update(CLAUDE_CODE_OAUTH_TOKEN=secret, CLAUDE_CONFIG_DIR=str(config_dir))
        return login
    if provider.cli == "codex":
        codex_home = home / "codex"
        _install(login, "auth.json", codex_home / "auth.json", saved, secret)
        _write(codex_home / "config.toml", 'cli_auth_credentials_store = "file"\n')
        env["CODEX_HOME"] = str(codex_home)
    elif provider.cli == "muse":
        env.update(CLI_ENV["muse"])
        if saved.get("auth.json") or _json_text(secret):
            config_home = home / "muse-config"
            _install(login, "auth.json", config_home / "muse" / "auth.json", saved, secret)
            env["XDG_CONFIG_HOME"] = str(config_home)
        else:
            # Not a login file: an API key, which Meta bills per token, not on the subscription.
            env["META_API_KEY"] = secret
            login.source = "api key"
    else:
        raise LoginError(f"the {provider.cli} CLI has no secret login; it logs in on the machine")
    for path in login.files.values():
        redact.remember(_leaves(path.read_text(encoding="utf-8")))
    return login


def _install(login: Login, name: str, path: Path, saved: dict[str, str], secret: str) -> None:
    text = saved.get(name)
    if text:
        login.source = "vault"
    else:
        text = _json_text(secret)
        if text is None:
            raise LoginError(f"`{login.provider}`'s secret is not {FILE_HINTS[login.cli]} "
                             "(as JSON, or base64 of it)")
    _write(path, text)
    login.files[name] = path
    login.initial[name] = text


def seal(login: Login) -> str | None:
    """The login's files sealed for the vault when the CLI changed them during the job, else
    None (nothing new to keep)."""
    current = {name: path.read_text(encoding="utf-8")
               for name, path in login.files.items() if path.is_file()}
    if not current or current == login.initial:
        return None
    redact.remember([leaf for text in current.values() for leaf in _leaves(text)])
    return vault.seal({"provider": login.provider, "at": iso(clock_now()), "files": current},
                      login.secret)
