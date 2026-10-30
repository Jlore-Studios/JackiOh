// Package gh is bot/harness/gh.py.
//
// The GitHub client. The only module that sends a token or writes to GitHub.
//
// Every body the harness posts is redacted and carries `config.MARKER`. In dry-run mode every
// write is recorded in `writes` and answered with a stand-in, and nothing is sent.
package gh

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// API = "https://api.github.com"
var API string = "https://api.github.com"

// USER_AGENT = f"jackioh-night-bot/{__version__}"
var UserAgent any // unported

// RETRIES = 3
var Retries int = 3

// BACKOFF_SECONDS = 2.0
var BackoffSeconds float64 = 2.0

// _CACHED: dict[tuple[str, str], tuple[str, str]] = {}
var Cached *pyjson.Object // unported

// CACHED_KEEP = 200
var CachedKeep int = 200

// `body`, redacted, with the marker once at the end.

func WithMarker(body string) string {
	panic("unported: gh.with_marker")
}

// A small REST and GraphQL client for one repository.

type GitHub struct {
	Repo          any              // repo
	Token         any              // token
	FallbackToken any              // fallback_token
	DryRun        any              // dry_run
	Open          any              // opener or urllib.request.urlopen
	Sleep         any              // sleep
	Writes        []*pyjson.Object // []
}

// NewGitHub is GitHub(...).

func NewGitHub(repo string, token string, dryRun bool /* = False */, opener func(...any) any /* = None */, sleep func(float64) /* = time.sleep */, fallbackToken string /* = "" */) *GitHub {
	panic("unported: gh.GitHub.__init__")
}

func (x *GitHub) headers(accept string /* = None */) map[string]string {
	panic("unported: gh.GitHub._headers")
}

func (x *GitHub) URL(path string) string {
	panic("unported: gh.GitHub._url")
}

// One request, retried on 5xx, connection errors and secondary rate limits. A `cached`
// GET is conditional: its last answer is kept, and given again when GitHub says it has
// not changed (`_CACHED`).

func (x *GitHub) Request(method string, path string, body any /* = None */, accept string /* = None */, raw bool /* = False */, cached bool /* = False */) any {
	panic("unported: gh.GitHub.request")
}

// Every item of a paged list, up to `limit` (each page a conditional GET if `cached`).

func (x *GitHub) Paginate(path string, params *pyjson.Object /* = None */, key string /* = None */, limit int /* = 1000 */, cached bool /* = False */) []any {
	panic("unported: gh.GitHub.paginate")
}

func (x *GitHub) Graphql(query string, variables *pyjson.Object) *pyjson.Object {
	panic("unported: gh.GitHub.graphql")
}

func (x *GitHub) r() string {
	panic("unported: gh.GitHub._r")
}

func (x *GitHub) RepoInfo() *pyjson.Object {
	panic("unported: gh.GitHub.repo_info")
}

func (x *GitHub) Viewer() *pyjson.Object {
	panic("unported: gh.GitHub.viewer")
}

func (x *GitHub) GetIssue(number int) *pyjson.Object {
	panic("unported: gh.GitHub.get_issue")
}

func (x *GitHub) ListIssues(labels string /* = "" */, state string /* = "open" */, limit int /* = 300 */, assignee string /* = "" */) []*pyjson.Object {
	panic("unported: gh.GitHub.list_issues")
}

// Every issue and pull request conversation comment updated since `since`, newest
// first, so a limit drops the oldest.

func (x *GitHub) ListRepoComments(since string, limit int /* = 500 */) []*pyjson.Object {
	panic("unported: gh.GitHub.list_repo_comments")
}

// Every line comment on a pull request's diff updated since `since`, newest first.

func (x *GitHub) ListRepoReviewComments(since string, limit int /* = 500 */) []*pyjson.Object {
	panic("unported: gh.GitHub.list_repo_review_comments")
}

func (x *GitHub) Reactions(commentID int, reviewComment bool /* = False */) []*pyjson.Object {
	panic("unported: gh.GitHub.reactions")
}

