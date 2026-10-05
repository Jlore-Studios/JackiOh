"""Configuration: `.harness/config.json` for the knobs, the environment for secrets and run facts.

This is the only module that reads `os.environ`. Everything else receives a `Config`.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Mapping

from harness import providers as providers_mod
from harness.errors import ConfigError
from harness.providers import Pool, Secrets

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

#: The environment key a run's model job receives its provider's secret in (`providers.py`).
PROVIDER_SECRET_KEY = "HARNESS_PROVIDER_SECRET"

#: Environment keys whose values are secrets. They are redacted from everything the harness
#: writes and stripped from the model's environment; a backend puts back only its own login.
SECRET_KEYS: tuple[str, ...] = (
    "BOT_GITHUB_TOKEN",
    "GITHUB_TOKEN",
    "GH_TOKEN",
    PROVIDER_SECRET_KEY,
    *providers_mod.SECRETS,
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "OPENAI_API_KEY",
    "CODEX_API_KEY",
    "GEMINI_API_KEY",
    "GOOGLE_API_KEY",
    "META_API_KEY",
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
    "ready for merge": ("0e8a16", "The night bot's reviews approved it, but auto-merge could not turn on: a person merges it"),
    "bot:cross-review": ("0052cc", "A night bot pull request waiting for its review: one strong model, or a second medium one"),
    "bot:needs-plan": ("1d76db", "Queued for the night bot: a strong model plans it first, into the description"),
    "bot:planned": ("0075ca", "It has a plan: in its description, which the builder starts from"),
    "bot:stuck": ("d93f0b", "It failed every review round it had: a comment says why, for a person to review"),
    "human": ("ededed", "People do this: the night bot never queues, plans, builds or labels it"),
    "difficulty:easy": ("c2e0c6", "Any model may build it, the weakest first (Sonnet, Devin)"),
    "difficulty:medium": ("fef2c0", "A medium model or stronger builds it (the default with no difficulty label)"),
    "difficulty:hard": ("b60205", "Only a strong model (Opus) plans, builds and reviews it"),
    "priority:high": ("d73a4a", "The night bot picks this up first"),
    "priority:medium": ("fbca04", "The night bot picks this up after priority:high"),
    "priority:low": ("0e8a16", "The night bot picks this up last, after unlabelled work"),
}

LABEL_BUILD = "bot:build"
LABEL_WORKING = "bot:working"
LABEL_PR_OPEN = "bot:pr-open"
LABEL_REVISE = "bot:revise"
LABEL_BLOCKED = "bot:blocked"
LABEL_PR = "bot:pr"
LABEL_SUGGESTION = "bot:suggestion"
LABEL_NEEDS_REVIEW = "bot:needs-review"
#: A bot pull request whose head the review rule approved but which auto-merge could not take
#: (a review-only path, `main`'s protection, GitHub refusing it, or `auto_merge` off), so it
#: waits for a person to merge it. It comes off when the pull request goes back into the queue (`queue.set_state_label`).
LABEL_READY = "ready for merge"
LABEL_CROSS = "bot:cross-review"
#: The Needs plan stage: a queued item (still `bot:build`) waiting for a strong model's plan, which
#: goes into its description (`issueplan.py`). Not a state label: it sits beside `bot:build`.
LABEL_NEEDS_PLAN = "bot:needs-plan"
#: A plan is in the description (`issueplan.py`): beside `bot:build` while queued, and kept after.
LABEL_PLANNED = "bot:planned"
#: A build or revision used every review round (`max_review_cycles`) without an approval; its
#: comment says why, round by round (`failures.py`), for a person to review.
LABEL_STUCK = "bot:stuck"
#: People do a thread labelled `human` (#96): a decision, an account or a secret, or repository
#: work the bot may not do (a change to bot/, .harness/ or .github/) or that a person is already
#: building. The bot never queues, plans, builds, labels or assigns it, and a request to build it
#: gets a reply saying so.
LABEL_HUMAN = "human"
#: An item's difficulty decides which models may plan, build and review it; no label counts as
#: medium, and with several the hardest counts. Like `human`, these match whatever their case.
DIFFICULTIES = ("easy", "medium", "hard")
DIFFICULTY_LABELS = {f"difficulty:{name}": name for name in DIFFICULTIES}
DEFAULT_DIFFICULTY = "medium"
#: The weakest tier that may build an item of each difficulty (`providers.TIERS`).
MIN_TIER = {"easy": "weak", "medium": "medium", "hard": "strong"}
#: The weakest tier whose plan an item of each difficulty builds from (#317 part 6): a medium
#: model may plan an easy item, only a strong one anything harder. An item no one has rated yet
#: (`UNRATED_PLAN_FLOOR`) may be rated and planned by a medium model, whose plan then stands only
#: if it rates the item easy (#317 part 8).
PLAN_FLOOR = {"easy": "medium", "medium": "strong", "hard": "strong"}
UNRATED_PLAN_FLOOR = "medium"
#: Failures of an item's own (not infra, a usage pause, a halt or a stop) at one difficulty before
#: the bot raises it a step, easy to medium to hard; three more at hard block it (#317 part 8).
STEP_UP_AFTER = 3
#: The pickup tiers, first to last; a thread with no priority label sits between medium and low
#: (#90). Like `human`, these match whatever their case.
LABEL_PRIORITY_HIGH = "priority:high"
LABEL_PRIORITY_MEDIUM = "priority:medium"
LABEL_PRIORITY_LOW = "priority:low"


@dataclass(frozen=True)
class Gate:
    name: str
    run: str
    timeout_minutes: int
    #: Runs in a model job on the bot's machine too. `false` leaves it to CI on the pull request
    #: there: the machine's two vCPUs are shared by every job on it, GitHub's runners are not.
    machine: bool = True


@dataclass(frozen=True)
class EasyRule:
    """The limits of `difficulty:easy` (`harness/easy.py`), from `easy` in `.harness/config.json`:
    at most `max_files` files and `max_lines` lines added plus removed (`generated` files aside),
    and nothing under `off_limits`."""

    max_files: int = 10
    max_lines: int = 400
    off_limits: tuple[str, ...] = ()
    generated: tuple[str, ...] = ()


@dataclass(frozen=True)
class Partner:
    """Another bot on the same subscription, and the steps of its workflows that spend it."""

    repo: str
    workflows: Mapping[str, tuple[str, ...]]


@dataclass(frozen=True)
class Quiet:
    enabled: bool
    interval_minutes: int
    max_wait_minutes: int
    ping_model: str
    partners: tuple[Partner, ...]


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
    max_turns: Mapping[str, int]
    call_timeout_minutes: int
    job_budget_minutes: int
    min_minutes_for_a_call: int
    max_review_cycles: int
    max_failures: int
    auto_merge: bool
    merge_method: str
    ci_workflow: str
    required_checks: tuple[str, ...]
    ci_reruns: int
    suggestions_enabled: bool
    suggestions_max_open: int
    suggestions_min_interval_hours: int
    quiet: Quiet
    forbidden_paths: tuple[str, ...]
    review_paths: tuple[str, ...]
    upload_transcripts: bool
    install: Gate
    gates: tuple[Gate, ...]
    #: The subscriptions, from `.harness/providers.json`.
    pool: Pool
    #: Self checks a self-checking builder gets before its change goes to review anyway.
    max_self_check_rounds: int = 3
    #: What `difficulty:easy` allows (`harness/easy.py`).
    easy: EasyRule = field(default_factory=EasyRule)
    # From the environment.
    bot_token: str = field(default="", repr=False)
    actions_token: str = field(default="", repr=False)
    backend: str = "cli"
    dry_run: bool = False
    #: The binary of each CLI (`HARNESS_<CLI>_BIN` overrides it, for tests).
    cli_bins: Mapping[str, str] = field(default_factory=dict)
    run_id: str = ""
    server_url: str = "https://github.com"
    now_override: str = ""
    #: Which provider secrets the workflow has; the plan job learns that, never their values.
    secrets: Secrets = field(default_factory=lambda: Secrets(frozenset(), False))
    #: The model job's one provider secret (`HARNESS_PROVIDER_SECRET`), or, run by hand, the
    #: provider secrets this shell happens to hold, by name.
    provider_secrets: Mapping[str, str] = field(default_factory=dict, repr=False)

    def secret_for(self, name: str) -> str:
        return self.provider_secrets.get(PROVIDER_SECRET_KEY) or self.provider_secrets.get(name, "")

    def bin(self, cli: str) -> str:
        return self.cli_bins.get(cli) or cli

    @property
    def claude_bin(self) -> str:
        return self.bin("claude")

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
    "max_turns",
    "call_timeout_minutes",
    "job_budget_minutes",
    "min_minutes_for_a_call",
    "max_review_cycles",
    "max_failures",
    "auto_merge",
    "merge_method",
    "ci_workflow",
    "required_checks",
    "ci_reruns",
    "suggestions",
    "quiet",
    "forbidden_paths",
    "review_paths",
    "upload_transcripts",
    "install",
    "gates",
)

_ROLES = ("plan", "build", "fix", "revise", "review", "suggest")
#: Keys `.harness/config.json` may leave out, with their defaults.
_OPTIONAL = {"max_self_check_rounds": 3, "easy": {}}


def _easy(raw: Any) -> EasyRule:
    if not isinstance(raw, Mapping):
        raise ConfigError("easy: expected an object")
    unknown = sorted(set(raw) - {"max_files", "max_lines", "off_limits", "generated"})
    if unknown:
        raise ConfigError(f"easy: unknown keys {', '.join(unknown)}")
    try:
        rule = EasyRule(int(raw.get("max_files", 10)), int(raw.get("max_lines", 400)),
                        tuple(str(p) for p in raw.get("off_limits") or ()),
                        tuple(str(p) for p in raw.get("generated") or ()))
    except (TypeError, ValueError) as exc:
        raise ConfigError(f"easy: {exc}") from exc
    if rule.max_files < 1 or rule.max_lines < 1:
        raise ConfigError("easy: max_files and max_lines must be at least 1")
    return rule


def _gate(raw: Any, where: str) -> Gate:
    if not isinstance(raw, Mapping):
        raise ConfigError(f"{where}: expected an object")
    try:
        return Gate(str(raw["name"]), str(raw["run"]), int(raw["timeout_minutes"]),
                    machine=bool(raw.get("machine", True)))
    except (KeyError, TypeError, ValueError) as exc:
        raise ConfigError(f"{where}: needs name, run and timeout_minutes ({exc})") from exc


def _quiet(raw: Any) -> Quiet:
    if not isinstance(raw, Mapping):
        raise ConfigError("quiet: expected an object")
    partners = []
    for i, entry in enumerate(raw.get("partners") or []):
        try:
            workflows = {str(k): tuple(str(s) for s in v) for k, v in dict(entry["workflows"]).items()}
            partners.append(Partner(str(entry["repo"]), workflows))
        except (KeyError, TypeError, ValueError) as exc:
            raise ConfigError(f"quiet.partners[{i}]: needs repo and workflows ({exc})") from exc
    interval = int(raw.get("interval_minutes", 10))
    if interval < 1:
        raise ConfigError("quiet.interval_minutes: must be at least 1")
    return Quiet(
        enabled=bool(raw.get("enabled", True)),
        interval_minutes=interval,
        max_wait_minutes=int(raw.get("max_wait_minutes", 120)),
        ping_model=str(raw.get("ping_model", "haiku")),
        partners=tuple(partners),
    )


def parse(raw: Mapping[str, Any], root: Path, env: Mapping[str, str],
          pool: Pool | None = None) -> Config:
    """A `Config` from the committed JSON and an environment mapping. Raises `ConfigError`.
    The subscriptions come from `pool`, or from `.harness/providers.json` under `root`."""
    missing = [key for key in _REQUIRED if key not in raw]
    if missing:
        raise ConfigError(f"{CONFIG_PATH}: missing keys {', '.join(missing)}")
    unknown = sorted(set(raw) - set(_REQUIRED) - set(_OPTIONAL))
    if unknown:
        raise ConfigError(f"{CONFIG_PATH}: unknown keys {', '.join(unknown)}")
    turns = {str(k): int(v) for k, v in dict(raw["max_turns"]).items()}
    absent = [role for role in _ROLES if role not in turns]
    if absent:
        raise ConfigError(f"max_turns: missing {', '.join(absent)}")
    if raw["merge_method"] not in ("squash", "merge", "rebase"):
        raise ConfigError(f"merge_method: {raw['merge_method']!r} is not squash, merge or rebase")
    suggestions = dict(raw["suggestions"])
    gates = tuple(_gate(g, f"gates[{i}]") for i, g in enumerate(raw["gates"]))
    if not gates:
        raise ConfigError("gates: at least one gate is required")
    self_checks = int(raw.get("max_self_check_rounds", _OPTIONAL["max_self_check_rounds"]))
    if self_checks < 1:
        raise ConfigError("max_self_check_rounds: must be at least 1")
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
        max_turns=turns,
        call_timeout_minutes=int(raw["call_timeout_minutes"]),
        job_budget_minutes=int(raw["job_budget_minutes"]),
        min_minutes_for_a_call=int(raw["min_minutes_for_a_call"]),
        max_review_cycles=int(raw["max_review_cycles"]),
        max_failures=int(raw["max_failures"]),
        auto_merge=bool(raw["auto_merge"]),
        merge_method=str(raw["merge_method"]),
        ci_workflow=str(raw["ci_workflow"]),
        required_checks=tuple(str(c) for c in raw["required_checks"]),
        ci_reruns=int(raw["ci_reruns"]),
        suggestions_enabled=bool(suggestions.get("enabled", True)),
        suggestions_max_open=int(suggestions.get("max_open", 4)),
        suggestions_min_interval_hours=int(suggestions.get("min_interval_hours", 20)),
        quiet=_quiet(raw["quiet"]),
        forbidden_paths=tuple(str(p) for p in raw["forbidden_paths"]),
        review_paths=tuple(str(p) for p in raw["review_paths"]),
        upload_transcripts=bool(raw["upload_transcripts"]),
        install=_gate(raw["install"] | {"name": "install"}, "install"),
        gates=gates,
        pool=pool if pool is not None else providers_mod.load(root),
        max_self_check_rounds=self_checks,
        easy=_easy(raw.get("easy", _OPTIONAL["easy"])),
        bot_token=env.get("BOT_GITHUB_TOKEN", ""),
        actions_token=env.get("GITHUB_TOKEN", "") or env.get("GH_TOKEN", ""),
        backend=backend,
        dry_run=env.get("HARNESS_DRY_RUN", "").lower() in ("1", "true", "yes"),
        cli_bins={cli: env[f"HARNESS_{cli.upper()}_BIN"] for cli in providers_mod.CLIS
                  if env.get(f"HARNESS_{cli.upper()}_BIN")},
        run_id=env.get("GITHUB_RUN_ID", ""),
        server_url=env.get("GITHUB_SERVER_URL", "https://github.com") or "https://github.com",
        now_override=env.get("HARNESS_NOW", ""),
        secrets=Secrets.of(env),
        provider_secrets={k: env[k] for k in (PROVIDER_SECRET_KEY, *providers_mod.SECRETS)
                          if env.get(k)},
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
