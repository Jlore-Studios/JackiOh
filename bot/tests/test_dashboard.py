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
from harness import cardsets, dashboard, providers, status
from harness.__main__ import cmd_dashboard
from harness.clock import iso
from harness.config import LABEL_BUILD, LABEL_PR, LABEL_REVISE, LABEL_WORKING
from harness.errors import GitHubError
from harness.state import item as state_item

from tests.fakes import OPERATOR, STRANGER, FakeGitHub
from tests.support import NIGHT, ROOT, make_config, make_ctx, raw_providers
from tests.test_cross import ALL
from tests.test_providers import EARLY, too_close_to_start


def opened(gh: FakeGitHub) -> list[int]:
    return [n for n, t in gh.threads.items() if t["title"] == dashboard.TITLE]


def offline(gh: FakeGitHub | None = None):
    """The loop's contexts on a fake GitHub, so a test never reaches the network."""
    gh = gh or FakeGitHub()
    return mock.patch.object(main_mod, "_ctx", lambda cfg, **_: make_ctx(gh, at=NIGHT, cfg=cfg))


class TrainingBoxTests(unittest.TestCase):
    """The training box's lanes in the status diagram, from their pull requests (#1272)."""

    def setUp(self) -> None:
        # The lanes' drafts as they were drawn until #1677 stopped them, so a lane named again
        # draws the same way.
        patch = mock.patch.object(dashboard, "TRAINING_LANES", ("improve", "unban"))
        patch.start()
        self.addCleanup(patch.stop)

    def boxes(self, gh: FakeGitHub) -> str:
        return "\n".join(dashboard.training_boxes(make_ctx(gh, at=NIGHT, cfg=make_config())))

    def lane(self, gh: FakeGitHub, number: int, lane: str, title: str, *, draft: bool,
             updated: timedelta) -> None:
        pull = gh.add_pull(number, f"ai/{lane}", title)
        pull.update(draft=draft, updated_at=iso(NIGHT - updated))

    def test_a_draft_is_a_session_at_work_and_an_idle_title_says_so(self):
        gh = FakeGitHub()
        self.lane(gh, 70, "improve", "AI gen 3 (improve): in training, nothing promoted yet",
                  draft=True, updated=timedelta(minutes=20))
        self.lane(gh, 71, "unban", "AI gen 9 (unban): idle, the AI on main bans no card",
                  draft=True, updated=timedelta(days=3))
        body = self.boxes(gh)
        self.assertIn('subgraph training["The training box, on Muse: 1 of 2 lanes working"]', body)
        self.assertIn('t0["<b>improve</b><br/>🟢 a Muse session on #70<br/>updated', body)
        self.assertIn(":::busy", body)
        # An idle unban lane's draft goes days unchanged, and that is not a stopped lane.
        self.assertIn('t1["<b>unban</b><br/>⏸️ idle on #71<br/>the AI on main bans no card"]'
                      ':::paused', body)
        self.assertIn('t2["<b>gather</b><br/>no model, no pull request<br/>its log is on the box"]'
                      ':::closed', body)

    def test_a_promotion_waits_on_ci_and_a_draft_left_unchanged_is_a_stopped_lane(self):
        gh = FakeGitHub()
        self.lane(gh, 72, "improve", "AI gen 4 (improve): the opening plays its two-drops first",
                  draft=False, updated=timedelta(minutes=5))
        self.lane(gh, 73, "unban", "AI gen 9 (unban): in training, nothing promoted yet",
                  draft=True, updated=dashboard.TRAINING_STALE + timedelta(hours=1))
        body = self.boxes(gh)
        self.assertIn("0 of 2 lanes working", body)
        self.assertIn('t0["<b>improve</b><br/>▶️ promotion #72<br/>waiting on CI<br/>updated',
                      body)
        self.assertIn(":::next", body)
        self.assertIn('t1["<b>unban</b><br/>⛔ #73 unchanged<br/>updated', body)
        self.assertIn("is the lane running?\"]:::broken", body)

    def test_with_the_lanes_stopped_the_box_shows_gather_alone(self):
        """#1677: `improve` and `unban` are stopped, so an `ai/` draft left open draws nothing."""
        gh = FakeGitHub()
        self.lane(gh, 74, "improve", "AI gen 14 (improve): in training, nothing promoted yet",
                  draft=True, updated=timedelta(minutes=20))
        with mock.patch.object(dashboard, "TRAINING_LANES", ()):
            body = self.boxes(gh)
        self.assertIn('subgraph training["The training box: gather alone, no model"]', body)
        self.assertIn('t0["<b>gather</b><br/>no model, no pull request<br/>its log is on the box"]'
                      ':::closed', body)
        self.assertNotIn("improve", body)
        self.assertNotIn("Muse", body)

    def test_unreadable_pull_requests_draw_no_training_box(self):
        gh = FakeGitHub()
        with mock.patch.object(gh, "list_pulls", side_effect=GitHubError("boom")):
            self.assertEqual(self.boxes(gh), "")


