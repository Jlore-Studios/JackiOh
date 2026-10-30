package py

import (
	"os"
	"reflect"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata.json holds what Python 3.13 answered for each case.
func TestMatchesPython(t *testing.T) {
	raw, err := os.ReadFile("testdata.json")
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(raw)
	if err != nil {
		t.Fatal(err)
	}
	c := pyjson.Obj(v)
	for _, e := range c.List("round") {
		p := pyjson.List(e)
		x, _ := pyjson.AsFloat(p[0])
		if got, want := Round(x), p[1].(int); got != want {
			t.Errorf("round(%v) = %d, want %d", x, got, want)
		}
	}
	for _, e := range c.List("roundn") {
		p := pyjson.List(e)
		x, _ := pyjson.AsFloat(p[0])
		want, _ := pyjson.AsFloat(p[2])
		if got := RoundN(x, p[1].(int)); got != want {
			t.Errorf("round(%v, %v) = %v, want %v", x, p[1], got, want)
		}
	}
	for _, e := range c.List("floordiv") {
		p := pyjson.List(e)
		a, b := p[0].(int), p[1].(int)
		if FloorDiv(a, b) != p[2].(int) || Mod(a, b) != p[3].(int) {
			t.Errorf("%d // %d, %% : got %d %d, want %v %v", a, b, FloorDiv(a, b), Mod(a, b), p[2], p[3])
		}
	}
	for _, e := range c.List("splitlines") {
		p := pyjson.List(e)
		got := Splitlines(p[0].(string))
		want := pyjson.Strings(p[1])
		if len(got) != len(want) || (len(got) > 0 && !reflect.DeepEqual(got, want)) {
			t.Errorf("splitlines(%q) = %q, want %q", p[0], got, want)
		}
	}
	for _, e := range c.List("thousands") {
		p := pyjson.List(e)
		if got := Thousands(p[0].(int)); got != p[1].(string) {
			t.Errorf("thousands(%v) = %s, want %s", p[0], got, p[1])
		}
	}
	for _, e := range c.List("fnmatch") {
		p := pyjson.List(e)
		if got := Fnmatch(p[0].(string), p[1].(string)); got != p[2].(bool) {
			t.Errorf("fnmatch(%q, %q) = %v, want %v", p[0], p[1], got, p[2])
		}
	}
	for _, e := range c.List("str") {
		p := pyjson.List(e)
		val, _ := pyjson.LoadsString(p[0].(string))
		if got := Str(val); got != p[1].(string) {
			t.Errorf("str(%s) = %s, want %s", p[0], got, p[1])
		}
	}
}

func TestSlice(t *testing.T) {
	s := "é—ab😀"
	for _, c := range []struct {
		start, end int
		want       string
	}{{0, 2, "é—"}, {-2, 5, "b😀"}, {3, 100, "b😀"}, {-100, 1, "é"}, {4, 2, ""}} {
		if got := Slice(s, c.start, c.end); got != c.want {
			t.Errorf("Slice(%d, %d) = %q, want %q", c.start, c.end, got, c.want)
		}
	}
	if Len(s) != 5 || SliceTo(s, -1) != "é—ab" || SliceFrom(s, -1) != "😀" {
		t.Error("Len, SliceTo or SliceFrom")
	}
	if Ljust("é", 3) != "é  " || Rjust("é", 3) != "  é" {
		t.Error("Ljust or Rjust")
	}
}
