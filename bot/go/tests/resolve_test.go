package tests

// test_resolve.py: the conflicts the harness resolves itself (`harness/resolve.py`, `resolvers`
// in the config): against real merges in real git repositories, and through the model job with
// a scripted model.

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gates"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/prompts"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/resolve"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/threads"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

const (
	resolveIndex  = "spec/INDEX.md"
	resolveGolden = "crates/engine/tests/golden/games.jsonl"
	resolveGame   = "src/game.txt"
	resolveBless  = "printf 'blessed\\n' > " + resolveGolden
)

// resolveNone stands for Python's None in a file map: the file is deleted.
const resolveNone = "\x00None"

func resolveTakeIndex() *config.Resolver {
	return &config.Resolver{Path: resolveIndex, Take: "main", TimeoutMinutes: 10}
}

func resolveRunGolden() *config.Resolver {
	return &config.Resolver{Path: resolveGolden, Run: resolveBless, TimeoutMinutes: 1}
}

// resolveRun is Resolver(path, run=command), with its default ten minutes.
func resolveRun(path, command string) *config.Resolver {
	return &config.Resolver{Path: path, Run: command, TimeoutMinutes: 10}
}

// resolveBaseNames is BASE's order.
var resolveBaseNames = []string{"README.md", resolveGame, resolveIndex, resolveGolden}

func resolveBase() map[string]string {
	return map[string]string{"README.md": "hello\n", resolveGame: "rules v1\n", resolveIndex: "| R1 |\n",
		resolveGolden: "game 1\n"}
}

func resolveLotsOfTime() float64 { return 3600.0 }

// resolveConflicted is conflicted: a clone of a repository whose `feature` branch and `main` both
// changed from BASE, with `main` merged into `feature` and the merge left open. A resolveNone
// deletes the file.
func resolveConflicted(root string, branch, main map[string]string) *git.Git {
	work := filepath.Join(root, "repo")
	if err := os.MkdirAll(work, 0o777); err != nil {
		panic(err)
	}
	fakes.Git(work, "init", "-q", "-b", "main")
	base := resolveBase()
	for _, name := range resolveBaseNames {
		crossPut(filepath.Join(work, name), base[name])
	}
	fakes.Git(work, "add", "-A")
	fakes.Git(work, "commit", "-q", "-m", "base")

	commit := func(changes map[string]string, message string) {
		names := []string{}
		for name := range changes {
			names = append(names, name)
		}
		sort.Strings(names)
		for _, name := range names {
			if changes[name] == resolveNone {
				fakes.Git(work, "rm", "-q", name)
			} else if err := os.WriteFile(filepath.Join(work, name), []byte(changes[name]), 0o666); err != nil {
				panic(err)
			}
		}
		fakes.Git(work, "add", "-A")
		fakes.Git(work, "commit", "-q", "-m", message)
	}

	fakes.Git(work, "checkout", "-q", "-b", "feature")
	commit(branch, "the branch")
	fakes.Git(work, "checkout", "-q", "main")
	commit(main, "main moves")
	fakes.Git(work, "checkout", "-q", "feature")
	repo := git.NewGit(work, nil)
	repo.Run(false, 600, map[string]string{"GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t",
		"GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@t"}, "merge", "--no-edit", "--no-ff", "main")
	return repo
}

func resolveUnmerged(repo *git.Git) []string {
	out := []string{}
	for _, p := range py.Splitlines(repo.Out(true, "diff", "--name-only", "--diff-filter=U")) {
		if p != "" {
			out = append(out, p)
		}
	}
	return out
}

// resolveOpts is run_resolve's keyword arguments: seconds_left (nil: LOTS_OF_TIME), commands
// (NoCommands is commands=False), reserve_seconds and base ("" is main).
type resolveOpts struct {
	SecondsLeft    func() float64
	NoCommands     bool
	ReserveSeconds float64
	Base           string
}

// resolveRunResolve is ResolveTests.run_resolve; nil resolvers are (TAKE_INDEX, RUN_GOLDEN).
func resolveRunResolve(repo *git.Git, resolvers []*config.Resolver, opts ...resolveOpts) ([]string, []*resolve.Resolution) {
	if resolvers == nil {
		resolvers = []*config.Resolver{resolveTakeIndex(), resolveRunGolden()}
	}
	var o resolveOpts
	if len(opts) > 0 {
		o = opts[0]
	}
	secondsLeft := o.SecondsLeft
	if secondsLeft == nil {
		secondsLeft = resolveLotsOfTime
	}
	base := o.Base
	if base == "" {
		base = "main"
	}
	return resolve.Resolve(repo, resolveUnmerged(repo), resolvers, map[string]string{"PATH": "/usr/bin:/bin"},
		secondsLeft, !o.NoCommands, o.ReserveSeconds, base)
}

func resolveBytes(t *testing.T, path string) []byte {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	return data
}

func resolvePathsOK(done []*resolve.Resolution) []any {
	out := []any{}
	for _, r := range done {
		out = append(out, []any{r.Path, r.OK})
	}
	return out
}

// ---------------------------------------------------------------------------- ResolveTests

