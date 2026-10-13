"""#797: a survey once a free lane has had nothing to start for 30 minutes, and a log of the
suggestions that came to nothing, which no survey offers again."""

from __future__ import annotations

import json
import unittest
from datetime import timedelta

from harness import declined
from harness import plan as plan_mod
from harness.clock import iso
from harness.config import LABEL_BUILD, LABEL_HUMAN_REQUIRED, LABEL_SUGGESTION
from harness.runner import FakeRunner, RunResult

from tests.support import idle_for
from tests.test_flow import Harness, ask
from tests.test_rating import day_harness, rated_plan
from tests.test_work import reviewer


def survey(*titles: str) -> FakeRunner:
    text = ('<!-- suggestions: ['
            + ",".join(json.dumps({"title": t, "body": "## Why\nx"}) for t in titles) + '] -->')
    return FakeRunner({"suggest": reviewer(text)})


class IdleTests(unittest.TestCase):
    def test_queued_work_that_waits_for_another_subscription_no_longer_stops_a_survey(self):
        # By day only Muse is free, and a hard item needs a strong model: Muse has nothing it can
        # start, so after 30 minutes it surveys, though the queue is not empty.
        h = day_harness(self, machine=("muse",))
        h.gh.add_issue(12, labels=(LABEL_BUILD, "difficulty:hard"))
        look = plan_mod.peek(h.ctx)
        self.assertEqual((look.work, look.idle), (False, True))
        idle_for(h.ctx, 29)
        self.assertFalse(plan_mod.peek(h.ctx).work)
        idle_for(h.ctx, 30)
        planned = plan_mod.make(h.ctx)
        self.assertEqual((planned["action"], planned["provider"], planned["count"]),
                         ("suggest", "muse", 4))

    def test_no_second_survey_while_one_runs(self):
        h = Harness(self)
        idle_for(h.ctx, 45)
        h.ctx.store.update(lambda s: s["suggest"].update(provider="claude-3", run_id="9"))
        self.assertEqual(plan_mod.suggestions_due(h.ctx, h.ctx.store.load()), 0)
        self.assertEqual(plan_mod.suggestions_due(h.ctx, h.ctx.store.load(), force=True), 4)

    def test_the_cap_still_holds(self):
        h = Harness(self)
        for n in (20, 21, 22, 23):
            h.gh.add_issue(n, labels=(LABEL_SUGGESTION,))
        idle_for(h.ctx, 120)
        self.assertEqual(plan_mod.make(h.ctx)["action"], "none")

    def test_an_unfinished_survey_gives_the_clock_back(self):
        h = Harness(self)
        idle_for(h.ctx, 45)
        before = h.ctx.store.load()["suggest"]["idle_since"]
        planned, _ = h.night(FakeRunner({"suggest": lambda r: RunResult(False, "", 1,
                                                                        error="boom")}))
        self.assertEqual(planned["action"], "suggest")
        self.assertEqual(h.ctx.store.load()["suggest"]["idle_since"], before)


class LogTests(unittest.TestCase):
    def test_a_reject_vote_is_logged(self):
        h = Harness(self)
        h.gh.add_issue(21, "Patch v0.3.Y: shinier coins", labels=(LABEL_SUGGESTION,))
        ask(h, 21, "reject", 61)
        [entry] = declined.entries(h.ctx.store.load())
        self.assertEqual((entry["number"], entry["title"], entry["why"]),
                         (21, "Patch v0.3.Y: shinier coins", "rejected by @jgoetzmann"))

    def test_the_survey_reads_every_one_and_a_repeat_is_not_opened(self):
        h = Harness(self)
        # Closed by hand long ago (#777's way), and rejected by a vote.
        h.gh.add_issue(30, "Architecture: a fuzz monitor for piles", state="closed",
                       labels=(LABEL_SUGGESTION,))
        h.gh.threads[30]["state_reason"] = "not_planned"
        h.gh.add_issue(31, "Patch v0.3.Y: shinier coins", labels=(LABEL_SUGGESTION,))
        ask(h, 31, "reject", 62)
        idle_for(h.ctx)
        runner = survey("Patch v0.3.Y: Shinier coins!", "Patch v0.3.Y: a new idea")
        planned, result = h.night(runner)
        prompt = runner.calls[0].prompt
        self.assertIn("## Suggestions that came to nothing", prompt)
        self.assertIn("#30 Architecture: a fuzz monitor for piles: closed unbuilt by a person "
                      "(not planned)", prompt)
        self.assertIn("#31 Patch v0.3.Y: shinier coins: rejected by @jgoetzmann", prompt)
        made = [t["title"] for t in h.gh.threads.values()
                if LABEL_SUGGESTION in {l["name"] for l in t["labels"]} and t["state"] == "open"]
        self.assertEqual(made, ["Patch v0.3.Y: a new idea"])  # the reworded repeat was dropped

    def test_an_approved_suggestion_the_bot_cannot_build_is_logged(self):
        h = day_harness(self)
        h.gh.add_issue(12, "Night bot: tune the sweep", labels=(LABEL_SUGGESTION,))
        ask(h, 12, "approve", 63)
        h.night(FakeRunner({"plan": lambda r: RunResult(
            True, rated_plan("medium", files=("bot/harness/sweep.py",)))}))
        self.assertIn(LABEL_HUMAN_REQUIRED, h.gh.label_names(12))
        [entry] = declined.entries(h.ctx.store.load())
        self.assertEqual(entry["number"], 12)
        self.assertTrue(entry["why"].startswith("the bot could not build it: Every file its plan "
                                                "touches"))

    def test_a_plain_issue_handed_to_people_is_not_a_declined_suggestion(self):
        h = day_harness(self)
        h.gh.add_issue(12, "Night bot: tune the sweep", labels=(LABEL_BUILD,))
        h.night(FakeRunner({"plan": lambda r: RunResult(
            True, rated_plan("medium", files=("bot/harness/sweep.py",)))}))
        self.assertIn(LABEL_HUMAN_REQUIRED, h.gh.label_names(12))
        self.assertEqual(declined.entries(h.ctx.store.load()), [])

    def test_the_log_keeps_one_entry_each_and_the_newest(self):
        state = {"suggest": {}}
        for n in range(declined.LIMIT + 5):
            declined.note(state, n, f"Idea {n}", "rejected", iso(plan_mod.parse_iso(
                "2026-10-13T00:00:00Z") + timedelta(minutes=n)))
        self.assertFalse(declined.note(state, declined.LIMIT + 4, "Again", "rejected", "now"))
        log = declined.entries(state)
        self.assertEqual((len(log), log[0]["number"]), (declined.LIMIT, 5))
        self.assertTrue(declined.known(state, "idea 7"))
        self.assertFalse(declined.known(state, "idea 4"))  # dropped as the oldest


if __name__ == "__main__":
    unittest.main()
