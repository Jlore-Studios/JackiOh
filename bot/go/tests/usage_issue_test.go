package tests

// The usage issue (#2749, section 2, internal/usage/issue.go): "Night bot usage", drawn by the
// status loop from the usage history at each flush, with sixteen sections, its tuning suggestions
// and GitHub's limit on an issue's body; #148 links to it.

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/usage"
)

// usageIssueAt is a time on the fixture's days, UTC.
func usageIssueAt(day, h, m int) time.Time { return time.Date(2026, 11, day, h, m, 0, 0, time.UTC) }

// usageIssueCtx is a context at the tests' clock (`usageAt`, Monday 2026-11-02 12:00 UTC, 06:00 in
// the bot's time zone) on a fresh fake GitHub, with the machine's subscriptions switched on.
func usageIssueCtx(t *testing.T) (*fakes.FakeGitHub, *hctx.Context) {
	t.Helper()
	gh := fakes.NewFakeGitHub()
	cfg := testsupport.MakeConfig(testsupport.Config{Machine: testsupport.Machine})
	return gh, testsupport.MakeCtx(gh, testsupport.Ctx{Cfg: cfg, At: usageAt})
}

// usageIssueFixture is a week of a few subscriptions' history, each part made for one section's
// numbers:
//
//   - claude-4: three sessions (two at its cap), its readings of the last two days, the ticks of
//     its session that reset Sunday 10:00 UTC (capped 08:05 to 09:05, work queued for the first
//     half, held for it 30 minutes, two lanes held), and three runs: 41 cut off at its cap one
//     point past it, 42 building item 7 (handed off, resumed), 43 approving it;
//   - claude-8: an hour idle with nothing queued;
//   - claude-1, the shared account: 15 points that rose between two of its runs;
//   - agy: two weeks of weekly readings, its last week closed on day 4 at its cap;
//   - the pools: three medium builds queued;
//   - claude-6: four sessions, each at its cap with work queued and nobody else's use (a raise).
func usageIssueFixture() *usage.Facts {
	at := usageIssueAt
	n := usage.N
	f := &usage.Facts{Now: usageAt}
	reset := at(1, 10, 0)
	read := func(sub string, when time.Time, util, cap float64, resets time.Time, run string) *usage.Reading {
		return &usage.Reading{ObservedAt: when, Subscription: sub, Source: "stream", During: "call", RunID: run,
			Util5h: n(util), Resets5h: resets, Cap5h: n(cap)}
	}
	f.Readings = []*usage.Reading{
		read("claude-4", at(1, 7, 0), 50, 70, reset, "41"), read("claude-4", at(1, 8, 0), 71, 70, reset, "41"),
		read("claude-4", at(1, 9, 10), 80, 100, reset, "42"), read("claude-4", at(1, 9, 40), 85, 100, reset, "42"),
		read("claude-1", at(1, 21, 0), 10, 100, at(2, 1, 30), "51"), read("claude-1", at(1, 22, 0), 20, 100, at(2, 1, 30), "51"),
		read("claude-1", at(2, 1, 0), 35, 100, at(2, 1, 30), "52"), read("claude-1", at(2, 1, 20), 40, 100, at(2, 1, 30), "52"),
	}
	week := func(when time.Time, util float64, resets time.Time) *usage.Reading {
		return &usage.Reading{ObservedAt: when, Subscription: "agy", Source: "usage-command", During: "after",
			Util7d: n(util), Resets7d: resets, Cap7d: n(100)}
	}
	last, this := time.Date(2026, 10, 30, 22, 0, 0, 0, time.UTC), time.Date(2026, 11, 6, 22, 0, 0, 0, time.UTC)
	f.Readings = append(f.Readings, week(time.Date(2026, 10, 25, 12, 0, 0, 0, time.UTC), 40, last),
		week(time.Date(2026, 10, 27, 1, 0, 0, 0, time.UTC), 100, last), week(at(1, 12, 0), 30, this),
		week(at(2, 6, 0), 45, this))
	tick := func(sub string, when time.Time, state string, queued, held, lanes float64) *usage.Tick {
		return &usage.Tick{At: when, Subscription: sub, Enabled: n(1), Suspended: n(0), InHours: n(1), State: state,
			QueuedForIt: n(queued), HeldForIt: n(held), LanesHeld: n(lanes), LanesAllowed: n(2), RunsHeld: n(lanes),
			Cap5hNow: n(70)}
	}
	f.Ticks = []*usage.Tick{
		tick("claude-4", at(1, 8, 5), "capped", 2, 1, 2), tick("claude-4", at(1, 8, 35), "capped", 0, 0, 1),
		tick("claude-4", at(1, 9, 5), "working", 1, 0, 1),
		tick("claude-8", at(1, 14, 0), "idle", 0, 0, 0), tick("claude-8", at(1, 15, 0), "working", 1, 0, 1),
	}
	f.Pools = []*usage.PoolTick{{At: at(1, 14, 0), GithubSlotsUsed: n(2), GithubSlots: n(8), AWSSlotsUsed: n(1),
		AWSSlots: n(6), PlanLanesUsed: n(0), PlanLanes: n(6), BuildMedium: n(3), Queued: n(3), HeldForOwner: n(1)},
		{At: at(1, 15, 0), GithubSlots: n(8), AWSSlots: n(6), PlanLanes: n(6)}}
	session := func(sub string, resets time.Time, hit bool, unused, queued float64) *usage.Session {
		s := &usage.Session{Subscription: sub, StartedAt: resets.Add(-5 * time.Hour), ResetAt: resets, CapMin: n(70),
			CapMax: n(70), UnusedAtReset: n(unused), UtilLastRead: n(100 - unused), MinutesCappedQueued: n(queued),
			OvershootMax: n(0), LastHourUsed: n(5)}
		if hit {
			s.HitCapAt = resets.Add(-2 * time.Hour)
		}
		return s
	}
	f.Sessions = []*usage.Session{session("claude-4", reset, true, 15, 30),
		session("claude-4", at(1, 10, 0).Add(-24*time.Hour), false, 60, 0),
		session("claude-4", at(1, 10, 0).Add(-48*time.Hour), true, 5, 0)}
	for i := range 4 {
		f.Sessions = append(f.Sessions, session("claude-6", at(1, 6, 0).Add(-time.Duration(i)*24*time.Hour), true, 30, 20))
	}
	f.Weeks = []*usage.Week{{Subscription: "agy", ResetAt: last, PeakUtil: n(100), UtilAtReset: n(100),
		UnusedAtReset: n(0), FirstCappedAt: time.Date(2026, 10, 27, 1, 0, 0, 0, time.UTC), DaysRefused: n(3.9)}}
	f.Runs = []*usage.Run{
		{EndedAt: at(1, 8, 10), RunID: "41", Subscription: "claude-4", Kind: "build", Item: "5", StopReason: "5-hour cap",
			ModelMinutes: n(40), TokensIn: n(1000), TokensOut: n(2000), USDReported: n(4), QueueWaitMin: n(20)},
		{EndedAt: at(1, 9, 50), RunID: "42", Subscription: "claude-4", Kind: "build", Item: "7", Outcome: "built",
			ModelMinutes: n(30), TokensIn: n(500), TokensOut: n(500), USDReported: n(3), QueueWaitMin: n(40),
			HandedOff: "cap", Session: "resumed"},
		{EndedAt: at(1, 11, 0), RunID: "43", Subscription: "claude-4", Kind: "review", Item: "7", Outcome: "approved",
			ModelMinutes: n(20), Session: "fresh"},
		{EndedAt: at(1, 22, 5), RunID: "51", Subscription: "claude-1", Kind: "plan", Item: "9", ModelMinutes: n(60)},
		{EndedAt: at(2, 1, 25), RunID: "52", Subscription: "claude-1", Kind: "build", Item: "9", ModelMinutes: n(20)},
	}
	f.Files = []string{"backfill.json", "readings-2026-10.csv", "readings-2026-11.csv", "runs-2026-11.csv",
		"sessions-2026-11.csv", "ticks-2026-11.csv", "weeks.csv"}
	return f
}

