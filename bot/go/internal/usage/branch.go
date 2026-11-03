package usage

import (
	"errors"
	"os"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// GitTimeout is how long one git command on the usage branch (or the journal's fetch) may take, in
// seconds: a shallow fetch of a branch of a few MB, or a push of a flush's rows. It bounds how long
// a flush can hold up the loop's tick.
var GitTimeout = 120

// Branch is a local clone of the usage branch, kept for the status loop's lifetime (the files
// grow to a few MB a month, past what the Contents API reads comfortably): rows are appended to
// its files, committed, and pushed at the loop's flushes.
type Branch struct {
	// Dir is the clone's root: the files are under `Dir/usage/`.
	Dir    string
	Name   string
	Remote string
	Token  string
	Who    *git.Identity
	Git    *git.Git
	// New: the remote had no such branch; this clone started it, and its first push makes it.
	New bool
}

// ErrRejected is a push the remote refused because the branch moved under it: fetch it again
// (`Sync`), write the rows again on top and push again.
var ErrRejected = errors.New("the usage branch moved: the push was rejected")

// OpenBranch clones `name` from `remote` (shallow: its tip alone) into a new directory under
// `parent`, or, when the remote has no such branch yet, starts it there as an orphan with an empty
// first commit, whose first push creates it. `token` ("" for none) is handed to git for GitHub.
func OpenBranch(parent, remote, name, token string, who *git.Identity) (*Branch, error) {
	if err := os.MkdirAll(parent, 0o777); err != nil {
		return nil, err
	}
	dir, err := os.MkdirTemp(parent, "usage-")
	if err != nil {
		return nil, err
	}
	b := &Branch{Dir: dir, Name: name, Remote: remote, Token: token, Who: who, Git: git.NewGit(dir, nil)}
	err = herr.Try(func() {
		b.run("init", "--quiet")
		b.run("symbolic-ref", "HEAD", "refs/heads/"+name)
		b.run("remote", "add", "origin", remote)
		heads := b.run("ls-remote", "--heads", "origin", "refs/heads/"+name)
		if py.Strip(heads) == "" {
			b.New = true
			b.Git.CommitEmpty("usage: start [skip ci]", who)
			return
		}
		b.fetch()
		b.run("checkout", "--quiet", "-B", name, "refs/remotes/origin/"+name)
	})
	if err != nil {
		os.RemoveAll(dir)
		return nil, err
	}
	return b, nil
}

// env is the environment that hands git the token, nil for none.
func (b *Branch) env() map[string]string {
	if b.Token == "" {
		return nil
	}
	return git.AuthEnv(b.Token)
}

// run is one git command in the clone, which raises when it fails; its stdout.
func (b *Branch) run(args ...string) string {
	return b.Git.Run(true, GitTimeout, b.env(), args...).Stdout
}

// fetch is the branch's tip, shallow, into `origin/<name>`.
func (b *Branch) fetch() {
	b.run("fetch", "--quiet", "--no-tags", "--depth=1", "origin",
		"+refs/heads/"+b.Name+":refs/remotes/origin/"+b.Name)
}

// Commit commits everything in the clone with `message`; false when nothing changed.
func (b *Branch) Commit(message string) (bool, error) {
	committed := false
	err := herr.Try(func() { committed = b.Git.CommitAll(message+" [skip ci]", b.Who) })
	return committed, err
}

// Ahead is: the clone has commits the remote's branch does not (or the branch is not there yet).
func (b *Branch) Ahead() bool {
	if b.New {
		return true
	}
	proc := b.Git.Run(false, GitTimeout, nil, "rev-list", "--count", "refs/remotes/origin/"+b.Name+"..HEAD")
	return proc.Returncode != 0 || py.Strip(proc.Stdout) != "0"
}

// Push pushes the clone's branch. ErrRejected when the remote's branch moved under it (or was
// made since, for a new one); any other failure is git's.
func (b *Branch) Push() error {
	var proc *git.CompletedProcess
	err := herr.Try(func() {
		proc = b.Git.Run(false, GitTimeout, b.env(), "push", "--quiet", "--no-verify", "origin",
			"HEAD:refs/heads/"+b.Name)
	})
	if err != nil {
		return err
	}
	if proc.Returncode != 0 {
		said := proc.Stderr + proc.Stdout
		for _, word := range []string{"[rejected]", "non-fast-forward", "fetch first", "stale info"} {
			if strings.Contains(said, word) {
				return ErrRejected
			}
		}
		return errors.New("git push failed: " + py.SliceFrom(py.Strip(said), -400))
	}
	b.New = false
	b.Git.Run(false, GitTimeout, nil, "update-ref", "refs/remotes/origin/"+b.Name, "HEAD")
	return nil
}

// Sync puts the clone at the remote branch's tip, dropping its own commits and files (the rows
// they held are written again on top: `Recorder`).
func (b *Branch) Sync() error {
	return herr.Try(func() {
		b.fetch()
		b.run("reset", "--quiet", "--hard", "refs/remotes/origin/"+b.Name)
		b.run("clean", "-fdq")
		b.New = false
	})
}

// Close removes the clone.
func (b *Branch) Close() { os.RemoveAll(b.Dir) }
