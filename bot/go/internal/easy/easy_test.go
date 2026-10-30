package easy

import (
	"os"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata/python.json is bot/harness/easy.py's answers to the same calls.
func expected(t *testing.T) *pyjson.Object {
	t.Helper()
	data, err := os.ReadFile("testdata/python.json")
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(data)
	if err != nil {
		t.Fatal(err)
	}
	return v.(*pyjson.Object)
}

func TestRuleIsPythons(t *testing.T) {
	if want := expected(t).Str("rule"); Rule != want {
		t.Errorf("Rule\n%q\nwant\n%q", Rule, want)
	}
}

func TestPlanPathsAsPython(t *testing.T) {
	for _, c := range expected(t).List("plan_paths") {
		plan, want := c.([]any)[0].(string), c.([]any)[1]
		if got := PlanPaths(plan); !pyjson.Equal(pyjson.ListOf(got), want) {
			t.Errorf("PlanPaths(%q) = %q, want %v", plan, got, want)
		}
	}
}

func TestUncertainWhyAsPython(t *testing.T) {
	for _, c := range expected(t).List("uncertain") {
		why, want := c.([]any)[0].(string), c.([]any)[1].(bool)
		if got := UncertainWhy(why); got != want {
			t.Errorf("UncertainWhy(%q) = %v, want %v", why, got, want)
		}
	}
}

func TestPlanBreachOnSize(t *testing.T) {
	rule := config.NewEasyRule()
	if got := PlanBreach("no table", rule, nil, nil); got != "line 8: the plan has no **Files to touch** table, so nothing shows it is small" {
		t.Error(got)
	}
	table := "## Files to touch\n"
	for _, p := range []string{"a/1", "a/2", "a/3", "a/4", "a/5", "a/6", "a/7", "a/8", "a/9"} {
		table += "| `" + p + "` |\n"
	}
	if got := PlanBreach(table, rule, nil, nil); got != "line 1: the plan touches 9 files, close to the limit of 10, so borderline easy is medium" {
		t.Error(got)
	}
	rule.MaxFiles = 4
	if got := PlanBreach(table, rule, nil, nil); got != "line 1: the plan touches 9 files, over 4" {
		t.Error(got)
	}
}
