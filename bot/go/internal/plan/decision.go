package plan

// A question goes ahead on its default (#2986, sections 2 and 3; the owner's decision of 2026-11-04;
// `queue/decision.go` has the record). `SyncDecisions`, in every plan's housekeeping and in the sweep,
// keeps the stops for a decision:
//
//   - one whose thread closed loses `needs decision` and its record;
//   - one that waits no more (a trusted person answered, and the item was queued again, which takes the
//     label off; or a person took the label or `bot:blocked` off) loses its record;
//   - one with a default past its end (`decision_wait_hours` after it was asked) goes ahead
//     (`goAhead`): the subscription that asked owns it, and takes it up again on the model that asked,
//     in a fresh session, by the ownership and hold rules (held for it while only its 5-hour window
//     keeps it, else handed off), to build its default; each other option is filed as an issue of its
//     own for a person to approve, and nothing queues those;
//   - but one a trusted person commented on since it was asked never goes ahead: the answer wins, and
//     the person queues it again with it (`/harness build`, or a mention), as ever.
//
// A hand-over (`human required`) and a question with no default never go ahead. `OlderQuestions`, in
// the sweep, labels the items that stopped on a question before #2986, with no deadline.

import (
	"fmt"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
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
	notes := []string{}
	state := ctx.Store.Load()
	for _, number := range queue.Decisions(state) {
		decision := queue.DecisionOf(recordOf(state, strconv.Itoa(number)))
		var note string
		if exc := herr.Try(func() { note = syncDecision(ctx, number, decision) }); exc != nil {
			note = fmt.Sprintf("#%d: could not see whether its question goes ahead (%s)", number,
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

// syncDecision is SyncDecisions for one item.
func syncDecision(ctx *hctx.Context, number int, decision *pyjson.Object) string {
	thread := ctx.GH.GetIssue(number)
	names := queue.LabelNames(thread)
	if !eqStr(thread.Get("state"), "open") {
		if names[config.LabelNeedsDecision] {
			ctx.GH.RemoveLabel(number, config.LabelNeedsDecision)
		}
		dropDecision(ctx, number)
		return fmt.Sprintf("#%d closed, so it waits for no decision (`%s` off)", number, config.LabelNeedsDecision)
	}
	if queue.WentAhead(decision) != "" {
		return "" // its default's run is queued or going: its delivery forgets it
	}
	if !names[config.LabelNeedsDecision] || !(names[config.LabelBlocked] || queue.IsHuman(names)) {
		dropDecision(ctx, number)
		return fmt.Sprintf("#%d waits for no decision any more", number)
	}
	ends := clock.ParseISOValue(decision.Get("ends"))
	if !queue.HasDefault(decision) || queue.IsHuman(names) || ends.IsZero() || ctx.Now().Before(ends) {
		return ""
	}
	if who := trustedReply(ctx, number, clock.ParseISOValue(decision.Get("asked_at"))); who != "" {
		ctx.Store.Update(func(s *pyjson.Object) any {
			if kept := queue.DecisionOf(hstate.Item(s, number)); kept != nil {
				kept.Pop("ends")
				kept.Set("replied", who)
			}
			return nil
		}, fmt.Sprintf("decision #%d answered", number))
		verb := "build"
		if thread.Has("pull_request") {
			verb = "revise <your answer>"
		}
		ctx.GH.CreateComment(number, fmt.Sprintf("@%s commented after the question, so I do not go ahead on its "+
			"default: the answer wins. `%s %s` (or `@%s <your answer>`) queues it again with it; until then it "+
			"keeps `%s`.", who, config.Slash, verb, ctx.Cfg.BotLogin, config.LabelNeedsDecision))
		return fmt.Sprintf("#%d: @%s commented after its question, so it does not go ahead on its default", number, who)
	}
	return goAhead(ctx, number, thread, names, decision)
}

// trustedReply is the login of the first trusted person (no bot) who commented on `number` after
// `since`, or "".
func trustedReply(ctx *hctx.Context, number int, since time.Time) string {
	bots := map[string]bool{strings.ToLower(ctx.Cfg.BotLogin): true, "github-actions[bot]": true}
	for _, other := range config.Others {
		bots[strings.ToLower(other.Login)] = true
	}
	for _, comment := range ctx.GH.ListComments(number, 300) {
		user := comment.Obj("user")
		login := py.OrStr(user.Get("login"), "")
		at := clock.ParseISOValue(comment.Get("created_at"))
		if bots[strings.ToLower(login)] || at.IsZero() || !at.After(since) ||
			strings.Contains(py.OrStr(comment.Get("body"), ""), config.Marker) {
			continue
		}
		var id *int
		if n, ok := pyjson.AsInt(user.Get("id")); ok {
			id = &n
		}
		if ctx.Trust != nil && ctx.Trust.Level(login, id, py.OrStr(comment.Get("author_association"), "")) >= 1 {
			return login
		}
	}
	return ""
}

// goAhead takes `number` up again on its default: owned by the subscription that asked, queued for
// it (a build of an issue, a revision of a pull request, `source: default`), `bot:blocked` and `needs
// decision` off, each other option filed as an issue, and a comment that says so.
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
	filed, failed := []int{}, []string{}
	for i, raw := range pyjson.List(decision.Get("options")) {
		if i+1 == chosen {
			continue
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
			py.Str(raw),
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

// DefaultNote is what a run that goes ahead on its question's default reads (#2986): the question,
// the option to build and what it gives up, and that the others are a person's.
func DefaultNote(decision *pyjson.Object) string {
	var options []string
	for i, raw := range pyjson.List(decision.Get("options")) {
		options = append(options, fmt.Sprintf("%d. %s", i+1, py.Str(raw)))
	}
	gives := ""
	if text := py.OrStr(decision.Get("gives_up"), ""); text != "" {
		gives = " It gives up: " + text
	}
	return fmt.Sprintf("An earlier run stopped to ask the question below, and no trusted person answered it in "+
		"time, so this run goes ahead on its default: option %d, the one that stays inside the issue's own "+
		"stated limits.%s Build that option and do not stop to ask this question again. Leave the other "+
		"options out: each is filed as an issue of its own for a person to decide on. Say in your report "+
		"that it was built on its default.\n\n%s", py.MustInt(decision.Get("default")), gives,
		prompts.Data(py.OrStr(decision.Get("question"), "")+"\n\n"+strings.Join(options, "\n"),
			"The question asked, and its options"))
}

// OlderKey is the state file's field that says `OlderQuestions` has labelled the older stops: it
// runs until one pass of it finishes.
var OlderKey = "older_questions_labelled"

// QuestionPhrases are what the bot's last comment on a thread says when it stopped there on a
// question or handed it over (`deliver`'s question comments, `human.Ask`).
var QuestionPhrases = []string{"need a decision before", "needs a decision before", "this needs a person"}

// OlderQuestions labels, once, the items this bot stopped on a question or handed over before #2986:
// still `bot:blocked` (or `human required`), the record keeping their `question` and no stop for a
// decision, and the bot's last comment on them its question. `needs decision` goes on, and the record
// keeps the stop, asked when that comment was made, with no default and no deadline. Returns what it
// did.
func OlderQuestions(ctx *hctx.Context) []string {
	state := ctx.Store.Load()
	if state.Truthy(OlderKey) {
		return []string{}
	}
	notes := []string{}
	seen := map[int]bool{}
	for _, label := range []string{config.LabelBlocked, config.LabelHumanRequired} {
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
			human := label == config.LabelHumanRequired
			ctx.Store.Update(func(s *pyjson.Object) any {
				hstate.Item(s, number).Set(queue.DecisionKey, pyjson.New("asked_at", clock.ISO(at),
					"older", true, "human_required", human, "question", py.SliceTo(question, queue.QuestionChars)))
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
