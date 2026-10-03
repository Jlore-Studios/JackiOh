"""The pinned status issue (`dashboard.py`): opened and pinned once, rewritten every sweep, adopted
again when the state file loses it, and drawn from the same facts as `/harness status`."""

from __future__ import annotations

import argparse
import contextlib
import io
import unittest
from datetime import timedelta

from harness import dashboard
from harness.__main__ import cmd_dashboard
from harness.clock import iso
from harness.config import LABEL_BUILD, LABEL_PR, LABEL_REVISE, LABEL_WORKING
from harness.errors import GitHubError
from harness.state import item as state_item

from tests.fakes import OPERATOR, STRANGER, FakeGitHub
from tests.support import NIGHT, make_config, make_ctx


def opened(gh: FakeGitHub) -> list[int]:
    return [n for n, t in gh.threads.items() if t["title"] == dashboard.TITLE]


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
        self.assertIn("| #7 | build | easy | high |", self.gh.threads[number]["body"])

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
                               "html_url": "https://github.com/x/y/actions/runs/200"}
        body = dashboard.render(self.ctx)
        # What each lane is doing, with a link to the run doing it.
        runs = "https://github.com/jgoetzmann/JackiOh/actions/runs"
        self.assertIn("| Item | Title | Doing | Subscription | For | Run |", body)
        self.assertIn(f"| #37 | An issue | building | `claude-1` `opus` (GitHub) | 47m "
                      f"| [run 101]({runs}/101) |", body)
        self.assertIn(f"| #49 | An issue | revising | `muse` `muse-spark-1.3-contributor` "
                      f"(machine) | 2h 03m | [run 102]({runs}/102) |", body)
        self.assertIn("by `bot-status`, which rewrites this issue every ten minutes", body)
        # A timeline of the runs going now, by subscription and where each runs.
        self.assertIn("```mermaid\ngantt", body)
        self.assertIn("    section claude-1 (GitHub)\n    item 37 build :active, ", body)
        self.assertIn("    section muse (machine)\n    item 49 revise :active, ", body)
        self.assertEqual(body.count("section "), 2)
        # The lanes in use.
        self.assertIn('pie showData title Lanes (7, at most 3 on the machine)', body)
        self.assertIn('    "On the machine" : 1\n    "On GitHub\'s runners" : 1\n    "Free" : 5',
                      body)
        # Each subscription, with its usage as a bar.
        self.assertIn("| `claude-1` |", body)
        self.assertIn("🟢 #37", body)
        self.assertIn("▰▰▰▱▱▱▱▱▱▱ 30%", body)
        # The queue and the last runs.
        self.assertIn("| #9 | revise | hard | — | Make it so |", body)
        self.assertIn("| schedule | ✅ | [812](https://github.com/x/y/actions/runs/200) |", body)
        # The full status, folded.
        self.assertIn("<details><summary>The full status</summary>", body)
        self.assertIn("**Night bot status**", body)

    def test_nothing_running(self):
        body = dashboard.render(self.ctx)
        self.assertIn("Nothing is running right now.", body)
        self.assertIn('    "Free" : 7', body)
        self.assertIn("Nothing is queued.", body)

    def test_the_loop_rewrites_it_every_ten_minutes_and_outlives_a_bad_tick(self):
        """`harness dashboard --every 600 --for 1200`, as bot-status runs it (with longer
        figures): a rewrite now and every ten minutes until the time is up, and a tick that fails
        is only a warning."""
        from unittest import mock
        import harness.__main__ as main_mod
        clock = [1000.0]
        ticks = []

        def update(ctx):
            ticks.append(clock[0])
            if len(ticks) == 2:
                raise RuntimeError("network down")
            return "rewrote #148"

        def sleep(seconds):
            clock[0] += seconds

        out = io.StringIO()
        with mock.patch.object(dashboard, "update", update), \
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

        def update(ctx):
            order.append(("rewrite", clock[0]))
            return "rewrote #148"

        out = io.StringIO()
        with mock.patch.object(main_mod.sweep_mod, "sweep", sweep), \
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
        with mock.patch.object(dashboard, "update", broken), contextlib.redirect_stdout(out):
            self.assertEqual(cmd_dashboard(cfg, argparse.Namespace()), 0)
        self.assertIn("::warning::the status issue was not updated", out.getvalue())


if __name__ == "__main__":
    unittest.main()
