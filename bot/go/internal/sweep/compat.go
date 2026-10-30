package sweep

// Private helpers this package needs from Python itself (`int()`, `d[key]` and their exceptions,
// `type(exc).__name__`) and the running bot's constants from `config`, read through `any` so this
// package compiles whether `config` declares them as `any` or as their own types.

import (
	"errors"
	"fmt"
	"io/fs"
	"reflect"
	"runtime"
	"strconv"
	"strings"
	"time"
	"unicode"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// pyError is one of Python's own exceptions where the Python raised it (KeyError, ValueError,
// TypeError), so a note that names the exception names the same one.
type pyError struct {
	name string
	msg  string
}

func (e *pyError) Error() string { return e.msg }

// key is d[k]: the value, or a KeyError.
func key(o *pyjson.Object, k string) any {
	v, ok := o.Lookup(k)
	if !ok {
		panic(&pyError{"KeyError", py.Repr(k)})
	}
	return v
}

// intOf is int(v) for a JSON value.
func intOf(v any) int {
	switch x := v.(type) {
	case bool:
		if x {
			return 1
		}
		return 0
	case string:
		t := strings.TrimFunc(x, unicode.IsSpace)
		body := strings.TrimLeft(t, "+-")
		ok := len(t)-len(body) <= 1 && body != "" && !strings.HasPrefix(body, "_") &&
			!strings.HasSuffix(body, "_") && !strings.Contains(body, "__")
		for _, r := range body {
			if (r < '0' || r > '9') && r != '_' {
				ok = false
			}
		}
		if ok {
			if n, err := strconv.Atoi(strings.ReplaceAll(t, "_", "")); err == nil {
				return n
			}
		}
		panic(&pyError{"ValueError", "invalid literal for int() with base 10: " + py.Repr(x)})
	case nil:
		panic(&pyError{"TypeError", "int() argument must be a string, a bytes-like object or a real number, not 'NoneType'"})
	}
	if n, ok := pyjson.AsInt(v); ok {
		return n
	}
	panic(&pyError{"TypeError", fmt.Sprintf("int() argument must be a string, a bytes-like object or a real number, not '%T'", v)})
}

// isDigit is str.isdigit().
func isDigit(s string) bool {
	if s == "" {
		return false
	}
	for _, r := range s {
		if !unicode.IsDigit(r) {
			return false
		}
	}
	return true
}

// typeName is `type(exc).__name__`. herr's types carry the Python classes' names, so the last
// part of a Go type's name is the Python one.
func typeName(err error) string {
	var pe *pyError
	if errors.As(err, &pe) {
		return pe.name
	}
	var p *herr.Panic
	if errors.As(err, &p) {
		return "Exception"
	}
	var re runtime.Error
	if errors.As(err, &re) {
		return "RuntimeError"
	}
	var pathErr *fs.PathError
	if errors.As(err, &pathErr) {
		return "OSError"
	}
	name := strings.TrimPrefix(reflect.TypeOf(err).String(), "*")
	if i := strings.LastIndex(name, "."); i >= 0 {
		name = name[i+1:]
	}
	return name
}

// parseISO is clock.parse_iso over any JSON value: zero (None) for a falsy one.
func parseISO(v any) time.Time {
	if !pyjson.Truthy(v) {
		return time.Time{}
	}
	return clock.ParseISO(py.Str(v))
}

// orStr is `str(v or "")`.
func orStr(v any) string {
	if !pyjson.Truthy(v) {
		return ""
	}
	return py.Str(v)
}

func contains(xs []string, s string) bool {
	for _, x := range xs {
		if x == s {
			return true
		}
	}
	return false
}

// containsAny is `v in (…)` for a value that may not be a string.
func containsAny(xs []string, v any) bool {
	s, ok := v.(string)
	return ok && contains(xs, s)
}

// intersects is `bool(names & set(others))`.
func intersects(names map[string]bool, others []string) bool {
	for _, o := range others {
		if names[o] {
			return true
		}
	}
	return false
}

func cfgStr(v any) string {
	switch x := v.(type) {
	case nil:
		return ""
	case string:
		return x
	case fmt.Stringer:
		return x.String()
	}
	panic(fmt.Sprintf("sweep: config constant of type %T, not a string", v))
}

func cfgStrings(v any) []string {
	switch x := v.(type) {
	case nil:
		return nil
	case []string:
		return x
	case []any:
		return pyjson.Strings(x)
	}
	panic(fmt.Sprintf("sweep: config constant of type %T, not a list of strings", v))
}

// modes is config.MODES.
func modes() []string { return cfgStrings(any(config.Modes)) }

// retiredLabels is config.RETIRED_LABELS.
func retiredLabels() []string { return cfgStrings(any(config.RetiredLabels)) }

// nightWorkflow is config.NIGHT_WORKFLOW.
func nightWorkflow() string { return cfgStr(any(config.NightWorkflow)) }

// labelSpec is one of config.LABELS: name: (color, description).
type labelSpec struct{ name, color, description string }

// labels is `LABELS.items()`, in order.
func labels() []labelSpec {
	var out []labelSpec
	var o *pyjson.Object
	switch x := any(config.Labels).(type) {
	case nil:
		return nil
	case *pyjson.Object:
		o = x
	default:
		panic(fmt.Sprintf("sweep: config.LABELS of type %T", config.Labels))
	}
	o.Items(func(name string, v any) bool {
		var pair []string
		switch t := v.(type) {
		case []any:
			pair = pyjson.Strings(t)
		case []string:
			pair = t
		case [2]string:
			pair = t[:]
		}
		if len(pair) != 2 {
			panic(fmt.Sprintf("sweep: config.LABELS[%q] of type %T", name, v))
		}
		out = append(out, labelSpec{name, pair[0], pair[1]})
		return true
	})
	return out
}

// The labels the queue uses (config.LABEL_*).
func labelBlocked() string { return cfgStr(any(config.LabelBlocked)) }
func labelBuild() string   { return cfgStr(any(config.LabelBuild)) }
func labelPlanned() string { return cfgStr(any(config.LabelPlanned)) }
func labelPR() string      { return cfgStr(any(config.LabelPR)) }
func labelPROpen() string  { return cfgStr(any(config.LabelPROpen)) }
func labelRevise() string  { return cfgStr(any(config.LabelRevise)) }
func labelTree() string    { return cfgStr(any(config.LabelTree)) }
func labelWorking() string { return cfgStr(any(config.LabelWorking)) }
