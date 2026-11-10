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
	var table []string
	if total > 0 {
		table = append(table, "| Subscription | Runs on | Spare lanes | Takes |", "|---|---|---|---|")
		for _, s := range spare {
			takes := "🟡 medium"
			if s.Hard {
				takes = "🔴 hard or 🟡 medium"
			}
			table = append(table, fmt.Sprintf("| `%s` | %s | %d | %s |", s.Provider.ID,
				lanesWhere(pool, s.Provider), s.Lanes, takes))
		}
		table = append(table, fmt.Sprintf("| **All** | | **%d** | **%d hard or medium, %d medium** |",
			total, hard, total-hard), "")
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
		"as a medium one; a medium model only a medium one.</sub>")
	if cfg := ctx.Cfg; cfg.SuggestionsEnabled {
		table = append(table, "", fmt.Sprintf("<sub>With %d or more spare, the bot runs a suggestion "+
			"survey (`suggestions.spare_lanes`), up to %d open suggestions (`%s`: %d open now).</sub>",
			cfg.SuggestionsSpareLanes, cfg.SuggestionsMaxOpen, config.LabelSuggestion,
			openSuggestions(issues)))
	}
	return append(lines, Fold("Which subscriptions have lanes spare, and what counts", table)...)
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