func TestResolve_TakeMainTakesMainsVersionAndStagesIt(t *testing.T) {
	root := crossMkdtemp(t)
	repo := resolveConflicted(root, map[string]string{resolveIndex: "| R1 |\n| R2 branch |\n"},
		map[string]string{resolveIndex: "| R1 |\n| R3 main |\n"})
	assertEqual(t, resolveUnmerged(repo), []string{resolveIndex})
	left, done := resolveRunResolve(repo, nil)
	assertEqual(t, left, []string{})
	assertEqual(t, resolvePathsOK(done), []any{[]any{resolveIndex, true}})
	assertEqual(t, readText(t, filepath.Join(repo.Cwd, resolveIndex)), "| R1 |\n| R3 main |\n")
	assertEqual(t, resolveUnmerged(repo), []string{})
}

func TestResolve_TakeMainFollowsMainWhenItDeletedTheFile(t *testing.T) {
	root := crossMkdtemp(t)
	repo := resolveConflicted(root, map[string]string{resolveIndex: "| R1 |\n| R2 branch |\n"},
		map[string]string{resolveIndex: resolveNone})
	assertEqual(t, resolveUnmerged(repo), []string{resolveIndex})
	left, done := resolveRunResolve(repo, nil)
	assertEqual(t, left, []string{})
	assertIn(t, "deleted", done[0].How)
	assertFalse(t, exists(filepath.Join(repo.Cwd, resolveIndex)))
	assertEqual(t, resolveUnmerged(repo), []string{})
}

func TestResolve_ACommandRewritesTheFileOnceNothingElseIsConflicted(t *testing.T) {
	root := crossMkdtemp(t)
	repo := resolveConflicted(root, map[string]string{resolveIndex: "| R2 |\n", resolveGolden: "game 1 branch\n"},
		map[string]string{resolveIndex: "| R3 |\n", resolveGolden: "game 1 main\n"})
	unmerged := resolveUnmerged(repo)
	sort.Strings(unmerged)
	assertEqual(t, unmerged, []string{resolveGolden, resolveIndex})
	left, done := resolveRunResolve(repo, nil)
	assertEqual(t, left, []string{})
	for _, r := range done {
		assertTrue(t, r.OK, fmt.Sprintf("%+v", done))
	}
	assertEqual(t, readText(t, filepath.Join(repo.Cwd, resolveGolden)), "blessed\n")
	assertEqual(t, resolveUnmerged(repo), []string{})
}

// It builds the merged tree, which a code file full of markers breaks: the builder resolves the
// code first, and the traces with it.
func TestResolve_ACommandNeverRunsWhileAFileNoResolverNamesIsConflicted(t *testing.T) {
	root := crossMkdtemp(t)
	repo := resolveConflicted(root, map[string]string{resolveGame: "rules branch\n", resolveGolden: "game 1 branch\n"},
		map[string]string{resolveGame: "rules main\n", resolveGolden: "game 1 main\n"})
	before := resolveBytes(t, filepath.Join(repo.Cwd, resolveGolden))
	left, done := resolveRunResolve(repo, []*config.Resolver{resolveTakeIndex(), resolveRun(resolveGolden, "touch ran")})
	sort.Strings(left)
	assertEqual(t, left, []string{resolveGolden, resolveGame})
	assertFalse(t, exists(filepath.Join(repo.Cwd, "ran")))
	assertEqual(t, resolveBytes(t, filepath.Join(repo.Cwd, resolveGolden)), before)
	assertEqual(t, len(done), 1)
	assertFalse(t, done[0].OK)
	assertIn(t, "`"+resolveGame+"`", done[0].WhyNot)
}

func TestResolve_ATakeResolverStillTakesWhileCodeIsConflicted(t *testing.T) {
	root := crossMkdtemp(t)
	repo := resolveConflicted(root, map[string]string{resolveGame: "rules branch\n", resolveIndex: "| R2 |\n"},
		map[string]string{resolveGame: "rules main\n", resolveIndex: "| R3 |\n"})
	left, done := resolveRunResolve(repo, nil)
	assertEqual(t, left, []string{resolveGame})
	assertEqual(t, resolvePathsOK(done), []any{[]any{resolveIndex, true}})
	assertIn(t, "<<<<<<<", readText(t, filepath.Join(repo.Cwd, resolveGame)))
	assertEqual(t, resolveUnmerged(repo), []string{resolveGame})
}

func TestResolve_AFailingCommandPutsTheConflictBackByteForByte(t *testing.T) {
	root := crossMkdtemp(t)
	repo := resolveConflicted(root, map[string]string{resolveGolden: "game 1 branch\n"},
		map[string]string{resolveGolden: "game 1 main\n"})
	before := resolveBytes(t, filepath.Join(repo.Cwd, resolveGolden))
	assertIn(t, "<<<<<<<", string(before))
	failing := resolveRun(resolveGolden, "printf 'half' > "+resolveGolden+"; echo boom >&2; exit 3")
	left, done := resolveRunResolve(repo, []*config.Resolver{failing})
	assertEqual(t, left, []string{resolveGolden})
	assertEqual(t, resolveBytes(t, filepath.Join(repo.Cwd, resolveGolden)), before)
	assertEqual(t, resolveUnmerged(repo), []string{resolveGolden})
	assertFalse(t, done[0].OK)
	assertIn(t, "exited 3", done[0].WhyNot)
	assertIn(t, "boom", done[0].WhyNot)
}

