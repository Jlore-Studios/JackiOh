package gh

import (
	"encoding/base64"
	"errors"
	"fmt"
	"net/http"
	"sort"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
)

// ------------------------------------------------------------------ repository

func (x *GitHub) RepoInfo() *pyjson.Object {
	return obj(x.Request("GET", x.r(), nil, "", false, false))
}

func (x *GitHub) Viewer() *pyjson.Object {
	return obj(x.Request("GET", "/user", nil, "", false, false))
}

// ------------------------------------------------------------------ issues and comments

func (x *GitHub) GetIssue(number int) *pyjson.Object {
	return obj(x.Request("GET", fmt.Sprintf("%s/issues/%d", x.r(), number), nil, "", false, false))
}

func (x *GitHub) ListIssues(labels string /* = "" */, state string /* = "open" */, limit int /* = 300 */, assignee string /* = "" */) []*pyjson.Object {
	params := pyjson.New("state", state)
	if labels != "" {
		params.Set("labels", labels)
	}
	if assignee != "" {
		params.Set("assignee", assignee)
	}
	return objs(x.Paginate(x.r()+"/issues", params, "", limit, false))
}

// ListRepoComments is every issue and pull request conversation comment updated since `since`,
// newest first, so a limit drops the oldest.
func (x *GitHub) ListRepoComments(since string, limit int /* = 500 */) []*pyjson.Object {
	return objs(x.Paginate(x.r()+"/issues/comments",
		pyjson.New("since", since, "sort", "created", "direction", "desc"), "", limit, false))
}

// ListRepoReviewComments is every line comment on a pull request's diff updated since `since`,
// newest first.
func (x *GitHub) ListRepoReviewComments(since string, limit int /* = 500 */) []*pyjson.Object {
	return objs(x.Paginate(x.r()+"/pulls/comments",
		pyjson.New("since", since, "sort", "created", "direction", "desc"), "", limit, false))
}

func (x *GitHub) Reactions(commentID int, reviewComment bool /* = False */) []*pyjson.Object {
	kind := "issues/comments"
	if reviewComment {
		kind = "pulls/comments"
	}
	return objs(x.Paginate(fmt.Sprintf("%s/%s/%d/reactions", x.r(), kind, commentID), nil, "", 1000, false))
}

func (x *GitHub) ListComments(number int, limit int /* = 300 */) []*pyjson.Object {
	return objs(x.Paginate(fmt.Sprintf("%s/issues/%d/comments", x.r(), number), nil, "", limit, false))
}

func (x *GitHub) CreateComment(number int, body string) *pyjson.Object {
	return obj(x.Request("POST", fmt.Sprintf("%s/issues/%d/comments", x.r(), number),
		pyjson.New("body", WithMarker(body)), "", false, false))
}

func (x *GitHub) UpdateComment(commentID int, body string) *pyjson.Object {
	return obj(x.Request("PATCH", fmt.Sprintf("%s/issues/comments/%d", x.r(), commentID),
		pyjson.New("body", WithMarker(body)), "", false, false))
}

func (x *GitHub) React(commentID int, content string, reviewComment bool /* = False */) {
	kind := "issues/comments"
	if reviewComment {
		kind = "pulls/comments"
	}
	// A reaction is a courtesy; its failure changes nothing.
	herr.Catch[*herr.GitHubError](func() {
		x.Request("POST", fmt.Sprintf("%s/%s/%d/reactions", x.r(), kind, commentID),
			pyjson.New("content", content), "", false, false)
	})
}

func (x *GitHub) CreateIssue(title string, body string, labels []string /* = () */) *pyjson.Object {
	payload := pyjson.New("title", py.SliceTo(redact.Redact(title, nil), 250), "body", WithMarker(body),
		"labels", pyjson.ListOf(labels))
	return obj(x.Request("POST", x.r()+"/issues", payload, "", false, false))
}

// SetIssueBody replaces an issue's description as it is: no redaction of a person's text and no
// bot marker, which would make the whole description read as the bot's (the plan section,
// `issueplan.py`, is redacted before it gets here).
func (x *GitHub) SetIssueBody(number int, body string) *pyjson.Object {
	return obj(x.Request("PATCH", fmt.Sprintf("%s/issues/%d", x.r(), number), pyjson.New("body", body),
		"", false, false))
}

