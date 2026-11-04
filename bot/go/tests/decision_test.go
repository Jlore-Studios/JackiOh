// A question a person answers and the bot carries out (#2986, sections 2 and 3; #3009, section 1;
// the owner's decisions of 2026-11-04): its comment explains the problem, numbers its options with
// their trade-offs and names its suggestion; `needs decision` assigns `decision_assignees`; a
// trusted reply revises it on the lane that asked, and with none it goes ahead on its suggestion
// after `decision_wait_hours`. A hand-over is no question (`human_test.go` has its archive).
package tests

import (
	"fmt"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
)

// decisionAsk is a builder's report that stops on a question (#3009): its problem, two options with
// their trade-offs, and the first suggested.
const decisionAsk = `<!-- bot: {"status": "blocked", "problem": "The issue asks for a win screen, and a ` +
	`win could also earn Jlore.\n\nThe issue never says, and earning changes the economy, so I would ` +
	`rather not guess.", "question": "Should a win earn Jlore?", "options": [{"change": "(A) No ` +
	`earning: already on the branch", "tradeoff": "Players earn nothing for a win yet"}, {"change": ` +
	`"(B) Earn 10 Jlore a win", "tradeoff": "It changes the economy the issue left alone"}], ` +
	`"suggestion": 1, "why": "it stays inside what the issue asks"} -->` + "\nStopped."

// decisionOlderAsk is the same stop as #2986's builders wrote it: each option its text, a default and
// what it gives up.
const decisionOlderAsk = `<!-- bot: {"status": "blocked", "question": "Should a win earn Jlore?", ` +
	`"options": ["(A) No earning: already on the branch", "(B) Earn 10 Jlore a win"], "default": 1, ` +
	`"gives_up": "Players earn nothing for a win yet"} -->` + "\nStopped."

// decisionAsked builds issue 12 until its builder stops on `report`, and returns the harness, the plan
// and the bot's last comment on it.
func decisionAsked(t *testing.T, report string) (*flowHarness, *pyjson.Object, string) {
	t.Helper()
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Title: "Patch: Earning", Labels: flowLabels(config.LabelBuild, "patch",
		"difficulty:medium")})
	planned, result := h.night(workFake(map[string]any{"build": workBuilder(map[string]any{}, report),
		"review": workReviewer(workApprove)}))
	assertEqual(t, result.Get("status"), "blocked")
	return h, planned, lastOf(t, h.gh.BotComments(12))
}

// decisionOf is issue `number`'s stop for a decision, as the state file keeps it.
func decisionOf(h *flowHarness, number int) *pyjson.Object {
	return queue.DecisionOf(state.Item(h.ctx.Store.Load(), number))
}

// decisionLater moves the harness's clock `hours` on.
func decisionLater(h *flowHarness, hours int) {
	testsupport.ClockOf(h.ctx).At = testsupport.ClockOf(h.ctx).At.Add(time.Duration(hours) * time.Hour)
}

// decisionAssignees is who is assigned to `number`.
func decisionAssignees(h *flowHarness, number int) map[string]bool {
	out := map[string]bool{}
	for _, a := range pyjson.List(h.gh.Threads.Item(number).Get("assignees")) {
		out[pyjson.Obj(a).Str("login")] = true
	}
	return out
}

// decisionReply is a trusted person's comment on `number`, an hour after the question.
func decisionReply(h *flowHarness, number int, body string) *pyjson.Object {
	return h.gh.AddComment(number, body, fakes.Comment{CreatedAt: clock.ISO(testsupport.Night.Add(time.Hour))})
}

func TestDecision_TheReportExplainsTheProblemAndSuggestsAnOption(t *testing.T) {
	report := verdicts.BuildReport_(decisionAsk)
	assertEqual(t, []any{report.Status, report.Default, report.Why}, []any{"blocked", 1, "it stays inside what the issue asks"})
	assertIn(t, "The issue never says, and earning changes the economy", report.Problem)
	assertEqual(t, report.Options, []verdicts.Option{
		{Change: "(A) No earning: already on the branch", Tradeoff: "Players earn nothing for a win yet"},
		{Change: "(B) Earn 10 Jlore a win", Tradeoff: "It changes the economy the issue left alone"}})
	// #2986's report still reads: each option its text, its default's trade-off its `gives_up`.
	older := verdicts.BuildReport_(decisionOlderAsk)
	assertEqual(t, []any{older.Problem, older.Default}, []any{"", 1})
	assertEqual(t, older.Options, []verdicts.Option{
		{Change: "(A) No earning: already on the branch", Tradeoff: "Players earn nothing for a win yet"},
		{Change: "(B) Earn 10 Jlore a win"}})
	// A suggestion that names no option is none; a hand-over is no question.
	none := verdicts.BuildReport_(`<!-- bot: {"status": "blocked", "question": "Q", "options": ["a"], "suggestion": 3} -->`)
	assertEqual(t, []any{len(none.Options), none.Default}, []any{1, 0})
	human := verdicts.BuildReport_(`<!-- bot: {"status": "blocked", "human_required": true, "question": "Q", ` +
		`"problem": "P", "options": ["a", "b"], "suggestion": 1} -->`)
	assertEqual(t, []any{human.HumanRequired, human.Problem, human.Options, human.Default},
		[]any{true, "", []verdicts.Option{}, 0})
}

