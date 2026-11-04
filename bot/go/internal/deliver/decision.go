package deliver

// A question a person answers and the bot carries out (#2986, sections 2 and 3; #3009, section 1;
// the owner's decisions of 2026-11-04; `queue/decision.go` has the record, `plan/decision.go` the
// answer and the go-ahead). A question goes on `bot:blocked` with `needs decision` beside it and
// assigns `decision_assignees`; its comment explains the problem, numbers the options with their
// trade-offs, names the one it suggests and says how to answer and when it goes ahead. A hand-over
// (`NeedsPerson`) is no question: it never gets the label.

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

// askDecision stops `number` on its question (`lead`, then what it kept, then the problem, the
// numbered options with their trade-offs, the one it suggests and why, and how to answer, #3009):
// `bot:blocked` and `needs decision` go on, `decision_assignees` are assigned, and the record keeps
// the question and what was asked (`queue.DecisionKey`), with the people it assigned. `kind` is what
// goes ahead on the default: `build` or `revise`.
func (x *Deliverer) askDecision(number int, kind, lead, kept, question string) {
	queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelBlocked)
	x.needsDecision(number)
	now := x.Ctx.Now()
	asked := x.Result.Obj("decision")
	model := py.OrStr(asked.Get("model"), "")
	if model == "" && x.BuildSeat != nil {
		model = x.BuildSeat.Model
	}
	decision := pyjson.New("asked_at", clock.ISO(now), "kind", kind,
		"question", py.SliceTo(question, queue.QuestionChars), "provider", x.Provider.ID, "model", model)
	if problem := strings.TrimSpace(py.OrStr(asked.Get("problem"), "")); problem != "" {
		decision.Set("problem", py.SliceTo(problem, queue.QuestionChars))
	}
	options := pyjson.List(asked.Get("options"))
	chosen := py.MustInt(py.Or(asked.Get("default"), 0))
	if chosen < 1 || chosen > len(options) {
		chosen = 0
	}
	if len(options) > 0 {
		decision.Set("options", options)
	}
	if chosen > 0 {
		decision.Update(pyjson.New("default", chosen, "why", py.OrStr(asked.Get("why"), "")))
		if x.Cfg.DecisionsGoAhead() {
			decision.Set("hours", x.Cfg.DecisionWaitHours)
			decision.Set("ends", clock.ISO(now.Add(time.Duration(x.Cfg.DecisionWaitHours)*time.Hour)))
		}
	}
	decision.Set("assigned", x.assignDecision(number))
	x.remember(number, pyjson.New("question", question, queue.DecisionKey, decision))
	x.GH.CreateComment(number, lead+kept+"\n\n"+x.decisionMd(decision))
}

// assignDecision assigns `decision_assignees` to `number` (#3009), those not assigned already, and
// returns them: the stop unassigns only those, so a person who was assigned already stays.
func (x *Deliverer) assignDecision(number int) []any {
	have := map[string]bool{}
	for _, a := range py.Each(x.GH.GetIssue(number).Get("assignees")) {
		have[strings.ToLower(py.OrStr(pyjson.Obj(a).Get("login"), ""))] = true
	}
	var add []string
	for _, login := range x.Cfg.DecisionAssignees {
		if !have[strings.ToLower(login)] {
			have[strings.ToLower(login)] = true
			add = append(add, login)
		}
	}
	if len(add) == 0 {
		return []any{}
	}
	if refused := x.try(func() { x.GH.AddAssignees(number, add) }); refused != "" {
		x.log(fmt.Sprintf("#%d: could not assign %s: %s", number, strings.Join(add, ", "), refused))
		return []any{}
	}
	return pyjson.ListOf(add)
}

