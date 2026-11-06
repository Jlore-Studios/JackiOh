#!/usr/bin/env python3
"""ci.yml's sharded jobs (#3072): the `rust-test` matrix, the web unit and e2e smoke shard lists, and
which tests one shard of a `rust-test` group runs.

    python3 .github/ci/matrix.py matrix          `<output>=<json>` lines for the `changes` job's
                                                 $GITHUB_OUTPUT: rust_test, web_unit_shards and
                                                 e2e_smoke_shards
    python3 .github/ci/matrix.py select <event> <rust_test> <scope>
                                                 `<output>=<json>` lines of what this run starts, from
                                                 the `matrix` command's rust_test and the scope step's
                                                 outputs (both JSON): rust_test, rust_test_due and
                                                 rust_binaries
    python3 .github/ci/matrix.py shard <k> <K>   the test names of shard k of K, one a line, from
                                                 `cargo test … -- --list --format terse` on stdin

The groups are .github/ci/rust-test.toml's, which only people change; the counts are
scripts/ci-shards.toml's, which the night bot may change too, because no count can skip a test: a
name lands in exactly one shard for any K, by a hash of the name alone. Python's standard library
only (tomllib, 3.11 and later), as on GitHub's runners; bot/tests/test_ci_matrix.py tests it.
"""

import fnmatch
import json
import sys
import tomllib
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GROUPS = ".github/ci/rust-test.toml"
COUNTS = "scripts/ci-shards.toml"
# The most jobs one count may make: each costs a runner, and the repository's runners are shared by
# every job of every run.
MAX_SHARDS = 8
# scripts/ci-shards.toml's tables past `rust-test`, each with the one count it holds.
COUNT_TABLES = {"web": "unit", "e2e": "smoke"}


class Bad(Exception):
    """A group or a count the generator refuses; its message says which file and what."""


def load(root, path):
    with open(Path(root) / path, "rb") as file:
        return tomllib.load(file)


def count(value, where):
    if type(value) is not int or not 1 <= value <= MAX_SHARDS:
        raise Bad(f"{COUNTS}: {where} = {value!r}: a count is a whole number from 1 to {MAX_SHARDS}")
    return value


def targets(root, group, pattern):
    """The test targets the group's manifest declares (`[[test]]`) that match `pattern`, in its order."""
    manifest = group.get("manifest")
    if not manifest:
        raise Bad(f"{GROUPS}: {group['group']}: the pattern {pattern} needs the group's `manifest`")
    found = [test["name"] for test in load(root, manifest).get("test", [])
             if fnmatch.fnmatchcase(test["name"], pattern)]
    if not found:
        raise Bad(f"{GROUPS}: {group['group']}: no test target of {manifest} matches {pattern}")
    return found


def jobs(root, group):
    """(name, test) for each job of the group before sharding: one per target its `tests` matches."""
    pattern = group.get("tests")
    if pattern is None:
        return [(group["group"], group["test"])]
    if not pattern.endswith("*") or "*" in pattern[:-1]:
        raise Bad(f"{GROUPS}: {group['group']}: `tests` is a name prefix and one trailing `*`, not {pattern}")
    prefix = pattern[:-1]
    return [(group["group"] + target[len(prefix):].replace("_", " "), group["test"].replace("{test}", target))
            for target in targets(root, group, pattern)]


def binaries(root, group):
    """The group's `binaries`, each pattern in it replaced by the targets it matches."""
    words = []
    for word in group.get("binaries", "").split():
        words += targets(root, group, word) if "*" in word else [word]
    return " ".join(words)


def rust_test(root, groups, counts):
    """The `rust-test` matrix: every group's jobs, each in its shards. The doc tests, the binaries
    check and the Luau check go to the group's first job only."""
    entries = []
    for group in groups:
        total = count(counts.get(group["group"], 1), f'rust-test."{group["group"]}"')
        queue = group.get("queue", False)
        if type(queue) is not bool:
            raise Bad(f"{GROUPS}: {group['group']}: `queue` is true or false, not {queue!r}")
        for index, (name, test) in enumerate(jobs(root, group)):
            for shard in range(1, total + 1):
                first = index == 0 and shard == 1
                entries.append({
                    "group": name if total == 1 else f"{name} {shard}/{total}",
                    "scope": group["scope"],
                    "cache": group["cache"],
                    "test": test,
                    "doc": group.get("doc", "") if first else "",
                    "binaries": binaries(root, group) if first else "",
                    "luau": bool(group.get("luau", False)) and first,
                    "queue": queue,
                    "shard": shard,
                    "shards": total,
                })
    names = [group["group"] for group in groups]
    unknown = sorted(set(counts) - set(names))
    if unknown:
        raise Bad(f"{COUNTS}: [rust-test] names no group of {GROUPS}: {', '.join(unknown)}")
    seen = [entry["group"] for entry in entries]
    twice = sorted({name for name in seen if seen.count(name) > 1})
    if twice:
        raise Bad(f"{GROUPS}: two jobs would have the name {', '.join(twice)}")
    return entries