// BlockedBy is the issues GitHub's own issue dependencies say block this one, open or closed.
func (x *GitHub) BlockedBy(number int) []*pyjson.Object {
	return objs(x.Paginate(fmt.Sprintf("%s/issues/%d/dependencies/blocked_by", x.r(), number), nil, "", 100, false))
}

// Blocking is the issues GitHub's own issue dependencies say this one blocks, open or closed.
func (x *GitHub) Blocking(number int) []*pyjson.Object {
	return objs(x.Paginate(fmt.Sprintf("%s/issues/%d/dependencies/blocking", x.r(), number), nil, "", 100, false))
}

// AddBlockedBy marks issue `number` blocked by the issue whose id (not number) is `blockerID`.
func (x *GitHub) AddBlockedBy(number int, blockerID int) {
	x.Request("POST", fmt.Sprintf("%s/issues/%d/dependencies/blocked_by", x.r(), number),
		pyjson.New("issue_id", blockerID), "", false, false)
}

// AddSubIssue makes the issue whose id (not number) is `childID` a sub-issue of issue `parent`.
func (x *GitHub) AddSubIssue(parent int, childID int) {
	x.Request("POST", fmt.Sprintf("%s/issues/%d/sub_issues", x.r(), parent),
		pyjson.New("sub_issue_id", childID), "", false, false)
}

// ListSubIssues is the sub-issues of issue `parent`, open and closed (#60: a tree the bot split).
func (x *GitHub) ListSubIssues(parent int) []*pyjson.Object {
	return objs(x.Paginate(fmt.Sprintf("%s/issues/%d/sub_issues", x.r(), parent), nil, "", 100, false))
}

// UpdateIssue is `update_issue(number, **fields)`: fields is not changed (Python's kwargs are a
// new dict).
func (x *GitHub) UpdateIssue(number int, fields *pyjson.Object) *pyjson.Object {
	fields = fields.Copy()
	if body, ok := fields.Lookup("body"); ok {
		fields.Set("body", WithMarker(py.OrStr(body, "")))
	}
	return obj(x.Request("PATCH", fmt.Sprintf("%s/issues/%d", x.r(), number), fields, "", false, false))
}

func (x *GitHub) AddLabels(number int, names []string) {
	names = nonEmpty(names)
	if len(names) > 0 {
		x.Request("POST", fmt.Sprintf("%s/issues/%d/labels", x.r(), number),
			pyjson.New("labels", pyjson.ListOf(names)), "", false, false)
	}
}

// AddAssignees adds to whoever is assigned; GitHub skips an account it cannot assign.
func (x *GitHub) AddAssignees(number int, logins []string) {
	logins = nonEmpty(logins)
	if len(logins) > 0 {
		x.Request("POST", fmt.Sprintf("%s/issues/%d/assignees", x.r(), number),
			pyjson.New("assignees", pyjson.ListOf(logins)), "", false, false)
	}
}

func (x *GitHub) RemoveAssignees(number int, logins []string) {
	found := nonEmpty(logins)
	if len(found) > 0 {
		x.Request("DELETE", fmt.Sprintf("%s/issues/%d/assignees", x.r(), number),
			pyjson.New("assignees", pyjson.ListOf(found)), "", false, false)
	}
}

// nonEmpty is `[n for n in names if n]`.
func nonEmpty(names []string) []string {
	out := []string{}
	for _, n := range names {
		if n != "" {
			out = append(out, n)
		}
	}
	return out
}

func (x *GitHub) RemoveLabel(number int, name string) {
	quoted := quote(name, "")
	if exc, ok := herr.Catch[*herr.GitHubError](func() {
		x.Request("DELETE", fmt.Sprintf("%s/issues/%d/labels/%s", x.r(), number, quoted), nil, "", false, false)
	}); ok && exc.Status != 404 {
		herr.Raise(exc)
	}
}

// Lock locks an issue's or pull request's conversation: only collaborators may comment
// (`.github/workflows/lock-conversations.yml`). The empty body sends a Content-Length.
func (x *GitHub) Lock(number int) {
	x.Request("PUT", fmt.Sprintf("%s/issues/%d/lock", x.r(), number), pyjson.New(), "", false, false)
}

func (x *GitHub) Unlock(number int) {
	x.Request("DELETE", fmt.Sprintf("%s/issues/%d/lock", x.r(), number), nil, "", false, false)
}

