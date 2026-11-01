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
	if !Met([]string{"medium"}, "easy") || Met([]string{"medium", "weak"}, "hard") || !Met([]string{"strong"}, "hard") {
		t.Error("met")
	}
	if !Reachable(nil, []string{"medium"}, "easy") || Reachable(nil, []string{"medium"}, "medium") {
		t.Error("reachable")
	}
}
