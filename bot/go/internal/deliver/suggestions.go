package deliver

import (
	"fmt"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/asks"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/declined"
	planmod "github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// ------------------------------------------------------------------ suggestions

// Suggestions is Deliverer._suggestions.
func (x *Deliverer) Suggestions() {
	if x.Result.Get("status") == "infra" {
		x.Chain = false
	}
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
		// itself once it has waited with no one's vote, comment or 👎 (`plan.AutoApprove`).
		judge := pyjson.Obj(entry.Get("judge"))
		auto := x.Cfg.AutoApprove
		approve := auto.Enabled && judge != nil && judge.Truthy("approve")
		if judge != nil && auto.Enabled {
			why := py.OrStr(judge.Get("why"), "")
			if why != "" {
				why = ": " + strings.TrimRight(why, ".")
			}
			if approve {
				due := x.Ctx.Now().Add(time.Duration(auto.WaitHours) * time.Hour)
				footer += fmt.Sprintf("\n\n**Judge:** would approve it%s. With no vote, no comment and no 👎 on "+
					"this issue by %s, it is approved by itself at `priority:low` (at most %d a day); "+
					"comment `reject`, or anything at all, to decide it yourself.", why,
					planmod.DecisionClock(x.Ctx, due), auto.PerDay)
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