def outputs(root=ROOT):
    """Every output of the `matrix` command, by name."""
    groups = load(root, GROUPS)["group"]
    counts = load(root, COUNTS)
    unknown = sorted(set(counts) - {"rust-test", *COUNT_TABLES})
    if unknown:
        raise Bad(f"{COUNTS}: no job reads the table {', '.join(unknown)}")
    out = {"rust_test": rust_test(root, groups, counts.get("rust-test", {}))}
    for table, key in COUNT_TABLES.items():
        held = counts.get(table, {})
        extra = sorted(set(held) - {key})
        if extra:
            raise Bad(f"{COUNTS}: [{table}] holds only `{key}`, not {', '.join(extra)}")
        if key not in held:
            raise Bad(f"{COUNTS}: [{table}] has no `{key}`")
        out[f"{table}_{key}_shards"] = list(range(1, count(held[key], f"{table}.{key}") + 1))
    return out


def select(event, rust_test, scope):
    """What this run starts, from the `matrix` command's `rust_test` and the scope step's outputs (its
    `name=value` lines, by name), so a job with nothing to do never takes a runner. A job that only
    skipped its own steps still waited its turn for one, and so did the summary that needed it: in the
    day to 2026-11-06 04:30 UTC a scoped run started 13 to 21 such jobs, which waited up to 79 minutes
    to exit in seconds.

      rust_test      the `rust-test` jobs whose group's line is not exactly `false` (an empty or
                     missing line, a doubt, runs it), less, on a pull request's own run, the groups
                     marked `queue`: those run in the merge queue's build and on the push to main
      rust_test_due  whether that list holds any job: the `rust-test` job runs only then, and the
                     `rust` summary accepts its skip only when this is exactly false
      rust_binaries  the release builds: `jackioh` always (the checks and the classifier run it), and
                     `jackioh-server`, which only e2e smoke boots, wherever e2e smoke runs: not on a
                     pull request's own run, and not on a line `e2e=false`"""
    pull = event == "pull_request"
    jobs = [job for job in rust_test
            if scope.get(f"rust_{job['scope']}") != "false" and not (pull and job.get("queue", False))]
    binaries = ["jackioh"]
    if not pull and scope.get("e2e") != "false":
        binaries.append("jackioh-server")
    return {"rust_test": jobs, "rust_test_due": jobs != [], "rust_binaries": binaries}


def shard(k, total, lines):
    """The names of shard k of `total` among a terse test list's `<name>: test` lines, sorted: a name
    lands in the shard its CRC-32 says, so every name lands in exactly one shard for any total. A doc
    test's name holds spaces (`src/lib.rs - f (line 3)`), which no other test's path does, and cargo
    splits what it hands rustdoc on spaces, so none can be run by name: they are left out here, and
    ci.yml runs a group's doc tests whole in its shard 1."""
    names = set()
    for line in lines:
        line = line.rstrip("\n")
        if line.endswith(": test") and " " not in line[: -len(": test")]:
            names.add(line[: -len(": test")])
    return [name for name in sorted(names) if zlib.crc32(name.encode()) % total == k - 1]


def main(argv):
    if argv[1:2] == ["matrix"] and len(argv) == 2:
        values = outputs()
        for name, value in values.items():
            print(f"{name}={json.dumps(value, separators=(',', ':'))}")
        # What the run will hold, for its log: stdout is $GITHUB_OUTPUT's.
        for job in values["rust_test"]:
            print(f"rust (test {job['group']})", file=sys.stderr)
        print(f"web (unit k/{len(values['web_unit_shards'])}), e2e smoke (k/{len(values['e2e_smoke_shards'])})",
              file=sys.stderr)
        return 0
    if argv[1:2] == ["select"] and len(argv) == 5:
        try:
            rust_test, scope = json.loads(argv[3]), json.loads(argv[4])
        except json.JSONDecodeError as error:
            raise Bad(f"select: the matrix and the scope are JSON: {error}") from error
        if type(rust_test) is not list or type(scope) is not dict:
            raise Bad("select: the matrix is a JSON list and the scope a JSON object")
        values = select(argv[2], rust_test, scope)
        for name, value in values.items():
            print(f"{name}={json.dumps(value, separators=(',', ':'))}")
        for job in values["rust_test"]:
            print(f"starts rust (test {job['group']})", file=sys.stderr)
        print(f"release builds: {' '.join(values['rust_binaries'])}", file=sys.stderr)
        return 0
    if argv[1:2] == ["shard"] and len(argv) == 4 and argv[2].isdigit() and argv[3].isdigit():
        k, total = int(argv[2]), int(argv[3])
        if not 1 <= k <= total <= MAX_SHARDS:
            raise Bad(f"shard {k} of {total}: k runs from 1 to its total, at most {MAX_SHARDS}")
        for name in shard(k, total, sys.stdin):
            print(name)
        return 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv))
    except Bad as error:
        print(f"matrix.py: {error}", file=sys.stderr)
        sys.exit(1)
