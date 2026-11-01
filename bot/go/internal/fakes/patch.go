// Patches and the gh.Client methods that check them: one field and one method for each method of
// the interface, in its order. A change to gh.Client changes them, and Own, with it.

package fakes

import "github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"

// Patches is a replacement for each gh.Client method, set on one FakeGitHub: Python's
// `gh.put_file = put` and mock.patch.object(gh, "put_file", ...). A nil one is the fake's own
// (FakeGitHub.Own).
type Patches struct {
	URL                    func(path string) string
	Request                func(method string, path string, body any, accept string, raw bool, cached bool) any
	Paginate               func(path string, params *pyjson.Object, key string, limit int, cached bool) []any
	Graphql                func(query string, variables *pyjson.Object) *pyjson.Object
	RepoInfo               func() *pyjson.Object
	Viewer                 func() *pyjson.Object
	GetIssue               func(number int) *pyjson.Object
	ListIssues             func(labels string, state string, limit int, assignee string) []*pyjson.Object
	ListRepoComments       func(since string, limit int) []*pyjson.Object
	ListRepoReviewComments func(since string, limit int) []*pyjson.Object
	Reactions              func(commentID int, reviewComment bool) []*pyjson.Object
	ListComments           func(number int, limit int) []*pyjson.Object
	CreateComment          func(number int, body string) *pyjson.Object
	UpdateComment          func(commentID int, body string) *pyjson.Object
	React                  func(commentID int, content string, reviewComment bool)
	CreateIssue            func(title string, body string, labels []string) *pyjson.Object
	SetIssueBody           func(number int, body string) *pyjson.Object
	BlockedBy              func(number int) []*pyjson.Object
	Blocking               func(number int) []*pyjson.Object
	AddBlockedBy           func(number int, blockerID int)
	AddSubIssue            func(parent int, childID int)
	ListSubIssues          func(parent int) []*pyjson.Object
	UpdateIssue            func(number int, fields *pyjson.Object) *pyjson.Object
	AddLabels              func(number int, names []string)
	AddAssignees           func(number int, logins []string)
	RemoveAssignees        func(number int, logins []string)
	RemoveLabel            func(number int, name string)
	Lock                   func(number int)
	Unlock                 func(number int)
	ListLabels             func() []*pyjson.Object
	ListIssueTypes         func() []*pyjson.Object
	EnsureLabel            func(name string, color string, description string) bool
	SyncLabel              func(name string, color string, description string) string
	DeleteLabel            func(name string) bool
	GetPull                func(number int) *pyjson.Object
	ListPulls              func(state string, head string, limit int) []*pyjson.Object
	CreatePull             func(title string, head string, base string, body string, draft bool) *pyjson.Object
	UpdatePull             func(number int, fields *pyjson.Object) *pyjson.Object
	ListReviews            func(number int) []*pyjson.Object
	ListReviewComments     func(number int) []*pyjson.Object
	RequestReviewers       func(number int, logins []string)
	RequiredChecks         func(branch string) map[string]bool
	EnableAutoMerge        func(nodeID string, method string, expectedHead string, headline string)
	DisableAutoMerge       func(nodeID string)
	InMergeQueue           func(nodeID string) bool
	QueueRemoval           func(nodeID string) map[string]string
	Dequeue                func(nodeID string)
	PinIssue               func(nodeID string)
	MarkReady              func(nodeID string)
	MergePull              func(number int, method string, title string) *pyjson.Object
	GetFile                func(path string, ref string) (string, string)
	PutFile                func(path string, text string, branch string, sha string, message string)
	ListDir                func(path string, ref string) [][2]string
	DeleteFile             func(path string, branch string, sha string, message string)
	BranchSHA              func(branch string) string
	CreateOrphanBranch     func(branch string, path string, text string, message string, extra map[string]string)
	CreateBranch           func(branch string, sha string)
	DeleteBranch           func(branch string)
	DispatchWorkflow       func(workflow string, ref string, inputs map[string]string)
	GetRun                 func(runID any) *pyjson.Object
	ListRuns               func(workflow string, status string, limit int, cached bool) []*pyjson.Object
	GetGist                func(gistID string, cached bool) *pyjson.Object
	ListRepoRuns           func(status string, limit int, cached bool) []*pyjson.Object
	ListCommits            func(author string, limit int) []*pyjson.Object
	RunsForSHA             func(sha string, limit int) []*pyjson.Object
	RerunFailedJobs        func(runID any)
	ListJobs               func(runID any, cached bool) []*pyjson.Object
	JobLog                 func(jobID any, tailChars int) string
	ListArtifacts          func(runID any) []*pyjson.Object
	DownloadArtifact       func(artifactID any) []byte
	CheckRuns              func(sha string) []*pyjson.Object
	UpdateRepo             func(fields *pyjson.Object) *pyjson.Object
	SetProtection          func(branch string, checks []string)
}

