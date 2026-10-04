"""Test doubles: an in-memory GitHub, and helpers for real git repositories in a temp dir."""

from __future__ import annotations

import copy
import itertools
import json
import os
import subprocess
from pathlib import Path
from typing import Any

from harness.errors import GitHubError
from harness.gh import with_marker

OWNER = "jgoetzmann"
REPO = "jgoetzmann/JackiOh"
BOT = {"login": "jgoetzmann-bot", "id": 324324626}
OPERATOR = {"login": "jgoetzmann", "id": 95732896}
STRANGER = {"login": "someone-else", "id": 5}


class FakeGitHub:
    """The subset of `harness.gh.GitHub` the harness calls, kept in memory."""

    def __init__(self, repo: str = REPO) -> None:
        self.repo = repo
        self.threads: dict[int, dict[str, Any]] = {}
        self.comments: dict[int, list[dict[str, Any]]] = {}
        self.reviews: dict[int, list[dict[str, Any]]] = {}
        self.review_comments: dict[int, list[dict[str, Any]]] = {}
        self.labels: dict[str, dict[str, Any]] = {}
        self.files: dict[tuple[str, str], tuple[str, str]] = {}
        self.branches: set[str] = {"main"}
        self.dispatches: list[dict[str, Any]] = []
        self.pinned: list[str] = []
        self.runs: dict[str, dict[str, Any]] = {}
        self.jobs: dict[str, list[dict[str, Any]]] = {}
        self.reruns: list[Any] = []
        self.reacted: list[tuple[int, str]] = []
        self.auto_merge: dict[str, str] = {}
        self.auto_merge_heads: dict[str, str] = {}
        self.blockers: dict[int, list[int]] = {}
        #: The merge commit title asked for, by pull request node id.
        self.merge_titles: dict[str, str] = {}
        self.auto_merge_error: str = ""
        self.protection: dict[str, Any] | None = None
        self.branch_checks: set[str] | None = None
        self.review_requests: list[tuple[int, list[str]]] = []
        self.repo_settings: dict[str, Any] = {"allow_auto_merge": True}
        self.conflicts_to_inject = 0
        self.dry_run = False
        self._ids = itertools.count(1000)
        self._shas = itertools.count(1)

    # ------------------------------------------------------------------ building the world

    def add_issue(self, number: int, title: str = "An issue", body: str = "Do the thing.",
                  labels: tuple[str, ...] = (), user: dict[str, Any] | None = None,
                  state: str = "open") -> dict[str, Any]:
        thread = {
            "number": number, "title": title, "body": body, "state": state,
            "labels": [{"name": n} for n in labels], "user": dict(user or OPERATOR),
            "created_at": f"2026-09-{10 + number % 15:02d}T00:00:00Z", "assignees": [],
            "id": 5_000_000 + number,
        }
        self.threads[number] = thread
        self.comments.setdefault(number, [])
        return thread

    def add_pull(self, number: int, branch: str, title: str = "A pull request", body: str = "",
                 labels: tuple[str, ...] = (), sha: str = "abc", fork: bool = False,
                 user: dict[str, Any] | None = None) -> dict[str, Any]:
        thread = self.add_issue(number, title, body, labels, user)
        thread.update({
            "pull_request": {},
            "head": {"ref": branch, "sha": sha,
                     "repo": {"full_name": "someone/fork" if fork else self.repo}},
            "base": {"ref": "main"},
            "node_id": f"PR_{number}",
            "draft": False,
            "auto_merge": None,
            "mergeable_state": "clean",
            "merged": False,
        })
        self.branches.add(branch)
        return thread

    def add_comment(self, number: int, body: str, user: dict[str, Any] | None = None,
                    association: str = "OWNER", created_at: str = "2026-09-20T00:00:00Z") -> dict:
        comment = {"id": next(self._ids), "body": body, "user": dict(user or OPERATOR),
                   "author_association": association, "created_at": created_at,
                   "issue_url": f"https://api.github.com/repos/{self.repo}/issues/{number}"}
        self.comments.setdefault(number, []).append(comment)
        return comment

    def add_review_comment(self, number: int, body: str, user: dict[str, Any] | None = None,
                           association: str = "OWNER",
                           created_at: str = "2026-09-20T00:00:00Z") -> dict:
        comment = {"id": next(self._ids), "body": body, "user": dict(user or OPERATOR),
                   "author_association": association, "created_at": created_at,
                   "path": "src/x.ts", "line": 3,
                   "pull_request_url": f"https://api.github.com/repos/{self.repo}/pulls/{number}"}
        self.review_comments.setdefault(number, []).append(comment)
        return comment

    def label_names(self, number: int) -> set[str]:
        return {label["name"] for label in self.threads[number]["labels"]}

    def bot_comments(self, number: int) -> list[str]:
        return [c["body"] for c in self.comments.get(number, []) if c["user"]["login"] == BOT["login"]]

    # ------------------------------------------------------------------ issues

    def get_issue(self, number: int) -> dict[str, Any]:
        if number not in self.threads:
            raise GitHubError(f"no issue {number}", 404)
        return copy.deepcopy(self.threads[number])

    def list_issues(self, *, labels: str = "", state: str = "open", limit: int = 300,
                    assignee: str = "") -> list[dict]:
        wanted = [n for n in labels.split(",") if n]
        found = []
        for number in sorted(self.threads):
            thread = self.threads[number]
            if state != "all" and thread["state"] != state:
                continue
            names = {label["name"] for label in thread["labels"]}
            assigned = {a.get("login") for a in thread.get("assignees", [])}
            if all(n in names for n in wanted) and (not assignee or assignee in assigned):
                found.append(copy.deepcopy(thread))
        return found[:limit]

    def block(self, number: int, *blockers: int) -> None:
        """Link `blockers` as GitHub issue dependencies blocking `number`."""
        self.blockers.setdefault(number, []).extend(blockers)
        open_ones = [b for b in self.blockers[number] if self.threads[b]["state"] == "open"]
        self.threads[number]["issue_dependencies_summary"] = {
            "blocked_by": len(open_ones), "total_blocked_by": len(self.blockers[number]),
            "blocking": 0, "total_blocking": 0}

    def blocked_by(self, number: int) -> list[dict]:
        return [copy.deepcopy(self.threads[b]) for b in self.blockers.get(number, [])]

    def blocking(self, number: int) -> list[dict]:
        return [copy.deepcopy(self.threads[n]) for n, found in sorted(self.blockers.items())
                if number in found]

    def _by_id(self, issue_id: int) -> int:
        found = [n for n, t in self.threads.items() if t.get("id") == issue_id]
        if not found:
            raise GitHubError(f"no issue with id {issue_id}", 404)
        return found[0]

    def add_blocked_by(self, number: int, blocker_id: int) -> None:
        self.block(number, self._by_id(blocker_id))

    def add_sub_issue(self, parent: int, child_id: int) -> None:
        child = self._by_id(child_id)
        self.threads[child]["parent_issue_url"] = (
            f"https://api.github.com/repos/{self.repo}/issues/{parent}")

    def list_comments(self, number: int, limit: int = 300) -> list[dict]:
        return copy.deepcopy(self.comments.get(number, []))

    def list_commits(self, *, author: str = "", limit: int = 1000) -> list[dict]:
        return [c for c in getattr(self, "commits", [])
                if not author or (c.get("author") or {}).get("login") == author][:limit]

    def create_comment(self, number: int, body: str) -> dict[str, Any]:
        comment = {"id": next(self._ids), "body": with_marker(body), "user": dict(BOT),
                   "author_association": "COLLABORATOR", "created_at": "2026-09-29T00:00:00Z",
                   "issue_url": f"https://api.github.com/repos/{self.repo}/issues/{number}"}
        self.comments.setdefault(number, []).append(comment)
        return comment

    def react(self, comment_id: int, content: str, *, review_comment: bool = False) -> None:
        self.reacted.append((comment_id, content))

    def reactions(self, comment_id: int, *, review_comment: bool = False) -> list[dict]:
        return [{"content": c, "user": dict(BOT)} for i, c in self.reacted if i == comment_id]

    def list_repo_comments(self, since: str, limit: int = 500) -> list[dict]:
        found = [c for items in self.comments.values() for c in items
                 if c.get("created_at", "") >= since]
        return copy.deepcopy(found[:limit])

    def list_repo_review_comments(self, since: str, limit: int = 500) -> list[dict]:
        found = [c for items in self.review_comments.values() for c in items
                 if c.get("created_at", "") >= since]
        return copy.deepcopy(found[:limit])

    def runs_for_sha(self, sha: str, limit: int = 30) -> list[dict]:
        return [dict(r) for r in self.runs.values() if r.get("head_sha") == sha][:limit]

    def create_issue(self, title: str, body: str, labels: Any = ()) -> dict[str, Any]:
        number = max(self.threads, default=0) + 1
        thread = self.add_issue(number, title, with_marker(body), tuple(labels), BOT)
        thread["node_id"] = f"I_{number}"
        return copy.deepcopy(thread)

    def pin_issue(self, node_id: str) -> None:
        if len(self.pinned) >= 3:
            raise GitHubError("a repository pins at most three issues", 422)
        self.pinned.append(node_id)

    def set_issue_body(self, number: int, body: str) -> dict[str, Any]:
        self.threads[number]["body"] = str(body)
        return copy.deepcopy(self.threads[number])

    def update_issue(self, number: int, **fields: Any) -> dict[str, Any]:
        self.threads[number].update(fields)
        return copy.deepcopy(self.threads[number])

    def add_labels(self, number: int, names: Any) -> None:
        current = self.label_names(number)
        for name in names:
            if name not in current:
                self.threads[number]["labels"].append({"name": name})

    def add_assignees(self, number: int, logins: Any) -> None:
        current = {a.get("login") for a in self.threads[number].setdefault("assignees", [])}
        for login in logins:
            if login not in current:
                self.threads[number]["assignees"].append({"login": login})

    def remove_label(self, number: int, name: str) -> None:
        self.threads[number]["labels"] = [l for l in self.threads[number]["labels"] if l["name"] != name]

    def list_labels(self) -> list[dict]:
        return list(self.labels.values())

    def list_issue_types(self) -> list[dict]:
        return [{"name": "Task", "description": "A specific piece of work"},
                {"name": "Bug", "description": "An unexpected problem or behavior"},
                {"name": "Feature", "description": "A request, idea, or new functionality"}]

    def ensure_label(self, name: str, color: str, description: str) -> bool:
        if name in self.labels:
            return False
        self.labels[name] = {"name": name, "color": color, "description": description}
        return True

    # ------------------------------------------------------------------ pull requests

    def get_pull(self, number: int) -> dict[str, Any]:
        thread = self.get_issue(number)
        if "pull_request" not in thread:
            raise GitHubError(f"{number} is not a pull request", 404)
        return thread

    def list_pulls(self, *, state: str = "open", head: str = "", limit: int = 200) -> list[dict]:
        found = []
        for thread in self.threads.values():
            if "pull_request" not in thread or (state != "all" and thread["state"] != state):
                continue
            if head and thread["head"]["ref"] != head:
                continue
            found.append(copy.deepcopy(thread))
        return found

    def create_pull(self, *, title: str, head: str, base: str, body: str,
                    draft: bool = False) -> dict[str, Any]:
        number = max(self.threads, default=0) + 1
        thread = self.add_pull(number, head, title, with_marker(body), (), sha="pushed", user=BOT)
        thread["draft"] = draft
        return copy.deepcopy(thread)

    def update_pull(self, number: int, **fields: Any) -> dict[str, Any]:
        self.threads[number].update(fields)
        return copy.deepcopy(self.threads[number])

    def list_reviews(self, number: int) -> list[dict]:
        return copy.deepcopy(self.reviews.get(number, []))

    def list_review_comments(self, number: int) -> list[dict]:
        return copy.deepcopy(self.review_comments.get(number, []))

    def _pull_by_node(self, node_id: str) -> dict[str, Any]:
        return next(t for t in self.threads.values() if t.get("node_id") == node_id)

    def request_reviewers(self, number: int, logins: list[str]) -> None:
        self.review_requests.append((number, list(logins)))

    def required_checks(self, branch: str) -> set[str] | None:
        return None if self.branch_checks is None else set(self.branch_checks)

    def enable_auto_merge(self, node_id: str, method: str, expected_head: str = "",
                          headline: str = "") -> None:
        if self.auto_merge_error:
            raise GitHubError(self.auto_merge_error, 200)
        self.auto_merge[node_id] = method
        self.auto_merge_heads[node_id] = expected_head
        self.merge_titles[node_id] = headline
        self._pull_by_node(node_id)["auto_merge"] = {"merge_method": method}

    def disable_auto_merge(self, node_id: str) -> None:
        self.auto_merge.pop(node_id, None)
        self._pull_by_node(node_id)["auto_merge"] = None

    def mark_ready(self, node_id: str) -> None:
        self._pull_by_node(node_id)["draft"] = False

    def merge_pull(self, number: int, method: str, title: str = "") -> dict[str, Any]:
        self.threads[number].update(state="closed", merged=True)
        self.merge_titles[f"PR_{number}"] = title
        return {"merged": True}

    # ------------------------------------------------------------------ contents

    def get_file(self, path: str, ref: str) -> tuple[str | None, str | None]:
        found = self.files.get((ref, path))
        return (found[0], found[1]) if found else (None, None)

    def put_file(self, path: str, text: str, *, branch: str, sha: str | None, message: str) -> None:
        if branch not in self.branches:
            raise GitHubError(f"no branch {branch}", 404)
        current = self.files.get((branch, path))
        if self.conflicts_to_inject > 0:
            self.conflicts_to_inject -= 1
            # Someone else wrote in between: their change lands, ours is refused.
            other = json.loads(current[0]) if current else {}
            other.setdefault("items", {})["999"] = {"note": "written by another run"}
            self.files[(branch, path)] = (json.dumps(other), f"sha{next(self._shas)}")
            raise GitHubError("conflict", 409)
        if (current[1] if current else None) != (sha or None):
            raise GitHubError("sha mismatch", 409)
        self.files[(branch, path)] = (text, f"sha{next(self._shas)}")

    def branch_sha(self, branch: str) -> str | None:
        return f"head-of-{branch}" if branch in self.branches else None

    def create_orphan_branch(self, branch: str, path: str, text: str, message: str) -> None:
        self.branches.add(branch)
        self.files[(branch, path)] = (text, f"sha{next(self._shas)}")

    def delete_branch(self, branch: str) -> None:
        self.branches.discard(branch)

    # ------------------------------------------------------------------ actions

    def dispatch_workflow(self, workflow: str, ref: str, inputs: dict[str, str]) -> None:
        self.dispatches.append({"workflow": workflow, "ref": ref, "inputs": dict(inputs)})

    def get_run(self, run_id: Any) -> dict[str, Any]:
        if str(run_id) not in self.runs:
            raise GitHubError("no run", 404)
        return dict(self.runs[str(run_id)])

    def list_runs(self, workflow: str, *, status: str = "", limit: int = 30) -> list[dict]:
        return list(self.runs.values())[:limit]

    def rerun_failed_jobs(self, run_id: Any) -> None:
        self.reruns.append(run_id)

    def list_jobs(self, run_id: Any) -> list[dict]:
        return copy.deepcopy(self.jobs.get(str(run_id), []))

    def job_log(self, job_id: Any, tail_chars: int = 6000) -> str:
        return f"log of job {job_id}: AssertionError: expected 3 to be 4"

    def check_runs(self, sha: str) -> list[dict]:
        return []

    # ------------------------------------------------------------------ settings

    def repo_info(self) -> dict[str, Any]:
        return dict(self.repo_settings)

    def viewer(self) -> dict[str, Any]:
        return dict(BOT)

    def get_protection(self, branch: str) -> dict[str, Any] | None:
        return self.protection

    def set_protection(self, branch: str, checks: list[str]) -> None:
        self.protection = {"required_status_checks": {"contexts": list(checks)}}

    def update_repo(self, **fields: Any) -> dict[str, Any]:
        self.repo_settings.update(fields)
        return dict(self.repo_settings)


