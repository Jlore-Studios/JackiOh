"""The pinned "Night bot statistics" issue (bot/harness/stats.py): what the bot's comments, pull
requests, commits and state file say it has done, and the two-hour rewrite."""

from __future__ import annotations

import unittest
from datetime import timedelta

from harness import stats

from tests.fakes import BOT, FakeGitHub
from tests.support import DAY, make_ctx

RUN = "https://github.com/Jlore-Studios/JackiOh/actions/runs"


def say(gh, number, body, at):
    gh.add_comment(number, body, user=dict(BOT), association="COLLABORATOR", created_at=at)


def world():
    gh = FakeGitHub()
    gh.add_issue(3, "Patch v0.2.Y: a thing")
    gh.add_issue(4, "Patch v0.2.X: another")
    say(gh, 3, f"Starting work on this now ([run]({RUN}/11)): it is difficulty:easy. I build it on "
               "`devin` (devin, `swe-2-max`, weak), and it checks itself.", "2026-09-28T01:00:00Z")
    say(gh, 3, f"Opened #9, built on `devin` (devin, `swe-2-max`, weak) ([run]({RUN}/11)). Its "
               "adversarial reviewer approved it.", "2026-09-28T03:00:00Z")
    say(gh, 4, f"Planning this now ([run]({RUN}/12)), on `claude-3` (claude, `opus`, strong) "
               "(`bot:needs-plan`).", "2026-09-28T04:00:00Z")
    say(gh, 4, f"Planned on `claude-3` (claude, `opus`, strong) ([run]({RUN}/12)). The plan is in "
               "this issue's description.", "2026-09-28T04:10:00Z")
    say(gh, 4, f"Starting work on this now ([run]({RUN}/13)): it is difficulty:medium. I build it "
               "on `agy` (agy, `gemini-3.8-flash-high`, medium).", "2026-09-28T05:00:00Z")
    say(gh, 4, f"Paused: `agy` reached its usage limit ([run]({RUN}/13)). It stays queued.",
        "2026-09-28T05:01:00Z")
    say(gh, 4, "@MaxGoetzmann Queued #4; I will build it in the next run.", "2026-09-28T00:30:00Z")
    gh.add_comment(4, "Not the bot's.", created_at="2026-09-28T06:00:00Z")
    pull = gh.add_pull(9, "bot/issue-3", title="Patch v0.2.Y: a thing (#3)", body="Closes #3",
                       user=dict(BOT))
    pull.update(created_at="2026-09-28T03:00:00Z", merged_at="2026-09-28T07:30:00Z",
                state="closed", additions=40, deletions=5, commits=3)
    gh.add_pull(10, "someone/else", title="A person's change")
    gh.commits = [{"sha": "a", "author": {"login": BOT["login"]}},
                  {"sha": "b", "author": {"login": "jgoetzmann"}}]
    gh.runs.update({"1": {"status": "completed", "conclusion": "success"},
                    "2": {"status": "completed", "conclusion": "failure"}})
    ctx = make_ctx(gh, at=DAY)
    ctx.store.update(lambda s: s.setdefault("providers", {}).update(
        devin={"spent": [{"at": "2026-09-28T03:00:00Z", "minutes": 120.0}]},
        agy={"spent": [{"at": "2026-09-28T05:01:00Z", "minutes": 0.1}]}))
    return gh, ctx


