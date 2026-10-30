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
	"maps"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	ts "github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// ---------------------------------------------------------------------------- from test_work

// failedCallsDone is test_work.DONE.
const failedCallsDone = "<!-- bot: {\"status\": \"done\", \"title\": \"Make the rules v2\"} -->\n## What changed\nRules v2."

// failedCallsApprove is test_work.APPROVE.
const failedCallsApprove = "<!-- review: {\"verdict\": \"approve\", \"findings\": []} -->\nLooked hard; fine."

// failedCallsGates is test_work.GATES.
func failedCallsGates() []any {
	return []any{pyjson.New("name", "rules exist", "run", "test -f src/game.txt", "timeout_minutes", 1),
		pyjson.New("name", "no broken file", "run", "test ! -f broken.txt", "timeout_minutes", 1)}
}

// failedCallsBuilder is test_work.builder(edits): a builder that writes `edits` (name → text) in
// its worktree and answers DONE.
func failedCallsBuilder(edits map[string]string) runner.Handler {
	return func(request *runner.RunRequest) *runner.RunResult {
		for name, content := range edits {
			target := filepath.Join(request.Cwd, name)
			if err := os.MkdirAll(filepath.Dir(target), 0o777); err != nil {
				panic(err)
			}
			if err := os.WriteFile(target, []byte(content), 0o666); err != nil {
				panic(err)
			}
		}
		result := runner.NewRunResult(true, failedCallsDone)
		result.Usage = pyjson.New("five_hour", pyjson.New("utilization", 0.3, "resets_at", nil))
		return result
	}
}

// failedCallsReviewer is test_work.reviewer(text).
func failedCallsReviewer(text string) runner.Handler {
	return func(request *runner.RunRequest) *runner.RunResult { return runner.NewRunResult(true, text) }
}

// ---------------------------------------------------------------------------- from test_cross

// failedCallsAll is test_cross.ALL.
func failedCallsAll() map[string]string {
	return map[string]string{"HARNESS_SECRETS_SET": strings.Join(providers.SecretNames, " ")}
}

// failedCallsTimedBuilder is test_cross.timed_builder(edits, minutes=10.0).
func failedCallsTimedBuilder(edits map[string]string, minutes float64) runner.Handler {
	inner := failedCallsBuilder(edits)
	return func(request *runner.RunRequest) *runner.RunResult {
		result := inner(request)
		result.DurationS = minutes * 60
		return result
	}
}

// failedCallsMedium is test_cross.MEDIUM: each medium subscription's model and family.
var failedCallsMedium = map[string][2]string{"muse": {"muse-spark-1.3-contributor", "muse"},
	"agy": {"gemini-3.8-flash-high", "gemini"}}

// failedCallsBuildOnMedium is test_cross.ReviewRuleTests.build_on_medium(h) (provider="muse",
// edits=None): by day the Claude accounts are closed: Muse, first of the medium models (#317 part
// 9), plans, builds and reviews it in one run, and its own approval is one medium vote.
func failedCallsBuildOnMedium(t *testing.T, h *failedCallsHarness) (int, *pyjson.Object) {
	t.Helper()
	provider := "muse"
	h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: []string{config.LabelBuild}})
	fake := runner.NewFakeRunner(map[string][]runner.Handler{
		"build":  {failedCallsTimedBuilder(map[string]string{"src/game.txt": "rules v2\n"}, 10.0)},
		"review": {failedCallsReviewer(failedCallsApprove)}})
	planned, result := h.night(fake, false)
	providersEq(t, []any{planned.Get("provider"), result.Get("status")}, []any{provider, "approved"})
	roles, models := []string{}, map[string]bool{}
	for _, c := range fake.Calls {
		roles = append(roles, c.Role)
		models[c.Model] = true
	}
	providersEq(t, roles, []string{"plan", "build", "review"})
	providersEq(t, models, map[string]bool{failedCallsMedium[provider][0]: true})
	pr := h.gh.ListPulls("open", "bot/issue-12", 200)[0].Int("number")
	return pr, result
}

// ---------------------------------------------------------------------------- from test_flow

// failedCallsHarness is test_flow.Harness: one repository, one fake GitHub, and the three jobs of
// a night run.
type failedCallsHarness struct {
	root        string
	origin      string
	clone       string
	gh          *fakes.FakeGitHub
	env         map[string]string
	machine     []string
	overrides   *pyjson.Object
	at          time.Time
	cfg         *config.Config
	ctx         *hctx.Context
	deliverRepo string
}

