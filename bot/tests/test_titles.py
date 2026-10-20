"""`harness.titles` (#1360, #1370): every title in the new form, and the threads a promotion shipped
retitled with their versions."""

from __future__ import annotations

import unittest

from harness import titles, triage

from tests.fakes import FakeGitHub


class TitleTests(unittest.TestCase):
    def test_what_it_does_drops_any_prefix_old_or_new_and_keeps_the_part(self):
        cases = {
            "Patch v0.3.X: Bigot costs (3)": ("", "Bigot costs (3)"),
            "Patch v0.3.Y: a fix": ("", "a fix"),
            "Patch v0.3.17b: a fix": ("", "a fix"),
            "Patch: a fix": ("", "a fix"),
            "v0.6.0: daily quests": ("", "daily quests"),
            "Night bot: a seventh subscription": ("", "a seventh subscription"),
            "CI: daily super run failed": ("", "daily super run failed"),
            "Architecture: patch numbers (part 2 of 8): fragments": ("", "patch numbers (part 2 of 8): fragments"),
            "Patch v0.2.X (part 2 of 4): the lobby": (" (part 2 of 4)", "the lobby"),
            "Patch (part 2 of 4): the lobby": (" (part 2 of 4)", "the lobby"),
            "Share a room as an invite link": ("", "Share a room as an invite link"),
        }
        for title, expected in cases.items():
            self.assertEqual(titles.what_it_does(title), expected, title)

    def test_new_titles_pass_the_title_check(self):
        for title in ("Patch v0.3.X: Bigot costs (3)", "Architecture: patch numbers (part 2 of 8): x",
                      "Patch v0.2.X (part 2 of 4): the lobby", "Night bot: a cap"):
            for version in ("", "v0.3.18", "v0.4.0c"):
                retitled = titles.new_title(title, version)
                self.assertTrue(triage.pull_title_ok(retitled), retitled)
        self.assertEqual(titles.new_title("Patch v0.3.X: Bigot costs (3)", "v0.3.18"),
                         "Patch v0.3.18: Bigot costs (3)")
        self.assertEqual(titles.new_title("Patch v0.2.X (part 2 of 4): the lobby", "v0.2.4"),
                         "Patch v0.2.4 (part 2 of 4): the lobby")
        self.assertEqual(titles.new_title("CI: daily super run failed"), "Patch: daily super run failed")

    def test_a_commits_pull_request_and_the_issues_it_closes(self):
        self.assertEqual(titles.pull_of("Patch v0.3.X: Bigot costs (3) (#1401)"), 1401)
        self.assertEqual(titles.pull_of("Merge pull request #1362 from Jlore-Studios/bot/issue-1300"), 1362)
        self.assertEqual(titles.pull_of("bot: build pass 1 for #85"), 0)
        self.assertEqual(titles.closed_by("Closes #1366\nPart of #1360\nfixes #7, Resolves #7"), [1366, 7])

    def test_retitles_what_a_promotion_shipped_and_names_the_version(self):
        gh = FakeGitHub()
        gh.add_issue(5, "Patch v0.3.X: Bigot costs (3)")
        gh.add_pull(9, "bot/issue-5", "Patch v0.3.X: Bigot costs (3)", body="Closes #5\nPart of #1")
        before = [{"version": "v0.3.17"}]
        after = [*before, {"version": "v0.3.17b", "commits": ["abc"]}]
        entries = titles.shipped(before, after)
        self.assertEqual([entry["version"] for entry in entries], ["v0.3.17b"])
        done = titles.retitle_shipped(gh, entries, subject_of=lambda sha: "Patch v0.3.X: Bigot costs (3) (#9)")
        self.assertEqual(done, ["#9: Patch v0.3.17b: Bigot costs (3)", "#5: Patch v0.3.17b: Bigot costs (3)"])
        self.assertEqual(gh.get_pull(9)["title"], "Patch v0.3.17b: Bigot costs (3)")
        self.assertEqual(gh.get_issue(5)["title"], "Patch v0.3.17b: Bigot costs (3)")
        self.assertIn("Shipped as **v0.3.17b**", gh.list_comments(5)[-1]["body"])
        self.assertEqual(titles.shipped(after, after), [])



