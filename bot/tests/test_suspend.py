"""#294: `/harness suspend <subscription>` starts no new work on one subscription until
`/harness resume <subscription>`, `--force` included; a halt and a suspension never lift each
other."""

from __future__ import annotations

import unittest

from harness import commands, dashboard, events
from harness import plan as plan_mod
from harness import providers
from harness.__main__ import make_probe
from harness.clock import iso
from harness.config import LABEL_BUILD
from harness.deliver import Deliverer
from harness.runner import FakeRunner
from harness.state import item as state_item
from harness.status import report
from harness.work import Worker

from tests.fakes import OPERATOR, FakeGitHub
from tests.support import NIGHT, make_ctx
from tests.test_flow import Harness
from tests.test_work import APPROVE, builder, reviewer

TRUST = "jgoetzmann 3 id:95732896\nhelper 2\n"
HELPER = {"login": "helper", "id": 7}
BOT_NAME = "jgoetzmann-bot"


def suspend(ctx, provider_id: str, reason: str = "") -> None:
    held = {"by": "jgoetzmann", "at": iso(ctx.now()), "reason": reason}
    ctx.store.update(lambda s: providers.record(s, provider_id).update(suspended=held))


def verbs(body: str) -> list[tuple[str, str]]:
    return [(c.verb, c.args) for c in commands.parse(body, BOT_NAME)]


class ParseTests(unittest.TestCase):
    def test_suspend_is_an_operator_command_after_either_prefix(self):
        self.assertEqual(verbs("/harness suspend claude-3 using it myself"),
                         [("suspend", "claude-3 using it myself")])
        self.assertEqual(verbs("@jgoetzmann-bot suspend claude-3"), [("suspend", "claude-3")])
        self.assertEqual(verbs("@jgoetzmann-bot suspend: claude-3 using it myself"),
                         [("suspend", "claude-3 using it myself")])
        self.assertEqual(verbs("@jgoetzmann-bot suspend the old animation")[0][0], "request")
        self.assertEqual(commands.parse("/harness suspend gpt", BOT_NAME)[0].level, 3)

    def test_resume_with_a_subscription_is_start_with_its_name(self):
        self.assertEqual(verbs("/harness resume claude-3"), [("start", "claude-3")])
        self.assertEqual(verbs("@jgoetzmann-bot resume claude-3"), [("start", "claude-3")])
        self.assertEqual(verbs("@jgoetzmann-bot start with option A")[0][0], "request")

    def test_help_explains_suspend(self):
        text = commands.help_text(BOT_NAME, "suspend")
        self.assertIn("`suspend <subscription> [reason]`", text)
        self.assertIn("For example: `@jgoetzmann-bot suspend: claude-3", text)
        self.assertIn("`suspend <subscription> [reason]`", commands.help_text(BOT_NAME))


class CommandTests(unittest.TestCase):
    def setUp(self):
        self.gh = FakeGitHub()
        self.gh.add_issue(5)
        self.ctx = make_ctx(self.gh, at=NIGHT, trust_text=TRUST)
        self.cid = 600

    def send(self, body: str, user=OPERATOR, association="OWNER") -> None:
        # A fresh comment id each time: a line already answered on one id never runs again.
        self.cid += 1
        events.handle(self.ctx, "issue_comment", {
            "action": "created", "sender": user, "issue": {"number": 5, "title": "t"},
            "comment": {"id": self.cid, "body": body, "user": user,
                        "author_association": association}})

    def reply(self) -> str:
        return self.gh.bot_comments(5)[-1]

    def why(self, provider_id: str, forced: bool = False) -> str | None:
        cfg = self.ctx.cfg
        return providers.availability(cfg.pool.get(provider_id), self.ctx.store.load(), NIGHT,
                                      cfg.timezone, cfg.secrets, forced=forced)

    def suspended(self, provider_id: str) -> dict | None:
        return providers.suspension(self.ctx.store.load(), provider_id)

    def test_suspend_and_resume_one_subscription(self):
        self.assertIsNone(self.why("claude-1"))
        self.send("/harness suspend claude-1 using it myself")
        self.assertIn("Suspended `claude-1`", self.reply())
        held = self.ctx.store.load()["providers"]["claude-1"]["suspended"]
        self.assertEqual((held["by"], held["reason"]), ("jgoetzmann", "using it myself"))
        self.assertEqual(self.why("claude-1"), "suspended by @jgoetzmann (using it myself); "
                                               "`/harness resume claude-1` lifts it")
        self.assertIn("suspended", self.why("claude-1", forced=True))
        self.assertIn("suspended by @jgoetzmann", report(self.ctx))
        self.assertFalse(self.ctx.store.load().get("halted"))
        self.send("/harness resume claude-1")
        self.assertIn("Resumed `claude-1`", self.reply())
        self.assertNotIn("suspended", self.ctx.store.load()["providers"]["claude-1"])
        self.assertIsNone(self.why("claude-1"))

    def test_the_mention_form_suspends_with_a_reason_after_a_colon(self):
        self.send("@jgoetzmann-bot suspend: claude-2 for the demo")
        self.assertEqual(self.suspended("claude-2")["reason"], "for the demo")
        self.send("@jgoetzmann-bot resume claude-2")
        self.assertIsNone(self.suspended("claude-2"))

    def test_suspend_needs_the_operator(self):
        self.send("/harness suspend claude-1", user=HELPER, association="COLLABORATOR")
        self.assertIn("operator level", self.reply())
        self.assertIsNone(self.suspended("claude-1"))

    def test_an_unknown_or_missing_subscription_changes_nothing(self):
        self.send("/harness suspend claude-9")
        self.assertIn("`claude-9` is not a subscription", self.reply())
        self.assertIn("`claude-1`", self.reply())
        self.send("/harness suspend")
        self.assertIn("needs a subscription", self.reply())
        self.assertEqual(self.ctx.store.load()["providers"], {})

    def test_resume_of_a_subscription_leaves_a_halt_alone(self):
        self.send("/harness halt away")
        self.send("/harness suspend claude-1")
        self.send("/harness resume claude-1")
        self.assertIn("still halted", self.reply())
        self.assertTrue(self.ctx.store.load()["halted"])
        self.assertIsNone(self.suspended("claude-1"))
        self.send("/harness resume over")
        self.assertIn("`over` is not a subscription", self.reply())
        self.assertTrue(self.ctx.store.load()["halted"])
        # A bare start, or one with a note that names no subscription, lifts the halt as it always
        # did, and never a suspension.
        self.send("/harness suspend gpt")
        self.send("/harness start now, back from vacation")
        self.assertIn("Started.", self.reply())
        self.assertFalse(self.ctx.store.load()["halted"])
        self.assertIsNotNone(self.suspended("gpt"))
        self.send("/harness halt again")
        self.send("/harness start")
        self.assertFalse(self.ctx.store.load()["halted"])
        self.assertIsNotNone(self.suspended("gpt"))

    def test_resume_of_one_not_suspended_says_so(self):
        self.send("/harness resume claude-1")
        self.assertIn("`claude-1` is not suspended", self.reply())