func (x *GitHub) ListLabels() []*pyjson.Object {
	return objs(x.Paginate(x.r()+"/labels", nil, "", 1000, false))
}

// ListIssueTypes is the owning organisation's issue types (none for a user's repository).
func (x *GitHub) ListIssueTypes() []*pyjson.Object {
	owner, _, _ := strings.Cut(x.Repo, "/")
	var found []*pyjson.Object
	if exc, ok := herr.Catch[*herr.GitHubError](func() {
		found = objs(pyjson.List(x.Request("GET", "/orgs/"+owner+"/issue-types", nil, "", false, false)))
	}); ok {
		if exc.Status == 403 || exc.Status == 404 {
			return []*pyjson.Object{}
		}
		herr.Raise(exc)
	}
	return found
}

// EnsureLabel creates the label unless it exists. True when it was created.
func (x *GitHub) EnsureLabel(name string, color string, description string) bool {
	quoted := quote(name, "")
	exc, missing := herr.Catch[*herr.GitHubError](func() {
		x.Request("GET", x.r()+"/labels/"+quoted, nil, "", false, false)
	})
	if !missing {
		return false
	}
	if exc.Status != 404 {
		herr.Raise(exc)
	}
	x.Request("POST", x.r()+"/labels", pyjson.New("name", name, "color", color, "description", description),
		"", false, false)
	return true
}

// SyncLabel creates the label, or brings its colour and description to these (`harness setup`,
// #187): "created", "updated", or "" when it matched already.
func (x *GitHub) SyncLabel(name string, color string, description string) string {
	quoted := quote(name, "")
	var found any
	if exc, ok := herr.Catch[*herr.GitHubError](func() {
		found = x.Request("GET", x.r()+"/labels/"+quoted, nil, "", false, false)
	}); ok {
		if exc.Status != 404 {
			herr.Raise(exc)
		}
		x.Request("POST", x.r()+"/labels", pyjson.New("name", name, "color", color, "description", description),
			"", false, false)
		return "created"
	}
	label := obj(found)
	if strings.ToLower(py.OrStr(label.Get("color"), "")) == strings.ToLower(color) &&
		py.OrStr(label.Get("description"), "") == description {
		return ""
	}
	x.Request("PATCH", x.r()+"/labels/"+quoted, pyjson.New("color", color, "description", description),
		"", false, false)
	return "updated"
}

// DeleteLabel deletes the label from the repository, and so from every thread. True when it was
// there.
func (x *GitHub) DeleteLabel(name string) bool {
	if exc, ok := herr.Catch[*herr.GitHubError](func() {
		x.Request("DELETE", x.r()+"/labels/"+quote(name, ""), nil, "", false, false)
	}); ok {
		if exc.Status != 404 {
			herr.Raise(exc)
		}
		return false
	}
	return true
}

// ------------------------------------------------------------------ pull requests

func (x *GitHub) GetPull(number int) *pyjson.Object {
	return obj(x.Request("GET", fmt.Sprintf("%s/pulls/%d", x.r(), number), nil, "", false, false))
}

func (x *GitHub) ListPulls(state string /* = "open" */, head string /* = "" */, limit int /* = 200 */) []*pyjson.Object {
	params := pyjson.New("state", state)
	if head != "" {
		owner, _, _ := strings.Cut(x.Repo, "/")
		params.Set("head", owner+":"+head)
	}
	return objs(x.Paginate(x.r()+"/pulls", params, "", limit, false))
}

func (x *GitHub) CreatePull(title string, head string, base string, body string, draft bool /* = False */) *pyjson.Object {
	payload := pyjson.New(
		"title", py.SliceTo(redact.Redact(title, nil), 250),
		"head", head,
		"base", base,
		"body", WithMarker(body),
		"draft", draft,
		"maintainer_can_modify", true,
	)
	return obj(x.Request("POST", x.r()+"/pulls", payload, "", false, false))
}

// UpdatePull is `update_pull(number, **fields)`: fields is not changed (Python's kwargs are a new
// dict).
func (x *GitHub) UpdatePull(number int, fields *pyjson.Object) *pyjson.Object {
	fields = fields.Copy()
	if body, ok := fields.Lookup("body"); ok {
		fields.Set("body", WithMarker(py.OrStr(body, "")))
	}
	if title, ok := fields.Lookup("title"); ok {
		fields.Set("title", py.SliceTo(redact.Redact(py.Str(title), nil), 250))
	}
	return obj(x.Request("PATCH", fmt.Sprintf("%s/pulls/%d", x.r(), number), fields, "", false, false))
}

