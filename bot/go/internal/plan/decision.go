package plan

// A question a person answers and the bot carries out (#2986, sections 2 and 3; #3009, section 1;
// the owners' decisions of 2026-11-04; `queue/decision.go` has the record). `SyncDecisions`, in
// every plan's housekeeping and in the sweep, keeps the stops for a decision, and `SyncDecisionsOn`
// does it at once for the item a comment that is no command lands on (`events.OnComment`):
//
//   - one whose thread closed loses `needs decision`, the people it assigned and its record;
//   - one handed to a person (`human required`; #2986 put the label on those too) loses the label
//     and its record, and a hand-over of #2986's is archived as #3009's are (`queue.Archive`);
//   - one that waits no more (a person queued it with a command, which takes the label off, or
//     took the label or `bot:blocked` off) loses its record, the label and the people it assigned;
//   - one a trusted person answered (`reply`: their first comment after the question, on the item
//     or its pull request, that is no command and no session's handoff) is taken up again at once
//     (`answer`): the subscription that asked owns it, and takes it up on the model that asked, in a
//     fresh session, by the ownership and hold rules, as a build of an issue or a revision
//     (`source: decision`) of a pull request, with the reply quoted to it as the decision; `needs
//     decision`, `bot:blocked` and the people it assigned come off;
//   - one with a default past its end (`decision_wait_hours` after it was asked) and no answer goes
//     ahead (`goAhead`) the same way, to build its default; each other option is filed as an issue of
//     its own for a person to approve, and nothing queues those.
//
// A question with no default never goes ahead, and one on an item a person labelled `human` or
// `tracking` waits for them. `OlderQuestions`, in the sweep, labels the items that stopped on a
// question before #2986, with no deadline; `HandedOver` takes `needs decision` off every open
// `human required` item, another bot's (#2980) among them.

import (
	"fmt"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/commands"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/prompts"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	hstate "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
)

// OptionTitleChars is how much of its thread's title an option's issue keeps.
var OptionTitleChars = 200

// DecisionClock is a question's time as its comments say it: the day, the date and the local time.
func DecisionClock(ctx *hctx.Context, at time.Time) string {
	return at.In(clock.Zone(ctx.Cfg.Timezone)).Format("Mon 2 Jan 15:04 MST")
}

// SyncDecisions keeps the stops for a decision (the package's rules above). Returns what it did.
func SyncDecisions(ctx *hctx.Context) []string {
	return syncDecisions(ctx, 0)
}

// SyncDecisionsOn is SyncDecisions for the stops a comment on thread `thread` may answer: the item
// itself, or the issue whose pull request it is (#3009). Returns what it did.
func SyncDecisionsOn(ctx *hctx.Context, thread int) []string {
	return syncDecisions(ctx, thread)
}

// syncDecisions is SyncDecisions for every stop (`thread` 0), or for those a comment on `thread` may
// answer.
func syncDecisions(ctx *hctx.Context, thread int) []string {
	notes := []string{}
	state := ctx.Store.Load()
	for _, number := range queue.Decisions(state) {
		record := recordOf(state, strconv.Itoa(number))
		if thread != 0 && number != thread && py.MustInt(py.Or(record.Get("pr"), 0)) != thread {
			continue
		}
		decision := queue.DecisionOf(record)
		var note string
		if exc := herr.Try(func() { note = syncDecision(ctx, number, record, decision) }); exc != nil {
			note = fmt.Sprintf("#%d: could not see whether its question is answered or goes ahead (%s)", number,
				py.SliceTo(herr.Str(exc), 200))
		}
		if note != "" {
			notes = append(notes, note)
		}
	}
	return notes
}

// dropDecision forgets `number`'s stop for a decision.
func dropDecision(ctx *hctx.Context, number int) {
	ctx.Store.Update(func(s *pyjson.Object) any {
		hstate.Item(s, number).Pop(queue.DecisionKey)
		return nil
	}, fmt.Sprintf("decision #%d dropped", number))
}

// unassign takes off `number` the people its stop assigned (`queue.Assigned`, #3009): a person who
// was assigned before it stays.
func unassign(ctx *hctx.Context, number int, decision *pyjson.Object) {
	if who := queue.Assigned(decision); len(who) > 0 {
		herr.Catch[*herr.GitHubError](func() { ctx.GH.RemoveAssignees(number, who) })
	}
}

