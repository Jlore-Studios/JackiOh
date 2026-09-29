"""Model calls: the `claude` CLI backend and a scripted fake for tests.

The prompt travels on stdin. Output is `stream-json`, written straight to a transcript file so a
long session never sits in memory, then read back for the result line and the subscription usage
(`rate_limit_event`). Every secret except the Claude token is stripped from the child's
environment.
"""

from __future__ import annotations

import json
import os
import re
import signal
import subprocess
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable, Mapping

from harness import config as config_mod
from harness.redact import redact

RATE_LIMIT_WORDS = re.compile(
    r"(?i)(usage limit|rate limit|too many requests|limit reached|hit your (?:\w+ )?limit)"
)
USAGE_WINDOWS = ("five_hour", "seven_day")
#: No telemetry, error reporting or update checks: the session talks to the API and nothing else.
QUIET_ENV = {"CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1", "DISABLE_AUTOUPDATER": "1"}
EXIT_TIMEOUT = 124
EXIT_NOT_FOUND = 127


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


def parse_stream(lines: Any) -> tuple[dict[str, Any] | None, dict[str, Any] | None]:
    """`(result line, last usage reading)` from stream-json lines."""
    result: dict[str, Any] | None = None
    usage: dict[str, Any] | None = None
    for line in lines:
        text = line.strip() if isinstance(line, str) else ""
        if not text.startswith("{"):
            continue
        try:
            event = json.loads(text)
        except ValueError:
            continue
        if not isinstance(event, dict):
            continue
        if event.get("type") == "result":
            result = event
        elif event.get("type") == "rate_limit_event":
            found = usage_from_event(event)
            if found is not None:
                usage = found
    return result, usage


class ClaudeCli:
    """Runs one model call through the `claude` binary."""

    def __init__(self, claude_bin: str = "claude",
                 popen: Callable[..., subprocess.Popen] = subprocess.Popen) -> None:
        self.claude_bin = claude_bin
        self._popen = popen

    def argv(self, request: RunRequest, system_file: Path) -> list[str]:
        argv = [
            self.claude_bin,
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
        started = time.monotonic()
        transcript = request.transcript or (request.cwd / f".claude-{request.role}.jsonl")
        transcript.parent.mkdir(parents=True, exist_ok=True)
        raw = transcript.with_suffix(".raw")
        system_file = transcript.with_suffix(".system.md")
        system_file.write_text(request.system_append, encoding="utf-8")
        env = config_mod.child_env(keep=("CLAUDE_CODE_OAUTH_TOKEN",))
        env.pop("ANTHROPIC_API_KEY", None)
        env.update(QUIET_ENV)
        timed_out = False
        try:
            with open(raw, "w", encoding="utf-8") as out, open(
                transcript.with_suffix(".stderr"), "w", encoding="utf-8"
            ) as err:
                proc = self._popen(
                    self.argv(request, system_file),
                    cwd=str(request.cwd),
                    env=env,
                    stdin=subprocess.PIPE,
                    stdout=out,
                    stderr=err,
                    text=True,
                    start_new_session=True,
                )
                try:
                    proc.communicate(request.prompt, timeout=request.timeout_s)
                except subprocess.TimeoutExpired:
                    timed_out = True
                    _kill(proc)
                code = proc.returncode if proc.returncode is not None else EXIT_TIMEOUT
        except FileNotFoundError:
            return RunResult(False, "", EXIT_NOT_FOUND, error=f"{self.claude_bin} not found")
        stderr = transcript.with_suffix(".stderr").read_text(encoding="utf-8", errors="replace")
        with open(raw, encoding="utf-8", errors="replace") as handle:
            result, usage = parse_stream(handle)
        _write_redacted(raw, transcript)
        raw.unlink(missing_ok=True)
        elapsed = time.monotonic() - started
        if timed_out:
            return RunResult(False, _text(result), EXIT_TIMEOUT, _turns(result), elapsed,
                             f"timed out after {request.timeout_s}s", usage, timed_out=True)
        is_error = bool(result.get("is_error")) if isinstance(result, dict) else True
        text = _text(result)
        error = None
        if is_error or code != 0:
            error = redact((text or stderr or f"claude exited {code}")[-2000:])
        reset_at = None
        rejected = isinstance(usage, dict) and usage.get("status") == "rejected"
        if (is_error or code != 0) and (rejected or RATE_LIMIT_WORDS.search(error or "")):
            reset_at = (usage or {}).get("rejected_resets_at") or _exhausted_reset(usage) or "+PT60M"
        return RunResult(
            ok=not is_error and code == 0,
            text=text,
            exit_code=code,
            turns=_turns(result),
            duration_s=elapsed,
            error=error,
            usage=usage,
            reset_at=reset_at,
        )


def _exhausted_reset(usage: dict[str, Any] | None) -> str | None:
    if not isinstance(usage, dict):
        return None
    resets = [
        w.get("resets_at") for w in usage.values()
        if isinstance(w, dict) and float(w.get("utilization", 0)) >= 1.0 and w.get("resets_at")
    ]
    return min(resets) if resets else None


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


def get_runner(cfg: config_mod.Config) -> Any:
    if cfg.backend == "fake":
        return FakeRunner()
    return ClaudeCli(cfg.claude_bin)
