package deliver

import (
	"fmt"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gates"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
)

// ------------------------------------------------------------------ helpers

func (x *Deliverer) try(action func()) string {
	if exc, ok := herr.Catch[*herr.GitHubError](action); ok {
		return exc.Error()
	}
	return ""
}

// What the branch changes against the default branch, as it now stands on GitHub.
func (x *Deliverer) changed(branch string) []string {
	defaultBranch := x.Cfg.DefaultBranch
	x.fetch(defaultBranch, false)
	local := "deliver/" + branch
	ref := "origin/" + branch
	if x.Repo.Rev(local) != "" {
		ref = local
	}
	if x.Repo.Rev(ref) == "" {
		x.fetch(branch, false)
		if x.Repo.Rev(ref) == "" {
			return []string{}
		}
	}
	return x.Repo.ChangedPaths("origin/"+defaultBranch, ref)
}

// The review rule approved the head but auto-merge cannot take it: label it `ready for
// merge` and ask the operator to merge it.
func (x *Deliverer) handToAPerson(pull *pyjson.Object, why string) string {
	number := py.MustInt(py.Or(pull.Get("number"), 0))
	if number != 0 {
		x.GH.AddLabels(number, []string{config.LabelReady})
		x.try(func() { x.GH.RequestReviewers(number, []string{x.Cfg.Operator}) })
	}
	return fmt.Sprintf("Auto-merge is off because %s. The reviews approved it, so it is labelled "+
		"`%s`: @%s, it waits for you to merge it.", why, config.LabelReady, x.Cfg.Operator)
}

// Turn on auto-merge only when nothing needs a person and CI must pass first, pinned to
// `expected_head` (the approved commit) when given.
func (x *Deliverer) autoMerge(pull *pyjson.Object, expectedHead string /* = "" */) string {
	if !x.Cfg.AutoMerge {
		if pull.Truthy("number") {
			x.GH.AddLabels(py.MustInt(pull.Get("number")), []string{config.LabelReady})
		}
		return fmt.Sprintf("Auto-merge is off in `.harness/config.json`, so it is labelled "+
			"`%s`; a person merges it.", config.LabelReady)
	}
	if !pull.Truthy("node_id") {
		return ""
	}
	branch := py.OrStr(py.Or(pull.Obj("head").Get("ref"), x.Plan.Get("branch")), "")
	if pull.Truthy("number") && x.Labels(py.MustInt(pull.Get("number")))[config.LabelPersonMerges] {
		return x.handToAPerson(pull, fmt.Sprintf("it is labelled `%s`", config.LabelPersonMerges))
	}
	var review []string
	for _, p := range x.changed(branch) {
		if git.Matches(p, x.Cfg.ReviewPaths) {
			review = append(review, p)
		}
	}
	if len(review) > 0 {
		shown := ticked(headN(review, 8))
		return x.handToAPerson(pull, fmt.Sprintf("the change touches %s, which a person reviews", shown))
	}
	var required map[string]bool
	if exc, ok := herr.Catch[*herr.GitHubError](func() {
		required = x.GH.RequiredChecks(x.Cfg.DefaultBranch)
	}); ok {
		return x.handToAPerson(pull, fmt.Sprintf("I could not read the branch protection (%s)", exc.Error()))
	}
	missing := []string{}
	for _, c := range x.Cfg.RequiredChecks {
		if _, ok := required[c]; !ok {
			missing = append(missing, c)
		}
	}
	if required == nil || len(missing) > 0 {
		what := "is not protected"
		if required != nil {
			what = "does not require " + py.Repr(missing)
		}
		return x.handToAPerson(pull, fmt.Sprintf("`%s` %s, so auto-merge would not wait for CI",
			x.Cfg.DefaultBranch, what))
	}
	headline := x.mergeTitle(pull)
	refused := x.try(func() {
		x.GH.EnableAutoMerge(py.Str(pull.Get("node_id")), x.Cfg.MergeMethod, expectedHead, headline)
	})
	if refused == "" {
		return "Auto-merge is on: it merges when every required check passes."
	}
	if strings.Contains(strings.ToLower(refused), "clean status") {
		// Every required check has already passed, so GitHub will not wait; merge it now.
		merged := x.try(func() {
			x.GH.MergePull(py.MustInt(pull.Get("number")), x.Cfg.MergeMethod, headline)
		})
		if merged == "" {
			return "Every required check had already passed, so I merged it."
		}
	}
	return x.handToAPerson(pull, fmt.Sprintf("GitHub refused it (%s)", refused))
}

func (x *Deliverer) mergeTitle(pull *pyjson.Object) string {
	return queue.MergeTitle(pull, x.Cfg.MergeMethod)
}

func (x *Deliverer) rerunCI(number int) {
	runID := x.Record(number).Get("ci_run_id")
	if pyjson.Truthy(runID) {
		x.try(func() { x.Ctx.Act().RerunFailedJobs(runID) })
	}
}

