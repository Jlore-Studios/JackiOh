"""An issue closed while a run works on it: the run keeps its lane until it ends, its work is not
delivered, and nothing queues it again."""

from __future__ import annotations

import unittest

from harness import plan as plan_mod
from harness.config import LABEL_BUILD, LABEL_WORKING
from harness.runner import FakeRunner
from harness.state import item as state_item
from harness.status import report

from tests.fakes import FakeGitHub
from tests.support import DAY, MACHINE, make_config, make_ctx
from tests.test_cross import ALL
from tests.test_flow import Harness
from tests.test_work import APPROVE, builder, reviewer

#: Three runs on the machine's subscriptions; someone closed #5 while its run went on.
RUNS = ((5, "gpt", "51"), (6, "agy", "61"), (7, "muse", "71"))


class ClosedWhileWorkingTests(unittest.TestCase):
    def test_its_run_keeps_its_lane(self):
        gh = FakeGitHub()
        for number, _, run_id in RUNS:
            gh.add_issue(number, labels=(LABEL_WORKING,), state="closed" if number == 5 else "open")
            gh.runs[run_id] = {"status": "in_progress"}
        gh.add_issue(8, labels=(LABEL_BUILD,))
        ctx = make_ctx(gh, at=DAY, cfg=make_config(env=ALL, machine=MACHINE))
        ctx.store.update(lambda s: [state_item(s, n).update(run_id=r, provider=p)
                                    for n, p, r in RUNS])
        planned = plan_mod.make(ctx)
        self.assertEqual(planned["action"], "none")
        self.assertIn("`gpt` is busy", planned["reason"])  # its lane is still the closed #5's
        text = report(ctx)
        self.assertIn("3 of 10 lanes, 7 free; 3 of 6 on the machine", text)
        self.assertIn("`gpt` (codex", text)

    def test_its_work_is_not_delivered(self):
        h = Harness(self, env=ALL)
        h.gh.add_issue(12, "Make the rules v2", labels=(LABEL_BUILD,))
        build = builder({"src/game.txt": "rules v2\n"})

        def closed_meanwhile(request):
            h.gh.threads[12]["state"] = "closed"
            return build(request)

        _, result = h.night(FakeRunner({"build": closed_meanwhile, "review": reviewer(APPROVE)}))
        self.assertEqual(result["status"], "approved")  # the run itself finished its work
        self.assertEqual(h.gh.list_pulls(head="bot/issue-12"), [])
        self.assertIsNone(h.origin_sha("bot/issue-12"))
        self.assertFalse(h.gh.label_names(12) & {LABEL_WORKING, LABEL_BUILD})
        self.assertIn("was closed while a run was working on it", h.gh.bot_comments(12)[-1])

    def test_a_dead_runs_label_is_cleared_not_requeued(self):
        gh = FakeGitHub()
        gh.add_issue(5, labels=(LABEL_WORKING,), state="closed")
        gh.runs["51"] = {"status": "completed", "conclusion": "cancelled"}
        ctx = make_ctx(gh, cfg=make_config(env=ALL, machine=MACHINE))
        ctx.store.update(lambda s: state_item(s, 5).update(run_id="51", provider="gpt"))
        self.assertIn("#5", plan_mod.housekeeping_due(ctx, ctx.store.load()))
        notes = plan_mod.housekeeping(ctx, ctx.store.load())
        self.assertEqual(gh.label_names(5), set())
        self.assertEqual(gh.bot_comments(5), [])  # no "back in the queue"
        self.assertTrue(any("#5 was closed" in note for note in notes), notes)


if __name__ == "__main__":
    unittest.main()
