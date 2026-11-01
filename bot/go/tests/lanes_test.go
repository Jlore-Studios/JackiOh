// The difficulty lanes: each role goes to the lane's weakest seat that meets its tier, and the
// builder is never the planner (#695).
//
// bot/tests/test_lanes.py, ported (#467).
package tests

import (
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
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

func TestLanes_ABuildKeepsItsPlannerWithANote(t *testing.T) {
	pool := testsupport.MakeConfig().Pool
	muse := pool.Get("muse")
	seat, note := lanesSeat(pool, []*providers.Provider{muse}, "medium", "muse", "")
	assertEqual(t, lanesSeatID(t, seat)[0], "muse")
	assertIn(t, "its planner `muse` keeps it", note)
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

// An easy item goes weak-first even when its medium planner is free (#725): the planner must
// not spend its lane on weak work.
func TestLanes_AnEasyBuildStillAvoidsItsPlanner(t *testing.T) {
	pool := testsupport.MakeConfig(testsupport.Config{Machine: []string{"muse", "lite"}}).Pool
	muse, lite := pool.Get("muse"), pool.Get("lite")
	seat, _ := lanesSeat(pool, []*providers.Provider{muse, lite}, "easy", "muse", "")
	assertEqual(t, lanesSeatID(t, seat)[0], "lite")
}

// A revision that answers a final audit stays at the auditor's tier or above (#725), so the
// auditor fixes and pushes it.
func TestLanes_ARevisionAnsweringAnAuditStaysAtItsTier(t *testing.T) {
	pool := testsupport.MakeConfig().Pool
	claude := pool.Get("claude-3")
	muse, agy := pool.Get("muse"), pool.Get("agy")
	seat, _ := lanesSeat(pool, []*providers.Provider{muse, agy, claude}, "medium", "", "strong")
	assertEqual(t, lanesSeatID(t, seat), []any{"claude-3", "strong"})
	seat, _ = lanesSeat(pool, []*providers.Provider{muse, agy}, "easy", "", "medium")
	assertEqual(t, lanesSeatID(t, seat)[1], "medium")
}

func TestLanes_AnEasyBuildTriesTheWeakOneBeforeGpt(t *testing.T) {
	// The tests' weak subscription is the one weak model; Codex has been medium since
	// 2026-10-20.
	pool := testsupport.MakeConfig(testsupport.Config{Machine: []string{"muse", "lite"}}).Pool
	gpt, lite := pool.Get("gpt"), pool.Get("lite")
	seat, _ := lanesSeat(pool, []*providers.Provider{gpt, lite}, "easy", "", "")
	assertEqual(t, lanesSeatID(t, seat)[0], "lite")
}

func TestLanes_AHardBuildTakesAStrongSeat(t *testing.T) {
	pool := testsupport.MakeConfig().Pool
	claude, muse := pool.Get("claude-3"), pool.Get("muse")
	seat, _ := lanesSeat(pool, []*providers.Provider{muse, claude}, "hard", "", "")
	assertEqual(t, lanesSeatID(t, seat), []any{"claude-3", "strong"})
}
