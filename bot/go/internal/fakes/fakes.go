// Package fakes is bot/tests/fakes.py: test doubles, an in-memory GitHub, and helpers for real
// git repositories in a temp dir.
//
// # How a test calls it
//
// The same conventions hold in internal/testsupport:
//
//   - Python's keyword arguments with defaults are one trailing options struct, whose zero fields
//     are the Python defaults; it may be left out:
//     gh.AddIssue(7), gh.AddIssue(7, fakes.Issue{Labels: []string{config.LabelBuild}}),
//     gh.AddPull(9, "bot/issue-7", fakes.Pull{SHA: "s1"}),
//     gh.AddComment(7, "Go.", fakes.Comment{User: fakes.Stranger, Association: "NONE"}).
//     A zero field cannot ask for "" where the default is not empty (an issue's title and body, a
//     pull request's title and sha, a comment's association and time): set it on the dict the
//     method returns, gh.AddIssue(7).Set("body", "").
//   - A trailing positional argument with a default is variadic: fakes.MakeOrigin(root) or
//     fakes.MakeOrigin(root, files); fakes.PushBranch(origin, root, branch, files) or
//     fakes.PushBranch(origin, root, branch, files, "dev").
//   - FakeGitHub's gh.Client methods take every argument, as the interface does.
//   - A method replaced on the instance (`gh.put_file = put`, or mock.patch.object(gh,
//     "list_pulls", ...)) is a func set in gh.Patch (gh.Patch.PutFile = put); nil puts the fake's
//     own back. A replacement that calls on to the fake's own method calls gh.Own().PutFile(...).
//     Every other caller (the harness through ctx.GH, the state store, the fake's own methods,
//     as Python's `self.get_issue` does) goes through the replacement.
//
// # What the fields hold
//
// Each field is the Python attribute of the same name, in a Go type that keeps what a test or the
// fake can observe:
//
//   - JSON (an issue, a comment, a run, a label, a job) is a *pyjson.Object, a list of it a
//     []*pyjson.Object; a list inside JSON is a []any.
//   - A dict whose order shows (the fake or a test walks it) keeps its order: a *Dict when keyed by
//     number (Threads, Comments, Reviews, ReviewComments), a *pyjson.Object when keyed by text
//     (Labels, Runs, RepoSettings). A dict only ever looked up is a Go map (Jobs, Artifacts,
//     AutoMerge, …).
//   - A set is a map[T]bool; nil stands for None where the Python allows it (BranchChecks).
//   - A tuple is a [2]string (Files' keys and values, CreatedBranches) or a small struct
//     (Reaction, ReviewRequest, Dispatch).
//   - An attribute fakes.py reads with getattr or creates on first use (commits, gists,
//     created_branches, deleted_branches) is a field that starts nil.
//   - A list the fake returns is never nil, as a Python list is never None.
//
// Where fakes.py raises, the fake panics with the same: a *herr.GitHubError with the same message
// and status where GitHub would refuse, a *PyError (KeyError, StopIteration, AttributeError, …)
// where Python itself would raise.
package fakes

