package cardsets

import (
	"fmt"
	"math"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// The Python a read leans on where Go differs and py does not hold it. A read raises what Python
// raised on the same response (`AttributeError` for a `.get` on a list, `KeyError` for a card with
// no id, `ValueError` from int() and float(), …), as a *ReadError, so the note a failed read
// leaves names the same error.

// get is v.get(key).
func get(v any, key string) any {
	o, ok := v.(*pyjson.Object)
	if !ok {
		raise("AttributeError", fmt.Sprintf("'%s' object has no attribute 'get'", py.TypeName(v)))
	}
	return o.Get(key)
}

// item is v[key] on a dict.
func item(v any, key string) any {
	o, ok := v.(*pyjson.Object)
	if !ok {
		raise("TypeError", fmt.Sprintf("'%s' object is not subscriptable", py.TypeName(v)))
	}
	found, ok := o.Lookup(key)
	if !ok {
		herr.Raise(py.KeyError(key))
	}
	return found
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
