// Package settle is a pull request's test-merges before it goes ready (#2948, section 2).
//
// After its own review run approves a draft, and before it is marked ready, it is test-merged with
// `git merge-tree --write-tree`, which merges two commits into a tree with no checkout and no model:
// against `main` (`TestMerge`), and against `main` as it will be once each other open pull request
// whose changed files overlap it has landed (`Pulls`, `Landed`). The work job runs them, with the
// job's read-only token (git fetches the other pull requests' heads, `refs/pull/<n>/head`, and the
// job's client lists them); deliver, which holds the state file, decides which of them are ahead
// (approved earlier and conflict-free) and runs them itself when the work job could not.
package settle

import (
	"fmt"
	"slices"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
)

// PullsLimit is how many open pull requests a test-merge reads.
var PullsLimit = 100

// PullRef is where a pull request's head is fetched to: `refs/settle/pull/<n>`.
var PullRef = "refs/settle/pull/"

// Merge is what a test-merge found: the tree it made, the files it left conflicted, or why git could
// not tell (`Problem`, then nothing else is said).
type Merge struct {
	Tree      string
	Conflicts []string
	Problem   string
}

// Clean is: git merged it with no conflict.
func (x *Merge) Clean() bool { return x.Problem == "" && len(x.Conflicts) == 0 }

// TestMerge test-merges `theirs` into `ours` in `repo`, with no checkout (`git merge-tree
// --write-tree`): exit 0 is clean, 1 conflicted (the tree, then the conflicted files), anything
// else a merge git could not make.
func TestMerge(repo *git.Git, ours, theirs string) *Merge {
	proc := repo.Run(false, 600, nil, "merge-tree", "--write-tree", "--name-only", "--no-messages", ours, theirs)
	var lines []string
	for _, line := range py.Splitlines(proc.Stdout) {
		if line = py.Strip(line); line != "" {
			lines = append(lines, line)
		}
	}
	if (proc.Returncode != 0 && proc.Returncode != 1) || len(lines) == 0 {
		text := proc.Stderr
		if text == "" {
			text = proc.Stdout
		}
		return &Merge{Problem: fmt.Sprintf("git could not test-merge `%s` into `%s` (%d): %s", short(theirs),
			short(ours), proc.Returncode, py.SliceTo(py.Strip(text), 200))}
	}
	found := &Merge{Tree: lines[0], Conflicts: []string{}}
	if proc.Returncode == 1 {
		for _, p := range lines[1:] {
			if !slices.Contains(found.Conflicts, p) {
				found.Conflicts = append(found.Conflicts, p)
			}
		}
	}
	return found
}

// Landed is `pull` merged into `base` as a commit with no ref (`git commit-tree` of their clean
// test-merge): `main` as it will be once that pull request lands, for testing what merges after it.
// "" and why when the two do not merge cleanly.
func Landed(repo *git.Git, base, pull string, who *git.Identity) (string, string) {
	merged := TestMerge(repo, base, pull)
	if !merged.Clean() {
		if merged.Problem != "" {
			return "", merged.Problem
		}
		return "", "it conflicts with `main` itself"
	}
	env := map[string]string{"GIT_AUTHOR_NAME": who.Name, "GIT_AUTHOR_EMAIL": who.Email,
		"GIT_COMMITTER_NAME": who.Name, "GIT_COMMITTER_EMAIL": who.Email}
	proc := repo.Run(false, 600, env, "commit-tree", merged.Tree, "-p", base, "-p", pull, "-m",
		"settle: "+short(pull)+" landed on "+short(base))
	if proc.Returncode != 0 {
		return "", "git could not make the landed commit: " + py.SliceTo(py.Strip(proc.Stderr), 200)
	}
	return py.Strip(proc.Stdout), ""
}

// Pull is an open pull request whose changed files overlap this one's, and what this one's
// test-merge after it found.
type Pull struct {
	Number int
	// Head is the commit fetched for it.
	Head string
	// Overlap is the files both change.
	Overlap []string
	// Conflicts is the files this one conflicts in once that one has landed.
	Conflicts []string
	// Problem is why it could not be tested: "it conflicts with `main` itself" takes it out of the
	// order until it is resolved; any other leaves it unknown.
	Problem string
}

// ToDict is the pull request's entry in the run's result.
func (x *Pull) ToDict() *pyjson.Object {
	return pyjson.New("number", x.Number, "head", x.Head, "overlap", pyjson.ListOf(x.Overlap),
		"conflicts", pyjson.ListOf(x.Conflicts), "problem", x.Problem)
}

// FromDict is a Pull read back from a result (`ToDict`).
func FromDict(entry *pyjson.Object) *Pull {
	return &Pull{Number: py.MustInt(py.Or(entry.Get("number"), 0)), Head: py.OrStr(entry.Get("head"), ""),
		Overlap: strs(entry.Get("overlap")), Conflicts: strs(entry.Get("conflicts")),
		Problem: py.OrStr(entry.Get("problem"), "")}
}

