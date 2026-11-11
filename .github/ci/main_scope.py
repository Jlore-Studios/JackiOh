#!/usr/bin/env python3
"""The scope of ci.yml's run on a push to `main` (#3564): the pieces the merge-queue build of the same
commit did not run.

    python3 .github/ci/main_scope.py <every> <queue> <changed>

prints the `changes` job's scope, one `<line>=<value>` a line for $GITHUB_OUTPUT, the same lines a
pull request's scope has, from three files:

    every    `sh scripts/ci-scope.sh` with no arguments: every line ci.yml reads, each `true`
    queue    the `ci-scope` artifact of the merge-queue build of this very commit, when that build
             passed: the lines its `changes` job wrote. Missing when there is none (a push that did
             not come through the queue) or it could not be read.
    changed  the files changed since the newest commit of main whose push run ran, one a line.
             Missing when that could not be worked out.

The queue lands a pull request by moving main to the commit it built, so a build that passed tested
this very tree. A piece it ran (`full` and the piece's own line exactly `true`) is `false` here: run
again, it could only flake, and the flakes #3564 counted on main were mostly pieces the queue had just
passed. Every other line stays `true`: main still runs what the queue's scope left out, which is
how a scoping mistake is still caught (on 2026-11-06 the queue skipped `bot selftest` and main's run
of it found the break). A line in ALWAYS stays `true` even when the build ran it: `db_deploy`,
whose `deploy-image` artifact a staging deploy loads from the push-to-main run alone. Then, so the
scope is one a pull request could have: `db` is any of its suites' lines, `web_promote` is `false`
once `web` is (its job needs `web`; super.yml runs both promotion tests daily), `full` is any
line's, and `bot` is `false` only when the build ran it.

Everything runs, every line `true`, when there is no such build, when anything is missing, or when
a file in RUN_ALL changed: the Rust and pnpm caches are saved from main alone
(.github/actions/setup), so a change to what keys them needs main to run every job once, or every
pull request would start cold; and a change to the CI itself gets one full run on main.

Python's standard library only, as on GitHub's runners; bot/tests/test_ci_main_scope.py tests it.
"""

import fnmatch
import re
import sys
from pathlib import Path

#: A changed file that runs everything on main, and why (fnmatch patterns; `*` crosses `/`).
RUN_ALL = (
    ("Cargo.lock", "it keys the Rust cache"),
    ("Cargo.toml", "it keys the Rust cache"),
    ("*/Cargo.toml", "it keys the Rust cache"),
    ("rust-toolchain.toml", "it keys the Rust cache"),
    (".cargo/*", "it keys the Rust cache"),
    ("pnpm-lock.yaml", "it keys the pnpm cache"),
    (".github/*", "the CI itself changed"),
)

#: A line main runs even when the queue's build ran it, and why: what its jobs leave behind is
#: kept only from main's runs, so the build's own run of it cannot stand in.
ALWAYS = (
    ("db_deploy", "the deploy rehearsal's `deploy-image` artifact is what a deploy loads "
                  "(scripts/server-image.sh, step 2), from the push-to-main run alone, and only "
                  "main and staging write the image's layer cache; without them a deploy builds "
                  "the image from a cold cache"),
)

#: The lines worked out from the others, never from the queue's own.
DERIVED = ("full", "db")

LINE = re.compile(r"^([a-z0-9_]+)=([a-z]*)$")


def read(path):
    """A file of `<line>=<value>` rows as {line: value}, or None when it cannot be read."""
    try:
        text = Path(path).read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None
    lines = {}
    for row in text.splitlines():
        match = LINE.match(row.strip())
        if match:
            lines[match.group(1)] = match.group(2)
    return lines


def files(path):
    """A file of paths, one a line, or None when it cannot be read."""
    try:
        text = Path(path).read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None
    return [row.strip() for row in text.splitlines() if row.strip()]


def everything(every, why):
    lines = {name: "true" for name in every}
    lines["bot"] = "true"
    return lines, why


def scope(every, queue, changed):
    """main's lines and why, from `every` ({line: value}, main's own scope: every line), `queue`
    ({line: value} or None) and `changed` ([path] or None)."""
    if queue is None:
        return everything(every, "no merge-queue build of this commit passed and kept its scope, so everything runs")
    if changed is None:
        return everything(every, "what changed since main's last run could not be read, so everything runs")
    for path in changed:
        for pattern, why in RUN_ALL:
            if fnmatch.fnmatchcase(path, pattern):
                return everything(every, f"{path} changed since main's last run ({why}), so everything runs")

    built = queue.get("full") == "true"
    always = {name for name, _ in ALWAYS}
    own = {}
    for name in every:
        if name not in DERIVED:
            ran = built and queue.get(name) == "true" and name not in always
            own[name] = "false" if ran else "true"
    if own.get("web") == "false" and "web_promote" in own:
        own["web_promote"] = "false"
    db = any(value == "true" for name, value in own.items() if name.startswith("db_"))
    full = any(value == "true" for value in own.values())

    lines = {}
    for name in every:
        if name == "full":
            lines[name] = "true" if full else "false"
        elif name == "db":
            lines[name] = "true" if db else "false"
        else:
            lines[name] = own[name]
    lines["bot"] = "false" if queue.get("bot") == "true" else "true"

    ran = [name for name, value in lines.items() if value == "false" and name not in DERIVED]
    left = [name for name, value in lines.items()
            if value == "true" and name not in DERIVED and name not in always]
    why = (f"the merge-queue build of this commit passed; what it ran is not run again "
           f"({' '.join(ran) or 'nothing'}), what it skipped runs here ({' '.join(left) or 'nothing'}), "
           f"and so does what main always runs ({' '.join(sorted(always & set(lines)))})")
    return lines, why


def main(argv):
    if len(argv) != 4:
        print("usage: main_scope.py <every> <queue> <changed>", file=sys.stderr)
        return 2
    every = read(argv[1])
    if not every:
        print(f"main-scope: {argv[1]} holds no lines (scripts/ci-scope.sh printed none)", file=sys.stderr)
        return 1
    lines, why = scope(every, read(argv[2]), files(argv[3]))
    print(f"main-scope: {why}", file=sys.stderr)
    for name, value in lines.items():
        print(f"{name}={value}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
