"""The bot machine's disk (bot/machine/README.md, Disk): what a model job there reads of it, the
room it makes before it starts, and the one issue that tells people while it is filling up.

Every subscription on the machine shares its one disk, and a full one fails whichever job writes
next: on 2026-10-04 a revision of #206 died on `OSError: [Errno 28] No space left on device`,
with 66 MB free when its job began, and it counted as that item's failure. So a model job there
(`work.Worker`):

- reads the disk as it starts and as it ends, into `result.json`'s `disk`;
- with less than `CLEAN_BELOW` free at its start, runs `bot/machine/clean.sh` first, as the
  machine does after every job and every ten minutes;
- with less than `FLOOR` free after that, does no work: the result is `infra` on the machine's
  account (`infra_scope: "machine"`), so the item stays queued and nothing counts against it;
- and when the disk fills up during the work anyway, it pauses (`interrupted`, `disk`) and keeps
  what it built, as for any pause.

The deliver job keeps the newest reading in the state file (`machine_disk`, `note`), and `alert`
opens one issue for people when a reading is `ALERT_PERCENT` full or has less than `FLOOR` free,
rewrites it as readings come in, and closes it once one is back at `CLEAR_PERCENT` or under. The
status loop (`bot-status.yml`, every ten minutes) runs `alert` again on the kept reading, so an
issue that a deliver job could not open or close is settled within ten minutes.
"""

from __future__ import annotations

import errno
import shutil
import subprocess
from datetime import datetime
from pathlib import Path
from typing import Any, Mapping

from harness.clock import iso, parse_iso
from harness.config import LABEL_HUMAN
from harness.context import Context
from harness.errors import GitHubError

GIB = 1024 ** 3
#: A job that starts with less free than this cleans first (`clean`).
CLEAN_BELOW = 8 * GIB
#: A job that has less free than this after cleaning does no work: one job's worktrees, install
#: and builds take about 1 to 2 GB (its Rust builds keep no incremental cache or debug info,
#: `bot-night.yml`), and every other job going needs room to finish.
FLOOR = 3 * GIB
#: The issue opens at this share of the disk used (or under `FLOOR` free) and closes at
#: `CLEAR_PERCENT` or under, so a disk that hovers near one line does not open and close it.
ALERT_PERCENT = 80
CLEAR_PERCENT = 70
#: The clean-up, in the checkout every job has (setup.sh installs the same file on the machine).
CLEAN_SCRIPT = Path("bot/machine/clean.sh")
CLEAN_SECONDS = 300
#: How a full disk reads in an error from a command (git, a CLI) rather than an `OSError`.
FULL_TEXT = "No space left on device"
TITLE = "Night bot: the machine's disk is filling up"
MARKER = "<!-- jackioh-bot:disk -->"
LABELS = ("night bot", LABEL_HUMAN)
#: The order a job's readings are taken in; the last one there is the newest.
READINGS = ("start", "after_clean", "end")


def reading(path: Path, at: datetime) -> dict[str, Any]:
    """How full the filesystem holding `path` is. `percent` is as `df` counts it: used over what
    an ordinary user can have (the blocks kept for root left out)."""
    usage = shutil.disk_usage(path)
    room = usage.used + usage.free
    return {"total": int(usage.total), "free": int(usage.free),
            "percent": round(100 * usage.used / room) if room else 100, "at": iso(at)}


def is_full(exc: BaseException) -> bool:
    """Whether `exc` is a full disk."""
    return ((isinstance(exc, OSError) and exc.errno == errno.ENOSPC)
            or FULL_TEXT in str(exc))


def too_full(found: Mapping[str, Any]) -> bool:
    return int(found["percent"]) >= ALERT_PERCENT or int(found["free"]) < FLOOR


def recovered(found: Mapping[str, Any]) -> bool:
    return int(found["percent"]) <= CLEAR_PERCENT and int(found["free"]) >= FLOOR


def size(n: Any) -> str:
    """`17.3 GB`."""
    return f"{int(n) / GIB:.1f} GB"


def describe(found: Mapping[str, Any]) -> str:
    """`83% full, 5.1 GB free of 30.0 GB`."""
    return f"{found['percent']}% full, {size(found['free'])} free of {size(found['total'])}"


def clean(root: Path, env: Mapping[str, str]) -> str:
    """Run the machine's clean-up as this job's user, and return the end of what it said. It
    never fails the job: a script that is missing or stuck is only reported."""
    script = Path(root) / CLEAN_SCRIPT
    try:
        proc = subprocess.run(["bash", str(script)], env=dict(env), capture_output=True,
                              text=True, timeout=CLEAN_SECONDS, check=False)
    except (OSError, subprocess.SubprocessError) as exc:
        return f"the clean-up did not run: {exc}"
    return (proc.stdout + proc.stderr).strip()[-2000:]


def _valid(found: Any) -> dict[str, Any] | None:
    """A reading from a model job's result, checked: that job's files are not trusted."""
    if not isinstance(found, Mapping):
        return None
    try:
        total, free, percent = int(found["total"]), int(found["free"]), int(found["percent"])
    except (KeyError, TypeError, ValueError):
        return None
    if total <= 0 or not 0 <= free <= total or not 0 <= percent <= 100:
        return None
    at = parse_iso(str(found.get("at") or ""))
    return {"total": total, "free": free, "percent": percent, "at": iso(at) if at else ""}


