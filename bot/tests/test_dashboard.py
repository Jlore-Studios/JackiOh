"""The pinned status issue (`dashboard.py`): opened and pinned once, rewritten every sweep, adopted
again when the state file loses it, and drawn from the same facts as `/harness status`."""

from __future__ import annotations

import argparse
import contextlib
import io
import json
import unittest
from datetime import timedelta
from unittest import mock

from harness import __main__ as main_mod
from harness import dashboard, providers, status
from harness.__main__ import cmd_dashboard
from harness.clock import iso
from harness.config import LABEL_BUILD, LABEL_PR, LABEL_REVISE, LABEL_WORKING
from harness.errors import GitHubError
from harness.state import item as state_item

from tests.fakes import OPERATOR, STRANGER, FakeGitHub
from tests.support import NIGHT, make_config, make_ctx, raw_providers
from tests.test_cross import ALL
from tests.test_providers import EARLY, too_close_to_start


def opened(gh: FakeGitHub) -> list[int]:
    return [n for n, t in gh.threads.items() if t["title"] == dashboard.TITLE]


def offline(gh: FakeGitHub | None = None):
    """The loop's contexts on a fake GitHub, so a test never reaches the network."""
    gh = gh or FakeGitHub()
    return mock.patch.object(main_mod, "_ctx", lambda cfg, **_: make_ctx(gh, at=NIGHT, cfg=cfg))


