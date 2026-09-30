"""The second adversarial review's findings, each as the scenario that used to lose a request."""

from __future__ import annotations

import io
import json
import unittest
import urllib.error

from harness import commands, events
from harness import plan as plan_mod
from harness.config import LABEL_BLOCKED, LABEL_BUILD, LABEL_PR, LABEL_PR_OPEN, LABEL_REVISE, LABEL_WORKING
from harness.deliver import Deliverer
from harness.gh import GitHub
from harness.runner import FakeRunner, RunResult
from harness.state import item as state_item
from harness.work import Worker

from tests.fakes import BOT, OPERATOR, FakeGitHub, git, push_branch
from tests.support import DAY, make_ctx
from tests.test_flow import Harness
from tests.test_work import APPROVE, builder, reviewer

TRUST = "jgoetzmann 3 id:95732896\nhelper 2\n"
HELPER = {"login": "helper", "id": 7}
BOT_NAME = "jgoetzmann-bot"


def comment(number, body, *, cid=500, user=OPERATOR, action="created", sender=None):
    return {"action": action, "sender": sender or user, "issue": {"number": number},
            "comment": {"id": cid, "body": body, "user": user, "author_association": "OWNER"}}


class ParserTests(unittest.TestCase):
    def verbs(self, body):
        return [(c.verb, c.args) for c in commands.parse(body, BOT_NAME)]

    def test_a_control_verb_in_plain_words_is_a_request(self):
        for body in ("@jgoetzmann-bot start with option A", "@jgoetzmann-bot go ahead with plan B",
                     "@jgoetzmann-bot stop using the old sprite", "@jgoetzmann-bot halt the timer when",
                     "@jgoetzmann-bot help the player find the Coin"):
            self.assertEqual(self.verbs(body)[0][0], "request", body)

    def test_a_bare_control_verb_is_still_a_command(self):
        self.assertEqual(self.verbs("@jgoetzmann-bot start"), [("start", "")])
        self.assertEqual(self.verbs("@jgoetzmann-bot run #12"), [("run", "#12")])
        self.assertEqual(self.verbs("@jgoetzmann-bot stop"), [("stop", "")])
        self.assertEqual(self.verbs("/harness halt it is late"), [("halt", "it is late")])

    def test_commands_written_as_markdown(self):
        for body in ("- /harness build", "`/harness build`", "**/harness build**", "1. /harness build"):
            self.assertEqual(self.verbs(body), [("build", "")], body)
        self.assertEqual(self.verbs("please /harness build this"), [])
        self.assertTrue(commands.names_the_bot("please /harness build this", BOT_NAME))


