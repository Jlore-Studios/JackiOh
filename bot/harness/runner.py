"""Model calls: one backend per CLI (`claude`, `codex`, `gemini`, `muse`), and a scripted fake.

Every backend runs its CLI the same way (`_Cli._launch`): the prompt on stdin or in a file, the
output written straight to a file so a long session never sits in memory, a timeout that kills
the whole process group, and afterwards every process the call left behind reaped. The child's
environment has every secret stripped (`config.child_env`) and gets back only its own login
(`logins.Login.env`). What differs is the command line, how the output is read (the final text,
success, the subscription's usage, a refusal and when it resets) and how a transcript is boiled
down to a trail for the agent that picks the work up next (`trail`).

Claude Code takes the system prompt as an appended system prompt and its tool lists as flags.
The other CLIs get the system text at the top of the prompt, with a note that the instructions
were written for Claude Code, and their own guard rails: Codex's workspace sandbox with the
network off, a Gemini policy file, Muse's `--yolo` inside a job that holds no write token.
"""

from __future__ import annotations

import json
import os
import re
import signal
import subprocess
import time
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Callable, Iterable, Mapping

from harness import clock
from harness import config as config_mod
from harness.redact import redact

RATE_LIMIT_WORDS = re.compile(
    r"(?i)(usage limit|rate limit|too many requests|limit reached|hit your (?:\w+ )?limit"
    r"|quota exceeded|exhausted your (?:\w+ )?quota|resource_exhausted|terminalquotaerror)"
)
AUTH_WORDS = re.compile(
    r"(?i)(invalid api key|authentication[_ ]error|oauth token|not logged in|please run /login"
    r"|invalid bearer|401 unauthorized|credit balance is too low|sign in again"
    r"|refresh token (?:has expired|was already used|was revoked)|could not be refreshed"
    r"|manual authorization is required|no meta credentials|api key from meta_api_key was rejected"
    r"|run `?muse login)"
)
USAGE_WINDOWS = ("five_hour", "seven_day")
#: Set in every model call's environment, so the processes it leaves behind can be found.
CALL_MARKER = "JACKIOH_BOT_CALL"
#: The Actions file-command variables: a process holding one can set the environment, path or
#: outputs of the job's later steps, so the model never sees them.
ACTIONS_FILES = ("GITHUB_ENV", "GITHUB_PATH", "GITHUB_OUTPUT", "GITHUB_STATE", "GITHUB_STEP_SUMMARY")
#: No telemetry, error reporting or update checks: the session talks to the API and nothing else.
QUIET_ENV = {"CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1", "DISABLE_AUTOUPDATER": "1"}
EXIT_TIMEOUT = 124
EXIT_NOT_FOUND = 127
#: When a refusal names no reset time: try again after this long.
DEFAULT_PARK = "+PT60M"
#: Gemini's daily quota resets at midnight Pacific time.
GEMINI_QUOTA_ZONE = "America/Los_Angeles"
#: How much of a transcript the next agent is shown (`trail`).
TRAIL_ENTRIES = 40
TRAIL_CHARS = 6000

CLI_NOTE = """\
## You are running in the {cli} CLI

These instructions were written for Claude Code. Where they mention subagents, `TodoWrite`,
`EnterWorktree` or another tool you do not have, do that work yourself, one step after another.
`CLAUDE.md` binds you exactly as it binds Claude: read it, and `AGENTS.md`, before you change
anything."""


@dataclass(frozen=True)
class RunRequest:
    role: str
    prompt: str
    cwd: Path
    system_append: str
    allowed_tools: tuple[str, ...]
    disallowed_tools: tuple[str, ...]
    max_turns: int
    timeout_s: int
    model: str
    effort: str
    transcript: Path | None = None
    #: A reviewer: may read and run things but not change them (the harness resets it anyway).
    read_only: bool = False
    #: Directories outside `cwd` the session may need to write (a worktree's git directory).
    extra_dirs: tuple[str, ...] = ()


