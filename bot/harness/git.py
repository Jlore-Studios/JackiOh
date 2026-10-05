"""Git, run as a subprocess. No hook ever runs: every command sets `core.hooksPath` to nothing."""

from __future__ import annotations

import fnmatch
import posixpath
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Mapping

from harness import config as config_mod
from harness.errors import GitError

NO_HOOKS = ("-c", "core.hooksPath=/dev/null", "-c", "commit.gpgsign=false")


@dataclass(frozen=True)
class Identity:
    name: str
    email: str

    @classmethod
    def bot(cls, login: str, user_id: int) -> "Identity":
        return cls(login, f"{user_id}+{login}@users.noreply.github.com")


#: A line git writes into a file it could not merge: the start or the end of a conflict. A
#: Markdown heading underline (`=======`) is not one, so the middle marker is left out.
MARKER_LINE = r"^(<<<<<<< |>>>>>>> )"

class Git:
    def __init__(self, cwd: Path, env: Mapping[str, str] | None = None) -> None:
        self.cwd = Path(cwd)
        self.env = dict(env) if env is not None else config_mod.child_env()
        self.env.setdefault("GIT_TERMINAL_PROMPT", "0")

    def run(self, *args: str, check: bool = True, timeout: int = 600,
            extra_env: Mapping[str, str] | None = None) -> subprocess.CompletedProcess:
        env = dict(self.env)
        if extra_env:
            env.update(extra_env)
        proc = subprocess.run(
            ["git", *NO_HOOKS, *args],
            cwd=str(self.cwd),
            env=env,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
        )
        if check and proc.returncode != 0:
            raise GitError(f"git {' '.join(args)} failed ({proc.returncode}): "
                           f"{(proc.stderr or proc.stdout).strip()[-800:]}")
        return proc

    def out(self, *args: str, check: bool = True) -> str:
        return self.run(*args, check=check).stdout.strip()

    # ------------------------------------------------------------------ facts

    def head(self) -> str:
        return self.out("rev-parse", "HEAD")

    def rev(self, ref: str) -> str | None:
        proc = self.run("rev-parse", "--verify", "--quiet", f"{ref}^{{commit}}", check=False)
        return proc.stdout.strip() or None

    def merge_base(self, a: str, b: str = "HEAD") -> str:
        return self.out("merge-base", a, b)

    def changed_paths(self, base: str, head: str = "HEAD") -> list[str]:
        """Paths the branch changed since it left `base` (merge-base diff, renames split)."""
        text = self.out("diff", "--name-only", "--no-renames", f"{base}...{head}")
        return [line for line in text.splitlines() if line.strip()]

    def markers(self, ref: str, paths: Iterable[str]) -> list[str]:
        """The `paths` that hold a conflict marker line at `ref` (`MARKER_LINE`)."""
        wanted = [p for p in paths if p]
        if not wanted:
            return []
        proc = self.run("grep", "-l", "-E", MARKER_LINE, ref, "--", *wanted, check=False)
        found = []
        for line in proc.stdout.splitlines():
            path = line.split(":", 1)[1] if line.startswith(f"{ref}:") else line
            if path.strip():
                found.append(path.strip())
        return found

    def numstat(self, base: str, head: str = "HEAD") -> dict[str, int]:
        """Lines added plus removed per path since `head` left `base` (a binary file counts 0)."""
        text = self.out("diff", "--numstat", "--no-renames", f"{base}...{head}")
        counts: dict[str, int] = {}
        for line in text.splitlines():
            parts = line.split("\t", 2)
            if len(parts) != 3:
                continue
            added, removed, path = parts
            counts[path] = (int(added) if added.isdigit() else 0) + (
                int(removed) if removed.isdigit() else 0)
        return counts

    def names_between(self, a: str, b: str = "HEAD") -> list[str]:
        """Paths that differ between two commits (a two-dot diff, renames split)."""
        text = self.out("diff", "--name-only", "--no-renames", a, b)
        return [line for line in text.splitlines() if line.strip()]

    def blob(self, ref: str, path: str) -> str | None:
        """The blob id of `path` at `ref`, or None when it is not there."""
        proc = self.run("rev-parse", "--verify", "--quiet", f"{ref}:{path}", check=False)
        return proc.stdout.strip() or None

    def unsanctioned(self, head: str, anchors: list[str], patterns: Iterable[str]) -> list[str]:
        """Paths matching `patterns` that `head` holds differently from every anchor.

        The anchors are the commits the work may legitimately carry content from: where the
        branch started and the default branch it merged. A path that equals one of them came
        from there; a path that equals none was changed by the work itself."""
        candidates: set[str] = set()
        for anchor in anchors:
            candidates.update(self.names_between(anchor, head))
        found = []
        for path in sorted(candidates):
            if not matches(path, patterns):
                continue
            at_head = self.blob(head, path)
            if any(self.blob(anchor, path) == at_head for anchor in anchors):
                continue
            found.append(path)
        return found

    def restore_from(self, anchors: list[str], paths: Iterable[str]) -> None:
        """Put each path back as the first anchor that has it holds it, or delete it."""
        for path in paths:
            source = next((a for a in anchors if self.blob(a, path)), None)
            if source is not None:
                self.run("checkout", source, "--", path)
            else:
                self.run("rm", "-r", "--quiet", "--cached", "--ignore-unmatch", "--", path)
                target = self.cwd / path
                if target.is_file() or target.is_symlink():
                    target.unlink()

    def diff(self, base: str, head: str = "HEAD", max_chars: int = 200_000) -> tuple[str, bool]:
        """The merge-base diff and whether it was cut to `max_chars`."""
        text = self.out("diff", "--stat", "--patch", "--no-color", f"{base}...{head}")
        if len(text) > max_chars:
            return text[:max_chars], True
        return text, False

    def diffstat(self, base: str, head: str = "HEAD") -> str:
        return self.out("diff", "--stat", "--no-color", f"{base}...{head}")

    def dirty(self) -> list[str]:
        """Porcelain lines for tracked changes and untracked files that are not ignored."""
        text = self.run("status", "--porcelain", "--untracked-files=all").stdout
        return [line for line in text.splitlines() if line.strip()]

    def log(self, base: str, head: str = "HEAD") -> str:
        return self.out("log", "--oneline", "--no-decorate", f"{base}..{head}", check=False)

    # ------------------------------------------------------------------ changes

    def commit_all(self, message: str, who: Identity) -> bool:
        """Stage everything (ignored files excepted) and commit. False when nothing changed."""
        self.run("add", "--all")
        if not self.run("diff", "--cached", "--quiet", check=False).returncode:
            return False
        env = {
            "GIT_AUTHOR_NAME": who.name,
            "GIT_AUTHOR_EMAIL": who.email,
            "GIT_COMMITTER_NAME": who.name,
            "GIT_COMMITTER_EMAIL": who.email,
        }
        self.run("commit", "--quiet", "--no-verify", "-m", message, extra_env=env)
        return True

    def restore_paths(self, base: str, paths: Iterable[str]) -> list[str]:
        """Put `paths` back as they are at `base` (deleting ones `base` lacks). Returns them."""
        restored: list[str] = []
        for path in paths:
            exists = self.run("cat-file", "-e", f"{base}:{path}", check=False).returncode == 0
            if exists:
                self.run("checkout", base, "--", path)
            else:
                self.run("rm", "-r", "--quiet", "--cached", "--ignore-unmatch", "--", path)
                target = self.cwd / path
                if target.is_file() or target.is_symlink():
                    target.unlink()
            restored.append(path)
        return restored

    def discard_worktree_changes(self) -> None:
        """Drop uncommitted edits and untracked, non-ignored files."""
        self.run("reset", "--quiet", "--hard", "HEAD")
        self.run("clean", "-fdq")

    def merge(self, ref: str, who: Identity) -> list[str]:
        """Merge `ref` into the current branch. Returns conflicted paths (merge left open)."""
        env = {
            "GIT_AUTHOR_NAME": who.name,
            "GIT_AUTHOR_EMAIL": who.email,
            "GIT_COMMITTER_NAME": who.name,
            "GIT_COMMITTER_EMAIL": who.email,
        }
        proc = self.run("merge", "--no-edit", "--no-ff", ref, check=False, extra_env=env)
        if proc.returncode == 0:
            return []
        conflicted = self.out("diff", "--name-only", "--diff-filter=U", check=False)
        paths = [p for p in conflicted.splitlines() if p.strip()]
        if not paths:
            raise GitError(f"git merge {ref} failed: {(proc.stderr or proc.stdout).strip()[-800:]}")
        return paths

    def merge_in_progress(self) -> bool:
        return self.run("rev-parse", "-q", "--verify", "MERGE_HEAD", check=False).returncode == 0

    # ------------------------------------------------------------------ transport

    def bundle(self, path: Path, branch: str, exclude: Iterable[str]) -> None:
        """A bundle holding `branch` and every commit it has that no `exclude` commit has."""
        self.run("bundle", "create", str(path), f"refs/heads/{branch}",
                 *[f"^{sha}" for sha in exclude])

    def fetch_bundle(self, path: Path, branch: str, local: str) -> str:
        self.run("bundle", "verify", str(path))
        self.run("fetch", "--quiet", str(path), f"+refs/heads/{branch}:refs/heads/{local}")
        return self.out("rev-parse", local)

    def push(self, remote_url: str, refspec: str, token: str, *, force_with_lease: str = "") -> None:
        """Push with the token in an http.extraheader passed through the environment."""
        import base64

        basic = base64.b64encode(f"x-access-token:{token}".encode()).decode()
        env = {
            "GIT_CONFIG_COUNT": "2",
            "GIT_CONFIG_KEY_0": "http.https://github.com/.extraheader",
            "GIT_CONFIG_VALUE_0": f"AUTHORIZATION: basic {basic}",
            "GIT_CONFIG_KEY_1": "credential.helper",
            "GIT_CONFIG_VALUE_1": "",
        }
        args = ["push", "--quiet", "--no-verify"]
        if force_with_lease:
            args.append(f"--force-with-lease={force_with_lease}")
        self.run(*args, remote_url, refspec, extra_env=env, timeout=300)


