// The statistics in the pinned status issue (bot/harness/stats.py, #148): what the bot's
// comments, pull requests, commits and state file say it has done over the last six hours, the
// last day, the last week and all time, who planned, built, revised and approved each pull
// request, and the count every 30 minutes, kept in the issue between counts.
//
// bot/tests/test_stats.py, ported (#467).
package tests

import (
	"fmt"
	"slices"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stats"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// statsRun is RUN.
const statsRun = "https://github.com/Jlore-Studios/JackiOh/actions/runs"

// DAY is 2026-09-29 17:00 UTC: the six-hour window starts at 11:00 that day, the day's at 17:00 on
// 09-28, the week's at 17:00 on 09-22.

// statsSay is say.
func statsSay(gh *fakes.FakeGitHub, number int, body, at string) {
	gh.AddComment(number, body, fakes.Comment{User: fakes.Bot.Copy(), Association: "COLLABORATOR",
		CreatedAt: at})
}

// statsMerged is merged_pull's keyword arguments; Files and Commits default to 1 (a zero field),
// and a nil Body is `Closes #<issue>`.
type statsMerged struct {
	Opened, Merged string
	Additions      int
	Deletions      int
	Files, Commits int
	Body           *string
}

// statsMergedPull is merged_pull.
func statsMergedPull(gh *fakes.FakeGitHub, number, issue int, o statsMerged) *pyjson.Object {
	body := fmt.Sprintf("Closes #%d", issue)
	if o.Body != nil {
		body = *o.Body
	}
	files, commits := o.Files, o.Commits
	if files == 0 {
		files = 1
	}
	if commits == 0 {
		commits = 1
	}
	pull := gh.AddPull(number, fmt.Sprintf("bot/issue-%d", issue), fakes.Pull{
		Title: fmt.Sprintf("Patch v0.2.Y: work for #%d", issue), Body: body, User: fakes.Bot.Copy()})
	pull.Update(pyjson.New("created_at", o.Opened, "merged_at", o.Merged, "state", "closed",
		"additions", o.Additions, "deletions", o.Deletions, "changed_files", files, "commits", commits))
	return pull
}

// statsWorld is world.
func statsWorld() (*fakes.FakeGitHub, *hctx.Context) {
	gh := fakes.NewFakeGitHub()
	for _, number := range []int{3, 4, 5, 6} {
		gh.AddIssue(number, fakes.Issue{Title: fmt.Sprintf("Patch v0.2.Y: item %d", number)})
	}
	// #9, for issue 3, inside the six hours: planned on claude-3, built on lite (its own reviewer
	// claude-2), revised on muse (which reviewed its own revision), and approved again by agy.
	statsSay(gh, 3, "Planning this now ([run]("+statsRun+"/12)), on `claude-3` (claude, `opus`, strong) "+
		"(`bot:needs-plan`).", "2026-09-29T11:30:00Z")
	statsSay(gh, 3, "Planned on `claude-3` (claude, `opus`, strong) ([run]("+statsRun+"/12)). The plan is in "+
		"this issue's description.", "2026-09-29T11:45:00Z")
	statsSay(gh, 3, "Starting work on this now ([run]("+statsRun+"/13)): it is difficulty:easy. I build it on "+
		"`lite` (lite, `lite-1`, weak).", "2026-09-29T11:50:00Z")
	statsSay(gh, 3, "Opened #9, built on `lite` (lite, `lite-1`, weak). Its adversarial reviewer, "+
		"`claude-2` (claude, `opus`, strong), approved it on round 1 of 10.",
		"2026-09-29T12:00:00Z")
	statsSay(gh, 9, "Starting a revision now ([run]("+statsRun+"/14)), because of: CI. I build it on `muse` "+
		"(muse, `muse-spark`, medium).", "2026-09-29T13:50:00Z")
	statsSay(gh, 9, "Revision pushed ([run]("+statsRun+"/14)) on `muse` (muse, `muse-spark`, medium); `muse` "+
		"(muse, `muse-spark`, medium) reviewed it adversarially and approved it",
		"2026-09-29T14:00:00Z")
	statsSay(gh, 9, "Starting a review now ([run]("+statsRun+"/15)), on `agy` (agy, `gemini-3.1-pro`, "+
		"medium).", "2026-09-29T14:30:00Z")
	statsSay(gh, 9, "Second review ([run]("+statsRun+"/15)): `agy` (agy, `gemini-3.1-pro`, medium) approved "+
		"it. Auto-merge is on.", "2026-09-29T14:40:00Z")
	statsSay(gh, 4, "@MaxGoetzmann Queued #4; I will build it in the next run.", "2026-09-29T16:30:00Z")
	statsMergedPull(gh, 9, 3, statsMerged{Opened: "2026-09-29T12:00:00Z", Merged: "2026-09-29T15:00:00Z",
		Additions: 40, Deletions: 5, Files: 4, Commits: 3})
	// #10, for issue 4, inside the day: no "Opened" comment named its builder, so it is the last
	// build started on its issue before it was opened (gpt).
	statsSay(gh, 4, "Starting work on this now ([run]("+statsRun+"/20)): it is difficulty:medium. I build it "+
		"on `gpt` (codex, `gpt-5.6-terra`, medium).", "2026-09-28T23:00:00Z")
	statsMergedPull(gh, 10, 4, statsMerged{Opened: "2026-09-29T00:00:00Z", Merged: "2026-09-29T03:00:00Z",
		Additions: 100, Deletions: 10, Files: 2})
	// #11, for issue 5, inside the week: the only build on its issue started after it was opened,
	// so nobody is recorded as its builder; that build failed.
	statsSay(gh, 5, "Starting work on this now ([run]("+statsRun+"/30)): it is difficulty:medium. I build it "+
		"on `claude-1` (claude, `opus`, strong).", "2026-09-26T10:00:00Z")
	statsSay(gh, 5, "This run failed ([run]("+statsRun+"/30)): RuntimeError", "2026-09-26T10:30:00Z")
	statsMergedPull(gh, 11, 5, statsMerged{Opened: "2026-09-26T00:00:00Z", Merged: "2026-09-26T12:00:00Z",
		Additions: 7, Deletions: 1})
	// #12, for issue 6, only in all time.
	fixes := "Fixes #6"
	statsMergedPull(gh, 12, 6, statsMerged{Opened: "2026-09-10T00:00:00Z", Merged: "2026-09-11T00:00:00Z",
		Additions: 1000, Deletions: 0, Body: &fixes})
	// An open one, and a person's.
	gh.AddPull(13, "bot/issue-7", fakes.Pull{Title: "Still open", User: fakes.Bot.Copy()}).Update(
		pyjson.New("created_at", "2026-09-29T16:00:00Z"))
	gh.AddPull(20, "someone/else", fakes.Pull{Title: "A person's change"}).Update(pyjson.New(
		"created_at", "2026-09-29T16:00:00Z", "merged_at", "2026-09-29T16:30:00Z", "state", "closed",
		"additions", 5000, "deletions", 0))
	gh.Commits = []*pyjson.Object{pyjson.New("sha", "a", "author", pyjson.New("login", fakes.Bot.Get("login"))),
		pyjson.New("sha", "b", "author", pyjson.New("login", "jgoetzmann"))}
	gh.Runs.Update(pyjson.New(
		"1", pyjson.New("status", "completed", "conclusion", "success", "created_at", "2026-09-29T16:00:00Z"),
		"2", pyjson.New("status", "completed", "conclusion", "failure", "created_at", "2026-09-20T00:00:00Z"),
		"3", pyjson.New("status", "completed", "conclusion", "success")))
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day})
	ctx.Store.Update(func(s *pyjson.Object) any {
		pyjson.Obj(s.SetDefault("providers", pyjson.New())).Update(pyjson.New(
			"lite", pyjson.New("spent", []any{pyjson.New("at", "2026-09-29T12:00:00Z", "minutes", 120.0),
				pyjson.New("at", "2026-09-20T00:00:00Z", "minutes", 60.0)})))
		return nil
	}, "state")
	return gh, ctx
}