func (x *GitHub) ListComments(number int, limit int /* = 300 */) []*pyjson.Object {
	panic("unported: gh.GitHub.list_comments")
}

func (x *GitHub) CreateComment(number int, body string) *pyjson.Object {
	panic("unported: gh.GitHub.create_comment")
}

func (x *GitHub) UpdateComment(commentID int, body string) *pyjson.Object {
	panic("unported: gh.GitHub.update_comment")
}

func (x *GitHub) React(commentID int, content string, reviewComment bool /* = False */) {
	panic("unported: gh.GitHub.react")
}

func (x *GitHub) CreateIssue(title string, body string, labels []string /* = () */) *pyjson.Object {
	panic("unported: gh.GitHub.create_issue")
}

// Replace an issue's description as it is: no redaction of a person's text and no bot
// marker, which would make the whole description read as the bot's (the plan section,
// `issueplan.py`, is redacted before it gets here).

func (x *GitHub) SetIssueBody(number int, body string) *pyjson.Object {
	panic("unported: gh.GitHub.set_issue_body")
}

// The issues GitHub's own issue dependencies say block this one, open or closed.

func (x *GitHub) BlockedBy(number int) []*pyjson.Object {
	panic("unported: gh.GitHub.blocked_by")
}

// The issues GitHub's own issue dependencies say this one blocks, open or closed.

func (x *GitHub) Blocking(number int) []*pyjson.Object {
	panic("unported: gh.GitHub.blocking")
}

// Mark issue `number` blocked by the issue whose id (not number) is `blocker_id`.

func (x *GitHub) AddBlockedBy(number int, blockerID int) {
	panic("unported: gh.GitHub.add_blocked_by")
}

// Make the issue whose id (not number) is `child_id` a sub-issue of issue `parent`.

func (x *GitHub) AddSubIssue(parent int, childID int) {
	panic("unported: gh.GitHub.add_sub_issue")
}

// The sub-issues of issue `parent`, open and closed (#60: a tree the bot split).

func (x *GitHub) ListSubIssues(parent int) []*pyjson.Object {
	panic("unported: gh.GitHub.list_sub_issues")
}

func (x *GitHub) UpdateIssue(number int, fields *pyjson.Object) *pyjson.Object {
	panic("unported: gh.GitHub.update_issue")
}

func (x *GitHub) AddLabels(number int, names []string) {
	panic("unported: gh.GitHub.add_labels")
}

// Adds to whoever is assigned; GitHub skips an account it cannot assign.

func (x *GitHub) AddAssignees(number int, logins []string) {
	panic("unported: gh.GitHub.add_assignees")
}

func (x *GitHub) RemoveAssignees(number int, logins []string) {
	panic("unported: gh.GitHub.remove_assignees")
}

func (x *GitHub) RemoveLabel(number int, name string) {
	panic("unported: gh.GitHub.remove_label")
}

// Lock an issue's or pull request's conversation: only collaborators may comment
// (`.github/workflows/lock-conversations.yml`). The empty body sends a Content-Length.

func (x *GitHub) Lock(number int) {
	panic("unported: gh.GitHub.lock")
}

func (x *GitHub) Unlock(number int) {
	panic("unported: gh.GitHub.unlock")
}

func (x *GitHub) ListLabels() []*pyjson.Object {
	panic("unported: gh.GitHub.list_labels")
}

// The owning organisation's issue types (none for a user's repository).

func (x *GitHub) ListIssueTypes() []*pyjson.Object {
	panic("unported: gh.GitHub.list_issue_types")
}

// Create the label unless it exists. True when it was created.

func (x *GitHub) EnsureLabel(name string, color string, description string) bool {
	panic("unported: gh.GitHub.ensure_label")
}

// Create the label, or bring its colour and description to these (`harness setup`,
// #187): "created", "updated", or "" when it matched already.

func (x *GitHub) SyncLabel(name string, color string, description string) string {
	panic("unported: gh.GitHub.sync_label")
}