// labelOff takes `needs decision` off `number` when it is on.
func labelOff(ctx *hctx.Context, number int, names map[string]bool) {
	if names[config.LabelNeedsDecision] {
		ctx.GH.RemoveLabel(number, config.LabelNeedsDecision)
	}
}

// syncDecision is SyncDecisions for one item.
func syncDecision(ctx *hctx.Context, number int, record, decision *pyjson.Object) string {
	thread := ctx.GH.GetIssue(number)
	names := queue.LabelNames(thread)
	if !eqStr(thread.Get("state"), "open") {
		labelOff(ctx, number, names)
		unassign(ctx, number, decision)
		dropDecision(ctx, number)
		return fmt.Sprintf("#%d closed, so it waits for no decision (`%s` off)", number, config.LabelNeedsDecision)
	}
	if names[config.LabelHumanRequired] || decision.Truthy("human_required") {
		// Handed to a person (#3009): a hand-over is no question. One #2986 made is archived as a
		// hand-over is now, when it is still people's, so nothing of the bot's takes it up again.
		labelOff(ctx, number, names)
		ctx.Store.Update(func(s *pyjson.Object) any {
			entry := hstate.Item(s, number)
			entry.Pop(queue.DecisionKey)
			if names[config.LabelHumanRequired] && decision.Truthy("human_required") &&
				queue.ArchivedOf(entry) == nil {
				linked := py.MustInt(py.Or(entry.Get("pr"), py.Or(entry.Get("issue"), 0)))
				queue.Archive(s, number, linked, pyjson.New("at", py.OrStr(decision.Get("asked_at"), clock.ISO(ctx.Now())),
					"why", py.OrStr(decision.Get("question"), ""), "older", true))
			}
			return nil
		}, fmt.Sprintf("decision #%d handed over", number))
		return fmt.Sprintf("#%d is handed to a person, so it waits for no decision (`%s` off)", number,
			config.LabelNeedsDecision)
	}
	if queue.Resumes(decision) != "" {
		return "" // its answer's or its default's run is queued or going: its delivery forgets it
	}
	if !names[config.LabelNeedsDecision] || !names[config.LabelBlocked] {
		// A person queued it with a command, or took `bot:blocked` or the label off: the label and
		// the people it assigned go with the stop.
		labelOff(ctx, number, names)
		unassign(ctx, number, decision)
		dropDecision(ctx, number)
		return fmt.Sprintf("#%d waits for no decision any more", number)
	}
	if queue.IsHuman(names) {
		return "" // `human` or `tracking`: people's, and it waits for them
	}
	if found := reply(ctx, number, record, decision, names); found != nil {
		return answer(ctx, number, thread, names, decision, found)
	}
	ends := clock.ParseISOValue(decision.Get("ends"))
	if !queue.HasDefault(decision) || ends.IsZero() || ctx.Now().Before(ends) {
		return ""
	}
	return goAhead(ctx, number, thread, names, decision)
}

