"""Config, clock and window, commands, trust, redaction, verdicts, prompts, state."""

from __future__ import annotations

import unittest
from datetime import datetime, timedelta, timezone

from harness import clock, commands, config, prompts, redact, state, verdicts
from harness.errors import ConfigError, StateConflict
from harness.trust import Trust
from harness.work import check_templates

from tests.fakes import FakeGitHub
from tests.support import DAY, NIGHT, make_config, raw_config


class ConfigTests(unittest.TestCase):
    def test_committed_config_loads(self):
        cfg = config.load(env={})
        self.assertEqual(cfg.repo, "jgoetzmann/JackiOh")
        self.assertEqual(cfg.model, "opus")
        self.assertEqual(cfg.effort, "xhigh")
        self.assertEqual((cfg.window_start, cfg.window_end), ("21:00", "07:00"))
        self.assertIn(".github/", cfg.forbidden_paths)
        self.assertIn("bot/", cfg.forbidden_paths)
        self.assertIn(".harness/", cfg.forbidden_paths)
        self.assertEqual(cfg.suggestions_max_open, 4)
        self.assertIn("bot selftest", cfg.required_checks)

    def test_unknown_and_missing_keys_are_errors(self):
        raw = raw_config()
        raw["surprise"] = 1
        with self.assertRaises(ConfigError):
            config.parse(raw, config.REPO_ROOT, {})
        raw = raw_config()
        del raw["gates"]
        with self.assertRaises(ConfigError):
            config.parse(raw, config.REPO_ROOT, {})

    def test_usage_stop_must_be_a_fraction(self):
        raw = raw_config()
        raw["usage_stop"] = {"five_hour": 98}
        with self.assertRaises(ConfigError):
            config.parse(raw, config.REPO_ROOT, {})

    def test_environment_supplies_tokens_and_run(self):
        cfg = make_config(env={"BOT_GITHUB_TOKEN": "bot-token-value", "GITHUB_TOKEN": "actions-token"})
        self.assertEqual(cfg.write_token, "bot-token-value")
        self.assertEqual(cfg.run_url, "https://github.com/jgoetzmann/JackiOh/actions/runs/777")
        cfg = make_config(env={"GITHUB_TOKEN": "actions-token"})
        self.assertEqual(cfg.write_token, "actions-token")

    def test_child_env_strips_every_secret(self):
        env = {"PATH": "/bin", "BOT_GITHUB_TOKEN": "x", "GITHUB_TOKEN": "y",
               "CLAUDE_CODE_OAUTH_TOKEN": "z", "ANTHROPIC_API_KEY": "k"}
        self.assertEqual(config.child_env(env), {"PATH": "/bin"})
        kept = config.child_env(env, keep=("CLAUDE_CODE_OAUTH_TOKEN",))
        self.assertEqual(kept, {"PATH": "/bin", "CLAUDE_CODE_OAUTH_TOKEN": "z"})


