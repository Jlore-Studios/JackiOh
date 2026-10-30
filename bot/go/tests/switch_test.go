// Each Claude account runs Opus and Sonnet both, and its run switches between them as the work
// needs: the builder may ask for the other model for its next pass, two rounds the reviewer sends
// back on Sonnet move the builder to Opus, and a planner's rating moves it to the model that fits,
// never under the item's difficulty. The reviewer stays as assigned: the review rule counts its
// tier. (bot/tests/test_switch.py)
package tests

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

var (
	switchMedium = "difficulty:medium"
	switchHard   = "difficulty:hard"
)

// switchDone is done(**extra): a builder's report whose header carries `extra` (key, value, ...).
func switchDone(extra ...string) string {
	header := pyjson.New("status", "done", "title", "Make the rules v2")
	for i := 0; i+1 < len(extra); i += 2 {
		header.Set(extra[i], extra[i+1])
	}
	return "<!-- bot: " + pyjson.Dumps(header) + " -->\n## What changed\nRules v2."
}

// switchModels is the model of each of `r`'s calls in `role`.
func switchModels(r *runner.FakeRunner, role string) []string {
	out := []string{}
	for _, request := range r.Calls {
		if request.Role == role {
			out = append(out, request.Model)
		}
	}
	return out
}

// switchRated is a planner that rates the item `difficulty`.
func switchRated(difficulty string) runner.Handler {
	text := fmt.Sprintf("<!-- bot: {\"difficulty\": \"%s\", \"why\": \"it reads the engine\"} -->\n", difficulty) +
		"1. **Goal.** Do what the issue asks.\n2. **Steps.** Change it, test it."
	return func(request *runner.RunRequest) *runner.RunResult { return runner.NewRunResult(true, text) }
}

// switchHarness is SwitchTests.harness.
func switchHarness(t *testing.T) *flowHarness {
	return newFlowHarness(t, flowHarnessOpts{Env: crossAll,
		Overrides: pyjson.New("max_review_cycles", 4)})
}

// switchPrompt is next(r.prompt for r in runner.calls if r.role == role).
func switchPrompt(t *testing.T, r *runner.FakeRunner, role string) string {
	t.Helper()
	for _, request := range r.Calls {
		if request.Role == role {
			return request.Prompt
		}
	}
	t.Fatalf("no %s call", role)
	return ""
}

// switchToOpus is planned["seats"]["build"] = {**planned["seats"]["build"], "model": "opus",
// "tier": "strong"}.
func switchToOpus(planned *pyjson.Object) {
	build := planned.Obj("seats").Obj("build").Copy()
	build.Set("model", "opus")
	build.Set("tier", "strong")
	planned.Obj("seats").Set("build", build)
}

func switchResult(t *testing.T, out string) *pyjson.Object {
	t.Helper()
	text, err := os.ReadFile(filepath.Join(out, "result.json"))
	if err != nil {
		t.Fatal(err)
	}
	return loadsObject(t, string(text))
}

// ---------------------------------------------------------------------------- ReportTests

func TestSwitch_TheReportHeaderCarriesTheModelAskedFor(t *testing.T) {
	assertEqual(t, verdicts.BuildReport_(switchDone("next_model", "Opus")).NextModel, "opus")
	assertEqual(t, verdicts.BuildReport_(switchDone()).NextModel, "")
}

// ---------------------------------------------------------------------------- SwitchTests

func TestSwitch_TheBuilderAsksForOpusAndItsNextPassRunsOnIt(t *testing.T) {
	h := switchHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, switchMedium}})
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"build": {workBuilder(map[string]string{"src/game.txt": "v2\n"}, switchDone("next_model", "opus"))},
		"fix":   {workBuilder(map[string]string{"src/game.txt": "v3\n"})},
		"review": {workReviewer(workChanges("v2 misses a rule")),
			workReviewer(workApprove)}})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Get("provider"), planned.Obj("seats").Obj("build").Get("model")},
		[]any{"claude-3", "sonnet"})
	assertEqual(t, result.Get("status"), "approved")
	assertEqual(t, [][]string{switchModels(r, "build"), switchModels(r, "fix")}, [][]string{{"sonnet"}, {"opus"}})
	assertEqual(t, switchModels(r, "review"), []string{"opus", "opus"})
	assertEqual(t, result.Get("switches"), []any{pyjson.New("n", 1, "from", "sonnet", "to", "opus",
		"why", "the builder asked for it")})
	cycles := result.List("cycles")
	assertEqual(t, pyjson.Obj(cycles[0]).Obj("builder").Get("next_model"), "opus")
	built := []any{}
	for _, c := range cycles {
		built = append(built, pyjson.Obj(c).Obj("builder").Get("model"))
	}
	assertEqual(t, built, []any{"sonnet", "opus"})
	said := h.gh.BotComments(12)
	assertIn(t, "after round 1 it switched to `opus`: the builder asked for it", said[len(said)-1])
}

