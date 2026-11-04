// ci.yml's `changes` job scopes a pull request with the base's scripts/ci-scope.sh (#2986, section
// 1): the step's own shell, taken from ci.yml and run in a throwaway repository, never lets a pull
// request's copy of the script shrink its CI, and runs a job only the pull request's copy scopes.
package tests

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"

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
	text, err := os.ReadFile(output)
	if err != nil {
		t.Fatal(err)
	}
	return strings.Split(strings.TrimSpace(string(text)), "\n")
}

// The base's script prints three lines; the pull request's copy shrinks them and adds a fourth.
const (
	ciscopeBase = "#!/bin/sh\necho full=true\necho web=true\necho e2e=true\n"
	ciscopeHead = "#!/bin/sh\necho full=false\necho web=false\necho new_job=false\n"
)

func TestCiscope_APullRequestCannotShrinkItsOwnCIAndItsNewLineRuns(t *testing.T) {
	for _, event := range []string{"pull_request", "merge_group"} {
		// The base's lines win, its `e2e` too, and the line only the pull request prints is `true`.
		assertEqual(t, ciscopeRun(t, event, ciscopeBase, ciscopeHead),
			[]string{"full=true", "web=true", "e2e=true", "new_job=true", "bot=true"}, event)
	}
}

func TestCiscope_ABaseWithNoScriptRunsEveryLineThePullRequestPrints(t *testing.T) {
	assertEqual(t, ciscopeRun(t, "pull_request", "", ciscopeHead),
		[]string{"full=true", "web=true", "new_job=true", "bot=true"})
}

func TestCiscope_ThePullRequestsCopyCannotWriteAnyOtherOutput(t *testing.T) {
	head := "#!/bin/sh\necho full=false\necho 'bad name=false'\necho 'x;y=false'\necho no-equals\necho ok_2=false\n"
	assertEqual(t, ciscopeRun(t, "pull_request", ciscopeBase, head),
		[]string{"full=true", "web=true", "e2e=true", "ok_2=true", "bot=true"})
	// A copy that fails adds nothing: the base's lines stand.
	assertEqual(t, ciscopeRun(t, "pull_request", ciscopeBase, ciscopeHead+"exit 3\n"),
		[]string{"full=true", "web=true", "e2e=true", "bot=true"})
}
