// #3675: the status loop keeps one clock for each kind of work, so slow work never holds up the
// issue: a tick of the sweep (with its deliveries), the statistics count and the runner gate run on
// goroutines of their own, and the loop's own thread alone writes the issue, every two minutes.
package tests

import (
	"fmt"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cardsets"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cigate"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cli"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	ghpkg "github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stats"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// statusLoopClock is the loop's fake clock, from 0: its background work (`cli.Background`) runs
// while the loop sleeps, the first of it (the first tick of the sweep) `slow` seconds after it
// started, the rest at once. GitHub's word on the token's calls left is none, until a test says.
func statusLoopClock(t *testing.T, slow float64) *float64 {
	now := 0.0
	type queued struct {
		run func()
		at  float64
	}
	var pending []queued
	first := true
	flush := func() {
		var later []queued
		for _, q := range pending {
			if q.at <= now {
				q.run()
			} else {
				later = append(later, q)
			}
		}
		pending = later
	}
	patch(t, &cli.Background, func(f func()) {
		at := now
		if first {
			at, first = now+slow, false
		}
		pending = append(pending, queued{f, at})
	})
	patch(t, &cli.Monotonic, func() float64 { return now })
	patch(t, &cli.Sleep, func(s float64) {
		flush()
		now += s
		flush()
	})
	patch(t, &cli.RateLeft, func(string) (ghpkg.RateLeft, bool) { return ghpkg.RateLeft{}, false })
	return &now
}

// A tick of the sweep that delivers for 400 seconds holds nothing up: the issue is rewritten every
// two minutes, its runners redrawn every minute between, and the gate ticks every 30 seconds all
// the while; the next tick of the sweep comes five minutes after that one ended, never two at once.
func TestStatusLoop_ASlowSweepNeverHoldsUpTheIssue(t *testing.T) {
	now := statusLoopClock(t, 400)
	var rewrites, sweeps, gates, runners []float64
	patch[dashboardUpdateFunc](t, &dashboard.Update, func(*hctx.Context, []string, bool, func() []string, bool,
		cardsets.Fetch) string {
		rewrites = append(rewrites, *now)
		return "rewrote #148"
	})
	patch(t, &cli.Sweep, func(*hctx.Context) []string {
		sweeps = append(sweeps, *now)
		return nil
	})
	patch(t, &cigate.Tick, func(*hctx.Context) string {
		gates = append(gates, *now)
		return ""
	})
	patch(t, &dashboard.RefreshRunners, func(*hctx.Context) string {
		runners = append(runners, *now)
		return "redrew #148's runners"
	})
	dashboardOffline(t, nil)
	capture(t, &cli.Stdout)
	assertEqual(t, cli.CmdDashboard(testsupport.MakeConfig(), &cli.Args{Every: 300, RewriteEvery: 120,
		RunnersEvery: 60, GateEvery: 30, ForSeconds: 720, Sweep: true}), 0)
	assertEqual(t, rewrites, []float64{0, 120, 240, 360, 480, 600, 720})
	assertEqual(t, sweeps, []float64{420, 720}) // the first ended at 420
	assertEqual(t, runners, []float64{60, 180, 300, 420, 540, 660})
	var every30 []float64
	for at := 30.0; at <= 720; at += 30 {
		every30 = append(every30, at)
	}
	assertEqual(t, gates, every30)
}

// While the bot's token has fewer than 1,500 calls left this hour, the issue is rewritten at the
// sweep's pace, and the loop says so; with the calls back, every two minutes again.
func TestStatusLoop_TheRewritesComeAtTheSweepsPaceWhileTheTokenRunsLow(t *testing.T) {
	now := statusLoopClock(t, 0)
	var rewrites []float64
	left := 1000
	patch[dashboardUpdateFunc](t, &dashboard.Update, func(*hctx.Context, []string, bool, func() []string, bool,
		cardsets.Fetch) string {
		rewrites = append(rewrites, *now)
		if *now >= 600 {
			left = 4000
		}
		return "rewrote #148"
	})
	patch(t, &cli.RateLeft, func(string) (ghpkg.RateLeft, bool) {
		return ghpkg.RateLeft{Remaining: left, Limit: 5000}, true
	})
	dashboardOffline(t, nil)
	out, _ := capture(t, &cli.Stdout)
	cli.CmdDashboard(testsupport.MakeConfig(), &cli.Args{Every: 300, RewriteEvery: 120, ForSeconds: 900})
	assertEqual(t, rewrites, []float64{0, 300, 600, 720, 840})
	assertIn(t, "rate: the bot's token has 1000 of 5000 calls left this hour: under 1500, so the issue is "+
		"rewritten every 300 seconds, the sweep's pace", out.String())
	assertIn(t, "rate: the bot's token has 4000 of 5000 calls left this hour\n", out.String())
}

// The statistics are counted on a clock of their own, every 10 minutes, and each rewrite draws the
// newest count: the first rewrite after a count says so, and none counts itself.
func TestStatusLoop_TheStatisticsAreCountedApartAndEachRewriteDrawsTheNewest(t *testing.T) {
	now := statusLoopClock(t, 0)
	var counted []float64
	patch(t, &dashboard.CountNow, func(ctx *hctx.Context, others []string, _ cardsets.Fetch) *dashboard.Count {
		counted = append(counted, *now)
		assertTrue(t, stats.Pulls != nil, "the loop keeps closed pull requests between counts")
		return &dashboard.Count{Blocks: map[int]string{}}
	})
	type rewrite struct {
		at          float64
		held, fresh bool
	}
	var rewrites []rewrite
	patch[dashboardUpdateFunc](t, &dashboard.Update, func(_ *hctx.Context, _ []string, withStats bool,
		_ func() []string, _ bool, _ cardsets.Fetch) string {
		assertTrue(t, withStats && dashboard.Counts != nil, "the rewrite draws the loop's count")
		held, fresh := dashboard.Counts.Newest()
		rewrites = append(rewrites, rewrite{*now, held != nil, fresh})
		return "rewrote #148"
	})
	dashboardOffline(t, nil)
	out, _ := capture(t, &cli.Stdout)
	cli.CmdDashboard(testsupport.MakeConfig(), &cli.Args{Every: 300, RewriteEvery: 120, ForSeconds: 720,
		Stats: true})
	assertEqual(t, counted, []float64{0, 600})
	// The first rewrite comes before the first count has ended, and keeps the block the issue holds.
	assertEqual(t, rewrites, []rewrite{{0, false, false}, {120, true, true}, {240, true, false},
		{360, true, false}, {480, true, false}, {600, true, false}, {720, true, true}})
	assertEqual(t, strings.Count(out.String(), "statistics: counted afresh"), 2)
	assertTrue(t, dashboard.Counts == nil && stats.Pulls == nil, "put back when the loop ends")
}

// A rewrite in the loop draws the statistics from the loop's newest count and never counts them
// itself; with no count yet it keeps the block the issue holds.
func TestStatusLoop_ARewriteDrawsTheLoopsCountAndNeverCounts(t *testing.T) {
	gh, ctx := statsWorld()
	dashboardUpdateStats(ctx, false) // opens the issue, with a block counted the old way
	number := dashboardOpened(gh)[0]
	kept := stats.Kept(gh.Threads.Get(number).Str("body"))
	assertTrue(t, kept != "", "a block")
	box := &dashboard.CountBox{}
	patch(t, &dashboard.Counts, box)
	restore := dashboardNoCount(t)
	later := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day.Add(time.Hour)})
	assertEqual(t, dashboardUpdateStats(later, false), fmt.Sprintf("rewrote #%d", number))
	assertEqual(t, stats.Kept(gh.Threads.Get(number).Str("body")), kept)
	block := stats.BlockStart + "\n## 📊 What it has done\n\nthe loop's count\n" + stats.BlockEnd
	blocks := map[int]string{}
	for _, shown := range dashboard.ShownSteps {
		blocks[shown] = block
	}
	box.Put(&dashboard.Count{Blocks: blocks})
	assertEqual(t, dashboardUpdateStats(later, false),
		fmt.Sprintf("rewrote #%d, with the statistics counted afresh", number))
	assertEqual(t, stats.Kept(gh.Threads.Get(number).Str("body")), block)
	restore()
}

