package tests

// test_fullsend.py: Fullsend (#505): an issue split into parts that land on one branch of its
// own, not `main`, and the reconcile that turns that branch into one pull request into `main`.
//
// These run in the night bot's process, which has the mode (`.harness/config.json`). Squishy's
// case is in `squishy_cases.py`.

import (
	"fmt"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/commands"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/issueplan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/sweep"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

var fullsendDone = `<!-- bot: {"status": "done", "title": "Part done"} -->` + "\n## What changed\nMy part."
var fullsendApprove = `<!-- review: {"verdict": "approve", "findings": []} -->` + "\nFine."

func fullsendGates() []any {
	return []any{pyjson.New("name", "rules exist", "run", "test -f src/game.txt", "timeout_minutes", 1)}
}

const fullsendLabel = "bot:fullsend"

func fullsendTree() *pyjson.Object {
	return pyjson.New("done", false, "summary", "Rules and client, side by side.", "issues", []any{
		pyjson.New("key", "rules", "title", "Part 1: the rules", "body", "Write `src/rules.txt`.",
			"plan", "1. Write src/rules.txt.", "difficulty", "medium", "labels", []any{"patch"},
			"blocked_by", []any{}),
		pyjson.New("key", "client", "title", "Part 2: the client", "body", "Write `src/client.txt`.",
			"plan", "1. Write src/client.txt.", "difficulty", "medium", "labels", []any{"patch"},
			"blocked_by", []any{})})
}

// The parts are opened in build order.
const (
	fullsendParent = 40
	fullsendRules  = 41
	fullsendClient = 42
)

const fullsendOnto = "bot/issue-40"

func fullsendAnswer(tree *pyjson.Object) string {
	return "How I cut it.\n\n```json\n" + pyjson.Dumps(tree) + "\n```"
}

func fullsendBuilder(edits map[string]string, prompts *[]string) runner.Handler {
	return func(request *runner.RunRequest) *runner.RunResult {
		if prompts != nil {
			*prompts = append(*prompts, request.Prompt)
		}
		names := []string{}
		for name := range edits {
			names = append(names, name)
		}
		sort.Strings(names)
		for _, name := range names {
			crossPut(filepath.Join(request.Cwd, name), edits[name])
		}
		return runner.NewRunResult(true, fullsendDone)
	}
}

// A runner whose builder writes `edits`: a part's run calls nothing else.
func fullsendBuilds(edits map[string]string) *runner.FakeRunner {
	return workFake(map[string]runner.Handler{"build": fullsendBuilder(edits, nil)})
}

// fullsendWorld is World: a repository, a fake GitHub and the night bot's three jobs (plan,
// work, deliver).
type fullsendWorld struct {
	t           *testing.T
	root        string
	origin      string
	clone       string
	gh          *fakes.FakeGitHub
	cfg         *config.Config
	ctx         *hctx.Context
	deliverRepo string
	runs        int
}

// fullsendWorldOpts is World's keyword arguments: env, machine and the config overrides.
type fullsendWorldOpts struct {
	Env       map[string]string
	Machine   []string
	Overrides *pyjson.Object
}

func newFullsendWorld(t *testing.T, opts ...fullsendWorldOpts) *fullsendWorld {
	t.Helper()
	var o fullsendWorldOpts
	if len(opts) > 0 {
		o = opts[0]
	}
	w := &fullsendWorld{t: t}
	w.root = crossMkdtemp(t)
	w.origin, w.clone = fakes.MakeOrigin(w.root)
	w.gh = fakes.NewFakeGitHub()
	env := map[string]string{"GITHUB_SERVER_URL": "file://" + filepath.Join(w.root, "remote"),
		"BOT_GITHUB_TOKEN": strings.Repeat("t", 20)}
	for k, v := range o.Env {
		env[k] = v
	}
	overrides := pyjson.New("gates", fullsendGates(), "install", pyjson.New("run", "true", "timeout_minutes", 1),
		"max_review_cycles", 2)
	overrides.Update(o.Overrides)
	w.cfg = testsupport.MakeConfig(testsupport.Config{Env: env, Machine: o.Machine, Overrides: overrides})
	w.ctx = testsupport.MakeCtx(w.gh, testsupport.Ctx{Cfg: w.cfg, At: testsupport.Night})
	w.gh.BranchChecks = setOf(w.cfg.RequiredChecks...)
	w.deliverRepo = filepath.Join(w.root, "deliver")
	fakes.Git(w.root, "clone", "-q", w.origin, w.deliverRepo)
	w.gh.EnsureLabel("patch", "ededed", "patch")
	return w
}

func (w *fullsendWorld) claim() *pyjson.Object {
	return plan.Make(w.ctx, false, nil, "auto", nil)
}

func (w *fullsendWorld) work(planned *pyjson.Object, r *runner.FakeRunner) string {
	w.runs++
	out := filepath.Join(w.root, fmt.Sprintf("out-%d", w.runs))
	work.NewWorker(w.cfg, planned, r, w.clone, filepath.Join(w.root, fmt.Sprintf("work-%d", w.runs)), out,
		nil, nil, nil, nil, nil, nil).Run()
	return out
}

func (w *fullsendWorld) deliver(planned *pyjson.Object, out string) *pyjson.Object {
	w.t.Helper()
	deliver.NewDeliverer(w.ctx, planned, out, w.deliverRepo, "", nil, "").Run()
	return loadJSON(w.t, filepath.Join(out, "result.json"))
}

func (w *fullsendWorld) run(r *runner.FakeRunner) (*pyjson.Object, *pyjson.Object) {
	w.t.Helper()
	planned := w.claim()
	if planned.Str("action") == "none" {
		return planned, pyjson.New()
	}
	return planned, w.deliver(planned, w.work(planned, r))
}

func (w *fullsendWorld) split() *pyjson.Object {
	w.t.Helper()
	w.gh.AddIssue(fullsendParent, fakes.Issue{Title: "A big project", Body: "Do it all.\n\nDone when: it works.",
		Labels: []string{config.LabelBuild, fullsendLabel}})
	_, result := w.run(workFake(map[string]runner.Handler{"plan": crossAnswer(fullsendAnswer(fullsendTree()))}))
	return result
}

func (w *fullsendWorld) tip(branch string) string {
	return fakes.Git(w.origin, "rev-parse", "refs/heads/"+branch)
}

func (w *fullsendWorld) has(branch, path string) bool {
	_, ok := crossGitOK(w.origin, "cat-file", "-e", "refs/heads/"+branch+":"+path)
	return ok
}

// ---------------------------------------------------------------------------- ModeTests

func TestFullsend_FullsendIsANightBotMode(t *testing.T) {
	assertEqual(t, config.Modes, []string{"fullsend"})
	assertEqual(t, config.ModeLabels, pyjson.New(fullsendLabel, "fullsend"))
	for _, name := range []string{fullsendLabel, config.LabelTree} {
		assertIn(t, name, config.Labels)
		text := pyjson.Str(dig(t, config.Labels.Get(name), 1))
		assertIn(t, "the night bot", text)
		assertTrue(t, py.Len(text) <= config.LabelDescriptionMax)
	}
	assertTrue(t, commands.Offered("fullsend"))
	assertFalse(t, commands.Offered("oneshot"))
	assertFalse(t, commands.Offered("split"))
	text := commands.HelpText("jgoetzmann-bot", "")
	assertIn(t, "`fullsend [notes]`", text)
	assertIn(t, "`build`, `fullsend`, `revise` or `suggest`", text)
	assertIn(t, "`/harness fullsend`", commands.HelpText("jgoetzmann-bot", "fullsend"))
	got := [][2]string{}
	for _, c := range commands.Parse("/harness fullsend\n@jgoetzmann-bot fullsend", "jgoetzmann-bot") {
		got = append(got, [2]string{c.Verb, c.Args})
	}
	assertEqual(t, got, [][2]string{{"fullsend", ""}, {"fullsend", ""}})
}

func TestFullsend_TheLabelQueuesItInFullsendModeAtHard(t *testing.T) {
	world := newFullsendWorld(t)
	world.gh.AddIssue(fullsendParent, fakes.Issue{Labels: []string{fullsendLabel}})
	events.OnIssueChange(world.ctx, pyjson.New("action", "labeled", "issue", world.gh.GetIssue(fullsendParent),
		"label", pyjson.New("name", fullsendLabel), "sender", fakes.Operator), false)
	assertEqual(t, world.gh.LabelNames(fullsendParent), setOf(config.LabelBuild, fullsendLabel))
	assertIn(t, "split it into parts that land on one branch", lastOf(t, world.gh.BotComments(fullsendParent)))
	found := queue.Candidates(world.ctx, world.ctx.Store.Load(), nil, nil)
	got := []any{}
	for _, c := range found {
		got = append(got, []any{c.Number, c.Mode, c.Difficulty, c.Planned})
	}
	assertEqual(t, got, []any{[]any{fullsendParent, "fullsend", "hard", true}})
	planned := world.claim()
	assertEqual(t, []any{planned.Get("action"), planned.Get("mode"), planned.Get("difficulty")},
		[]any{"build", "fullsend", "hard"})
	assertEqual(t, dig(t, planned, "seats", "build", "tier"), "strong")
	assertIn(t, "Splitting this now for fullsend", lastOf(t, world.gh.BotComments(fullsendParent)))
}

func TestFullsend_TheCommandQueuesItAndSquishysFullsendIsLeftAlone(t *testing.T) {
	world := newFullsendWorld(t)
	world.gh.AddIssue(3)
	asked := commands.Parse("/harness fullsend", "jgoetzmann-bot")
	replies := events.RunCommands(world.ctx, asked, world.gh.GetIssue(3), fakes.Operator, 3, nil)
	assertIn(t, "Queued #3", replies[0])
	assertEqual(t, world.gh.LabelNames(3), setOf(config.LabelBuild, fullsendLabel))
	world.gh.AddIssue(4, fakes.Issue{Labels: []string{"squishy:fullsend"}})
	mode := "fullsend"
	assertIn(t, "is Squishy's (@squishy-squooby)",
		queue.QueueBuild(world.ctx, 4, "jgoetzmann", false, false, nil, &mode))
	assertEqual(t, world.gh.LabelNames(4), setOf("squishy:fullsend"))
}

// ---------------------------------------------------------------------------- SplitTests

func TestFullsend_TheSplitSetsUpTheBranchAndOpensParts(t *testing.T) {
	world := newFullsendWorld(t)
	prompts := []string{}
	split := func(request *runner.RunRequest) *runner.RunResult {
		prompts = append(prompts, request.Prompt)
		return runner.NewRunResult(true, fullsendAnswer(fullsendTree()))
	}
	world.gh.AddIssue(fullsendParent, fakes.Issue{Title: "A big project", Body: "Do it all.",
		Labels: []string{config.LabelBuild, fullsendLabel, "priority:high"}})
	_, result := world.run(workFake(map[string]runner.Handler{"plan": split}))
	assertEqual(t, result.Get("status"), "split")
	assertIn(t, "## This is a fullsend split", prompts[0])
	assertIn(t, "Each part lands on `"+fullsendOnto+"`", prompts[0])
	// The branch the parts land on, made from main's head.
	assertEqual(t, world.tip(fullsendOnto), world.tip("main"))
	for _, part := range []int{fullsendRules, fullsendClient} {
		assertEqual(t, world.gh.LabelNames(part), setOf("patch", "difficulty:medium", "priority:high", config.LabelBuild))
		assertIn(t, "Its build lands on `"+fullsendOnto+"`, not `main` (fullsend).",
			crossStr(t, world.gh.GetIssue(part), "body"))
		record := crossObj(t, world.ctx.Store.Load(), "items", strconv.Itoa(part))
		assertEqual(t, []any{record.Get("onto"), record.Get("part_of")}, []any{fullsendOnto, fullsendParent})
	}
	assertEqual(t, world.gh.LabelNames(fullsendParent), setOf("priority:high", config.LabelTree))
	assertIn(t, "This is a fullsend tree: each part lands on `"+fullsendOnto+"`",
		lastOf(t, world.gh.BotComments(fullsendParent)))
	tree := crossObj(t, world.ctx.Store.Load(), "items", strconv.Itoa(fullsendParent), "tree")
	assertEqual(t, []any{tree.Get("mode"), tree.Get("children"), tree.Get("onto")},
		[]any{"fullsend", []any{fullsendRules, fullsendClient}, fullsendOnto})
}

func TestFullsend_FullsendWaitsWhilePartsAreOpen(t *testing.T) {
	world := newFullsendWorld(t)
	world.split()
	mode := "fullsend"
	reply := queue.QueueBuild(world.ctx, fullsendParent, "jgoetzmann", false, false, nil, &mode)
	assertIn(t, fmt.Sprintf("#%d's parts are still open (#%d, #%d)", fullsendParent, fullsendRules, fullsendClient), reply)
	assertNotIn(t, config.LabelBuild, world.gh.LabelNames(fullsendParent))
	// A queue label put on by hand does not build it either.
	world.gh.AddLabels(fullsendParent, []string{config.LabelBuild})
	skipped := []string{}
	found := queue.Candidates(world.ctx, world.ctx.Store.Load(), &skipped, nil)
	numbers := []int{}
	for _, c := range found {
		numbers = append(numbers, c.Number)
	}
	assertNotIn(t, fullsendParent, numbers)
	assertIn(t, fmt.Sprintf("#%d skipped: its fullsend parts #%d, #%d are still open", fullsendParent, fullsendRules,
		fullsendClient), skipped)
	reply = queue.QueueBuild(world.ctx, fullsendParent, "jgoetzmann", false, true, nil, nil)
	assertIn(t, "parts are still open", reply)
	assertNotIn(t, config.LabelBuild, world.gh.LabelNames(fullsendParent))
}

// ---------------------------------------------------------------------------- PartTests

func fullsendPartWorld(t *testing.T) *fullsendWorld {
	world := newFullsendWorld(t)
	world.split()
	return world
}

func TestFullsend_APartLandsOnTheBranchWithoutAPullRequest(t *testing.T) {
	world := fullsendPartWorld(t)
	prompts := []string{}
	r := workFake(map[string]runner.Handler{"build": fullsendBuilder(map[string]string{"src/rules.txt": "the rules\n"}, &prompts)})
	planned, result := world.run(r)
	assertEqual(t, []any{planned.Get("number"), planned.Get("onto"), planned.Get("part_of")},
		[]any{fullsendRules, fullsendOnto, fullsendParent})
	assertEqual(t, result.Get("status"), "part")
	assertEqual(t, workRoles(r), []string{"build"}) // no checks, no review
	assertIn(t, fmt.Sprintf("This is part of fullsend tree #%d. Your change lands on `%s`", fullsendParent, fullsendOnto),
		prompts[0])
	assertEqual(t, len(world.gh.ListPulls("all", "", 200)), 0)
	assertTrue(t, world.has(fullsendOnto, "src/rules.txt"))
	assertFalse(t, world.has("main", "src/rules.txt"))
	issue := world.gh.GetIssue(fullsendRules)
	assertEqual(t, []any{issue.Get("state"), issue.Get("state_reason")}, []any{"closed", "completed"})
	assertEqual(t, crossAnd(world.gh.LabelNames(fullsendRules), setOf(config.LabelBuild, config.LabelWorking)), setOf())
	assertIn(t, fmt.Sprintf("It landed on `%s` at `%s`.", fullsendOnto, world.tip(fullsendOnto)[:12]),
		lastOf(t, world.gh.BotComments(fullsendRules)))
	assertEqual(t, dig(t, world.ctx.Store.Load(), "items", strconv.Itoa(fullsendRules), "landed"), world.tip(fullsendOnto))
}

func TestFullsend_TwoPartsLandSideBySide(t *testing.T) {
	world := fullsendPartWorld(t)
	first := world.claim()
	second := world.claim() // the second starts before the first lands
	assertEqual(t, []any{first.Get("number"), second.Get("number")}, []any{fullsendRules, fullsendClient})
	outFirst := world.work(first, fullsendBuilds(map[string]string{"src/rules.txt": "rules\n"}))
	outSecond := world.work(second, fullsendBuilds(map[string]string{"src/client.txt": "ui\n"}))
	main := world.tip("main")
	assertEqual(t, world.deliver(first, outFirst).Get("status"), "part")
	landed := world.tip(fullsendOnto)
	assertEqual(t, fakes.Git(world.origin, "rev-list", "--count", main+".."+landed), "1")
	assertEqual(t, world.deliver(second, outSecond).Get("status"), "part")
	tip := world.tip(fullsendOnto)
	assertTrue(t, world.has(fullsendOnto, "src/rules.txt"))
	assertTrue(t, world.has(fullsendOnto, "src/client.txt"))
	parents := strings.Fields(fakes.Git(world.origin, "rev-list", "--parents", "-n", "1", tip))[1:]
	assertEqual(t, len(parents), 2) // joined by a merge commit, never pushed with force
	assertEqual(t, parents[0], landed)
	assertIn(t, fmt.Sprintf("land #%d on %s", fullsendClient, fullsendOnto),
		fakes.Git(world.origin, "log", "-1", "--format=%s", tip))
	for _, part := range []int{fullsendRules, fullsendClient} {
		assertEqual(t, world.gh.GetIssue(part).Get("state"), "closed")
	}
}

func TestFullsend_AConflictingPartIsParked(t *testing.T) {
	world := fullsendPartWorld(t)
	first := world.claim()
	second := world.claim()
	outFirst := world.work(first, fullsendBuilds(map[string]string{"src/game.txt": "rules A\n"}))
	outSecond := world.work(second, fullsendBuilds(map[string]string{"src/game.txt": "rules B\n"}))
	world.deliver(first, outFirst)
	landed := world.tip(fullsendOnto)
	result := world.deliver(second, outSecond)
	assertEqual(t, world.tip(fullsendOnto), landed) // the branch keeps the part that landed first
	assertEqual(t, world.tip(fmt.Sprintf("bot/issue-%d", fullsendClient)), result.Get("head"))
	assertEqual(t, dig(t, world.ctx.Store.Load(), "items", strconv.Itoa(fullsendParent), "parked"),
		[]any{fmt.Sprintf("bot/issue-%d", fullsendClient)})
	assertEqual(t, world.gh.GetIssue(fullsendClient).Get("state"), "closed")
	assertIn(t, fmt.Sprintf("kept on `bot/issue-%d` instead, for #%d's reconcile to merge", fullsendClient, fullsendParent),
		lastOf(t, world.gh.BotComments(fullsendClient)))
}

func TestFullsend_ALaterPartStartsFromThePartsBeforeIt(t *testing.T) {
	world := fullsendPartWorld(t)
	world.run(fullsendBuilds(map[string]string{"src/rules.txt": "rules\n"}))
	landed := world.tip(fullsendOnto)
	seen := []bool{}
	client := func(request *runner.RunRequest) *runner.RunResult {
		seen = append(seen, isFile(filepath.Join(request.Cwd, "src", "rules.txt")))
		return fullsendBuilder(map[string]string{"src/client.txt": "ui\n"}, nil)(request)
	}
	planned, result := world.run(workFake(map[string]runner.Handler{"build": client}))
	assertEqual(t, []any{planned.Get("number"), result.Get("status")}, []any{fullsendClient, "part"})
	assertEqual(t, seen, []bool{true})
	assertEqual(t, result.Get("start"), landed)
	assertEqual(t, result.Get("changed_paths"), []any{"src/client.txt"}) // measured against the branch
	assertEqual(t, fakes.Git(world.origin, "rev-parse", world.tip(fullsendOnto)+"^"), landed)
}

func TestFullsend_AHaltBeforeDeliveryLandsNothing(t *testing.T) {
	world := fullsendPartWorld(t)
	planned := world.claim()
	out := world.work(planned, fullsendBuilds(map[string]string{"src/rules.txt": "rules\n"}))
	world.ctx.Store.Update(func(s *pyjson.Object) any { s.Set("halted", true); return nil }, "state")
	assertEqual(t, world.deliver(planned, out).Get("status"), "part")
	assertEqual(t, world.tip(fullsendOnto), world.tip("main"))
	assertEqual(t, world.gh.GetIssue(fullsendRules).Get("state"), "open")
	assertIn(t, config.LabelBuild, world.gh.LabelNames(fullsendRules))
	assertTrue(t, world.has(fmt.Sprintf("bot/issue-%d", fullsendRules), "src/rules.txt")) // kept for the next run
}

func TestFullsend_APartTouchingAForbiddenPathLandsNothing(t *testing.T) {
	world := fullsendPartWorld(t)
	planned := world.claim()
	r := fullsendBuilds(map[string]string{"src/rules.txt": "rules\n", ".harness/HALT": "no\n"})
	// The work job's guard puts such a change back; deliver refuses it all the same.
	guard := work.Guard
	work.Guard = func(*work.Worker) []*verdicts.Finding { return []*verdicts.Finding{} }
	out := func() string {
		defer func() { work.Guard = guard }()
		return world.work(planned, r)
	}()
	assertEqual(t, world.deliver(planned, out).Get("status"), "part")
	assertEqual(t, world.tip(fullsendOnto), world.tip("main"))
	assertIn(t, "the change touches paths the bot may not change: .harness/HALT",
		lastOf(t, world.gh.BotComments(fullsendRules)))
	assertEqual(t, world.gh.GetIssue(fullsendRules).Get("state"), "open")
	assertIn(t, config.LabelBuild, world.gh.LabelNames(fullsendRules))
}

// ---------------------------------------------------------------------------- EasyPartTests

// The easy rule counts only the part's own change, not the parts that landed before it: here
// one file is the limit, and an earlier part already added another.
func TestFullsend_AWeakBuildersPartIsMeasuredAgainstTheBranch(t *testing.T) {
	world := newFullsendWorld(t, fullsendWorldOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"lite"}, Overrides: pyjson.New("easy", pyjson.New("max_files", 1, "max_lines", 400))})
	world.gh.AddIssue(fullsendParent, fakes.Issue{Labels: []string{config.LabelTree}})
	fakes.PushBranch(world.origin, world.root, fullsendOnto, map[string]string{"src/rules.txt": "rules\n"})
	world.gh.AddIssue(fullsendClient, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:easy"},
		Body: issueplan.WithPlan("Write `src/client.txt`.", "1. Write src/client.txt.", "a person", "")})
	world.gh.AddSubIssue(fullsendParent, crossInt(t, world.gh.GetIssue(fullsendClient), "id"))
	partOf := func(number string) {
		world.ctx.Store.Update(func(s *pyjson.Object) any {
			pyjson.Obj(pyjson.Obj(s.Get("items")).SetDefault(number, pyjson.New())).Update(
				pyjson.New("onto", fullsendOnto, "part_of", fullsendParent))
			return nil
		}, "state")
	}
	partOf(strconv.Itoa(fullsendClient))
	r := fullsendBuilds(map[string]string{"src/client.txt": "ui\n"})
	planned, result := world.run(r)
	assertEqual(t, []any{planned.Get("provider"), result.Get("status")}, []any{"lite", "part"})
	assertEqual(t, workRoles(r), []string{"build"}) // no self check
	assertTrue(t, world.has(fullsendOnto, "src/client.txt"))
	assertEqual(t, world.gh.GetIssue(fullsendClient).Get("state"), "closed")
	// A part of its own that breaks the rule lands nothing, and a stronger model takes it.
	world.gh.AddIssue(43, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:easy"},
		Body: issueplan.WithPlan("Write two files.", "1. Write them.", "a person", "")})
	partOf("43")
	landed := world.tip(fullsendOnto)
	_, result = world.run(fullsendBuilds(map[string]string{"src/a.txt": "a\n", "src/b.txt": "b\n"}))
	assertEqual(t, result.Get("status"), "part")
	assertEqual(t, world.tip(fullsendOnto), landed)
	assertIn(t, "breaks the easy rule (line 1: it changes 2 files, over 1)", lastOf(t, world.gh.BotComments(43)))
	assertEqual(t, world.gh.GetIssue(43).Get("state"), "open")
	assertIn(t, config.LabelBuild, world.gh.LabelNames(43))
}

