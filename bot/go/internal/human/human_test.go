package human

import (
	"os"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata/python.json is bot/harness/human.py's answers to the same calls (the night bot's), and
// the hand-over's summary (`ask`) as #3009 changed it.
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

// #3352: a planner that writes "none" and then an aside needs nobody, while a line that asks for
// something, whatever its punctuation, still hands the issue over.
func TestPlanNeedsReadsNoneBeforeAnAside(t *testing.T) {
	nobodyLines := []string{
		"**Human required:** none. One plan, one pull request, no split (see Open questions).",
		"**Human required:** None (see Open questions).",
		"**Human required:** not needed; the bot can build all of it.",
		"**Human required:** N/A - every file is under apps/web/.",
		"**Human required:** no — nothing outside the bot's paths.",
	}
	for _, line := range nobodyLines {
		if got := PlanNeeds(line+"\n\n## Goal\nx", []string{"nothing/"}); got != "" {
			t.Errorf("PlanNeeds(%q) = %q, want no person", line, got)
		}
	}
	asks := map[string]string{
		"**Human required:** set the secret FOO. Then queue it again.":    "set the secret FOO. Then queue it again.",
		"**Human required:** no secrets; a person must flip the setting.": "no secrets; a person must flip the setting.",
		"**Human required:** nothing builds until the dashboard is set.":  "nothing builds until the dashboard is set.",
	}
	for line, want := range asks {
		if got := PlanNeeds(line+"\n\n## Goal\nx", []string{"nothing/"}); got != want {
			t.Errorf("PlanNeeds(%q) = %q, want %q", line, got, want)
		}
	}
}

func TestTheHandOverAsPython(t *testing.T) {
	data := expected(t)
	if People() != data.Str("people") {
		t.Errorf("People: %s", People())
	}
	ask := data.List("ask")
	if got := Ask("  Set it up.  ", "jgoetzmann-bot", "build this", "", "", ""); got != ask[0] {
		t.Errorf("Ask\n%s\nwant\n%s", got, ask[0])
	}
	if got := Ask("", "bot", "merge this", " I kept the branch.", "branch `b`, at `abc`", "ask again"); got != ask[1] {
		t.Errorf("Ask\n%s\nwant\n%s", got, ask[1])
	}
	if !LetsGo["bot:needs-plan"] {
		t.Error("the Needs plan stage comes off")
	}
}