// statsSection is section: one window's folded section of the block.
func statsSection(t *testing.T, body, name string) string {
	t.Helper()
	start := strIndex(t, body, "<details><summary><b>"+name+"</b>")
	return body[start : start+strIndex(t, body[start:], "</details>")]
}

// statsBlock is stats.block(ctx, facts) with its defaults.
func statsBlock(ctx *hctx.Context, facts *stats.Facts) string {
	return stats.Block(ctx, facts, stats.MergesShown, nil, "")
}

// ---------------------------------------------------------------------------- ParseTests

func TestStats_ACommentSaysWhatHappenedInWhichRunOnWhichSubscription(t *testing.T) {
	event := stats.Parse(pyjson.New("body", "Starting a revision now ([run]("+statsRun+"/7)), because of: CI. I "+
		"build it on `muse` (muse, `muse-spark-1.3-contributor`, medium).",
		"issue_url", "https://api.github.com/repos/o/r/issues/12",
		"created_at", "2026-09-28T01:00:00Z"))
	assertEqual(t, []any{event.Kind, event.Action, event.Run, event.Provider, event.Model, event.Number},
		[]any{"start", "revise", "7", "muse", "muse-spark-1.3-contributor", 12})
	opened := stats.Parse(pyjson.New("body", "Opened #44, built on `gpt` (codex, gpt-5.6-terra).",
		"issue_url", "x/issues/3"))
	assertTrue(t, opened.PR != nil, "opened.pr is None")
	if opened.PR != nil {
		assertEqual(t, []any{opened.Kind, *opened.PR, opened.Provider, opened.Model},
			[]any{"opened", 44, "gpt", "gpt-5.6-terra"})
	}
	for _, c := range [][2]string{{"Revision pushed on `lite` …", "revised"},
		{"Paused: the run's time budget is spent.", "paused"},
		{"My revision did not pass review after 10 rounds", "not approved"},
		{"The run could not work on this on `claude-3`: 401", "infra"},
		{"This run failed: RuntimeError", "failed"},
		{"Queued #4; I will build it", "queued"}} {
		assertEqual(t, stats.Classify(c[0]), c[1], c[0])
	}
}