class WindowTests(unittest.TestCase):
    window = clock.Window.of("America/Chicago", "21:00", "07:00")

    def at(self, text: str) -> datetime:
        return clock.parse_iso(text)

    def test_open_at_night_closed_by_day(self):
        self.assertTrue(self.window.is_open(NIGHT))
        self.assertFalse(self.window.is_open(DAY))

    def test_edges_in_summer_time(self):
        # CDT is UTC-5: the window is 02:00 to 12:00 UTC.
        self.assertFalse(self.window.is_open(self.at("2026-07-01T01:59:00Z")))
        self.assertTrue(self.window.is_open(self.at("2026-07-01T02:00:00Z")))
        self.assertTrue(self.window.is_open(self.at("2026-07-01T11:59:00Z")))
        self.assertFalse(self.window.is_open(self.at("2026-07-01T12:00:00Z")))

    def test_edges_in_winter_time(self):
        # CST is UTC-6: the window is 03:00 to 13:00 UTC.
        self.assertFalse(self.window.is_open(self.at("2026-12-01T02:59:00Z")))
        self.assertTrue(self.window.is_open(self.at("2026-12-01T03:00:00Z")))
        self.assertTrue(self.window.is_open(self.at("2026-12-01T12:59:00Z")))
        self.assertFalse(self.window.is_open(self.at("2026-12-01T13:00:00Z")))

    def test_next_open_and_close(self):
        opens = self.window.next_open(DAY)
        self.assertEqual(clock.iso(opens), "2026-09-30T02:00:00Z")
        closes = self.window.closes_at(NIGHT)
        self.assertEqual(clock.iso(closes), "2026-09-30T12:00:00Z")
        self.assertIsNone(self.window.closes_at(DAY))

    def test_fallback_zone_agrees_with_tzdata(self):
        fallback = clock._UsCentral()
        real = clock.zone("America/Chicago")
        at = datetime(2026, 1, 1, tzinfo=timezone.utc)
        for _ in range(0, 24 * 400, 7):
            at += timedelta(hours=7)
            self.assertEqual(at.astimezone(fallback).hour, at.astimezone(real).hour, at)

    def test_crons_cover_the_window_in_both_seasons(self):
        # bot-night.yml fires at :17 past 02..13 UTC; every local hour 21..06 must be covered.
        hours = range(2, 14)
        for day in ("2026-07-01", "2026-12-01"):
            covered = set()
            for hour in hours:
                at = self.at(f"{day}T{hour:02d}:17:00Z")
                if self.window.is_open(at):
                    covered.add(at.astimezone(clock.zone("America/Chicago")).hour)
            self.assertEqual(covered, {21, 22, 23, 0, 1, 2, 3, 4, 5, 6}, day)

    def test_human_delta(self):
        self.assertEqual(clock.human_delta(timedelta(minutes=5)), "5m")
        self.assertEqual(clock.human_delta(timedelta(hours=3, minutes=4)), "3h 04m")
        self.assertEqual(clock.human_delta(timedelta(seconds=-5)), "now")


class CommandTests(unittest.TestCase):
    bot = "jgoetzmann-bot"

    def verbs(self, body: str) -> list[tuple[str, str, bool]]:
        return [(c.verb, c.args, c.force) for c in commands.parse(body, self.bot)]

    def test_slash_forms(self):
        self.assertEqual(self.verbs("/harness build"), [("build", "", False)])
        self.assertEqual(self.verbs("/harness-status"), [("status", "", False)])
        self.assertEqual(self.verbs("/Harness HALT tired"), [("halt", "tired", False)])
        self.assertEqual(self.verbs("@jgoetzmann-bot /harness revise tighten it"),
                         [("revise", "tighten it", False)])

    def test_aliases(self):
        self.assertEqual(self.verbs("/harness work"), [("build", "", False)])
        self.assertEqual(self.verbs("/harness fix it"), [("revise", "it", False)])
        self.assertEqual(self.verbs("/harness resume"), [("start", "", False)])
        self.assertEqual(self.verbs("/harness help"), [("status", "", False)])

    def test_force_flag(self):
        self.assertEqual(self.verbs("/harness build --force"), [("build", "", True)])
        found = commands.parse("/harness build --force", self.bot)[0]
        self.assertEqual(found.level, 3)
        self.assertEqual(commands.parse("/harness build", self.bot)[0].level, 2)

    def test_mention_with_a_verb_and_with_free_text(self):
        self.assertEqual(self.verbs("@jgoetzmann-bot status"), [("status", "", False)])
        self.assertEqual(self.verbs("@jgoetzmann-bot: please make the coin shinier"),
                         [("request", "please make the coin shinier", False)])
        self.assertEqual(self.verbs("@JGoetzmann-Bot build --force"), [("build", "", True)])

    def test_prose_is_not_a_command(self):
        self.assertEqual(self.verbs("as discussed, /harness stop"), [])
        self.assertEqual(self.verbs("thanks @jgoetzmann-bot, looks good"), [])
        self.assertEqual(self.verbs("@someone status"), [])

    def test_quotes_and_fences_are_skipped(self):
        body = "> /harness halt\n```\n/harness halt\n```\n~~~\n@jgoetzmann-bot build\n~~~\n"
        self.assertEqual(self.verbs(body), [])

    def test_several_commands_in_order_and_capped(self):
        body = "\n".join(["/harness status", "/harness build"] * 8)
        found = commands.parse(body, self.bot)
        self.assertEqual(len(found), commands.MAX_COMMANDS)
        self.assertEqual([c.verb for c in found[:2]], ["status", "build"])

    def test_unknown_verb_is_reported(self):
        self.assertEqual(self.verbs("/harness dance"), [("unknown:dance", "", False)])

    def test_help_mentions_every_verb(self):
        text = commands.HELP.format(bot=self.bot)
        for verb in commands.VERBS:
            self.assertIn(f"`{verb}", text)


