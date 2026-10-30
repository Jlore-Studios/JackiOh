// Package waiting is bot/harness/waiting.py.
//
// What the queue waits for, and what each subscription can do now: the status issue's Queue and
// its Subscriptions (#148).
//
// Worked out with the planner's own rules, never a copy of them: the queue is `queue.candidates`,
// and who takes what is `plan.pairs` run on the lanes held now, one item at a time as the night
// runs claim them (`plan.fill_lanes`). So the issue says an item starts next only when the planner
// would start it, and calls a subscription idle only when it may start work and nothing queued is
// for it. An item no run can take says the tier its difficulty needs (an Opus for a hard one, or a
// revision that answers a strong audit), which subscriptions have such a seat, and when the first
// of them can start one: at a reset, at its hours, after its back-off, or when its run ends.
package waiting

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"time"
)

// TIER_WORDS = {"weak": "any model", "medium": "a medium or strong model", "strong": "a strong model"}
var TierWords = map[string]string{"weak": "any model", "medium": "a medium or strong model", "strong": "a strong model"}

// DIFFICULTY_MARK = {"easy": "🟢", "medium": "🟡", "hard": "🔴", "unrated": "⚪"}
var DifficultyMark = map[string]string{"easy": "🟢", "medium": "🟡", "hard": "🔴", "unrated": "⚪"}

// One subscription now: `kind` is busy, next (the planner gives it an item at the next
// run), idle (it may start work, and nothing queued is for it), reviews (capped for long runs,
// but under the cap a review goes up to), full (no lane free where it runs), capped, closed
// (outside its hours), broken (backing off after a run that could not work) or off. `until` is
// when it can start a build or a plan again, when that is known.

type Now struct {
	Kind  string
	Text  string
	Until time.Time // default: None
	Items []int     // default: field(default_factory=list)
}

// One queued item, and what it waits for.

type Row struct {
	Number     int
	Title      string
	Step       string
	Difficulty string
	Tier       string
	TierWhy    string
	Priority   string
	Starts     any    // default: None
	Waiting    string // default: ""
	Held       bool   // default: False
	Forced     bool   // default: False
}

// Explained is waiting.Explained.

type Explained struct {
	Rows   []*Row
	Now    *pyjson.Object
	Halted string // default: ""
}

// The models of `tier`, short: "Opus", or "Muse, agy (Gemini), Sonnet".

func TierNames(pool *providers.Pool, tier string) string {
	panic("unported: waiting.tier_names")
}

// `plan.read_lanes` from the lanes the issue has read already (`status.lanes_now`), so the
// queue costs no second look at the runs.

func LanesOf(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object) *plan.Lanes {
	panic("unported: waiting.lanes_of")
}

// (step, tier, why): what the next run does to `c`, the weakest tier that may do it
// (`plan.assign`, `plan.review_seat`), and why that tier is not its difficulty's own.

func Need(c *queue.Candidate) (string, string, string) {
	panic("unported: waiting.need")
}

// `20:27` today (Central time), `Fri 18:47` on another day.

func Clock(ctx *hctx.Context, at time.Time) string {
	panic("unported: waiting._clock")
}

// Why `provider` may not start a run now, as a `Now`, or None when it may (its lanes
// aside). In the order `providers.availability` checks.

func HeldBy(ctx *hctx.Context, state *pyjson.Object, provider *providers.Provider, forced bool /* = False */, starting bool /* = True */) *Now {
	panic("unported: waiting.held_by")
}

// Which limit holds a capped subscription, and when it can start a build or a plan again.

func CappedText(ctx *hctx.Context, provider *providers.Provider, entry *pyjson.Object, frees time.Time, reviews bool /* = False */) string {
	panic("unported: waiting.capped_text")
}

// The queue in the planner's order, each item with what takes it next or what it waits
// for, and each subscription now.

func Explain(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object, issues []*pyjson.Object /* = () */, halted string /* = "" */) *Explained {
	panic("unported: waiting.explain")
}

// The labelled items the planner leaves out, each with why.

func HeldRows(heldBack *pyjson.Object, issues any) []*Row {
	panic("unported: waiting.held_rows")
}

// Each subscription now. Busy is a run going; next is an item the planner gives it at the
// next run; idle is free to start and nothing queued is for it. One that may start but has no
// lane where it runs is full, never free.

func Subscriptions(ctx *hctx.Context, state *pyjson.Object, lanes *plan.Lanes, after *plan.Lanes, starts *pyjson.Object, halted string /* = "" */) *pyjson.Object {
	panic("unported: waiting.subscriptions")
}

// Why no run takes `c` now: the kind of model it needs, and which of them frees up first.

func WaitsFor(ctx *hctx.Context, state *pyjson.Object, c *queue.Candidate, step string, tier string, lanes *plan.Lanes, now *pyjson.Object) string {
	panic("unported: waiting.waits_for")
}