func TestResolve_ACommandThatLeavesAMarkerResolvesNothing(t *testing.T) {
	root := crossMkdtemp(t)
	repo := resolveConflicted(root, map[string]string{resolveGolden: "game 1 branch\n"},
		map[string]string{resolveGolden: "game 1 main\n"})
	before := resolveBytes(t, filepath.Join(repo.Cwd, resolveGolden))
	left, done := resolveRunResolve(repo, []*config.Resolver{resolveRun(resolveGolden, "true")})
	assertEqual(t, left, []string{resolveGolden})
	assertIn(t, "as the merge left it", done[0].WhyNot)
	assertEqual(t, resolveBytes(t, filepath.Join(repo.Cwd, resolveGolden)), before)
	sneaky := resolveRun(resolveGolden, "printf '<<<<<<< ours\\nx\\n>>>>>>> theirs\\n' > "+resolveGolden)
	left, done = resolveRunResolve(repo, []*config.Resolver{sneaky})
	assertEqual(t, left, []string{resolveGolden})
	assertIn(t, "marker", done[0].WhyNot)
	assertEqual(t, resolveBytes(t, filepath.Join(repo.Cwd, resolveGolden)), before)
}

// resolveStopped is the test's Stopped exception.
type resolveStopped struct{ msg string }

func (e *resolveStopped) Error() string { return e.msg }

// The job's signal handler raises inside the command (`work.Worker._on_signal`): the
// half-written file must not survive for a work-in-progress commit to take up.
func TestResolve_ARunCutOffMidCommandStillPutsTheConflictBack(t *testing.T) {
	root := crossMkdtemp(t)
	repo := resolveConflicted(root, map[string]string{resolveGolden: "game 1 branch\n"},
		map[string]string{resolveGolden: "game 1 main\n"})
	before := resolveBytes(t, filepath.Join(repo.Cwd, resolveGolden))

	cutOff := func(gate *config.Gate, cwd string, env map[string]string, timeoutS *int) *gates.GateResult {
		crossPut(filepath.Join(cwd, resolveGolden), "half-writt")
		panic(&resolveStopped{msg: "SIGTERM"})
	}
	original := gates.RunGate
	gates.RunGate = cutOff
	func() {
		defer func() { gates.RunGate = original }()
		_, raised := herr.Catch[*resolveStopped](func() {
			resolveRunResolve(repo, []*config.Resolver{resolveRun(resolveGolden, "anything")})
		})
		assertTrue(t, raised, "Stopped not raised")
	}()
	assertEqual(t, resolveBytes(t, filepath.Join(repo.Cwd, resolveGolden)), before)
	assertEqual(t, resolveUnmerged(repo), []string{resolveGolden})
}

func TestResolve_ACommandNeverDecidesADeletion(t *testing.T) {
	for _, c := range [][2]map[string]string{
		{{resolveGolden: "game 1 branch\n"}, {resolveGolden: resolveNone}},
		{{resolveGolden: resolveNone}, {resolveGolden: "game 1 main\n"}},
	} {
		branch, main := c[0], c[1]
		t.Run(fmt.Sprintf("main_deleted=%v", main[resolveGolden] == resolveNone), func(t *testing.T) {
			repo := resolveConflicted(crossMkdtemp(t), branch, main)
			assertEqual(t, resolveUnmerged(repo), []string{resolveGolden})
			left, done := resolveRunResolve(repo, []*config.Resolver{resolveRun(resolveGolden, "touch ran")})
			assertEqual(t, left, []string{resolveGolden})
			assertIn(t, "deleted", done[0].WhyNot)
			assertFalse(t, exists(filepath.Join(repo.Cwd, "ran")))
		})
	}
}

func TestResolve_ACommandWaitsForTimeBeyondTheReserve(t *testing.T) {
	root := crossMkdtemp(t)
	repo := resolveConflicted(root, map[string]string{resolveGolden: "game 1 branch\n"},
		map[string]string{resolveGolden: "game 1 main\n"})
	left, done := resolveRunResolve(repo, []*config.Resolver{resolveRun(resolveGolden, "touch ran")},
		resolveOpts{SecondsLeft: func() float64 { return 1000.0 }, ReserveSeconds: 900.0})
	assertEqual(t, left, []string{resolveGolden})
	assertIn(t, "too little", done[0].WhyNot)
	assertFalse(t, exists(filepath.Join(repo.Cwd, "ran")))
}

func TestResolve_TakeNamesTheBranchItTookFrom(t *testing.T) {
	root := crossMkdtemp(t)
	repo := resolveConflicted(root, map[string]string{resolveIndex: "| R2 |\n"}, map[string]string{resolveIndex: "| R3 |\n"})
	_, done := resolveRunResolve(repo, nil, resolveOpts{Base: "bot/tree-5"})
	assertEqual(t, done[0].How, "took `bot/tree-5`'s version")
}

func TestResolve_ACommandThatDeletesTheFileResolvesNothing(t *testing.T) {
	root := crossMkdtemp(t)
	repo := resolveConflicted(root, map[string]string{resolveGolden: "game 1 branch\n"},
		map[string]string{resolveGolden: "game 1 main\n"})
	before := resolveBytes(t, filepath.Join(repo.Cwd, resolveGolden))
	left, done := resolveRunResolve(repo, []*config.Resolver{resolveRun(resolveGolden, "rm "+resolveGolden)})
	assertEqual(t, left, []string{resolveGolden})
	assertIn(t, "no file", done[0].WhyNot)
	assertEqual(t, resolveBytes(t, filepath.Join(repo.Cwd, resolveGolden)), before)
}

