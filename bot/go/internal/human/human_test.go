package human

import (
	"os"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata/python.json is bot/harness/human.py's answers to the same calls (the night bot's).
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

func TestPlanNeedsAsPython(t *testing.T) {
	// None of these plans has a Files to touch table, so no path is matched against `forbidden`.
	for _, c := range expected(t).List("plan_needs") {
		plan, want := c.([]any)[0].(string), c.([]any)[1].(string)
		if got := PlanNeeds(plan, []string{"nothing/"}); got != want {
			t.Errorf("PlanNeeds(%q) = %q, want %q", plan, got, want)
		}
	}
}

func TestTheHandOverAsPython(t *testing.T) {
	data := expected(t)
	if People() != data.Str("people") {
		t.Errorf("People: %s", People())
	}
	ask := data.List("ask")
	if got := Ask("  Set it up.  ", "jgoetzmann-bot", "build this", "", ""); got != ask[0] {
		t.Errorf("Ask\n%s\nwant\n%s", got, ask[0])
	}
	if got := Ask("", "bot", "merge this", " I kept the branch.", "ask again"); got != ask[1] {
		t.Errorf("Ask\n%s\nwant\n%s", got, ask[1])
	}
	if !LetsGo["bot:needs-plan"] {
		t.Error("the Needs plan stage comes off")
	}
}