// newFailedCallsHarness is Harness(test, env, at, machine) with no config overrides.
func newFailedCallsHarness(t *testing.T, env map[string]string, at time.Time, machine []string) *failedCallsHarness {
	h := &failedCallsHarness{root: t.TempDir()}
	h.origin, h.clone = fakes.MakeOrigin(h.root)
	h.gh = fakes.NewFakeGitHub()
	h.env = map[string]string{"GITHUB_SERVER_URL": "file://" + filepath.Join(h.root, "remote"),
		"BOT_GITHUB_TOKEN": strings.Repeat("t", 20)}
	maps.Copy(h.env, env)
	h.overrides = pyjson.New("gates", failedCallsGates(),
		"install", pyjson.New("run", "true", "timeout_minutes", 1), "max_review_cycles", 2)
	h.machine, h.at = machine, at
	h.cfg = ts.MakeConfig(ts.Config{Env: h.env, Machine: machine, Overrides: h.overrides.Copy()})
	h.ctx = ts.MakeCtx(h.gh, ts.Ctx{Cfg: h.cfg, At: at})
	h.gh.BranchChecks = map[string]bool{}
	for _, check := range h.cfg.RequiredChecks {
		h.gh.BranchChecks[check] = true
	}
	h.deliverRepo = filepath.Join(h.root, "deliver")
	fakes.Git(h.root, "clone", "-q", h.origin, h.deliverRepo)
	return h
}

// committedHours is Harness.committed_hours: from now on, the subscriptions keep the hours
// providers.json commits (claude-2 and claude-3 all day), with the same GitHub and state.
func (h *failedCallsHarness) committedHours() {
	h.cfg = ts.MakeConfig(ts.Config{Env: h.env, Machine: h.machine, CommittedHours: true,
		Overrides: h.overrides.Copy()})
	h.ctx = ts.MakeCtx(h.gh, ts.Ctx{Cfg: h.cfg, At: h.at})
}

// night is Harness.night(runner, force=force): plan, work and deliver.
func (h *failedCallsHarness) night(fake *runner.FakeRunner, force bool) (*pyjson.Object, *pyjson.Object) {
	planned := plan.Make(h.ctx, force, nil, "auto", nil)
	outs, err := filepath.Glob(filepath.Join(h.root, "out-*"))
	if err != nil {
		panic(err)
	}
	out := filepath.Join(h.root, "out-"+strconv.Itoa(len(outs)))
	if !pyjson.Equal(planned.Get("action"), "none") {
		work.NewWorker(h.cfg, planned, fake, h.clone, filepath.Join(h.root, "work"), out,
			nil, nil, nil, nil, nil, nil).Run()
		deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	}
	result := pyjson.New()
	if text, err := os.ReadFile(filepath.Join(out, "result.json")); err == nil {
		v, err := pyjson.Loads(text)
		if err != nil {
			panic(err)
		}
		result = v.(*pyjson.Object)
	}
	return planned, result
}

// originSHA is Harness.origin_sha(branch): "" (None) when origin has no such branch.
func (h *failedCallsHarness) originSHA(branch string) string {
	sha := ""
	if caught, ok := herr.Catch[*fakes.PyError](func() {
		sha = fakes.Git(h.origin, "rev-parse", "--verify", "--quiet", "refs/heads/"+branch)
	}); ok {
		if caught.Type != "AssertionError" {
			panic(caught)
		}
		return ""
	}
	return sha
}

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

// failedCallsRunner is FakeRunner(handlers), one handler a role.
func failedCallsRunner(handlers map[string]runner.Handler) *runner.FakeRunner {
	queues := map[string][]runner.Handler{}
	for role, handler := range handlers {
		queues[role] = []runner.Handler{handler}
	}
	return runner.NewFakeRunner(queues)
}

// ---------------------------------------------------------------------------- InstantFailureTests

