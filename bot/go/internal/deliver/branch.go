package deliver

import (
	"fmt"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
)

// ------------------------------------------------------------------ the branch

func (x *Deliverer) openPull() *pyjson.Object {
	branch := py.OrStr(x.Plan.Get("branch"), "")
	if branch == "" {
		return nil
	}
	return queue.OpenPullForBranch(x.Ctx, branch)
}

// Turn auto-merge off on the branch's open PR (or on PR `number`), and take it out of the
// merge queue, before unapproved work lands on it. `number` 0 is the Python's None.
func (x *Deliverer) holdAutoMerge(number int /* = None */) {
	var pull *pyjson.Object
	if number != 0 {
		pull = x.GH.GetPull(number)
	} else {
		pull = x.openPull()
	}
	if pull != nil && pull.Truthy("node_id") {
		queue.HoldMerge(x.Ctx, pull)
		if pull.Truthy("auto_merge") {
			x.log("auto-merge off on #" + py.Str(pull.Get("number")))
		}
	}
}

// fetch is `self.repo.run("fetch", "--quiet", "--no-tags", "origin",
// f"+refs/heads/{ref}:refs/remotes/origin/{ref}", check=check)`.
func (x *Deliverer) fetch(ref string, check bool) {
	x.Repo.Run(check, 600, nil, "fetch", "--quiet", "--no-tags", "origin",
		fmt.Sprintf("+refs/heads/%s:refs/remotes/origin/%s", ref, ref))
}

// isAncestor is `self.repo.run("merge-base", "--is-ancestor", a, b, check=False).returncode`.
func (x *Deliverer) isAncestor(a, b string) int {
	return x.Repo.Run(false, 600, nil, "merge-base", "--is-ancestor", a, b).Returncode
}

func (x *Deliverer) remoteURL() string {
	return fmt.Sprintf("%s/%s.git", x.Cfg.ServerURL, x.Cfg.Repo)
}

// Push the bundle's branch. `(pushed, problem)`; no bundle is `(False, "")`. A bundle
// whose head holds a conflict marker in a file it changed is refused (#317 part 2).
func (x *Deliverer) publish(approved bool) (bool, string) {
	name := x.Result.Get("bundle")
	branch := py.OrStr(x.Plan.Get("branch"), "")
	if !pyjson.Truthy(name) || branch == "" {
		return false, ""
	}
	bundle := outPath(x.OutDir, py.Str(name))
	if !isFile(bundle) {
		return false, "the result names a bundle the model job did not upload"
	}
	defaultBranch := x.Cfg.DefaultBranch
	x.fetch(defaultBranch, true)
	x.fetch(branch, false)
	local := "deliver/" + branch
	var head string
	if exc, ok := herr.Catch[*herr.GitError](func() { head = x.Repo.FetchBundle(bundle, branch, local) }); ok {
		return false, "the bundle could not be read: " + exc.Error()
	}
	if head != x.Result.Get("head") {
		return false, "the bundle's head is not the head the result names"
	}
	start := py.OrStr(x.Result.Get("start"), "")
	if start == "" || x.isAncestor(start, head) != 0 {
		return false, "the bundle does not descend from the commit the work started at"
	}
	remote := x.Repo.Rev("origin/" + branch)
	if remote != "" && remote != start {
		return false, fmt.Sprintf("`%s` moved on GitHub while I worked, so I did not overwrite it", branch)
	}
	mergedMain := x.Repo.MergeBase("origin/"+defaultBranch, local)
	forbidden := x.Repo.Unsanctioned(local, []string{start, mergedMain}, x.Cfg.ForbiddenPaths)
	if len(forbidden) > 0 {
		return false, "the change touches paths the bot may not change: " + strings.Join(forbidden, ", ")
	}
	markers := x.Repo.Markers(local, x.Repo.ChangedPaths("origin/"+defaultBranch, local))
	if len(markers) > 0 {
		return false, "conflict markers are still in " + shownPaths(markers) + ", so I pushed nothing"
	}
	if !approved {
		x.holdAutoMerge(0)
	}
	if x.Cfg.DryRun {
		x.log(fmt.Sprintf("dry run: would push %s to %s", head, branch))
		return true, ""
	}
	url := x.remoteURL()
	if exc, ok := herr.Catch[*herr.GitError](func() {
		x.Repo.Push(url, local+":refs/heads/"+branch, x.Cfg.WriteToken(), "")
	}); ok {
		return false, "the push was refused: " + exc.Error()
	}
	x.log(fmt.Sprintf("pushed %s to %s", py.SliceTo(head, 12), branch))
	return true, ""
}
