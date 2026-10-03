"""Starts the night bot's machine when a job is waiting for one of its runners (an AWS Lambda).

EventBridge runs `handler` every five minutes. It reads GitHub's public API (the repository is
public, so no token is needed; `GITHUB_TOKEN`, if set, only raises the rate limit) and calls EC2
for one thing: starting the one instance. Nothing reaches it from outside.

The machine powers itself off after 30 idle minutes (`setup.sh`), so a stopped machine and a
`night-vm-*` job queued in a bot-night run mean a job is waiting for it. A job that has waited
longer than `MAX_WAIT` is left alone: its runner is missing, and starting the machine for it
would only keep it running for nothing until GitHub gives the job up (after 24 hours).

Environment: `INSTANCE_ID`, `REPO_ID` (the repository's numeric id, which survives a rename or
a transfer), optionally `WORKFLOWS` (comma-separated, default `bot-night.yml,triage.yml`) and
`GITHUB_TOKEN`. Deployed by `bot/machine/deploy-starter.sh`.
"""

from __future__ import annotations

import json
import os
import urllib.error
import urllib.request
from datetime import datetime, timedelta, timezone
from typing import Any, Callable

API = "https://api.github.com"
LABEL_PREFIX = "night-vm-"
#: A job waiting for a runner. (`waiting` is a deployment approval, not a runner.)
QUEUED = ("queued", "pending", "requested")
MAX_WAIT = timedelta(hours=3)
#: The workflows whose jobs run on the machine: the night bot's, and triage's Devin call.
DEFAULT_WORKFLOWS = "bot-night.yml,triage.yml"

Get = Callable[[str], dict[str, Any]]


def _github(token: str) -> Get:
    def get(path: str) -> dict[str, Any]:
        request = urllib.request.Request(API + path, headers={
            "Accept": "application/vnd.github+json", "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "jackioh-night-vm-starter"})
        if token:
            request.add_header("Authorization", f"Bearer {token}")
        with urllib.request.urlopen(request, timeout=20) as response:
            return json.load(response)
    return get


def _when(text: str | None) -> datetime | None:
    try:
        return datetime.fromisoformat(str(text).replace("Z", "+00:00")) if text else None
    except ValueError:
        return None


def waiting_jobs(repo_id: str, workflows: list[str], get: Get, now: datetime) -> list[dict]:
    """The jobs of active runs that wait for a runner on the machine."""
    found = []
    seen: set[int] = set()
    for workflow in workflows:
        for status in ("queued", "in_progress"):
            runs = get(f"/repositories/{repo_id}/actions/workflows/{workflow}/runs"
                       f"?status={status}&per_page=30").get("workflow_runs") or []
            for run in runs:
                if run["id"] in seen:
                    continue
                seen.add(run["id"])
                jobs = get(f"/repositories/{repo_id}/actions/runs/{run['id']}/jobs"
                           "?filter=latest&per_page=100").get("jobs") or []
                for job in jobs:
                    labels = [label for label in job.get("labels") or []
                              if label.startswith(LABEL_PREFIX)]
                    since = _when(job.get("created_at"))
                    if job.get("status") in QUEUED and labels and (
                            since is None or now - since < MAX_WAIT):
                        found.append({"run": run["id"], "job": job.get("name"), "label": labels[0]})
    return found


def handler(event: Any = None, context: Any = None) -> dict[str, Any]:
    import boto3  # in the Lambda runtime; the tests drive `waiting_jobs` alone

    ec2 = boto3.client("ec2")
    instance = os.environ["INSTANCE_ID"]
    reservations = ec2.describe_instances(InstanceIds=[instance])["Reservations"]
    state = reservations[0]["Instances"][0]["State"]["Name"]
    if state != "stopped":  # running, or on its way up or down: the next tick looks again
        return {"state": state, "started": False}
    workflows = [w.strip() for w in os.environ.get("WORKFLOWS", DEFAULT_WORKFLOWS).split(",")
                 if w.strip()]
    try:
        waiting = waiting_jobs(os.environ["REPO_ID"], workflows,
                               _github(os.environ.get("GITHUB_TOKEN", "")),
                               datetime.now(timezone.utc))
    except (urllib.error.URLError, TimeoutError, ValueError) as exc:  # rate limit, outage
        print(json.dumps({"github": f"{type(exc).__name__}: {exc}"}))
        return {"state": state, "started": False, "error": str(exc)}
    if not waiting:
        return {"state": state, "started": False}
    ec2.start_instances(InstanceIds=[instance])
    print(json.dumps({"started": instance, "for": waiting}))
    return {"state": state, "started": True, "for": waiting}
