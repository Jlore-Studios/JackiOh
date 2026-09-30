"""No request is lost: every way in either answers at once or is found again by the sweep."""

from __future__ import annotations

import unittest
from datetime import timedelta

from harness import events, sweep
from harness.clock import iso
from datetime import timedelta as _td  # noqa: F401
from harness.config import LABEL_BLOCKED, LABEL_BUILD, LABEL_PR, LABEL_PR_OPEN, LABEL_REVISE
from harness.errors import GitHubError, StateConflict
from harness.state import item as state_item

from tests.fakes import BOT, OPERATOR, STRANGER, FakeGitHub
from tests.support import DAY, make_ctx

TRUST = "jgoetzmann 3 id:95732896\nhelper 2\n"
AN_HOUR_AGO = iso(DAY - timedelta(hours=1))
JUST_NOW = iso(DAY - timedelta(minutes=2))
LAST_WEEK = iso(DAY - timedelta(days=5))


class Base(unittest.TestCase):
    def setUp(self):
        self.gh = FakeGitHub()
        self.gh.add_issue(5, "Make the Coin shiny")
        self.ctx = make_ctx(self.gh, at=DAY, trust_text=TRUST)
        # The first sweep ever only sets its baseline; these tests start from a later one.
        self.ctx.store.update(lambda s: s.update(last_sweep={"since": "2026-09-01T00:00:00Z"}))

    def replies(self, number: int) -> list[str]:
        return self.gh.bot_comments(number)


class LostCommentTests(Base):
    def test_a_command_nobody_answered_is_answered_by_the_sweep(self):
        comment = self.gh.add_comment(5, "/harness build", OPERATOR, created_at=AN_HOUR_AGO)
        notes = sweep.sweep(self.ctx)
        self.assertEqual(self.gh.label_names(5), {LABEL_BUILD})
        self.assertIn("Queued #5", self.replies(5)[-1])
        self.assertIn((comment["id"], "rocket"), self.gh.reacted)
        self.assertIn("answered a comment on #5", notes[0])
        self.assertEqual(sweep.sweep(self.ctx), [])  # answered once, never twice

    def test_what_the_sweep_leaves_alone(self):
        self.gh.add_comment(5, "/harness build", OPERATOR, created_at=JUST_NOW)  # handler may run
        self.gh.add_comment(5, "/harness build", OPERATOR, created_at=LAST_WEEK)  # too old
        self.gh.add_comment(5, "/harness halt", STRANGER, "NONE", created_at=AN_HOUR_AGO)
        self.gh.add_comment(5, "a plain comment", OPERATOR, created_at=AN_HOUR_AGO)
        self.gh.create_comment(5, "/harness build")  # the bot's own words
        answered = self.gh.add_comment(5, "/harness status", OPERATOR, created_at=AN_HOUR_AGO)
        self.gh.react(answered["id"], "rocket")
        before = len(self.replies(5))
        self.assertEqual(sweep.sweep(self.ctx), [])
        self.assertEqual(len(self.replies(5)), before)
        self.assertFalse(self.ctx.store.load()["halted"])

    def test_a_line_comment_on_a_diff_is_answered_too(self):
        self.gh.add_pull(9, "bot/issue-5", labels=(LABEL_PR,))
        self.gh.add_review_comment(9, "@jgoetzmann-bot rename this", OPERATOR, created_at=AN_HOUR_AGO)
        sweep.sweep(self.ctx)
        self.assertIn(LABEL_REVISE, self.gh.label_names(9))