func TestResolve_NoCommandRunsWithoutTheDependenciesOrTheTime(t *testing.T) {
	for _, c := range []struct {
		opts resolveOpts
		why  string
	}{
		{resolveOpts{NoCommands: true}, "did not install"},
		{resolveOpts{SecondsLeft: func() float64 { return 30.0 }}, "too little"},
	} {
		t.Run("why="+c.why, func(t *testing.T) {
			root := crossMkdtemp(t)
			repo := resolveConflicted(root, map[string]string{resolveGolden: "game 1 branch\n"},
				map[string]string{resolveGolden: "game 1 main\n"})
			left, done := resolveRunResolve(repo, []*config.Resolver{resolveRun(resolveGolden, "touch ran")}, c.opts)
			assertEqual(t, left, []string{resolveGolden})
			assertFalse(t, exists(filepath.Join(repo.Cwd, "ran")))
			assertIn(t, c.why, done[0].WhyNot)
		})
	}
}

func TestResolve_AFileNoResolverNamesIsLeftExactlyAsGitLeftIt(t *testing.T) {
	root := crossMkdtemp(t)
	repo := resolveConflicted(root, map[string]string{resolveGame: "rules branch\n"}, map[string]string{resolveGame: "rules main\n"})
	before := resolveBytes(t, filepath.Join(repo.Cwd, resolveGame))
	left, done := resolveRunResolve(repo, nil)
	assertEqual(t, []any{pyjson.ListOf(left), len(done)}, []any{[]any{resolveGame}, 0})
	assertEqual(t, resolveBytes(t, filepath.Join(repo.Cwd, resolveGame)), before)
}

func TestResolve_TheSummarySaysWhatWasDoneAndWhatIsLeft(t *testing.T) {
	done := []*resolve.Resolution{
		{Path: resolveIndex, How: "took `main`'s version", OK: true},
		{Path: resolveGolden, How: "ran `x`", OK: false, WhyNot: "it exited 1: no"},
	}
	line := resolve.Summary(done, []string{resolveGolden, resolveGame})
	assertIn(t, "`"+resolveIndex+"`: took `main`'s version", line)
	assertIn(t, "`"+resolveGolden+"`: ran `x`, not resolved: it exited 1: no", line)
	assertIn(t, "2 file(s) left for the builder", line)
}

// ---------------------------------------------------------------------------- ConfigTests

func resolveParse(resolvers any) []*config.Resolver {
	return testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("resolvers", resolvers)}).Resolvers
}

// INDEX.md takes `main`'s version (the daily spec fold rewrites it, and no branch should touch
// it); the golden traces are blessed from the merged tree, naming the file outright so a binary
// built elsewhere cannot write somewhere else.
func TestResolve_TheCommittedResolvers(t *testing.T) {
	resolvers := map[string]*config.Resolver{}
	for _, r := range testsupport.MakeConfig().Resolvers {
		resolvers[r.Path] = r
	}
	paths := map[string]bool{}
	for p := range resolvers {
		paths[p] = true
	}
	assertEqual(t, paths, setOf(resolveIndex, resolveGolden))
	assertEqual(t, resolvers[resolveIndex].Take, "main")
	assertEqual(t, resolvers[resolveGolden].Run, "cargo jackioh golden bless --file "+resolveGolden)
	assertTrue(t, resolvers[resolveGolden].TimeoutMinutes >= 10)
	squishy := filepath.Join(testsupport.Root, ".squishy", "config.json")
	assertEqual(t, dig(t, loadJSON(t, squishy), "resolvers"), dig(t, testsupport.RawConfig(), "resolvers"))
}

func TestResolve_NoneIsNone(t *testing.T) {
	raw := testsupport.RawConfig()
	raw.Pop("resolvers")
	assertEqual(t, len(config.Parse(raw, testsupport.Root, testsupport.TestEnv, testsupport.TestPool()).Resolvers), 0)
}

