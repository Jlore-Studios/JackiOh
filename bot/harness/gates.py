"""The repository's own checks, run in a worktree after each build pass.

The list is `gates` in `.harness/config.json`. It is never widened by the bot: a red gate the
bot cannot fix is reported, not skipped. A gate that is also red on the untouched base is marked
pre-existing, so the change is not blamed for it.
"""

from __future__ import annotations

import subprocess
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Callable, Mapping

from harness.config import Gate
from harness.redact import redact

TAIL_CHARS = 6000


@dataclass
class GateResult:
    name: str
    run: str
    ok: bool
    exit_code: int
    seconds: float
    tail: str
    pre_existing: bool = False
    skipped: str = ""

    def to_dict(self) -> dict:
        return asdict(self)


def run_gate(gate: Gate, cwd: Path, env: Mapping[str, str], timeout_s: int | None = None) -> GateResult:
    started = time.monotonic()
    limit = min(gate.timeout_minutes * 60, timeout_s) if timeout_s else gate.timeout_minutes * 60
    try:
        proc = subprocess.run(
            ["bash", "-c", gate.run],
            cwd=str(cwd),
            env=dict(env),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=max(30, int(limit)),
        )
        output = (proc.stdout or "") + ("\n" + proc.stderr if proc.stderr else "")
        code = proc.returncode
    except subprocess.TimeoutExpired as exc:
        output = _as_text(exc.stdout) + "\n" + _as_text(exc.stderr) + f"\n(timed out after {limit}s)"
        code = 124
    return GateResult(
        name=gate.name,
        run=gate.run,
        ok=code == 0,
        exit_code=code,
        seconds=round(time.monotonic() - started, 1),
        tail=redact(output[-TAIL_CHARS:]),
    )


def run_all(gates: tuple[Gate, ...], cwd: Path, env: Mapping[str, str],
            seconds_left: Callable[[], float] | None = None) -> list[GateResult]:
    """Every gate in order. A gate with no time left is recorded as skipped, never as passed."""
    results: list[GateResult] = []
    for gate in gates:
        left = seconds_left() if seconds_left else None
        if left is not None and left < 60:
            results.append(GateResult(gate.name, gate.run, False, -1, 0.0, "", skipped="no time left"))
            continue
        results.append(run_gate(gate, cwd, env, int(left) if left is not None else None))
    return results


def green(results: list[GateResult]) -> bool:
    """True when every gate passed or was already red on the base."""
    return all(r.ok or r.pre_existing for r in results)


def table(results: list[GateResult]) -> str:
    if not results:
        return "_No gates ran._"
    rows = ["| Gate | Result | Time |", "|---|---|---|"]
    for r in results:
        if r.skipped:
            verdict = f"skipped ({r.skipped})"
        elif r.ok:
            verdict = "pass"
        elif r.pre_existing:
            verdict = f"fail (exit {r.exit_code}), also red on main"
        else:
            verdict = f"**fail** (exit {r.exit_code})"
        rows.append(f"| {r.name} (`{r.run}`) | {verdict} | {r.seconds:.0f}s |")
    return "\n".join(rows)


def failures_text(results: list[GateResult]) -> str:
    """The output of every gate this change turned red, for the next build pass."""
    parts = []
    for r in results:
        if r.ok or r.pre_existing:
            continue
        parts.append(f"### {r.name}: `{r.run}` exited {r.exit_code}\n\n{r.tail or r.skipped}")
    return "\n\n".join(parts)


def _as_text(value: object) -> str:
    if isinstance(value, bytes):
        return value.decode("utf-8", "replace")
    return str(value or "")