class HandlerTests(Base):
    def test_a_line_comment_event_queues_a_revision_and_reacts_on_the_right_endpoint(self):
        self.gh.add_pull(9, "bot/issue-5", labels=(LABEL_PR,))
        payload = {"action": "created", "sender": OPERATOR, "pull_request": self.gh.get_pull(9),
                   "comment": {"id": 77, "body": "@jgoetzmann-bot use a constant here",
                               "user": OPERATOR, "author_association": "OWNER"}}
        events.handle(self.ctx, "pull_request_review_comment", payload)
        self.assertIn(LABEL_REVISE, self.gh.label_names(9))
        self.assertIn((77, "rocket"), self.gh.reacted)

    def test_an_edit_that_adds_a_command_is_heard_once(self):
        payload = {"action": "edited", "sender": OPERATOR, "issue": {"number": 5},
                   "comment": {"id": 88, "body": "on second thought\n/harness build",
                               "user": OPERATOR, "author_association": "OWNER"}}
        events.handle(self.ctx, "issue_comment", payload)
        self.assertEqual(self.gh.label_names(5), {LABEL_BUILD})
        count = len(self.replies(5))
        events.handle(self.ctx, "issue_comment", payload)  # edited again: already answered
        self.assertEqual(len(self.replies(5)), count)

    def test_naming_the_bot_mid_sentence_gets_an_answer_not_silence(self):
        payload = {"action": "created", "sender": OPERATOR, "issue": {"number": 5},
                   "comment": {"id": 90, "body": "thanks, @jgoetzmann-bot could you also add sound?",
                               "user": OPERATOR, "author_association": "OWNER"}}
        events.handle(self.ctx, "issue_comment", payload)
        self.assertIn("I saw my name", self.replies(5)[-1])
        self.assertIn((90, "rocket"), self.gh.reacted)
        self.assertEqual(self.gh.label_names(5), set())  # a hint, never a guessed build

    def test_a_failing_command_is_answered_and_its_neighbours_still_run(self):
        real = self.ctx.store.update
        def conflicted(change, message="state"):
            if message.startswith("claim"):
                return real(change, message)
            raise StateConflict("the state kept changing")
        self.ctx.store.update = conflicted
        payload = {"action": "created", "sender": OPERATOR, "issue": {"number": 5},
                   "comment": {"id": 91, "body": "/harness halt\n/harness status",
                               "user": OPERATOR, "author_association": "OWNER"}}
        events.handle(self.ctx, "issue_comment", payload)
        reply = self.replies(5)[-1]
        self.assertIn("That failed (StateConflict", reply)
        self.assertIn("Night bot status", reply)
        self.assertIn((91, "rocket"), self.gh.reacted)

    def test_a_force_that_cannot_start_a_run_still_queues_it(self):
        def refused(**kwargs):
            raise GitHubError("dispatch refused", 422)
        self.ctx.dispatch = refused
        payload = {"action": "created", "sender": OPERATOR, "issue": {"number": 5},
                   "comment": {"id": 92, "body": "/harness build --force", "user": OPERATOR,
                               "author_association": "OWNER"}}
        events.handle(self.ctx, "issue_comment", payload)
        self.assertEqual(self.gh.label_names(5), {LABEL_BUILD})
        self.assertTrue(self.ctx.store.load()["items"]["5"]["forced"])
        self.assertIn("the next hourly run starts it", self.replies(5)[-1])


