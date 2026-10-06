"""#342: every run leaves its section in the item's journal, the next run picks up from it, and a
job that is cancelled or killed hands its work on instead of failing the item.

Before this, a run's handoff was one slot in the state file (the next run overwrote it, a finished
item dropped it, and nobody could read it from the pull request), it kept the last 8,000
characters of the notes (the end of the log, not the plan or the state at the top), and a
cancelled job wrote its first status, `failed`, with no work kept: a strike against the item.
"""

from __future__ import annotations

import json
import os
import shutil
import signal
import time
import unittest

from harness import journal
from harness import plan as plan_mod
from harness.config import LABEL_BUILD, LABEL_PR, LABEL_REVISE, LABEL_WORKING
from harness.deliver import Deliverer
from harness.errors import GitHubError
from harness.runner import FakeRunner, RunResult
from harness.work import NOTES_FILE, Worker

from tests.fakes import FakeGitHub, git
from tests.support import DAY, MACHINE
from tests.test_cross import ALL
from tests.test_flow import Harness
from tests.test_work import APPROVE, DONE, builder, reviewer
from tests.test_wip import capped

NOTES = ("## State\n\nRules v2 is half written; the Coin is next. Tried a table of rules: too "
         "slow, dropped.\n\n## Log\n\n- about to write the rules: the issue asks for v2\n"
         "- wrote half of them\n")


def journal_text(h: Harness, number: int = 12) -> str:
    text, _ = h.gh.get_file(journal.path_for(number), journal.BRANCH)
    return text or ""


def calls(runner: FakeRunner, role: str) -> list:
    return [c for c in runner.calls if c.role == role]


class CompactTests(unittest.TestCase):
    def test_short_notes_stay_whole(self):
        self.assertEqual(journal.compact(NOTES, 8000), NOTES)

    def test_the_state_stays_whole_and_the_log_keeps_its_end(self):
        log = "".join(f"- step {n}: did a thing for a reason\n" for n in range(2000))
        notes = "# Plan\n\nDo the rules.\n\n## State\n\nHalf done.\n\n## Log\n\n" + log
        cut = journal.compact(notes, 4000)
        self.assertLessEqual(len(cut), 4000)
        self.assertTrue(cut.startswith("# Plan\n\nDo the rules.\n\n## State\n\nHalf done."))
        self.assertIn(journal.CUT, cut)
        self.assertTrue(cut.rstrip().endswith("- step 1999: did a thing for a reason"))
        self.assertNotIn("- step 0:", cut)

    def test_a_state_too_long_keeps_its_start(self):
        notes = "## State\n\n" + "s" * 10_000 + "\n\n## Log\n\n" + "- the last line\n"
        cut = journal.compact(notes, 4000)
        self.assertLessEqual(len(cut), 4000)
        self.assertIn("(the rest of the state was cut here)", cut)
        self.assertTrue(cut.rstrip().endswith("- the last line"))

    def test_notes_without_a_log_keep_their_end_as_before(self):
        notes = "x" * 5000 + "the end"
        self.assertEqual(journal.compact(notes, 100), notes[-100:])


