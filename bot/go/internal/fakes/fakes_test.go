// The fake against fakes.py: TestScenario builds the world testdata/scenario.py builds on the
// Python fake and must dump the same JSON, byte for byte (testdata/scenario.json).
package fakes_test

import (
	"os"
	"sort"
	"strconv"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

func ghErr(fn func()) []any {
	e, ok := herr.Catch[*herr.GitHubError](fn)
	if !ok {
		return nil
	}
	return []any{e.Msg, e.Status}
}

func pyErr(fn func()) *fakes.PyError {
	var got *fakes.PyError
	func() {
		defer func() {
			if r := recover(); r != nil {
				got = r.(*fakes.PyError)
			}
		}()
		fn()
	}()
	return got
}

func objs(xs []*pyjson.Object) []any { return pyjson.ListOf(xs) }

func TestScenario(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	out := pyjson.New()
	gh.AddIssue(7, fakes.Issue{Labels: []string{"bot:build"}})
	gh.AddIssue(3, fakes.Issue{Title: "Title", Body: "Body", User: fakes.Stranger, State: "closed"})
	gh.AddPull(9, "bot/issue-7", fakes.Pull{Labels: []string{"bot:pr"}, SHA: "s1"})
	gh.AddPull(5, "x", fakes.Pull{Fork: true})
	gh.AddIssue(20, fakes.Issue{Labels: []string{"bot:build", "human"}})
	gh.AddComment(7, "hi")
	gh.AddComment(9, "yo", fakes.Comment{User: fakes.Stranger, Association: "NONE", CreatedAt: "2026-09-25T00:00:00Z"})
	gh.AddComment(7, "bot says", fakes.Comment{User: fakes.Bot.Copy()})
	gh.AddReviewComment(9, "line")
	gh.AddLabels(7, []string{"a", "a", "bot:build"})
	gh.AddAssignees(7, []string{"x", "y", "x"})
	gh.RemoveAssignees(7, []string{"X"})
	gh.AddLabels(20, []string{"c"})
	gh.RemoveLabel(7, "a")
	gh.Block(7, 3)
	gh.AddBlockedBy(20, 5000007)
	gh.AddSubIssue(7, 5000003)
	out.Set("ensure", []any{gh.EnsureLabel("human", "ededed", "d"), gh.EnsureLabel("human", "x", "y")})
	out.Set("sync", []any{gh.SyncLabel("human", "EDEDED", "d"), gh.SyncLabel("human", "fff", "e"),
		gh.SyncLabel("new", "1", "2")})
	out.Set("delete_label", []any{gh.DeleteLabel("nope"), gh.DeleteLabel("new")})
	gh.PutFile("state.json", "{}", "main", "", "m")
	gh.ConflictsToInject = 1
	out.Set("conflict", ghErr(func() { gh.PutFile("state.json", "{}", "main", "sha1", "m") }))
	put := []any{}
	for _, sha := range []string{"", "sha1", "sha2"} {
		var shaV any = sha
		if sha == "" {
			shaV = nil
		}
		if e := ghErr(func() { gh.PutFile("state.json", `{"a": 1}`, "main", sha, "m") }); e != nil {
			put = append(put, append(e, shaV))
		} else {
			put = append(put, []any{"ok", shaV})
		}
	}
	out.Set("put", put)
	gh.PutFile("a/b.txt", "x", "main", "", "m")
	gh.PutFile("a/c/d.txt", "x", "main", "", "m")
	gh.CreateOrphanBranch("bot-state", "state.json", "{}", "m", map[string]string{"a": "1", "b": "2"})
	out.Set("no_branch", ghErr(func() { gh.PutFile("x", "x", "nope", "", "m") }))
	gh.Runs.Set("2", pyjson.New("id", 2, "status", "queued", "head_sha", "s1"))
	gh.Runs.Set("1", pyjson.New("id", 1, "status", "queued", "workflow", "x.yml", "head_sha", "s1"))
	gh.Runs.Set("3", pyjson.New("id", 3, "status", "completed", "workflow", "y.yml"))
	gh.Jobs["2"] = []*pyjson.Object{pyjson.New("name", "j", "labels", []any{"a"})}
	gh.EnableAutoMerge("PR_9", "SQUASH", "s1", "T")
	if gh.QueueRemoval("PR_9") == nil {
		out.Set("queue_removal_auto", nil)
	}
	gh.DisableAutoMerge("PR_9")
	gh.QueueRemovals["PR_9"] = map[string]string{"actor": "a", "type": "User", "reason": "MANUAL"}
	qr := gh.QueueRemoval("PR_9")
	out.Set("queue_removal", pyjson.New("actor", qr["actor"], "type", qr["type"], "reason", qr["reason"]))
	gh.MergeQueue["PR_5"] = true
	if gh.QueueRemoval("PR_5") == nil {
		out.Set("queue_removal_queued", nil)
	}
	gh.Dequeue("PR_5")
	out.Set("dequeue", ghErr(func() { gh.Dequeue("PR_5") }))
	out.Set("get_issue_missing", ghErr(func() { gh.GetIssue(99) }))
	out.Set("get_pull_issue", ghErr(func() { gh.GetPull(7) }))
	out.Set("label_names_missing", pyErr(func() { gh.LabelNames(99) }).Msg)
	out.Set("stop", pyErr(func() { gh.MarkReady("PR_99") }).Type)
	gh.React(1000, "eyes", false)
	gh.React(1000, "+1", false)
	gh.React(1001, "rocket", false)
	gh.RequestReviewers(9, []string{"jgoetzmann"})
	gh.DispatchWorkflow("bot-night.yml", "main", map[string]string{"item": "5"})
	gh.RerunFailedJobs(99)
	gh.CreateBranch("b1", "h1")
	out.Set("create_branch", ghErr(func() { gh.CreateBranch("b1", "h1") }))
	gh.DeleteBranch("b1")
	gh.SetProtection("main", []string{"rust", "web"})
	gh.UpdateRepo(pyjson.New("allow_auto_merge", false, "x", 1))
	gh.Lock(3)
	gh.UpdateIssue(3, pyjson.New("title", "New", "extra", pyjson.New("k", []any{1, 2.5})))
	gh.UpdatePull(9, pyjson.New("mergeable_state", "dirty"))
	gh.MergePull(5, "SQUASH", "Merged it")
	gh.PinIssue("a")
	gh.PinIssue("b")
	gh.PinIssue("c")
	out.Set("pin", ghErr(func() { gh.PinIssue("d") }))
	gh.Commits = []*pyjson.Object{pyjson.New("sha", "a", "author", pyjson.New("login", "jgoetzmann-bot")),
		pyjson.New("sha", "b", "author", nil), pyjson.New("sha", "c")}
	gh.Gists = map[string]*pyjson.Object{"g1": pyjson.New("files", pyjson.New("x", pyjson.New("content", "1")))}
	gh.ArtifactZips[1] = []byte("zip")

	labelNames := []string{}
	for n := range gh.LabelNames(7) {
		labelNames = append(labelNames, n)
	}
	sort.Strings(labelNames)
	files := [][]any{}
	for k, v := range gh.Files {
		files = append(files, []any{k[0], k[1], v[0], v[1]})
	}
	sort.Slice(files, func(i, j int) bool {
		for n := 0; n < 4; n++ {
			if files[i][n] != files[j][n] {
				return files[i][n].(string) < files[j][n].(string)
			}
		}
		return false
	})
	threads := pyjson.New()
	order := []any{}
	for n, th := range gh.Threads.All() {
		threads.Set(strconv.Itoa(n), th)
		order = append(order, n)
	}
	reacted := []any{}
	for _, r := range gh.Reacted {
		reacted = append(reacted, []any{r.ID, r.Content})
	}
	rr := []any{}
	for _, r := range gh.ReviewRequests {
		rr = append(rr, []any{r.Number, r.Logins})
	}
	ds := []any{}
	for _, d := range gh.Dispatches {
		ds = append(ds, pyjson.New("workflow", d.Workflow, "ref", d.Ref, "inputs", d.Inputs))
	}
	branches := []string{}
	for b := range gh.Branches {
		branches = append(branches, b)
	}
	sort.Strings(branches)
	file, sha := gh.GetFile("state.json", "main")
	var required any
	if rc := gh.RequiredChecks("main"); rc != nil {
		required = rc
	}
	nope := any(gh.BranchSHA("nope"))
	if nope == "" {
		nope = nil
	}
	out.Update(pyjson.New(
		"get_issue", gh.GetIssue(7),
		"list_issues", objs(gh.ListIssues("bot:build", "open", 300, "")),
		"list_issues_assignee", objs(gh.ListIssues("", "open", 300, "y")),
		"list_all", objs(gh.ListIssues("", "all", 2, "")),
		"list_closed", objs(gh.ListIssues("", "closed", 300, "")),
		"list_neg", objs(gh.ListIssues("", "open", -1, "")),
		"list_pulls", objs(gh.ListPulls("open", "", 200)),
		"list_pulls_all", objs(gh.ListPulls("all", "", 200)),
		"list_pulls_head", objs(gh.ListPulls("open", "bot/issue-7", 200)),
		"label_names", labelNames,
		"bot_comments", gh.BotComments(7),
		"list_comments", objs(gh.ListComments(7, 300)),
		"list_repo_comments", objs(gh.ListRepoComments("2026-09-21", 500)),
		"list_repo_comments_all", objs(gh.ListRepoComments("", 2)),
		"list_repo_review_comments", objs(gh.ListRepoReviewComments("", 500)),
		"list_review_comments", objs(gh.ListReviewComments(9)),
		"list_reviews", objs(gh.ListReviews(9)),
		"list_sub_issues", objs(gh.ListSubIssues(7)),
		"blocked_by", objs(gh.BlockedBy(7)),
		"blocking", objs(gh.Blocking(3)),
		"blocked_20", objs(gh.BlockedBy(20)),
		"labels", objs(gh.ListLabels()),
		"issue_types", objs(gh.ListIssueTypes()),
		"list_dir", gh.ListDir("a", "main"),
		"list_dir_root", gh.ListDir("", "bot-state"),
		"get_file", []any{file, sha},
		"files", files,
		"runs_for_sha", objs(gh.RunsForSHA("s1", 30)),
		"list_runs", objs(gh.ListRuns("y.yml", "", 30, false)),
		"list_runs_x", objs(gh.ListRuns("x.yml", "", 1, false)),
		"list_repo_runs", objs(gh.ListRepoRuns("queued", 100, false)),
		"get_run", gh.GetRun(2),
		"list_jobs", objs(gh.ListJobs(2, false)),
		"job_log", gh.JobLog(17, 6000),
		"reactions", objs(gh.Reactions(1000, false)),
		"get_pull", gh.GetPull(9),
		"threads", threads,
		"thread_order", order,
		"auto_merge", gh.AutoMerge, "auto_merge_heads", gh.AutoMergeHeads,
		"merge_titles", pyjson.New("PR_9", gh.MergeTitles["PR_9"], "PR_5", gh.MergeTitles["PR_5"]),
		"reacted", reacted, "review_requests", rr,
		"dispatches", ds, "reruns", gh.Reruns,
		"created_branches", gh.CreatedBranches, "deleted_branches", gh.DeletedBranches,
		"branches", branches, "protection", gh.Protection,
		"repo_info", gh.RepoInfo(), "viewer", gh.Viewer(), "pinned", gh.Pinned,
		"list_commits", objs(gh.ListCommits("jgoetzmann-bot", 1000)),
		"list_commits_all", objs(gh.ListCommits("", 2)),
		"gist", gh.GetGist("g1", false),
		"artifact", string(gh.DownloadArtifact("1")),
		"branch_sha", []any{gh.BranchSHA("main"), nope},
		"required_checks", required,
		"check_runs", objs(gh.CheckRuns("x")),
	))
	want, err := os.ReadFile("testdata/scenario.json")
	if err != nil {
		t.Fatal(err)
	}
	if got := pyjson.Dumps(out) + "\n"; got != string(want) {
		t.Fatalf("the Go fake's world is not the Python's:\n go: %s\n py: %s", got, want)
	}

	// The patch seam.
	gh.Patch.GetIssue = func(n int) *pyjson.Object { return pyjson.New("pull_request", pyjson.New(), "n", n) }
	if got := gh.GetPull(77).Int("n"); got != 77 {
		t.Fatalf("patched GetIssue not used by GetPull: %d", got)
	}
	if e := ghErr(func() { gh.Own().GetIssue(77) }); e == nil {
		t.Fatal("Own().GetIssue went through the patch")
	}
	gh.Patch.GetIssue = nil
	if pyErr(func() { gh.Request("GET", "x", nil, "", false, false) }).Msg != "FakeGitHub: Request not faked" {
		t.Fatal("not faked")
	}
}

func TestGitHelpers(t *testing.T) {
	root := t.TempDir()
	origin, clone := fakes.MakeOrigin(root)
	sha := fakes.PushBranch(origin, root, "bot/issue-7", map[string]string{"src/game.txt": "rules v2\n"})
	if got := fakes.Git(clone, "ls-remote", "origin", "bot/issue-7"); got != sha+"\trefs/heads/bot/issue-7" {
		t.Fatalf("ls-remote: %q vs %q", got, sha)
	}
	sha2 := fakes.PushBranch(origin, root, "bot/issue-7", map[string]string{"x.txt": "x\n"}, "main")
	if sha2 == sha {
		t.Fatal("same sha")
	}
	e := pyErr(func() { fakes.Git(clone, "checkout", "nope") })
	if e.Type != "AssertionError" || e.Msg[:17] != "git checkout nope" {
		t.Fatalf("%#v", e)
	}
	if pyErr(func() { fakes.MakeOrigin(root) }).Type != "FileExistsError" {
		t.Fatal("second MakeOrigin")
	}
	_, clone2 := fakes.MakeOrigin(t.TempDir(), map[string]string{"a/b.txt": "b\n"})
	if got := fakes.Git(clone2, "ls-files"); got != "a/b.txt" {
		t.Fatalf("ls-files %q", got)
	}
}
