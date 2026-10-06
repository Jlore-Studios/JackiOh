"""No request is lost: every way in either answers at once or is found again by the sweep."""

from __future__ import annotations

import unittest
from datetime import timedelta

from harness import events, sweep
from harness.clock import iso
from datetime import timedelta as _td  # noqa: F401
from harness.config import (LABELS, LABEL_BLOCKED, LABEL_BUILD, LABEL_NEEDS_PLAN, LABEL_PR,
                            LABEL_PR_OPEN, LABEL_REVISE)
from harness.errors import GitHubError, StateConflict
from harness.state import item as state_item

from tests.fakes import BOT, OPERATOR, STRANGER, FakeGitHub
from tests.support import DAY, NIGHT, make_ctx

TRUST = "jgoetzmann 3 id:95732896\nhelper 2\n"
AN_HOUR_AGO = iso(DAY - timedelta(hours=1))
JUST_NOW = iso(DAY - timedelta(minutes=2))
LAST_WEEK = iso(DAY - timedelta(days=5))


class Base(unittest.TestCase):
    def setUp(self):
        self.gh = FakeGitHub()
        self.gh.add_issue(5, "Make the Coin shiny")["type"] = {"name": "Task"}  # typed already
        self.ctx = make_ctx(self.gh, at=DAY, trust_text=TRUST)
        # The repository has every label the bot uses, as it does once the sweep made them.
        self.gh.labels.update({name: {"name": name} for name in LABELS})
        # The first sweep ever only sets its baseline; these tests start from a later one.
        self.ctx.store.update(lambda s: s.update(last_sweep={"since": "2026-09-01T00:00:00Z"}))

    def replies(self, number: int) -> list[str]:
        return self.gh.bot_comments(number)


class LostCommentTests(Base):
    def test_a_command_nobody_answered_is_answered_by_the_sweep(self):
        comment = self.gh.add_comment(5, "/harness build", OPERATOR, created_at=AN_HOUR_AGO)
        notes = sweep.sweep(self.ctx)
        self.assertEqual(self.gh.label_names(5), {LABEL_BUILD, LABEL_NEEDS_PLAN})  # queued, then planned first
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
        self.assertEqual(self.gh.label_names(5), {LABEL_BUILD, LABEL_NEEDS_PLAN})  # queued, then planned first
        self.assertIn("assigned here but never queued", self.replies(5)[-1])
        self.assertIn("from an assignment", notes[0])
        self.assertEqual(sweep.sweep(self.ctx), [])

    def test_an_assignment_already_handled_or_stopped_is_left_alone(self):
        self.gh.threads[5]["assignees"] = [BOT]
        self.ctx.store.update(lambda s: state_item(s, 5).update(queued_at=AN_HOUR_AGO))
        self.gh.add_issue(6)["type"] = {"name": "Task"}
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
        self.assertEqual(gh.label_names(5), {LABEL_BUILD, LABEL_NEEDS_PLAN})  # queued, then planned first

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
        self.assertEqual(self.gh.label_names(5), {LABEL_BUILD, LABEL_NEEDS_PLAN})  # queued, then planned first
        self.assertTrue(any("skipped one" in n for n in notes))

    def test_a_review_body_command_nobody_answered(self):
        self.gh.add_pull(9, "feature/x")
        self.gh.reviews[9] = [{"id": 55, "state": "COMMENTED", "body": "@jgoetzmann-bot fix the lint",
                               "user": OPERATOR, "author_association": "OWNER",
                               "submitted_at": AN_HOUR_AGO}]
        sweep.sweep(self.ctx)
        self.assertIn(LABEL_REVISE, self.gh.label_names(9))
        self.assertEqual(sweep.sweep(self.ctx), [])



class NightRunTests(Base):
    """GitHub drops scheduled `bot-night` runs; the sweep starts one when work waits and none is
    going, so a dropped hour costs ten minutes."""

    def at_night(self):
        self.ctx.clock_fn.at = NIGHT
        self.ctx.store.update(lambda s: s.update(last_sweep={"since": iso(NIGHT - timedelta(days=1))}))

    def test_a_due_survey_counts_as_work_too(self):
        self.at_night()
        sweep.sweep(self.ctx)
        self.assertEqual(len(self.gh.dispatches), 1)

    def test_queued_work_in_the_window_with_no_run_going_starts_one(self):
        self.at_night()
        self.gh.threads[5]["labels"] = [{"name": LABEL_BUILD}]
        notes = sweep.sweep(self.ctx)
        self.assertEqual(len(self.gh.dispatches), 1)
        self.assertEqual(self.gh.dispatches[0]["workflow"], "bot-night.yml")
        self.assertIn("started a night run", notes[-1])

    def test_a_run_already_queued_or_going_is_left_to_it(self):
        self.at_night()
        self.gh.threads[5]["labels"] = [{"name": LABEL_BUILD}]
        for status in ("queued", "in_progress", "waiting", "pending"):
            self.gh.runs = {"1": {"status": "completed"}, "2": {"status": status}}
            sweep.sweep(self.ctx)
            self.assertEqual(self.gh.dispatches, [], status)

    def test_no_run_without_work_or_outside_the_window(self):
        self.at_night()
        self.ctx.store.update(lambda s: s["suggest"].update(last_run=iso(NIGHT - timedelta(hours=1))))
        sweep.sweep(self.ctx)  # nothing queued, and no survey due
        self.ctx.clock_fn.at = DAY
        self.gh.threads[5]["labels"] = [{"name": LABEL_BUILD}]
        sweep.sweep(self.ctx)  # queued, but the window is closed
        self.assertEqual(self.gh.dispatches, [])

    def test_forced_work_starts_a_run_outside_the_window_but_never_while_halted(self):
        self.gh.threads[5]["labels"] = [{"name": LABEL_BUILD}]
        self.ctx.store.update(lambda s: state_item(s, 5).update(forced=True))
        self.ctx.store.update(lambda s: s.update(halted=True))
        sweep.sweep(self.ctx)
        self.assertEqual(self.gh.dispatches, [])
        self.ctx.store.update(lambda s: s.update(halted=False))
        sweep.sweep(self.ctx)
        self.assertEqual(len(self.gh.dispatches), 1)

    def test_a_run_that_cannot_be_started_is_noted_and_tried_again_next_sweep(self):
        self.at_night()
        self.gh.threads[5]["labels"] = [{"name": LABEL_BUILD}]

        def refuse(*args, **kwargs):
            raise GitHubError("workflow dispatch refused", 422)

        self.gh.dispatch_workflow = refuse
        notes = sweep.sweep(self.ctx)
        self.assertIn("night_run: could not finish", notes[-1])