class LearnLaneTests(unittest.TestCase):
    """The learn lane's section (#466): read from GitHub alone, since no harness runs the lane."""

    def run_with(self, gh: FakeGitHub, run_id: int, *, number: int, event: str = "schedule",
                 status: str = "completed", jobs: list[dict] | None = None) -> None:
        gh.runs[str(run_id)] = {"id": run_id, "workflow": dashboard.LEARN_WORKFLOW, "event": event,
                                "status": status, "conclusion": "success", "run_number": number,
                                "created_at": iso(NIGHT - timedelta(days=1)),
                                "html_url": f"https://github.com/x/y/actions/runs/{run_id}"}
        gh.jobs[str(run_id)] = jobs or []

    @staticmethod
    def retrain(**steps: str) -> list[dict]:
        names = {"fit": "retrain and screen (training/ml/retrain.sh)",
                 "gate": "the gate on main as it is now (training/ml/learn-promote.sh)",
                 "deliver": "deliver (push ai/learn, open its pull request, auto-merge)",
                 "moving": "main kept moving, so another run on this export"}
        return [{"name": "plan", "conclusion": "success"},
                {"name": "export / plan the shards", "conclusion": "success"},
                {"name": "retrain, screen, gate", "conclusion": "success",
                 "steps": [{"name": names[key], "conclusion": value} for key, value in steps.items()]}]

    def test_each_way_a_run_ends(self):
        outcome = lambda jobs, status="completed": dashboard.learn_outcome({"status": status}, jobs)
        self.assertEqual(outcome([], status="in_progress"), "🔄 running")
        self.assertEqual(outcome([{"name": "plan", "conclusion": "success"},
                                  {"name": "retrain, screen, gate", "conclusion": "skipped"}]),
                         "⏭️ waited on its open pull request")
        # Before the retrain went on with the shards that finished, a failed shard stopped the run.
        self.assertEqual(outcome([{"name": "export / shard from 1", "conclusion": "failure",
                                   "runner_name": "pc-selfplay-1021011405"}]),
                         "❌ the export failed")
        self.assertEqual(outcome(self.retrain(fit="success", gate="skipped")), "screened out")
        self.assertEqual(outcome(self.retrain(fit="success", gate="success", deliver="skipped",
                                              moving="skipped")), "gated out")
        self.assertEqual(outcome(self.retrain(fit="success", gate="success", deliver="success")),
                         "🎉 promoted")
        self.assertIn("LEARN_LANE_TOKEN", outcome(self.retrain(fit="success", gate="success",
                                                               deliver="failure")))
        self.assertEqual(outcome(self.retrain(fit="success", gate="success", deliver="skipped",
                                              moving="success")),
                         "🔁 main kept moving: another run on the export")
        # A run from before the lane gated on the newest main ("the gate, once") reads the same.
        old = self.retrain(fit="success")
        old[2]["steps"].append({"name": "the gate, once (promote --lane learn)", "conclusion": "skipped"})
        self.assertEqual(outcome(old), "screened out")

    def test_a_run_the_pcs_runner_never_took_or_took_in_part(self):
        """Every job runs on the owner's PC, and GitHub fails a job no runner takes in 24 hours: a
        job that ended with no runner is the PC off, asleep or busy, not the lane broken."""
        outcome = lambda jobs, conclusion="failure": dashboard.learn_outcome(
            {"status": "completed", "conclusion": conclusion}, jobs)
        never = lambda name: {"name": name, "conclusion": "cancelled", "runner_name": None}
        took = lambda name, conclusion="success": {"name": name, "conclusion": conclusion,
                                                   "runner_name": "pc-selfplay-1021011405"}
        self.assertEqual(outcome([never("plan"), {"name": "retrain, screen, gate",
                                                  "conclusion": "skipped"}]),
                         dashboard.LEARN_NO_RUNNER)
        # The retrain goes on with the shards that finished; with none, the shards step stops it.
        none_finished = [took("plan"), took("export / plan the shards"),
                         never("export / shard from 1"), never("export / shard from 5001"),
                         {**took("retrain, screen, gate", "failure"), "steps": [
                             {"name": "the export's shards", "conclusion": "failure"}]}]
        self.assertEqual(outcome(none_finished), dashboard.LEARN_NO_RUNNER)
        none_finished[2] = took("export / shard from 1", "failure")
        self.assertEqual(outcome(none_finished), "❌ the export failed")
        # Some shards lost, the rest retrained on: the run reads as the retrain ended.
        some = self.retrain(fit="success", gate="skipped")
        some[1:1] = [never("export / shard from 5001"), took("export / shard from 1")]
        self.assertEqual(outcome(some, "cancelled"), "screened out")
        self.assertEqual(outcome([took("plan"), {"name": "retrain, screen, gate",
                                                 "conclusion": "skipped"}], "cancelled"),
                         "⏹️ cancelled")

    def test_the_next_run_is_the_next_monday_at_0623_utc(self):
        wednesday = NIGHT  # 2026-09-30 03:00 UTC
        self.assertEqual(dashboard.next_learn_run(wednesday).isoformat(), "2026-10-05T06:23:00+00:00")
        monday_after = NIGHT.replace(day=28, hour=7)
        self.assertEqual(dashboard.next_learn_run(monday_after).isoformat(), "2026-10-05T06:23:00+00:00")
        monday_before = NIGHT.replace(day=28, hour=5)
        self.assertEqual(dashboard.next_learn_run(monday_before).isoformat(), "2026-09-28T06:23:00+00:00")

    def test_the_section_says_what_is_on_main_its_pull_request_and_its_runs(self):
        gh = FakeGitHub()
        gh.files[("main", "crates/ai/generation.json")] = (json.dumps(
            {"generation": 13, "lane": "learn", "date": "2026-09-29", "vsParent": "220/400"}), "sha1")
        pull = gh.add_pull(1477, "ai/learn", "AI gen 14 (learn): the learned evaluation")
        pull.update(created_at=iso(NIGHT - timedelta(hours=2)), html_url="https://github.com/x/y/pull/1477")
        self.run_with(gh, 31, number=20, jobs=self.retrain(fit="success", gate="success",
                                                           deliver="success"))
        self.run_with(gh, 32, number=21, event="workflow_run")
        self.run_with(gh, 33, number=22, status="in_progress")
        body = "\n".join(dashboard.learn_lane(make_ctx(gh, at=NIGHT, cfg=make_config())))
        self.assertIn("## 🧠 The learn lane", body)
        self.assertIn("- **The AI on main:** gen 13 (learn), promoted 2026-09-29, 220/400 against its "
                      "parent.", body)
        self.assertIn("▶️ [#1477](https://github.com/x/y/pull/1477) open since", body)
        self.assertIn("CI decides, and auto-merge lands it.", body)
        self.assertIn("- **Next scheduled run:** Mon Oct 05 01:23 (Central).", body)
        self.assertIn("| 🎉 promoted | [20](https://github.com/x/y/actions/runs/31) |", body)
        self.assertIn("| 🔄 running | [22]", body)
        # A watch run (CI on the lane's pull request) is not one of the lane's runs.
        self.assertNotIn("[21]", body)

    def test_a_promotion_open_too_long_is_one_the_next_run_closes(self):
        gh = FakeGitHub()
        pull = gh.add_pull(1477, "ai/learn", "AI gen 14 (learn): the learned evaluation")
        pull.update(created_at=iso(NIGHT - dashboard.LEARN_PR_STALE - timedelta(hours=1)))
        body = "\n".join(dashboard.learn_lane(make_ctx(gh, at=NIGHT, cfg=make_config())))
        self.assertIn("⛔ [#1477]", body)
        self.assertIn("the next run closes it.", body)
        self.assertIn("- **The AI on main:** no generation.json.", body)
        self.assertIn("No runs yet.", body)

    def test_unreadable_github_still_draws_the_section(self):
        gh = FakeGitHub()
        with mock.patch.object(gh, "list_pulls", side_effect=GitHubError("down", 502)), \
                mock.patch.object(gh, "list_runs", side_effect=GitHubError("down", 502)):
            body = "\n".join(dashboard.learn_lane(make_ctx(gh, at=NIGHT, cfg=make_config())))
        self.assertIn("- **Its pull request:** could not be read.", body)
        self.assertIn("Its runs could not be read.", body)

    def test_the_issue_carries_it_after_the_night_bots_runs(self):
        ctx = make_ctx(FakeGitHub(), at=NIGHT, cfg=make_config())
        body = dashboard.render(ctx)
        self.assertLess(body.index("## 🏃 Last night-bot runs"), body.index("## 🧠 The learn lane"))


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
        self.assertTrue(body.startswith("**Last updated at 2026-09-29 22:00 CDT**\n\n# 🤖 "))
        self.assertIn("`bot-status` rewrites this issue every 5 minutes", body)
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
        self.assertIn('subgraph hosted["Claude accounts, on GitHub\'s runners: 1 of 8 working"]',
                      body)
        self.assertIn('h0["<b>claude-1</b><br/>🟢 building #37 · since 21:13<br/>5h ▰▰▱▱▱ 30%"]'
                      ':::busy', body)
        self.assertIn('h1["<b>claude-2</b><br/>⚫ off: no secret set<br/>5h not read yet · cap 90%'
                      '<br/>7d not read yet · cap 90%"]:::off', body)
        slots = self.ctx.cfg.pool.machine_parallel
        self.assertIn(f'subgraph machine["The night box: 1 of {slots} slots in use"]', body)
        self.assertIn('m3["<b>muse</b><br/>🟢 revising #49 · since 19:57<br/>5h not read yet'
                      '<br/>7d not read yet"]:::busy', body)
        self.assertIn("m0 ~~~ m1 ~~~ m2 ~~~ m3", body)
        self.assertNotIn(":::free", body)  # no empty slot is drawn as a free subscription
        # The training box runs outside the harness, and since #1677 stopped its two LLM lanes
        # it shows `gather` alone.
        self.assertIn('subgraph training["The training box: gather alone, no model"]', body)
        self.assertIn('t0["<b>gather</b><br/>no model, no pull request<br/>its log is on the box"]'
                      ':::closed', body)
        self.assertIn("machine ~~~ training", body)
        # GitHub's lanes count claude-1's run; Muse's is in the night box's slots.
        self.assertIn("Lanes on GitHub's runners: 1 of 8 in use", body)
        self.assertNotIn("pie", body)
        # Each subscription, with its usage as a bar.
        self.assertIn("| `claude-1` |", body)
        self.assertIn("| `claude-1` | 🟢 building #37 |", body)
        self.assertIn("▰▰▰▱▱▱▱▱▱▱ 30%", body)
        # The queue and the last runs.
        self.assertIn("| #9 | Make it so | revise | 🔴 hard → strong | ⏳ **a strong model (Opus)**: "
                      "busy `claude-1` | — |", body)
        self.assertIn("> 🟢 **2 running** · 📋 **1 queued**: 1 waits for a strong model · 🔋 "
                      "**none of 12 subscriptions can start work now**", body)
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
        self.assertIn("Lanes on GitHub's runners: 0 of 8 in use", body)
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
        # Both read which set's cards win most from production (cardsets.py).
        self.assertEqual([c["read_cards"] for c in calls], [cardsets.fetch_json] * 2)
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
        self.assertIn("Last updated at 2026-09-29 12:20 CDT", gh.threads[number]["body"])
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

    def test_more_runners_busy_are_cut_to_the_room_before_a_count(self):
        """Between counts the issue keeps its statistics; when more runners are busy than the
        issue has room for, fewer are listed rather than the statistics counted again."""
        from harness import stats
        from tests.support import DAY
        from tests.test_stats import world
        gh, ctx = world()
        dashboard.update(ctx, stats=True)
        [number] = opened(gh)
        for n in range(12):
            gh.runs[str(900 + n)] = {"id": 900 + n, "status": "in_progress",
                                     "path": ".github/workflows/ci.yml", "event": "push",
                                     "display_title": "Patch: something", "created_at": iso(DAY)}
            gh.jobs[str(900 + n)] = [{"name": f"shard {n}", "status": "in_progress",
                                      "labels": ["ubuntu-latest"],
                                      "runner_name": f"GitHub Actions {n}"}]
        soon = make_ctx(gh, at=DAY + timedelta(minutes=20))
        body = dashboard.render(soon, (), stats.kept(gh.threads[number]["body"]))
        with mock.patch.object(stats, "collect", side_effect=AssertionError("counted")), \
                mock.patch.object(dashboard, "MAX_BODY", len(body) - 500):
            self.assertEqual(dashboard.update(soon, stats=True), f"rewrote #{number}")
        body = gh.threads[number]["body"]
        self.assertIn("…and ", body)
        self.assertIsNotNone(stats.kept(body))

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
        # Then fewer busy runners, with no merge listed, before the statistics go.
        busy = {"name": "work", "status": "in_progress", "labels": ["ubuntu-latest"],
                "runner_name": "GitHub Actions 1"}
        gh.runs["900"] = {"id": 900, "status": "in_progress", "path": "x/ci.yml", "event": "push"}
        gh.jobs["900"] = [dict(busy, runner_name=f"GitHub Actions {n}") for n in range(6)]
        view = dashboard.look(ctx)
        view.found = dashboard.read_runners(ctx)
        view.runners = dashboard.runners_section(ctx, view.state, view.found)
        bare = dashboard.render(ctx, (), stats.block(ctx, facts, shown=0), view)
        with mock.patch.object(dashboard, "MAX_BODY", len(bare) - 1):
            body = dashboard.fit(ctx, view, (), draw)
        self.assertRegex(body, r"\n…and \d more\.\n")
        self.assertLessEqual(len(body), len(bare) - 1)
        self.assertIsNotNone(stats.kept(body))
        view.runners = dashboard.runners_section(ctx, view.state, view.found, room=0)
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


