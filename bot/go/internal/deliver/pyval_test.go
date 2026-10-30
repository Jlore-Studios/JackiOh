package deliver

import (
	"math"
	"os"
	"slices"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata/python.json holds what Python 3.13 answered: `list({a, b})` for pairs of issue
// numbers, and int() and float() of JSON values.
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
	for _, e := range cases.List("numbers") {
		c := pyjson.List(e)
		in, wantInt, wantFloat := c[0], pyjson.List(c[1]), pyjson.List(c[2])
		n, ok := pyInt(in)
		if ok != (wantInt[0] == "ok") || (ok && n != wantInt[1].(int)) {
			t.Errorf("int(%#v) = %d, %v; want %v", in, n, ok, wantInt)
		}
		f, ok := pyFloat(in)
		if ok != (wantFloat[0] == "ok") {
			t.Errorf("float(%#v) ok = %v; want %v", in, ok, wantFloat)
			continue
		}
		if !ok {
			continue
		}
		switch w := wantFloat[1].(type) {
		case string:
			if (w == "nan") != math.IsNaN(f) || (w == "inf") != math.IsInf(f, 1) || (w == "-inf") != math.IsInf(f, -1) {
				t.Errorf("float(%#v) = %v, want %s", in, f, w)
			}
		default:
			if wf, _ := pyjson.AsFloat(w); wf != f {
				t.Errorf("float(%#v) = %v, want %v", in, f, wf)
			}
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
