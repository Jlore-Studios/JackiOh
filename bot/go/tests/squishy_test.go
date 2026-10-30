// squishy_test.go is bot/tests/test_squishy.py: Squishy (#60): the night bot's harness run as a
// second bot, and how the two live side by side.
//
// These run in the night bot's process. Squishy's own cases (its labels, commands, modes, splits
// and section) need a process that is Squishy from the start, since the labels and branches are
// fixed when the process starts: TestSquishy_SquishysOwnCasesPassAsSquishy (SquishyProcessTests)
// runs squishy_cases_test.go's tests in this test binary again, with `HARNESS_HOME=.squishy`.
package tests

import (
	"bytes"
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cardsets"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cli"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/identity"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/split"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stats"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// SQUISHY
var squishyUser = pyjson.New("login", "squishy-squooby", "id", 334289562)

// ------------------------------------------------------------------ SquishyProcessTests

// squishy_cases_test.go's tests in a process started as Squishy: this test binary again, with
// `HARNESS_HOME=.squishy`, from bot/go/tests as `go test` starts it (TestMain moves to bot/, the
// Python's cwd=BOT_DIR).
func TestSquishy_SquishysOwnCasesPassAsSquishy(t *testing.T) {
	binary, err := os.Executable() // os.Args[0], wherever this process has moved to
	if err != nil {
		t.Fatal(err)
	}
	here, err := os.Getwd() // bot/, TestMain's
	if err != nil {
		t.Fatal(err)
	}
	run, cancel := context.WithTimeout(context.Background(), 600*time.Second)
	defer cancel()
	cmd := exec.CommandContext(run, binary, "-test.run", "^TestSquishyCases_", "-test.v", "-test.count=1")
	cmd.Dir = filepath.Join(here, "go", "tests")
	cmd.Env = append(os.Environ(), "HARNESS_HOME=.squishy")
	var out bytes.Buffer
	cmd.Stdout, cmd.Stderr = &out, &out
	err = cmd.Run()
	text := out.String()
	assertTrue(t, err == nil, fmt.Sprintf("%v\n%s", err, text))
	ran := regexp.MustCompile(`(?m)^--- PASS: TestSquishyCases_`).FindAllString(text, -1)
	assertTrue(t, len(ran) > 0, "no test ran:\n"+text)
	assertNotIn(t, "--- SKIP", text)
}

// ------------------------------------------------------------------ IdentityTests

func TestSquishy_ThisProcessIsTheNightBot(t *testing.T) {
	assertEqual(t, config.Home, ".harness")
	assertEqual(t, []string{config.LabelBuild, config.Slash, config.StateBranch, config.Marker},
		[]string{"bot:build", "/harness", "bot-state", "<!-- jackioh-bot -->"})
	assertEqual(t, config.Modes, []string{"fullsend"})
	logins := []string{}
	for _, o := range config.Others {
		logins = append(logins, o.Login)
	}
	assertEqual(t, logins, []string{"squishy-squooby"})
	assertEqual(t, queue.BranchForIssue(7), "bot/issue-7")
}

func TestSquishy_SquishysConfigSaysWhoItIs(t *testing.T) {
	found, err := identity.Load(config.RepoRoot, ".squishy")
	if err != nil {
		t.Fatal(err)
	}
	assertEqual(t, []string{found.Name, found.Command, found.LabelPrefix, found.BranchPrefix},
		[]string{"Squishy", "squishy", "squishy:", "squishy/"})
	assertEqual(t, []string{found.StateBranch, found.JournalBranch, found.Workflow},
		[]string{"squishy-state", "squishy-journal", "squishy-run.yml"})
	assertEqual(t, found.Modes, []string{"oneshot", "split", "split-bot", "fullsend"})
	assertFalse(t, found.Suggestions)
	logins := []string{}
	for _, o := range found.Others {
		logins = append(logins, o.Login)
	}
	assertEqual(t, logins, []string{"jgoetzmann-bot"})
	assertNotEqual(t, found.Marker, config.Marker)
}

func TestSquishy_SquishysSubscriptionIsItsOwnOnGithubsRunners(t *testing.T) {
	raw := loadsObject(t, readText(t, filepath.Join(config.RepoRoot, ".squishy", "providers.json")))
	pool := providers.Parse(raw)
	assertEqual(t, pool.Providers.Keys(), []string{"claude-squishy"})
	provider := pool.Get("claude-squishy")
	assertEqual(t, []string{provider.CLI, provider.Model, provider.Tier, provider.RunsOn},
		[]string{"claude", "opus", "strong", "ubuntu-latest"})
	assertEqual(t, provider.Secret, "CLAUDE_CODE_OAUTH_TOKEN_SQUISHY")
	assertNotIn(t, "suggest", provider.Roles)
	assertEqual(t, [2]int{pool.MachineParallel, pool.PlanLanes}, [2]int{0, 0})
	night := map[string]bool{}
	testsupport.MakeConfig().Pool.Providers.Items(func(_ string, p any) bool {
		night[p.(*providers.Provider).Secret] = true
		return true
	})
	assertNotIn(t, provider.Secret, night)
}

func TestSquishy_BothConfigsKeepEachOthersSwitchesOutOfReach(t *testing.T) {
	for _, home := range []string{".harness", ".squishy"} {
		raw := loadsObject(t, readText(t, filepath.Join(config.RepoRoot, home, "config.json")))
		for _, path := range []string{".github/", ".harness/", ".squishy/", "bot/", ".claude/"} {
			assertIn(t, path, pyjson.Strings(raw.Get("forbidden_paths")), home)
		}
	}
}

func TestSquishy_ABadIdentityIsRefused(t *testing.T) {
	for _, raw := range []*pyjson.Object{pyjson.New("label_prefix", "squishy"),
		pyjson.New("modes", []any{"teleport"}), pyjson.New("colour", "blue")} {
		_, err := identity.Parse(raw, true)
		assertTrue(t, err != nil, "ValueError not raised: ", pyjson.Dumps(raw))
	}
	found, err := identity.Parse(nil, true)
	assertTrue(t, err == nil)
	assertEqual(t, found, identity.NewIdentity())
}

// ------------------------------------------------------------------ SplitTests

// TREE
const squishyTree = "I cut it in two: the engine first, then the client.\n\n```json\n" +
	`{"done": false, "summary": "Engine, then client.",
 "issues": [
  {"key": "client", "title": "Part 2: the client", "body": "Draw it.", "plan": "1. draw",
   "difficulty": "easy", "labels": ["patch"], "blocked_by": ["engine", "#77"]},
  {"key": "engine", "title": "Part 1: the engine", "body": "Rule it.", "plan": "1. rule",
   "difficulty": "hard", "labels": ["patch", "bot:build"], "blocked_by": []}
 ]}` + "\n```"

func TestSquishy_ATreeIsReadAndOrderedByWhatBlocksWhat(t *testing.T) {
	tree := split.Parse(squishyTree, split.Difficulties)
	assertTrue(t, tree.OK(), tree.Problems)
	keys := []string{}
	for _, i := range split.Ordered(tree.Issues) {
		keys = append(keys, i.Key)
	}
	assertEqual(t, keys, []string{"engine", "client"})
	client := tree.Issues[0]
	assertEqual(t, client.BlockedBy, []string{"engine"})
	assertEqual(t, client.BlockedByIssues, []int{77})
	assertEqual(t, split.FromDict(tree.ToDict(), split.Difficulties).ToDict(), tree.ToDict())
}

func TestSquishy_AHeaderWorksAsWellAsAFencedBlock(t *testing.T) {
	tree := split.Parse(`<!-- split: {"done": true, "summary": "It holds."} -->`, split.Difficulties)
	assertTrue(t, tree.OK())
	assertTrue(t, tree.Done)
}

func TestSquishy_ATreeThatCannotBeUsedSaysWhy(t *testing.T) {
	problems := func(raw *pyjson.Object) string {
		return strings.Join(split.FromDict(raw, split.Difficulties).Problems, " ")
	}
	assertTrue(t, len(split.Parse("no tree here", split.Difficulties).Problems) > 0)
	assertIn(t, "go round in a circle", problems(pyjson.New("issues", []any{
		pyjson.New("key", "a", "title", "A", "body", "a", "blocked_by", []any{"b"}),
		pyjson.New("key", "b", "title", "B", "body", "b", "blocked_by", []any{"a"})})))
	assertIn(t, "unknown key", problems(pyjson.New("issues", []any{
		pyjson.New("key", "a", "title", "A", "body", "a", "blocked_by", []any{"z"})})))
	assertIn(t, "needs a title and a body", problems(pyjson.New("issues", []any{pyjson.New("key", "a")})))
	assertIn(t, "`done` is true", problems(pyjson.New("done", true, "issues", []any{
		pyjson.New("key", "a", "title", "A", "body", "a")})))
	many := []any{}
	for i := range split.MaxIssues + 1 {
		many = append(many, pyjson.New("key", fmt.Sprintf("k%d", i), "title", "T", "body", "b"))
	}
	assertIn(t, "at most", problems(pyjson.New("issues", many)))
	assertIn(t, "no sub-issues", problems(pyjson.New("done", false, "issues", []any{})))
}

func TestSquishy_TheChecklistNamesEachOneAndWhatItWaitsFor(t *testing.T) {
	tree := split.Parse(squishyTree, split.Difficulties)
	ordered := split.Ordered(tree.Issues)
	engine, client := ordered[0], ordered[1]
	text := split.Checklist(40, []split.Made{{Issue: engine, Number: 41}, {Issue: client, Number: 42}}, "me",
		"Engine, then client.", "the run", "")
	assertIn(t, "- [ ] #41\n- [ ] #42 (after #41, #77)", text)
	assertIn(t, "Blocked by #41, #77.", split.Body(client, 40, "Squishy", []int{41, 77}, ""))
}

// ------------------------------------------------------------------ NightBotLeavesSquishyAloneTests
//
// The night bot never takes an issue that is Squishy's: its labels on it, or assigned to it.
// Squishy's process has the same rule the other way (squishy_cases_test.go).

func TestSquishy_ASquishyLabelKeepsItSquishys(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(5, fakes.Issue{Labels: []string{"squishy:oneshot", "squishy:build"}})
	reply := queue.QueueBuild(ctx, 5, "jgoetzmann", false, false, nil, nil)
	assertIn(t, "is Squishy's (@squishy-squooby)", reply)
	assertNotIn(t, config.LabelBuild, gh.LabelNames(5))
	gh.AddIssue(6, fakes.Issue{Labels: []string{"squishy:tree", config.LabelBuild}})
	skipped := []string{}
	assertEqual(t, queue.Candidates(ctx, ctx.Store.Load(), &skipped, nil), []*queue.Candidate{})
	assertIn(t, "#6 skipped: it is Squishy's", skipped)
}

func TestSquishy_AnOldPlannedLabelDoesNot(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(7, fakes.Issue{Labels: []string{"squishy:planned"}})
	assertIn(t, "Queued #7", queue.QueueBuild(ctx, 7, "jgoetzmann", false, false, nil, nil))
}

func TestSquishy_AssignedToSquishyKeepsItSquishys(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(8)
	gh.AddAssignees(8, []string{"squishy-squooby"})
	assertIn(t, "is Squishy's", queue.QueueBuild(ctx, 8, "jgoetzmann", false, false, nil, nil))
}

// ------------------------------------------------------------------ BotsDoNotTalkForEverTests
//
// The bots name each other in their replies; only a bare `stop` from the other bot is a command,
// and nothing answers it with a hint.

// squishyComment is BotsDoNotTalkForEverTests.comment: `user` comments `body` on `number`, and the
// bot hears it.
func squishyComment(ctx *hctx.Context, gh *fakes.FakeGitHub, number int, body string, user *pyjson.Object) []string {
	gh.AddComment(number, body, fakes.Comment{User: user})
	comments := gh.Comments.Get(number)
	comment := comments[len(comments)-1]
	return events.OnComment(ctx, pyjson.New("action", "created", "comment", comment,
		"issue", gh.GetIssue(number)), false)
}

func TestSquishy_SquishysReplyNamingTheNightBotIsNoRequest(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(9, fakes.Issue{Labels: []string{config.LabelBuild}})
	out := squishyComment(ctx, gh, 9, "@jgoetzmann-bot\n\n#9 is the night bot's (@jgoetzmann-bot): it "+
		"carries the night bot's labels.", squishyUser)
	assertEqual(t, out, []string{"no command"})
	assertEqual(t, gh.BotComments(9), []string{})
}

func TestSquishy_ABareStopFromSquishyStopsItsSubIssue(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(10, fakes.Issue{Labels: []string{config.LabelBuild}})
	squishyComment(ctx, gh, 10, "@jgoetzmann-bot stop\n\njgoetzmann stopped the tree this belongs to "+
		"(#4).", squishyUser)
	assertNotIn(t, config.LabelBuild, gh.LabelNames(10))
	assertTrue(t, ctx.Store.Load().Obj("items").Obj("10").Truthy("stop_requested"))
}

func TestSquishy_APersonStillGetsEveryCommand(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(11)
	squishyComment(ctx, gh, 11, "@jgoetzmann-bot make it blue", fakes.Operator)
	assertIn(t, config.LabelBuild, gh.LabelNames(11))
}

func TestSquishy_OneshotAndSplitAreSquishys(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(12)
	squishyComment(ctx, gh, 12, "/harness oneshot", fakes.Operator)
	comments := gh.BotComments(12)
	assertIn(t, "I have no `oneshot` mode. Squishy (@squishy-squooby) may", comments[len(comments)-1])
	assertNotIn(t, config.LabelBuild, gh.LabelNames(12))
}

// ------------------------------------------------------------------ StatusSectionTests
//
// Squishy shows in the pinned status issue in a section of its own, drawn by its own process and
// set apart from the night bot's facts.

func TestSquishy_TheSectionGoesAfterTheNightBotsFacts(t *testing.T) {
	ctx := testsupport.MakeCtx(fakes.NewFakeGitHub())
	body := dashboard.Render(ctx, []string{"<!-- s -->\n## 🫧 Squishy\n\nIts facts."}, "", nil, true)
	assertTrue(t, strIndex(t, body, "## 🏃 Last night-bot runs") < strIndex(t, body, "## 🫧 Squishy"))
	assertTrue(t, strIndex(t, body, "## 🫧 Squishy") < strIndex(t, body, "The full status"))
	assertIn(t, "---\n\n<!-- s -->", body)
}

// squishyCall is one subprocess.run the loop made: its argv and its env.
type squishyCall struct {
	argv []string
	env  map[string]string
}

// The command line a companion runs is `python -m harness <argv>` in the Python, so its tests read
// argv[3:]; the Go runs this binary, `<binary> <argv>`, so argv[1:] is the same words.
const squishyArgvFrom = 1

func TestSquishy_TheLoopDrawsSquishyInAProcessOfItsOwn(t *testing.T) {
	seen := [][]string{}
	calls := []squishyCall{}
	run := func(argv []string, opts cli.RunOptions) (*cli.Completed, error) {
		calls = append(calls, squishyCall{argv, opts.Env})
		out := "swept\n"
		for _, a := range argv {
			if a == "--section-only" {
				out = "## 🫧 Squishy\n"
			}
		}
		return &cli.Completed{Args: argv, Returncode: 0, Stdout: out, Stderr: ""}, nil
	}
	update := func(ctx *hctx.Context, extra []string, withStats bool, statsExtra func() []string, forceStats bool,
		readCards cardsets.Fetch) string {
		seen = append(seen, extra)
		return "rewrote #148"
	}
	var out bytes.Buffer
	for k, v := range map[string]string{"SQUISHY_GITHUB_TOKEN": strings.Repeat("s", 20),
		"BOT_GITHUB_TOKEN": strings.Repeat("n", 20), "HARNESS_HOME": ".harness"} {
		t.Setenv(k, v)
	}
	realRun, realSweep, realUpdate, realStdout := cli.SubprocessRun, cli.Sweep, dashboard.Update, cli.Stdout
	cli.SubprocessRun, dashboard.Update, cli.Stdout = run, update, &out
	cli.Sweep = func(ctx *hctx.Context) []string { return []string{} }
	dashboardOffline(t, nil)
	defer func() {
		cli.SubprocessRun, cli.Sweep, dashboard.Update, cli.Stdout = realRun, realSweep, realUpdate, realStdout
	}()
	cli.CmdDashboard(testsupport.MakeConfig(), &cli.Args{Sweep: true, Companions: true})
	assertEqual(t, seen, [][]string{{"## 🫧 Squishy\n"}})
	argvs := [][]string{}
	for _, c := range calls {
		argvs = append(argvs, c.argv[squishyArgvFrom:])
	}
	assertEqual(t, argvs, [][]string{{"sweep"}, {"dashboard", "--section-only"}})
	for _, c := range calls {
		assertEqual(t, c.env["HARNESS_HOME"], ".squishy")
		assertEqual(t, c.env["BOT_GITHUB_TOKEN"], strings.Repeat("s", 20)) // Squishy's, not the night bot's
	}
	assertIn(t, "Squishy: swept", out.String())
}

func TestSquishy_WithoutItsTokenSquishyIsDrawnButNeverSwept(t *testing.T) {
	other := config.Others[0]
	env := config.CompanionEnv(other, map[string]string{"BOT_GITHUB_TOKEN": strings.Repeat("n", 20), "PATH": "/bin"})
	assertNotIn(t, "BOT_GITHUB_TOKEN", env)
	assertEqual(t, env["HARNESS_HOME"], ".squishy")
}

func TestSquishy_NoCompanionsUnlessAsked(t *testing.T) {
	run := func(argv []string, opts cli.RunOptions) (*cli.Completed, error) {
		panic("AssertionError: no process should start")
	}
	update := func(ctx *hctx.Context, extra []string, withStats bool, statsExtra func() []string, forceStats bool,
		readCards cardsets.Fetch) string {
		return "rewrote #148"
	}
	realRun, realUpdate, realStdout := cli.SubprocessRun, dashboard.Update, cli.Stdout
	cli.SubprocessRun, dashboard.Update, cli.Stdout = run, update, &bytes.Buffer{}
	dashboardOffline(t, nil)
	defer func() { cli.SubprocessRun, dashboard.Update, cli.Stdout = realRun, realUpdate, realStdout }()
	assertEqual(t, cli.CmdDashboard(testsupport.MakeConfig(), &cli.Args{}), 0)
}

// ------------------------------------------------------------------ StatisticsSectionTests
//
// Squishy's record has a section of its own in the night bot's statistics, folded in the status
// issue, drawn by its own process, and only when the statistics are counted.

func TestSquishy_TheSectionGoesAfterTheNightBotsRecord(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day})
	drawn := []int{}
	extra := func() []string {
		drawn = append(drawn, 1)
		return []string{"<!-- s -->\n🫧 Squishy\n\nIts record."}
	}
	dashboard.Update(ctx, nil, true, extra, false, nil)
	issues := []*pyjson.Object{}
	for _, thread := range gh.Threads.Values() {
		if pyjson.Equal(thread.Get("title"), dashboard.Title) {
			issues = append(issues, thread)
		}
	}
	if len(issues) != 1 {
		t.Fatalf("not one status issue: %d", len(issues))
	}
	issue := issues[0]
	body := issue.Str("body")
	assertTrue(t, strIndex(t, body, "<b>All time</b>") < strIndex(t, body, "Its record."))
	assertTrue(t, strIndex(t, body, "Its record.") < strIndex(t, body, stats.BlockEnd))
	assertEqual(t, dashboard.Update(ctx, nil, true, extra, false, nil), fmt.Sprintf("rewrote #%d", issue.Int("number")))
	assertEqual(t, drawn, []int{1})                                              // not drawn again when nothing is counted
	assertIn(t, "Its record.", gh.Threads.Item(issue.Int("number")).Str("body")) // but kept
}