func TestSwitch_RoundsSentBackOnSonnetMoveTheBuilderToOpus(t *testing.T) {
	h := switchHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, switchMedium}})
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"build": {workBuilder(map[string]string{"src/game.txt": "v2\n"})},
		"fix": {workBuilder(map[string]string{"src/game.txt": "v3\n"}),
			workBuilder(map[string]string{"src/game.txt": "v4\n"})},
		"review": {workReviewer(workChanges("not yet")), workReviewer(workChanges("still no")),
			workReviewer(workApprove)}})
	_, result := h.night(r)
	assertEqual(t, work.SwitchUpAfter, 2)
	assertEqual(t, result.Get("status"), "approved")
	assertEqual(t, [][]string{switchModels(r, "build"), switchModels(r, "fix")},
		[][]string{{"sonnet"}, {"sonnet", "opus"}})
	assertEqual(t, result.Get("switches"), []any{pyjson.New(
		"n", 2, "from", "sonnet", "to", "opus",
		"why", "the reviewer sent back 2 rounds in a row on `sonnet`")})
}

func TestSwitch_AHardItemKeepsOpusWhateverTheBuilderAsks(t *testing.T) {
	h := switchHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, switchHard}})
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"build":  {workBuilder(map[string]string{"src/game.txt": "v2\n"}, switchDone("next_model", "sonnet"))},
		"fix":    {workBuilder(map[string]string{"src/game.txt": "v3\n"})},
		"review": {workReviewer(workChanges("not yet")), workReviewer(workApprove)}})
	_, result := h.night(r)
	assertEqual(t, [][]string{switchModels(r, "build"), switchModels(r, "fix")}, [][]string{{"opus"}, {"opus"}})
	assertEqual(t, result.Get("switches"), []any{})
	assertIn(t, "keeps its builder on `opus`", switchPrompt(t, r, "build"))
}

func TestSwitch_OpusOnAMediumItemMayHandThePlainWorkToSonnet(t *testing.T) {
	h := switchHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, switchMedium}})
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	switchToOpus(planned)
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"build":  {workBuilder(map[string]string{"src/game.txt": "v2\n"}, switchDone("next_model", "sonnet"))},
		"fix":    {workBuilder(map[string]string{"src/game.txt": "v3\n"})},
		"review": {workReviewer(workChanges("a typo")), workReviewer(workApprove)}})
	out := filepath.Join(h.root, "out-opus")
	work.NewWorker(h.cfg, planned, r, h.clone, filepath.Join(h.root, "work"), out, nil, nil, nil, nil, nil, nil).Run()
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	result := switchResult(t, out)
	assertEqual(t, [][]string{switchModels(r, "build"), switchModels(r, "fix")}, [][]string{{"opus"}, {"sonnet"}})
	assertEqual(t, pyjson.Obj(result.List("switches")[0]).Get("to"), "sonnet")
}

// The builder's ask is taken once its round is judged, so a round Opus built and the reviewer sent
// back counts against Opus, not Sonnet.
func TestSwitch_SonnetGetsItsTwoTriesAfterOpusHandsItTheWork(t *testing.T) {
	h := switchHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, switchMedium}})
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	switchToOpus(planned)
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"build": {workBuilder(map[string]string{"src/game.txt": "v2\n"}, switchDone("next_model", "sonnet"))},
		"fix": {workBuilder(map[string]string{"src/game.txt": "v3\n"}),
			workBuilder(map[string]string{"src/game.txt": "v4\n"}),
			workBuilder(map[string]string{"src/game.txt": "v5\n"})},
		"review": {workReviewer(workChanges("one")), workReviewer(workChanges("two")),
			workReviewer(workChanges("three")), workReviewer(workApprove)}})
	out := filepath.Join(h.root, "out-tries")
	work.NewWorker(h.cfg, planned, r, h.clone, filepath.Join(h.root, "work"), out, nil, nil, nil, nil, nil, nil).Run()
	result := switchResult(t, out)
	assertEqual(t, [][]string{switchModels(r, "build"), switchModels(r, "fix")},
		[][]string{{"opus"}, {"sonnet", "sonnet", "opus"}})
	moves := []any{}
	for _, m := range result.List("switches") {
		moves = append(moves, []any{pyjson.Obj(m).Get("to"), pyjson.Obj(m).Get("why")})
	}
	assertEqual(t, moves, []any{
		[]any{"sonnet", "the builder asked for it"},
		[]any{"opus", "the reviewer sent back 2 rounds in a row on `sonnet`"}})
}

