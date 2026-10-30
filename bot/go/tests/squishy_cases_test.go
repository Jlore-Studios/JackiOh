// squishy_cases_test.go is bot/tests/squishy_cases.py: Squishy's own cases (#60), in a process that
// is Squishy from the start.
//
// squishy_test.go's TestSquishy_SquishysOwnCasesPassAsSquishy runs these with
// `HARNESS_HOME=.squishy` (the labels, the slash command and the branches are fixed when a process
// starts, so the night bot's process cannot check them). Each skips itself in any other process,
// as the Python's setUpModule does.
package tests

import (
	"fmt"
	"path/filepath"
	"sort"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/commands"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/issueplan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/journal"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stats"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/sweep"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// ENV
var squishyCasesEnv = map[string]string{"HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN_SQUISHY"}

// DONE
const squishyCasesDone = "<!-- bot: {\"status\": \"done\", \"title\": \"Rules v2, in one shot\"} -->\n## What changed\nAll."

// APPROVE
const squishyCasesApprove = "<!-- review: {\"verdict\": \"approve\", \"findings\": []} -->\nFine."

// GATES
func squishyCasesGates() []any {
	return []any{pyjson.New("name", "rules exist", "run", "test -f src/game.txt", "timeout_minutes", 1)}
}

// TREE
func squishyCasesTree() *pyjson.Object {
	return pyjson.New("done", false, "summary", "Engine first, then the client.", "issues", []any{
		pyjson.New("key", "engine", "title", "Part 1: the engine", "body", "Rule it in `engine.ts`.",
			"plan", "1. Write the rule. 2. Test it.", "difficulty", "hard", "labels", []any{"patch", "human"},
			"blocked_by", []any{}),
		pyjson.New("key", "client", "title", "Part 2: the client", "body", "Draw it.", "plan", "1. Draw.",
			"difficulty", "easy", "labels", []any{"patch", "squishy:build"}, "blocked_by", []any{"engine"})})
}

// squishyCasesModule is setUpModule: these run only as Squishy.
func squishyCasesModule(t *testing.T) {
	t.Helper()
	if config.Home != ".squishy" {
		t.Skip("run as Squishy (HARNESS_HOME=.squishy) by squishy_test.go")
	}
}

// squishyCasesAnswer is answer(tree).
func squishyCasesAnswer(tree *pyjson.Object) string {
	return "How I cut it.\n\n```json\n" + pyjson.Dumps(tree) + "\n```"
}

// squishyCasesWorld is World: a repository, a fake GitHub and Squishy's three jobs (plan, work,
// deliver).
type squishyCasesWorld struct {
	t           *testing.T
	root        string
	origin      string
	clone       string
	gh          *fakes.FakeGitHub
	cfg         *config.Config
	ctx         *hctx.Context
	deliverRepo string
}

// newSquishyCasesWorld is World(at=NIGHT); a zero `at` is NIGHT.
func newSquishyCasesWorld(t *testing.T, at ...time.Time) *squishyCasesWorld {
	t.Helper()
	when := testsupport.Night
	if len(at) > 0 && !at[0].IsZero() {
		when = at[0]
	}
	w := &squishyCasesWorld{t: t, root: t.TempDir(), gh: fakes.NewFakeGitHub()}
	w.origin, w.clone = fakes.MakeOrigin(w.root)
	env := map[string]string{}
	for k, v := range squishyCasesEnv {
		env[k] = v
	}
	env["GITHUB_SERVER_URL"] = "file://" + filepath.Join(w.root, "remote")
	env["BOT_GITHUB_TOKEN"] = strings.Repeat("t", 20)
	w.cfg = testsupport.MakeConfig(testsupport.Config{Env: env, Overrides: pyjson.New("gates", squishyCasesGates(),
		"install", pyjson.New("run", "true", "timeout_minutes", 1), "max_review_cycles", 2)})
	w.ctx = testsupport.MakeCtx(w.gh, testsupport.Ctx{Cfg: w.cfg, At: when})
	w.gh.BranchChecks = setOf(w.cfg.RequiredChecks...)
	w.deliverRepo = filepath.Join(w.root, "deliver")
	fakes.Git(w.root, "clone", "-q", w.origin, w.deliverRepo)
	for _, name := range []string{"patch", "human", "priority:high"} {
		w.gh.EnsureLabel(name, "ededed", name)
	}
	return w
}

// ask is World.ask: the operator's commands in `body`, on `number`.
func (w *squishyCasesWorld) ask(number int, body string) []string {
	return events.RunCommands(w.ctx, commands.Parse(body, w.cfg.BotLogin), w.gh.GetIssue(number), fakes.Operator, 3, nil)
}

// run is World.run: plan, work and deliver one run.
func (w *squishyCasesWorld) run(r *runner.FakeRunner) (*pyjson.Object, *pyjson.Object) {
	w.t.Helper()
	planned := plan.Make(w.ctx, false, nil, "auto", nil)
	out := filepath.Join(w.root, fmt.Sprintf("out-%d", flowOuts(w.root)))
	if planned.Str("action") != "none" {
		work.NewWorker(w.cfg, planned, r, w.clone, filepath.Join(w.root, "work"), out, nil, nil, nil, nil, nil, nil).Run()
		deliver.NewDeliverer(w.ctx, planned, out, w.deliverRepo, "", nil, "").Run()
	}
	result := pyjson.New()
	if exists(filepath.Join(out, "result.json")) {
		result = loadJSON(w.t, filepath.Join(out, "result.json"))
	}
	return planned, result
}

// ------------------------------------------------------------------ IdentityCases

func TestSquishyCases_EverythingIsSquishys(t *testing.T) {
	squishyCasesModule(t)
	assertEqual(t, []string{config.LabelBuild, config.Slash, config.StateBranch},
		[]string{"squishy:build", "/squishy", "squishy-state"})
	assertEqual(t, config.NightWorkflow, "squishy-run.yml")
	assertEqual(t, journal.Branch, "squishy-journal")
	assertEqual(t, queue.BranchForIssue(5), "squishy/issue-5")
	assertEqual(t, queue.WipBranch(5), "squishy/wip/5")
	assertEqual(t, config.CommitPrefix, "squishy")
	assertEqual(t, setOf(config.ModeLabels.Keys()...), setOf("squishy:oneshot", "squishy:split",
		"squishy:split-bot", "squishy:fullsend"))
}

func TestSquishyCases_ItsLabelsAndNoSuggestions(t *testing.T) {
	squishyCasesModule(t)
	labels := config.Labels
	description := func(name string) string { return pyjson.Str(pyjson.List(labels.Get(name))[1]) }
	assertNotIn(t, "squishy:suggestion", labels)
	assertNotIn(t, "squishy:approved", labels)
	for _, name := range []string{"squishy:build", "squishy:oneshot", "squishy:split", "squishy:split-bot",
		"squishy:fullsend", config.LabelTree, "human"} {
		assertIn(t, name, labels)
	}
	assertIn(t, "Squishy", description("squishy:working"))
	assertIn(t, "Queued for Squishy to split into parts", description("squishy:fullsend"))
	assertIn(t, "whose sub-issues Squishy made", description(config.LabelTree))
	assertIn(t, "that the night bot builds", description("squishy:split-bot"))
	all := true
	for _, name := range labels.Keys() {
		all = all && len([]rune(description(name))) <= config.LabelDescriptionMax
	}
	assertTrue(t, all)
	assertFalse(t, testsupport.MakeConfig(testsupport.Config{Env: squishyCasesEnv}).SuggestionsEnabled)
}

func TestSquishyCases_ItsCommands(t *testing.T) {
	squishyCasesModule(t)
	bot := "squishy-squooby"
	read := [][2]string{}
	for _, c := range commands.Parse("/squishy oneshot\n/squishy split bot\n@squishy-squooby split", bot) {
		read = append(read, [2]string{c.Verb, c.Args})
	}
	assertEqual(t, read, [][2]string{{"oneshot", ""}, {"split", "bot"}, {"split", ""}})
	assertEqual(t, commands.Parse("/harness build", bot), []*commands.Command{})
	text := commands.HelpText(bot, "")
	assertIn(t, "`oneshot [notes]`", text)
	assertIn(t, "`split [bot] [notes]`", text)
	assertIn(t, "`fullsend [notes]`", text)
	assertIn(t, "hand it back, then `/squishy build`", text) // #802: its own slash
	assertIn(t, "`build`, `oneshot`, `split`, `fullsend` or `revise`", text)
	assertNotIn(t, "`suggest`", text)
	assertNotIn(t, "`approve`", text) // it makes no suggestions to vote on (#796)
	assertIn(t, "I do not know `approve`", commands.HelpText(bot, "approve"))
	assertIn(t, "`/squishy help`", commands.Pointer_(bot))
}

// ------------------------------------------------------------------ QueueCases

func TestSquishyCases_OneshotAndSplitQueueInTheirMode(t *testing.T) {
	squishyCasesModule(t)
	world := newSquishyCasesWorld(t)
	gh := world.gh
	gh.AddIssue(3)
	reply := world.ask(3, "/squishy oneshot")[0]
	assertIn(t, "build it in one run with fullsend", reply)
	assertEqual(t, gh.LabelNames(3), setOf("squishy:build", "squishy:oneshot"))
	world.ask(3, "/squishy split bot")
	assertEqual(t, gh.LabelNames(3), setOf("squishy:build", "squishy:split-bot"))
	world.ask(3, "/squishy build")
	assertEqual(t, gh.LabelNames(3), setOf("squishy:build"))
}

func TestSquishyCases_AModeLabelQueuesItInThatMode(t *testing.T) {
	squishyCasesModule(t)
	world := newSquishyCasesWorld(t)
	gh := world.gh
	gh.AddIssue(4, fakes.Issue{Labels: []string{"squishy:split"}})
	events.OnIssueChange(world.ctx, pyjson.New("action", "labeled", "issue", gh.GetIssue(4),
		"label", pyjson.New("name", "squishy:split"), "sender", fakes.Operator), false)
	assertEqual(t, gh.LabelNames(4), setOf("squishy:build", "squishy:split"))
	found := queue.Candidates(world.ctx, world.ctx.Store.Load(), nil, nil)
	type row struct {
		Number  int
		Mode    string
		Planned bool
	}
	got := []row{}
	for _, c := range found {
		got = append(got, row{c.Number, c.Mode, c.Planned})
	}
	assertEqual(t, got, []row{{4, "split", true}})
}

func TestSquishyCases_TheNightBotsIssuesAreLeftAlone(t *testing.T) {
	squishyCasesModule(t)
	world := newSquishyCasesWorld(t)
	gh := world.gh
	gh.AddIssue(5, fakes.Issue{Labels: []string{"bot:build"}})
	reply := world.ask(5, "/squishy oneshot")[0]
	assertIn(t, "is the night bot's (@jgoetzmann-bot)", reply)
	assertEqual(t, gh.LabelNames(5), setOf("bot:build"))
	gh.AddIssue(6)
	gh.AddAssignees(6, []string{"jgoetzmann-bot"})
	assertIn(t, "the night bot's", world.ask(6, "/squishy build")[0])
}

func TestSquishyCases_NoSuggestions(t *testing.T) {
	squishyCasesModule(t)
	world := newSquishyCasesWorld(t)
	world.gh.AddIssue(7)
	assertEqual(t, world.ask(7, "/squishy suggest"), []string{"Squishy makes no suggestions."})
}

func TestSquishyCases_AModeNeverRunsOverAnOpenPullRequest(t *testing.T) {
	squishyCasesModule(t)
	world := newSquishyCasesWorld(t)
	world.gh.AddIssue(8, fakes.Issue{Labels: []string{config.LabelPROpen}})
	world.gh.AddPull(9, "squishy/issue-8")
	assertIn(t, "`/squishy rebuild`", world.ask(8, "/squishy oneshot")[0])
}

// ------------------------------------------------------------------ OneShotCases

func TestSquishyCases_TheBuilderIsToldToRunFullsendAndItsChangeShipsLikeAnyBuild(t *testing.T) {
	squishyCasesModule(t)
	world := newSquishyCasesWorld(t)
	world.gh.AddIssue(12, fakes.Issue{Title: "Rules v2", Body: "Make the rules v2.",
		Labels: []string{"squishy:build", "squishy:oneshot"}})
	prompts := []string{}
	build := func(request *runner.RunRequest) *runner.RunResult {
		prompts = append(prompts, request.Prompt)
		writeText(t, filepath.Join(request.Cwd, "src", "game.txt"), "rules v2\n")
		return runner.NewRunResult(true, squishyCasesDone)
	}
	planned, result := world.run(workFake(map[string]runner.Handler{"build": build,
		"review": func(r *runner.RunRequest) *runner.RunResult { return runner.NewRunResult(true, squishyCasesApprove) }}))
	assertEqual(t, []any{planned.Get("action"), planned.Get("mode")}, []any{"build", "oneshot"})
	assertEqual(t, planned.Get("branch"), "squishy/issue-12")
	planSeat, ok := planned.Obj("seats").Lookup("plan") // planned["seats"]["plan"]: a KeyError when absent
	assertTrue(t, ok, "KeyError: 'plan'")
	assertEqual(t, planSeat, nil) // it writes its own spec: no planner first
	assertEqual(t, planned.Get("provider"), "claude-squishy")
	assertIn(t, "one-shot build", world.gh.BotComments(12)[0])
	if len(prompts) == 0 {
		t.Fatal("IndexError: the builder was never called")
	}
	prompt := prompts[0]
	assertIn(t, ".claude/skills/fullsend/SKILL.md", prompt)
	assertIn(t, "git worktree add -b squishy/oneshot-12/<slice> .fullsend/worktrees/<slice>", prompt)
	assertIn(t, "git commit -m\n   \"fullsend: phase N for #12\"", prompt)
	assertEqual(t, result.Get("status"), "approved")
	exclude := filepath.Join(fakes.Git(world.clone, "rev-parse", "--path-format=absolute", "--git-common-dir"),
		"info", "exclude")
	assertIn(t, "/.fullsend/", readText(t, exclude))
	pulls := world.gh.ListPulls("open", "squishy/issue-12", 200)
	assertEqual(t, len(pulls), 1)
	if len(pulls) > 0 {
		assertIn(t, "squishy:pr", world.gh.LabelNames(pulls[0].Int("number")))
	}
}

// ------------------------------------------------------------------ SplitCases

// squishyCasesSplit is SplitCases.split(mode, tree=TREE): a big issue in `mode`, and the planner's
// answer `tree`.
func squishyCasesSplit(t *testing.T, mode string, tree ...*pyjson.Object) (*squishyCasesWorld, *pyjson.Object) {
	t.Helper()
	answer := squishyCasesTree()
	if len(tree) > 0 {
		answer = tree[0]
	}
	world := newSquishyCasesWorld(t)
	world.gh.AddIssue(40, fakes.Issue{Title: "A big project", Body: "Do it all.\n\nDone when: it works.",
		Labels: []string{"squishy:build", "squishy:" + mode, "priority:high"}})
	_, result := world.run(workFake(map[string]runner.Handler{"plan": func(r *runner.RunRequest) *runner.RunResult {
		return runner.NewRunResult(true, squishyCasesAnswer(answer))
	}}))
	return world, result
}

func TestSquishyCases_SplitOpensLinkedSubIssuesSquishyBuilds(t *testing.T) {
	squishyCasesModule(t)
	world, result := squishyCasesSplit(t, "split")
	gh := world.gh
	assertEqual(t, result.Get("status"), "split")
	engine, client := 41, 42 // opened in build order
	assertEqual(t, gh.GetIssue(engine).Get("title"), "Part 1: the engine")
	// Its type labels as far as the repository has them, never a reserved one, then the parent's
	// priority, its difficulty and Squishy's queue label.
	assertEqual(t, gh.LabelNames(engine), setOf("patch", "difficulty:hard", "priority:high", "squishy:build"))
	assertEqual(t, gh.LabelNames(client), setOf("patch", "difficulty:easy", "priority:high", "squishy:build"))
	body := gh.GetIssue(client).Str("body")
	assertIn(t, fmt.Sprintf("Blocked by #%d.", engine), body)
	assertIn(t, "Part of #40, split by Squishy", body)
	assertIn(t, issueplan.Start, body)
	assertEqual(t, gh.BlockedBy(client)[0].Get("number"), engine)
	children := []any{}
	for _, c := range gh.ListSubIssues(40) {
		children = append(children, c.Get("number"))
	}
	assertEqual(t, children, []any{engine, client})
	assertEqual(t, gh.LabelNames(40), setOf("priority:high", config.LabelTree))
	assertIn(t, fmt.Sprintf("- [ ] #%d (after #%d)", client, engine), lastOf(t, gh.BotComments(40)))
	record := world.ctx.Store.Load().Obj("items").Obj("40")
	assertEqual(t, record.Obj("tree").Get("children"), []any{engine, client})
	// Both are queued for Squishy, the engine first; each counts as planned.
	found := queue.Candidates(world.ctx, world.ctx.Store.Load(), nil, nil)
	got := [][2]any{}
	for _, c := range found {
		got = append(got, [2]any{c.Number, c.Planned})
	}
	assertEqual(t, got, [][2]any{{engine, true}})
}

func TestSquishyCases_SplitBotQueuesThemForTheNightBot(t *testing.T) {
	squishyCasesModule(t)
	world, _ := squishyCasesSplit(t, "split-bot")
	assertEqual(t, world.gh.LabelNames(41), setOf("patch", "difficulty:hard", "priority:high", "bot:build"))
	assertIn(t, "queued for the night bot (@jgoetzmann-bot)", lastOf(t, world.gh.BotComments(40)))
	assertEqual(t, queue.Candidates(world.ctx, world.ctx.Store.Load(), nil, nil), []*queue.Candidate{})
}

func TestSquishyCases_AnAnswerItCannotUseOpensNothing(t *testing.T) {
	squishyCasesModule(t)
	world, result := squishyCasesSplit(t, "split", pyjson.New("done", false, "issues", []any{pyjson.New("key", "a")}))
	assertEqual(t, result.Get("status"), "failed")
	numbers := world.gh.Threads.Keys()
	sort.Ints(numbers)
	assertEqual(t, numbers, []int{40})
	assertIn(t, "could not be used", lastOf(t, world.gh.BotComments(40)))
}

func TestSquishyCases_TheCloseOutClosesATreeThatIsDone(t *testing.T) {
	squishyCasesModule(t)
	world, _ := squishyCasesSplit(t, "split")
	for _, child := range []int{41, 42} {
		world.gh.UpdateIssue(child, pyjson.New("state", "closed"))
	}
	world.ctx.Store.Update(func(s *pyjson.Object) any {
		s.Set("last_sweep", pyjson.New("since", "2026-09-29T00:00:00Z"))
		return nil
	}, "state")
	notes := sweep.Trees(world.ctx, testsupport.Night)
	assertEqual(t, notes, []string{"#40: its sub-issues have all closed; queued its close-out"})
	assertEqual(t, world.gh.LabelNames(40), setOf("priority:high", config.LabelTree, "squishy:build",
		"squishy:split"))
	prompts := []string{}
	closeOut := func(request *runner.RunRequest) *runner.RunResult {
		prompts = append(prompts, request.Prompt)
		return runner.NewRunResult(true, `<!-- split: {"done": true, "summary": "It works on main."} -->`)
	}
	_, result := world.run(workFake(map[string]runner.Handler{"plan": closeOut}))
	assertEqual(t, result.Get("status"), "split")
	if len(prompts) == 0 {
		t.Fatal("IndexError: the planner was never called")
	}
	assertIn(t, "#41 [closed] Part 1: the engine", prompts[0])
	assertEqual(t, world.gh.GetIssue(40).Get("state"), "closed")
	assertNotIn(t, config.LabelTree, world.gh.LabelNames(40))
}

func TestSquishyCases_FullsendLandsOnSquishysBranch(t *testing.T) {
	squishyCasesModule(t)
	world, result := squishyCasesSplit(t, "fullsend")
	assertEqual(t, result.Get("status"), "split")
	onto := "squishy/issue-40"
	assertEqual(t, fakes.Git(world.origin, "rev-parse", "refs/heads/"+onto),
		fakes.Git(world.origin, "rev-parse", "refs/heads/main"))
	for _, child := range []int{41, 42} {
		assertIn(t, "squishy:build", world.gh.LabelNames(child))
		record := world.ctx.Store.Load().Obj("items").Obj(fmt.Sprint(child))
		assertEqual(t, []any{record.Get("onto"), record.Get("part_of")}, []any{onto, 40})
	}
	assertEqual(t, world.gh.LabelNames(40), setOf("priority:high", config.LabelTree))
	assertIn(t, "This is a fullsend tree: each part lands on `"+onto+"`", lastOf(t, world.gh.BotComments(40)))

	build := func(request *runner.RunRequest) *runner.RunResult {
		writeText(t, filepath.Join(request.Cwd, "src", "engine.ts"), "rule\n")
		return runner.NewRunResult(true, squishyCasesDone)
	}
	r := workFake(map[string]runner.Handler{"build": build})
	planned, result := world.run(r)
	assertEqual(t, []any{planned.Get("number"), planned.Get("onto"), result.Get("status")}, []any{41, onto, "part"})
	roles := []string{}
	for _, call := range r.Calls {
		roles = append(roles, call.Role)
	}
	assertEqual(t, roles, []string{"build"})
	assertEqual(t, fakes.Git(world.origin, "show", "refs/heads/"+onto+":src/engine.ts"), "rule")
	assertEqual(t, world.gh.GetIssue(41).Get("state"), "closed")
	assertEqual(t, world.gh.ListPulls("all", "", 200), []*pyjson.Object{})
}

func TestSquishyCases_TheNightBotsFullsendIsLeftAlone(t *testing.T) {
	squishyCasesModule(t)
	world := newSquishyCasesWorld(t)
	world.gh.AddIssue(44, fakes.Issue{Labels: []string{"bot:fullsend"}})
	assertIn(t, "is the night bot's (@jgoetzmann-bot)", world.ask(44, "/squishy fullsend")[0])
	assertEqual(t, world.gh.LabelNames(44), setOf("bot:fullsend"))
}

func TestSquishyCases_StopOnATreeStopsItsSubIssues(t *testing.T) {
	squishyCasesModule(t)
	world, _ := squishyCasesSplit(t, "split")
	world.gh.AddIssue(43, fakes.Issue{Labels: []string{"bot:build"}})
	world.gh.AddSubIssue(40, world.gh.GetIssue(43).Int("id"))
	replies := world.ask(40, "/squishy stop")
	assertIn(t, "Its open sub-issues leave the queue too: #41, #42, #43.", replies[0])
	assertNotIn(t, "squishy:build", world.gh.LabelNames(41))
	assertIn(t, "@jgoetzmann-bot stop", lastOf(t, world.gh.BotComments(43)))
}

// ------------------------------------------------------------------ SectionCases

func TestSquishyCases_ItsSectionOfTheStatusIssue(t *testing.T) {
	squishyCasesModule(t)
	world := newSquishyCasesWorld(t)
	world.gh.AddIssue(50, fakes.Issue{Title: "Huge", Labels: []string{"squishy:build", "squishy:oneshot"}})
	world.gh.AddIssue(51, fakes.Issue{Title: "Parent", Labels: []string{config.LabelTree}})
	world.gh.AddIssue(52, fakes.Issue{State: "closed"})
	world.gh.AddIssue(53)
	for _, child := range []int{52, 53} {
		world.gh.AddSubIssue(51, world.gh.GetIssue(child).Int("id"))
	}
	text := dashboard.Section(world.ctx)
	assertTrue(t, strings.HasPrefix(text, dashboard.SectionStart))
	assertIn(t, "## 🫧 Squishy", text)
	assertIn(t, "| #50 | Huge | one-shot (fullsend) |", text)
	assertIn(t, "| #51 | Parent | 1 of 2 closed", text)
	assertIn(t, "`claude-squishy`", text)
	assertIn(t, "### Last Squishy runs", text)
}

// The timeline of runs going now and the lanes as boxes, as the night bot's part of the issue has
// above it: one box for its one account, and no machine, which it never uses.
func TestSquishyCases_ItsSectionDrawsTheNightBotsChartsForItsOwnLanes(t *testing.T) {
	squishyCasesModule(t)
	world := newSquishyCasesWorld(t)
	world.gh.AddIssue(60, fakes.Issue{Title: "Busy", Labels: []string{config.LabelWorking}})
	world.ctx.Store.Update(func(s *pyjson.Object) any {
		pyjson.Obj(s.Obj("items").SetDefault("60", pyjson.New())).Update(pyjson.New(
			"kind", "build", "action", "build", "run_id", "901", "provider", "claude-squishy",
			"started_at", "2026-09-30T02:30:00Z"))
		return nil
	}, "state")
	world.gh.Runs.Set("901", pyjson.New("id", 901, "status", "in_progress"))
	text := dashboard.Section(world.ctx)
	assertEqual(t, strings.Count(text, "```mermaid"), 2)
	assertIn(t, "gantt", text)
	assertIn(t, "claude-squishy · building 60", text)
	assertIn(t, "Squishy's Claude account, on GitHub's runners: 1 of 1 working", text)
	assertIn(t, "🟢 building #60", text)
	assertNotIn(t, "subgraph machine", text)
	assertNotIn(t, "~~~", text)
	idle := dashboard.Section(newSquishyCasesWorld(t).ctx)
	assertIn(t, "Nothing is running right now.", idle)
	assertEqual(t, strings.Count(idle, "Nothing is running right now."), 1)
	// No full status in the shared issue: the line under its heading points to it.
	assertNotIn(t, "Squishy's full status", text)
	assertNotIn(t, "**Squishy status**", text)
	assertIn(t, "`/squishy status` gives the same facts on demand", text)
	// Only the box styles its chart uses.
	assertIn(t, "classDef busy", text)
	assertNotIn(t, "classDef broken", text)
}

func TestSquishyCases_ItsSectionOfTheStatisticsIssue(t *testing.T) {
	squishyCasesModule(t)
	world := newSquishyCasesWorld(t)
	squishy := pyjson.New("login", "squishy-squooby", "id", 334289562)
	run := "https://github.com/Jlore-Studios/JackiOh/actions/runs"
	for _, number := range []int{60, 61} {
		world.gh.AddIssue(number)
	}
	say := func(number int, body, at string) {
		world.gh.AddComment(number, body, fakes.Comment{User: squishy, Association: "COLLABORATOR", CreatedAt: at})
	}
	say(60, "Splitting this now ([run]("+run+"/1)), on `claude-squishy` (claude, `opus`, "+
		"strong): one session reads it.", "2026-09-29T23:00:00Z")
	say(60, "Split #60 into 3 sub-issue(s) ([run]("+run+"/1)), queued for me in this order",
		"2026-09-29T23:30:00Z")
	say(61, "Starting a one-shot build of this now ([run]("+run+"/2)), on `claude-squishy` "+
		"(claude, `opus`, strong), with fullsend.", "2026-09-29T23:40:00Z")
	say(60, "Every sub-issue of this has closed, and its end state holds on `main` "+
		"([run]("+run+"/3)), so I am closing it.", "2026-09-30T02:00:00Z")
	text := stats.BotSection(world.ctx)
	assertTrue(t, strings.HasPrefix(text, stats.SectionStart))
	assertIn(t, "🫧 <b>Squishy's record</b>", text)
	assertIn(t, "| Runs started | 2 | 2 | 2 | 2 |", text)
	assertIn(t, "| One-shot builds started | 1 | 1 | 1 | 1 |", text)
	assertIn(t, "| Splits run | 1 | 1 | 1 | 1 |", text)
	assertIn(t, "| Sub-issues they opened | 3 | 3 | 3 | 3 |", text)
	assertIn(t, "| Trees closed, their end state holding | 1 | 1 | 1 | 1 |", text)
	assertIn(t, "`squishy-run.yml` workflow runs", text)
	assertIn(t, "**Its pull requests merged** (0)", text)
}
