// #317 part 3: a revision cut off by a usage cap keeps its work, and pauses add up.
//
// `deliver._interrupted` published only a build's work. A revision cut off mid-call lost
// everything but its notes, so #143 had 16 Opus revision runs end "Paused: … stops mid-call" while
// its branch never moved, and a usage pause never counted towards blocking it.
// (bot/tests/test_wip.py)
package tests

import (
	"fmt"
	"os"
	"path/filepath"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
)

// wipCapped: the subscription's usage crossed its cap mid-call (`usage_stop`): a pause, no
// parking.
func wipCapped(request *runner.RunRequest) *runner.RunResult {
	result := runner.NewRunResult(false, "")
	result.ExitCode, result.DurationS, result.Error = 1, 1800, "stopped mid-call"
	result.Extra = pyjson.New("usage_stop", "5-hour usage is 91%, at or over its 90% cap")
	return result
}

// wipPull is pull(h, number=40): a bot pull request asked to revise, its branch on origin.
func wipPull(h *difficultyHarness) string {
	start := fakes.PushBranch(h.origin, h.root, "bot/issue-12", map[string]string{"src/game.txt": "v2\n"})
	fakes.Git(h.clone, "fetch", "-q", "origin")
	h.gh.AddPull(40, "bot/issue-12", fakes.Pull{Body: "Closes #12", Labels: []string{config.LabelPR,
		config.LabelRevise}, SHA: start})
	return start
}

// wipRecord is h.ctx.store.load()["items"]["40"].
func wipRecord(h *difficultyHarness) *pyjson.Object {
	return h.ctx.Store.Load().Obj("items").Obj("40")
}

// wipInterruptions is int(record.get("interruptions", 0)), or record["interruptions"] when
// `present` (a KeyError when it is not there).
func wipInterruptions(t *testing.T, record *pyjson.Object, present bool) int {
	t.Helper()
	if present && !record.Has("interruptions") {
		t.Fatalf("KeyError: 'interruptions' in %s", pyjson.Dumps(record))
	}
	n, ok := pyjson.AsInt(record.GetOr("interruptions", 0))
	if !ok {
		t.Fatalf("interruptions is not a number: %v", record.Get("interruptions"))
	}
	return n
}

// wipRunner is FakeRunner(handlers), one handler for each role.
func wipRunner(handlers map[string]runner.Handler) *runner.FakeRunner {
	queued := map[string][]runner.Handler{}
	for role, h := range handlers {
		queued[role] = []runner.Handler{h}
	}
	return runner.NewFakeRunner(queued)
}

func wipRead(path string) string {
	text, err := os.ReadFile(path)
	if err != nil {
		panic(err)
	}
	return string(text)
}

func TestWip_ACutOffRevisionKeepsItsWorkBesideThePullRequest(t *testing.T) {
	is := difficultyCheckOf(t)
	h := newDifficultyHarness(t)
	start := wipPull(h)
	_, first := h.night(wipRunner(map[string]runner.Handler{
		"revise": difficultyBuilder(map[string]string{"src/game.txt": "half\n"}), "review": wipCapped}))
	is.Equal(first.Get("status"), "interrupted")
	is.Equal(h.originSHA("bot/issue-12"), start) // the pull request is untouched
	is.Equal(h.originSHA("bot/wip/40"), first.Get("head"))
	record := wipRecord(h)
	is.Equal([]any{record.Obj("wip").Get("sha"), record.Obj("wip").Get("start")}, []any{first.Get("head"), start})
	said := h.gh.BotComments(40)
	is.In("The work so far is on `bot/wip/40`", said[len(said)-1])
	is.Equal(wipInterruptions(t, record, false), 0) // it moved on: no strike

	// The next revision starts from it, is told so, and finishes the job.
	seen := map[string]string{}
	finishing := func(request *runner.RunRequest) *runner.RunResult {
		seen["file"] = wipRead(filepath.Join(request.Cwd, "src", "game.txt"))
		return difficultyBuilder(map[string]string{"src/game.txt": "whole\n"})(request)
	}
	r := wipRunner(map[string]runner.Handler{"revise": finishing, "review": difficultyReviewer(difficultyApprove)})
	planned, second := h.night(r)
	is.Equal(planned.Obj("wip").Get("sha"), first.Get("head"))
	is.Equal(seen["file"], "half\n")
	is.In("Your worktree starts from `bot/wip/40`", r.Calls[0].Prompt)
	is.Equal(second.Get("status"), "approved")
	is.Equal(second.Get("start"), start) // deliver still checks against the PR's head
	is.Equal(fakes.Git(h.origin, "show", h.originSHA("bot/issue-12")+":src/game.txt"), "whole")
	is.Equal(h.originSHA("bot/wip/40"), "") // done with
	is.NotIn("wip", wipRecord(h))
}