// ---------------------------------------------------------------------------- AdoptTests

const (
	fullsendTracker     = 60
	fullsendPlanned     = 61
	fullsendUnqueued    = 62
	fullsendPeoples     = 63
	fullsendRunning     = 64
	fullsendDoneAlready = 65
)

const fullsendTrackerOnto = "bot/issue-60"

// `fullsend` on an issue whose sub-issues a person opened already: they are its parts, with no
// split, and a medium subscription builds each from a strong model's plan.
func fullsendAdoptWorld(t *testing.T, opts ...fullsendWorldOpts) *fullsendWorld {
	world := newFullsendWorld(t, opts...)
	gh := world.gh
	gh.AddIssue(fullsendTracker, fakes.Issue{Title: "Patch v0.3.X: the set on one branch", Body: "Its parts below."})
	gh.AddIssue(fullsendPlanned, fakes.Issue{Title: "Batch 1", Labels: []string{config.LabelBuild, "difficulty:hard"}})
	gh.AddIssue(fullsendUnqueued, fakes.Issue{Title: "Batch 2", Labels: []string{"difficulty:medium"}})
	gh.AddIssue(fullsendPeoples, fakes.Issue{Title: "The release", Labels: []string{"human"}})
	gh.AddIssue(fullsendRunning, fakes.Issue{Title: "Batch 3", Labels: []string{config.LabelWorking, "difficulty:hard"}})
	gh.AddIssue(fullsendDoneAlready, fakes.Issue{Title: "The foundation", State: "closed"})
	for _, child := range []int{fullsendPlanned, fullsendUnqueued, fullsendPeoples, fullsendRunning, fullsendDoneAlready} {
		gh.AddSubIssue(fullsendTracker, crossInt(t, gh.GetIssue(child), "id"))
	}
	world.ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, fullsendPlanned).Update(pyjson.New("planned_at", "2026-09-29T00:00:00Z", "planned_tier", "strong"))
		return nil
	}, "state")
	gh.Runs.Set(strconv.Itoa(fullsendRunning), pyjson.New("status", "in_progress")) // claude-1 is building it
	world.ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, fullsendRunning).Update(pyjson.New("run_id", strconv.Itoa(fullsendRunning), "provider", "claude-1"))
		return nil
	}, "state")
	return world
}

