"""A planning run cut off mid-call keeps its draft, and the next planner goes on from it.

#37 was planned three times on 2026-10-06 and never got a plan: `claude-1` read for 20 minutes
and was stopped at its 40% cap, `claude-4` started just under its 70% cap and was stopped four
minutes in, and `claude-5` stopped before it started. The planner was a reader, so it wrote
nothing down; its transcript's end was never kept; and each run started from scratch. Now the
planner keeps a draft (`work.PLAN_DRAFT_FILE`), a cut-off run hands it on as a `draft` handoff
and in the journal, and the planning lane starts only `start_headroom` under each cap.
"""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from harness import issueplan
from harness import plan as plan_mod
from harness.config import LABEL_BUILD
from harness.deliver import Deliverer
from harness.journal import BRANCH as JOURNAL_BRANCH
from harness.runner import FakeRunner, RunResult
from harness.state import item as state_item
from harness.work import DRAFT_ASK, PLAN_DRAFT_FILE, PLANNER_TOOLS, Worker

from tests.fakes import FakeGitHub, git, make_origin
from tests.support import DAY, MACHINE, make_config, make_ctx
from tests.test_flow import Harness
from tests.test_providers import secrets
from tests.test_work import GATES

DRAFT = "## Goal\nRules v2.\n\n## Files to touch\n| `src/game.txt` | change | the rules |"
PLAN = "1. **Goal.** Rules v2.\n2. **Steps.** Edit src/game.txt; run the checks."


def cut_off_planner(request):
    """A planner that writes its draft, strays into a tracked file, and is stopped at its cap."""
    (request.cwd / PLAN_DRAFT_FILE).write_text(DRAFT + "\n")
    (request.cwd / "src" / "game.txt").write_text("a planner's edit\n")
    return RunResult(False, "", 1, duration_s=1200, error="stopped mid-call",
                     extra={"usage_stop": "5-hour usage is 70%, at or over its 70% cap"})


class TrailRunner(FakeRunner):
    """A fake that leaves a transcript, whose end is something to hand on."""

    def run(self, request):
        request.transcript.parent.mkdir(parents=True, exist_ok=True)
        request.transcript.write_text("{}\n")
        return super().run(request)

    def trail(self, transcript: Path) -> str:
        return "- Read: SPEC.md §4\n- said: the change is in src/game.txt"


def planning_harness(test):
    h = Harness(test, env=secrets("CLAUDE_CODE_OAUTH_TOKEN_2"), machine=MACHINE, at=DAY,
                plan_lanes=None)
    h.committed_hours()
    h.gh.add_issue(12, labels=(LABEL_BUILD,), body="Make the rules v2.")
    return h