class FileTests(unittest.TestCase):
    def section(self, n: int, notes: str = "notes") -> str:
        return journal.section(
            {"status": "approved", "reason": f"run {n}", "steps": [
                {"at": "2026-10-06T01:02:03Z", "step": "call", "role": "build", "model": "opus",
                 "why": "build | the issue", "minutes": 4.2, "outcome": "done"}],
             "handoff": {"notes": notes}},
            provider="claude-3", models="`opus`", action="build", number=12,
            at=f"2026-10-06T0{n % 10}:00:00Z", run_url=f"https://example/runs/{n}")

    def test_a_section_says_what_the_run_did_and_why(self):
        text = self.section(1, notes="## State\nhalf\n")
        self.assertTrue(text.startswith(journal.RUN_MARKER + "\n## 2026-10-06 01:00 UTC · build #12"))
        self.assertIn("[run](https://example/runs/1) on `claude-3` (`opus`). It ended "
                      "**approved**: run 1", text)
        self.assertIn("| 1 | 01:02 | build (`opus`) | build \\| the issue | 4.2 min | done |", text)
        self.assertIn("<summary>The builder's notes when it stopped</summary>", text)
        self.assertIn("## State\nhalf", text)

    def test_a_killed_run_reads_as_died(self):
        text = journal.section({"status": "interrupted", "interrupt": "died", "reason": "killed"},
                               provider="muse", models="`muse`", action="revise", number=40,
                               at="2026-10-06T01:00:00Z")
        self.assertIn("· revise #40 · `muse` · died", text)

    def test_appends_make_the_branch_then_add_to_the_file(self):
        gh = FakeGitHub()
        journal.append(gh, 12, self.section(1), sleep=lambda _: None)
        self.assertIn(journal.BRANCH, gh.branches)
        # Vercel reads the pushed commit's settings: the branch must say "no deployments".
        self.assertIn('"deploymentEnabled": false', gh.get_file("vercel.json", journal.BRANCH)[0])
        journal.append(gh, 12, self.section(2), sleep=lambda _: None)
        text, _ = gh.get_file("12.md", journal.BRANCH)
        self.assertTrue(text.startswith("# Journal of #12"))
        self.assertEqual(len(journal.runs_in(text)), 2)
        self.assertLess(text.index("run 1"), text.index("run 2"))

    def test_a_write_that_lost_a_race_reads_again(self):
        gh = FakeGitHub()
        journal.append(gh, 12, self.section(1), sleep=lambda _: None)
        real_put = gh.put_file
        raced = []

        def put(path, text, *, branch, sha, message):
            if not raced:  # another run's section lands first
                raced.append(True)
                other, other_sha = gh.get_file(path, branch)
                real_put(path, other + "\n" + self.section(3), branch=branch, sha=other_sha,
                         message="other")
                raise GitHubError("conflict", 409)
            real_put(path, text, branch=branch, sha=sha, message=message)
        gh.put_file = put
        journal.append(gh, 12, self.section(2), sleep=lambda _: None)
        text, _ = gh.get_file("12.md", journal.BRANCH)
        self.assertIn("run 3", text)
        self.assertIn("run 2", text)

    def test_a_long_file_drops_old_notes_then_old_runs(self):
        text = journal.header(12) + "\n" + "\n".join(self.section(n, notes="n" * 5000)
                                                      for n in range(20))
        trimmed = journal.trim(text, 60_000)
        self.assertLessEqual(len(trimmed), 60_000)
        self.assertEqual(len(journal.runs_in(trimmed)), 20)  # every run's lines stay
        self.assertNotIn("n" * 5000, trimmed.split(journal.RUN_MARKER)[1])
        self.assertIn("n" * 5000, trimmed.split(journal.RUN_MARKER)[-1])  # the newest keep them
        tiny = journal.trim(text, 2_000)
        self.assertLess(len(journal.runs_in(tiny)), 20)
        self.assertIn("older run(s) dropped from this file", tiny)

    def test_reading_keeps_the_end_from_a_run(self):
        gh = FakeGitHub()
        for n in range(30):
            journal.append(gh, 12, self.section(n, notes="n" * 3000), sleep=lambda _: None)
        whole, _ = gh.get_file("12.md", journal.BRANCH)
        read = journal.read(gh, 12, limit=20_000)
        self.assertLess(len(read), len(whole))
        self.assertIn("earlier runs are on the branch", read)
        self.assertIn("run 29", read)
        self.assertEqual(read.split(journal.RUN_MARKER, 1)[1].lstrip()[:3], "## ")
        self.assertEqual(journal.read(gh, 99), "")


