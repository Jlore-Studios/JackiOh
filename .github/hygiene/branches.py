#!/usr/bin/env python3
"""The weekly hygiene run's branch survey, its two issues, and the deletions a person ticks
(.github/workflows/hygiene.yml, hygiene-delete.yml).

  survey                             every branch, its pull requests and what its bot name points at
  report SURVEY                      the stale branches as checkboxes, for a person to tick
  issue KIND TEMPLATE REPORT GUIDE   open the week's `files` or `instructions` issue for Squishy,
                                     unless that kind's last one is still open
  delete ISSUE                       delete the branches ticked in that issue, after surveying again

A branch is safe to delete when nothing live uses it and deleting it loses nothing that cannot come
back. Its last commit is at least STALE_DAYS old. It is not infrastructure. No open pull request has
it as head, and the issue or pull request its bot name points at is closed. And either a closed or
merged pull request had it as head (that page restores it), or the default branch holds every commit
on it. Anything else that is stale is listed for a person. Standard library only; GitHub through the
`gh` CLI, as `GH_TOKEN` and `REPO` say. `DRY_RUN=true` opens, deletes and edits nothing.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from datetime import datetime, timedelta, timezone
from typing import Any, Callable
from urllib.parse import quote

STALE_DAYS = 14
#: Branches no sweep touches, whatever their age: the deploy and the bots' state.
KEEP = {"production", "staging", "bot-state", "bot-journal", "squishy-state", "squishy-journal"}
#: Prefixes no sweep touches: the AI's training lanes.
KEEP_PREFIXES = ("ai/",)
#: What a bot's branch name points at (bot/README.md): its issue, or the pull request a cut-off
#: revision left its work for.
NAMED = re.compile(r"(?:bot|squishy)/(?:issue-(?P<issue>\d+)|old/issue-(?P<old>\d+)-.+"
                   r"|oneshot-(?P<oneshot>\d+)/.+|wip/(?P<pr>\d+))")
TITLE = "Architecture: weekly hygiene"
#: The two issues a week opens, so each cleanup is a pull request of its own (#888).
KINDS = {"files": "dead files and archives", "instructions": "agent instructions"}
#: A ticked checkbox in the branch section, its branch the first code span; a struck one no longer matches.
TICKED = re.compile(r"^\s*- \[[xX]\] `([^`]+)`", re.M)
LABELS = ("architecture", "difficulty:hard", "squishy:build", "person merges")
#: The issue body's limit is 65,536 characters; the report gives way first.
REPORT_LIMIT = 30_000

QUERY = """
query($owner: String!, $name: String!, $endCursor: String) {
  repository(owner: $owner, name: $name) {
    refs(refPrefix: "refs/heads/", first: 100, after: $endCursor) {
      pageInfo { hasNextPage endCursor }
      nodes {
        name
        target { ... on Commit { oid committedDate messageHeadline } }
        associatedPullRequests(first: 10, states: [OPEN, CLOSED, MERGED]) {
          nodes { number state title }
        }
      }
    }
  }
}"""


def named(name: str) -> int | None:
    """The issue or pull request number a bot's branch name points at, if it is one."""
    m = NAMED.fullmatch(name)
    return int(next(g for g in m.groups() if g)) if m else None


def classify(branch: dict[str, Any], *, default: str, protected: set[str], now: datetime,
             ahead: Callable[[str], int | None]) -> tuple[str, str]:
    """(`keep`, `list` or `candidate`, why). `ahead(name)` is how many commits the branch has that
    the default branch lacks, or None when unknown; it is asked only for a stale branch with no
    pull request."""
    name = branch["name"]
    if name == default or name in protected or name in KEEP or name.startswith(KEEP_PREFIXES):
        return "keep", "infrastructure"
    prs = branch.get("prs") or []
    if any(pr["state"] == "OPEN" for pr in prs):
        return "keep", "an open pull request"
    if branch.get("named_state") == "open":
        return "keep", "the issue or pull request its name points at is open"
    if branch.get("named_state") is None and named(name) is not None:
        return "keep", "what its name points at could not be read"
    age = now - datetime.fromisoformat(branch["date"].replace("Z", "+00:00"))
    if age < timedelta(days=STALE_DAYS):
        return "keep", f"a commit in the last {STALE_DAYS} days"
    if prs:
        return "candidate", "its pull requests are closed or merged; the pull request page restores it"
    if ahead(name) == 0:
        return "candidate", "the default branch holds every commit on it"
    return "list", "no pull request, and commits the default branch does not have"


