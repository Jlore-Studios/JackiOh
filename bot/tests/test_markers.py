"""#317 part 2: no path from the model job to a push carries a conflict marker.

Devin committed nested conflict markers in SPEC §11 and the rulings index on #203, #214 and #287,
more than once each: its self checks ran out with them still open, the run ended `built`, and
deliver pushed it. Review runs then spent themselves finding them.
"""

from __future__ import annotations

import json
import unittest

from harness import plan as plan_mod
from harness.config import LABEL_BLOCKED, LABEL_BUILD
from harness.deliver import Deliverer
from harness.runner import FakeRunner

from tests.fakes import git, push_branch
from tests.test_flow import Harness
from tests.test_work import APPROVE, builder, reviewer

MARKED = "<<<<<<< HEAD\nrules v2\n=======\nrules from main\n>>>>>>> origin/main\n"
PLAN = ("<!-- jackioh-bot:plan -->\n## Plan\n1. Edit src/game.txt.\n"
        "<!-- /jackioh-bot:plan -->")


def devin_harness(test):
    h = Harness(test, env={"HARNESS_SECRETS_SET": ""}, machine=("devin",))
    h.gh.add_issue(12, labels=(LABEL_BUILD, "difficulty:easy"), body=PLAN)
    return h


class WorkTests(unittest.TestCase):
    def test_markers_that_survive_the_self_checks_and_one_fix_end_unapproved(self):
        h = devin_harness(self)
        runner = FakeRunner({"build": builder({"src/game.txt": MARKED}),
                             "self_check": reviewer(APPROVE),
                             "fix": builder({"src/game.txt": MARKED + "again\n"})})
        planned, result = h.night(runner)
        self.assertEqual(planned["provider"], "devin")
        self.assertEqual(result["status"], "not_approved")
        self.assertIn("conflict markers are still in src/game.txt", result["reason"])
        self.assertIn("marker_fix", result["cycles"][0])
        self.assertIsNone(h.origin_sha("bot/issue-12"))
        self.assertEqual(h.gh.list_pulls(), [])
        self.assertIn("conflict markers are still in `src/game.txt`", h.gh.bot_comments(12)[-1])

    def test_a_fix_that_clears_them_lets_it_go_to_review(self):
        h = devin_harness(self)
        runner = FakeRunner({"build": builder({"src/game.txt": MARKED}),
                             "self_check": reviewer(APPROVE),
                             "fix": builder({"src/game.txt": "rules v2, with main's\n"})})
        _, result = h.night(runner)
        self.assertEqual(result["status"], "built")
        self.assertEqual(git(h.origin, "show", f"{result['head']}:src/game.txt"),
                         "rules v2, with main's")


class DeliverTests(unittest.TestCase):
    def bundle(self, h, files):
        """A bundle of `bot/issue-12` changing `files`, and the result that names it."""
        planned = plan_mod.make(h.ctx)
        out = h.root / f"out-{len(list(h.root.glob('out-*')))}"
        work = h.root / f"hand-{out.name}"
        git(h.root, "clone", "-q", str(h.origin), str(work))
        git(work, "checkout", "-q", "-b", "bot/issue-12")
        for name, text in files.items():
            (work / name).parent.mkdir(parents=True, exist_ok=True)
            (work / name).write_text(text)
        git(work, "add", "-A")
        git(work, "commit", "-q", "-m", "a change")
        out.mkdir()
        base = git(work, "rev-parse", "origin/main")
        git(work, "bundle", "create", str(out / "branch.bundle"), "refs/heads/bot/issue-12",
            f"^{base}")
        (out / "result.json").write_text(json.dumps({
            "status": "approved", "head": git(work, "rev-parse", "HEAD"), "start": base,
            "bundle": "branch.bundle", "cycles": [], "title": "t"}))
        return planned, out

    def test_deliver_refuses_markers_in_a_file_the_change_touched(self):
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned, out = self.bundle(h, {"src/game.txt": MARKED})
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        self.assertIsNone(h.origin_sha("bot/issue-12"))
        self.assertEqual(h.gh.label_names(12), {LABEL_BLOCKED})
        self.assertIn("conflict markers are still in `src/game.txt`", h.gh.bot_comments(12)[-1])

    def test_a_marker_line_in_a_file_the_change_left_alone_is_not_its_fault(self):
        h = Harness(self)
        push_branch(h.origin, h.root, "main", {"docs/merging.md": MARKED})
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned, out = self.bundle(h, {"src/game.txt": "rules v2\n"})
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        self.assertIsNotNone(h.origin_sha("bot/issue-12"))

if __name__ == "__main__":
    unittest.main()
