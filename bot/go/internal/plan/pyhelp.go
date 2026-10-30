package plan

// Private helpers for the Python idioms plan.py leans on: tuple sort keys, `x or default`,
// `int()`, and the constants of other modules whose stubs still give them as `any`.

import (
	"cmp"
	"fmt"
	"slices"
	"strconv"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/identity"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
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

// inStrs is `v in (s, ...)` for a JSON value v.
func inStrs(v any, options ...string) bool {
	x, ok := v.(string)
	return ok && slices.Contains(options, x)
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

// cs is a config.py constant as a string (LABEL_WORKING, SLASH, …): str in the Python, `any`
// in config's stub until it is ported.
func cs(v any) string { return pyjson.Str(v) }

// othersOf is config.OTHERS as the tuple of `identity.Other` it is.
func othersOf(v any) []*identity.Other {
	others, _ := v.([]*identity.Other)
	return others
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

// tierModels is `[e.model for e in pool.tiers.get(tier, ())]`, whichever shape providers' port
// gives `Pool.tiers` (its stub's *pyjson.Object, or a map of `TierEntry` lists).
func tierModels(pool *providers.Pool, tier string) []string {
	var entries any
	switch tiers := any(pool.Tiers).(type) {
	case *pyjson.Object:
		entries = tiers.Get(tier)
	case map[string][]*providers.TierEntry:
		entries = tiers[tier]
	}
	var models []string
	switch list := entries.(type) {
	case []*providers.TierEntry:
		for _, entry := range list {
			models = append(models, entry.Model)
		}
	case []any:
		for _, e := range list {
			if entry, ok := e.(*providers.TierEntry); ok {
				models = append(models, entry.Model)
			}
		}
	}
	return models
}

// kindOrder is queue.KIND_ORDER[kind].
func kindOrder(kind string) int {
	switch order := any(queue.KindOrder).(type) {
	case map[string]int:
		return order[kind]
	case *pyjson.Object:
		return order.Int(kind)
	}
	panic("plan: queue.KIND_ORDER is not ported")
}

// priorityName is queue.PRIORITY_NAMES[priority].
func priorityName(priority int) string {
	switch names := any(queue.PriorityNames).(type) {
	case map[int]string:
		return names[priority]
	case *pyjson.Object:
		return names.Str(strconv.Itoa(priority))
	}
	panic("plan: queue.PRIORITY_NAMES is not ported")
}

// difficultyIndex is DIFFICULTIES.index(difficulty).
func difficultyIndex(difficulty string) int { return slices.Index(config.Difficulties, difficulty) }

// candidates is queue.candidates(ctx, state, skipped): queue extends `skipped` in place with a
// line for each item it passes over, as the Python does.
func candidates(ctx *hctx.Context, state *pyjson.Object, skipped *[]string) []*queue.Candidate {
	return queue.Candidates(ctx, state, skipped, nil)
}
