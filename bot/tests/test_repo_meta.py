"""The repository's meta (#187): the pull request title rule and the check that runs it, the issue
forms, and the one list of labels. Stdlib only, so the YAML is read with regular expressions."""

from __future__ import annotations

import argparse
import contextlib
import io
import json
import re
import unittest
from unittest import mock

from harness import __main__ as main_mod
from harness import config, dashboard, disk, stats, triage

from tests.fakes import FakeGitHub
from tests.support import make_config, make_ctx
from tests.test_workflows import WORKFLOWS, job, read

ROOT = config.REPO_ROOT
FORMS = ROOT / ".github" / "ISSUE_TEMPLATE"


class PullTitleTests(unittest.TestCase):
    """`triage.pull_title_ok`: the convention, the titles tools give their own pull requests, and a
    revert of either. Everything else fails the `pr title` check."""

    AUTOMATED = (
        "patches ship: v0.3.1",  # patches-ship.yml
        "patches ship: v0.2.16b, v0.2.17",
        "Promote main to production: 28 commit(s) up to 6d44be2",  # scripts/promote-production.sh
        "Promote main to production: 1 commit(s) up to c9ca419",
        "AI gen 8 (improve): the opening plays its two-drops first",  # training/improve.md
        "AI gen 12 (unban): Mirror Image is played again",  # training/unban.md
    )
    CONVENTIONAL = (
        "Patch v0.3.X: Activate abilities work again",
        "Patch v0.3.Y: fix the UI of the rematch button",
        "Patch v0.3.1: card text pass",
        "Patch v0.2.7b: one more fix",
        "Patch v0.3.X (part 2 of 3): the lobby",
        "v0.3.0: the Rust rewrite",
        "v0.4.0 (part 1 of 5): the engine",
        "Night bot: a seventh subscription, any hour with no caps",
        "CI: daily super run failed",
        "Architecture: issue and PR templates, labels as code and repository meta",
        'Revert "Patch v0.3.X: Activate abilities work again"',
        'Revert "patches ship: v0.3.1"',
    )
    BAD = (
        "Share a room as an invite link that opens the join form filled in (R767)",  # #490
        "Practice: a worker whose script never loaded says to reload",  # #471
        "v0.3.0 part 40: the sweep of record",  # #472
        "bot: build pass 1 for #85",  # a bot commit's subject, #169
        "Patch v0.2.X Classic & Classic+ balance",
        "patch v0.3.X: lower case",
        "Architecture:",
        "patches ship: ",
        "patches ship: v0.3.Y",  # ship names a micro patch's version
        "Promote main to production: soon",
        "AI gen 3 (sideways): a lane that does not exist",
        'Revert "Share a room"',
        "",
    )

    def test_conventional_and_automated_titles_pass(self):
        for title in (*self.CONVENTIONAL, *self.AUTOMATED):
            self.assertTrue(triage.pull_title_ok(title), title)

    def test_every_other_title_fails(self):
        for title in self.BAD:
            self.assertFalse(triage.pull_title_ok(title), title)

    def test_the_automated_titles_are_the_ones_their_tools_write(self):
        """Each tool's title, as its source writes it, so a change to one fails here first."""
        self.assertIn('--title "patches ship: $versions"', read("patches-ship.yml"))
        self.assertIn('versions=$(sed -n \'s/^patches ship: shipped //p\'', read("patches-ship.yml"))
        promote = (ROOT / "scripts" / "promote-production.sh").read_text(encoding="utf-8")
        self.assertIn('--title "Promote main to production: $COUNT commit(s) up to ${cand:0:7}"',
                      promote)
        loop = (ROOT / "training" / "loop.sh").read_text(encoding="utf-8")
        self.assertIn("title=$(git log -1 --format=%s)", loop)
        self.assertIn('gh pr create --base main --head "$branch" --title "$title"', loop)
        for lane in ("improve", "unban"):
            text = (ROOT / "training" / f"{lane}.md").read_text(encoding="utf-8")
            self.assertIn(f'git commit -m "AI gen <N> ({lane}): <what changed>"', text)

    def test_a_bot_takes_its_builders_title_when_it_passes_else_its_issues(self):
        self.assertEqual(triage.pull_title("Patch v0.3.2: the rules, v2", "Patch v0.3.X: the rules"),
                         "Patch v0.3.2: the rules, v2")
        self.assertEqual(triage.pull_title("Make the rules v2", "Patch v0.3.X: the rules"),
                         "Patch v0.3.X: the rules")
        self.assertEqual(triage.pull_title("Make the rules v2", "the rules"), "Make the rules v2")
        self.assertEqual(triage.pull_title("", "the rules"), "the rules")
        self.assertEqual(triage.pull_title("", ""), "")


class TitleWorkflowTests(unittest.TestCase):
    text = read("pr-title.yml")

    def test_every_title_and_head_is_checked_never_with_the_pull_requests_code(self):
        self.assertRegex(self.text, r"pull_request_target:\n\s+types: \[opened, edited, reopened, "
                                    r"synchronize\]")
        self.assertNotRegex(self.text, r"(?m)^  pull_request:", "would run the pull request's code")
        self.assertEqual(re.findall(r"ref: (.*)", self.text),
                         ["${{ github.event.repository.default_branch }}"])
        self.assertIn("persist-credentials: false", self.text)
        self.assertNotIn("secrets.", self.text)
        self.assertRegex(self.text, r"(?m)^permissions: \{\}$")
        block = job(self.text, "title")
        grants = re.search(r"permissions:\n((?:\s{6}.*\n)+)", block).group(1)
        self.assertEqual(dict(re.findall(r"^\s+([\w-]+):\s*(\w+)", grants, re.M)),
                         {"contents": "read"})
        self.assertIn("timeout-minutes:", block)

    def test_the_title_reaches_the_bots_rule_through_the_environment(self):
        self.assertIn("TITLE: ${{ github.event.pull_request.title }}", self.text)
        self.assertEqual(self.text.count("github.event.pull_request.title"), 1)
        self.assertIn("from harness.triage import pull_title_ok", self.text)
        self.assertIn(".github/workflows/pr-title.yml", read("bot-selftest.yml"))