def gh(*args: str) -> str:
    return subprocess.run(["gh", *args], check=True, capture_output=True, text=True).stdout


def gh_json(*args: str) -> Any:
    return json.loads(gh(*args))


def survey(repo: str, now: datetime) -> dict[str, Any]:
    owner, name = repo.split("/", 1)
    default = gh_json("api", f"repos/{repo}", "--jq", "{d: .default_branch}")["d"]
    pages = gh_json("api", "--paginate", "--slurp", f"repos/{repo}/branches?protected=true&per_page=100")
    protected = {branch["name"] for page in pages for branch in page}
    branches, cursor = [], None
    while True:
        args = ["api", "graphql", "-f", f"query={QUERY}", "-F", f"owner={owner}", "-F", f"name={name}"]
        if cursor:
            args += ["-F", f"endCursor={cursor}"]
        refs = gh_json(*args)["data"]["repository"]["refs"]
        for node in refs["nodes"]:
            commit = node.get("target") or {}
            branches.append({"name": node["name"], "sha": commit.get("oid", ""),
                             "date": commit.get("committedDate", "1970-01-01T00:00:00Z"),
                             "headline": commit.get("messageHeadline", ""),
                             "prs": node["associatedPullRequests"]["nodes"]})
        if not refs["pageInfo"]["hasNextPage"]:
            break
        cursor = refs["pageInfo"]["endCursor"]
    for branch in branches:
        number = named(branch["name"])
        if number is not None:
            try:
                branch["named_state"] = gh_json("api", f"repos/{repo}/issues/{number}",
                                                "--jq", "{s: .state}")["s"]
            except subprocess.CalledProcessError:
                branch["named_state"] = None

    def ahead(branch_name: str) -> int | None:
        try:
            return int(gh("api", f"repos/{repo}/compare/{quote(default, safe='')}..."
                                 f"{quote(branch_name, safe='')}", "--jq", ".ahead_by").strip())
        except (subprocess.CalledProcessError, ValueError):
            return None

    for branch in branches:
        branch["bucket"], branch["why"] = classify(branch, default=default, protected=protected,
                                                   now=now, ahead=ahead)
    return {"repo": repo, "default": default, "at": now.isoformat(), "branches": branches}


def pr_cell(branch: dict[str, Any]) -> str:
    return ", ".join(f"#{pr['number']} ({pr['state'].lower()})" for pr in branch.get("prs") or []) or "none"


def commit_cell(branch: dict[str, Any]) -> str:
    headline = branch["headline"].replace("|", "\\|")[:70]
    return f"{branch['date'][:10]} `{branch['sha'][:7]}` {headline}"


def report(data: dict[str, Any]) -> str:
    """The branch section of the dead-files issue: suggestions for a person to tick, never the builder's."""
    safe = [b for b in data["branches"] if b["bucket"] == "candidate"]
    listed = [b for b in data["branches"] if b["bucket"] == "list"]
    lines = [f"For a person, not the builder. {len(data['branches'])} branches surveyed on {data['at'][:10]}"
             " by the rule in `.github/hygiene/branches.py`; nothing has been deleted. Tick the ones to remove,"
             " then comment `/delete-branches`: `hygiene-delete.yml` surveys again, deletes each ticked branch"
             " the rule does not keep, strikes it here and replies with its sha.", ""]
    if safe:
        lines += [f"**Safe to delete ({len(safe)}).** Its pull request page restores it, or main holds every"
                  " commit on it.", ""]
        lines += [f"- [ ] `{b['name']}` · {commit_cell(b)} · {pr_cell(b)}" for b in safe]
        lines.append("")
    if listed:
        lines += [f"**Have commits main lacks ({len(listed)}).** No pull request: tick one only once nobody"
                  " wants its work.", ""]
        lines += [f"- [ ] `{b['name']}` · {commit_cell(b)}" for b in listed]
    text = "\n".join(lines)
    if len(text) > REPORT_LIMIT:
        text = text[:REPORT_LIMIT] + "\n\n_The list was cut short; the run's survey artifact has all of it._"
    return text


def to_delete(ticked: list[str], data: dict[str, Any]) -> tuple[list[dict[str, Any]], list[tuple[str, str]]]:
    """The ticked branches a fresh survey still lets go, and why each other ticked one stays."""
    found = {b["name"]: b for b in data["branches"]}
    go, skipped = [], []
    for name in dict.fromkeys(ticked):
        branch = found.get(name)
        if branch is None:
            skipped.append((name, "it is already gone"))
        elif branch["bucket"] == "keep":
            skipped.append((name, f"the rule keeps it now: {branch['why']}"))
        else:
            go.append(branch)
    return go, skipped