class RunnersTests(unittest.TestCase):
    """Every runner and what it is doing (`runners_section`): each job a runner holds, the step
    it is on and what it is for, the jobs queued for one, the self-hosted labels with no job, and
    what each runner and workflow is for; redrawn alone every two minutes (`refresh_runners`)."""

    def setUp(self):
        self.gh = FakeGitHub()
        self.ctx = make_ctx(self.gh, at=NIGHT, cfg=make_config())
        ago = lambda minutes: iso(NIGHT - timedelta(minutes=minutes))

        def seed(state):
            state_item(state, 37).update(provider="claude-1", kind="build", run_id="101",
                                         started_at=ago(47))
            # A run the status loop started, before it found the run's id (#1455).
            state_item(state, 49).update(provider="muse", kind="revise",
                                         run_id="pending:49-1790000000", started_at=ago(20))
        self.ctx.store.update(seed, "seed")
        night = ".github/workflows/bot-night.yml"
        self.gh.runs.update({
            "101": {"id": 101, "status": "in_progress", "path": night, "display_title": "bot-night",
                    "event": "workflow_dispatch", "created_at": ago(48)},
            "102": {"id": 102, "status": "in_progress", "path": night,
                    "display_title": "bot-night 49-1790000000 on night-vm-muse",
                    "event": "workflow_dispatch", "created_at": ago(20)},
            "103": {"id": 103, "status": "in_progress", "path": ".github/workflows/ci.yml",
                    "display_title": "Patch: a Sets tab", "event": "pull_request",
                    "pull_requests": [{"number": 1966}], "created_at": ago(4)},
            "104": {"id": 104, "status": "queued", "path": ".github/workflows/learn.yml",
                    "display_title": "learn", "event": "schedule", "created_at": ago(90)},
        })
        doing = {"name": "Build, check and review", "status": "in_progress"}
        self.gh.jobs.update({
            "101": [{"name": "plan", "status": "completed", "labels": ["ubuntu-latest"]},
                    {"name": "work", "status": "in_progress", "labels": ["ubuntu-latest"],
                     "runner_name": "GitHub Actions 1000123", "started_at": ago(47),
                     "html_url": "https://github.com/x/y/actions/runs/101/job/1",
                     "steps": [{"name": "Set up job", "status": "completed"}, doing]}],
            "102": [{"name": "work", "status": "in_progress", "labels": ["night-vm-muse"],
                     "runner_name": "night-vm-muse-b2", "started_at": ago(19),
                     "html_url": "https://github.com/x/y/actions/runs/102/job/3",
                     "steps": [{"name": "Set up job", "status": "completed"}]}],
            "103": [{"name": "rust (test engine)", "status": "in_progress",
                     "labels": ["ubuntu-latest"], "runner_name": "GitHub Actions 1000124",
                     "started_at": ago(3), "steps": [{"name": "cargo test",
                                                      "status": "in_progress"}]},
                    {"name": "web (unit 2/4)", "status": "queued", "labels": ["ubuntu-latest"],
                     "created_at": ago(4)},
                    {"name": "web (unit 3/4)", "status": "queued", "labels": ["ubuntu-latest"],
                     "created_at": ago(3)}],
            "104": [{"name": "plan", "status": "queued", "labels": [dashboard.LOCAL_LABEL],
                     "created_at": ago(90)}],
        })

    def section(self) -> str:
        return dashboard.runners_section(self.ctx, self.ctx.store.load())

    def test_each_busy_runner_says_what_it_does_and_what_for(self):
        body = self.section()
        self.assertTrue(body.startswith(dashboard.RUNNERS_START + "\n## 🖥️ Every runner"))
        self.assertTrue(body.endswith(dashboard.RUNNERS_END))
        self.assertIn("🟢 **3 runners busy**: 2 on GitHub's (of the 20 the organization may hold), "
                      "1 on the Muse box · ⏳ **3 jobs queued** for a runner · read at 22:00", body)
        # GitHub's runners first, the longest-running first; the night bot's runs name the item
        # they work on, found by the run's id or the pending claim's reference in its title.
        hosted = ("| `GitHub Actions 1000123` (Linux) | bot-night · work | Build, check and review "
                  "| building #37 on `claude-1` | [47m](https://github.com/x/y/actions/runs/101/job/1) |")
        ci = ("| `GitHub Actions 1000124` (Linux) | ci · rust (test engine) | cargo test "
              "| #1966 Patch: a Sets tab | 3m |")
        muse = ("| `night-vm-muse-b2` (Muse box) | bot-night · work | after Set up job "
                "| revising #49 on `muse` | [19m](https://github.com/x/y/actions/runs/102/job/3) |")
        for row in (hosted, ci, muse):
            self.assertIn(row, body)
        self.assertLess(body.index(hosted), body.index(ci))
        self.assertLess(body.index(ci), body.index(muse))
        # The queued jobs by label, the most first, each with its longest wait.
        self.assertIn("⏳ **Queued for a runner:** `ubuntu-latest` 2 (ci · web (unit 2/4), the "
                      "longest 4m); `jackioh-selfplay-local` 1 (learn · plan, the longest 1h 30m).",
                      body)
        # The self-hosted labels no job holds; Muse's runner holds one.
        self.assertIn("💤 **No job on any of its runners:** `night-vm-agy`, `night-vm-gpt`, "
                      "`night-vm-devin`, `jackioh-selfplay-local` (idle", body)

    def test_a_long_list_of_busy_runners_is_cut(self):
        self.gh.jobs["103"] = [{"name": f"shard {n}", "status": "in_progress",
                                "labels": ["ubuntu-latest"], "runner_name": f"GitHub Actions {n}"}
                               for n in range(dashboard.RUNNER_ROWS + 2)]
        body = self.section()
        self.assertIn(f"🟢 **{dashboard.RUNNER_ROWS + 4} runners busy**", body)
        self.assertEqual(body.count(" (Linux) | "), dashboard.RUNNER_ROWS)
        self.assertIn("…and 4 more.", body)
        # With no room for a table, the runners are counted and the rest still says what waits.
        tight = dashboard.runners_section(self.ctx, self.ctx.store.load(), room=0)
        self.assertNotIn("| Runner |", tight)
        self.assertIn(f"{dashboard.RUNNER_ROWS + 4} not listed: no room.", tight)
        self.assertIn("⏳ **Queued for a runner:**", tight)
        self.assertTrue(tight.endswith(dashboard.RUNNERS_END))

    def test_what_each_runner_is_for_is_a_comment_kept_from_the_pool(self):
        """The explanation changes only with the pool or the workflows, so it is a comment on
        the status issue, written once and again only when its text changes; the section links
        it."""
        self.gh.add_issue(148, dashboard.TITLE, "x", user=OPERATOR)
        state = self.ctx.store.load()
        self.assertIn("What each runner and workflow is for is in a comment below.",
                      dashboard.runners_section(self.ctx, state))
        dashboard.explain(self.ctx, 148, state)
        [comment] = self.gh.comments[148]
        text = comment["body"]
        self.assertTrue(text.startswith("## 🖥️ What each runner is for"))
        self.assertIn(dashboard.EXPLAINED, text)
        self.assertIn("| `ubuntu-latest` | GitHub's Linux", text)
        self.assertIn("| `night-vm-devin` | the bot's AWS boxes, 6 lanes | `bot-night`'s `work` for "
                      "`devin`: the `devin` CLI with `swe-2-max`, as the Linux user "
                      "`agent-devin`, which holds that login alone |", text)
        self.assertIn("| `night-vm-gpt` | the bot's AWS boxes, 1 lane | ", text)
        self.assertIn(f"| `{dashboard.LOCAL_LABEL}` | the owner's PC", text)
        self.assertIn("| `voice` | renders a branch's voice lines", text)
        # None of the statistics' outcomes reads it as a run's.
        from harness import stats
        self.assertEqual(stats.classify(text), "other")
        self.assertIn("What each runner and workflow is for: [its comment](https://github.com/"
                      f"jgoetzmann/JackiOh/issues/148#issuecomment-{comment['id']}).",
                      dashboard.runners_section(self.ctx, state))
        # Unchanged, it costs nothing; changed, it is rewritten in place; deleted, written again.
        with mock.patch.object(self.gh, "update_comment", side_effect=AssertionError("written")), \
                mock.patch.object(self.gh, "create_comment", side_effect=AssertionError("posted")):
            dashboard.explain(self.ctx, 148, self.ctx.store.load())
        with mock.patch.object(dashboard, "BOXES_NOTE", "Another note."):
            dashboard.explain(self.ctx, 148, self.ctx.store.load())
        self.assertEqual(len(self.gh.comments[148]), 1)
        self.assertIn("Another note.", self.gh.comments[148][0]["body"])
        self.gh.comments[148].clear()
        dashboard.explain(self.ctx, 148, self.ctx.store.load())
        self.assertEqual(len(self.gh.comments[148]), 1)
        self.assertNotIn("Another note.", self.gh.comments[148][0]["body"])
        # A rewrite of the issue writes it, and links it in the same rewrite.
        self.ctx.store.update(lambda s: s.update(dashboard={"issue": 148},
                                                 runners_explained=None))
        self.gh.comments[148].clear()
        dashboard.update(self.ctx)
        [comment] = self.gh.comments[148]
        self.assertIn(f"#issuecomment-{comment['id']}", self.gh.threads[148]["body"])

    def test_every_workflow_says_what_it_is_for(self):
        on_disk = {path.name for path in (ROOT / ".github" / "workflows").glob("*.yml")}
        self.assertEqual(set(dashboard.WORKFLOWS), on_disk)

    def test_where_a_job_runs(self):
        for name, label, where in (("GitHub Actions 7", "ubuntu-latest", "Linux"),
                                   ("GitHub Actions 8", "macos-15", "macOS"),
                                   ("GitHub Actions 9", "windows-2022", "Windows"),
                                   ("night-vm-devin-3", "night-vm-devin", "night box"),
                                   ("night-vm-muse-b2", "night-vm-muse", "Muse box"),
                                   ("pc-devin-1026021400", "night-vm-devin", "owner's PC"),
                                   ("", dashboard.LOCAL_LABEL, "owner's PC"),
                                   ("someone-elses", "gpu", "unknown box")):
            self.assertEqual(dashboard._where({"runner_name": name, "labels": [label]})[1], where)

    def test_nothing_running_and_runs_that_cannot_be_read(self):
        self.gh.runs.clear()
        body = self.section()
        self.assertIn("No runner holds a job right now.", body)
        self.assertNotIn("Queued for a runner", body)
        with mock.patch.object(self.gh, "list_repo_runs", side_effect=GitHubError("down", 502)):
            body = self.section()
        self.assertIn("The runs going could not be read at 22:00.", body)
        self.assertIn("What each runner and workflow is for", body)  # its link stands regardless
        self.assertTrue(body.endswith(dashboard.RUNNERS_END))

    def test_the_issue_carries_it_after_running_now_and_before_the_queue(self):
        body = dashboard.render(self.ctx)
        self.assertLess(body.index("## ⚡ Running now"), body.index(dashboard.RUNNERS_START))
        self.assertLess(body.index(dashboard.RUNNERS_END), body.index("## 📋 Queue"))
        self.assertIn("`bot-status` rewrites this issue every 5 minutes, and its runners every 2",
                      body)

    def test_a_redraw_changes_the_runners_section_alone(self):
        self.gh.add_issue(148, dashboard.TITLE, "x", user=OPERATOR)
        self.ctx.store.update(lambda s: s.update(dashboard={"issue": 148}))
        self.assertIn("has no runners section", dashboard.refresh_runners(self.ctx))
        self.assertEqual(self.gh.threads[148]["body"], "x")  # the next rewrite draws it
        before = (f"top\n{dashboard.RUNNERS_START}\nold runners\n{dashboard.RUNNERS_END}"
                  "\nbottom")
        self.gh.threads[148]["body"] = before
        self.assertEqual(dashboard.refresh_runners(self.ctx), "redrew #148's runners")
        body = self.gh.threads[148]["body"]
        self.assertTrue(body.startswith("top\n" + dashboard.RUNNERS_START))
        self.assertTrue(body.endswith(dashboard.RUNNERS_END + "\nbottom"))
        self.assertNotIn("old runners", body)
        self.assertIn("building #37 on `claude-1`", body)
        # Near GitHub's limit, fewer runners are listed; past it, the redraw waits.
        self.gh.threads[148]["body"] = before
        full = len(before) + len(self.section()) - len(dashboard.RUNNERS_START + "\nold runners\n"
                                                        + dashboard.RUNNERS_END)
        with mock.patch.object(dashboard, "MAX_BODY", full - 1):
            self.assertEqual(dashboard.refresh_runners(self.ctx), "redrew #148's runners")
        self.assertLessEqual(len(self.gh.threads[148]["body"]), full - 1)
        self.assertIn("…and 2 more.", self.gh.threads[148]["body"])
        self.gh.threads[148]["body"] = before
        with mock.patch.object(dashboard, "MAX_BODY", len(before)):
            self.assertIn("would pass GitHub's limit", dashboard.refresh_runners(self.ctx))
        self.assertEqual(self.gh.threads[148]["body"], before)

    def test_the_loop_redraws_the_runners_between_rewrites(self):
        """`--every 300 --runners-every 120`, as bot-status runs it: the rewrites keep their five
        minutes, and between them the runners are redrawn every two; a failed redraw is only a
        warning."""
        clock, ticks = [0.0], []

        def update(ctx, extra=(), **_):
            ticks.append(("rewrite", clock[0]))
            return "rewrote #148"

        def refresh(ctx):
            ticks.append(("runners", clock[0]))
            if len(ticks) == 3:
                raise RuntimeError("network down")
            return "redrew #148's runners"

        out = io.StringIO()
        with mock.patch.object(dashboard, "update", update), \
                mock.patch.object(dashboard, "refresh_runners", refresh), offline(), \
                mock.patch.object(main_mod.time, "monotonic", lambda: clock[0]), \
                mock.patch.object(main_mod.time, "sleep",
                                  lambda s: clock.__setitem__(0, clock[0] + s)), \
                contextlib.redirect_stdout(out):
            cmd_dashboard(make_config(), argparse.Namespace(every=300, runners_every=120,
                                                            for_seconds=600))
        self.assertEqual(ticks, [("rewrite", 0.0), ("runners", 120.0), ("runners", 240.0),
                                 ("rewrite", 300.0), ("runners", 420.0), ("runners", 540.0),
                                 ("rewrite", 600.0)])
        self.assertIn("::warning::the runners were not redrawn: network down", out.getvalue())
        self.assertIn("runners: redrew #148's runners", out.getvalue())


if __name__ == "__main__":
    unittest.main()
