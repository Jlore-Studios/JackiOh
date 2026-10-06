"""An item's journal: every run on it, in order, on the unprotected `bot-journal` branch (#342).

A run that does not finish its item hands the next one its notes and the end of its session
(`work.Worker._handoff`), but that handoff is one slot in the state file: the next run overwrites
it, a finished item drops it, and nobody can read it from the pull request. The journal keeps
them all. Deliver appends one section per run to `<issue>.md`, whatever the run did and however it
ended: what it was, a line per step the harness took (each model call, why it ran and what came of
it; each check run; the merges of `main`), and the builder's notes as they stood at the end. A pull
request's runs go to the file of the issue it closes, so one file holds the build and every
revision and review after it.

The plan job reads the end of the file into the plan (`read`), and the work job puts it in the
worktree as `JOURNAL_FILE`, which git ignores, so the next agent, on any subscription and any
CLI, can read every run before it. The model job needs no GitHub credential for that, which is
how it works on the bot's machine too. The builder's own notes come in two parts (`work.NOTES_ASK`):
a `## State` it rewrites in place, which is the compressed history, and a `## Log` it adds to;
`compact` keeps the first whole and the end of the second when the notes must be cut.

Writes go through the Contents API with the blob sha as a compare-and-set, as the state file's do
(`state.py`). The journal is a record, never a gate: a failed read or write is logged, and the
run's delivery goes on.
"""

from __future__ import annotations

import random
import re
import time
from typing import Any, Callable

from harness.config import NO_DEPLOY
from harness.errors import GitHubError
from harness.redact import redact

BRANCH = "bot-journal"
#: Where the work job puts the item's journal in its worktree: beside the notes, ignored by git.
JOURNAL_FILE = ".bot-journal.md"
#: Each run's section starts with this line, so the file can be cut at a run.
RUN_MARKER = "<!-- jackioh-bot:run -->"
#: How much of the journal the plan carries to the work job: its end, from a run's start.
READ_CHARS = 100_000
#: A file over this drops its oldest runs' notes, then its oldest runs, until it fits (the
#: Contents API reads files up to 1 MB). The branch's history keeps every version.
FILE_CHARS = 300_000
#: How much of a run's notes its section keeps.
NOTES_CHARS = 20_000
#: Steps of a run its section lists, and the characters of one cell.
MAX_STEPS = 200
CELL_CHARS = 300
MAX_ATTEMPTS = 6
#: What `compact` leaves where it cut the log.
CUT = "(earlier lines of the log were cut here)\n"
_LOG = re.compile(r"^##\s+Log\b.*$", re.M)
_DETAILS = re.compile(r"\n<details><summary>The builder's notes.*?</details>\n", re.S)


def path_for(number: int) -> str:
    return f"{int(number)}.md"


def key(plan: dict[str, Any]) -> int:
    """The item a run's journal belongs to: the issue a pull request closes, else the thread."""
    try:
        return int(plan.get("issue_number") or plan.get("number") or 0)
    except (TypeError, ValueError):
        return 0