func TestSquishy_TheLoopDrawsSquishysRecordInAProcessOfItsOwn(t *testing.T) {
	calls := [][]string{}
	run := func(argv []string, opts cli.RunOptions) (*cli.Completed, error) {
		calls = append(calls, argv[squishyArgvFrom:])
		return &cli.Completed{Args: argv, Returncode: 0, Stdout: "## 🫧 Squishy\n", Stderr: ""}, nil
	}
	seen := [][]string{}
	update := func(ctx *hctx.Context, extra []string, withStats bool, statsExtra func() []string, forceStats bool,
		readCards cardsets.Fetch) string {
		seen = append(seen, statsExtra())
		return "rewrote #148"
	}
	realRun, realUpdate, realStdout := cli.SubprocessRun, dashboard.Update, cli.Stdout
	cli.SubprocessRun, dashboard.Update, cli.Stdout = run, update, &bytes.Buffer{}
	dashboardOffline(t, nil)
	defer func() { cli.SubprocessRun, dashboard.Update, cli.Stdout = realRun, realUpdate, realStdout }()
	cli.CmdDashboard(testsupport.MakeConfig(), &cli.Args{Stats: true, Companions: true})
	assertEqual(t, seen, [][]string{{"## 🫧 Squishy\n"}})
	assertIn(t, []string{"stats", "--section-only"}, calls)
	assertIn(t, []string{"dashboard", "--section-only"}, calls)
}

