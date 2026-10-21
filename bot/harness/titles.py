"""Titles after the patch numbers follow the cards (#1360): every thread opens `Patch: <what it
does>`, and a shipped card change `Patch v<version>: <what it does>`.

- `what_it_does(title)` is a title without its prefix, the old ones included (`Patch v0.3.X`,
  `Patch v0.3.Y`, `vX.Y.0`, `Night bot`, `CI`, `Architecture`) and the new, with the `(part n of
  m)` a part carries after its prefix kept apart.
- `new_title(title, version)` is the title in the new form: `Patch: …`, or `Patch v<version>: …`
  for a shipped card change, its part kept.
- `python3 -m harness.titles shipped --before <sha> --after <sha>` (`patches-ship.yml`, once a
  promotion merges, #1370) retitles each patch the push shipped: the pull request whose commit
  added its fragment and the issues that pull request closes become `Patch v<version>: …`, and each
  gets a comment naming the version. A version is read from `patches.json` at the two commits,
  never parsed (R105).

- `python3 -m harness.titles retcon [--write] [--numbers 1,2] [--limit N] [--csv path]` (#1372)
  retitles every issue and pull request, closed ones included, from a reviewed dry run: a shipped
  card change's threads become `Patch v<version>: …` (the version from `patches.json`, linked by its
  `source` and by the issues its fragment's pull request closes), every other thread `Patch: …`.
  Titles a workflow or a bot finds by their text, the `production merge` threads, the status issue
  and the tools' own titles are left alone (`left_alone`). Open issues whose old prefix names their
  kind (`Architecture`, `CI`, `Night bot`: internal; `vX.Y.0`: app) get its `kind:*` label; a
  pull request gets its own from the classifier. Without `--write` it writes nothing, only the CSV.

Nothing fires on a title edit (triage runs on `opened` and `labeled`; `bot-commands` and
`squishy-commands` on `labeled` and `assigned`), and an edit made with the workflow's token starts
no workflow at all.
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from typing import Any

from harness.errors import GitHubError

#: A title's prefix, old or new, and the `(part n of m)` right after it.
_PREFIX = re.compile(
    r"^(?:Patch(?: v\d+\.\d+\.(?:\d+[a-z]?|X|Y)[a-z]?)?|v\d+\.\d+\.0|Night bot|CI|Architecture)"
    r"(?P<part> \(part \d+ of \d+\))?: (?P<rest>.*)$", re.S)
#: "Closes #12", "Fixes #12, #14": the issues a pull request's body says it closes.
_CLOSES = re.compile(r"(?i)\b(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?)\s+#(\d+)")
#: A squash commit's subject ends with its pull request's number; a merge commit says it.
_SQUASH = re.compile(r"\(#(\d+)\)\s*$")
_MERGE = re.compile(r"^Merge pull request #(\d+)\b")

PATCHES_JSON = "crates/cards/patches/patches.json"

#: Threads whose titles a workflow or a bot finds by their text, or that a tool names, which the
#: retcon leaves as they are (#1372): ci-duration.yml's alert, super.yml's, deploy-watch.yml's and
#: patches-ship.yml's refusal, the disk alert (`disk.TITLE`), the weekly hygiene issues
#: (.github/hygiene/branches.py), the status issues (`dashboard.TITLE`, Squishy's), and the
#: tools' own pull requests.
LEFT_ALONE = re.compile(
    r"^(?:CI: a job ran over \d+ minutes$"
    r"|CI: daily super run failed$"
    r"|Render: the live server is not serving main's catalog$"
    r"|patches ship refused to promote the pending patches$"
    r"|Night bot: the machine's disk is filling up$"
    r"|Architecture: weekly hygiene\b"
    r"|(?:Night bot|Squishy) status$"
    r"|patches ship: "
    r"|Promote main to production: "
    r"|Merging to production\b"
    r"|AI gen \d+ "
    r'|Revert ")')
#: The status issue (#148), whatever its title says.
STATUS_ISSUE = 148
#: The label of the production merge's issues and pull requests, which their workflow titles.
PRODUCTION_LABEL = "production merge"
#: The kind an open issue's old prefix names (#1372): its label goes on open threads only, since
#: every `labeled` event starts a `bot-commands` run. A `Patch v…` title names none.
PREFIX_KINDS: tuple[tuple[re.Pattern[str], str], ...] = (
    (re.compile(r"^(?:Architecture|CI|Night bot)(?: \(part \d+ of \d+\))?: "), "kind:internal"),
    (re.compile(r"^v\d+\.\d+\.0(?: \(part \d+ of \d+\))?: "), "kind:app"),
)


def what_it_does(title: str) -> tuple[str, str]:
    """`(part, rest)`: the ` (part n of m)` right after the prefix ("" for none) and the title
    without its prefix. A title with no prefix is all rest."""
    text = " ".join(str(title or "").split())
    match = _PREFIX.match(text)
    if not match:
        return "", text
    return match.group("part") or "", match.group("rest").strip()


def new_title(title: str, version: str = "") -> str:
    """The title in the new form: `Patch: …`, or `Patch v<version>: …` for a shipped card change."""
    part, rest = what_it_does(title)
    named = f" {version}" if version else ""
    return f"Patch{named}{part}: {rest}"


def pull_of(subject: str) -> int:
    """The pull request a commit on main landed, from its subject, or 0."""
    match = _SQUASH.search(subject) or _MERGE.match(subject)
    return int(match.group(1)) if match else 0


def closed_by(body: str) -> list[int]:
    """The issues a pull request's body closes, in order, each once."""
    seen: list[int] = []
    for number in (int(n) for n in _CLOSES.findall(str(body or ""))):
        if number not in seen:
            seen.append(number)
    return seen