// decisionMd is a question comment after its lead (#3009): the problem (an older report's question),
// the numbered options with their trade-offs, the one it suggests and why, and how to answer.
func (x *Deliverer) decisionMd(decision *pyjson.Object) string {
	var lines []string
	problem := strings.TrimSpace(py.OrStr(decision.Get("problem"), ""))
	question := strings.TrimSpace(py.OrStr(decision.Get("question"), ""))
	switch {
	case problem == "":
		lines = append(lines, question, "")
	case question != "":
		lines = append(lines, problem, "", "**The question:** "+question, "")
	default:
		lines = append(lines, problem, "")
	}
	options := queue.OptionsOf(decision)
	chosen := py.MustInt(py.Or(decision.Get("default"), 0))
	if len(options) > 0 {
		lines = append(lines, "**My recommendations:**", "")
		for i, option := range options {
			line := fmt.Sprintf("%d. %s", i+1, oneLine(option.Change))
			if option.Tradeoff != "" {
				line += " *Trade-off:* " + sentence(option.Tradeoff)
			}
			lines = append(lines, line)
		}
		lines = append(lines, "")
	}
	switch {
	case chosen > 0:
		why := oneLine(py.OrStr(decision.Get("why"), ""))
		if why == "" {
			why = "it stays inside the issue's own stated limits"
		}
		lines = append(lines, fmt.Sprintf("**My suggestion:** option %d, because %s", chosen, sentence(why)), "")
	case len(options) > 0:
		lines = append(lines, "**My suggestion:** none of them: no option stays inside the issue's own "+
			"limits, so this waits for your answer.", "")
	}
	answer := "Reply with a number, or tell me something else in your own words, and I'll revise it that way."
	switch {
	case chosen > 0 && decision.Truthy("ends"):
		ends := clock.ParseISOValue(decision.Get("ends"))
		answer += fmt.Sprintf(" If nobody answers by %s, I go ahead with option %d. (After %d hours, "+
			"`decision_wait_hours`, `%s` takes it up again on `%s`, the model that asked, in a fresh "+
			"session; each other option is filed as an issue of its own for a person to approve, and "+
			"nothing queues those.)", planmod.DecisionClock(x.Ctx, ends), chosen, x.Cfg.DecisionWaitHours,
			py.Str(decision.Get("provider")), py.Str(decision.Get("model")))
	case chosen > 0:
		answer += " I never go ahead by myself here (`decision_wait_hours` is 0): it waits for your answer."
	default:
		answer += " With no suggestion, it waits for your answer: I never go ahead by myself."
	}
	return strings.Join(append(lines, answer), "\n")
}

// oneLine is `text` on one line, its whitespace collapsed.
func oneLine(text string) string { return strings.Join(strings.Fields(text), " ") }

// sentence is `text` on one line, ending with a full stop.
func sentence(text string) string {
	text = oneLine(text)
	if text == "" || strings.HasSuffix(text, ".") || strings.HasSuffix(text, "!") || strings.HasSuffix(text, "?") {
		return text
	}
	return text + "."
}

// onDefault is the go-ahead this run builds (`plan`'s `on_default`, #2986), or nil: nobody answered
// its question in time, so it builds the default.
func (x *Deliverer) onDefault() *pyjson.Object {
	found, _ := x.Plan.Get("on_default").(*pyjson.Object)
	return found
}

// resumed is the stop this run took up again (`plan`'s `on_default`, or `on_answer`: a trusted
// person's reply, #3009), or nil.
func (x *Deliverer) resumed() *pyjson.Object {
	if found := x.onDefault(); found != nil {
		return found
	}
	found, _ := x.Plan.Get("on_answer").(*pyjson.Object)
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
	if chosen := py.MustInt(py.Or(decision.Get("default"), 0)); chosen >= 1 && chosen <= len(queue.OptionsOf(decision)) {
		if gives := queue.OptionsOf(decision)[chosen-1].Tradeoff; gives != "" {
			text += ". Its trade-off: " + strings.Join(strings.Fields(gives), " ")
		}
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

// closeDecision drops the stop this run took up again (its default's go-ahead, or a person's
// answer) once its run is delivered (anything but a run that goes back in the queue to go on: an
// infrastructure fault, a cut-off run or a failure), unless the delivery asked a new question, which
// keeps its own.
func (x *Deliverer) closeDecision(number int, status string) {
	decision := x.resumed()
	if decision == nil || status == "infra" || status == "interrupted" || status == "failed" {
		return
	}
	x.Ctx.Store.Update(func(s *pyjson.Object) any {
		entry := statemod.Item(s, number)
		if kept := queue.DecisionOf(entry); kept != nil && queue.Resumes(kept) != "" &&
			kept.Str("asked_at") == decision.Str("asked_at") {
			entry.Pop(queue.DecisionKey)
		}
		return nil
	}, fmt.Sprintf("decision #%s delivered", strconv.Itoa(number)))
}