@dataclass
class RunResult:
    ok: bool
    text: str
    exit_code: int = 0
    turns: int | None = None
    duration_s: float = 0.0
    error: str | None = None
    usage: dict[str, Any] | None = None
    reset_at: str | None = None
    timed_out: bool = False
    extra: dict[str, Any] = field(default_factory=dict)
    #: The backend knows the CLI could not run at all (its exit code says so).
    infra_hint: bool = False

    @property
    def infra(self) -> bool:
        """The CLI could not run at all: missing, or refused by authentication."""
        if self.ok:
            return False
        return (self.infra_hint or self.exit_code == EXIT_NOT_FOUND
                or bool(AUTH_WORDS.search(self.error or "")))

    @property
    def rate_limited(self) -> bool:
        if self.reset_at:
            return True
        if isinstance(self.usage, dict) and self.usage.get("status") == "rejected":
            return not self.ok
        return not self.ok and bool(RATE_LIMIT_WORDS.search(self.error or ""))


def _epoch_iso(value: Any) -> str | None:
    if isinstance(value, bool) or value is None:
        return None
    if isinstance(value, (int, float)):
        return datetime.fromtimestamp(float(value), timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    if isinstance(value, str) and value.strip():
        return _epoch_iso(int(value)) if value.strip().isdigit() else value.strip()
    return None


def usage_from_event(event: Mapping[str, Any]) -> dict[str, Any] | None:
    """One `rate_limit_event` as `{five_hour: {...}, seven_day: {...}, status}`, or None."""
    info = event.get("rate_limit_info") if isinstance(event, Mapping) else None
    if not isinstance(info, Mapping):
        return None
    windows = info.get("unifiedWindows")
    usage: dict[str, Any] = {}
    if isinstance(windows, Mapping):
        for name in USAGE_WINDOWS:
            window = windows.get(name)
            if isinstance(window, Mapping) and isinstance(window.get("utilization"), (int, float)):
                usage[name] = {
                    "utilization": float(window["utilization"]),
                    "resets_at": _epoch_iso(window.get("resetsAt")),
                }
    status = info.get("status")
    if isinstance(status, str):
        usage["status"] = status
        if status == "rejected":
            if info.get("isUsingOverage") or info.get("overageInUse"):
                usage["status"] = "allowed_overage"
            else:
                usage["rejected_resets_at"] = _epoch_iso(info.get("resetsAt"))
    return usage or None


def _json_lines(lines: Iterable[Any]) -> Iterable[dict[str, Any]]:
    for line in lines:
        text = line.strip() if isinstance(line, str) else ""
        if not text.startswith("{"):
            continue
        try:
            event = json.loads(text)
        except ValueError:
            continue
        if isinstance(event, dict):
            yield event


def parse_stream(lines: Any) -> tuple[dict[str, Any] | None, dict[str, Any] | None]:
    """`(result line, last usage reading)` from Claude's stream-json lines."""
    result: dict[str, Any] | None = None
    usage: dict[str, Any] | None = None
    for event in _json_lines(lines):
        if event.get("type") == "result":
            result = event
        elif event.get("type") == "rate_limit_event":
            found = usage_from_event(event)
            if found is not None:
                usage = found
    return result, usage


def _clip(text: Any, limit: int = 300) -> str:
    flat = " ".join(str(text or "").split())
    return flat if len(flat) <= limit else flat[: limit - 1] + "…"


def _trail_text(entries: list[str]) -> str:
    text = "\n".join(f"- {e}" for e in entries[-TRAIL_ENTRIES:])
    return redact(text[-TRAIL_CHARS:])


def _read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


@dataclass
class _Launch:
    code: int
    timed_out: bool
    raw: Path
    stderr: str
    elapsed: float


class _Cli:
    """What every CLI backend shares: the environment, the launch, the transcript."""

    cli = ""

    def __init__(self, binary: str, login: Any = None,
                 popen: Callable[..., subprocess.Popen] = subprocess.Popen) -> None:
        self.binary = binary
        self.login = login
        self._popen = popen

    @property
    def claude_bin(self) -> str:  # the name older callers used
        return self.binary

    def env(self) -> dict[str, str]:
        env = config_mod.child_env()
        for key in ACTIONS_FILES:
            env.pop(key, None)
        if self.login is not None:
            env.update(self.login.env)
        return env

    def _paths(self, request: RunRequest) -> tuple[Path, Path]:
        transcript = request.transcript or (request.cwd / f".{self.cli}-{request.role}.jsonl")
        transcript.parent.mkdir(parents=True, exist_ok=True)
        return transcript, transcript.with_suffix(".raw")

    def _launch(self, argv: list[str], request: RunRequest, env: dict[str, str],
                stdin_text: str | None, raw: Path) -> _Launch | RunResult:
        started = time.monotonic()
        marker = f"{request.role}-{os.getpid()}-{time.time_ns()}"
        env = {**env, CALL_MARKER: marker}
        timed_out = False
        before = _own_pids()
        session: list[int] = []
        stderr_path = raw.with_suffix(".stderr")
        try:
            with open(raw, "w", encoding="utf-8") as out, open(stderr_path, "w",
                                                                encoding="utf-8") as err:
                proc = self._popen(
                    argv, cwd=str(request.cwd), env=env,
                    stdin=subprocess.PIPE if stdin_text is not None else subprocess.DEVNULL,
                    stdout=out, stderr=err, text=True, start_new_session=True,
                )
                session.append(proc.pid)
                try:
                    proc.communicate(stdin_text, timeout=request.timeout_s)
                except subprocess.TimeoutExpired:
                    timed_out = True
                    _kill(proc)
                code = proc.returncode if proc.returncode is not None else EXIT_TIMEOUT
        except FileNotFoundError:
            return RunResult(False, "", EXIT_NOT_FOUND, error=f"{self.binary} not found")
        finally:
            _reap(before, marker, session[0] if session else None)
        return _Launch(code, timed_out, raw, _read(stderr_path), time.monotonic() - started)

    def _keep(self, launch: _Launch, transcript: Path) -> None:
        """The raw output, redacted, becomes the transcript; the raw file goes."""
        _write_redacted(launch.raw, transcript)
        launch.raw.unlink(missing_ok=True)

    def _prompt(self, request: RunRequest) -> str:
        """The system text, a note on the CLI, then the task: for a CLI with no flag for an
        appended system prompt."""
        return (f"{request.system_append.rstrip()}\n\n{CLI_NOTE.format(cli=self.cli)}\n\n---\n\n"
                f"{request.prompt}")

    def trail(self, transcript: Path) -> str:
        """The end of a session, as a short list a later agent can read."""
        lines = _read(transcript).splitlines()
        return _trail_text([_clip(line) for line in lines if line.strip()])


class ClaudeCli(_Cli):
    """Runs one model call through the `claude` binary."""

    cli = "claude"

    def __init__(self, claude_bin: str = "claude", login: Any = None,
                 popen: Callable[..., subprocess.Popen] = subprocess.Popen) -> None:
        super().__init__(claude_bin, login, popen)

    def env(self) -> dict[str, str]:
        if self.login is None:
            # Run by hand or in a test: the token comes from this process's own environment.
            env = config_mod.child_env(keep=("CLAUDE_CODE_OAUTH_TOKEN",))
            for key in ACTIONS_FILES:
                env.pop(key, None)
        else:
            env = super().env()
        for key in ("ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_PROFILE"):
            env.pop(key, None)
        env.update(QUIET_ENV)
        return env

    def argv(self, request: RunRequest, system_file: Path) -> list[str]:
        argv = [
            self.binary,
            "--print",
            "--output-format", "stream-json",
            "--verbose",
            "--model", request.model,
            "--effort", request.effort,
            "--max-turns", str(request.max_turns),
            "--permission-mode", "acceptEdits",
            "--allowed-tools", ",".join(request.allowed_tools),
            "--append-system-prompt-file", str(system_file),
            "--strict-mcp-config",
        ]
        if request.disallowed_tools:
            argv += ["--disallowed-tools", ",".join(request.disallowed_tools)]
        return argv

    def run(self, request: RunRequest) -> RunResult:
        transcript, raw = self._paths(request)
        system_file = transcript.with_suffix(".system.md")
        system_file.write_text(request.system_append, encoding="utf-8")
        launch = self._launch(self.argv(request, system_file), request, self.env(),
                              request.prompt, raw)
        if isinstance(launch, RunResult):
            return launch
        with open(raw, encoding="utf-8", errors="replace") as handle:
            result, usage = parse_stream(handle)
        self._keep(launch, transcript)
        if launch.timed_out:
            return RunResult(False, _text(result), EXIT_TIMEOUT, _turns(result), launch.elapsed,
                             f"timed out after {request.timeout_s}s", usage, timed_out=True)
        code = launch.code
        is_error = bool(result.get("is_error")) if isinstance(result, dict) else True
        text = _text(result)
        error = None
        if is_error or code != 0:
            error = redact((text or launch.stderr or f"claude exited {code}")[-2000:])
        reset_at = None
        rejected = isinstance(usage, dict) and usage.get("status") == "rejected"
        if (is_error or code != 0) and (rejected or RATE_LIMIT_WORDS.search(error or "")):
            reset_at = (usage or {}).get("rejected_resets_at") or _exhausted_reset(usage) or DEFAULT_PARK
        return RunResult(ok=not is_error and code == 0, text=text, exit_code=code,
                         turns=_turns(result), duration_s=launch.elapsed, error=error,
                         usage=usage, reset_at=reset_at)

    def trail(self, transcript: Path) -> str:
        entries: list[str] = []
        for event in _json_lines(_read(transcript).splitlines()):
            message = event.get("message") if isinstance(event.get("message"), dict) else {}
            for block in message.get("content") or [] if isinstance(message, dict) else []:
                if not isinstance(block, dict):
                    continue
                if block.get("type") == "text" and event.get("type") == "assistant":
                    entries.append(f"said: {_clip(block.get('text'))}")
                elif block.get("type") == "tool_use":
                    entries.append(f"{block.get('name')}: {_clip(json.dumps(block.get('input')), 200)}")
                elif block.get("type") == "tool_result" and block.get("is_error"):
                    entries.append(f"tool error: {_clip(block.get('content'), 200)}")
        return _trail_text(entries)


class CodexCli(_Cli):
    """OpenAI's Codex CLI (`codex exec`), signed in with ChatGPT."""

    cli = "codex"

    def argv(self, request: RunRequest, last_message: Path) -> list[str]:
        argv = [self.binary, "exec", "--json", "--skip-git-repo-check", "--cd", str(request.cwd),
                "--model", request.model, "--sandbox", "workspace-write",
                "-c", "sandbox_workspace_write.network_access=false",
                "--output-last-message", str(last_message)]
        if request.effort:
            argv += ["-c", f'model_reasoning_effort="{request.effort}"']
        for directory in request.extra_dirs:
            argv += ["--add-dir", directory]
        return argv + ["-"]

    def run(self, request: RunRequest) -> RunResult:
        transcript, raw = self._paths(request)
        last = transcript.with_suffix(".last.md")
        since = time.time() - 5
        launch = self._launch(self.argv(request, last), request, self.env(),
                              self._prompt(request), raw)
        if isinstance(launch, RunResult):
            return launch
        failed: list[str] = []
        messages: list[str] = []
        for event in _json_lines(_read(raw).splitlines()):
            kind = event.get("type")
            if kind in ("turn.failed", "error"):
                error = event.get("error") if isinstance(event.get("error"), dict) else event
                failed.append(str(error.get("message") or ""))
            elif kind == "item.completed":
                item = event.get("item") or {}
                if item.get("type") == "agent_message":
                    messages.append(str(item.get("text") or ""))
        self._keep(launch, transcript)
        text = _read(last).strip() or (messages[-1] if messages else "")
        usage = codex_usage(Path(self.env().get("CODEX_HOME") or Path.home() / ".codex"), since)
        if launch.timed_out:
            return RunResult(False, text, EXIT_TIMEOUT, None, launch.elapsed,
                             f"timed out after {request.timeout_s}s", usage, timed_out=True)
        ok = launch.code == 0 and not failed
        error = None if ok else redact(("; ".join(failed) or launch.stderr
                                        or f"codex exited {launch.code}")[-2000:])
        reset_at = None
        if not ok and RATE_LIMIT_WORDS.search(error or ""):
            reset_at = _exhausted_reset(usage) or DEFAULT_PARK
        return RunResult(ok, text, launch.code, None, launch.elapsed, error, usage, reset_at)

    def trail(self, transcript: Path) -> str:
        entries: list[str] = []
        for event in _json_lines(_read(transcript).splitlines()):
            if event.get("type") != "item.completed":
                continue
            item = event.get("item") or {}
            kind = item.get("type")
            if kind == "agent_message":
                entries.append(f"said: {_clip(item.get('text'))}")
            elif kind == "command_execution":
                entries.append(f"ran `{_clip(item.get('command'), 160)}` (exit "
                               f"{item.get('exit_code')})")
            elif kind == "file_change":
                paths = [c.get("path") for c in item.get("changes") or [] if isinstance(c, dict)]
                entries.append(f"changed {', '.join(str(p) for p in paths)}")
        return _trail_text(entries)


def codex_usage(codex_home: Path, since: float) -> dict[str, Any] | None:
    """The subscription's 5-hour and weekly use, from the newest `token_count` event in the
    session logs Codex wrote since `since` (the `exec` stream itself carries none)."""
    sessions = Path(codex_home) / "sessions"
    newest: dict[str, Any] | None = None
    if not sessions.is_dir():
        return None
    for path in sorted(sessions.rglob("rollout-*.jsonl")):
        try:
            if path.stat().st_mtime < since:
                continue
        except OSError:
            continue
        for event in _json_lines(_read(path).splitlines()):
            payload = event.get("payload") if isinstance(event.get("payload"), dict) else {}
            if payload.get("type") == "token_count" and isinstance(payload.get("rate_limits"), dict):
                newest = payload["rate_limits"]
    if not newest:
        return None
    usage: dict[str, Any] = {"status": "allowed"}
    for key in ("primary", "secondary"):
        window = newest.get(key)
        if not isinstance(window, dict) or not isinstance(window.get("used_percent"), (int, float)):
            continue
        minutes = window.get("window_minutes")
        name = "five_hour" if isinstance(minutes, (int, float)) and minutes <= 360 else "seven_day"
        if key == "primary" and minutes is None:
            name = "five_hour"
        usage[name] = {"utilization": float(window["used_percent"]) / 100.0,
                       "resets_at": _epoch_iso(window.get("resets_at"))}
    return usage if len(usage) > 1 else None


GEMINI_DENY_ALWAYS = ("web_fetch", "google_web_search")
GEMINI_DENY_READER = ("write_file", "replace")


class GeminiCli(_Cli):
    """Google's Gemini CLI, signed in with Google (OAuth)."""

    cli = "gemini"

    def argv(self, request: RunRequest, policy: Path) -> list[str]:
        return [self.binary, "--output-format", "stream-json", "--model", request.model,
                "--approval-mode", "yolo", "--skip-trust", "--policy", str(policy)]

    def _settings(self, request: RunRequest) -> None:
        home = self.env().get("GEMINI_CLI_HOME")
        if not home:
            return
        from harness.logins import GEMINI_SETTINGS
        settings = json.loads(json.dumps(GEMINI_SETTINGS))
        settings["model"] = {"maxSessionTurns": int(request.max_turns)}
        path = Path(home) / ".gemini" / "settings.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(settings, indent=2) + "\n", encoding="utf-8")

    def run(self, request: RunRequest) -> RunResult:
        transcript, raw = self._paths(request)
        policy = transcript.with_suffix(".policy.toml")
        denied = GEMINI_DENY_ALWAYS + (GEMINI_DENY_READER if request.read_only else ())
        policy.write_text("".join(f'[[rule]]\ntoolName = "{tool}"\ndecision = "deny"\n'
                                  f"priority = 100\n\n" for tool in denied), encoding="utf-8")
        self._settings(request)
        launch = self._launch(self.argv(request, policy), request, self.env(),
                              self._prompt(request), raw)
        if isinstance(launch, RunResult):
            return launch
        text_parts: list[str] = []
        result: dict[str, Any] = {}
        errors: list[str] = []
        for event in _json_lines(_read(raw).splitlines()):
            kind = event.get("type")
            if kind == "message" and event.get("role") in ("assistant", "model"):
                text_parts.append(str(event.get("content") or ""))
            elif kind in ("tool_use", "tool_result"):
                text_parts = []  # the final answer is what comes after the last tool
            elif kind == "error":
                errors.append(str(event.get("message") or ""))
            elif kind == "result":
                result = event
        self._keep(launch, transcript)
        text = "".join(text_parts).strip()
        if launch.timed_out:
            return RunResult(False, text, EXIT_TIMEOUT, None, launch.elapsed,
                             f"timed out after {request.timeout_s}s", None, timed_out=True)
        status = str(result.get("status") or "")
        ok = launch.code == 0 and status not in ("error", "failure", "failed")
        error = None
        if not ok:
            detail = result.get("error")
            if isinstance(detail, dict):
                detail = detail.get("message")
            error = redact(("; ".join(e for e in [str(detail or ""), *errors] if e)
                            or launch.stderr or f"gemini exited {launch.code}")[-2000:])
        reset_at = None
        if not ok and RATE_LIMIT_WORDS.search(error or ""):
            # The daily quota resets at midnight Pacific; a per-minute throttle in a minute or so.
            daily = re.search(r"(?i)(daily|per day|terminalquotaerror)", error or "")
            reset_at = _next_midnight(GEMINI_QUOTA_ZONE) if daily else DEFAULT_PARK
        # 41: no usable login; 55: the workspace is not trusted. Neither is the item's fault.
        return RunResult(ok, text, launch.code, None, launch.elapsed, error, None, reset_at,
                         infra_hint=launch.code in (41, 55))

    def trail(self, transcript: Path) -> str:
        entries: list[str] = []
        said: list[str] = []
        for event in _json_lines(_read(transcript).splitlines()):
            kind = event.get("type")
            if kind == "message" and event.get("role") in ("assistant", "model"):
                said.append(str(event.get("content") or ""))
                continue
            if said:
                entries.append(f"said: {_clip(''.join(said))}")
                said = []
            if kind == "tool_use":
                entries.append(f"{event.get('tool_name')}: "
                               f"{_clip(json.dumps(event.get('parameters')), 200)}")
            elif kind == "tool_result" and event.get("status") != "success":
                entries.append(f"tool {event.get('status')}: {_clip(event.get('error') or event.get('output'), 200)}")
        if said:
            entries.append(f"said: {_clip(''.join(said))}")
        return _trail_text(entries)


class MuseCli(_Cli):
    """Meta's Muse Code CLI (`muse exec`)."""

    cli = "muse"

    def argv(self, request: RunRequest, prompt_file: Path) -> list[str]:
        argv = [self.binary, "exec", "--yolo", "--disable-web-tools",
                "--workspace", str(request.cwd), "--model", request.model,
                "--prompt-file", str(prompt_file), "--max-model-steps", str(request.max_turns)]
        if request.effort:
            argv += ["--reasoning-effort", request.effort]
        return argv

    def run(self, request: RunRequest) -> RunResult:
        transcript, raw = self._paths(request)
        prompt_file = transcript.with_suffix(".prompt.md")
        prompt_file.write_text(self._prompt(request), encoding="utf-8")
        launch = self._launch(self.argv(request, prompt_file), request, self.env(), None, raw)
        if isinstance(launch, RunResult):
            return launch
        text = _read(raw).strip()
        stderr = launch.stderr
        transcript_text = text + ("\n\n--- stderr ---\n" + stderr if stderr.strip() else "")
        raw.write_text(transcript_text, encoding="utf-8")
        self._keep(launch, transcript)
        if launch.timed_out:
            return RunResult(False, text, EXIT_TIMEOUT, None, launch.elapsed,
                             f"timed out after {request.timeout_s}s", None, timed_out=True)
        ok = launch.code == 0
        error = None
        if not ok:
            last = [line for line in stderr.splitlines() if line.strip()]
            error = redact((last[-1] if last else f"muse exited {launch.code}")[-2000:])
        reset_at = DEFAULT_PARK if not ok and RATE_LIMIT_WORDS.search(error or "") else None
        return RunResult(ok, text, launch.code, None, launch.elapsed, error, None, reset_at,
                         infra_hint=launch.code == 2)


def _next_midnight(zone_name: str) -> str:
    local = datetime.now(timezone.utc).astimezone(clock.zone(zone_name))
    midnight = (local + timedelta(days=1)).replace(hour=0, minute=5, second=0, microsecond=0)
    return midnight.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


BACKENDS: dict[str, type[_Cli]] = {"claude": ClaudeCli, "codex": CodexCli, "gemini": GeminiCli,
                                   "muse": MuseCli}


PING_PROMPT = "Reply with the word ok."
PING_TIMEOUT_S = 180


def ping_usage(claude_bin: str, model: str, run: Callable[..., Any] = subprocess.run) -> dict | None:
    """The Claude subscription's usage now, read off the smallest call there is: one turn of
    `model` in an empty directory. None when the CLI gave no reading."""
    import tempfile

    argv = [claude_bin, "--print", "--output-format", "stream-json", "--verbose",
            "--model", model, "--max-turns", "1", "--strict-mcp-config"]
    env = config_mod.child_env(keep=("CLAUDE_CODE_OAUTH_TOKEN",))
    env.pop("ANTHROPIC_API_KEY", None)
    for key in ACTIONS_FILES:
        env.pop(key, None)
    env.update(QUIET_ENV)
    with tempfile.TemporaryDirectory(prefix="bot-ping-") as empty:
        env["CLAUDE_CONFIG_DIR"] = str(Path(empty) / ".claude")
        try:
            proc = run(argv, cwd=empty, env=env, input=PING_PROMPT, capture_output=True,
                       text=True, encoding="utf-8", errors="replace", timeout=PING_TIMEOUT_S)
        except (OSError, subprocess.TimeoutExpired):
            return None
    _, usage = parse_stream((proc.stdout or "").splitlines())
    return usage


def _exhausted_reset(usage: dict[str, Any] | None) -> str | None:
    if not isinstance(usage, dict):
        return None
    resets = [
        w.get("resets_at") for w in usage.values()
        if isinstance(w, dict) and float(w.get("utilization", 0)) >= 1.0 and w.get("resets_at")
    ]
    return min(resets) if resets else None


def _own_pids() -> set[int] | None:
    """Every process of this user, or None where /proc cannot say (then nothing is reaped)."""
    proc = Path("/proc")
    if not proc.is_dir():
        return None
    uid = os.getuid()
    found = set()
    for entry in proc.iterdir():
        if entry.name.isdigit():
            try:
                if entry.stat().st_uid == uid:
                    found.add(int(entry.name))
            except OSError:
                continue
    return found


def _ancestors() -> set[int]:
    found, pid = set(), os.getpid()
    while pid > 1:
        found.add(pid)
        try:
            stat = Path(f"/proc/{pid}/stat").read_text()
            pid = int(stat.rsplit(")", 1)[1].split()[1])
        except (OSError, ValueError, IndexError):
            break
    return found


def _belongs(pid: int, marker: str, session: int | None) -> bool:
    """True when `pid` came from the call: it carries the call's marker, or its session."""
    try:
        if f"{CALL_MARKER}={marker}".encode() in Path(f"/proc/{pid}/environ").read_bytes():
            return True
        fields = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
        return session is not None and int(fields[3]) == session
    except (OSError, ValueError, IndexError):
        return False


def _reap(before: set[int] | None, marker: str, session: int | None) -> None:
    """Kill what the call left behind (daemons, `nohup`, `setsid`), so nothing the model started
    can touch the worktree after its session ends. Only processes that carry the call's marker or
    share its session are touched, never another program of the same user."""
    if before is None:
        return
    after = _own_pids() or set()
    for pid in sorted(after - before - _ancestors()):
        if not _belongs(pid, marker, session):
            continue
        try:
            os.kill(pid, signal.SIGKILL)
        except (ProcessLookupError, PermissionError, OSError):
            pass


def _kill(proc: subprocess.Popen) -> None:
    try:
        os.killpg(proc.pid, signal.SIGTERM)
        proc.wait(timeout=20)
    except (ProcessLookupError, PermissionError, OSError, subprocess.TimeoutExpired):
        try:
            os.killpg(proc.pid, signal.SIGKILL)
        except (ProcessLookupError, PermissionError, OSError):
            proc.kill()
        proc.wait()


def _text(result: dict[str, Any] | None) -> str:
    if isinstance(result, dict) and isinstance(result.get("result"), str):
        return result["result"]
    return ""


def _turns(result: dict[str, Any] | None) -> int | None:
    if isinstance(result, dict) and isinstance(result.get("num_turns"), int):
        return result["num_turns"]
    return None


def _write_redacted(source: Path, target: Path) -> None:
    with open(source, encoding="utf-8", errors="replace") as src, open(
        target, "w", encoding="utf-8"
    ) as dst:
        for line in src:
            dst.write(redact(line))


class FakeRunner:
    """Replays scripted calls. Each handler receives the request and returns a RunResult."""

    name = "fake"
    cli = "fake"

    def __init__(self, handlers: Mapping[str, Any] | None = None) -> None:
        self.handlers: dict[str, list[Callable[[RunRequest], RunResult]]] = {}
        self.calls: list[RunRequest] = []
        for role, handler in (handlers or {}).items():
            self.handlers[role] = list(handler) if isinstance(handler, (list, tuple)) else [handler]

    def run(self, request: RunRequest) -> RunResult:
        self.calls.append(request)
        queue = self.handlers.get(request.role) or []
        if not queue:
            return RunResult(False, "", 1, error=f"fake runner has no handler for {request.role}")
        handler = queue.pop(0) if len(queue) > 1 else queue[0]
        return handler(request)

    def trail(self, transcript: Path) -> str:
        return ""


def get_runner(cfg: config_mod.Config, provider: Any = None, login: Any = None) -> Any:
    """The backend for `provider` (the first Claude account when there is none), signed in with
    `login`."""
    if cfg.backend == "fake":
        return FakeRunner()
    cli = provider.cli if provider is not None else "claude"
    return BACKENDS[cli](cfg.bin(cli), login)
