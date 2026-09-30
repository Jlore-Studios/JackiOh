"""What the model is told about an issue or a pull request, assembled from GitHub.

Only people the trust file names at level 1 or above are quoted, apart from the issue's own body
(someone trusted chose to build it). The bot's own comments are left out. Everything is returned
already fenced as data.
"""

from __future__ import annotations

import re
from typing import Any

from harness.clock import parse_iso
from harness.config import MARKER
from harness.prompts import data
from harness.trust import Trust

MAX_CHARS = 60_000
_CLOSES = re.compile(r"(?i)\b(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?)\s+#(\d+)\b")


def _is_bot(entry: dict[str, Any], bot_login: str) -> bool:
    login = str((entry.get("user") or {}).get("login", "")).lower()
    return login in (bot_login.lower(), "github-actions[bot]") or MARKER in str(entry.get("body") or "")


def _trusted(entry: dict[str, Any], trust: Trust) -> bool:
    user = entry.get("user") or {}
    return trust.level(user.get("login", ""), user.get("id"), entry.get("author_association")) >= 1


def _clip(text: str) -> str:
    return text if len(text) <= MAX_CHARS else text[:MAX_CHARS] + "\n\n(cut here: too long)"


def issue_thread(gh: Any, trust: Trust, number: int, bot_login: str) -> str:
    issue = gh.get_issue(number)
    author = (issue.get("user") or {}).get("login", "?")
    parts = [f"# #{number}: {issue.get('title', '')}", f"Opened by @{author}.", "",
             str(issue.get("body") or "(no description)")]
    skipped = 0
    for comment in gh.list_comments(number):
        if _is_bot(comment, bot_login):
            continue
        if not _trusted(comment, trust):
            skipped += 1
            continue
        who = (comment.get("user") or {}).get("login", "?")
        parts += ["", f"---\nComment by @{who} at {comment.get('created_at', '')}:", "",
                  str(comment.get("body") or "")]
    if skipped:
        parts += ["", f"({skipped} comment(s) from people outside the trust list were left out.)"]
    return data(_clip("\n".join(parts)), f"Issue #{number}")


def linked_issue(pull: dict[str, Any]) -> int | None:
    match = _CLOSES.search(str(pull.get("body") or ""))
    return int(match.group(1)) if match else None


def pull_text(pull: dict[str, Any]) -> str:
    body = str(pull.get("body") or "(no description)")
    body = body.replace(MARKER, "")
    text = f"# PR #{pull.get('number')}: {pull.get('title', '')}\n\n{body}"
    return data(_clip(text), f"Pull request #{pull.get('number')}")


def pull_feedback(gh: Any, trust: Trust, number: int, bot_login: str, since: str | None,
                  issue: int | None = None) -> str:
    """Trusted comments, reviews and line comments on a PR newer than `since`, and trusted
    comments on the issue it closes, since a request made there is routed to the PR."""
    cutoff = parse_iso(since)
    entries: list[tuple[str, str]] = []
    if issue:
        try:
            for comment in gh.list_comments(issue):
                at = parse_iso(comment.get("created_at"))
                if _is_bot(comment, bot_login) or not _trusted(comment, trust):
                    continue
                if cutoff is None or at is None or at > cutoff:
                    who = comment["user"]["login"]
                    entries.append((comment.get("created_at", ""),
                                    f"Comment by @{who} on issue #{issue}:\n\n{comment.get('body') or ''}"))
        except Exception:  # noqa: BLE001 - the PR's own feedback still goes through
            pass

    def newer(entry: dict[str, Any], key: str) -> bool:
        at = parse_iso(entry.get(key))
        return cutoff is None or at is None or at > cutoff

    for comment in gh.list_comments(number):
        if not _is_bot(comment, bot_login) and _trusted(comment, trust) and newer(comment, "created_at"):
            who = comment["user"]["login"]
            entries.append((comment.get("created_at", ""),
                            f"Comment by @{who}:\n\n{comment.get('body') or ''}"))
    for review in gh.list_reviews(number):
        body = str(review.get("body") or "").strip()
        state = str(review.get("state", ""))
        if _is_bot(review, bot_login) or not _trusted(review, trust):
            continue
        if not newer(review, "submitted_at") or (not body and state != "CHANGES_REQUESTED"):
            continue
        who = review["user"]["login"]
        entries.append((review.get("submitted_at", ""),
                        f"Review by @{who} ({state.lower().replace('_', ' ')}):\n\n{body}"))
    for line in gh.list_review_comments(number):
        if _is_bot(line, bot_login) or not _trusted(line, trust) or not newer(line, "created_at"):
            continue
        who = line["user"]["login"]
        where = f"{line.get('path', '?')}:{line.get('line') or line.get('original_line') or '?'}"
        entries.append((line.get("created_at", ""),
                        f"Line comment by @{who} on {where}:\n\n{line.get('body') or ''}"))
    entries.sort(key=lambda pair: pair[0])
    if not entries:
        return data("(no new comments from trusted people)", f"Feedback on #{number}")
    text = "\n\n---\n\n".join(body for _, body in entries)
    return data(_clip(text), f"Feedback on #{number}")