// Delete the label from the repository, and so from every thread. True when it was
// there.

func (x *GitHub) DeleteLabel(name string) bool {
	panic("unported: gh.GitHub.delete_label")
}

func (x *GitHub) GetPull(number int) *pyjson.Object {
	panic("unported: gh.GitHub.get_pull")
}

func (x *GitHub) ListPulls(state string /* = "open" */, head string /* = "" */, limit int /* = 200 */) []*pyjson.Object {
	panic("unported: gh.GitHub.list_pulls")
}

func (x *GitHub) CreatePull(title string, head string, base string, body string, draft bool /* = False */) *pyjson.Object {
	panic("unported: gh.GitHub.create_pull")
}

func (x *GitHub) UpdatePull(number int, fields *pyjson.Object) *pyjson.Object {
	panic("unported: gh.GitHub.update_pull")
}

func (x *GitHub) ListReviews(number int) []*pyjson.Object {
	panic("unported: gh.GitHub.list_reviews")
}

func (x *GitHub) ListReviewComments(number int) []*pyjson.Object {
	panic("unported: gh.GitHub.list_review_comments")
}

func (x *GitHub) RequestReviewers(number int, logins []string) {
	panic("unported: gh.GitHub.request_reviewers")
}

// The status checks branch protection requires on `branch`, or None when unprotected.
//
// Read through the branch endpoint, which any reader may call, rather than the protection
// endpoint, which needs admin rights the bot account does not have.

func (x *GitHub) RequiredChecks(branch string) map[string]bool {
	panic("unported: gh.GitHub.required_checks")
}

// Auto-merge, pinned to `expected_head` when given: GitHub then refuses to merge a head
// that moved after the approval it was turned on for. `headline` is the merge commit's
// title; without it GitHub squashes a one-commit pull request under that commit's message.

func (x *GitHub) EnableAutoMerge(nodeID string, method string, expectedHead string /* = "" */, headline string /* = "" */) {
	panic("unported: gh.GitHub.enable_auto_merge")
}

func (x *GitHub) DisableAutoMerge(nodeID string) {
	panic("unported: gh.GitHub.disable_auto_merge")
}

// Whether a pull request is in `main`'s merge queue now. REST has no field for it, and its
// `auto_merge` reads null for a queued pull request.

func (x *GitHub) InMergeQueue(nodeID string) bool {
	panic("unported: gh.GitHub.in_merge_queue")
}

// How the merge queue took a pull request out, when that is the last thing auto-merge or
// the queue did to it (`{"actor", "type", "reason"}`: the remover's login and GraphQL type,
// `User` or `Bot`), else None: still queued, never queued, or auto-merge turned on or off
// since.

func (x *GitHub) QueueRemoval(nodeID string) map[string]string {
	panic("unported: gh.GitHub.queue_removal")
}

// Take a pull request out of `main`'s merge queue (infra/github.tf). GitHub answers with
// an error when it is not in the queue.

func (x *GitHub) Dequeue(nodeID string) {
	panic("unported: gh.GitHub.dequeue")
}

// Pin an issue to the top of the issue list (at most three are pinned).

func (x *GitHub) PinIssue(nodeID string) {
	panic("unported: gh.GitHub.pin_issue")
}

func (x *GitHub) MarkReady(nodeID string) {
	panic("unported: gh.GitHub.mark_ready")
}

func (x *GitHub) MergePull(number int, method string, title string /* = "" */) *pyjson.Object {
	panic("unported: gh.GitHub.merge_pull")
}

// `(text, blob sha)` of a file on `ref`, or `(None, None)` when it does not exist.

func (x *GitHub) GetFile(path string, ref string) (string, string) {
	panic("unported: gh.GitHub.get_file")
}

// Create or replace a file. A stale `sha` raises GitHubError with status 409 or 422.

func (x *GitHub) PutFile(path string, text string, branch string, sha string, message string) {
	panic("unported: gh.GitHub.put_file")
}