def shipped(before: list[dict[str, Any]], after: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """The patches `after` appends to `before`, in ship order. A push that renames or drops a
    version `before` lists (the version retcon, #1371) shipped nothing: its threads are the title
    retcon's (#1372), paced and reviewed, never a comment on each."""
    names = [str(entry.get("version")) for entry in before]
    if [str(entry.get("version")) for entry in after[:len(before)]] != names:
        return []
    return after[len(before):]


def _git(*args: str) -> str:
    return subprocess.run(["git", *args], check=True, capture_output=True, text=True).stdout


def _patches_at(rev: str) -> list[dict[str, Any]]:
    try:
        return json.loads(_git("show", f"{rev}:{PATCHES_JSON}"))
    except (subprocess.CalledProcessError, json.JSONDecodeError):
        return []


def retitle_shipped(gh: Any, entries: list[dict[str, Any]], subject_of=None) -> list[str]:
    """Retitles, for each shipped patch, the pull request that added its fragment and the issues
    it closes, and comments the version on each. What it did, one line per thread."""
    subject_of = subject_of or (lambda sha: _git("log", "-1", "--format=%s", sha).strip())
    done: list[str] = []
    for entry in entries:
        version = str(entry.get("version") or "")
        for commit in entry.get("commits") or []:
            number = pull_of(subject_of(commit))
            if not number:
                continue
            try:
                pull = gh.get_pull(number)
            except GitHubError:
                continue
            threads = [(number, str(pull.get("title") or ""), True)]
            for issue in closed_by(str(pull.get("body") or "")):
                try:
                    threads.append((issue, str(gh.get_issue(issue).get("title") or ""), False))
                except GitHubError:
                    continue
            for thread, title, is_pull in threads:
                retitled = new_title(title, version)
                if retitled != title:
                    if is_pull:
                        gh.update_pull(thread, title=retitled)
                    else:
                        gh.update_issue(thread, title=retitled)
                gh.create_comment(thread, f"Shipped as **{version}** (`patches ship`, #1360): "
                                          f"its title is now `{retitled}`.")
                done.append(f"#{thread}: {retitled}")
    return done


def left_alone(thread: dict[str, Any]) -> bool:
    """Whether the retcon leaves a thread's title as it is (#1372)."""
    labels = {str(label.get("name") if isinstance(label, dict) else label) for label in thread.get("labels") or []}
    title = " ".join(str(thread.get("title") or "").split())
    return (int(thread.get("number") or 0) == STATUS_ISSUE or PRODUCTION_LABEL in labels
            or bool(LEFT_ALONE.match(title)))


def versions_by_thread(patches: list[dict[str, Any]], gh: Any, subject_of) -> dict[int, str]:
    """Each thread of a shipped card change, to the version that shipped it: the issues its
    `source` names, the pull request whose commit added its fragment and the issues that pull
    request closes. A thread several patches name takes the newest."""
    found: dict[int, str] = {}
    for entry in patches:
        version = str(entry.get("version") or "")
        numbers = [int(n) for n in re.findall(r"#(\d+)", str(entry.get("source") or ""))]
        for commit in entry.get("commits") or []:
            try:
                number = pull_of(subject_of(commit))
            except subprocess.CalledProcessError:
                number = 0
            if not number:
                continue
            numbers.append(number)
            try:
                numbers.extend(closed_by(str(gh.get_pull(number).get("body") or "")))
            except GitHubError:
                pass
        for number in numbers:
            found[number] = version
    return found


@dataclass
class Retitle:
    """One row of the retcon's CSV."""

    number: int
    kind: str
    state: str
    old: str
    new: str
    label: str = ""


def plan_retcon(threads: list[dict[str, Any]], versions: dict[int, str]) -> list[Retitle]:
    """Every thread the retcon changes, in number order: its new title, and the kind label an open
    issue gets from its old prefix."""
    rows: list[Retitle] = []
    for thread in sorted(threads, key=lambda t: int(t["number"])):
        if left_alone(thread):
            continue
        number = int(thread["number"])
        old = " ".join(str(thread.get("title") or "").split())
        new = new_title(old, versions.get(number, ""))
        pull = "pull_request" in thread
        state = str(thread.get("state") or "")
        label = ""
        if not pull and state == "open":
            label = next((kind for pattern, kind in PREFIX_KINDS if pattern.match(old)), "")
        held = {str(l.get("name") if isinstance(l, dict) else l) for l in thread.get("labels") or []}
        if label in held:
            label = ""
        if new != old or label:
            rows.append(Retitle(number, "pull" if pull else "issue", state, old, new, label))
    return rows


def write_csv(rows: list[Retitle], path: str) -> None:
    with open(path, "w", encoding="utf-8", newline="") as out:
        writer = csv.writer(out)
        writer.writerow(["number", "kind", "state", "old title", "new title", "label"])
        for row in rows:
            writer.writerow([row.number, row.kind, row.state, row.old, row.new, row.label])


def apply_retcon(gh: Any, rows: list[Retitle], *, pause: float = 0.0, sleep=None) -> int:
    """Writes the rows: each title, then its label. Returns how many threads it changed."""
    import time

    sleep = sleep or time.sleep
    changed = 0
    for row in rows:
        if row.new != row.old:
            gh.update_issue(row.number, title=row.new)
        if row.label:
            gh.add_labels(row.number, [row.label])
        changed += 1
        if pause:
            sleep(pause)
    return changed


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="python3 -m harness.titles")
    sub = parser.add_subparsers(dest="command", required=True)
    ship = sub.add_parser("shipped", help="retitle what a push to main shipped")
    ship.add_argument("--before", required=True)
    ship.add_argument("--after", required=True)
    retcon = sub.add_parser("retcon", help="retitle every issue and pull request (#1372)")
    retcon.add_argument("--write", action="store_true", help="write the titles; without it, only the CSV")
    retcon.add_argument("--numbers", default="", help="only these threads, comma-separated")
    retcon.add_argument("--limit", type=int, default=0, help="at most this many threads")
    retcon.add_argument("--csv", default="title-retcon.csv")
    retcon.add_argument("--pause", type=float, default=2.0, help="seconds between writes")
    args = parser.parse_args(argv)
    from harness.gh import GitHub

    gh = GitHub(os.environ["GITHUB_REPOSITORY"], os.environ["GH_TOKEN"])
    if args.command == "retcon":
        threads = gh.paginate(f"repos/{gh.repo}/issues", {"state": "all"}, limit=100_000)
        patches = json.loads(open(os.path.join(_git("rev-parse", "--show-toplevel").strip(),
                                               PATCHES_JSON), encoding="utf-8").read())
        subject_of = lambda sha: _git("log", "-1", "--format=%s", sha).strip()  # noqa: E731
        rows = plan_retcon(threads, versions_by_thread(patches, gh, subject_of))
        if args.numbers:
            wanted = {int(n) for n in args.numbers.split(",") if n.strip()}
            rows = [row for row in rows if row.number in wanted]
        if args.limit:
            rows = rows[:args.limit]
        write_csv(rows, args.csv)
        print(f"titles: {len(rows)} of {len(threads)} threads change; the CSV is {args.csv}")
        if args.write:
            print(f"titles: wrote {apply_retcon(gh, rows, pause=args.pause)} thread(s)")
        return 0

    entries = shipped(_patches_at(args.before), _patches_at(args.after))
    if not entries:
        print("titles: nothing shipped in this push")
        return 0
    for line in retitle_shipped(gh, entries):
        print(f"titles: {line}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
