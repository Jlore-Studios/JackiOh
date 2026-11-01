package issueplan

import (
	"crypto/sha256"
	"encoding/hex"
	"os"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

func digest(text string) []any {
	sum := sha256.Sum256([]byte(text))
	return []any{hex.EncodeToString(sum[:]), py.Len(text), py.SliceFrom(text, -120)}
}

// testdata.json holds what Python 3.13 answered (bot/harness/issueplan.py); a long result is kept
// as its SHA-256, length and tail.
func TestMatchesPython(t *testing.T) {
	raw, err := os.ReadFile("testdata.json")
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(raw)
	if err != nil {
		t.Fatal(err)
	}
	data := pyjson.Obj(v)
	for _, e := range data.List("without") {
		c := pyjson.List(e)
		if got := WithoutPlan(pyjson.Str(c[0])); got != c[1] {
			t.Errorf("without_plan(%q) = %q, want %q", c[0], got, c[1])
		}
	}
	for _, e := range data.List("plan_of") {
		c := pyjson.List(e)
		if got := PlanOf(pyjson.Str(c[0])); got != c[1] {
			t.Errorf("plan_of(%q) = %q, want %q", c[0], got, c[1])
		}
	}
	for _, e := range data.List("with") {
		c := pyjson.List(e)
		plan := strings.Repeat(c[1].(string), c[2].(int))
		got := digest(WithPlan(pyjson.Str(c[0]), plan, "Opus", c[3].(string)))
		if !pyjson.Equal(got, c[4:]) {
			t.Errorf("with_plan(%q, %q×%v, %q) = %v, want %v", c[0], c[1], c[2], c[3], got, c[4:])
		}
	}
	if got := digest(WithPlan(strings.Repeat("t", 65000), "plan", "who", "")); !pyjson.Equal(got, data.Get("big")) {
		t.Errorf("with_plan(big) = %v, want %v", got, data.Get("big"))
	}
}
