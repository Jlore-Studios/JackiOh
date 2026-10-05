"""Each Claude account runs Opus and Sonnet both, and its run switches between them as the work
needs: the builder may ask for the other model for its next pass, two rounds the reviewer sends
back on Sonnet move the builder to Opus, and a planner's rating moves it to the model that fits,
never under the item's difficulty. The reviewer stays as assigned: the review rule counts its tier.
"""

from __future__ import annotations

import json
import unittest

from harness import plan as plan_mod
from harness import verdicts
from harness.config import LABEL_BUILD
from harness.deliver import Deliverer
from harness.runner import FakeRunner, RunResult
from harness.state import item as state_item
from harness.work import SWITCH_UP_AFTER, Worker

from tests.support import DAY
from tests.test_cross import ALL
from tests.test_flow import Harness
from tests.test_work import APPROVE, builder, changes, reviewer

MEDIUM, HARD = "difficulty:medium", "difficulty:hard"


def done(**extra: str) -> str:
    header = {"status": "done", "title": "Make the rules v2", **extra}
    return f"<!-- bot: {json.dumps(header)} -->\n## What changed\nRules v2."


def models(runner: FakeRunner, role: str) -> list[str]:
    return [request.model for request in runner.calls if request.role == role]


def rated(difficulty: str):
    plan = (f'<!-- bot: {{"difficulty": "{difficulty}", "why": "it reads the engine"}} -->\n'
            "1. **Goal.** Do what the issue asks.\n2. **Steps.** Change it, test it.")
    return lambda request: RunResult(True, plan)


class ReportTests(unittest.TestCase):
    def test_the_report_header_carries_the_model_asked_for(self):
        self.assertEqual(verdicts.build_report(done(next_model="Opus")).next_model, "opus")
        self.assertEqual(verdicts.build_report(done()).next_model, "")