func TestDecision_TheQuestionExplainsTheProblemAndSaysHowToAnswer(t *testing.T) {
	h, planned, said := decisionAsked(t, decisionAsk)
	labels := h.gh.LabelNames(12)
	assertIn(t, config.LabelBlocked, labels)
	assertIn(t, config.LabelNeedsDecision, labels) // beside `bot:blocked`, which keeps its meaning
	assertTrue(t, strings.HasPrefix(said, "I need a decision before I can build this."), said)
	// 1. the problem, 2. the numbered options with their trade-offs, 3. the suggestion, 4. how to answer.
	problem := strIndex(t, said, "The issue asks for a win screen, and a win could also earn Jlore.\n\n"+
		"The issue never says, and earning changes the economy")
	assertIn(t, "**The question:** Should a win earn Jlore?", said)
	options := strIndex(t, said, "**My recommendations:**\n\n"+
		"1. (A) No earning: already on the branch *Trade-off:* Players earn nothing for a win yet.\n"+
		"2. (B) Earn 10 Jlore a win *Trade-off:* It changes the economy the issue left alone.\n")
	suggestion := strIndex(t, said, "**My suggestion:** option 1, because it stays inside what the issue asks.")
	answer := strIndex(t, said, "Reply with a number, or tell me something else in your own words, and I'll "+
		"revise it that way. If nobody answers by Wed 30 Sep 22:00 CDT, I go ahead with option 1.")
	assertTrue(t, problem < options && options < suggestion && suggestion < answer, problem, options, suggestion, answer)
	provider := planned.Str("provider")
	assertIn(t, fmt.Sprintf("(After 24 hours, `decision_wait_hours`, `%s` takes it up again on `", provider), said)
	assertNotIn(t, "(the default)", said)
	decision := decisionOf(h, 12)
	assertEqual(t, []any{decision.Get("provider"), decision.Get("default"), decision.Get("kind"), decision.Get("hours")},
		[]any{provider, 1, "build", 24})
	assertEqual(t, decision.Get("ends"), clock.ISO(testsupport.Night.Add(24*time.Hour)))
	assertIn(t, "earning changes the economy", decision.Str("problem"))
	assertTrue(t, decision.Truthy("model"))
	// Setting the label assigns `decision_assignees`, and the record keeps whom.
	assertEqual(t, h.cfg.DecisionAssignees, []string{"jgoetzmann", "MaxGoetzmann"})
	assertEqual(t, decisionAssignees(h, 12), setOf("jgoetzmann", "MaxGoetzmann"))
	assertEqual(t, queue.Assigned(decision), []string{"jgoetzmann", "MaxGoetzmann"})
	// Being assigned does not make it a person's: it is not `human`, and no other bot's.
	thread := h.gh.GetIssue(12)
	assertFalse(t, queue.IsHuman(queue.LabelNames(thread)))
	assertTrue(t, queue.Owner(thread) == nil)
	// Before its end, and with no reply, nothing goes ahead.
	decisionLater(h, 23)
	assertEqual(t, plan.SyncDecisions(h.ctx), []string{})
	assertIn(t, config.LabelNeedsDecision, h.gh.LabelNames(12))
}

func TestDecision_AnOlderReportStillReads(t *testing.T) {
	h, _, said := decisionAsked(t, decisionOlderAsk)
	assertIn(t, "Should a win earn Jlore?\n\n**My recommendations:**", said) // its question is its problem
	assertIn(t, "1. (A) No earning: already on the branch *Trade-off:* Players earn nothing for a win yet.\n"+
		"2. (B) Earn 10 Jlore a win\n", said)
	assertIn(t, "**My suggestion:** option 1, because it stays inside the issue's own stated limits.", said)
	assertEqual(t, queue.DefaultOption(decisionOf(h, 12)), "(A) No earning: already on the branch")
	// And a record #2986 wrote, with its options as text and its `gives_up`.
	older := pyjson.New("asked_at", "2026-09-29T06:00:00Z", "options", []any{"A", "B"}, "default", 2,
		"gives_up", "Less")
	assertEqual(t, queue.OptionsOf(older), []verdicts.Option{{Change: "A"}, {Change: "B", Tradeoff: "Less"}})
}

