package stats

import (
	"fmt"
	"math"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// The Python this module leans on where Go differs: Counter, int(), float(), sum() of floats,
// statistics.median of timedeltas, and the `x or default` of JSON values.

// pyError is a Python exception the module does not catch (its class name and str()): a value the
// Python could not read stops the count, as it did there.
type pyError struct{ typ, msg string }

func (e *pyError) Error() string { return e.typ + ": " + e.msg }

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

var intLiteral = regexp.MustCompile(`^[+-]?[0-9]+(?:_[0-9]+)*$`)

// pyInt is int(v).
func pyInt(v any) int {
	switch x := v.(type) {
	case int:
		return x
	case float64:
		if math.IsInf(x, 0) {
			panic(&pyError{"OverflowError", "cannot convert float infinity to integer"})
		}
		if math.IsNaN(x) {
			panic(&pyError{"ValueError", "cannot convert float NaN to integer"})
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
		panic(&pyError{"ValueError", "invalid literal for int() with base 10: " + py.Repr(x)})
	}
	panic(&pyError{"TypeError", fmt.Sprintf(
		"int() argument must be a string, a bytes-like object or a real number, not '%s'", typeName(v))})
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
		panic(&pyError{"ValueError", "could not convert string to float: " + py.Repr(x)})
	}
	panic(&pyError{"TypeError", fmt.Sprintf(
		"float() argument must be a string or a real number, not '%s'", typeName(v))})
}

// orDefault is `v or d`.
func orDefault(v, d any) any {
	if pyjson.Truthy(v) {
		return v
	}
	return d
}

// orStr is str(v or "").
func orStr(v any) string { return py.Str(orDefault(v, "")) }

// intOr is int(v or 0).
func intOr(v any) int { return pyInt(orDefault(v, 0)) }

// floatOr is float(v or 0).
func floatOr(v any) float64 { return pyFloat(orDefault(v, 0)) }

// listOr is `v or []`, as a list to iterate.
func listOr(v any) []any { return pyjson.List(orDefault(v, nil)) }

// pySum is sum(xs) over floats in Python 3.12 and later: from int 0, then Neumaier's compensated
// summation, so it can differ from a plain running sum in the last bits.
func pySum(xs []float64) float64 {
	if len(xs) == 0 {
		return 0
	}
	total, c := 0.0+xs[0], 0.0
	for _, x := range xs[1:] {
		t := total + x
		if math.Abs(total) >= math.Abs(x) {
			c += (total - t) + x
		} else {
			c += (x - t) + total
		}
		total = t
	}
	if c != 0 && !math.IsInf(c, 0) && !math.IsNaN(c) {
		total += c
	}
	return total
}

// parseISO is clock.parse_iso on a JSON value: None (the zero time) when it is falsy.
func parseISO(v any) time.Time {
	if !pyjson.Truthy(v) {
		return time.Time{}
	}
	return clock.ParseISO(py.Str(v))
}

// minus is a - b between two parsed times; a None among them raises TypeError, as in Python.
func minus(a, b time.Time) time.Duration {
	if a.IsZero() || b.IsZero() {
		panic(&pyError{"TypeError", "unsupported operand type(s) for -: 'datetime.datetime' and 'NoneType'"})
	}
	return a.Sub(b)
}

// medianDuration is statistics.median over timedeltas: the middle one, or the two middle ones'
// sum halved to the microsecond, a tie to even (timedelta / 2).
func medianDuration(took []time.Duration) time.Duration {
	data := append([]time.Duration{}, took...)
	sort.SliceStable(data, func(i, j int) bool { return data[i] < data[j] })
	n := len(data)
	if n%2 == 1 {
		return data[n/2]
	}
	us := int((data[n/2-1] + data[n/2]) / time.Microsecond)
	q, r := py.FloorDiv(us, 2), py.Mod(us, 2)
	if r*2 == 2 && py.Mod(q, 2) == 1 {
		q++
	}
	return time.Duration(q) * time.Microsecond
}

// Counter is collections.Counter over strings: a missing key counts 0 (reading it adds nothing),
// keys keep the order they were first counted in, and MostCommon puts equal counts in that order,
// as Python's does.
type Counter struct {
	keys []string
	n    map[string]int
}

// Count is one of Counter.MostCommon's (key, count) pairs.
type Count struct {
	Key string
	N   int
}

func NewCounter() *Counter { return &Counter{n: map[string]int{}} }

// Get is c[key].
func (c *Counter) Get(key string) int { return c.n[key] }

// Add is c[key] += by (the key is counted even when by is 0).
func (c *Counter) Add(key string, by int) {
	if _, ok := c.n[key]; !ok {
		c.keys = append(c.keys, key)
	}
	c.n[key] += by
}

// Len is len(c): the keys counted, whatever their counts.
func (c *Counter) Len() int { return len(c.keys) }

// Keys is list(c).
func (c *Counter) Keys() []string { return append([]string(nil), c.keys...) }

// Total is sum(c.values()).
func (c *Counter) Total() int {
	sum := 0
	for _, k := range c.keys {
		sum += c.n[k]
	}
	return sum
}

// MostCommon is c.most_common(n), every key when n < 0.
func (c *Counter) MostCommon(n int) []Count {
	out := make([]Count, len(c.keys))
	for i, k := range c.keys {
		out[i] = Count{k, c.n[k]}
	}
	sort.SliceStable(out, func(i, j int) bool { return out[i].N > out[j].N })
	if n >= 0 && n < len(out) {
		out = out[:n]
	}
	return out
}

// counterAt is `d[key]` on a defaultdict(Counter): the key's Counter, made (and kept) when missing.
func counterAt(d *pyjson.Object, key string) *Counter {
	if c, ok := d.Get(key).(*Counter); ok {
		return c
	}
	c := NewCounter()
	d.Set(key, c)
	return c
}