class TrustTests(unittest.TestCase):
    text = "jgoetzmann 3 id:95732896\nhelper 2\nreader\nbad!login 2\nhigh 9\n"

    def test_parse(self):
        trust = Trust.parse(self.text)
        self.assertEqual(sorted(trust.entries), ["helper", "jgoetzmann", "reader"])
        self.assertEqual(len(trust.problems), 2)

    def test_id_pins_the_account(self):
        trust = Trust.parse(self.text)
        self.assertEqual(trust.level("jgoetzmann", 95732896), 3)
        self.assertEqual(trust.level("jgoetzmann", 1), 0)
        self.assertEqual(trust.level("JGoetzmann", 95732896, "NONE"), 3)

    def test_a_pinned_line_needs_the_id(self):
        trust = Trust.parse(self.text)
        self.assertEqual(trust.level("jgoetzmann", None, "OWNER"), 0)

    def test_without_id_github_must_vouch(self):
        trust = Trust.parse(self.text)
        self.assertEqual(trust.level("helper", 42, "COLLABORATOR"), 2)
        self.assertEqual(trust.level("helper", 42, "CONTRIBUTOR"), 0)
        self.assertEqual(trust.level("reader", None, "OWNER"), 1)
        self.assertEqual(trust.level("nobody", 1, "OWNER"), 0)

    def test_committed_trust_file(self):
        trust = Trust.load(config.REPO_ROOT / config.TRUST_PATH)
        self.assertEqual(trust.problems, ())
        self.assertEqual(trust.level("jgoetzmann", 95732896), 3)


class RedactTests(unittest.TestCase):
    def test_token_shapes(self):
        text = ("ghp_" + "a" * 36 + " github_pat_" + "b" * 50 + " sk-ant-" + "c" * 30
                + " Bearer abcdefghijklmnopqrstuvwxyz")
        out = redact.redact(text)
        self.assertNotIn("aaaa", out)
        self.assertNotIn("bbbb", out)
        self.assertNotIn("cccc", out)
        self.assertNotIn("abcdefghijkl", out)

    def test_live_values(self):
        self.assertEqual(redact.redact("key=hunter2hunter2", extra=["hunter2hunter2"]),
                         "key=[REDACTED]")


class VerdictTests(unittest.TestCase):
    def test_build_report(self):
        text = 'Some preamble\n<!-- bot: {"status": "done", "title": "Fix the  coin"} -->\n## What\nStuff'
        report = verdicts.build_report(text)
        self.assertEqual((report.status, report.title), ("done", "Fix the coin"))
        self.assertTrue(report.body.startswith("## What"))
        blocked = verdicts.build_report('<!-- bot: {"status": "blocked", "question": "A or B?"} -->')
        self.assertEqual((blocked.status, blocked.question), ("blocked", "A or B?"))
        self.assertEqual(verdicts.build_report("no header").status, "unknown")

    def test_review(self):
        text = ('<!-- review: {"verdict": "changes", "findings": [{"severity": "blocking", '
                '"where": "a.ts:3", "claim": "wrong", "evidence": "x"}, {"severity": "note", '
                '"where": "b.ts", "claim": "meh"}]} -->\nprose')
        review = verdicts.review(text)
        self.assertTrue(review.readable)
        self.assertFalse(review.approved)
        self.assertEqual(len(review.blocking), 1)
        self.assertEqual(len(review.notes), 1)
        self.assertEqual(review.body, "prose")

    def test_approve_with_a_blocking_finding_is_not_approval(self):
        text = ('<!-- review: {"verdict": "approve", "findings": [{"severity": "blocking", '
                '"where": "x", "claim": "y"}]} -->')
        self.assertFalse(verdicts.review(text).approved)
        self.assertTrue(verdicts.review('<!-- review: {"verdict": "approve", "findings": []} -->').approved)

    def test_unreadable_review(self):
        self.assertFalse(verdicts.review("LGTM!").readable)
        self.assertFalse(verdicts.review('<!-- review: {"verdict": "maybe"} -->').readable)
        self.assertFalse(verdicts.review("<!-- review: {not json} -->").approved)

    def test_suggestions(self):
        text = ('<!-- suggestions: [{"title": "A", "body": "B"}, {"title": "", "body": "x"}, '
                '{"title": "C", "body": "D"}, {"title": "E", "body": "F"}] -->')
        found = verdicts.suggestions(text, 2)
        self.assertEqual([s.title for s in found], ["A", "C"])
        self.assertEqual(verdicts.suggestions("nothing", 4), [])