func (c *FakeGitHub) URL(path string) string {
	if c.Patch.URL != nil {
		return c.Patch.URL(path)
	}
	return c.Own().URL(path)
}

func (c *FakeGitHub) Request(method string, path string, body any, accept string, raw bool, cached bool) any {
	if c.Patch.Request != nil {
		return c.Patch.Request(method, path, body, accept, raw, cached)
	}
	return c.Own().Request(method, path, body, accept, raw, cached)
}

func (c *FakeGitHub) Paginate(path string, params *pyjson.Object, key string, limit int, cached bool) []any {
	if c.Patch.Paginate != nil {
		return c.Patch.Paginate(path, params, key, limit, cached)
	}
	return c.Own().Paginate(path, params, key, limit, cached)
}

func (c *FakeGitHub) Graphql(query string, variables *pyjson.Object) *pyjson.Object {
	if c.Patch.Graphql != nil {
		return c.Patch.Graphql(query, variables)
	}
	return c.Own().Graphql(query, variables)
}

func (c *FakeGitHub) RepoInfo() *pyjson.Object {
	if c.Patch.RepoInfo != nil {
		return c.Patch.RepoInfo()
	}
	return c.Own().RepoInfo()
}

func (c *FakeGitHub) Viewer() *pyjson.Object {
	if c.Patch.Viewer != nil {
		return c.Patch.Viewer()
	}
	return c.Own().Viewer()
}

func (c *FakeGitHub) GetIssue(number int) *pyjson.Object {
	if c.Patch.GetIssue != nil {
		return c.Patch.GetIssue(number)
	}
	return c.Own().GetIssue(number)
}

func (c *FakeGitHub) ListIssues(labels string, state string, limit int, assignee string) []*pyjson.Object {
	if c.Patch.ListIssues != nil {
		return c.Patch.ListIssues(labels, state, limit, assignee)
	}
	return c.Own().ListIssues(labels, state, limit, assignee)
}

func (c *FakeGitHub) ListRepoComments(since string, limit int) []*pyjson.Object {
	if c.Patch.ListRepoComments != nil {
		return c.Patch.ListRepoComments(since, limit)
	}
	return c.Own().ListRepoComments(since, limit)
}

func (c *FakeGitHub) ListRepoReviewComments(since string, limit int) []*pyjson.Object {
	if c.Patch.ListRepoReviewComments != nil {
		return c.Patch.ListRepoReviewComments(since, limit)
	}
	return c.Own().ListRepoReviewComments(since, limit)
}

func (c *FakeGitHub) Reactions(commentID int, reviewComment bool) []*pyjson.Object {
	if c.Patch.Reactions != nil {
		return c.Patch.Reactions(commentID, reviewComment)
	}
	return c.Own().Reactions(commentID, reviewComment)
}

func (c *FakeGitHub) ListComments(number int, limit int) []*pyjson.Object {
	if c.Patch.ListComments != nil {
		return c.Patch.ListComments(number, limit)
	}
	return c.Own().ListComments(number, limit)
}

func (c *FakeGitHub) CreateComment(number int, body string) *pyjson.Object {
	if c.Patch.CreateComment != nil {
		return c.Patch.CreateComment(number, body)
	}
	return c.Own().CreateComment(number, body)
}

func (c *FakeGitHub) UpdateComment(commentID int, body string) *pyjson.Object {
	if c.Patch.UpdateComment != nil {
		return c.Patch.UpdateComment(commentID, body)
	}
	return c.Own().UpdateComment(commentID, body)
}

func (c *FakeGitHub) React(commentID int, content string, reviewComment bool) {
	if c.Patch.React != nil {
		c.Patch.React(commentID, content, reviewComment)
		return
	}
	c.Own().React(commentID, content, reviewComment)
}

func (c *FakeGitHub) CreateIssue(title string, body string, labels []string) *pyjson.Object {
	if c.Patch.CreateIssue != nil {
		return c.Patch.CreateIssue(title, body, labels)
	}
	return c.Own().CreateIssue(title, body, labels)
}

func (c *FakeGitHub) SetIssueBody(number int, body string) *pyjson.Object {
	if c.Patch.SetIssueBody != nil {
		return c.Patch.SetIssueBody(number, body)
	}
	return c.Own().SetIssueBody(number, body)
}

