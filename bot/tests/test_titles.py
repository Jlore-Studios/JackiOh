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

    def test_a_push_that_renames_shipped_versions_ships_nothing(self):
        before = [{"version": "v0.3.16"}, {"version": "v0.3.17"}]
        retcon = [{"version": "v0.4.0b"}, {"version": "v0.4.0c"}]
        self.assertEqual(titles.shipped(before, retcon), [])
        self.assertEqual(titles.shipped(before, [*retcon, {"version": "v0.4.0d"}]), [])
        self.assertEqual(titles.shipped(before, before[:1]), [])


if __name__ == "__main__":
    unittest.main()