func (x *GitHub) ListReviews(number int) []*pyjson.Object {
	return objs(x.Paginate(fmt.Sprintf("%s/pulls/%d/reviews", x.r(), number), nil, "", 1000, false))
}

func (x *GitHub) ListReviewComments(number int) []*pyjson.Object {
	return objs(x.Paginate(fmt.Sprintf("%s/pulls/%d/comments", x.r(), number), nil, "", 1000, false))
}

func (x *GitHub) RequestReviewers(number int, logins []string) {
	x.Request("POST", fmt.Sprintf("%s/pulls/%d/requested_reviewers", x.r(), number),
		pyjson.New("reviewers", pyjson.ListOf(logins)), "", false, false)
}

// RequiredChecks is the status checks branch protection requires on `branch`, or nil when
// unprotected (a protected branch with none is an empty, non-nil set).
//
// Read through the branch endpoint, which any reader may call, rather than the protection
// endpoint, which needs admin rights the bot account does not have.
func (x *GitHub) RequiredChecks(branch string) map[string]bool {
	data, ok := x.Request("GET", x.r()+"/branches/"+quote(branch, "/"), nil, "", false, false).(*pyjson.Object)
	if !ok || !data.Truthy("protected") {
		return nil
	}
	checks := data.Obj("protection").Obj("required_status_checks")
	found := map[string]bool{}
	for _, c := range py.Each(checks.Get("contexts")) {
		found[py.Str(c)] = true
	}
	for _, c := range py.Each(checks.Get("checks")) {
		if o, ok := c.(*pyjson.Object); ok {
			found[py.Str(o.Get("context"))] = true
		}
	}
	return found
}

// EnableAutoMerge turns auto-merge on, pinned to `expectedHead` when given: GitHub then refuses
// to merge a head that moved after the approval it was turned on for. `headline` is the merge
// commit's title; without it GitHub squashes a one-commit pull request under that commit's
// message.
func (x *GitHub) EnableAutoMerge(nodeID string, method string, expectedHead string /* = "" */, headline string /* = "" */) {
	params := []string{"$id: ID!", "$m: PullRequestMergeMethod!"}
	fields := []string{"pullRequestId: $id", "mergeMethod: $m"}
	values := pyjson.New("id", nodeID, "m", strings.ToUpper(method))
	if expectedHead != "" {
		params = append(params, "$h: GitObjectID!")
		fields = append(fields, "expectedHeadOid: $h")
		values.Set("h", expectedHead)
	}
	if headline != "" {
		params = append(params, "$t: String!")
		fields = append(fields, "commitHeadline: $t")
		values.Set("t", headline)
	}
	x.Graphql(
		"mutation("+strings.Join(params, ", ")+") {"+
			" enablePullRequestAutoMerge(input: {"+strings.Join(fields, ", ")+"}) { clientMutationId } }",
		values,
	)
}

func (x *GitHub) DisableAutoMerge(nodeID string) {
	x.Graphql(
		"mutation($id: ID!) { disablePullRequestAutoMerge(input: {pullRequestId: $id})"+
			" { clientMutationId } }",
		pyjson.New("id", nodeID),
	)
}

// InMergeQueue is whether a pull request is in `main`'s merge queue now. REST has no field for
// it, and its `auto_merge` reads null for a queued pull request.
func (x *GitHub) InMergeQueue(nodeID string) bool {
	data := x.Graphql(
		"query($id: ID!) { node(id: $id) { ... on PullRequest { isInMergeQueue } } }",
		pyjson.New("id", nodeID),
	)
	return data.Obj("node").Truthy("isInMergeQueue")
}

