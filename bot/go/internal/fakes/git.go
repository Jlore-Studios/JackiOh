package fakes

import (
	"bytes"
	"errors"
	"fmt"
	"maps"
	"os"
	"os/exec"
	"path/filepath"
	"slices"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// ---------------------------------------------------------------------- git helpers

// GitEnv = {"GIT_AUTHOR_NAME": "Test", ...}
var GitEnv = map[string]string{
	"GIT_AUTHOR_NAME": "Test", "GIT_AUTHOR_EMAIL": "test@example.com",
	"GIT_COMMITTER_NAME": "Test", "GIT_COMMITTER_EMAIL": "test@example.com",
	"GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_NOSYSTEM": "1",
}

// Git runs git in `cwd` and returns its output, stripped. A failure raises an AssertionError
// (a *PyError) carrying git's stderr, as the Python's does.
func Git(cwd string, args ...string) string {
	cmd := exec.Command("git", args...)
	cmd.Dir = cwd
	cmd.Env = gitEnv()
	var stdout, stderr bytes.Buffer
	cmd.Stdout, cmd.Stderr = &stdout, &stderr
	if err := cmd.Run(); err != nil {
		var exit *exec.ExitError
		if !errors.As(err, &exit) {
			panic(err) // no git to run: Python's FileNotFoundError
		}
		panic(&PyError{Type: "AssertionError", Msg: fmt.Sprintf("git %s: %s", strings.Join(args, " "), stderr.String())})
	}
	return strings.TrimSpace(stdout.String())
}

// gitEnv is {**os.environ, **GIT_ENV}.
func gitEnv() []string {
	var env []string
	for _, kv := range os.Environ() {
		name, _, _ := strings.Cut(kv, "=")
		if _, ours := GitEnv[name]; !ours {
			env = append(env, kv)
		}
	}
	for _, name := range slices.Sorted(maps.Keys(GitEnv)) {
		env = append(env, name+"="+GitEnv[name])
	}
	return env
}

// MakeOrigin is a bare `origin` holding a `main` branch, and a clone of it. Returns (origin,
// clone). `files` (name → text) defaults to a README and src/game.txt.
func MakeOrigin(root string, files ...map[string]string) (origin, clone string) {
	seed := filepath.Join(root, "seed")
	mkdirNew(seed)
	Git(seed, "init", "-q", "-b", "main")
	given := optional(files, "MakeOrigin", nil)
	if len(given) == 0 {
		given = map[string]string{"README.md": "hello\n", "src/game.txt": "rules v1\n"}
	}
	writeFiles(seed, given)
	Git(seed, "add", "-A")
	Git(seed, "commit", "-q", "-m", "initial")
	origin = filepath.Join(root, "remote", "jgoetzmann", "JackiOh.git")
	mkdirNew(filepath.Dir(origin))
	Git(root, "clone", "-q", "--bare", seed, origin)
	clone = filepath.Join(root, "clone")
	Git(root, "clone", "-q", origin, clone)
	return origin, clone
}

// PushBranch puts a branch on `origin` that changes `files` on top of `base` (default "main").
// Returns its sha.
func PushBranch(origin, root, branch string, files map[string]string, base ...string) string {
	from := optional(base, "PushBranch", "main")
	work := filepath.Join(root, "push-"+strings.ReplaceAll(branch, "/", "-"))
	if _, err := os.Stat(work); err != nil {
		Git(root, "clone", "-q", origin, work)
	}
	Git(work, "fetch", "-q", "origin")
	Git(work, "checkout", "-q", "-B", branch, "origin/"+from)
	writeFiles(work, files)
	Git(work, "add", "-A")
	Git(work, "commit", "-q", "-m", "change on "+branch)
	Git(work, "push", "-q", "-f", "origin", branch+":"+branch)
	return Git(work, "rev-parse", "HEAD")
}

// writeFiles is `target.parent.mkdir(parents=True, exist_ok=True); target.write_text(text)` for
// each file (in sorted order: a Go map has no other).
func writeFiles(dir string, files map[string]string) {
	for _, name := range slices.Sorted(maps.Keys(files)) {
		target := filepath.Join(dir, name)
		if err := os.MkdirAll(filepath.Dir(target), 0o777); err != nil {
			panic(err)
		}
		if err := os.WriteFile(target, []byte(files[name]), 0o666); err != nil {
			panic(err)
		}
	}
}

// mkdirNew is path.mkdir(parents=True): a FileExistsError when it is there already.
func mkdirNew(path string) {
	if _, err := os.Lstat(path); err == nil {
		panic(&PyError{Type: "FileExistsError", Msg: fmt.Sprintf("[Errno 17] File exists: %s", py.Repr(path))})
	}
	if err := os.MkdirAll(path, 0o777); err != nil {
		panic(err)
	}
}

// optional is a trailing positional argument with a default: the one given, or `def`.
func optional[T any](given []T, fn string, def T) T {
	switch len(given) {
	case 0:
		return def
	case 1:
		return given[0]
	}
	panic(&PyError{Type: "TypeError", Msg: fmt.Sprintf("%s takes at most one optional argument, got %d", fn, len(given))})
}