// usageIssueSection is section `n` of a body: from its heading to the next one's.
func usageIssueSection(t *testing.T, body string, n int) string {
	t.Helper()
	start := strings.Index(body, fmt.Sprintf("## %d. ", n))
	if start < 0 {
		t.Fatalf("no section %d in:\n%s", n, body)
	}
	rest := body[start+1:]
	if end := strings.Index(rest, "\n## "); end >= 0 {
		rest = rest[:end]
	}
	return rest
}

// usageIssueRow is the table row of `section` that starts with `start`.
func usageIssueRow(t *testing.T, section, start string) string {
	t.Helper()
	for _, line := range strings.Split(section, "\n") {
		if strings.HasPrefix(line, start) {
			return line
		}
	}
	t.Fatalf("no row %q in:\n%s", start, section)
	return ""
}

// usageIssueChart is the chart titled `title` in `section`, its lines.
func usageIssueChart(t *testing.T, section, title string) string {
	t.Helper()
	i := strings.Index(section, `title "`+title)
	if i < 0 {
		t.Fatalf("no chart %q in:\n%s", title, section)
	}
	chart := section[i:]
	return chart[:strings.Index(chart, "```")]
}

// usageIssueSeries is the values of a chart's `kind` series, the `nth` of that kind (0 the first).
func usageIssueSeries(t *testing.T, chart, kind string, nth int) []string {
	t.Helper()
	found := regexp.MustCompile(`(?m)^    `+kind+` \[(.*)\]$`).FindAllStringSubmatch(chart, -1)
	if nth >= len(found) {
		t.Fatalf("no %s series %d in:\n%s", kind, nth, chart)
	}
	return strings.Split(found[nth][1], ", ")
}

// ------------------------------------------------------------------ the sections

