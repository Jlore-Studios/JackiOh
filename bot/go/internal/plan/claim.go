package plan

import (
	"fmt"
	"strconv"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/asks"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/issueplan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/journal"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/prompts"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/reviewrule"
	hstate "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/threads"
)

// providerFields is what the model job needs to know about the subscription it runs on. The
// secret's value is never here: the workflow hands the model job only the secret named.
func providerFields(ctx *hctx.Context, provider *providers.Provider) *pyjson.Object {
	vault, _ := ctx.GH.GetFile(VaultPath(provider.ID), config.StateBranch)
	// `shared` names the run's model step "Build, check and review", which the partner bot
	// reads as this bot spending the shared subscription, so it excuses the rise. claude-1 is
	// that subscription whether or not this bot waits for it to be quiet (`quiet_check`).
	shared := provider.QuietCheck || provider.ID == "claude-1"
	return pyjson.New("provider", provider.ID, "cli", provider.CLI, "secret", provider.Secret,
		"family", provider.Family, "shared", shared, "vault", vault,
		"login", provider.Login, "runs_on", provider.RunsOn)
}

// startMessage is what a build or revision run says it does, by who runs each role.
func startMessage(assignment *Assignment, cfg *config.Config) string {
	var steps []string
	if assignment.Plan != nil {
		steps = append(steps, "plan it on "+assignment.Plan.Describe())
	}
	if assignment.Build != nil {
		steps = append(steps, "build it on "+assignment.Build.Describe())
	} else {
		steps = append(steps, "build it")
	}
	steps = append(steps, "run the repository's checks")
	if assignment.SelfCheck() {
		steps = append(steps, fmt.Sprintf("have the builder check its own change, up to %d times",
			cfg.MaxSelfCheckRounds))
	}
	var end string
	if assignment.Review != nil {
		steps = append(steps, fmt.Sprintf("have %s review the change adversarially, up to %s rounds",
			assignment.Review.Describe(), py.Str(cfg.MaxReviewCycles.GetOr(assignment.Difficulty, 3))))
		end = "Only an approved change becomes a pull request."
	} else {
		end = "No model here reviews it, so the pull request then waits for a review run, until " +
			reviewrule.Summary + " approved it."
	}
	return fmt.Sprintf("I %s and %s. %s", strings.Join(steps[:len(steps)-1], ", "), steps[len(steps)-1], end)
}