// reply is the first trusted person's answer to `number`'s question (#3009): their first comment
// (no bot's) after it was asked, on the item or on the pull request its record names, that is no
// command (`commands.Parse`, a mention of the bot, a vote on a suggestion: each is answered as a
// command) and no session's handoff (`config.IsSessionHandoff`). nil for none.
func reply(ctx *hctx.Context, number int, record, decision *pyjson.Object, names map[string]bool) *pyjson.Object {
	since := clock.ParseISOValue(decision.Get("asked_at"))
	bots := map[string]bool{strings.ToLower(ctx.Cfg.BotLogin): true, "github-actions[bot]": true}
	for _, other := range config.Others {
		bots[strings.ToLower(other.Login)] = true
	}
	read := []int{number}
	if pr := py.MustInt(py.Or(record.Get("pr"), 0)); pr != 0 && pr != number {
		read = append(read, pr)
	}
	var first *pyjson.Object
	var firstAt time.Time
	for _, n := range read {
		var comments []*pyjson.Object
		if _, failed := herr.Catch[*herr.GitHubError](func() { comments = ctx.GH.ListComments(n, 300) }); failed {
			continue
		}
		for _, comment := range comments {
			user := comment.Obj("user")
			login := py.OrStr(user.Get("login"), "")
			at := clock.ParseISOValue(comment.Get("created_at"))
			body := py.OrStr(comment.Get("body"), "")
			if bots[strings.ToLower(login)] || at.IsZero() || !at.After(since) || strings.Contains(body, config.Marker) ||
				config.IsSessionHandoff(body) || len(commands.Parse(body, ctx.Cfg.BotLogin)) > 0 ||
				commands.NamesTheBot(body, ctx.Cfg.BotLogin) {
				continue
			}
			if voted, _ := commands.Vote(body, ctx.Cfg.BotLogin); voted != "" && n == number &&
				names[config.LabelSuggestion] {
				continue // a vote on one of my suggestions (`events.AnswerVote`)
			}
			var id *int
			if uid, ok := pyjson.AsInt(user.Get("id")); ok {
				id = &uid
			}
			if ctx.Trust == nil || ctx.Trust.Level(login, id, py.OrStr(comment.Get("author_association"), "")) < 1 {
				continue
			}
			if first == nil || at.Before(firstAt) {
				first, firstAt = comment, at
			}
			break
		}
	}
	return first
}

// answer takes `number` up again on a trusted person's reply to its question (#3009): owned by the
// subscription that asked, queued for it on the model that asked, in a fresh session (a build of an
// issue, a revision of a pull request, `source: decision`), the reply kept as the decision
// (`answered_at`, which its claim reads); `bot:blocked`, `needs decision` and the people it assigned
// off, and a comment that says so.
func answer(ctx *hctx.Context, number int, thread *pyjson.Object, names map[string]bool, decision,
	found *pyjson.Object) string {
	isPR := thread.Has("pull_request")
	kind, label, verb := "build", config.LabelBuild, "builds"
	if isPR {
		kind, label, verb = "revise", config.LabelRevise, "revises"
	}
	who := py.OrStr(found.Obj("user").Get("login"), "")
	provider, model := decision.Str("provider"), decision.Str("model")
	now := ctx.Now()
	ctx.Store.Update(func(s *pyjson.Object) any {
		entry := hstate.Item(s, number)
		kept := queue.DecisionOf(entry)
		if kept == nil {
			return nil
		}
		kept.Update(pyjson.New("answered_at", clock.ISO(now), "answered_by", who,
			"answer", py.SliceTo(strings.TrimSpace(py.OrStr(found.Get("body"), "")), queue.QuestionChars),
			"answer_url", py.OrStr(found.Get("html_url"), "")))
		kept.Pop("ends")
		kept.Pop("replied")
		fields := pyjson.New("kind", kind, "queued_at", clock.ISO(now), "failures", 0, "stop_requested", false,
			"forced", false)
		if isPR {
			fields.Set("source", "decision")
		}
		entry.Update(fields)
		if provider != "" {
			Own(s, number, provider, "a person answered its question, so the lane that asked carries the answer out", now)
		}
		return nil
	}, fmt.Sprintf("decision #%d answered", number))
	queue.SetStateLabel(ctx, number, names, label)
	unassign(ctx, number, decision)
	lane := "the next lane that can takes it up in a fresh session"
	if provider != "" && model != "" {
		lane = fmt.Sprintf("`%s` takes it up again on `%s`, the model that asked, in a fresh session (held for it "+
			"while only its 5-hour window keeps it, else handed off)", provider, model)
	}
	ctx.GH.CreateComment(number, fmt.Sprintf("✅ **Answered** (`%s` off): I take @%s's reply above as the "+
		"decision: %s, and %s it that way. `%s stop` stops it.", config.LabelNeedsDecision, who, lane, verb,
		config.Slash))
	return fmt.Sprintf("#%d: @%s answered its question, so it is queued again with the answer", number, who)
}

