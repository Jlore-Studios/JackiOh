// #317 parts 4, 8 and 10: the bot rates each item's difficulty when it plans it, under the easy
// rule; three failures of an item's own raise it a step and rebuild its pull request from `main`;
// `/harness rebuild` and `/harness review [strong|medium]`.
//
// bot/tests/test_rating.py. Its file also holds, under the `rating` prefix, what the Python
// imports from test_flow.py (Harness, ask), test_work.py (APPROVE, GATES, DONE, builder,
// reviewer) and test_difficulty.py (seats) for the files ported with it: those files are ported
// apart, so these are their code again under names of this file's, to fold into theirs once both
// are in.
package tests

import (
	"errors"
	"fmt"
	"maps"
	"os"
	"path/filepath"
	"slices"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/commands"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/issueplan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stepup"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// ---------------------------------------------------------------------------- from test_work.py

// ratingDone is test_work.DONE.
const ratingDone = "<!-- bot: {\"status\": \"done\", \"title\": \"Make the rules v2\"} -->\n## What changed\nRules v2."

// ratingApprove is test_work.APPROVE.
const ratingApprove = "<!-- review: {\"verdict\": \"approve\", \"findings\": []} -->\nLooked hard; fine."

// ratingGates is test_work.GATES.
func ratingGates() []any {
	return []any{
		pyjson.New("name", "rules exist", "run", "test -f src/game.txt", "timeout_minutes", 1),
		pyjson.New("name", "no broken file", "run", "test ! -f broken.txt", "timeout_minutes", 1),
	}
}

// ratingBuilder is test_work.builder(edits, text=DONE): a handler that writes `edits` (in
// sorted order: a Go map has no other) under the request's cwd and answers `text`.
func ratingBuilder(edits map[string]string, text ...string) runner.Handler {
	said := ratingDone
	if len(text) > 0 {
		said = text[0]
	}
	return func(request *runner.RunRequest) *runner.RunResult {
		for _, name := range slices.Sorted(maps.Keys(edits)) {
			target := filepath.Join(request.Cwd, name)
			if err := os.MkdirAll(filepath.Dir(target), 0o777); err != nil {
				panic(err)
			}
			if err := os.WriteFile(target, []byte(edits[name]), 0o666); err != nil {
				panic(err)
			}
		}
		result := runner.NewRunResult(true, said)
		result.Usage = pyjson.New("five_hour", pyjson.New("utilization", 0.3, "resets_at", nil))
		return result
	}
}

// ratingReviewer is test_work.reviewer(text).
func ratingReviewer(text string) runner.Handler {
	return func(request *runner.RunRequest) *runner.RunResult { return runner.NewRunResult(true, text) }
}

// ratingRunner is FakeRunner({role: handler, ...}).
func ratingRunner(handlers map[string]runner.Handler) *runner.FakeRunner {
	queues := map[string][]runner.Handler{}
	for role, h := range handlers {
		queues[role] = []runner.Handler{h}
	}
	return runner.NewFakeRunner(queues)
}

// ---------------------------------------------------------------------------- from test_difficulty.py

// ratingSeats is test_difficulty.seats: each seat of a plan as (provider, model, tier), None for
// an empty one, the self check's left out.
func ratingSeats(t *testing.T, planned *pyjson.Object) map[string]any {
	t.Helper()
	out := map[string]any{}
	eventsDig(t, planned, "seats").(*pyjson.Object).Items(func(role string, v any) bool {
		if role == "self_check" {
			return true
		}
		if seat := pyjson.Obj(v); pyjson.Truthy(seat) {
			out[role] = []any{eventsDig(t, seat, "provider"), eventsDig(t, seat, "model"), eventsDig(t, seat, "tier")}
		} else {
			out[role] = nil
		}
		return true
	})
	return out
}

// ---------------------------------------------------------------------------- from test_flow.py

// ratingHarness is test_flow.Harness: one repository, one fake GitHub, and the three jobs of a
// night run.
type ratingHarness struct {
	t           *testing.T
	root        string
	origin      string
	clone       string
	gh          *fakes.FakeGitHub
	cfg         *config.Config
	ctx         *hctx.Context
	deliverRepo string
}

// ratingHarnessOpts is Harness's keyword arguments; a zero field is the default.
type ratingHarnessOpts struct {
	Env     map[string]string // env = None
	At      time.Time         // at = NIGHT (the zero time is Night)
	Machine []string          // machine = ()
	// PlanLanesNone is the cfg override plan_lanes=None: the committed planning lanes.
	PlanLanesNone bool
	Overrides     *pyjson.Object // **cfg_overrides that are config.json keys, in order
}

func newRatingHarness(t *testing.T, opts ...ratingHarnessOpts) *ratingHarness {
	t.Helper()
	var o ratingHarnessOpts
	if len(opts) > 0 {
		o = opts[0]
	}
	h := &ratingHarness{t: t, root: t.TempDir()}
	h.origin, h.clone = fakes.MakeOrigin(h.root)
	h.gh = fakes.NewFakeGitHub()
	env := map[string]string{"GITHUB_SERVER_URL": "file://" + filepath.Join(h.root, "remote"),
		"BOT_GITHUB_TOKEN": strings.Repeat("t", 20)}
	maps.Copy(env, o.Env)
	overrides := pyjson.New("gates", ratingGates(), "install", pyjson.New("run", "true", "timeout_minutes", 1),
		"max_review_cycles", 2)
	overrides.Update(o.Overrides)
	h.cfg = testsupport.MakeConfig(testsupport.Config{Env: env, Machine: o.Machine,
		CommittedPlanLanes: o.PlanLanesNone, Overrides: overrides})
	h.ctx = testsupport.MakeCtx(h.gh, testsupport.Ctx{Cfg: h.cfg, At: o.At})
	h.gh.BranchChecks = map[string]bool{}
	for _, name := range h.cfg.RequiredChecks {
		h.gh.BranchChecks[name] = true
	}
	h.deliverRepo = filepath.Join(h.root, "deliver")
	fakes.Git(h.root, "clone", "-q", h.origin, h.deliverRepo)
	return h
}

// outs is len(list(self.root.glob('out-*'))).
func (h *ratingHarness) outs() int {
	found, err := filepath.Glob(filepath.Join(h.root, "out-*"))
	if err != nil {
		panic(err)
	}
	return len(found)
}

// night is Harness.night(runner, **plan_args): plan, work and deliver; the plan and the model
// job's result.json ({} when it left none).
func (h *ratingHarness) night(fake *runner.FakeRunner) (*pyjson.Object, *pyjson.Object) {
	return h.nightWith(fake, false, "auto")
}

func (h *ratingHarness) nightWith(fake *runner.FakeRunner, force bool, mode string) (*pyjson.Object, *pyjson.Object) {
	planned := plan.Make(h.ctx, force, nil, mode, nil)
	out := filepath.Join(h.root, fmt.Sprintf("out-%d", h.outs()))
	if pyjson.Str(planned.Get("action")) != "none" {
		work.NewWorker(h.cfg, planned, fake, h.clone, filepath.Join(h.root, "work"), out,
			nil, nil, nil, nil, nil, nil).Run()
		deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	}
	return planned, ratingReadResult(h.t, out)
}

// ratingReadResult is json.loads((out / "result.json").read_text()), or {} when it is not there.
func ratingReadResult(t *testing.T, out string) *pyjson.Object {
	t.Helper()
	text, err := os.ReadFile(filepath.Join(out, "result.json"))
	if errors.Is(err, os.ErrNotExist) {
		return pyjson.New()
	}
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(text)
	if err != nil {
		t.Fatal(err)
	}
	return v.(*pyjson.Object)
}

// originSHA is Harness.origin_sha: the branch's sha on origin, or nil (None) when it has none.
func (h *ratingHarness) originSHA(branch string) (sha *string) {
	defer func() {
		if r := recover(); r != nil {
			if e, ok := r.(*fakes.PyError); ok && e.Type == "AssertionError" {
				sha = nil
				return
			}
			panic(r)
		}
	}()
	found := fakes.Git(h.origin, "rev-parse", "--verify", "--quiet", "refs/heads/"+branch)
	return &found
}

// ratingAsk is test_flow.ask: a comment from the operator, through the event handler.
func ratingAsk(h *ratingHarness, number int, body string, cid int) {
	events.Handle(h.ctx, "issue_comment", pyjson.New(
		"action", "created", "sender", fakes.Operator, "issue", pyjson.New("number", number),
		"comment", pyjson.New("id", cid, "body", body, "user", fakes.Operator, "author_association", "OWNER")))
}

// ---------------------------------------------------------------------------- test_rating.py

const (
	ratingEasy   = "difficulty:easy"
	ratingMedium = "difficulty:medium"
	ratingHard   = "difficulty:hard"
)

// ratingRatedPlan is rated_plan(difficulty, files=("src/game.txt",), why="one file, one test").
func ratingRatedPlan(difficulty string, files []string, why string) string {
	if files == nil {
		files = []string{"src/game.txt"}
	}
	if why == "" {
		why = "one file, one test"
	}
	rows := make([]string, 0, len(files))
	for _, path := range files {
		rows = append(rows, fmt.Sprintf("| `%s` | change | the change |", path))
	}
	return fmt.Sprintf("<!-- bot: {\"difficulty\": \"%s\", \"why\": \"%s\"} -->\n", difficulty, why) +
		"## 1. Goal\n\nRules v2.\n\n## 2. Files to touch\n\n| Path | new/change | What |\n" +
		"|---|---|---|\n" + strings.Join(rows, "\n") + "\n\n## 3. Steps\n\n1. Edit it.\n"
}

// ratingPlanner is planner(text): a runner whose planner answers `text` in five minutes.
func ratingPlanner(text string) *runner.FakeRunner {
	return ratingRunner(map[string]runner.Handler{"plan": func(request *runner.RunRequest) *runner.RunResult {
		result := runner.NewRunResult(true, text)
		result.DurationS = 300
		return result
	}})
}

// ratingDayHarness is day_harness(test, machine=("muse", "lite")): by day, no Claude account
// (they keep to the night in tests), Muse and the weak one on the machine, and the planning lane
// on.
func ratingDayHarness(t *testing.T, machine ...string) *ratingHarness {
	if machine == nil {
		machine = []string{"muse", "lite"}
	}
	return newRatingHarness(t, ratingHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: machine, At: testsupport.Day, PlanLanesNone: true})
}

