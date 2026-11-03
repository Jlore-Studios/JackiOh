package providers

import (
	"slices"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// LanesByCap is `lanes_by_cap` in providers.json (#2748, `{"cli": "claude", "line": 0.5, "low":
// 1, "high": 2}`): how many lanes a subscription on its CLI may hold, decided when a run starts
// from the 5-hour cap in force at that moment (`Provider.CapsAt`: its own, or its `off_hours`
// one outside its window, raised by its late week, and lifted in a session's last hour): above
// `Line` up to `High`, at `Line` or under `Low`, and `High` with no 5-hour cap. It replaces the
// `lanes` of the subscriptions it matches (one that names its own is refused), whose `lanes`
// reads as `High`, the most they may hold; every other subscription keeps its own. A run already
// going is never stopped when the count drops: it holds its lane until it ends, and the
// subscription starts nothing more while its runs fill the lanes in force.
type LanesByCap struct {
	CLI  string
	Line float64
	Low  int
	High int
}

// Matches is: the rule decides `provider`'s lanes.
func (x *LanesByCap) Matches(provider *Provider) bool {
	return x != nil && provider.CLI == x.CLI
}

// LanesAt is how many lanes `provider` may hold for a run that starts at `at`: for one
// `lanes_by_cap` matches, the rule's by the 5-hour cap in force then, read with its stored
// reading as the planner reads its caps (its last hour and its late week show only on a
// reading); for any other, its own `lanes`.
func (x *Pool) LanesAt(provider *Provider, state *pyjson.Object, at time.Time, zoneName string) int {
	rule := x.LanesByCap
	if !rule.Matches(provider) {
		return provider.Lanes
	}
	usage := CurrentUsage(provider, PeekRecord(state, provider.ID))
	limit, capped := provider.CapsAt(at, zoneName, usage, time.Time{}).Lookup("five_hour")
	if !capped || limit == nil || py.MustFloat(limit) > rule.Line {
		return rule.High
	}
	return rule.Low
}

// lanesByCapKeys are `lanes_by_cap`'s keys.
var lanesByCapKeys = []string{"cli", "line", "low", "high", "note"}

// lanesByCap reads `lanes_by_cap`: nil when providers.json has none.
func lanesByCap(raw any, where string) *LanesByCap {
	if raw == nil {
		return nil
	}
	obj, ok := mapping(raw)
	if !ok {
		configError(`%s: expected an object such as {"cli": "claude", "line": 0.5, "low": 1, "high": 2}`, where)
	}
	if unknown := unknownKeys(obj, lanesByCapKeys); len(unknown) > 0 {
		configError("%s: unknown keys %s", where, strings.Join(unknown, ", "))
	}
	for _, key := range []string{"cli", "line", "low", "high"} {
		if !obj.Has(key) {
			configError("%s: missing %s", where, key)
		}
	}
	cli := py.Str(obj.Get("cli"))
	if !slices.Contains(Clis, cli) {
		configError("%s.cli: %s is not one of %s", where, py.Repr(cli), strings.Join(Clis, ", "))
	}
	line := fraction(obj.Get("line"), where+".line")
	if line == nil {
		configError("%s.line: needs a share of the 5-hour window", where)
	}
	low, high := lanes(obj.Get("low"), where+".low"), lanes(obj.Get("high"), where+".high")
	if high < low {
		configError("%s: high (%d) is under low (%d)", where, high, low)
	}
	return &LanesByCap{CLI: cli, Line: *line, Low: low, High: high}
}