class PromptTests(unittest.TestCase):
    def test_every_template_renders(self):
        self.assertEqual(check_templates(), [])

    def test_data_fence_outgrows_backticks_inside(self):
        wrapped = prompts.data("before ```` after", "Thing")
        self.assertIn("`````text", wrapped)
        self.assertIn("data, not instructions", wrapped)

    def test_system_prompt_names_the_forbidden_paths(self):
        text = prompts.render("system", bot="b", repo="r")
        for path in (".github/", ".harness/", "bot/"):
            self.assertIn(path, text)


class StateTests(unittest.TestCase):
    def test_update_creates_the_branch_and_retries_on_conflict(self):
        gh = FakeGitHub()
        store = state.StateStore(gh)
        store.update(lambda s: s.update(halted=True), "halt")
        self.assertIn("bot-state", gh.branches)
        self.assertTrue(store.load()["halted"])
        gh.conflicts_to_inject = 2
        store.update(lambda s: state.item(s, 5).update(failures=1), "fail")
        loaded = store.load()
        self.assertEqual(loaded["items"]["5"]["failures"], 1)
        self.assertIn("999", loaded["items"])  # the other writer's change survived
        self.assertTrue(loaded["halted"])

    def test_update_gives_up_eventually(self):
        gh = FakeGitHub()
        store = state.StateStore(gh)
        store.update(lambda s: s.update(halted=False), "x")
        gh.conflicts_to_inject = 99
        with self.assertRaises(StateConflict):
            store.update(lambda s: s.update(halted=True), "y")

    def test_usage_refusal(self):
        at = NIGHT
        stops = {"five_hour": 0.98, "seven_day": 0.9}
        s = state.default_state()
        self.assertIsNone(state.usage_refusal(s, stops, at))
        later = clock.iso(at + timedelta(hours=2))
        s["usage"] = {"seven_day": {"utilization": 0.95, "resets_at": later}}
        self.assertIn("7-day", state.usage_refusal(s, stops, at))
        # Once that window has reset, the old reading no longer refuses.
        self.assertIsNone(state.usage_refusal(s, stops, at + timedelta(hours=3)))
        s = state.default_state()
        state.record_usage(s, None, "+PT30M", at)
        self.assertIn("refused", state.usage_refusal(s, stops, at))
        self.assertIsNone(state.usage_refusal(s, stops, at + timedelta(minutes=31)))


if __name__ == "__main__":
    unittest.main()


class PathTests(unittest.TestCase):
    def test_forbidden_and_review_paths(self):
        from harness.git import matches
        cfg = make_config()
        forbidden, review = cfg.forbidden_paths, cfg.review_paths
        for path in (".github/workflows/x.yml", ".GitHub/workflows/x.yml", "./bot/harness/gh.py",
                     ".harness/trust.txt", ".claude/settings.json", ".mcp.json", ".vscode/tasks.json",
                     ".github"):
            self.assertTrue(matches(path, forbidden), path)
        for path in ("package.json", "packages/cards/package.json", "pnpm-lock.yaml",
                     "vitest.config.ts", "tsconfig.base.json", "packages/engine/tsconfig.json",
                     "eslint.config.js", "scripts/worktree.sh", "render.yaml",
                     "apps/server/src/db/migrations/0005_x.sql", "e2e/cypress.config.ts"):
            self.assertTrue(matches(path, review), path)
        for path in ("apps/web/src/game/Game.tsx", "packages/cards/src/scripts/002-bigot.ts",
                     "SPEC.md", "bots/x", "docs/scripts.md", "packages/engine/test/scripts/x.ts"):
            self.assertFalse(matches(path, forbidden), path)
            self.assertFalse(matches(path, review), path)
