"""The pinned "Night bot statistics" issue (bot/harness/stats.py): what the bot's comments, pull
requests, commits and state file say it has done over the last six hours, the last day, the last
week and all time, who planned, built, revised and approved each pull request, and the hourly
rewrite."""

from __future__ import annotations

import unittest
from datetime import timedelta
from unittest import mock

from harness import stats

from tests.fakes import BOT, FakeGitHub
from tests.support import DAY, make_ctx

RUN = "https://github.com/Jlore-Studios/JackiOh/actions/runs"
# DAY is 2026-09-29 17:00 UTC: the six-hour window starts at 11:00 that day, the day's at 17:00 on
# 09-28, the week's at 17:00 on 09-22.


def say(gh, number, body, at):
    gh.add_comment(number, body, user=dict(BOT), association="COLLABORATOR", created_at=at)


def merged_pull(gh, number, issue, *, opened, merged, additions, deletions, files=1, commits=1,
                body=None):
    pull = gh.add_pull(number, f"bot/issue-{issue}", title=f"Patch v0.2.Y: work for #{issue}",
                       body=f"Closes #{issue}" if body is None else body, user=dict(BOT))
    pull.update(created_at=opened, merged_at=merged, state="closed", additions=additions,
                deletions=deletions, changed_files=files, commits=commits)
    return pull


def world():
    gh = FakeGitHub()
    for number in (3, 4, 5, 6):
        gh.add_issue(number, f"Patch v0.2.Y: item {number}")
    # #9, for issue 3, inside the six hours: planned on claude-3, built on devin (its own reviewer
    # claude-2), revised on muse (which reviewed its own revision), and approved again by agy.
    say(gh, 3, f"Planning this now ([run]({RUN}/12)), on `claude-3` (claude, `opus`, strong) "
               "(`bot:needs-plan`).", "2026-09-29T11:30:00Z")
    say(gh, 3, f"Planned on `claude-3` (claude, `opus`, strong) ([run]({RUN}/12)). The plan is in "
               "this issue's description.", "2026-09-29T11:45:00Z")
    say(gh, 3, f"Starting work on this now ([run]({RUN}/13)): it is difficulty:easy. I build it on "
               "`devin` (devin, `swe-2-max`, weak).", "2026-09-29T11:50:00Z")
    say(gh, 3, "Opened #9, built on `devin` (devin, `swe-2-max`, weak). Its adversarial reviewer, "
               "`claude-2` (claude, `opus`, strong), approved it on round 1 of 10.",
        "2026-09-29T12:00:00Z")
    say(gh, 9, f"Starting a revision now ([run]({RUN}/14)), because of: CI. I build it on `muse` "
               "(muse, `muse-spark`, medium).", "2026-09-29T13:50:00Z")
    say(gh, 9, f"Revision pushed ([run]({RUN}/14)) on `muse` (muse, `muse-spark`, medium); `muse` "
               "(muse, `muse-spark`, medium) reviewed it adversarially and approved it",
        "2026-09-29T14:00:00Z")
    say(gh, 9, f"Starting a review now ([run]({RUN}/15)), on `agy` (agy, `gemini-3.1-pro`, "
               "medium).", "2026-09-29T14:30:00Z")
    say(gh, 9, f"Second review ([run]({RUN}/15)): `agy` (agy, `gemini-3.1-pro`, medium) approved "
               "it. Auto-merge is on.", "2026-09-29T14:40:00Z")
    say(gh, 4, "@MaxGoetzmann Queued #4; I will build it in the next run.", "2026-09-29T16:30:00Z")
    merged_pull(gh, 9, 3, opened="2026-09-29T12:00:00Z", merged="2026-09-29T15:00:00Z",
                additions=40, deletions=5, files=4, commits=3)
    # #10, for issue 4, inside the day: no "Opened" comment named its builder, so it is the last
    # build started on its issue before it was opened (gpt).
    say(gh, 4, f"Starting work on this now ([run]({RUN}/20)): it is difficulty:medium. I build it "
               "on `gpt` (codex, `gpt-5.6-terra`, medium).", "2026-09-28T23:00:00Z")
    merged_pull(gh, 10, 4, opened="2026-09-29T00:00:00Z", merged="2026-09-29T03:00:00Z",
                additions=100, deletions=10, files=2)
    # #11, for issue 5, inside the week: the only build on its issue started after it was opened,
    # so nobody is recorded as its builder; that build failed.
    say(gh, 5, f"Starting work on this now ([run]({RUN}/30)): it is difficulty:medium. I build it "
               "on `claude-1` (claude, `opus`, strong).", "2026-09-26T10:00:00Z")
    say(gh, 5, f"This run failed ([run]({RUN}/30)): RuntimeError", "2026-09-26T10:30:00Z")
    merged_pull(gh, 11, 5, opened="2026-09-26T00:00:00Z", merged="2026-09-26T12:00:00Z",
                additions=7, deletions=1)
    # #12, for issue 6, only in all time.
    merged_pull(gh, 12, 6, opened="2026-09-10T00:00:00Z", merged="2026-09-11T00:00:00Z",
                additions=1000, deletions=0, body="Fixes #6")
    # An open one, and a person's.
    gh.add_pull(13, "bot/issue-7", title="Still open", user=dict(BOT)).update(
        created_at="2026-09-29T16:00:00Z")
    gh.add_pull(20, "someone/else", title="A person's change").update(
        created_at="2026-09-29T16:00:00Z", merged_at="2026-09-29T16:30:00Z", state="closed",
        additions=5000, deletions=0)
    gh.commits = [{"sha": "a", "author": {"login": BOT["login"]}},
                  {"sha": "b", "author": {"login": "jgoetzmann"}}]
    gh.runs.update({"1": {"status": "completed", "conclusion": "success",
                          "created_at": "2026-09-29T16:00:00Z"},
                    "2": {"status": "completed", "conclusion": "failure",
                          "created_at": "2026-09-20T00:00:00Z"},
                    "3": {"status": "completed", "conclusion": "success"}})
    ctx = make_ctx(gh, at=DAY)
    ctx.store.update(lambda s: s.setdefault("providers", {}).update(
        devin={"spent": [{"at": "2026-09-29T12:00:00Z", "minutes": 120.0},
                         {"at": "2026-09-20T00:00:00Z", "minutes": 60.0}]}))
    return gh, ctx