// The bot rated it medium (a step up, say); the run's planner rates it hard. The run moves its
// builder to Opus rather than building a hard item on Sonnet.
func TestSwitch_AnItemTheBotRatedAndItsPlannerRatesHarderIsBuiltOnOpus(t *testing.T) {
	h := switchHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, switchMedium}})
	h.ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, 12).Update(pyjson.New("difficulty_by", pyjson.New("difficulty", "medium", "provider", "step-up")))
		return nil
	}, "state")
	r := runner.NewFakeRunner(map[string][]runner.Handler{"plan": {switchRated("hard")},
		"build":  {workBuilder(map[string]string{"src/game.txt": "v2\n"})},
		"review": {workReviewer(workApprove)}})
	planned, result := h.night(r)
	assertEqual(t, []any{planned.Obj("seats").Obj("build").Get("model"), planned.Obj("rating").Get("source")},
		[]any{"sonnet", "bot"})
	assertEqual(t, switchModels(r, "build"), []string{"opus"})
	assertEqual(t, pyjson.Obj(result.List("switches")[0]).Get("to"), "opus")
}

// An unrated item starts on Sonnet; its planner rates it hard, so the same run builds it on the
// account's Opus rather than queueing it again for a strong builder.
func TestSwitch_ThePlannersRatingMovesTheBuilderInsteadOfSendingItBack(t *testing.T) {
	h := switchHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	r := runner.NewFakeRunner(map[string][]runner.Handler{"plan": {switchRated("hard")},
		"build":  {workBuilder(map[string]string{"src/game.txt": "v2\n"})},
		"review": {workReviewer(workApprove)}})
	planned, result := h.night(r)
	assertEqual(t, planned.Obj("seats").Obj("build").Get("model"), "sonnet")
	assertEqual(t, result.Get("status"), "approved")
	assertEqual(t, switchModels(r, "build"), []string{"opus"})
	assertEqual(t, result.Get("switches"), []any{pyjson.New("n", 0, "from", "sonnet", "to", "opus",
		"why", "its planner rated it difficulty:hard")})
	latest := 0
	for n, thread := range h.gh.Threads.All() {
		if thread.Has("pull_request") && n > latest {
			latest = n
		}
	}
	labels := h.gh.LabelNames(12)
	for name := range h.gh.LabelNames(latest) {
		labels[name] = true
	}
	assertIn(t, switchHard, labels)
	assertIn(t, "before round 1 it switched to `opus`", strings.Join(h.gh.BotComments(12), "\n"))
}

func TestSwitch_TheBuilderIsToldItMaySwitch(t *testing.T) {
	h := switchHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, switchMedium}})
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"build":  {workBuilder(map[string]string{"src/game.txt": "v2\n"})},
		"review": {workReviewer(workApprove)}})
	h.night(r)
	buildPrompt := switchPrompt(t, r, "build")
	assertIn(t, "## Your model", buildPrompt)
	assertIn(t, "`opus` (strong) and `sonnet` (medium)", buildPrompt)
	assertIn(t, "\"next_model\": \"opus\"", buildPrompt)
	// Its reviewer reads no such note: it reviews, it does not choose.
	reviewPrompt := switchPrompt(t, r, "review")
	assertNotIn(t, "## Your model", reviewPrompt)
}

func TestSwitch_ALaneWithOneModelNeverSwitches(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"muse"}, At: testsupport.Day})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}}) // unrated, so a medium model may plan it
	r := runner.NewFakeRunner(map[string][]runner.Handler{
		"build":  {workBuilder(map[string]string{"src/game.txt": "v2\n"}, switchDone("next_model", "opus"))},
		"fix":    {workBuilder(map[string]string{"src/game.txt": "v3\n"})},
		"review": {workReviewer(workChanges("not yet")), workReviewer(workApprove)}})
	planned, result := h.night(r)
	assertEqual(t, planned.Get("provider"), "muse")
	used := map[string]bool{}
	for _, m := range append(switchModels(r, "build"), switchModels(r, "fix")...) {
		used[m] = true
	}
	assertEqual(t, used, map[string]bool{"muse-spark-1.3-contributor": true})
	assertEqual(t, result.Get("switches"), []any{})
	assertNotIn(t, "## Your model", switchPrompt(t, r, "build"))
}