func TestStats_ItReadsWhoApprovedAChangeInEachWayTheBotSaysIt(t *testing.T) {
	approvers := func(body string) []string {
		return stats.Parse(pyjson.New("body", body, "issue_url", "x/issues/3")).Approvers
	}
	none := func(body string) {
		t.Helper()
		if got := approvers(body); len(got) != 0 {
			t.Errorf("approvers(%q) = %#v, want ()", body, got)
		}
	}

	assertEqual(t, approvers("Opened #9, built on `lite` (lite, `lite-1`, weak). Its "+
		"adversarial reviewer, `claude-2` (claude, `opus`, strong), "+
		"approved it on round 2 of 10."), []string{"claude-2"})
	assertEqual(t, approvers("Revision pushed (run) on `muse` (muse, `m`, medium); `agy` "+
		"(agy, `g`, medium) reviewed it adversarially and approved it"),
		[]string{"agy"})
	assertEqual(t, approvers("Second review (run): `gpt` (codex, `gpt-5.6-terra`, medium) "+
		"approved it. Auto-merge is on."), []string{"gpt"})
	// A vote tally names families, not seats, and a builder is not an approver.
	none("`claude-opus` (strong) approved it, so it waits")
	none("Opened #9, built on `lite` (lite, `lite-1`, weak). " +
		"Nobody has approved it yet.")
}

func TestStats_AnOutcomeTakesItsRunsSubscription(t *testing.T) {
	events := stats.Joined([]*stats.Event{
		{At: "1", Number: 3, Kind: "start", Run: "5", Provider: "agy", Model: "g"},
		{At: "2", Number: 3, Kind: "paused", Run: "5"},
		{At: "3", Number: 4, Kind: "failed", Run: "6"},
	})
	var got [][2]string
	for _, e := range events {
		got = append(got, [2]string{e.Provider, e.Model})
	}
	assertEqual(t, got, [][2]string{{"agy", "g"}, {"agy", "g"}, {"", ""}})
}

// ---------------------------------------------------------------------------- CreditTests

func TestStats_EachPullRequestNamesWhoPlannedBuiltRevisedAndApprovedIt(t *testing.T) {
	_, ctx := statsWorld()
	facts := stats.Collect(ctx)
	who := stats.Credits_(facts.Events, facts.Pulls)
	assertEqual(t, who.Planned[9], []string{"claude-3"})
	assertEqual(t, who.Built[9], "lite")
	assertEqual(t, who.Revised[9], []string{"muse"})
	assertEqual(t, who.Approved[9], []string{"claude-2", "muse", "agy"})
	// No "Opened" comment: the last build started on its issue before it was opened.
	assertEqual(t, who.Built[10], "gpt")
	// The only build on its issue started after it was opened.
	_, built := who.Built[11]
	assertTrue(t, !built, "11 in who.built")
	assertEqual(t, who.Builder(pyjson.New("number", 11)), stats.Unrecorded)
	var numbers []any
	for _, p := range facts.Pulls {
		numbers = append(numbers, p.Get("number"))
	}
	assertEqual(t, numbers, []any{9, 10, 11, 12, 13})
}