// Claim builds the plan for one queued thread on one subscription, and marks it `bot:working`.
// nil when the thread is closed, or a pull request from a fork.
func Claim(ctx *hctx.Context, candidate *queue.Candidate, assignment *Assignment) *pyjson.Object {
	cfg := ctx.Cfg
	provider := assignment.Provider
	number, kind := candidate.Number, candidate.Kind
	labelPR, labelNeedsPlan := config.LabelPR, config.LabelNeedsPlan
	thread := ctx.GH.GetIssue(number)
	names := queue.LabelNames(thread)
	if !eqStr(thread.Get("state"), "open") {
		queue.SetStateLabel(ctx, number, names, "")
		return nil
	}
	record := recordOf(ctx.Store.Load(), strconv.Itoa(number))
	// (The Python's `who = provider.describe()` is never read, so it is left out.)
	var planned *pyjson.Object
	var message string
	if kind == "build" {
		planned = pyjson.New(
			"action", assignment.Action,
			"number", number,
			"title", thread.GetOr("title", ""),
			"branch", queue.BranchForIssue(number),
			"thread", threads.IssueThread(ctx.GH, ctx.Trust, number, cfg.BotLogin),
			"previous_findings", py.Or(record.Get("last_findings"), []any{}),
			"previous_question", py.Or(record.Get("question"), ""),
		)
		written := issueplan.PlanOf(py.OrStr(thread.Get("body"), ""))
		if written != "" && assignment.Action == "build" {
			// A person may have edited the plan in the description: the builder starts from that.
			planned.Set("plan_in_issue", written)
		}
		// Who rated it, for the planner's prompt and for deliver (#317 part 8).
		by := record.Obj("difficulty_by")
		planned.Set("rating", pyjson.New("difficulty", candidate.Difficulty,
			"source", queue.RatingSource(names, record),
			"by", py.OrStr(by.Get("provider"), "")))
		if candidate.Mode != "" {
			planned.Set("mode", candidate.Mode)
			if strings.HasPrefix(candidate.Mode, "split") || candidate.Mode == "fullsend" {
				planned.Set("children", TreeText(ctx, number))
				planned.Set("builders", SplitBuilders(candidate.Mode))
			}
			if candidate.Mode == "fullsend" {
				// The parts its reconcile merges first: those that could not land (#505).
				parked := []any{}
				for _, b := range pyjson.List(py.Or(record.Get("parked"), nil)) {
					parked = append(parked, py.Str(b))
				}
				planned.Set("parked", parked)
			}
		}
		if record.Truthy("onto") {
			// A fullsend part (#505): it lands on its tree's branch, not `main`.
			planned.Set("onto", py.Str(record.Get("onto")))
			planned.Set("part_of", py.MustInt(py.Or(record.Get("part_of"), 0)))
		}
		if record.Truthy("previous_pr") {
			planned.Set("previous_pr", record.Get("previous_pr"))
			planned.Set("previous_branch", py.OrStr(record.Get("previous_branch"), ""))
			planned.Set("previous_why", py.OrStr(record.Get("previous_why"), ""))
		}
		switch {
		case candidate.Mode == "fullsend" && planned.Truthy("children"):
			message = fmt.Sprintf("Starting work on this now%s: the reconcile of this fullsend "+
				"tree. Every part has closed, so this run merges them on `%s`, follows fullsend's "+
				"reconcile and makes every check green, for one pull request into `main`. %s",
				RunLink(cfg), py.Str(planned.Get("branch")), startMessage(assignment, cfg))
		case candidate.Mode == "fullsend":
			message = fmt.Sprintf("Splitting this now for fullsend%s, on %s: one session reads it "+
				"and the code and answers with parts that each own their files. When the run ends I "+
				"open them, and each lands on `%s`, not `main`, with no pull request of its own; "+
				"once they have all closed, one run reconciles them into one pull request.",
				RunLink(cfg), assignment.Build.Describe(), py.Str(planned.Get("branch")))
		case strings.HasPrefix(candidate.Mode, "split"):
			message = fmt.Sprintf("Splitting this now%s, on %s: one session reads it and the code "+
				"and answers with sub-issues, each small enough for one run. When the run ends I "+
				"open them, queued for %s in the order they depend on each other.",
				RunLink(cfg), assignment.Build.Describe(), SplitBuilders(candidate.Mode))
		case planned.Truthy("onto") && assignment.Action == "build":
			message = fmt.Sprintf("Starting work on this now%s: a fullsend part, "+
				"difficulty:%s, built on %s. It lands on `%s`, the branch of #%s, with no checks, "+
				"review or pull request of its own: the tree's reconcile checks and reviews every "+
				"part at once.", RunLink(cfg), candidate.Difficulty, assignment.Build.Describe(),
				py.Str(planned.Get("onto")), py.Str(planned.Get("part_of")))
		case candidate.Mode == "oneshot":
			message = fmt.Sprintf("Starting a one-shot build of this now%s, on %s, with fullsend: a "+
				"spec first, then many agents writing its parts at once, each in its own worktree "+
				"and branch, reconciled against tests written from the spec. %s",
				RunLink(cfg), assignment.Build.Describe(), startMessage(assignment, cfg))
		case assignment.Action == "plan":
			message = fmt.Sprintf("Planning this now%s, on %s (`%s`): it is difficulty:%s. The plan "+
				"goes into this issue's description, under **Plan**, and the lane's owner builds "+
				"from it (a medium or hard item stays with its planner; an easy one goes to the "+
				"weakest weak seat).", RunLink(cfg), assignment.Plan.Describe(), labelNeedsPlan,
				candidate.Difficulty)
		default:
			message = fmt.Sprintf("Starting work on this now%s: it is difficulty:%s. %s",
				RunLink(cfg), candidate.Difficulty, startMessage(assignment, cfg))
		}
	} else {
		pull := ctx.GH.GetPull(number)
		head := pull.Obj("head")
		if !eqStr(head.Obj("repo").Get("full_name"), cfg.Repo) {
			queue.SetStateLabel(ctx, number, names, "")
			ctx.GH.CreateComment(number, "This pull request comes from a fork, so I cannot push "+
				"to it. Taking it out of the queue.")
			return nil
		}
		issueNumber := threads.LinkedIssue(pull)
		issueText := ""
		if issueNumber != nil && *issueNumber != 0 {
			if _, failed := herr.Catch[*herr.GitHubError](func() {
				issueText = threads.IssueThread(ctx.GH, ctx.Trust, *issueNumber, cfg.BotLogin)
			}); failed {
				issueNumber = nil
			}
		}
		pullText := threads.PullText(pull)
		if kind == "review" {
			builder := candidate.Builder
			if builder == "" {
				builder = "an unknown model"
			}
			planned = pyjson.New(
				"action", "review",
				"number", number,
				"title", pull.GetOr("title", ""),
				"branch", head.GetOr("ref", ""),
				"head", head.GetOr("sha", ""),
				"builder", candidate.Builder,
				"approved", pyjson.ListOf(candidate.Approved),
				"pull", pullText,
				"issue", issueText,
				"thread", joinNonEmpty("\n\n", pullText, issueText),
				"bot_pr", true,
				"issue_number", intOrNone(issueNumber),
				"review_notes", py.OrStr(record.Get("review_notes"), ""),
			)
			message = fmt.Sprintf("Starting a review now%s, on %s. `%s` built the change; it merges "+
				"once %s approved the same commit.", RunLink(cfg), assignment.Review.Describe(),
				builder, reviewrule.Summary)
		} else {
			source := py.OrStr(record.Get("source"), "request")
			feedback := threads.PullFeedback(ctx.GH, ctx.Trust, number, cfg.BotLogin,
				pyjson.Str(record.Get("feedback_since")), issueNumber)
			if source == "ci" {
				feedback += "\n\n" + CILogs(ctx, record.Get("ci_run_id"))
			}
			if source == "cross-review" && record.Truthy("last_findings") {
				var lines []string
				for _, f := range pyjson.List(record.Get("last_findings")) {
					finding := pyjson.Obj(f)
					lines = append(lines, fmt.Sprintf("- `%s`: %s (%s)", py.Str(finding.GetOr("where", "?")),
						py.Str(finding.GetOr("claim", "")), py.Str(finding.GetOr("evidence", ""))))
				}
				feedback += "\n\n" + prompts.Data(strings.Join(lines, "\n"), "The second review's blocking findings")
			}
			planned = pyjson.New(
				"action", "revise",
				"number", number,
				"title", pull.GetOr("title", ""),
				"branch", head.GetOr("ref", ""),
				"source", source,
				"pull", pullText,
				"issue", issueText,
				"feedback", feedback,
				"thread", joinNonEmpty("\n\n", pullText, issueText, feedback),
				"bot_pr", names[labelPR],
				"issue_number", intOrNone(issueNumber),
			)
			message = fmt.Sprintf("Starting a revision now%s, because of: %s. %s",
				RunLink(cfg), source, startMessage(assignment, cfg))
			if names[labelPR] && queue.Carries(record) {
				// The work job tells its builder and reviewer; deliver decides from git alone.
				planned.Set("cleared", queue.Cleared(record))
				if assignment.Review != nil {
					message += " The reviews had cleared it before `main` moved, so if this " +
						"revision changes nothing but the conflicted files and its " +
						"reviewer approves, it merges without another review run."
				}
			}
		}
	}
	planned.Update(providerFields(ctx, provider))
	if assignment.RunsOn != "" {
		planned.Set("runs_on", assignment.RunsOn) // its fill-in runners, or its own
	}
	planned.Set("difficulty", candidate.Difficulty)
	planned.Set("seats", assignment.Seats())
	planned.Set("routing", pyjson.ListOf(assignment.Notes))
	planned.Set("assignment", assignment.Describe())
	if kind == "build" || kind == "revise" {
		// Verdict rounds the item already used in its current owner's hands (#725).
		planned.Set("rounds_used", candidate.RoundsUsed)
		planned.Set("round_tier", candidate.RoundTier)
	}
	if kind == "review" && record.Truthy("self_check_findings") {
		planned.Set("self_check_findings", record.Get("self_check_findings"))
	}
	if record.Truthy("handoff") {
		planned.Set("handoff", record.Get("handoff"))
	}
	// Every earlier run on the item, for the work job to put in its worktree (#342).
	journalOf := journal.Key(planned)
	if journalOf == 0 {
		journalOf = number
	}
	if text := journal.Read(ctx.GH, journalOf, journal.ReadChars); text != "" {
		planned.Set("journal", text)
	}
	if wip, isDict := record.Get("wip").(*pyjson.Object); kind == "revise" && isDict && wip != nil {
		planned.Set("wip", wip) // the last cut-off revision's work (#317 part 3)
	}
	if planned.Truthy("plan_in_issue") {
		handoff := planned.Obj("handoff")
		if handoff.Len() == 0 || py.In(handoff.Get("kind"), "plan", "draft") {
			if eqStr(handoff.Get("kind"), "draft") {
				handoff = nil // the plan in the description is the plan: a draft gives way
			}
			merged := handoff.Copy()
			merged.Set("kind", "plan")
			merged.Set("provider", py.Or(py.Or(handoff.Get("provider"), record.Get("planned_by")),
				"the issue's description"))
			merged.Set("notes", planned.Get("plan_in_issue"))
			planned.Set("handoff", merged)
		}
	}
	queue.SetStateLabel(ctx, number, names, config.LabelWorking)
	if names[labelNeedsPlan] && assignment.Action != "plan" {
		ctx.GH.RemoveLabel(number, labelNeedsPlan) // this run plans it first, in the run
	}
	var taken []string
	ctx.Store.Update(func(state *pyjson.Object) any {
		entry := hstate.Item(state, number)
		entry.Update(pyjson.New("run_id", cfg.RunID, "started_at", clock.ISO(ctx.Now()), "kind", kind,
			"action", assignment.Action, "stop_requested", false, "pending_request", false,
			"provider", provider.ID, "runs_on", planned.Get("runs_on")))
		taken = asks.Take(entry)
		state.Set("last_run", pyjson.New("at", clock.ISO(ctx.Now()), "url", cfg.RunURL(),
			"what", fmt.Sprintf("%s #%d on %s", assignment.Action, number, provider.ID)))
		return nil
	}, fmt.Sprintf("claim #%d", number))
	ctx.GH.CreateComment(number, message)
	asks.React(ctx.GH, taken, asks.Working)
	return planned
}

