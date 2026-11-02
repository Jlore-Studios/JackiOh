package deliver

import (
	"fmt"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/spotcheck"
	statemod "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
)

// FollowUpFindings is how many of a spot-check's findings its follow-up issue lists, and
// FindingChars how much of each part of one.
var (
	FollowUpFindings = 20
	FindingChars     = 1500
)

// SpotCheck is the end of an after-merge spot-check (#2748, section 5; `spotcheck`): a review run
// of a pull request that has merged. It blocks nothing and reverts nothing. An approval (or a
// verdict with no blocking finding) records `clean`; blocking findings open one follow-up issue,
// queued to build at the merged change's difficulty (`followUp`), with a comment on the pull request that
// links it, and record `findings`. A run that was cut off, or could not work, puts the check back
// in the queue; one that failed spends a try, and the `max_failures`th records it `failed`.
func (x *Deliverer) SpotCheck(number int, status string) {
	// The run is over: the closed pull request loses its live label.
	x.try(func() { queue.SetStateLabel(x.Ctx, number, x.Labels(number), "") })
	if spotcheck.Find(x.Ctx.Store.Load(), number) == nil {
		x.log(fmt.Sprintf("#%d: no spot-check of it is open any more", number))
		return
	}
	now := x.Ctx.Now()
	reason := py.OrStr(x.Result.Get("reason"), "no reason recorded")
	if status != "reviewed" {
		limit := x.Cfg.MaxFailures
		if status == "infra" || status == "interrupted" {
			limit = 0 // not the check's fault: no try spent
			if status == "infra" {
				x.noteInfra(reason)
			}
		}
		var gaveUp bool
		x.Ctx.Store.Update(func(s *pyjson.Object) any {
			gaveUp = spotcheck.Requeue(s, number, status+": "+reason, limit, now)
			return nil
		}, fmt.Sprintf("spot-check #%d not done", number))
		if gaveUp {
			x.log(fmt.Sprintf("#%d: the spot-check failed %d times, so it is recorded failed: %s", number, limit, reason))
		} else {
			x.log(fmt.Sprintf("#%d: the spot-check ended %s, so it is queued again: %s", number, status, reason))
		}
		return
	}
	findings := objList(x.Result.Get("findings"))
	if x.Result.Get("verdict") == "approve" || len(findings) == 0 {
		x.Ctx.Store.Update(func(s *pyjson.Object) any {
			spotcheck.Finish(s, number, pyjson.New("verdict", spotcheck.Clean, "findings", 0, "run", x.Cfg.RunURL()), now)
			return nil
		}, fmt.Sprintf("spot-check #%d clean", number))
		x.log(fmt.Sprintf("#%d: the after-merge spot-check found nothing blocking", number))
		return
	}
	difficulty := spotcheck.FollowUpDifficulty(spotcheck.Find(x.Ctx.Store.Load(), number))
	follow, problem := x.followUp(number, findings)
	x.Ctx.Store.Update(func(s *pyjson.Object) any {
		fields := pyjson.New("verdict", spotcheck.Findings, "findings", len(findings), "follow_up", follow,
			"run", x.Cfg.RunURL())
		if problem != "" {
			fields.Set("reason", py.SliceTo(problem, 300))
		}
		spotcheck.Finish(s, number, fields, now)
		return nil
	}, fmt.Sprintf("spot-check #%d findings", number))
	if follow == 0 {
		x.log(fmt.Sprintf("#%d: the spot-check found %d blocking problem(s), but the follow-up issue "+
			"could not be opened: %s", number, len(findings), problem))
		return
	}
	x.try(func() {
		x.GH.CreateComment(number, fmt.Sprintf("After-merge spot-check (%s): %s found %d blocking "+
			"problem(s) in this change as it merged. It blocks nothing and reverts nothing: #%d follows "+
			"them up, queued to build at `difficulty:%s`.\n\n%s", x.link(), x.spotChecker(),
			len(findings), follow, difficulty, x.findingsMd()))
	})
	x.log(fmt.Sprintf("#%d: the spot-check found %d blocking problem(s): opened #%d", number, len(findings), follow))
}