func TestSquishy_SquishysRunsAndModesAreReadFromItsComments(t *testing.T) {
	run := "https://github.com/Jlore-Studios/JackiOh/actions/runs/5"
	start := stats.Parse(pyjson.New("body", "Starting a one-shot build of this now ([run]("+run+")), on "+
		"`claude-squishy` (claude, `opus`, strong)", "issue_url", "x/7"))
	assertEqual(t, []string{start.Kind, start.Action, start.Provider}, []string{"start", "oneshot", "claude-squishy"})
	assertEqual(t, stats.Parse(pyjson.New("body", "Splitting this now ([run]("+run+"))")).Action, "split")
	split_ := stats.Parse(pyjson.New("body", "Split #40 into 3 sub-issue(s) ([run]("+run+")), queued for me"))
	assertEqual(t, split_.Kind, "split")
	assertEqual(t, split_.Count, 3)
	done := stats.Parse(pyjson.New("body", "Every sub-issue of this has closed, and its end state holds "+
		"on `main` (the run), so I am closing it."))
	assertEqual(t, done.Kind, "closed tree")
}

// ------------------------------------------------------------------ SquishyWorkflowTests

func TestSquishy_EveryJobRunsAsSquishy(t *testing.T) {
	for _, text := range []string{workflowsRead(t, "squishy-run.yml"), workflowsRead(t, "squishy-commands.yml")} {
		assertIn(t, "  HARNESS_HOME: .squishy\n", text)
		assertNotIn(t, "secrets.BOT_GITHUB_TOKEN", text)
		assertIn(t, "BOT_GITHUB_TOKEN: ${{ secrets.SQUISHY_GITHUB_TOKEN }}", text)
	}
}

