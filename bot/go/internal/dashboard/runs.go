package dashboard

// ------------------------------------------------------------------ trees, runs, Squishy

import (
	"fmt"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
)

// TreesTable is trees_table: the trees the bot split (#60): each parent, how many of its
// sub-issues have closed, and who builds them.
func TreesTable(ctx *hctx.Context, state *pyjson.Object) []string {
	var rows []string
	for _, issue := range ctx.GH.ListIssues(config.LabelTree, "open", 300, "") {
		if issue.Has("pull_request") {
			continue
		}
		number := py.MustInt(issue.Get("number"))
		var children []*pyjson.Object
		herr.Catch[*herr.GitHubError](func() { children = ctx.GH.ListSubIssues(number) })
		closed := 0
		for _, child := range children {
			if child.Str("state") == "closed" {
				closed++
			}
		}
		record := status.Record(state, number)
		tree, _ := record.Get("tree").(*pyjson.Object)
		who := config.Title
		if pyjson.Equal(tree.Get("mode"), "split-bot") {
			who = "the night bot"
		}
		done := "—"
		if len(children) > 0 {
			done = barOf(float64(closed)/float64(len(children)), BarCells)
		}
		rows = append(rows, fmt.Sprintf("| #%d | %s | %d of %d closed %s | %s |", number,
			Cell(issue.Get("title"), 50), closed, len(children), done, who))
	}
	if len(rows) == 0 {
		return []string{"No open trees."}
	}
	return append([]string{"| Parent | Title | Sub-issues | Built by |", "|---|---|---|---|"}, rows...)
}

// resultOf is `RESULT.get(str(run.get("conclusion")), str(run.get("conclusion")))`.
func resultOf(run *pyjson.Object) string {
	conclusion := py.Str(run.Get("conclusion"))
	if mark, ok := Result[conclusion]; ok {
		return mark
	}
	return conclusion
}

// runRow is a run as a row of the last runs (`runs_table`, `learn_lane`): when it started,
// hovering how long it took, its trigger, `result` and its link.
func runRow(ctx *hctx.Context, run *pyjson.Object, result string) string {
	at := clock.ParseISOValue(run.Get("created_at"))
	when := "?"
	if !at.IsZero() {
		when = hover(Local(ctx, at).Format("Mon 15:04"), took(ctx, run, at), "")
	}
	link := py.Str(run.GetOr("id", ""))
	if run.Truthy("html_url") {
		label := "run"
		if run.Truthy("run_number") {
			label = py.Str(run.Get("run_number"))
		}
		link = "[" + label + "](" + py.Str(run.Get("html_url")) + ")"
	}
	return "| " + when + " | " + Cell(run.Get("event"), 20) + " | " + result + " | " + link + " |"
}

// RunsTable is runs_table.
func RunsTable(ctx *hctx.Context) []string {
	var runs []*pyjson.Object
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		runs = ctx.GH.ListRuns(config.NightWorkflow, "", RunRows, false)
	}); failed {
		return []string{"The last runs could not be read."}
	}
	var rows []string
	for _, run := range upTo(runs, RunRows) {
		result := "🔄 running"
		if run.Str("status") == "completed" {
			result = resultOf(run)
		}
		rows = append(rows, runRow(ctx, run, result))
	}
	if len(rows) == 0 {
		return []string{"No runs yet."}
	}
	return append([]string{"| Started (Central) | Trigger | Result | Run |", "|---|---|---|---|"}, rows...)
}

// took is _took: how long a run took, or had been going when this was written.
func took(ctx *hctx.Context, run *pyjson.Object, created time.Time) string {
	began := clock.ParseISOValue(run.Get("run_started_at"))
	if began.IsZero() {
		began = created
	}
	if run.Str("status") != "completed" {
		return runningFor(ctx, began)
	}
	ended := clock.ParseISOValue(run.Get("updated_at"))
	if ended.IsZero() {
		return "finished"
	}
	t := clock.HumanDelta(ended.Sub(began))
	if t == "now" {
		return "took under a minute"
	}
	return "took " + t
}
