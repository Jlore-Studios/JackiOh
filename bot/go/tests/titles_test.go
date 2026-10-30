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

// ------------------------------------------------------------------ RetconTests
//
// #1372: every thread retitled from a reviewed dry run, but those a workflow finds by text.

// titlesThread is a thread's options: RetconTests.thread's keyword arguments.
type titlesThread struct {
	State  string // = "open"
	Pull   bool
	Labels []string
}

// titlesThreadOf is RetconTests.thread.
func titlesThreadOf(number int, title string, opts ...titlesThread) *pyjson.Object {
	var o titlesThread
	if len(opts) > 0 {
		o = opts[0]
	}
	if o.State == "" {
		o.State = "open"
	}
	labels := []any{}
	for _, name := range o.Labels {
		labels = append(labels, pyjson.New("name", name))
	}
	thread := pyjson.New("number", number, "title", title, "state", o.State, "labels", labels)
	if o.Pull {
		thread.Set("pull_request", pyjson.New("url", "x"))
	}
	return thread
}

func TestTitles_TitlesAWorkflowOrAToolFindsAreLeftAlone(t *testing.T) {
	for _, title := range []string{"CI: a job ran over 7 minutes", "CI: daily super run failed",
		"Render: the live server is not serving main's catalog",
		"patches ship refused to promote the pending patches",
		"CI: a patches promotion or log is stuck",
		"Night bot: the machine's disk is filling up",
		"Architecture: weekly hygiene, dead files and archives, 2026-10-19",
		"Night bot status", "patches ship: v0.3.17", "AI gen 13 (improve): in training",
		"Promote main to production: 3 commit(s) up to abc1234",
		`Revert "Patch v0.3.X: a fix"`} {
		assertTrue(t, titles.LeftAlone_(titlesThreadOf(9, title)), title)
	}
	assertTrue(t, titles.LeftAlone_(titlesThreadOf(148, "Anything")))
	assertTrue(t, titles.LeftAlone_(titlesThreadOf(9, "Merging to production in 3 hours",
		titlesThread{Labels: []string{"production merge"}})))
	assertFalse(t, titles.LeftAlone_(titlesThreadOf(9, "CI: the learn lane folds an export")))
}

func TestTitles_PlansTheNewTitlesAndTheOpenIssuesKindLabels(t *testing.T) {
	threads := []*pyjson.Object{
		titlesThreadOf(1, "Patch v0.3.X: Bigot costs (3)", titlesThread{State: "closed"}),
		titlesThreadOf(2, "Architecture: patch numbers (part 2 of 8): fragments"),
		titlesThreadOf(3, "v0.6.0: daily quests"),
		titlesThreadOf(4, "Patch v0.3.Y: a client fix", titlesThread{Pull: true}),
		titlesThreadOf(5, "CI: daily super run failed"),
		titlesThreadOf(6, "Patch: already new"),
		titlesThreadOf(7, "Night bot: a cap", titlesThread{Labels: []string{"kind:internal"}}),
	}
	type row struct {
		Number     int
		New, Label string
	}
	rows := titles.PlanRetcon(threads, map[int]string{1: "v0.3.4"})
	got := []row{}
	for _, r := range rows {
		got = append(got, row{r.Number, r.New, r.Label})
	}
	assertEqual(t, got, []row{
		{1, "Patch v0.3.4: Bigot costs (3)", ""},
		{2, "Patch: patch numbers (part 2 of 8): fragments", "kind:internal"},
		{3, "Patch: daily quests", "kind:app"},
		{4, "Patch: a client fix", ""},
		{7, "Patch: a cap", ""},
	})
	for _, r := range rows {
		assertTrue(t, triage.PullTitleOK(r.New), r.New)
	}
	// Run again over its own result, it changes nothing.
	done := []*pyjson.Object{}
	for _, r := range rows {
		labels := []string{}
		if r.Label != "" {
			labels = []string{r.Label}
		}
		done = append(done, titlesThreadOf(r.Number, r.New, titlesThread{Labels: labels}))
	}
	assertEqual(t, titles.PlanRetcon(done, map[int]string{1: "v0.3.4"}), []*titles.Retitle{})
}

func TestTitles_AShippedPatchNamesItsSourceItsPullRequestAndTheIssuesItCloses(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddPull(9, "bot/issue-5", fakes.Pull{Title: "Patch v0.3.X: Bigot costs (3)", Body: "Closes #5"})
	patches := []*pyjson.Object{
		pyjson.New("version", "v0.3.4", "source", "#7 and #8", "commits", []any{"abc"}),
		pyjson.New("version", "v0.3.5", "source", "#8", "commits", []any{}),
	}
	found := titles.VersionsByThread(patches, gh, func(sha string) string { return "Bigot costs (3) (#9)" })
	assertEqual(t, found, map[int]string{7: "v0.3.4", 8: "v0.3.5", 9: "v0.3.4", 5: "v0.3.4"})
}

func TestTitles_WritesEachTitleAndLabelWithAPauseBetween(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(2, fakes.Issue{Title: "Architecture: a thing"})
	rows := titles.PlanRetcon([]*pyjson.Object{titlesThreadOf(2, "Architecture: a thing")}, map[int]string{})
	slept := []float64{}
	assertEqual(t, titles.ApplyRetcon(gh, rows, 2.0, func(s float64) { slept = append(slept, s) }), 1)
	assertEqual(t, gh.GetIssue(2).Get("title"), "Patch: a thing")
	names := []string{}
	for _, l := range pyjson.Objs(gh.GetIssue(2).Get("labels")) {
		names = append(names, l.Str("name"))
	}
	assertIn(t, "kind:internal", names)
	assertEqual(t, slept, []float64{2.0})
}
