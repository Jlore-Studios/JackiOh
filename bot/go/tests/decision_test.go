// A question goes ahead on its default after `decision_wait_hours`, and `needs decision` marks every
// stop only a person can unblock (#2986, sections 2 and 3; the owner's decision of 2026-11-04).
package tests

import (
	"fmt"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
)

// decisionAsk is a builder's report that stops on a question with two options, the first its default.
const decisionAsk = `<!-- bot: {"status": "blocked", "question": "Should a win earn Jlore?", ` +
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

func TestDecision_TheReportNamesItsOptionsAndDefault(t *testing.T) {
	report := verdicts.BuildReport_(decisionAsk)
	assertEqual(t, []any{report.Status, report.Options, report.Default, report.GivesUp},
		[]any{"blocked", []string{"(A) No earning: already on the branch", "(B) Earn 10 Jlore a win"}, 1,
			"Players earn nothing for a win yet"})
	// A default that names no option is none; a hand-over never has one.
	none := verdicts.BuildReport_(`<!-- bot: {"status": "blocked", "question": "Q", "options": ["a"], "default": 3} -->`)
	assertEqual(t, []any{none.Options, none.Default}, []any{[]string{"a"}, 0})
	human := verdicts.BuildReport_(`<!-- bot: {"status": "blocked", "human_required": true, "question": "Q", ` +
		`"options": ["a", "b"], "default": 1} -->`)
	assertEqual(t, []any{human.HumanRequired, human.Options, human.Default}, []any{true, []string{}, 0})
}

func TestDecision_AQuestionNamesItsDefaultAndWhenItGoesAhead(t *testing.T) {
	h, planned, said := decisionAsked(t, decisionAsk)
	labels := h.gh.LabelNames(12)
	assertIn(t, config.LabelBlocked, labels)
	assertIn(t, config.LabelNeedsDecision, labels) // beside `bot:blocked`, which keeps its meaning
	assertTrue(t, strings.HasPrefix(said, "I need a decision before I can build this."), said)
	assertIn(t, "1. (A) No earning: already on the branch **(the default)**", said)
	assertIn(t, "2. (B) Earn 10 Jlore a win", said)
	provider := planned.Str("provider")
	assertIn(t, "If no trusted person answers by Wed 30 Sep 22:00 CDT (24 hours", said)
	assertIn(t, fmt.Sprintf("(24 hours, `decision_wait_hours`), `%s` takes it up again", provider), said)
	assertIn(t, "The default gives up: Players earn nothing for a win yet.", said)
	assertIn(t, "@jgoetzmann: answer here, then `/harness build`", said)
	decision := decisionOf(h, 12)
	assertEqual(t, []any{decision.Get("provider"), decision.Get("default"), decision.Get("kind"), decision.Get("hours")},
		[]any{provider, 1, "build", 24})
	assertEqual(t, decision.Get("ends"), clock.ISO(testsupport.Night.Add(24*time.Hour)))
	assertTrue(t, decision.Truthy("model"))
	// Before its end nothing goes ahead.
	decisionLater(h, 23)
	assertEqual(t, plan.SyncDecisions(h.ctx), []string{})
	assertIn(t, config.LabelNeedsDecision, h.gh.LabelNames(12))
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
	assertEqual(t, queue.OwnerOf(h.ctx.Store.Load(), 12), provider)
	said := lastOf(t, h.gh.BotComments(12))
	assertIn(t, "**Going ahead on its default**", said)
	assertIn(t, "goes ahead on option 1, its default: (A) No earning: already on the branch.", said)
	// The other option is an issue of its own for a person to approve: not queued.
	filed := pyjson.List(decisionOf(h, 12).Get("filed"))
	assertEqual(t, len(filed), 1)
	option := h.gh.Threads.Get(filed[0].(int))
	assertEqual(t, option.Str("title"), "Patch: Earning (option 2 of #12)")
	assertIn(t, "(B) Earn 10 Jlore a win", option.Str("body"))
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
	assertEqual(t, result.Get("status"), "approved")
	pull := h.gh.ListPulls("open", "bot/issue-12", 200)[0]
	assertIn(t, "### Built on its default", pull.Str("body"))
	assertIn(t, "This was built on its default because nobody answered its question by", pull.Str("body"))
	assertTrue(t, decisionOf(h, 12) == nil, decisionOf(h, 12))
}