// Each of the sixteen sections draws from the history's rows: one number of each, as the fixture
// makes it.
func TestUsageIssue_EachSectionDrawsFromTheHistorysRows(t *testing.T) {
	_, ctx := usageIssueCtx(t)
	body := usage.Render(ctx, usageIssueFixture(), usage.Shrinks[0], 0)
	section := func(n int) string { return usageIssueSection(t, body, n) }
	// 1: claude-4's sessions at its cap, its median unused share and its minutes capped with work
	// queued (30 in the week); claude-6's suggestion, linked to its row in section 15.
	assertIn(t, "| `claude-4` | 2 of 3 (66.7%) | 15% | 4.3 min a day | — | — | 5-hour cap 1 | 1.5 | 4k | $7.00 |",
		usageIssueRow(t, section(1), "| `claude-4` |"))
	assertIn(t, "[`limits.five_hour` 70% → 80%](#usage-suggestion-claude-6)", usageIssueRow(t, section(1), "| `claude-6` |"))
	// 2: claude-4's readings (85% its highest) against its cap (70%, lifted to 100% in its last
	// hour), and its reset at 04:00.
	five := usageIssueChart(t, section(2), "claude-4: 5-hour usage")
	assertIn(t, "85", usageIssueSeries(t, five, "line", 0))
	assertIn(t, "70", usageIssueSeries(t, five, "line", 1))
	assertIn(t, "claude-4's resets: Sun 04:00.", section(2))
	// 3: agy's week, at 100% before its reset on Friday.
	assertIn(t, "100", usageIssueSeries(t, usageIssueChart(t, section(3), "agy: weekly usage"), "line", 0))
	assertIn(t, "agy's weeks reset: Fri 10-30 17:00.", section(3))
	// 4: claude-4's cut-off at 02:10, the most in any hour.
	cuts := section(4)[strings.Index(section(4), "**Cut-offs"):]
	assertTrue(t, strings.HasPrefix(usageIssueRow(t, cuts, "| `claude-4` |"), "| `claude-4` | · | · | █ | · |"))
	assertTrue(t, strings.HasSuffix(usageIssueRow(t, cuts, "| `claude-4` |"), "| 1 |"))
	// 5: claude-4's sessions unused at their resets, day by day (5%, 60%, 15%).
	left := usageIssueSeries(t, usageIssueChart(t, section(5), "claude-4: unused"), "bar", 0)
	assertEqual(t, left[len(left)-4:], []string{"5", "60", "15", "0"})
	// 6: claude-4's 30 minutes at its cap with work queued, on Sunday.
	capped := usageIssueSeries(t, usageIssueChart(t, section(6), "claude-4: minutes"), "bar", 0)
	assertEqual(t, capped[len(capped)-2:], []string{"30", "0"})
	// 7: claude-8's hour idle with nothing queued, on Sunday.
	idle := usageIssueChart(t, section(7), "claude-8: minutes idle")
	assertEqual(t, usageIssueSeries(t, idle, "bar", 0)[12], "60")
	assertEqual(t, usageIssueSeries(t, idle, "bar", 1)[12], "0")
	// 8: claude-4's two lanes held, of two allowed.
	assertIn(t, "2", usageIssueSeries(t, usageIssueChart(t, section(8), "claude-4: lanes"), "line", 0))
	assertIn(t, "8", usageIssueSeries(t, usageIssueChart(t, section(8), "GitHub runner slots"), "line", 1))
	// 9: three medium builds queued; the builds' median wait.
	assertIn(t, "3", usageIssueSeries(t, usageIssueChart(t, section(9), "Queued to build"), "line", 0))
	assertIn(t, "| build | 2 | 30 min | 40 min |", section(9))
	// 10: claude-4's 90 model minutes on Sunday; item 7 built by it, at 50 minutes and $3.
	assertTrue(t, strings.HasSuffix(usageIssueRow(t, section(10), "| `claude-4` |"), "| 90 | · |"))
	assertIn(t, "| `claude-4` | 1 | 50 | $3.00 |", section(10))
	// 11: run 41 stopped a point past claude-4's cap.
	assertIn(t, "| `claude-4` | 1 | 1 point | 1 point | 0 | — | — |", section(11))
	// 12: claude-1 rose 15 points between its two runs.
	assertIn(t, "`claude-1` rose 15 points outside the bot's runs in the last 14 days", section(12))
	// 13: agy's last week, at its cap on day 4 and refused 3.9 days; this week so far.
	assertIn(t, "| `agy` | to Fri 10-30 17:00 | 100% | day 4, Mon 10-26 20:00 | 3.9 |", section(13))
	assertIn(t, "| `agy` | this one, to Fri 11-06 16:00 | 45% | not yet |", section(13))
	// 14: claude-4's 30-minute hold, run 42's handoff and its resumed session.
	assertIn(t, "| `claude-4` | 1 | 30 | 30 | 0 |", section(14))
	assertIn(t, "| cap | 1 | 40 min |", section(14))
	assertIn(t, "| resumed | 1 | 30 | 30 |", section(14))
	// 15: claude-6's raise, with the anchor section 1 links.
	assertIn(t, `| <a name="usage-suggestion-claude-6"></a>`+"`claude-6` | `limits.five_hour` | 70% | 80% | hit its cap "+
		"with work queued in 4 of its 4 sessions over 14 days", section(15))
	// 16: the raw links, and a spreadsheet's import.
	raw := "https://raw.githubusercontent.com/jgoetzmann/JackiOh/bot-usage/usage/sessions-2026-11.csv"
	assertIn(t, "[raw]("+raw+")", section(16))
	assertIn(t, `=IMPORTDATA("`+raw+`")`, section(16))
	assertIn(t, "File → Import", section(16))
	// Every section folds under its heading, and no chart is a pie.
	assertEqual(t, strings.Count(body, "<details open><summary>"), 16)
	assertNotIn(t, "pie", body)
}

// A section with nothing to draw yet says so in a line.
func TestUsageIssue_ASectionWithNothingYetSaysSo(t *testing.T) {
	_, ctx := usageIssueCtx(t)
	body := usage.Render(ctx, &usage.Facts{Now: usageAt}, usage.Shrinks[0], 0)
	assertIn(t, "No Claude account was read in the last 48 hours.", usageIssueSection(t, body, 2))
	assertIn(t, "No 5-hour session has closed in the last 14 days yet", usageIssueSection(t, body, 5))
	assertIn(t, "No tick recorded in the last 14 days yet.", usageIssueSection(t, body, 6))
	assertIn(t, "No week of `agy`, `gpt`, `muse` read yet", usageIssueSection(t, body, 13))
	assertIn(t, "No suggestion", usageIssueSection(t, body, 15))
	assertIn(t, "Nothing is on `bot-usage` yet", usageIssueSection(t, body, 16))
	assertNotIn(t, "```mermaid", body)
}

// The hour the clocks go back is drawn twice, so its second label carries its zone: mermaid
// draws points with one label as one.
func TestUsageIssue_AnHourThatRepeatsIsLabelledApart(t *testing.T) {
	_, ctx := usageIssueCtx(t)
	utc := usage.Hourly(usageIssueAt(1, 9, 0), time.UTC, 4*time.Hour, 1)
	assertEqual(t, utc.Points(), 4)
	assertEqual(t, utc.Labels(time.UTC, "Mon 15h"), []string{"Sun 06h", "Sun 07h", "Sun 08h", "Sun 09h"})
	zone := dashboard.Local(ctx, usageAt).Location() // America/Chicago: back an hour at 02:00 on 2026-11-01
	local := usage.Hourly(usageIssueAt(1, 9, 0), zone, 4*time.Hour, 1)
	assertEqual(t, local.Labels(zone, "Mon 15h"), []string{"Sun 01h", "Sun 01h CST", "Sun 02h", "Sun 03h"})
}

// ------------------------------------------------------------------ the suggestions

// usageIssueRule is `rule`'s suggestion for subscription `id` on `f`.
func usageIssueRule(t *testing.T, ctx *hctx.Context, f *usage.Facts, id string,
	rule func(*hctx.Context, *usage.Facts, *providers.Provider) *usage.Suggestion) *usage.Suggestion {
	t.Helper()
	p := ctx.Cfg.Pool.Get(id)
	if p == nil {
		t.Fatalf("no %s in the pool", id)
	}
	return rule(ctx, f, p)
}

// usageIssueSessions is `count` of `id`'s sessions, the last resetting an hour before the clock,
// one every five hours, each at its 70% cap with work queued when `hit` holds for its index.
func usageIssueSessions(id string, count int, hit func(i int) bool) []*usage.Session {
	var out []*usage.Session
	for i := range count {
		reset := usageAt.Add(-time.Hour - time.Duration(i)*5*time.Hour)
		s := &usage.Session{Subscription: id, StartedAt: reset.Add(-5 * time.Hour), ResetAt: reset, CapMin: usage.N(70),
			CapMax: usage.N(70), UnusedAtReset: usage.N(30), MinutesCappedQueued: usage.N(0)}
		if hit(i) {
			s.HitCapAt, s.MinutesCappedQueued = reset.Add(-time.Hour), usage.N(45)
		}
		out = append(out, s)
	}
	return out
}