class SchedulingTests(unittest.TestCase):
    def setUp(self):
        self.gh = FakeGitHub()
        self.ctx = make_ctx(self.gh)

    def test_a_suspended_subscription_takes_no_work_even_forced(self):
        self.gh.add_issue(3, labels=(LABEL_BUILD,))
        suspend(self.ctx, "claude-1")
        planned = plan_mod.make(self.ctx, force=True)
        self.assertEqual(planned["action"], "none")
        self.assertIn("`claude-1` suspended by @jgoetzmann", planned["reason"])
        self.ctx.store.update(lambda s: providers.record(s, "claude-1").pop("suspended"))
        self.assertEqual(plan_mod.make(self.ctx)["action"], "build")

    def test_the_probe_stops_a_run_on_a_suspended_subscription(self):
        self.gh.add_issue(5)
        pool = self.ctx.cfg.pool
        self.assertIsNone(make_probe(self.ctx, 5, pool.get("claude-1"))(None))
        suspend(self.ctx, "claude-1")
        self.assertEqual(make_probe(self.ctx, 5, pool.get("claude-1"))(None),
                         ("`claude-1` was suspended by @jgoetzmann", "suspend"))
        self.assertIsNone(make_probe(self.ctx, 5, pool.get("claude-2"))(None))

    def test_the_status_issue_shows_a_suspended_account_as_off(self):
        suspend(self.ctx, "claude-1")
        self.assertEqual(dashboard._account(self.ctx, self.ctx.store.load(),
                                            self.ctx.cfg.pool.get("claude-1")),
                         ("off", "⚫ suspended"))


class RunTests(unittest.TestCase):
    def run_with_probe(self, h: Harness, runner: FakeRunner) -> tuple[dict, dict]:
        planned = plan_mod.make(h.ctx)
        self.assertEqual(planned["provider"], "claude-1")
        out = h.root / "out-suspend"
        result = Worker(h.cfg, planned, runner, h.clone, h.root / "work", out,
                        probe=make_probe(h.ctx, 12, h.cfg.pool.get("claude-1"))).run()
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        return planned, result

    def test_a_run_going_when_its_subscription_is_suspended_pauses_uncharged(self):
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))

        def building(request):
            suspend(h.ctx, "claude-1")
            return builder({"src/game.txt": "v2\n"})(request)
        runner = FakeRunner({"build": building, "review": reviewer(APPROVE)})
        _, result = self.run_with_probe(h, runner)
        self.assertEqual((result["status"], result["interrupt"]), ("interrupted", "suspend"))
        self.assertEqual([c.role for c in runner.calls], ["plan", "build"])  # no review
        self.assertIn(LABEL_BUILD, h.gh.label_names(12))
        self.assertIn("Paused: `claude-1` was suspended by @jgoetzmann", h.gh.bot_comments(12)[-1])
        record = state_item(h.ctx.store.load(), 12)
        self.assertEqual(int(record.get("interruptions") or 0), 0)
        self.assertEqual(int(record.get("failures") or 0), 0)
        self.assertIsNotNone(h.origin_sha("bot/issue-12"))  # the work so far is kept

    def test_a_finished_change_is_still_handed_on(self):
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))

        def reviewing(request):
            suspend(h.ctx, "claude-1")
            return reviewer(APPROVE)(request)
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}), "review": reviewing})
        _, result = self.run_with_probe(h, runner)
        self.assertEqual(result["status"], "approved")
        self.assertEqual(len(h.gh.list_pulls(head="bot/issue-12")), 1)


if __name__ == "__main__":
    unittest.main()