func TestDecision_ItUnassignsOnlyThoseItAssigned(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Title: "Patch: Earning", Labels: flowLabels(config.LabelBuild)})
	h.gh.AddAssignees(12, []string{"MaxGoetzmann"}) // assigned before the question
	h.night(workFake(map[string]any{"build": workBuilder(map[string]any{}, decisionAsk),
		"review": workReviewer(workApprove)}))
	assertEqual(t, queue.Assigned(decisionOf(h, 12)), []string{"jgoetzmann"})
	assertEqual(t, decisionAssignees(h, 12), setOf("MaxGoetzmann", "jgoetzmann"))
	decisionLater(h, 25)
	plan.SyncDecisions(h.ctx) // it goes ahead on its suggestion
	assertEqual(t, decisionAssignees(h, 12), setOf("MaxGoetzmann"))
}

func TestDecision_NoAnswerInTimeBuildsTheDefaultOnTheLaneThatAsked(t *testing.T) {
	h, planned, _ := decisionAsked(t, decisionAsk)
	provider, model := planned.Str("provider"), decisionOf(h, 12).Str("model")
	decisionLater(h, 25)
	notes := plan.SyncDecisions(h.ctx)
	assertEqual(t, notes, []string{fmt.Sprintf("#12: nobody answered its question in time, so `%s` goes ahead on its default", provider)})
	labels := h.gh.LabelNames(12)
	assertIn(t, config.LabelBuild, labels)
	assertNotIn(t, config.LabelBlocked, labels)
	assertNotIn(t, config.LabelNeedsDecision, labels)
	assertEqual(t, decisionAssignees(h, 12), setOf()) // the people it assigned come off
	assertEqual(t, queue.OwnerOf(h.ctx.Store.Load(), 12), provider)
	said := lastOf(t, h.gh.BotComments(12))
	assertIn(t, "**Going ahead on its default**", said)
	assertIn(t, "goes ahead on option 1, its default: (A) No earning: already on the branch.", said)
	// The other option is an issue of its own for a person to approve, with its trade-off: not queued.
	filed := pyjson.List(decisionOf(h, 12).Get("filed"))
	assertEqual(t, len(filed), 1)
	option := h.gh.Threads.Get(filed[0].(int))
	assertEqual(t, option.Str("title"), "Patch: Earning (option 2 of #12)")
	assertIn(t, "(B) Earn 10 Jlore a win\n\nIts trade-off: It changes the economy the issue left alone", option.Str("body"))
	assertIn(t, "The issue never says, and earning changes the economy", option.Str("body"))
	assertIn(t, "Nothing queues this issue", option.Str("body"))
	assertEqual(t, h.gh.LabelNames(filed[0].(int)), setOf("patch", "difficulty:medium"))
	assertIn(t, fmt.Sprintf("#%d", filed[0].(int)), said)
	// It does not go ahead twice.
	assertEqual(t, plan.SyncDecisions(h.ctx), []string{})
	// Its build runs on the subscription and model that asked, in a fresh session, told to build the
	// default; its pull request says why.
	var prompt, ranOn string
	builder := func(request *runner.RunRequest) *runner.RunResult {
		prompt, ranOn = request.Prompt, request.Model
		return workBuilder(map[string]any{"src/game.txt": "rules v2\n"})(request)
	}
	planned, result := h.night(workFake(map[string]any{"build": runner.Handler(builder),
		"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("number"), planned.Get("provider"), ranOn}, []any{12, provider, model})
	assertTrue(t, planned.Truthy("on_default"))
	assertFalse(t, planned.Has("session"))
	assertIn(t, "no trusted person answered it in time, so this run goes ahead on its default: option 1", prompt)
	assertIn(t, "It gives up: Players earn nothing for a win yet", prompt)
	assertEqual(t, result.Get("status"), "approved")
	pull := h.gh.ListPulls("open", "bot/issue-12", 200)[0]
	assertIn(t, "### Built on its default", pull.Str("body"))
	assertIn(t, "This was built on its default because nobody answered its question by", pull.Str("body"))
	assertIn(t, "Its trade-off: Players earn nothing for a win yet.", pull.Str("body"))
	assertTrue(t, decisionOf(h, 12) == nil, decisionOf(h, 12))
}

func TestDecision_ATrustedReplyRevisesItOnTheLaneThatAsked(t *testing.T) {
	h, planned, _ := decisionAsked(t, decisionAsk)
	provider, model := planned.Str("provider"), decisionOf(h, 12).Str("model")
	// Comments that are no answer: a stranger's, a session's handoff, one before the question.
	h.gh.AddComment(12, "Drive-by.", fakes.Comment{User: pyjson.New("login", "stranger", "id", 9),
		Association: "NONE", CreatedAt: "2026-09-30T04:00:00Z"})
	h.gh.AddComment(12, "<!-- jackioh-session:handoff -->\nAsked: earning. Left: everything.",
		fakes.Comment{CreatedAt: "2026-09-30T04:30:00Z"})
	h.gh.AddComment(12, "An old note.", fakes.Comment{CreatedAt: "2026-09-29T00:00:00Z"})
	assertEqual(t, plan.SyncDecisions(h.ctx), []string{})
	// The first trusted reply, in words, is the answer, before the deadline too.
	decisionReply(h, 12, "Go with 2, but make it 5 Jlore.")
	h.gh.AddComment(12, "And thanks!", fakes.Comment{CreatedAt: "2026-09-30T06:00:00Z"})
	assertEqual(t, plan.SyncDecisions(h.ctx),
		[]string{"#12: @jgoetzmann answered its question, so it is queued again with the answer"})
	labels := h.gh.LabelNames(12)
	assertIn(t, config.LabelBuild, labels)
	assertNotIn(t, config.LabelBlocked, labels)
	assertNotIn(t, config.LabelNeedsDecision, labels)
	assertEqual(t, decisionAssignees(h, 12), setOf())
	assertEqual(t, queue.OwnerOf(h.ctx.Store.Load(), 12), provider)
	said := lastOf(t, h.gh.BotComments(12))
	assertIn(t, "✅ **Answered** (`needs decision` off): I take @jgoetzmann's reply above as the decision: "+
		fmt.Sprintf("`%s` takes it up again on `%s`, the model that asked, in a fresh session", provider, model), said)
	decision := decisionOf(h, 12)
	assertEqual(t, []any{decision.Get("answered_by"), decision.Get("answer"), decision.Has("ends")},
		[]any{"jgoetzmann", "Go with 2, but make it 5 Jlore.", false})
	// It is taken up once, and neither goes ahead on its default nor files an option.
	before := h.gh.Threads.Len()
	decisionLater(h, 25)
	assertEqual(t, plan.SyncDecisions(h.ctx), []string{})
	assertEqual(t, h.gh.Threads.Len(), before)
	// The build runs on the subscription and model that asked, in a fresh session, with the answer.
	var prompt, ranOn string
	builder := func(request *runner.RunRequest) *runner.RunResult {
		prompt, ranOn = request.Prompt, request.Model
		return workBuilder(map[string]any{"src/game.txt": "rules v2\n"})(request)
	}
	planned, result := h.night(workFake(map[string]any{"build": runner.Handler(builder),
		"review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("number"), planned.Get("provider"), ranOn}, []any{12, provider, model})
	assertTrue(t, planned.Truthy("on_answer"))
	assertFalse(t, planned.Has("on_default"))
	assertFalse(t, planned.Has("session"))
	assertEqual(t, planned.Get("previous_question"), "")
	assertIn(t, "and @jgoetzmann answered it. Their answer is the decision", prompt)
	assertIn(t, "Go with 2, but make it 5 Jlore.", prompt)
	assertIn(t, "2. (B) Earn 10 Jlore a win Trade-off: It changes the economy", prompt)
	assertEqual(t, result.Get("status"), "approved")
	assertTrue(t, decisionOf(h, 12) == nil, decisionOf(h, 12))
}

func TestDecision_AReplyOfANumberNamesItsOption(t *testing.T) {
	note := plan.AnswerNote(pyjson.New("question", "Q?", "options", []any{
		pyjson.New("change", "A", "tradeoff", "x"), pyjson.New("change", "B", "tradeoff", "y")},
		"answered_by", "jgoetzmann", "answer", "Option 2."))
	assertIn(t, "It names option 2: B.", note)
	assertNotIn(t, "It names option", plan.AnswerNote(pyjson.New("question", "Q?", "options", []any{"A"},
		"answer", "Neither: drop it.")))
}

func TestDecision_AReplyTakesItUpAtOnceThroughTheEvent(t *testing.T) {
	h, _, _ := decisionAsked(t, decisionAsk)
	comment := decisionReply(h, 12, "Earn 10 a win, as (B) says.")
	notes := events.Handle(h.ctx, "issue_comment", pyjson.New("action", "created", "sender", fakes.Operator.Copy(),
		"issue", pyjson.New("number", 12), "comment", comment))
	assertEqual(t, notes, []string{"#12: @jgoetzmann answered its question, so it is queued again with the answer"})
	assertIn(t, config.LabelBuild, h.gh.LabelNames(12))
	// A plain comment on a thread with no question is still no command.
	h.gh.AddIssue(13, fakes.Issue{})
	other := h.gh.AddComment(13, "Nice.", fakes.Comment{CreatedAt: "2026-09-30T05:00:00Z"})
	assertEqual(t, events.Handle(h.ctx, "issue_comment", pyjson.New("action", "created", "sender",
		fakes.Operator.Copy(), "issue", pyjson.New("number", 13), "comment", other)), []string{"no command"})
}

func TestDecision_AReplyOnItsPullRequestCounts(t *testing.T) {
	h, _, _ := decisionAsked(t, decisionAsk)
	h.gh.AddPull(40, "bot/issue-12", fakes.Pull{Body: "Closes #12", Labels: []string{config.LabelPR}})
	h.ctx.Store.Update(func(s *pyjson.Object) any { state.Item(s, 12).Set("pr", 40); return nil }, "pr")
	comment := decisionReply(h, 40, "Take (A).")
	notes := events.Handle(h.ctx, "issue_comment", pyjson.New("action", "created", "sender", fakes.Operator.Copy(),
		"issue", pyjson.New("number", 40, "pull_request", pyjson.New()), "comment", comment))
	assertEqual(t, notes, []string{"#12: @jgoetzmann answered its question, so it is queued again with the answer"})
	assertEqual(t, decisionOf(h, 12).Get("answer"), "Take (A).")
}

func TestDecision_ACommandIsStillACommand(t *testing.T) {
	h, _, _ := decisionAsked(t, decisionAsk)
	// A mention is a request: it queues the item, which takes the label off, as ever.
	h.gh.AddComment(12, "@jgoetzmann-bot take (B): earn 10 a win", fakes.Comment{CreatedAt: "2026-09-30T05:00:00Z"})
	flowAsk(h, 12, "@jgoetzmann-bot take (B): earn 10 a win", 901)
	labels := h.gh.LabelNames(12)
	assertIn(t, config.LabelBuild, labels)
	assertNotIn(t, config.LabelNeedsDecision, labels)
	assertIn(t, "Queued #12", lastOf(t, h.gh.BotComments(12)))
	assertEqual(t, plan.SyncDecisions(h.ctx), []string{"#12 waits for no decision any more"})
	assertEqual(t, decisionAssignees(h, 12), setOf()) // the people it assigned come off with it
	before := h.gh.Threads.Len()
	decisionLater(h, 25)
	assertEqual(t, plan.SyncDecisions(h.ctx), []string{})
	assertEqual(t, h.gh.Threads.Len(), before) // no option filed
	// The next build reads the question and the request, not the default, and not as an answer.
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	assertEqual(t, planned.Get("number"), 12)
	assertFalse(t, planned.Has("on_default"))
	assertFalse(t, planned.Has("on_answer"))
	assertEqual(t, planned.Get("previous_question"), "Should a win earn Jlore?")
	// A `/harness` line is no answer either, nor a vote on a suggestion.
	h2, _, _ := decisionAsked(t, decisionAsk)
	h2.gh.AddComment(12, "/harness status", fakes.Comment{CreatedAt: "2026-09-30T05:00:00Z"})
	h2.gh.AddLabels(12, []string{config.LabelSuggestion})
	h2.gh.AddComment(12, "approve", fakes.Comment{CreatedAt: "2026-09-30T05:10:00Z"})
	assertEqual(t, plan.SyncDecisions(h2.ctx), []string{})
	assertIn(t, config.LabelNeedsDecision, h2.gh.LabelNames(12))
}

// #2986 kept a trusted comment after the question (`replied`) and waited for a person to queue it;
// that comment is the answer now.
func TestDecision_AnOlderRepliedStopIsAnswered(t *testing.T) {
	h, _, _ := decisionAsked(t, decisionAsk)
	decisionReply(h, 12, "Hmm, let me think about earning.")
	h.ctx.Store.Update(func(s *pyjson.Object) any {
		kept := queue.DecisionOf(state.Item(s, 12))
		kept.Pop("ends")
		kept.Set("replied", "jgoetzmann")
		return nil
	}, "replied")
	section := strings.Join(dashboard.DecisionsSection(h.ctx, h.ctx.Store.Load(), h.gh.ListIssues("", "open", 300, "")), "\n")
	assertIn(t, "now: @jgoetzmann answered, so it is queued again with the answer", section)
	assertEqual(t, plan.SyncDecisions(h.ctx),
		[]string{"#12: @jgoetzmann answered its question, so it is queued again with the answer"})
	assertFalse(t, decisionOf(h, 12).Has("replied"))
}

func TestDecision_AHandOverIsNoQuestion(t *testing.T) {
	h, _, _ := decisionAsked(t, humanBlocked("Set `VITE_SUPABASE_URL` in Vercel's project settings.", true))
	labels := h.gh.LabelNames(12)
	assertIn(t, config.LabelHumanRequired, labels)
	assertNotIn(t, config.LabelNeedsDecision, labels)
	assertTrue(t, decisionOf(h, 12) == nil)
	assertTrue(t, queue.ArchivedOf(state.Item(h.ctx.Store.Load(), 12)) != nil)
	section := strings.Join(dashboard.DecisionsSection(h.ctx, h.ctx.Store.Load(), h.gh.ListIssues("", "open", 300, "")), "\n")
	assertIn(t, dashboard.NoDecisions, section)
	assertIn(t, "| #12 Patch: Earning | Tue 29 Sep 22:00 CDT | now |", section)
	decisionLater(h, 3)
	section = strings.Join(dashboard.DecisionsSection(h.ctx, h.ctx.Store.Load(), h.gh.ListIssues("", "open", 300, "")), "\n")
	assertIn(t, "| #12 Patch: Earning | Tue 29 Sep 22:00 CDT | 3h 00m |", section)
	// #2986 put both labels on a hand-over and kept a stop for it: the sweep takes the label off and
	// archives the stop.
	h.gh.AddIssue(14, fakes.Issue{Title: "CI: a secret", Labels: []string{config.LabelHumanRequired,
		config.LabelNeedsDecision}})
	h.ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, 14).Update(pyjson.New("kind", "build", "owner", "claude-1", queue.DecisionKey,
			pyjson.New("asked_at", "2026-09-29T06:00:00Z", "human_required", true, "question", "Add it.")))
		return nil
	}, "older hand-over")
	assertEqual(t, plan.SyncDecisions(h.ctx), []string{"#14 is handed to a person, so it waits for no decision (`needs decision` off)"})
	assertNotIn(t, config.LabelNeedsDecision, h.gh.LabelNames(14))
	record := state.Item(h.ctx.Store.Load(), 14)
	assertEqual(t, []any{queue.DecisionOf(record) == nil, record.Has("kind"), record.Get("owner")}, []any{true, false, ""})
	assertEqual(t, queue.ArchivedOf(record).Get("at"), "2026-09-29T06:00:00Z")
	// Another bot's, with no record here (#2980): every sweep takes the label off a hand-over.
	h.gh.AddIssue(2980, fakes.Issue{Title: "Architecture: weekly hygiene",
		Labels: []string{"squishy:planned", config.LabelHumanRequired, config.LabelNeedsDecision}})
	assertEqual(t, plan.HandedOver(h.ctx), []string{"#2980 is handed to a person, so it carries no `needs decision`"})
	assertNotIn(t, config.LabelNeedsDecision, h.gh.LabelNames(2980))
	assertEqual(t, plan.HandedOver(h.ctx), []string{})
}

func TestDecision_AQuestionWithNoSuggestionWaitsForAPerson(t *testing.T) {
	h, _, said := decisionAsked(t, `<!-- bot: {"status": "blocked", "question": "Option A or B?", `+
		`"options": ["A", "B"]} -->`+"\nStopped.")
	assertIn(t, config.LabelNeedsDecision, h.gh.LabelNames(12))
	assertIn(t, "1. A\n2. B\n", said) // its options, with no trade-off given
	assertIn(t, "**My suggestion:** none of them", said)
	assertIn(t, "With no suggestion, it waits for your answer: I never go ahead by myself.", said)
	assertFalse(t, decisionOf(h, 12).Has("ends"))
	decisionLater(h, 100)
	assertEqual(t, plan.SyncDecisions(h.ctx), []string{})
	section := strings.Join(dashboard.DecisionsSection(h.ctx, h.ctx.Store.Load(), h.gh.ListIssues("", "open", 300, "")), "\n")
	assertIn(t, "never: it names no default, so it waits for a person's answer", section)
	// A reply still revises it.
	decisionReply(h, 12, "B.")
	assertEqual(t, plan.SyncDecisions(h.ctx),
		[]string{"#12: @jgoetzmann answered its question, so it is queued again with the answer"})
}

func TestDecision_ClosingTheItemTakesTheLabelAndAssigneesOff(t *testing.T) {
	h, _, _ := decisionAsked(t, decisionAsk)
	h.gh.Threads.Item(12).Set("state", "closed")
	assertEqual(t, plan.SyncDecisions(h.ctx), []string{"#12 closed, so it waits for no decision (`needs decision` off)"})
	assertNotIn(t, config.LabelNeedsDecision, h.gh.LabelNames(12))
	assertEqual(t, decisionAssignees(h, 12), setOf())
	assertTrue(t, decisionOf(h, 12) == nil)
}

func TestDecision_TheStatusIssueListsDecisionsThenHandOvers(t *testing.T) {
	h, _, _ := decisionAsked(t, decisionAsk)
	decisionLater(h, 3)
	issues := h.gh.ListIssues("", "open", 300, "")
	section := strings.Join(dashboard.DecisionsSection(h.ctx, h.ctx.Store.Load(), issues), "\n")
	assertIn(t, dashboard.DecisionsHeading, section)
	assertIn(t, "| #12 Patch: Earning | 3h 00m | Wed 30 Sep 22:00 CDT ", section)
	assertIn(t, "(in 21h 00m), on option 1, its default |", section)
	assertIn(t, dashboard.HandedHeading+"\n\n"+dashboard.NoneHanded, section)
	// A hand-over, another bot's or a person's, is listed below the decisions, never among them.
	h.gh.AddIssue(2980, fakes.Issue{Title: "Architecture: weekly hygiene",
		Labels: []string{"squishy:planned", config.LabelHumanRequired, config.LabelNeedsDecision}})
	section = strings.Join(dashboard.DecisionsSection(h.ctx, h.ctx.Store.Load(),
		h.gh.ListIssues("", "open", 300, "")), "\n")
	assertNotIn(t, "| #2980 Architecture: weekly hygiene | — | "+dashboard.NeverPerson+" |", section)
	assertIn(t, "| #2980 Architecture: weekly hygiene | — | — |", section)
	body := dashboard.Render(h.ctx, nil, "", nil, true)
	top, handed, running := strings.Index(body, dashboard.DecisionsHeading), strings.Index(body, dashboard.HandedHeading),
		strings.Index(body, "## ⚡ Running now")
	assertTrue(t, top > 0 && top < handed && handed < running, top, handed, running)
	assertLess(t, strIndex(t, body, "| #12 Patch: Earning |"), handed)
	assertLess(t, handed, strIndex(t, body, "| #2980 Architecture: weekly hygiene | — | — |"))
	// With nothing waiting it says so.
	h.gh.RemoveLabel(12, config.LabelNeedsDecision)
	assertIn(t, dashboard.NoDecisions, strings.Join(dashboard.DecisionsSection(h.ctx, h.ctx.Store.Load(),
		h.gh.ListIssues("", "open", 300, "")), "\n"))
}

func TestDecision_TheSweepLabelsTheQuestionsAskedBeforeOnce(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(2882, fakes.Issue{Title: "Patch: Earning", Labels: []string{config.LabelBlocked}})
	gh.AddComment(2882, "I need a decision before I can build this.\n\n(A) or (B)?",
		fakes.Comment{User: fakes.Bot, CreatedAt: "2026-09-29T06:00:00Z"})
	// Blocked since for another reason: its question was answered, and CI capped it.
	gh.AddIssue(40, fakes.Issue{Labels: []string{config.LabelBlocked}})
	gh.AddComment(40, "This failed 3 times in a row.", fakes.Comment{User: fakes.Bot})
	// A hand-over is no question (#3009).
	gh.AddIssue(41, fakes.Issue{Labels: []string{config.LabelHumanRequired}})
	gh.AddComment(41, "@MaxGoetzmann @jgoetzmann this needs a person: I cannot build this myself.",
		fakes.Comment{User: fakes.Bot})
	ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, 2882).Set("question", "(A) or (B)?")
		state.Item(s, 40).Set("question", "An old one")
		state.Item(s, 41).Set("question", "Add the secret.")
		return nil
	}, "older")
	assertEqual(t, plan.OlderQuestions(ctx),
		[]string{"#2882 stopped on a question before #2986, so `needs decision`, with no deadline"})
	assertIn(t, config.LabelNeedsDecision, gh.LabelNames(2882))
	assertNotIn(t, config.LabelNeedsDecision, gh.LabelNames(40))
	assertNotIn(t, config.LabelNeedsDecision, gh.LabelNames(41))
	decision := queue.DecisionOf(state.Item(ctx.Store.Load(), 2882))
	assertEqual(t, []any{decision.Get("asked_at"), decision.Get("older"), decision.Has("ends")},
		[]any{"2026-09-29T06:00:00Z", true, false})
	// Only the first time.
	gh.RemoveLabel(2882, config.LabelNeedsDecision)
	assertEqual(t, plan.OlderQuestions(ctx), []string{})
	assertNotIn(t, config.LabelNeedsDecision, gh.LabelNames(2882))
}

