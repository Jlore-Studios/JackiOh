"""The conflicts the harness resolves itself when it merges `main`, with no model (`resolvers` in
`.harness/config.json`).

A conflict in a file a tool rewrites, or in one no branch should change, takes no judgment, yet it
cost a model run like any other: in the two weeks to 2026-10-28, 169 of the 328 merges the bots
left conflicted held nothing but `spec/INDEX.md` and `spec/11-rulings.md`, and 27 more nothing but
those and the golden traces (`crates/engine/tests/golden/games.jsonl`). A resolver names one path
and how its conflict is settled:

- `take: "main"`: the version the merge brought in (`git checkout --theirs`), or its deletion. For
  a file a job on `main` rewrites and no branch should touch (`spec/INDEX.md`, the daily spec fold).
- `run: <command>`: a command that rewrites the file from the merged tree (`cargo jackioh golden
  bless`). It runs only once nothing else is conflicted, because it builds that tree, only when
  both sides kept the file, and only with time left for it and for a review after it. A command
  that fails, times out, is cut off, leaves the file as it was or leaves a conflict marker puts
  the file back exactly as git left it, markers and all, so the builder resolves it as before.

Every other file stays as git left it. The run's checks then judge the merged head as they judge
a builder's work, and its reviewer reviews it (`work.Worker._resolve`).
"""

from __future__ import annotations

import re
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Callable, Mapping

from harness import gates as gates_mod
from harness.config import Gate, Resolver
from harness.git import MARKER_LINE, Git

#: A command resolver needs at least this many seconds of the run left to start, beyond what the
#: caller keeps for the calls after it (`reserve_seconds`).
MIN_SECONDS = 120


@dataclass
class Resolution:
    """What the harness did with one conflicted file: `ok` when it is resolved and staged."""

    path: str
    how: str
    ok: bool
    why_not: str = ""

    def to_dict(self) -> dict[str, object]:
        return asdict(self)


def _marked(path: Path) -> bool:
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except FileNotFoundError:
        return False
    return re.search(MARKER_LINE, text, re.M) is not None


def _exists_at(wt: Git, ref: str, path: str) -> bool:
    return wt.run("cat-file", "-e", f"{ref}:{path}", check=False).returncode == 0


def _take_main(wt: Git, path: str, base: str) -> Resolution:
    """`base`'s side of `path` (the branch merged in, `main` but for a fullsend part): its version,
    or its deletion when it deleted the file."""
    if _exists_at(wt, "MERGE_HEAD", path):
        wt.run("checkout", "--theirs", "--", path)
        wt.run("add", "--", path)
        return Resolution(path, f"took `{base}`'s version", True)
    wt.run("rm", "-q", "--", path)
    return Resolution(path, f"took `{base}`'s side: it deleted the file", True)


def _run(wt: Git, resolver: Resolver, env: Mapping[str, str], seconds: float) -> Resolution:
    """Rewrite `resolver.path` with its command from the merged tree. Unless that succeeds, the
    file is put back byte for byte as the merge left it, even when the run is cut off mid-command
    (the job's signal handler raises), so it is the builder's again and never a half-written file
    a work-in-progress commit could take up."""
    path = resolver.path
    how = f"ran `{resolver.run}`"
    if not (_exists_at(wt, "HEAD", path) and _exists_at(wt, "MERGE_HEAD", path)):
        return Resolution(path, how, False, "one side deleted the file, which no command decides")
    target = wt.cwd / path
    original = target.read_bytes() if target.exists() else None
    resolved = False
    try:
        gate = Gate(f"resolve {path}", resolver.run, resolver.timeout_minutes)
        result = gates_mod.run_gate(gate, wt.cwd, env, int(seconds))
        if not result.ok:
            why = f"it exited {result.exit_code}: {result.tail.strip()[-600:]}"
        elif not target.exists():
            why = "it left no file"
        elif target.read_bytes() == original:
            why = "it left the file as the merge left it"
        elif _marked(target):
            why = "it left a conflict marker in the file"
        else:
            wt.run("add", "--", path)
            resolved = True
            return Resolution(path, how, True)
    finally:
        if not resolved:
            if original is None:
                target.unlink(missing_ok=True)
            else:
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(original)
    return Resolution(path, how, False, why)


def resolve(wt: Git, conflicts: list[str], resolvers: tuple[Resolver, ...],
            env: Mapping[str, str], seconds_left: Callable[[], float], *,
            commands: bool = True, reserve_seconds: float = 0.0,
            base: str = "main") -> tuple[list[str], list[Resolution]]:
    """Settle the conflicts of the merge in progress in `wt` that a resolver names. Returns the
    paths still conflicted, in git's order, and what was done (resolved or not, and why not).
    `commands` False (the dependencies did not install) runs no command resolver; one starts only
    with `MIN_SECONDS` left beyond `reserve_seconds`. `base` names the branch merged in."""
    by_path = {r.path: r for r in resolvers}
    left = list(conflicts)
    done: list[Resolution] = []
    for path in conflicts:
        resolver = by_path.get(path)
        if resolver is not None and resolver.take:
            done.append(_take_main(wt, path, base))
            left.remove(path)
    pending = [by_path[p] for p in left if p in by_path and by_path[p].run]
    if not pending:
        return left, done
    if len(pending) != len(left):
        rest = ", ".join(f"`{p}`" for p in left if p not in by_path)
        why = f"other files are still conflicted ({rest}), so the merged tree would not build"
        return left, done + [Resolution(r.path, f"ran `{r.run}`", False, why) for r in pending]
    if not commands:
        return left, done + [Resolution(r.path, f"ran `{r.run}`", False,
                                        "the dependencies did not install") for r in pending]
    for resolver in pending:
        seconds = seconds_left() - reserve_seconds
        if seconds < MIN_SECONDS:
            done.append(Resolution(resolver.path, f"ran `{resolver.run}`", False,
                                   "too little of the run was left"))
            continue
        resolution = _run(wt, resolver, env, seconds)
        done.append(resolution)
        if resolution.ok:
            left.remove(resolver.path)
    return left, done


def summary(done: list[Resolution], left: list[str]) -> str:
    """One line for the journal: what was resolved, what was not and why, what is left."""
    parts = [f"`{r.path}`: {r.how}" + ("" if r.ok else f", not resolved: {r.why_not}")
             for r in done]
    if left:
        parts.append(f"{len(left)} file(s) left for the builder")
    return "; ".join(parts)