// ---------------------------------------------------------------------------- IssueTests

func TestStats_TheFourWindowsSitSideBySideInOneTable(t *testing.T) {
	_, ctx := statsWorld()
	body := statsBlock(ctx, stats.Collect(ctx))
	assertIn(t, "| | Last 6 hours | Last 24 hours | Last 7 days | All time |", body)
	for _, line := range []string{"| Runs started | 4 | 5 | 6 | 6 |",
		"| … plans · builds · revisions · reviews | 1 · 1 · 1 · 1 | 1 · 2 · 1 · 1 " +
			"| 1 · 3 · 1 · 1 | 1 · 3 · 1 · 1 |",
		"| Outcomes that delivered something | 4 of 4 (100%) | 4 of 4 (100%) " +
			"| 4 of 5 (80%) | 4 of 5 (80%) |",
		"| Failed review or failed | 0 | 0 | 1 | 1 |",
		"| Pull requests opened | 2 | 3 | 4 | 5 |",
		"| Pull requests merged | 1 | 2 | 3 | 4 |",
		"| Issues closed by them | 1 | 2 | 3 | 4 |",
		"| Lines merged (added / removed) | +40 / −5 | +140 / −15 | +147 / −16 " +
			"| +1,147 / −16 |",
		"| Files changed in them | 4 | 6 | 7 | 8 |",
		"| Commits in them | 3 | 4 | 5 | 6 |",
		"| Median time from opening to merge | 3h 00m | 3h 00m | 3h 00m | 7h 30m |",
		"| Model hours | 2.0 | 2.0 | 2.0 | 3.0 |",
		"| `bot-night.yml` workflow runs | 1 | 1 | 1 | 3 |",
		"| Requests answered (`Queued …`) | 1 | 1 | 1 | 1 |"} {
		assertIn(t, line, body)
	}
	assertIn(t, "Now: 1 of its pull requests open, 0 closed without merging, 1 commits on "+
		"`main` by the bot itself. `bot-night.yml` runs ended: success 2, failure 1",
		body)
	var headings []string
	for _, line := range py.Splitlines(body) {
		if strings.HasPrefix(line, "#") {
			headings = append(headings, line)
		}
	}
	assertEqual(t, headings, []string{"## 📊 What it has done", "### The four windows",
		"### The last two weeks", "### Per window"})
	assertTrue(t, strings.HasPrefix(body, stats.BlockStart))
	assertTrue(t, strings.HasSuffix(body, stats.BlockEnd))
	assertEqual(t, stats.Kept("before\n"+body+"\nafter"), body)
	assertEqual(t, stats.Kept("no block here"), "") // None
}