// `(name, blob sha)` of each file in a directory on `ref`; empty when there is none.

func (x *GitHub) ListDir(path string, ref string) [][2]string {
	panic("unported: gh.GitHub.list_dir")
}

func (x *GitHub) DeleteFile(path string, branch string, sha string, message string) {
	panic("unported: gh.GitHub.delete_file")
}

func (x *GitHub) BranchSHA(branch string) string {
	panic("unported: gh.GitHub.branch_sha")
}

// A new branch whose only commit holds one file, and any `extra` ones.

func (x *GitHub) CreateOrphanBranch(branch string, path string, text string, message string, extra map[string]string /* = None */) {
	panic("unported: gh.GitHub.create_orphan_branch")
}

// A branch at `sha` (an existing commit).

func (x *GitHub) CreateBranch(branch string, sha string) {
	panic("unported: gh.GitHub.create_branch")
}

func (x *GitHub) DeleteBranch(branch string) {
	panic("unported: gh.GitHub.delete_branch")
}

func (x *GitHub) DispatchWorkflow(workflow string, ref string, inputs map[string]string) {
	panic("unported: gh.GitHub.dispatch_workflow")
}

func (x *GitHub) GetRun(runID any) *pyjson.Object {
	panic("unported: gh.GitHub.get_run")
}

func (x *GitHub) ListRuns(workflow string, status string /* = "" */, limit int /* = 30 */, cached bool /* = False */) []*pyjson.Object {
	panic("unported: gh.GitHub.list_runs")
}

// A gist, by id (the owner's PC's heartbeat, `dashboard.PC_FILE`).

func (x *GitHub) GetGist(gistID string, cached bool /* = False */) *pyjson.Object {
	panic("unported: gh.GitHub.get_gist")
}

// The runs of every workflow with `status` (`in_progress`, `queued`), newest first.

func (x *GitHub) ListRepoRuns(status string, limit int /* = 100 */, cached bool /* = False */) []*pyjson.Object {
	panic("unported: gh.GitHub.list_repo_runs")
}

// Commits on the default branch, newest first, by `author` when given.

func (x *GitHub) ListCommits(author string /* = "" */, limit int /* = 1000 */) []*pyjson.Object {
	panic("unported: gh.GitHub.list_commits")
}

func (x *GitHub) RunsForSHA(sha string, limit int /* = 30 */) []*pyjson.Object {
	panic("unported: gh.GitHub.runs_for_sha")
}

func (x *GitHub) RerunFailedJobs(runID any) {
	panic("unported: gh.GitHub.rerun_failed_jobs")
}

func (x *GitHub) ListJobs(runID any, cached bool /* = False */) []*pyjson.Object {
	panic("unported: gh.GitHub.list_jobs")
}

// The last `tail_chars` of a job's log. The redirect is followed without the token.

func (x *GitHub) JobLog(jobID any, tailChars int /* = 6000 */) string {
	panic("unported: gh.GitHub.job_log")
}

func (x *GitHub) ListArtifacts(runID any) []*pyjson.Object {
	panic("unported: gh.GitHub.list_artifacts")
}

// An artifact's zip. GitHub answers with a redirect to its storage, which is followed
// without the token, as `job_log`'s is.

func (x *GitHub) DownloadArtifact(artifactID any) []byte {
	panic("unported: gh.GitHub.download_artifact")
}

func (x *GitHub) CheckRuns(sha string) []*pyjson.Object {
	panic("unported: gh.GitHub.check_runs")
}

func (x *GitHub) UpdateRepo(fields *pyjson.Object) *pyjson.Object {
	panic("unported: gh.GitHub.update_repo")
}

func (x *GitHub) SetProtection(branch string, checks []string) {
	panic("unported: gh.GitHub.set_protection")
}

