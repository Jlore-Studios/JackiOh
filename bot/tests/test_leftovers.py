"""#317 part 12, what was left of #160: the state file pruned, fix passes at their own effort, and
`main` merged again before a review when it moved and merges cleanly."""

from __future__ import annotations

import unittest
from datetime import timedelta

from harness import plan as plan_mod
from harness.clock import iso
from harness.config import LABEL_BUILD
from harness.runner import FakeRunner
from harness.state import item as state_item

from tests.fakes import FakeGitHub, git, push_branch
from tests.support import NIGHT, make_ctx
from tests.test_flow import Harness
from tests.test_work import APPROVE, builder, changes, reviewer


class PruneTests(unittest.TestCase):
    def test_closed_items_are_dropped_after_a_week_and_lightened_before(self):
        gh = FakeGitHub()
        ctx = make_ctx(gh)
        gh.add_issue(1, state="closed")
        gh.threads[1]["closed_at"] = iso(NIGHT - timedelta(days=8))
        gh.add_issue(2, state="closed")
        gh.threads[2]["closed_at"] = iso(NIGHT - timedelta(days=1))
        gh.add_issue(3)
        ctx.store.update(lambda s: [state_item(s, n).update(handoff={"notes": "x" * 9000},
                                                            failures=1) for n in (1, 2, 3)])
        notes = plan_mod.prune(ctx, ctx.store.load())
        self.assertEqual(notes, ["pruned the state file: dropped 1 closed item(s), lightened 1"])
        items = ctx.store.load()["items"]
        self.assertNotIn("1", items)
        self.assertEqual(items["2"], {"failures": 1})
        self.assertIn("handoff", items["3"])  # open: untouched
        # Once a day.
        gh.threads[3]["state"] = "closed"
        self.assertEqual(plan_mod.prune(ctx, ctx.store.load()), [])


class FixEffortTests(unittest.TestCase):
    def test_a_fix_pass_runs_at_the_fix_effort(self):
        h = Harness(self, machine=())
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                             "fix": builder({"src/game.txt": "v3\n"}),
                             "review": [reviewer(changes("Not yet.")), reviewer(APPROVE)]})
        h.night(runner)
        efforts = {c.role: c.effort for c in runner.calls}
        self.assertEqual((efforts["build"], efforts["fix"], efforts["review"]),
                         ("xhigh", "high", "xhigh"))


class CatchUpTests(unittest.TestCase):
    def test_main_that_moved_during_the_build_is_merged_before_the_review(self):
        h = Harness(self, machine=())
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        moved = {}

        def building(request):
            moved["main"] = push_branch(h.origin, h.root, "main", {"README.md": "news\n"})
            return builder({"src/game.txt": "v2\n"})(request)
        seen = {}

        def reviewing(request):
            seen["readme"] = (request.cwd / "README.md").read_text()
            return reviewer(APPROVE)(request)
        _, result = h.night(FakeRunner({"build": building, "review": reviewing}))
        self.assertEqual(result["status"], "approved")
        self.assertEqual(result["cycles"][0]["caught_up"], moved["main"])
        self.assertEqual(seen["readme"], "news\n")
        head = h.origin_sha("bot/issue-12")
        self.assertEqual(git(h.origin, "merge-base", "--is-ancestor", moved["main"], head), "")

    def test_a_main_that_would_conflict_is_left_for_the_conflict_path(self):
        h = Harness(self, machine=())
        h.gh.add_issue(12, labels=(LABEL_BUILD,))

        def building(request):
            push_branch(h.origin, h.root, "main", {"src/game.txt": "rules from main\n"})
            return builder({"src/game.txt": "v2\n"})(request)
        _, result = h.night(FakeRunner({"build": building, "review": reviewer(APPROVE)}))
        self.assertEqual(result["status"], "approved")
        self.assertNotIn("caught_up", result["cycles"][0])


if __name__ == "__main__":
    unittest.main()