// QueueRemoval is how the merge queue took a pull request out, when that is the last thing
// auto-merge or the queue did to it (`{"actor", "type", "reason"}`: the remover's login and
// GraphQL type, `User` or `Bot`), else nil: still queued, never queued, or auto-merge turned on
// or off since.
func (x *GitHub) QueueRemoval(nodeID string) map[string]string {
	data := x.Graphql(
		"query($id: ID!) { node(id: $id) { ... on PullRequest { isInMergeQueue"+
			" timelineItems(last: 1, itemTypes: [ADDED_TO_MERGE_QUEUE_EVENT,"+
			" REMOVED_FROM_MERGE_QUEUE_EVENT, AUTO_MERGE_ENABLED_EVENT, AUTO_MERGE_DISABLED_EVENT])"+
			" { nodes { __typename ... on RemovedFromMergeQueueEvent { reason"+
			" actor { __typename login } } } } } } }",
		pyjson.New("id", nodeID),
	)
	node := data.Obj("node")
	nodes := node.Obj("timelineItems").Get("nodes")
	if !pyjson.Truthy(nodes) {
		nodes = []any{nil}
	}
	var last *pyjson.Object
	if list := pyjson.List(nodes); len(list) > 0 {
		last = pyjson.Obj(list[len(list)-1])
	}
	if node.Truthy("isInMergeQueue") || last.Get("__typename") != "RemovedFromMergeQueueEvent" {
		return nil
	}
	actor := last.Obj("actor")
	return map[string]string{
		"actor":  py.OrStr(actor.Get("login"), ""),
		"type":   py.OrStr(actor.Get("__typename"), ""),
		"reason": py.OrStr(last.Get("reason"), ""),
	}
}

// Dequeue takes a pull request out of `main`'s merge queue (infra/github.tf). GitHub answers
// with an error when it is not in the queue.
func (x *GitHub) Dequeue(nodeID string) {
	x.Graphql(
		"mutation($id: ID!) { dequeuePullRequest(input: {id: $id}) { clientMutationId } }",
		pyjson.New("id", nodeID),
	)
}

// PinIssue pins an issue to the top of the issue list (at most three are pinned).
func (x *GitHub) PinIssue(nodeID string) {
	x.Graphql(
		"mutation($id: ID!) { pinIssue(input: {issueId: $id}) { issue { number } } }",
		pyjson.New("id", nodeID),
	)
}

func (x *GitHub) MarkReady(nodeID string) {
	x.Graphql(
		"mutation($id: ID!) { markPullRequestReadyForReview(input: {pullRequestId: $id})"+
			" { clientMutationId } }",
		pyjson.New("id", nodeID),
	)
}

func (x *GitHub) MergePull(number int, method string, title string /* = "" */) *pyjson.Object {
	body := pyjson.New("merge_method", method)
	if title != "" {
		body.Set("commit_title", title)
	}
	return obj(x.Request("PUT", fmt.Sprintf("%s/pulls/%d/merge", x.r(), number), body, "", false, false))
}

// ------------------------------------------------------------------ contents and refs

// GetFile is `(text, blob sha)` of a file on `ref`, or `("", "")` when it does not exist
// (Python's `(None, None)`). A file that exists always has its sha, so a caller tells a missing
// file from an empty one by the sha.
func (x *GitHub) GetFile(path string, ref string) (string, string) {
	quoted := quote(path, "/")
	var answer any
	if exc, ok := herr.Catch[*herr.GitHubError](func() {
		answer = x.Request("GET", x.r()+"/contents/"+quoted+"?ref="+quote(ref, "/"), nil, "", false, false)
	}); ok {
		if exc.Status == 404 {
			return "", ""
		}
		herr.Raise(exc)
	}
	data, isDict := answer.(*pyjson.Object)
	if !isDict || data.Get("type") != "file" {
		return "", ""
	}
	content := py.OrStr(data.Get("content"), "")
	if content == "" && data.Truthy("size") && data.Truthy("sha") {
		// Over 1 MB the contents API leaves `content` empty (`encoding: "none"`); the blob API
		// serves up to 100 MB. Read as empty, state.json was wiped on 2026-10-25
		// (reviews/2026-10-27-muse-usage.md, finding 5).
		blob := x.Request("GET", x.r()+"/git/blobs/"+py.Str(data.Get("sha")), nil, "", false, false)
		content = ""
		if b, ok := blob.(*pyjson.Object); ok {
			content = py.OrStr(b.Get("content"), "")
		}
	}
	raw, err := b64decode(content)
	if err != nil {
		herr.Raise(&ValueError{Msg: err.Error()})
	}
	sha := ""
	if v := data.Get("sha"); v != nil {
		sha = py.Str(v)
	}
	return py.DecodeReplace(raw), sha
}