// The first blocking finding's claim, cut short, for a strike's reason.
func (x *Deliverer) firstFinding() string {
	findings := pyjson.List(py.Or(x.Result.Get("findings"), []any{}))
	if len(findings) == 0 {
		return ""
	}
	first, ok := findings[0].(*pyjson.Object)
	if !ok {
		return ""
	}
	where := py.OrStr(first.Get("where"), "")
	claim := py.SliceTo(strings.Join(strings.Fields(py.OrStr(first.Get("claim"), "")), " "), 140)
	if where != "" {
		return fmt.Sprintf("`%s`: %s", where, claim)
	}
	return claim
}

func (x *Deliverer) findingsMd() string {
	findings := x.Result.Get("findings")
	if !pyjson.Truthy(findings) {
		return "_No findings were recorded._"
	}
	lines := []string{"**Open findings:**"}
	for _, f := range headN(objList(findings), 20) {
		lines = append(lines, fmt.Sprintf("- `%s`: %s", py.Str(f.GetOr("where", "?")), py.Str(f.GetOr("claim", ""))))
	}
	return strings.Join(lines, "\n")
}

func (x *Deliverer) pullBody(number int, approved bool) string {
	cycles := objList(x.Result.Get("cycles"))
	review := pyjson.Obj(x.Result.Get("review"))
	report := py.OrStr(x.Result.Get("report"), "_The builder wrote no description._")
	parts := []string{fmt.Sprintf("Closes #%d", number), "", py.SliceTo(report, ReportChars), ""}
	if approved {
		parts = append(parts, fmt.Sprintf("### Adversarial review\n\nApproved on round %d of "+
			"%d.", len(cycles), x.CyclesCap()))
		var notes []*pyjson.Object
		for _, f := range objList(review.Get("findings")) {
			if f.Get("severity") == "note" {
				notes = append(notes, f)
			}
		}
		if len(notes) > 0 {
			parts = append(parts, "\nNotes the reviewer left (not blocking):\n")
			for _, f := range headN(notes, 20) {
				parts = append(parts, fmt.Sprintf("- `%s`: %s", py.Str(f.Get("where")), py.Str(f.Get("claim"))))
			}
		}
	} else {
		parts = append(parts, fmt.Sprintf("### Not approved\n\n%s, so this is a draft "+
			"and will not merge by itself.", x.notApprovedHead()))
		parts = append(parts, "")
		parts = append(parts, x.findingsMd())
	}
	rows := []string{"| Round | Builder | Checks | Review |", "|---|---|---|---|"}
	for _, c := range cycles {
		builder := c.Obj("builder")
		var red []string
		for _, g := range objList(c.Get("gates")) {
			if !g.Truthy("ok") && !g.Truthy("pre_existing") && !g.Truthy("inconclusive") {
				red = append(red, py.Str(g.Get("name")))
			}
		}
		verdict := py.Str(c.Obj("review").GetOr("verdict", "-"))
		blocking := 0
		for _, f := range objList(c.Obj("review").Get("findings")) {
			if f.Get("severity") == "blocking" {
				blocking++
			}
		}
		checks := "green"
		if len(red) > 0 {
			checks = "red: " + strings.Join(red, ", ")
		}
		blocked := ""
		if blocking > 0 {
			blocked = fmt.Sprintf(" (%d blocking)", blocking)
		}
		rows = append(rows, fmt.Sprintf("| %s | %s, %s min | %s | %s%s |", py.Str(c.Get("n")),
			py.Str(builder.Get("role")), py.Str(builder.Get("minutes")), checks, verdict, blocked))
	}
	// `GateResult(**{k: g.get(k) ...})`: a missing field is Go's zero value here, where Python
	// would carry None into the table.
	var results []*gates.GateResult
	for _, g := range objList(x.Result.Get("gates")) {
		results = append(results, &gates.GateResult{
			Name:         py.Str(g.Get("name")),
			Run:          py.Str(g.Get("run")),
			OK:           g.Truthy("ok"),
			ExitCode:     g.Int("exit_code"),
			Seconds:      g.Float("seconds"),
			Tail:         py.OrStr(g.Get("tail"), ""),
			PreExisting:  g.Truthy("pre_existing"),
			Skipped:      py.OrStr(g.Get("skipped"), ""),
			Inconclusive: g.Truthy("inconclusive"),
		})
	}
	parts = append(parts, "", "<details><summary>Rounds and checks</summary>", "")
	parts = append(parts, rows...)
	parts = append(parts, "", gates.Table(results), "", "</details>", "")
	run := ""
	if url := x.Cfg.RunURL(); url != "" {
		run = fmt.Sprintf(" ([run](%s))", url)
	}
	parts = append(parts, fmt.Sprintf("---\nBuilt overnight by @%s%s; every run on it, "+
		"step by step, is in its %s. Comment "+
		"`@%s <what to change>` or `%s revise <notes>` to ask "+
		"for a revision, or `%s stop` to stop it.", x.Cfg.BotLogin, run, x.journalLink(number),
		x.Cfg.BotLogin, config.Slash, config.Slash))
	return strings.Join(parts, "\n")
}