func TestResolve_AResolverNamesOneFileAndOneWay(t *testing.T) {
	r := func(kv ...any) *pyjson.Object { return pyjson.New(kv...) }
	l := func(xs ...any) []any { return xs }
	for _, c := range []struct {
		bad any
		why string
	}{
		{l(r("path", resolveIndex)), "exactly one"},
		{l(r("path", resolveIndex, "take", "main", "run", "x")), "exactly one"},
		{l(r("path", resolveIndex, "take", "branch")), "`take` is one of"},
		{l(r("path", "spec/*.md", "take", "main")), "no glob"},
		{l(r("path", "../etc/passwd", "take", "main")), "no glob"},
		{l(r("path", "/etc/passwd", "take", "main")), "no glob"},
		{l(r("path", resolveIndex, "take", "main", "when", "always")), "unknown keys"},
		{l(r("path", resolveGolden, "run", "x", "timeout_minutes", 0)), "at least 1"},
		{l(r("path", resolveIndex, "take", "main"), r("path", resolveIndex, "run", "x")), "already"},
		{r("path", resolveIndex), "expected a list"},
		{l(r("path", "./spec/INDEX.md", "take", "main")), "plain repository path"},
		{l(r("path", "spec//INDEX.md", "take", "main")), "plain repository path"},
		{l(r("path", "spec/", "take", "main")), "plain repository path"},
		{l(r("path", "spec\\INDEX.md", "take", "main")), "plain repository path"},
		{l(r("path", resolveGolden, "run", true)), "strings"},
		{l(r("path", resolveGolden, "run", l("cargo", "x"))), "strings"},
		{l(r("path", resolveIndex, "take", "main", "timeout_minutes", 5)), "for a `run` resolver"},
		{l(r("path", resolveGolden, "run", "x", "timeout_minutes", 2.9)), "whole number"},
		{l(r("path", resolveGolden, "run", "x", "timeout_minutes", "7")), "whole number"},
		{l(r("path", resolveGolden, "run", "x", "timeout_minutes", true)), "whole number"},
		{l(r("path", resolveGolden, "run", "x"), r("path", resolveGame, "run", "y")), "one `run` resolver"},
	} {
		t.Run("bad="+pyjson.Dumps(c.bad), func(t *testing.T) {
			caught, ok := herr.Catch[*herr.ConfigError](func() { resolveParse(c.bad) })
			assertTrue(t, ok, "ConfigError not raised")
			assertIn(t, c.why, caught.Error())
		})
	}
}

// ---------------------------------------------------------------------------- WorkerTests

var resolveDone = `<!-- bot: {"status": "done", "title": "PR"} -->` + "\n## What changed\nResolved."
var resolveApprove = `<!-- review: {"verdict": "approve", "findings": []} -->` + "\nRead the merge; fine."
var resolveChanges = `<!-- review: {"verdict": "changes", "findings": [{"severity": "blocking", ` +
	`"where": "src/game.txt", "claim": "say v3", "evidence": "read it"}]} -->` + "\nNo."

func resolveGates() []any {
	return []any{
		pyjson.New("name", "rules exist", "run", "test -f "+resolveGame, "timeout_minutes", 1),
		pyjson.New("name", "no markers", "run", "! git grep -q -E '^(<<<<<<< |>>>>>>> )' -- .", "timeout_minutes", 1),
	}
}

// resolveWorld is WorkerTests' setUp: a conflict revision whose every conflict the harness settles
// spends no builder session: its checks and its reviewer judge the merge. Anything else reaches
// the builder, told what the harness already did.
type resolveWorld struct {
	t      *testing.T
	root   string
	origin string
	clone  string
	cfg    *config.Config
}

func newResolveWorld(t *testing.T) *resolveWorld {
	w := &resolveWorld{t: t}
	w.root = crossMkdtemp(t)
	w.origin, w.clone = fakes.MakeOrigin(w.root, resolveBase())
	w.cfg = testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("gates", resolveGates(),
		"install", pyjson.New("run", "true", "timeout_minutes", 1), "max_review_cycles", 3,
		"resolvers", []any{pyjson.New("path", resolveIndex, "take", "main"),
			pyjson.New("path", resolveGolden, "run", resolveBless, "timeout_minutes", 1)})})
	return w
}

func (w *resolveWorld) diverge(branch, main map[string]string) string {
	start := fakes.PushBranch(w.origin, w.root, "feature/x", branch)
	fakes.PushBranch(w.origin, w.root, "main", main)
	fakes.Git(w.clone, "fetch", "-q", "origin")
	return start
}

// plan is WorkerTests.plan(feedback, **extra); "" feedback is the default, no new comments.
func (w *resolveWorld) plan(feedback string, extra ...any) *pyjson.Object {
	if feedback == "" {
		feedback = prompts.Data(threads.NoFeedback, "Feedback on #30")
	}
	p := pyjson.New("action", "revise", "number", 30, "title", "PR", "branch", "feature/x",
		"source", "conflict", "pull", "p", "issue", "", "feedback", feedback, "thread", "t")
	p.Update(pyjson.New(extra...))
	return p
}

func (w *resolveWorld) runWorker(r *runner.FakeRunner, plan *pyjson.Object, cfg ...*config.Config) *pyjson.Object {
	c := w.cfg
	if len(cfg) > 0 && cfg[0] != nil {
		c = cfg[0]
	}
	worker := work.NewWorker(c, plan, r, w.clone, filepath.Join(w.root, "work"), filepath.Join(w.root, "out"),
		nil, nil, nil, nil, nil, nil)
	return worker.Run()
}

func (w *resolveWorld) head() (string, string) {
	check := filepath.Join(w.root, "check")
	if !exists(check) {
		fakes.Git(w.root, "clone", "-q", w.origin, check)
	}
	fakes.Git(check, "fetch", "-q", filepath.Join(w.root, "out", "branch.bundle"), "+refs/heads/feature/x:refs/heads/got")
	return check, fakes.Git(check, "rev-parse", "got")
}