// ratingDifficulties is {n for n in labels if n.startswith("difficulty:")}.
func ratingDifficulties(labels map[string]bool) map[string]bool {
	out := map[string]bool{}
	for n := range labels {
		if strings.HasPrefix(n, "difficulty:") {
			out[n] = true
		}
	}
	return out
}

// ratingPrompt is runner.calls[i].prompt.
func ratingPrompt(t *testing.T, fake *runner.FakeRunner, i int) string {
	t.Helper()
	if i >= len(fake.Calls) {
		t.Fatalf("IndexError: the runner had %d calls", len(fake.Calls))
	}
	return fake.Calls[i].Prompt
}

// ---------------------------------------------------------------------------- RatingTests

func TestRating_AMediumPlannerRatesAnUnratedItemEasyAndItIsCleared(t *testing.T) {
	h := ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Make the rules v2."})
	fake := ratingPlanner(ratingRatedPlan("easy", nil, ""))
	planned, result := h.night(fake)
	eventsEqual(t, []any{eventsDig(t, planned, "action"), eventsDig(t, planned, "provider"), eventsDig(t, result, "status")},
		[]any{"plan", "muse", "planned"})
	eventsEqual(t, eventsDig(t, planned, "rating"), pyjson.New("difficulty", "medium", "source", "", "by", ""))
	eventsIn(t, "Rate how hard it is", ratingPrompt(t, fake, 0))
	eventsIn(t, "At most 10 files changed", ratingPrompt(t, fake, 0))
	eventsIn(t, ratingEasy, h.gh.LabelNames(12))
	record := eventsDig(t, h.ctx.Store.Load(), "items", "12")
	eventsEqual(t, []any{eventsDig(t, record, "difficulty_by", "difficulty"), eventsDig(t, record, "difficulty_by", "tier")},
		[]any{"easy", "medium"})
	said := eventsLast(t, h.gh.BotComments(12))
	eventsIn(t, "Rated `difficulty:easy` by `muse`", said)
	eventsIn(t, "Cleared and queued to build", said)
	eventsNotIn(t, "<!-- bot:", issueplan.PlanOf(pyjson.Str(eventsDig(t, h.gh.Threads.Item(12), "body"))))
	// The weak one builds it from Muse's plan, which meets an easy item's floor.
	planned = plan.Make(h.ctx, false, nil, "auto", nil)
	eventsEqual(t, []any{eventsDig(t, planned, "action"), eventsDig(t, planned, "provider")}, []any{"build", "lite"})
}

