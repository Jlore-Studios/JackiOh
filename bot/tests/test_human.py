"""#793: `human required`. An item only a person can do (a change to `bot/` or `.github/`, a
secret, a dashboard, an account) gets the label and a comment naming the people, and the bot
leaves it alone until a person takes the label off."""

from __future__ import annotations

import unittest

from harness import config as config_mod
from harness import human, issueplan, triage, verdicts
from harness import plan as plan_mod
from harness.config import (LABEL_BLOCKED, LABEL_BUILD, LABEL_HUMAN_REQUIRED, LABEL_PR,
                            LABEL_PR_OPEN, LABEL_REVISE)
from harness.runner import FakeRunner
from harness.trust import Trust

from tests.fakes import OPERATOR, git, push_branch
from tests.test_flow import Harness, ask
from tests.test_rating import day_harness, planner, rated_plan
from tests.test_work import APPROVE, builder, reviewer

PEOPLE = "@MaxGoetzmann @jgoetzmann"
BOT_FILES = ("bot/harness/events.py", "bot/README.md", "bot/tests/test_flow.py")
FORBIDDEN = config_mod.load(env={}).forbidden_paths


def blocked(question: str, *, human_required: bool) -> str:
    flag = ', "human_required": true' if human_required else ""
    return f'<!-- bot: {{"status": "blocked"{flag}, "question": "{question}"}} -->\nStopped.'


def plan_body(files: tuple[str, ...], head: str = "") -> str:
    """A plan as the harness keeps it: its rating line taken off."""
    return head + rated_plan("medium", files=files).split("\n", 1)[1]


class PlanNeedsTests(unittest.TestCase):
    def test_a_plan_that_touches_only_forbidden_paths_needs_a_person(self):
        why = human.plan_needs(plan_body(BOT_FILES), FORBIDDEN)
        self.assertIn("`bot/harness/events.py`, `bot/README.md`", why)
        self.assertIn("a person has to make the change", why)

    def test_a_plan_with_anything_the_bot_may_change_does_not(self):
        self.assertEqual(human.plan_needs(plan_body((*BOT_FILES, "src/game.txt")), FORBIDDEN), "")
        self.assertEqual(human.plan_needs("1. Edit it.", FORBIDDEN), "")  # no table at all
        self.assertEqual(human.plan_needs("", FORBIDDEN), "")

    def test_the_planners_own_line_before_the_goal_says_it(self):
        head = "**Human required:** add `RENDER_API_KEY` in GitHub's\n  Actions secrets.\n\n"
        self.assertEqual(human.plan_needs(plan_body(("src/game.txt",), head), FORBIDDEN),
                         "add `RENDER_API_KEY` in GitHub's")
        said = "Human required: set `VITE_FLAG` in Vercel.\n\n"
        self.assertEqual(human.plan_needs(plan_body(("src/game.txt",), said), FORBIDDEN),
                         "set `VITE_FLAG` in Vercel.")
        # Past the Goal the words are the plan's own, not the planner's verdict.
        late = plan_body(("src/game.txt",)) + "\n**Human required:** nothing, really.\n"
        self.assertEqual(human.plan_needs(late, FORBIDDEN), "")
        # A planner that says nobody is needed is not taken at its word the other way.
        for nobody in ("**Human required:** none.\n\n", "Human required: no\n\n"):
            self.assertEqual(human.plan_needs(plan_body(("src/game.txt",), nobody), FORBIDDEN), "")
        # With no Goal heading, only the plan's opening lines count.
        bare = "1. Edit `src/game.txt`.\n" * 6 + "Human required: a dashboard.\n"
        self.assertEqual(human.plan_needs(bare, FORBIDDEN), "")
        self.assertEqual(human.plan_needs("Human required: a dashboard.\n" + bare, FORBIDDEN),
                         "a dashboard.")

    def test_the_comment_names_the_people_and_the_way_back(self):
        text = human.ask("Add the secret.", "jgoetzmann-bot", kept=" What I had is on `b`.")
        self.assertTrue(text.startswith(f"{PEOPLE} this needs a person: I cannot build this "
                                        "myself. What I had is on `b`."))
        self.assertIn("Add the secret.", text)
        self.assertIn(f"assigned you and labelled it `{LABEL_HUMAN_REQUIRED}`", text)
        self.assertIn("take the label off and `/harness build`", text)