# ---------------------------------------------------------------------- git helpers

GIT_ENV = {
    "GIT_AUTHOR_NAME": "Test", "GIT_AUTHOR_EMAIL": "test@example.com",
    "GIT_COMMITTER_NAME": "Test", "GIT_COMMITTER_EMAIL": "test@example.com",
    "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_NOSYSTEM": "1",
}


def git(cwd: Path, *args: str) -> str:
    env = {**os.environ, **GIT_ENV}
    proc = subprocess.run(["git", *args], cwd=str(cwd), env=env, capture_output=True, text=True)
    if proc.returncode != 0:
        raise AssertionError(f"git {' '.join(args)}: {proc.stderr}")
    return proc.stdout.strip()


def make_origin(root: Path, files: dict[str, str] | None = None) -> tuple[Path, Path]:
    """A bare `origin` holding a `main` branch, and a clone of it. Returns (origin, clone)."""
    seed = root / "seed"
    seed.mkdir(parents=True)
    git(seed, "init", "-q", "-b", "main")
    for name, text in (files or {"README.md": "hello\n", "src/game.txt": "rules v1\n"}).items():
        target = seed / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)
    git(seed, "add", "-A")
    git(seed, "commit", "-q", "-m", "initial")
    origin = root / "remote" / "jgoetzmann" / "JackiOh.git"
    origin.parent.mkdir(parents=True)
    git(root, "clone", "-q", "--bare", str(seed), str(origin))
    clone = root / "clone"
    git(root, "clone", "-q", str(origin), str(clone))
    return origin, clone


def push_branch(origin: Path, root: Path, branch: str, files: dict[str, str], base: str = "main") -> str:
    """Put a branch on `origin` that changes `files` on top of `base`. Returns its sha."""
    work = root / f"push-{branch.replace('/', '-')}"
    if not work.exists():
        git(root, "clone", "-q", str(origin), str(work))
    git(work, "fetch", "-q", "origin")
    git(work, "checkout", "-q", "-B", branch, f"origin/{base}")
    for name, text in files.items():
        target = work / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)
    git(work, "add", "-A")
    git(work, "commit", "-q", "-m", f"change on {branch}")
    git(work, "push", "-q", "-f", "origin", f"{branch}:{branch}")
    return git(work, "rev-parse", "HEAD")
