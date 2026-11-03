package plan

import (
	"fmt"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/journal"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/spotcheck"
	hstate "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/threads"
)

// claimSpotCheck is `Claim` for an after-merge spot-check (#2748, section 5; `spotcheck`): the plan
// of a review run of a pull request that has merged, at the commit that merged it (`merge_sha`,
// which the work job checks out against its first parent). The closed pull request carries the
// run's live label (`bot:reviewing`) while the run holds it, so its lane counts (`ReadLanes`) and
// housekeeping puts the check back in the queue if the run dies (`Housekeeping`); no comment is
// left, since a spot-check blocks nothing. nil when there is nothing to review: the check is
// recorded `failed` then.
func claimSpotCheck(ctx *hctx.Context, candidate *queue.Candidate, assignment *Assignment) *pyjson.Object {
	cfg := ctx.Cfg
	number := candidate.Number
	entry := spotcheck.Find(ctx.Store.Load(), number)
	if entry == nil {
		return nil
	}
	pull := ctx.GH.GetPull(number)
	sha := entry.Str("sha")
	if sha == "" {
		sha = py.OrStr(pull.Get("merge_commit_sha"), "")
	}
	if !pull.Truthy("merged") || sha == "" {
		ctx.Store.Update(func(s *pyjson.Object) any {
			spotcheck.Finish(s, number, pyjson.New("verdict", spotcheck.Failed, "findings", 0,
				"reason", "no merged commit of it is on record to review"), ctx.Now())
			return nil
		}, fmt.Sprintf("spot-check #%d has nothing to review", number))
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
	approver := entry.Obj("approver")
	approved := "an approval no record names"
	if who := approver.Str("provider"); who != "" {
		approved = fmt.Sprintf("`%s`'s approval", who)
		if approver.Str("model") != "" {
			approved = fmt.Sprintf("`%s`'s approval (`%s`, %s)", who, approver.Str("model"), approver.Str("tier"))
		}
	}
	planned := pyjson.New(
		"action", "review",
		"number", number,
		"title", pull.GetOr("title", ""),
		"branch", pull.Obj("head").GetOr("ref", ""),
		"head", sha,
		"spot_check", true,
		"merge_sha", sha,
		"builder", candidate.Builder,
		"approver", approved,
		"approved", []any{},
		"pull", pullText,
		"issue", issueText,
		"thread", joinNonEmpty("\n\n", pullText, issueText),
		"bot_pr", true,
		"issue_number", intOrNone(issueNumber),
		"review_notes", "",
	)
	planned.Update(providerFields(ctx, assignment.Provider))
	if assignment.RunsOn != "" {
		planned.Set("runs_on", assignment.RunsOn)
	}
	planned.Set("difficulty", candidate.Difficulty)
	planned.Set("seats", assignment.Seats())
	planned.Set("routing", pyjson.ListOf(assignment.Notes))
	planned.Set("assignment", assignment.Describe())
	journalOf := journal.Key(planned)
	if journalOf == 0 {
		journalOf = number
	}
	if text := journal.Read(ctx.GH, journalOf, journal.ReadChars); text != "" {
		planned.Set("journal", text)
	}
	live := config.LiveRun("review", "review", "", "request", false, true)
	queue.SetStateLabel(ctx, number, queue.LabelNames(ctx.GH.GetIssue(number)), config.LiveLabel(live))
	seat := assignment.Review
	reviewer := pyjson.New("provider", assignment.Provider.ID, "family", assignment.Provider.Family)
	if seat != nil {
		reviewer.Set("family", seat.Family()).Set("model", seat.Model).Set("tier", seat.Tier)
	}
	ctx.Store.Update(func(state *pyjson.Object) any {
		hstate.Item(state, number).Update(pyjson.New("run_id", cfg.RunID, "started_at", clock.ISO(ctx.Now()),
			"kind", "review", "action", "review", "stop_requested", false, "pending_request", false,
			"provider", assignment.Provider.ID, "runs_on", planned.Get("runs_on"), "live", live,
			"spot_check", true))
		spotcheck.Start(state, number, cfg.RunID, reviewer, ctx.Now())
		state.Set("last_run", pyjson.New("at", clock.ISO(ctx.Now()), "url", cfg.RunURL(),
			"what", fmt.Sprintf("spot-check #%d on %s", number, assignment.Provider.ID)))
		return nil
	}, fmt.Sprintf("claim the spot-check of #%d", number))
	return planned
}