// goAhead takes `number` up again on its default: owned by the subscription that asked, queued for
// it (a build of an issue, a revision of a pull request, `source: default`), `bot:blocked`, `needs
// decision` and the people it assigned off, each other option filed as an issue, and a comment that
// says so.
func goAhead(ctx *hctx.Context, number int, thread *pyjson.Object, names map[string]bool, decision *pyjson.Object) string {
	isPR := thread.Has("pull_request")
	kind, label := "build", config.LabelBuild
	if isPR {
		kind, label = "revise", config.LabelRevise
	}
	provider, model := decision.Str("provider"), decision.Str("model")
	chosen := py.MustInt(decision.Get("default"))
	filed, failed := fileOptions(ctx, number, thread, names, decision)
	now := ctx.Now()
	ctx.Store.Update(func(s *pyjson.Object) any {
		entry := hstate.Item(s, number)
		kept := queue.DecisionOf(entry)
		if kept == nil {
			return nil
		}
		kept.Set("went_ahead", clock.ISO(now))
		kept.Set("filed", pyjson.ListOf(filed))
		fields := pyjson.New("kind", kind, "queued_at", clock.ISO(now), "failures", 0, "stop_requested", false,
			"forced", false)
		if isPR {
			fields.Set("source", "default")
		}
		entry.Update(fields)
		Own(s, number, provider, "nobody answered its question in time, so the lane that asked goes ahead on its default", now)
		return nil
	}, fmt.Sprintf("default #%d", number))
	queue.SetStateLabel(ctx, number, names, label)
	unassign(ctx, number, decision)
	var others string
	switch {
	case len(filed) > 0:
		refs := make([]string, len(filed))
		for i, n := range filed {
			refs[i] = "#" + strconv.Itoa(n)
		}
		others = fmt.Sprintf(" Each other option is filed for a person to approve (%s); nothing queues them.",
			strings.Join(refs, ", "))
	case len(pyjson.List(decision.Get("options"))) > 1:
		others = " I could not file the other options as issues (" + strings.Join(failed, "; ") + ")."
	}
	ctx.GH.CreateComment(number, fmt.Sprintf("⏭️ **Going ahead on its default** (`%s` off): no trusted person "+
		"answered the question above within %d hours (`decision_wait_hours`), so `%s` takes it up again on `%s`, "+
		"the model that asked, in a fresh session (held for it while only its 5-hour window keeps it, else "+
		"handed off), and goes ahead on option %d, its default: %s%s `%s stop` stops it.",
		config.LabelNeedsDecision, py.MustInt(py.Or(decision.Get("hours"), ctx.Cfg.DecisionWaitHours)), provider,
		model, chosen, strings.TrimSuffix(strings.Join(strings.Fields(queue.DefaultOption(decision)), " "), ".")+".",
		others, config.Slash))
	return fmt.Sprintf("#%d: nobody answered its question in time, so `%s` goes ahead on its default", number, provider)
}

// fileOptions opens an issue for each option but the default, for a person to approve: not queued,
// with the thread's type and difficulty labels. Returns their numbers, and what GitHub refused.
func fileOptions(ctx *hctx.Context, number int, thread *pyjson.Object, names map[string]bool,
	decision *pyjson.Object) ([]int, []string) {
	var labels []string
	for _, name := range config.TypeLabels {
		if names[name] {
			labels = append(labels, name)
		}
	}
	sorted := make([]string, 0, len(names))
	for name := range names {
		sorted = append(sorted, name)
	}
	sort.Strings(sorted)
	for _, name := range sorted {
		if _, ok := config.LabelDifficulty(py.Lower(name)); ok {
			labels = append(labels, name)
		}
	}
	chosen := py.MustInt(decision.Get("default"))
	title := py.SliceTo(strings.Join(strings.Fields(py.OrStr(thread.Get("title"), "")), " "), OptionTitleChars)
	question := py.OrStr(decision.Get("question"), "")
	if problem := strings.TrimSpace(py.OrStr(decision.Get("problem"), "")); problem != "" {
		question = problem + "\n\n" + question
	}
	filed, failed := []int{}, []string{}
	for i, option := range queue.OptionsOf(decision) {
		if i+1 == chosen {
			continue
		}
		text := option.Change
		if option.Tradeoff != "" {
			text += "\n\nIts trade-off: " + option.Tradeoff
		}
		body := strings.Join([]string{
			fmt.Sprintf("#%d stopped to ask a question that no trusted person answered within %d hours "+
				"(`decision_wait_hours`), so `%s` went ahead on its default, option %d: %s", number,
				py.MustInt(py.Or(decision.Get("hours"), ctx.Cfg.DecisionWaitHours)), decision.Str("provider"), chosen,
				strings.Join(strings.Fields(queue.DefaultOption(decision)), " ")),
			"",
			fmt.Sprintf("This is option %d, which it did not take, filed for a person to decide on.", i+1),
			"",
			"## The question",
			"",
			question,
			"",
			"## This option",
			"",
			text,
			"",
			"---",
			fmt.Sprintf("Nothing queues this issue: a person approves it by queuing it (`%s`, or `%s build`), "+
				"or closes it. Filed by @%s from #%d.", config.LabelBuild, config.Slash, ctx.Cfg.BotLogin, number),
		}, "\n")
		var issue *pyjson.Object
		if exc, ok := herr.Catch[*herr.GitHubError](func() {
			issue = ctx.GH.CreateIssue(fmt.Sprintf("%s (option %d of #%d)", title, i+1, number), body, labels)
		}); ok {
			failed = append(failed, fmt.Sprintf("option %d: %s", i+1, py.SliceTo(exc.Error(), 200)))
			continue
		}
		if n := py.MustInt(py.Or(issue.Get("number"), 0)); n != 0 {
			filed = append(filed, n)
		}
	}
	return filed, failed
}

