package failures

import (
	"os"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata.json holds what Python 3.13 answered (bot/harness/failures.py).
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
	for i, c := range pyjson.Objs(data.Get("why")) {
		cycles := pyjson.Objs(c.Get("cycles"))
		if got := Why(cycles, c.Int("limit"), c.Str("reason")); got != c.Str("why") {
			t.Errorf("case %d: why =\n%s\nwant\n%s", i, got, c.Str("why"))
		}
		if got := Stuck(cycles, c.Int("limit")); got != c.Bool("stuck") {
			t.Errorf("case %d: stuck = %v", i, got)
		}
	}
	for _, e := range data.List("cells") {
		c := pyjson.List(e)
		if got := Cell(c[0], c[1].(int)); got != c[2] {
			t.Errorf("_cell(%v, %v) = %q, want %q", c[0], c[1], got, c[2])
		}
	}
	for _, e := range data.List("sums") {
		c := pyjson.List(e)
		var xs []float64
		for _, x := range pyjson.List(c[0]) {
			f, _ := pyjson.AsFloat(x)
			xs = append(xs, f)
		}
		want, _ := pyjson.AsFloat(c[1])
		if got := py.Sum(xs); got != want {
			t.Errorf("sum(%v) = %v, want %v", xs, got, want)
		}
	}
}