// A 5-hour cap may go up 10 points when it was hit with work queued in at least half of its
// sessions over 14 days and nobody used the reserve; not at less than half, and not when a
// reading rose outside the bot's runs.
func TestUsageIssue_ACapMayGoUpWhenItsReserveWentUnused(t *testing.T) {
	_, ctx := usageIssueCtx(t)
	half := &usage.Facts{Now: usageAt, Sessions: usageIssueSessions("claude-6", 4, func(i int) bool { return i < 2 })}
	s := usageIssueRule(t, ctx, half, "claude-6", usage.SuggestRaiseCap)
	assertTrue(t, s != nil, "2 of 4 sessions at the cap")
	assertEqual(t, []string{s.Key, s.Now, s.Suggested}, []string{"limits.five_hour", "70%", "80%"})
	short := &usage.Facts{Now: usageAt, Sessions: usageIssueSessions("claude-6", 5, func(i int) bool { return i < 2 })}
	assertTrue(t, usageIssueRule(t, ctx, short, "claude-6", usage.SuggestRaiseCap) == nil, "2 of 5")
	used := &usage.Facts{Now: usageAt, Sessions: half.Sessions, Readings: []*usage.Reading{
		{ObservedAt: usageAt.Add(-3 * time.Hour), Subscription: "claude-6", RunID: "1", Util5h: usage.N(40), Resets5h: usageAt.Add(time.Hour)},
		{ObservedAt: usageAt.Add(-2 * time.Hour), Subscription: "claude-6", RunID: "2", Util5h: usage.N(41), Resets5h: usageAt.Add(time.Hour)}}}
	assertTrue(t, usageIssueRule(t, ctx, used, "claude-6", usage.SuggestRaiseCap) == nil, "a point rose outside")
	assertEqual(t, usage.RisesOutside(used.Readings, nil), []usage.Rise{{At: usageAt.Add(-2 * time.Hour), Points: 1}})
	// claude-3 has no cap to raise.
	assertTrue(t, usageIssueRule(t, ctx, &usage.Facts{Now: usageAt, Sessions: usageIssueSessions("claude-3", 4,
		func(int) bool { return true })}, "claude-3", usage.SuggestRaiseCap) == nil)
}

// usageIssueWeek is a week's readings of `id`, from 36 hours before the clock at 50% to the clock
// at 70%, resetting `left` after it, against a weekly cap of 90%, with a 5-hour reading or not.
func usageIssueWeek(id string, left time.Duration, fiveHour bool) []*usage.Reading {
	read := func(ago time.Duration, util float64) *usage.Reading {
		r := &usage.Reading{ObservedAt: usageAt.Add(-ago), Subscription: id, Util7d: usage.N(util),
			Resets7d: usageAt.Add(left), Cap7d: usage.N(90)}
		if fiveHour {
			r.Util5h, r.Resets5h = usage.N(10), usageAt.Add(time.Hour)
		}
		return r
	}
	return []*usage.Reading{read(36*time.Hour, 50), read(18*time.Hour, 60), read(0, 70)}
}

// A 5-hour cap should come down when the last three days' trend takes the week past its weekly cap
// before its reset (claude-2: 70% rising 0.56 points an hour, 72 hours to go, past 90%); a reset
// sooner than the trend reaches the cap says nothing. A subscription with no 5-hour reading gets a
// budget of model minutes instead.
func TestUsageIssue_ACapComesDownWhenTheWeekIsOnCourseToPassItsCap(t *testing.T) {
	_, ctx := usageIssueCtx(t)
	s := usageIssueRule(t, ctx, &usage.Facts{Now: usageAt, Readings: usageIssueWeek("claude-2", 72*time.Hour, true)},
		"claude-2", usage.SuggestWeekPace)
	assertTrue(t, s != nil, "on course past 90%")
	assertEqual(t, []string{s.Key, s.Now, s.Suggested}, []string{"limits.five_hour", "90%", "45%"})
	assertIn(t, "110% by its reset", s.Evidence)
	assertTrue(t, usageIssueRule(t, ctx, &usage.Facts{Now: usageAt, Readings: usageIssueWeek("claude-2", 24*time.Hour, true)},
		"claude-2", usage.SuggestWeekPace) == nil, "83% by its reset")
	runs := []*usage.Run{{EndedAt: usageAt.Add(-time.Hour), RunID: "9", Subscription: "agy", ModelMinutes: usage.N(144)}}
	agy := usageIssueRule(t, ctx, &usage.Facts{Now: usageAt, Readings: usageIssueWeek("agy", 72*time.Hour, false), Runs: runs},
		"agy", usage.SuggestWeekPace)
	assertTrue(t, agy != nil, "agy's week")
	assertEqual(t, []string{agy.Key, agy.Now, agy.Suggested}, []string{"limits.five_hour_minutes", "none", "5"})
}

// usageIssueOvershoots is `count` sessions of claude-3 in the last week, each `past` points past
// its cap with two lanes held in it.
func usageIssueOvershoots(count int, past float64) *usage.Facts {
	f := &usage.Facts{Now: usageAt}
	for _, s := range usageIssueSessions("claude-3", count, func(int) bool { return true }) {
		s.OvershootMax = usage.N(past)
		f.Sessions = append(f.Sessions, s)
		f.Ticks = append(f.Ticks, &usage.Tick{At: s.StartedAt.Add(time.Hour), Subscription: "claude-3",
			LanesHeld: usage.N(2), RunsHeld: usage.N(2)})
	}
	return f
}

