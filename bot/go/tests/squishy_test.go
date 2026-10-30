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

// ------------------------------------------------------------------ what test_squishy imports
//
// From test_dashboard (offline) and test_workflows (read, job, selftest, linted), as this file's
// own: each test file of package tests keeps its helpers.

// squishyOffline is offline() (test_dashboard): the loop's contexts on a fake GitHub, so a test
// never reaches the network, until the test ends.
func squishyOffline(t *testing.T, gh ...*fakes.FakeGitHub) {
	t.Helper()
	fake := fakes.NewFakeGitHub()
	if len(gh) > 0 && gh[0] != nil {
		fake = gh[0]
	}
	real := cli.Ctx
	cli.Ctx = func(cfg *config.Config, write bool) *hctx.Context {
		return testsupport.MakeCtx(fake, testsupport.Ctx{At: testsupport.Night, Cfg: cfg})
	}
	t.Cleanup(func() { cli.Ctx = real })
}

// WORKFLOWS (test_workflows)
var squishyWorkflows = filepath.Join(config.RepoRoot, ".github", "workflows")

// squishyRead is read(name) (test_workflows).
func squishyRead(t *testing.T, name string) string {
	t.Helper()
	return basicsReadFile(t, filepath.Join(squishyWorkflows, name))
}

// The next job at a job's indent: Python's lookahead `(?=^  [a-z][\w-]*:\n|\Z)`, which Go's
// regexp has not, found after the job's own line.
var squishyNextJob = regexp.MustCompile(`(?m)^  [a-z][\w-]*:\n`)

// squishyJob is job(text, name) (test_workflows): the block of one job: from `  name:` to the next
// job at the same indent.
func squishyJob(t *testing.T, text, name string) string {
	t.Helper()
	start := regexp.MustCompile(`(?m)^  ` + regexp.QuoteMeta(name) + `:\n`).FindStringIndex(text)
	if start == nil {
		t.Fatalf("no job %s", name)
	}
	rest := text[start[1]:]
	if next := squishyNextJob.FindStringIndex(rest); next != nil {
		return rest[:next[0]]
	}
	return rest
}

// squishySelftest is selftest() (test_workflows): the `bot selftest` job's block: a job of CI's
// since #1455, which had a workflow of its own.
func squishySelftest(t *testing.T) string {
	t.Helper()
	return squishyJob(t, squishyRead(t, "ci.yml"), "bot-selftest")
}

// ACTIONLINT_EVERY_WORKFLOW (test_workflows)
const squishyActionlintEveryWorkflow = "rhysd/actionlint:1.7.7 -color $(ls .github/workflows/*.yml)"

// squishyLinted is linted(name) (test_workflows): whether bot selftest's actionlint reads the
// workflow `name`: it reads every one there is.
func squishyLinted(t *testing.T, name string) bool {
	t.Helper()
	info, err := os.Stat(filepath.Join(squishyWorkflows, name))
	return strings.Contains(squishySelftest(t), squishyActionlintEveryWorkflow) && err == nil && info.Mode().IsRegular()
}

// squishyIndex is str.index: where `sub` is in `text`, the test failing (Python's ValueError) when
// it is not.
func squishyIndex(t *testing.T, text, sub string) int {
	t.Helper()
	i := strings.Index(text, sub)
	if i < 0 {
		t.Fatalf("substring not found: %q", sub)
	}
	return i
}

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
	basicsTrue(t, err == nil, fmt.Sprintf("%v\n%s", err, text))
	ran := regexp.MustCompile(`(?m)^--- PASS: TestSquishyCases_`).FindAllString(text, -1)
	basicsTrue(t, len(ran) > 0, "no test ran:\n"+text)
	basicsNotIn(t, "--- SKIP", text)
}

// ------------------------------------------------------------------ IdentityTests

