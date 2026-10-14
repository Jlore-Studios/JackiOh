#!/usr/bin/env python3
"""The weekly hygiene run's branch sweep and its issue (.github/workflows/hygiene.yml).

  survey                        every branch, its pull requests and what its bot name points at
  candidates SURVEY             the survey's candidates alone, for the model's check
  apply SURVEY VERDICT          delete the candidates the model also said to delete; print a report
  issue TEMPLATE REPORT GUIDE   open the week's issue for Squishy, unless one is still open

A branch is a candidate when nothing live uses it and deleting it loses nothing that cannot come
back. Its last commit is at least STALE_DAYS old. It is not infrastructure. No open pull request has
it as head, and the issue or pull request its bot name points at is closed. And either a closed or
merged pull request had it as head (that page restores it), or the default branch holds every commit
on it. Anything else that is stale is listed for a person. Standard library only; GitHub through the
`gh` CLI, as `GH_TOKEN` and `REPO` say. `DRY_RUN=true` deletes and opens nothing.
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
LABELS = ("architecture", "difficulty:hard", "squishy:build")
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


def to_delete(candidates: list[dict[str, Any]], verdict_text: str) -> tuple[list[dict], dict]:
    """The candidates the model also said to delete, and the model's reasons by branch. Only names
    from the survey, and only an explicit `delete`: a missing, malformed or unsure answer keeps the
    branch."""
    reasons: dict[str, str] = {}
    match = re.search(r"\[.*\]", verdict_text, re.DOTALL)
    try:
        answers = json.loads(match.group(0)) if match else []
    except json.JSONDecodeError:
        answers = []
    said = set()
    for answer in answers if isinstance(answers, list) else []:
        if not isinstance(answer, dict) or not isinstance(answer.get("branch"), str):
            continue
        reasons[answer["branch"]] = str(answer.get("reason", ""))
        if answer.get("verdict") == "delete":
            said.add(answer["branch"])
    return [c for c in candidates if c["name"] in said], reasons


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


def apply(data: dict[str, Any], verdict_text: str, *, dry_run: bool) -> str:
    repo = data["repo"]
    candidates = [b for b in data["branches"] if b["bucket"] == "candidate"]
    listed = [b for b in data["branches"] if b["bucket"] == "list"]
    chosen, reasons = to_delete(candidates, verdict_text)
    deleted, failed = [], []
    for branch in chosen:
        ref = f"repos/{repo}/git/refs/heads/{quote(branch['name'], safe='/')}"
        try:
            if gh_json("api", ref, "--jq", "{s: .object.sha}")["s"] != branch["sha"]:
                failed.append((branch, "it moved since the survey"))
                continue
            if not dry_run:
                gh("api", "-X", "DELETE", ref)
            deleted.append(branch)
        except subprocess.CalledProcessError as exc:
            failed.append((branch, (exc.stderr or "").strip()[:120] or "the API refused"))
    kept = [b for b in candidates if b not in chosen]
    lines = [f"Swept {len(data['branches'])} branches on {data['at'][:10]}"
             f"{' (a dry run: nothing was deleted)' if dry_run else ''}. A branch is deleted only when"
             f" the rule in `.github/hygiene/branches.py` and a model's check both say so.", ""]
    lines += [f"**Deleted ({len(deleted)}).** Restore one from its pull request's page, or with"
              " `git push origin <sha>:refs/heads/<branch>`.", ""]
    if deleted:
        lines += ["| Branch | Last commit | Pull requests | Full sha |", "|---|---|---|---|"]
        lines += [f"| `{b['name']}` | {commit_cell(b)} | {pr_cell(b)} | `{b['sha']}` |" for b in deleted]
        lines.append("")
    if kept or failed:
        lines += [f"**Candidates kept ({len(kept) + len(failed)}).**", "", "| Branch | Why |", "|---|---|"]
        lines += [f"| `{b['name']}` | the model kept it: {reasons.get(b['name'], 'no answer')} |" for b in kept]
        lines += [f"| `{b['name']}` | not deleted: {why} |" for b, why in failed]
        lines.append("")
    if listed:
        lines += [f"**Stale, for a person ({len(listed)}).** No pull request, and commits the default"
                  " branch lacks: delete each one by hand once it is not wanted.", "",
                  "| Branch | Last commit |", "|---|---|"]
        lines += [f"| `{b['name']}` | {commit_cell(b)} |" for b in listed]
    report = "\n".join(lines)
    if len(report) > REPORT_LIMIT:
        report = report[:REPORT_LIMIT] + "\n\n_The report was cut short; the run's log has all of it._"
    return report


def open_issue(repo: str, template: str, report: str, guidance: str, run_url: str, today: str, *,
               dry_run: bool) -> str:
    """Opens the week's issue, or says why not: one hygiene issue is open at a time, since its
    builder reads only the issue body, never a workflow's comment."""
    still_open = gh_json("issue", "list", "--repo", repo, "--state", "open", "--search",
                         f'in:title "{TITLE}"', "--json", "number,title")
    still_open = [i for i in still_open if i["title"].startswith(TITLE)]
    if still_open:
        return f"#{still_open[0]['number']} is still open, so no new issue this week"
    body = (template.replace("<!-- run -->", run_url).replace("<!-- branches -->", report)
            .replace("<!-- guidance -->", guidance.strip() or "_The guidance step gave nothing this week._"))
    title = f"{TITLE}, {today}"
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
    elif command == "candidates":
        data = json.loads(read(rest[0]))
        print(json.dumps([{k: b.get(k) for k in ("name", "date", "headline", "prs", "why")}
                          for b in data["branches"] if b["bucket"] == "candidate"], indent=1))
    elif command == "apply":
        print(apply(json.loads(read(rest[0])), read(rest[1]), dry_run=dry_run))
    elif command == "issue":
        print(open_issue(repo, read(rest[0]), read(rest[1]), read(rest[2]),
                         os.environ.get("RUN_URL", ""), datetime.now(timezone.utc).date().isoformat(),
                         dry_run=dry_run))
    else:
        print(__doc__, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