// Two lanes may be too many when three sessions in a week went more than `LastHourSlack` past the
// cap with two runs going; not at two sessions, nor at the slack itself.
func TestUsageIssue_TwoLanesMayBeTooMany(t *testing.T) {
	_, ctx := usageIssueCtx(t)
	s := usageIssueRule(t, ctx, usageIssueOvershoots(3, 1.5), "claude-3", usage.SuggestFewerLanes)
	assertTrue(t, s != nil, "three sessions 1.5 points past")
	assertEqual(t, []string{s.Key, s.Now, s.Suggested}, []string{"lanes_by_cap.high", "2", "1"})
	assertTrue(t, usageIssueRule(t, ctx, usageIssueOvershoots(2, 1.5), "claude-3", usage.SuggestFewerLanes) == nil)
	assertTrue(t, usageIssueRule(t, ctx, usageIssueOvershoots(3, 100*providers.LastHourSlack), "claude-3",
		usage.SuggestFewerLanes) == nil)
}

// usageIssueStarts is four runs of claude-4 that started at 66% under its 70% cap (inside its 5
// points of headroom), `cut` of them cut off at it within five minutes.
func usageIssueStarts(cut int) *usage.Facts {
	f := &usage.Facts{Now: usageAt}
	for i := range 4 {
		start := usageAt.Add(-time.Duration(i+1) * 2 * time.Hour)
		id := fmt.Sprint(100 + i)
		run := &usage.Run{EndedAt: start.Add(5 * time.Minute), RunID: id, Subscription: "claude-4", ModelMinutes: usage.N(5)}
		if i < cut {
			run.StopReason = "5-hour cap"
		}
		f.Runs = append(f.Runs, run)
		f.Readings = append(f.Readings, &usage.Reading{ObservedAt: start, Subscription: "claude-4", RunID: id,
			Util5h: usage.N(66), Cap5h: usage.N(70), Resets5h: start.Add(time.Hour)})
	}
	return f
}

// `start_headroom` may be too small when more than half the runs started inside it were cut off
// within ten minutes; not at half.
func TestUsageIssue_StartHeadroomMayBeTooSmall(t *testing.T) {
	_, ctx := usageIssueCtx(t)
	s := usageIssueRule(t, ctx, usageIssueStarts(3), "claude-4", usage.SuggestHeadroom)
	assertTrue(t, s != nil, "3 of 4 cut off")
	assertEqual(t, []string{s.Key, s.Now, s.Suggested}, []string{"start_headroom.five_hour", "5%", "10%"})
	assertTrue(t, usageIssueRule(t, ctx, usageIssueStarts(2), "claude-4", usage.SuggestHeadroom) == nil, "2 of 4")
}

// usageIssueLastHours is `count` sessions of claude-4 whose last hour lifted the cap with work
// queued and no run, and went unused.
func usageIssueLastHours(count int) *usage.Facts {
	f := &usage.Facts{Now: usageAt}
	for _, s := range usageIssueSessions("claude-4", count, func(int) bool { return true }) {
		s.LastHourUsed = usage.N(0)
		f.Sessions = append(f.Sessions, s)
		f.Ticks = append(f.Ticks, &usage.Tick{At: s.ResetAt.Add(-30 * time.Minute), Subscription: "claude-4",
			LastHourNow: usage.N(1), QueuedForIt: usage.N(2), RunsHeld: usage.N(0)})
	}
	return f
}

// `last_hour` is being wasted when three sessions let its lift go unused with work queued; not two.
func TestUsageIssue_LastHourIsWastedWhenItsLiftGoesUnusedWithWorkQueued(t *testing.T) {
	_, ctx := usageIssueCtx(t)
	s := usageIssueRule(t, ctx, usageIssueLastHours(3), "claude-4", usage.SuggestLastHour)
	assertTrue(t, s != nil, "three sessions")
	assertEqual(t, s.Key, "last_hour")
	assertIn(t, "a routing problem, not a cap", s.Evidence)
	assertTrue(t, usageIssueRule(t, ctx, usageIssueLastHours(2), "claude-4", usage.SuggestLastHour) == nil)
}

// usageIssueHolds is four holds for claude-4 a day apart, each `minutes` long (a tick each
// `usage.TickEvery` while it lasts, as the loop takes them), and a run handed off that waited
// `wait` minutes (none when it is 0).
func usageIssueHolds(minutes []float64, wait float64) *usage.Facts {
	f := &usage.Facts{Now: usageAt}
	for i, m := range minutes {
		from := usageAt.Add(-time.Duration(i+1) * 24 * time.Hour)
		end := from.Add(time.Duration(m) * time.Minute)
		for at := from; at.Before(end); at = at.Add(usage.TickEvery) {
			f.Ticks = append(f.Ticks, &usage.Tick{At: at, Subscription: "claude-4", HeldForIt: usage.N(1)})
		}
		f.Ticks = append(f.Ticks, &usage.Tick{At: end, Subscription: "claude-4", HeldForIt: usage.N(0)})
	}
	if wait > 0 {
		f.Runs = []*usage.Run{{EndedAt: usageAt.Add(-time.Hour), RunID: "7", Subscription: "claude-1", HandedOff: "cap",
			QueueWaitMin: usage.N(wait)}}
	}
	return f
}

// `hold_minutes` needs moving when holds last longer than a handoff's wait, or more than half of
// them run out; not when a handoff waits longer, nor at half run out.
func TestUsageIssue_HoldMinutesNeedsMoving(t *testing.T) {
	_, ctx := usageIssueCtx(t)
	// The committed hold (120 since #2903): a hold that lasts all of it ran out.
	hold := float64(ctx.Cfg.HoldMinutes)
	assertEqual(t, ctx.Cfg.HoldMinutes, 120)
	s := usage.SuggestHoldMinutes(ctx, usageIssueHolds([]float64{30, 30, 30, 30}, 10))
	assertTrue(t, s != nil, "holds of 30 minutes, a handoff of 10")
	assertEqual(t, []string{s.Subscription, s.Key, s.Now, s.Suggested},
		[]string{"", "hold_minutes (config.json)", strconv.Itoa(ctx.Cfg.HoldMinutes), "10"})
	assertTrue(t, usage.SuggestHoldMinutes(ctx, usageIssueHolds([]float64{30, 30, 30, 30}, 40)) == nil, "a handoff of 40")
	out := usage.SuggestHoldMinutes(ctx, usageIssueHolds([]float64{hold, hold, hold, 20}, 0))
	assertTrue(t, out != nil, "3 of 4 ran out")
	assertEqual(t, out.Suggested, "20")
	assertIn(t, "3 of the 4 holds ran out", out.Evidence)
	assertTrue(t, usage.SuggestHoldMinutes(ctx, usageIssueHolds([]float64{hold, hold, 20, 20}, 0)) == nil, "2 of 4 ran out")
}