if __name__ == "__main__":
    unittest.main()


class IssueTypeTests(Base):
    """Every open issue gets a type: triage types the ones people open, and the sweep the rest."""

    def type_of(self, number):
        return self.gh.threads[number].get("type")

    def test_a_bots_issue_is_typed_at_once_and_a_persons_after_triage_had_its_chance(self):
        alert = self.gh.add_issue(20, "CI: a job ran over 7 minutes",
                                  user={"login": "github-actions[bot]", "id": 41898282})
        alert["created_at"] = JUST_NOW
        fresh = self.gh.add_issue(21, "Patch v0.2.X: the Almanac crashes on a phone")
        fresh["created_at"] = JUST_NOW  # triage may still type it
        self.gh.add_issue(22, "Patch v0.2.X: a public card and player stats page")
        self.gh.add_issue(23, "Patch v0.2.X: Animated pass on Field Spells").update(type={"name": "Task"})
        self.gh.add_issue(24, "Easter egg: Glitch", labels=("bot:pr",)).update(pull_request={})
        notes = sweep.sweep(self.ctx)
        self.assertEqual(self.type_of(20), "Task")
        self.assertIsNone(self.type_of(21))
        self.assertEqual(self.type_of(22), "Feature")
        self.assertEqual(self.type_of(23), {"name": "Task"})  # a type someone set stays
        self.assertNotIn("type", {k for k, v in self.gh.threads[24].items() if v})  # a PR has none
        self.assertIn("#20: typed it Task", notes)
        self.gh.add_issue(25, "Make the Coin shiny")
        sweep.sweep(self.ctx)
        self.assertEqual(self.type_of(25), "Task")  # nothing else fits
        self.ctx = make_ctx(self.gh, at=DAY + timedelta(hours=4), trust_text=TRUST)
        sweep.sweep(self.ctx)
        self.assertEqual(self.type_of(21), "Bug")

    def test_the_fallback_rules(self):
        from harness import triage
        types = triage.DEFAULT_ISSUE_TYPES
        def kind(title, *labels):
            return triage.fallback_type({"title": title, "labels": [{"name": n} for n in labels]},
                                        types)
        self.assertEqual(kind("Patch v0.2.X: Rendering lag with large Radiant hands"), "Bug")
        self.assertEqual(kind("CI: the Postgres test runners wait for the real server"), "Task")
        self.assertEqual(kind("Night bot: a help command"), "Task")
        self.assertEqual(kind("Patch v0.2.X: Music improvements"), "Task")
        self.assertEqual(kind("Ranked: a new leaderboard page"), "Feature")
        self.assertEqual(kind("Patch v0.2.X: a settings page", "architecture"), "Task")
        self.assertEqual(triage.fallback_type({"title": "x"}, {"Chore": "upkeep"}), "")


class LabelTests(unittest.TestCase):
    """The sweep creates the labels the bot uses that the repository lacks: `method:manual` and
    `method:use-bot` (#307) shipped in #328 without existing, waiting for a person to run
    `python3 -m harness setup`, so no issue could be triaged."""

    def test_missing_labels_are_created_once_per_change_to_the_list(self):
        gh = FakeGitHub()
        gh.labels = {name: {"name": name} for name in LABELS
                     if name not in ("method:manual", "method:use-bot", "Info")}
        ctx = make_ctx(gh, at=DAY)
        notes = sweep._labels(ctx, DAY)
        self.assertEqual(sorted(notes), ["created the label `Info`",
                                         "created the label `method:manual`",
                                         "created the label `method:use-bot`"])
        self.assertEqual(gh.labels["method:use-bot"]["color"], LABELS["method:use-bot"][0])
        self.assertIn("two minutes after it goes on", gh.labels["method:manual"]["description"])
        # The list has not changed since: the next tick reads nothing.
        calls = []
        gh.ensure_label = lambda *a: calls.append(a) or False
        self.assertEqual(sweep._labels(ctx, DAY), [])
        self.assertEqual(calls, [])

    def test_every_sweep_runs_it(self):
        gh = FakeGitHub()
        ctx = make_ctx(gh, at=DAY)
        ctx.store.update(lambda s: s.update(last_sweep={"at": iso(DAY), "since": iso(DAY)}))
        notes = sweep.sweep(ctx)
        self.assertIn("created the label `method:manual`", notes)
        self.assertIn("method:use-bot", gh.labels)