func TestSquishy_ThisProcessIsTheNightBot(t *testing.T) {
	basicsEqual(t, config.Home, ".harness")
	basicsEqual(t, []string{config.LabelBuild, config.Slash, config.StateBranch, config.Marker},
		[]string{"bot:build", "/harness", "bot-state", "<!-- jackioh-bot -->"})
	basicsEqual(t, config.Modes, []string{"fullsend"})
	logins := []string{}
	for _, o := range config.Others {
		logins = append(logins, o.Login)
	}
	basicsEqual(t, logins, []string{"squishy-squooby"})
	basicsEqual(t, queue.BranchForIssue(7), "bot/issue-7")
}

func TestSquishy_SquishysConfigSaysWhoItIs(t *testing.T) {
	found, err := identity.Load(config.RepoRoot, ".squishy")
	if err != nil {
		t.Fatal(err)
	}
	basicsEqual(t, []string{found.Name, found.Command, found.LabelPrefix, found.BranchPrefix},
		[]string{"Squishy", "squishy", "squishy:", "squishy/"})
	basicsEqual(t, []string{found.StateBranch, found.JournalBranch, found.Workflow},
		[]string{"squishy-state", "squishy-journal", "squishy-run.yml"})
	basicsEqual(t, found.Modes, []string{"oneshot", "split", "split-bot", "fullsend"})
	basicsFalse(t, found.Suggestions)
	logins := []string{}
	for _, o := range found.Others {
		logins = append(logins, o.Login)
	}
	basicsEqual(t, logins, []string{"jgoetzmann-bot"})
	basicsNotEqual(t, found.Marker, config.Marker)
}

func TestSquishy_SquishysSubscriptionIsItsOwnOnGithubsRunners(t *testing.T) {
	raw := basicsLoadObject(t, basicsReadFile(t, filepath.Join(config.RepoRoot, ".squishy", "providers.json")))
	pool := providers.Parse(raw)
	basicsEqual(t, pool.Providers.Keys(), []string{"claude-squishy"})
	provider := pool.Get("claude-squishy")
	basicsEqual(t, []string{provider.CLI, provider.Model, provider.Tier, provider.RunsOn},
		[]string{"claude", "opus", "strong", "ubuntu-latest"})
	basicsEqual(t, provider.Secret, "CLAUDE_CODE_OAUTH_TOKEN_SQUISHY")
	basicsNotIn(t, "suggest", provider.Roles)
	basicsEqual(t, [2]int{pool.MachineParallel, pool.PlanLanes}, [2]int{0, 0})
	night := map[string]bool{}
	testsupport.MakeConfig().Pool.Providers.Items(func(_ string, p any) bool {
		night[p.(*providers.Provider).Secret] = true
		return true
	})
	basicsNotIn(t, provider.Secret, night)
}

func TestSquishy_BothConfigsKeepEachOthersSwitchesOutOfReach(t *testing.T) {
	for _, home := range []string{".harness", ".squishy"} {
		raw := basicsLoadObject(t, basicsReadFile(t, filepath.Join(config.RepoRoot, home, "config.json")))
		for _, path := range []string{".github/", ".harness/", ".squishy/", "bot/", ".claude/"} {
			basicsIn(t, path, pyjson.Strings(raw.Get("forbidden_paths")), home)
		}
	}
}

func TestSquishy_ABadIdentityIsRefused(t *testing.T) {
	for _, raw := range []*pyjson.Object{pyjson.New("label_prefix", "squishy"),
		pyjson.New("modes", []any{"teleport"}), pyjson.New("colour", "blue")} {
		_, err := identity.Parse(raw, true)
		basicsTrue(t, err != nil, "ValueError not raised: ", pyjson.Dumps(raw))
	}
	found, err := identity.Parse(nil, true)
	basicsTrue(t, err == nil)
	basicsEqual(t, found, identity.NewIdentity())
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
	basicsTrue(t, tree.OK(), tree.Problems)
	keys := []string{}
	for _, i := range split.Ordered(tree.Issues) {
		keys = append(keys, i.Key)
	}
	basicsEqual(t, keys, []string{"engine", "client"})
	client := tree.Issues[0]
	basicsEqual(t, client.BlockedBy, []string{"engine"})
	basicsEqual(t, client.BlockedByIssues, []int{77})
	basicsEqual(t, split.FromDict(tree.ToDict(), split.Difficulties).ToDict(), tree.ToDict())
}