// A planner that leaves out its `<!-- bot: ... -->` line must not leave the item unlabelled
// (#735): it is `difficulty:medium`, the bot's own rating, so a later planner may raise it.
func TestRating_APlanWithNoRatingLineStillLabelsTheItemMedium(t *testing.T) {
	h := ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Make the rules v2."})
	_, rest, _ := strings.Cut(ratingRatedPlan("easy", nil, ""), "\n")
	h.night(ratingPlanner(rest))
	eventsEqual(t, ratingDifficulties(h.gh.LabelNames(12)), eventsSet(ratingMedium))
	record := eventsDig(t, h.ctx.Store.Load(), "items", "12")
	eventsEqual(t, eventsDig(t, record, "difficulty_by", "difficulty"), "medium")
	eventsIn(t, "gave no rating, so it is `difficulty:medium`", eventsLast(t, h.gh.BotComments(12)))
}

// A medium plan meets a medium item's floor (#695): no strong plan comes next, it is cleared to
// build.
func TestRating_AMediumRatingByAMediumPlannerStands(t *testing.T) {
	h := ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Make the rules v2."})
	h.night(ratingPlanner(ratingRatedPlan("medium", nil, "a new ruling")))
	eventsIn(t, ratingMedium, h.gh.LabelNames(12))
	eventsNotIn(t, config.LabelNeedsPlan, h.gh.LabelNames(12))
	eventsIn(t, "Cleared and queued to build", eventsLast(t, h.gh.BotComments(12)))
}

