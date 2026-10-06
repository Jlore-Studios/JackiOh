"""#316: drop a pull request that can't be saved and build it again from `main`; and #315's routing
of a conflict revision.

#141 was opened by a run whose builder failed with a 401 in 0.0 minutes, and then revised for two
days with nothing ever giving up on it. A strike is a failure of the item's own; strikes survive a
person's request, end only on a head that met the review rule and is green in CI, and at three the
pull request is closed and its issue built again, its last findings kept for the new build.
"""

from __future__ import annotations

import json
import unittest

from harness import events, stepup, sweep
from harness import plan as plan_mod
from harness.clock import iso
from harness.config import LABEL_BUILD, LABEL_PR, LABEL_PR_OPEN, LABEL_REVISE
from harness.deliver import Deliverer
from harness.state import item as state_item

from tests.fakes import BOT, OPERATOR, FakeGitHub, git
from tests.support import DAY, make_config, make_ctx
from tests.test_flow import Harness
from tests import test_markers

TRUST = "jgoetzmann 3 id:95732896\n"
FINDING = {"severity": "blocking", "where": "SPEC.md", "claim": "R700 is cited twice",
           "evidence": "grep"}


class NoBuildNoPullTests(unittest.TestCase):
    def test_deliver_opens_no_pull_request_when_no_builder_session_worked(self):
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned, out = test_markers.DeliverTests.bundle(self, h, {"src/game.txt": "left by an earlier run\n"})
        result = json.loads((out / "result.json").read_text())
        result.update(status="not_approved", reason="the reviewer's answer could not be read",
                      cycles=[{"n": 1, "builder": {"role": "build", "ok": False, "changed": False,
                                                   "error": "401 OAuth access token is invalid"}}])
        (out / "result.json").write_text(json.dumps(result))
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        self.assertFalse([t for t in h.gh.threads.values() if "pull_request" in t])
        self.assertIsNone(h.origin_sha("bot/issue-12"))
        self.assertIn("no builder session in the run worked", "\n".join(h.gh.bot_comments(12)))


class StrikeTests(unittest.TestCase):
    def setUp(self):
        self.gh = FakeGitHub()
        self.gh.add_issue(5, labels=(LABEL_PR_OPEN,))
        self.gh.add_pull(9, "bot/issue-5", body="Closes #5", labels=(LABEL_PR,), sha="s1")
        self.ctx = make_ctx(self.gh, at=DAY, trust_text=TRUST)
        self.ctx.store.update(lambda s: state_item(s, 9).update(issue=5))
        self.cid = 900

    def strikes(self) -> int:
        return int(self.ctx.store.load()["items"].get("5", {}).get("strikes") or 0)

    def test_strikes_survive_a_persons_request(self):
        stepup.strike(self.ctx, 9, "its reviewer did not approve it")
        stepup.strike(self.ctx, 9, "CI still failed after 1 fix(es)")
        self.assertEqual(self.strikes(), 2)
        self.cid += 1
        events.handle(self.ctx, "issue_comment", {
            "action": "created", "sender": OPERATOR,
            "issue": {"number": 9, "title": "t", "pull_request": {}},
            "comment": {"id": self.cid, "body": "@jgoetzmann-bot please name it better",
                        "user": OPERATOR, "author_association": "OWNER"}})
        self.assertIn(LABEL_REVISE, self.gh.label_names(9))
        self.assertEqual(self.strikes(), 2)

    def test_only_an_approved_head_that_is_green_clears_them(self):
        stepup.strike(self.ctx, 9, "its reviewer did not approve it")
        self.ctx.store.update(lambda s: state_item(s, 9).update(
            cleared={"sha": "s1", "at": iso(DAY), "by": "`opus` (strong)"}))
        required = list(self.ctx.cfg.required_checks)
        self.gh.check_runs = lambda sha: [{"name": n, "conclusion": "success"}
                                          for n in required[:-1]]
        sweep._green_heads(self.ctx, DAY)
        self.assertEqual(self.strikes(), 1)  # approved, but one required check has not passed
        self.gh.check_runs = lambda sha: [{"name": n, "conclusion": "success"} for n in required]
        notes = sweep._green_heads(self.ctx, DAY)
        self.assertEqual(self.strikes(), 0)
        self.assertIn("#9: approved and green, so its strikes are cleared", notes)

    def test_a_green_head_nobody_approved_clears_nothing(self):
        stepup.strike(self.ctx, 9, "its reviewer did not approve it")
        self.gh.check_runs = lambda sha: [{"name": n, "conclusion": "success"}
                                          for n in self.ctx.cfg.required_checks]
        sweep._green_heads(self.ctx, DAY)
        self.assertEqual(self.strikes(), 1)

    def test_a_merge_clears_them(self):
        stepup.strike(self.ctx, 9, "its reviewer did not approve it")
        pull = dict(self.gh.get_pull(9), merged=True, state="closed",
                    base={"ref": self.ctx.cfg.default_branch})
        events.handle(self.ctx, "pull_request_target", {"action": "closed", "sender": BOT,
                                                        "pull_request": pull})
        self.assertEqual(self.strikes(), 0)

    def ci(self, attempt: int = 1, sha: str = "s1"):
        run = {"name": "CI", "conclusion": "failure", "head_branch": "bot/issue-5",
               "head_sha": sha, "id": 99, "run_attempt": attempt, "html_url": "u",
               "pull_requests": [{"number": 9}]}
        return events.handle(self.ctx, "workflow_run",
                             {"action": "completed", "sender": BOT, "workflow_run": run})

    def new_head(self, sha: str, **record) -> None:
        self.gh.threads[9]["head"]["sha"] = sha
        self.gh.threads[9]["labels"] = [{"name": LABEL_PR}]
        self.ctx.store.update(lambda s: state_item(s, 9).update(**record))

    def test_every_ci_fix_that_left_it_red_is_a_strike(self):
        self.ci()           # re-run once
        self.ci(attempt=2)  # then a first fix is queued: no strike, nothing was fixed yet
        self.assertEqual(self.strikes(), 0)
        self.new_head("s2", ci_fixed_head="s2")  # the fix pushed s2
        self.ci(sha="s2")
        self.ci(attempt=2, sha="s2")  # and CI is still red on it
        self.assertEqual(self.strikes(), 1)

    def test_a_head_no_ci_fix_made_is_no_strike(self):
        """`ci_fixes` is never reset, so a head a conflict or cross-review revision made later must
        not be struck for the CI fixes before it."""
        self.ctx.store.update(lambda s: state_item(s, 9).update(ci_fixes=1, ci_fixed_head="s1"))
        self.new_head("s3")
        self.ci(sha="s3")
        self.ci(attempt=2, sha="s3")
        self.assertEqual(self.strikes(), 0)
        self.assertIn(LABEL_REVISE, self.gh.label_names(9))  # it is fixed, not struck