class EventTests(unittest.TestCase):
    def setUp(self):
        self.gh = FakeGitHub()
        self.gh.add_issue(5)
        self.ctx = make_ctx(self.gh, at=DAY, trust_text=TRUST)

    def test_an_edit_by_someone_else_runs_nothing(self):
        out = events.handle(self.ctx, "issue_comment",
                            comment(5, "/harness halt", action="edited", sender=HELPER))
        self.assertIn("someone other than", out[0])
        self.assertFalse(self.ctx.store.load()["halted"])

    def test_an_edit_runs_only_what_it_added(self):
        events.handle(self.ctx, "issue_comment", comment(5, "thanks @jgoetzmann-bot!", cid=7))
        self.assertIn("I saw my name", self.gh.bot_comments(5)[-1])
        events.handle(self.ctx, "issue_comment", comment(5, "/harness status", cid=8))
        replies = len(self.gh.bot_comments(5))
        events.handle(self.ctx, "issue_comment",
                      comment(5, "/harness status\n/harness halt", cid=8, action="edited"))
        self.assertTrue(self.ctx.store.load()["halted"])
        self.assertNotIn("Night bot status", self.gh.bot_comments(5)[-1])
        self.assertEqual(len(self.gh.bot_comments(5)), replies + 1)

    def test_the_same_comment_is_never_run_twice(self):
        events.handle(self.ctx, "issue_comment", comment(5, "/harness halt", cid=9))
        self.ctx.store.update(lambda s: s.update(halted=False))
        events.handle(self.ctx, "issue_comment", comment(5, "/harness halt", cid=9))
        self.assertFalse(self.ctx.store.load()["halted"])

    def test_a_review_that_names_the_bot_gets_a_hint(self):
        self.gh.add_pull(9, "feature/x")
        payload = {"action": "submitted", "sender": OPERATOR, "pull_request": self.gh.get_pull(9),
                   "review": {"id": 3, "state": "COMMENTED", "user": OPERATOR,
                              "author_association": "OWNER",
                              "body": "looks fine, maybe @jgoetzmann-bot could tidy it"}}
        events.handle(self.ctx, "pull_request_review", payload)
        self.assertIn("I saw my name", self.gh.bot_comments(9)[-1])

    def test_the_build_label_on_a_pull_request_asks_for_a_revision(self):
        self.gh.add_pull(9, "feature/x", labels=(LABEL_BUILD,))
        payload = {"action": "labeled", "sender": OPERATOR, "label": {"name": LABEL_BUILD},
                   "pull_request": {"number": 9}}
        events.handle(self.ctx, "pull_request_target", payload)
        self.assertEqual(self.gh.label_names(9), {LABEL_REVISE})

    def test_the_selftest_and_a_cancelled_run_count_as_ci(self):
        self.gh.add_pull(9, "bot/issue-5", labels=(LABEL_PR,), sha="s1")
        for name, conclusion in (("bot selftest", "failure"), ("CI", "cancelled")):
            run = {"name": name, "conclusion": conclusion, "head_branch": "bot/issue-5",
                   "head_sha": "s1", "id": len(self.gh.reruns) + 1, "run_attempt": 1,
                   "pull_requests": [{"number": 9}]}
            self.ctx.store.update(lambda s: state_item(s, 9).update(ci_reruns={}))
            events.handle(self.ctx, "workflow_run", {"action": "completed", "sender": BOT,
                                                     "workflow_run": run})
        self.assertEqual(len(self.gh.reruns), 2)

    def test_run_now_is_written_down_before_it_starts(self):
        def refused(**kwargs):
            from harness.errors import GitHubError
            raise GitHubError("no", 500)
        self.ctx.dispatch = refused
        events.handle(self.ctx, "issue_comment", comment(5, "/harness run #5", cid=10))
        self.assertIn("the next hourly run does it", self.gh.bot_comments(5)[-1])
        self.gh.threads[5]["labels"] = [{"name": LABEL_BUILD}]
        work, _, forced = plan_mod.peek(self.ctx)  # by day, outside the window
        self.assertEqual((work, forced), (True, True))
        planned = plan_mod.make(self.ctx)
        self.assertEqual(planned["number"], 5)
        self.assertIsNone(self.ctx.store.load().get("run_requested"))  # taken up once


class TokenTests(unittest.TestCase):
    def test_a_refused_bot_token_falls_back_to_the_actions_token(self):
        seen = []

        def opener(req, timeout=60):
            token = req.headers.get("Authorization", "")
            seen.append(token)
            if token == "Bearer bot-token":
                raise urllib.error.HTTPError(req.full_url, 401, "Bad credentials", {}, io.BytesIO(b"{}"))
            return io.BytesIO(json.dumps({"ok": True}).encode())

        gh = GitHub("o/r", "bot-token", opener=opener, fallback_token="actions-token")
        self.assertEqual(gh.request("GET", "/x"), {"ok": True})
        self.assertEqual(seen, ["Bearer bot-token", "Bearer actions-token"])
        gh.request("GET", "/y")
        self.assertEqual(seen[-1], "Bearer actions-token")


