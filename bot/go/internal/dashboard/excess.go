package dashboard

// ------------------------------------------------------------------ the excess compute

import (
	"fmt"
	"slices"
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

// ExcessHeading is the excess compute's heading, the issue's first section (#3659).
var ExcessHeading = "## 🧮 Excess compute"

// ExcessSection is the excess compute (#3659), the issue's first section, under the headline:
// whether lanes are spare (`plan.Spare`: free on subscriptions that may start a build or a plan
// now, a runner having room for them, once the planner has handed out every queued item it can
// start), so whether people should open more issues, and of which difficulty: a subscription with
// a strong seat (Opus) takes a hard issue or a medium one, any other a medium one only. A verdict
// line, then a table by subscription and what counts, folded. GitHub's runners are shared with the
// other bots, so their runs there (`others`, their sections, `MarkedLanes`) take their room first.
// `issues` are the open threads, for the suggestions open now.
func ExcessSection(ctx *hctx.Context, state *pyjson.Object, explained *waiting.Explained,
	others []string, issues []*pyjson.Object) []string {
	pool := ctx.Cfg.Pool
	lines := []string{ExcessHeading, ""}
	if explained.Halted != "" || explained.After == nil {
		why := "nothing may start"
		if explained.Halted != "" {
			why = "the bot is halted"
		}
		return append(lines, "> ⚪ **No excess compute**: "+why+".", "")
	}
	after := *explained.After
	after.Shared = othersOnGitHub(others)
	spare := plan.Spare(ctx, state, &after, false)
	total := plan.SpareTotal(spare)
	var hardIDs, mediumIDs []string
	hard := 0
	for _, s := range spare {
		if s.Hard {
			hard += s.Lanes
			hardIDs = append(hardIDs, "`"+s.Provider.ID+"`")
		} else {
			mediumIDs = append(mediumIDs, "`"+s.Provider.ID+"`")
		}
	}
	if total == 0 {
		lines = append(lines, "> ⚪ **No excess compute**: every lane that may start work now has "+
			"queued work for it, or none may start a build or a plan (each is busy, near a cap, "+
			"outside its hours or off). No more issues are needed for now.", "")
	} else {
		var parts []string
		if hard > 0 {
			parts = append(parts, fmt.Sprintf("**%d hard or medium**, for %s (a strong model takes "+
				"either)", hard, strings.Join(hardIDs, ", ")))
		}
		if medium := total - hard; medium > 0 {
			parts = append(parts, fmt.Sprintf("**%d medium**, for %s (medium models only)", medium,
				strings.Join(mediumIDs, ", ")))
		}
		lines = append(lines, fmt.Sprintf("> 🟢 **%s spare**: free on subscriptions that may start "+
			"work now, and nothing queued is for them. **Queue %s** to fill them: %s.",
			plural(total, "lane", "lanes"), plural(total, "more issue", "more issues"),
			strings.Join(parts, ", and ")), "")
	}
	idle := Idle(ctx, state, explained, &after, spare)
	if total > 0 {
		lines[len(lines)-2] += " Why: " + IdleWhy[MostIdle(idle)] + "."
	}
	var table []string
	if len(idle) > 0 {
		table = append(table, "| Subscription | Runs on | Free | Spare | Takes | Why it isn't working |",
			"|---|---|---|---|---|---|")
		free := 0
		for _, row := range idle {
			takes := "—"
			if row.Spare > 0 {
				takes = "🟡 medium"
				if row.Hard {
					takes = "🔴 hard or 🟡 medium"
				}
			}
			free += row.Free
			table = append(table, fmt.Sprintf("| `%s` | %s | %d | %d | %s | %s |", row.Provider.ID,
				lanesWhere(pool, row.Provider), row.Free, row.Spare, takes, row.Why))
		}
		if total > 0 {
			table = append(table, fmt.Sprintf("| **All** | | **%d** | **%d** | **%d hard or medium, %d "+
				"medium** | |", free, total, hard, total-hard))
		}
		table = append(table, "")
	}
	headroom := ""
	if text := headroomText(pool); text != "" {
		headroom = ", " + text + " under it as `.harness/providers.json` sets it"
	}
	where := fmt.Sprintf("GitHub's runners have %d free", after.Free())
	if pool.MachineParallel != 0 {
		where += fmt.Sprintf(", the AWS boxes %d", after.MachineFree())
	}
	table = append(table, "<sub>A lane is spare when it is free on a subscription that may start a "+
		"build or a plan now, a runner where it runs has room for it ("+where+", once the planner "+
		"has handed out every queued item it can start), and nothing queued is for it. A "+
		"subscription inside its start headroom of a cap (`start_headroom`"+headroom+"), outside "+
		"its hours, backing off or off counts none. A strong seat (Opus) takes a hard issue as well "+
		"as a medium one; a medium model only a medium one.</sub>", "", "<sub>"+IdleLegend+"</sub>")
	if cfg := ctx.Cfg; cfg.SuggestionsEnabled {
		table = append(table, "", fmt.Sprintf("<sub>With %d or more spare, the bot runs a suggestion "+
			"survey (`suggestions.spare_lanes`), up to %d open suggestions (`%s`: %d open now).</sub>",
			cfg.SuggestionsSpareLanes, cfg.SuggestionsMaxOpen, config.LabelSuggestion,
			openSuggestions(issues)))
	}
	return append(lines, Fold("Which subscriptions have lanes free, why each is not working, and what "+
		"counts", table)...)
}

// othersOnGitHub is how many runs the other bots hold on GitHub's runners now, from their sections
// (`MarkedLanes`): what `LaneLimits` counts against `max_parallel` beside this bot's.
func othersOnGitHub(others []string) int {
	held := 0
	for _, row := range MarkedLanes(others) {
		if row.Truthy("github") {
			held += row.Int("runs")
		}
	}
	return held
}

// openSuggestions is how many of `issues` (the open threads) are suggestions (`bot:suggestion`),
// pull requests aside.
func openSuggestions(issues []*pyjson.Object) int {
	n := 0
	for _, issue := range issues {
		if !issue.Has("pull_request") && queue.LabelNames(issue)[config.LabelSuggestion] {
			n++
		}
	}
	return n
}

// headroomText is each start headroom the pool sets (`start_headroom`), as percentages: "5%", or
// "" with none.
func headroomText(pool *providers.Pool) string {
	var found []string
	for _, p := range pool.Ordered() {
		p.Limits.Headroom.Items(func(_ string, v any) bool {
			if share, ok := pyjson.AsFloat(v); ok && share > 0 {
				if text := py.Percent(share, 0); !slices.Contains(found, text) {
					found = append(found, text)
				}
			}
			return true
		})
	}
	return strings.Join(found, " or ")
}

// plural is "1 lane" or "3 lanes".
func plural(n int, one, many string) string {
	if n == 1 {
		return "1 " + one
	}
	return fmt.Sprintf("%d %s", n, many)
}

// ------------------------------------------------------------------ why a free lane is idle

// Why a lane is free and not working (#3675). A spare lane (`plan.Spare`) is one nothing queued is
// for, and its subscription's cell counts how the queued items fall for it, each in the first of
// these that fits; a free lane that is not spare says why it may not start.
var (
	IdleNothingQueued = "📭 nothing queued"
	IdleOnHold        = "✋ on hold"
	IdleHeldFor       = "🔒 held for"
	IdleStarts        = "▶️ starting at the next run"
	IdleTooHard       = "🔴 too hard for it"
	IdleNotItsKind    = "🧩 not its kind of work"
	IdleWaits         = "⏳ waiting on something else"
	IdleNoRoom        = "🚫 no runner room"
	IdleCapped        = "🪫 near a cap"
	IdleClosed        = "🌙 outside its hours"
	IdleBroken        = "⛔ backing off"
	IdleOff           = "⚫ off"
)

// IdleWhy is the verdict line's word on why the spare lanes are not working: the state that holds
// the most of what is queued (`MostIdle`).
var IdleWhy = map[string]string{
	IdleNothingQueued: IdleNothingQueued[:strings.Index(IdleNothingQueued, " ")] + " nothing is queued",
	IdleOnHold:        "✋ what is queued is on hold",
	IdleHeldFor:       "🔒 what is queued is held for the subscriptions that own it",
	IdleStarts:        "▶️ what is queued starts on other subscriptions at the next run",
	IdleTooHard:       "🔴 what is queued is too hard for them",
	IdleNotItsKind:    "🧩 what is queued is not their kind of work",
	IdleWaits:         "⏳ what is queued waits on something else",
}

// IdleLegend says what each state means, under the table.
var IdleLegend = "Why it isn't working: on a spare lane, how the queued items fall for it: " +
	IdleNothingQueued + " (the queue is empty); " + IdleOnHold + " (the planner leaves them out: " +
	"`human`, a blocker, open parts); " + IdleHeldFor + " the subscription that owns them; " +
	IdleStarts + ", on another subscription; " + IdleTooHard + " (they need a stronger seat: a " +
	"hard item, or a revision that answers a strong audit, needs Opus); " + IdleNotItsKind + " (a " +
	"step it has no role for, a spot-check of its own build, or another lane's own review); " +
	IdleWaits + " (it could take them; they wait on something else). On a free lane that is not " +
	"spare: " + IdleNoRoom + " (no room where it runs); " + IdleCapped + " (inside `start_headroom` " +
	"of a cap, or it refused a call); " + IdleClosed + "; " + IdleBroken + " (its last run could " +
	"not work); " + IdleOff + " (switched off, suspended, or no secret set)."

// IdleRow is a subscription with a free lane: how many are free once the planner has handed out
// all it can start, how many of them are spare, whether those take a hard issue, and why it is not
// working.
type IdleRow struct {
	Provider *providers.Provider
	Free     int
	Spare    int
	Hard     bool
	Why      string
	// Counts is how the queued items fall for it, in the states' order (a spare one's).
	Counts []IdleCount
}

// IdleCount is how many queued items fall in one state, for one subscription.
type IdleCount struct {
	State string
	N     int
	Who   []string // for IdleHeldFor: the subscriptions they are held for
}

// Idle is every subscription that builds on a medium or strong seat (those `plan.Spare` counts)
// with a lane free on `after`, the lanes once the planner has handed out every queued item it can
// start, in the pool's order, and why each is not working (#3675).
func Idle(ctx *hctx.Context, state *pyjson.Object, explained *waiting.Explained, after *plan.Lanes,
	spare []plan.SpareLanes) []IdleRow {
	pool := ctx.Cfg.Pool
	spareOf := map[string]plan.SpareLanes{}
	for _, s := range spare {
		spareOf[s.Provider.ID] = s
	}
	var found []IdleRow
	for _, p := range pool.Ordered() {
		if pool.BestSeat(p, "medium") == nil || !slices.Contains(p.Roles, "build") {
			continue
		}
		free := max(0, after.LanesOf(p)-after.Used(p))
		s, spare := spareOf[p.ID]
		if free == 0 && !spare {
			continue
		}
		row := IdleRow{Provider: p, Free: max(free, s.Lanes)} // a fill-in's spare lanes are its owners'
		if spare {
			row.Spare, row.Hard = s.Lanes, s.Hard
			row.Counts = idleCounts(pool, p, explained.Rows)
			row.Why = idleText(row.Counts)
			if more := row.Free - row.Spare; more > 0 {
				row.Why += fmt.Sprintf("; %s for its other %d", IdleNoRoom, more)
			}
		} else {
			row.Why = notSpare(ctx, state, explained, p, after)
		}
		found = append(found, row)
	}
	return found
}

// idleCounts is how the queued items `rows` fall for `p`, whose lanes are spare: each in the first
// state that fits it.
func idleCounts(pool *providers.Pool, p *providers.Provider, rows []*waiting.Row) []IdleCount {
	order := []string{IdleOnHold, IdleHeldFor, IdleStarts, IdleTooHard, IdleNotItsKind, IdleWaits}
	counts := map[string]*IdleCount{}
	for _, state := range order {
		counts[state] = &IdleCount{State: state}
	}
	for _, row := range rows {
		state := IdleWaits
		switch {
		case row.Held:
			state = IdleOnHold
		case row.HeldFor != "" && row.HeldFor != p.ID:
			state = IdleHeldFor
			if !slices.Contains(counts[state].Who, row.HeldFor) {
				counts[state].Who = append(counts[state].Who, row.HeldFor)
			}
		case row.Starts != nil:
			state = IdleStarts
		case !slices.Contains(row.Able, p.ID):
			role, ok := plan.RoleOf[row.Step]
			if !ok {
				role = row.Step
			}
			state = IdleNotItsKind
			if slices.Contains(p.Roles, role) && row.Tier != "" && pool.BestSeat(p, row.Tier) == nil {
				state = IdleTooHard
			}
		}
		counts[state].N++
	}
	var found []IdleCount
	for _, state := range order {
		if counts[state].N > 0 {
			found = append(found, *counts[state])
		}
	}
	return found
}

// idleText is a spare subscription's cell: each state its queued items fall in, with how many, or
// that nothing is queued.
func idleText(counts []IdleCount) string {
	if len(counts) == 0 {
		return IdleNothingQueued
	}
	var parts []string
	for _, c := range counts {
		mark, words, _ := strings.Cut(c.State, " ")
		part := fmt.Sprintf("%s %d %s", mark, c.N, words)
		if c.State == IdleHeldFor {
			var who []string
			for _, id := range c.Who {
				who = append(who, "`"+id+"`")
			}
			part += " " + strings.Join(who, ", ")
		}
		parts = append(parts, part)
	}
	return strings.Join(parts, " · ")
}

// notSpare is why `p`'s free lanes are not spare: it may not start a build or a plan now (as its
// box says, `waiting.HeldBy`), or there is no room where it runs.
func notSpare(ctx *hctx.Context, state *pyjson.Object, explained *waiting.Explained, p *providers.Provider,
	after *plan.Lanes) string {
	if held := waiting.HeldBy(ctx, state, p, false, true); held != nil {
		_, detail, _ := strings.Cut(held.Text, " ")
		why := IdleCapped
		switch held.Kind {
		case "off":
			return IdleOff + ": " + strings.TrimPrefix(detail, "off: ") // "⚫ off: no secret set"
		case "closed":
			why = IdleClosed
		case "broken":
			why = IdleBroken
		}
		if _, rest, cut := strings.Cut(detail, "<br/>"); cut && why != IdleCapped {
			detail = rest // "opens 18:00", "tries again 14:20"
		}
		return why + ": " + strings.ReplaceAll(detail, "<br/>", ", ")
	}
	if now, ok := explained.Now.Get(p.ID).(*waiting.Now); ok && now != nil && now.Kind == "full" {
		_, detail, _ := strings.Cut(now.Text, " ")
		return IdleNoRoom + ": " + strings.ReplaceAll(detail, "<br/>", " ")
	}
	if after.Machine[p.ID] {
		return IdleNoRoom + ": the AWS boxes have no slot free for it"
	}
	return IdleNoRoom + ": GitHub's runners have no room for it"
}

// MostIdle is the state that holds the most of the spare subscriptions' queued items (nothing
// queued when none holds any), for the verdict line.
func MostIdle(rows []IdleRow) string {
	totals := map[string]int{}
	var order []string
	for _, row := range rows {
		if row.Spare == 0 {
			continue
		}
		if len(row.Counts) == 0 {
			if totals[IdleNothingQueued] == 0 {
				order = append(order, IdleNothingQueued)
			}
			totals[IdleNothingQueued] += row.Spare
		}
		for _, c := range row.Counts {
			if totals[c.State] == 0 {
				order = append(order, c.State)
			}
			totals[c.State] += c.N
		}
	}
	best := IdleNothingQueued
	for _, state := range order {
		if totals[state] > totals[best] {
			best = state
		}
	}
	return best
}