// b64decode is base64.b64decode(text): the characters outside the alphabet (GitHub's line
// breaks) dropped, then decoded with its padding.
func b64decode(text string) ([]byte, error) {
	var clean strings.Builder
	for _, r := range text {
		if r >= 'A' && r <= 'Z' || r >= 'a' && r <= 'z' || r >= '0' && r <= '9' || r == '+' || r == '/' || r == '=' {
			clean.WriteRune(r)
		}
	}
	data, err := base64.StdEncoding.DecodeString(clean.String())
	if err != nil {
		return nil, errors.New("Incorrect padding")
	}
	return data, nil
}

// PutFile creates or replaces a file. A stale `sha` raises GitHubError with status 409 or 422.
// `sha` "" is None: a new file.
func (x *GitHub) PutFile(path string, text string, branch string, sha string, message string) {
	payload := pyjson.New(
		"message", message,
		"content", base64.StdEncoding.EncodeToString([]byte(text)),
		"branch", branch,
	)
	if sha != "" {
		payload.Set("sha", sha)
	}
	x.Request("PUT", x.r()+"/contents/"+quote(path, "/"), payload, "", false, false)
}

// ListDir is `(name, blob sha)` of each file in a directory on `ref`; empty when there is none.
func (x *GitHub) ListDir(path string, ref string) [][2]string {
	quoted := quote(path, "/")
	var answer any
	if exc, ok := herr.Catch[*herr.GitHubError](func() {
		answer = x.Request("GET", x.r()+"/contents/"+quoted+"?ref="+quote(ref, "/"), nil, "", false, false)
	}); ok {
		if exc.Status == 404 {
			return [][2]string{}
		}
		herr.Raise(exc)
	}
	list, ok := answer.([]any)
	if !ok {
		return [][2]string{}
	}
	out := [][2]string{}
	for _, item := range list {
		if entry, ok := item.(*pyjson.Object); ok && entry.Get("type") == "file" {
			out = append(out, [2]string{py.Str(entry.Get("name")), py.Str(entry.Get("sha"))})
		}
	}
	return out
}

func (x *GitHub) DeleteFile(path string, branch string, sha string, message string) {
	x.Request("DELETE", x.r()+"/contents/"+quote(path, "/"),
		pyjson.New("message", message, "sha", sha, "branch", branch), "", false, false)
}

// BranchSHA is the commit a branch points at, or "" (Python's None) when there is no such branch.
func (x *GitHub) BranchSHA(branch string) string {
	var answer any
	if exc, ok := herr.Catch[*herr.GitHubError](func() {
		answer = x.Request("GET", x.r()+"/git/ref/heads/"+quote(branch, "/"), nil, "", false, false)
	}); ok {
		if exc.Status == 404 {
			return ""
		}
		herr.Raise(exc)
	}
	data, ok := answer.(*pyjson.Object)
	if !ok {
		return ""
	}
	sha := data.Obj("object").Get("sha")
	if sha == nil {
		return ""
	}
	return py.Str(sha)
}

// CreateOrphanBranch makes a new branch whose only commit holds one file, and any `extra` ones.
// A Go map has no order: the extra files go in their names' order, then `path` (Python's
// `{**extra, path: text}`; GitHub sorts a tree's entries itself).
func (x *GitHub) CreateOrphanBranch(branch string, path string, text string, message string, extra map[string]string /* = None */) {
	names := make([]string, 0, len(extra)+1)
	for name := range extra {
		names = append(names, name)
	}
	sort.Strings(names)
	files := map[string]string{}
	for name, body := range extra {
		files[name] = body
	}
	if _, ok := files[path]; !ok {
		names = append(names, path)
	}
	files[path] = text
	entries := make([]any, 0, len(names))
	for _, name := range names {
		entries = append(entries, pyjson.New("path", name, "mode", "100644", "type", "blob", "content", files[name]))
	}
	tree := obj(x.Request("POST", x.r()+"/git/trees", pyjson.New("tree", entries), "", false, false))
	commit := obj(x.Request("POST", x.r()+"/git/commits",
		pyjson.New("message", message, "tree", tree.GetOr("sha", ""), "parents", []any{}), "", false, false))
	x.Request("POST", x.r()+"/git/refs",
		pyjson.New("ref", "refs/heads/"+branch, "sha", commit.GetOr("sha", "")), "", false, false)
}