func TestFailedCalls_ACallThatFailsInSecondsBacksTheSubscriptionOffAndSparesTheItem(t *testing.T) {
	h := newFailedCallsHarness(t, failedCallsAll(), ts.Day, ts.Machine)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned, result := h.night(failedCallsRunner(map[string]runner.Handler{"plan": failedCallsFailing(2, "}")}), false)
	provider := planned.Str("provider")
	providersEq(t, result.Get("status"), "infra")
	providersIn(t, "CLI failed at once: }", result.Get("reason"))
	st := h.ctx.Store.Load()
	providersEq(t, st.Obj("providers").Obj(provider).Obj("infra").Get("streak"), 1)
	failures, _ := pyjson.AsInt(st.Obj("items").Obj("12").GetOr("failures", 0))
	providersEq(t, failures, 0)
	both := map[string]bool{} // label_names(12) & {LABEL_BUILD}
	if h.gh.LabelNames(12)[config.LabelBuild] {
		both[config.LabelBuild] = true
	}
	providersEq(t, both, providersSetOf(config.LabelBuild))
	comments := h.gh.BotComments(12)
	providersIn(t, "That is not this item's fault", comments[len(comments)-1])
	// The subscription is left alone: the next run goes elsewhere.
	planned, _ = h.night(failedCallsRunner(map[string]runner.Handler{
		"build":  failedCallsBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": failedCallsReviewer(failedCallsApprove)}), false)
	providersNe(t, planned.Get("provider"), provider)
}

func TestFailedCalls_ThreeInARowAskAPerson(t *testing.T) {
	h := newFailedCallsHarness(t, map[string]string{"HARNESS_SECRETS_SET": ""}, ts.Day, []string{"muse"})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	for range 3 {
		h.night(failedCallsRunner(map[string]runner.Handler{"plan": failedCallsFailing(3, "Error: model unavailable")}),
			true)
	}
	issues := []*pyjson.Object{}
	for _, thread := range h.gh.Threads.Values() {
		if strings.Contains(failedCallsPyStr(thread.Get("title")), "could not work 3 times in a row") {
			issues = append(issues, thread)
		}
	}
	providersEq(t, len(issues), 1)
	providersIn(t, "model unavailable", issues[0].Get("body"))
	names := map[string]bool{}
	for _, label := range pyjson.List(issues[0].Get("labels")) {
		names[pyjson.Obj(label).Str("name")] = true
	}
	for _, want := range []string{"night bot", "human"} {
		providersTrue(t, names[want], want, " not in ", names)
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
	h := newFailedCallsHarness(t, failedCallsAll(), ts.Day, ts.Machine)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	_, result := h.night(failedCallsRunner(map[string]runner.Handler{"build": failedCallsFailing(600, "the session crashed")}),
		false)
	providersEq(t, result.Get("status"), "failed")
	providersIn(t, "the builder's session failed and changed nothing: the session crashed", result.Get("reason"))
	cycles := pyjson.List(result.Get("cycles"))
	providersEq(t, pyjson.Obj(cycles[0]).Obj("builder").Get("changed"), false)
	providersNone(t, h.originSHA("bot/issue-12")) // no "pass 1" commit, nothing pushed
	failures, _ := pyjson.AsInt(h.ctx.Store.Load().Obj("items").Obj("12").Get("failures"))
	providersEq(t, failures, 1)
}

// Claude's turn limit ends a long session `ok: false`: its work is still a pass.
func TestFailedCalls_ABuilderThatFailedAfterDoingWorkKeepsIt(t *testing.T) {
	h := newFailedCallsHarness(t, failedCallsAll(), ts.Day, ts.Machine)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	inner := failedCallsBuilder(map[string]string{"src/game.txt": "v2\n"})

	turnLimit := func(request *runner.RunRequest) *runner.RunResult {
		done := inner(request)
		result := runner.NewRunResult(false, done.Text)
		result.ExitCode, result.DurationS, result.Error = 1, 5400, "error_max_turns"
		return result
	}
	_, result := h.night(failedCallsRunner(map[string]runner.Handler{"build": turnLimit,
		"review": failedCallsReviewer(failedCallsApprove)}), false)
	providersEq(t, result.Get("status"), "approved")
	cycles := pyjson.List(result.Get("cycles"))
	providersEq(t, pyjson.Obj(cycles[0]).Obj("builder").Get("changed"), true)
}

func TestFailedCalls_AReviewCallThatFailsNamesItsError(t *testing.T) {
	h := newFailedCallsHarness(t, failedCallsAll(), ts.Day, ts.Machine)
	h.committedHours() // claude-3 works all day: the Strong final can audit
	pr, _ := failedCallsBuildOnMedium(t, h)
	providersEq(t, h.gh.LabelNames(pr), providersSetOf(config.LabelPR, config.LabelCross, "difficulty:medium"))
	_, review := h.night(failedCallsRunner(map[string]runner.Handler{"review": failedCallsFailing(900, "overloaded")}), false)
	providersEq(t, review.Get("status"), "failed")
	providersEq(t, review.Get("reason"), "the reviewer's call failed: overloaded")
	comments := h.gh.BotComments(pr)
	providersIn(t, "the reviewer's call failed: overloaded", comments[len(comments)-1])
	providersNotIn(t, "could not be read", comments[len(comments)-1])
}

// ---------------------------------------------------------------------------- StderrTests

func TestFailedCalls_AJSONErrorBodyGivesItsMessage(t *testing.T) {
	stderr := "Welcome\n{\n  \"error\": {\n    \"type\": \"rate\",\n" +
		"    \"message\": \"Your daily allowance is spent\"\n  }\n}\n"
	providersEq(t, runner.StderrError(stderr, "x"), "Your daily allowance is spent")
}

func TestFailedCalls_OtherwiseTheLastLines(t *testing.T) {
	lines := []string{}
	for n := range 30 {
		lines = append(lines, fmt.Sprintf("line %d", n))
	}
	found := runner.StderrError(strings.Join(lines, "\n"), "x")
	providersTrue(t, strings.HasPrefix(found, "line 10\n") && strings.HasSuffix(found, "line 29"))
	providersEq(t, runner.StderrError("  \n", "lite exited 1"), "lite exited 1")
}
