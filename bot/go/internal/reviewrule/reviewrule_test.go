package reviewrule

import (
	"reflect"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// The expected values are what Python 3.13 answered (bot/harness/review_rule.py).
func TestMatchesPython(t *testing.T) {
	who := Who([][2]string{{"gemini", "medium"}, {"muse", "medium"}, {"gemini", "medium"},
		{"claude", "strong"}, {"muse", "medium"}, {"muse", "medium"}})
	if want := "`gemini` (medium) twice, `muse` (medium) 3 times, `claude` (strong)"; who != want {
		t.Errorf("who = %q, want %q", who, want)
	}
	tier := func(f string) string {
		return map[string]string{"claude": "strong", "gemini": "medium"}[f] + map[bool]string{true: "weak"}[f != "claude" && f != "gemini"]
	}
	cases := []struct {
		votes string
		want  [][2]string
	}{
		{`{"reviews": [{"family": "claude"}, {"family": "gemini", "tier": "strong"}, 5, {"tier": "medium"}]}`,
			[][2]string{{"claude", "strong"}, {"gemini", "strong"}, {"", "medium"}}},
		{`{"approvals": ["claude", "gemini", "x"], "tiers": {"x": "medium", "claude": ""}}`,
			[][2]string{{"claude", "strong"}, {"gemini", "medium"}, {"x", "medium"}}},
		{`{"approvals": "ab"}`, [][2]string{{"a", "weak"}, {"b", "weak"}}},
	}
	for _, c := range cases {
		votes, _ := pyjson.LoadsString(c.votes)
		if got := Approvals(pyjson.Obj(votes), tier); !reflect.DeepEqual(got, c.want) {
			t.Errorf("approvals(%s) = %v, want %v", c.votes, got, c.want)
		}
	}
}

// #2748: a medium approval ships a medium item, a strong one either; a weak one nothing.
func TestMetAndReachable(t *testing.T) {
	if !Met([]string{"medium"}, "medium") || Met([]string{"medium", "weak"}, "hard") ||
		!Met([]string{"strong"}, "hard") || !Met([]string{"strong"}, "medium") || Met([]string{"weak"}, "medium") {
		t.Error("met")
	}
	if !Reachable(nil, []string{"medium"}, "medium") || Reachable(nil, []string{"medium"}, "hard") ||
		!Reachable(nil, []string{"strong"}, "hard") || Reachable(nil, []string{"weak"}, "medium") {
		t.Error("reachable")
	}
}

// #2748: only an approval from a subscription other than the builder's counts; votes from before
// it, which name no builder subscription, count their strong approvals alone.
func TestCountedByTheBuildersSubscription(t *testing.T) {
	tier := func(string) string { return "weak" }
	cases := []struct {
		votes string
		want  []string
		tiers []string
	}{
		{`{"built_by": "muse", "reviews": [{"family": "muse", "tier": "medium", "provider": "muse"},
			{"family": "gemini", "tier": "medium", "provider": "agy"}]}`, []string{"medium"}, []string{"medium", "strong"}},
		{`{"built_by": "claude-7", "reviews": [{"family": "claude", "tier": "strong", "provider": "claude-7"},
			{"family": "claude", "tier": "medium", "provider": "claude-2"}]}`, []string{"medium"}, []string{"medium", "strong"}},
		{`{"built_by": "", "reviews": [{"family": "muse", "tier": "medium", "provider": "muse"}]}`,
			[]string{"medium"}, []string{"medium", "strong"}},
		{`{"builder": "muse", "reviews": [{"family": "muse", "tier": "medium"}, {"family": "claude", "tier": "strong"}]}`,
			[]string{"strong"}, []string{"strong"}},
	}
	for _, c := range cases {
		votes, err := pyjson.LoadsString(c.votes)
		if err != nil {
			t.Fatal(err)
		}
		if got := Counted(pyjson.Obj(votes), tier); !reflect.DeepEqual(got, c.want) {
			t.Errorf("counted(%s) = %v, want %v", c.votes, got, c.want)
		}
		if got := Tiers(pyjson.Obj(votes), "medium"); !reflect.DeepEqual(got, c.tiers) {
			t.Errorf("tiers(%s) = %v, want %v", c.votes, got, c.tiers)
		}
		if got := Tiers(pyjson.Obj(votes), "hard"); !reflect.DeepEqual(got, []string{"strong"}) {
			t.Errorf("tiers(%s, hard) = %v", c.votes, got)
		}
	}
}