class SwitchTests(unittest.TestCase):
    def harness(self) -> Harness:
        return Harness(self, env=ALL, max_review_cycles=4)

    def test_the_builder_asks_for_opus_and_its_next_pass_runs_on_it(self):
        h = self.harness()
        h.gh.add_issue(12, labels=(LABEL_BUILD, MEDIUM))
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}, done(next_model="opus")),
                             "fix": builder({"src/game.txt": "v3\n"}),
                             "review": [reviewer(changes("v2 misses a rule")), reviewer(APPROVE)]})
        planned, result = h.night(runner)
        self.assertEqual((planned["provider"], planned["seats"]["build"]["model"]),
                         ("claude-3", "sonnet"))
        self.assertEqual(result["status"], "approved")
        self.assertEqual((models(runner, "build"), models(runner, "fix")), (["sonnet"], ["opus"]))
        self.assertEqual(models(runner, "review"), ["opus", "opus"])
        self.assertEqual(result["switches"], [{"n": 1, "from": "sonnet", "to": "opus",
                                               "why": "the builder asked for it"}])
        self.assertEqual(result["cycles"][0]["builder"]["next_model"], "opus")
        self.assertEqual([c["builder"]["model"] for c in result["cycles"]], ["sonnet", "opus"])
        self.assertIn("after round 1 it switched to `opus`: the builder asked for it",
                      h.gh.bot_comments(12)[-1])

    def test_rounds_sent_back_on_sonnet_move_the_builder_to_opus(self):
        h = self.harness()
        h.gh.add_issue(12, labels=(LABEL_BUILD, MEDIUM))
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                             "fix": [builder({"src/game.txt": f"v{n}\n"}) for n in (3, 4)],
                             "review": [reviewer(changes("not yet")), reviewer(changes("still no")),
                                        reviewer(APPROVE)]})
        _, result = h.night(runner)
        self.assertEqual(SWITCH_UP_AFTER, 2)
        self.assertEqual(result["status"], "approved")
        self.assertEqual((models(runner, "build"), models(runner, "fix")),
                         (["sonnet"], ["sonnet", "opus"]))
        self.assertEqual(result["switches"], [{
            "n": 2, "from": "sonnet", "to": "opus",
            "why": "the reviewer sent back 2 rounds in a row on `sonnet`"}])

    def test_a_hard_item_keeps_opus_whatever_the_builder_asks(self):
        h = self.harness()
        h.gh.add_issue(12, labels=(LABEL_BUILD, HARD))
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}, done(next_model="sonnet")),
                             "fix": builder({"src/game.txt": "v3\n"}),
                             "review": [reviewer(changes("not yet")), reviewer(APPROVE)]})
        _, result = h.night(runner)
        self.assertEqual((models(runner, "build"), models(runner, "fix")), (["opus"], ["opus"]))
        self.assertEqual(result["switches"], [])
        build_prompt = next(r.prompt for r in runner.calls if r.role == "build")
        self.assertIn("keeps its builder on `opus`", build_prompt)

    def test_opus_on_a_medium_item_may_hand_the_plain_work_to_sonnet(self):
        h = self.harness()
        h.gh.add_issue(12, labels=(LABEL_BUILD, MEDIUM))
        planned = plan_mod.make(h.ctx)
        planned["seats"]["build"] = {**planned["seats"]["build"], "model": "opus", "tier": "strong"}
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}, done(next_model="sonnet")),
                             "fix": builder({"src/game.txt": "v3\n"}),
                             "review": [reviewer(changes("a typo")), reviewer(APPROVE)]})
        out = h.root / "out-opus"
        Worker(h.cfg, planned, runner, h.clone, h.root / "work", out).run()
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        result = json.loads((out / "result.json").read_text())
        self.assertEqual((models(runner, "build"), models(runner, "fix")), (["opus"], ["sonnet"]))
        self.assertEqual(result["switches"][0]["to"], "sonnet")

    def test_sonnet_gets_its_two_tries_after_opus_hands_it_the_work(self):
        """The builder's ask is taken once its round is judged, so a round Opus built and the
        reviewer sent back counts against Opus, not Sonnet."""
        h = self.harness()
        h.gh.add_issue(12, labels=(LABEL_BUILD, MEDIUM))
        planned = plan_mod.make(h.ctx)
        planned["seats"]["build"] = {**planned["seats"]["build"], "model": "opus", "tier": "strong"}
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}, done(next_model="sonnet")),
                             "fix": [builder({"src/game.txt": f"v{n}\n"}) for n in (3, 4, 5)],
                             "review": [reviewer(changes("one")), reviewer(changes("two")),
                                        reviewer(changes("three")), reviewer(APPROVE)]})
        out = h.root / "out-tries"
        Worker(h.cfg, planned, runner, h.clone, h.root / "work", out).run()
        result = json.loads((out / "result.json").read_text())
        self.assertEqual((models(runner, "build"), models(runner, "fix")),
                         (["opus"], ["sonnet", "sonnet", "opus"]))
        self.assertEqual([(m["to"], m["why"]) for m in result["switches"]], [
            ("sonnet", "the builder asked for it"),
            ("opus", "the reviewer sent back 2 rounds in a row on `sonnet`")])

    def test_an_item_the_bot_rated_and_its_planner_rates_harder_is_built_on_opus(self):
        """The bot rated it medium (a step up, say); the run's planner rates it hard. The run
        moves its builder to Opus rather than building a hard item on Sonnet."""
        h = self.harness()
        h.gh.add_issue(12, labels=(LABEL_BUILD, MEDIUM))
        h.ctx.store.update(lambda s: state_item(s, 12).update(
            difficulty_by={"difficulty": "medium", "provider": "step-up"}))
        runner = FakeRunner({"plan": rated("hard"), "build": builder({"src/game.txt": "v2\n"}),
                             "review": reviewer(APPROVE)})
        planned, result = h.night(runner)
        self.assertEqual((planned["seats"]["build"]["model"], planned["rating"]["source"]),
                         ("sonnet", "bot"))
        self.assertEqual(models(runner, "build"), ["opus"])
        self.assertEqual(result["switches"][0]["to"], "opus")

    def test_the_planners_rating_moves_the_builder_instead_of_sending_it_back(self):
        """An unrated item starts on Sonnet; its planner rates it hard, so the same run builds it
        on the account's Opus rather than queueing it again for a strong builder."""
        h = self.harness()
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        runner = FakeRunner({"plan": rated("hard"), "build": builder({"src/game.txt": "v2\n"}),
                             "review": reviewer(APPROVE)})
        planned, result = h.night(runner)
        self.assertEqual(planned["seats"]["build"]["model"], "sonnet")
        self.assertEqual(result["status"], "approved")
        self.assertEqual(models(runner, "build"), ["opus"])
        self.assertEqual(result["switches"], [{"n": 0, "from": "sonnet", "to": "opus",
                                               "why": "its planner rated it difficulty:hard"}])
        self.assertIn(HARD, h.gh.label_names(12) | h.gh.label_names(
            max(n for n, t in h.gh.threads.items() if "pull_request" in t)))
        self.assertIn("before round 1 it switched to `opus`", "\n".join(h.gh.bot_comments(12)))

    def test_the_builder_is_told_it_may_switch(self):
        h = self.harness()
        h.gh.add_issue(12, labels=(LABEL_BUILD, MEDIUM))
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                             "review": reviewer(APPROVE)})
        h.night(runner)
        build_prompt = next(r.prompt for r in runner.calls if r.role == "build")
        self.assertIn("## Your model", build_prompt)
        self.assertIn("`opus` (strong) and `sonnet` (medium)", build_prompt)
        self.assertIn('"next_model": "opus"', build_prompt)
        # Its reviewer reads no such note: it reviews, it does not choose.
        review_prompt = next(r.prompt for r in runner.calls if r.role == "review")
        self.assertNotIn("## Your model", review_prompt)

    def test_a_lane_with_one_model_never_switches(self):
        h = Harness(self, env={"HARNESS_SECRETS_SET": ""}, machine=("muse",), at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))  # unrated, so a medium model may plan it
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}, done(next_model="opus")),
                             "fix": builder({"src/game.txt": "v3\n"}),
                             "review": [reviewer(changes("not yet")), reviewer(APPROVE)]})
        planned, result = h.night(runner)
        self.assertEqual(planned["provider"], "muse")
        self.assertEqual(set(models(runner, "build") + models(runner, "fix")),
                         {"muse-spark-1.3-contributor"})
        self.assertEqual(result.get("switches"), [])
        self.assertNotIn("## Your model", next(r.prompt for r in runner.calls if r.role == "build"))


if __name__ == "__main__":
    unittest.main()