class DashboardTests(unittest.TestCase):
    def setUp(self):
        self.gh = FakeGitHub()
        self.ctx = make_ctx(self.gh, at=NIGHT)

    def test_opened_and_pinned_once_then_rewritten_in_place(self):
        note = dashboard.update(self.ctx)
        [number] = opened(self.gh)
        self.assertEqual(note, f"opened #{number} and pinned it")
        self.assertEqual(self.gh.pinned, [f"I_{number}"])
        self.assertEqual(self.gh.label_names(number), {"night bot"})
        self.assertIn(dashboard.MARKER, self.gh.threads[number]["body"])
        self.assertEqual(self.ctx.store.load()["dashboard"], {"issue": number})
        self.gh.add_issue(7, labels=(LABEL_BUILD, "difficulty:easy", "priority:high"))
        self.assertEqual(dashboard.update(self.ctx), f"rewrote #{number}")
        self.assertEqual(opened(self.gh), [number])  # no second issue, no second pin
        self.assertEqual(self.gh.pinned, [f"I_{number}"])
        body = self.gh.threads[number]["body"]
        row = next(line for line in body.splitlines() if line.startswith("| #7 |"))
        self.assertIn("| plan, then build | 🟢 easy → medium<br/><sub>a plan first</sub> | "
                      "▶️ **starts at the next run** on `claude-1` (`sonnet`, medium) | high |", row)

    def test_a_lost_number_adopts_a_trusted_issue_never_a_strangers(self):
        body = f"old\n\n{dashboard.MARKER}"
        self.gh.add_issue(40, dashboard.TITLE, body, user=STRANGER)
        self.gh.add_issue(41, dashboard.TITLE, body, user=OPERATOR)
        self.assertEqual(dashboard.update(self.ctx), "rewrote #41")
        self.assertEqual(self.ctx.store.load()["dashboard"], {"issue": 41})
        self.assertEqual(self.gh.threads[40]["body"], body)  # the stranger's is left alone

    def test_a_closed_issue_is_replaced_and_a_full_pin_list_is_said(self):
        self.gh.add_issue(41, dashboard.TITLE, dashboard.MARKER, user=OPERATOR, state="closed")
        self.ctx.store.update(lambda s: s.update(dashboard={"issue": 41}))
        self.gh.pinned = ["a", "b", "c"]
        note = dashboard.update(self.ctx)
        [number] = [n for n in opened(self.gh) if n != 41]
        self.assertIn(f"opened #{number}; could not pin it (422)", note)
        self.assertEqual(self.ctx.store.load()["dashboard"], {"issue": number})

    def test_the_visuals(self):
        for number in (37, 49):
            self.gh.add_issue(number, labels=(LABEL_WORKING,))
        self.gh.runs.update({"101": {"status": "in_progress"}, "102": {"status": "in_progress"}})
        ago = lambda minutes: iso(NIGHT - timedelta(minutes=minutes))
        def seed(state):
            state_item(state, 37).update(provider="claude-1", kind="build", run_id="101",
                                         started_at=ago(47))
            state_item(state, 49).update(provider="muse", kind="revise", run_id="102",
                                         started_at=ago(123))
            state["providers"]["claude-1"] = {"usage": {"five_hour": {"utilization": 0.3}}}
        self.ctx.store.update(seed, "seed")
        self.gh.add_pull(9, "bot/issue-3", "Make it so", labels=(LABEL_PR, LABEL_REVISE,
                                                                  "difficulty:hard"))
        self.gh.runs["200"] = {"id": 200, "status": "completed", "conclusion": "success",
                               "event": "schedule", "created_at": ago(30), "run_number": 812,
                               "run_started_at": ago(30), "updated_at": ago(26),
                               "html_url": "https://github.com/x/y/actions/runs/200"}
        body = dashboard.render(self.ctx)
        # What each lane is doing, the longest-running first. The start time is a clock time
        # that links to the run; hovering it says how long it had run when this was written.
        runs = "https://github.com/jgoetzmann/JackiOh/actions/runs"
        self.assertIn("| Item | Title | Doing | Subscription | Started (Central) |", body)
        muse_row = (f"| #49 | An issue | revising | `muse` `muse-spark-1.3-contributor` (machine) "
                    f"| [19:57]({runs}/102 \"running 2h 03m when this was written at 22:00\") |")
        claude_row = (f"| #37 | An issue | building | `claude-1` `opus` (GitHub) "
                      f"| [21:13]({runs}/101 \"running 47m when this was written at 22:00\") |")
        self.assertIn(muse_row, body)
        self.assertIn(claude_row, body)
        self.assertLess(body.index(muse_row), body.index(claude_row))
        self.assertIn("by `bot-status`, which rewrites this issue every 5 minutes", body)
        # A timeline of the runs going now: GitHub's runners, then the machine.
        self.assertIn("```mermaid\ngantt\n    title Runs going now, as of 22:00 (Central time)",
                      body)
        self.assertIn("    tickInterval 30minute\n    todayMarker off", body)
        self.assertIn("    section GitHub's runners\n    claude-1 · building 37 · 47m :active, "
                      "2026-09-29 21:13, 2026-09-29 22:00", body)
        self.assertIn("    section The night box\n    muse · revising 49 · 2h 03m :active, ", body)
        gantt = body.split("```mermaid\ngantt")[1].split("```")[0]
        self.assertNotIn("#", gantt)  # a gantt chart reads `#` as a comment
        # Every subscription as a box, with what it does or why not and its usage, always:
        # the Claude accounts, then the night box's.
        self.assertIn('subgraph hosted["Claude accounts, on GitHub\'s runners: 1 of 7 working"]',
                      body)
        self.assertIn('h0["<b>claude-1</b><br/>🟢 building #37 · since 21:13<br/>5h ▰▰▱▱▱ 30%"]'
                      ':::busy', body)
        self.assertIn('h1["<b>claude-2</b><br/>⚫ off: no secret set<br/>5h not read yet · cap 90%'
                      '<br/>7d not read yet · cap 90%"]:::off', body)
        slots = self.ctx.cfg.pool.machine_parallel
        self.assertIn(f'subgraph machine["The night box: 1 of {slots} slots in use"]', body)
        self.assertIn('m3["<b>muse</b><br/>🟢 revising #49 · since 19:57<br/>5h not read yet · '
                      'cap 95%<br/>7d not read yet"]:::busy', body)
        self.assertIn("m0 ~~~ m1 ~~~ m2 ~~~ m3", body)
        self.assertNotIn(":::free", body)  # no empty slot is drawn as a free subscription
        # The AI's training lanes run outside the harness: no box of theirs here.
        self.assertNotIn("training", body)
        # GitHub's lanes count claude-1's run; Muse's is in the night box's slots.
        self.assertIn("Lanes on GitHub's runners: 1 of 11 in use", body)
        self.assertNotIn("pie", body)
        # Each subscription, with its usage as a bar.
        self.assertIn("| `claude-1` |", body)
        self.assertIn("| `claude-1` | 🟢 building #37 |", body)
        self.assertIn("▰▰▰▱▱▱▱▱▱▱ 30%", body)
        # The queue and the last runs.
        self.assertIn("| #9 | Make it so | revise | 🔴 hard → strong | ⏳ **a strong model (Opus)**: "
                      "busy `claude-1` | — |", body)
        self.assertIn("> 🟢 **2 running** · 📋 **1 queued**: 1 waits for a strong model · 🔋 "
                      "**none of 11 subscriptions can start work now**", body)
        self.assertIn('| <span title="took 4m">Tue 21:30</span> | schedule | ✅ '
                      '| [812](https://github.com/x/y/actions/runs/200) |', body)
        # The full status, folded.
        self.assertIn("<details><summary>The full status</summary>", body)
        self.assertIn("**Night bot status**", body)

    def test_nothing_running(self):
        body = dashboard.render(self.ctx)
        self.assertIn("Nothing is running right now.", body)
        slots = self.ctx.cfg.pool.machine_parallel
        self.assertIn(f"The night box: 0 of {slots} slots in use", body)
        self.assertIn("Lanes on GitHub's runners: 0 of 11 in use", body)
        self.assertIn("Nothing is queued.", body)

    def test_the_loop_rewrites_it_every_ten_minutes_and_outlives_a_bad_tick(self):
        """`harness dashboard --every 600 --for 1200`, as bot-status runs it (with other
        figures): a rewrite now and every `--every` until the time is up, and a tick that fails
        is only a warning."""
        from unittest import mock
        import harness.__main__ as main_mod
        clock = [1000.0]
        ticks = []

        def update(ctx, extra=(), **_):
            ticks.append(clock[0])
            if len(ticks) == 2:
                raise RuntimeError("network down")
            return "rewrote #148"

        def sleep(seconds):
            clock[0] += seconds

        out = io.StringIO()
        with mock.patch.object(dashboard, "update", update), offline(), \
                mock.patch.object(main_mod.time, "monotonic", lambda: clock[0]), \
                mock.patch.object(main_mod.time, "sleep", sleep), contextlib.redirect_stdout(out):
            code = cmd_dashboard(make_config(), argparse.Namespace(every=600, for_seconds=1200))
        self.assertEqual(code, 0)
        self.assertEqual(ticks, [1000.0, 1600.0, 2200.0])
        self.assertIn("::warning::the status issue was not updated: network down", out.getvalue())
        self.assertEqual(out.getvalue().count("dashboard: rewrote #148"), 2)

    def test_the_loop_sweeps_before_each_rewrite(self):
        """`--sweep`: each tick runs the sweep first (it starts a night run when a lane and work
        are free), so a broken chain of runs restarts within ten minutes; a failed sweep is only
        a warning and the rewrite still happens."""
        from unittest import mock
        import harness.__main__ as main_mod
        clock = [0.0]
        order = []

        def sweep(ctx):
            order.append(("sweep", clock[0]))
            if len(order) == 1:
                raise RuntimeError("rate limited")
            return ["started a night run"]

        def update(ctx, extra=(), **_):
            order.append(("rewrite", clock[0]))
            return "rewrote #148"

        out = io.StringIO()
        with mock.patch.object(main_mod.sweep_mod, "sweep", sweep), offline(), \
                mock.patch.object(dashboard, "update", update), \
                mock.patch.object(main_mod.time, "monotonic", lambda: clock[0]), \
                mock.patch.object(main_mod.time, "sleep",
                                  lambda s: clock.__setitem__(0, clock[0] + s)), \
                contextlib.redirect_stdout(out):
            cmd_dashboard(make_config(), argparse.Namespace(every=600, for_seconds=600,
                                                            sweep=True))
        self.assertEqual(order, [("sweep", 0.0), ("rewrite", 0.0),
                                 ("sweep", 600.0), ("rewrite", 600.0)])
        self.assertIn("::warning::the sweep failed: rate limited", out.getvalue())
        self.assertIn("sweep: started a night run", out.getvalue())

    def test_the_loop_carries_the_statistics_and_harness_stats_counts_them_now(self):
        """`--stats` puts them in the status issue at each rewrite (counted when due); `harness
        stats --force` rewrites the issue now with them counted afresh."""
        calls = []

        def update(ctx, extra=(), **kwargs):
            calls.append(kwargs)
            return "rewrote #148"

        out = io.StringIO()
        with mock.patch.object(dashboard, "update", update), offline(), \
                contextlib.redirect_stdout(out):
            cmd_dashboard(make_config(), argparse.Namespace(stats=True))
            main_mod.cmd_stats(make_config(), argparse.Namespace(force=True))
        self.assertEqual([(c["stats"], c.get("force_stats", False)) for c in calls],
                         [(True, False), (True, True)])
        self.assertIn("dashboard: rewrote #148", out.getvalue())

    def test_each_idle_account_says_why_in_its_box(self):
        """A Claude account with no run: open, at a cap, outside its hours, resting after a run
        that could not work, or off, each with the time it changes."""
        later = iso(NIGHT + timedelta(minutes=70))
        self.ctx = make_ctx(self.gh, at=NIGHT, cfg=make_config(env=ALL))  # every secret set

        def seed(state):
            state["providers"]["claude-2"] = {"usage": {"five_hour": {"utilization": 0.95,
                                                                      "resets_at": later}}}
            state["providers"]["claude-3"] = {"infra": {"at": iso(NIGHT - timedelta(minutes=10)),
                                                        "reason": "401", "streak": 2}}
        self.ctx.store.update(seed, "seed")
        state = self.ctx.store.load()
        pool = self.ctx.cfg.pool
        self.assertEqual(dashboard._account(self.ctx, state, pool.get("claude-1")),
                         ("idle", "⚪ idle"))
        self.assertEqual(dashboard._account(self.ctx, state, pool.get("claude-2")),
                         ("paused", "⏸️ 5-hour 95% of its 90% cap<br/>starts again 23:10"))
        self.assertEqual(dashboard._account(self.ctx, state, pool.get("claude-3")),
                         ("broken", "⛔ its last run could not work<br/>tries again 23:50"))
        day = make_ctx(self.gh, at=NIGHT + timedelta(hours=12), cfg=self.ctx.cfg)
        self.assertEqual(dashboard._account(day, day.store.load(), pool.get("claude-1")),
                         ("closed", "🌙 outside its hours<br/>opens 21:00"))
        unset = make_ctx(self.gh, at=NIGHT, cfg=make_config(env={"HARNESS_SECRETS_SET": ""}))
        self.assertEqual(dashboard._account(unset, state, unset.cfg.pool.get("claude-4")),
                         ("off", "⚫ off: no secret set"))

    def test_an_account_too_close_to_its_cap_to_start_is_paused_not_open(self):
        """claude-4 and claude-6 at 01:00, outside their hours: under their 50% cap there but not
        `start_headroom` under it, so the planner starts no build or plan on them. Their boxes
        and rows say so, and when each can start one again; the usage cells carry the caps in
        and outside their hours, the line a build starts under and the reset; and the full
        status says the same."""
        cfg = make_config(env=ALL, committed_hours=True)
        ctx = make_ctx(self.gh, at=EARLY, cfg=cfg)
        ctx.store.update(lambda s: s["providers"].update(too_close_to_start()["providers"]),
                         "seed")
        state = ctx.store.load()
        pool = cfg.pool
        self.assertEqual(dashboard._account(ctx, state, pool.get("claude-4")),
                         ("paused", "⏸️ 5-hour 49% of its 50% cap<br/>long runs start under 45%; "
                                    "reviews only<br/>starts again 03:00"))
        self.assertEqual(dashboard._account(ctx, state, pool.get("claude-6")),
                         ("paused", "⏸️ 5-hour 45% of its 50% cap<br/>long runs start under 45%; "
                                    "reviews only<br/>starts again 02:50"))
        body = dashboard.render(ctx)
        self.assertNotIn("⚪ idle", body.split("## 📋 Queue")[0].split("claude-4")[1][:80])
        self.assertIn("| Subscription | Now | 5-hour session | 7-day week | Read | Models (tier) "
                      "| Hours |", body)
        self.assertIn("| `claude-4` | ⏸️ 5-hour 49% of its 50% cap<br/>long runs start under 45%; "
                      "reviews only<br/>starts again 03:00 | ▰▰▰▰▰▱▱▱▱▱ 49%<br/><sub>cap 50% now, "
                      "70% in its hours · builds and plans start under 45% · resets 04:10</sub> "
                      "| —<br/><sub>not reported</sub> | "
                      '<span title="read at Thu 23:37">1h 22m ago</span> |', body)
        self.assertIn("`claude-4` (claude: `opus` strong, `sonnet` medium; 03:00–15:00 "
                      "America/Chicago, outside them up to 50% of 5-hour): 5-hour usage is 49%, "
                      "too close to its 50% cap outside its hours", body)
        self.assertIn("Usage: 5-hour 49% (cap 50% now, 70% in its hours; builds and plans start "
                      "under 45%; resets in 3h 10m) (read 1h 22m ago).", body)
        # In its hours the other cap is the one outside them; an uncapped account says so.
        day = make_ctx(self.gh, at=EARLY + timedelta(hours=3), cfg=cfg)
        self.assertEqual(status.cap_text(day, pool.get("claude-4"), "five_hour"),
                         "cap 70% now, 50% outside its hours")
        self.assertEqual(status.cap_text(day, pool.get("claude-3"), "five_hour"), "no cap")
        # A refusal already names its reset: no second time for it.
        self.assertIn("claude-3` (claude: `opus` strong, `sonnet` medium; any time): it refused "
                      "a call; its limit resets at 2026-10-09T12:00:00Z. Usage:", body)

    def test_a_cell_stays_one_line_without_pipes(self):
        self.assertEqual(dashboard._cell("a | b\nc"), "a \\| b c")
        self.assertEqual(dashboard.bar(None), "—")
        self.assertEqual(dashboard.bar(1.4), "▰" * 10 + " 100%")

    def test_a_failure_never_fails_the_sweep(self):
        def broken(*args, **kwargs):
            raise GitHubError("server error", 502)
        cfg = make_config()
        out = io.StringIO()
        from unittest import mock
        with mock.patch.object(dashboard, "update", broken), offline(), \
                contextlib.redirect_stdout(out):
            self.assertEqual(cmd_dashboard(cfg, argparse.Namespace()), 0)
        self.assertIn("::warning::the status issue was not updated", out.getvalue())


