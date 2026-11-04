package deliver

// A stop only a person can unblock (#2986, sections 2 and 3; the owner's decision of 2026-11-04;
// `queue/decision.go` has the record, `plan/decision.go` the go-ahead). A question goes on
// `bot:blocked` with `needs decision` beside it; its comment lists the options and the default and
// says when it goes ahead, and names the operator. A hand-over (`NeedsPerson`) gets the label too,
// and never goes ahead.

import (
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	planmod "github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	statemod "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
)

// needsDecision puts `needs decision` on `number`, with its colour and description first (a label
// GitHub makes as it is put on is grey and bare).
func (x *Deliverer) needsDecision(number int) {
	x.try(func() {
		spec := pyjson.List(py.Item(config.Labels, config.LabelNeedsDecision))
		x.GH.SyncLabel(config.LabelNeedsDecision, py.Str(spec[0]), py.Str(spec[1]))
	})
	if !x.Labels(number)[config.LabelNeedsDecision] {
		x.GH.AddLabels(number, []string{config.LabelNeedsDecision})
	}
}

// askDecision stops `number` on its question (`lead`, then what it kept, the question, the options
// and the default, and when it goes ahead): `bot:blocked` and `needs decision` go on, the comment
// names the operator and says how to answer (`again`), and the record keeps the question and what was
// asked (`queue.DecisionKey`). `kind` is what goes ahead on the default: `build` or `revise`.
func (x *Deliverer) askDecision(number int, kind, lead, kept, question, again string) {
	queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelBlocked)
	x.needsDecision(number)
	now := x.Ctx.Now()
	asked := x.Result.Obj("decision")
	decision := pyjson.New("asked_at", clock.ISO(now), "kind", kind,
		"question", py.SliceTo(question, queue.QuestionChars), "provider", x.Provider.ID)
	options := pyjson.List(asked.Get("options"))
	chosen := py.MustInt(py.Or(asked.Get("default"), 0))
	if chosen < 1 || chosen > len(options) {
		chosen = 0
	}
	if len(options) > 0 {
		decision.Set("options", options)
	}
	if chosen > 0 {
		model := py.OrStr(asked.Get("model"), "")
		if model == "" && x.BuildSeat != nil {
			model = x.BuildSeat.Model
		}
		decision.Update(pyjson.New("default", chosen, "gives_up", py.OrStr(asked.Get("gives_up"), ""),
			"model", model))
		if x.Cfg.DecisionsGoAhead() {
			decision.Set("hours", x.Cfg.DecisionWaitHours)
			decision.Set("ends", clock.ISO(now.Add(time.Duration(x.Cfg.DecisionWaitHours)*time.Hour)))
		}
	}
	x.remember(number, pyjson.New("question", question, queue.DecisionKey, decision))
	x.GH.CreateComment(number, lead+kept+"\n\n"+question+"\n\n"+x.decisionMd(decision, again))
}

// decisionMd is a question comment's options, its default and when it goes ahead, and whom it asks.
func (x *Deliverer) decisionMd(decision *pyjson.Object, again string) string {
	var lines []string
	options := pyjson.List(decision.Get("options"))
	chosen := py.MustInt(py.Or(decision.Get("default"), 0))
	if len(options) > 0 {
		lines = append(lines, "**The options:**", "")
		for i, option := range options {
			mark := ""
			if i+1 == chosen {
				mark = " **(the default)**"
			}
			lines = append(lines, fmt.Sprintf("%d. %s%s", i+1, strings.Join(strings.Fields(py.Str(option)), " "), mark))
		}
		lines = append(lines, "")
	}
	var when string
	switch {
	case chosen == 0:
		when = "It names no default, so it waits for your answer and never goes ahead by itself."
	case !decision.Truthy("ends"):
		when = fmt.Sprintf("Option %d is its default, but it waits for your answer: this bot never goes "+
			"ahead on a default (`decision_wait_hours` is 0).", chosen)
	default:
		ends := clock.ParseISOValue(decision.Get("ends"))
		when = fmt.Sprintf("If no trusted person answers by %s (%d hours, `decision_wait_hours`), `%s` "+
			"takes it up again on `%s`, the model that asked, in a fresh session and goes ahead on option %d, "+
			"its default; each other option is filed as an issue of its own for a person to approve, and "+
			"nothing queues those.", planmod.DecisionClock(x.Ctx, ends), x.Cfg.DecisionWaitHours,
			py.Str(decision.Get("provider")), py.Str(decision.Get("model")), chosen)
	}
	if gives := py.OrStr(decision.Get("gives_up"), ""); chosen > 0 && gives != "" {
		when += " The default gives up: " + strings.Join(strings.Fields(gives), " ")
		if !strings.HasSuffix(when, ".") {
			when += "."
		}
	}
	lines = append(lines, when, "", fmt.Sprintf("@%s: %s. Your answer wins over the default.", x.Cfg.Operator, again))
	return strings.Join(lines, "\n")
}

// onDefault is the go-ahead this run builds (`plan`'s `on_default`, #2986), or nil: nobody answered
// its question in time, so it builds the default.
func (x *Deliverer) onDefault() *pyjson.Object {
	found, _ := x.Plan.Get("on_default").(*pyjson.Object)
	return found
}

// defaultMd is the pull request's and the comment's words for a run that went ahead on its default.
func (x *Deliverer) defaultMd(number int) string {
	decision := x.onDefault()
	if decision == nil {
		return ""
	}
	asked := clock.ParseISOValue(decision.Get("asked_at"))
	ends := clock.ParseISOValue(decision.Get("ends"))
	text := fmt.Sprintf("This was built on its default because nobody answered its question by %s: "+
		"#%d asked at %s, and no trusted person answered within %d hours (`decision_wait_hours`), so `%s` went "+
		"ahead on option %d, the one inside the issue's own limits: %s", planmod.DecisionClock(x.Ctx, ends), number,
		planmod.DecisionClock(x.Ctx, asked), py.MustInt(py.Or(decision.Get("hours"), x.Cfg.DecisionWaitHours)),
		py.Str(decision.Get("provider")), py.MustInt(decision.Get("default")),
		strings.Join(strings.Fields(queue.DefaultOption(decision)), " "))
	if gives := py.OrStr(decision.Get("gives_up"), ""); gives != "" {
		text += ". It gives up: " + strings.Join(strings.Fields(gives), " ")
	}
	text = strings.TrimSuffix(text, ".") + "."
	var filed []string
	for _, n := range pyjson.List(decision.Get("filed")) {
		filed = append(filed, "#"+py.Str(n))
	}
	if len(filed) > 0 {
		text += fmt.Sprintf(" The other options are %s, for a person to approve.", strings.Join(filed, ", "))
	}
	return text
}

// closeDecision drops the go-ahead this run built once its run is delivered (anything but a run that
// goes back in the queue to go on: an infrastructure fault, a cut-off run or a failure), unless the
// delivery asked a new question, which keeps its own.
func (x *Deliverer) closeDecision(number int, status string) {
	decision := x.onDefault()
	if decision == nil || status == "infra" || status == "interrupted" || status == "failed" {
		return
	}
	x.Ctx.Store.Update(func(s *pyjson.Object) any {
		entry := statemod.Item(s, number)
		if kept := queue.DecisionOf(entry); kept != nil && queue.WentAhead(kept) == queue.WentAhead(decision) {
			entry.Pop(queue.DecisionKey)
		}
		return nil
	}, fmt.Sprintf("default #%s delivered", strconv.Itoa(number)))
}