func (c *FakeGitHub) BlockedBy(number int) []*pyjson.Object {
	if c.Patch.BlockedBy != nil {
		return c.Patch.BlockedBy(number)
	}
	return c.Own().BlockedBy(number)
}

func (c *FakeGitHub) Blocking(number int) []*pyjson.Object {
	if c.Patch.Blocking != nil {
		return c.Patch.Blocking(number)
	}
	return c.Own().Blocking(number)
}

func (c *FakeGitHub) AddBlockedBy(number int, blockerID int) {
	if c.Patch.AddBlockedBy != nil {
		c.Patch.AddBlockedBy(number, blockerID)
		return
	}
	c.Own().AddBlockedBy(number, blockerID)
}

func (c *FakeGitHub) AddSubIssue(parent int, childID int) {
	if c.Patch.AddSubIssue != nil {
		c.Patch.AddSubIssue(parent, childID)
		return
	}
	c.Own().AddSubIssue(parent, childID)
}

func (c *FakeGitHub) ListSubIssues(parent int) []*pyjson.Object {
	if c.Patch.ListSubIssues != nil {
		return c.Patch.ListSubIssues(parent)
	}
	return c.Own().ListSubIssues(parent)
}

func (c *FakeGitHub) UpdateIssue(number int, fields *pyjson.Object) *pyjson.Object {
	if c.Patch.UpdateIssue != nil {
		return c.Patch.UpdateIssue(number, fields)
	}
	return c.Own().UpdateIssue(number, fields)
}

func (c *FakeGitHub) AddLabels(number int, names []string) {
	if c.Patch.AddLabels != nil {
		c.Patch.AddLabels(number, names)
		return
	}
	c.Own().AddLabels(number, names)
}

func (c *FakeGitHub) AddAssignees(number int, logins []string) {
	if c.Patch.AddAssignees != nil {
		c.Patch.AddAssignees(number, logins)
		return
	}
	c.Own().AddAssignees(number, logins)
}

func (c *FakeGitHub) RemoveAssignees(number int, logins []string) {
	if c.Patch.RemoveAssignees != nil {
		c.Patch.RemoveAssignees(number, logins)
		return
	}
	c.Own().RemoveAssignees(number, logins)
}

func (c *FakeGitHub) RemoveLabel(number int, name string) {
	if c.Patch.RemoveLabel != nil {
		c.Patch.RemoveLabel(number, name)
		return
	}
	c.Own().RemoveLabel(number, name)
}

func (c *FakeGitHub) Lock(number int) {
	if c.Patch.Lock != nil {
		c.Patch.Lock(number)
		return
	}
	c.Own().Lock(number)
}

func (c *FakeGitHub) Unlock(number int) {
	if c.Patch.Unlock != nil {
		c.Patch.Unlock(number)
		return
	}
	c.Own().Unlock(number)
}

func (c *FakeGitHub) ListLabels() []*pyjson.Object {
	if c.Patch.ListLabels != nil {
		return c.Patch.ListLabels()
	}
	return c.Own().ListLabels()
}

func (c *FakeGitHub) ListIssueTypes() []*pyjson.Object {
	if c.Patch.ListIssueTypes != nil {
		return c.Patch.ListIssueTypes()
	}
	return c.Own().ListIssueTypes()
}

func (c *FakeGitHub) EnsureLabel(name string, color string, description string) bool {
	if c.Patch.EnsureLabel != nil {
		return c.Patch.EnsureLabel(name, color, description)
	}
	return c.Own().EnsureLabel(name, color, description)
}

func (c *FakeGitHub) SyncLabel(name string, color string, description string) string {
	if c.Patch.SyncLabel != nil {
		return c.Patch.SyncLabel(name, color, description)
	}
	return c.Own().SyncLabel(name, color, description)
}

func (c *FakeGitHub) DeleteLabel(name string) bool {
	if c.Patch.DeleteLabel != nil {
		return c.Patch.DeleteLabel(name)
	}
	return c.Own().DeleteLabel(name)
}

func (c *FakeGitHub) GetPull(number int) *pyjson.Object {
	if c.Patch.GetPull != nil {
		return c.Patch.GetPull(number)
	}
	return c.Own().GetPull(number)
}

func (c *FakeGitHub) ListPulls(state string, head string, limit int) []*pyjson.Object {
	if c.Patch.ListPulls != nil {
		return c.Patch.ListPulls(state, head, limit)
	}
	return c.Own().ListPulls(state, head, limit)
}

func (c *FakeGitHub) CreatePull(title string, head string, base string, body string, draft bool) *pyjson.Object {
	if c.Patch.CreatePull != nil {
		return c.Patch.CreatePull(title, head, base, body, draft)
	}
	return c.Own().CreatePull(title, head, base, body, draft)
}

