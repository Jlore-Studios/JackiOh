package disk

import (
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"syscall"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata/python.json holds what bot/harness/disk.py (Python 3.13) answered for each input.
func python(t *testing.T, key string) [][]any {
	t.Helper()
	raw, err := os.ReadFile(filepath.Join("testdata", "python.json"))
	if err != nil {
		t.Fatal(err)
	}
	all, err := pyjson.Loads(raw)
	if err != nil {
		t.Fatal(err)
	}
	var cases [][]any
	for _, c := range pyjson.List(all.(*pyjson.Object).Get(key)) {
		cases = append(cases, pyjson.List(c))
	}
	return cases
}

func TestSizeDescribeAndTheRules(t *testing.T) {
	for _, c := range python(t, "size") {
		n, _ := pyjson.AsInt(c[0])
		if got := Size(n); got != c[1] {
			t.Errorf("size(%d): got %s, want %s", n, got, c[1])
		}
	}
	for _, c := range python(t, "rules") {
		found := c[0].(*pyjson.Object)
		if got := Describe(found); got != c[1] {
			t.Errorf("describe: got %s, want %s", got, c[1])
		}
		if TooFull(found) != c[2] || Recovered(found) != c[3] {
			t.Errorf("%s: too full %v, recovered %v", c[1], TooFull(found), Recovered(found))
		}
	}
}

func TestTheIssueBody(t *testing.T) {
	for _, c := range python(t, "body") {
		if got := body(c[0].(*pyjson.Object)); got != c[1] {
			t.Errorf("got\n%s\nwant\n%s", got, c[1])
		}
	}
}

func TestAFullDiskInAnyError(t *testing.T) {
	if !IsFull(&os.PathError{Op: "write", Path: "x", Err: syscall.ENOSPC}) {
		t.Error("ENOSPC")
	}
	if !IsFull(errors.New("git commit: fatal: No space left on device")) {
		t.Error("the words")
	}
	if IsFull(&os.PathError{Op: "open", Path: "x", Err: syscall.ENOENT}) || IsFull(nil) {
		t.Error("not full")
	}
}

func TestDiskUsageIsDfs(t *testing.T) {
	total, free, used := diskUsage(os.TempDir())
	if total == 0 || free > total || used > total {
		t.Errorf("total %d, free %d, used %d", total, free, used)
	}
	var raised error
	func() {
		defer func() { raised, _ = recover().(error) }()
		diskUsage(filepath.Join(t.TempDir(), "missing"))
	}()
	var pe *os.PathError
	if !errors.As(raised, &pe) || !errors.Is(raised, syscall.ENOENT) {
		t.Errorf("a missing path raises its OSError: %v", raised)
	}
}

func TestClean(t *testing.T) {
	if _, err := exec.LookPath("bash"); err != nil {
		t.Skip("no bash")
	}
	root := t.TempDir()
	script := filepath.Join(root, CleanScript)
	if err := os.MkdirAll(filepath.Dir(script), 0o777); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(script, []byte("echo cleaned one\necho \"to stderr\" >&2\nsleep \"${SLEEP:-0}\"\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	path := map[string]string{"PATH": os.Getenv("PATH")}
	// What the Python answered for the same script and environments.
	for _, c := range []struct {
		root string
		env  map[string]string
		want string
	}{
		{root, path, "cleaned one\nto stderr"},
		{root, map[string]string{"PATH": "/nonexistent"}, "the clean-up did not run: [Errno 2] No such file or directory: 'bash'"},
		{root, map[string]string{}, "cleaned one\nto stderr"},
		{"/no/root", path, "bash: /no/root/bot/machine/clean.sh: No such file or directory"},
	} {
		if got := Clean(c.root, c.env); got != c.want {
			t.Errorf("got %q, want %q", got, c.want)
		}
	}
	old := CleanSeconds
	CleanSeconds = 1
	t.Cleanup(func() { CleanSeconds = old })
	want := fmt.Sprintf("the clean-up did not run: Command '['bash', '%s']' timed out after 1 seconds", script)
	if got := Clean(root, map[string]string{"PATH": os.Getenv("PATH"), "SLEEP": "5"}); got != want {
		t.Errorf("got %q, want %q", got, want)
	}
}
