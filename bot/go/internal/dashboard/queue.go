package dashboard

// ------------------------------------------------------------------ the queue

import (
	"fmt"
	"sort"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/waiting"
)

// DifficultyLegend is difficulty_legend: what each difficulty may be built by, in one line above
// the queue.
func DifficultyLegend(pool *providers.Pool) string {
	names := map[string]string{}
	for _, tier := range []string{"weak", "medium", "strong"} {
		names[tier] = waiting.TierNames(pool, tier)
	}
	weak := ""
	if names["weak"] != "none" {
		weak = ", the weak ones first (" + names["weak"] + ")"
	}
	return "<sub>🟢 **easy**: any model" + weak + " · 🟡 **medium**, and ⚪ **unrated** until its " +
		"planner rates it: " +
		"a medium or strong model (" + names["medium"] + ", then " + names["strong"] + ") · 🔴 **hard**: " +
		"a strong model only (" + names["strong"] + "). A build needs a plan first, by a medium " +
		"or strong model (strong for a hard one); a review run of a medium or hard change " +
		"needs a strong model; a revision that answers a strong audit stays strong.</sub>"
}

// Step is STEP: what a run does to a queued item, as the queue says it.
var Step = map[string]string{"plan": "plan", "build": "build", "revise": "revise", "review": "review run"}

// startsOf is a row's `starts`: the assignment that takes it at the next run, or nil.
func startsOf(row *waiting.Row) *plan.Assignment {
	var starts any = row.Starts
	a, _ := starts.(*plan.Assignment)
	return a
}

// startsText is _starts_text.
func startsText(assignment *plan.Assignment) string {
	var seat *providers.Seat
	switch assignment.Action {
	case "plan":
		seat = assignment.Plan
	case "review":
		seat = assignment.Review
	default:
		seat = assignment.Build
	}
	for _, s := range []*providers.Seat{assignment.Plan, assignment.Build, assignment.Review} {
		if seat == nil {
			seat = s
		}
	}
	if seat == nil {
		return "▶️ **starts at the next run** on `" + assignment.Provider.ID + "`"
	}
	return "▶️ **starts at the next run** on `" + seat.Provider.ID + "` (`" + seat.Model + "`, " +
		seat.Tier + ")"
}

// QueueTable is queue_table: the queue in the planner's order: what the next run does to each
// item, its difficulty and the tier that needs, and what takes it next or what it waits for.
func QueueTable(ctx *hctx.Context, explained *waiting.Explained, issues []*pyjson.Object) []string {
	rows := explained.Rows
	modes := map[int]string{}
	for _, i := range issues {
		modes[py.MustInt(i.Get("number"))] = queue.ModeOf(queue.LabelNames(i))
	}
	var lines []string
	for _, row := range upTo(rows, QueueRows) {
		if row.Held {
			lines = append(lines, fmt.Sprintf("| #%d | %s | — | — | ⏸️ held back: %s | — |", row.Number,
				Cell(row.Title, 55), row.Waiting))
			continue
		}
		step, ok := Step[row.Step]
		if !ok {
			step = row.Step
		}
		starts := startsOf(row)
		if starts != nil && starts.Action == "build" && starts.Plan != nil {
			step = "plan, then build" // no planner free on the planning lane: its builder plans
		}
		if mode := modes[row.Number]; mode != "" && row.Step == "build" {
			if kind, ok := ModeKind[mode]; ok {
				step = kind
			}
		}
		difficulty := waiting.DifficultyMark[row.Difficulty] + " " + row.Difficulty + " → " + row.Tier
		if row.TierWhy != "" {
			difficulty += "<br/><sub>" + row.TierWhy + "</sub>"
		}
		what := row.Waiting
		if starts != nil {
			what = startsText(starts)
		}
		forced := ""
		if row.Forced {
			forced = " (forced)"
		}
		priority := row.Priority
		if priority == "none" {
			priority = "—"
		}
		lines = append(lines, fmt.Sprintf("| #%d | %s | %s%s | %s | %s | %s |", row.Number,
			Cell(row.Title, 55), step, forced, difficulty, what, priority))
	}
	if len(lines) == 0 {
		return []string{"Nothing is queued."}
	}
	out := append([]string{"| Item | Title | Next step | Difficulty → needs | Starts, or waits for | Priority |",
		"|---|---|---|---|---|---|"}, lines...)
	if more := len(rows) - QueueRows; more > 0 {
		out = append(out, fmt.Sprintf("\n…and %d more.", more))
	}
	return out
}

