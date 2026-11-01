package py

import (
	"cmp"
	"fmt"
	"math"
	"slices"
	"sort"
	"strings"
)

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

// NewCounter is Counter().
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

// Sum is sum(xs) over floats in Python 3.12 and later: the int 0 plus the first (so -0.0 sums
// to 0.0), then Neumaier's compensated summation, so it can differ from a plain running sum in
// the last bits (sum([0.1] * 10) is 1.0).
func Sum(xs []float64) float64 {
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

// Percent is f"{x:.{prec}%}": x times 100, fixed to prec decimals (rounded half to even, as
// both languages round a decimal they hold exactly), and a percent sign.
func Percent(x float64, prec int) string { return fmt.Sprintf("%.*f%%", prec, x*100) }

// Compare compares two sort keys as Python compares tuples: element by element, the first
// difference deciding, and a key that is a prefix of the other first. An element is a number (an
// int, a float, or a bool: False before True, and equal to 0 and 1 as in Python), a string (by
// code point), a [2]int or a nested key.
func Compare(a, b []any) int {
	for i := 0; i < len(a) && i < len(b); i++ {
		if c := compareElem(a[i], b[i]); c != 0 {
			return c
		}
	}
	return cmp.Compare(len(a), len(b))
}

func compareElem(a, b any) int {
	if x, ok := keyInt(a); ok {
		if y, ok := keyInt(b); ok {
			return cmp.Compare(x, y)
		}
	}
	if x, ok := keyFloat(a); ok {
		y, _ := keyFloat(b)
		return cmp.Compare(x, y)
	}
	switch x := a.(type) {
	case string:
		return strings.Compare(x, b.(string)) // UTF-8 byte order is code point order
	case [2]int:
		y := b.([2]int)
		return Compare([]any{x[0], x[1]}, []any{y[0], y[1]})
	case []any:
		return Compare(x, b.([]any))
	}
	panic(fmt.Sprintf("py: cannot compare %T in a sort key", a))
}

func keyInt(v any) (int, bool) {
	switch x := v.(type) {
	case int:
		return x, true
	case bool:
		if x {
			return 1, true
		}
		return 0, true
	}
	return 0, false
}

func keyFloat(v any) (float64, bool) {
	if n, ok := keyInt(v); ok {
		return float64(n), true
	}
	f, ok := v.(float64)
	return f, ok
}

// SortedBy is sorted(xs, key=key): a new slice, stable, each key computed once.
func SortedBy[T any](xs []T, key func(T) []any) []T {
	type keyed struct {
		k []any
		v T
	}
	all := make([]keyed, len(xs))
	for i, x := range xs {
		all[i] = keyed{key(x), x}
	}
	slices.SortStableFunc(all, func(a, b keyed) int { return Compare(a.k, b.k) })
	out := make([]T, len(xs))
	for i, e := range all {
		out[i] = e.v
	}
	return out
}

// MinBy is min(xs, key=key): the first of the smallest. xs is not empty.
func MinBy[T any](xs []T, key func(T) []any) T {
	best, bestKey := xs[0], key(xs[0])
	for _, x := range xs[1:] {
		if k := key(x); Compare(k, bestKey) < 0 {
			best, bestKey = x, k
		}
	}
	return best
}
