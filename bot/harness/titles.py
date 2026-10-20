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

Nothing fires on a title edit (triage runs on `opened` and `labeled`; `bot-commands` and
`squishy-commands` on `labeled` and `assigned`), and an edit made with the workflow's token starts
no workflow at all.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
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


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="python3 -m harness.titles")
    sub = parser.add_subparsers(dest="command", required=True)
    ship = sub.add_parser("shipped", help="retitle what a push to main shipped")
    ship.add_argument("--before", required=True)
    ship.add_argument("--after", required=True)
    args = parser.parse_args(argv)
    from harness.gh import GitHub

    entries = shipped(_patches_at(args.before), _patches_at(args.after))
    if not entries:
        print("titles: nothing shipped in this push")
        return 0
    gh = GitHub(os.environ["GITHUB_REPOSITORY"], os.environ["GH_TOKEN"])
    for line in retitle_shipped(gh, entries):
        print(f"titles: {line}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