// usageIssueWalls is gpt's two closed weeks, at its cap on the given days of each.
func usageIssueWalls(days ...int) *usage.Facts {
	f := &usage.Facts{Now: usageAt}
	for i, day := range days {
		reset := usageAt.Add(-time.Duration(len(days)-i) * 7 * 24 * time.Hour)
		f.Weeks = append(f.Weeks, &usage.Week{Subscription: "gpt", ResetAt: reset, PeakUtil: usage.N(100),
			UnusedAtReset: usage.N(0), FirstCappedAt: reset.Add(-7*24*time.Hour + time.Duration(day-1)*24*time.Hour + time.Hour)})
	}
	return f
}

// The plan load may move when a medium subscription reached its weekly cap before day 6 in two
// weeks running; not when one of them reached it on day 6.
func TestUsageIssue_ThePlanLoadMayMoveOffAMediumSubscriptionThatHitsItsWall(t *testing.T) {
	_, ctx := usageIssueCtx(t)
	s := usageIssueRule(t, ctx, usageIssueWalls(4, 5), "gpt", usage.SuggestPlanLoad)
	assertTrue(t, s != nil, "day 4 and day 5")
	assertEqual(t, []string{s.Key, s.Now}, []string{"roles", "plans"})
	assertIn(t, "day 4 and day 5", s.Evidence)
	assertTrue(t, usageIssueRule(t, ctx, usageIssueWalls(4, 6), "gpt", usage.SuggestPlanLoad) == nil, "day 6")
	assertTrue(t, usageIssueRule(t, ctx, usageIssueWalls(4, 5), "claude-2", usage.SuggestPlanLoad) == nil, "not medium")
}

// usageIssueSnapshot is every file under `dirs`, by path, to its bytes.
func usageIssueSnapshot(t *testing.T, dirs ...string) map[string]string {
	t.Helper()
	files := map[string]string{}
	for _, dir := range dirs {
		herr.Must(filepath.Walk(dir, func(path string, info os.FileInfo, err error) error {
			if err != nil || info.IsDir() {
				return err
			}
			data, err := os.ReadFile(path)
			files[path] = string(data)
			return err
		}))
	}
	return files
}

// No suggestion writes a file: the history and the committed config are as they were after the
// issue is drawn with suggestions in it.
func TestUsageIssue_NoSuggestionWritesAFile(t *testing.T) {
	_, ctx := usageIssueCtx(t)
	root := t.TempDir()
	f := usageIssueFixture()
	var rows []usage.Row
	for _, r := range f.Readings {
		rows = append(rows, r)
	}
	for _, s := range f.Sessions {
		rows = append(rows, s)
	}
	_, err := usage.NewHistory(root).Append(rows...)
	herr.Must(err)
	harness := filepath.Join(testsupport.Root, ".harness")
	before := usageIssueSnapshot(t, root, harness)
	assertTrue(t, len(usage.Suggestions(ctx, usageIssueFixture())) > 0)
	assertIn(t, "opened #", usage.DrawIssue(ctx, root))
	assertIn(t, "`limits.five_hour` | 70% | 80%", ctx.GH.GetIssue(1).Str("body"))
	assertEqual(t, usageIssueSnapshot(t, root, harness), before)
}

// ------------------------------------------------------------------ the issue

// usageIssueRoot is a history on disk holding the fixture's readings.
func usageIssueRoot(t *testing.T) string {
	t.Helper()
	root := t.TempDir()
	var rows []usage.Row
	for _, r := range usageIssueFixture().Readings {
		rows = append(rows, r)
	}
	_, err := usage.NewHistory(root).Append(rows...)
	herr.Must(err)
	return root
}

// usageIssueOpened is the open issues titled as the usage issue.
func usageIssueOpened(gh *fakes.FakeGitHub) []int {
	numbers := []int{}
	for n, thread := range gh.Threads.All() {
		if pyjson.Equal(thread.Get("title"), usage.IssueTitle) && pyjson.Equal(thread.Get("state"), "open") {
			numbers = append(numbers, n)
		}
	}
	return numbers
}

// The loop opens the usage issue once, labelled `night bot` and `tracking` and pinned, keeps its
// number in the state file, rewrites it after, and finds it again by its marker when the state
// file loses it.
func TestUsageIssue_OpenedOnceLabelledPinnedAndFoundAgainByItsMarker(t *testing.T) {
	gh, ctx := usageIssueCtx(t)
	root := usageIssueRoot(t)
	note := usage.DrawIssue(ctx, root)
	opened := usageIssueOpened(gh)
	assertEqual(t, len(opened), 1)
	number := opened[0]
	assertEqual(t, note, fmt.Sprintf("opened #%d and pinned it", number))
	assertEqual(t, gh.LabelNames(number), setOf("night bot", config.LabelTracking))
	assertEqual(t, gh.Pinned, []string{fmt.Sprintf("I_%d", number)})
	assertIn(t, usage.IssueMarker, gh.Threads.Get(number).Str("body"))
	assertTrue(t, pyjson.Equal(ctx.Store.Load().Get(dashboard.UsageIssueKey), pyjson.New("issue", number, "pinned", true)))
	assertEqual(t, usage.DrawIssue(ctx, root), fmt.Sprintf("rewrote #%d", number))
	assertEqual(t, usageIssueOpened(gh), []int{number})
	assertEqual(t, len(gh.Pinned), 1)
	// The state file loses it, and a person took its labels off: found again by its marker, and
	// labelled again.
	ctx.Store.Update(func(s *pyjson.Object) any { s.Delete(dashboard.UsageIssueKey); return nil }, "state")
	gh.Threads.Get(number).Set("labels", []any{})
	assertIn(t, fmt.Sprintf("rewrote #%d", number), usage.DrawIssue(ctx, root))
	assertEqual(t, usageIssueOpened(gh), []int{number})
	assertEqual(t, gh.LabelNames(number), setOf("night bot", config.LabelTracking))
	assertEqual(t, ctx.Store.Load().Obj(dashboard.UsageIssueKey).Get("issue"), number)
}

