"""Configuration: `.harness/config.json` for the knobs, the environment for secrets and run facts.

This is the only module that reads `os.environ`. Everything else receives a `Config`.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Mapping

from harness.errors import ConfigError

#: The repository root: `bot/harness/config.py` sits two directories below it.
REPO_ROOT = Path(__file__).resolve().parents[2]

CONFIG_PATH = Path(".harness") / "config.json"
TRUST_PATH = Path(".harness") / "trust.txt"
HALT_PATH = ".harness/HALT"
PROMPTS_DIR = Path(__file__).resolve().parents[1] / "prompts"

#: Every comment, issue and PR body the harness writes carries this, so the event workflow can
#: tell the bot's own words from a person's.
MARKER = "<!-- jackioh-bot -->"

STATE_BRANCH = "bot-state"
STATE_FILE = "state.json"
NIGHT_WORKFLOW = "bot-night.yml"

#: Environment keys whose values are secrets. They are redacted from everything the harness
#: writes and stripped from the model's environment.
SECRET_KEYS: tuple[str, ...] = (
    "BOT_GITHUB_TOKEN",
    "GITHUB_TOKEN",
    "GH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "ANTHROPIC_API_KEY",
    "ACTIONS_RUNTIME_TOKEN",
    "ACTIONS_ID_TOKEN_REQUEST_TOKEN",
)

LABELS: dict[str, tuple[str, str]] = {
    # name: (color, description)
    "bot:build": ("1d76db", "Queued for the night bot to build"),
    "bot:working": ("fbca04", "The night bot is working on this now"),
    "bot:pr-open": ("0e8a16", "The night bot opened a PR for this"),
    "bot:revise": ("5319e7", "Queued for the night bot to revise this PR"),
    "bot:blocked": ("b60205", "The night bot needs a person before it can go on"),
    "bot:pr": ("c5def5", "A pull request the night bot opened"),
    "bot:suggestion": ("d4c5f9", "An improvement the night bot suggests; add bot:build to build it"),
    "bot:needs-review": ("e99695", "A bot pull request that a person must merge: it touches review-only paths"),
}

LABEL_BUILD = "bot:build"
LABEL_WORKING = "bot:working"
LABEL_PR_OPEN = "bot:pr-open"
LABEL_REVISE = "bot:revise"
LABEL_BLOCKED = "bot:blocked"
LABEL_PR = "bot:pr"
LABEL_SUGGESTION = "bot:suggestion"
LABEL_NEEDS_REVIEW = "bot:needs-review"


@dataclass(frozen=True)
class Gate:
    name: str
    run: str
    timeout_minutes: int


@dataclass(frozen=True)
class Config:
    """Everything the harness reads. Built once by `load()`."""

    root: Path
    repo: str
    default_branch: str
    bot_login: str
    bot_user_id: int
    operator: str
    timezone: str
    window_start: str
    window_end: str
    model: str
    effort: str
    max_turns: Mapping[str, int]
    call_timeout_minutes: int
    job_budget_minutes: int
    min_minutes_for_a_call: int
    max_review_cycles: int
    max_failures: int
    usage_stop: Mapping[str, float]
    auto_merge: bool
    merge_method: str
    ci_workflow: str
    required_checks: tuple[str, ...]
    ci_reruns: int
    suggestions_enabled: bool
    suggestions_max_open: int
    suggestions_min_interval_hours: int
    forbidden_paths: tuple[str, ...]
    review_paths: tuple[str, ...]
    upload_transcripts: bool
    install: Gate
    gates: tuple[Gate, ...]
    # From the environment.
    bot_token: str = field(default="", repr=False)
    actions_token: str = field(default="", repr=False)
    backend: str = "cli"
    dry_run: bool = False
    claude_bin: str = "claude"
    run_id: str = ""
    server_url: str = "https://github.com"
    now_override: str = ""
    claude_token_present: bool = False
    #: True when the workflow said whether the Claude secret exists (the plan job never sees it).
    claude_ready_known: bool = False

    @property
    def write_token(self) -> str:
        """The token GitHub writes use: the bot account's when present, else the job's own."""
        return self.bot_token or self.actions_token

    @property
    def run_url(self) -> str:
        if not self.run_id:
            return ""
        return f"{self.server_url}/{self.repo}/actions/runs/{self.run_id}"


_REQUIRED = (
    "repo",
    "default_branch",
    "bot_login",
    "bot_user_id",
    "operator",
    "timezone",
    "window",
    "model",
    "effort",
    "max_turns",
    "call_timeout_minutes",
    "job_budget_minutes",
    "min_minutes_for_a_call",
    "max_review_cycles",
    "max_failures",
    "usage_stop",
    "auto_merge",
    "merge_method",
    "ci_workflow",
    "required_checks",
    "ci_reruns",
    "suggestions",
    "forbidden_paths",
    "review_paths",
    "upload_transcripts",
    "install",
    "gates",
)

_ROLES = ("build", "fix", "revise", "review", "suggest")


def _gate(raw: Any, where: str) -> Gate:
    if not isinstance(raw, Mapping):
        raise ConfigError(f"{where}: expected an object")
    try:
        return Gate(str(raw["name"]), str(raw["run"]), int(raw["timeout_minutes"]))
    except (KeyError, TypeError, ValueError) as exc:
        raise ConfigError(f"{where}: needs name, run and timeout_minutes ({exc})") from exc


def parse(raw: Mapping[str, Any], root: Path, env: Mapping[str, str]) -> Config:
    """A `Config` from the committed JSON and an environment mapping. Raises `ConfigError`."""
    missing = [key for key in _REQUIRED if key not in raw]
    if missing:
        raise ConfigError(f"{CONFIG_PATH}: missing keys {', '.join(missing)}")
    unknown = sorted(set(raw) - set(_REQUIRED))
    if unknown:
        raise ConfigError(f"{CONFIG_PATH}: unknown keys {', '.join(unknown)}")
    window = raw["window"]
    turns = {str(k): int(v) for k, v in dict(raw["max_turns"]).items()}
    absent = [role for role in _ROLES if role not in turns]
    if absent:
        raise ConfigError(f"max_turns: missing {', '.join(absent)}")
    stop = {str(k): float(v) for k, v in dict(raw["usage_stop"]).items()}
    for name, value in stop.items():
        if not 0.0 < value <= 1.0:
            raise ConfigError(f"usage_stop.{name}: {value} is outside (0, 1]")
    if raw["merge_method"] not in ("squash", "merge", "rebase"):
        raise ConfigError(f"merge_method: {raw['merge_method']!r} is not squash, merge or rebase")
    suggestions = dict(raw["suggestions"])
    gates = tuple(_gate(g, f"gates[{i}]") for i, g in enumerate(raw["gates"]))
    if not gates:
        raise ConfigError("gates: at least one gate is required")
    repo = env.get("GITHUB_REPOSITORY") or str(raw["repo"])
    backend = env.get("HARNESS_BACKEND", "cli") or "cli"
    if backend not in ("cli", "fake"):
        raise ConfigError(f"HARNESS_BACKEND: {backend!r} is not cli or fake")
    return Config(
        root=root,
        repo=repo,
        default_branch=str(raw["default_branch"]),
        bot_login=str(raw["bot_login"]),
        bot_user_id=int(raw["bot_user_id"]),
        operator=str(raw["operator"]),
        timezone=str(raw["timezone"]),
        window_start=str(window["start"]),
        window_end=str(window["end"]),
        model=str(raw["model"]),
        effort=str(raw["effort"]),
        max_turns=turns,
        call_timeout_minutes=int(raw["call_timeout_minutes"]),
        job_budget_minutes=int(raw["job_budget_minutes"]),
        min_minutes_for_a_call=int(raw["min_minutes_for_a_call"]),
        max_review_cycles=int(raw["max_review_cycles"]),
        max_failures=int(raw["max_failures"]),
        usage_stop=stop,
        auto_merge=bool(raw["auto_merge"]),
        merge_method=str(raw["merge_method"]),
        ci_workflow=str(raw["ci_workflow"]),
        required_checks=tuple(str(c) for c in raw["required_checks"]),
        ci_reruns=int(raw["ci_reruns"]),
        suggestions_enabled=bool(suggestions.get("enabled", True)),
        suggestions_max_open=int(suggestions.get("max_open", 4)),
        suggestions_min_interval_hours=int(suggestions.get("min_interval_hours", 20)),
        forbidden_paths=tuple(str(p) for p in raw["forbidden_paths"]),
        review_paths=tuple(str(p) for p in raw["review_paths"]),
        upload_transcripts=bool(raw["upload_transcripts"]),
        install=_gate(raw["install"] | {"name": "install"}, "install"),
        gates=gates,
        bot_token=env.get("BOT_GITHUB_TOKEN", ""),
        actions_token=env.get("GITHUB_TOKEN", "") or env.get("GH_TOKEN", ""),
        backend=backend,
        dry_run=env.get("HARNESS_DRY_RUN", "").lower() in ("1", "true", "yes"),
        claude_bin=env.get("HARNESS_CLAUDE_BIN", "claude") or "claude",
        run_id=env.get("GITHUB_RUN_ID", ""),
        server_url=env.get("GITHUB_SERVER_URL", "https://github.com") or "https://github.com",
        now_override=env.get("HARNESS_NOW", ""),
        claude_token_present=bool(env.get("CLAUDE_CODE_OAUTH_TOKEN"))
        or env.get("HARNESS_CLAUDE_READY", "").lower() == "true",
        claude_ready_known="HARNESS_CLAUDE_READY" in env,
    )


def load(root: Path | None = None, env: Mapping[str, str] | None = None) -> Config:
    """Read `.harness/config.json` under `root` (the repository root by default)."""
    base = Path(root) if root is not None else REPO_ROOT
    path = base / CONFIG_PATH
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise ConfigError(f"{path} does not exist") from exc
    except ValueError as exc:
        raise ConfigError(f"{path} is not valid JSON: {exc}") from exc
    if not isinstance(raw, dict):
        raise ConfigError(f"{path} must hold a JSON object")
    return parse(raw, base, os.environ if env is None else env)


def secret_values(env: Mapping[str, str] | None = None) -> list[str]:
    """The live values of every secret key, for redaction. Short values are ignored."""
    source = os.environ if env is None else env
    return [source[k] for k in SECRET_KEYS if len(source.get(k, "")) >= 8]


def child_env(env: Mapping[str, str] | None = None, keep: tuple[str, ...] = ()) -> dict[str, str]:
    """The environment for a child process: every secret key removed except those in `keep`."""
    source = os.environ if env is None else env
    return {k: v for k, v in source.items() if k not in SECRET_KEYS or k in keep}


def github_env() -> dict[str, str]:
    """The Actions facts a command reads: event name and path, output and summary files."""
    keys = ("GITHUB_EVENT_NAME", "GITHUB_EVENT_PATH", "GITHUB_OUTPUT", "GITHUB_STEP_SUMMARY")
    return {k: os.environ.get(k, "") for k in keys}