func TestResolve_AConflictOnlyInResolverFilesSpendsNoBuilderSession(t *testing.T) {
	w := newResolveWorld(t)
	start := w.diverge(map[string]string{resolveIndex: "| R1 |\n| R2 |\n", resolveGolden: "game 1 branch\n", resolveGame: "rules v2\n"},
		map[string]string{resolveIndex: "| R1 |\n| R3 |\n", resolveGolden: "game 1 main\n"})
	r := workFake(map[string]runner.Handler{"review": crossAnswer(resolveApprove)})
	result := w.runWorker(r, w.plan(""))
	assertEqual(t, result.Get("status"), "approved", result.Get("reason"))
	assertEqual(t, workRoles(r), []string{"review"})
	check, head := w.head()
	parents := strings.Fields(fakes.Git(check, "rev-list", "--parents", "-n", "1", head))
	assertEqual(t, parents[1], start)
	assertEqual(t, fakes.Git(check, "show", head+":"+resolveIndex), "| R1 |\n| R3 |")
	assertEqual(t, fakes.Git(check, "show", head+":"+resolveGolden), "blessed")
	assertEqual(t, fakes.Git(check, "show", head+":"+resolveGame), "rules v2")
	builder := crossObj(t, result, "cycles", 0, "builder")
	assertEqual(t, []any{builder.Get("role"), builder.Get("model"), builder.Get("ok")}, []any{"merge", "the harness", true})
	for _, g := range pyjson.Objs(dig(t, result, "cycles", 0, "gates")) {
		assertTrue(t, pyjson.Truthy(dig(t, g, "ok")))
	}
	// The take resolvers go first, then the command once nothing else is conflicted.
	got := []any{}
	for _, r := range pyjson.Objs(dig(t, result, "resolved")) {
		got = append(got, []any{dig(t, r, "path"), dig(t, r, "ok")})
	}
	assertEqual(t, got, []any{[]any{resolveIndex, true}, []any{resolveGolden, true}})
	assertIn(t, "no builder ran", crossStr(t, result, "report"))
	assertIn(t, "`"+resolveGolden+"`: ran `"+resolveBless+"`", crossStr(t, result, "report"))
}

func TestResolve_ACommentThatQuotesTheEmptyFeedbackIsStillFeedback(t *testing.T) {
	w := newResolveWorld(t)
	w.diverge(map[string]string{resolveIndex: "| R2 |\n"}, map[string]string{resolveIndex: "| R3 |\n"})
	r := workFake(map[string]runner.Handler{"revise": crossAnswer(resolveDone), "review": crossAnswer(resolveApprove)})
	quoting := prompts.Data("Comment by @jgoetzmann:\n\nYou said "+threads.NoFeedback+"; rename rules to laws.",
		"Feedback on #30")
	w.runWorker(r, w.plan(quoting))
	assertEqual(t, r.Calls[0].Role, "revise")
}

func TestResolve_EveryReviewerHearsTheHarnessResolvedByARule(t *testing.T) {
	w := newResolveWorld(t)
	w.diverge(map[string]string{resolveIndex: "| R2 |\n", resolveGame: "rules v2\n"}, map[string]string{resolveIndex: "| R3 |\n"})
	r := workFake(map[string]runner.Handler{"review": crossAnswer(resolveApprove)})
	result := w.runWorker(r, w.plan(""))
	assertEqual(t, result.Get("status"), "approved", result.Get("reason"))
	review := r.Calls[0].Prompt
	assertIn(t, "a file it took from `main` keeps `main`'s version on purpose", review)
	assertIn(t, "- `"+resolveIndex+"`: took `main`'s version", review)
	var step *pyjson.Object
	for _, s := range pyjson.Objs(dig(t, result, "steps")) {
		if pyjson.Equal(dig(t, s, "step"), "resolve") {
			step = s
			break
		}
	}
	if step == nil {
		t.Fatal("StopIteration: no resolve step")
	}
	assertIn(t, "minutes", step)
	assertIn(t, "`"+resolveIndex+"`: took `main`'s version", crossStr(t, step, "outcome"))
}

// `main` changed only the traces, and blessing the merged tree gives the branch's own: the merged
// tree is the branch's, yet the merge commit must still name `main`, or the pull request would
// conflict for ever.
func TestResolve_AMergeWhoseTreeIsTheBranchsStillRecordsMain(t *testing.T) {
	w := newResolveWorld(t)
	w.diverge(map[string]string{resolveGolden: "game branch\n"}, map[string]string{resolveGolden: "game main\n"})
	main := fakes.Git(w.clone, "rev-parse", "origin/main")
	cfg := testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("gates", resolveGates(),
		"install", pyjson.New("run", "true", "timeout_minutes", 1), "max_review_cycles", 3,
		"resolvers", []any{pyjson.New("path", resolveGolden, "run", "printf 'game branch\\n' > "+resolveGolden,
			"timeout_minutes", 1)})})
	r := workFake(map[string]runner.Handler{"review": crossAnswer(resolveApprove)})
	result := w.runWorker(r, w.plan(""), cfg)
	assertEqual(t, result.Get("status"), "approved", result.Get("reason"))
	assertEqual(t, workRoles(r), []string{"review"})
	check, head := w.head()
	assertIn(t, main, strings.Fields(fakes.Git(check, "rev-list", "--parents", "-n", "1", head))[1:])
	assertEqual(t, fakes.Git(check, "show", head+":"+resolveGolden), "game branch")
}

