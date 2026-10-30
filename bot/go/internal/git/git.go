// Package git is bot/harness/git.py.
//
// Git, run as a subprocess. No hook ever runs: every command sets `core.hooksPath` to nothing.
package git

import (
	"bytes"
	"context"
	"encoding/base64"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path"
	"sort"
	"strings"
	"syscall"
	"time"
	"unicode"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// NoHooks is put before every command's arguments.
var NoHooks = []string{"-c", "core.hooksPath=/dev/null", "-c", "commit.gpgsign=false"}

// Identity is git.Identity: who a commit is by.
type Identity struct {
	Name  string
	Email string
}

// IdentityBot is Identity.bot: a bot account's commit identity.
func IdentityBot(login string, userID int) *Identity {
	return &Identity{login, fmt.Sprintf("%d+%s@users.noreply.github.com", userID, login)}
}

// MarkerLine is a line git writes into a file it could not merge: the start or the end of a
// conflict. A Markdown heading underline (`=======`) is not one, so the middle marker is left
// out.
var MarkerLine = "^(<<<<<<< |>>>>>>> )"

// Git is git.Git: git in one directory, with one environment.
type Git struct {
	Cwd string
	Env map[string]string
}

// NewGit is Git(...). A nil env is None: the process's own, its secrets removed
// (`config.child_env()`); an empty one is empty.
func NewGit(cwd string, env map[string]string /* = None */) *Git {
	own := map[string]string{}
	if env != nil {
		for k, v := range env {
			own[k] = v
		}
	} else {
		own = config.ChildEnv(nil, nil)
	}
	if _, ok := own["GIT_TERMINAL_PROMPT"]; !ok {
		own["GIT_TERMINAL_PROMPT"] = "0"
	}
	return &Git{Cwd: cwd, Env: own}
}

// CompletedProcess is subprocess.CompletedProcess: what a command did. Returncode is negative
// (the signal) for a command a signal ended, as Python's is.
type CompletedProcess struct {
	Args       []string
	Returncode int
	Stdout     string
	Stderr     string
}

// TimeoutExpired is subprocess.TimeoutExpired: the command ran past its timeout and was killed.
type TimeoutExpired struct {
	Cmd     []string
	Timeout int
	Stdout  string
	Stderr  string
}

func (e *TimeoutExpired) Error() string {
	return fmt.Sprintf("Command '%s' timed out after %d seconds", py.Repr(e.Cmd), e.Timeout)
}

// Run runs `git <NoHooks> <args>` in Cwd and returns what it did; with `check`, a non-zero exit
// raises GitError. Output is text as Python's `text=True` reads it: UTF-8 with each bad sequence
// replaced, and \r\n and \r turned to \n. `extraEnv` nil is None.
func (x *Git) Run(check bool /* = True */, timeout int /* = 600 */, extraEnv map[string]string /* = None */, args ...string) *CompletedProcess {
	env := map[string]string{}
	for k, v := range x.Env {
		env[k] = v
	}
	for k, v := range extraEnv {
		env[k] = v
	}
	argv := append(append([]string{"git"}, NoHooks...), args...)
	proc := runText(argv, x.Cwd, env, timeout)
	if check && proc.Returncode != 0 {
		text := proc.Stderr
		if text == "" {
			text = proc.Stdout
		}
		herr.Raise(&herr.GitError{Msg: fmt.Sprintf("git %s failed (%d): %s",
			strings.Join(args, " "), proc.Returncode, py.SliceFrom(py.Strip(text), -800))})
	}
	return proc
}

// runText is subprocess.run(argv, cwd=cwd, env=env, capture_output=True, text=True,
// encoding="utf-8", errors="replace", timeout=timeout).
func runText(argv []string, cwd string, env map[string]string, timeout int) *CompletedProcess {
	ctx, cancel := context.WithTimeout(context.Background(), time.Duration(timeout)*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, argv[0], argv[1:]...)
	cmd.Dir = cwd
	keys := make([]string, 0, len(env))
	for k := range env {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	cmd.Env = make([]string, 0, len(keys))
	for _, k := range keys {
		cmd.Env = append(cmd.Env, k+"="+env[k])
	}
	var stdout, stderr bytes.Buffer
	cmd.Stdout, cmd.Stderr = &stdout, &stderr
	// Python waits for the process it killed, not for pipes a grandchild may still hold.
	cmd.WaitDelay = time.Second
	err := cmd.Run()
	proc := &CompletedProcess{Args: argv, Stdout: text(stdout.Bytes()), Stderr: text(stderr.Bytes())}
	if err != nil && ctx.Err() == context.DeadlineExceeded {
		herr.Raise(&TimeoutExpired{Cmd: argv, Timeout: timeout, Stdout: proc.Stdout, Stderr: proc.Stderr})
	}
	var exit *exec.ExitError
	switch {
	case err == nil, errors.Is(err, exec.ErrWaitDelay):
		proc.Returncode = 0
	case errors.As(err, &exit):
		proc.Returncode = exit.ExitCode()
		if status, ok := exit.Sys().(syscall.WaitStatus); ok && status.Signaled() {
			proc.Returncode = -int(status.Signal())
		}
	default:
		herr.Raise(err) // OSError: git is not there, or the directory is not
	}
	return proc
}

// text is bytes as `text=True` reads them: decode("utf-8", "replace"), then universal newlines.
func text(b []byte) string { return py.Universal(py.DecodeReplace(b)) }

// Out is the stripped stdout of Run(check, 600, nil, args...).
func (x *Git) Out(check bool /* = True */, args ...string) string {
	return py.Strip(x.Run(check, 600, nil, args...).Stdout)
}

// ------------------------------------------------------------------ facts

func (x *Git) Head() string {
	return x.Out(true, "rev-parse", "HEAD")
}

// Rev is the commit `ref` names, or "" (None) when it names none.
func (x *Git) Rev(ref string) string {
	proc := x.Run(false, 600, nil, "rev-parse", "--verify", "--quiet", ref+"^{commit}")
	return py.Strip(proc.Stdout)
}

func (x *Git) MergeBase(a string, b string /* = "HEAD" */) string {
	return x.Out(true, "merge-base", a, b)
}

// ChangedPaths is the paths the branch changed since it left `base` (merge-base diff, renames
// split).
func (x *Git) ChangedPaths(base string, head string /* = "HEAD" */) []string {
	return nonBlank(x.Out(true, "diff", "--name-only", "--no-renames", base+"..."+head))
}

// nonBlank is `[line for line in text.splitlines() if line.strip()]`.
func nonBlank(text string) []string {
	out := []string{}
	for _, line := range py.Splitlines(text) {
		if py.Strip(line) != "" {
			out = append(out, line)
		}
	}
	return out
}

// Markers is the `paths` that hold a conflict marker line at `ref` (`MarkerLine`).
func (x *Git) Markers(ref string, paths []string) []string {
	var wanted []string
	for _, p := range paths {
		if p != "" {
			wanted = append(wanted, p)
		}
	}
	if len(wanted) == 0 {
		return []string{}
	}
	proc := x.Run(false, 600, nil, append([]string{"grep", "-l", "-E", MarkerLine, ref, "--"}, wanted...)...)
	found := []string{}
	for _, line := range py.Splitlines(proc.Stdout) {
		p := line
		if strings.HasPrefix(line, ref+":") {
			_, p, _ = strings.Cut(line, ":")
		}
		if py.Strip(p) != "" {
			found = append(found, py.Strip(p))
		}
	}
	return found
}

// Numstat is the lines added plus removed per path since `head` left `base` (a binary file
// counts 0), in git's order.
func (x *Git) Numstat(base string, head string /* = "HEAD" */) *pyjson.Object {
	text := x.Out(true, "diff", "--numstat", "--no-renames", base+"..."+head)
	counts := pyjson.New()
	for _, line := range py.Splitlines(text) {
		parts := strings.SplitN(line, "\t", 3)
		if len(parts) != 3 {
			continue
		}
		counts.Set(parts[2], count(parts[0])+count(parts[1]))
	}
	return counts
}

// count is `int(n) if n.isdigit() else 0`.
func count(n string) int {
	if n == "" {
		return 0
	}
	value := 0
	for _, r := range n {
		if !unicode.IsDigit(r) {
			return 0
		}
		value = value*10 + py.DigitValue(r)
	}
	return value
}

// NamesBetween is the paths that differ between two commits (a two-dot diff, renames split).
func (x *Git) NamesBetween(a string, b string /* = "HEAD" */) []string {
	return nonBlank(x.Out(true, "diff", "--name-only", "--no-renames", a, b))
}

// Blob is the blob id of `path` at `ref`, or "" (None) when it is not there.
func (x *Git) Blob(ref string, path string) string {
	proc := x.Run(false, 600, nil, "rev-parse", "--verify", "--quiet", ref+":"+path)
	return py.Strip(proc.Stdout)
}

// Unsanctioned is the paths matching `patterns` that `head` holds differently from every anchor.
//
// The anchors are the commits the work may legitimately carry content from: where the branch
// started and the default branch it merged. A path that equals one of them came from there; a
// path that equals none was changed by the work itself.
func (x *Git) Unsanctioned(head string, anchors []string, patterns []string) []string {
	candidates := map[string]bool{}
	for _, anchor := range anchors {
		for _, p := range x.NamesBetween(anchor, head) {
			candidates[p] = true
		}
	}
	sorted := make([]string, 0, len(candidates))
	for p := range candidates {
		sorted = append(sorted, p)
	}
	sort.Strings(sorted)
	found := []string{}
	for _, p := range sorted {
		if !Matches(p, patterns) {
			continue
		}
		atHead := x.Blob(head, p)
		same := false
		for _, anchor := range anchors {
			if x.Blob(anchor, p) == atHead {
				same = true
				break
			}
		}
		if same {
			continue
		}
		found = append(found, p)
	}
	return found
}

// RestoreFrom puts each path back as the first anchor that has it holds it, or deletes it.
func (x *Git) RestoreFrom(anchors []string, paths []string) {
	for _, p := range paths {
		source, ok := "", false
		for _, a := range anchors {
			if x.Blob(a, p) != "" {
				source, ok = a, true
				break
			}
		}
		if ok {
			x.Run(true, 600, nil, "checkout", source, "--", p)
			continue
		}
		x.Run(true, 600, nil, "rm", "-r", "--quiet", "--cached", "--ignore-unmatch", "--", p)
		target := joinPath(x.Cwd, p)
		info, err := os.Lstat(target)
		if err != nil {
			continue
		}
		isFile := info.Mode()&os.ModeSymlink != 0
		if !isFile {
			isFile = info.Mode().IsRegular()
		}
		if isFile {
			herr.Must(os.Remove(target))
		}
	}
}

// Diff is the merge-base diff and whether it was cut to `maxChars`.
func (x *Git) Diff(base string, head string /* = "HEAD" */, maxChars int /* = 200_000 */) (string, bool) {
	text := x.Out(true, "diff", "--stat", "--patch", "--no-color", base+"..."+head)
	if py.Len(text) > maxChars {
		return py.SliceTo(text, maxChars), true
	}
	return text, false
}

func (x *Git) Diffstat(base string, head string /* = "HEAD" */) string {
	return x.Out(true, "diff", "--stat", "--no-color", base+"..."+head)
}

// Dirty is the porcelain lines for tracked changes and untracked files that are not ignored.
func (x *Git) Dirty() []string {
	return nonBlank(x.Run(true, 600, nil, "status", "--porcelain", "--untracked-files=all").Stdout)
}

func (x *Git) Log(base string, head string /* = "HEAD" */) string {
	return x.Out(false, "log", "--oneline", "--no-decorate", base+".."+head)
}

// ------------------------------------------------------------------ changes

// CommitAll stages everything (ignored files excepted) and commits. False when nothing changed.
func (x *Git) CommitAll(message string, who *Identity) bool {
	x.Run(true, 600, nil, "add", "--all")
	if x.Run(false, 600, nil, "diff", "--cached", "--quiet").Returncode == 0 {
		return false
	}
	x.Run(true, 600, who.env(), "commit", "--quiet", "--no-verify", "-m", message)
	return true
}

func (who *Identity) env() map[string]string {
	return map[string]string{
		"GIT_AUTHOR_NAME":     who.Name,
		"GIT_AUTHOR_EMAIL":    who.Email,
		"GIT_COMMITTER_NAME":  who.Name,
		"GIT_COMMITTER_EMAIL": who.Email,
	}
}

// DiscardWorktreeChanges drops uncommitted edits and untracked, non-ignored files.
func (x *Git) DiscardWorktreeChanges() {
	x.Run(true, 600, nil, "reset", "--quiet", "--hard", "HEAD")
	x.Run(true, 600, nil, "clean", "-fdq")
}

// Merge merges `ref` into the current branch. Returns conflicted paths (merge left open).
func (x *Git) Merge(ref string, who *Identity) []string {
	proc := x.Run(false, 600, who.env(), "merge", "--no-edit", "--no-ff", ref)
	if proc.Returncode == 0 {
		return []string{}
	}
	conflicted := x.Out(false, "diff", "--name-only", "--diff-filter=U")
	paths := nonBlank(conflicted)
	if len(paths) == 0 {
		text := proc.Stderr
		if text == "" {
			text = proc.Stdout
		}
		herr.Raise(&herr.GitError{Msg: fmt.Sprintf("git merge %s failed: %s", ref, py.SliceFrom(py.Strip(text), -800))})
	}
	return paths
}

func (x *Git) MergeInProgress() bool {
	return x.Run(false, 600, nil, "rev-parse", "-q", "--verify", "MERGE_HEAD").Returncode == 0
}

// ------------------------------------------------------------------ transport

// Bundle writes a bundle holding `branch` and every commit it has that no `exclude` commit has.
func (x *Git) Bundle(path string, branch string, exclude []string) {
	args := []string{"bundle", "create", path, "refs/heads/" + branch}
	for _, sha := range exclude {
		args = append(args, "^"+sha)
	}
	x.Run(true, 600, nil, args...)
}

func (x *Git) FetchBundle(path string, branch string, local string) string {
	x.Run(true, 600, nil, "bundle", "verify", path)
	x.Run(true, 600, nil, "fetch", "--quiet", path, "+refs/heads/"+branch+":refs/heads/"+local)
	return x.Out(true, "rev-parse", local)
}

// Push pushes with the token in an http.extraheader passed through the environment.
func (x *Git) Push(remoteURL string, refspec string, token string, forceWithLease string /* = "" */) {
	basic := base64.StdEncoding.EncodeToString([]byte("x-access-token:" + token))
	env := map[string]string{
		"GIT_CONFIG_COUNT":   "2",
		"GIT_CONFIG_KEY_0":   "http.https://github.com/.extraheader",
		"GIT_CONFIG_VALUE_0": "AUTHORIZATION: basic " + basic,
		"GIT_CONFIG_KEY_1":   "credential.helper",
		"GIT_CONFIG_VALUE_1": "",
	}
	args := []string{"push", "--quiet", "--no-verify"}
	if forceWithLease != "" {
		args = append(args, "--force-with-lease="+forceWithLease)
	}
	x.Run(true, 300, env, append(args, remoteURL, refspec)...)
}

// Normalise is a repository path in one canonical form: forward slashes, no `./`, lower case.
func Normalise(p string) string {
	clean := strings.TrimLeft(path.Clean(strings.ReplaceAll(p, `\`, "/")), "/")
	for strings.HasPrefix(clean, "./") {
		clean = clean[2:]
	}
	return strings.ToLower(clean)
}

// Matches is true when `path` matches a pattern: `dir/` is that directory and everything under
// it, a pattern with a `/` is a glob on the whole path, and a bare name is a glob on the last
// part of the path at any depth. Case never matters. A pattern that starts with `!` is an
// exception, as in a `.gitignore`: a path it matches matches none of the others (`.claude/` with
// `!.claude/skills/` is everything under `.claude/` but its skills).
func Matches(path string, patterns []string) bool {
	var exceptions, others []string
	for _, p := range patterns {
		if strings.HasPrefix(p, "!") {
			exceptions = append(exceptions, p[1:])
		} else {
			others = append(others, p)
		}
	}
	if len(exceptions) > 0 && matches(path, exceptions) {
		return false
	}
	return matches(path, others)
}

func matches(path string, patterns []string) bool {
	clean := Normalise(path)
	name := clean
	if i := strings.LastIndex(clean, "/"); i >= 0 {
		name = clean[i+1:]
	}
	for _, raw := range patterns {
		pattern := Normalise(raw)
		if strings.HasSuffix(raw, "/") {
			pattern += "/"
		}
		switch {
		case strings.HasSuffix(pattern, "/"):
			if clean == strings.TrimRight(pattern, "/") || strings.HasPrefix(clean, pattern) {
				return true
			}
		case strings.Contains(pattern, "/"):
			if py.Fnmatch(clean, pattern) {
				return true
			}
		default:
			if py.Fnmatch(name, pattern) || py.Fnmatch(clean, pattern) {
				return true
			}
		}
	}
	return false
}

// WorktreeAdd makes a worktree at `path` with `branch` (re)set to `start` and checked out. An
// older worktree of this clone that still has `branch` checked out (an earlier item on the same
// branch) is removed first, since git lets only one worktree hold a branch.
func WorktreeAdd(repo *Git, path string, branch string, start string) *Git {
	if _, err := os.Stat(path); err == nil {
		repo.Run(false, 600, nil, "worktree", "remove", "--force", path)
	}
	listing := repo.Run(false, 600, nil, "worktree", "list", "--porcelain").Stdout
	current := ""
	for _, line := range py.Splitlines(listing) {
		if strings.HasPrefix(line, "worktree ") {
			current = line[len("worktree "):]
		} else if line == "branch refs/heads/"+branch && current != "" && purePath(current) != purePath(repo.Cwd) {
			repo.Run(false, 600, nil, "worktree", "remove", "--force", current)
		}
	}
	repo.Run(false, 600, nil, "worktree", "prune")
	repo.Run(true, 600, nil, "worktree", "add", "-q", "-B", branch, path, start)
	return NewGit(path, repo.Env)
}

// purePath is a path as pathlib compares it: repeated slashes and `.` parts gone, no trailing
// slash, `..` kept as it is.
func purePath(p string) string {
	if p == "" {
		return "."
	}
	parts := []string{}
	for _, part := range strings.Split(p, "/") {
		if part != "" && part != "." {
			parts = append(parts, part)
		}
	}
	joined := strings.Join(parts, "/")
	if strings.HasPrefix(p, "/") {
		return "/" + joined
	}
	if joined == "" {
		return "."
	}
	return joined
}

// joinPath is `Path(cwd) / path`: an absolute path stands alone.
func joinPath(dir, p string) string {
	if strings.HasPrefix(p, "/") || dir == "" {
		return p
	}
	return strings.TrimRight(dir, "/") + "/" + p
}