func TestSquishy_AHeaderWorksAsWellAsAFencedBlock(t *testing.T) {
	tree := split.Parse(`<!-- split: {"done": true, "summary": "It holds."} -->`, split.Difficulties)
	basicsTrue(t, tree.OK())
	basicsTrue(t, tree.Done)
}

func TestSquishy_ATreeThatCannotBeUsedSaysWhy(t *testing.T) {
	problems := func(raw *pyjson.Object) string {
		return strings.Join(split.FromDict(raw, split.Difficulties).Problems, " ")
	}
	basicsTrue(t, len(split.Parse("no tree here", split.Difficulties).Problems) > 0)
	basicsIn(t, "go round in a circle", problems(pyjson.New("issues", []any{
		pyjson.New("key", "a", "title", "A", "body", "a", "blocked_by", []any{"b"}),
		pyjson.New("key", "b", "title", "B", "body", "b", "blocked_by", []any{"a"})})))
	basicsIn(t, "unknown key", problems(pyjson.New("issues", []any{
		pyjson.New("key", "a", "title", "A", "body", "a", "blocked_by", []any{"z"})})))
	basicsIn(t, "needs a title and a body", problems(pyjson.New("issues", []any{pyjson.New("key", "a")})))
	basicsIn(t, "`done` is true", problems(pyjson.New("done", true, "issues", []any{
		pyjson.New("key", "a", "title", "A", "body", "a")})))
	many := []any{}
	for i := range split.MaxIssues + 1 {
		many = append(many, pyjson.New("key", fmt.Sprintf("k%d", i), "title", "T", "body", "b"))
	}
	basicsIn(t, "at most", problems(pyjson.New("issues", many)))
	basicsIn(t, "no sub-issues", problems(pyjson.New("done", false, "issues", []any{})))
}

func TestSquishy_TheChecklistNamesEachOneAndWhatItWaitsFor(t *testing.T) {
	tree := split.Parse(squishyTree, split.Difficulties)
	ordered := split.Ordered(tree.Issues)
	engine, client := ordered[0], ordered[1]
	text := split.Checklist(40, []split.Made{{Issue: engine, Number: 41}, {Issue: client, Number: 42}}, "me",
		"Engine, then client.", "the run", "")
	basicsIn(t, "- [ ] #41\n- [ ] #42 (after #41, #77)", text)
	basicsIn(t, "Blocked by #41, #77.", split.Body(client, 40, "Squishy", []int{41, 77}, ""))
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
	basicsIn(t, "is Squishy's (@squishy-squooby)", reply)
	basicsNotIn(t, config.LabelBuild, gh.LabelNames(5))
	gh.AddIssue(6, fakes.Issue{Labels: []string{"squishy:tree", config.LabelBuild}})
	skipped := []string{}
	basicsEqual(t, queue.Candidates(ctx, ctx.Store.Load(), &skipped, nil), []*queue.Candidate{})
	basicsIn(t, "#6 skipped: it is Squishy's", skipped)
}

func TestSquishy_AnOldPlannedLabelDoesNot(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(7, fakes.Issue{Labels: []string{"squishy:planned"}})
	basicsIn(t, "Queued #7", queue.QueueBuild(ctx, 7, "jgoetzmann", false, false, nil, nil))
}

func TestSquishy_AssignedToSquishyKeepsItSquishys(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(8)
	gh.AddAssignees(8, []string{"squishy-squooby"})
	basicsIn(t, "is Squishy's", queue.QueueBuild(ctx, 8, "jgoetzmann", false, false, nil, nil))
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
	basicsEqual(t, out, []string{"no command"})
	basicsEqual(t, gh.BotComments(9), []string{})
}

