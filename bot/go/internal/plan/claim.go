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

// UsageFacts is where a claim puts in the plan what the run's row in the usage history reads when
// it is delivered (#2749, `usage.RunOf`): `queued_at` and `claimed_at`, `previous_provider` (the
// subscription the thread's last run was on) and `handed_off` (its item's `owner_why` when the
// claim took it from another owner, "" when it did not). Only with `usage_history`.
var UsageFacts = "usage_facts"

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
func startMessage(assignment *Assignment, cfg *config.Config, chained bool) string {
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
	if assignment.Review != nil && chained {
		// Its own review run on this subscription reviews it (#2748, 2026-11-02), on the model that
		// built it (#2903).
		reviewer := assignment.Review
		if assignment.Build != nil {
			reviewer = assignment.Build
		}
		steps = append(steps, fmt.Sprintf("open the pull request, so CI starts, and hand this lane to "+
			"its own review run, where %s reviews the change adversarially in a fresh session and fixes "+
			"what it finds, up to %s rounds", reviewer.Describe(),
			py.Str(cfg.MaxReviewCycles.GetOr(assignment.Difficulty, 3))))
		end = "It merges once " + reviewrule.Summary + " approved it."
	} else if assignment.Review != nil {
		steps = append(steps, fmt.Sprintf("have %s review the change adversarially, up to %s rounds",
			assignment.Review.Describe(), py.Str(cfg.MaxReviewCycles.GetOr(assignment.Difficulty, 3))))
		end = "Only an approved change becomes a pull request, and it merges once " +
			reviewrule.Summary + " approved it."
	} else {
		end = "No model here reviews it, so the pull request then waits for a review run, until " +
			reviewrule.Summary + " approved it."
	}
	return fmt.Sprintf("I %s and %s. %s", strings.Join(steps[:len(steps)-1], ", "), steps[len(steps)-1], end)
}