func fullsendAdopt(world *fullsendWorld) string {
	asked := commands.Parse("/harness fullsend", "jgoetzmann-bot")
	return events.RunCommands(world.ctx, asked, world.gh.GetIssue(fullsendTracker), fakes.Operator, 3, nil)[0]
}

func TestFullsend_TheSubIssuesSomeoneOpenedBecomeItsParts(t *testing.T) {
	world := fullsendAdoptWorld(t)
	reply := fullsendAdopt(world)
	assertIn(t, fmt.Sprintf("#%d's sub-issues are its fullsend parts now, with no split: #%d, #%d land on `%s`",
		fullsendTracker, fullsendPlanned, fullsendUnqueued, fullsendTrackerOnto), reply)
	assertIn(t, fmt.Sprintf("#%d (labelled `human`), #%d (a run holds it now)", fullsendPeoples, fullsendRunning), reply)
	assertEqual(t, world.gh.CreatedBranches, [][2]string{{fullsendTrackerOnto, "head-of-main"}})
	items := crossItems(t, world.ctx)
	for _, part := range []int{fullsendPlanned, fullsendUnqueued} {
		record := crossObj(t, items, strconv.Itoa(part))
		assertEqual(t, []any{dig(t, record, "onto"), dig(t, record, "part_of")},
			[]any{fullsendTrackerOnto, fullsendTracker})
	}
	for _, kept := range []int{fullsendPeoples, fullsendRunning, fullsendDoneAlready} {
		assertNotIn(t, "onto", items.Obj(strconv.Itoa(kept)))
	}
	assertEqual(t, world.gh.LabelNames(fullsendUnqueued), setOf("difficulty:medium", config.LabelBuild))
	assertEqual(t, world.gh.LabelNames(fullsendTracker), setOf(config.LabelTree))
	tree := crossObj(t, items, strconv.Itoa(fullsendTracker), "tree")
	assertEqual(t, []any{tree.Get("mode"), tree.Get("onto"), tree.Get("adopted")},
		[]any{"fullsend", fullsendTrackerOnto, true})
	// Asked again, it waits for its parts as any fullsend tree does, and builds nothing.
	assertIn(t, fmt.Sprintf("#%d's parts are still open (#%d, #%d, #%d, #%d)", fullsendTracker, fullsendPlanned,
		fullsendUnqueued, fullsendPeoples, fullsendRunning), fullsendAdopt(world))
	found := map[int]*queue.Candidate{}
	for _, c := range queue.Candidates(world.ctx, world.ctx.Store.Load(), nil, nil) {
		found[c.Number] = c
	}
	numbers := []int{}
	for n := range found {
		numbers = append(numbers, n)
	}
	sort.Ints(numbers)
	assertEqual(t, numbers, []int{fullsendPlanned, fullsendUnqueued})
	assertTrue(t, found[fullsendPlanned].Part && found[fullsendUnqueued].Part)
	// Once every sub-issue has closed, the sweep queues its reconcile.
	for _, child := range []int{fullsendPlanned, fullsendUnqueued, fullsendPeoples, fullsendRunning} {
		world.gh.UpdateIssue(child, pyjson.New("state", "closed"))
	}
	assertEqual(t, sweep.Trees(world.ctx, testsupport.Night),
		[]string{fmt.Sprintf("#%d: its parts have all closed; queued its reconcile", fullsendTracker)})
	assertEqual(t, world.gh.LabelNames(fullsendTracker), setOf(config.LabelTree, config.LabelBuild, fullsendLabel))
}

