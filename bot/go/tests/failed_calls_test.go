package tests

// bot/tests/test_failed_calls.py: #317 part 1: a model call that fails in seconds is the
// subscription's failure, not the item's.
//
// A weak subscription's CLI failed every call in seconds from 2026-10-05 07:30Z. Each failed
// builder call was committed as a pass, checked, self-checked and delivered as "Revision done",
// and each failed review call was read as an unreadable answer, so the items were charged and
// blocked while the broken subscription kept getting work.

import (
	"fmt"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	ts "github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// ---------------------------------------------------------------------------- the file's own

// failedCallsFailing is failing(seconds, error, text=""): the handler of a call that fails after
// `seconds`.
func failedCallsFailing(seconds float64, error_ string) runner.Handler {
	return func(request *runner.RunRequest) *runner.RunResult {
		result := runner.NewRunResult(false, "")
		result.ExitCode, result.DurationS, result.Error = 1, seconds, error_
		return result
	}
}

// ---------------------------------------------------------------------------- InstantFailureTests

func TestFailedCalls_ACallThatFailsInSecondsBacksTheSubscriptionOffAndSparesTheItem(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, At: ts.Day, Machine: ts.Machine})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned, result := h.night(workFake(map[string]runner.Handler{"plan": failedCallsFailing(2, "}")}))
	provider := planned.Str("provider")
	assertEqual(t, result.Get("status"), "infra")
	assertIn(t, "CLI failed at once: }", result.Get("reason"))
	st := h.ctx.Store.Load()
	assertEqual(t, st.Obj("providers").Obj(provider).Obj("infra").Get("streak"), 1)
	failures, _ := pyjson.AsInt(st.Obj("items").Obj("12").GetOr("failures", 0))
	assertEqual(t, failures, 0)
	both := map[string]bool{} // label_names(12) & {LABEL_BUILD}
	if h.gh.LabelNames(12)[config.LabelBuild] {
		both[config.LabelBuild] = true
	}
	assertEqual(t, both, setOf(config.LabelBuild))
	comments := h.gh.BotComments(12)
	assertIn(t, "That is not this item's fault", comments[len(comments)-1])
	// The subscription is left alone: the next run goes elsewhere.
	planned, _ = h.night(workFake(map[string]runner.Handler{
		"build":  workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)}))
	assertNotEqual(t, planned.Get("provider"), provider)
}

func TestFailedCalls_ThreeInARowAskAPerson(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""}, At: ts.Day, Machine: []string{"muse"}})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	for range 3 {
		h.night(workFake(map[string]runner.Handler{"plan": failedCallsFailing(3, "Error: model unavailable")}), flowPlanArgs{Force: true})
	}
	issues := []*pyjson.Object{}
	for _, thread := range h.gh.Threads.Values() {
		if strings.Contains(failedCallsPyStr(thread.Get("title")), "could not work 3 times in a row") {
			issues = append(issues, thread)
		}
	}
	assertEqual(t, len(issues), 1)
	assertIn(t, "model unavailable", issues[0].Get("body"))
	names := map[string]bool{}
	for _, label := range pyjson.List(issues[0].Get("labels")) {
		names[pyjson.Obj(label).Str("name")] = true
	}
	for _, want := range []string{"night bot", "human"} {
		assertTrue(t, names[want], want, " not in ", names)
	}
}

// failedCallsPyStr is str(v): None is "None".
func failedCallsPyStr(v any) string {
	if v == nil {
		return "None"
	}
	if s, ok := v.(string); ok {
		return s
	}
	return fmt.Sprint(v)
}

func TestFailedCalls_ABuilderThatFailsAfterWorkingAWhileButChangedNothingFailsTheRun(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, At: ts.Day, Machine: ts.Machine})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	_, result := h.night(workFake(map[string]runner.Handler{"build": failedCallsFailing(600, "the session crashed")}))
	assertEqual(t, result.Get("status"), "failed")
	assertIn(t, "the builder's session failed and changed nothing: the session crashed", result.Get("reason"))
	cycles := pyjson.List(result.Get("cycles"))
	assertEqual(t, pyjson.Obj(cycles[0]).Obj("builder").Get("changed"), false)
	assertIsNone(t, h.originSHA("bot/issue-12")) // no "pass 1" commit, nothing pushed
	failures, _ := pyjson.AsInt(h.ctx.Store.Load().Obj("items").Obj("12").Get("failures"))
	assertEqual(t, failures, 1)
}

// Claude's turn limit ends a long session `ok: false`: its work is still a pass.
func TestFailedCalls_ABuilderThatFailedAfterDoingWorkKeepsIt(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, At: ts.Day, Machine: ts.Machine})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	inner := workBuilder(map[string]string{"src/game.txt": "v2\n"})

	turnLimit := func(request *runner.RunRequest) *runner.RunResult {
		done := inner(request)
		result := runner.NewRunResult(false, done.Text)
		result.ExitCode, result.DurationS, result.Error = 1, 5400, "error_max_turns"
		return result
	}
	_, result := h.night(workFake(map[string]runner.Handler{"build": turnLimit,
		"review": workReviewer(workApprove)}))
	assertEqual(t, result.Get("status"), "approved")
	cycles := pyjson.List(result.Get("cycles"))
	assertEqual(t, pyjson.Obj(cycles[0]).Obj("builder").Get("changed"), true)
}

func TestFailedCalls_AReviewCallThatFailsNamesItsError(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: crossAll, At: ts.Day, Machine: ts.Machine})
	h.committedHours() // claude-3 works all day: the Strong final can audit
	pr, _ := crossBuildOnMedium(t, h, "", nil)
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelCross, "difficulty:medium"))
	_, review := h.night(workFake(map[string]runner.Handler{"review": failedCallsFailing(900, "overloaded")}))
	assertEqual(t, review.Get("status"), "failed")
	assertEqual(t, review.Get("reason"), "the reviewer's call failed: overloaded")
	comments := h.gh.BotComments(pr)
	assertIn(t, "the reviewer's call failed: overloaded", comments[len(comments)-1])
	assertNotIn(t, "could not be read", comments[len(comments)-1])
}

// ---------------------------------------------------------------------------- StderrTests

func TestFailedCalls_AJSONErrorBodyGivesItsMessage(t *testing.T) {
	stderr := "Welcome\n{\n  \"error\": {\n    \"type\": \"rate\",\n" +
		"    \"message\": \"Your daily allowance is spent\"\n  }\n}\n"
	assertEqual(t, runner.StderrError(stderr, "x"), "Your daily allowance is spent")
}

func TestFailedCalls_OtherwiseTheLastLines(t *testing.T) {
	lines := []string{}
	for n := range 30 {
		lines = append(lines, fmt.Sprintf("line %d", n))
	}
	found := runner.StderrError(strings.Join(lines, "\n"), "x")
	assertTrue(t, strings.HasPrefix(found, "line 10\n") && strings.HasSuffix(found, "line 29"))
	assertEqual(t, runner.StderrError("  \n", "lite exited 1"), "lite exited 1")
}