func TestResolve_TheReviewStillJudgesTheHarnessMergeAndSendsFindingsToTheBuilder(t *testing.T) {
	w := newResolveWorld(t)
	w.diverge(map[string]string{resolveIndex: "| R2 |\n", resolveGame: "rules v2\n"}, map[string]string{resolveIndex: "| R3 |\n"})

	fix := func(request *runner.RunRequest) *runner.RunResult {
		crossPut(filepath.Join(request.Cwd, resolveGame), "rules v3\n")
		return runner.NewRunResult(true, resolveDone)
	}
	r := workFake(map[string][]runner.Handler{
		"review": {crossAnswer(resolveChanges), crossAnswer(resolveApprove)},
		"fix":    {fix}})
	result := w.runWorker(r, w.plan(""))
	assertEqual(t, result.Get("status"), "approved", result.Get("reason"))
	assertEqual(t, workRoles(r), []string{"review", "fix", "review"})
	assertIn(t, "say v3", r.Calls[1].Prompt)
	assertIn(t, "- `"+resolveIndex+"`: took `main`'s version; leave it as it is", r.Calls[1].Prompt)
	check, head := w.head()
	assertEqual(t, fakes.Git(check, "show", head+":"+resolveGame), "rules v3")
}

// The traces the harness blessed are the merged tree's: a fix pass that then changes what they
// record must bless them again, not leave them.
func TestResolve_AFixAfterARegeneratedFileIsToldToRunItsCommandAgain(t *testing.T) {
	w := newResolveWorld(t)
	w.diverge(map[string]string{resolveGolden: "game 1 branch\n", resolveGame: "rules v2\n"},
		map[string]string{resolveGolden: "game 1 main\n"})
	r := workFake(map[string][]runner.Handler{
		"review": {crossAnswer(resolveChanges), crossAnswer(resolveApprove)},
		"fix":    {crossAnswer(resolveDone)}})
	w.runWorker(r, w.plan(""))
	fix := r.Calls[1].Prompt
	assertIn(t, "- `"+resolveGolden+"`: ran `"+resolveBless+"`; it is that command's output from the merged "+
		"tree, so run `"+resolveBless+"` again if you change anything it reads", fix)
	assertNotIn(t, "leave it as it is", fix)
}

// A harness merge the checks turn red is not approved, whatever its reviewer says: the red goes
// back as a finding, as on any round.
func TestResolve_RedChecksOnTheHarnessMergeAreNotApproved(t *testing.T) {
	w := newResolveWorld(t)
	w.diverge(map[string]string{resolveIndex: "| R2 |\n", resolveGame: "rules v2\n"}, map[string]string{resolveIndex: "| R3 |\n"})
	// Red on the merged head only: `main` never had v2, so the red is this change's.
	red := pyjson.New("name", "no v2", "run", "! grep -q v2 "+resolveGame, "timeout_minutes", 1)
	cfg := testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("gates", []any{red},
		"install", pyjson.New("run", "true", "timeout_minutes", 1), "max_review_cycles", 1,
		"resolvers", []any{pyjson.New("path", resolveIndex, "take", "main")})})
	r := workFake(map[string]runner.Handler{"review": crossAnswer(resolveApprove), "fix": crossAnswer(resolveDone)})
	result := w.runWorker(r, w.plan(""), cfg)
	assertNotEqual(t, result.Get("status"), "approved")
	assertEqual(t, r.Calls[0].Role, "review")
}

func TestResolve_NewFeedbackStillGoesToTheBuilderToldWhatTheHarnessDid(t *testing.T) {
	w := newResolveWorld(t)
	w.diverge(map[string]string{resolveIndex: "| R2 |\n"}, map[string]string{resolveIndex: "| R3 |\n"})
	seen := map[string]string{}

	revise := func(request *runner.RunRequest) *runner.RunResult {
		seen["index"] = mustReadText(filepath.Join(request.Cwd, resolveIndex))
		return runner.NewRunResult(true, resolveDone)
	}
	r := workFake(map[string]runner.Handler{"revise": revise, "review": crossAnswer(resolveApprove)})
	feedback := prompts.Data("Comment by @jgoetzmann:\n\nPlease also rename it.", "Feedback on #30")
	result := w.runWorker(r, w.plan(feedback))
	assertEqual(t, result.Get("status"), "approved", result.Get("reason"))
	assertEqual(t, workRoles(r), []string{"revise", "review"})
	assertEqual(t, seen["index"], "| R3 |\n")
	prompt := r.Calls[0].Prompt
	assertIn(t, "The harness resolved these conflicts itself", prompt)
	assertIn(t, "- `"+resolveIndex+"`: took `main`'s version", prompt)
	// Never "clean": the merge is still open, staged, for the harness to commit.
	assertNotIn(t, "was clean", prompt)
	assertIn(t, "the harness resolved every one (below). The merge is staged, not committed", prompt)
}

