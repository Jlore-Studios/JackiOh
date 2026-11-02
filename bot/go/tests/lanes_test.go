// The difficulty lanes: each role goes to the lane's weakest seat that meets its tier, and the
// builder is never the planner (#695).
//
// bot/tests/test_lanes.py, ported (#467).
package tests

import (
	"path/filepath"
	"strconv"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// lanesSeat is plan_mod.builder_seat(pool, providers, difficulty, avoid=..., audit_tier=...),
// its other keyword arguments at their defaults.
func lanesSeat(pool *providers.Pool, offered []*providers.Provider, difficulty, avoid, auditTier string) (*providers.Seat, string) {
	return plan.BuilderSeat(pool, offered, difficulty, false, avoid, auditTier, nil)
}

// lanesSeatID is (seat.provider.id, seat.tier), stopping the test where the seat is None (the
// Python's AttributeError).
func lanesSeatID(t *testing.T, seat *providers.Seat) []any {
	t.Helper()
	if seat == nil {
		t.Fatal("AttributeError: 'NoneType' object has no attribute 'provider'")
	}
	return []any{seat.Provider.ID, seat.Tier}
}

// One lane, one owner (#725): a medium item stays with the subscription that planned it when
// that one can build it.
func TestLanes_AMediumBuildStaysWithItsPlanner(t *testing.T) {
	pool := testsupport.MakeConfig().Pool
	claude := pool.Get("claude-3")
	muse, agy := pool.Get("muse"), pool.Get("agy")
	seat, note := lanesSeat(pool, []*providers.Provider{claude, muse, agy}, "medium", "muse", "")
	assertEqual(t, lanesSeatID(t, seat), []any{"muse", "medium"})
	assertIn(t, "one lane, one owner", note)
}

// The note names the subscription that keeps it as its owner (#2748): the one that planned it,
// or took it up last.
func TestLanes_ABuildKeepsItsPlannerWithANote(t *testing.T) {
	pool := testsupport.MakeConfig().Pool
	muse := pool.Get("muse")
	seat, note := lanesSeat(pool, []*providers.Provider{muse}, "medium", "muse", "")
	assertEqual(t, lanesSeatID(t, seat)[0], "muse")
	assertIn(t, "its owner `muse` keeps it", note)
}

// The owner keeps it only when it can build it (#725): with the planner not free, the lane's
// tier order picks.
func TestLanes_ABusyPlannerLosesItToTheLane(t *testing.T) {
	pool := testsupport.MakeConfig().Pool
	claude := pool.Get("claude-3")
	agy := pool.Get("agy")
	seat, _ := lanesSeat(pool, []*providers.Provider{claude, agy}, "medium", "muse", "")
	assertEqual(t, lanesSeatID(t, seat), []any{"agy", "medium"})
}

func TestLanes_NoPlannerMeansNoAvoidance(t *testing.T) {
	pool := testsupport.MakeConfig().Pool
	claude, muse := pool.Get("claude-3"), pool.Get("muse")
	seat, note := lanesSeat(pool, []*providers.Provider{claude, muse}, "medium", "", "")
	assertEqual(t, lanesSeatID(t, seat), []any{"muse", "medium"})
	assertEqual(t, note, "")
}

// Since #2748 nothing builds at weak, so an item stays with its medium planner, the weak one free
// or not, and one still rated `easy` (a record from before) is medium.
func TestLanes_TheWeakOneNeverTakesAnItemFromItsPlanner(t *testing.T) {
	pool := testsupport.MakeConfig(testsupport.Config{Machine: []string{"muse", "lite"}}).Pool
	muse, lite := pool.Get("muse"), pool.Get("lite")
	for _, difficulty := range []string{"medium", "easy"} {
		seat, note := lanesSeat(pool, []*providers.Provider{muse, lite}, difficulty, "muse", "")
		assertEqual(t, lanesSeatID(t, seat), []any{"muse", "medium"})
		assertIn(t, "one lane, one owner", note)
	}
	seat, _ := lanesSeat(pool, []*providers.Provider{lite}, "medium", "", "")
	assertTrue(t, seat == nil, seat)
}

// A revision that answers a final audit is its auditor's own fix (#2748, section 4): the audit
// made the auditor's subscription its owner, which fixes it on its seat of the audit's tier, though
// Muse comes first in the medium tier's order. Handed off (no owner now), it goes to the item's own
// tier, as any owner's work does (#725 kept it at the auditor's tier for anyone).
func TestLanes_AnAuditorFixesAtItsAuditsTierAndAHandoffAtTheItems(t *testing.T) {
	pool := testsupport.MakeConfig().Pool
	claude := pool.Get("claude-3")
	muse, agy := pool.Get("muse"), pool.Get("agy")
	seat, note := lanesSeat(pool, []*providers.Provider{muse, agy, claude}, "medium", "claude-3", "strong")
	assertEqual(t, lanesSeatID(t, seat), []any{"claude-3", "strong"})
	assertIn(t, "its owner `claude-3` keeps it: one lane, one owner", note)
	seat, _ = lanesSeat(pool, []*providers.Provider{muse, agy, claude}, "medium", "", "strong")
	assertEqual(t, lanesSeatID(t, seat), []any{"muse", "medium"})
	seat, _ = lanesSeat(pool, []*providers.Provider{muse, agy}, "medium", "", "medium")
	assertEqual(t, lanesSeatID(t, seat)[1], "medium")
}

func TestLanes_AMediumBuildPassesTheWeakOneByForGpt(t *testing.T) {
	// The tests' weak subscription is the one weak model, which builds nothing (#2748); Codex has
	// been medium since 2026-10-20.
	pool := testsupport.MakeConfig(testsupport.Config{Machine: []string{"muse", "lite"}}).Pool
	gpt, lite := pool.Get("gpt"), pool.Get("lite")
	seat, note := lanesSeat(pool, []*providers.Provider{gpt, lite}, "medium", "", "")
	assertEqual(t, lanesSeatID(t, seat), []any{"gpt", "medium"})
	assertEqual(t, note, "")
}

func TestLanes_AHardBuildTakesAStrongSeat(t *testing.T) {
	pool := testsupport.MakeConfig().Pool
	claude, muse := pool.Get("claude-3"), pool.Get("muse")
	seat, _ := lanesSeat(pool, []*providers.Provider{muse, claude}, "hard", "", "")
	assertEqual(t, lanesSeatID(t, seat), []any{"claude-3", "strong"})
}

// lanesCentral is a time on the bot's clock (America/Chicago, CDT here) on 2026-09-29, a Tuesday.
func lanesCentral(hour, minute int) time.Time {
	return time.Date(2026, 9, 29, hour+5, minute, 0, 0, time.UTC)
}

// lanesReading is a state whose `provider` last read its 5-hour session at `used`, resetting at
// `resets`, and its week resetting at `week` (zero: no weekly reading).
func lanesReading(provider string, at time.Time, used float64, resets, week time.Time) *pyjson.Object {
	usage := pyjson.New("five_hour", pyjson.New("utilization", used, "resets_at", clock.ISO(resets)),
		"observed_at", clock.ISO(at))
	if !week.IsZero() {
		usage.Set("seven_day", pyjson.New("utilization", 0.5, "resets_at", clock.ISO(week)))
	}
	return pyjson.New("providers", pyjson.New(provider, pyjson.New("usage", usage)))
}

// Each Claude account's lanes are decided when a run starts, from the 5-hour cap in force then,
// by `lanes_by_cap` in providers.json as it stands (#2748): above 50% up to 2, 50% or under 1, no
// 5-hour cap 2, and a session's last hour (while `last_hour` lifts the cap) above 50%. Muse keeps
// its own four, and agy and gpt their one.
func TestLanes_EachClaudeAccountsLanesFollowItsCapInForce(t *testing.T) {
	cfg := testsupport.MakeConfig(testsupport.Config{CommittedHours: true, Machine: testsupport.Machine})
	pool, zone := cfg.Pool, cfg.Timezone
	assertEqual(t, []any{pool.LanesByCap.CLI, pool.LanesByCap.Line, pool.LanesByCap.Low, pool.LanesByCap.High},
		[]any{"claude", 0.5, 1, 2})
	none := pyjson.New()
	night, noon, early := lanesCentral(22, 0), lanesCentral(12, 0), lanesCentral(5, 0)
	for _, c := range []struct {
		provider string
		at       time.Time
		lanes    int
		why      string
	}{
		{"claude-1", night, 2, "100% in its 21:00-07:00 hours"},
		{"claude-1", noon, 1, "40% outside them"},
		{"claude-2", noon, 2, "90%"},
		{"claude-2", night, 2, "90%"},
		{"claude-3", noon, 2, "no cap"},
		{"claude-4", noon, 2, "70% in its 03:00-15:00 window"},
		{"claude-4", night, 1, "50% outside it"},
		{"claude-6", noon, 2, "70% in its 03:00-15:00 window"},
		{"claude-6", night, 1, "50% outside it"},
		{"claude-5", noon, 1, "40%"},
		{"claude-5", night, 1, "40%"},
		{"claude-7", early, 2, "70% in its 01:00-10:00 window"},
		{"claude-7", noon, 1, "50% outside it"},
		{"claude-8", night, 2, "85%"},
		{"muse", noon, 4, "its own lanes"},
		{"agy", noon, 1, "its own lanes"},
		{"gpt", noon, 1, "its own lanes"},
	} {
		assertEqual(t, pool.LanesAt(pool.Get(c.provider), none, c.at, zone), c.lanes, c.provider, c.why)
	}
	// claude-5's late week: from Sunday 17:00 until its week resets, 80% of its session.
	sunday := time.Date(2026, 10, 5, 1, 0, 0, 0, time.UTC) // Sun 20:00 CDT
	late := lanesReading("claude-5", sunday, 0.2, sunday.Add(3*time.Hour), time.Date(2026, 10, 5, 9, 0, 0, 0, time.UTC))
	assertEqual(t, pool.LanesAt(pool.Get("claude-5"), late, sunday, zone), 2)
	// Its reading before the late week starts is held to its 40%.
	assertEqual(t, pool.LanesAt(pool.Get("claude-5"), late, sunday.Add(-4*time.Hour), zone), 1)
	// A session's last hour lifts claude-4's 50% outside its window: two lanes then, one before.
	resets := night.Add(30 * time.Minute)
	last := lanesReading("claude-4", night, 0.45, resets, time.Time{})
	assertEqual(t, pool.LanesAt(pool.Get("claude-4"), last, night, zone), 2)
	assertEqual(t, pool.LanesAt(pool.Get("claude-4"), last, night.Add(-time.Hour), zone), 1)
	// Somebody else spent the session (a reading past its cap): no last hour, so one lane.
	spent := lanesReading("claude-4", night, 0.7, resets, time.Time{})
	assertEqual(t, pool.LanesAt(pool.Get("claude-4"), spent, night, zone), 1)
	// No Claude account names its own lanes; each reads as the rule's most.
	for _, p := range pool.Ordered() {
		if p.CLI == "claude" {
			assertEqual(t, p.Lanes, 2, p.ID)
		}
	}
}

// The rule is the night bot's alone: Squishy's providers.json has none, so its account keeps its
// own two lanes, whatever its cap. A subscription the rule matches may not name its own lanes,
// and a rule must say which CLI, where the line is and the two counts.
func TestLanes_TheRuleIsTheNightBotsAndItsOwn(t *testing.T) {
	raw := loadsObject(t, readText(t, filepath.Join(testsupport.Root, ".squishy", "providers.json")))
	squishy := providers.Parse(raw)
	assertTrue(t, squishy.LanesByCap == nil)
	account := squishy.Ordered()[0]
	assertEqual(t, squishy.LanesAt(account, pyjson.New(), lanesCentral(12, 0), "America/Chicago"), account.Lanes)
	assertEqual(t, account.Lanes, 2)
	refused := func(change func(raw *pyjson.Object)) string {
		raw := testsupport.RawProviders()
		change(raw)
		var msg string
		if exc, ok := herr.Catch[*herr.ConfigError](func() { providers.Parse(raw) }); ok {
			msg = exc.Error()
		}
		return msg
	}
	assertIn(t, "providers.claude-8.lanes: lanes_by_cap decides the lanes of a claude subscription",
		refused(func(raw *pyjson.Object) { raw.Obj("providers").Obj("claude-8").Set("lanes", 2) }))
	assertIn(t, "lanes_by_cap: missing line", refused(func(raw *pyjson.Object) { raw.Obj("lanes_by_cap").Delete("line") }))
	assertIn(t, "lanes_by_cap: high (1) is under low (2)", refused(func(raw *pyjson.Object) {
		raw.Obj("lanes_by_cap").Set("low", 2).Set("high", 1)
	}))
	assertIn(t, "lanes_by_cap.cli: 'gemini' is not one of", refused(func(raw *pyjson.Object) {
		raw.Obj("lanes_by_cap").Set("cli", "gemini")
	}))
	assertIn(t, "lanes_by_cap.line: 1.5 is outside (0, 1]", refused(func(raw *pyjson.Object) {
		raw.Obj("lanes_by_cap").Set("line", 1.5)
	}))
}

// Losing a lane never stops a run already going (#2748): claude-4 holds two runs in its window
// (70%, two lanes); at 15:00 its cap drops to 50% outside it, one lane, and both runs go on: no
// run is requeued or relabelled, and it starts nothing more until its runs fit the one lane.
func TestLanes_ARunGoingIsNeverStoppedWhenTheLanesDrop(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := providersCtxFor(gh, providersCtx{At: lanesCentral(14, 30), Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN_4"),
		Machine: []string{}, CommittedHours: true})
	for _, n := range []int{80, 81, 82} {
		difficultyQueue(gh, ctx, n, true, "difficulty:hard")
	}
	for _, want := range []int{80, 81} {
		planned := flowMake(ctx)
		assertEqual(t, []any{planned.Get("action"), planned.Get("number"), planned.Get("provider")},
			[]any{"build", want, "claude-4"})
		providersRun(gh, strconv.Itoa(want), "in_progress")
		flowUpdateItem(ctx, want, pyjson.New("run_id", strconv.Itoa(want)))
	}
	assertEqual(t, flowMake(ctx).Get("action"), "none") // both lanes held
	testsupport.ClockOf(ctx).At = lanesCentral(15, 30)
	lanes := plan.ReadLanes(ctx, ctx.Store.Load())
	claude4 := ctx.Cfg.Pool.Get("claude-4")
	assertEqual(t, []any{lanes.LanesOf(claude4), lanes.Used(claude4)}, []any{1, 2})
	planned := flowMake(ctx)
	assertEqual(t, planned.Get("action"), "none")
	assertIn(t, "`claude-4` is busy", planned.Get("reason"))
	for _, n := range []int{80, 81} {
		assertEqual(t, gh.LabelNames(n), setOf(config.LabelBuilding, config.LabelPlanned, "difficulty:hard"), n)
	}
	assertEqual(t, plan.HousekeepingDue(ctx, ctx.Store.Load()), "")
	// One run ends (its deliver takes its live label off): one run on one lane, still full.
	gh.RemoveLabel(80, config.LabelBuilding)
	gh.Runs.Delete("80")
	assertEqual(t, flowMake(ctx).Get("action"), "none")
	// The other ends too: its one lane takes #82.
	gh.RemoveLabel(81, config.LabelBuilding)
	gh.Runs.Delete("81")
	planned = flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("number"), planned.Get("provider")},
		[]any{"build", 82, "claude-4"})
}
