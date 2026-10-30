package config

import (
	"fmt"
	"math"
	"strconv"
	"strings"
	"unicode"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// The Python built-ins `parse` applies to the committed JSON (`int()`, `dict()`, iteration,
// `raw[key]`), with the exceptions they raise. Where the Python catches one (`except (KeyError,
// TypeError, ValueError)`) the Go checks the error; where it does not, the error is raised as it
// was (herr.Raise of a pyError), which ends the command as an uncaught exception did.

// pyError is one of Python's own exceptions: Kind is its class (`TypeError`), Msg its str().
type pyError struct {
	Kind string
	Msg  string
}

func (e *pyError) Error() string { return e.Msg }

func typeError(format string, args ...any) error {
	return &pyError{Kind: "TypeError", Msg: fmt.Sprintf(format, args...)}
}

func valueError(format string, args ...any) error {
	return &pyError{Kind: "ValueError", Msg: fmt.Sprintf(format, args...)}
}

// typeName is type(v).__name__ for a JSON value.
func typeName(v any) string {
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

// pyInt is int(v): a bool is 0 or 1, a float is truncated, a string is parsed as Python parses
// it (surrounding whitespace, a sign, underscores between digits).
func pyInt(v any) (int, error) {
	switch x := v.(type) {
	case bool:
		if x {
			return 1, nil
		}
		return 0, nil
	case int:
		return x, nil
	case int64:
		return int(x), nil
	case int32:
		return int(x), nil
	case float64:
		if math.IsNaN(x) {
			return 0, valueError("cannot convert float NaN to integer")
		}
		if math.IsInf(x, 0) {
			return 0, &pyError{Kind: "OverflowError", Msg: "cannot convert float infinity to integer"}
		}
		return int(x), nil
	case string:
		if n, ok := parseIntLiteral(x); ok {
			return n, nil
		}
		return 0, valueError("invalid literal for int() with base 10: %s", py.Repr(x))
	}
	return 0, typeError("int() argument must be a string, a bytes-like object or a real number, not '%s'", typeName(v))
}

// parseIntLiteral is int(s) for a str in base 10.
func parseIntLiteral(s string) (int, bool) {
	s = strings.TrimFunc(s, unicode.IsSpace)
	sign := ""
	if strings.HasPrefix(s, "+") || strings.HasPrefix(s, "-") {
		sign, s = s[:1], s[1:]
	}
	if s == "" || strings.HasPrefix(s, "_") || strings.HasSuffix(s, "_") || strings.Contains(s, "__") {
		return 0, false
	}
	digits := strings.ReplaceAll(s, "_", "")
	for _, r := range digits {
		if r < '0' || r > '9' {
			return 0, false
		}
	}
	n, err := strconv.Atoi(sign + digits)
	return n, err == nil
}

// mustInt is int(v) where the Python lets the exception go.
func mustInt(v any) int {
	n, err := pyInt(v)
	if err != nil {
		panic(err)
	}
	return n
}

// pyIter is list(v): a list's items, a string's characters, a dict's keys.
func pyIter(v any) ([]any, error) {
	switch x := v.(type) {
	case []any:
		return x, nil
	case []string:
		return pyjson.ListOf(x), nil
	case string:
		out := []any{}
		for _, r := range x {
			out = append(out, string(r))
		}
		return out, nil
	case *pyjson.Object:
		return pyjson.ListOf(x.Keys()), nil
	}
	return nil, typeError("'%s' object is not iterable", typeName(v))
}

// strs is tuple(str(x) for x in v).
func strs(v any) ([]string, error) {
	items, err := pyIter(v)
	if err != nil {
		return nil, err
	}
	out := make([]string, 0, len(items))
	for _, x := range items {
		out = append(out, py.Str(x))
	}
	return out, nil
}

// mustStrs is strs where the Python lets the exception go.
func mustStrs(v any) []string {
	out, err := strs(v)
	if err != nil {
		panic(err)
	}
	return out
}

// pyDict is dict(v): a copy of a dict, or a dict from a sequence of pairs. The keys are kept as
// str(key), which is all `parse` reads them as.
func pyDict(v any) (*pyjson.Object, error) {
	if o, ok := v.(*pyjson.Object); ok {
		return o.Copy(), nil
	}
	items, err := pyIter(v)
	if err != nil {
		return nil, err
	}
	out := pyjson.New()
	for i, item := range items {
		var pair []any
		switch x := item.(type) {
		case []any:
			pair = x
		case string:
			pair, _ = pyIter(x)
		case *pyjson.Object:
			pair = pyjson.ListOf(x.Keys())
		default:
			return nil, typeError("cannot convert dictionary update sequence element #%d to a sequence", i)
		}
		if len(pair) != 2 {
			return nil, valueError("dictionary update sequence element #%d has length %d; 2 is required", i, len(pair))
		}
		out.Set(py.Str(pair[0]), pair[1])
	}
	return out, nil
}

// mustDict is dict(v) where the Python lets the exception go.
func mustDict(v any) *pyjson.Object {
	o, err := pyDict(v)
	if err != nil {
		panic(err)
	}
	return o
}

// subscript is v[key] with a str key.
func subscript(v any, key string) (any, error) {
	switch x := v.(type) {
	case *pyjson.Object:
		if found, ok := x.Lookup(key); ok {
			return found, nil
		}
		return nil, &pyError{Kind: "KeyError", Msg: py.Repr(key)}
	case []any:
		return nil, typeError("list indices must be integers or slices, not str")
	case string:
		return nil, typeError("string indices must be integers, not 'str'")
	}
	return nil, typeError("'%s' object is not subscriptable", typeName(v))
}

// contains is `s in xs` for a tuple of strings.
func contains(xs []string, s string) bool {
	for _, x := range xs {
		if x == s {
			return true
		}
	}
	return false
}