class RetconTests(unittest.TestCase):
    """#1372: every thread retitled from a reviewed dry run, but those a workflow finds by text."""

    def thread(self, number, title, *, state="open", pull=False, labels=()):
        thread = {"number": number, "title": title, "state": state,
                  "labels": [{"name": name} for name in labels]}
        if pull:
            thread["pull_request"] = {"url": "x"}
        return thread

    def test_titles_a_workflow_or_a_tool_finds_are_left_alone(self):
        for title in ("CI: a job ran over 7 minutes", "CI: daily super run failed",
                      "Render: the live server is not serving main's catalog",
                      "patches ship refused to promote the pending patches",
                      "Night bot: the machine's disk is filling up",
                      "Architecture: weekly hygiene, dead files and archives, 2026-10-19",
                      "Night bot status", "patches ship: v0.3.17", "AI gen 13 (improve): in training",
                      "Promote main to production: 3 commit(s) up to abc1234",
                      'Revert "Patch v0.3.X: a fix"'):
            self.assertTrue(titles.left_alone(self.thread(9, title)), title)
        self.assertTrue(titles.left_alone(self.thread(148, "Anything")))
        self.assertTrue(titles.left_alone(self.thread(9, "Merging to production in 3 hours",
                                                       labels=("production merge",))))
        self.assertFalse(titles.left_alone(self.thread(9, "CI: the learn lane folds an export")))

    def test_plans_the_new_titles_and_the_open_issues_kind_labels(self):
        threads = [
            self.thread(1, "Patch v0.3.X: Bigot costs (3)", state="closed"),
            self.thread(2, "Architecture: patch numbers (part 2 of 8): fragments"),
            self.thread(3, "v0.6.0: daily quests"),
            self.thread(4, "Patch v0.3.Y: a client fix", pull=True),
            self.thread(5, "CI: daily super run failed"),
            self.thread(6, "Patch: already new"),
            self.thread(7, "Night bot: a cap", labels=("kind:internal",)),
        ]
        rows = titles.plan_retcon(threads, {1: "v0.3.4"})
        self.assertEqual([(r.number, r.new, r.label) for r in rows], [
            (1, "Patch v0.3.4: Bigot costs (3)", ""),
            (2, "Patch: patch numbers (part 2 of 8): fragments", "kind:internal"),
            (3, "Patch: daily quests", "kind:app"),
            (4, "Patch: a client fix", ""),
            (7, "Patch: a cap", ""),
        ])
        for row in rows:
            self.assertTrue(triage.pull_title_ok(row.new), row.new)
        # Run again over its own result, it changes nothing.
        done = [self.thread(r.number, r.new, labels=(r.label,) if r.label else ()) for r in rows]
        self.assertEqual(titles.plan_retcon(done, {1: "v0.3.4"}), [])

    def test_a_shipped_patch_names_its_source_its_pull_request_and_the_issues_it_closes(self):
        gh = FakeGitHub()
        gh.add_pull(9, "bot/issue-5", "Patch v0.3.X: Bigot costs (3)", body="Closes #5")
        patches = [{"version": "v0.3.4", "source": "#7 and #8", "commits": ["abc"]},
                   {"version": "v0.3.5", "source": "#8", "commits": []}]
        found = titles.versions_by_thread(patches, gh, lambda sha: "Bigot costs (3) (#9)")
        self.assertEqual(found, {7: "v0.3.4", 8: "v0.3.5", 9: "v0.3.4", 5: "v0.3.4"})

    def test_writes_each_title_and_label_with_a_pause_between(self):
        gh = FakeGitHub()
        gh.add_issue(2, "Architecture: a thing")
        rows = titles.plan_retcon([self.thread(2, "Architecture: a thing")], {})
        slept = []
        self.assertEqual(titles.apply_retcon(gh, rows, pause=2.0, sleep=slept.append), 1)
        self.assertEqual(gh.get_issue(2)["title"], "Patch: a thing")
        self.assertIn("kind:internal", [l["name"] for l in gh.get_issue(2)["labels"]])
        self.assertEqual(slept, [2.0])


if __name__ == "__main__":
    unittest.main()
