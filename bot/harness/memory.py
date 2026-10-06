"""The bot machine's memory (#312): whether its jobs run short of it, read by the jobs themselves.

The night box has 2 vCPUs and 7.7 GB of RAM with 8 GB of swap, and CloudWatch keeps only its CPU
unless the agent `bot/machine/setup.sh` installs is allowed to publish (the instance role needs
`CloudWatchAgentServerPolicy`). The box powers off when idle, so `free` shows only the present.
So every model job on the machine (`work.Worker`) reads the kernel's own counters as it starts,
after each model call and check run, and as it ends, into `result.json`'s `memory`:

- the memory pressure stall time (`/proc/pressure/memory`, PSI): how long some task, and every
  task at once (`full`), waited on memory during the job;
- the pages swapped in and out (`/proc/vmstat`);
- the least memory available at any reading (`/proc/meminfo`'s `MemAvailable`), and the most
  swap in use.

The deliver job keeps the last `KEEP` runs' figures in the state file (`machine_memory`), and the
status issue shows the last day of them (`line`): with stalls and swapping near zero the box has
memory to spare and `machine_parallel` is held by its CPU; with seconds of `full` stall a run, it
is short of memory, and fewer machine lanes or a bigger box is the answer
(`bot/machine/README.md`, Memory).
"""

from __future__ import annotations

from datetime import datetime, timedelta
from pathlib import Path
from typing import Any, Mapping

from harness.clock import iso, parse_iso

PROC = Path("/proc")
#: How many machine runs' figures the state file keeps.
KEEP = 60
#: How far back the status issue's line looks.
WINDOW = timedelta(hours=24)
#: A run whose `full` stall reached this many seconds ran short of memory.
SHORT_FULL_S = 5.0
GIB = 1024 ** 3


def _fields(path: Path) -> dict[str, str]:
    """`name value` lines (`/proc/vmstat`) or `name: value` ones (`/proc/meminfo`)."""
    found: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        name, _, rest = line.replace(":", " ", 1).partition(" ")
        found[name] = rest.strip()
    return found


def _pressure(path: Path) -> dict[str, float]:
    """`some` and `full` stall totals, in seconds, from a PSI file."""
    totals: dict[str, float] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        kind, _, rest = line.partition(" ")
        for part in rest.split():
            key, _, value = part.partition("=")
            if key == "total" and value.isdigit():
                totals[kind] = int(value) / 1_000_000
    return totals


def reading(at: datetime, proc: Path = PROC) -> dict[str, Any] | None:
    """The machine's memory now, or None where the kernel does not say (not Linux)."""
    try:
        info = _fields(proc / "meminfo")
        kib = {name: int(str(info.get(name, "0")).split()[0]) * 1024
               for name in ("MemTotal", "MemAvailable", "SwapTotal", "SwapFree")}
    except (OSError, ValueError, IndexError):
        return None
    found: dict[str, Any] = {"total": kib["MemTotal"], "available": kib["MemAvailable"],
                             "swap_used": kib["SwapTotal"] - kib["SwapFree"], "at": iso(at)}
    try:
        stat = _fields(proc / "vmstat")
        found["swapped"] = int(stat.get("pswpin", "0")) + int(stat.get("pswpout", "0"))
    except (OSError, ValueError):
        pass
    try:
        stall = _pressure(proc / "pressure" / "memory")
        found.update(stall_some=stall.get("some", 0.0), stall_full=stall.get("full", 0.0))
    except OSError:
        pass
    return found


def add(record: dict[str, Any], found: Mapping[str, Any] | None) -> None:
    """Fold one reading into a job's `memory` record: the first and the last, the least
    available and the most swap used."""
    if found is None:
        return
    record.setdefault("start", dict(found))
    record["end"] = dict(found)
    record["low_available"] = min(int(record.get("low_available", found["available"])),
                                  int(found["available"]))
    record["high_swap"] = max(int(record.get("high_swap", 0)), int(found["swap_used"]))
    record["readings"] = int(record.get("readings", 0)) + 1


def summary(record: Mapping[str, Any] | None) -> dict[str, Any] | None:
    """A job's memory over its run: stall seconds, pages swapped, the least available."""
    if not isinstance(record, Mapping) or not isinstance(record.get("start"), Mapping):
        return None
    start, end = record["start"], record.get("end") or record["start"]

    def delta(key: str) -> float:
        try:
            return max(0.0, float(end.get(key, 0)) - float(start.get(key, 0)))
        except (TypeError, ValueError):
            return 0.0
    return {"at": str(end.get("at") or ""), "total": int(start.get("total") or 0),
            "low_available": int(record.get("low_available") or 0),
            "high_swap": int(record.get("high_swap") or 0),
            "stall_some": round(delta("stall_some"), 1), "stall_full": round(delta("stall_full"), 1),
            "swapped": int(delta("swapped"))}


def note(state: dict[str, Any], found: Mapping[str, Any], runner: str) -> None:
    """Keep one machine run's summary, the newest `KEEP` of them."""
    kept = [entry for entry in state.get("machine_memory") or [] if isinstance(entry, dict)]
    kept.append({**found, "runner": runner})
    state["machine_memory"] = kept[-KEEP:]


def _gib(value: int) -> str:
    return f"{value / GIB:.1f} GB"


def line(state: Mapping[str, Any], now: datetime) -> str:
    """The status issue's line on the machine's memory over the last day, or ""."""
    recent = []
    for entry in state.get("machine_memory") or []:
        at = parse_iso(str((entry or {}).get("at") or "")) if isinstance(entry, dict) else None
        if at is not None and now - at <= WINDOW:
            recent.append(entry)
    if not recent:
        return ""
    total = max(int(e.get("total") or 0) for e in recent)
    low = min(int(e.get("low_available") or 0) for e in recent)
    short = [e for e in recent if float(e.get("stall_full") or 0) >= SHORT_FULL_S]
    swapped = [e for e in recent if int(e.get("swapped") or 0) > 0]
    worst = max(float(e.get("stall_full") or 0) for e in recent)
    verdict = ("**short of memory**: fewer machine lanes (`machine_parallel`) or a bigger box"
               if short else "memory to spare: its CPU, not its memory, sets `machine_parallel`")
    return (f"**Machine memory, last 24 hours** ({len(recent)} runs): {verdict}. Least available "
            f"{_gib(low)} of {_gib(total)}; {len(short)} run(s) stalled on memory for "
            f"{SHORT_FULL_S:.0f} s or more (longest {worst:.0f} s); {len(swapped)} swapped.")