func TestStats_EachWindowSaysWhoDidWhatAndEveryMergeIsListedOnce(t *testing.T) {
	_, ctx := statsWorld()
	body := statsBlock(ctx, stats.Collect(ctx))
	six := statsSection(t, body, "Last 6 hours")
	assertIn(t, "<b>Last 6 hours</b>: 4 runs, 1 pull requests merged, +40 / −5 lines", six)
	assertIn(t, "| `lite` | `lite-1` | 1 | 0 | 1 | 0 | 0 | 1 of 1 | 1 | 1 | +40 / −5 "+
		"| 0 | 0 | 0 | 2.0 h |", six)
	assertIn(t, "| `agy` | `gemini-3.1-pro` | 1 | 0 | 0 | 0 | 1 | 1 of 1 | 0 | 0 | +0 / −0 "+
		"| 0 | 0 | 0 | 0.0 h |", six)
	assertNotIn(t, "#10 ", six)
	assertNotIn(t, "`gpt`", six)

	week := statsSection(t, body, "Last 7 days")
	// #11 and the open #13: no comment names who built either.
	assertIn(t, "| _not recorded_ | — | 0 | 0 | 0 | 0 | 0 | 0 of 0 | 2 | 1 | +7 / −1 |",
		week)
	assertIn(t, "| `claude-1` | `opus` | 1 | 0 | 1 | 0 | 0 | 0 of 1 | 0 | 0 | +0 / −0 "+
		"| 0 | 1 | 0 | 0.0 h |", week)

	ever := statsSection(t, body, "All time")
	assertIn(t, "| `lite` | `lite-1` | 1 | 0 | 1 | 0 | 0 | 1 of 1 | 1 | 1 | +40 / −5 "+
		"| 0 | 0 | 0 | 3.0 h (its last 2 runs) |", ever)
	// One list of every merge, newest first, with when it merged and who did what.
	merged := body[strIndex(t, body, "<b>Pull requests merged</b>"):]
	assertIn(t, "<b>Pull requests merged</b>: the newest 4 of 4", merged)
	var numbers []string
	for _, line := range py.Splitlines(merged) {
		if strings.HasPrefix(line, "| #") {
			numbers = append(numbers, strings.Fields(line)[1])
		}
	}
	assertEqual(t, numbers, []string{"#9", "#10", "#11", "#12"})
	assertIn(t, "| #9 Patch v0.2.Y: work for #3 | Tue 09-29 10:00 | `claude-3` | `lite` "+
		"| `muse` | `claude-2`, `muse`, `agy` | 3h 00m | +40 / −5 | 4 |", merged)
	assertIn(t, "| #10 Patch v0.2.Y: work for #4 | Mon 09-28 22:00 | — | `gpt` | — | — "+
		"| 3h 00m | +100 / −10 | 2 |", merged)
	assertIn(t, "| #11 Patch v0.2.Y: work for #5 | Sat 09-26 07:00 | — | not recorded | — "+
		"| — | 12h 00m | +7 / −1 | 1 |", merged)
	assertEqual(t, strings.Count(body, "| #9 Patch"), 1)
	// A person's pull request is not the bot's work.
	assertNotIn(t, "#20 ", body)
	assertNotIn(t, "+5,000", body)
}

func TestStats_TheChartsAreBarsAndLinesNeverPies(t *testing.T) {
	_, ctx := statsWorld()
	body := statsBlock(ctx, stats.Collect(ctx))
	assertNotIn(t, "pie", body)
	assertEqual(t, strings.Count(body, "```mermaid"), strings.Count(body, "xychart-beta"))
	assertIn(t, `title "Runs started, by subscription (last 6 hours)"`, body)
	assertIn(t, `title "Lines added in merged pull requests, by builder (last 24 hours)"`,
		body)
	lines := py.Splitlines(statsSection(t, body, "Last 24 hours"))
	at := slices.Index(lines, `    title "Lines added in merged pull requests, by builder `+
		`(last 24 hours)"`)
	if at < 0 {
		t.Fatalf("the chart's title is not a line of the section")
	}
	assertEqual(t, lines[at+1:at+4], []string{`    x-axis ["gpt", "lite"]`, `    y-axis "lines"`,
		`    bar [100, 40]`})
	assertIn(t, `title "What runs ended in (all time)"`, body)
	assertEqual(t, strings.Count(body, `title "Runs started per day (last 14 days)"`), 1)
}

func TestStats_AQuietWindowSaysSo(t *testing.T) {
	gh, _ := statsWorld()
	later := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day.Add(48 * time.Hour)})
	body := statsBlock(later, stats.Collect(later))
	assertIn(t, "<details><summary><b>Last 6 hours</b>: nothing happened</summary>", body)
	assertIn(t, "<details><summary><b>Last 24 hours</b>: nothing happened</summary>", body)
	assertIn(t, "| Pull requests merged | 0 | 0 | 3 | 4 |", body)
}

func TestStats_TheMergeListIsCutToItsNewest(t *testing.T) {
	gh, ctx := statsWorld()
	for number := 100; number < 160; number++ {
		statsMergedPull(gh, number, 3, statsMerged{Opened: "2026-09-29T12:00:00Z",
			Merged: "2026-09-29T15:00:00Z", Additions: 1, Deletions: 1})
	}
	facts := stats.Collect(ctx)
	// 64 merged in all: the block lists its newest MERGES_SHOWN, or none at all.
	assertIn(t, fmt.Sprintf("_… and %d older ones._", 64-stats.MergesShown), statsBlock(ctx, facts))
	assertIn(t, "_… and 54 older ones._", stats.Block(ctx, facts, 10, nil, ""))
	none := stats.Block(ctx, facts, 0, nil, "")
	assertIn(t, "_64 pull requests merged; the list is left out", none)
	assertNotIn(t, "| #100 ", none)
}