func TestSquishy_ClaudeRunsOnGithubsRunnersWithOnlyItsOwnSecret(t *testing.T) {
	runText := workflowsRead(t, "squishy-run.yml")
	work := workflowsJob(t, runText, "work")
	assertIn(t, "runs-on: ubuntu-latest", work)
	assertNotIn(t, "night-vm", runText)
	assertNotIn(t, "BOT_GITHUB_TOKEN", work)
	secrets := map[string]bool{}
	for _, m := range regexp.MustCompile(`\$\{\{\s*(secrets[^}]*?)\s*\}\}`).FindAllStringSubmatch(work, -1) {
		secrets[m[1]] = true
	}
	assertEqual(t, secrets, setOf("secrets[needs.plan.outputs.secret]"))
	block := regexp.MustCompile(`permissions:\n((?:\s{6}.*\n)+)`).FindStringSubmatch(work)
	if block == nil {
		t.Fatal("no permissions block in the work job")
	}
	grants := map[string]string{}
	for _, m := range regexp.MustCompile(`(?m)^\s+([\w-]+):\s*(\w+)`).FindAllStringSubmatch(block[1], -1) {
		grants[m[1]] = m[2]
	}
	assertEqual(t, grants, map[string]string{"contents": "read"})
	// Never the step name bright-bots-harness reads as the shared subscription's spending.
	assertNotIn(t, "name: Build, check and review\n", runText)
	assertIn(t, "group: squishy-run-plan", workflowsJob(t, runText, "plan"))
	assertIn(t, ".squishy/HALT", workflowsJob(t, runText, "gate"))
	assertIn(t, ".squishy/HALT", workflowsJob(t, runText, "plan"))
}

