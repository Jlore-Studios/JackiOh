"""The GitHub client. The only module that sends a token or writes to GitHub.

Every body the harness posts is redacted and carries `config.MARKER`. In dry-run mode every
write is recorded in `writes` and answered with a stand-in, and nothing is sent.
"""

from __future__ import annotations

import base64
import json
import time
import urllib.error
import urllib.parse
import urllib.request
from typing import Any, Callable, Iterable

from harness import __version__
from harness.config import MARKER
from harness.errors import GitHubError
from harness.redact import redact

API = "https://api.github.com"
USER_AGENT = f"jackioh-night-bot/{__version__}"
RETRIES = 3
BACKOFF_SECONDS = 2.0


def with_marker(body: str) -> str:
    """`body`, redacted, with the marker once at the end."""
    text = redact(str(body or ""))
    if MARKER in text:
        return text
    return f"{text}\n\n{MARKER}" if text.strip() else MARKER


class _NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):  # noqa: D401
        return None


class GitHub:
    """A small REST and GraphQL client for one repository."""

    def __init__(
        self,
        repo: str,
        token: str,
        *,
        dry_run: bool = False,
        opener: Callable[..., Any] | None = None,
        sleep: Callable[[float], None] = time.sleep,
        fallback_token: str = "",
    ) -> None:
        self.repo = repo
        self.token = token
        #: Used for the rest of the run when `token` is refused (a revoked or expired bot token),
        #: so replies still go out, as the Actions bot.
        self.fallback_token = fallback_token
        self.dry_run = dry_run
        self._open = opener or urllib.request.urlopen
        self._sleep = sleep
        self.writes: list[dict[str, Any]] = []

    # ------------------------------------------------------------------ transport

    def _headers(self, accept: str | None = None) -> dict[str, str]:
        headers = {
            "User-Agent": USER_AGENT,
            "Accept": accept or "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
        }
        if self.token:
            headers["Authorization"] = f"Bearer {self.token}"
        return headers

    def _url(self, path: str) -> str:
        if path.startswith("https://"):
            return path
        return API + (path if path.startswith("/") else "/" + path)

    def request(
        self,
        method: str,
        path: str,
        body: Any = None,
        *,
        accept: str | None = None,
        raw: bool = False,
    ) -> Any:
        """One request, retried on 5xx, connection errors and secondary rate limits."""
        if method != "GET" and self.dry_run:
            self.writes.append({"method": method, "path": path, "body": body})
            return {}
        url = self._url(path)
        data = None if body is None else json.dumps(body).encode("utf-8")
        last: Exception | None = None
        for attempt in range(RETRIES):
            req = urllib.request.Request(url, data=data, method=method, headers=self._headers(accept))
            if data is not None:
                req.add_header("Content-Type", "application/json")
            try:
                with self._open(req, timeout=60) as resp:
                    payload = resp.read()
                    text = payload.decode("utf-8", "replace") if isinstance(payload, bytes) else str(payload)
                    if raw:
                        return text
                    return json.loads(text) if text.strip() else {}
            except urllib.error.HTTPError as exc:
                detail = exc.read().decode("utf-8", "replace") if exc.fp else ""
                if exc.code == 401 and self.fallback_token and self.token != self.fallback_token:
                    self.token = self.fallback_token
                    continue
                retry_after = exc.headers.get("Retry-After") if exc.headers else None
                limited = exc.code in (403, 429) and (
                    retry_after or "secondary rate limit" in detail.lower()
                )
                # A POST that failed with a 5xx may still have happened (a comment posted, a
                # pull request opened), so only a rate-limit refusal is retried for it.
                retryable = limited or (exc.code >= 500 and method != "POST")
                if retryable and attempt + 1 < RETRIES:
                    self._sleep(float(retry_after or BACKOFF_SECONDS * (attempt + 1)))
                    last = exc
                    continue
                parsed: Any = detail
                try:
                    parsed = json.loads(detail)
                except ValueError:
                    pass
                message = parsed.get("message", detail) if isinstance(parsed, dict) else detail
                raise GitHubError(
                    f"{method} {path}: HTTP {exc.code}: {redact(str(message))[:500]}",
                    exc.code,
                    parsed,
                ) from None
            except (urllib.error.URLError, TimeoutError, ConnectionError) as exc:
                last = exc
                if attempt + 1 < RETRIES:
                    self._sleep(BACKOFF_SECONDS * (attempt + 1))
                    continue
        raise GitHubError(f"{method} {path}: {redact(str(last))}", 0)

    def paginate(self, path: str, params: dict[str, Any] | None = None, *, key: str | None = None,
                 limit: int = 1000) -> list[Any]:
        """Every item of a paged list, up to `limit`."""
        query = dict(params or {})
        query.setdefault("per_page", 100)
        items: list[Any] = []
        page = 1
        while len(items) < limit:
            query["page"] = page
            chunk = self.request("GET", f"{path}?{urllib.parse.urlencode(query)}")
            if key is not None and isinstance(chunk, dict):
                chunk = chunk.get(key, [])
            if not isinstance(chunk, list) or not chunk:
                break
            items.extend(chunk)
            if len(chunk) < int(query["per_page"]):
                break
            page += 1
        return items[:limit]

    def graphql(self, query: str, variables: dict[str, Any]) -> dict[str, Any]:
        result = self.request("POST", "/graphql", {"query": query, "variables": variables})
        if self.dry_run:
            return {}
        errors = result.get("errors") if isinstance(result, dict) else None
        if errors:
            text = "; ".join(str(e.get("message", e)) for e in errors)
            raise GitHubError(f"graphql: {redact(text)[:500]}", 200, errors)
        return result.get("data", {}) if isinstance(result, dict) else {}

    @property
    def _r(self) -> str:
        return f"/repos/{self.repo}"

    # ------------------------------------------------------------------ repository

    def repo_info(self) -> dict[str, Any]:
        return self.request("GET", self._r)

    def viewer(self) -> dict[str, Any]:
        return self.request("GET", "/user")

    def get_user(self, login: str) -> dict[str, Any]:
        return self.request("GET", f"/users/{urllib.parse.quote(login)}")

    # ------------------------------------------------------------------ issues and comments

    def get_issue(self, number: int) -> dict[str, Any]:
        return self.request("GET", f"{self._r}/issues/{int(number)}")

    def list_issues(self, *, labels: str = "", state: str = "open", limit: int = 300,
                    assignee: str = "") -> list[dict]:
        params: dict[str, Any] = {"state": state}
        if labels:
            params["labels"] = labels
        if assignee:
            params["assignee"] = assignee
        return self.paginate(f"{self._r}/issues", params, limit=limit)

    def list_repo_comments(self, since: str, limit: int = 500) -> list[dict]:
        """Every issue and pull request conversation comment updated since `since`, newest
        first, so a limit drops the oldest."""
        return self.paginate(f"{self._r}/issues/comments",
                             {"since": since, "sort": "created", "direction": "desc"}, limit=limit)

    def list_repo_review_comments(self, since: str, limit: int = 500) -> list[dict]:
        """Every line comment on a pull request's diff updated since `since`, newest first."""
        return self.paginate(f"{self._r}/pulls/comments",
                             {"since": since, "sort": "created", "direction": "desc"}, limit=limit)

    def reactions(self, comment_id: int, *, review_comment: bool = False) -> list[dict]:
        kind = "pulls/comments" if review_comment else "issues/comments"
        return self.paginate(f"{self._r}/{kind}/{int(comment_id)}/reactions")

    def list_comments(self, number: int, limit: int = 300) -> list[dict]:
        return self.paginate(f"{self._r}/issues/{int(number)}/comments", limit=limit)

    def create_comment(self, number: int, body: str) -> dict[str, Any]:
        return self.request(
            "POST", f"{self._r}/issues/{int(number)}/comments", {"body": with_marker(body)}
        )

    def react(self, comment_id: int, content: str, *, review_comment: bool = False) -> None:
        kind = "pulls/comments" if review_comment else "issues/comments"
        try:
            self.request("POST", f"{self._r}/{kind}/{int(comment_id)}/reactions", {"content": content})
        except GitHubError:
            pass  # a reaction is a courtesy; its failure changes nothing

    def create_issue(self, title: str, body: str, labels: Iterable[str] = ()) -> dict[str, Any]:
        payload = {"title": redact(title)[:250], "body": with_marker(body), "labels": list(labels)}
        return self.request("POST", f"{self._r}/issues", payload)

    def set_issue_body(self, number: int, body: str) -> dict[str, Any]:
        """Replace an issue's description as it is: no redaction of a person's text and no bot
        marker, which would make the whole description read as the bot's (the plan section,
        `issueplan.py`, is redacted before it gets here)."""
        return self.request("PATCH", f"{self._r}/issues/{int(number)}", {"body": str(body)})

    def blocked_by(self, number: int) -> list[dict]:
        """The issues GitHub's own issue dependencies say block this one, open or closed."""
        return self.paginate(f"{self._r}/issues/{int(number)}/dependencies/blocked_by", limit=100)

    def blocking(self, number: int) -> list[dict]:
        """The issues GitHub's own issue dependencies say this one blocks, open or closed."""
        return self.paginate(f"{self._r}/issues/{int(number)}/dependencies/blocking", limit=100)

    def add_blocked_by(self, number: int, blocker_id: int) -> None:
        """Mark issue `number` blocked by the issue whose id (not number) is `blocker_id`."""
        self.request("POST", f"{self._r}/issues/{int(number)}/dependencies/blocked_by",
                     {"issue_id": int(blocker_id)})

    def add_sub_issue(self, parent: int, child_id: int) -> None:
        """Make the issue whose id (not number) is `child_id` a sub-issue of issue `parent`."""
        self.request("POST", f"{self._r}/issues/{int(parent)}/sub_issues",
                     {"sub_issue_id": int(child_id)})

    def update_issue(self, number: int, **fields: Any) -> dict[str, Any]:
        if "body" in fields:
            fields["body"] = with_marker(fields["body"])
        return self.request("PATCH", f"{self._r}/issues/{int(number)}", fields)

    def add_labels(self, number: int, names: Iterable[str]) -> None:
        names = [n for n in names if n]
        if names:
            self.request("POST", f"{self._r}/issues/{int(number)}/labels", {"labels": names})

    def add_assignees(self, number: int, logins: Iterable[str]) -> None:
        """Adds to whoever is assigned; GitHub skips an account it cannot assign."""
        logins = [login for login in logins if login]
        if logins:
            self.request("POST", f"{self._r}/issues/{int(number)}/assignees",
                         {"assignees": logins})

    def remove_label(self, number: int, name: str) -> None:
        quoted = urllib.parse.quote(name, safe="")
        try:
            self.request("DELETE", f"{self._r}/issues/{int(number)}/labels/{quoted}")
        except GitHubError as exc:
            if exc.status != 404:
                raise

    def list_labels(self) -> list[dict]:
        return self.paginate(f"{self._r}/labels")

    def list_issue_types(self) -> list[dict]:
        """The owning organisation's issue types (none for a user's repository)."""
        owner = self.repo.split("/", 1)[0]
        try:
            return list(self.request("GET", f"/orgs/{owner}/issue-types") or [])
        except GitHubError as exc:
            if exc.status in (403, 404):
                return []
            raise

    def ensure_label(self, name: str, color: str, description: str) -> bool:
        """Create the label unless it exists. True when it was created."""
        quoted = urllib.parse.quote(name, safe="")
        try:
            self.request("GET", f"{self._r}/labels/{quoted}")
            return False
        except GitHubError as exc:
            if exc.status != 404:
                raise
        self.request(
            "POST", f"{self._r}/labels", {"name": name, "color": color, "description": description}
        )
        return True

    # ------------------------------------------------------------------ pull requests

    def get_pull(self, number: int) -> dict[str, Any]:
        return self.request("GET", f"{self._r}/pulls/{int(number)}")

    def list_pulls(self, *, state: str = "open", head: str = "", limit: int = 200) -> list[dict]:
        params: dict[str, Any] = {"state": state}
        if head:
            params["head"] = f"{self.repo.split('/')[0]}:{head}"
        return self.paginate(f"{self._r}/pulls", params, limit=limit)

    def create_pull(self, *, title: str, head: str, base: str, body: str,
                    draft: bool = False) -> dict[str, Any]:
        payload = {
            "title": redact(title)[:250],
            "head": head,
            "base": base,
            "body": with_marker(body),
            "draft": draft,
            "maintainer_can_modify": True,
        }
        return self.request("POST", f"{self._r}/pulls", payload)

    def update_pull(self, number: int, **fields: Any) -> dict[str, Any]:
        if "body" in fields:
            fields["body"] = with_marker(fields["body"])
        if "title" in fields:
            fields["title"] = redact(fields["title"])[:250]
        return self.request("PATCH", f"{self._r}/pulls/{int(number)}", fields)

    def list_reviews(self, number: int) -> list[dict]:
        return self.paginate(f"{self._r}/pulls/{int(number)}/reviews")

    def list_review_comments(self, number: int) -> list[dict]:
        return self.paginate(f"{self._r}/pulls/{int(number)}/comments")

    def request_reviewers(self, number: int, logins: list[str]) -> None:
        self.request("POST", f"{self._r}/pulls/{int(number)}/requested_reviewers",
                     {"reviewers": list(logins)})

    def required_checks(self, branch: str) -> set[str] | None:
        """The status checks branch protection requires on `branch`, or None when unprotected.

        Read through the branch endpoint, which any reader may call, rather than the protection
        endpoint, which needs admin rights the bot account does not have."""
        data = self.request("GET", f"{self._r}/branches/{urllib.parse.quote(branch)}")
        if not isinstance(data, dict) or not data.get("protected"):
            return None
        checks = ((data.get("protection") or {}).get("required_status_checks") or {})
        found = {str(c) for c in checks.get("contexts") or []}
        found |= {str(c.get("context")) for c in checks.get("checks") or [] if isinstance(c, dict)}
        return found

    def enable_auto_merge(self, node_id: str, method: str, expected_head: str = "",
                          headline: str = "") -> None:
        """Auto-merge, pinned to `expected_head` when given: GitHub then refuses to merge a head
        that moved after the approval it was turned on for. `headline` is the merge commit's
        title; without it GitHub squashes a one-commit pull request under that commit's message."""
        params = ["$id: ID!", "$m: PullRequestMergeMethod!"]
        fields = ["pullRequestId: $id", "mergeMethod: $m"]
        values: dict[str, Any] = {"id": node_id, "m": method.upper()}
        if expected_head:
            params.append("$h: GitObjectID!")
            fields.append("expectedHeadOid: $h")
            values["h"] = expected_head
        if headline:
            params.append("$t: String!")
            fields.append("commitHeadline: $t")
            values["t"] = headline
        self.graphql(
            f"mutation({', '.join(params)}) {{"
            f" enablePullRequestAutoMerge(input: {{{', '.join(fields)}}}) {{ clientMutationId }} }}",
            values,
        )

    def disable_auto_merge(self, node_id: str) -> None:
        self.graphql(
            "mutation($id: ID!) { disablePullRequestAutoMerge(input: {pullRequestId: $id})"
            " { clientMutationId } }",
            {"id": node_id},
        )

    def pin_issue(self, node_id: str) -> None:
        """Pin an issue to the top of the issue list (at most three are pinned)."""
        self.graphql(
            "mutation($id: ID!) { pinIssue(input: {issueId: $id}) { issue { number } } }",
            {"id": node_id},
        )

    def mark_ready(self, node_id: str) -> None:
        self.graphql(
            "mutation($id: ID!) { markPullRequestReadyForReview(input: {pullRequestId: $id})"
            " { clientMutationId } }",
            {"id": node_id},
        )

    def merge_pull(self, number: int, method: str, title: str = "") -> dict[str, Any]:
        body: dict[str, Any] = {"merge_method": method}
        if title:
            body["commit_title"] = title
        return self.request("PUT", f"{self._r}/pulls/{int(number)}/merge", body)

    # ------------------------------------------------------------------ contents and refs

    def get_file(self, path: str, ref: str) -> tuple[str | None, str | None]:
        """`(text, blob sha)` of a file on `ref`, or `(None, None)` when it does not exist."""
        quoted = urllib.parse.quote(path)
        try:
            data = self.request("GET", f"{self._r}/contents/{quoted}?ref={urllib.parse.quote(ref)}")
        except GitHubError as exc:
            if exc.status == 404:
                return None, None
            raise
        if not isinstance(data, dict) or data.get("type") != "file":
            return None, None
        text = base64.b64decode(data.get("content", "")).decode("utf-8", "replace")
        return text, data.get("sha")

    def put_file(self, path: str, text: str, *, branch: str, sha: str | None, message: str) -> None:
        """Create or replace a file. A stale `sha` raises GitHubError with status 409 or 422."""
        payload: dict[str, Any] = {
            "message": message,
            "content": base64.b64encode(text.encode("utf-8")).decode("ascii"),
            "branch": branch,
        }
        if sha:
            payload["sha"] = sha
        self.request("PUT", f"{self._r}/contents/{urllib.parse.quote(path)}", payload)

    def branch_sha(self, branch: str) -> str | None:
        try:
            data = self.request("GET", f"{self._r}/git/ref/heads/{urllib.parse.quote(branch)}")
        except GitHubError as exc:
            if exc.status == 404:
                return None
            raise
        return (data.get("object") or {}).get("sha") if isinstance(data, dict) else None

    def create_orphan_branch(self, branch: str, path: str, text: str, message: str) -> None:
        """A new branch whose only commit holds one file."""
        tree = self.request(
            "POST",
            f"{self._r}/git/trees",
            {"tree": [{"path": path, "mode": "100644", "type": "blob", "content": text}]},
        )
        commit = self.request(
            "POST",
            f"{self._r}/git/commits",
            {"message": message, "tree": tree.get("sha", ""), "parents": []},
        )
        self.request(
            "POST", f"{self._r}/git/refs", {"ref": f"refs/heads/{branch}", "sha": commit.get("sha", "")}
        )

    def create_branch(self, branch: str, sha: str) -> None:
        """A branch at `sha` (an existing commit)."""
        self.request("POST", f"{self._r}/git/refs", {"ref": f"refs/heads/{branch}", "sha": sha})

    def delete_branch(self, branch: str) -> None:
        try:
            self.request("DELETE", f"{self._r}/git/refs/heads/{urllib.parse.quote(branch)}")
        except GitHubError as exc:
            if exc.status not in (404, 422):
                raise

    # ------------------------------------------------------------------ actions and checks

    def dispatch_workflow(self, workflow: str, ref: str, inputs: dict[str, str]) -> None:
        self.request(
            "POST",
            f"{self._r}/actions/workflows/{workflow}/dispatches",
            {"ref": ref, "inputs": {k: str(v) for k, v in inputs.items()}},
        )

    def get_run(self, run_id: int | str) -> dict[str, Any]:
        return self.request("GET", f"{self._r}/actions/runs/{run_id}")

    def list_runs(self, workflow: str, *, status: str = "", limit: int = 30) -> list[dict]:
        params: dict[str, Any] = {"per_page": min(limit, 100)}
        if status:
            params["status"] = status
        return self.paginate(
            f"{self._r}/actions/workflows/{workflow}/runs", params, key="workflow_runs", limit=limit
        )

    def list_commits(self, *, author: str = "", limit: int = 1000) -> list[dict]:
        """Commits on the default branch, newest first, by `author` when given."""
        params: dict[str, Any] = {"author": author} if author else {}
        return self.paginate(f"{self._r}/commits", params, limit=limit)

    def runs_for_sha(self, sha: str, limit: int = 30) -> list[dict]:
        return self.paginate(f"{self._r}/actions/runs", {"head_sha": sha}, key="workflow_runs",
                             limit=limit)

    def rerun_failed_jobs(self, run_id: int | str) -> None:
        self.request("POST", f"{self._r}/actions/runs/{run_id}/rerun-failed-jobs")

    def list_jobs(self, run_id: int | str) -> list[dict]:
        return self.paginate(f"{self._r}/actions/runs/{run_id}/jobs", key="jobs")

    def job_log(self, job_id: int | str, tail_chars: int = 6000) -> str:
        """The last `tail_chars` of a job's log. The redirect is followed without the token."""
        opener = urllib.request.build_opener(_NoRedirect)
        req = urllib.request.Request(
            self._url(f"{self._r}/actions/jobs/{job_id}/logs"), headers=self._headers()
        )
        location = ""
        try:
            with opener.open(req, timeout=60) as resp:
                text = resp.read().decode("utf-8", "replace")
                return redact(text[-tail_chars:])
        except urllib.error.HTTPError as exc:
            if exc.code in (301, 302, 303, 307, 308):
                location = exc.headers.get("Location", "")
            else:
                return f"(log unavailable: HTTP {exc.code})"
        except (urllib.error.URLError, TimeoutError) as exc:
            return f"(log unavailable: {exc})"
        if not location:
            return "(log unavailable)"
        try:
            with urllib.request.urlopen(location, timeout=60) as resp:
                text = resp.read().decode("utf-8", "replace")
        except (urllib.error.URLError, TimeoutError) as exc:
            return f"(log unavailable: {exc})"
        return redact(text[-tail_chars:])

    def check_runs(self, sha: str) -> list[dict]:
        return self.paginate(f"{self._r}/commits/{sha}/check-runs", key="check_runs")

    # ------------------------------------------------------------------ settings (operator only)

    def update_repo(self, **fields: Any) -> dict[str, Any]:
        return self.request("PATCH", self._r, fields)

    def get_protection(self, branch: str) -> dict[str, Any] | None:
        try:
            return self.request("GET", f"{self._r}/branches/{branch}/protection")
        except GitHubError as exc:
            if exc.status == 404:
                return None
            raise

    def set_protection(self, branch: str, checks: list[str]) -> None:
        self.request(
            "PUT",
            f"{self._r}/branches/{branch}/protection",
            {
                "required_status_checks": {"strict": False, "contexts": checks},
                "enforce_admins": False,
                "required_pull_request_reviews": None,
                "restrictions": None,
                "allow_force_pushes": False,
                "allow_deletions": False,
            },
        )
