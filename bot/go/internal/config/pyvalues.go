package config

import (
	"fmt"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// The Python built-ins `parse` applies to the committed JSON (`dict()`, `tuple(str(x) ...)`,
// `raw[key]`; int() and iteration are py's), with the exceptions they raise. Where the Python
// catches one (`except (KeyError, TypeError, ValueError)`) the Go checks the error; where it does
// not, the error is raised as it was (herr.Raise of a *py.Error), which ends the command as an
// uncaught exception did.

func typeError(format string, args ...any) error {
	return &py.Error{Type: "TypeError", Msg: fmt.Sprintf(format, args...)}
}

func valueError(format string, args ...any) error {
	return &py.Error{Type: "ValueError", Msg: fmt.Sprintf(format, args...)}
}

// strs is tuple(str(x) for x in v).
func strs(v any) ([]string, error) {
	items, err := py.Iter(v)
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
	items, err := py.Iter(v)
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
			pair, _ = py.Iter(x)
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
		return nil, py.KeyError(key)
	case []any:
		return nil, typeError("list indices must be integers or slices, not str")
	case string:
		return nil, typeError("string indices must be integers, not 'str'")
	}
	return nil, typeError("'%s' object is not subscriptable", py.TypeName(v))
}