// Conflicted is: this one conflicts once that one has landed.
func (x *Pull) Conflicted() bool { return x.Problem == "" && len(x.Conflicts) > 0 }

func strs(v any) []string {
	out := []string{}
	for _, s := range pyjson.List(v) {
		out = append(out, py.Str(s))
	}
	return out
}

// Pulls test-merges pull request `number`, at `head`, after each other open pull request into
// `branch` whose changed files overlap it: each is fetched from origin into `repo` (its
// `refs/pull/<n>/head`, else its branch), `main` (`origin/<branch>`, which the caller fetched) is
// merged with it as it will land (`Landed`), and `head` is test-merged into that. Pull requests that
// overlap nothing are left out. The second value is why the open pull requests could not be read,
// "" when they were.
func Pulls(repo *git.Git, client gh.Client, number int, head string, branch string, who *git.Identity) ([]*Pull, string) {
	found := []*Pull{}
	if client == nil {
		return found, "this run has no client to read the open pull requests with"
	}
	var listed []*pyjson.Object
	if err := herr.Try(func() { listed = client.ListPulls("open", "", PullsLimit) }); err != nil {
		return found, "the open pull requests could not be read: " + py.SliceTo(redact.Redact(herr.Str(err), nil), 200)
	}
	base := "origin/" + branch
	mine := repo.ChangedPaths(base, head)
	var others []*pyjson.Object
	for _, pull := range listed {
		other := py.MustInt(py.Or(pull.Get("number"), 0))
		if other != 0 && other != number && pyjson.Equal(pull.Obj("base").Get("ref"), branch) {
			others = append(others, pull)
		}
	}
	batched := fetchAll(repo, others)
	for _, pull := range others {
		other := py.MustInt(pull.Get("number"))
		entry := &Pull{Number: other, Overlap: []string{}, Conflicts: []string{}}
		sha := ""
		if batched {
			sha = repo.Rev(fmt.Sprintf("%s%d", PullRef, other))
		}
		if sha == "" {
			sha = fetchPull(repo, other, py.OrStr(pull.Obj("head").Get("ref"), ""))
		}
		if sha == "" {
			entry.Problem = "its head could not be fetched"
			found = append(found, entry)
			continue
		}
		entry.Head = sha
		theirs := repo.ChangedPaths(base, sha)
		for _, p := range mine {
			if slices.Contains(theirs, p) {
				entry.Overlap = append(entry.Overlap, p)
			}
		}
		if len(entry.Overlap) == 0 {
			continue
		}
		landed, why := Landed(repo, base, sha, who)
		if why != "" {
			entry.Problem = why
			found = append(found, entry)
			continue
		}
		merged := TestMerge(repo, head, landed)
		entry.Conflicts, entry.Problem = merged.Conflicts, merged.Problem
		if entry.Conflicts == nil {
			entry.Conflicts = []string{}
		}
		found = append(found, entry)
	}
	return found, ""
}

// fetchAll fetches every one of `pulls`' heads (`refs/pull/<n>/head`) into `refs/settle/pull/<n>`
// in one fetch: true when git fetched them all.
func fetchAll(repo *git.Git, pulls []*pyjson.Object) bool {
	if len(pulls) == 0 {
		return false
	}
	args := []string{"fetch", "--quiet", "--no-tags", "origin"}
	for _, pull := range pulls {
		n := py.MustInt(pull.Get("number"))
		args = append(args, fmt.Sprintf("+refs/pull/%d/head:%s%d", n, PullRef, n))
	}
	return repo.Run(false, 600, nil, args...).Returncode == 0
}

// fetchPull fetches pull request `number`'s head into `refs/settle/pull/<n>`: GitHub's
// `refs/pull/<n>/head`, else its branch (`ref`, a branch of this repository). The commit, or "".
func fetchPull(repo *git.Git, number int, ref string) string {
	local := fmt.Sprintf("%s%d", PullRef, number)
	refspecs := []string{fmt.Sprintf("+refs/pull/%d/head:%s", number, local)}
	if ref != "" {
		refspecs = append(refspecs, "+refs/heads/"+ref+":"+local)
	}
	for _, refspec := range refspecs {
		if repo.Run(false, 600, nil, "fetch", "--quiet", "--no-tags", "origin", refspec).Returncode == 0 {
			if sha := repo.Rev(local); sha != "" {
				return sha
			}
		}
	}
	return ""
}

// short is a commit's first 12 characters, or the ref as it is.
func short(ref string) string {
	if py.Len(ref) == 40 && strings.Trim(ref, "0123456789abcdef") == "" {
		return py.SliceTo(ref, 12)
	}
	return ref
}