func TestSquishy_EverySecretListNamesEverySecret(t *testing.T) {
	lists := regexp.MustCompile(`HARNESS_SECRETS_SET: >-\n((?:\s+\$\{\{.*\}\}\n)+)`)
	named := regexp.MustCompile(`secrets\.(\w+) != '' && '(\w+)'`)
	for _, name := range []string{"squishy-run.yml", "squishy-commands.yml", "bot-status.yml"} {
		for _, found := range lists.FindAllStringSubmatch(workflowsRead(t, name), -1) {
			secrets := []string{}
			for _, m := range named.FindAllStringSubmatch(found[1], -1) {
				secrets = append(secrets, m[1])
			}
			assertEqual(t, secrets, providers.SecretNames, name)
		}
	}
}

func TestSquishy_ItsEarsHearOnlySquishy(t *testing.T) {
	text := workflowsRead(t, "squishy-commands.yml")
	for _, wanted := range []string{"github.actor != 'squishy-squooby'", "'/squishy'", "'@squishy-squooby'",
		"startsWith(github.event.label.name, 'squishy:')",
		"github.event.assignee.login == 'squishy-squooby'",
		"startsWith(github.event.workflow_run.head_branch, 'squishy/')",
		"contains(github.event.pull_request.labels.*.name, 'squishy:pr')"} {
		assertIn(t, wanted, text)
	}
	assertNotIn(t, "'bot:", text)
	assertNotIn(t, "secrets[", text)
	assertNotIn(t, "claude-code", text)
}

