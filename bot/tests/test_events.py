"""Events: comment commands, labels, assignment, reviews, CI results, closed pull requests."""

from __future__ import annotations

import unittest

from harness import events
from harness.config import (LABEL_BLOCKED, LABEL_BUILD, LABEL_PR, LABEL_PR_OPEN, LABEL_REVISE,
                            LABEL_WORKING, MARKER)
from harness.state import item as state_item

from tests.fakes import BOT, OPERATOR, STRANGER, FakeGitHub
from tests.support import DAY, make_ctx

TRUST = "jgoetzmann 3 id:95732896\nhelper 2\nreader 1\n"
HELPER = {"login": "helper", "id": 7}
READER = {"login": "reader", "id": 8}


def comment_event(number: int, body: str, user=OPERATOR, association="OWNER", pr=False) -> dict:
    issue = {"number": number, "title": "t"}
    if pr:
        issue["pull_request"] = {}
    return {"action": "created", "sender": user, "issue": issue,
            "comment": {"id": 501, "body": body, "user": user, "author_association": association}}


class CommentTests(unittest.TestCase):
    def setUp(self):
        self.gh = FakeGitHub()
        self.gh.add_issue(5, "Make the Coin shiny")
        self.ctx = make_ctx(self.gh, at=DAY, trust_text=TRUST)

    def send(self, body, **kwargs):
        return events.handle(self.ctx, "issue_comment", comment_event(5, body, **kwargs))

    def reply(self) -> str:
        return self.gh.bot_comments(5)[-1]

    def test_build_queues_the_issue(self):
        self.send("/harness build")
        self.assertEqual(self.gh.label_names(5), {LABEL_BUILD})
        self.assertIn("Queued #5", self.reply())
        self.assertIn((501, "eyes"), self.gh.reacted)
        self.assertEqual(self.gh.dispatches, [])
        self.assertEqual(self.ctx.store.load()["items"]["5"]["requested_by"], "jgoetzmann")

    def test_force_starts_a_run_now(self):
        self.send("@jgoetzmann-bot build --force")
        self.assertEqual(self.gh.dispatches[0]["inputs"], {"item": "5", "force": "true", "mode": "build"})

    def test_force_needs_the_operator(self):
        self.send("/harness build --force", user=HELPER, association="COLLABORATOR")
        self.assertEqual(self.gh.dispatches, [])
        self.assertEqual(self.gh.label_names(5), set())
        self.assertIn("operator level", self.reply())

    def test_a_maintainer_may_build_without_force(self):
        self.send("/harness build", user=HELPER, association="COLLABORATOR")
        self.assertEqual(self.gh.label_names(5), {LABEL_BUILD})

    def test_an_asker_may_only_ask(self):
        self.send("/harness status\n/harness build", user=READER, association="COLLABORATOR")
        self.assertIn("Night bot status", self.reply())
        self.assertIn("maintainer level", self.reply())
        self.assertEqual(self.gh.label_names(5), set())

    def test_strangers_are_ignored_silently(self):
        out = self.send("/harness halt", user=STRANGER, association="NONE")
        self.assertIn("not on the trust list", out[0])
        self.assertEqual(self.gh.bot_comments(5), [])
        self.assertFalse(self.ctx.store.load()["halted"])
        # A listed name without an id needs GitHub's word too.
        self.send("/harness build", user=HELPER, association="CONTRIBUTOR")
        self.assertEqual(self.gh.label_names(5), set())

    def test_the_bots_own_comments_are_ignored(self):
        out = events.handle(self.ctx, "issue_comment",
                            comment_event(5, f"/harness build\n{MARKER}", user=BOT))
        self.assertEqual(out, ["ignored: the bot's own event"])
        out = events.handle(self.ctx, "issue_comment",
                            comment_event(5, f"/harness build\n{MARKER}", user=OPERATOR))
        self.assertEqual(out, ["ignored: the bot's own comment"])

    def test_quoting_the_bot_still_lets_a_person_command(self):
        self.send(f"> earlier reply\n> {MARKER}\n/harness build")
        self.assertEqual(self.gh.label_names(5), {LABEL_BUILD})

    def test_halt_and_start(self):
        self.send("/harness halt going on holiday")
        state = self.ctx.store.load()
        self.assertTrue(state["halted"])
        self.assertEqual(state["halt"]["reason"], "going on holiday")
        self.send("/harness start")
        self.assertFalse(self.ctx.store.load()["halted"])

    def test_halt_needs_the_operator(self):
        self.send("/harness halt", user=HELPER, association="COLLABORATOR")
        self.assertFalse(self.ctx.store.load()["halted"])

    def test_status_reports_the_queue_and_window(self):
        self.gh.add_issue(6, labels=(LABEL_BUILD,))
        self.send("/harness status")
        text = self.reply()
        self.assertIn("Queued to build: #6", text)
        self.assertIn("closed, opens in", text)
        self.assertIn("`opus` at `xhigh`", text)

    def test_free_text_on_an_issue_asks_for_a_build(self):
        self.send("@jgoetzmann-bot please also make it sparkle")
        self.assertEqual(self.gh.label_names(5), {LABEL_BUILD})
        self.assertIn("read your notes", self.reply())

    def test_a_build_request_on_an_issue_with_a_pr_revises_the_pr(self):
        self.gh.threads[5]["labels"] = [{"name": LABEL_PR_OPEN}]
        self.gh.add_pull(9, "bot/issue-5", labels=(LABEL_PR,))
        self.gh.threads[9]["auto_merge"] = {"merge_method": "squash"}
        self.gh.auto_merge["PR_9"] = "squash"
        self.send("@jgoetzmann-bot also make it sparkle")
        self.assertEqual(self.gh.label_names(9), {LABEL_PR, LABEL_REVISE})
        self.assertNotIn("PR_9", self.gh.auto_merge)

    def test_stop_takes_it_out_and_tells_a_running_job(self):
        self.gh.threads[5]["labels"] = [{"name": LABEL_WORKING}]
        self.send("/harness stop")
        # The run still holds it until it stops, so a new request waits for that run.
        self.assertEqual(self.gh.label_names(5), {LABEL_WORKING})
        self.assertTrue(self.ctx.store.load()["items"]["5"]["stop_requested"])
        self.assertIn("next checkpoint", self.reply())

    def test_suggest_and_run(self):
        self.send("/harness suggest")
        self.assertTrue(self.ctx.store.load()["suggest"]["requested"])
        self.send("/harness suggest --force")
        self.assertEqual(self.gh.dispatches[-1]["inputs"]["mode"], "suggest")
        self.send("/harness run #5")
        self.assertEqual(self.gh.dispatches[-1]["inputs"], {"item": "5", "force": "true", "mode": "auto"})

    def test_unknown_verbs_get_the_help(self):
        self.send("/harness dance")
        self.assertIn("I do not know `dance`", self.reply())

    def test_revise_on_a_pull_request_from_a_fork_is_refused(self):
        self.gh.add_pull(9, "their-branch", fork=True)
        events.handle(self.ctx, "issue_comment", comment_event(9, "/harness revise x", pr=True))
        self.assertIn("comes from a fork", self.gh.bot_comments(9)[-1])
        self.assertEqual(self.gh.label_names(9), set())

    def test_a_person_can_ask_for_changes_on_their_own_pull_request(self):
        self.gh.add_pull(9, "feature/mine")
        events.handle(self.ctx, "issue_comment",
                      comment_event(9, "@jgoetzmann-bot fix the failing test", pr=True))
        self.assertEqual(self.gh.label_names(9), {LABEL_REVISE})


