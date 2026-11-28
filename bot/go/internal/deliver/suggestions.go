package deliver

import (
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/asks"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/declined"
	planmod "github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
)

// ------------------------------------------------------------------ suggestions

// Suggestions is Deliverer._suggestions.
func (x *Deliverer) Suggestions() {
	if x.Result.Get("status") == "infra" {
		x.Chain = false
	}
	// First, and whether or not the survey finished: the judge's word on suggestions an earlier
	// judge never answered (#5104).
	x.applyRejudged()
	lane := planmod.SurveyLaneOf(x.Plan) // its own record (#4026), or lane 0 for a plan from before
	if x.Result.Get("status") != "suggested" {
		// A survey that did not finish gives its slot back: it is due again as before, and its
		// asks wait for the next one.
		before := pyjson.New("last_run", x.Plan.Get("previous_last_run"),
			"requested", x.Plan.Truthy("was_requested"),
			"idle_since", x.Plan.Get("previous_idle_since"))

		unfinished := func(s *pyjson.Object) any {
			ended := planmod.EndSurvey(s, lane)
			suggest := s.Obj("suggest")
			suggest.Update(before)
			asks.Add(suggest, planmod.SurveyAsks(ended))
			return nil
		}

		x.Ctx.Store.Update(unfinished, "survey unfinished")
	} else {
		var settled []string

		done := func(s *pyjson.Object) any {
			settled = planmod.SurveyAsks(planmod.EndSurvey(s, lane))
			return nil
		}

		x.Ctx.Store.Update(done, "survey asks")
		asks.React(x.GH, settled, asks.Done)
	}
	found := objList(x.Result.Get("suggestions"))
	openNow := 0
	for _, i := range x.GH.ListIssues(config.LabelSuggestion, "open", 300, "") {
		if !i.Has("pull_request") {
			openNow++
		}
	}
	room := max(0, x.Cfg.SuggestionsMaxOpen-openNow)
	made := []any{}
	state := x.Ctx.Store.Load()
	var fresh []*pyjson.Object
	for _, entry := range found {
		if declined.Known(state, py.OrStr(entry.Get("title"), "")) {
			// It came to nothing before (#797): the survey was told, and it is not opened.
			x.log("dropped a suggestion that came to nothing before: " + py.Str(entry.Get("title")))
			continue
		}
		fresh = append(fresh, entry)
	}
	for _, entry := range headN(fresh, room) {
		title := strings.TrimSpace(py.OrStr(entry.Get("title"), ""))
		body := strings.TrimSpace(py.OrStr(entry.Get("body"), ""))
		if title == "" || body == "" {
			continue
		}
		footer := fmt.Sprintf("\n\n---\nSuggested by @%s while the queue was empty. "+
			"Comment `approve` to have it built (the "+
			"`%s` label works too), and the bot rates its difficulty when "+
			"it plans it. Comment `reject` to close it without building it.", x.Cfg.BotLogin, config.LabelBuild)
		if area := x.Plan.Str("area"); area != "" {
			footer += fmt.Sprintf("\n\nArea: `%s` (`suggestions.areas`).", area) // #4026
		}
		labels := []string{config.LabelSuggestion}
		// Its class sets its priority (#1409); a class with none, or no class, sets nothing.
		kind := py.OrStr(entry.Get("class"), "")
		if level := x.Cfg.SuggestionPriorities[kind]; level != "" {
			labels = append(labels, "priority:"+level)
			footer += fmt.Sprintf("\n\nClass: `%s` → `priority:%s`; relabel to change it.", kind, level)
		}
		// The judge's word on it (`suggestions.auto_approve`): one it would approve goes ahead by
		// itself once it has waited with no one's vote, comment or 👎 (`plan.AutoApprove`), or as it
		// opens when `wait_hours` is 0 (#5104).
		judge := pyjson.Obj(entry.Get("judge"))
		auto := x.Cfg.AutoApprove
		approve := auto.Enabled && judge != nil && judge.Truthy("approve")
		if judge != nil && auto.Enabled {
			why := py.OrStr(judge.Get("why"), "")
			if why != "" {
				why = ": " + strings.TrimRight(why, ".")
			}
			// `per_day` 0 caps none, so the footer names a cap only when there is one.
			capped := ""
			if auto.PerDay > 0 {
				capped = fmt.Sprintf(" (at most %d a day; one past that waits for room)", auto.PerDay)
			}
			if approve && auto.WaitHours == 0 {
				footer += fmt.Sprintf("\n\n**Judge:** would approve it%s, so it is approved by itself as it opens, "+
					"at `priority:low`%s. `%s stop` takes it back out of the queue.", why, capped, config.Slash)
			} else if approve {
				due := x.Ctx.Now().Add(time.Duration(auto.WaitHours) * time.Hour)
				footer += fmt.Sprintf("\n\n**Judge:** would approve it%s. With no vote, no comment and no 👎 on "+
					"this issue by %s, it is approved by itself at `priority:low`%s; "+
					"comment `reject`, or anything at all, to decide it yourself.", why,
					planmod.DecisionClock(x.Ctx, due), capped)
			} else {
				footer += fmt.Sprintf("\n\n**Judge:** leaves it to a person%s.", why)
			}
		}
		issue := x.GH.CreateIssue(title, body+footer, labels)
		made = append(made, issue.Get("number"))
		if approve {
			number := py.MustInt(issue.Get("number"))
			x.Ctx.Store.Update(func(s *pyjson.Object) any {
				planmod.MarkAutoApprove(s, number, clock.ISO(x.Ctx.Now()), py.OrStr(judge.Get("why"), ""))
				return nil
			}, fmt.Sprintf("suggestion #%d may approve itself", number))
		}
	}
	x.log("opened suggestions: " + py.Repr(made))
	// #5104: with no wait, each one the judge would approve is queued now, not at a later sweep.
	if auto := x.Cfg.AutoApprove; auto.Enabled && auto.WaitHours == 0 {
		for _, note := range planmod.AutoApprove(x.Ctx) {
			x.log(note)
		}
	}
}