class RebuildTests(unittest.TestCase):
    def test_the_rebuild_keeps_the_last_findings_and_names_the_old_pull_request(self):
        gh = FakeGitHub()
        ctx = make_ctx(gh, at=DAY, cfg=make_config(env={"HARNESS_SECRETS_SET": ""},
                                                   machine=("muse",)))
        gh.add_issue(5, labels=(LABEL_PR_OPEN,))
        gh.add_pull(9, "bot/issue-5", body="Closes #5", labels=(LABEL_PR,), sha="s1")
        ctx.store.update(lambda s: state_item(s, 9).update(issue=5, last_findings=[FINDING]))
        stepup.rebuild(ctx, 5, 9, why="it failed 3 times at difficulty:medium")
        self.assertEqual(gh.threads[9]["state"], "closed")
        record = ctx.store.load()["items"]["5"]
        self.assertEqual((record["previous_pr"], record["last_findings"]), (9, [FINDING]))
        self.assertIn(LABEL_BUILD, gh.label_names(5))
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], planned["number"]), ("build", 5))
        self.assertEqual(planned["previous_pr"], 9)
        self.assertEqual(planned["previous_findings"], [FINDING])


class ConflictRoutingTests(unittest.TestCase):
    def test_a_conflict_revision_waits_for_a_builder_that_reviews_in_its_own_run(self):
        """#315: with only Devin free, a conflict revision is not assigned at all; with Muse
        free too, Muse takes it."""
        gh = FakeGitHub()
        ctx = make_ctx(gh, at=DAY, cfg=make_config(env={"HARNESS_SECRETS_SET": ""},
                                                   machine=("devin",)))
        gh.add_pull(9, "bot/issue-2", labels=(LABEL_PR, LABEL_REVISE, "difficulty:easy"))
        ctx.store.update(lambda s: state_item(s, 9).update(kind="revise", source="conflict"))
        self.assertEqual(plan_mod.make(ctx)["action"], "none")
        ctx = make_ctx(gh, at=DAY, cfg=make_config(env={"HARNESS_SECRETS_SET": ""},
                                                   machine=("devin", "muse")))
        planned = plan_mod.make(ctx)
        self.assertEqual((planned["action"], planned["provider"]), ("revise", "muse"))


if __name__ == "__main__":
    unittest.main()
