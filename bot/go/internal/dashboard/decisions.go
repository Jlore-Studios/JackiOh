package dashboard

// The status issue's "Needs a decision" section (#2986, section 3; the owner's decision of
// 2026-11-04): every open item labelled `needs decision`, how long it has waited and when it goes
// ahead on its default (or why it never does). The night bot draws it at the top of the issue, and
// Squishy at the top of its own section; another bot's item is named, and its own section says when.

import (
	"fmt"
	"strconv"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
)

// DecisionsHeading is the section's heading, and NoDecisions what it says with nothing in it.
var (
	DecisionsHeading = "## ❓ Needs a decision"
	NoDecisions      = "_Nothing waits for a person's decision._"
)

// NeverPerson is what the section says of a stop only a person can unblock.
var NeverPerson = "never: only a person can do this"

// DecisionRows is the section's table: each open item in `issues` labelled `needs decision`, this
// bot's with how long it has waited and when it goes ahead, another's by name; `others` lists
// another bot's too. Empty when there is none.
func DecisionRows(ctx *hctx.Context, state *pyjson.Object, issues []*pyjson.Object, others bool) []string {
	rows := []string{"| Item | Waiting | Goes ahead |", "|---|---|---|"}
	for _, issue := range issues {
		if !queue.LabelNames(issue)[config.LabelNeedsDecision] {
			continue
		}
		number := py.MustInt(py.Or(issue.Get("number"), 0))
		decision := queue.DecisionOf(state.Obj("items").Obj(strconv.Itoa(number)))
		item := fmt.Sprintf("#%d %s", number, Cell(issue.Get("title"), 60))
		if decision == nil {
			if !others {
				continue
			}
			// Not this bot's item: another bot's names it, and one only people can unblock
			// (`human`, `human required`) says so even when no bot's owning label is on it.
			whose := "—"
			if other := queue.Owner(issue); other != nil {
				whose = other.Name + "'s: its section says"
			} else if queue.IsHuman(queue.LabelNames(issue)) {
				whose = NeverPerson
			}
			rows = append(rows, fmt.Sprintf("| %s | — | %s |", item, whose))
			continue
		}
		waited := clock.HumanDelta(ctx.Now().Sub(clock.ParseISOValue(decision.Get("asked_at"))))
		when := goesAhead(ctx, decision)
		if queue.IsHuman(queue.LabelNames(issue)) {
			when = NeverPerson // `human`, `human required`: people's, whatever its default
		}
		rows = append(rows, fmt.Sprintf("| %s | %s | %s |", item, waited, when))
	}
	if len(rows) == 2 {
		return nil
	}
	return rows
}

// goesAhead is when a stop goes ahead on its default, or why it never does.
func goesAhead(ctx *hctx.Context, decision *pyjson.Object) string {
	ends := clock.ParseISOValue(decision.Get("ends"))
	switch {
	case decision.Truthy("human_required"):
		return NeverPerson
	case decision.Truthy("replied"):
		return fmt.Sprintf("never: @%s answered; it resumes once queued again", py.Str(decision.Get("replied")))
	case !queue.HasDefault(decision):
		return "never: it names no default, so it waits for a person's answer"
	case ends.IsZero():
		return "never: `decision_wait_hours` is 0"
	case !ctx.Now().Before(ends):
		return fmt.Sprintf("now, on option %d, its default (at the next sweep)", py.MustInt(decision.Get("default")))
	}
	return fmt.Sprintf("%s (in %s), on option %d, its default", plan.DecisionClock(ctx, ends),
		clock.HumanDelta(ends.Sub(ctx.Now())), py.MustInt(decision.Get("default")))
}

// DecisionsSection is the section: its heading, and its table or a line saying nothing waits.
func DecisionsSection(ctx *hctx.Context, state *pyjson.Object, issues []*pyjson.Object) []string {
	lines := []string{DecisionsHeading, ""}
	if rows := DecisionRows(ctx, state, issues, true); len(rows) > 0 {
		return append(append(lines, rows...), "")
	}
	return append(lines, NoDecisions, "")
}