class LostEventTests(Base):
    def test_an_assignment_nothing_answered_is_queued(self):
        self.gh.threads[5]["assignees"] = [BOT]
        notes = sweep.sweep(self.ctx)
        self.assertEqual(self.gh.label_names(5), {LABEL_BUILD})
        self.assertIn("assigned here but never queued", self.replies(5)[-1])
        self.assertIn("from an assignment", notes[0])
        self.assertEqual(sweep.sweep(self.ctx), [])

    def test_an_assignment_already_handled_or_stopped_is_left_alone(self):
        self.gh.threads[5]["assignees"] = [BOT]
        self.ctx.store.update(lambda s: state_item(s, 5).update(queued_at=AN_HOUR_AGO))
        self.gh.add_issue(6)
        self.gh.threads[6]["assignees"] = [BOT]
        self.ctx.store.update(lambda s: state_item(s, 6).update(stop_requested=True))
        self.assertEqual(sweep.sweep(self.ctx), [])

    def test_a_review_asking_for_changes_that_nothing_answered(self):
        self.gh.add_pull(9, "bot/issue-5", labels=(LABEL_PR,))
        self.ctx.store.update(lambda s: state_item(s, 9).update(feedback_since=LAST_WEEK))
        self.gh.reviews[9] = [{"id": 61, "state": "CHANGES_REQUESTED", "body": "No.", "user": OPERATOR,
                               "author_association": "OWNER", "submitted_at": AN_HOUR_AGO}]
        sweep.sweep(self.ctx)
        self.assertIn(LABEL_REVISE, self.gh.label_names(9))
        self.assertIn("Queued a revision of #9", self.replies(9)[-1])

    def test_a_review_the_bot_already_answered_is_not_redone(self):
        self.gh.add_pull(9, "bot/issue-5", labels=(LABEL_PR,))
        review = {"id": 62, "state": "CHANGES_REQUESTED", "body": "No.", "user": OPERATOR,
                  "author_association": "OWNER", "submitted_at": AN_HOUR_AGO}
        self.gh.reviews[9] = [review]
        events.handle(self.ctx, "pull_request_review", {"action": "submitted", "sender": OPERATOR,
                                                        "review": review,
                                                        "pull_request": self.gh.get_pull(9)})
        self.gh.threads[9]["labels"] = [{"name": LABEL_PR}]  # the revision has since landed
        self.assertEqual(sweep.sweep(self.ctx), [])

    def test_a_failed_ci_run_nothing_answered_is_rerun_then_fixed(self):
        self.gh.add_pull(9, "bot/issue-5", labels=(LABEL_PR,), sha="s1")
        self.gh.runs["99"] = {"id": 99, "name": "CI", "head_sha": "s1", "head_branch": "bot/issue-5",
                              "status": "completed", "conclusion": "failure", "run_attempt": 1,
                              "created_at": AN_HOUR_AGO, "html_url": "u", "pull_requests": []}
        sweep.sweep(self.ctx)
        self.assertEqual(self.gh.reruns, [99])
        self.gh.runs["99"]["run_attempt"] = 2
        sweep.sweep(self.ctx)
        self.assertIn(LABEL_REVISE, self.gh.label_names(9))
        self.assertEqual(sweep.sweep(self.ctx), [])  # queued: nothing more to do

    def test_a_blocked_or_green_pr_is_left_alone(self):
        self.gh.add_pull(9, "bot/issue-5", labels=(LABEL_PR, LABEL_BLOCKED), sha="s1")
        self.gh.add_pull(10, "bot/issue-6", labels=(LABEL_PR,), sha="s2")
        self.gh.runs["1"] = {"id": 1, "name": "CI", "head_sha": "s1", "status": "completed",
                             "conclusion": "failure", "created_at": AN_HOUR_AGO}
        self.gh.runs["2"] = {"id": 2, "name": "CI", "head_sha": "s2", "status": "completed",
                             "conclusion": "success", "created_at": AN_HOUR_AGO}
        self.assertEqual(sweep.sweep(self.ctx), [])

    def test_the_sweep_records_when_it_ran(self):
        sweep.sweep(self.ctx)
        self.assertIn("at", self.ctx.store.load()["last_sweep"])

    def test_the_first_sweep_replays_nothing(self):
        gh = FakeGitHub()
        gh.add_issue(5)
        gh.add_comment(5, "/harness build", OPERATOR, created_at=AN_HOUR_AGO)
        ctx = make_ctx(gh, at=DAY, trust_text=TRUST)
        self.assertIn("first sweep", sweep.sweep(ctx)[0])
        self.assertEqual(gh.label_names(5), set())
        ctx.clock_fn.at = DAY + timedelta(minutes=30)
        gh.add_comment(5, "/harness build", OPERATOR, created_at=iso(DAY + timedelta(minutes=5)))
        sweep.sweep(ctx)
        self.assertEqual(gh.label_names(5), {LABEL_BUILD})

    def test_an_edited_comment_is_left_to_its_edit_event(self):
        comment = self.gh.add_comment(5, "/harness halt", OPERATOR, created_at=AN_HOUR_AGO)
        comment["updated_at"] = iso(DAY - timedelta(minutes=30))
        self.assertEqual(sweep.sweep(self.ctx), [])
        self.assertFalse(self.ctx.store.load()["halted"])

    def test_one_bad_comment_does_not_stop_the_rest(self):
        self.gh.comments[404] = [{"id": 1, "body": "/harness build", "user": OPERATOR,
                                  "author_association": "OWNER", "created_at": AN_HOUR_AGO,
                                  "issue_url": "https://api.github.com/repos/x/y/issues/404"}]
        self.gh.add_comment(5, "/harness build", OPERATOR, created_at=AN_HOUR_AGO)
        notes = sweep.sweep(self.ctx)
        self.assertEqual(self.gh.label_names(5), {LABEL_BUILD})
        self.assertTrue(any("skipped one" in n for n in notes))

    def test_a_review_body_command_nobody_answered(self):
        self.gh.add_pull(9, "feature/x")
        self.gh.reviews[9] = [{"id": 55, "state": "COMMENTED", "body": "@jgoetzmann-bot fix the lint",
                               "user": OPERATOR, "author_association": "OWNER",
                               "submitted_at": AN_HOUR_AGO}]
        sweep.sweep(self.ctx)
        self.assertIn(LABEL_REVISE, self.gh.label_names(9))
        self.assertEqual(sweep.sweep(self.ctx), [])


if __name__ == "__main__":
    unittest.main()