class ParseTests(unittest.TestCase):
    def test_a_comment_says_what_happened_in_which_run_on_which_subscription(self):
        event = stats.parse({"body": f"Starting a revision now ([run]({RUN}/7)), because of: CI. I "
                                     "build it on `muse` (muse, `muse-spark-1.3-contributor`, medium).",
                             "issue_url": "https://api.github.com/repos/o/r/issues/12",
                             "created_at": "2026-09-28T01:00:00Z"})
        self.assertEqual((event.kind, event.action, event.run, event.provider, event.model,
                          event.number),
                         ("start", "revise", "7", "muse", "muse-spark-1.3-contributor", 12))
        opened = stats.parse({"body": "Opened #44, built on `gpt` (codex, gpt-5.6-terra).",
                              "issue_url": "x/issues/3"})
        self.assertEqual((opened.kind, opened.pr, opened.provider, opened.model),
                         ("opened", 44, "gpt", "gpt-5.6-terra"))
        for body, kind in (("Revision pushed on `devin` …", "revised"),
                           ("Paused: the run's time budget is spent.", "paused"),
                           ("My revision did not pass review after 10 rounds", "not approved"),
                           ("The run could not work on this on `claude-3`: 401", "infra"),
                           ("This run failed: RuntimeError", "failed"),
                           ("Queued #4; I will build it", "queued")):
            self.assertEqual(stats.classify(body), kind, body)

    def test_an_outcome_takes_its_runs_subscription(self):
        events = stats.joined([
            stats.Event(at="1", number=3, kind="start", run="5", provider="agy", model="g"),
            stats.Event(at="2", number=3, kind="paused", run="5"),
            stats.Event(at="3", number=4, kind="failed", run="6"),
        ])
        self.assertEqual([(e.provider, e.model) for e in events],
                         [("agy", "g"), ("agy", "g"), ("", "")])


class IssueTests(unittest.TestCase):
    def test_the_issue_counts_and_charts_what_was_done(self):
        gh, ctx = world()
        body = stats.render(ctx, stats.collect(ctx))
        for line in ("| Runs started (plan, build, revise, review) | 3 |",
                     "| Pull requests opened | 1 |", "| Merged | 1 (100%) |",
                     "| Issues closed by its pull requests | 1 |",
                     "| Commits on its branches | 3 |", "| Commits on `main` by the bot | 1 |",
                     "| Lines merged | +40 / −5 |",
                     "| Median time from pull request to merge | 4h 30m |",
                     "| Requests answered (`Queued …`) | 1 |",
                     "`bot-night.yml` workflow runs | 2 "):
            self.assertIn(line, body)
        self.assertIn("| `devin` | `swe-2-max` | 1 | 0 | 1 | 0 | 0 | 1 | 1 | 0 | 0 | 0 "
                      "| 2.0 h over its last 1 runs |", body)
        self.assertIn("| `agy` | `gemini-3.8-flash-high` | 1 | 0 | 1 | 0 | 0 | 0 | 0 | 1 |", body)
        self.assertIn("| `claude-3` | `opus` | 1 | 1 | 0 |", body)
        self.assertIn('pie showData title Pull requests merged, by builder', body)
        self.assertIn('    "devin" : 1', body)
        self.assertIn("xychart-beta", body)
        self.assertIn("| #9 Patch v0.2.Y: a thing (#3) | `devin` | 4h 30m | +40 / −5 |", body)
        self.assertIn("2 of 3 outcomes delivered something (67%)", body)
        self.assertTrue(body.endswith(stats.MARKER))

    def test_it_is_opened_pinned_and_then_rewritten_every_two_hours(self):
        gh, ctx = world()
        note = stats.update(ctx)
        self.assertRegex(note, r"statistics: opened #\d+ and pinned it")
        number = ctx.store.load()["statistics"]["issue"]
        self.assertEqual(gh.threads[number]["title"], stats.TITLE)
        self.assertEqual(gh.pinned, [gh.threads[number]["node_id"]])
        self.assertEqual(stats.update(ctx), "statistics: not due")
        self.assertEqual(stats.update(ctx, force=True), f"statistics: rewrote #{number}")
        later = make_ctx(gh, at=DAY + stats.STATS_EVERY + timedelta(minutes=1))
        self.assertEqual(stats.update(later), f"statistics: rewrote #{number}")
        # Someone closes it: the next rewrite opens a new one.
        gh.threads[number]["state"] = "closed"
        much_later = make_ctx(gh, at=DAY + 3 * stats.STATS_EVERY)
        self.assertRegex(stats.update(much_later), r"opened #\d+")


if __name__ == "__main__":
    unittest.main()