// optionsText is a stop's question (after its problem, when it has one) and its numbered options
// with their trade-offs, as a run that takes it up again reads them.
func optionsText(decision *pyjson.Object) string {
	text := py.OrStr(decision.Get("question"), "")
	if problem := strings.TrimSpace(py.OrStr(decision.Get("problem"), "")); problem != "" {
		text = problem + "\n\n" + text
	}
	var options []string
	for i, option := range queue.OptionsOf(decision) {
		line := fmt.Sprintf("%d. %s", i+1, option.Change)
		if option.Tradeoff != "" {
			line += " Trade-off: " + option.Tradeoff
		}
		options = append(options, line)
	}
	return text + "\n\n" + strings.Join(options, "\n")
}

// DefaultNote is what a run that goes ahead on its question's default reads (#2986): the question,
// the option to build and what it gives up, and that the others are a person's.
func DefaultNote(decision *pyjson.Object) string {
	gives := ""
	if chosen := py.MustInt(py.Or(decision.Get("default"), 0)); chosen >= 1 && chosen <= len(queue.OptionsOf(decision)) {
		if text := queue.OptionsOf(decision)[chosen-1].Tradeoff; text != "" {
			gives = " It gives up: " + text
		}
	}
	return fmt.Sprintf("An earlier run stopped to ask the question below, and no trusted person answered it in "+
		"time, so this run goes ahead on its default: option %d, the one it suggested, which stays inside the "+
		"issue's own stated limits.%s Build that option and do not stop to ask this question again. Leave the "+
		"other options out: each is filed as an issue of its own for a person to decide on. Say in your report "+
		"that it was built on its default.\n\n%s", py.MustInt(decision.Get("default")), gives,
		prompts.Data(optionsText(decision), "The question asked, and its options"))
}

// AnswerNote is what a run that carries a person's answer out reads (#3009): the question and its
// options, and the answer, which is the decision whatever it says (an option's number or words, or
// something else).
func AnswerNote(decision *pyjson.Object) string {
	who := py.OrStr(decision.Get("answered_by"), "a trusted person")
	answer := py.OrStr(decision.Get("answer"), "")
	named := ""
	options := queue.OptionsOf(decision)
	if n, err := py.Int(strings.Trim(strings.TrimPrefix(py.Lower(py.Strip(answer)), "option"), " .!#")); err == nil &&
		n >= 1 && n <= len(options) {
		named = fmt.Sprintf(" It names option %d: %s", n, strings.TrimSuffix(options[n-1].Change, ".")+".")
	}
	return fmt.Sprintf("An earlier run stopped to ask the question below, and @%s answered it. Their answer is "+
		"the decision, whether it names an option by its number or its words or says something else: carry "+
		"it out, and do not stop to ask this question again.%s Say in your report how you carried it out.\n\n"+
		"%s\n\n%s", who, named, prompts.Data(optionsText(decision), "The question asked, and its options"),
		prompts.Data(answer, "@"+who+"'s answer, the decision"))
}

