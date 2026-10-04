"""Why a run's rounds failed, written for the person who reviews it.

A build or revision that uses up every review round it is given (`max_review_cycles`) without an
approval is labelled `bot:stuck`, and its comment says why, round by round: what the builder's
session did, which checks were red, what the reviewer said, and which findings kept coming back.
A run that stopped sooner (an unreadable review, a refusal) says its real round count and reason.
"""

from __future__ import annotations

from collections import Counter
from typing import Any, Mapping

#: Findings shown in the summary, and the longest claim or error quoted in a table cell.
TOP_FINDINGS = 5
CELL_CHARS = 140


def _cell(text: Any, limit: int = CELL_CHARS) -> str:
    flat = " ".join(str(text or "").split()).replace("|", "\\|")
    return flat if len(flat) <= limit else flat[:limit - 1].rstrip() + "…"


def _red(gates: list[Mapping[str, Any]]) -> list[Mapping[str, Any]]:
    """The checks this change turned red (not the ones red on main too, nor skipped)."""
    return [g for g in gates or [] if not g.get("ok") and not g.get("pre_existing")
            and not g.get("skipped")]


def _blocking(review: Mapping[str, Any] | None) -> list[Mapping[str, Any]]:
    return [f for f in (review or {}).get("findings") or [] if f.get("severity") == "blocking"]


def _builder_cell(builder: Mapping[str, Any] | None) -> str:
    if not builder:
        return "—"
    what = f"{builder.get('role', 'build')} on `{builder.get('model', '?')}`, " \
           f"{builder.get('minutes', 0)} min"
    if builder.get("timed_out"):
        return f"{what}: **timed out**"
    if not builder.get("ok"):
        return f"{what}: **failed**: {_cell(builder.get('error'), 90)}"
    return what


def _checks_cell(gates: list[Mapping[str, Any]]) -> str:
    if not gates:
        return "—"
    red = _red(gates)
    if not red:
        return "green"
    return "red: " + ", ".join(f"`{g.get('name')}` (exit {g.get('exit_code')})" for g in red)


def _review_cell(review: Mapping[str, Any] | None, self_checks: list[Mapping[str, Any]]) -> str:
    parts = []
    if self_checks:
        flagged = sum(1 for s in self_checks if s.get("flagged"))
        parts.append(f"self check ×{len(self_checks)}, {flagged} flagged")
    if review is None:
        parts.append("no review")
    elif not review.get("readable", True):
        parts.append("**review unreadable**")
    else:
        blocking = _blocking(review)
        if blocking:
            first = blocking[0]
            parts.append(f"blocked, {len(blocking)} finding(s); first `{_cell(first.get('where'), 60)}`: "
                         f"{_cell(first.get('claim'), 90)}")
        else:
            parts.append(str(review.get("verdict") or "approved"))
    return "; ".join(parts)


def stuck(cycles: list[Mapping[str, Any]] | None, limit: int) -> bool:
    """It used every round it was given."""
    return len(cycles or []) >= limit > 0


def why(cycles: list[Mapping[str, Any]] | None, limit: int, reason: str = "") -> str:
    """Markdown: why the rounds failed, a summary and then round by round."""
    cycles = list(cycles or [])
    rounds = len(cycles)
    title = (f"### Why it failed {rounds} times" if stuck(cycles, limit)
             else f"### Why it stopped after {rounds} of {limit} round(s)")
    lines = [title, ""]
    if reason:
        lines.append(f"- **The run's own reason:** {_cell(reason, 300)}.")
    reviews = [c.get("review") for c in cycles if c.get("review") is not None]
    unreadable = sum(1 for r in reviews if not r.get("readable", True))
    blocked = sum(1 for r in reviews if r.get("readable", True) and _blocking(r))
    if reviews:
        lines.append(f"- **The reviewer blocked {blocked} of {len(reviews)} reviewed round(s)**"
                     + (f"; its answer could not be read in {unreadable}" if unreadable else "")
                     + ".")
    red_counts = Counter(g.get("name") for c in cycles for g in _red(c.get("gates") or []))
    red_rounds = sum(1 for c in cycles if _red(c.get("gates") or []))
    if red_rounds:
        lines.append(f"- **Checks were red in {red_rounds} round(s):** "
                     + ", ".join(f"`{name}` ×{n}" for name, n in red_counts.most_common()) + ".")
    builders = [c.get("builder") or {} for c in cycles]
    broken = [b for b in builders if b and (not b.get("ok") or b.get("timed_out"))]
    if broken:
        errors = Counter(_cell(b.get("error") or ("timed out" if b.get("timed_out") else "failed"),
                               160) for b in broken)
        error, times = errors.most_common(1)[0]
        lines.append(f"- **The builder's session failed in {len(broken)} round(s)**, most often: "
                     f"{error}" + (f" (×{times})" if times > 1 else "") + ".")
    minutes = sum(float(b.get("minutes") or 0) for b in builders if b)
    if minutes:
        lines.append(f"- **Builder time:** {minutes:.0f} min over {rounds} round(s).")
    seen = Counter((_cell(f.get("where"), 80), _cell(f.get("claim"))) for r in reviews
                   for f in _blocking(r))
    again = [(key, n) for key, n in seen.most_common() if n > 1][:TOP_FINDINGS]
    if again:
        lines += ["", "**Findings that kept coming back** (the builder never settled them):"]
        lines += [f"- `{where}`: {claim} (×{n})" for (where, claim), n in again]
    elif seen:
        lines += ["", "No finding came back twice: each round's review found something new."]
    if cycles:
        lines += ["", "<details><summary>Round by round</summary>", "",
                  "| Round | Builder | Checks | Review |", "|---|---|---|---|"]
        for c in cycles:
            lines.append(f"| {c.get('n', '?')} | {_builder_cell(c.get('builder'))} "
                         f"| {_checks_cell(c.get('gates') or [])} "
                         f"| {_review_cell(c.get('review'), c.get('self_check') or [])} |")
        lines += ["", "</details>"]
    return "\n".join(lines)