class DeliverTests(unittest.TestCase):
    def approved_run(self, h, number=12):
        planned = plan_mod.make(h.ctx)
        out = h.root / f"out-{number}-{len(list(h.root.glob('out-*')))}"
        Worker(h.cfg, planned, FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                                           "review": reviewer(APPROVE)}),
               h.clone, h.root / "work", out).run()
        return planned, out

    def test_a_stop_after_the_last_checkpoint_publishes_no_pull_request(self):
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned, out = self.approved_run(h)
        h.ctx.store.update(lambda s: state_item(s, 12).update(
            stop_requested=True, stopped_by="jgoetzmann", stopped_at="2026-09-30T03:30:00Z"))
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        self.assertEqual(h.gh.list_pulls(head="bot/issue-12"), [])
        self.assertEqual(h.gh.label_names(12), set())
        self.assertIn("Stopped, as asked", h.gh.bot_comments(12)[-1])

    def test_a_halt_after_the_last_checkpoint_keeps_the_work_queued(self):
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned, out = self.approved_run(h)
        h.ctx.store.update(lambda s: s.update(halted=True))
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        self.assertEqual(h.gh.list_pulls(head="bot/issue-12"), [])
        self.assertEqual(h.gh.label_names(12), {LABEL_BUILD})
        self.assertIsNotNone(h.origin_sha("bot/issue-12"))

    def test_the_model_job_itself_stops_after_the_last_review(self):
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned = plan_mod.make(h.ctx)
        calls = {"n": 0}
        def probe(usage):
            calls["n"] += 1
            return ("stopped by @jgoetzmann", "stop") if usage is None else None
        result = Worker(h.cfg, planned, FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                                                    "review": reviewer(APPROVE)}),
                        h.clone, h.root / "work", h.root / "out-probe", probe=probe).run()
        self.assertEqual(result["status"], "stopped")

    def test_stop_then_a_new_request_is_answered_after_the_stop(self):
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned, out = self.approved_run(h)
        events.handle(h.ctx, "issue_comment", comment(12, "/harness stop", cid=21))
        self.assertEqual(h.gh.label_names(12), {LABEL_WORKING})
        events.handle(h.ctx, "issue_comment",
                      comment(12, "@jgoetzmann-bot do it with a Token class instead", cid=22))
        h.ctx.clock_fn.at = h.ctx.clock_fn.at.replace(minute=20)
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        self.assertEqual(h.gh.list_pulls(head="bot/issue-12"), [])
        self.assertIn(LABEL_BUILD, h.gh.label_names(12))  # the new request is queued

    def test_a_revision_asked_for_during_the_build_is_kept(self):
        h = Harness(self)
        start = push_branch(h.origin, h.root, "bot/issue-12", {"src/game.txt": "v1b\n"})
        git(h.clone, "fetch", "-q", "origin")
        git(h.deliver_repo, "fetch", "-q", "origin")
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        h.gh.add_pull(40, "bot/issue-12", body="Closes #12", labels=(LABEL_PR, LABEL_REVISE), sha=start)
        h.gh.threads[40]["draft"] = True
        planned = plan_mod.make(h.ctx)
        self.assertEqual(planned["action"], "revise")  # revisions first
        h.gh.threads[40]["labels"] = [{"name": LABEL_PR}]
        planned = {"action": "build", "number": 12, "branch": "bot/issue-12", "title": "t"}
        h.ctx.store.update(lambda s: state_item(s, 12).update(started_at="2026-09-30T03:00:00Z"))
        out = h.root / "out-keep"
        Worker(h.cfg, planned, FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                                           "review": reviewer(APPROVE)}),
               h.clone, h.root / "work", out).run()
        h.gh.add_labels(40, [LABEL_REVISE])  # a person asks for a change while it builds
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        self.assertIn(LABEL_REVISE, h.gh.label_names(40))
        self.assertNotIn("PR_40", h.gh.auto_merge)

    def test_a_failed_survey_gives_back_the_request(self):
        h = Harness(self)
        h.ctx.store.update(lambda s: s["suggest"].update(requested=True))
        planned = plan_mod.make(h.ctx, mode="suggest")
        self.assertTrue(planned["was_requested"])
        out = h.root / "out-survey"
        out.mkdir()
        (out / "result.json").write_text(json.dumps({"status": "failed", "reason": "boom"}))
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        self.assertTrue(h.ctx.store.load()["suggest"]["requested"])

    def test_an_unfinished_survey_is_due_again(self):
        h = Harness(self)
        h.ctx.store.update(lambda s: s.update(suggest={"last_run": None, "requested": False}))
        planned = plan_mod.make(h.ctx, force=True, mode="suggest")
        self.assertEqual(planned["action"], "suggest")
        out = h.root / "out-cut-short"
        out.mkdir()
        (out / "result.json").write_text(json.dumps({"status": "interrupted",
                                                     "reason": "the usage limit was reached"}))
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        self.assertIsNone(h.ctx.store.load()["suggest"]["last_run"])
        self.assertTrue(plan_mod.suggestions_due(h.ctx, h.ctx.store.load()))


if __name__ == "__main__":
    unittest.main()
