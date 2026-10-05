"""Stdio client for the arena bridge (``packages/ai/scripts/arena-bridge.ts``).

One ``Bridge`` owns one long-lived Node process (one per arena run, not per
game) and speaks JSON lines: one line sent, one line read, with a timeout.
A bridge-level failure (the process dies, a reply is not JSON, a command
errors) raises ``BridgeError``: that is an arena crash, never a forfeit.
Only an agent returning an action outside its ``legal`` set forfeits.
"""

from __future__ import annotations

import json
import subprocess
from typing import Any

BRIDGE_TIMEOUT_S = 60


class BridgeError(RuntimeError):
    pass


class Bridge:
    def __init__(self, command: list[str] | None = None) -> None:
        cmd = command or ["pnpm", "--filter", "@jackioh/ai", "arena-bridge"]
        try:
            self._proc = subprocess.Popen(
                cmd,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=1,
            )
        except OSError as exc:
            raise BridgeError(f"cannot start arena bridge {cmd}: {exc}") from exc

    def call(self, cmd: dict[str, Any], timeout: float = BRIDGE_TIMEOUT_S) -> dict[str, Any]:
        proc = self._proc
        if proc.poll() is not None:
            raise BridgeError(f"bridge exited with code {proc.returncode}")
        assert proc.stdin is not None and proc.stdout is not None
        try:
            proc.stdin.write(json.dumps(cmd) + "\n")
            proc.stdin.flush()
        except (BrokenPipeError, OSError) as exc:
            raise BridgeError(f"bridge write failed: {exc}") from exc
        line = self._readline(timeout)
        try:
            reply = json.loads(line)
        except json.JSONDecodeError as exc:
            raise BridgeError(f"bridge replied non-JSON: {line[:200]!r}") from exc
        if not isinstance(reply, dict):
            raise BridgeError(f"bridge replied non-object: {line[:200]!r}")
        if not reply.get("ok", False):
            raise BridgeError(f"bridge error on {cmd.get('cmd')}: {reply.get('error')}")
        return reply

    def _readline(self, timeout: float) -> str:
        import select

        stream = self._proc.stdout
        assert stream is not None
        ready, _, _ = select.select([stream], [], [], timeout)
        if not ready:
            raise BridgeError("bridge read timed out")
        line = stream.readline()
        if line == "":
            raise BridgeError("bridge closed stdout")
        return line

    def close(self) -> None:
        try:
            self._proc.kill()
        except OSError:
            pass
        self._proc.wait()

    def __enter__(self) -> Bridge:
        return self

    def __exit__(self, *args: object) -> None:
        self.close()
