// unittest.TestCase's assertions, for every file of the package: each is the Python method of
// the same name (assertEqual, assertIn, …) with the test first, and fails the test at once, as a
// Python assertion raises. Python's ==, `in`, bool() and `is None` are pyEqual, pyIn, pyTruthy and
// pyIsNone, over JSON values (internal/pyjson) and the Go values the ports hold Python's in (a set
// is a map[T]bool, a tuple an array or a struct, None a nil or "").
package tests

import (
	"fmt"
	"maps"
	"math"
	"reflect"
	"regexp"
	"slices"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// assertEqual is assertEqual(first, second, msg).
func assertEqual(t *testing.T, got, want any, msg ...any) {
	t.Helper()
	if !pyEqual(got, want) {
		t.Fatalf("assertEqual failed%s\n got: %s\nwant: %s", assertMsg(msg), show(got), show(want))
	}
}

// assertNotEqual is assertNotEqual(first, second, msg).
func assertNotEqual(t *testing.T, got, other any, msg ...any) {
	t.Helper()
	if pyEqual(got, other) {
		t.Fatalf("assertNotEqual failed%s: both are %s", assertMsg(msg), show(got))
	}
}

// assertIn is assertIn(member, container, msg).
func assertIn(t *testing.T, member, container any, msg ...any) {
	t.Helper()
	if !pyIn(member, container) {
		t.Fatalf("assertIn failed%s\n%s\nnot found in\n%s", assertMsg(msg), show(member),
			showContainer(member, container))
	}
}

// assertNotIn is assertNotIn(member, container, msg).
func assertNotIn(t *testing.T, member, container any, msg ...any) {
	t.Helper()
	if pyIn(member, container) {
		t.Fatalf("assertNotIn failed%s\n%s\nunexpectedly found in\n%s", assertMsg(msg), show(member),
			showContainer(member, container))
	}
}

// assertTrue is assertTrue(expr, msg): Python's bool(expr).
func assertTrue(t *testing.T, v any, msg ...any) {
	t.Helper()
	if !pyTruthy(v) {
		t.Fatalf("assertTrue failed%s: %s is not true", assertMsg(msg), showBool(v))
	}
}

// assertFalse is assertFalse(expr, msg).
func assertFalse(t *testing.T, v any, msg ...any) {
	t.Helper()
	if pyTruthy(v) {
		t.Fatalf("assertFalse failed%s: %s is not false", assertMsg(msg), showBool(v))
	}
}

// assertIsNone is assertIsNone(obj, msg).
func assertIsNone(t *testing.T, v any, msg ...any) {
	t.Helper()
	if !pyIsNone(v) {
		t.Fatalf("assertIsNone failed%s: %s is not None", assertMsg(msg), show(v))
	}
}

// assertIsNotNone is assertIsNotNone(obj, msg).
func assertIsNotNone(t *testing.T, v any, msg ...any) {
	t.Helper()
	if pyIsNone(v) {
		t.Fatalf("assertIsNotNone failed%s: unexpectedly None (%s)", assertMsg(msg), show(v))
	}
}

// assertAlmostEqual is assertAlmostEqual(first, second, msg): equal, or their difference rounded
// to 7 places is 0.
func assertAlmostEqual(t *testing.T, got any, want float64, msg ...any) {
	t.Helper()
	f, ok := pyjson.AsFloat(got)
	if !ok || (f != want && py.RoundN(math.Abs(f-want), 7) != 0) {
		t.Fatalf("assertAlmostEqual failed%s: %s != %v within 7 places", assertMsg(msg), show(got), want)
	}
}

// assertLess is assertLess(a, b, msg).
func assertLess(t *testing.T, a, b int, msg ...any) {
	t.Helper()
	if !(a < b) {
		t.Fatalf("assertLess failed%s: %d not less than %d", assertMsg(msg), a, b)
	}
}

// assertLessEqual is assertLessEqual(a, b, msg).
func assertLessEqual(t *testing.T, a, b int, msg ...any) {
	t.Helper()
	if !(a <= b) {
		t.Fatalf("assertLessEqual failed%s: %d not less than or equal to %d", assertMsg(msg), a, b)
	}
}

// assertRegex is assertRegex(text, regex, msg): re.search finds it.
func assertRegex(t *testing.T, text, pattern string, msg ...any) {
	t.Helper()
	if !regexp.MustCompile(pattern).MatchString(text) {
		t.Fatalf("assertRegex failed%s: Regex didn't match: %q not found in\n%s", assertMsg(msg), pattern,
			showContainer(pattern, text))
	}
}

// assertNotRegex is assertNotRegex(text, regex, msg).
func assertNotRegex(t *testing.T, text, pattern string, msg ...any) {
	t.Helper()
	if m := regexp.MustCompile(pattern).FindStringIndex(text); m != nil {
		t.Fatalf("assertNotRegex failed%s: Regex matched: %q matches %q in\n%s", assertMsg(msg),
			text[m[0]:m[1]], pattern, showContainer(text[m[0]:m[1]], text))
	}
}

// assertRaises is assertRaises(E): fn must raise (panic with) an E, which it returns.
func assertRaises[E error](t *testing.T, fn func(), msg ...any) E {
	t.Helper()
	caught, ok := herr.Catch[E](fn)
	if !ok {
		var want E
		t.Fatalf("assertRaises failed%s: %T not raised", assertMsg(msg), want)
	}
	return caught
}

// assertRaisesRegex is assertRaisesRegex(E, regex): fn raises an E whose text re.search finds
// the pattern in.
func assertRaisesRegex[E error](t *testing.T, pattern string, fn func(), msg ...any) E {
	t.Helper()
	caught := assertRaises[E](t, fn, msg...)
	if !regexp.MustCompile(pattern).MatchString(caught.Error()) {
		t.Fatalf("assertRaisesRegex failed%s: %q does not match %q", assertMsg(msg), pattern, caught.Error())
	}
	return caught
}

// ---------------------------------------------------------------------------- Python's operators

// pyEqual is Python's ==: JSON values as pyjson.Equal compares them (an int equals a float of its
// value, objects regardless of key order), an aware datetime by its instant, and Go's own values
// (sets, tuples, lists, structs) element by element, where a nil slice or map is the empty one, as
// both are Python's [] or {}.
func pyEqual(got, want any) bool {
	if reflect.DeepEqual(got, want) || pyjson.Equal(got, want) {
		return true
	}
	if gt, ok := got.(time.Time); ok {
		wt, ok := want.(time.Time)
		return ok && gt.Equal(wt)
	}
	a, b := reflect.ValueOf(got), reflect.ValueOf(want)
	if !a.IsValid() || !b.IsValid() || a.Type() != b.Type() {
		return false
	}
	switch a.Kind() {
	case reflect.Slice, reflect.Array:
		if a.Len() != b.Len() {
			return false
		}
		for i := range a.Len() {
			if !pyEqual(a.Index(i).Interface(), b.Index(i).Interface()) {
				return false
			}
		}
		return true
	case reflect.Map:
		if a.Len() != b.Len() {
			return false
		}
		for _, k := range a.MapKeys() {
			v := b.MapIndex(k)
			if !v.IsValid() || !pyEqual(a.MapIndex(k).Interface(), v.Interface()) {
				return false
			}
		}
		return true
	case reflect.Struct:
		for i := range a.NumField() {
			if !a.Type().Field(i).IsExported() {
				return false
			}
			if !pyEqual(a.Field(i).Interface(), b.Field(i).Interface()) {
				return false
			}
		}
		return true
	}
	return false
}

// pyIn is Python's `member in container`: a substring of a string, a key of a JSON object or of a
// map (a dict or a set), an element (by pyEqual) of a list or tuple. None is not a container.
func pyIn(member, container any) bool {
	switch c := container.(type) {
	case string:
		s, ok := member.(string)
		return ok && strings.Contains(c, s)
	case *pyjson.Object:
		s, ok := member.(string)
		return ok && c != nil && c.Has(s)
	case nil:
		panic("TypeError: argument of type 'NoneType' is not iterable")
	}
	v := reflect.ValueOf(container)
	switch v.Kind() {
	case reflect.Map:
		k := reflect.ValueOf(member)
		if !k.IsValid() || !k.Type().AssignableTo(v.Type().Key()) {
			return false
		}
		return v.MapIndex(k).IsValid()
	case reflect.Slice, reflect.Array:
		for i := range v.Len() {
			if pyEqual(v.Index(i).Interface(), member) {
				return true
			}
		}
		return false
	}
	panic(fmt.Sprintf("TypeError: argument of type %T is not iterable", container))
}

// pyTruthy is Python's bool(v): a bool as it is, JSON as pyjson.Truthy, a Go slice or map by its
// length, a nil pointer false.
func pyTruthy(v any) bool {
	r := reflect.ValueOf(v)
	switch r.Kind() {
	case reflect.Slice, reflect.Map:
		if _, ok := v.([]any); !ok {
			return r.Len() > 0
		}
	case reflect.Pointer:
		if r.IsNil() {
			return false
		}
	}
	return pyjson.Truthy(v)
}

// pyIsNone is `v is None`: a nil (a nil pointer, map, slice or func among them), or the "" a port
// gives for a text that is None.
func pyIsNone(v any) bool {
	if v == nil || v == "" {
		return true
	}
	r := reflect.ValueOf(v)
	switch r.Kind() {
	case reflect.Pointer, reflect.Map, reflect.Slice, reflect.Func, reflect.Interface:
		return r.IsNil()
	}
	return false
}

// ---------------------------------------------------------------------------- what a failure prints

// show is a value as a failure prints it: JSON as JSON, text quoted, a set sorted, anything else
// as Go writes it.
func show(v any) string {
	switch x := v.(type) {
	case *pyjson.Object, []any:
		return pyjson.Dumps(x)
	case []*pyjson.Object:
		return pyjson.Dumps(pyjson.ListOf(x))
	case string:
		return strconv.Quote(x)
	case map[string]bool:
		return fmt.Sprintf("set%q", slices.Sorted(maps.Keys(x)))
	case map[int]bool:
		return fmt.Sprintf("set%v", slices.Sorted(maps.Keys(x)))
	}
	return fmt.Sprintf("%#v", v)
}

// showBool is what assertTrue and assertFalse print of their argument: Python's True or False for a
// bool, the value otherwise.
func showBool(v any) string {
	if b, ok := v.(bool); ok {
		return map[bool]string{true: "True", false: "False"}[b]
	}
	return show(v)
}

// showContainer is a container as assertIn prints it: all of it, and, when it is long text, first
// the lines that hold the start of the member (its first line), so the failure says where to look.
func showContainer(member, container any) string {
	text, ok := container.(string)
	if !ok || py.Len(text) <= 3000 {
		return show(container)
	}
	probe := ""
	if s, ok := member.(string); ok {
		probe = py.SliceTo(strings.SplitN(s, "\n", 2)[0], 24)
	}
	var near []string
	for _, line := range py.Splitlines(text) {
		if probe != "" && strings.Contains(line, probe) {
			near = append(near, line)
			if len(near) == 8 {
				break
			}
		}
	}
	return fmt.Sprintf("(%d characters; the lines holding %q:)\n%s\n(all of it:)\n%s", py.Len(text), probe,
		strings.Join(near, "\n"), show(text))
}

// assertMsg is an assertion's msg argument, as the end of the failure's first line.
func assertMsg(msg []any) string {
	if len(msg) == 0 {
		return ""
	}
	parts := make([]string, len(msg))
	for i, m := range msg {
		if s, ok := m.(string); ok {
			parts[i] = s
		} else {
			parts[i] = show(m)
		}
	}
	return ": " + strings.Join(parts, " ")
}

// ---------------------------------------------------------------------------- Python's builtins

// strIndex is s.index(sub): where sub starts in s, the test failing (Python's ValueError) when it
// is not there.
func strIndex(t *testing.T, s, sub string) int {
	t.Helper()
	i := strings.Index(s, sub)
	if i < 0 {
		t.Fatalf("ValueError: substring not found: %q in:\n%s", sub, showContainer(sub, s))
	}
	return i
}
