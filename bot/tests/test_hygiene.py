"""The weekly hygiene run's branch rule (.github/hygiene/branches.py, #839): what it deletes once a
model agrees, what it lists for a person, and what it never touches."""

from __future__ import annotations

import importlib.util
import unittest
from datetime import datetime, timedelta, timezone

from harness import config

_spec = importlib.util.spec_from_file_location(
    "hygiene_branches", config.REPO_ROOT / ".github" / "hygiene" / "branches.py")
branches = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(branches)

NOW = datetime(2026, 10, 19, 6, 41, tzinfo=timezone.utc)
OLD = (NOW - timedelta(days=30)).isoformat()
NEW = (NOW - timedelta(days=3)).isoformat()


def branch(name, *, date=OLD, prs=(), named_state="", **rest):
    out = {"name": name, "sha": "a" * 40, "date": date, "headline": "x",
           "prs": [{"number": n, "state": s, "title": "t"} for n, s in prs], **rest}
    if named_state != "":
        out["named_state"] = named_state
    return out


def bucket(b, ahead=None):
    return branches.classify(b, default="main", protected={"release"}, now=NOW,
                             ahead=lambda _name: ahead)[0]


class RuleTests(unittest.TestCase):
    def test_infrastructure_is_never_swept(self):
        for name in ("main", "release", "production", "bot-state", "squishy-journal", "ai/improve"):
            self.assertEqual(bucket(branch(name, prs=[(1, "MERGED")]), ahead=0), "keep", name)

    def test_anything_live_or_recent_stays(self):
        self.assertEqual(bucket(branch("claude/x", prs=[(1, "OPEN"), (2, "CLOSED")])), "keep")
        self.assertEqual(bucket(branch("bot/issue-5", prs=[(9, "MERGED")], named_state="open")), "keep")
        self.assertEqual(bucket(branch("bot/wip/9", named_state=None), ahead=0), "keep")
        self.assertEqual(bucket(branch("claude/x", date=NEW, prs=[(1, "MERGED")])), "keep")

    def test_a_stale_branch_goes_only_when_nothing_is_lost(self):
        self.assertEqual(bucket(branch("bot/issue-5", prs=[(9, "CLOSED")], named_state="closed")),
                         "candidate")
        self.assertEqual(bucket(branch("claude/x"), ahead=0), "candidate")
        self.assertEqual(bucket(branch("claude/x"), ahead=3), "list")
        self.assertEqual(bucket(branch("claude/x"), ahead=None), "list")

    def test_bot_names(self):
        self.assertEqual(branches.named("bot/issue-12"), 12)
        self.assertEqual(branches.named("squishy/wip/40"), 40)
        self.assertEqual(branches.named("bot/old/issue-7-20261001-0102"), 7)
        self.assertEqual(branches.named("squishy/oneshot-3/engine"), 3)
        self.assertIsNone(branches.named("claude/issue-12"))


class VerdictTests(unittest.TestCase):
    CANDIDATES = [branch("a"), branch("b"), branch("c")]

    def test_only_an_explicit_delete_of_a_surveyed_branch_counts(self):
        text = ('Sure:\n[{"branch": "a", "verdict": "delete", "reason": "merged"},'
                ' {"branch": "b", "verdict": "keep", "reason": "draft"},'
                ' {"branch": "main", "verdict": "delete", "reason": "x"}]')
        chosen, reasons = branches.to_delete(self.CANDIDATES, text)
        self.assertEqual([c["name"] for c in chosen], ["a"])
        self.assertEqual(reasons["b"], "draft")

    def test_no_answer_deletes_nothing(self):
        for text in ("", "I could not decide.", "[not json]", '{"branch": "a", "verdict": "delete"}'):
            self.assertEqual(branches.to_delete(self.CANDIDATES, text)[0], [], text)


if __name__ == "__main__":
    unittest.main()