// In the loop a count reads a closed pull request once, and again only once it has changed; an
// open one at every count. Outside the loop every count reads every one, as it always did.
func TestStatusLoop_ACountReadsAClosedPullRequestOnce(t *testing.T) {
	gh, ctx := statsWorld()
	for _, n := range []int{9, 10, 11, 12, 13} {
		gh.Threads.Get(n).Set("updated_at", "2026-09-29T16:00:00Z")
	}
	read := map[int]int{}
	gh.Patch.GetPull = func(number int) *pyjson.Object {
		read[number]++
		return gh.Own().GetPull(number)
	}
	stats.Collect(ctx)
	stats.Collect(ctx)
	assertEqual(t, read, map[int]int{9: 2, 10: 2, 11: 2, 12: 2, 13: 2})
	patch(t, &stats.Pulls, stats.NewPullCache())
	stats.Collect(ctx)
	stats.Collect(ctx)
	assertEqual(t, read, map[int]int{9: 3, 10: 3, 11: 3, 12: 3, 13: 4})
	gh.Threads.Get(9).Set("updated_at", "2026-09-30T00:00:00Z")
	facts := stats.Collect(ctx)
	assertEqual(t, read, map[int]int{9: 4, 10: 3, 11: 3, 12: 3, 13: 5})
	assertEqual(t, len(facts.Pulls), 5)
	assertEqual(t, stats.Pulls.Len(), 4)
}
