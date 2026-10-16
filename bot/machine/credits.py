"""Opens an issue when the AWS project's credits run low (an AWS Lambda).

EventBridge runs `handler` every six hours. It reads the plan's remaining credits from the Free
Tier API and, once they are under `FLOOR`, opens one issue on the repository saying so, unless an
issue with its title was ever opened, open or closed, so closing it does not bring it back. On the
Free plan the project closes when its credits run out, and the night and training boxes with it.

Environment: `REPO` (owner/name) and `GITHUB_TOKEN` (a fine-grained token that can read and write
the repository's issues; without it the function only logs). Deployed by
`bot/machine/deploy-credits.sh`.
"""

from __future__ import annotations

import json
import os
import urllib.parse
import urllib.request
from typing import Any, Callable

API = "https://api.github.com"
#: The remaining credits, in dollars, under which the issue opens.
FLOOR = 20.0
TITLE = f"AWS credits are under ${FLOOR:.0f}"
#: People decide what happens next, so no bot takes it.
LABELS = ["human"]

Call = Callable[[str, str, dict | None], Any]


def _github(token: str) -> Call:
    def call(method: str, path: str, body: dict | None = None) -> Any:
        request = urllib.request.Request(
            API + path, method=method,
            data=json.dumps(body).encode() if body is not None else None,
            headers={"Accept": "application/vnd.github+json",
                     "X-GitHub-Api-Version": "2022-11-28",
                     "User-Agent": "jackioh-credits-watch",
                     "Authorization": f"Bearer {token}"})
        with urllib.request.urlopen(request, timeout=20) as response:
            return json.load(response)
    return call


def credits(state: dict) -> float | None:
    """The remaining credits in the plan state `GetAccountPlanState` returns, if it names any."""
    amount = (state.get("accountPlanRemainingCredits") or {}).get("amount")
    return None if amount is None else float(amount)


def issue(state: dict) -> dict | None:
    """The issue to open for the plan state, or None while the credits last."""
    amount = credits(state)
    if amount is None or amount >= FLOOR:
        return None
    plan = state.get("accountPlanType", "unknown")
    ends = str(state.get("accountPlanExpirationDate", "unknown"))[:10]
    body = (
        f"The AWS project has **${amount:.2f}** in credits left (plan `{plan}`, ending {ends}). "
        "On the Free plan, AWS closes the project when its credits run out, and the night box "
        "and the training box stop with it.\n\n"
        "A person decides what happens next:\n\n"
        "- upgrade the project to the paid plan in AWS Settings (settings.aws.com → Billing), "
        "so the boxes keep running and are billed once the credits are gone; or\n"
        "- let the credits run out, and stop the boxes first.\n\n"
        "Opened once by the credits watch (`bot/machine/credits.py`), which checks every six "
        "hours. Closing this issue does not bring it back.")
    return {"title": TITLE, "body": body, "labels": LABELS}


def already_opened(repo: str, call: Call) -> bool:
    """Whether an issue with the title was ever opened (search matches loosely, so the title is
    compared exactly)."""
    query = urllib.parse.quote(f'repo:{repo} is:issue in:title "{TITLE}"')
    found = call("GET", f"/search/issues?q={query}&per_page=100", None)
    return any(item.get("title") == TITLE for item in found.get("items") or [])


def handler(event: Any = None, context: Any = None) -> dict[str, Any]:
    import boto3  # in the Lambda runtime; the tests drive `issue` and `already_opened` alone

    state = boto3.client("freetier").get_account_plan_state()
    amount = credits(state)
    wanted = issue(state)
    if wanted is None:
        return {"credits": amount, "opened": False}
    token = os.environ.get("GITHUB_TOKEN", "")
    if not token:
        print(json.dumps({"credits": amount, "github": "no GITHUB_TOKEN, so no issue"}))
        return {"credits": amount, "opened": False}
    repo = os.environ["REPO"]
    call = _github(token)
    if already_opened(repo, call):
        return {"credits": amount, "opened": False}
    created = call("POST", f"/repos/{repo}/issues", wanted)
    print(json.dumps({"credits": amount, "opened": created.get("html_url")}))
    return {"credits": amount, "opened": True, "url": created.get("html_url")}