func TestSquishy_TheStatusLoopRunsSquishyBesideTheNightBot(t *testing.T) {
	refresh := workflowsJob(t, workflowsRead(t, "bot-status.yml"), "refresh")
	assertIn(t, "SQUISHY_GITHUB_TOKEN: ${{ secrets.SQUISHY_GITHUB_TOKEN }}", refresh)
	assertIn(t, "--companions", refresh)
	// The Python reads `python -m harness dashboard --companions`: the workflows run the Go
	// binary now (`.github/actions/harness`), whose command line is `harness …`.
	assertIn(t, "harness dashboard --companions", workflowsRead(t, "bot-commands.yml"))
}

func TestSquishy_SelftestLintsAndLoadsSquishy(t *testing.T) {
	text := workflowsSelftest(t)
	// The Python reads `HARNESS_HOME=.squishy python3.13 -m harness window`: the selftest loads
	// Squishy with the Go binary now.
	assertIn(t, "HARNESS_HOME=.squishy harness window", text)
	for _, name := range []string{"squishy-run.yml", "squishy-commands.yml"} {
		assertTrue(t, workflowsLinted(t, name), name)
	}
}

func TestSquishy_FullsendIsInTheRepository(t *testing.T) {
	skill := filepath.Join(config.RepoRoot, ".claude", "skills", "fullsend")
	for _, name := range []string{"SKILL.md", "LICENSE", "agents/builder.md", "agents/spec-tester.md",
		"agents/reconciler.md", "agents/culler.md"} {
		info, err := os.Stat(filepath.Join(skill, name))
		assertTrue(t, err == nil && info.Mode().IsRegular(), name)
	}
	assertIn(t, "v2.2.0", readText(t, filepath.Join(skill, "SOURCE.md")))
}
