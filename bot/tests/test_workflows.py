"""The workflows keep their safety shape. Parsed with regular expressions: stdlib only."""

from __future__ import annotations

import re
import unittest

from harness import config

WORKFLOWS = config.REPO_ROOT / ".github" / "workflows"


def read(name: str) -> str:
    return (WORKFLOWS / name).read_text(encoding="utf-8")


def job(text: str, name: str) -> str:
    """The block of one job: from `  name:` to the next job at the same indent."""
    match = re.search(rf"^  {re.escape(name)}:\n(.*?)(?=^  [a-z][\w-]*:\n|\Z)", text, re.M | re.S)
    if not match:
        raise AssertionError(f"no job {name}")
    return match.group(1)


class NightWorkflowTests(unittest.TestCase):
    text = read("bot-night.yml")

    def test_four_jobs_in_order(self):
        self.assertIn("needs: gate", job(self.text, "plan"))
        self.assertIn("if: needs.gate.outputs.go == 'true'", job(self.text, "plan"))
        self.assertIn("needs: plan", job(self.text, "work"))
        self.assertIn("needs: [plan, work]", job(self.text, "deliver"))
        self.assertIn("always()", job(self.text, "deliver"))

    def test_the_model_job_holds_no_github_write_token(self):
        work = job(self.text, "work")
        self.assertNotIn("BOT_GITHUB_TOKEN", work)
        block = re.search(r"permissions:\n((?:\s{6}.*\n)+)", work).group(1)
        grants = dict(re.findall(r"^\s+([\w-]+):\s*(\w+)", block, re.M))
        self.assertEqual(grants, {"contents": "read"})
        self.assertIn("CLAUDE_CODE_OAUTH_TOKEN", work)

    def test_only_jobs_that_cannot_write_hold_the_claude_token(self):
        self.assertNotIn("CLAUDE_CODE_OAUTH_TOKEN", job(self.text, "deliver"))
        plan = job(self.text, "plan")
        self.assertNotRegex(plan, r"CLAUDE_CODE_OAUTH_TOKEN\s*:")  # never set as a variable
        mentions = re.findall(r"secrets\.CLAUDE_CODE_OAUTH_TOKEN[^}]*", plan)
        self.assertEqual(mentions, ["secrets.CLAUDE_CODE_OAUTH_TOKEN != '' "])  # presence only
        for name in ("gate", "work"):
            block = job(self.text, name)
            self.assertRegex(block, r"CLAUDE_CODE_OAUTH_TOKEN\s*:")
            self.assertNotIn("BOT_GITHUB_TOKEN", block)
            perms = re.search(r"permissions:\n((?:\s{6}.*\n)+)", block).group(1)
            grants = dict(re.findall(r"^\s+([\w-]+):\s*(\w+)", perms, re.M))
            self.assertEqual(set(grants.values()), {"read"}, name)

    def test_the_step_names_the_partner_reads(self):
        """bright-bots-harness treats `Build, check and review` as this bot spending, and its
        own gate must never look like spending (bot/harness/quiet.py)."""
        work = job(self.text, "work")
        self.assertIn("- name: Build, check and review", work)
        gate = job(self.text, "gate")
        self.assertIn("- name: Wait until the subscription is quiet", gate)
        cfg = config.load(env={})
        prefixes = [p for partner in cfg.quiet.partners for steps in partner.workflows.values()
                    for p in steps]
        for name in re.findall(r"- name: (.*)", self.text):
            if name.startswith("Build, check and review"):
                continue
            self.assertFalse(any(name.startswith(p) for p in prefixes), name)

    def test_halt_is_the_first_step_of_the_gate_and_the_plan(self):
        for name in ("gate", "plan"):
            steps = re.findall(r"^      - (?:name|uses): (.*)$", job(self.text, name), re.M)
            self.assertTrue(steps[0].startswith("HALT check"), (name, steps))

    def test_no_checkout_keeps_credentials(self):
        for name in ("bot-night.yml", "bot-commands.yml", "bot-selftest.yml"):
            text = read(name)
            checkouts = text.count("uses: actions/checkout@")
            self.assertEqual(text.count("persist-credentials: false"), checkouts, name)

    def test_every_job_has_a_timeout_and_the_group_never_cancels(self):
        for name in ("gate", "plan", "work", "deliver"):
            self.assertRegex(job(self.text, name), r"timeout-minutes: \d+")
        gate = int(re.search(r"timeout-minutes: (\d+)", job(self.text, "gate")).group(1))
        cfg = config.load(env={})
        self.assertGreater(gate, cfg.quiet.max_wait_minutes + cfg.quiet.interval_minutes)
        self.assertIn("cancel-in-progress: false", self.text)

    def test_the_crons_cover_the_window(self):
        crons = re.findall(r'cron: "([^"]+)"', self.text)
        self.assertEqual(crons, ["17 2-13 * * *"])

    def test_deliver_takes_the_item_from_the_plan_jobs_outputs(self):
        deliver = job(self.text, "deliver")
        self.assertIn("needs.plan.outputs.action", deliver)
        self.assertIn("needs.plan.outputs.number", deliver)

    def test_the_budget_fits_inside_the_job_timeout(self):
        work = job(self.text, "work")
        timeout = int(re.search(r"timeout-minutes: (\d+)", work).group(1))
        cfg = config.load(env={})
        self.assertLess(cfg.job_budget_minutes + 10, timeout)
        self.assertLessEqual(timeout, 360)


class CommandsWorkflowTests(unittest.TestCase):
    text = read("bot-commands.yml")

    def test_no_model_and_no_pull_request_code(self):
        self.assertNotIn("CLAUDE_CODE_OAUTH_TOKEN", self.text)
        self.assertNotIn("claude-code", self.text)
        self.assertNotIn("github.event.pull_request.head", self.text)
        self.assertIn("ref: ${{ github.event.repository.default_branch || 'main' }}", self.text)
        self.assertNotIn("concurrency:", self.text)

    def test_the_bot_does_not_wake_itself(self):
        cfg = config.load(env={})
        self.assertIn(f"github.actor != '{cfg.bot_login}'", self.text)
        self.assertIn(f"'@{cfg.bot_login}'", self.text)
        self.assertIn(f"workflows: [{cfg.ci_workflow}]", self.text)

    def test_pull_request_events_use_target(self):
        self.assertIn("pull_request_target:", self.text)
        self.assertNotRegex(self.text, r"^  pull_request:", )


class RequiredChecksTests(unittest.TestCase):
    def test_every_required_check_is_a_job_that_runs_on_every_pull_request(self):
        cfg = config.load(env={})
        names = set()
        for path in WORKFLOWS.glob("*.yml"):
            text = path.read_text(encoding="utf-8")
            if not re.search(r"^  pull_request:", text, re.M):
                continue
            for raw in re.findall(r"^    name: (.+)$", text, re.M):
                raw = raw.strip().strip('"')
                if "${{ matrix.browser }}" in raw:
                    names.update(raw.replace("${{ matrix.browser }}", b) for b in ("chrome", "electron"))
                else:
                    names.add(raw)
        missing = [check for check in cfg.required_checks if check not in names]
        self.assertEqual(missing, [], f"required checks with no job: {missing}; jobs: {sorted(names)}")


if __name__ == "__main__":
    unittest.main()