// ModeKind is MODE_KIND: a queued issue's kind in a bot with modes (#60).
var ModeKind = map[string]string{"oneshot": "one-shot (fullsend)", "split": "split",
	"split-bot": "split for the night bot"}

// WaitingForPeople is waiting_for_people: the items waiting for a person (`bot:blocked`), in a
// line, or "".
func WaitingForPeople(issues []*pyjson.Object) string {
	blocked := config.LabelBlocked
	var numbers []string
	for _, i := range issues {
		if queue.LabelNames(i)[blocked] {
			numbers = append(numbers, "#"+py.Str(i.Get("number")))
		}
	}
	if len(numbers) == 0 {
		return ""
	}
	return "👤 **Waiting for a person:** " + strings.Join(numbers, ", ") + "."
}

// Headline is headline: one line: how many runs go, what the queue waits on, and which
// subscriptions can start work now.
func Headline(ctx *hctx.Context, live *pyjson.Object, explained *waiting.Explained) string {
	var queued []*waiting.Row
	for _, r := range explained.Rows {
		if !r.Held {
			queued = append(queued, r)
		}
	}
	starting := 0
	waits := pyjson.New() // what the item waits for -> how many, in order
	for _, row := range queued {
		if startsOf(row) != nil {
			starting++
			continue
		}
		key := ""
		if row.Step == "plan" {
			key = "a " + row.Tier + " planner"
		} else {
			key = lookup(waiting.TierWords, row.Tier)
		}
		waits.Set(key, waits.Int(key)+1)
	}
	held := len(explained.Rows) - len(queued)
	kinds := map[string][]string{}
	explained.Now.Items(func(providerID string, v any) bool {
		now := nowFor(explained.Now, providerID)
		kinds[now.Kind] = append(kinds[now.Kind], providerID)
		return true
	})
	parts := []string{fmt.Sprintf("🟢 **%d running**", live.Len())}
	queueText := fmt.Sprintf("📋 **%d queued**", len(queued))
	var details []string
	if starting > 0 {
		verb := "start"
		if starting == 1 {
			verb = "starts"
		}
		details = append(details, fmt.Sprintf("%d %s at the next run", starting, verb))
	}
	type count struct {
		what string
		n    int
	}
	var counts []count
	waits.Items(func(what string, n any) bool {
		counts = append(counts, count{what, n.(int)})
		return true
	})
	sort.SliceStable(counts, func(i, j int) bool { return counts[i].n > counts[j].n })
	for _, c := range counts {
		verb := "wait"
		if c.n == 1 {
			verb = "waits"
		}
		details = append(details, fmt.Sprintf("%d %s for %s", c.n, verb, c.what))
	}
	if len(details) > 0 {
		queueText += ": " + strings.Join(details, ", ")
	}
	if held != 0 {
		queueText += fmt.Sprintf(" (%d more held back)", held)
	}
	parts = append(parts, queueText)
	ready := append(append([]string{}, kinds["idle"]...), kinds["next"]...)
	if len(ready) > 0 {
		var named []string
		for _, p := range ready {
			named = append(named, "`"+p+"`")
		}
		parts = append(parts, fmt.Sprintf("🔋 **%d of %d subscriptions can start work now**: %s",
			len(ready), explained.Now.Len(), strings.Join(named, ", ")))
	} else {
		parts = append(parts, fmt.Sprintf("🔋 **none of %d subscriptions can start work now**",
			explained.Now.Len()))
	}
	return strings.Join(parts, " · ")
}