def section(body: str, name: str) -> str:
    start = body.index(f"## {name}\n")
    following = [body.index(f"## {other}\n") for other, _ in stats.WINDOWS
                 if body.index(f"## {other}\n") > start]
    return body[start:min(following, default=len(body))]


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

    def test_it_reads_who_approved_a_change_in_each_way_the_bot_says_it(self):
        def approvers(body):
            return stats.parse({"body": body, "issue_url": "x/issues/3"}).approvers

        self.assertEqual(approvers("Opened #9, built on `devin` (devin, `swe-2-max`, weak). Its "
                                   "adversarial reviewer, `claude-2` (claude, `opus`, strong), "
                                   "approved it on round 2 of 10."), ("claude-2",))
        self.assertEqual(approvers("Revision pushed (run) on `muse` (muse, `m`, medium); `agy` "
                                   "(agy, `g`, medium) reviewed it adversarially and approved it"),
                         ("agy",))
        self.assertEqual(approvers("Second review (run): `gpt` (codex, `gpt-5.6-terra`, medium) "
                                   "approved it. Auto-merge is on."), ("gpt",))
        # A vote tally names families, not seats, and a builder is not an approver.
        self.assertEqual(approvers("`claude-opus` (strong) approved it, so it waits"), ())
        self.assertEqual(approvers("Opened #9, built on `devin` (devin, `swe-2-max`, weak). "
                                   "Nobody has approved it yet."), ())

    def test_an_outcome_takes_its_runs_subscription(self):
        events = stats.joined([
            stats.Event(at="1", number=3, kind="start", run="5", provider="agy", model="g"),
            stats.Event(at="2", number=3, kind="paused", run="5"),
            stats.Event(at="3", number=4, kind="failed", run="6"),
        ])
        self.assertEqual([(e.provider, e.model) for e in events],
                         [("agy", "g"), ("agy", "g"), ("", "")])


class CreditTests(unittest.TestCase):
    def test_each_pull_request_names_who_planned_built_revised_and_approved_it(self):
        gh, ctx = world()
        facts = stats.collect(ctx)
        who = stats.credits(facts.events, facts.pulls)
        self.assertEqual(who.planned.get(9), ["claude-3"])
        self.assertEqual(who.built.get(9), "devin")
        self.assertEqual(who.revised.get(9), ["muse"])
        self.assertEqual(who.approved.get(9), ["claude-2", "muse", "agy"])
        # No "Opened" comment: the last build started on its issue before it was opened.
        self.assertEqual(who.built.get(10), "gpt")
        # The only build on its issue started after it was opened.
        self.assertNotIn(11, who.built)
        self.assertEqual(who.builder({"number": 11}), stats.UNRECORDED)
        self.assertEqual([p["number"] for p in facts.pulls], [9, 10, 11, 12, 13])


