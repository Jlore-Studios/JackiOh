package py

// Python's built-ins over JSON values: int(), float(), `x or default`, iteration and
// type(v).__name__, with the exceptions Python raises from them.

import (
	"fmt"
	"math"
	"regexp"
	"slices"
	"strconv"
	"strings"
	"unicode"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Error is one of Python's own exceptions where the harness raises (herr.Raise) or returns what
// Python raised: Type is its class name ("ValueError"), Msg its str(). herr.Name reads Type, and
// a traceback's last line is `Type: Msg`.
type Error struct {
	Type string
	Msg  string
}

func (e *Error) Error() string { return e.Msg }

// KeyError is Python's KeyError for a missing key, whose str() is the key's repr.
func KeyError(key string) *Error { return &Error{"KeyError", Repr(key)} }

// TypeName is type(v).__name__ for a JSON value.
func TypeName(v any) string {
	switch v.(type) {
	case nil:
		return "NoneType"
	case bool:
		return "bool"
	case int, int64, int32:
		return "int"
	case float64, float32:
		return "float"
	case string:
		return "str"
	case []any, []string:
		return "list"
	case *pyjson.Object:
		return "dict"
	}
	return fmt.Sprintf("%T", v)
}

// Number is isinstance(v, (int, float)) for a JSON value, and its value as a float: a bool is an
// int in Python, so it counts as 0 or 1.
func Number(v any) (float64, bool) {
	if b, ok := v.(bool); ok {
		if b {
			return 1, true
		}
		return 0, true
	}
	return pyjson.AsFloat(v)
}

// In is `v in (a, b, …)` for a JSON value v and strings: v is a str equal to one of them.
func In(v any, options ...string) bool {
	s, ok := v.(string)
	return ok && slices.Contains(options, s)
}

// Item is d[key]: the value, raising KeyError when the key is absent.
func Item(d *pyjson.Object, key string) any {
	v, ok := d.Lookup(key)
	if !ok {
		panic(KeyError(key))
	}
	return v
}

// Or is `v or def`: v when it is truthy, else def.
func Or(v, def any) any {
	if pyjson.Truthy(v) {
		return v
	}
	return def
}

// OrStr is `str(v or def)`: str(v) when v is truthy, else def.
func OrStr(v any, def string) string {
	if pyjson.Truthy(v) {
		return Str(v)
	}
	return def
}

// Iter is `list(v)`, what `for x in v` goes through: a list's items, a dict's keys, a str's
// characters, and TypeError for anything else.
func Iter(v any) ([]any, error) {
	switch x := v.(type) {
	case []any:
		return x, nil
	case []string:
		return pyjson.ListOf(x), nil
	case *pyjson.Object:
		return pyjson.ListOf(x.Keys()), nil
	case string:
		out := []any{}
		for _, r := range x {
			out = append(out, string(r))
		}
		return out, nil
	}
	return nil, &Error{"TypeError", fmt.Sprintf("'%s' object is not iterable", TypeName(v))}
}

// MustIter is `for x in v` where the Python lets the TypeError go up: Iter's items, or it raises.
func MustIter(v any) []any {
	items, err := Iter(v)
	if err != nil {
		panic(err)
	}
	return items
}

// Each is `for x in v or []`: nothing for a falsy v, else MustIter(v).
func Each(v any) []any {
	if !pyjson.Truthy(v) {
		return nil
	}
	return MustIter(v)
}

// Int is int(v): a bool 0 or 1, a float cut toward zero, a str as IntText reads it, and the
// ValueError, OverflowError or TypeError Python raises for anything else. A str beyond a C long
// is an OverflowError, where Python's int would hold it.
func Int(v any) (int, error) {
	switch x := v.(type) {
	case bool:
		if x {
			return 1, nil
		}
		return 0, nil
	case float64:
		if math.IsNaN(x) {
			return 0, &Error{"ValueError", "cannot convert float NaN to integer"}
		}
		if math.IsInf(x, 0) {
			return 0, &Error{"OverflowError", "cannot convert float infinity to integer"}
		}
		return int(x), nil
	case string:
		n, big, err := IntText(x)
		if big {
			return 0, &Error{"OverflowError", "Python int too large to convert to C long"}
		}
		return n, err
	}
	if n, ok := pyjson.AsInt(v); ok {
		return n, nil
	}
	return 0, &Error{"TypeError", fmt.Sprintf(
		"int() argument must be a string, a bytes-like object or a real number, not '%s'", TypeName(v))}
}

// MustInt is int(v) where the Python lets its exception go up: it raises Int's error.
func MustInt(v any) int {
	n, err := Int(v)
	if err != nil {
		panic(err)
	}
	return n
}

// IntText is int(s) for a str, base 10, as CPython reads it: whitespace around it (Go's
// unicode.IsSpace, which is CPython's here: \x1c to \x1f are not), a sign, decimal digits of
// any script and single underscores between two of them. big says the value is beyond a C long
// (Python's int holds it; whatever takes it as a C integer raises OverflowError). The error is
// Python's ValueError, its repr cut to 200 characters as CPython's %.200R does.
func IntText(s string) (n int, big bool, err error) {
	t := strings.TrimSpace(s)
	neg := false
	if t != "" && (t[0] == '+' || t[0] == '-') {
		neg, t = t[0] == '-', t[1:]
	}
	invalid := &Error{"ValueError", "invalid literal for int() with base 10: " + SliceTo(Repr(s), 200)}
	if t == "" || t[0] == '_' || t[len(t)-1] == '_' || strings.Contains(t, "__") {
		return 0, false, invalid
	}
	limit := uint64(math.MaxInt64)
	if neg {
		limit++
	}
	var u uint64
	for _, r := range t {
		if r == '_' {
			continue
		}
		if !IsDigit(r) {
			return 0, false, invalid
		}
		d := uint64(DigitValue(r))
		if big || u > (limit-d)/10 {
			big = true
			continue
		}
		u = u*10 + d
	}
	switch {
	case big:
		return 0, true, nil
	case neg:
		return int(-u), false, nil
	}
	return int(u), false, nil
}

// floatSyntax is what float() reads, once stripped: a decimal number whose digits may be any
// script's, with `_` between two of them, or inf, infinity or nan in any case, signed or not.
var floatSyntax = regexp.MustCompile(`^[+-]?(?:(?i:inf|infinity|nan)|(?:\p{Nd}(?:_?\p{Nd})*(?:\.(?:\p{Nd}(?:_?\p{Nd})*)?)?|\.\p{Nd}(?:_?\p{Nd})*)(?:[eE][+-]?\p{Nd}(?:_?\p{Nd})*)?)$`)

// Float is float(v): a number, a bool, or a str Python's float() reads (whitespace around it as
// IntText strips, underscores between digits, any script's digits, inf and nan; a literal past
// the largest float is inf), and the ValueError or TypeError Python raises for anything else.
func Float(v any) (float64, error) {
	switch x := v.(type) {
	case bool:
		if x {
			return 1, nil
		}
		return 0, nil
	case string:
		if f, ok := floatText(x); ok {
			return f, nil
		}
		return 0, &Error{"ValueError", "could not convert string to float: " + Repr(x)}
	}
	if f, ok := pyjson.AsFloat(v); ok {
		return f, nil
	}
	return 0, &Error{"TypeError", fmt.Sprintf("float() argument must be a string or a real number, not '%s'",
		TypeName(v))}
}

// MustFloat is float(v) where the Python lets its exception go up: it raises Float's error.
func MustFloat(v any) float64 {
	f, err := Float(v)
	if err != nil {
		panic(err)
	}
	return f
}

func floatText(text string) (float64, bool) {
	clean := strings.TrimSpace(text)
	if !floatSyntax.MatchString(clean) {
		return 0, false
	}
	if strings.EqualFold(strings.TrimLeft(clean, "+-"), "nan") {
		return math.NaN(), true // Go reads no sign on a NaN
	}
	var b strings.Builder
	for _, r := range clean {
		switch {
		case r == '_':
		case r > 0x7f && unicode.IsDigit(r):
			b.WriteByte(byte('0' + DigitValue(r)))
		default:
			b.WriteRune(r)
		}
	}
	f, err := strconv.ParseFloat(b.String(), 64)
	if err != nil {
		if ne, ok := err.(*strconv.NumError); !ok || ne.Err != strconv.ErrRange {
			return 0, false
		}
	}
	return f, true
}