func TestRating_ARatingIsNeverLoweredAndAPersonsLabelNeverReplaced(t *testing.T) {
	// A hard item with only a medium plan still needs a strong one: by night claude-1, strong,
	// plans it, and its easy rating does not lower it.
	h := newRatingHarness(t, ratingHarnessOpts{Machine: []string{"muse"}, PlanLanesNone: true})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, ratingHard}, Body: "Make the rules v2."})
	eventsUpdateItem(h.ctx, 12,
		"difficulty_by", pyjson.New("difficulty", "hard", "provider", "muse", "tier", "medium"),
		"planned_at", "2026-09-29T00:00:00Z", "planned_tier", "medium")
	fake := ratingPlanner(ratingRatedPlan("easy", nil, ""))
	planned, _ := h.night(fake)
	eventsEqual(t, []any{eventsDig(t, planned, "action"), eventsDig(t, planned, "provider")}, []any{"plan", "claude-1"})
	eventsIn(t, "you may rate it higher, never lower", ratingPrompt(t, fake, 0))
	eventsIn(t, ratingHard, h.gh.LabelNames(12))
	eventsIn(t, "a rating is never lowered", eventsLast(t, h.gh.BotComments(12)))
	eventsIn(t, "Cleared and queued", eventsLast(t, h.gh.BotComments(12)))
	// A person's label: the planner is told not to rate, and a rating it gives anyway is
	// ignored.
	h = ratingDayHarness(t, "muse")
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, ratingEasy}, Body: "Make the rules v2."})
	fake = ratingPlanner(ratingRatedPlan("hard", nil, ""))
	planned, _ = h.night(fake)
	eventsEqual(t, eventsDig(t, planned, "rating", "source"), "person")
	eventsIn(t, "A person set this item's difficulty", ratingPrompt(t, fake, 0))
	eventsEqual(t, ratingDifficulties(h.gh.LabelNames(12)), eventsSet(ratingEasy))
}