func (c *FakeGitHub) UpdatePull(number int, fields *pyjson.Object) *pyjson.Object {
	if c.Patch.UpdatePull != nil {
		return c.Patch.UpdatePull(number, fields)
	}
	return c.Own().UpdatePull(number, fields)
}

func (c *FakeGitHub) ListReviews(number int) []*pyjson.Object {
	if c.Patch.ListReviews != nil {
		return c.Patch.ListReviews(number)
	}
	return c.Own().ListReviews(number)
}

func (c *FakeGitHub) ListReviewComments(number int) []*pyjson.Object {
	if c.Patch.ListReviewComments != nil {
		return c.Patch.ListReviewComments(number)
	}
	return c.Own().ListReviewComments(number)
}

func (c *FakeGitHub) RequestReviewers(number int, logins []string) {
	if c.Patch.RequestReviewers != nil {
		c.Patch.RequestReviewers(number, logins)
		return
	}
	c.Own().RequestReviewers(number, logins)
}

func (c *FakeGitHub) RequiredChecks(branch string) map[string]bool {
	if c.Patch.RequiredChecks != nil {
		return c.Patch.RequiredChecks(branch)
	}
	return c.Own().RequiredChecks(branch)
}

func (c *FakeGitHub) EnableAutoMerge(nodeID string, method string, expectedHead string, headline string) {
	if c.Patch.EnableAutoMerge != nil {
		c.Patch.EnableAutoMerge(nodeID, method, expectedHead, headline)
		return
	}
	c.Own().EnableAutoMerge(nodeID, method, expectedHead, headline)
}

func (c *FakeGitHub) DisableAutoMerge(nodeID string) {
	if c.Patch.DisableAutoMerge != nil {
		c.Patch.DisableAutoMerge(nodeID)
		return
	}
	c.Own().DisableAutoMerge(nodeID)
}

func (c *FakeGitHub) InMergeQueue(nodeID string) bool {
	if c.Patch.InMergeQueue != nil {
		return c.Patch.InMergeQueue(nodeID)
	}
	return c.Own().InMergeQueue(nodeID)
}

func (c *FakeGitHub) QueueRemoval(nodeID string) map[string]string {
	if c.Patch.QueueRemoval != nil {
		return c.Patch.QueueRemoval(nodeID)
	}
	return c.Own().QueueRemoval(nodeID)
}

func (c *FakeGitHub) Dequeue(nodeID string) {
	if c.Patch.Dequeue != nil {
		c.Patch.Dequeue(nodeID)
		return
	}
	c.Own().Dequeue(nodeID)
}

func (c *FakeGitHub) PinIssue(nodeID string) {
	if c.Patch.PinIssue != nil {
		c.Patch.PinIssue(nodeID)
		return
	}
	c.Own().PinIssue(nodeID)
}

func (c *FakeGitHub) MarkReady(nodeID string) {
	if c.Patch.MarkReady != nil {
		c.Patch.MarkReady(nodeID)
		return
	}
	c.Own().MarkReady(nodeID)
}

func (c *FakeGitHub) MergePull(number int, method string, title string) *pyjson.Object {
	if c.Patch.MergePull != nil {
		return c.Patch.MergePull(number, method, title)
	}
	return c.Own().MergePull(number, method, title)
}

func (c *FakeGitHub) GetFile(path string, ref string) (string, string) {
	if c.Patch.GetFile != nil {
		return c.Patch.GetFile(path, ref)
	}
	return c.Own().GetFile(path, ref)
}

func (c *FakeGitHub) PutFile(path string, text string, branch string, sha string, message string) {
	if c.Patch.PutFile != nil {
		c.Patch.PutFile(path, text, branch, sha, message)
		return
	}
	c.Own().PutFile(path, text, branch, sha, message)
}

func (c *FakeGitHub) ListDir(path string, ref string) [][2]string {
	if c.Patch.ListDir != nil {
		return c.Patch.ListDir(path, ref)
	}
	return c.Own().ListDir(path, ref)
}

func (c *FakeGitHub) DeleteFile(path string, branch string, sha string, message string) {
	if c.Patch.DeleteFile != nil {
		c.Patch.DeleteFile(path, branch, sha, message)
		return
	}
	c.Own().DeleteFile(path, branch, sha, message)
}

func (c *FakeGitHub) BranchSHA(branch string) string {
	if c.Patch.BranchSHA != nil {
		return c.Patch.BranchSHA(branch)
	}
	return c.Own().BranchSHA(branch)
}