import (
	"fmt"
	"maps"
	"slices"
	"sort"
	"strconv"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Repo = "jgoetzmann/JackiOh"
var Repo = "jgoetzmann/JackiOh"

// Bot = {"login": "jgoetzmann-bot", "id": 324324626}
var Bot = pyjson.New("login", "jgoetzmann-bot", "id", 324324626)

// Operator = {"login": "jgoetzmann", "id": 95732896}
var Operator = pyjson.New("login", "jgoetzmann", "id", 95732896)

// Stranger = {"login": "someone-else", "id": 5}
var Stranger = pyjson.New("login", "someone-else", "id", 5)

// The subset of `harness.gh.GitHub` the harness calls, kept in memory.
type FakeGitHub struct {
	Repo           string
	Threads        *Dict[int, *pyjson.Object]
	Comments       *Dict[int, []*pyjson.Object]
	Reviews        *Dict[int, []*pyjson.Object]
	ReviewComments *Dict[int, []*pyjson.Object]
	Labels         *pyjson.Object
	// (ref, path) → (text, blob sha).
	Files      map[[2]string][2]string
	Branches   map[string]bool
	Dispatches []Dispatch
	Pinned     []string
	Runs       *pyjson.Object
	Jobs       map[string][]*pyjson.Object
	// A run's uploads, by run id, and each upload's zip, by its id.
	Artifacts      map[string][]*pyjson.Object
	ArtifactZips   map[int][]byte
	Reruns         []any
	Reacted        []Reaction
	AutoMerge      map[string]string
	AutoMergeHeads map[string]string
	// Pull requests in `main`'s merge queue, by node id (`queue.hold_merge`).
	MergeQueue map[string]bool
	// How the queue last took a pull request out, by node id (`GitHub.queue_removal`).
	QueueRemovals map[string]map[string]string
	Blockers      map[int][]int
	// The merge commit title asked for, by pull request node id.
	MergeTitles       map[string]string
	AutoMergeError    string
	Protection        *pyjson.Object  // None: nil
	BranchChecks      map[string]bool // None: nil
	ReviewRequests    []ReviewRequest
	RepoSettings      *pyjson.Object
	ConflictsToInject int
	DryRun            bool

	// What fakes.py reads with getattr(self, "commits", []) and getattr(self, "gists", {}): a
	// test sets them.
	Commits []*pyjson.Object
	Gists   map[string]*pyjson.Object
	// What create_branch and delete_branch record, from the first call.
	CreatedBranches [][2]string // (branch, sha)
	DeletedBranches []string

	// Methods a test replaced on this instance; nil is the fake's own (see the package doc).
	Patch Patches

	ids  int // itertools.count(1000)
	shas int // itertools.count(1)
}

// Dispatch is one entry of `dispatches`: {"workflow", "ref", "inputs"}.
type Dispatch struct {
	Workflow string
	Ref      string
	Inputs   map[string]string
}

// Reaction is one entry of `reacted`: (comment id, content).
type Reaction struct {
	ID      int
	Content string
}

// ReviewRequest is one entry of `review_requests`: (number, logins).
type ReviewRequest struct {
	Number int
	Logins []string
}

// NewFakeGitHub is FakeGitHub(). Its repo is Repo; a test that wants another sets gh.Repo.
func NewFakeGitHub() *FakeGitHub {
	return &FakeGitHub{
		Repo:           Repo,
		Threads:        NewDict[int, *pyjson.Object](),
		Comments:       NewDict[int, []*pyjson.Object](),
		Reviews:        NewDict[int, []*pyjson.Object](),
		ReviewComments: NewDict[int, []*pyjson.Object](),
		Labels:         pyjson.New(),
		Files:          map[[2]string][2]string{},
		Branches:       map[string]bool{"main": true},
		Dispatches:     []Dispatch{},
		Pinned:         []string{},
		Runs:           pyjson.New(),
		Jobs:           map[string][]*pyjson.Object{},
		Artifacts:      map[string][]*pyjson.Object{},
		ArtifactZips:   map[int][]byte{},
		Reruns:         []any{},
		Reacted:        []Reaction{},
		AutoMerge:      map[string]string{},
		AutoMergeHeads: map[string]string{},
		MergeQueue:     map[string]bool{},
		QueueRemovals:  map[string]map[string]string{},
		Blockers:       map[int][]int{},
		MergeTitles:    map[string]string{},
		ReviewRequests: []ReviewRequest{},
		RepoSettings:   pyjson.New("allow_auto_merge", true),
		ids:            1000,
		shas:           1,
	}
}

var _ gh.Client = (*FakeGitHub)(nil)

// Own is the fake's own methods, fakes.py's, past any replacement in Patch: what a replacement
// calls to go on to the real thing (Python's `real_put = gh.put_file` taken before the patch).
type Own FakeGitHub

// Own is the fake's own methods (see the type).
func (c *FakeGitHub) Own() *Own { return (*Own)(c) }

func (o *Own) self() *FakeGitHub { return (*FakeGitHub)(o) }

func (c *FakeGitHub) nextID() int {
	id := c.ids
	c.ids++
	return id
}

func (c *FakeGitHub) nextSHA() string {
	n := c.shas
	c.shas++
	return fmt.Sprintf("sha%d", n)
}

// ---------------------------------------------------------------------------- building the world

// Issue is AddIssue's keyword arguments; a zero field is the default.
type Issue struct {
	Title  string         // = "An issue"
	Body   string         // = "Do the thing."
	Labels []string       // = ()
	User   *pyjson.Object // = None: Operator
	State  string         // = "open"
}

// Pull is AddPull's keyword arguments; a zero field is the default.
type Pull struct {
	Title  string         // = "A pull request"
	Body   string         // = ""
	Labels []string       // = ()
	SHA    string         // = "abc"
	Fork   bool           // = False
	User   *pyjson.Object // = None: Operator
}

// Comment is AddComment's and AddReviewComment's keyword arguments; a zero field is the default.
type Comment struct {
	User        *pyjson.Object // = None: Operator
	Association string         // = "OWNER"
	CreatedAt   string         // = "2026-09-20T00:00:00Z"
}

// AddIssue puts an issue in the world and returns it (the thread itself, not a copy).
func (c *FakeGitHub) AddIssue(number int, opts ...Issue) *pyjson.Object {
	o := one(opts, "AddIssue")
	return c.addIssue(number, orDefault(o.Title, "An issue"), orDefault(o.Body, "Do the thing."),
		o.Labels, o.User, orDefault(o.State, "open"))
}

func (c *FakeGitHub) addIssue(number int, title, body string, labels []string, user *pyjson.Object,
	state string) *pyjson.Object {
	names := make([]any, 0, len(labels))
	for _, n := range labels {
		names = append(names, pyjson.New("name", n))
	}
	thread := pyjson.New(
		"number", number, "title", title, "body", body, "state", state,
		"labels", names, "user", userOr(user).Copy(),
		"created_at", fmt.Sprintf("2026-09-%02dT00:00:00Z", 10+py.Mod(number, 15)), "assignees", []any{},
		"id", 5_000_000+number,
	)
	c.Threads.Set(number, thread)
	c.Comments.SetDefault(number, []*pyjson.Object{})
	return thread
}

// AddPull puts a pull request from `branch` in the world and returns it (the thread itself).
func (c *FakeGitHub) AddPull(number int, branch string, opts ...Pull) *pyjson.Object {
	o := one(opts, "AddPull")
	return c.addPull(number, branch, orDefault(o.Title, "A pull request"), o.Body, o.Labels,
		orDefault(o.SHA, "abc"), o.Fork, o.User)
}

func (c *FakeGitHub) addPull(number int, branch, title, body string, labels []string, sha string,
	fork bool, user *pyjson.Object) *pyjson.Object {
	thread := c.addIssue(number, title, body, labels, user, "open")
	fullName := c.Repo
	if fork {
		fullName = "someone/fork"
	}
	thread.Update(pyjson.New(
		"pull_request", pyjson.New(),
		"head", pyjson.New("ref", branch, "sha", sha, "repo", pyjson.New("full_name", fullName)),
		"base", pyjson.New("ref", "main"),
		"node_id", fmt.Sprintf("PR_%d", number),
		"draft", false,
		"auto_merge", nil,
		"mergeable_state", "clean",
		"merged", false,
	))
	c.Branches[branch] = true
	return thread
}

// AddComment puts a conversation comment on `number` and returns it (the comment itself).
func (c *FakeGitHub) AddComment(number int, body string, opts ...Comment) *pyjson.Object {
	o := one(opts, "AddComment")
	comment := pyjson.New(
		"id", c.nextID(), "body", body, "user", userOr(o.User).Copy(),
		"author_association", orDefault(o.Association, "OWNER"),
		"created_at", orDefault(o.CreatedAt, "2026-09-20T00:00:00Z"),
		"issue_url", fmt.Sprintf("https://api.github.com/repos/%s/issues/%d", c.Repo, number),
	)
	c.Comments.Set(number, append(c.Comments.Get(number), comment))
	return comment
}

// AddReviewComment puts a line comment on pull request `number` and returns it.
func (c *FakeGitHub) AddReviewComment(number int, body string, opts ...Comment) *pyjson.Object {
	o := one(opts, "AddReviewComment")
	comment := pyjson.New(
		"id", c.nextID(), "body", body, "user", userOr(o.User).Copy(),
		"author_association", orDefault(o.Association, "OWNER"),
		"created_at", orDefault(o.CreatedAt, "2026-09-20T00:00:00Z"),
		"path", "src/x.ts", "line", 3,
		"pull_request_url", fmt.Sprintf("https://api.github.com/repos/%s/pulls/%d", c.Repo, number),
	)
	c.ReviewComments.Set(number, append(c.ReviewComments.Get(number), comment))
	return comment
}

// LabelNames is the names of the labels on `number`.
func (c *FakeGitHub) LabelNames(number int) map[string]bool {
	names := map[string]bool{}
	for _, label := range pyjson.List(item(c.Threads.Item(number), "labels")) {
		names[pyjson.Str(item(pyjson.Obj(label), "name"))] = true
	}
	return names
}

// BotComments is the bodies of the bot's comments on `number`, in order.
func (c *FakeGitHub) BotComments(number int) []string {
	out := []string{}
	for _, comment := range c.Comments.Get(number) {
		user := pyjson.Obj(item(comment, "user"))
		if pyjson.Equal(item(user, "login"), Bot.Get("login")) {
			out = append(out, pyjson.Str(item(comment, "body")))
		}
	}
	return out
}

// ---------------------------------------------------------------------------- issues

func (o *Own) GetIssue(number int) *pyjson.Object {
	thread, ok := o.Threads.Lookup(number)
	if !ok {
		panic(githubError(fmt.Sprintf("no issue %d", number), 404))
	}
	return deep(thread)
}

func (o *Own) ListIssues(labels string, state string, limit int, assignee string) []*pyjson.Object {
	var wanted []string
	for _, n := range strings.Split(labels, ",") {
		if n != "" {
			wanted = append(wanted, n)
		}
	}
	found := []*pyjson.Object{}
	numbers := o.Threads.Keys()
	sort.Ints(numbers)
	for _, number := range numbers {
		thread := o.Threads.Get(number)
		if state != "all" && !pyjson.Equal(item(thread, "state"), state) {
			continue
		}
		names := map[any]bool{}
		for _, label := range pyjson.List(item(thread, "labels")) {
			names[item(pyjson.Obj(label), "name")] = true
		}
		assigned := map[any]bool{}
		for _, a := range pyjson.List(thread.GetOr("assignees", []any{})) {
			assigned[pyjson.Obj(a).Get("login")] = true
		}
		all := true
		for _, n := range wanted {
			if !names[n] {
				all = false
				break
			}
		}
		if all && (assignee == "" || assigned[assignee]) {
			found = append(found, deep(thread))
		}
	}
	return head(found, limit)
}

// Block links `blockers` as GitHub issue dependencies blocking `number`.
func (c *FakeGitHub) Block(number int, blockers ...int) {
	c.Blockers[number] = append(c.Blockers[number], blockers...)
	open := 0
	for _, b := range c.Blockers[number] {
		if pyjson.Equal(item(c.Threads.Item(b), "state"), "open") {
			open++
		}
	}
	c.Threads.Item(number).Set("issue_dependencies_summary", pyjson.New(
		"blocked_by", open, "total_blocked_by", len(c.Blockers[number]),
		"blocking", 0, "total_blocking", 0))
}

func (o *Own) BlockedBy(number int) []*pyjson.Object {
	out := []*pyjson.Object{}
	for _, b := range o.Blockers[number] {
		out = append(out, deep(o.Threads.Item(b)))
	}
	return out
}

func (o *Own) Blocking(number int) []*pyjson.Object {
	out := []*pyjson.Object{}
	for _, n := range slices.Sorted(maps.Keys(o.Blockers)) {
		if slices.Contains(o.Blockers[n], number) {
			out = append(out, deep(o.Threads.Item(n)))
		}
	}
	return out
}

func (c *FakeGitHub) byID(issueID int) int {
	for n, thread := range c.Threads.All() {
		if pyjson.Equal(thread.Get("id"), issueID) {
			return n
		}
	}
	panic(githubError(fmt.Sprintf("no issue with id %d", issueID), 404))
}

func (o *Own) AddBlockedBy(number int, blockerID int) {
	o.self().Block(number, o.self().byID(blockerID))
}

func (o *Own) AddSubIssue(parent int, childID int) {
	child := o.self().byID(childID)
	o.Threads.Item(child).Set("parent_issue_url",
		fmt.Sprintf("https://api.github.com/repos/%s/issues/%d", o.Repo, parent))
}

func (o *Own) ListSubIssues(parent int) []*pyjson.Object {
	suffix := fmt.Sprintf("/issues/%d", parent)
	out := []*pyjson.Object{}
	numbers := o.Threads.Keys()
	sort.Ints(numbers)
	for _, n := range numbers {
		thread := o.Threads.Get(n)
		if strings.HasSuffix(py.OrStr(thread.Get("parent_issue_url"), ""), suffix) {
			out = append(out, deep(thread))
		}
	}
	return out
}

func (o *Own) ListComments(number int, limit int) []*pyjson.Object {
	return deepList(o.Comments.Get(number))
}

func (o *Own) ListCommits(author string, limit int) []*pyjson.Object {
	out := []*pyjson.Object{}
	for _, commit := range o.Commits {
		if author == "" || pyjson.Equal(commit.Obj("author").Get("login"), author) {
			out = append(out, commit)
		}
	}
	return head(out, limit)
}

func (o *Own) Lock(number int) { o.Threads.Item(number).Set("locked", true) }

func (o *Own) Unlock(number int) { o.Threads.Item(number).Set("locked", false) }

func (o *Own) CreateComment(number int, body string) *pyjson.Object {
	if o.Threads.Get(number).Truthy("locked") {
		// As the workflow's own token finds a locked thread (lock-conversations.yml).
		panic(githubError("Unable to create comment because issue is locked.", 403))
	}
	comment := pyjson.New(
		"id", o.self().nextID(), "body", gh.WithMarker(body), "user", Bot.Copy(),
		"author_association", "COLLABORATOR", "created_at", "2026-09-29T00:00:00Z",
		"issue_url", fmt.Sprintf("https://api.github.com/repos/%s/issues/%d", o.Repo, number),
	)
	o.Comments.Set(number, append(o.Comments.Get(number), comment))
	return comment
}

func (o *Own) UpdateComment(commentID int, body string) *pyjson.Object {
	for _, comments := range o.Comments.Values() {
		for _, comment := range comments {
			if pyjson.Equal(item(comment, "id"), commentID) {
				comment.Set("body", gh.WithMarker(body))
				return comment.Copy()
			}
		}
	}
	panic(githubError("no comment", 404))
}

func (o *Own) React(commentID int, content string, reviewComment bool) {
	o.Reacted = append(o.Reacted, Reaction{ID: commentID, Content: content})
}

func (o *Own) Reactions(commentID int, reviewComment bool) []*pyjson.Object {
	out := []*pyjson.Object{}
	for _, r := range o.Reacted {
		if r.ID == commentID {
			out = append(out, pyjson.New("content", r.Content, "user", Bot.Copy()))
		}
	}
	return out
}

func (o *Own) ListRepoComments(since string, limit int) []*pyjson.Object {
	return deepList(head(commentsSince(o.Comments, since), limit))
}

func (o *Own) ListRepoReviewComments(since string, limit int) []*pyjson.Object {
	return deepList(head(commentsSince(o.ReviewComments, since), limit))
}

// commentsSince is `[c for items in d.values() for c in items if c.get("created_at", "") >= since]`.
func commentsSince(d *Dict[int, []*pyjson.Object], since string) []*pyjson.Object {
	found := []*pyjson.Object{}
	for _, items := range d.Values() {
		for _, comment := range items {
			if pyjson.Str(comment.GetOr("created_at", "")) >= since {
				found = append(found, comment)
			}
		}
	}
	return found
}

func (o *Own) RunsForSHA(sha string, limit int) []*pyjson.Object {
	out := []*pyjson.Object{}
	for _, run := range runs(o.Runs) {
		if pyjson.Equal(run.Get("head_sha"), sha) {
			out = append(out, run.Copy())
		}
	}
	return head(out, limit)
}

func (o *Own) CreateIssue(title string, body string, labels []string) *pyjson.Object {
	number := o.self().nextNumber()
	thread := o.self().addIssue(number, title, gh.WithMarker(body), labels, Bot, "open")
	thread.Set("node_id", fmt.Sprintf("I_%d", number))
	return deep(thread)
}

func (o *Own) PinIssue(nodeID string) {
	if len(o.Pinned) >= 3 {
		panic(githubError("a repository pins at most three issues", 422))
	}
	o.Pinned = append(o.Pinned, nodeID)
}

func (o *Own) SetIssueBody(number int, body string) *pyjson.Object {
	o.Threads.Item(number).Set("body", body)
	return deep(o.Threads.Item(number))
}

func (o *Own) UpdateIssue(number int, fields *pyjson.Object) *pyjson.Object {
	o.Threads.Item(number).Update(fields)
	return deep(o.Threads.Item(number))
}

func (o *Own) AddLabels(number int, names []string) {
	current := o.self().LabelNames(number)
	for _, name := range names {
		if !current[name] {
			thread := o.Threads.Item(number)
			thread.Set("labels", append(pyjson.List(item(thread, "labels")), pyjson.New("name", name)))
		}
	}
}

func (o *Own) AddAssignees(number int, logins []string) {
	thread := o.Threads.Item(number)
	current := map[any]bool{}
	for _, a := range pyjson.List(thread.SetDefault("assignees", []any{})) {
		current[pyjson.Obj(a).Get("login")] = true
	}
	for _, login := range logins {
		if !current[login] {
			thread.Set("assignees", append(pyjson.List(item(thread, "assignees")), pyjson.New("login", login)))
		}
	}
}

func (o *Own) RemoveAssignees(number int, logins []string) {
	gone := map[string]bool{}
	for _, login := range logins {
		gone[strings.ToLower(login)] = true
	}
	thread := o.Threads.Item(number)
	kept := []any{}
	for _, a := range pyjson.List(thread.GetOr("assignees", []any{})) {
		if !gone[strings.ToLower(py.Str(pyjson.Obj(a).Get("login")))] {
			kept = append(kept, a)
		}
	}
	thread.Set("assignees", kept)
}

func (o *Own) RemoveLabel(number int, name string) {
	thread := o.Threads.Item(number)
	kept := []any{}
	for _, label := range pyjson.List(item(thread, "labels")) {
		if !pyjson.Equal(item(pyjson.Obj(label), "name"), name) {
			kept = append(kept, label)
		}
	}
	thread.Set("labels", kept)
}

func (o *Own) ListLabels() []*pyjson.Object {
	out := []*pyjson.Object{}
	o.Labels.Items(func(_ string, v any) bool {
		out = append(out, pyjson.Obj(v))
		return true
	})
	return out
}

func (o *Own) ListIssueTypes() []*pyjson.Object {
	return []*pyjson.Object{
		pyjson.New("name", "Task", "description", "A specific piece of work"),
		pyjson.New("name", "Bug", "description", "An unexpected problem or behavior"),
		pyjson.New("name", "Feature", "description", "A request, idea, or new functionality"),
	}
}

func (o *Own) EnsureLabel(name string, color string, description string) bool {
	if o.Labels.Has(name) {
		return false
	}
	o.Labels.Set(name, pyjson.New("name", name, "color", color, "description", description))
	return true
}

func (o *Own) DeleteLabel(name string) bool {
	if v, _ := o.Labels.Pop(name); isNone(v) {
		return false
	}
	for _, thread := range o.Threads.Values() {
		kept := []any{}
		for _, label := range pyjson.List(item(thread, "labels")) {
			if !pyjson.Equal(pyjson.Obj(label).Get("name"), name) {
				kept = append(kept, label)
			}
		}
		thread.Set("labels", kept)
	}
	return true
}

func (o *Own) SyncLabel(name string, color string, description string) string {
	v := o.Labels.Get(name)
	if isNone(v) {
		o.Labels.Set(name, pyjson.New("name", name, "color", color, "description", description))
		return "created"
	}
	found := pyjson.Obj(v)
	if strings.ToLower(py.OrStr(found.Get("color"), "")) == strings.ToLower(color) &&
		py.OrStr(found.Get("description"), "") == description {
		return ""
	}
	found.Set("color", color)
	found.Set("description", description)
	return "updated"
}

// ---------------------------------------------------------------------------- pull requests

func (o *Own) GetPull(number int) *pyjson.Object {
	thread := o.self().GetIssue(number)
	if !thread.Has("pull_request") {
		panic(githubError(fmt.Sprintf("%d is not a pull request", number), 404))
	}
	return thread
}

func (o *Own) ListPulls(state string, head string, limit int) []*pyjson.Object {
	found := []*pyjson.Object{}
	for _, thread := range o.Threads.Values() {
		if !thread.Has("pull_request") || (state != "all" && !pyjson.Equal(item(thread, "state"), state)) {
			continue
		}
		if head != "" && !pyjson.Equal(item(pyjson.Obj(item(thread, "head")), "ref"), head) {
			continue
		}
		found = append(found, deep(thread))
	}
	return found
}

func (o *Own) CreatePull(title string, head string, base string, body string, draft bool) *pyjson.Object {
	number := o.self().nextNumber()
	thread := o.self().addPull(number, head, title, gh.WithMarker(body), nil, "pushed", false, Bot)
	thread.Set("draft", draft)
	return deep(thread)
}

func (o *Own) UpdatePull(number int, fields *pyjson.Object) *pyjson.Object {
	o.Threads.Item(number).Update(fields)
	return deep(o.Threads.Item(number))
}

func (o *Own) ListReviews(number int) []*pyjson.Object {
	return deepList(o.Reviews.Get(number))
}

func (o *Own) ListReviewComments(number int) []*pyjson.Object {
	return deepList(o.ReviewComments.Get(number))
}

// pullByNode is `next(t for t in self.threads.values() if t.get("node_id") == node_id)`: a
// StopIteration when there is none.
func (c *FakeGitHub) pullByNode(nodeID string) *pyjson.Object {
	for _, thread := range c.Threads.Values() {
		if pyjson.Equal(thread.Get("node_id"), nodeID) {
			return thread
		}
	}
	panic(&PyError{Type: "StopIteration"})
}

func (o *Own) RequestReviewers(number int, logins []string) {
	o.ReviewRequests = append(o.ReviewRequests, ReviewRequest{Number: number, Logins: append([]string{}, logins...)})
}

func (o *Own) RequiredChecks(branch string) map[string]bool {
	if o.BranchChecks == nil {
		return nil
	}
	return maps.Clone(o.BranchChecks)
}

func (o *Own) EnableAutoMerge(nodeID string, method string, expectedHead string, headline string) {
	if o.AutoMergeError != "" {
		panic(githubError(o.AutoMergeError, 200))
	}
	o.AutoMerge[nodeID] = method
	o.AutoMergeHeads[nodeID] = expectedHead
	o.MergeTitles[nodeID] = headline
	o.self().pullByNode(nodeID).Set("auto_merge", pyjson.New("merge_method", method))
}

func (o *Own) DisableAutoMerge(nodeID string) {
	delete(o.AutoMerge, nodeID)
	o.self().pullByNode(nodeID).Set("auto_merge", nil)
}

func (o *Own) InMergeQueue(nodeID string) bool { return o.MergeQueue[nodeID] }

func (o *Own) QueueRemoval(nodeID string) map[string]string {
	if o.MergeQueue[nodeID] || o.self().pullByNode(nodeID).Truthy("auto_merge") {
		return nil
	}
	return o.QueueRemovals[nodeID]
}

func (o *Own) Dequeue(nodeID string) {
	if !o.MergeQueue[nodeID] {
		panic(githubError("not in the merge queue", 200))
	}
	delete(o.MergeQueue, nodeID)
}

func (o *Own) MarkReady(nodeID string) { o.self().pullByNode(nodeID).Set("draft", false) }

func (o *Own) MergePull(number int, method string, title string) *pyjson.Object {
	thread := o.Threads.Item(number)
	thread.Set("state", "closed")
	thread.Set("merged", true)
	o.MergeTitles[fmt.Sprintf("PR_%d", number)] = title
	return pyjson.New("merged", true)
}

// ---------------------------------------------------------------------------- contents

// GetFile is `(text, sha)`, or ("", "") where fakes.py answers (None, None): gh.Client's
// signature has no None, so a missing file reads as an empty sha.
func (o *Own) GetFile(path string, ref string) (string, string) {
	if found, ok := o.Files[[2]string{ref, path}]; ok {
		return found[0], found[1]
	}
	return "", ""
}

// PutFile refuses a stale `sha` as GitHub does; "" is fakes.py's None (a new file).
func (o *Own) PutFile(path string, text string, branch string, sha string, message string) {
	if !o.Branches[branch] {
		panic(githubError(fmt.Sprintf("no branch %s", branch), 404))
	}
	key := [2]string{branch, path}
	current, exists := o.Files[key]
	if o.ConflictsToInject > 0 {
		o.ConflictsToInject--
		// Someone else wrote in between: their change lands, ours is refused.
		other := pyjson.New()
		if exists {
			other = loadsObject(current[0])
		}
		pyjson.Obj(other.SetDefault("items", pyjson.New())).Set("999", pyjson.New("note", "written by another run"))
		o.Files[key] = [2]string{pyjson.Dumps(other), o.self().nextSHA()}
		panic(githubError("conflict", 409))
	}
	// (current[1] if current else None) != (sha or None)
	if exists && (sha == "" || current[1] != sha) || !exists && sha != "" {
		panic(githubError("sha mismatch", 409))
	}
	o.Files[key] = [2]string{text, o.self().nextSHA()}
}

func (o *Own) ListDir(path string, ref string) [][2]string {
	prefix := strings.TrimRight(path, "/") + "/"
	out := [][2]string{}
	for key, found := range o.Files {
		branch, name := key[0], key[1]
		if branch == ref && strings.HasPrefix(name, prefix) && !strings.Contains(name[len(prefix):], "/") {
			out = append(out, [2]string{name[len(prefix):], found[1]})
		}
	}
	sort.Slice(out, func(i, j int) bool {
		if out[i][0] != out[j][0] {
			return out[i][0] < out[j][0]
		}
		return out[i][1] < out[j][1]
	})
	return out
}

func (o *Own) DeleteFile(path string, branch string, sha string, message string) {
	key := [2]string{branch, path}
	current, ok := o.Files[key]
	if !ok {
		panic(githubError("no file", 404))
	}
	if current[1] != sha {
		panic(githubError("sha mismatch", 409))
	}
	delete(o.Files, key)
}

// BranchSHA is "" where fakes.py answers None.
func (o *Own) BranchSHA(branch string) string {
	if o.Branches[branch] {
		return "head-of-" + branch
	}
	return ""
}

// CreateOrphanBranch writes `extra` and then `path` (`{**extra, path: text}`), each with the next
// sha. gh.Client's `extra` is a Go map, which has none of the dict's order, so its files are
// written in sorted order.
func (o *Own) CreateOrphanBranch(branch string, path string, text string, message string, extra map[string]string) {
	o.Branches[branch] = true
	names := slices.Sorted(maps.Keys(extra))
	if _, ok := extra[path]; !ok {
		names = append(names, path)
	}
	for _, name := range names {
		body := extra[name]
		if name == path {
			body = text
		}
		o.Files[[2]string{branch, name}] = [2]string{body, o.self().nextSHA()}
	}
}

func (o *Own) CreateBranch(branch string, sha string) {
	if o.Branches[branch] {
		panic(githubError(fmt.Sprintf("%s exists", branch), 422))
	}
	o.Branches[branch] = true
	o.CreatedBranches = append(slices.Clone(o.CreatedBranches), [2]string{branch, sha})
}

func (o *Own) DeleteBranch(branch string) {
	delete(o.Branches, branch)
	o.DeletedBranches = append(slices.Clone(o.DeletedBranches), branch)
}

// ---------------------------------------------------------------------------- actions

func (o *Own) DispatchWorkflow(workflow string, ref string, inputs map[string]string) {
	copied := map[string]string{}
	maps.Copy(copied, inputs)
	o.Dispatches = append(o.Dispatches, Dispatch{Workflow: workflow, Ref: ref, Inputs: copied})
}

func (o *Own) GetRun(runID any) *pyjson.Object {
	key := py.Str(runID)
	if !o.Runs.Has(key) {
		panic(githubError("no run", 404))
	}
	return pyjson.Obj(o.Runs.Get(key)).Copy()
}

func (o *Own) ListRuns(workflow string, status string, limit int, cached bool) []*pyjson.Object {
	// A run that names its workflow is listed only for it; one that does not, for any.
	out := []*pyjson.Object{}
	for _, run := range runs(o.Runs) {
		if pyjson.Equal(run.GetOr("workflow", workflow), workflow) {
			out = append(out, run)
		}
	}
	return head(out, limit)
}

func (o *Own) RerunFailedJobs(runID any) { o.Reruns = append(o.Reruns, runID) }

func (o *Own) GetGist(gistID string, cached bool) *pyjson.Object {
	gist, ok := o.Gists[gistID]
	if !ok {
		panic(githubError("no gist", 404))
	}
	return deep(gist)
}

func (o *Own) ListRepoRuns(status string, limit int, cached bool) []*pyjson.Object {
	out := []*pyjson.Object{}
	for _, run := range runs(o.Runs) {
		if pyjson.Equal(run.Get("status"), status) {
			out = append(out, run.Copy())
		}
	}
	return head(out, limit)
}

func (o *Own) ListJobs(runID any, cached bool) []*pyjson.Object {
	return deepList(o.Jobs[py.Str(runID)])
}

func (o *Own) JobLog(jobID any, tailChars int) string {
	return fmt.Sprintf("log of job %s: AssertionError: expected 3 to be 4", py.Str(jobID))
}

func (o *Own) ListArtifacts(runID any) []*pyjson.Object {
	return deepList(o.Artifacts[py.Str(runID)])
}

func (o *Own) DownloadArtifact(artifactID any) []byte {
	zipped, ok := o.ArtifactZips[pyInt(artifactID)]
	if !ok {
		panic(githubError("no artifact", 404))
	}
	return zipped
}

func (o *Own) CheckRuns(sha string) []*pyjson.Object { return []*pyjson.Object{} }

// ---------------------------------------------------------------------------- settings

func (o *Own) RepoInfo() *pyjson.Object { return o.RepoSettings.Copy() }

func (o *Own) Viewer() *pyjson.Object { return Bot.Copy() }

func (o *Own) SetProtection(branch string, checks []string) {
	o.Protection = pyjson.New("required_status_checks", pyjson.New("contexts", pyjson.ListOf(checks)))
}

func (o *Own) UpdateRepo(fields *pyjson.Object) *pyjson.Object {
	o.RepoSettings.Update(fields)
	return o.RepoSettings.Copy()
}

// ---------------------------------------------------------------------------- not in fakes.py

// The gh.Client methods fakes.py does not define: Python's AttributeError, unless a test set one
// in Patch.

func (o *Own) URL(path string) string { panic(notFaked("URL")) }

func (o *Own) Request(method string, path string, body any, accept string, raw bool, cached bool) any {
	panic(notFaked("Request"))
}

func (o *Own) Paginate(path string, params *pyjson.Object, key string, limit int, cached bool) []any {
	panic(notFaked("Paginate"))
}

func (o *Own) Graphql(query string, variables *pyjson.Object) *pyjson.Object {
	panic(notFaked("Graphql"))
}

func notFaked(method string) *PyError {
	return &PyError{Type: "AttributeError", Msg: fmt.Sprintf("FakeGitHub: %s not faked", method)}
}

// ---------------------------------------------------------------------------- Python's semantics

func githubError(msg string, status int) error { return &herr.GitHubError{Msg: msg, Status: status} }

// one is the options struct a method was given, or the zero one (every default).
func one[T any](given []T, method string) T {
	var zero T
	switch len(given) {
	case 0:
		return zero
	case 1:
		return given[0]
	}
	panic(&PyError{Type: "TypeError", Msg: fmt.Sprintf("FakeGitHub.%s takes one options struct, got %d", method, len(given))})
}

func orDefault(s, def string) string {
	if s == "" {
		return def
	}
	return s
}

// userOr is `user or OPERATOR`.
func userOr(user *pyjson.Object) *pyjson.Object {
	if user.Len() == 0 {
		return Operator
	}
	return user
}

// item is d[key]: a KeyError when the key is absent.
func item(o *pyjson.Object, key string) any {
	v, ok := o.Lookup(key)
	if !ok {
		panic(keyError(key))
	}
	return v
}

// isNone is `v is None`.
func isNone(v any) bool {
	o, isObject := v.(*pyjson.Object)
	return v == nil || isObject && o == nil
}

// head is xs[:n].
func head[T any](xs []T, n int) []T {
	if n < 0 {
		n = max(len(xs)+n, 0)
	}
	n = min(n, len(xs))
	return xs[:n:n]
}

func deep(o *pyjson.Object) *pyjson.Object {
	c, _ := pyjson.DeepCopy(o).(*pyjson.Object)
	return c
}

func deepList(xs []*pyjson.Object) []*pyjson.Object {
	out := make([]*pyjson.Object, 0, len(xs))
	for _, x := range xs {
		out = append(out, deep(x))
	}
	return out
}

// runs is runs.values(), in order.
func runs(d *pyjson.Object) []*pyjson.Object {
	out := []*pyjson.Object{}
	d.Items(func(_ string, v any) bool {
		out = append(out, pyjson.Obj(v))
		return true
	})
	return out
}

// nextNumber is max(self.threads, default=0) + 1.
func (c *FakeGitHub) nextNumber() int {
	numbers := c.Threads.Keys()
	if len(numbers) == 0 {
		return 1
	}
	return slices.Max(numbers) + 1
}

// loadsObject is json.loads(text) where the caller goes on to treat it as a dict.
func loadsObject(text string) *pyjson.Object {
	v, err := pyjson.LoadsString(text)
	if err != nil {
		panic(&PyError{Type: "JSONDecodeError", Msg: err.Error()})
	}
	o, ok := v.(*pyjson.Object)
	if !ok {
		panic(&PyError{Type: "AttributeError", Msg: fmt.Sprintf("'%T' object has no attribute 'setdefault'", v)})
	}
	return o
}

// pyInt is int(v).
func pyInt(v any) int {
	if b, ok := v.(bool); ok {
		if b {
			return 1
		}
		return 0
	}
	if n, ok := pyjson.AsInt(v); ok {
		return n
	}
	if s, ok := v.(string); ok {
		if n, err := strconv.Atoi(strings.TrimSpace(s)); err == nil {
			return n
		}
		panic(&PyError{Type: "ValueError", Msg: "invalid literal for int() with base 10: " + py.Repr(s)})
	}
	panic(&PyError{Type: "TypeError", Msg: fmt.Sprintf("int() argument must be a string, a bytes-like object or a real number, not '%T'", v)})
}