class LabelAndAssignTests(unittest.TestCase):
    def setUp(self):
        self.gh = FakeGitHub()
        self.gh.add_issue(5, labels=(LABEL_BUILD,))
        self.ctx = make_ctx(self.gh, at=DAY, trust_text=TRUST)

    def test_the_build_label_from_a_trusted_person_queues(self):
        payload = {"action": "labeled", "sender": OPERATOR, "label": {"name": LABEL_BUILD},
                   "issue": {"number": 5}}
        events.handle(self.ctx, "issues", payload)
        self.assertEqual(self.gh.label_names(5), {LABEL_BUILD})
        self.assertIn("Queued #5", self.gh.bot_comments(5)[-1])
        self.assertIn("queued_at", self.ctx.store.load()["items"]["5"])

    def test_the_build_label_from_anyone_else_is_removed(self):
        payload = {"action": "labeled", "sender": STRANGER, "label": {"name": LABEL_BUILD},
                   "issue": {"number": 5}}
        events.handle(self.ctx, "issues", payload)
        self.assertEqual(self.gh.label_names(5), set())

    def test_assigning_the_bot_queues(self):
        self.gh.threads[5]["labels"] = []
        payload = {"action": "assigned", "sender": OPERATOR, "assignee": BOT, "issue": {"number": 5}}
        events.handle(self.ctx, "issues", payload)
        self.assertEqual(self.gh.label_names(5), {LABEL_BUILD})

    def test_assigning_someone_else_does_nothing(self):
        self.gh.threads[5]["labels"] = []
        payload = {"action": "assigned", "sender": OPERATOR, "assignee": STRANGER, "issue": {"number": 5}}
        events.handle(self.ctx, "issues", payload)
        self.assertEqual(self.gh.label_names(5), set())

    def test_the_revise_label_on_a_pull_request(self):
        self.gh.add_pull(9, "bot/issue-5", labels=(LABEL_PR, LABEL_REVISE))
        payload = {"action": "labeled", "sender": OPERATOR, "label": {"name": LABEL_REVISE},
                   "pull_request": {"number": 9}}
        events.handle(self.ctx, "pull_request_target", payload)
        self.assertEqual(self.ctx.store.load()["items"]["9"]["source"], "request")