class ReportTests(unittest.TestCase):
    def test_only_a_blocked_report_with_a_true_flag_counts(self):
        report = verdicts.build_report(blocked("Add the secret.", human_required=True))
        self.assertEqual((report.status, report.question, report.human_required),
                         ("blocked", "Add the secret.", True))
        self.assertFalse(verdicts.build_report(blocked("Which?", human_required=False))
                         .human_required)
        self.assertFalse(verdicts.build_report(
            '<!-- bot: {"status": "done", "human_required": true} -->').human_required)
        self.assertFalse(verdicts.build_report(
            '<!-- bot: {"status": "blocked", "human_required": "yes"} -->').human_required)


class FlowTests(unittest.TestCase):
    def handed_to_people(self, h, number: int) -> str:
        """The item is people's, as `human` makes it: `human required` on, none of the bot's
        labels that say it has the item (`bot:blocked` included), the bot unassigned and both
        people assigned, and its last comment names them. Returns that comment."""
        labels = h.gh.label_names(number)
        self.assertIn(LABEL_HUMAN_REQUIRED, labels)
        self.assertEqual(labels & human.LETS_GO, set(), labels)
        assigned = {a["login"] for a in h.gh.threads[number]["assignees"]}
        self.assertEqual(assigned, {"MaxGoetzmann", "jgoetzmann"})
        said = h.gh.bot_comments(number)[-1]
        self.assertTrue(said.startswith(PEOPLE), said)
        self.assertIn("So it is yours now: I took my labels off, unassigned myself", said)
        self.assertTrue(h.ctx.store.load()["items"][str(number)]["question"])
        return said

    def test_a_plan_that_touches_only_bot_files_is_handed_to_people_unbuilt(self):
        # #760: its plan's every file was under bot/, and two build runs found the same wall.
        h = day_harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,), body="Bring back suggestions.")
        h.gh.add_assignees(12, ["jgoetzmann-bot"])  # #760 was assigned to the bot
        planned, result = h.night(planner(rated_plan("medium", files=BOT_FILES)))
        self.assertEqual((planned["action"], result["status"]), ("plan", "blocked"))
        self.assertTrue(result["human_required"])
        said = self.handed_to_people(h, 12)
        self.assertIn("`bot/harness/events.py`", said)
        self.assertIn("the plan is in this issue's description", said)
        self.assertIn("Files to touch", issueplan.plan_of(h.gh.threads[12]["body"]))
        self.assertIn("bot:planned", h.gh.label_names(12))  # what it has stays
        self.assertNotEqual(plan_mod.make(h.ctx).get("number"), 12)

    def test_a_planner_that_says_only_a_person_can_do_it_is_believed(self):
        h = day_harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,), body="Turn on the deploy hook.")
        text = rated_plan("medium").replace(
            "## 1. Goal", "**Human required:** create `RENDER_DEPLOY_HOOK_URL` in GitHub's "
            "secrets.\n\n## 1. Goal")
        _, result = h.night(planner(text))
        self.assertEqual(result["status"], "blocked")
        self.assertIn("create `RENDER_DEPLOY_HOOK_URL` in GitHub's secrets.",
                      self.handed_to_people(h, 12))

    def test_a_builder_that_says_only_a_person_can_do_it_hands_it_on(self):
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        # #796: GitHub had made the label grey and bare when it was first put on.
        h.gh.labels[LABEL_HUMAN_REQUIRED] = {"name": LABEL_HUMAN_REQUIRED, "color": "ededed",
                                             "description": ""}
        _, result = h.night(FakeRunner({
            "build": builder({}, blocked("Set `VITE_SUPABASE_URL` in Vercel's project settings.",
                                         human_required=True)),
            "review": reviewer(APPROVE)}))
        self.assertEqual((result["status"], result["human_required"]), ("blocked", True))
        self.assertIn("Set `VITE_SUPABASE_URL` in Vercel's project settings.",
                      self.handed_to_people(h, 12))
        color, description = config_mod.LABELS[LABEL_HUMAN_REQUIRED]
        self.assertEqual(h.gh.labels[LABEL_HUMAN_REQUIRED],
                         {"name": LABEL_HUMAN_REQUIRED, "color": color,
                          "description": description})

    def test_a_build_whose_whole_change_the_path_guard_put_back_hands_it_on(self):
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        _, result = h.night(FakeRunner({"build": builder({"bot/harness/new.py": "x = 1\n"}),
                                        "review": reviewer(APPROVE)}))
        self.assertEqual((result["status"], result["human_required"]), ("blocked", True))
        self.assertIn("is in paths the bot may not change", self.handed_to_people(h, 12))

    def test_a_question_a_person_can_answer_is_still_only_a_question(self):
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        h.night(FakeRunner({"build": builder({}, blocked("Option A or B?", human_required=False)),
                            "review": reviewer(APPROVE)}))
        labels = h.gh.label_names(12)
        self.assertIn(LABEL_BLOCKED, labels)
        self.assertNotIn(LABEL_HUMAN_REQUIRED, labels)
        said = h.gh.bot_comments(12)[-1]
        self.assertTrue(said.startswith("I need a decision before I can build this."))
        self.assertNotIn(PEOPLE, said)

    def test_a_revision_only_a_person_can_make_hands_the_pull_request_on(self):
        h = Harness(self)
        start = push_branch(h.origin, h.root, "bot/issue-12", {"src/game.txt": "v2\n"})
        git(h.clone, "fetch", "-q", "origin")
        git(h.deliver_repo, "fetch", "-q", "origin")
        h.gh.add_issue(12, labels=(LABEL_PR_OPEN,))
        h.gh.add_pull(40, "bot/issue-12", body="Closes #12", labels=(LABEL_PR, LABEL_REVISE),
                      sha=start)
        h.gh.add_comment(40, "@jgoetzmann-bot also run it in CI", OPERATOR)
        _, result = h.night(FakeRunner({
            "revise": builder({}, blocked("Add a job to `.github/workflows/ci.yml`.",
                                          human_required=True)),
            "review": reviewer(APPROVE)}))
        self.assertEqual(result["status"], "blocked")
        self.assertEqual(h.gh.label_names(40), {LABEL_PR, LABEL_HUMAN_REQUIRED})  # `bot:pr` stays
        said = h.gh.bot_comments(40)[-1]
        self.assertTrue(said.startswith(f"{PEOPLE} this needs a person: I cannot revise this"))
        self.assertIn("`/harness revise <notes>`", said)

    def test_the_bot_leaves_it_alone_until_the_label_comes_off(self):
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_HUMAN_REQUIRED,))
        ask(h, 12, "@jgoetzmann-bot exception for this issue", 51)
        self.assertIn(f"#12 is labelled `{LABEL_HUMAN_REQUIRED}`", h.gh.bot_comments(12)[-1])
        self.assertNotIn(LABEL_BUILD, h.gh.label_names(12))
        # A queue label put on by hand is not picked up while it is on.
        h.gh.add_labels(12, [LABEL_BUILD])
        skipped: list[str] = []
        self.assertEqual(plan_mod.candidates(h.ctx, h.ctx.store.load(), skipped), [])
        self.assertIn(f"#12 skipped: labelled `{LABEL_HUMAN_REQUIRED}`", skipped[0])
        # A person did their part and took it off: the next request queues it.
        h.gh.remove_label(12, LABEL_HUMAN_REQUIRED)
        ask(h, 12, "/harness build", 52)
        self.assertIn("Queued #12", h.gh.bot_comments(12)[-1])
        self.assertEqual(plan_mod.make(h.ctx)["number"], 12)

    def test_a_persons_human_required_hands_the_issue_to_people_as_human_does(self):
        payload = {"action": "labeled", "label": {"name": "Human required"},
                   "sender": {"login": "MaxGoetzmann"},
                   "issue": {"number": 12, "labels": [], "user": {"login": "MaxGoetzmann"}}}
        self.assertEqual(triage.gate(payload, Trust.parse(""), "jgoetzmann-bot"),
                         ("human", "#12: @MaxGoetzmann labelled it human required"))
        thread = {"number": 12, "state": "open", "assignees": [{"login": "jgoetzmann-bot"}],
                  "labels": [{"name": n} for n in (LABEL_HUMAN_REQUIRED, LABEL_BUILD)]}
        plan = triage.decide("human", thread, False, "jgoetzmann-bot")
        self.assertEqual((plan.assignees, plan.unassign, plan.remove),
                         (["MaxGoetzmann", "jgoetzmann"], ["jgoetzmann-bot"], [LABEL_BUILD]))
        # The bot's own hand-over is the bot's to make: triage leaves it alone.
        payload["sender"] = {"login": "jgoetzmann-bot"}
        self.assertEqual(triage.gate(payload, Trust.parse(""), "jgoetzmann-bot")[0], "")

    def test_the_label_is_one_the_sweep_makes(self):
        color, description = config_mod.LABELS[LABEL_HUMAN_REQUIRED]
        self.assertEqual(len(color), 6)
        self.assertLessEqual(len(description), config_mod.LABEL_DESCRIPTION_MAX)


if __name__ == "__main__":
    unittest.main()