def compact(notes: str, limit: int) -> str:
    """`notes` cut to `limit` characters, keeping what the next agent needs: everything before
    `## Log` (the plan the harness seeded and the State the builder rewrites) whole, or its first
    half of `limit` when it alone is longer, then as much of the end of the log as fits. Notes
    with no `## Log` heading keep their end, as before."""
    if len(notes) <= limit:
        return notes
    found = _LOG.search(notes)
    if found is None:
        return notes[-limit:]
    state = notes[:found.start()].rstrip() + "\n\n"
    if len(state) > limit // 2:
        state = state[:limit // 2].rstrip() + "\n(the rest of the state was cut here)\n\n"
    heading = found.group(0) + "\n"
    room = max(0, limit - len(state) - len(heading) - len(CUT))
    log = notes[found.end():].lstrip("\n")
    return state + heading + CUT + (log[-room:] if room else "")


def _cell(text: Any) -> str:
    flat = " ".join(str(text if text is not None else "").split()).replace("|", "\\|")
    return flat if len(flat) <= CELL_CHARS else flat[:CELL_CHARS - 1] + "…"


def _clock(at: Any) -> str:
    text = str(at or "")
    return text[11:16] if len(text) >= 16 else text


def step_rows(steps: list[dict[str, Any]]) -> list[str]:
    """One table row per step the work job recorded (`work.Worker._step`)."""
    rows = []
    for n, step in enumerate(steps[:MAX_STEPS], 1):
        if not isinstance(step, dict):
            continue
        what = str(step.get("step") or "?")
        if what == "call":
            model = f" (`{step['model']}`)" if step.get("model") else ""
            what = f"{step.get('role') or 'call'}{model}"
        took = step.get("minutes")
        took = f"{took} min" if isinstance(took, (int, float)) else ""
        rows.append(f"| {n} | {_clock(step.get('at'))} | {_cell(what)} | {_cell(step.get('why'))} "
                    f"| {took} | {_cell(step.get('outcome'))} |")
    if len(steps) > MAX_STEPS:
        rows.append(f"| … | | {len(steps) - MAX_STEPS} more steps | | | |")
    return rows


def section(result: dict[str, Any], *, provider: str, models: str, action: str, number: int,
            at: str, run_url: str = "") -> str:
    """The journal's section for one run, from the model job's result."""
    status = str(result.get("status") or "unknown")
    if result.get("interrupt") == "died":
        status = "died"
    reason = redact(str(result.get("reason") or "")).strip()
    link = f"[run]({run_url})" if run_url else "a run"
    lines = [RUN_MARKER, f"## {str(at)[:16].replace('T', ' ')} UTC · {action} #{number} · "
             f"`{provider}` · {status}", "",
             f"{link} on `{provider}` ({models}). It ended **{status}**"
             + (f": {_cell(reason)}" if reason else ".")]
    minutes, calls = result.get("minutes"), result.get("model_calls")
    if minutes is not None or calls is not None:
        lines.append(f"Model calls: {calls or 0}, {minutes or 0} model minutes.")
    if result.get("head"):
        lines.append(f"Its work ended at `{str(result['head'])[:12]}`"
                     + (f" on `{result['branch']}`." if result.get("branch") else "."))
    steps = result.get("steps") if isinstance(result.get("steps"), list) else []
    if steps:
        lines += ["", "| # | When (UTC) | Step | Why | Took | What came of it |",
                  "|---|---|---|---|---|---|", *step_rows(steps)]
    handoff = result.get("handoff") if isinstance(result.get("handoff"), dict) else {}
    notes = redact(str(handoff.get("notes") or "")).strip()
    if notes:
        notes = compact(notes, NOTES_CHARS)
        lines += ["", "<details><summary>The builder's notes when it stopped</summary>", "",
                  "````markdown", notes.replace("````", "``​``"), "````", "", "</details>"]
    return "\n".join(lines).rstrip() + "\n"


def header(number: int) -> str:
    return (f"# Journal of #{number}\n\n"
            f"Every night-bot run on #{number}, and on the pull requests that close it, adds a "
            "section here when it ends, oldest first (`bot/harness/journal.py`). The next run "
            f"reads the end of this file, as `{JOURNAL_FILE}` in its worktree, before it starts.\n")


def _split(text: str) -> tuple[str, list[str]]:
    parts = text.split("\n" + RUN_MARKER + "\n")
    return parts[0], [RUN_MARKER + "\n" + part for part in parts[1:]]


def trim(text: str, limit: int = FILE_CHARS) -> str:
    """The file cut to `limit`: the oldest runs lose their notes first, then whole runs go."""
    if len(text) <= limit:
        return text
    head, runs = _split(text)
    i = 0
    while len("\n".join([head, *runs])) > limit and i < len(runs):
        runs[i] = _DETAILS.sub("\n", runs[i])
        i += 1
    dropped = 0
    while len("\n".join([head, *runs])) > limit and len(runs) > 1:
        runs.pop(0)
        dropped += 1
    if dropped:
        head = head.rstrip() + (f"\n\n_{dropped} older run(s) dropped from this file; the "
                                "branch's history keeps them._\n")
    return "\n".join([head, *runs])


def append(gh: Any, number: int, text: str, *,
           sleep: Callable[[float], None] = time.sleep) -> None:
    """Add one run's section to the item's file, creating the branch and the file as needed.
    Raises GitHubError when GitHub keeps refusing; the caller logs it."""
    path = path_for(number)
    for _ in range(MAX_ATTEMPTS):
        old, sha = gh.get_file(path, BRANCH)
        new = trim((old.rstrip() + "\n\n" if old else header(number) + "\n") + text)
        if old is None and gh.branch_sha(BRANCH) is None:
            try:
                gh.create_orphan_branch(BRANCH, path, new, f"journal #{number}: start [skip ci]",
                                        extra=NO_DEPLOY)
                return
            except GitHubError as exc:
                if exc.status != 422:  # 422: another run made the branch first
                    raise
                continue
        try:
            gh.put_file(path, new, branch=BRANCH, sha=sha, message=f"journal #{number} [skip ci]")
            return
        except GitHubError as exc:
            if exc.status not in (409, 422):
                raise
            sleep(random.uniform(0.05, 0.3))  # another run wrote in between: read it again
    raise GitHubError(f"{path} on {BRANCH} kept changing; gave up after {MAX_ATTEMPTS} attempts",
                      409)


def read(gh: Any, number: int, limit: int = READ_CHARS) -> str:
    """The end of an item's journal, from the start of a run, or "" (none yet, or unreadable)."""
    try:
        text, _ = gh.get_file(path_for(number), BRANCH)
    except Exception:  # noqa: BLE001 - a journal that cannot be read never stops a claim
        return ""
    if not text:
        return ""
    if len(text) <= limit:
        return text
    tail = text[-limit:]
    at = tail.find(RUN_MARKER)
    tail = tail[at:] if at >= 0 else tail
    return header(number) + "\n_(earlier runs are on the branch, not shown here)_\n\n" + tail


def runs_in(text: str) -> list[str]:
    """One line per run in a journal: its section's heading."""
    found = []
    for part in text.split(RUN_MARKER)[1:]:
        for line in part.splitlines():
            if line.startswith("## "):
                found.append(line[3:].strip())
                break
    return found


def url(server_url: str, repo: str, number: int) -> str:
    return f"{server_url}/{repo}/blob/{BRANCH}/{path_for(number)}"
