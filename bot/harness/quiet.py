"""Start only when the Claude subscription is quiet: nobody else is spending it.

The bot shares one subscription with the person who owns it and with a partner bot
(bright-bots-harness). Before a run claims work, it reads the subscription's usage with the
smallest model call there is, waits `interval_minutes`, and reads it again. Usage that did not
rise means nobody else is working, so the run goes ahead. Usage that rose while the partner was
in one of its spending steps is the partner's, and does not hold the bot back. Any other rise is
a person or another agent, so it waits and looks again, up to `max_wait_minutes`, then gives up
until the next run. A forced run skips all of this. The partner implements the same rule.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timedelta
from typing import Any, Callable

from harness.clock import iso, parse_iso
from harness.config import Partner, Quiet
from harness.errors import GitHubError

#: Utilization is reported to the hundredth; anything under this is the same reading.
EPSILON = 0.001


@dataclass(frozen=True)
class Sample:
    at: datetime
    five_hour: float | None
    seven_day: float | None
    five_resets: str | None
    seven_resets: str | None
    status: str

    @classmethod
    def of(cls, usage: dict | None, at: datetime) -> "Sample | None":
        if not isinstance(usage, dict):
            return None
        def window(name: str) -> tuple[float | None, str | None]:
            reading = usage.get(name)
            if not isinstance(reading, dict):
                return None, None
            value = reading.get("utilization")
            return (float(value) if isinstance(value, (int, float)) else None,
                    reading.get("resets_at"))
        five, five_resets = window("five_hour")
        seven, seven_resets = window("seven_day")
        if five is None and seven is None:
            return None
        return cls(at, five, seven, five_resets, seven_resets, str(usage.get("status", "")))

    def to_dict(self) -> dict[str, Any]:
        return {"at": iso(self.at), "five_hour": self.five_hour, "seven_day": self.seven_day,
                "five_resets": self.five_resets, "status": self.status}


def compare(a: Sample, b: Sample) -> str:
    """`quiet`, `rose`, `inconclusive` (a window turned over between the two) or `refused`."""
    if b.status == "rejected":
        return "refused"
    if a.five_resets != b.five_resets or a.five_hour is None or b.five_hour is None:
        return "inconclusive"
    if b.five_hour > a.five_hour + EPSILON:
        return "rose"
    if (a.seven_day is not None and b.seven_day is not None and a.seven_resets == b.seven_resets
            and b.seven_day > a.seven_day + EPSILON):
        return "rose"
    return "quiet"


def _overlaps(step: dict, t1: datetime, t2: datetime) -> bool:
    if step.get("conclusion") == "skipped":
        return False  # GitHub skipped it, so it spent nothing
    started = parse_iso(step.get("started_at"))
    if started is None or started > t2:
        return False
    ended = parse_iso(step.get("completed_at"))
    return ended is None or ended >= t1


def partner_spending(partners: tuple[Partner, ...], client_for: Callable[[str], Any],
                     t1: datetime, t2: datetime) -> str | None:
    """Which partner step spent the subscription at some point in [t1, t2], or None.

    Reads each partner's recent runs and their jobs' steps. An error reading a partner counts as
    not spending, so a rise is never excused on a guess."""
    for partner in partners:
        try:
            gh = client_for(partner.repo)
            for workflow, prefixes in partner.workflows.items():
                for run in gh.list_runs(workflow, limit=10):
                    updated = parse_iso(run.get("updated_at"))
                    if run.get("status") == "completed" and (updated is None or updated < t1):
                        continue
                    for job in gh.list_jobs(run.get("id")):
                        for step in job.get("steps") or []:
                            name = str(step.get("name", ""))
                            if any(name.startswith(p) for p in prefixes) and _overlaps(step, t1, t2):
                                return f"{partner.repo} {workflow}: {name}"
        except GitHubError:
            continue
    return None


@dataclass
class Verdict:
    quiet: bool
    reason: str
    samples: list[dict[str, Any]] = field(default_factory=list)
    excused_by: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {"quiet": self.quiet, "reason": self.reason, "samples": self.samples,
                "excused_by": self.excused_by}


def wait_for_quiet(settings: Quiet, ping: Callable[[], dict | None],
                   partner: Callable[[datetime, datetime], str | None],
                   now: Callable[[], datetime], sleep: Callable[[float], None]) -> Verdict:
    """Sample, wait an interval, sample again, until the subscription is quiet or the wait is up."""
    interval = timedelta(minutes=settings.interval_minutes)
    deadline = now() + timedelta(minutes=settings.max_wait_minutes)
    first = Sample.of(ping(), now())
    if first is None:
        # No reading at all: nothing may depend on the usage signal being there, so the run goes
        # ahead (bright-bots-harness does the same) rather than waiting every night for ever.
        return Verdict(True, "the CLI gave no usage reading, so quiet cannot be told; going ahead")
    samples = [first.to_dict()]
    previous = first
    while now() + interval <= deadline + timedelta(seconds=30):
        sleep(interval.total_seconds())
        current = Sample.of(ping(), now())
        if current is None:
            continue  # this pair cannot be compared; the next reading is compared with `previous`
        samples.append(current.to_dict())
        verdict = compare(previous, current)
        if verdict == "quiet":
            return Verdict(True, f"usage held for {settings.interval_minutes} minutes", samples)
        if verdict == "refused":
            return Verdict(False, "the subscription refused the call (its limit is reached)",
                           samples)
        if verdict == "rose":
            spender = partner(previous.at, current.at)
            if spender:
                return Verdict(True, "usage rose, but only while the partner bot was spending",
                               samples, excused_by=spender)
        previous = current
    return Verdict(False, f"someone else kept using the subscription for "
                          f"{settings.max_wait_minutes} minutes", samples)


class PartnerReader:
    """Reads a partner's public Actions runs: with the job's token first, then without one,
    since a job's own token may not reach another repository."""

    def __init__(self, repo: str, token: str, factory: Callable[..., Any] | None = None) -> None:
        from harness.gh import GitHub

        make = factory or GitHub
        self.clients = [make(repo, token)] if token else []
        self.clients.append(make(repo, ""))

    def _call(self, name: str, *args: Any, **kwargs: Any) -> Any:
        last: GitHubError | None = None
        for client in self.clients:
            try:
                return getattr(client, name)(*args, **kwargs)
            except GitHubError as exc:
                if exc.status not in (401, 403, 404):
                    raise
                last = exc
        raise last or GitHubError(f"{name}: no client", 0)

    def list_runs(self, workflow: str, limit: int = 10) -> list[dict]:
        return self._call("list_runs", workflow, limit=limit)

    def list_jobs(self, run_id: Any) -> list[dict]:
        return self._call("list_jobs", run_id)
