package plan

import (
	"slices"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/identity"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

func TestLanesCount(t *testing.T) {
	lanes := &Lanes{Limit: 3, Held: map[int]string{4: "a", 5: "a", 6: "b", 0: "c"},
		Planning: map[int]bool{5: true, 9: true}, PlanLimit: 2, MachineLimit: 2,
		Machine: map[string]bool{"b": true}, Shared: 1, Filling: map[int]bool{6: true}}
	if got := lanes.Count("a"); got != 1 {
		t.Errorf("Count(a) = %d, want 1 (its planning run aside)", got)
	}
	if got := lanes.PlanningBy("a"); got != 1 {
		t.Errorf("PlanningBy(a) = %d", got)
	}
	if got := lanes.PlanningBy(""); got != 0 {
		t.Errorf("PlanningBy(\"\") = %d: an item planning but held by none is no one's", got)
	}
	if got := (lanes.FillingBy("b")); got != 1 {
		t.Errorf("FillingBy(b) = %d", got)
	}
	if got := lanes.AllBy("a"); got != 2 {
		t.Errorf("AllBy(a) = %d", got)
	}
	// GitHub: 3 - 1 shared - 3 hosted (a, a, c); the machine: 2 - 1 (b).
	if lanes.Free() != 0 || lanes.MachineFree() != 1 {
		t.Errorf("Free, MachineFree = %d, %d", lanes.Free(), lanes.MachineFree())
	}
	if lanes.RoomFor("a") || !lanes.RoomFor("b") {
		t.Error("RoomFor: GitHub is full, the machine is not")
	}
	if lanes.PlanFree() != 0 || !lanes.AnyRoom() {
		t.Errorf("PlanFree = %d, AnyRoom = %v", lanes.PlanFree(), lanes.AnyRoom())
	}
	if busy := lanes.Busy(); len(busy) != 3 || !busy["c"] {
		t.Errorf("Busy = %v", busy)
	}
}

func TestLanesWithRunAndDescribe(t *testing.T) {
	empty := &Lanes{Limit: 3}
	if empty.Describe() != "none" || empty.Free() != 3 || empty.Count("a") != 0 {
		t.Errorf("a zero Lanes: %q, free %d", empty.Describe(), empty.Free())
	}
	after := empty.WithRun(12, "muse", "plan", false).WithRun(0, "gpt", "suggest", false).
		WithRun(7, "muse", "build", true)
	if len(empty.Held) != 0 {
		t.Errorf("WithRun changed its receiver: %v", empty.Held)
	}
	if got := after.Describe(); got != "a survey with `gpt`, #7 with `muse`, #12 with `muse` (planning)" {
		t.Errorf("Describe = %q", got)
	}
	if after.PlanningBy("muse") != 1 || after.FillingBy("muse") != 1 || after.Count("muse") != 1 {
		t.Errorf("planning %d, filling %d, count %d", after.PlanningBy("muse"), after.FillingBy("muse"),
			after.Count("muse"))
	}
}

func TestRankedGoesByTheBalancerThenTheUsageOrder(t *testing.T) {
	pool := &providers.Pool{Priority: []string{"claude-3", "claude-7", "claude-1"}}
	seat := func(id string) *providers.Seat {
		return &providers.Seat{Provider: &providers.Provider{ID: id}, Tier: "strong"}
	}
	seats := []*providers.Seat{seat("claude-7"), seat("claude-3"), seat("claude-1"), seat("muse")}
	ids := func(seats []*providers.Seat) []string {
		var out []string
		for _, s := range seats {
			out = append(out, s.Provider.ID)
		}
		return out
	}
	if got := ids(Ranked(pool, seats, nil)); !slices.Equal(got, []string{"claude-3", "claude-7", "claude-1", "muse"}) {
		t.Errorf("Ranked = %v", got)
	}
	spread := Balance{"claude-3": {-2, 1}, "claude-7": {-9, 0}, "claude-1": {-9, 1}}
	// muse has no place in the balancer: LEVEL, (0, 0), after every negative one.
	if got := ids(Ranked(pool, seats, spread)); !slices.Equal(got, []string{"claude-7", "claude-1", "claude-3", "muse"}) {
		t.Errorf("Ranked with a spread = %v", got)
	}
}

func TestLoopRef(t *testing.T) {
	run := pyjson.New("display_title", "bot-night 0a1b2c3d-1761800000 on night-vm-muse")
	if got := LoopRef(run); got != "0a1b2c3d-1761800000" {
		t.Errorf("LoopRef = %q", got)
	}
	for _, title := range []any{"bot-night 0A1B2C3D-1761800000 on x", "bot-night 0a1b2c3d-17618 on x", nil} {
		if got := LoopRef(pyjson.New("display_title", title)); got != "" {
			t.Errorf("LoopRef(%v) = %q, want none", title, got)
		}
	}
}

func TestNothingAndVaultPath(t *testing.T) {
	if got := pyjson.Dumps(Nothing("nothing is queued")); got != `{"action": "none", "reason": "nothing is queued"}` {
		t.Errorf("Nothing = %s", got)
	}
	if got := VaultPath("claude-3"); got != "vault/claude-3.enc" {
		t.Errorf("VaultPath = %q", got)
	}
}

func TestSplitBuilders(t *testing.T) {
	saved := Others
	t.Cleanup(func() { Others = saved })
	Others = nil
	if got := SplitBuilders("split-bot"); got != "me" {
		t.Errorf("with no other bot: %q", got)
	}
	Others = []*identity.Other{{Name: "Squishy", Login: "squishy-squooby"}}
	if got := SplitBuilders("split-bot"); got != "Squishy (@squishy-squooby)" {
		t.Errorf("split-bot: %q", got)
	}
	if got := SplitBuilders("split"); got != "me" {
		t.Errorf("split: %q", got)
	}
}

func TestAssignmentFillsInAndSelfCheck(t *testing.T) {
	muse := &providers.Provider{ID: "muse", RunsOn: "night-vm-muse",
		FillIn: &providers.FillIn{RunsOn: "night-vm-fill", Of: []string{"gpt"}}}
	own := &Assignment{Provider: muse, RunsOn: "night-vm-muse"}
	fill := &Assignment{Provider: muse, RunsOn: "night-vm-fill",
		Build: &providers.Seat{Provider: muse, SelfCheck: true}}
	if own.FillsIn() || !fill.FillsIn() {
		t.Error("FillsIn: only the fill-in runners fill a lane")
	}
	if own.SelfCheck() || !fill.SelfCheck() {
		t.Error("SelfCheck follows the builder's seat")
	}
}