func TestSquishy_ABareStopFromSquishyStopsItsSubIssue(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(10, fakes.Issue{Labels: []string{config.LabelBuild}})
	squishyComment(ctx, gh, 10, "@jgoetzmann-bot stop\n\njgoetzmann stopped the tree this belongs to "+
		"(#4).", squishyUser)
	basicsNotIn(t, config.LabelBuild, gh.LabelNames(10))
	basicsTrue(t, ctx.Store.Load().Obj("items").Obj("10").Truthy("stop_requested"))
}

func TestSquishy_APersonStillGetsEveryCommand(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(11)
	squishyComment(ctx, gh, 11, "@jgoetzmann-bot make it blue", fakes.Operator)
	basicsIn(t, config.LabelBuild, gh.LabelNames(11))
}

func TestSquishy_OneshotAndSplitAreSquishys(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	gh.AddIssue(12)
	squishyComment(ctx, gh, 12, "/harness oneshot", fakes.Operator)
	comments := gh.BotComments(12)
	basicsIn(t, "I have no `oneshot` mode. Squishy (@squishy-squooby) may", comments[len(comments)-1])
	basicsNotIn(t, config.LabelBuild, gh.LabelNames(12))
}

// ------------------------------------------------------------------ StatusSectionTests
//
// Squishy shows in the pinned status issue in a section of its own, drawn by its own process and
// set apart from the night bot's facts.

func TestSquishy_TheSectionGoesAfterTheNightBotsFacts(t *testing.T) {
	ctx := testsupport.MakeCtx(fakes.NewFakeGitHub())
	body := dashboard.Render(ctx, []string{"<!-- s -->\n## 🫧 Squishy\n\nIts facts."}, "", nil, true)
	basicsTrue(t, squishyIndex(t, body, "## 🏃 Last night-bot runs") < squishyIndex(t, body, "## 🫧 Squishy"))
	basicsTrue(t, squishyIndex(t, body, "## 🫧 Squishy") < squishyIndex(t, body, "The full status"))
	basicsIn(t, "---\n\n<!-- s -->", body)
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
	squishyOffline(t)
	defer func() {
		cli.SubprocessRun, cli.Sweep, dashboard.Update, cli.Stdout = realRun, realSweep, realUpdate, realStdout
	}()
	cli.CmdDashboard(testsupport.MakeConfig(), &cli.Args{Sweep: true, Companions: true})
	basicsEqual(t, seen, [][]string{{"## 🫧 Squishy\n"}})
	argvs := [][]string{}
	for _, c := range calls {
		argvs = append(argvs, c.argv[squishyArgvFrom:])
	}
	basicsEqual(t, argvs, [][]string{{"sweep"}, {"dashboard", "--section-only"}})
	for _, c := range calls {
		basicsEqual(t, c.env["HARNESS_HOME"], ".squishy")
		basicsEqual(t, c.env["BOT_GITHUB_TOKEN"], strings.Repeat("s", 20)) // Squishy's, not the night bot's
	}
	basicsIn(t, "Squishy: swept", out.String())
}

