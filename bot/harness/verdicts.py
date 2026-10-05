"""Reading the one-line JSON header each model call opens its final message with.

`<!-- bot: {...} -->` from a builder, `<!-- review: {...} -->` from the reviewer and
`<!-- suggestions: [...] -->` from the suggestion survey. The header is searched for anywhere in
the message, so a model that wrote a sentence first is still read; the text after it is the
report a person sees.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass, field
from typing import Any

MAX_FINDINGS = 20


def header(text: str, tag: str) -> tuple[Any, str]:
    """`(parsed JSON or None, the text after the header)`."""
    source = str(text or "")
    match = re.search(r"<!--\s*" + re.escape(tag) + r"\s*:\s*", source)
    if not match:
        return None, source.strip()
    tail = source[match.end():]
    try:
        value, used = json.JSONDecoder(strict=False).raw_decode(tail)
    except ValueError:
        end = tail.find("-->")
        return None, (tail[end + 3:] if end >= 0 else tail).strip()
    rest = tail[used:]
    close = rest.find("-->")
    rest = rest[close + 3:] if close >= 0 else rest
    return value, rest.strip()


@dataclass
class BuildReport:
    status: str  # "done" | "blocked" | "unknown"
    title: str
    question: str
    body: str


def build_report(text: str) -> BuildReport:
    value, rest = header(text, "bot")
    if not isinstance(value, dict):
        return BuildReport("unknown", "", "", rest)
    status = str(value.get("status", "")).lower()
    if status not in ("done", "blocked"):
        status = "unknown"
    title = " ".join(str(value.get("title", "")).split())[:200]
    question = str(value.get("question", "")).strip()[:4000]
    return BuildReport(status, title, question, rest)


@dataclass
class Finding:
    severity: str
    where: str
    claim: str
    evidence: str

    def to_dict(self) -> dict:
        return {"severity": self.severity, "where": self.where, "claim": self.claim,
                "evidence": self.evidence}

    def markdown(self) -> str:
        evidence = f" Evidence: {self.evidence}" if self.evidence else ""
        return f"- **{self.severity}** `{self.where}`: {self.claim}{evidence}"


@dataclass
class Review:
    readable: bool
    verdict: str  # "approve" | "changes" | "unreadable"
    findings: list[Finding] = field(default_factory=list)
    body: str = ""
    reviewed_sha: str = ""
    #: The reviewer's call failed (`RunResult.error`), so there was no answer to read.
    error: str = ""

    @property
    def blocking(self) -> list[Finding]:
        return [f for f in self.findings if f.severity == "blocking"]

    @property
    def notes(self) -> list[Finding]:
        return [f for f in self.findings if f.severity != "blocking"]

    @property
    def approved(self) -> bool:
        """No blocking finding in a readable review. A `changes` verdict that names nothing
        blocking has nothing for a builder to fix, so it counts as approval too."""
        return self.readable and not self.blocking

    def to_dict(self) -> dict:
        found = {"verdict": self.verdict, "readable": self.readable,
                 "findings": [f.to_dict() for f in self.findings], "body": self.body[:20000]}
        if self.error:
            found["error"] = self.error[:2000]
        return found

    @property
    def why_unreadable(self) -> str:
        """Why there was no verdict, for a reason or a finding."""
        if self.error:
            return f"the reviewer's call failed: {self.error[:500]}"
        return "the reviewer's answer could not be read twice in a row"


def review(text: str) -> Review:
    value, rest = header(text, "review")
    if not isinstance(value, dict):
        return Review(False, "unreadable", [], rest)
    verdict = str(value.get("verdict", "")).lower()
    raw = value.get("findings") or []
    findings: list[Finding] = []
    if isinstance(raw, list):
        for entry in raw[:MAX_FINDINGS]:
            if not isinstance(entry, dict):
                continue
            severity = str(entry.get("severity", "blocking")).lower()
            findings.append(Finding(
                "blocking" if severity != "note" else "note",
                str(entry.get("where", "?"))[:300],
                str(entry.get("claim", "")).strip()[:2000],
                str(entry.get("evidence", "")).strip()[:2000],
            ))
    if verdict not in ("approve", "changes"):
        return Review(False, "unreadable", findings, rest)
    return Review(True, verdict, findings, rest)


@dataclass
class Suggestion:
    title: str
    body: str


def suggestions(text: str, limit: int) -> list[Suggestion]:
    value, _ = header(text, "suggestions")
    if not isinstance(value, list):
        return []
    found: list[Suggestion] = []
    for entry in value:
        if not isinstance(entry, dict):
            continue
        title = " ".join(str(entry.get("title", "")).split())[:200]
        body = str(entry.get("body", "")).strip()
        if title and body:
            found.append(Suggestion(title, body))
        if len(found) >= limit:
            break
    return found