// followUp opens the follow-up issue of a spot-check of pull request `number` that found
// `findings`: `spotcheck.Title`, its body the findings with their evidence, labelled `bot:build`
// and the merged change's difficulty (`spotcheck.FollowUpDifficulty`, recorded as the bot's own
// rating, so a planner may raise a medium one) and
// recorded as `spot_check_of`, so its own pull request is never spot-checked. Its number, or 0
// and why not.
func (x *Deliverer) followUp(number int, findings []*pyjson.Object) (int, string) {
	entry := spotcheck.Find(x.Ctx.Store.Load(), number)
	if entry == nil {
		entry = pyjson.New()
	}
	var lines []string
	for _, f := range headN(findings, FollowUpFindings) {
		clip := func(key string, fallback string) string {
			text := strings.Join(strings.Fields(redact.Redact(py.OrStr(f.Get(key), fallback), nil)), " ")
			return py.SliceTo(text, FindingChars)
		}
		line := fmt.Sprintf("- [ ] `%s`: %s", clip("where", "?"), clip("claim", ""))
		if evidence := clip("evidence", ""); evidence != "" {
			line += "\n  Evidence: " + evidence
		}
		lines = append(lines, line)
	}
	sha := py.SliceTo(entry.Str("sha"), 12)
	who := func(key string) string {
		seat := entry.Obj(key)
		if seat.Str("provider") == "" {
			return "no subscription on record"
		}
		model := seat.Str("family")
		if seat.Str("model") != "" {
			model = seat.Str("model")
		}
		return fmt.Sprintf("`%s` (`%s`)", seat.Str("provider"), model)
	}
	body := strings.Join([]string{
		spotcheck.Marker(number),
		fmt.Sprintf("The after-merge spot-check of #%d (%s) found %d blocking problem(s) in the change as "+
			"it merged (`%s`). %s reviewed it. #%d stays merged: nothing is reverted, and this issue fixes "+
			"what it found in a change of its own, from `%s`.", number, x.link(), len(findings), sha,
			x.spotChecker(), number, x.Cfg.DefaultBranch),
		"",
		"## What to fix",
		"",
		strings.Join(lines, "\n"),
		"",
		"## Done when",
		"",
		"- Each finding above is fixed where it points, or answered with the evidence that it does not hold.",
		"- Every check is green, with no test skipped, ignored or weakened to get there (CLAUDE.md rule 12).",
		"",
		"---",
		fmt.Sprintf("Opened by @%s from the after-merge spot-check (one in %d medium or hard merges, "+
			"`spot_check_every`): %s built #%d and %s approved it. This issue's own pull request is "+
			"never spot-checked.", x.Cfg.BotLogin, py.MustInt(py.Or(entry.Get("every"), x.Cfg.SpotCheckEvery)),
			who("builder"), number, who("approver")),
	}, "\n")
	difficulty := spotcheck.FollowUpDifficulty(entry)
	labels := []string{config.LabelBuild, "difficulty:" + difficulty}
	var issue *pyjson.Object
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		issue = x.GH.CreateIssue(spotcheck.Title(number), body, labels)
	}); failed {
		return 0, exc.Error()
	}
	follow := py.MustInt(py.Or(issue.Get("number"), 0))
	if follow == 0 {
		return 0, "GitHub gave the new issue no number"
	}
	now := clock.ISO(x.Ctx.Now())
	x.Ctx.Store.Update(func(s *pyjson.Object) any {
		statemod.Item(s, follow).Update(pyjson.New("kind", "build", "queued_at", now,
			"requested_by", x.Cfg.BotLogin, "forced", false, "stop_requested", false, "failures", 0,
			"interruptions", 0, "pending_request", false, "spot_check_of", number,
			"difficulty_by", pyjson.New("difficulty", difficulty, "provider", x.Provider.ID,
				"tier", spotcheck.Tier, "at", now,
				"why", fmt.Sprintf("the after-merge spot-check's follow-up of #%d, a %s change", number,
					difficulty))))
		return nil
	}, fmt.Sprintf("follow-up #%d of #%d", follow, number))
	return follow, ""
}

// spotChecker is who reviewed the merged change: the run's review seat.
func (x *Deliverer) spotChecker() string {
	if x.ReviewSeat != nil {
		return x.ReviewSeat.Describe()
	}
	return x.Provider.Describe()
}