func TestSquishy_WithoutItsTokenSquishyIsDrawnButNeverSwept(t *testing.T) {
	other := config.Others[0]
	env := config.CompanionEnv(other, map[string]string{"BOT_GITHUB_TOKEN": strings.Repeat("n", 20), "PATH": "/bin"})
	basicsNotIn(t, "BOT_GITHUB_TOKEN", env)
	basicsEqual(t, env["HARNESS_HOME"], ".squishy")
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
	squishyOffline(t)
	defer func() { cli.SubprocessRun, dashboard.Update, cli.Stdout = realRun, realUpdate, realStdout }()
	basicsEqual(t, cli.CmdDashboard(testsupport.MakeConfig(), &cli.Args{}), 0)
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
	basicsTrue(t, squishyIndex(t, body, "<b>All time</b>") < squishyIndex(t, body, "Its record."))
	basicsTrue(t, squishyIndex(t, body, "Its record.") < squishyIndex(t, body, stats.BlockEnd))
	basicsEqual(t, dashboard.Update(ctx, nil, true, extra, false, nil), fmt.Sprintf("rewrote #%d", issue.Int("number")))
	basicsEqual(t, drawn, []int{1})                                              // not drawn again when nothing is counted
	basicsIn(t, "Its record.", gh.Threads.Item(issue.Int("number")).Str("body")) // but kept
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
	squishyOffline(t)
	defer func() { cli.SubprocessRun, dashboard.Update, cli.Stdout = realRun, realUpdate, realStdout }()
	cli.CmdDashboard(testsupport.MakeConfig(), &cli.Args{Stats: true, Companions: true})
	basicsEqual(t, seen, [][]string{{"## 🫧 Squishy\n"}})
	basicsIn(t, []string{"stats", "--section-only"}, calls)
	basicsIn(t, []string{"dashboard", "--section-only"}, calls)
}

func TestSquishy_SquishysRunsAndModesAreReadFromItsComments(t *testing.T) {
	run := "https://github.com/Jlore-Studios/JackiOh/actions/runs/5"
	start := stats.Parse(pyjson.New("body", "Starting a one-shot build of this now ([run]("+run+")), on "+
		"`claude-squishy` (claude, `opus`, strong)", "issue_url", "x/7"))
	basicsEqual(t, []string{start.Kind, start.Action, start.Provider}, []string{"start", "oneshot", "claude-squishy"})
	basicsEqual(t, stats.Parse(pyjson.New("body", "Splitting this now ([run]("+run+"))")).Action, "split")
	split_ := stats.Parse(pyjson.New("body", "Split #40 into 3 sub-issue(s) ([run]("+run+")), queued for me"))
	basicsEqual(t, split_.Kind, "split")
	basicsEqual(t, split_.Count, 3)
	done := stats.Parse(pyjson.New("body", "Every sub-issue of this has closed, and its end state holds "+
		"on `main` (the run), so I am closing it."))
	basicsEqual(t, done.Kind, "closed tree")
}

// ------------------------------------------------------------------ SquishyWorkflowTests

func TestSquishy_EveryJobRunsAsSquishy(t *testing.T) {
	for _, text := range []string{squishyRead(t, "squishy-run.yml"), squishyRead(t, "squishy-commands.yml")} {
		basicsIn(t, "  HARNESS_HOME: .squishy\n", text)
		basicsNotIn(t, "secrets.BOT_GITHUB_TOKEN", text)
		basicsIn(t, "BOT_GITHUB_TOKEN: ${{ secrets.SQUISHY_GITHUB_TOKEN }}", text)
	}
}

func TestSquishy_ClaudeRunsOnGithubsRunnersWithOnlyItsOwnSecret(t *testing.T) {
	runText := squishyRead(t, "squishy-run.yml")
	work := squishyJob(t, runText, "work")
	basicsIn(t, "runs-on: ubuntu-latest", work)
	basicsNotIn(t, "night-vm", runText)
	basicsNotIn(t, "BOT_GITHUB_TOKEN", work)
	secrets := map[string]bool{}
	for _, m := range regexp.MustCompile(`\$\{\{\s*(secrets[^}]*?)\s*\}\}`).FindAllStringSubmatch(work, -1) {
		secrets[m[1]] = true
	}
	basicsEqual(t, secrets, basicsSet("secrets[needs.plan.outputs.secret]"))
	block := regexp.MustCompile(`permissions:\n((?:\s{6}.*\n)+)`).FindStringSubmatch(work)
	if block == nil {
		t.Fatal("no permissions block in the work job")
	}
	grants := map[string]string{}
	for _, m := range regexp.MustCompile(`(?m)^\s+([\w-]+):\s*(\w+)`).FindAllStringSubmatch(block[1], -1) {
		grants[m[1]] = m[2]
	}
	basicsEqual(t, grants, map[string]string{"contents": "read"})
	// Never the step name bright-bots-harness reads as the shared subscription's spending.
	basicsNotIn(t, "name: Build, check and review\n", runText)
	basicsIn(t, "group: squishy-run-plan", squishyJob(t, runText, "plan"))
	basicsIn(t, ".squishy/HALT", squishyJob(t, runText, "gate"))
	basicsIn(t, ".squishy/HALT", squishyJob(t, runText, "plan"))
}