// Claim builds the plan for one queued thread on one subscription, and marks it with its run's
// live label (`config.LiveRun`: `bot:planning`, `bot:building`, `bot:fixing-ci`, …, #2748), which
// its record keeps as `live`. A planning run, a build or a revision makes its subscription the
// item's owner (`takeOver`, #2748), and ends any hold on it (its `bot:waiting` goes with the
// queue's label). nil when the thread is closed, or a pull request from a fork. An after-merge
// spot-check, of a pull request closed by its merge, is claimed apart (`claimSpotCheck`).
func Claim(ctx *hctx.Context, candidate *queue.Candidate, assignment *Assignment) *pyjson.Object {
	if candidate.SpotCheck {
		return claimSpotCheck(ctx, candidate, assignment)
	}
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
	if queue.ArchivedOf(recordOf(ctx.Store.Load(), strconv.Itoa(number))) != nil {
		// Handed to a person, and queued again by one (#3009): the bot has it back, and starts from
		// its issue and its branch's history.
		ctx.Store.Update(func(s *pyjson.Object) any {
			queue.Unarchive(s, number)
			return nil
		}, fmt.Sprintf("#%d unarchived", number))
	}
	record := recordOf(ctx.Store.Load(), strconv.Itoa(number))
	if kind == "build" && candidate.Mode == "" && !candidate.Part {
		// A queued parent with open sub-issues is a fullsend tree's (#2748, section 2), however it
		// was queued: a label an issue form or triage put on reaches the queue with no event.
		children, unread := queue.QueuedParent(ctx, thread, record)
		if unread != "" {
			return nil // its sub-issues could not be read: the next plan asks again
		}
		if len(children) > 0 {
			by := py.OrStr(record.Get("requested_by"), cfg.BotLogin)
			ctx.GH.CreateComment(number, queue.AutoAdopt(ctx, number, names, children, by))
			return nil
		}
	}
	// (The Python's `who = provider.describe()` is never read, so it is left out.)
	var planned *pyjson.Object
	var message string
	// A run that changes code hands its lane to its own review run (#2748, 2026-11-02).
	chains := false
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
		switch decision := queue.DecisionOf(record); queue.Resumes(decision) {
		case "answer":
			// A trusted person answered its question (#3009): this build carries the answer out.
			planned.Set("on_answer", decision)
			planned.Set("previous_question", "")
			planned.Set("answer_note", AnswerNote(decision))
		case "default":
			// Nobody answered its question in time (#2986): this build goes ahead on its default.
			planned.Set("on_default", decision)
			planned.Set("previous_question", "")
			planned.Set("default_note", DefaultNote(decision))
		}
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
				if planned.Truthy("children") {
					// The parts its reconcile may send back to their builders (#2748).
					planned.Set("send_backs", SendBackable(ctx, number))
					planned.Set("send_back_cap", cfg.FullsendSendBacks)
				}
			}
		}
		chains = Chains(cfg, candidate, assignment, names, record, planned.Truthy("children"))
		if record.Truthy("onto") {
			// A fullsend part (#505): it lands on its tree's branch, not `main`.
			planned.Set("onto", py.Str(record.Get("onto")))
			planned.Set("part_of", py.MustInt(py.Or(record.Get("part_of"), 0)))
			if record.Truthy("sent_back") {
				// Its tree's reconcile sent it back to its builder, with its findings (#2748).
				planned.Set("sent_back", record.Get("sent_back"))
			}
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
				RunLink(cfg), py.Str(planned.Get("branch")), startMessage(assignment, cfg, chains))
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
			again := ""
			if planned.Truthy("sent_back") {
				again = fmt.Sprintf(" #%s's reconcile sent it back with its findings, which this "+
					"build answers on top of what it landed before.", py.Str(planned.Get("part_of")))
			}
			message = fmt.Sprintf("Starting work on this now%s: a fullsend part, "+
				"difficulty:%s, built on %s. It lands on `%s`, the branch of #%s, with no checks, "+
				"review or pull request of its own: the tree's reconcile checks and reviews every "+
				"part at once.%s", RunLink(cfg), candidate.Difficulty, assignment.Build.Describe(),
				py.Str(planned.Get("onto")), py.Str(planned.Get("part_of")), again)
		case candidate.Mode == "oneshot":
			message = fmt.Sprintf("Starting a one-shot build of this now%s, on %s, with fullsend: a "+
				"spec first, then many agents writing its parts at once, each in its own worktree "+
				"and branch, reconciled against tests written from the spec. %s",
				RunLink(cfg), assignment.Build.Describe(), startMessage(assignment, cfg, chains))
		case assignment.Action == "plan" && assignment.InLane:
			// In a lane of its own subscription (#2903): its build follows there, on the same model.
			planned.Set("build_next", true)
			message = fmt.Sprintf("Planning this now%s, on %s (`%s`), in a lane of `%s`: it is "+
				"difficulty:%s. The plan goes into this issue's description, under **Plan**; then this "+
				"lane is kept for its build, which starts at once on the same model in a fresh session, "+
				"and that build hands the lane on to its own review run, on the same model too.",
				RunLink(cfg), assignment.Plan.Describe(), labelNeedsPlan, provider.ID, candidate.Difficulty)
		case assignment.Action == "plan":
			message = fmt.Sprintf("Planning this now%s, on %s (`%s`): it is difficulty:%s. The plan "+
				"goes into this issue's description, under **Plan**, and the lane's owner builds "+
				"from it (the planner owns the item, so it builds it when it can).", RunLink(cfg),
				assignment.Plan.Describe(), labelNeedsPlan, candidate.Difficulty)
		default:
			message = fmt.Sprintf("Starting work on this now%s: it is difficulty:%s. %s",
				RunLink(cfg), candidate.Difficulty, startMessage(assignment, cfg, chains))
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
			if candidate.OwnReview {
				ownReviewPlan(planned, record)
				message = fmt.Sprintf("Starting its own review run now%s, on `%s`, whose run built it: %s "+
					"reviews the change adversarially in a fresh session, reads its CI, and %s fixes what "+
					"it finds in the same run, then a fresh review again, up to %s rounds. Once it "+
					"approves, I mark the pull request ready and turn auto-merge on: it merges once %s "+
					"approved the same commit and CI passes.", RunLink(cfg), provider.ID,
					assignment.Review.Describe(), assignment.Build.Describe(),
					py.Str(cfg.MaxReviewCycles.GetOr(candidate.Difficulty, 3)), reviewrule.Summary)
			}
		} else {
			source := py.OrStr(record.Get("source"), "request")
			feedback := threads.PullFeedback(ctx.GH, ctx.Trust, number, cfg.BotLogin,
				pyjson.Str(record.Get("feedback_since")), issueNumber)
			rerunsLeft := -1 // a CI fix of the merge queue's build, or one queued before #2948: none
			if source == "ci" {
				feedback += "\n\n" + ciLogs(ctx, record.Get("ci_run_id"), record.Truthy("ci_early"))
				if record.Truthy("ci_on_head") {
					rerunsLeft = queue.CIRerunsLeft(record, cfg.CIReruns)
					feedback += "\n\n" + CILocalNote(rerunsLeft)
				}
			}
			decision := queue.DecisionOf(record)
			switch queue.Resumes(decision) {
			case "answer":
				// A trusted person answered its question (#3009): this revision carries the answer out.
				feedback += "\n\n" + AnswerNote(decision)
			case "default":
				// Nobody answered its question in time (#2986): this revision goes ahead on its default.
				feedback += "\n\n" + DefaultNote(decision)
			default:
				decision = nil
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
			chains = Chains(cfg, candidate, assignment, names, record, false)
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
				RunLink(cfg), source, startMessage(assignment, cfg, chains))
			if decision != nil {
				planned.Set("on_"+queue.Resumes(decision), decision) // `on_answer` or `on_default`
			}
			if rerunsLeft >= 0 {
				// The fix run runs the failing check first (#2948): a pass re-runs CI with no
				// commit while the head has re-runs left (`ci_reruns`).
				planned.Set("ci_reruns_left", rerunsLeft)
			}
			if candidate.TurnNext {
				// Its turn came after it waited behind another pull request (#2948): a merge of `main`
				// the harness settles, cleanly or with no conflict left, needs no builder.
				planned.Set("turn", true)
			}
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
	if kind == "build" || kind == "revise" || candidate.OwnReview {
		// Verdict rounds the item already used in its current owner's hands (#725), across runs.
		planned.Set("rounds_used", candidate.RoundsUsed)
		planned.Set("round_tier", candidate.RoundTier)
	}
	if chains {
		planned.Set("own_review_next", true)
	}
	if kind == "review" && record.Truthy("self_check_findings") {
		planned.Set("self_check_findings", record.Get("self_check_findings"))
	}
	if record.Truthy("handoff") {
		planned.Set("handoff", record.Get("handoff"))
	}
	if record.Truthy("session") && !candidate.OwnReview && !candidate.TurnNext && !planned.Has("on_default") &&
		!planned.Has("on_answer") {
		// The Claude session the item's last run kept, which a run on its subscription resumes
		// (#2748): the record only; the work job fetches the sealed file itself. Not an own review
		// run's: it reviews and fixes in fresh sessions, never its build's (#2903); nor a turn's
		// resolution, a fresh session too (#2948); nor a go-ahead on a question's default (#2986), nor
		// a run that carries a person's answer out (#3009).
		planned.Set("session", record.Get("session"))
	}
	// Every earlier run on the item, for the work job to put in its worktree (#342).
	journalOf := journal.Key(planned)
	if journalOf == 0 {
		journalOf = number
	}
	text := journal.Read(ctx.GH, journalOf, journal.ReadChars)
	// And the handoffs people's Claude sessions left on the issue or its pull request (CLAUDE.md
	// rule 14, #2748 section 10), which no session can write to the journal's branch.
	if handoffs := journal.SessionHandoffs(ctx.GH, journal.TrustedAuthor(ctx.Trust), journalOf, number); handoffs != "" {
		if text != "" {
			text = strings.TrimRight(text, "\n") + "\n\n"
		}
		text += handoffs
	}
	if text != "" {
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
	live := config.LiveRun(kind, assignment.Action, candidate.Mode, py.OrStr(record.Get("source"), "request"),
		planned.Truthy("children"), kind != "build")
	queue.SetStateLabel(ctx, number, names, config.LiveLabel(live))
	if names[labelNeedsPlan] && assignment.Action != "plan" {
		ctx.GH.RemoveLabel(number, labelNeedsPlan) // this run plans it first, in the run
	}
	var taken []string
	ctx.Store.Update(func(state *pyjson.Object) any {
		entry := hstate.Item(state, number)
		previous := entry.Get("provider")
		entry.Update(pyjson.New("run_id", cfg.RunID, "started_at", clock.ISO(ctx.Now()), "kind", kind,
			"action", assignment.Action, "stop_requested", false, "pending_request", false,
			"provider", provider.ID, "runs_on", planned.Get("runs_on"), "live", live))
		if assignment.Action == "plan" && assignment.InLane {
			entry.Set(queue.InLaneKey, true) // it holds a lane of its subscription (#2903)
		} else {
			entry.Pop(queue.InLaneKey)
		}
		if kind == "build" {
			entry.Pop(queue.BuildNextKey) // the lane its planning run kept is this run's now (#2903)
		}
		if kind == "revise" {
			entry.Pop(queue.TurnKey) // and the lane kept for its turn (#2948)
		}
		handedOff := ""
		if kind != "review" {
			// The subscription that takes it up owns it (#2748); a held item waits no more.
			was := queue.OwnerOf(state, number)
			takeOver(state, number, provider.ID, assignment, ctx.Now())
			why := recordOf(state, strconv.Itoa(queue.OwnerItem(state, number))).Str("owner_why")
			if was != provider.ID && (was != "" || strings.Contains(why, "released")) {
				handedOff = why // from another owner, or released from one: not a first claim
			}
			entry.Pop("hold")
		}
		if cfg.RecordsUsage() {
			// What the run's row in the usage history reads at delivery (#2749): how long it was
			// queued, the subscription its last run was on, and why it changed hands, if it did.
			planned.Set(UsageFacts, pyjson.New("queued_at", candidate.QueuedAt,
				"claimed_at", clock.ISO(ctx.Now()), "previous_provider", previous, "handed_off", handedOff))
		}
		if reserved := queue.OwnReviewOf(entry); candidate.OwnReview && reserved != nil {
			// Its own review run takes it up (#2748): the lane its build reserved is this run's now.
			reserved.Set("claimed", true).Set("claimed_at", clock.ISO(ctx.Now()))
			entry.Pop("hold")
		}
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

// SendBackable is the parts of fullsend tree `number` that its reconcile may send back to the
// subscription that built each (#2748, section 2): every closed sub-issue the bot landed (or
// parked) as a part of this tree (`part_of`, `onto`, `landed`), whose builder is on record
// (`queue.PartBuilder`), and that was sent back fewer times than `fullsend_send_backs`.
// `[{"part", "title", "built_by", "sent_backs"}]`, in the sub-issues' order; none when the cap is
// 0 (Squishy's) or the sub-issues could not be read. Past the cap the reconcile fixes it itself.
func SendBackable(ctx *hctx.Context, number int) []any {
	out := []any{}
	limit := ctx.Cfg.FullsendSendBacks
	if limit <= 0 {
		return out
	}
	var children []*pyjson.Object
	if _, failed := herr.Catch[*herr.GitHubError](func() { children = ctx.GH.ListSubIssues(number) }); failed {
		return out
	}
	state := ctx.Store.Load()
	onto := queue.BranchForIssue(number)
	for _, c := range children {
		part := py.MustInt(py.Or(c.Get("number"), 0))
		record := recordOf(state, strconv.Itoa(part))
		if !eqStr(c.Get("state"), "closed") || py.MustInt(py.Or(record.Get("part_of"), 0)) != number ||
			!eqStr(record.Get("onto"), onto) || !record.Has("landed") {
			continue
		}
		builder := queue.PartBuilder(record)
		sent := py.MustInt(py.Or(record.Get("send_backs"), 0))
		if builder == "" || sent >= limit {
			continue
		}
		out = append(out, pyjson.New("part", part, "title", py.OrStr(c.Get("title"), ""),
			"built_by", builder, "sent_backs", sent))
	}
	return out
}

// CILogs is the tails of the failed jobs of one CI run, fenced as data.
func CILogs(ctx *hctx.Context, runID any) string {
	return ciLogs(ctx, runID, false)
}

// CIEarlyNote is what a CI fix of a run the status loop stopped early is told of it (#2948).
var CIEarlyNote = "The harness stopped this CI run early, once a required job had failed on the head " +
	"(#2948): the rest of its jobs were cancelled, not failed, so only the failed ones are below, " +
	"and the required checks that did not run yet run in full on whatever you push."

// ciLogs is CILogs; `early` is a run the status loop stopped once a required job had failed
// (`events.StopRedCI`): its cancelled jobs say nothing, so only the failed ones are given.
func ciLogs(ctx *hctx.Context, runID any, early bool) string {
	if !pyjson.Truthy(runID) {
		return prompts.Data("(no CI run was recorded)", "CI failure")
	}
	red := []string{"failure", "timed_out", "cancelled"}
	if early {
		red = config.CIRed
	}
	var jobs []*pyjson.Object
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		jobs = filter(ctx.GH.ListJobs(runID, false), func(j *pyjson.Object) bool {
			return py.In(j.Get("conclusion"), red...)
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
	if early {
		return CIEarlyNote + "\n\n" + prompts.Data(text, "CI run "+py.Str(runID))
	}
	return prompts.Data(text, "CI run "+py.Str(runID))
}

// CILocalNote is what a CI fix of a pull request's own head is told to do first (#2948, the
// owner's decision of 2026-11-03: no flake re-run before the fix; the fix run judges): run the
// failing check itself. With re-runs of the head left (`ci_reruns`), a pass changes nothing and
// says so, and CI runs again with no commit, which costs none of its CI fixes; with none left, a
// pass is not enough.
func CILocalNote(rerunsLeft int) string {
	first := "Before you change anything, run the failing check locally: the failed jobs' steps as " +
		"`.github/workflows/ci.yml` runs them (CLAUDE.md's Commands name most of them), on this branch " +
		"as it stands, `main` merged in."
	if rerunsLeft > 0 {
		return first + " If it passes and nothing on this branch needs changing for it, change nothing, " +
			"and add `\"ci_local\": \"passed\"` to the first line of your final message: the harness " +
			"then runs CI again on this head with no commit, and that costs none of its CI fixes (once " +
			"per head). If it fails, fix the cause; the harness runs the full local checks before it " +
			"pushes."
	}
	return first + " CI already ran this head again with no commit after a local pass, and it " +
		"failed again, so a local pass is not enough now: find why CI fails where your run passes (a " +
		"flaky or order-dependent test, a timeout, what the runner has that your run has not) and fix " +
		"that. A revision that changes nothing counts as a CI fix that failed."
}

// Chains is whether the run `assignment` makes of `candidate` hands its lane to the change's own
// review run (#2748, the owner's decision of 2026-11-02; `own_review_run`): a build or a revision
// on a subscription with a reviewer of its own, of a bot pull request (a revision a person asked
// for, an auditor's fix, a CI fix, a conflict), whose deliver opens or pushes the pull request and
// reserves the lane for that run. A fullsend tree's reconcile (`reconcile`) chains as any hard
// build does. Not a fullsend part (no review per part), a split (no pull request), a person's pull
// request (its reviewer must be in the run), or a conflict on a cleared change, whose own run's
// review carries its clearance (`deliver.Carry`).
func Chains(cfg *config.Config, candidate *queue.Candidate, assignment *Assignment, names map[string]bool,
	record *pyjson.Object, reconcile bool) bool {
	if !cfg.OwnReviewRun || assignment.Review == nil || candidate.Part {
		return false
	}
	switch candidate.Kind {
	case "build":
		if strings.HasPrefix(candidate.Mode, "split") || (candidate.Mode == "fullsend" && !reconcile) {
			return false
		}
		return assignment.Action == "build"
	case "revise":
		return names[config.LabelPR] && !queue.Carries(record)
	}
	return false
}

// ownReviewPlan puts into an own review run's plan what its build left on the pull request's record
// (`own_review`): the head it pushed, the checks it ran on it, who built it, and why it ran.
func ownReviewPlan(planned *pyjson.Object, record *pyjson.Object) {
	reserved := queue.OwnReviewOf(record)
	planned.Set("own_review", true)
	planned.Set("built_head", py.OrStr(reserved.Get("head"), ""))
	planned.Set("built_gates", py.Or(reserved.Get("gates"), []any{}))
	planned.Set("built_by", reserved.Str("provider"))
	planned.Set("source", py.OrStr(reserved.Get("source"), "build"))
	// The CI fixes the pull request has had (#2948): its final check fixes red CI in the run only
	// while they are not used up (`max_failures`).
	planned.Set("ci_fixes", py.MustInt(py.Or(record.Get("ci_fixes"), 0)))
}