func TestFullsend_MuseBuildsAHardPartFromItsPlanAndLandsIt(t *testing.T) {
	world := fullsendAdoptWorld(t, fullsendWorldOpts{Machine: []string{"muse"}})
	fullsendAdopt(world)
	fakes.Git(world.origin, "branch", fullsendTrackerOnto, "main") // what the adoption made on GitHub
	planned := world.claim()
	assertEqual(t, []any{planned.Get("number"), planned.Get("provider"), planned.Get("onto")},
		[]any{fullsendPlanned, "muse", fullsendTrackerOnto})
	assertEqual(t, dig(t, planned, "seats", "build", "tier"), "medium")
	assertEqual(t, dig(t, planned, "seats", "plan"), nil) // built from the strong plan on record
	result := world.deliver(planned, world.work(planned, fullsendBuilds(map[string]string{"src/batch1.txt": "one\n"})))
	assertEqual(t, result.Get("status"), "part")
	assertTrue(t, world.has(fullsendTrackerOnto, "src/batch1.txt"))
	assertFalse(t, world.has("main", "src/batch1.txt"))
	assertEqual(t, world.gh.GetIssue(fullsendPlanned).Get("state"), "closed")
	// The unplanned medium part goes to the lane's weakest seat that meets its floor:
	// Muse plans it first in its own run (#695).
	planned = world.claim()
	assertEqual(t, []any{planned.Get("number"), planned.Get("provider")}, []any{fullsendUnqueued, "muse"})
	assertEqual(t, dig(t, planned, "seats", "plan", "tier"), "medium")
}