class OpusLaneTests(unittest.TestCase):
    """The Claude accounts' runs, on GitHub's runners: a run cut off at its usage cap writes the
    journal, and the next one starts from it."""

    def test_a_cut_off_run_journals_and_the_next_picks_up(self):
        h = Harness(self, env=ALL, machine=MACHINE)
        h.gh.add_issue(12, labels=(LABEL_BUILD, "difficulty:hard"))
        first = FakeRunner({"build": builder({"src/game.txt": "half\n", NOTES_FILE: NOTES}),
                            "review": capped})
        planned, result = h.night(first)
        self.assertEqual((planned["provider"], planned["runs_on"], result["status"]),
                         ("claude-3", "ubuntu-latest", "interrupted"))
        build = next(s for s in result["steps"] if s.get("role") == "build")
        self.assertEqual(build["model"], "opus")
        self.assertTrue(build["why"].startswith("build the issue"))
        self.assertIn("committed", build["outcome"])
        review = next(s for s in result["steps"] if s.get("role") == "review")
        self.assertIn("cut off at its usage cap", review["outcome"])
        self.assertIn("green", next(s for s in result["steps"] if s["step"] == "checks")["outcome"])
        self.assertEqual(result["steps"][-1]["step"], "stop")

        text = journal_text(h)
        self.assertEqual(len(journal.runs_in(text)), 1)
        self.assertIn("build #12 · `claude-3` · interrupted", text)
        self.assertIn("| review (`opus`) | review of round 1", text)
        self.assertIn("Tried a table of rules: too slow, dropped.", text)
        self.assertIn("blob/bot-journal/12.md", h.gh.bot_comments(12)[-1])  # the pause links it

        seen = {}

        def picking_up(request):
            seen["journal"] = (request.cwd / journal.JOURNAL_FILE).read_text()
            seen["file"] = (request.cwd / "src" / "game.txt").read_text()
            return builder({"src/game.txt": "whole\n"})(request)
        second = FakeRunner({"build": picking_up, "review": reviewer(APPROVE)})
        planned, result = h.night(second)
        self.assertIn("Tried a table of rules", planned["journal"])
        self.assertIn("Tried a table of rules", seen["journal"])
        self.assertEqual(seen["file"], "half\n")  # and its code
        prompt = calls(second, "build")[0].prompt
        self.assertIn("## The journal of earlier runs", prompt)
        self.assertIn("build #12 · `claude-3` · interrupted", prompt)
        self.assertIn("bring the `## State` part of `.bot-notes.md` up to date", prompt)
        self.assertIn("`## State`, rewritten in place", prompt)  # the two-part notes ask
        self.assertEqual(result["status"], "approved")
        tree = git(h.origin, "ls-tree", "-r", "--name-only", result["head"])
        self.assertNotIn(journal.JOURNAL_FILE, tree)
        self.assertNotIn(NOTES_FILE, tree)

        text = journal_text(h)
        self.assertEqual(len(journal.runs_in(text)), 2)
        self.assertIn("· approved", journal.runs_in(text)[1])
        pr = h.gh.list_pulls(head="bot/issue-12")[0]
        self.assertIn("blob/bot-journal/12.md", pr["body"])

    def test_a_pull_requests_runs_go_to_its_issues_journal(self):
        h = Harness(self, env=ALL, machine=MACHINE)
        h.gh.add_issue(12, labels=(LABEL_BUILD, "difficulty:hard"))
        h.night(FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                            "review": reviewer(APPROVE)}))
        pr = int(h.gh.list_pulls(head="bot/issue-12")[0]["number"])
        h.gh.add_labels(pr, [LABEL_REVISE])  # a person asked for another pass
        planned, result = h.night(FakeRunner({"revise": builder({"src/game.txt": "v3\n"}),
                                              "review": reviewer(APPROVE)}))
        self.assertEqual((planned["action"], planned["issue_number"]), ("revise", 12))
        self.assertIn("build #12", planned["journal"])
        runs = journal.runs_in(journal_text(h))
        self.assertEqual(len(runs), 2)
        self.assertIn(f"revise #{pr}", runs[1])
        self.assertIsNone(h.gh.get_file(journal.path_for(pr), journal.BRANCH)[0])


class MachineLaneTests(unittest.TestCase):
    """The subscriptions on the bot's machine (Muse here): the same journal, read from the plan,
    since the machine holds no GitHub credential, and ignored in the clone it keeps."""

    def test_a_machine_run_reads_and_writes_the_journal(self):
        h = Harness(self, env=ALL, machine=MACHINE, at=DAY)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        earlier = journal.section(
            {"status": "interrupted", "interrupt": "usage", "reason": "claude-5 stops mid-call",
             "handoff": {"notes": NOTES}},
            provider="claude-5", models="`opus`", action="build", number=12,
            at="2026-09-29T09:00:00Z")
        journal.append(h.gh, 12, earlier, sleep=lambda _: None)
        seen = {}

        def building(request):
            seen["journal"] = (request.cwd / journal.JOURNAL_FILE).read_text()
            return builder({"src/game.txt": "v2\n"})(request)
        runner = FakeRunner({"build": building, "review": reviewer(APPROVE)})
        planned, result = h.night(runner)
        self.assertEqual((planned["provider"], planned["runs_on"]), ("muse", "night-vm-muse"))
        self.assertIn("claude-5 stops mid-call", seen["journal"])
        prompt = calls(runner, "build")[0].prompt
        self.assertIn("1 earlier run(s) worked on this", prompt)
        self.assertIn("`## Log`, added to", prompt)  # the notes ask reaches every CLI's prompt
        exclude = git(h.clone, "rev-parse", "--git-common-dir")
        ignored = (h.clone / exclude / "info" / "exclude").read_text().splitlines()
        self.assertIn(f"/{journal.JOURNAL_FILE}", ignored)
        self.assertIn(f"/{NOTES_FILE}", ignored)
        runs = journal.runs_in(journal_text(h))
        self.assertEqual(len(runs), 2)
        self.assertIn("`muse`", runs[1])


