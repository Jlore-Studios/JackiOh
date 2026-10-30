package cardsets

import (
	"fmt"
	"math"
	"regexp"
	"strconv"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// The Python a read leans on where Go differs. A read raises what Python raised on the same
// response (`AttributeError` for a `.get` on a list, `KeyError` for a card with no id, `ValueError`
// from int() and float(), …), as a *ReadError, so the note a failed read leaves names the same
// error.

// typeName is type(v).__name__ for a JSON value.
func typeName(v any) string {
	switch v.(type) {
	case nil:
		return "NoneType"
	case bool:
		return "bool"
	case int:
		return "int"
	case float64:
		return "float"
	case string:
		return "str"
	case []any:
		return "list"
	case *pyjson.Object:
		return "dict"
	}
	return fmt.Sprintf("%T", v)
}

// orDefault is `v or d`.
func orDefault(v, d any) any {
	if pyjson.Truthy(v) {
		return v
	}
	return d
}

// get is v.get(key).
func get(v any, key string) any {
	o, ok := v.(*pyjson.Object)
	if !ok {
		raise("AttributeError", fmt.Sprintf("'%s' object has no attribute 'get'", typeName(v)))
	}
	return o.Get(key)
}

// item is v[key] on a dict.
func item(v any, key string) any {
	o, ok := v.(*pyjson.Object)
	if !ok {
		raise("TypeError", fmt.Sprintf("'%s' object is not subscriptable", typeName(v)))
	}
	found, ok := o.Lookup(key)
	if !ok {
		raise("KeyError", py.Repr(key))
	}
	return found
}

// iterate is `for x in v`: a list's items, a dict's keys, a string's characters.
func iterate(v any) []any {
	switch x := v.(type) {
	case []any:
		return x
	case *pyjson.Object:
		return pyjson.ListOf(x.Keys())
	case string:
		var out []any
		for _, r := range x {
			out = append(out, string(r))
		}
		return out
	}
	raise("TypeError", fmt.Sprintf("'%s' object is not iterable", typeName(v)))
	return nil
}

var intLiteral = regexp.MustCompile(`^[+-]?[0-9]+(?:_[0-9]+)*$`)

// pyInt is int(v).
func pyInt(v any) int {
	switch x := v.(type) {
	case int:
		return x
	case float64:
		if math.IsInf(x, 0) {
			panic(fmt.Errorf("OverflowError: cannot convert float infinity to integer"))
		}
		if math.IsNaN(x) {
			raise("ValueError", "cannot convert float NaN to integer")
		}
		return int(x)
	case bool:
		if x {
			return 1
		}
		return 0
	case string:
		text := strings.TrimSpace(x)
		if intLiteral.MatchString(text) {
			if n, err := strconv.Atoi(strings.ReplaceAll(text, "_", "")); err == nil {
				return n
			}
		}
		raise("ValueError", "invalid literal for int() with base 10: "+py.Repr(x))
	}
	raise("TypeError", fmt.Sprintf(
		"int() argument must be a string, a bytes-like object or a real number, not '%s'", typeName(v)))
	return 0
}

// pyFloat is float(v).
func pyFloat(v any) float64 {
	switch x := v.(type) {
	case int:
		return float64(x)
	case float64:
		return x
	case bool:
		if x {
			return 1
		}
		return 0
	case string:
		text := strings.TrimSpace(x)
		if !strings.ContainsAny(text, "xX") {
			if f, err := strconv.ParseFloat(text, 64); err == nil {
				return f
			}
		}
		raise("ValueError", "could not convert string to float: "+py.Repr(x))
	}
	raise("TypeError", fmt.Sprintf("float() argument must be a string or a real number, not '%s'", typeName(v)))
	return 0
}

// sqrt is math.sqrt: a negative raises ValueError.
func sqrt(x float64) float64 {
	if x < 0 {
		raise("ValueError", "math domain error")
	}
	return math.Sqrt(x)
}

// fsum is math.fsum, which statistics.fmean sums with: the correctly rounded sum (Shewchuk's
// partials, as CPython's math_fsum keeps them).
func fsum(xs []float64) float64 {
	var partials []float64
	specialSum, infSum := 0.0, 0.0
	for _, x := range xs {
		xsave := x
		i := 0
		for _, y := range partials {
			if math.Abs(x) < math.Abs(y) {
				x, y = y, x
			}
			hi := x + y
			lo := y - (hi - x)
			if lo != 0 {
				partials[i] = lo
				i++
			}
			x = hi
		}
		partials = partials[:i]
		if x != 0 {
			if math.IsInf(x, 0) || math.IsNaN(x) {
				// A nonfinite x is an intermediate overflow, or a nan or inf among the summands.
				if !math.IsInf(xsave, 0) && !math.IsNaN(xsave) {
					panic(fmt.Errorf("OverflowError: intermediate overflow in fsum"))
				}
				if math.IsInf(xsave, 0) {
					infSum += xsave
				}
				specialSum += xsave
				partials = partials[:0]
			} else {
				partials = append(partials, x)
			}
		}
	}
	if specialSum != 0 {
		if math.IsNaN(infSum) {
			raise("ValueError", "-inf + inf in fsum")
		}
		return specialSum
	}
	hi := 0.0
	n := len(partials)
	if n > 0 {
		n--
		hi = partials[n]
		lo := 0.0
		// Sum exactly from the top, and stop when the sum becomes inexact.
		for n > 0 {
			x := hi
			n--
			y := partials[n]
			hi = x + y
			lo = y - (hi - x)
			if lo != 0 {
				break
			}
		}
		// Make half-even rounding work across multiple partials.
		if n > 0 && ((lo < 0 && partials[n-1] < 0) || (lo > 0 && partials[n-1] > 0)) {
			y := lo * 2
			x := hi + y
			if y == x-hi {
				hi = x
			}
		}
	}
	return hi
}