class ReviewAndCiTests(unittest.TestCase):
    def setUp(self):
        self.gh = FakeGitHub()
        self.gh.add_issue(5, labels=(LABEL_PR_OPEN,))
        self.pull = self.gh.add_pull(9, "bot/issue-5", body="Closes #5", labels=(LABEL_PR,), sha="s1")
        self.ctx = make_ctx(self.gh, at=DAY, trust_text=TRUST)

    def test_changes_requested_on_a_bot_pr_queues_a_revision(self):
        payload = {"action": "submitted", "sender": OPERATOR, "pull_request": self.gh.get_pull(9),
                   "review": {"state": "changes_requested", "body": "Name it better.",
                              "user": OPERATOR, "author_association": "OWNER"}}
        events.handle(self.ctx, "pull_request_review", payload)
        self.assertIn(LABEL_REVISE, self.gh.label_names(9))

    def ci(self, attempt=1, sha="s1"):
        run = {"name": "CI", "conclusion": "failure", "head_branch": "bot/issue-5", "head_sha": sha,
               "id": 99, "run_attempt": attempt, "html_url": "u", "pull_requests": [{"number": 9}]}
        # The bot pushed the branch, so GitHub names it as the sender of the CI run.
        return events.handle(self.ctx, "workflow_run",
                             {"action": "completed", "sender": BOT, "workflow_run": run})

    def test_ci_failure_is_rerun_once_then_fixed(self):
        self.ci()
        self.assertEqual(self.gh.reruns, [99])
        self.assertNotIn(LABEL_REVISE, self.gh.label_names(9))
        self.ci(attempt=2)
        self.assertIn(LABEL_REVISE, self.gh.label_names(9))
        record = self.ctx.store.load()["items"]["9"]
        self.assertEqual((record["source"], record["ci_run_id"], record["ci_fixes"]), ("ci", 99, 1))

    def test_actions_calls_use_the_jobs_own_token(self):
        actions = FakeGitHub()
        self.ctx.actions_gh = actions
        self.ci()
        self.assertEqual((actions.reruns, self.gh.reruns), ([99], []))
        self.ctx.dispatch(force=True)
        self.assertEqual((len(actions.dispatches), self.gh.dispatches), (1, []))

    def test_ci_fixes_are_capped(self):
        self.ctx.store.update(lambda s: state_item(s, 9).update(ci_fixes=3,
                                                                ci_reruns={"s1": 1}))
        self.gh.threads[9]["auto_merge"] = {"merge_method": "squash"}
        self.gh.auto_merge["PR_9"] = "squash"
        self.ci(attempt=2)
        self.assertEqual(self.gh.label_names(9), {LABEL_PR, LABEL_BLOCKED})
        self.assertNotIn("PR_9", self.gh.auto_merge)
        self.assertIn("It needs a person", self.gh.bot_comments(9)[-1])

    def test_a_person_asking_again_does_not_reset_the_ci_cap(self):
        self.ctx.store.update(lambda s: state_item(s, 9).update(ci_fixes=2))
        events.handle(self.ctx, "issue_comment",
                      comment_event(9, "/harness revise try again", pr=True))
        self.assertEqual(self.ctx.store.load()["items"]["9"]["ci_fixes"], 2)

    def test_ci_leaves_a_stopped_or_blocked_pr_alone(self):
        self.ctx.store.update(lambda s: state_item(s, 9).update(stop_requested=True))
        self.ci(attempt=3)
        self.assertEqual(self.gh.reruns, [])
        self.assertEqual(self.gh.label_names(9), {LABEL_PR})

    def test_ci_on_a_stale_commit_or_a_human_pr_is_ignored(self):
        run = {"name": "CI", "conclusion": "failure", "head_branch": "bot/issue-5", "head_sha": "old",
               "id": 99, "run_attempt": 3, "pull_requests": [{"number": 9}]}
        events.handle(self.ctx, "workflow_run", {"action": "completed", "sender": OPERATOR,
                                                  "workflow_run": run})
        self.assertEqual(self.gh.label_names(9), {LABEL_PR})
        self.gh.add_pull(10, "feature/human", sha="h1")
        run = {"name": "CI", "conclusion": "failure", "head_branch": "feature/human", "head_sha": "h1",
               "id": 98, "run_attempt": 3, "pull_requests": [{"number": 10}]}
        events.handle(self.ctx, "workflow_run", {"action": "completed", "sender": OPERATOR,
                                                  "workflow_run": run})
        self.assertEqual(self.gh.label_names(10), set())

    def test_a_merged_bot_pr_clears_the_issue(self):
        pull = self.gh.get_pull(9)
        pull["merged"] = True
        # Auto-merge is credited to whoever turned it on: the bot.
        events.handle(self.ctx, "pull_request_target", {"action": "closed", "sender": BOT,
                                                         "pull_request": pull})
        self.assertEqual(self.gh.label_names(5), set())
        self.assertTrue(self.ctx.store.load()["items"]["9"]["merged"])

    def test_a_bot_pr_closed_unmerged_says_how_to_retry(self):
        pull = self.gh.get_pull(9)
        events.handle(self.ctx, "pull_request_target", {"action": "closed", "sender": OPERATOR,
                                                         "pull_request": pull})
        self.assertIn("closed without merging", self.gh.bot_comments(5)[-1])


if __name__ == "__main__":
    unittest.main()
