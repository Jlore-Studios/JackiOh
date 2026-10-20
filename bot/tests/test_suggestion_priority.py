"""#1409: a suggestion opens with the priority label its class maps to in
`.harness/config.json` (`suggestions.priorities`), and approving it keeps that label."""

from __future__ import annotations

import json
import unittest

from harness import config, prompts, verdicts
from harness.config import LABEL_BUILD, LABEL_SUGGESTION, SUGGESTION_CLASSES
from harness.errors import ConfigError
from harness.runner import FakeRunner
from harness.work import class_priorities

from tests.fakes import BOT
from tests.support import idle_for, raw_config
from tests.test_flow import Harness, ask
from tests.test_work import reviewer


def survey(*entries: dict) -> FakeRunner:
    text = '<!-- suggestions: [' + ",".join(json.dumps(e) for e in entries) + '] -->'
    return FakeRunner({"suggest": reviewer(text)})


def idea(title: str, kind: object = None) -> dict:
    entry = {"title": title, "body": "## Why\nx"}
    if kind is not None:
        entry["class"] = kind
    return entry


def opened(h: Harness) -> dict[str, dict]:
    return {t["title"]: t for t in h.gh.threads.values()
            if t["user"] == BOT and LABEL_SUGGESTION in {l["name"] for l in t["labels"]}}


def labels(thread: dict) -> set[str]:
    return {l["name"] for l in thread["labels"]}


def with_priorities(priorities: dict) -> dict:
    return raw_config()["suggestions"] | {"priorities": priorities}


class DeliveryTests(unittest.TestCase):
    def test_each_class_opens_with_its_priority(self):
        h = Harness(self)
        idle_for(h.ctx)
        runner = survey(idea("Fuzz", "fuzz"), idea("Security", "security"), idea("Data", "data"),
                        idea("Feature", "feature"), idea("Unknown", "nonsense"), idea("None"),
                        idea("Odd", 7))
        h.night(runner)
        made = opened(h)
        self.assertEqual({title: labels(t) for title, t in made.items()}, {
            "Fuzz": {LABEL_SUGGESTION, "priority:low"},
            "Security": {LABEL_SUGGESTION, "priority:medium"},
            "Data": {LABEL_SUGGESTION, "priority:high"},
            "Feature": {LABEL_SUGGESTION},
            "Unknown": {LABEL_SUGGESTION},
            "None": {LABEL_SUGGESTION},
            "Odd": {LABEL_SUGGESTION},
        })
        self.assertIn("Class: `fuzz` → `priority:low`; relabel to change it.", made["Fuzz"]["body"])
        self.assertNotIn("Class:", made["Feature"]["body"])
        # The survey was told every class and the priority it sets, highest first.
        prompt = runner.calls[0].prompt
        self.assertIn("- `data`: `priority:high`\n- `security`: `priority:medium`", prompt)
        # In pickup order: no priority comes before low, so `feature` wins a tie with `fuzz`.
        self.assertIn("- `polish`: `priority:medium`\n- `feature`: no priority (the owner's call)\n"
                      "- `fuzz`: `priority:low`", prompt)

    def test_the_config_alone_sets_the_priority(self):
        h = Harness(self, suggestions=with_priorities({"fuzz": "high"}))
        idle_for(h.ctx)
        h.night(survey(idea("Fuzz", "fuzz"), idea("Data", "data")))
        made = opened(h)
        self.assertEqual(labels(made["Fuzz"]), {LABEL_SUGGESTION, "priority:high"})
        self.assertEqual(labels(made["Data"]), {LABEL_SUGGESTION})

    def test_approving_keeps_the_priority(self):
        h = Harness(self)
        h.gh.add_issue(20, labels=(LABEL_SUGGESTION, "priority:low"))
        ask(h, 20, "approve", 41)
        self.assertEqual(h.gh.label_names(20), {LABEL_BUILD, "priority:low"})


class ConfigTests(unittest.TestCase):
    def test_the_committed_table(self):
        cfg = config.load(env={})
        self.assertEqual(dict(cfg.suggestion_priorities), {
            "fuzz": "low", "checks": "low", "telemetry": "low", "refactor": "low", "ai": "low",
            "fairness": "low", "security": "medium", "polish": "medium", "data": "high"})

    def test_bad_tables_fail_to_load(self):
        for priorities, message in (({"fuzz": "urgent"}, "fuzz: 'urgent' is not one of"),
                                    ({"fuzz": None}, "fuzz: None is not one of"),
                                    ({"bugs": "low"}, "unknown classes bugs"),
                                    (["fuzz"], "expected an object")):
            raw = raw_config()
            raw["suggestions"] = with_priorities(priorities)
            with self.assertRaisesRegex(ConfigError, message):
                config.parse(raw, config.REPO_ROOT, {})

    def test_no_table_sets_no_priority(self):
        raw = raw_config()
        del raw["suggestions"]["priorities"]
        cfg = config.parse(raw, config.REPO_ROOT, {})
        self.assertEqual(dict(cfg.suggestion_priorities), {})
        self.assertIn("- `data`: no priority", class_priorities(cfg.suggestion_priorities))


class PromptTests(unittest.TestCase):
    def test_the_survey_lists_every_class(self):
        text = prompts.load("suggest").template
        self.assertTrue(text.startswith("<!-- version: 4 -->"))
        for kind in SUGGESTION_CLASSES:
            self.assertIn(f"- `{kind}`: ", text)
        self.assertIn("priorities", prompts.placeholders("suggest"))

    def test_the_class_is_read(self):
        text = ('<!-- suggestions: [{"title": "A", "body": "B", "class": " fuzz "}, '
                '{"title": "C", "body": "D", "class": ["x"]}, {"title": "E", "body": "F"}] -->')
        self.assertEqual([s.kind for s in verdicts.suggestions(text, 4)], ["fuzz", "", ""])


if __name__ == "__main__":
    unittest.main()