func TestDecision_ATrustedAnswerBeforeTheDeadlineWins(t *testing.T) {
	h, _, _ := decisionAsked(t, decisionAsk)
	flowAsk(h, 12, "@jgoetzmann-bot take (B): earn 10 a win", 901)
	labels := h.gh.LabelNames(12)
	assertIn(t, config.LabelBuild, labels)
	assertNotIn(t, config.LabelNeedsDecision, labels) // a trusted answer takes it off
	assertEqual(t, plan.SyncDecisions(h.ctx), []string{"#12 waits for no decision any more"})
	before := h.gh.Threads.Len()
	decisionLater(h, 25)
	assertEqual(t, plan.SyncDecisions(h.ctx), []string{})
	assertEqual(t, h.gh.Threads.Len(), before) // no option filed
	// The next build reads the question and the answer, not the default.
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	assertEqual(t, planned.Get("number"), 12)
	assertFalse(t, planned.Has("on_default"))
	assertEqual(t, planned.Get("previous_question"), "Should a win earn Jlore?")
}

func TestDecision_ATrustedCommentKeepsItFromGoingAhead(t *testing.T) {
	h, _, _ := decisionAsked(t, decisionAsk)
	h.gh.AddComment(12, "Hmm, let me think about earning.", fakes.Comment{CreatedAt: "2026-09-30T05:00:00Z"})
	h.gh.AddComment(12, "Drive-by.", fakes.Comment{User: pyjson.New("login", "stranger", "id", 9),
		Association: "NONE", CreatedAt: "2026-09-30T06:00:00Z"})
	decisionLater(h, 25)
	assertEqual(t, plan.SyncDecisions(h.ctx),
		[]string{"#12: @jgoetzmann commented after its question, so it does not go ahead on its default"})
	assertIn(t, "the answer wins", lastOf(t, h.gh.BotComments(12)))
	labels := h.gh.LabelNames(12)
	assertIn(t, config.LabelNeedsDecision, labels)
	assertIn(t, config.LabelBlocked, labels)
	assertEqual(t, decisionOf(h, 12).Get("replied"), "jgoetzmann")
	assertEqual(t, plan.SyncDecisions(h.ctx), []string{})
}

func TestDecision_AStopOnlyAPersonCanUnblockNeverGoesAhead(t *testing.T) {
	h, _, _ := decisionAsked(t, humanBlocked("Set `VITE_SUPABASE_URL` in Vercel's project settings.", true))
	labels := h.gh.LabelNames(12)
	assertIn(t, config.LabelHumanRequired, labels)
	assertIn(t, config.LabelNeedsDecision, labels)
	assertTrue(t, decisionOf(h, 12).Truthy("human_required"))
	assertFalse(t, decisionOf(h, 12).Has("ends"))
	decisionLater(h, 100)
	assertEqual(t, plan.SyncDecisions(h.ctx), []string{})
	assertIn(t, config.LabelNeedsDecision, h.gh.LabelNames(12))
	section := strings.Join(dashboard.DecisionsSection(h.ctx, h.ctx.Store.Load(), h.gh.ListIssues("", "open", 300, "")), "\n")
	assertIn(t, "| #12 Patch: Earning | 100h 00m | "+dashboard.NeverPerson+" |", section)
}

