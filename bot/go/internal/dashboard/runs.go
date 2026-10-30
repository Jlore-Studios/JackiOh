package dashboard

// ------------------------------------------------------------------ trees, runs, Squishy

import (
	"fmt"
	"strings"
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

// ------------------------------------------------------------------ the learn lane

// The learn lane (#466; training/learn.md, .github/workflows/learn.yml): the learned evaluation,
// retrained and promoted on the owner's PC (the local runner, bot/machine/README.md) with no model
// in the loop and no harness either, so its facts come from GitHub alone: the generation on main,
// the lane's pull request and its runs.
var (
	LearnWorkflow = "learn.yml"
	LearnBranch   = "ai/learn"
)

// LearnSchedule is LEARN_SCHEDULE: learn.yml's schedule, "23 6,18 * * *": twice a day, at 06:23
// and 18:23 UTC. Each run starts the next as it ends (its `next` job), so the schedule only starts
// the lane again when that chain stops.
var LearnSchedule = [][2]int{{6, 23}, {18, 23}}

// LearnPRStale is LEARN_PR_STALE: learn.yml's LEARN_PR_STALE_HOURS: a lane pull request open this
// long is closed by the next run.
var LearnPRStale = 48 * time.Hour

// LearnRows is LEARN_ROWS.
var LearnRows = 5

// NextLearnRun is next_learn_run: when learn.yml's schedule next fires (GitHub fires schedules
// late; never early).
func NextLearnRun(now time.Time) time.Time {
	day := now.UTC()
	var next time.Time
	for ahead := 0; ahead <= 1; ahead++ {
		for _, at := range LearnSchedule {
			t := time.Date(day.Year(), day.Month(), day.Day(), at[0], at[1], 0, 0, time.UTC).
				Add(time.Duration(ahead) * 24 * time.Hour)
			if t.After(now) && (next.IsZero() || t.Before(next)) {
				next = t
			}
		}
	}
	if next.IsZero() {
		panic(fmt.Errorf("ValueError: min() arg is an empty sequence"))
	}
	return next
}

// LearnNoRunner is LEARN_NO_RUNNER: a job that ended this way with no runner was never taken: the
// PC was off, asleep or busy for the 24 hours GitHub lets a job wait for a self-hosted runner.
var LearnNoRunner = "⏸️ the PC's runner took none of it in 24 hours"

// LearnOutcome is learn_outcome: how a learn.yml run ended, from its jobs' and steps' conclusions
// (learn.yml's names).
func LearnOutcome(run *pyjson.Object, jobs []*pyjson.Object) string {
	if run.Str("status") != "completed" {
		return "🔄 running"
	}
	named := func(items []*pyjson.Object, prefix string) *pyjson.Object {
		for _, i := range items {
			if strings.HasPrefix(py.OrStr(i.Get("name"), ""), prefix) {
				return i
			}
		}
		return nil
	}
	untaken := func(job *pyjson.Object) bool {
		return job != nil && !job.Truthy("runner_name") &&
			py.In(job.Get("conclusion"), "failure", "cancelled")
	}
	conclusion := func(item *pyjson.Object) any { return item.Get("conclusion") }
	retrain, planJob := named(jobs, "retrain"), named(jobs, "plan")
	var lost []*pyjson.Object
	for _, j := range jobs {
		if strings.HasPrefix(py.OrStr(j.Get("name"), ""), "export") &&
			!py.In(j.Get("conclusion"), "success", "skipped") {
			lost = append(lost, j)
		}
	}
	exportFailed := LearnNoRunner
	for _, j := range lost {
		if !untaken(j) {
			exportFailed = "❌ the export failed"
			break
		}
	}
	if untaken(planJob) || untaken(retrain) {
		return LearnNoRunner
	}
	if retrain == nil || pyjson.Equal(conclusion(retrain), "skipped") {
		if planJob != nil && pyjson.Equal(conclusion(planJob), "failure") {
			return "❌ the plan failed"
		}
		// Stopped in its export, by a person or by a new set (learn-set.yml): its shards were
		// cancelled, not lost.
		if pyjson.Equal(run.Get("conclusion"), "cancelled") {
			return "⏹️ cancelled"
		}
		if len(lost) > 0 { // a run from before the retrain went on with the shards that finished
			return exportFailed
		}
		return "⏭️ waited on its open pull request"
	}
	steps := pyjson.Objs(retrain.Get("steps"))
	// The retrain goes on with the shards that finished; it stops here only when none did.
	shards := named(steps, "the export's shards")
	if shards != nil && pyjson.Equal(conclusion(shards), "failure") {
		if len(lost) > 0 {
			return exportFailed
		}
		return "❌ the export failed"
	}
	fit, gate := named(steps, "retrain and screen"), named(steps, "the gate")
	deliver, moving := named(steps, "deliver"), named(steps, "main kept moving")
	waits := named(steps, "a promotion still open")
	if fit != nil && pyjson.Equal(conclusion(fit), "failure") {
		return "❌ the retrain broke"
	}
	if gate == nil || pyjson.Equal(conclusion(gate), "skipped") {
		if pyjson.Equal(conclusion(retrain), "success") {
			return "screened out"
		}
		return "❌ the retrain broke"
	}
	if pyjson.Equal(conclusion(gate), "failure") {
		return "❌ the gate broke"
	}
	if deliver != nil && pyjson.Equal(conclusion(deliver), "success") {
		return "🎉 promoted"
	}
	if deliver != nil && pyjson.Equal(conclusion(deliver), "failure") {
		return "❌ passed, but could not open its pull request (LEARN_LANE_TOKEN?)"
	}
	if waits != nil && pyjson.Equal(conclusion(waits), "success") {
		return "⏸️ passed, behind a promotion still open: kept as its artifact"
	}
	if moving != nil && pyjson.Equal(conclusion(moving), "success") {
		return "🔁 main kept moving: another run on the export"
	}
	return "gated out"
}

// LearnLane is learn_lane: the learn lane's section: the AI on main, the lane's pull request, the
// next scheduled run, the owner's PC it runs on (its heartbeat: whether it is working and on what,
// from the runners `found`, and how loaded it is, with and without its runners), and its last runs
// with how each ended. Each part says so when GitHub cannot be read. Without `found` it reads the
// heartbeat alone, which says a runner is in a job but not which.
func LearnLane(ctx *hctx.Context, found *Found /* = None */) []string {
	lines := []string{"## 🧠 The learn lane", "",
		"<sub>`learn.yml` retrains the learned evaluation on fresh self-play on the owner's " +
			"PC, one run after another (the local runner, only while the PC is idle: a run takes " +
			"about 40 hours of it), files each export's games into the card statistics, and " +
			"promotes the network through the gate, with no model in the loop. It closes a stuck " +
			"promotion, and gates again when main moves (training/learn.md, Running " +
			"unattended).</sub>", ""}
	var onMain string
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		text, _ := ctx.GH.GetFile("crates/ai/generation.json", "main")
		var record any = pyjson.New()
		if text != "" {
			var err error
			if record, err = pyjson.LoadsString(text); err != nil {
				onMain = "could not be read" // json.loads's ValueError
				return
			}
		}
		if !pyjson.Truthy(record) {
			onMain = "no generation.json"
			return
		}
		r, ok := record.(*pyjson.Object)
		if !ok {
			panic(fmt.Errorf("AttributeError: %T object has no attribute 'get'", record))
		}
		onMain = fmt.Sprintf("gen %s (%s), promoted %s, %s against its parent",
			py.Str(r.Get("generation")), py.Str(r.Get("lane")), py.Str(r.Get("date")),
			py.Str(r.Get("vsParent")))
	}); failed {
		onMain = "could not be read"
	}
	lines = append(lines, "- **The AI on main:** "+onMain+".")
	var pulled string
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		var pull *pyjson.Object
		for _, p := range ctx.GH.ListPulls("open", "", 200) {
			if py.Str(p.Obj("head").Get("ref")) == LearnBranch {
				pull = p
				break
			}
		}
		if pull == nil {
			pulled = "none open."
			return
		}
		opened := clock.ParseISOValue(pull.Get("created_at"))
		since := " open"
		if !opened.IsZero() {
			since = " open since " + clockText(ctx, opened)
		}
		link := "[#" + py.Str(pull.Get("number")) + "](" + py.Str(pull.GetOr("html_url", "")) + ")"
		if !opened.IsZero() && ctx.Now().Sub(opened) > LearnPRStale {
			pulled = "⛔ " + link + since + ": the next run closes it."
		} else {
			pulled = "▶️ " + link + since + ": CI decides, and auto-merge lands it."
		}
	}); failed {
		pulled = "could not be read."
	}
	lines = append(lines, "- **Its pull request:** "+pulled)
	upcoming := Local(ctx, NextLearnRun(ctx.Now())).Format("Mon Jan 02 15:04")
	lines = append(lines, "- **Next scheduled run:** "+upcoming+" (Central).")
	var beat *pyjson.Object
	var note string
	var jobs [][2]*pyjson.Object
	if found != nil {
		beat, note, jobs = found.PC, found.PCNote, pcJobsOf(found)
	} else {
		beat, note = heartbeat(ctx)
	}
	lines = append(lines, "- **The owner's PC:** "+pcWords(ctx, beat, note, jobs, true)+".", "")
	var runs []*pyjson.Object
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		for _, r := range ctx.GH.ListRuns(LearnWorkflow, "", 20, false) {
			if r.Str("event") != "workflow_run" {
				runs = append(runs, r)
			}
		}
	}); failed {
		return append(lines, "Its runs could not be read.")
	}
	var rows []string
	for _, run := range runs {
		if len(rows) == LearnRows {
			break
		}
		var runJobs []*pyjson.Object
		fallback := false // the except (GitHubError, KeyError): its result is the run's conclusion
		if run.Str("status") == "completed" {
			id, has := run.Lookup("id")
			if !has {
				fallback = true
			} else if _, failed := herr.Catch[*herr.GitHubError](func() {
				runJobs = ctx.GH.ListJobs(id, false)
			}); failed {
				fallback = true
			}
		}
		var result string
		if fallback {
			result = resultOf(run)
		} else {
			// A scheduled run queued behind a running one is cancelled by GitHub when the next
			// schedule queues another (the `learn` concurrency group holds one waiting): it never
			// ran, so it is no row.
			ran := false
			for _, j := range runJobs {
				ran = ran || j.Truthy("runner_name")
			}
			if pyjson.Equal(run.Get("conclusion"), "cancelled") && !ran {
				continue
			}
			result = LearnOutcome(run, runJobs)
		}
		rows = append(rows, runRow(ctx, run, result))
	}
	if len(rows) == 0 {
		return append(lines, "No runs yet.")
	}
	return append(append(lines, "| Started (Central) | Trigger | Result | Run |", "|---|---|---|---|"),
		rows...)
}