// ------------------------------------------------------------------ the next run

// Chain_ is Deliverer._chain (the field `Chain` holds `self.chain`): start the next run at once
// when a lane is free and a subscription can take more.
func (x *Deliverer) Chain_() {
	if !x.Chain {
		return
	}
	if planmod.LoopAlive(x.Ctx) {
		x.log("the status loop starts the next run (#1455)")
		return
	}
	look := planmod.Peek_(x.Ctx, false, nil, "auto")
	if look.Work {
		x.Ctx.Dispatch(nil, false, "auto")
		x.log("dispatched the next run: " + look.Reason)
	}
}

// applyRejudged puts the judge's word on each suggestion an earlier judge never answered (the
// result's `rejudged`, `work.Worker.rejudge`), as a comment carrying `plan.JudgeMarker`, so no later
// survey judges it again. One it would approve is kept for `plan.AutoApprove`, which queues it as it
// queues one approved as it opened. One it still could not answer counts a try
// (`suggest.judge_tries`), and after `plan.RejudgeTries` it is left to a person with a comment that
// says so. A suggestion a person decided in the meantime (closed, or no longer `bot:suggestion`) is
// left alone.
func (x *Deliverer) applyRejudged() {
	auto := x.Cfg.AutoApprove
	for _, entry := range objList(x.Result.Get("rejudged")) {
		number := py.MustInt(entry.Get("number"))
		issue := x.GH.GetIssue(number)
		if issue.Get("state") != "open" || !queue.LabelNames(issue)[config.LabelSuggestion] {
			x.forgetJudgeTries(number)
			continue
		}
		judge := pyjson.Obj(entry.Get("judge"))
		if judge == nil {
			tries := 0
			x.Ctx.Store.Update(func(s *pyjson.Object) any {
				counts := pyjson.Obj(pyjson.Obj(s.SetDefault("suggest", pyjson.New())).SetDefault("judge_tries", pyjson.New()))
				tries = py.MustInt(py.Or(counts.Get(strconv.Itoa(number)), 0)) + 1
				counts.Set(strconv.Itoa(number), tries)
				return nil
			}, fmt.Sprintf("suggestion #%d not judged yet", number))
			if tries < planmod.RejudgeTries {
				x.log(fmt.Sprintf("#%d: the judge could not answer it (try %d of %d)", number, tries, planmod.RejudgeTries))
				continue
			}
			x.GH.CreateComment(number, fmt.Sprintf("%s could not answer after %d tries, so this suggestion "+
				"waits for a person's vote: comment `approve` or `reject`.\n\n%s\n%s", planmod.JudgeLine, tries,
				planmod.JudgeMarker, config.Marker))
			x.forgetJudgeTries(number)
			continue
		}
		why := strings.TrimRight(py.OrStr(judge.Get("why"), ""), ".")
		if why != "" {
			why = ": " + why
		}
		if judge.Truthy("approve") {
			when := "now"
			if auto.WaitHours > 0 {
				when = fmt.Sprintf("once it has waited %d hours with no one's vote, comment or 👎", auto.WaitHours)
			}
			x.GH.CreateComment(number, fmt.Sprintf("%s would approve it%s. It opened before the judge could "+
				"answer, so it is approved by itself %s, at `priority:low`. `%s stop` takes it back out of the "+
				"queue.\n\n%s\n%s", planmod.JudgeLine, why, when, config.Slash, planmod.JudgeMarker, config.Marker))
			x.Ctx.Store.Update(func(s *pyjson.Object) any {
				planmod.MarkAutoApprove(s, number, clock.ISO(x.Ctx.Now()), py.OrStr(judge.Get("why"), ""))
				return nil
			}, fmt.Sprintf("suggestion #%d may approve itself", number))
		} else {
			x.GH.CreateComment(number, fmt.Sprintf("%s leaves it to a person%s.\n\n%s\n%s", planmod.JudgeLine,
				why, planmod.JudgeMarker, config.Marker))
		}
		x.forgetJudgeTries(number)
	}
}

// forgetJudgeTries drops suggestion `number`'s count of surveys that could not judge it.
func (x *Deliverer) forgetJudgeTries(number int) {
	if x.Ctx.Store.Load().Obj("suggest").Obj("judge_tries").Has(strconv.Itoa(number)) {
		x.Ctx.Store.Update(func(s *pyjson.Object) any {
			s.Obj("suggest").Obj("judge_tries").Delete(strconv.Itoa(number))
			return nil
		}, fmt.Sprintf("suggestion #%d judged", number))
	}
}
