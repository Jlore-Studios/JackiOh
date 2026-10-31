// titles_test.go is bot/tests/test_titles.py: `harness.titles` (#1360, #1370): every title in the
// new form, and the threads a promotion shipped retitled with their versions.
package tests

import (
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/titles"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/triage"
)

// ------------------------------------------------------------------ TitleTests

func TestTitles_WhatItDoesDropsAnyPrefixOldOrNewAndKeepsThePart(t *testing.T) {
	cases := [][3]string{
		{"Patch v0.3.X: Bigot costs (3)", "", "Bigot costs (3)"},
		{"Patch v0.3.Y: a fix", "", "a fix"},
		{"Patch v0.3.17b: a fix", "", "a fix"},
		{"Patch: a fix", "", "a fix"},
		{"v0.6.0: daily quests", "", "daily quests"},
		{"Night bot: a seventh subscription", "", "a seventh subscription"},
		{"CI: daily super run failed", "", "daily super run failed"},
		{"Architecture: patch numbers (part 2 of 8): fragments", "", "patch numbers (part 2 of 8): fragments"},
		{"Patch v0.2.X (part 2 of 4): the lobby", " (part 2 of 4)", "the lobby"},
		{"Patch (part 2 of 4): the lobby", " (part 2 of 4)", "the lobby"},
		{"Share a room as an invite link", "", "Share a room as an invite link"},
	}
	for _, c := range cases {
		part, rest := titles.WhatItDoes(c[0])
		assertEqual(t, [2]string{part, rest}, [2]string{c[1], c[2]}, c[0])
	}
}

func TestTitles_NewTitlesPassTheTitleCheck(t *testing.T) {
	for _, title := range []string{"Patch v0.3.X: Bigot costs (3)", "Architecture: patch numbers (part 2 of 8): x",
		"Patch v0.2.X (part 2 of 4): the lobby", "Night bot: a cap"} {
		for _, version := range []string{"", "v0.3.18", "v0.4.0c"} {
			retitled := titles.NewTitle(title, version)
			assertTrue(t, triage.PullTitleOK(retitled), retitled)
		}
	}
	assertEqual(t, titles.NewTitle("Patch v0.3.X: Bigot costs (3)", "v0.3.18"), "Patch v0.3.18: Bigot costs (3)")
	assertEqual(t, titles.NewTitle("Patch v0.2.X (part 2 of 4): the lobby", "v0.2.4"),
		"Patch v0.2.4 (part 2 of 4): the lobby")
	assertEqual(t, titles.NewTitle("CI: daily super run failed", ""), "Patch: daily super run failed")
}

func TestTitles_ACommitsPullRequestAndTheIssuesItCloses(t *testing.T) {
	assertEqual(t, titles.PullOf("Patch v0.3.X: Bigot costs (3) (#1401)"), 1401)
	assertEqual(t, titles.PullOf("Merge pull request #1362 from Jlore-Studios/bot/issue-1300"), 1362)
	assertEqual(t, titles.PullOf("bot: build pass 1 for #85"), 0)
	assertEqual(t, titles.ClosedBy("Closes #1366\nPart of #1360\nfixes #7, Resolves #7"), []int{1366, 7})
}

func TestTitles_RetitlesWhatAPromotionShippedAndNamesTheVersion(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5, fakes.Issue{Title: "Patch v0.3.X: Bigot costs (3)"})
	gh.AddPull(9, "bot/issue-5", fakes.Pull{Title: "Patch v0.3.X: Bigot costs (3)", Body: "Closes #5\nPart of #1"})
	before := []*pyjson.Object{pyjson.New("version", "v0.3.17")}
	after := append(append([]*pyjson.Object{}, before...), pyjson.New("version", "v0.3.17b", "commits", []any{"abc"}))
	entries := titles.Shipped(before, after)
	versions := []string{}
	for _, entry := range entries {
		versions = append(versions, entry.Str("version"))
	}
	assertEqual(t, versions, []string{"v0.3.17b"})
	done := titles.RetitleShipped(gh, entries, func(sha string) string { return "Patch v0.3.X: Bigot costs (3) (#9)" })
	assertEqual(t, done, []string{"#9: Patch v0.3.17b: Bigot costs (3)", "#5: Patch v0.3.17b: Bigot costs (3)"})
	assertEqual(t, gh.GetPull(9).Get("title"), "Patch v0.3.17b: Bigot costs (3)")
	assertEqual(t, gh.GetIssue(5).Get("title"), "Patch v0.3.17b: Bigot costs (3)")
	comments := gh.ListComments(5, 300)
	assertIn(t, "Shipped as **v0.3.17b**", comments[len(comments)-1].Str("body"))
	assertEqual(t, titles.Shipped(after, after), []*pyjson.Object{})
}

func TestTitles_ALockedThreadIsUnlockedForTheCommentAndLockedAgain(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5, fakes.Issue{Title: "Patch v0.3.X: Bigot costs (3)"})
	gh.AddPull(9, "bot/issue-5", fakes.Pull{Title: "Patch v0.3.X: Bigot costs (3)", Body: "Closes #5"})
	gh.Lock(5)
	gh.Lock(9)
	entries := []*pyjson.Object{pyjson.New("version", "v0.3.17b", "commits", []any{"abc"})}
	titles.RetitleShipped(gh, entries, func(sha string) string { return "Patch v0.3.X: Bigot costs (3) (#9)" })
	for _, number := range []int{5, 9} {
		comments := gh.ListComments(number, 300)
		assertIn(t, "Shipped as **v0.3.17b**", comments[len(comments)-1].Str("body"))
		assertTrue(t, gh.GetIssue(number).Truthy("locked"))
	}
}

func TestTitles_APushThatRenamesShippedVersionsShipsNothing(t *testing.T) {
	before := []*pyjson.Object{pyjson.New("version", "v0.3.16"), pyjson.New("version", "v0.3.17")}
	retcon := []*pyjson.Object{pyjson.New("version", "v0.4.0b"), pyjson.New("version", "v0.4.0c")}
	assertEqual(t, titles.Shipped(before, retcon), []*pyjson.Object{})
	assertEqual(t, titles.Shipped(before, append(append([]*pyjson.Object{}, retcon...),
		pyjson.New("version", "v0.4.0d"))), []*pyjson.Object{})
	assertEqual(t, titles.Shipped(before, before[:1]), []*pyjson.Object{})
}