// A lost number adopts an issue with the title and marker opened by someone on the trust list,
// never a stranger's.
func TestUsageIssue_ALostNumberAdoptsATrustedIssueNeverAStrangers(t *testing.T) {
	gh, ctx := usageIssueCtx(t)
	body := "old\n\n" + usage.IssueMarker
	gh.AddIssue(40, fakes.Issue{Title: usage.IssueTitle, Body: body, User: fakes.Stranger})
	gh.AddIssue(41, fakes.Issue{Title: usage.IssueTitle, Body: body, User: fakes.Operator})
	assertIn(t, "rewrote #41", usage.DrawIssue(ctx, usageIssueRoot(t)))
	assertEqual(t, gh.Threads.Get(40).Str("body"), body)
	assertIn(t, "## 16. The raw data", gh.Threads.Get(41).Str("body"))
}

// A pin that fails (three issues pinned) is said and is not fatal; the next drawing pins it once
// a pin is free.
func TestUsageIssue_APinThatFailsIsTriedAgainWhenAPinIsFree(t *testing.T) {
	gh, ctx := usageIssueCtx(t)
	gh.Pinned = []string{"a", "b", "c"}
	root := usageIssueRoot(t)
	note := usage.DrawIssue(ctx, root)
	number := usageIssueOpened(gh)[0]
	assertEqual(t, note, fmt.Sprintf("opened #%d; could not pin it (422): no pin is free, so the next drawing tries "+
		"again", number))
	assertEqual(t, ctx.Store.Load().Obj(dashboard.UsageIssueKey).Get("pinned"), false)
	gh.Pinned = []string{"a", "b"}
	assertEqual(t, usage.DrawIssue(ctx, root), fmt.Sprintf("rewrote #%d and pinned it", number))
	assertEqual(t, gh.Pinned, []string{"a", "b", fmt.Sprintf("I_%d", number)})
	assertEqual(t, usage.DrawIssue(ctx, root), fmt.Sprintf("rewrote #%d", number))
}

// The status issue (#148) gains one line linking the usage issue, from the state file, and
// nothing else.
func TestUsageIssue_TheStatusIssueLinksItAndNothingElse(t *testing.T) {
	_, ctx := dashboardSetUp()
	before := dashboardRender(ctx, nil, "")
	assertNotIn(t, "📈", before)
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.Set(dashboard.UsageIssueKey, pyjson.New("issue", 2751, "pinned", true))
		return nil
	}, "state")
	after := dashboardRender(ctx, nil, "")
	line := "📈 How each subscription's usage works out over time, its charts and the tuning suggestions: #2751."
	assertEqual(t, strings.Count(after, line), 1)
	assertEqual(t, strings.Replace(after, line+"\n\n", "", 1), before)
}

// usageIssueMonth is a month of eleven subscriptions' history: a tick every five minutes, a reading
// every fifteen, a run every hour and a session every five, with values that move so every chart
// draws.
func usageIssueMonth() *usage.Facts {
	f := &usage.Facts{Now: usageAt}
	ids := []string{"claude-1", "claude-2", "claude-3", "claude-4", "claude-5", "claude-6", "claude-7", "claude-8",
		"agy", "gpt", "muse"}
	states := []string{"working", "ready", "idle", "capped", "outside-hours", "backing-off", "no-runner"}
	start := usageAt.Add(-30 * 24 * time.Hour)
	n := usage.N
	for k, id := range ids {
		for i := 0; start.Add(time.Duration(i) * 5 * time.Minute).Before(usageAt); i++ {
			at := start.Add(time.Duration(i) * 5 * time.Minute)
			reset := at.Truncate(5 * time.Hour).Add(5 * time.Hour)
			util := float64((i*3 + k*7) % 100)
			f.Ticks = append(f.Ticks, &usage.Tick{At: at, Subscription: id, Enabled: n(1), Suspended: n(0), InHours: n(1),
				State: states[(i/6+k)%len(states)], QueuedForIt: n(float64(i % 3)), HeldForIt: n(float64(i / 7 % 2)),
				LanesHeld: n(float64(i % 3)), LanesAllowed: n(2), RunsHeld: n(float64(i % 2)), Cap5hNow: n(70),
				Util5h: n(util), Resets5h: reset, LastHourNow: n(float64(i / 11 % 2))})
			if i%3 == 0 {
				f.Readings = append(f.Readings, &usage.Reading{ObservedAt: at, Subscription: id, RunID: fmt.Sprint(i / 12),
					Util5h: n(util), Resets5h: reset, Cap5h: n(70), Util7d: n(float64((i / 3) % 100)),
					Resets7d: at.Truncate(7 * 24 * time.Hour).Add(7 * 24 * time.Hour), Cap7d: n(90)})
			}
			if i%12 == 0 {
				f.Runs = append(f.Runs, &usage.Run{EndedAt: at, RunID: fmt.Sprintf("%s-%d", id, i), Subscription: id,
					Kind: usage.QueueKinds[i%4], Item: fmt.Sprint(i % 40), Outcome: []string{"built", "approved"}[i%2],
					StopReason: []string{"", "5-hour cap", "weekly cap", ""}[i%4], ModelMinutes: n(float64(i % 50)),
					TokensIn: n(1e5), TokensOut: n(2e4), TokensCached: n(1e6), USDReported: n(3.5), QueueWaitMin: n(float64(i % 90)),
					HandedOff: []string{"", "cap", "person", "other"}[i%4], Session: []string{"resumed", "fresh"}[i%2]})
			}
			if i%60 == 0 {
				f.Sessions = append(f.Sessions, &usage.Session{Subscription: id, StartedAt: at, ResetAt: at.Add(5 * time.Hour),
					CapMin: n(70), CapMax: n(100), PeakUtil: n(util), UnusedAtReset: n(100 - util), HitCapAt: at.Add(time.Hour),
					MinutesCappedQueued: n(30), OvershootMax: n(2), LastHourUsed: n(0)})
			}
		}
	}
	for i := 0; start.Add(time.Duration(i) * 5 * time.Minute).Before(usageAt); i++ {
		f.Pools = append(f.Pools, &usage.PoolTick{At: start.Add(time.Duration(i) * 5 * time.Minute), GithubSlotsUsed: n(float64(i % 8)),
			AWSSlotsUsed: n(float64(i % 6)), PlanLanesUsed: n(float64(i % 6)), GithubSlots: n(8), AWSSlots: n(6), PlanLanes: n(6),
			PlanMedium: n(float64(i % 4)), BuildHard: n(float64(i % 5)), ReviseUnrated: n(float64(i % 2)),
			ReviewMedium: n(float64(i % 3)), HeldForOwner: n(float64(i % 2))})
	}
	for _, id := range ids {
		for w := 1; w <= 4; w++ {
			reset := usageAt.Add(-time.Duration(w) * 7 * 24 * time.Hour)
			f.Weeks = append(f.Weeks, &usage.Week{Subscription: id, ResetAt: reset, PeakUtil: n(100),
				FirstCappedAt: reset.Add(-3 * 24 * time.Hour), DaysRefused: n(2)})
		}
	}
	for _, kind := range []string{"readings", "ticks", "pools", "sessions", "runs"} {
		f.Files = append(f.Files, kind+"-2026-10.csv", kind+"-2026-11.csv")
	}
	f.Files = append(f.Files, "weeks.csv", "backfill.json")
	return f
}