class KilledJobTests(unittest.TestCase):
    def test_a_cancelled_job_is_a_pause_that_keeps_its_work(self):
        """GitHub cancels with SIGINT: the run commits and bundles what it has and pauses."""
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))

        def cancelled(request):
            (request.cwd / "src" / "game.txt").write_text("half\n")
            os.kill(os.getpid(), signal.SIGINT)
            time.sleep(5)  # the handler stops the call before this ends
            return RunResult(True, DONE)
        planned, result = h.night(FakeRunner({"build": cancelled}))
        self.assertIs(signal.getsignal(signal.SIGINT), signal.default_int_handler)  # put back
        self.assertEqual((result["status"], result["interrupt"]), ("interrupted", "died"))
        self.assertIn("SIGINT", result["reason"])
        self.assertEqual(git(h.origin, "show", f"{h.origin_sha('bot/issue-12')}:src/game.txt"),
                         "half")
        record = h.ctx.store.load()["items"]["12"]
        self.assertFalse(record.get("failures"))
        self.assertFalse(record.get("strikes"))
        self.assertIn(LABEL_BUILD, h.gh.label_names(12))  # queued again
        self.assertNotIn(LABEL_WORKING, h.gh.label_names(12))
        self.assertIn("Paused: the job was stopped (SIGINT)", h.gh.bot_comments(12)[-1])
        self.assertIn("· died", journal.runs_in(journal_text(h))[0])

    def test_a_killed_job_leaves_its_last_checkpoint(self):
        """A job killed outright never reaches its own end: deliver gets the checkpoint the work
        job wrote after its last step, and keeps the work."""
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned = plan_mod.make(h.ctx)
        out, killed = h.root / "out-0", h.root / "killed"

        def killed_during_review(request):
            shutil.copytree(out, killed)  # what the upload step finds if the job dies now
            return RunResult(True, APPROVE)
        runner = FakeRunner({"build": builder({"src/game.txt": "v2\n"}),
                             "review": killed_during_review})
        Worker(h.cfg, planned, runner, h.clone, h.root / "work", out).run()
        checkpoint = json.loads((killed / "result.json").read_text())
        self.assertEqual((checkpoint["status"], checkpoint["interrupt"], checkpoint["checkpoint"]),
                         ("interrupted", "died", True))
        self.assertIn("its last checkpoint, after 2 model call(s)", checkpoint["reason"])
        self.assertTrue((killed / checkpoint["bundle"]).is_file())
        self.assertEqual(checkpoint["steps"][-1]["step"], "checks")
        final = json.loads((out / "result.json").read_text())
        self.assertEqual(final["status"], "approved")  # the run's own end writes over it
        self.assertNotIn("checkpoint", final)

        Deliverer(h.ctx, planned, killed, h.deliver_repo).run()
        self.assertEqual(git(h.origin, "show", f"{h.origin_sha('bot/issue-12')}:src/game.txt"),
                         "v2")
        self.assertIn(LABEL_BUILD, h.gh.label_names(12))  # queued again
        self.assertNotIn(LABEL_WORKING, h.gh.label_names(12))
        self.assertFalse(h.gh.list_pulls(head="bot/issue-12"))  # no pull request from it
        record = h.ctx.store.load()["items"]["12"]
        self.assertFalse(record.get("failures"))
        self.assertEqual(int(record.get("interruptions", 0)), 0)  # its work moved on
        self.assertIn("Paused: the model job stopped before it finished", h.gh.bot_comments(12)[-1])

    def test_a_cancel_something_swallowed_still_stops_the_run(self):
        """The stop a signal raises can land inside a reading's `except Exception` (memory, the
        disk): it is kept, and the next checkpoint or call raises it again."""
        from harness.work import Interrupt
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        worker = Worker(h.cfg, plan_mod.make(h.ctx), FakeRunner({}), h.clone, h.root / "work",
                        h.root / "out-0")
        worker.signalled = "the job was stopped (SIGTERM): cancelled, or past its time limit"
        with self.assertRaises(Interrupt) as stopped:
            worker.check()
        self.assertEqual(stopped.exception.kind, "died")
        with self.assertRaises(Interrupt):
            worker.call("build", "go", h.clone, reader=False)
        self.assertEqual(worker.calls, 0)  # no call started after the cancel

    def test_a_killed_job_that_got_nowhere_counts(self):
        h = Harness(self)
        h.gh.add_issue(12, labels=(LABEL_BUILD,))
        planned = plan_mod.make(h.ctx)
        out = h.root / "out-0"
        Worker(h.cfg, planned, FakeRunner({"build": builder({}), "review": reviewer(APPROVE)}),
               h.clone, h.root / "work", out).run()
        result = json.loads((out / "result.json").read_text())
        result.update(status="interrupted", interrupt="died", reason="killed", head=None)
        (out / "result.json").write_text(json.dumps(result))
        Deliverer(h.ctx, planned, out, h.deliver_repo).run()
        self.assertEqual(int(h.ctx.store.load()["items"]["12"]["interruptions"]), 1)
        self.assertNotIn(LABEL_PR, h.gh.label_names(12))


if __name__ == "__main__":
    unittest.main()
