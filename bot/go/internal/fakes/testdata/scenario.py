"""One world built on tests/fakes.py, dumped as JSON: `scenario.json`, which fakes_test.go holds the
Go fake to byte for byte. From `bot/`: `python3 -I go/internal/fakes/testdata/scenario.py . >
go/internal/fakes/testdata/scenario.json`."""
import json, sys
sys.path.insert(0, sys.argv[1])
from harness.errors import GitHubError
from tests.fakes import FakeGitHub, STRANGER, BOT

gh = FakeGitHub()
out = {}
gh.add_issue(7, labels=("bot:build",))
gh.add_issue(3, "Title", "Body", user=STRANGER, state="closed")
gh.add_pull(9, "bot/issue-7", labels=("bot:pr",), sha="s1")
gh.add_pull(5, "x", fork=True)
gh.add_issue(20, labels=("bot:build", "human"))
gh.add_comment(7, "hi")
gh.add_comment(9, "yo", user=STRANGER, association="NONE", created_at="2026-09-25T00:00:00Z")
gh.add_comment(7, "bot says", user=dict(BOT))
gh.add_review_comment(9, "line")
gh.add_labels(7, ["a", "a", "bot:build"])
gh.add_assignees(7, ["x", "y", "x"])
gh.remove_assignees(7, ["X"])
gh.add_labels(20, ["c"])
gh.remove_label(7, "a")
gh.block(7, 3)
gh.add_blocked_by(20, 5000007)
gh.add_sub_issue(7, 5000003)
out["ensure"] = [gh.ensure_label("human", "ededed", "d"), gh.ensure_label("human", "x", "y")]
out["sync"] = [gh.sync_label("human", "EDEDED", "d"), gh.sync_label("human", "fff", "e"),
               gh.sync_label("new", "1", "2")]
out["delete_label"] = [gh.delete_label("nope"), gh.delete_label("new")]
gh.put_file("state.json", "{}", branch="main", sha=None, message="m")
gh.conflicts_to_inject = 1
try:
    gh.put_file("state.json", "{}", branch="main", sha="sha1", message="m")
except GitHubError as e:
    out["conflict"] = [str(e), e.status]
for sha in (None, "sha1", "sha2"):
    try:
        gh.put_file("state.json", '{"a": 1}', branch="main", sha=sha, message="m")
        out.setdefault("put", []).append(["ok", sha])
    except GitHubError as e:
        out.setdefault("put", []).append([str(e), e.status, sha])
gh.put_file("a/b.txt", "x", branch="main", sha=None, message="m")
gh.put_file("a/c/d.txt", "x", branch="main", sha=None, message="m")
gh.create_orphan_branch("bot-state", "state.json", "{}", "m", {"a": "1", "b": "2"})
try:
    gh.put_file("x", "x", branch="nope", sha=None, message="m")
except GitHubError as e:
    out["no_branch"] = [str(e), e.status]
gh.runs["2"] = {"id": 2, "status": "queued", "head_sha": "s1"}
gh.runs["1"] = {"id": 1, "status": "queued", "workflow": "x.yml", "head_sha": "s1"}
gh.runs["3"] = {"id": 3, "status": "completed", "workflow": "y.yml"}
gh.jobs["2"] = [{"name": "j", "labels": ["a"]}]
gh.enable_auto_merge("PR_9", "SQUASH", "s1", "T")
out["queue_removal_auto"] = gh.queue_removal("PR_9")
gh.disable_auto_merge("PR_9")
gh.queue_removals["PR_9"] = {"actor": "a", "type": "User", "reason": "MANUAL"}
out["queue_removal"] = gh.queue_removal("PR_9")
gh.merge_queue.add("PR_5")
out["queue_removal_queued"] = gh.queue_removal("PR_5")
gh.dequeue("PR_5")
try:
    gh.dequeue("PR_5")
except GitHubError as e:
    out["dequeue"] = [str(e), e.status]
try:
    gh.get_issue(99)
except GitHubError as e:
    out["get_issue_missing"] = [str(e), e.status]
try:
    gh.get_pull(7)
except GitHubError as e:
    out["get_pull_issue"] = [str(e), e.status]
try:
    gh.label_names(99)
except KeyError as e:
    out["label_names_missing"] = str(e)
try:
    gh.mark_ready("PR_99")
except StopIteration as e:
    out["stop"] = "StopIteration"
