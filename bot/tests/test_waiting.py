"""What the status issue says the queue waits for and what each subscription can do now
(`waiting.py`, #148): the planner's own rules on the lanes held now, so an item never "starts
next" when the planner would not start it and a subscription is never free when it gets no work."""

from __future__ import annotations

import unittest
from datetime import timedelta

from harness import dashboard, waiting
from harness.clock import iso
from harness.config import LABEL_BUILD, LABEL_PR, LABEL_REVISE, LABEL_WORKING
from harness.state import item as state_item

from tests.fakes import FakeGitHub
from tests.support import NIGHT, make_config, make_ctx, with_lanes
from tests.test_cross import ALL


def claude_refused(state, pool, first="claude-2"):
    """Every Claude account refused a call: `first` until 22:05, the others later."""
    claude = [p.id for p in pool.ordered() if p.cli == "claude"]
    claude.sort(key=lambda pid: pid != first)
    for i, provider_id in enumerate(claude):
        state["providers"][provider_id] = {"refused_until": iso(NIGHT + timedelta(minutes=5 + 10 * i))}


class QueueTests(unittest.TestCase):
    def setUp(self):
        self.gh = FakeGitHub()
        self.ctx = make_ctx(self.gh, at=NIGHT, cfg=make_config(env=ALL, machine=("muse",)))

    def explain(self, live=None):
        state = self.ctx.store.load()
        return waiting.explain(self.ctx, state, live or {}, issues=self.gh.list_issues())

    def row(self, explained, number):
        return next(r for r in explained.rows if r.number == number)

    def test_a_hard_item_waits_for_a_strong_model_and_says_which_frees_first(self):
        self.gh.add_issue(41, "Hard work", labels=(LABEL_BUILD, "difficulty:hard"))
        pool = self.ctx.cfg.pool

        def seed(state):
            state_item(state, 41).update(planned_at=iso(NIGHT), planned_tier="strong")
            claude_refused(state, pool)
        self.ctx.store.update(seed, "seed")
        explained = self.explain()
        row = self.row(explained, 41)
        self.assertEqual((row.step, row.difficulty, row.tier), ("build", "hard", "strong"))
        self.assertIsNone(row.starts)
        self.assertEqual(row.waiting, "⏳ **a strong model (Opus)**: first free `claude-2` at 22:05")
        # Muse may start work, but nothing queued is for it: idle, never "free".
        self.assertEqual(explained.now["muse"].kind, "idle")
        self.assertEqual(explained.now["claude-2"].kind, "capped")
        body = dashboard.render(self.ctx)
        self.assertIn("| #41 | Hard work | build | 🔴 hard → strong | ⏳ **a strong model (Opus)**: "
                      "first free `claude-2` at 22:05 | — |", body)
        self.assertIn("1 waits for a strong model", body)
        self.assertIn("🔋 **1 of 11 subscriptions can start work now**: `muse`", body)
        self.assertIn('<b>muse</b><br/>⚪ idle<br/>nothing queued is for it', body)

    def test_a_revision_that_answers_a_strong_audit_waits_for_strong(self):
        self.gh.add_pull(9, "bot/issue-3", "Fix it", labels=(LABEL_PR, LABEL_REVISE))
        pool = self.ctx.cfg.pool

        def seed(state):
            state_item(state, 9).update(audit_tier="strong")
            claude_refused(state, pool)
        self.ctx.store.update(seed, "seed")
        row = self.row(self.explain(), 9)
        self.assertEqual((row.step, row.difficulty, row.tier), ("revise", "medium", "strong"))
        self.assertEqual(row.tier_why, "it answers a strong reviewer's audit")
        self.assertIsNone(row.starts)
        # Without the audit, Muse (medium) takes it at the next run, and says so.
        self.ctx.store.update(lambda s: state_item(s, 9).update(audit_tier=""), "no audit")
        explained = self.explain()
        row = self.row(explained, 9)
        self.assertEqual(row.starts.provider.id, "muse")
        self.assertEqual((explained.now["muse"].kind, explained.now["muse"].text),
                         ("next", "▶️ takes #9 next"))
        self.assertIn("▶️ **starts at the next run** on `muse` (`muse-spark-1.3-contributor`, "
                      "medium)", dashboard.render(self.ctx))

    def test_a_part_waits_for_the_part_before_it(self):
        self.gh.add_issue(1, "Patch v0.9.X (part 1 of 2): the base")
        self.gh.add_issue(2, "Patch v0.9.X (part 2 of 2): the rest", labels=(LABEL_BUILD,))
        row = self.row(self.explain(), 2)
        self.assertTrue(row.held)
        self.assertEqual(row.waiting, "waits for #1 to close")
        self.assertIn("| #2 | Patch v0.9.X (part 2 of 2): the rest | — | — | ⏸️ held back: waits "
                      "for #1 to close | — |", dashboard.render(self.ctx))

    def test_an_unrated_build_needs_a_medium_planner_first(self):
        self.gh.add_issue(5, "New thing", labels=(LABEL_BUILD,))
        self.ctx.store.update(lambda s: claude_refused(s, self.ctx.cfg.pool), "seed")
        row = self.row(self.explain(), 5)
        self.assertEqual((row.step, row.difficulty, row.tier, row.tier_why),
                         ("plan", "unrated", "medium", "rates and plans it first"))
        self.assertEqual(row.starts.provider.id, "muse")  # Muse plans it in its own run

    def test_a_subscription_with_no_lane_where_it_runs_is_full_not_free(self):
        cfg = with_lanes(make_config(env=ALL, machine=("muse", "agy")), 11, 1)
        ctx = make_ctx(self.gh, at=NIGHT, cfg=cfg)
        self.gh.add_issue(50, labels=(LABEL_WORKING,))
        self.gh.runs["300"] = {"status": "in_progress"}
        ctx.store.update(lambda s: state_item(s, 50).update(provider="muse", kind="build",
                                                            run_id="300"), "seed")
        state = ctx.store.load()
        explained = waiting.explain(ctx, state, {50: "muse"}, issues=self.gh.list_issues())
        self.assertEqual(explained.now["muse"].kind, "busy")
        self.assertEqual(explained.now["agy"].kind, "full")
        self.assertIn("the night box is full", explained.now["agy"].text)

    def test_halted_nothing_starts(self):
        self.gh.add_issue(7, labels=(LABEL_BUILD, "difficulty:easy"))
        self.ctx.store.update(lambda s: s.update(halted=True), "halt")
        body = dashboard.render(self.ctx)
        self.assertIn("> 🛑 **Halted**: halted by /harness halt.", body)
        explained = waiting.explain(self.ctx, self.ctx.store.load(), {},
                                    issues=self.gh.list_issues(), halted="halted by /harness halt")
        row = self.row(explained, 7)
        self.assertIsNone(row.starts)
        self.assertEqual(row.waiting, "halted by /harness halt")

    def test_the_difficulty_legend_names_each_tiers_models(self):
        legend = dashboard.difficulty_legend(self.ctx.cfg.pool)
        self.assertIn("🔴 **hard**: a strong model only (Opus)", legend)
        self.assertIn("a medium or strong model (muse, agy, Sonnet, then Opus)", legend)
        self.assertIn("any model, the weak ones first (gpt, devin)", legend)


if __name__ == "__main__":
    unittest.main()