// Client is what the harness calls on GitHub: the real client (GitHub) and the tests'
// fake both implement it (Python's duck typing of `ctx.gh`).
type Client interface {
	URL(path string) string
	Request(method string, path string, body any, accept string, raw bool, cached bool) any
	Paginate(path string, params *pyjson.Object, key string, limit int, cached bool) []any
	Graphql(query string, variables *pyjson.Object) *pyjson.Object
	RepoInfo() *pyjson.Object
	Viewer() *pyjson.Object
	GetIssue(number int) *pyjson.Object
	ListIssues(labels string, state string, limit int, assignee string) []*pyjson.Object
	ListRepoComments(since string, limit int) []*pyjson.Object
	ListRepoReviewComments(since string, limit int) []*pyjson.Object
	Reactions(commentID int, reviewComment bool) []*pyjson.Object
	ListComments(number int, limit int) []*pyjson.Object
	CreateComment(number int, body string) *pyjson.Object
	UpdateComment(commentID int, body string) *pyjson.Object
	React(commentID int, content string, reviewComment bool)
	CreateIssue(title string, body string, labels []string) *pyjson.Object
	SetIssueBody(number int, body string) *pyjson.Object
	BlockedBy(number int) []*pyjson.Object
	Blocking(number int) []*pyjson.Object
	AddBlockedBy(number int, blockerID int)
	AddSubIssue(parent int, childID int)
	ListSubIssues(parent int) []*pyjson.Object
	UpdateIssue(number int, fields *pyjson.Object) *pyjson.Object
	AddLabels(number int, names []string)
	AddAssignees(number int, logins []string)
	RemoveAssignees(number int, logins []string)
	RemoveLabel(number int, name string)
	Lock(number int)
	Unlock(number int)
	ListLabels() []*pyjson.Object
	ListIssueTypes() []*pyjson.Object
	EnsureLabel(name string, color string, description string) bool
	SyncLabel(name string, color string, description string) string
	DeleteLabel(name string) bool
	GetPull(number int) *pyjson.Object
	ListPulls(state string, head string, limit int) []*pyjson.Object
	CreatePull(title string, head string, base string, body string, draft bool) *pyjson.Object
	UpdatePull(number int, fields *pyjson.Object) *pyjson.Object
	ListReviews(number int) []*pyjson.Object
	ListReviewComments(number int) []*pyjson.Object
	RequestReviewers(number int, logins []string)
	RequiredChecks(branch string) map[string]bool
	EnableAutoMerge(nodeID string, method string, expectedHead string, headline string)
	DisableAutoMerge(nodeID string)
	InMergeQueue(nodeID string) bool
	QueueRemoval(nodeID string) map[string]string
	Dequeue(nodeID string)
	PinIssue(nodeID string)
	MarkReady(nodeID string)
	MergePull(number int, method string, title string) *pyjson.Object
	GetFile(path string, ref string) (string, string)
	PutFile(path string, text string, branch string, sha string, message string)
	ListDir(path string, ref string) [][2]string
	DeleteFile(path string, branch string, sha string, message string)
	BranchSHA(branch string) string
	CreateOrphanBranch(branch string, path string, text string, message string, extra map[string]string)
	CreateBranch(branch string, sha string)
	DeleteBranch(branch string)
	DispatchWorkflow(workflow string, ref string, inputs map[string]string)
	GetRun(runID any) *pyjson.Object
	ListRuns(workflow string, status string, limit int, cached bool) []*pyjson.Object
	GetGist(gistID string, cached bool) *pyjson.Object
	ListRepoRuns(status string, limit int, cached bool) []*pyjson.Object
	ListCommits(author string, limit int) []*pyjson.Object
	RunsForSHA(sha string, limit int) []*pyjson.Object
	RerunFailedJobs(runID any)
	ListJobs(runID any, cached bool) []*pyjson.Object
	JobLog(jobID any, tailChars int) string
	ListArtifacts(runID any) []*pyjson.Object
	DownloadArtifact(artifactID any) []byte
	CheckRuns(sha string) []*pyjson.Object
	UpdateRepo(fields *pyjson.Object) *pyjson.Object
	SetProtection(branch string, checks []string)
}
