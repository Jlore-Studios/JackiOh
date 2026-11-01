package gh

import "github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"

// Client is what the harness calls on GitHub: the real client (GitHub) and the tests'
// fake both implement it (Python's duck typing of `ctx.gh`).
// Its methods are GitHub's exported ones; the check at the end holds GitHub to it.
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

var _ Client = (*GitHub)(nil)