func TestRating_AnEasyPlanThatBreaksTheRuleIsMedium(t *testing.T) {
	h := ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Make the rules v2."})
	h.night(ratingPlanner(ratingRatedPlan("easy", []string{"src/game.txt", "SPEC.md"}, "")))
	eventsIn(t, ratingMedium, h.gh.LabelNames(12))
	said := eventsLast(t, h.gh.BotComments(12))
	eventsIn(t, "The easy rule does not hold (lines 3–5: the plan touches `SPEC.md`)", said)
	h = ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Make the rules v2."})
	files := []string{}
	for n := range 11 {
		files = append(files, fmt.Sprintf("src/f%d.txt", n))
	}
	h.night(ratingPlanner(ratingRatedPlan("easy", files, "")))
	eventsIn(t, "line 1: the plan touches 11 files, over 10", eventsLast(t, h.gh.BotComments(12)))
}

// With the planning lane off, Muse plans the unrated item in its own build run; it rates it hard,
// which Muse may not build, so the run stops after planning.
func TestRating_ABuildRunThatRatesItsItemOutOfReachStopsAfterPlanning(t *testing.T) {
	h := newRatingHarness(t, ratingHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"muse"}, At: testsupport.Day})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Make the rules v2."})
	planned, result := h.night(ratingRunner(map[string]runner.Handler{
		"plan": func(r *runner.RunRequest) *runner.RunResult {
			return runner.NewRunResult(true, ratingRatedPlan("hard", nil, "the resolution loop"))
		},
		"build": ratingBuilder(map[string]string{})}))
	eventsEqual(t, []any{eventsDig(t, planned, "action"), eventsDig(t, result, "status")}, []any{"build", "planned"})
	eventsIn(t, "rated difficulty:hard", eventsDig(t, result, "reason"))
	eventsIn(t, ratingHard, h.gh.LabelNames(12))
	eventsIn(t, config.LabelBuild, h.gh.LabelNames(12))
	eventsNone(t, h.originSHA("bot/issue-12"))
}

func TestRating_AWeakBuildersChangeThatBreaksTheRuleIsNotPushed(t *testing.T) {
	h := newRatingHarness(t, ratingHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"lite"}})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, ratingEasy}, Body: issueplan.WithPlan(
		"Make it.", "1. Edit src/game.txt.", "a person", "")})
	many := map[string]string{}
	for n := range 11 {
		many[fmt.Sprintf("src/f%d.txt", n)] = "x\n"
	}
	planned, result := h.night(ratingRunner(map[string]runner.Handler{"build": ratingBuilder(many),
		"self_check": ratingReviewer(ratingApprove)}))
	eventsEqual(t, []any{eventsDig(t, planned, "provider"), eventsDig(t, result, "status")}, []any{"lite", "built"})
	eventsEqual(t, h.gh.ListPulls("open", "", 200), []*pyjson.Object{})
	eventsIn(t, "breaks the easy rule (line 1: it changes 11 files, over 10)",
		eventsLast(t, h.gh.BotComments(12)))
	// A person labelled it easy, so the label stays, under a medium floor.
	eventsIn(t, ratingEasy, h.gh.LabelNames(12))
	eventsIn(t, config.LabelBuild, h.gh.LabelNames(12))
	eventsEqual(t, eventsDig(t, h.ctx.Store.Load(), "items", "12", "difficulty_floor"), "medium")
	eventsNotNone(t, h.originSHA("bot/issue-12"))                                             // the work is kept on its branch
	eventsEqual(t, eventsDig(t, plan.Make(h.ctx, false, nil, "auto", nil), "action"), "none") // The weak one may not take it now
}

// ---------------------------------------------------------------------------- StepUpTests

// ratingCtx is StepUpTests.ctx and CommandTests.ctx.
func ratingCtx() (*fakes.FakeGitHub, *hctx.Context) {
	gh := fakes.NewFakeGitHub()
	return gh, testsupport.MakeCtx(gh, testsupport.Ctx{Cfg: testsupport.MakeConfig()})
}

