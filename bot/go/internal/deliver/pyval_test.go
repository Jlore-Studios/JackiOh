package deliver

import (
	"os"
	"slices"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata/python.json holds what Python 3.13 answered: `list({a, b})` for pairs of issue
// numbers.
func TestPyvalMatchesPython(t *testing.T) {
	raw, err := os.ReadFile("testdata/python.json")
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(raw)
	if err != nil {
		t.Fatal(err)
	}
	cases := pyjson.Obj(v)
	for _, e := range cases.List("set_order") {
		c := pyjson.List(e)
		a, b := c[0].(int), c[1].(int)
		var want []int
		for _, n := range pyjson.List(c[2]) {
			want = append(want, n.(int))
		}
		if got := setOrder(a, b); !slices.Equal(got, want) {
			t.Errorf("list({%d, %d}) = %v, want %v", a, b, got, want)
		}
	}
}

func TestRunnerNameIsPythonsWordClass(t *testing.T) {
	for name, want := range map[string]bool{
		"night-vm-claude-1": true, "ubuntu-latest": true, "box.local_2": true, "Ünïcødé": true,
		"": false, "has space": false, "a/b": false, strings.Repeat("a", 81): false, strings.Repeat("a", 80): true,
	} {
		if got := runnerName.MatchString(name); got != want {
			t.Errorf("fullmatch(%q) = %v, want %v", name, got, want)
		}
	}
}