// ---------------------------------------------------------------------------- PartBuilderTests

func TestFullsend_APartGoesToTheMediumSubscriptionsFirstAndNeverToSonnetAtHard(t *testing.T) {
	pool := testsupport.MakeConfig(testsupport.Config{Machine: []string{"muse", "lite"}}).Pool
	claude, muse, lite := pool.Get("claude-3"), pool.Get("muse"), pool.Get("lite")
	who := func(offered []*providers.Provider, difficulty string, part bool) []any {
		seat, note := plan.BuilderSeat(pool, offered, difficulty, part, "", "", nil)
		if seat != nil {
			return []any{[]any{seat.Provider.ID, seat.Tier}, note}
		}
		return []any{nil, note}
	}
	p := func(xs ...*providers.Provider) []*providers.Provider { return xs }
	assertEqual(t, who(p(claude, muse), "hard", false), []any{[]any{"claude-3", "strong"}, ""})
	assertEqual(t, who(p(claude, muse), "hard", true), []any{[]any{"muse", "medium"},
		"a fullsend part, built on medium from its plan: its tree's reconcile checks and reviews it"})
	assertEqual(t, who(p(claude), "hard", true), []any{[]any{"claude-3", "strong"}, ""})
	assertEqual(t, who(p(claude, muse), "medium", false)[0], []any{"muse", "medium"})
	assertEqual(t, who(p(claude, muse), "medium", true), []any{[]any{"muse", "medium"}, ""})
	assertEqual(t, who(p(lite), "hard", true), []any{nil, ""}) // never under PART_FLOOR
}