class StatisticsTests(unittest.TestCase):
    """The statistics live in the status issue (#148; they had an issue of their own, #190):
    counted afresh every 30 minutes, kept as they are between counts, and cut to fit GitHub's
    limit on an issue's body."""

    def test_counted_every_30_minutes_and_kept_between(self):
        from harness import stats
        from tests.support import DAY
        from tests.test_stats import world
        gh, ctx = world()
        note = dashboard.update(ctx, stats=True)
        [number] = opened(gh)
        self.assertEqual(note, f"opened #{number} and pinned it, with the statistics counted afresh")
        body = gh.threads[number]["body"]
        block = stats.kept(body)
        self.assertIn("## 📊 What it has done", block)
        self.assertIn("| Runs started | 4 | 5 | 6 | 6 |", block)
        self.assertLess(body.index("## 🏃 Last night-bot runs"), body.index(stats.BLOCK_START))
        self.assertLess(body.index(stats.BLOCK_END), body.index(dashboard.MARKER))
        self.assertEqual(ctx.store.load()["statistics"], {"at": iso(DAY)})
        # Twenty minutes on, nothing is counted: the block the issue holds is kept as it is.
        soon = make_ctx(gh, at=DAY + timedelta(minutes=20))
        with mock.patch.object(stats, "collect", side_effect=AssertionError("counted")):
            self.assertEqual(dashboard.update(soon, stats=True), f"rewrote #{number}")
        self.assertEqual(stats.kept(gh.threads[number]["body"]), block)
        self.assertIn("Updated 2026-09-29 12:20 CDT", gh.threads[number]["body"])
        # Forced, or once 30 minutes have passed, they are counted afresh.
        self.assertTrue(dashboard.update(soon, stats=True, force_stats=True).endswith("afresh"))
        later = make_ctx(gh, at=DAY + timedelta(minutes=51))
        self.assertTrue(dashboard.update(later, stats=True).endswith("afresh"))
        # No issue of their own: one pinned issue, and no "Night bot statistics" in it.
        self.assertEqual(opened(gh), [number])
        self.assertEqual(gh.pinned, [f"I_{number}"])
        self.assertFalse(any(t["title"] == "Night bot statistics" for t in gh.threads.values()))
        # Without --stats (Squishy's own loop, a test) the issue carries none.
        dashboard.update(later)
        self.assertIsNone(stats.kept(gh.threads[number]["body"]))

    def test_a_body_too_long_for_github_lists_fewer_merges_then_none(self):
        from harness import stats
        from tests.test_stats import merged_pull, world
        gh, ctx = world()
        for n in range(100, 160):
            merged_pull(gh, n, 3, opened="2026-09-29T12:00:00Z", merged="2026-09-29T15:00:00Z",
                        additions=1, deletions=1)
        facts = stats.collect(ctx)
        view = dashboard.look(ctx)
        full = dashboard.render(ctx, (), stats.block(ctx, facts), view)

        def draw(shown):
            return stats.block(ctx, facts, shown=shown)
        with mock.patch.object(dashboard, "MAX_BODY", len(full) - 1):
            body = dashboard.fit(ctx, view, (), draw)
        self.assertIn(f"_… and {64 - stats.MERGES_SHOWN // 2} older ones._", body)
        with mock.patch.object(dashboard, "MAX_BODY", len(dashboard.render(ctx, (), "", view))):
            body = dashboard.fit(ctx, view, (), draw)
        self.assertIsNone(stats.kept(body))  # left out last of all
        self.assertTrue(body.endswith(dashboard.MARKER))