// SplitBuilders is who builds a split's sub-issues (#60): this bot for `split` and `fullsend`,
// the other bot for `split-bot`.
func SplitBuilders(mode string) string {
	if mode == "split-bot" && len(Others) > 0 {
		return fmt.Sprintf("%s (@%s)", Others[0].Name, Others[0].Login)
	}
	return "me"
}

// TreeText is the sub-issues an issue already has, for a split that closes its tree out: what
// was done and what is still open. Fenced as data; "" when it has none.
func TreeText(ctx *hctx.Context, number int) string {
	var children []*pyjson.Object
	if _, failed := herr.Catch[*herr.GitHubError](func() { children = ctx.GH.ListSubIssues(number) }); failed {
		return ""
	}
	if len(children) == 0 {
		return ""
	}
	lines := make([]string, 0, len(children))
	for _, c := range children {
		lines = append(lines, fmt.Sprintf("- #%s [%s] %s", py.Str(c.Get("number")), py.Str(c.Get("state")),
			py.Str(c.GetOr("title", ""))))
	}
	return prompts.Data(strings.Join(lines, "\n"), fmt.Sprintf("The sub-issues #%d has already", number))
}

// CILogs is the tails of the failed jobs of one CI run, fenced as data.
func CILogs(ctx *hctx.Context, runID any) string {
	if !pyjson.Truthy(runID) {
		return prompts.Data("(no CI run was recorded)", "CI failure")
	}
	var jobs []*pyjson.Object
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		jobs = filter(ctx.GH.ListJobs(runID, false), func(j *pyjson.Object) bool {
			return py.In(j.Get("conclusion"), "failure", "timed_out", "cancelled")
		})
		// The failed first: a merge-queue build cancels the rest of its run once a required check
		// failed (ci.yml's `queue-cancel-*`), and those cancelled jobs say nothing.
		jobs = py.SortedBy(jobs, func(j *pyjson.Object) []any { return []any{eqStr(j.Get("conclusion"), "cancelled")} })
	}); failed {
		return prompts.Data(fmt.Sprintf("(could not read the CI run: %s)", exc.Error()), "CI failure")
	}
	var parts []string
	for _, job := range jobs[:min(CILogJobs, len(jobs))] {
		var failedSteps []string
		for _, s := range pyjson.Objs(job.GetOr("steps", []any{})) {
			if eqStr(s.Get("conclusion"), "failure") {
				failedSteps = append(failedSteps, py.Str(s.Get("name")))
			}
		}
		steps := strings.Join(failedSteps, ", ")
		if steps == "" {
			steps = "?"
		}
		parts = append(parts, fmt.Sprintf("## Job: %s (%s)\nFailed steps: %s\n\n%s",
			py.Str(job.Get("name")), py.Str(job.Get("conclusion")), steps, ctx.GH.JobLog(job.Get("id"), 6000)))
	}
	text := strings.Join(parts, "\n\n")
	if text == "" {
		text = "(no failed job found)"
	}
	return prompts.Data(text, "CI run "+py.Str(runID))
}
