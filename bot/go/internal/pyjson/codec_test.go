package pyjson

import (
	"os"
	"testing"
)

// testdata/python.json is what Python 3.13's json.dumps wrote for each case and option, and
// repr() for each float: the Go side must write the same bytes.
func TestDumpsMatchesPython(t *testing.T) {
	raw, err := os.ReadFile("testdata/python.json")
	if err != nil {
		t.Fatal(err)
	}
	top, err := Loads(raw)
	if err != nil {
		t.Fatal(err)
	}
	for i, c := range Objs(Obj(top).Get("cases")) {
		v, err := LoadsString(c.Str("in"))
		if err != nil {
			t.Fatalf("case %d: %v", i, err)
		}
		for name, opts := range map[string]Options{
			"dumps":       {},
			"indent2":     {Indent: 2, Indented: true},
			"sort_indent": {Indent: 2, Indented: true, SortKeys: true},
			"sort":        {SortKeys: true},
			"noascii":     {NoASCII: true},
		} {
			if got, want := DumpsWith(v, opts), c.Str(name); got != want {
				t.Errorf("case %d %s:\n got %s\nwant %s", i, name, got, want)
			}
		}
	}
	for _, pair := range List(Obj(top).Get("floats")) {
		p := List(pair)
		f, _ := AsFloat(p[1])
		if got := FloatRepr(f); got != p[0].(string) {
			t.Errorf("FloatRepr(%v) = %s, want %s", f, got, p[0])
		}
	}
}

func TestObjectOrderAndNil(t *testing.T) {
	o := New("b", 1, "a", 2)
	o.Set("b", 3)
	o.Set("c", 4)
	if got := Dumps(o); got != `{"b": 3, "a": 2, "c": 4}` {
		t.Fatal(got)
	}
	o.Delete("a")
	if got := Dumps(o); got != `{"b": 3, "c": 4}` {
		t.Fatal(got)
	}
	var none *Object
	if none.Obj("x").Str("y") != "" || none.Len() != 0 || none.Has("x") {
		t.Fatal("a nil object should read as empty")
	}
	v, _ := LoadsString(`{"n": 1, "f": 1.0, "dup": 1, "x": 2, "dup": 3}`)
	m := Obj(v)
	if _, ok := m.Get("n").(int); !ok {
		t.Fatal("an integer literal should load as int")
	}
	if _, ok := m.Get("f").(float64); !ok {
		t.Fatal("a float literal should load as float64")
	}
	if Dumps(m) != `{"n": 1, "f": 1.0, "dup": 3, "x": 2}` {
		t.Fatal(Dumps(m))
	}
	if !Equal(m, DeepCopy(m)) || Equal(m, New()) {
		t.Fatal("Equal")
	}
}
