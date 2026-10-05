"""The proposer (issue #55, phase 3): context bundle, lenient parsing, strict
validation, retries with error feedback, and the history entry.

The bundle is capped at ``max_context_chars``: over the cap, the oldest
history entries drop first, then the card list trims to the cards referenced
in recent history plus all cards with shadowban or balance flags.

Parsing is lenient (fences and surrounding prose are stripped); validation
against ``schemas/spec.schema.json`` is strict. A parse or validation failure
retries up to ``max_retries`` times, feeding the error back to the model.
"""

from __future__ import annotations

import json
import re
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import yaml
from jsonschema import Draft7Validator

FILE_BLOCK = re.compile(
    r"^--- FILE:\s*(?P<path>.+?)\s*---\s*\n(?P<body>.*?)^--- END FILE ---\s*$",
    re.MULTILINE | re.DOTALL,
)
FENCE = re.compile(r"^```(?:yaml|yml|python|py)?\s*\n(.*?)^```\s*$", re.MULTILINE | re.DOTALL)


class ProposalError(ValueError):
    pass


def _strip_fences(text: str) -> str:
    """Remove code fences, keeping their contents; surrounding prose stays."""
    return FENCE.sub(lambda m: m.group(1), text)


def extract_files(text: str) -> tuple[str, dict[str, str]]:
    """Split delimited code blocks off; return (remainder, {path: body})."""
    files: dict[str, str] = {}

    def take(match: re.Match) -> str:
        path = match.group("path").strip()
        if path in files:
            raise ProposalError(f"duplicate FILE block for {path!r}")
        files[path] = match.group("body")
        return ""

    remainder = FILE_BLOCK.sub(take, text)
    return remainder, files


def parse_spec_yaml(text: str) -> dict[str, Any]:
    """Parse the spec YAML out of free text, tolerating fences and prose.

    Fenced blocks are tried first (the usual shape: prose, a yaml fence, more
    prose), then the whole text as YAML documents.
    """
    candidates = FENCE.findall(text) + [_strip_fences(text)]
    errors = []
    for candidate in candidates:
        try:
            docs = list(yaml.safe_load_all(candidate.strip()))
        except yaml.YAMLError as exc:
            errors.append(str(exc))
            continue
        for doc in docs:
            if isinstance(doc, dict) and "agent" in doc:
                return doc
    if errors:
        raise ProposalError(f"spec is not valid YAML: {errors[0]}")
    raise ProposalError("no spec mapping with an 'agent' key found in the reply")


def load_schema(path: str | Path) -> dict[str, Any]:
    with open(path, encoding="utf-8") as f:
        schema = json.load(f)
    if not isinstance(schema, dict):
        raise ValueError(f"{path} does not hold a schema object")
    return schema


def validate_spec(spec: dict[str, Any], schema: dict[str, Any]) -> None:
    """Validate strictly: every schema violation is a ProposalError."""
    errors = sorted(Draft7Validator(schema).iter_errors(spec), key=lambda e: list(e.path))
    if errors:
        detail = "; ".join(
            f"{'/'.join(str(p) for p in e.path) or '<root>'}: {e.message}" for e in errors
        )
        raise ProposalError(f"spec violates the schema: {detail}")


def check_files(spec: dict[str, Any], files: dict[str, str]) -> None:
    """Every spec-listed file needs exactly one delivered block, and vice versa."""
    wanted = [f["path"] for f in spec.get("files", [])]
    missing = [p for p in wanted if p not in files]
    extra = [p for p in files if p not in wanted]
    problems = [f"missing code for {p!r}" for p in missing] + [f"undocumented file {p!r}" for p in extra]
    if problems:
        raise ProposalError("; ".join(problems))


def parse_proposal(text: str, schema: dict[str, Any]) -> tuple[dict[str, Any], dict[str, str]]:
    """Lenient parse, strict validate: the one function the retry loop calls."""
    remainder, files = extract_files(text)
    spec = parse_spec_yaml(remainder)
    validate_spec(spec, schema)
    check_files(spec, files)
    return spec, files


def build_context(
    interface: str,
    schema_text: str,
    champions: str,
    history: list[str],
    cards: list[str],
    flagged: list[str],
    max_chars: int,
) -> dict[str, Any]:
    """Assemble the capped bundle, dropping oldest history first.

    Returns the context plus what was dropped: the count of history entries
    cut and whether the card list was trimmed (to history-referenced plus
    flagged cards). The interface, schema and champions always survive.
    """
    kept_history = list(history)
    while kept_history and _length(interface, schema_text, champions, kept_history, cards) > max_chars:
        kept_history.pop(0)
    trimmed_cards = False
    if _length(interface, schema_text, champions, kept_history, cards) > max_chars:
        referenced = {c for c in cards if any(c in entry for entry in kept_history)}
        cards = sorted(set(referenced) | set(flagged))
        trimmed_cards = True
    parts = [
        "# Agent interface\n" + interface,
        "# Spec schema\n" + schema_text,
        "# Champions\n" + champions,
        *[f"# History ({i})\n{entry}" for i, entry in enumerate(kept_history)],
        "# Cards\n" + "\n".join(cards),
    ]
    return {
        "context": "\n\n".join(parts),
        "dropped_history": len(history) - len(kept_history),
        "trimmed_cards": trimmed_cards,
    }


def _length(interface: str, schema_text: str, champions: str, history: list[str], cards: list[str]) -> int:
    return len("\n\n".join([interface, schema_text, champions, *history, "\n".join(cards)]))


def propose(
    client: Any,
    system: str,
    context: str,
    schema: dict[str, Any],
    max_tokens: int,
    max_retries: int,
) -> dict[str, Any]:
    """Ask the model, retrying failures with the error fed back.

    ``max_retries`` extra attempts follow the first; then the last error raises.
    """
    messages: list[dict] = [{"role": "user", "content": context}]
    attempts = 0
    while True:
        attempts += 1
        reply = client.complete(system, messages, max_tokens)
        try:
            spec, files = parse_proposal(reply, schema)
            return {"spec": spec, "files": files, "attempts": attempts}
        except ProposalError as exc:
            if attempts > max_retries:
                raise
            messages.append({"role": "assistant", "content": reply})
            messages.append(
                {
                    "role": "user",
                    "content": f"That proposal failed validation with this error:\n{exc}\n"
                    "Reply again with the full corrected proposal (spec plus every file).",
                }
            )


def history_entry(
    provider: str,
    model: str,
    spec: dict[str, Any] | None,
    results: dict[str, Any],
    passed: bool,
    failure_reasons: list[str],
) -> dict[str, Any]:
    """One ``history/proposals.jsonl`` entry: provider, model, summary, results."""
    summary: dict[str, Any] = {}
    if spec is not None:
        agent = spec.get("agent", {})
        summary = {
            "name": spec.get("name"),
            "kind": agent.get("kind"),
            "algorithm": agent.get("algorithm"),
            "specialists": [s.get("name") for s in agent.get("specialists", [])],
            "files": [f.get("path") for f in spec.get("files", [])],
        }
    return {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "provider": provider,
        "model": model,
        "spec": summary,
        "results": results,
        "passed": passed,
        "failure_reasons": failure_reasons,
    }


def append_history(path: str | Path, entry: dict[str, Any]) -> None:
    with open(path, "a", encoding="utf-8") as f:
        f.write(json.dumps(entry) + "\n")
