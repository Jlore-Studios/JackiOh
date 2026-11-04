package deliver

// The merge gates on a condition (#2986, section 1; the owner's decision of 2026-11-04, amended the
// same day; `config/merge.go`, `internal/mergegate`): what a path off `review_paths` must not hold
// for its pull request to merge itself, read from the branch's files with no model. `autoMerge`
// asks after the review paths, and says in its comment which conditional paths merged themselves
// and why, or what sends the pull request to a person.

import (
	"fmt"
	"path"
	"slices"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/mergegate"
)

// GateNames is how many paths or packages a gate's sentence names at most.
const GateNames = 8

// MigrationOK and LocksOK are what a merge note says of a conditional path that merged itself.
const (
	MigrationOK = "a migration with no data-losing or access-changing statement"
	LocksOK     = "no new package in the locks"
)

// branchRef is the ref the branch's head is read at: the deliver job's own copy (`deliver/<branch>`)
// or GitHub's, fetched; "" when neither is there.
func (x *Deliverer) branchRef(branch string) string {
	x.fetch(x.Cfg.DefaultBranch, false)
	local := "deliver/" + branch
	ref := "origin/" + branch
	if x.Repo.Rev(local) != "" {
		ref = local
	}
	if x.Repo.Rev(ref) == "" {
		x.fetch(branch, false)
		if x.Repo.Rev(ref) == "" {
			return ""
		}
	}
	return ref
}

// mergeGates is why the branch's change goes to a person though no review path matched: a migration
// that loses data, changes access, or edits or deletes one `main` has; a lock that gains a package
// `main`'s lock lacks; a Cargo manifest naming a package its lock does not hold; "" when none does.
// `notes` says which conditional paths it changes and why each merges itself, for the comment.
func (x *Deliverer) mergeGates(branch string) (person string, notes []string) {
	if len(x.Cfg.MigrationPaths) == 0 && len(x.Cfg.LockFiles) == 0 {
		return "", nil
	}
	ref := x.branchRef(branch)
	if ref == "" {
		return "", nil
	}
	base := "origin/" + x.Cfg.DefaultBranch
	var migrations, locks, manifests []string
	for _, entry := range x.Repo.ChangedStatus(base, ref) {
		status, p := entry[0], entry[1]
		switch {
		case git.Matches(p, x.Cfg.MigrationPaths):
			if status != "A" {
				what := "edits"
				if status == "D" {
					what = "deletes"
				}
				return fmt.Sprintf("it %s `%s`, a migration `%s` already has, and an applied migration "+
					"never changes", what, p, x.Cfg.DefaultBranch), nil
			}
			text, _ := x.Repo.Show(ref, p)
			if found := mergegate.Migration(text); found != nil {
				return fmt.Sprintf("the migration `%s` %s: `%s`", p, found.Kind, found.Statement), nil
			}
			migrations = append(migrations, p)
		case git.Matches(p, x.Cfg.LockFiles):
			if why := x.lockGains(base, ref, p); why != "" {
				return why, nil
			}
			locks = append(locks, p)
		case len(x.Cfg.LockFiles) > 0 && slices.Contains([]string{"Cargo.toml", "package.json"}, path.Base(p)):
			if path.Base(p) == "Cargo.toml" && status != "D" {
				if why := x.unlocked(ref, p); why != "" {
					return why, nil
				}
			}
			manifests = append(manifests, p)
		}
	}
	if len(migrations) > 0 {
		notes = append(notes, fmt.Sprintf("%s: %s", namesOf(migrations), MigrationOK))
	}
	if changed := append(locks, manifests...); len(changed) > 0 {
		notes = append(notes, fmt.Sprintf("%s: %s", namesOf(changed), LocksOK))
	}
	return "", notes
}

// lockGains is why lock `p` at `ref` goes to a person (it is gone, cannot be read, or names a package
// the lock at `base` lacks), or "".
func (x *Deliverer) lockGains(base, ref, p string) string {
	text, ok := x.Repo.Show(ref, p)
	if !ok {
		return fmt.Sprintf("it deletes the lock `%s`", p)
	}
	now, readable := mergegate.LockPackages(p, text)
	if !readable {
		return fmt.Sprintf("it changes `%s`, a lock whose packages I cannot read", p)
	}
	before := map[string]bool{}
	if old, there := x.Repo.Show(base, p); there {
		names, _ := mergegate.LockPackages(p, old)
		for _, name := range names {
			before[name] = true
		}
	}
	var gained []string
	for _, name := range now {
		if !before[name] && !slices.Contains(gained, name) {
			gained = append(gained, name)
		}
	}
	if len(gained) == 0 {
		return ""
	}
	return fmt.Sprintf("`%s` gains %s, which `%s`'s lock does not have: a new dependency", p,
		packagesOf(gained), x.Cfg.DefaultBranch)
}

// unlocked is why Cargo manifest `p` at `ref` goes to a person: it names a package the `Cargo.lock`
// above it does not hold (CI builds without `--locked`, so such a dependency would never meet the
// lock's check), or "". A manifest with no lock of `lock_files` above it is not checked.
func (x *Deliverer) unlocked(ref, p string) string {
	text, _ := x.Repo.Show(ref, p)
	for dir := path.Dir(p); ; dir = path.Dir(dir) {
		lock := path.Join(dir, "Cargo.lock")
		if dir == "." {
			lock = "Cargo.lock"
		}
		if lockText, ok := x.Repo.Show(ref, lock); ok && git.Matches(lock, x.Cfg.LockFiles) {
			held, _ := mergegate.LockPackages(lock, lockText)
			var missing []string
			for _, name := range mergegate.CargoDependencies(text) {
				if !slices.Contains(held, name) {
					missing = append(missing, name)
				}
			}
			if len(missing) == 0 {
				return ""
			}
			return fmt.Sprintf("`%s` depends on %s, which `%s` does not lock: a new dependency", p,
				packagesOf(missing), lock)
		}
		if dir == "." || dir == "/" {
			return ""
		}
	}
}

// namesOf is `a`, `b` and `c`, at most `GateNames` of them.
func namesOf(paths []string) string {
	shown := ticked(headN(paths, GateNames))
	if len(paths) > GateNames {
		shown += ", …"
	}
	return shown
}

// packagesOf is "the package `a`" or "the packages `a`, `b`".
func packagesOf(names []string) string {
	if len(names) == 1 {
		return "the package `" + names[0] + "`"
	}
	return "the packages " + namesOf(names)
}