// OlderKey is the state file's field that says `OlderQuestions` has labelled the older stops: it
// runs until one pass of it finishes.
var OlderKey = "older_questions_labelled"

// QuestionPhrases are what the bot's last comment on a thread says when it stopped there on a
// question (`deliver`'s question comments).
var QuestionPhrases = []string{"need a decision before", "needs a decision before"}

// OlderQuestions labels, once, the items this bot stopped on a question before #2986: still
// `bot:blocked`, the record keeping their `question` and no stop for a decision, and the bot's last
// comment on them its question. `needs decision` goes on, and the record keeps the stop, asked when
// that comment was made, with no default and no deadline. A hand-over (`human required`) is no
// question (#3009): it is left alone. Returns what it did.
func OlderQuestions(ctx *hctx.Context) []string {
	state := ctx.Store.Load()
	if state.Truthy(OlderKey) {
		return []string{}
	}
	notes := []string{}
	seen := map[int]bool{}
	for _, label := range []string{config.LabelBlocked} {
		for _, thread := range ctx.GH.ListIssues(label, "open", 300, "") {
			number := py.MustInt(py.Or(thread.Get("number"), 0))
			record := recordOf(state, strconv.Itoa(number))
			question := strings.TrimSpace(py.OrStr(record.Get("question"), ""))
			if number == 0 || seen[number] || question == "" || queue.DecisionOf(record) != nil {
				continue
			}
			seen[number] = true
			var last *pyjson.Object
			for _, comment := range ctx.GH.ListComments(number, 300) {
				if strings.EqualFold(py.OrStr(comment.Obj("user").Get("login"), ""), ctx.Cfg.BotLogin) {
					last = comment
				}
			}
			body := py.Lower(py.OrStr(last.Get("body"), ""))
			asked := false
			for _, phrase := range QuestionPhrases {
				asked = asked || strings.Contains(body, phrase)
			}
			if !asked {
				continue // blocked for another reason since (CI, failures): its question was answered
			}
			at := clock.ParseISOValue(last.Get("created_at"))
			if at.IsZero() {
				at = ctx.Now()
			}
			if !queue.LabelNames(thread)[config.LabelNeedsDecision] {
				ctx.GH.AddLabels(number, []string{config.LabelNeedsDecision})
			}
			ctx.Store.Update(func(s *pyjson.Object) any {
				hstate.Item(s, number).Set(queue.DecisionKey, pyjson.New("asked_at", clock.ISO(at),
					"older", true, "question", py.SliceTo(question, queue.QuestionChars)))
				return nil
			}, fmt.Sprintf("decision #%d", number))
			notes = append(notes, fmt.Sprintf("#%d stopped on a question before #2986, so `%s`, with no deadline",
				number, config.LabelNeedsDecision))
		}
	}
	ctx.Store.Update(func(s *pyjson.Object) any { s.Set(OlderKey, clock.ISO(ctx.Now())); return nil },
		"older questions labelled")
	return notes
}

// HandedOver takes `needs decision` off every open item labelled `human required` (#3009): a
// hand-over is the bot deciding it cannot do the item itself, never a question, and #2986 put both
// labels on some (#2980, Squishy's, among them). Returns what it did.
func HandedOver(ctx *hctx.Context) []string {
	notes := []string{}
	for _, thread := range ctx.GH.ListIssues(config.LabelHumanRequired, "open", 300, "") {
		number := py.MustInt(py.Or(thread.Get("number"), 0))
		if number == 0 || !queue.LabelNames(thread)[config.LabelNeedsDecision] {
			continue
		}
		if _, failed := herr.Catch[*herr.GitHubError](func() { ctx.GH.RemoveLabel(number, config.LabelNeedsDecision) }); failed {
			continue
		}
		notes = append(notes, fmt.Sprintf("#%d is handed to a person, so it carries no `%s`", number,
			config.LabelNeedsDecision))
	}
	return notes
}
