// Package git is bot/harness/git.py.
//
// Git, run as a subprocess. No hook ever runs: every command sets `core.hooksPath` to nothing.
package git

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// NO_HOOKS = ("-c", "core.hooksPath=/dev/null", "-c", "commit.gpgsign=false")
var NoHooks = []string{"-c", "core.hooksPath=/dev/null", "-c", "commit.gpgsign=false"}

// Identity is git.Identity.

type Identity struct {
	Name  string
	Email string
}

func IdentityBot(login string, userID int) *Identity {
	panic("unported: git.Identity.bot")
}

// MARKER_LINE = r"^(<<<<<<< |>>>>>>> )"
var MarkerLine string = "^(<<<<<<< |>>>>>>> )"

// Git is git.Git.

type Git struct {
	Cwd any // Path(cwd)
	Env any // dict(env) if env is not None else config_mod.child_env()
}

// NewGit is Git(...).

func NewGit(cwd string, env map[string]string /* = None */) *Git {
	panic("unported: git.Git.__init__")
}

func (x *Git) Run(check bool /* = True */, timeout int /* = 600 */, extraEnv map[string]string /* = None */, args ...string) any {
	panic("unported: git.Git.run")
}

func (x *Git) Out(check bool /* = True */, args ...string) string {
	panic("unported: git.Git.out")
}

func (x *Git) Head() string {
	panic("unported: git.Git.head")
}

func (x *Git) Rev(ref string) string {
	panic("unported: git.Git.rev")
}

func (x *Git) MergeBase(a string, b string /* = "HEAD" */) string {
	panic("unported: git.Git.merge_base")
}

// Paths the branch changed since it left `base` (merge-base diff, renames split).

func (x *Git) ChangedPaths(base string, head string /* = "HEAD" */) []string {
	panic("unported: git.Git.changed_paths")
}

// The `paths` that hold a conflict marker line at `ref` (`MARKER_LINE`).

func (x *Git) Markers(ref string, paths []string) []string {
	panic("unported: git.Git.markers")
}

// Lines added plus removed per path since `head` left `base` (a binary file counts 0).

func (x *Git) Numstat(base string, head string /* = "HEAD" */) *pyjson.Object {
	panic("unported: git.Git.numstat")
}

// Paths that differ between two commits (a two-dot diff, renames split).

func (x *Git) NamesBetween(a string, b string /* = "HEAD" */) []string {
	panic("unported: git.Git.names_between")
}

// The blob id of `path` at `ref`, or None when it is not there.

func (x *Git) Blob(ref string, path string) string {
	panic("unported: git.Git.blob")
}

// Paths matching `patterns` that `head` holds differently from every anchor.
//
// The anchors are the commits the work may legitimately carry content from: where the
// branch started and the default branch it merged. A path that equals one of them came
// from there; a path that equals none was changed by the work itself.

func (x *Git) Unsanctioned(head string, anchors []string, patterns []string) []string {
	panic("unported: git.Git.unsanctioned")
}

// Put each path back as the first anchor that has it holds it, or delete it.

func (x *Git) RestoreFrom(anchors []string, paths []string) {
	panic("unported: git.Git.restore_from")
}

// The merge-base diff and whether it was cut to `max_chars`.

func (x *Git) Diff(base string, head string /* = "HEAD" */, maxChars int /* = 200_000 */) (string, bool) {
	panic("unported: git.Git.diff")
}

func (x *Git) Diffstat(base string, head string /* = "HEAD" */) string {
	panic("unported: git.Git.diffstat")
}

// Porcelain lines for tracked changes and untracked files that are not ignored.

func (x *Git) Dirty() []string {
	panic("unported: git.Git.dirty")
}

func (x *Git) Log(base string, head string /* = "HEAD" */) string {
	panic("unported: git.Git.log")
}

// Stage everything (ignored files excepted) and commit. False when nothing changed.

func (x *Git) CommitAll(message string, who *Identity) bool {
	panic("unported: git.Git.commit_all")
}

// Drop uncommitted edits and untracked, non-ignored files.

func (x *Git) DiscardWorktreeChanges() {
	panic("unported: git.Git.discard_worktree_changes")
}

// Merge `ref` into the current branch. Returns conflicted paths (merge left open).

func (x *Git) Merge(ref string, who *Identity) []string {
	panic("unported: git.Git.merge")
}

func (x *Git) MergeInProgress() bool {
	panic("unported: git.Git.merge_in_progress")
}

// A bundle holding `branch` and every commit it has that no `exclude` commit has.

func (x *Git) Bundle(path string, branch string, exclude []string) {
	panic("unported: git.Git.bundle")
}

func (x *Git) FetchBundle(path string, branch string, local string) string {
	panic("unported: git.Git.fetch_bundle")
}

// Push with the token in an http.extraheader passed through the environment.

func (x *Git) Push(remoteURL string, refspec string, token string, forceWithLease string /* = "" */) {
	panic("unported: git.Git.push")
}

// A repository path in one canonical form: forward slashes, no `./`, lower case.

func Normalise(path string) string {
	panic("unported: git.normalise")
}

// True when `path` matches a pattern: `dir/` is that directory and everything under it,
// a pattern with a `/` is a glob on the whole path, and a bare name is a glob on the last
// part of the path at any depth. Case never matters. A pattern that starts with `!` is an
// exception, as in a `.gitignore`: a path it matches matches none of the others (`.claude/`
// with `!.claude/skills/` is everything under `.claude/` but its skills).

func Matches(path string, patterns []string) bool {
	panic("unported: git.matches")
}

// A worktree at `path` with `branch` (re)set to `start` and checked out. An older worktree
// of this clone that still has `branch` checked out (an earlier item on the same branch) is
// removed first, since git lets only one worktree hold a branch.

func WorktreeAdd(repo *Git, path string, branch string, start string) *Git {
	panic("unported: git.worktree_add")
}