func TestDecision_ARevisionsQuestionIsAnsweredAsARevision(t *testing.T) {
	h := newFlowHarness(t)
	start := fakes.PushBranch(h.origin, h.root, "bot/issue-12", map[string]string{"src/game.txt": "v2\n"})
	fakes.Git(h.clone, "fetch", "-q", "origin")
	fakes.Git(h.deliverRepo, "fetch", "-q", "origin")
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelPROpen}})
	h.gh.AddPull(40, "bot/issue-12", fakes.Pull{Body: "Closes #12", Labels: []string{config.LabelPR, config.LabelRevise},
		SHA: start})
	h.gh.AddComment(40, "@jgoetzmann-bot earn something", fakes.Comment{User: fakes.Operator})
	_, result := h.night(workFake(map[string]any{"revise": workBuilder(map[string]any{}, decisionAsk),
		"review": workReviewer(workApprove)}))
	assertEqual(t, result.Get("status"), "blocked")
	assertEqual(t, h.gh.LabelNames(40), setOf(config.LabelPR, config.LabelBlocked, config.LabelNeedsDecision))
	assertEqual(t, decisionAssignees(h, 40), setOf("jgoetzmann", "MaxGoetzmann"))
	said := lastOf(t, h.gh.BotComments(40))
	assertIn(t, "I need a decision before I can revise this.", said)
	assertIn(t, "Reply with a number, or tell me something else in your own words", said)
	decisionReply(h, 40, "(B), please.")
	plan.SyncDecisions(h.ctx)
	assertEqual(t, h.gh.LabelNames(40), setOf(config.LabelPR, config.LabelRevise))
	assertEqual(t, decisionAssignees(h, 40), setOf())
	assertEqual(t, state.Item(h.ctx.Store.Load(), 40).Get("source"), "decision")
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	assertEqual(t, []any{planned.Get("action"), planned.Get("number")}, []any{"revise", 40})
	assertTrue(t, planned.Truthy("on_answer"))
	assertIn(t, "Their answer is the decision", planned.Str("feedback"))
	assertIn(t, "(B), please.", planned.Str("feedback"))
}

