"""The weekly hygiene run's branch rule (.github/hygiene/branches.py, #839, #888): what it offers a
person to tick, what it never touches, and how a `/delete-branches` comment deletes only the ticked
branches a fresh survey still lets go."""

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


class ReportTests(unittest.TestCase):
    def survey(self, *found):
        return {"at": NOW.isoformat(), "branches": list(found)}

    def test_each_stale_branch_is_a_box_to_tick(self):
        text = branches.report(self.survey(
            dict(branch("bot/issue-5", prs=[(9, "MERGED")]), bucket="candidate", why="restorable"),
            dict(branch("claude/lost"), bucket="list", why="unique commits"),
            dict(branch("main"), bucket="keep", why="infrastructure")))
        self.assertIn("- [ ] `bot/issue-5` · ", text)
        self.assertIn("- [ ] `claude/lost` · ", text)
        self.assertNotIn("`main`", text)
        self.assertIn("comment `/delete-branches`", text)
        self.assertEqual(branches.TICKED.findall(text), [])

    def test_no_stale_branch_offers_no_box(self):
        text = branches.report(self.survey(dict(branch("main"), bucket="keep", why="infrastructure")))
        self.assertNotIn("- [ ]", text)


class DeleteTests(unittest.TestCase):
    BODY = ("intro\n- [x] `bot/issue-5` · 2026-09-01 `aaaaaaa` x · #9 (merged)\n"
            "- [ ] `claude/same` · 2026-09-01 `aaaaaaa` x\n"
            "  - [X] `claude/lost` · 2026-09-01 `aaaaaaa` x\n"
            "- [x] ~~`bot/issue-3`~~ deleted · gone\n- [x] `claude/now-open` · x\n- [x] `gone/branch` · x\n")

    def test_only_ticked_and_not_yet_deleted_boxes_count(self):
        self.assertEqual(branches.TICKED.findall(self.BODY),
                         ["bot/issue-5", "claude/lost", "claude/now-open", "gone/branch"])

    def test_a_fresh_survey_decides_what_goes(self):
        data = {"branches": [dict(branch("bot/issue-5"), bucket="candidate", why="restorable"),
                             dict(branch("claude/lost"), bucket="list", why="unique commits"),
                             dict(branch("claude/now-open"), bucket="keep", why="an open pull request")]}
        go, skipped = branches.to_delete(branches.TICKED.findall(self.BODY), data)
        self.assertEqual([b["name"] for b in go], ["bot/issue-5", "claude/lost"])
        self.assertEqual(skipped, [("claude/now-open", "the rule keeps it now: an open pull request"),
                                   ("gone/branch", "it is already gone")])

    def test_a_deleted_branch_is_struck_and_not_ticked_again(self):
        body = branches.strike(self.BODY, ["bot/issue-5", "claude/lost"])
        self.assertIn("- [x] ~~`bot/issue-5`~~ deleted · 2026-09-01", body)
        self.assertIn("  - [X] ~~`claude/lost`~~ deleted · ", body)
        self.assertIn("- [ ] `claude/same` · ", body)
        self.assertEqual(branches.TICKED.findall(body), ["claude/now-open", "gone/branch"])


if __name__ == "__main__":
    unittest.main()
