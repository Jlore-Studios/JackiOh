// ci.yml's `changes` job scopes a pull request with the base's scripts/ci-scope.sh (#2986, section
// 1): the step's own shell, taken from ci.yml and run in a throwaway repository, never lets a pull
// request's copy of the script shrink its CI, and runs a job only the pull request's copy scopes.
package tests

import (
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
)

// ciscopeStep is the `changes` job's scope step's `run: |` script, as the runner gets it.
func ciscopeStep(t *testing.T) string {
	t.Helper()
	step := workflowsStep(t, workflowsJob(t, workflowsRead(t, "ci.yml"), "changes"), "id: scope")
	_, body, ok := strings.Cut(step, "        run: |\n")
	if !ok {
		t.Fatal("the scope step has no `run: |`")
	}
	var lines []string
	for _, line := range strings.Split(body, "\n") {
		if strings.TrimSpace(line) != "" && !strings.HasPrefix(line, strings.Repeat(" ", 10)) {
			break
		}
		lines = append(lines, strings.TrimPrefix(line, strings.Repeat(" ", 10)))
	}
	return strings.Join(lines, "\n")
}

// ciscopeRun runs the step in a repository whose base commit holds `base` as scripts/ci-scope.sh
// (none for "") and whose head holds `head`, for `event`, and returns what it wrote to
// $GITHUB_OUTPUT.
func ciscopeRun(t *testing.T, event, base, head string) []string {
	t.Helper()
	root := t.TempDir()
	repo := filepath.Join(root, "repo")
	if err := os.MkdirAll(filepath.Join(repo, "scripts"), 0o777); err != nil {
		t.Fatal(err)
	}
	fakes.Git(repo, "init", "-q", "-b", "main")
	write := func(name, text string) {
		if err := os.WriteFile(filepath.Join(repo, name), []byte(text), 0o666); err != nil {
			t.Fatal(err)
		}
	}
	write("README.md", "base\n")
	if base != "" {
		write("scripts/ci-scope.sh", base)
	}
	fakes.Git(repo, "add", "-A")
	fakes.Git(repo, "commit", "-q", "-m", "base")
	baseSHA := fakes.Git(repo, "rev-parse", "HEAD")
	write("scripts/ci-scope.sh", head)
	fakes.Git(repo, "add", "-A")
	fakes.Git(repo, "commit", "-q", "-m", "head")
	headSHA := fakes.Git(repo, "rev-parse", "HEAD")
	output := filepath.Join(root, "output")
	// GitHub runs a `run:` step as `bash -e {0}` with pipefail (`shell: bash`, the default).
	cmd := exec.Command("bash", "--noprofile", "--norc", "-eo", "pipefail", "-c", ciscopeStep(t))
	cmd.Dir = repo
	cmd.Env = append(os.Environ(), "EVENT="+event, "BASE="+baseSHA, "HEAD="+headSHA, "GITHUB_OUTPUT="+output)
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("the scope step failed: %v\n%s", err, out)
	}
	if isFile(filepath.Join(repo, "ran")) {
		t.Fatal("the scope step ran the pull request's copy of the script")
	}
	text, err := os.ReadFile(output)
	if err != nil {
		t.Fatal(err)
	}
	return strings.Split(strings.TrimSpace(string(text)), "\n")
}

// The base's script prints three lines. The pull request's copy declares a fourth and tries to
// shrink them, both by what it prints and by writing $GITHUB_OUTPUT itself, which it never gets to:
// the step reads its `LINES=` and never runs it.
const (
	ciscopeBase = "#!/bin/sh\nLINES=\"full web e2e\"\nfor name in $LINES; do echo \"$name=true\"; done\n"
	ciscopeHead = "#!/bin/sh\nLINES=\"full web new_job\"\necho web=false >> \"$GITHUB_OUTPUT\"\n" +
		"touch ran\nfor name in $LINES; do echo \"$name=false\"; done\n"
)

func TestCiscope_APullRequestCannotShrinkItsOwnCIAndItsNewLineRuns(t *testing.T) {
	for _, event := range []string{"pull_request", "merge_group"} {
		// The base's lines win, its `e2e` too, and the line only the pull request declares is `true`.
		assertEqual(t, ciscopeRun(t, event, ciscopeBase, ciscopeHead),
			[]string{"full=true", "web=true", "e2e=true", "new_job=true", "bot=true"}, event)
	}
}

