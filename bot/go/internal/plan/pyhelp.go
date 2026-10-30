package plan

// Private helpers for the Python idioms plan.py leans on that py does not hold.

import (
	"slices"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

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