func TestRating_ThreeStrikesRaiseABotRatedItemAStep(t *testing.T) {
	gh, ctx := ratingCtx()
	gh.AddIssue(12, fakes.Issue{Labels: []string{ratingEasy}})
	eventsUpdateItem(ctx, 12, "difficulty_by", pyjson.New("difficulty", "easy", "provider", "muse"))
	eventsFalse(t, stepup.Strike(ctx, 12, "the run failed: a", ""))
	eventsFalse(t, stepup.Strike(ctx, 12, "its reviewer did not approve it: b", ""))
	eventsTrue(t, stepup.Strike(ctx, 12, "the run failed: c", ""))
	eventsIn(t, ratingMedium, gh.LabelNames(12))
	eventsNotIn(t, ratingEasy, gh.LabelNames(12))
	eventsIn(t, config.LabelBuild, gh.LabelNames(12))
	said := eventsLast(t, gh.BotComments(12))
	eventsIn(t, "This failed 3 times at `difficulty:easy`", said)
	eventsIn(t, "the run failed: a; its reviewer did not approve it: b; the run failed: c",
		said)
	record := eventsDig(t, ctx.Store.Load(), "items", "12")
	eventsEqual(t, []any{eventsDig(t, record, "strikes"), eventsDig(t, record, "difficulty_by", "difficulty")}, []any{0, "medium"})
}

func TestRating_APersonsLabelOrHardAsksAPerson(t *testing.T) {
	gh, ctx := ratingCtx()
	gh.AddIssue(12, fakes.Issue{Labels: []string{ratingEasy, config.LabelBuild}})
	for n := range 3 {
		stepup.Strike(ctx, 12, fmt.Sprintf("failure %d", n), "")
	}
	eventsEqual(t, gh.LabelNames(12), eventsSet(ratingEasy, config.LabelBlocked))
	eventsIn(t, "A person set `difficulty:easy`, so I do not change it: should it be "+
		"`difficulty:medium`?", eventsLast(t, gh.BotComments(12)))
	gh.AddIssue(13, fakes.Issue{Labels: []string{ratingHard}})
	eventsUpdateItem(ctx, 13, "difficulty_by", pyjson.New("difficulty", "hard"))
	for n := range 3 {
		stepup.Strike(ctx, 13, fmt.Sprintf("failure %d", n), "")
	}
	eventsIn(t, config.LabelBlocked, gh.LabelNames(13))
	eventsIn(t, "no stronger model", eventsLast(t, gh.BotComments(13)))
}

func TestRating_StrikesOnAPullRequestCountOnItsIssueAndRebuildIt(t *testing.T) {
	gh, ctx := ratingCtx()
	gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelPROpen}})
	gh.AddPull(40, "bot/issue-12", fakes.Pull{Body: "Closes #12", Labels: []string{config.LabelPR, config.LabelCross},
		SHA: "h1"})
	eventsUpdateItem(ctx, 40, "issue", 12, "wip", pyjson.New("sha", "w"))
	for n := range 3 {
		stepup.Strike(ctx, 40, fmt.Sprintf("rejected %d", n), "")
	}
	eventsEqual(t, eventsDig(t, gh.Threads.Item(40), "state"), "closed")
	eventsIn(t, ratingHard, gh.LabelNames(12)) // unrated, so medium, and one step up is hard
	both := map[string]bool{}
	for n := range gh.LabelNames(12) {
		if n == config.LabelBuild || n == config.LabelPROpen {
			both[n] = true
		}
	}
	eventsEqual(t, both, eventsSet(config.LabelBuild))
	eventsIn(t, [2]string{"bot/old/issue-12-20260930-0300", "h1"}, gh.CreatedBranches)
	eventsIn(t, "bot/issue-12", gh.DeletedBranches)
	eventsIn(t, "bot/wip/40", gh.DeletedBranches)
	record := eventsDig(t, ctx.Store.Load(), "items", "12")
	eventsEqual(t, []any{eventsDig(t, record, "previous_pr"), eventsDig(t, record, "previous_branch")},
		[]any{40, "bot/old/issue-12-20260930-0300"})
	eventsTrue(t, eventsDig(t, ctx.Store.Load(), "items", "40", "rebuilt"))
}