func TestCiscope_ABaseWithNoScriptRunsEveryLineThePullRequestDeclares(t *testing.T) {
	assertEqual(t, ciscopeRun(t, "pull_request", "", ciscopeHead),
		[]string{"full=true", "web=true", "new_job=true", "bot=true"})
}

// Only names are read from the pull request's copy, each a plain name, and only as `true`; a copy
// whose `LINES=` the step cannot read adds nothing, and the base's lines stand.
func TestCiscope_ThePullRequestsCopyCannotWriteAnyOtherOutput(t *testing.T) {
	head := "#!/bin/sh\nLINES=\"full ok_2 $(touch ran)\"\necho full=false\n"
	assertEqual(t, ciscopeRun(t, "pull_request", ciscopeBase, head),
		[]string{"full=true", "web=true", "e2e=true", "bot=true"})
	head = "#!/bin/sh\nLINES=\"full ok_2\"\necho full=false\nexit 3\n"
	assertEqual(t, ciscopeRun(t, "pull_request", ciscopeBase, head),
		[]string{"full=true", "web=true", "e2e=true", "ok_2=true", "bot=true"})
	assertEqual(t, ciscopeRun(t, "pull_request", ciscopeBase, "#!/bin/sh\nLINES=\n"),
		[]string{"full=true", "web=true", "e2e=true", "bot=true"})
}

// ci-scope.sh keeps its line names in one `LINES="…"` line, the one the step reads.
func TestCiscope_TheScriptDeclaresItsLinesOnOneLine(t *testing.T) {
	script := readText(t, filepath.Join(config.RepoRoot, "scripts", "ci-scope.sh"))
	assertRegex(t, script, `(?m)^LINES="[a-z0-9_ ]+"$`)
	assertEqual(t, len(regexp.MustCompile(`(?m)^LINES=`).FindAllString(script, -1)), 1)
}

// A push to main skips what the merge queue's build of the same commit ran and passed (#3292): the
// step finds that build (`gh api`, faked here by a `gh` on PATH that prints its answer), scopes it
// again from the commit it stood on, and turns each line it printed `true` to `false`, but `full`,
// `db` and `db_deploy`. The rest, what the build's scope left out, still runs.

// ciscopePushScript prints every line `true` run bare, as a push runs it, and, on a diff, `true`
// for `full`, `db`, `db_deploy` and `web` (what the queue's build ran) and `false` for the rest.
const ciscopePushScript = "#!/bin/sh\nLINES=\"full db db_deploy db_sql web e2e\"\nfor name in $LINES; do\n" +
	"  if [ $# -eq 0 ]; then echo \"$name=true\"\n" +
	"  else case $name in full | db | db_deploy | web) echo \"$name=true\" ;; *) echo \"$name=false\" ;; esac\n" +
	"  fi\ndone\n"

// ciscopeAll is every line of ciscopePushScript `true`, and `bot`: what a push ran before.
var ciscopeAll = []string{"full=true", "db=true", "db_deploy=true", "db_sql=true", "web=true", "e2e=true", "bot=true"}

