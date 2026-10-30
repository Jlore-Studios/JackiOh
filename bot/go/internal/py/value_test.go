package py

import (
	"errors"
	"math"
	"slices"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// raised is what fn panicked with, as an error (herr.Try, which imports py).
func raised(fn func()) (err error) {
	defer func() {
		if r := recover(); r != nil {
			err = r.(error)
		}
	}()
	fn()
	return nil
}

// caseValue is a case's input: a JSON value, or ["float", "nan" | "inf" | "-inf"].
func caseValue(c []any) any {
	if c[0] == "float" {
		return map[string]float64{"nan": math.NaN(), "inf": math.Inf(1), "-inf": math.Inf(-1)}[c[1].(string)]
	}
	return c[1]
}

// outcome is what a call answered as Python's helper wrote it: ["ok", value] (a float as its
// repr) or [exception class, str(exception)].
func outcome(v any, err error) []any {
	if err != nil {
		var e *Error
		if !errors.As(err, &e) {
			return []any{"not a *py.Error", err.Error()}
		}
		return []any{e.Type, e.Msg}
	}
	if f, ok := v.(float64); ok {
		return []any{"ok", pyjson.FloatRepr(f)}
	}
	return []any{"ok", v}
}

func TestValuesMatchPython(t *testing.T) {
	c := cases(t)
	for _, e := range c.List("int") {
		p := pyjson.List(e)
		in := caseValue(pyjson.List(p[0]))
		n, err := Int(in)
		if got := outcome(n, err); !pyjson.Equal(pyjson.ListOf(got), p[1]) {
			t.Errorf("int(%s) = %s, want %s", pyjson.Dumps(p[0]), pyjson.Dumps(pyjson.ListOf(got)), pyjson.Dumps(p[1]))
		}
	}
	for _, e := range c.List("float") {
		p := pyjson.List(e)
		in := caseValue(pyjson.List(p[0]))
		f, err := Float(in)
		if got := outcome(f, err); !pyjson.Equal(pyjson.ListOf(got), p[1]) {
			t.Errorf("float(%s) = %s, want %s", pyjson.Dumps(p[0]), pyjson.Dumps(pyjson.ListOf(got)), pyjson.Dumps(p[1]))
		}
	}
	for _, e := range c.List("typename") {
		p := pyjson.List(e)
		if got := TypeName(p[0]); got != p[1] {
			t.Errorf("type(%s).__name__ = %s, want %s", pyjson.Dumps(p[0]), got, p[1])
		}
	}
	for _, e := range c.List("or") {
		p := pyjson.List(e)
		if got := OrStr(p[0], "def"); got != p[1] || Str(Or(p[0], "def")) != p[1] {
			t.Errorf("str(%s or 'def') = %s, want %s", pyjson.Dumps(p[0]), got, p[1])
		}
	}
	for _, e := range c.List("iter") {
		p := pyjson.List(e)
		items, err := Iter(p[0])
		var v any = items
		if err == nil {
			v = pyjson.ListOf(items)
		}
		if got := outcome(v, err); !pyjson.Equal(pyjson.ListOf(got), p[1]) {
			t.Errorf("list(%s) = %s, want %s", pyjson.Dumps(p[0]), pyjson.Dumps(pyjson.ListOf(got)), pyjson.Dumps(p[1]))
		}
	}
}

func TestValueHelpers(t *testing.T) {
	// A str beyond a C long: Python's int holds it, the harness raises what taking it as one would.
	if _, big, err := IntText("99999999999999999999"); !big || err != nil {
		t.Errorf("IntText of a big number: %v, %v", big, err)
	}
	if _, err := Int("-99999999999999999999"); err == nil || err.(*Error).Type != "OverflowError" {
		t.Errorf("int() of a big number: %v", err)
	}
	if n, big, err := IntText("-9223372036854775808"); n != math.MinInt64 || big || err != nil {
		t.Errorf("IntText of the smallest int: %d, %v, %v", n, big, err)
	}
	err := raised(func() { MustInt("x") })
	if e, ok := err.(*Error); !ok || e.Type != "ValueError" {
		t.Errorf("MustInt raises Int's error: %v", err)
	}
	if Each(nil) != nil || Each(0) != nil || len(Each("ab")) != 2 {
		t.Error("Each is `for x in v or []`")
	}
	if err := raised(func() { MustIter(nil) }); err == nil || err.Error() != "'NoneType' object is not iterable" {
		t.Errorf("MustIter(None): %v", err)
	}
	if err := raised(func() { Item(pyjson.New("a", 1), "b") }); err == nil || err.(*Error).Type != "KeyError" ||
		err.Error() != "'b'" {
		t.Errorf("d['b']: %v", err)
	}
	if !In("a", "b", "a") || In(1, "1") || In(nil) {
		t.Error("In is `v in (…)` for strings")
	}
	if f, ok := Number(true); !ok || f != 1 {
		t.Error("a bool is a number")
	}
	if _, ok := Number("1"); ok {
		t.Error("a str is not a number")
	}
}

func TestCollectionsMatchPython(t *testing.T) {
	c := cases(t)
	for _, e := range c.List("sum") {
		p := pyjson.List(e)
		var xs []float64
		for _, r := range pyjson.List(p[0]) {
			f, err := Float(r)
			if err != nil {
				t.Fatal(err)
			}
			xs = append(xs, f)
		}
		if got := pyjson.FloatRepr(Sum(xs)); got != p[1] {
			t.Errorf("sum(%v) = %s, want %s", p[0], got, p[1])
		}
	}
	for _, e := range c.List("counter") {
		p := pyjson.List(e)
		counter := NewCounter()
		for _, k := range pyjson.Strings(p[0]) {
			counter.Add(k, 1)
		}
		var got []any
		for _, kn := range counter.MostCommon(-1) {
			got = append(got, []any{kn.Key, kn.N})
		}
		if !pyjson.Equal(pyjson.ListOf(got), p[1]) && len(got)+len(pyjson.List(p[1])) > 0 ||
			counter.Len() != p[2].(int) || counter.Total() != len(pyjson.List(p[0])) {
			t.Errorf("Counter(%v).most_common() = %v, want %v", p[0], got, p[1])
		}
		if top := counter.MostCommon(1); counter.Len() > 0 && (len(top) != 1 || top[0].Key != pyjson.Str(pyjson.List(pyjson.List(p[1])[0])[0])) {
			t.Errorf("most_common(1) of %v = %v", p[0], top)
		}
	}
	empty := NewCounter()
	empty.Add("e", 0)
	if empty.Get("zzz") != 0 || empty.Len() != 1 || empty.Total() != 0 || !slices.Equal(empty.Keys(), []string{"e"}) {
		t.Error("a key counted 0 is counted; reading a missing one adds nothing")
	}
	for _, e := range c.List("percent") {
		p := pyjson.List(e)
		x, _ := pyjson.AsFloat(p[0])
		if got := Percent(x, p[1].(int)); got != p[2] {
			t.Errorf("f\"{%v:.%v%%}\" = %s, want %s", x, p[1], got, p[2])
		}
	}
	for _, e := range c.List("compare") {
		p := pyjson.List(e)
		if got := Compare(pyjson.List(p[0]), pyjson.List(p[1])); got != p[2].(int) {
			t.Errorf("compare %v with %v = %d, want %v", p[0], p[1], got, p[2])
		}
	}
	if Compare([]any{[2]int{-2, 1}}, []any{[2]int{-9, 0}}) != 1 {
		t.Error("a [2]int compares as a pair")
	}
	s := c.List("sorted")
	byLen := func(w string) []any { return []any{len(w)} }
	words := pyjson.Strings(s[0])
	if got := SortedBy(words, byLen); !slices.Equal(got, pyjson.Strings(s[1])) || words[0] != "bb" {
		t.Errorf("sorted(key=len) = %v, want %v (and the input as it was)", got, s[1])
	}
	if MinBy(words, byLen) != s[2] || MinBy([]string{"bb", "cc"}, byLen) != s[3] {
		t.Error("min(key=len) is the first of the smallest")
	}
}