func TestWip_ABranchThatMovedSinceStartsAfresh(t *testing.T) {
	is := difficultyCheckOf(t)
	h := newDifficultyHarness(t)
	wipPull(h)
	h.night(wipRunner(map[string]runner.Handler{
		"revise": difficultyBuilder(map[string]string{"src/game.txt": "half\n"}), "review": wipCapped}))
	moved := fakes.PushBranch(h.origin, h.root, "bot/issue-12", map[string]string{"src/other.txt": "a person's\n"},
		"bot/issue-12")
	h.gh.Threads.Item(40).Obj("head").Set("sha", moved)
	seen := map[string]string{}
	looking := func(request *runner.RunRequest) *runner.RunResult {
		seen["file"] = wipRead(filepath.Join(request.Cwd, "src", "game.txt"))
		return difficultyBuilder(map[string]string{"src/game.txt": "redone\n"})(request)
	}
	r := wipRunner(map[string]runner.Handler{"revise": looking, "review": difficultyReviewer(difficultyApprove)})
	_, result := h.night(r)
	is.Equal(seen["file"], "v2\n")
	is.Equal(result.Obj("wip").Get("ignored"), "bot/wip/40")
	is.In("the pull request's branch moved after it stopped", r.Calls[0].Prompt)
}

func TestWip_PausesThatGetNowhereBlockItAfterThree(t *testing.T) {
	is := difficultyCheckOf(t)
	h := newDifficultyHarness(t)
	wipPull(h)
	for n := range 3 {
		_, result := h.night(wipRunner(map[string]runner.Handler{"revise": wipCapped}))
		is.Equal(result.Get("status"), "interrupted")
		if n < 2 {
			is.Equal(h.gh.LabelNames(40), map[string]bool{config.LabelPR: true, config.LabelRevise: true})
		}
	}
	is.Equal(h.gh.LabelNames(40), map[string]bool{config.LabelPR: true, config.LabelBlocked: true})
	said := h.gh.BotComments(40)
	is.In("cut off 3 runs in a row without its work moving on", said[len(said)-1])
}

// A revision that resumes from `bot/wip/<pr>` pushes it again every time; only a commit of its own
// past the kept head is progress, so pauses with none still add up.
func TestWip_AResumedRevisionThatGetsNowhereStillCounts(t *testing.T) {
	is := difficultyCheckOf(t)
	h := newDifficultyHarness(t)
	wipPull(h)
	_, first := h.night(wipRunner(map[string]runner.Handler{
		"revise": difficultyBuilder(map[string]string{"src/game.txt": "half\n"}), "review": wipCapped}))
	is.Equal(wipInterruptions(t, wipRecord(h), false), 0)
	for _, n := range []int{1, 2} {
		_, result := h.night(wipRunner(map[string]runner.Handler{"revise": wipCapped}))
		is.Equal(result.Get("status"), "interrupted")
		is.Equal(h.originSHA("bot/wip/40"), first.Get("head")) // kept, not moved on
		is.Equal(wipInterruptions(t, wipRecord(h), true), n)
	}
}

// Every revision merges `main` in; its squash commits are no merges, but they are not the run's
// work, so a cut-off run that did nothing else still counts.
func TestWip_MainsCommitsMergedInAreNoProgress(t *testing.T) {
	is := difficultyCheckOf(t)
	h := newDifficultyHarness(t)
	wipPull(h)
	h.night(wipRunner(map[string]runner.Handler{
		"revise": difficultyBuilder(map[string]string{"src/game.txt": "half\n"}), "review": wipCapped}))
	for _, n := range []int{1, 2} {
		fakes.PushBranch(h.origin, h.root, "main", map[string]string{
			fmt.Sprintf("docs/other-%d.md", n): "someone else's\n"})
		_, result := h.night(wipRunner(map[string]runner.Handler{"revise": wipCapped}))
		is.Equal(result.Get("status"), "interrupted")
		is.Equal(wipInterruptions(t, wipRecord(h), true), n)
	}
}

func TestWip_APauseThatMovesTheWorkOnResetsTheCount(t *testing.T) {
	is := difficultyCheckOf(t)
	h := newDifficultyHarness(t)
	wipPull(h)
	h.night(wipRunner(map[string]runner.Handler{"revise": wipCapped}))
	h.night(wipRunner(map[string]runner.Handler{"revise": wipCapped}))
	is.Equal(wipInterruptions(t, wipRecord(h), true), 2)
	_, result := h.night(wipRunner(map[string]runner.Handler{
		"revise": difficultyBuilder(map[string]string{"src/game.txt": "half\n"}), "review": wipCapped}))
	is.Equal(result.Get("status"), "interrupted")
	is.Equal(wipInterruptions(t, wipRecord(h), true), 0)
	is.Equal(h.gh.LabelNames(40), map[string]bool{config.LabelPR: true, config.LabelRevise: true})
}

func TestWip_ClosingThePullRequestDropsItsWip(t *testing.T) {
	is := difficultyCheckOf(t)
	h := newDifficultyHarness(t)
	h.gh.AddIssue(12)
	wipPull(h)
	h.night(wipRunner(map[string]runner.Handler{
		"revise": difficultyBuilder(map[string]string{"src/game.txt": "half\n"}), "review": wipCapped}))
	is.In("wip", wipRecord(h))
	pr := h.gh.Threads.Item(40).Copy()
	pr.Set("merged", false)
	pr.Set("state", "closed")
	events.Handle(h.ctx, "pull_request", pyjson.New("action", "closed", "pull_request", pr))
	is.In("bot/wip/40", h.gh.DeletedBranches)
	is.NotIn("wip", wipRecord(h))
}
