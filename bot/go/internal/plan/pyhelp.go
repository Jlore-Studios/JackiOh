package plan

// Private helpers for the Python idioms plan.py leans on that py does not hold.

import (
	"cmp"
	"fmt"
	"slices"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// cmpKey compares two sort keys as Python compares tuples: element by element, the first
// difference deciding, and a key that is a prefix of the other first. An element is an int, a
// bool (False before True, and equal to 0 and 1 as in Python), a string, a [2]int or a nested key.
func cmpKey(a, b []any) int {
	for i := 0; i < len(a) && i < len(b); i++ {
		if c := cmpElem(a[i], b[i]); c != 0 {
			return c
		}
	}
	return cmp.Compare(len(a), len(b))
}

func cmpElem(a, b any) int {
	if x, ok := keyInt(a); ok {
		y, _ := keyInt(b)
		return cmp.Compare(x, y)
	}
	switch x := a.(type) {
	case string:
		return strings.Compare(x, b.(string)) // UTF-8 byte order is code point order
	case [2]int:
		y := b.([2]int)
		return cmpKey([]any{x[0], x[1]}, []any{y[0], y[1]})
	case []any:
		return cmpKey(x, b.([]any))
	}
	panic(fmt.Sprintf("plan: cannot compare %T in a sort key", a))
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

// sortedBy is sorted(xs, key=key): a new slice, stable, each key computed once.
func sortedBy[T any](xs []T, key func(T) []any) []T {
	type keyed struct {
		k []any
		v T
	}
	all := make([]keyed, len(xs))
	for i, x := range xs {
		all[i] = keyed{key(x), x}
	}
	slices.SortStableFunc(all, func(a, b keyed) int { return cmpKey(a.k, b.k) })
	out := make([]T, len(xs))
	for i, e := range all {
		out[i] = e.v
	}
	return out
}

// minBy is min(xs, key=key): the first of the smallest. xs is not empty.
func minBy[T any](xs []T, key func(T) []any) T {
	best, bestKey := xs[0], key(xs[0])
	for _, x := range xs[1:] {
		if k := key(x); cmpKey(k, bestKey) < 0 {
			best, bestKey = x, k
		}
	}
	return best
}

// filter is [x for x in xs if keep(x)].
func filter[T any](xs []T, keep func(T) bool) []T {
	var out []T
	for _, x := range xs {
		if keep(x) {
			out = append(out, x)
		}
	}
	return out
}

// sortedInts is sorted(set).
func sortedInts(set map[int]bool) []int {
	out := make([]int, 0, len(set))
	for n := range set {
		out = append(out, n)
	}
	slices.Sort(out)
	return out
}

// eqStr is `v == s` for a JSON value v and a string s.
func eqStr(v any, s string) bool {
	x, ok := v.(string)
	return ok && x == s
}

// intPtr is an `int | None` read from JSON: nil for None, else int(v).
func intPtr(v any) *int {
	if v == nil {
		return nil
	}
	n := py.MustInt(v)
	return &n
}

// intOrNone is the JSON value of an `int | None`.
func intOrNone(n *int) any {
	if n == nil {
		return nil
	}
	return *n
}

// recordOf is `state["items"].get(key, {})`.
func recordOf(state *pyjson.Object, key string) *pyjson.Object {
	if record := state.Obj("items").Obj(key); record != nil {
		return record
	}
	return pyjson.New()
}

// joinNonEmpty is sep.join(p for p in parts if p).
func joinNonEmpty(sep string, parts ...string) string {
	return strings.Join(filter(parts, func(p string) bool { return p != "" }), sep)
}

// tierRank is providers.TIER_RANK, `{name: rank for rank, name in enumerate(TIERS)}`.
var tierRank = func() map[string]int {
	ranks := map[string]int{}
	for rank, name := range providers.Tiers {
		ranks[name] = rank
	}
	return ranks
}()

// rankOr is TIER_RANK.get(tier, def).
func rankOr(tier string, def int) int {
	if rank, ok := tierRank[tier]; ok {
		return rank
	}
	return def
}

// difficultyIndex is DIFFICULTIES.index(difficulty).
func difficultyIndex(difficulty string) int { return slices.Index(config.Difficulties, difficulty) }