// ciscopePush runs the step for `event` on a head commit that changes `changed` (path to text) on
// top of a base holding ciscopePushScript, with `gh` printing `answer` ({base} the base's commit, ""
// nothing), and returns what it wrote to $GITHUB_OUTPUT.
func ciscopePush(t *testing.T, event, answer string, changed map[string]string) []string {
	t.Helper()
	root := t.TempDir()
	repo := filepath.Join(root, "repo")
	bin := filepath.Join(root, "bin")
	for _, dir := range []string{filepath.Join(repo, "scripts"), bin} {
		if err := os.MkdirAll(dir, 0o777); err != nil {
			t.Fatal(err)
		}
	}
	fakes.Git(repo, "init", "-q", "-b", "main")
	write := func(name, text string, mode os.FileMode) {
		if err := os.MkdirAll(filepath.Dir(name), 0o777); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(name, []byte(text), mode); err != nil {
			t.Fatal(err)
		}
	}
	write(filepath.Join(repo, "README.md"), "base\n", 0o666)
	write(filepath.Join(repo, "scripts", "ci-scope.sh"), ciscopePushScript, 0o666)
	fakes.Git(repo, "add", "-A")
	fakes.Git(repo, "commit", "-q", "-m", "base")
	baseSHA := fakes.Git(repo, "rev-parse", "HEAD")
	for name, text := range changed {
		write(filepath.Join(repo, name), text, 0o666)
	}
	fakes.Git(repo, "add", "-A")
	fakes.Git(repo, "commit", "-q", "-m", "head")
	headSHA := fakes.Git(repo, "rev-parse", "HEAD")
	write(filepath.Join(bin, "gh"), "#!/bin/sh\nprintf '%s' '"+strings.ReplaceAll(answer, "{base}", baseSHA)+"'\n", 0o777)
	output := filepath.Join(root, "output")
	cmd := exec.Command("bash", "--noprofile", "--norc", "-eo", "pipefail", "-c", ciscopeStep(t))
	cmd.Dir = repo
	cmd.Env = append(os.Environ(), "EVENT="+event, "BASE=", "HEAD=", "SHA="+headSHA, "REPO=o/r",
		"QUEUE_BUILD_WAIT_SECONDS=0", "GITHUB_OUTPUT="+output, "PATH="+bin+string(os.PathListSeparator)+os.Getenv("PATH"))
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("the scope step failed: %v\n%s", err, out)
	}
	text, err := os.ReadFile(output)
	if err != nil {
		t.Fatal(err)
	}
	return strings.Split(strings.TrimSpace(string(text)), "\n")
}

func TestCiscope_APushSkipsWhatTheQueuesBuildOfItsCommitPassed(t *testing.T) {
	// The queue's build ran `web` and passed: `web` is skipped. It left `db_sql`, `e2e` and `bot`
	// (the diff touches none of the bot's paths) out: they run. `full`, `db` and `db_deploy` always do.
	assertEqual(t, ciscopePush(t, "push", "1 success gh-readonly-queue/main/pr-7-{base}", map[string]string{"app.txt": "x\n"}),
		[]string{"full=true", "db=true", "db_deploy=true", "db_sql=true", "web=false", "e2e=true", "bot=true"})
	// The build ran `bot selftest` too (the diff touches scripts/): skipped as well.
	assertEqual(t, ciscopePush(t, "push", "1 success gh-readonly-queue/main/pr-7-{base}", map[string]string{"scripts/x.sh": "x\n"}),
		[]string{"full=true", "db=true", "db_deploy=true", "db_sql=true", "web=false", "e2e=true", "bot=false"})
}

func TestCiscope_APushRunsEverythingWithoutAPassingQueueBuildOfItsCommit(t *testing.T) {
	app := map[string]string{"app.txt": "x\n"}
	// No build (a push past the queue), or one still going once the wait is up.
	assertEqual(t, ciscopePush(t, "push", "", app), ciscopeAll)
	assertEqual(t, ciscopePush(t, "push", "1 going gh-readonly-queue/main/pr-7-{base}", app), ciscopeAll)
	// A build whose base is no commit before this one, or a branch with no commit in its name.
	assertEqual(t, ciscopePush(t, "push", "1 success gh-readonly-queue/main/pr-7-"+strings.Repeat("0", 40), app), ciscopeAll)
	assertEqual(t, ciscopePush(t, "push", "1 success gh-readonly-queue/main/pr-7-abc", app), ciscopeAll)
	// A commit that changes what the build caches are keyed on: main alone saves them.
	for _, name := range []string{"Cargo.lock", "crates/x/Cargo.toml", "rust-toolchain.toml", ".cargo/config.toml", "pnpm-lock.yaml"} {
		assertEqual(t, ciscopePush(t, "push", "1 success gh-readonly-queue/main/pr-7-{base}", map[string]string{name: "x\n"}),
			ciscopeAll, name)
	}
	// A dispatch never looks.
	assertEqual(t, ciscopePush(t, "workflow_dispatch", "1 success gh-readonly-queue/main/pr-7-{base}", app), ciscopeAll)
}