class IssueTests(unittest.TestCase):
    def test_the_four_windows_sit_side_by_side_in_one_table(self):
        gh, ctx = world()
        body = stats.render(ctx, stats.collect(ctx))
        self.assertIn("| | Last 6 hours | Last 24 hours | Last 7 days | All time |", body)
        for line in ("| Runs started | 4 | 5 | 6 | 6 |",
                     "| … plans · builds · revisions · reviews | 1 · 1 · 1 · 1 | 1 · 2 · 1 · 1 "
                     "| 1 · 3 · 1 · 1 | 1 · 3 · 1 · 1 |",
                     "| Outcomes that delivered something | 4 of 4 (100%) | 4 of 4 (100%) "
                     "| 4 of 5 (80%) | 4 of 5 (80%) |",
                     "| Failed review or failed | 0 | 0 | 1 | 1 |",
                     "| Pull requests opened | 2 | 3 | 4 | 5 |",
                     "| Pull requests merged | 1 | 2 | 3 | 4 |",
                     "| Issues closed by them | 1 | 2 | 3 | 4 |",
                     "| Lines merged (added / removed) | +40 / −5 | +140 / −15 | +147 / −16 "
                     "| +1147 / −16 |",
                     "| Files changed in them | 4 | 6 | 7 | 8 |",
                     "| Commits in them | 3 | 4 | 5 | 6 |",
                     "| Median time from opening to merge | 3h 00m | 3h 00m | 3h 00m | 7h 30m |",
                     "| Model hours | 2.0 | 2.0 | 2.0 | 3.0 |",
                     "| `bot-night.yml` workflow runs | 1 | 1 | 1 | 3 |",
                     "| Requests answered (`Queued …`) | 1 | 1 | 1 | 1 |"):
            self.assertIn(line, body)
        self.assertIn("Now: 1 of its pull requests open, 0 closed without merging, 1 commits on "
                      "`main` by the bot itself. `bot-night.yml` runs ended: success 2, failure 1.",
                      body)
        self.assertEqual([line for line in body.splitlines() if line.startswith("## ")],
                         ["## The four windows"] + [f"## {name}" for name, _ in stats.WINDOWS])
        self.assertTrue(body.endswith(stats.MARKER))

    def test_each_window_says_who_did_what_and_lists_what_merged_in_it(self):
        gh, ctx = world()
        body = stats.render(ctx, stats.collect(ctx))
        six = section(body, "Last 6 hours")
        self.assertIn("| `devin` | `swe-2-max` | 1 | 0 | 1 | 0 | 0 | 1 of 1 | 1 | 1 | +40 / −5 "
                      "| 0 | 0 | 0 | 2.0 h |", six)
        self.assertIn("| `agy` | `gemini-3.1-pro` | 1 | 0 | 0 | 0 | 1 | 1 of 1 | 0 | 0 | +0 / −0 "
                      "| 0 | 0 | 0 | 0.0 h |", six)
        self.assertIn("### Pull requests merged (1)", six)
        self.assertIn("| #9 Patch v0.2.Y: work for #3 | `claude-3` | `devin` | `muse` "
                      "| `claude-2`, `muse`, `agy` | 3h 00m | +40 / −5 | 4 |", six)
        self.assertNotIn("#10 ", six)
        self.assertNotIn("`gpt`", six)

        day = section(body, "Last 24 hours")
        self.assertIn("### Pull requests merged (2)", day)
        self.assertIn("| #10 Patch v0.2.Y: work for #4 | — | `gpt` | — | — | 3h 00m "
                      "| +100 / −10 | 2 |", day)

        week = section(body, "Last 7 days")
        self.assertIn("| #11 Patch v0.2.Y: work for #5 | — | not recorded | — | — | 12h 00m "
                      "| +7 / −1 | 1 |", week)
        # #11 and the open #13: no comment names who built either.
        self.assertIn("| _not recorded_ | — | 0 | 0 | 0 | 0 | 0 | 0 of 0 | 2 | 1 | +7 / −1 |",
                      week)
        self.assertIn("| `claude-1` | `opus` | 1 | 0 | 1 | 0 | 0 | 0 of 1 | 0 | 0 | +0 / −0 "
                      "| 0 | 1 | 0 | 0.0 h |", week)

        ever = section(body, "All time")
        self.assertIn("### Pull requests merged (4)", ever)
        self.assertIn("#12 Patch v0.2.Y: work for #6", ever)
        self.assertIn("| `devin` | `swe-2-max` | 1 | 0 | 1 | 0 | 0 | 1 of 1 | 1 | 1 | +40 / −5 "
                      "| 0 | 0 | 0 | 3.0 h (its last 2 runs) |", ever)
        self.assertIn("### Over time", ever)
        self.assertEqual(body.count("### Over time"), 1)
        # A person's pull request is not the bot's work.
        self.assertNotIn("#20 ", body)
        self.assertNotIn("+5000", body)

    def test_the_charts_are_bars_and_lines_never_pies(self):
        gh, ctx = world()
        body = stats.render(ctx, stats.collect(ctx))
        self.assertNotIn("pie", body)
        self.assertEqual(body.count("```mermaid"), body.count("xychart-beta"))
        self.assertIn('title "Runs started, by subscription (last 6 hours)"', body)
        self.assertIn('title "Lines added in merged pull requests, by builder (last 24 hours)"',
                      body)
        lines = section(body, "Last 24 hours").splitlines()
        at = lines.index('    title "Lines added in merged pull requests, by builder '
                         '(last 24 hours)"')
        self.assertEqual(lines[at + 1:at + 4], ['    x-axis ["devin", "gpt"]', '    y-axis "lines"',
                                                '    bar [40, 100]'])
        self.assertIn('title "What runs ended in (all time)"', body)

    def test_a_quiet_window_says_so(self):
        gh, ctx = world()
        later = make_ctx(gh, at=DAY + timedelta(days=2))
        body = stats.render(later, stats.collect(later))
        self.assertIn("## Last 6 hours\n\nNothing happened.", body)
        self.assertIn("## Last 24 hours\n\nNothing happened.", body)
        self.assertIn("| Pull requests merged | 0 | 0 | 3 | 4 |", body)

    def test_a_body_too_long_for_github_lists_fewer_merges(self):
        gh, ctx = world()
        for number in range(100, 160):
            merged_pull(gh, number, 3, opened="2026-09-29T12:00:00Z",
                        merged="2026-09-29T15:00:00Z", additions=1, deletions=1)
        facts = stats.collect(ctx)
        full = stats.render(ctx, facts)
        # 64 merged in all: each window lists its newest MERGES_SHOWN.
        self.assertIn(f"_… and {64 - stats.MERGES_SHOWN} older ones._", full)
        with mock.patch.object(stats, "MAX_BODY", len(full) - 1):
            body = stats.fit(ctx, facts)
        self.assertLess(len(body), len(full))
        self.assertIn(f"_… and {64 - stats.MERGES_SHOWN // 2} older ones._", body)
        self.assertTrue(body.endswith(stats.MARKER))

    def test_it_is_opened_pinned_and_then_rewritten_every_hour(self):
        gh, ctx = world()
        note = stats.update(ctx)
        self.assertRegex(note, r"statistics: opened #\d+ and pinned it")
        number = ctx.store.load()["statistics"]["issue"]
        self.assertEqual(gh.threads[number]["title"], stats.TITLE)
        self.assertEqual(gh.pinned, [gh.threads[number]["node_id"]])
        self.assertEqual(stats.update(ctx), "statistics: not due")
        self.assertEqual(stats.update(ctx, force=True), f"statistics: rewrote #{number}")
        self.assertEqual(stats.STATS_EVERY, timedelta(hours=1))
        soon = make_ctx(gh, at=DAY + timedelta(minutes=50))
        self.assertEqual(stats.update(soon), "statistics: not due")
        later = make_ctx(gh, at=DAY + stats.STATS_EVERY + timedelta(minutes=1))
        self.assertEqual(stats.update(later), f"statistics: rewrote #{number}")
        # Someone closes it: the next rewrite opens a new one.
        gh.threads[number]["state"] = "closed"
        much_later = make_ctx(gh, at=DAY + 3 * stats.STATS_EVERY)
        self.assertRegex(stats.update(much_later), r"opened #\d+")


if __name__ == "__main__":
    unittest.main()