def normalise(path: str) -> str:
    """A repository path in one canonical form: forward slashes, no `./`, lower case."""
    clean = posixpath.normpath(str(path).replace("\\", "/")).lstrip("/")
    while clean.startswith("./"):
        clean = clean[2:]
    return clean.lower()


def matches(path: str, patterns: Iterable[str]) -> bool:
    """True when `path` matches a pattern: `dir/` is that directory and everything under it,
    a pattern with a `/` is a glob on the whole path, and a bare name is a glob on the last
    part of the path at any depth. Case never matters."""
    clean = normalise(path)
    name = clean.rsplit("/", 1)[-1]
    for raw in patterns:
        pattern = normalise(raw) + ("/" if str(raw).endswith("/") else "")
        if pattern.endswith("/"):
            if clean == pattern.rstrip("/") or clean.startswith(pattern):
                return True
        elif "/" in pattern:
            if fnmatch.fnmatchcase(clean, pattern):
                return True
        elif fnmatch.fnmatchcase(name, pattern) or fnmatch.fnmatchcase(clean, pattern):
            return True
    return False


def is_forbidden(path: str, forbidden: Iterable[str]) -> bool:
    return matches(path, forbidden)


def worktree_add(repo: Git, path: Path, branch: str, start: str) -> Git:
    """A worktree at `path` with `branch` (re)set to `start` and checked out. An older worktree
    of this clone that still has `branch` checked out (an earlier item on the same branch) is
    removed first, since git lets only one worktree hold a branch."""
    if path.exists():
        repo.run("worktree", "remove", "--force", str(path), check=False)
    listing = repo.run("worktree", "list", "--porcelain", check=False).stdout
    current = ""
    for line in listing.splitlines():
        if line.startswith("worktree "):
            current = line[len("worktree "):]
        elif line == f"branch refs/heads/{branch}" and current and Path(current) != repo.cwd:
            repo.run("worktree", "remove", "--force", current, check=False)
    repo.run("worktree", "prune", check=False)
    repo.run("worktree", "add", "-q", "-B", branch, str(path), start)
    return Git(path, repo.env)