func TestDecision_AQuestionWithNoDefaultWaitsForAPerson(t *testing.T) {
	h, _, said := decisionAsked(t, humanBlocked("Option A or B?", false))
	assertIn(t, config.LabelNeedsDecision, h.gh.LabelNames(12))
	assertIn(t, "It names no default, so it waits for your answer", said)
	assertFalse(t, decisionOf(h, 12).Has("ends"))
	decisionLater(h, 100)
	assertEqual(t, plan.SyncDecisions(h.ctx), []string{})
	section := strings.Join(dashboard.DecisionsSection(h.ctx, h.ctx.Store.Load(), h.gh.ListIssues("", "open", 300, "")), "\n")
	assertIn(t, "never: it names no default, so it waits for a person's answer", section)
}

func TestDecision_ClosingTheItemTakesTheLabelOff(t *testing.T) {
	h, _, _ := decisionAsked(t, decisionAsk)
	h.gh.Threads.Item(12).Set("state", "closed")
	assertEqual(t, plan.SyncDecisions(h.ctx), []string{"#12 closed, so it waits for no decision (`needs decision` off)"})
	assertNotIn(t, config.LabelNeedsDecision, h.gh.LabelNames(12))
	assertTrue(t, decisionOf(h, 12) == nil)
}

func TestDecision_TheStatusIssueListsWhatWaitsAtTheTop(t *testing.T) {
	h, _, _ := decisionAsked(t, decisionAsk)
	decisionLater(h, 3)
	issues := h.gh.ListIssues("", "open", 300, "")
	section := strings.Join(dashboard.DecisionsSection(h.ctx, h.ctx.Store.Load(), issues), "\n")
	assertIn(t, dashboard.DecisionsHeading, section)
	assertIn(t, "| #12 Patch: Earning | 3h 00m | Wed 30 Sep 22:00 CDT ", section)
	assertIn(t, "(in 21h 00m), on option 1, its default |", section)
	body := dashboard.Render(h.ctx, nil, "", nil, true)
	top, running := strings.Index(body, dashboard.DecisionsHeading), strings.Index(body, "## ⚡ Running now")
	assertTrue(t, top > 0 && top < running, top, running)
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
	ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, 2882).Set("question", "(A) or (B)?")
		state.Item(s, 40).Set("question", "An old one")
		return nil
	}, "older")
	assertEqual(t, plan.OlderQuestions(ctx),
		[]string{"#2882 stopped on a question before #2986, so `needs decision`, with no deadline"})
	assertIn(t, config.LabelNeedsDecision, gh.LabelNames(2882))
	assertNotIn(t, config.LabelNeedsDecision, gh.LabelNames(40))
	decision := queue.DecisionOf(state.Item(ctx.Store.Load(), 2882))
	assertEqual(t, []any{decision.Get("asked_at"), decision.Get("older"), decision.Has("ends")},
		[]any{"2026-09-29T06:00:00Z", true, false})
	// Only the first time.
	gh.RemoveLabel(2882, config.LabelNeedsDecision)
	assertEqual(t, plan.OlderQuestions(ctx), []string{})
	assertNotIn(t, config.LabelNeedsDecision, gh.LabelNames(2882))
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
	assertEqual(t, h.gh.LabelNames(40), setOf(config.LabelPR, config.LabelBlocked, config.LabelNeedsDecision))
	said := lastOf(t, h.gh.BotComments(40))
	assertIn(t, "I need a decision before I can revise this.", said)
	assertIn(t, "`/harness revise <your answer>`", said)
	decisionLater(h, 25)
	plan.SyncDecisions(h.ctx)
	assertEqual(t, h.gh.LabelNames(40), setOf(config.LabelPR, config.LabelRevise))
	assertEqual(t, state.Item(h.ctx.Store.Load(), 40).Get("source"), "default")
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	assertEqual(t, []any{planned.Get("action"), planned.Get("number")}, []any{"revise", 40})
	assertTrue(t, planned.Truthy("on_default"))
	assertIn(t, "goes ahead on its default: option 1", planned.Str("feedback"))
}