func TestSquishy_EverySecretListNamesEverySecret(t *testing.T) {
	lists := regexp.MustCompile(`HARNESS_SECRETS_SET: >-\n((?:\s+\$\{\{.*\}\}\n)+)`)
	named := regexp.MustCompile(`secrets\.(\w+) != '' && '(\w+)'`)
	for _, name := range []string{"squishy-run.yml", "squishy-commands.yml", "bot-status.yml"} {
		for _, found := range lists.FindAllStringSubmatch(squishyRead(t, name), -1) {
			secrets := []string{}
			for _, m := range named.FindAllStringSubmatch(found[1], -1) {
				secrets = append(secrets, m[1])
			}
			basicsEqual(t, secrets, providers.SecretNames, name)
		}
	}
}

func TestSquishy_ItsEarsHearOnlySquishy(t *testing.T) {
	text := squishyRead(t, "squishy-commands.yml")
	for _, wanted := range []string{"github.actor != 'squishy-squooby'", "'/squishy'", "'@squishy-squooby'",
		"startsWith(github.event.label.name, 'squishy:')",
		"github.event.assignee.login == 'squishy-squooby'",
		"startsWith(github.event.workflow_run.head_branch, 'squishy/')",
		"contains(github.event.pull_request.labels.*.name, 'squishy:pr')"} {
		basicsIn(t, wanted, text)
	}
	basicsNotIn(t, "'bot:", text)
	basicsNotIn(t, "secrets[", text)
	basicsNotIn(t, "claude-code", text)
}

func TestSquishy_TheStatusLoopRunsSquishyBesideTheNightBot(t *testing.T) {
	refresh := squishyJob(t, squishyRead(t, "bot-status.yml"), "refresh")
	basicsIn(t, "SQUISHY_GITHUB_TOKEN: ${{ secrets.SQUISHY_GITHUB_TOKEN }}", refresh)
	basicsIn(t, "--companions", refresh)
	// The Python reads `python -m harness dashboard --companions`: the workflows run the Go
	// binary now (`.github/actions/harness`), whose command line is `harness …`.
	basicsIn(t, "harness dashboard --companions", squishyRead(t, "bot-commands.yml"))
}

func TestSquishy_SelftestLintsAndLoadsSquishy(t *testing.T) {
	text := squishySelftest(t)
	// The Python reads `HARNESS_HOME=.squishy python3.13 -m harness window`: the selftest loads
	// Squishy with the Go binary now.
	basicsIn(t, "HARNESS_HOME=.squishy harness window", text)
	for _, name := range []string{"squishy-run.yml", "squishy-commands.yml"} {
		basicsTrue(t, squishyLinted(t, name), name)
	}
}

func TestSquishy_FullsendIsInTheRepository(t *testing.T) {
	skill := filepath.Join(config.RepoRoot, ".claude", "skills", "fullsend")
	for _, name := range []string{"SKILL.md", "LICENSE", "agents/builder.md", "agents/spec-tester.md",
		"agents/reconciler.md", "agents/culler.md"} {
		info, err := os.Stat(filepath.Join(skill, name))
		basicsTrue(t, err == nil && info.Mode().IsRegular(), name)
	}
	basicsIn(t, "v2.2.0", basicsReadFile(t, filepath.Join(skill, "SOURCE.md")))
}