func (c *FakeGitHub) CreateOrphanBranch(branch string, path string, text string, message string, extra map[string]string) {
	if c.Patch.CreateOrphanBranch != nil {
		c.Patch.CreateOrphanBranch(branch, path, text, message, extra)
		return
	}
	c.Own().CreateOrphanBranch(branch, path, text, message, extra)
}

func (c *FakeGitHub) CreateBranch(branch string, sha string) {
	if c.Patch.CreateBranch != nil {
		c.Patch.CreateBranch(branch, sha)
		return
	}
	c.Own().CreateBranch(branch, sha)
}

func (c *FakeGitHub) DeleteBranch(branch string) {
	if c.Patch.DeleteBranch != nil {
		c.Patch.DeleteBranch(branch)
		return
	}
	c.Own().DeleteBranch(branch)
}

func (c *FakeGitHub) DispatchWorkflow(workflow string, ref string, inputs map[string]string) {
	if c.Patch.DispatchWorkflow != nil {
		c.Patch.DispatchWorkflow(workflow, ref, inputs)
		return
	}
	c.Own().DispatchWorkflow(workflow, ref, inputs)
}

func (c *FakeGitHub) GetRun(runID any) *pyjson.Object {
	if c.Patch.GetRun != nil {
		return c.Patch.GetRun(runID)
	}
	return c.Own().GetRun(runID)
}

func (c *FakeGitHub) ListRuns(workflow string, status string, limit int, cached bool) []*pyjson.Object {
	if c.Patch.ListRuns != nil {
		return c.Patch.ListRuns(workflow, status, limit, cached)
	}
	return c.Own().ListRuns(workflow, status, limit, cached)
}

func (c *FakeGitHub) GetGist(gistID string, cached bool) *pyjson.Object {
	if c.Patch.GetGist != nil {
		return c.Patch.GetGist(gistID, cached)
	}
	return c.Own().GetGist(gistID, cached)
}

func (c *FakeGitHub) ListRepoRuns(status string, limit int, cached bool) []*pyjson.Object {
	if c.Patch.ListRepoRuns != nil {
		return c.Patch.ListRepoRuns(status, limit, cached)
	}
	return c.Own().ListRepoRuns(status, limit, cached)
}

func (c *FakeGitHub) ListCommits(author string, limit int) []*pyjson.Object {
	if c.Patch.ListCommits != nil {
		return c.Patch.ListCommits(author, limit)
	}
	return c.Own().ListCommits(author, limit)
}

func (c *FakeGitHub) RunsForSHA(sha string, limit int) []*pyjson.Object {
	if c.Patch.RunsForSHA != nil {
		return c.Patch.RunsForSHA(sha, limit)
	}
	return c.Own().RunsForSHA(sha, limit)
}

func (c *FakeGitHub) RerunFailedJobs(runID any) {
	if c.Patch.RerunFailedJobs != nil {
		c.Patch.RerunFailedJobs(runID)
		return
	}
	c.Own().RerunFailedJobs(runID)
}

func (c *FakeGitHub) ListJobs(runID any, cached bool) []*pyjson.Object {
	if c.Patch.ListJobs != nil {
		return c.Patch.ListJobs(runID, cached)
	}
	return c.Own().ListJobs(runID, cached)
}

func (c *FakeGitHub) JobLog(jobID any, tailChars int) string {
	if c.Patch.JobLog != nil {
		return c.Patch.JobLog(jobID, tailChars)
	}
	return c.Own().JobLog(jobID, tailChars)
}

func (c *FakeGitHub) ListArtifacts(runID any) []*pyjson.Object {
	if c.Patch.ListArtifacts != nil {
		return c.Patch.ListArtifacts(runID)
	}
	return c.Own().ListArtifacts(runID)
}

func (c *FakeGitHub) DownloadArtifact(artifactID any) []byte {
	if c.Patch.DownloadArtifact != nil {
		return c.Patch.DownloadArtifact(artifactID)
	}
	return c.Own().DownloadArtifact(artifactID)
}

func (c *FakeGitHub) CheckRuns(sha string) []*pyjson.Object {
	if c.Patch.CheckRuns != nil {
		return c.Patch.CheckRuns(sha)
	}
	return c.Own().CheckRuns(sha)
}

func (c *FakeGitHub) UpdateRepo(fields *pyjson.Object) *pyjson.Object {
	if c.Patch.UpdateRepo != nil {
		return c.Patch.UpdateRepo(fields)
	}
	return c.Own().UpdateRepo(fields)
}

func (c *FakeGitHub) SetProtection(branch string, checks []string) {
	if c.Patch.SetProtection != nil {
		c.Patch.SetProtection(branch, checks)
		return
	}
	c.Own().SetProtection(branch, checks)
}