def newest(result: Mapping[str, Any]) -> dict[str, Any] | None:
    """The newest valid reading a model job's result carries, if any."""
    readings = result.get("disk") if isinstance(result.get("disk"), Mapping) else {}
    for key in reversed(READINGS):
        found = _valid(readings.get(key))
        if found:
            return found
    return None


def note(state: dict[str, Any], found: Mapping[str, Any], runner: str, run_url: str) -> None:
    """Keep `found` as the machine's newest reading, unless the state holds a newer one."""
    entry = state.get("machine_disk") if isinstance(state.get("machine_disk"), dict) else {}
    kept = parse_iso(str((entry.get("reading") or {}).get("at") or ""))
    at = parse_iso(str(found.get("at") or ""))
    if kept and at and at < kept:
        return
    entry.update(reading=dict(found), runner=runner, run=run_url)
    state["machine_disk"] = entry


def _find_open(ctx: Context, number: Any) -> dict[str, Any] | None:
    """The open disk issue: the one the state names, or one with the title and the marker."""
    if number:
        try:
            issue = ctx.gh.get_issue(int(number))
            if issue.get("state") == "open":
                return issue
        except GitHubError:
            pass
    for issue in ctx.gh.list_issues(labels=LABELS[0]):
        if (issue.get("title") == TITLE and MARKER in str(issue.get("body") or "")
                and "pull_request" not in issue):
            return issue
    return None


def _closed_since(ctx: Context, number: Any, found: Mapping[str, Any]) -> bool:
    """Whether the issue the state names was closed after `found` was read."""
    if not number:
        return False
    try:
        closed = parse_iso(str(ctx.gh.get_issue(int(number)).get("closed_at") or ""))
    except GitHubError:
        return False
    at = parse_iso(str(found.get("at") or ""))
    return bool(closed and at and closed >= at)


def _body(entry: Mapping[str, Any]) -> str:
    found = entry["reading"]
    run = f" ([run]({entry['run']}))" if entry.get("run") else ""
    runner = f"`{entry['runner']}`" if entry.get("runner") else "a job"
    return "\n".join([
        f"The bot's machine is **{describe(found)}**, as {runner} read it at {found['at']}{run}.",
        "",
        "A full disk fails whichever job on it writes next. Every job there cleans up after "
        "itself, the machine cleans every ten minutes (`bot/machine/clean.sh`), and a job that "
        f"starts with under {size(FLOOR)} free does no work, so its item waits, uncounted. What "
        "fills the disk now is something those leave.",
        "",
        "To fix it, a person (the bot cannot reach the machine):",
        "- sees what fills it: `bot/machine/on-machine.sh bot/machine/disk-report.sh`;",
        "- deletes what the report shows, or makes the disk larger in EC2 (Modify volume); the "
        "machine grows into it at its next start, or at once with "
        "`bot/machine/on-machine.sh bot/machine/setup.sh gpt agy muse devin`;",
        "- and, if the clean-up missed something that comes back, adds it to `clean.sh`.",
        "",
        f"The bot rewrites this issue as readings come in and closes it once one is back at "
        f"{CLEAR_PERCENT}% or under with at least {size(FLOOR)} free.",
        "",
        MARKER,
    ])


def alert(ctx: Context) -> str | None:
    """Open, rewrite or close the disk issue for the newest kept reading. Returns what it did."""
    state = ctx.store.load()
    entry = state.get("machine_disk") if isinstance(state.get("machine_disk"), dict) else {}
    found = _valid(entry.get("reading"))
    if not found:
        return None
    entry = {**entry, "reading": found}
    issue = _find_open(ctx, entry.get("issue"))
    number = int(issue["number"]) if issue else None
    if too_full(found):
        if number is None and _closed_since(ctx, entry.get("issue"), found):
            return None  # a person closed it after this reading: wait for a newer one
        if number is None:
            number = int(ctx.gh.create_issue(TITLE, _body(entry), LABELS)["number"])
            note_text = f"opened #{number}: the machine's disk is {describe(found)}"
        elif entry.get("shown") != found["at"]:
            ctx.gh.update_issue(number, body=_body(entry))
            note_text = f"rewrote #{number}: the machine's disk is {describe(found)}"
        else:
            return None
        shown = found["at"]
        ctx.store.update(lambda s: s.setdefault("machine_disk", {}).update(
            issue=number, shown=shown), "machine disk issue")
        return note_text
    if not recovered(found) or (number is None and not entry.get("issue")):
        return None
    if number is not None:
        ctx.gh.create_comment(number, f"The machine's disk is back to {describe(found)} "
                                      f"(read at {found['at']}). Closing this.")
        ctx.gh.update_issue(number, state="closed", state_reason="completed")
    ctx.store.update(lambda s: s.setdefault("machine_disk", {}).update(
        issue=None, shown=None), "machine disk recovered")
    return (f"closed #{number}: the machine's disk is {describe(found)}" if number is not None
            else None)


def line(state: Mapping[str, Any]) -> str | None:
    """The status issue's line about the disk, or None before any machine job reported one."""
    entry = state.get("machine_disk") if isinstance(state.get("machine_disk"), Mapping) else {}
    found = _valid(entry.get("reading"))
    if not found:
        return None
    mark = "🔴" if too_full(found) else "🟢"
    issue = f"; see #{entry['issue']}" if entry.get("issue") else ""
    runner = f" by `{entry['runner']}`" if entry.get("runner") else ""
    return f"{mark} **The machine's disk:** {describe(found)}, read{runner} at {found['at']}{issue}."