func TestDecision_ARevisionsQuestionGoesAheadAsARevision(t *testing.T) {
	h := newFlowHarness(t)
	start := fakes.PushBranch(h.origin, h.root, "bot/issue-12", map[string]string{"src/game.txt": "v2\n"})
	fakes.Git(h.clone, "fetch", "-q", "origin")
	fakes.Git(h.deliverRepo, "fetch", "-q", "origin")
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelPROpen}})
	h.gh.AddPull(40, "bot/issue-12", fakes.Pull{Body: "Closes #12", Labels: []string{config.LabelPR, config.LabelRevise},
		SHA: start})
	h.gh.AddComment(40, "@jgoetzmann-bot earn something", fakes.Comment{User: fakes.Operator})
	_, result := h.night(workFake(map[string]any{"revise": workBuilder(map[string]any{}, decisionAsk),
		"review": workReviewer(workApprove)}))
	assertEqual(t, result.Get("status"), "blocked")
	decisionLater(h, 25)
	plan.SyncDecisions(h.ctx)
	assertEqual(t, h.gh.LabelNames(40), setOf(config.LabelPR, config.LabelRevise))
	assertEqual(t, state.Item(h.ctx.Store.Load(), 40).Get("source"), "default")
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	assertEqual(t, []any{planned.Get("action"), planned.Get("number")}, []any{"revise", 40})
	assertTrue(t, planned.Truthy("on_default"))
	assertIn(t, "goes ahead on its default: option 1", planned.Str("feedback"))
}

// The docs list the label: the conventions' labels table and the bot's README.
func TestDecision_TheDocsSayWhatTheLabelMeans(t *testing.T) {
	for _, doc := range [][]string{{"docs", "issues-and-patches.md"}, {"bot", "README.md"}} {
		text := readText(t, filepath.Join(append([]string{testsupport.Root}, doc...)...))
		assertIn(t, "| `"+config.LabelNeedsDecision+"` |", text, doc)
	}
	entry := pyjson.List(config.Labels.Get(config.LabelNeedsDecision))
	assertEqual(t, entry[0], pyjson.List(config.Labels.Get(config.LabelSuggestion))[0]) // `d4c5f9`
	assertEqual(t, entry[0], "d4c5f9")
	assertTrue(t, len([]rune(pyjson.Str(entry[1]))) <= config.LabelDescriptionMax)
}