class FreshEachTickTests(unittest.TestCase):
    """The loop's job runs for hours, and GitHub fixes its secrets when its run is created (hours
    before it starts, for a run queued behind the last loop): each tick reads the subscriptions
    from the default branch again and takes the newest plan job's record of the secrets."""

    def loop(self, gh, cfg, ticks, between=lambda tick: None):
        seen = []
        clock = [0.0]

        def update(ctx, extra=(), **_):
            seen.append(ctx.cfg)
            between(len(seen))
            return "rewrote #148"

        out = io.StringIO()
        with mock.patch.object(dashboard, "update", update), offline(gh), \
                mock.patch.object(main_mod.time, "monotonic", lambda: clock[0]), \
                mock.patch.object(main_mod.time, "sleep",
                                  lambda s: clock.__setitem__(0, clock[0] + s)), \
                contextlib.redirect_stdout(out):
            cmd_dashboard(cfg, argparse.Namespace(every=600, for_seconds=600 * (ticks - 1)))
        return seen, out.getvalue()

    def test_a_secret_a_plan_job_saw_after_this_run_was_created_shows_at_the_next_tick(self):
        gh = FakeGitHub()
        gh.runs["777"] = {"status": "in_progress", "created_at": "2026-09-30T01:00:00Z"}
        cfg = make_config(env={"HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN"})
        store = make_ctx(gh, cfg=cfg).store
        # A record older than this run says nothing new: its own list stands.
        store.update(lambda s: s.update(secrets={"set": ["CLAUDE_CODE_OAUTH_TOKEN"],
                                                 "at": "2026-09-30T00:30:00Z"}), "seed")

        def add_token_4(tick):
            if tick == 1:  # a plan job, after this run was created, has token 4
                store.update(lambda s: s.update(secrets={
                    "set": ["CLAUDE_CODE_OAUTH_TOKEN", "CLAUDE_CODE_OAUTH_TOKEN_4"],
                    "at": "2026-09-30T02:00:00Z"}), "plan")

        seen, _ = self.loop(gh, cfg, 2, add_token_4)
        self.assertEqual([c.secrets.has("CLAUDE_CODE_OAUTH_TOKEN_4") for c in seen],
                         [False, True])
        self.assertTrue(all(c.secrets.has("CLAUDE_CODE_OAUTH_TOKEN") for c in seen))

    def test_a_subscription_changed_on_main_shows_at_the_next_tick(self):
        gh = FakeGitHub()
        raw = raw_providers()
        raw["providers"]["claude-4"]["limits"]["five_hour"] = 0.33

        def change_main(tick):
            if tick == 1:
                gh.files[("main", ".harness/providers.json")] = (json.dumps(raw), "b1")
            if tick == 2:
                gh.files[("main", ".harness/providers.json")] = ("{not json", "b2")

        seen, out = self.loop(gh, make_config(), 3, change_main)
        caps = [c.pool.get("claude-4").limits.stops["five_hour"] for c in seen]
        # The file is not on main in the first tick; then the change; then a broken file keeps it.
        self.assertEqual(caps[1:], [0.33, 0.33])
        self.assertNotEqual(caps[0], 0.33)
        self.assertIn("::warning::kept the subscriptions and secrets as they were", out)

    def test_subscriptions_this_checkout_cannot_read_end_the_loop(self):
        """A subscription on main that only newer code reads (claude-7's new secret, which kept a
        loop started before it drawing six accounts): the loop ends, without a rewrite, and the
        next one starts on main's code."""
        gh = FakeGitHub()
        raw = raw_providers()
        raw["providers"]["claude-4"]["secret"] = "CLAUDE_CODE_OAUTH_TOKEN_99"

        def change_main(tick):
            if tick == 1:
                gh.files[("main", ".harness/providers.json")] = (json.dumps(raw), "b1")

        seen, out = self.loop(gh, make_config(), 3, change_main)
        self.assertEqual(len(seen), 1)
        self.assertIn("::notice::this checkout cannot read the subscriptions on main", out)
        self.assertIn("CLAUDE_CODE_OAUTH_TOKEN_99", out)
        self.assertNotIn("::warning::kept the subscriptions", out)

    def test_a_plan_job_records_the_secrets_it_saw(self):
        from harness import plan as plan_mod
        gh = FakeGitHub()
        both = "CLAUDE_CODE_OAUTH_TOKEN CLAUDE_CODE_OAUTH_TOKEN_5"
        ctx = make_ctx(gh, cfg=make_config(env={"HARNESS_SECRETS_SET": both}))
        plan_mod.make(ctx)
        self.assertEqual(ctx.store.load()["secrets"],
                         {"set": ["CLAUDE_CODE_OAUTH_TOKEN", "CLAUDE_CODE_OAUTH_TOKEN_5"],
                          "at": iso(NIGHT)})

    def test_the_record_and_which_list_wins(self):
        known = providers.Secrets(frozenset({"A"}), True)
        record = providers.secrets_record(known, {}, NIGHT)
        self.assertEqual(record, {"set": ["A"], "at": iso(NIGHT)})
        state = {"secrets": record}
        # The same list is written again only every SECRETS_NOTE_EVERY; a new one at once.
        self.assertIsNone(providers.secrets_record(known, state, NIGHT + timedelta(minutes=10)))
        self.assertIsNotNone(providers.secrets_record(
            known, state, NIGHT + providers.SECRETS_NOTE_EVERY))
        self.assertIsNotNone(providers.secrets_record(
            providers.Secrets(frozenset({"A", "B"}), True), state, NIGHT + timedelta(minutes=1)))
        # Run by hand, nothing is known, so nothing is written.
        self.assertIsNone(providers.secrets_record(providers.Secrets(frozenset(), False), {}, NIGHT))
        own = providers.Secrets(frozenset({"B"}), True)
        self.assertEqual(providers.newer_secrets(own, state, NIGHT - timedelta(hours=1)).present,
                         {"A"})
        self.assertIs(providers.newer_secrets(own, state, NIGHT + timedelta(hours=1)), own)
        self.assertEqual(providers.newer_secrets(own, state, None).present, {"A"})
        self.assertIs(providers.newer_secrets(own, {}, None), own)


if __name__ == "__main__":
    unittest.main()