func TestResolve_ACodeConflictReachesTheBuilderWithOnlyWhatIsLeft(t *testing.T) {
	w := newResolveWorld(t)
	w.diverge(map[string]string{resolveIndex: "| R2 |\n", resolveGame: "rules branch\n", resolveGolden: "game branch\n"},
		map[string]string{resolveIndex: "| R3 |\n", resolveGame: "rules main\n", resolveGolden: "game main\n"})
	seen := map[string]string{}

	revise := func(request *runner.RunRequest) *runner.RunResult {
		seen["index"] = mustReadText(filepath.Join(request.Cwd, resolveIndex))
		seen["golden"] = mustReadText(filepath.Join(request.Cwd, resolveGolden))
		crossPut(filepath.Join(request.Cwd, resolveGame), "rules main and branch\n")
		crossPut(filepath.Join(request.Cwd, resolveGolden), "game main and branch\n")
		return runner.NewRunResult(true, resolveDone)
	}
	r := workFake(map[string]runner.Handler{"revise": revise, "review": crossAnswer(resolveApprove)})
	result := w.runWorker(r, w.plan(""))
	assertEqual(t, result.Get("status"), "approved", result.Get("reason"))
	assertEqual(t, workRoles(r), []string{"revise", "review"})
	assertEqual(t, seen["index"], "| R3 |\n")
	assertIn(t, "<<<<<<<", seen["golden"]) // the traces wait for the code: the builder's
	prompt := r.Calls[0].Prompt
	parts := strings.Split(prompt, "Files merged with conflict markers")
	if len(parts) < 2 {
		t.Fatalf("IndexError: no conflict list in the prompt")
	}
	listed := strings.Split(parts[1], "The harness resolved")[0]
	assertIn(t, resolveGame, listed)
	assertIn(t, resolveGolden, listed)
	assertNotIn(t, resolveIndex, listed)
	assertIn(t, "- `"+resolveIndex+"`: took `main`'s version", prompt)
	unresolved := []any{}
	for _, r := range pyjson.Objs(dig(t, result, "resolved")) {
		if !pyjson.Truthy(dig(t, r, "ok")) {
			unresolved = append(unresolved, dig(t, r, "path"))
		}
	}
	assertEqual(t, unresolved, []any{resolveGolden})
}

func TestResolve_EarlierFindingsStillGoToTheBuilder(t *testing.T) {
	w := newResolveWorld(t)
	w.diverge(map[string]string{resolveIndex: "| R2 |\n"}, map[string]string{resolveIndex: "| R3 |\n"})
	r := workFake(map[string]runner.Handler{"revise": crossAnswer(resolveDone), "review": crossAnswer(resolveApprove)})
	findings := []any{pyjson.New("severity", "blocking", "where", resolveGame, "claim", "still open", "evidence", "e")}
	w.runWorker(r, w.plan("", "previous_findings", findings))
	assertEqual(t, r.Calls[0].Role, "revise")
}

func TestResolve_ACIRevisionIsNeverMergeOnly(t *testing.T) {
	w := newResolveWorld(t)
	w.diverge(map[string]string{resolveIndex: "| R2 |\n"}, map[string]string{resolveIndex: "| R3 |\n"})
	r := workFake(map[string]runner.Handler{"revise": crossAnswer(resolveDone), "review": crossAnswer(resolveApprove)})
	p := w.plan("")
	p.Set("source", "ci")
	w.runWorker(r, p)
	assertEqual(t, r.Calls[0].Role, "revise")
}

func TestResolve_WithoutResolversTheBuilderGetsEveryConflictAsBefore(t *testing.T) {
	w := newResolveWorld(t)
	w.diverge(map[string]string{resolveIndex: "| R2 |\n"}, map[string]string{resolveIndex: "| R3 |\n"})
	cfg := testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("gates", resolveGates()[:1],
		"install", pyjson.New("run", "true", "timeout_minutes", 1), "max_review_cycles", 3, "resolvers", []any{})})
	seen := map[string]string{}

	revise := func(request *runner.RunRequest) *runner.RunResult {
		seen["index"] = mustReadText(filepath.Join(request.Cwd, resolveIndex))
		crossPut(filepath.Join(request.Cwd, resolveIndex), "| R2 |\n| R3 |\n")
		return runner.NewRunResult(true, resolveDone)
	}
	r := workFake(map[string]runner.Handler{"revise": revise, "review": crossAnswer(resolveApprove)})
	result := w.runWorker(r, w.plan(""), cfg)
	assertEqual(t, result.Get("status"), "approved", result.Get("reason"))
	assertIn(t, "<<<<<<<", seen["index"])
	assertNotIn(t, "resolved", result)
	assertNotIn(t, "The harness resolved", r.Calls[0].Prompt)
	// The standing guidance is true with no resolvers too: the builder takes `main`'s index.
	assertIn(t, "`spec/INDEX.md` or the end of `spec/11-rulings.md`", r.Calls[0].Prompt)
	assertIn(t, "takes\n  `main`'s side", r.Calls[0].Prompt)
}

func TestResolve_AClearedChangesReviewerHearsTheHarnessResolvedIt(t *testing.T) {
	w := newResolveWorld(t)
	start := w.diverge(map[string]string{resolveIndex: "| R2 |\n", resolveGame: "rules v2\n"},
		map[string]string{resolveIndex: "| R3 |\n"})
	r := workFake(map[string]runner.Handler{"review": crossAnswer(resolveApprove)})
	result := w.runWorker(r, w.plan("", "cleared", pyjson.New("sha", start, "by", "Opus")))
	assertEqual(t, result.Get("status"), "approved", result.Get("reason"))
	assertEqual(t, workRoles(r), []string{"review"})
	review := r.Calls[0].Prompt
	assertIn(t, "the harness resolved `"+resolveIndex+"` itself (took `main`'s version)", review)
	assertIn(t, "the files the harness resolved follow its rule", review)
}