// ---------------------------------------------------------------------------- ReconcileTests

func TestFullsend_TheReconcileOpensOnePullIntoMain(t *testing.T) {
	world := newFullsendWorld(t)
	world.split()
	first := world.claim()
	second := world.claim()
	outFirst := world.work(first, fullsendBuilds(map[string]string{"src/game.txt": "rules A\n"}))
	outSecond := world.work(second, fullsendBuilds(map[string]string{"src/game.txt": "rules B\n"}))
	world.deliver(first, outFirst)
	world.deliver(second, outSecond) // parked
	world.ctx.Store.Update(func(s *pyjson.Object) any {
		s.Set("last_sweep", pyjson.New("since", "2026-09-29T00:00:00Z"))
		return nil
	}, "state")
	notes := sweep.Trees(world.ctx, testsupport.Night)
	assertEqual(t, notes, []string{fmt.Sprintf("#%d: its parts have all closed; queued its reconcile", fullsendParent)})
	assertEqual(t, world.gh.LabelNames(fullsendParent), setOf(config.LabelTree, config.LabelBuild, fullsendLabel))
	assertIn(t, "one run merges the parts on `"+fullsendOnto+"`", lastOf(t, world.gh.BotComments(fullsendParent)))
	assertIn(t, "I will reconcile its parts into one pull request", lastOf(t, world.gh.BotComments(fullsendParent)))
	prompts := []string{}
	created := []*pyjson.Object{}
	world.gh.Patch.CreatePull = func(title, head, base, body string, draft bool) *pyjson.Object {
		created = append(created, pyjson.New("title", title, "head", head, "base", base, "body", body, "draft", draft))
		return world.gh.Own().CreatePull(title, head, base, body, draft)
	}
	planned, result := func() (*pyjson.Object, *pyjson.Object) {
		defer func() { world.gh.Patch.CreatePull = nil }()
		return world.run(workFake(map[string]runner.Handler{
			"build":  fullsendBuilder(map[string]string{"src/game.txt": "rules A and B\n"}, &prompts),
			"review": crossAnswer(fullsendApprove)}))
	}()
	assertEqual(t, []any{planned.Get("number"), planned.Get("mode"), planned.Get("difficulty")},
		[]any{fullsendParent, "fullsend", "hard"})
	assertIn(t, fmt.Sprintf("#%d [closed] Part 1: the rules", fullsendRules), crossStr(t, planned, "children"))
	assertEqual(t, planned.Get("parked"), []any{fmt.Sprintf("bot/issue-%d", fullsendClient)})
	assertNotIn(t, "onto", planned)
	assertIn(t, fmt.Sprintf("# Reconcile fullsend tree #%d", fullsendParent), prompts[0])
	assertIn(t, fmt.Sprintf("`origin/bot/issue-%d`", fullsendClient), prompts[0])
	assertIn(t, ".claude/skills/fullsend/SKILL.md", prompts[0])
	assertEqual(t, result.Get("status"), "approved")
	assertEqual(t, len(created), 1)
	assertEqual(t, []any{created[0].Get("head"), created[0].Get("base")}, []any{fullsendOnto, "main"})
	assertIn(t, fmt.Sprintf("Closes #%d", fullsendParent), crossStr(t, created[0], "body"))
	// The pull request holds the parts that landed and the reconcile's own change.
	assertEqual(t, fakes.Git(world.origin, "show", "refs/heads/"+fullsendOnto+":src/game.txt"), "rules A and B")
}