// ---------------------------------------------------------------------------- CommandTests

// ratingRunCommand is CommandTests.run_command.
func ratingRunCommand(ctx *hctx.Context, number int, line string) string {
	command := commands.Parse(line, "jgoetzmann-bot")[0]
	thread := ctx.GH.GetIssue(number)
	return events.Execute(ctx, command, thread, fakes.Operator.Copy(), nil)
}

func TestRating_RebuildClosesThePullRequestAndQueuesTheIssue(t *testing.T) {
	gh, ctx := ratingCtx()
	gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelPROpen}})
	gh.AddPull(40, "bot/issue-12", fakes.Pull{Body: "Closes #12", Labels: []string{config.LabelPR}, SHA: "h1"})
	reply := ratingRunCommand(ctx, 40, "/harness rebuild")
	eventsEqual(t, reply, "Closed #40 and queued #12 to build again from `main`.")
	eventsEqual(t, eventsDig(t, gh.Threads.Item(40), "state"), "closed")
	eventsIn(t, config.LabelBuild, gh.LabelNames(12))
	eventsIn(t, "@jgoetzmann asked for it", eventsDig(t, ctx.Store.Load(), "items", "12", "previous_why"))
	// The same from the issue; and an issue with no pull request has nothing to rebuild.
	gh.AddIssue(13)
	eventsIn(t, "no open pull request of mine to rebuild",
		ratingRunCommand(ctx, 13, "@jgoetzmann-bot rebuild"))
	// The new build is told about the old pull request.
	planned := plan.Make(ctx, false, nil, "auto", nil)
	eventsEqual(t, []any{eventsDig(t, planned, "number"), eventsDig(t, planned, "previous_pr")}, []any{12, 40})
}

func TestRating_ReviewQueuesAReviewRunAtATierAndNoRevision(t *testing.T) {
	gh, ctx := ratingCtx()
	gh.AddPull(40, "bot/issue-12", fakes.Pull{Body: "Closes #12", Labels: []string{config.LabelPR}, SHA: "h1"})
	reply := ratingRunCommand(ctx, 40, "/harness review strong check the replay")
	eventsIn(t, "Queued a review run of #40, by a strong model or stronger", reply)
	eventsEqual(t, gh.LabelNames(40), eventsSet(config.LabelPR, config.LabelCross))
	record := eventsDig(t, ctx.Store.Load(), "items", "40")
	eventsEqual(t, []any{eventsDig(t, record, "review_floor"), eventsDig(t, record, "review_notes")},
		[]any{"strong", "check the replay"})
	// By night a strong model is free: it takes the review, with the notes.
	planned := plan.Make(ctx, false, nil, "auto", nil)
	review := ratingSeats(t, planned)["review"]
	eventsEqual(t, []any{eventsDig(t, planned, "action"), pyjson.List(review)[2]}, []any{"review", "strong"})
	eventsEqual(t, eventsDig(t, planned, "review_notes"), "check the replay")
	// By day only medium models are free (in tests): a strong-floor review waits.
	gh2 := fakes.NewFakeGitHub()
	ctx2 := testsupport.MakeCtx(gh2, testsupport.Ctx{At: testsupport.Day, Cfg: testsupport.MakeConfig(testsupport.Config{Machine: []string{"muse"}})})
	gh2.AddPull(40, "bot/issue-12", fakes.Pull{Body: "Closes #12", Labels: []string{config.LabelPR}, SHA: "h1"})
	ratingRunCommand(ctx2, 40, "/harness review strong")
	eventsEqual(t, eventsDig(t, plan.Make(ctx2, false, nil, "auto", nil), "action"), "none")
	ratingRunCommand(ctx2, 40, "/harness review medium")
	eventsEqual(t, eventsDig(t, plan.Make(ctx2, false, nil, "auto", nil), "provider"), "muse")
	// On an issue it says where it works.
	gh.AddIssue(13)
	eventsIn(t, "one of my pull requests", ratingRunCommand(ctx, 13, "/harness review"))
}