class IssueFormTests(unittest.TestCase):
    forms = {path.name: path.read_text(encoding="utf-8") for path in sorted(FORMS.glob("*.yml"))
             if path.name != "config.yml"}

    def test_one_form_per_kind(self):
        self.assertEqual(sorted(self.forms), ["architecture.yml", "bug.yml", "micro-patch.yml",
                                              "night-bot.yml", "patch.yml"])

    def test_each_form_titles_labels_and_types_its_issue(self):
        for name, text in self.forms.items():
            title = re.search(r'^title: "(.*)"$', text, re.M).group(1)
            self.assertTrue(triage.follows_convention(title + "what it does"), name)
            labels = json.loads(re.search(r"^labels: (\[.*\])$", text, re.M).group(1))
            # The sync creates every one, and every issue carries a type label.
            self.assertLessEqual(set(labels), set(config.LABELS), name)
            self.assertTrue(set(labels) & set(config.TYPE_LABELS), name)
            self.assertIn(re.search(r"^type: (\w+)$", text, re.M).group(1),
                          triage.DEFAULT_ISSUE_TYPES, name)
            self.assertIn("docs/issues-and-patches.md", text, name)

    def test_no_blank_issues_and_a_link_to_the_conventions(self):
        text = (FORMS / "config.yml").read_text(encoding="utf-8")
        self.assertIn("blank_issues_enabled: false", text)
        self.assertIn("/blob/main/docs/issues-and-patches.md", text)
        self.assertTrue((ROOT / "docs" / "issues-and-patches.md").is_file())


class LabelListTests(unittest.TestCase):
    """`config.LABELS` is the one list of the repository's labels: the sweep creates a missing one
    and `harness setup` brings a hand-changed one back."""

    def test_every_label_in_use_is_on_the_list(self):
        docs = (ROOT / "docs" / "issues-and-patches.md").read_text(encoding="utf-8")
        table = re.search(r"^## Labels\n(.*?)^## ", docs, re.M | re.S).group(1)
        named = re.findall(r"^\| `([^`]+)` \|", table, re.M)
        self.assertEqual(named[:5], ["patch", "major version", "architecture", "night bot", "Info"])
        used = {*named, *dashboard.LABELS, *disk.LABELS, *stats.LABELS, *triage.TYPE_LABELS,
                config.LABEL_HUMAN, config.LABEL_READY, *triage.METHODS, *config.DIFFICULTY_LABELS,
                config.LABEL_PRIORITY_HIGH, config.LABEL_PRIORITY_MEDIUM, config.LABEL_PRIORITY_LOW}
        # And every label a workflow or a script puts on by name.
        for path in (*WORKFLOWS.glob("*.yml"), *(ROOT / "scripts").glob("*.sh")):
            used |= set(re.findall(r'--(?:add-)?label "([^"$]+)"', path.read_text(encoding="utf-8")))
        self.assertEqual(sorted(used - set(config.LABELS)), [])

    def test_the_production_merge_label_is_as_its_script_writes_it(self):
        """promote-production.sh creates its label with `--force`, which would undo any other."""
        script = (ROOT / "scripts" / "promote-production.sh").read_text(encoding="utf-8")
        color, description = config.LABELS["production merge"]
        self.assertIn(': "${RELEASE_LABEL:=production merge}"', script)
        self.assertIn(f'gh label create "$RELEASE_LABEL" --color {color} --force', script)
        self.assertIn(f'--description "{description}"', script)

    def test_setup_creates_the_missing_and_brings_back_the_hand_changed(self):
        gh = FakeGitHub()
        gh.labels = {"human": {"name": "human", "color": "f5126e", "description": "old words"},
                     "patch": {"name": "patch", "color": config.LABELS["patch"][0].upper(),
                               "description": config.LABELS["patch"][1]}}
        cfg = make_config()
        out = io.StringIO()
        with mock.patch.object(main_mod, "_ctx", lambda cfg, **_: make_ctx(gh, cfg=cfg)), \
                contextlib.redirect_stdout(out):
            self.assertEqual(main_mod.cmd_setup(cfg, argparse.Namespace(repo_settings=False)), 0)
        said = out.getvalue()
        self.assertIn("updated label human", said)
        self.assertIn("created label production merge", said)
        self.assertNotIn("label patch", said)  # it matched, whatever the colour's case
        for name, (color, description) in config.LABELS.items():
            self.assertEqual((gh.labels[name]["color"].lower(), gh.labels[name]["description"]),
                             (color, description), name)


class CodeownersTests(unittest.TestCase):
    def test_the_paths_the_bots_may_not_change_ask_a_person(self):
        text = (ROOT / ".github" / "CODEOWNERS").read_text(encoding="utf-8")
        owners = dict(re.findall(r"^(\S+)\s+(.+)$", text, re.M))
        for path in ("/.github/", "/bot/", "/.harness/", "/.squishy/"):
            self.assertEqual(owners.get(path, "").split(), ["@jgoetzmann", "@MaxGoetzmann"], path)


if __name__ == "__main__":
    unittest.main()
