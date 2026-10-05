"""#317 part 3: a revision cut off by a usage cap keeps its work, and pauses add up.

`deliver._interrupted` published only a build's work. A revision cut off mid-call lost
everything but its notes, so #143 had 16 Opus revision runs end "Paused: … stops mid-call" while
its branch never moved, and a usage pause never counted towards blocking it.
"""

from __future__ import annotations

import unittest

from harness.config import LABEL_BLOCKED, LABEL_PR, LABEL_REVISE
from harness.runner import FakeRunner, RunResult

from tests.fakes import git, push_branch
from tests.test_flow import Harness
from tests.test_work import APPROVE, builder, reviewer


def capped(request):
    """The subscription's usage crossed its cap mid-call (`usage_stop`): a pause, no parking."""
    return RunResult(False, "", 1, duration_s=1800, error="stopped mid-call",
                     extra={"usage_stop": "5-hour usage is 91%, at or over its 90% cap"})


def pull(h, number=40):
    start = push_branch(h.origin, h.root, "bot/issue-12", {"src/game.txt": "v2\n"})
    git(h.clone, "fetch", "-q", "origin")
    h.gh.add_pull(number, "bot/issue-12", body="Closes #12", labels=(LABEL_PR, LABEL_REVISE),
                  sha=start)
    return start


class WipTests(unittest.TestCase):
    def test_a_cut_off_revision_keeps_its_work_beside_the_pull_request(self):
        h = Harness(self)
        start = pull(h)
        _, first = h.night(FakeRunner({"revise": builder({"src/game.txt": "half\n"}),
                                       "review": capped}))
        self.assertEqual(first["status"], "interrupted")
        self.assertEqual(h.origin_sha("bot/issue-12"), start)  # the pull request is untouched
        self.assertEqual(h.origin_sha("bot/wip/40"), first["head"])
        record = h.ctx.store.load()["items"]["40"]
        self.assertEqual((record["wip"]["sha"], record["wip"]["start"]), (first["head"], start))
        self.assertIn("The work so far is on `bot/wip/40`", h.gh.bot_comments(40)[-1])
        self.assertEqual(int(record.get("interruptions", 0)), 0)  # it moved on: no strike

        # The next revision starts from it, is told so, and finishes the job.
        seen = {}

        def finishing(request):
            seen["file"] = (request.cwd / "src" / "game.txt").read_text()
            return builder({"src/game.txt": "whole\n"})(request)
        runner = FakeRunner({"revise": finishing, "review": reviewer(APPROVE)})
        planned, second = h.night(runner)
        self.assertEqual(planned["wip"]["sha"], first["head"])
        self.assertEqual(seen["file"], "half\n")
        self.assertIn("Your worktree starts from `bot/wip/40`", runner.calls[0].prompt)
        self.assertEqual(second["status"], "approved")
        self.assertEqual(second["start"], start)  # deliver still checks against the PR's head
        self.assertEqual(git(h.origin, "show", f"{h.origin_sha('bot/issue-12')}:src/game.txt"),
                         "whole")
        self.assertIsNone(h.origin_sha("bot/wip/40"))  # done with
        self.assertNotIn("wip", h.ctx.store.load()["items"]["40"])

    def test_a_branch_that_moved_since_starts_afresh(self):
        h = Harness(self)
        pull(h)
        h.night(FakeRunner({"revise": builder({"src/game.txt": "half\n"}), "review": capped}))
        moved = push_branch(h.origin, h.root, "bot/issue-12", {"src/other.txt": "a person's\n"},
                            base="bot/issue-12")
        h.gh.threads[40]["head"]["sha"] = moved
        seen = {}

        def looking(request):
            seen["file"] = (request.cwd / "src" / "game.txt").read_text()
            return builder({"src/game.txt": "redone\n"})(request)
        runner = FakeRunner({"revise": looking, "review": reviewer(APPROVE)})
        _, result = h.night(runner)
        self.assertEqual(seen["file"], "v2\n")
        self.assertEqual(result["wip"]["ignored"], "bot/wip/40")
        self.assertIn("the pull request's branch moved after it stopped", runner.calls[0].prompt)

    def test_pauses_that_get_nowhere_block_it_after_three(self):
        h = Harness(self)
        pull(h)
        for n in range(3):
            _, result = h.night(FakeRunner({"revise": capped}))
            self.assertEqual(result["status"], "interrupted")
            if n < 2:
                self.assertEqual(h.gh.label_names(40), {LABEL_PR, LABEL_REVISE})
        self.assertEqual(h.gh.label_names(40), {LABEL_PR, LABEL_BLOCKED})
        self.assertIn("cut off 3 runs in a row without its work moving on",
                      h.gh.bot_comments(40)[-1])

    def test_a_resumed_revision_that_gets_nowhere_still_counts(self):
        """A revision that resumes from `bot/wip/<pr>` pushes it again every time; only a commit
        of its own past the kept head is progress, so pauses with none still add up."""
        h = Harness(self)
        pull(h)
        _, first = h.night(FakeRunner({"revise": builder({"src/game.txt": "half\n"}),
                                       "review": capped}))
        self.assertEqual(int(h.ctx.store.load()["items"]["40"].get("interruptions", 0)), 0)
        for n in (1, 2):
            _, result = h.night(FakeRunner({"revise": capped}))
            self.assertEqual(result["status"], "interrupted")
            self.assertEqual(h.origin_sha("bot/wip/40"), first["head"])  # kept, not moved on
            self.assertEqual(int(h.ctx.store.load()["items"]["40"]["interruptions"]), n)

    def test_mains_commits_merged_in_are_no_progress(self):
        """Every revision merges `main` in; its squash commits are no merges, but they are not the
        run's work, so a cut-off run that did nothing else still counts."""
        h = Harness(self)
        pull(h)
        h.night(FakeRunner({"revise": builder({"src/game.txt": "half\n"}), "review": capped}))
        for n in (1, 2):
            push_branch(h.origin, h.root, "main", {f"docs/other-{n}.md": "someone else's\n"})
            _, result = h.night(FakeRunner({"revise": capped}))
            self.assertEqual(result["status"], "interrupted")
            self.assertEqual(int(h.ctx.store.load()["items"]["40"]["interruptions"]), n)

    def test_a_pause_that_moves_the_work_on_resets_the_count(self):
        h = Harness(self)
        pull(h)
        h.night(FakeRunner({"revise": capped}))
        h.night(FakeRunner({"revise": capped}))
        self.assertEqual(int(h.ctx.store.load()["items"]["40"]["interruptions"]), 2)
        _, result = h.night(FakeRunner({"revise": builder({"src/game.txt": "half\n"}),
                                        "review": capped}))
        self.assertEqual(result["status"], "interrupted")
        self.assertEqual(int(h.ctx.store.load()["items"]["40"]["interruptions"]), 0)
        self.assertEqual(h.gh.label_names(40), {LABEL_PR, LABEL_REVISE})

    def test_closing_the_pull_request_drops_its_wip(self):
        from harness import events
        h = Harness(self)
        h.gh.add_issue(12)
        pull(h)
        h.night(FakeRunner({"revise": builder({"src/game.txt": "half\n"}), "review": capped}))
        self.assertIn("wip", h.ctx.store.load()["items"]["40"])
        pr = dict(h.gh.threads[40], merged=False, state="closed")
        events.handle(h.ctx, "pull_request", {"action": "closed", "pull_request": pr})
        self.assertIn("bot/wip/40", h.gh.deleted_branches)
        self.assertNotIn("wip", h.ctx.store.load()["items"]["40"])


if __name__ == "__main__":
    unittest.main()