// CreateBranch makes a branch at `sha` (an existing commit).
func (x *GitHub) CreateBranch(branch string, sha string) {
	x.Request("POST", x.r()+"/git/refs", pyjson.New("ref", "refs/heads/"+branch, "sha", sha), "", false, false)
}

func (x *GitHub) DeleteBranch(branch string) {
	if exc, ok := herr.Catch[*herr.GitHubError](func() {
		x.Request("DELETE", x.r()+"/git/refs/heads/"+quote(branch, "/"), nil, "", false, false)
	}); ok && exc.Status != 404 && exc.Status != 422 {
		herr.Raise(exc)
	}
}

// ------------------------------------------------------------------ actions and checks

// DispatchWorkflow starts a workflow_dispatch run. A Go map has no order: the inputs go in their
// names' order (the Python's are the caller's; GitHub reads them by name).
func (x *GitHub) DispatchWorkflow(workflow string, ref string, inputs map[string]string) {
	names := make([]string, 0, len(inputs))
	for name := range inputs {
		names = append(names, name)
	}
	sort.Strings(names)
	values := pyjson.New()
	for _, name := range names {
		values.Set(name, inputs[name])
	}
	x.Request("POST", x.r()+"/actions/workflows/"+workflow+"/dispatches",
		pyjson.New("ref", ref, "inputs", values), "", false, false)
}

func (x *GitHub) GetRun(runID any) *pyjson.Object {
	return obj(x.Request("GET", x.r()+"/actions/runs/"+py.Str(runID), nil, "", false, false))
}

func (x *GitHub) ListRuns(workflow string, status string /* = "" */, limit int /* = 30 */, cached bool /* = False */) []*pyjson.Object {
	params := pyjson.New("per_page", min(limit, 100))
	if status != "" {
		params.Set("status", status)
	}
	return objs(x.Paginate(x.r()+"/actions/workflows/"+workflow+"/runs", params, "workflow_runs", limit, cached))
}

// GetGist is a gist, by id (the owner's PC's heartbeat, `dashboard.PC_FILE`).
func (x *GitHub) GetGist(gistID string, cached bool /* = False */) *pyjson.Object {
	return obj(x.Request("GET", "/gists/"+quote(gistID, "/"), nil, "", false, cached))
}

// ListRepoRuns is the runs of every workflow with `status` (`in_progress`, `queued`), newest
// first.
func (x *GitHub) ListRepoRuns(status string, limit int /* = 100 */, cached bool /* = False */) []*pyjson.Object {
	return objs(x.Paginate(x.r()+"/actions/runs", pyjson.New("status", status), "workflow_runs", limit, cached))
}

// ListCommits is the commits on the default branch, newest first, by `author` when given.
func (x *GitHub) ListCommits(author string /* = "" */, limit int /* = 1000 */) []*pyjson.Object {
	params := pyjson.New()
	if author != "" {
		params.Set("author", author)
	}
	return objs(x.Paginate(x.r()+"/commits", params, "", limit, false))
}

func (x *GitHub) RunsForSHA(sha string, limit int /* = 30 */) []*pyjson.Object {
	return objs(x.Paginate(x.r()+"/actions/runs", pyjson.New("head_sha", sha), "workflow_runs", limit, false))
}

func (x *GitHub) RerunFailedJobs(runID any) {
	x.Request("POST", x.r()+"/actions/runs/"+py.Str(runID)+"/rerun-failed-jobs", nil, "", false, false)
}

func (x *GitHub) ListJobs(runID any, cached bool /* = False */) []*pyjson.Object {
	return objs(x.Paginate(x.r()+"/actions/runs/"+py.Str(runID)+"/jobs", nil, "jobs", 1000, cached))
}