gh.react(1000, "eyes")
gh.react(1000, "+1")
gh.react(1001, "rocket")
gh.request_reviewers(9, ["jgoetzmann"])
gh.dispatch_workflow("bot-night.yml", "main", {"item": "5"})
gh.rerun_failed_jobs(99)
gh.create_branch("b1", "h1")
try:
    gh.create_branch("b1", "h1")
except GitHubError as e:
    out["create_branch"] = [str(e), e.status]
gh.delete_branch("b1")
gh.set_protection("main", ["rust", "web"])
gh.update_repo(allow_auto_merge=False, x=1)
gh.lock(3)
gh.update_issue(3, title="New", extra={"k": [1, 2.5]})
gh.update_pull(9, mergeable_state="dirty")
gh.merge_pull(5, "SQUASH", "Merged it")
gh.pin_issue("a")
gh.pin_issue("b")
gh.pin_issue("c")
try:
    gh.pin_issue("d")
except GitHubError as e:
    out["pin"] = [str(e), e.status]
gh.commits = [{"sha": "a", "author": {"login": "jgoetzmann-bot"}}, {"sha": "b", "author": None},
              {"sha": "c"}]
gh.gists = {"g1": {"files": {"x": {"content": "1"}}}}
gh.artifact_zips[1] = b"zip"
out.update({
    "get_issue": gh.get_issue(7),
    "list_issues": gh.list_issues(labels="bot:build"),
    "list_issues_assignee": gh.list_issues(assignee="y"),
    "list_all": gh.list_issues(state="all", limit=2),
    "list_closed": gh.list_issues(state="closed"),
    "list_neg": gh.list_issues(limit=-1),
    "list_pulls": gh.list_pulls(),
    "list_pulls_all": gh.list_pulls(state="all"),
    "list_pulls_head": gh.list_pulls(head="bot/issue-7"),
    "label_names": sorted(gh.label_names(7)),
    "bot_comments": gh.bot_comments(7),
    "list_comments": gh.list_comments(7),
    "list_repo_comments": gh.list_repo_comments("2026-09-21"),
    "list_repo_comments_all": gh.list_repo_comments("", limit=2),
    "list_repo_review_comments": gh.list_repo_review_comments(""),
    "list_review_comments": gh.list_review_comments(9),
    "list_reviews": gh.list_reviews(9),
    "list_sub_issues": gh.list_sub_issues(7),
    "blocked_by": gh.blocked_by(7),
    "blocking": gh.blocking(3),
    "blocked_20": gh.blocked_by(20),
    "labels": gh.list_labels(),
    "issue_types": gh.list_issue_types(),
    "list_dir": gh.list_dir("a", "main"),
    "list_dir_root": gh.list_dir("", "bot-state"),
    "get_file": list(gh.get_file("state.json", "main")),
    "files": sorted([list(k) + list(v) for k, v in gh.files.items()]),
    "runs_for_sha": gh.runs_for_sha("s1"),
    "list_runs": gh.list_runs("y.yml"),
    "list_runs_x": gh.list_runs("x.yml", limit=1),
    "list_repo_runs": gh.list_repo_runs(status="queued"),
    "get_run": gh.get_run(2),
    "list_jobs": gh.list_jobs(2),
    "job_log": gh.job_log(17),
    "reactions": gh.reactions(1000),
    "get_pull": gh.get_pull(9),
    "threads": {str(k): v for k, v in gh.threads.items()},
    "thread_order": list(gh.threads),
    "auto_merge": gh.auto_merge, "auto_merge_heads": gh.auto_merge_heads,
    "merge_titles": gh.merge_titles,
    "reacted": gh.reacted, "review_requests": gh.review_requests,
    "dispatches": gh.dispatches, "reruns": gh.reruns,
    "created_branches": gh.created_branches, "deleted_branches": gh.deleted_branches,
    "branches": sorted(gh.branches), "protection": gh.protection,
    "repo_info": gh.repo_info(), "viewer": gh.viewer(), "pinned": gh.pinned,
    "list_commits": gh.list_commits(author="jgoetzmann-bot"),
    "list_commits_all": gh.list_commits(limit=2),
    "gist": gh.get_gist("g1"),
    "artifact": gh.download_artifact("1").decode(),
    "branch_sha": [gh.branch_sha("main"), gh.branch_sha("nope")],
    "required_checks": gh.required_checks("main"),
    "check_runs": gh.check_runs("x"),
})
print(json.dumps(out))