class PlanDraftTests(unittest.TestCase):
    def test_a_cut_off_plan_keeps_its_draft_and_the_next_planner_goes_on_from_it(self):
        h = planning_harness(self)
        runner = TrailRunner({"plan": cut_off_planner})
        planned, first = h.night(runner)
        self.assertEqual((planned["action"], first["status"], first["interrupt"]),
                         ("plan", "interrupted", "usage"))
        call = runner.calls[0]
        self.assertEqual(call.allowed_tools, PLANNER_TOOLS)
        self.assertIn("Write", call.allowed_tools)
        self.assertNotIn("Write", call.disallowed_tools)
        self.assertIn("Bash(git commit:*)", call.disallowed_tools)
        self.assertIn(DRAFT_ASK.strip(), call.prompt)
        self.assertIn(f"The one file you write is your draft, `{PLAN_DRAFT_FILE}`", call.prompt)
        self.assertEqual(first["handoff"]["draft"], DRAFT)
        # The draft is kept for the next run, with the end of the planner's session.
        record = h.ctx.store.load()["items"]["12"]
        self.assertEqual(record["handoff"]["kind"], "draft")
        self.assertEqual(record["handoff"]["draft"], DRAFT)
        self.assertIn("the change is in src/game.txt", record["handoff"]["trail"])
        self.assertIn("Its draft so far is kept, and the next planning run goes on from it.",
                      h.gh.bot_comments(12)[-1])
        journal, _ = h.gh.get_file("12.md", JOURNAL_BRANCH)
        self.assertIn("The planner's draft when it stopped", journal)
        self.assertIn("Rules v2.", journal)
        # Nothing was built, and the planner's stray edit went nowhere.
        self.assertIsNone(h.origin_sha("bot/issue-12"))
        self.assertNotIn("a planner's edit", git(h.origin, "show", "main:src/game.txt"))
        self.assertIn(LABEL_BUILD, h.gh.label_names(12))

        seen = {}

        def finishing(request):
            seen["draft"] = (request.cwd / PLAN_DRAFT_FILE).read_text()
            seen["prompt"] = request.prompt
            return RunResult(True, PLAN)
        planned, second = h.night(FakeRunner({"plan": finishing}))
        self.assertEqual((planned["action"], second["status"]), ("plan", "planned"))
        self.assertEqual(planned["handoff"]["kind"], "draft")
        self.assertEqual(seen["draft"].strip(), DRAFT)
        self.assertIn("## An unfinished plan from an earlier run", seen["prompt"])
        self.assertIn("`claude-2` (claude) was cut off before it finished", seen["prompt"])
        self.assertIn(f"which is now `{PLAN_DRAFT_FILE}` at the top of your worktree",
                      seen["prompt"])
        self.assertIn("the change is in src/game.txt", seen["prompt"])
        # The finished plan supersedes the draft everywhere.
        self.assertNotIn("draft", second["handoff"])
        self.assertFalse((h.root / "work" / "item-12" / PLAN_DRAFT_FILE).exists())
        record = h.ctx.store.load()["items"]["12"]
        self.assertEqual(record["handoff"]["kind"], "plan")
        self.assertNotIn("draft", record["handoff"])
        self.assertIn("Edit src/game.txt", issueplan.plan_of(h.gh.threads[12]["body"]))

    def test_a_run_stopped_before_it_starts_keeps_the_last_draft(self):
        """#37's third planning run (`claude-5`) stopped before any model call: the draft the
        run before left stays for the one after."""
        h = planning_harness(self)
        h.night(FakeRunner({"plan": cut_off_planner}))
        planned = plan_mod.make(h.ctx)
        self.assertEqual(planned["action"], "plan")
        out = h.root / "out-stopped"
        worker = Worker(h.cfg, planned, FakeRunner({}), h.clone, h.root / "work", out)
        worker.start_usage = {"status": "allowed", "five_hour": {
            "utilization": 0.95, "resets_at": "2099-01-01T00:00:00Z"}}
        result = worker.run()
        self.assertEqual((result["status"], result["model_calls"]), ("interrupted", 0))
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        handoff = h.ctx.store.load()["items"]["12"]["handoff"]
        self.assertEqual((handoff["kind"], handoff["draft"]), ("draft", DRAFT))
        self.assertNotIn("draft so far is kept", h.gh.bot_comments(12)[-1])

    def test_a_planner_cut_off_before_any_draft_still_hands_on_what_it_found(self):
        """No draft yet, only the end of the session: still a planner's handoff, never one a
        builder is told to pick up from."""
        h = planning_harness(self)
        stopped = RunResult(False, "", 1, duration_s=240, error="stopped mid-call",
                            extra={"usage_stop": "5-hour usage is 70%, at or over its 70% cap"})
        h.night(TrailRunner({"plan": lambda request: stopped}))
        handoff = h.ctx.store.load()["items"]["12"]["handoff"]
        self.assertEqual(handoff["kind"], "draft")
        self.assertNotIn("draft", handoff)
        self.assertIn("the change is in src/game.txt", handoff["trail"])
        self.assertIn("The end of its session is kept for the next planning run.",
                      h.gh.bot_comments(12)[-1])
        seen = {}

        def finishing(request):
            seen["prompt"] = request.prompt
            return RunResult(True, PLAN)
        h.night(FakeRunner({"plan": finishing}))
        self.assertIn("It left no draft, only the end of its session below", seen["prompt"])
        self.assertNotIn("Picking up from another agent", seen["prompt"])

    def test_a_finished_plan_hands_on_no_draft(self):
        root = Path(tempfile.mkdtemp())
        _, clone = make_origin(root)
        cfg = make_config(gates=GATES, install={"run": "true", "timeout_minutes": 1})

        def drafting(request):
            (request.cwd / PLAN_DRAFT_FILE).write_text(DRAFT)
            return RunResult(True, PLAN)
        plan = {"action": "plan", "number": 12, "title": "Rules v2", "branch": "bot/issue-12",
                "thread": "Please make the rules v2."}
        result = Worker(cfg, plan, FakeRunner({"plan": drafting}), clone, root / "work",
                        root / "out").run()
        self.assertEqual(result["status"], "planned")
        self.assertNotIn("draft", result["handoff"])
        self.assertTrue(result["handoff"]["notes"].startswith("# Plan"))
        self.assertNotIn("## Files to touch", result["handoff"]["notes"])

    def test_a_builder_never_starts_from_a_draft(self):
        """A build run that plans first gives its planner the draft; its builder starts from the
        plan that planner wrote, and is not told about the draft."""
        root = Path(tempfile.mkdtemp())
        _, clone = make_origin(root)
        cfg = make_config(gates=GATES, install={"run": "true", "timeout_minutes": 1})
        handoff = {"kind": "draft", "provider": "claude-4", "family": "claude",
                   "reason": "`claude-4` stops mid-call", "notes": "", "trail": "",
                   "draft": DRAFT}
        plan = {"action": "build", "number": 12, "title": "Rules v2", "branch": "bot/issue-12",
                "thread": "Please make the rules v2.", "handoff": handoff,
                "seats": {"plan": {"provider": "claude-1", "model": "opus", "tier": "strong"},
                          "build": {"provider": "claude-1", "model": "opus", "tier": "strong"},
                          "review": {"provider": "claude-1", "model": "opus", "tier": "strong"}}}
        worker = Worker(cfg, plan, FakeRunner({}), clone, root / "work", root / "out")
        planner = worker._handoff_text(builder=False)
        self.assertIn("## An unfinished plan from an earlier run", planner)
        self.assertIn(DRAFT, planner)
        builder = worker._handoff_text(builder=True)
        self.assertNotIn("unfinished plan", builder)
        self.assertNotIn("Picking up from another agent", builder)
        self.assertNotIn("picking up", worker._first_why("build", []))

    def test_a_plan_in_the_description_replaces_a_draft(self):
        """A person writes the plan after a planner was cut off: the builder starts from theirs,
        and the draft is dropped."""
        gh = FakeGitHub()
        ctx = make_ctx(gh, at=DAY, cfg=make_config(env=secrets("CLAUDE_CODE_OAUTH_TOKEN_2"),
                                                   machine=MACHINE, plan_lanes=None))
        body = issueplan.with_plan("Make the rules v2.", PLAN, "a person")
        gh.add_issue(12, labels=(LABEL_BUILD,), body=body)
        ctx.store.update(lambda s: state_item(s, 12).update(handoff={
            "kind": "draft", "provider": "claude-4", "family": "claude", "notes": "",
            "trail": "the end of claude-4's session", "draft": DRAFT}))
        planned = plan_mod.make(ctx, force=True)
        self.assertEqual(planned["action"], "build")
        self.assertEqual(planned["handoff"]["kind"], "plan")
        self.assertNotIn("draft", planned["handoff"])
        self.assertNotIn("trail", planned["handoff"])
        self.assertIn("Edit src/game.txt", planned["handoff"]["notes"])
        self.assertEqual(planned["handoff"]["provider"], "the issue's description")


if __name__ == "__main__":
    unittest.main()