// JobLog is the last `tailChars` of a job's log. The redirect is followed without the token.
func (x *GitHub) JobLog(jobID any, tailChars int /* = 6000 */) string {
	opener := BuildOpener()
	req := &Request{
		Method: "GET",
		URL:    x.URL(x.r() + "/actions/jobs/" + py.Str(jobID) + "/logs"),
		Header: x.headers(""),
	}
	location := ""
	resp, err := opener(req, 60*time.Second)
	if err == nil {
		return redact.Redact(py.SliceFrom(py.DecodeReplace(resp.Body), -tailChars), nil)
	}
	var exc *HTTPError
	switch {
	case errors.As(err, &exc):
		if !redirectCodes[exc.Code] {
			return fmt.Sprintf("(log unavailable: HTTP %d)", exc.Code)
		}
		location = exc.Header.Get("Location")
	case isURLOrTimeout(err):
		return fmt.Sprintf("(log unavailable: %s)", err)
	default:
		herr.Raise(err)
	}
	if location == "" {
		return "(log unavailable)"
	}
	resp, err = Urlopen(&Request{Method: "GET", URL: location, Header: http.Header{}}, 60*time.Second)
	if err != nil {
		if isURLOrTimeout(err) {
			return fmt.Sprintf("(log unavailable: %s)", err)
		}
		herr.Raise(err)
	}
	return redact.Redact(py.SliceFrom(py.DecodeReplace(resp.Body), -tailChars), nil)
}

// isURLOrTimeout is `except (urllib.error.URLError, TimeoutError)`, an HTTPError among them.
func isURLOrTimeout(err error) bool {
	var h *HTTPError
	var u *URLError
	var t *TimeoutError
	return errors.As(err, &h) || errors.As(err, &u) || errors.As(err, &t)
}

func (x *GitHub) ListArtifacts(runID any) []*pyjson.Object {
	return objs(x.Paginate(x.r()+"/actions/runs/"+py.Str(runID)+"/artifacts", nil, "artifacts", 1000, false))
}

// DownloadArtifact is an artifact's zip. GitHub answers with a redirect to its storage, which is
// followed without the token, as `job_log`'s is.
func (x *GitHub) DownloadArtifact(artifactID any) []byte {
	what := "GET " + x.r() + "/actions/artifacts/" + py.Str(artifactID) + "/zip"
	opener := BuildOpener()
	req := &Request{
		Method: "GET",
		URL:    x.URL(x.r() + "/actions/artifacts/" + py.Str(artifactID) + "/zip"),
		Header: x.headers(""),
	}
	location := ""
	resp, err := opener(req, 60*time.Second)
	if err == nil {
		return resp.Body
	}
	var exc *HTTPError
	switch {
	case errors.As(err, &exc):
		if !redirectCodes[exc.Code] {
			herr.Raise(&herr.GitHubError{Msg: fmt.Sprintf("%s: HTTP %d", what, exc.Code), Status: exc.Code})
		}
		location = exc.Header.Get("Location")
	case isURLOrTimeout(err):
		herr.Raise(&herr.GitHubError{Msg: what + ": " + redact.Redact(err.Error(), nil), Status: 0})
	default:
		herr.Raise(err)
	}
	if location == "" {
		herr.Raise(&herr.GitHubError{Msg: what + ": a redirect with no location", Status: 0})
	}
	resp, err = Urlopen(&Request{Method: "GET", URL: location, Header: http.Header{}}, 120*time.Second)
	if err != nil {
		if errors.As(err, &exc) {
			herr.Raise(&herr.GitHubError{Msg: fmt.Sprintf("%s: the archive answered HTTP %d", what, exc.Code), Status: exc.Code})
		}
		if isURLOrTimeout(err) {
			herr.Raise(&herr.GitHubError{Msg: what + ": the archive: " + redact.Redact(err.Error(), nil), Status: 0})
		}
		herr.Raise(err)
	}
	return resp.Body
}

func (x *GitHub) CheckRuns(sha string) []*pyjson.Object {
	return objs(x.Paginate(x.r()+"/commits/"+sha+"/check-runs", nil, "check_runs", 1000, false))
}

// ------------------------------------------------------------------ settings (operator only)

// UpdateRepo is `update_repo(**fields)`.
func (x *GitHub) UpdateRepo(fields *pyjson.Object) *pyjson.Object {
	return obj(x.Request("PATCH", x.r(), fields.Copy(), "", false, false))
}

func (x *GitHub) SetProtection(branch string, checks []string) {
	x.Request(
		"PUT",
		x.r()+"/branches/"+branch+"/protection",
		pyjson.New(
			"required_status_checks", pyjson.New("strict", false, "contexts", pyjson.ListOf(checks)),
			"enforce_admins", false,
			"required_pull_request_reviews", nil,
			"restrictions", nil,
			"allow_force_pushes", false,
			"allow_deletions", false,
		),
		"", false, false,
	)
}