def strike(body: str, names: list[str]) -> str:
    """The issue body with each deleted branch's checkbox struck, so it is not ticked twice."""
    for name in names:
        body = re.sub(rf"^(\s*- \[[xX]\] )`{re.escape(name)}`(.*)$",
                      lambda m: f"{m.group(1)}~~`{name}`~~ deleted{m.group(2)}", body, count=1, flags=re.M)
    return body


def delete_ticked(repo: str, number: int, now: datetime, *, dry_run: bool) -> str:
    body = gh_json("issue", "view", str(number), "--repo", repo, "--json", "body")["body"]
    ticked = TICKED.findall(body)
    if not ticked:
        return "No branch is ticked, so nothing was deleted."
    go, skipped = to_delete(ticked, survey(repo, now))
    deleted = []
    for branch in go:
        try:
            if not dry_run:
                gh("api", "-X", "DELETE", f"repos/{repo}/git/refs/heads/{quote(branch['name'], safe='/')}")
            deleted.append(branch)
        except subprocess.CalledProcessError as exc:
            skipped.append((branch["name"], (exc.stderr or "").strip()[:120] or "the API refused"))
    lines = [f"Deleted {len(deleted)} of the {len(dict.fromkeys(ticked))} ticked branches"
             f"{' (a dry run: nothing was deleted)' if dry_run else ''}."]
    if deleted:
        lines += ["", "| Branch | Restore with |", "|---|---|"]
        lines += [f"| `{b['name']}` | `git push origin {b['sha']}:refs/heads/{b['name']}` |" for b in deleted]
    if skipped:
        lines += ["", "Kept:"] + [f"- `{name}`: {why}" for name, why in skipped]
    reply = "\n".join(lines) + "\n\n---\n_Posted by `.github/workflows/hygiene-delete.yml`._"
    if not dry_run:
        if deleted:
            gh("issue", "edit", str(number), "--repo", repo, "--body", strike(body, [b["name"] for b in deleted]))
        gh("issue", "comment", str(number), "--repo", repo, "--body", reply)
    return reply


def open_issue(repo: str, kind: str, template: str, report: str, guidance: str, run_url: str,
               today: str, *, dry_run: bool) -> str:
    """Opens the week's issue of `kind`, or says why not: one of each kind is open at a time, since
    its builder reads only the issue body, never a workflow's comment."""
    prefix = f"{TITLE}, {KINDS[kind]}"
    still_open = gh_json("issue", "list", "--repo", repo, "--state", "open", "--search",
                         f'in:title "{prefix}"', "--json", "number,title")
    still_open = [i for i in still_open if i["title"].startswith(prefix)]
    if still_open:
        return f"#{still_open[0]['number']} is still open, so no new {kind} issue this week"
    body = (template.replace("<!-- run -->", run_url).replace("<!-- branches -->", report)
            .replace("<!-- guidance -->", guidance.strip() or "_The guidance step gave nothing this week._"))
    title = f"{prefix}, {today}"
    if dry_run:
        return f"(dry run) would open {title!r}, labelled {', '.join(LABELS)}:\n\n{body}"
    args = ["issue", "create", "--repo", repo, "--title", title, "--body", body]
    for label in LABELS:
        args += ["--label", label]
    return f"opened {gh(*args).strip()}"


def read(path: str) -> str:
    try:
        with open(path, encoding="utf-8") as f:
            return f.read()
    except FileNotFoundError:
        return ""


def main(argv: list[str]) -> int:
    repo = os.environ.get("REPO", "")
    dry_run = os.environ.get("DRY_RUN", "false") == "true"
    command, *rest = argv
    if command == "survey":
        print(json.dumps(survey(repo, datetime.now(timezone.utc)), indent=1))
    elif command == "report":
        print(report(json.loads(read(rest[0]))))
    elif command == "issue":
        print(open_issue(repo, rest[0], read(rest[1]), read(rest[2]), read(rest[3]),
                         os.environ.get("RUN_URL", ""), datetime.now(timezone.utc).date().isoformat(),
                         dry_run=dry_run))
    elif command == "delete":
        print(delete_ticked(repo, int(rest[0]), datetime.now(timezone.utc), dry_run=dry_run))
    else:
        print(__doc__, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