// A month of ticks for eleven subscriptions does not fit at full detail: the issue shrinks in its
// order and stays under GitHub's limit, with the raw links.
func TestUsageIssue_AMonthOfElevenSubscriptionsStaysUnderTheLimit(t *testing.T) {
	_, ctx := usageIssueCtx(t)
	f := usageIssueMonth()
	assertTrue(t, len(f.Ticks) > 90000, "a month of ticks")
	full := usage.Render(ctx, f, usage.Shrinks[0], 0)
	assertTrue(t, py.Len(full) > dashboard.MaxBody, "too big at full detail")
	body := usage.Fit(ctx, f)
	assertLessEqual(t, py.Len(body), dashboard.MaxBody)
	assertIn(t, "> Shrunk to keep under GitHub's limit on an issue's body: hourly charts at 2-hour points", body)
	for n := 1; n <= 16; n++ {
		usageIssueSection(t, body, n)
	}
	assertIn(t, "[raw](https://raw.githubusercontent.com/jgoetzmann/JackiOh/bot-usage/usage/sessions-2026-11.csv)", body)
	assertIn(t, "=IMPORTDATA(", body)
}

// When the last shrink is not enough, whole sections go from the bottom, the suggestions and the
// raw data aside, and a line says which.
func TestUsageIssue_PastTheLastShrinkSectionsGoFromTheBottom(t *testing.T) {
	_, ctx := usageIssueCtx(t)
	f := usageIssueFixture()
	last := usage.Render(ctx, f, usage.Shrinks[len(usage.Shrinks)-1], 0)
	patch(t, &dashboard.MaxBody, py.Len(last)-1)
	body := usage.Fit(ctx, f)
	assertLessEqual(t, py.Len(body), dashboard.MaxBody)
	assertIn(t, "> Left out to keep under GitHub's limit on an issue's body: section 14.", body)
	assertNotIn(t, "## 14. ", body)
	assertIn(t, "## 13. ", body)
	assertIn(t, "## 15. Tuning suggestions", body)
	assertIn(t, "sessions-2026-11.csv)", body)
	assertIn(t, "> Shrunk to keep under GitHub's limit on an issue's body: hourly charts at 2-hour points; "+
		"hourly charts at 3-hour points; hourly charts at 6-hour points; daily charts at 2-day points; section 4 for "+
		"the Claude accounts alone.", body)
	patch(t, &dashboard.MaxBody, 2000)
	tiny := usage.Fit(ctx, f)
	for _, n := range usage.Cuts() {
		assertNotIn(t, fmt.Sprintf("## %d. ", n), tiny)
	}
	assertIn(t, "## 15. ", tiny)
	assertIn(t, "## 16. ", tiny)
	assertEqual(t, usage.Cuts(), []int{14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1})
}

// The issue is drawn at a flush only: the loop's first tick and every 30 minutes after, never on
// the five-minute ticks between.
func TestUsageIssue_NothingIsDrawnOnATickThatDoesNotFlush(t *testing.T) {
	w := newUsageWorld(t)
	drawn := 0
	patch(t, &usage.Draw, func(ctx *hctx.Context, root string) string {
		drawn++
		return usage.DrawIssue(ctx, root)
	})
	w.tick()
	assertEqual(t, drawn, 1)
	opened := usageIssueOpened(w.gh)
	assertEqual(t, len(opened), 1)
	body := w.gh.Threads.Get(opened[0]).Str("body")
	for _, minutes := range []int{5, 10, 15, 20, 25} {
		w.at(usageAt.Add(time.Duration(minutes) * time.Minute))
		w.tick()
	}
	assertEqual(t, drawn, 1)
	assertEqual(t, w.gh.Threads.Get(opened[0]).Str("body"), body)
	w.at(usageAt.Add(30 * time.Minute))
	w.tick()
	assertEqual(t, drawn, 2)
	assertIn(t, "Last drawn at 2026-11-02 06:30 CST", w.gh.Threads.Get(opened[0]).Str("body"))
	assertTrue(t, slicesContainsPart(w.logs, fmt.Sprintf("usage: the usage issue: rewrote #%d", opened[0])))
	assertEqual(t, w.rec.Problem, "")
}

// slicesContainsPart is: some line of `lines` holds `part`.
func slicesContainsPart(lines []string, part string) bool {
	for _, line := range lines {
		if strings.Contains(line, part) {
			return true
		}
	}
	return false
}

// A bot that keeps no usage history draws no usage issue.
func TestUsageIssue_ABotWithoutAHistoryDrawsNothing(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	off := testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("usage_history", false)})
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{Cfg: off, At: usageAt})
	assertEqual(t, usage.DrawIssue(ctx, usageIssueRoot(t)), "")
	assertEqual(t, usageIssueOpened(gh), []int{})
	assertFalse(t, ctx.Store.Load().Has(dashboard.UsageIssueKey))
}

func TestUsageIssue_DumpForALook(t *testing.T) {
	if os.Getenv("USAGE_ISSUE_DUMP") == "" {
		t.Skip("set USAGE_ISSUE_DUMP to a file to write the fixture's body there")
	}
	_, ctx := usageIssueCtx(t)
	writeText(t, os.Getenv("USAGE_ISSUE_DUMP"), usage.Fit(ctx, usageIssueMonth()))
}
