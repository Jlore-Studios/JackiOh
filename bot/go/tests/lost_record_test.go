// lost_record_test.go: a delivery whose state record GitHub lost (2026-11-04: `PUT state.json`
// answered 500 after run 41800432972 pushed #2984's revision, so the loop delivered the run again,
// found the branch at its own push, called that "moved on GitHub" and blocked the item). A state
// write that GitHub answers with a server error is tried again, and a delivery that finds its
// branch already at its own head takes it as pushed.
package tests

import (
	"path/filepath"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// A write GitHub answers with a 5xx is read again and made once more; a 5xx after GitHub made the
// write reads back as made, so the change is not made twice.
func TestLostRecord_AStateWriteRetriesAServerError(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	slept := []float64{}
	store := state.NewStateStore(gh, config.StateBranch, config.StateFile, func(s float64) { slept = append(slept, s) })
	store.Update(func(s *pyjson.Object) any { s.Set("halted", false); return nil }, "x")
	realPut := gh.Own().PutFile
	errs := 0
	gh.Patch.PutFile = func(path, text, branch, sha, message string) {
		if errs == 0 {
			errs++
			herr.Raise(&herr.GitHubError{Msg: "Server Error", Status: 500})
		}
		if errs == 1 { // GitHub made this one, then answered 502
			errs++
			realPut(path, text, branch, sha, message)
			herr.Raise(&herr.GitHubError{Msg: "Bad Gateway", Status: 502})
		}
		realPut(path, text, branch, sha, message)
	}
	writes := 0
	store.Update(func(s *pyjson.Object) any {
		item := state.Item(s, 5)
		if item.Get("delivered") == nil {
			writes++
			item.Set("delivered", writes)
		}
		return nil
	}, "delivered run 1")
	assertEqual(t, store.Load().Obj("items").Obj("5").Get("delivered"), 2)
	assertEqual(t, errs, 2)
	assertEqual(t, slept, []float64{state.ServerErrorSeconds, state.ServerErrorSeconds})
	// Another refusal is still the caller's.
	gh.Patch.PutFile = func(path, text, branch, sha, message string) {
		herr.Raise(&herr.GitHubError{Msg: "Forbidden", Status: 403})
	}
	_, failed := herr.Catch[*herr.GitHubError](func() {
		store.Update(func(s *pyjson.Object) any { s.Set("halted", true); return nil }, "halt")
	})
	assertTrue(t, failed)
}

// The same run delivered twice, its first delivery's record lost: the second finds the branch at
// the run's own head, takes it as pushed, and blocks nothing.
func TestLostRecord_ADeliveryTwiceFindsItsOwnPush(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelBuild)})
	planned := flowMake(h.ctx)
	out := filepath.Join(h.root, "out-x")
	r := workFake(map[string]any{"build": workBuilder(map[string]any{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)})
	work.NewWorker(h.cfg, planned, r, h.clone, filepath.Join(h.root, "work"), out, nil, nil, nil, nil, nil, nil).Run()
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	pushed := h.originSHA("bot/issue-12")
	assertTrue(t, pushed != "")
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	assertEqual(t, h.originSHA("bot/issue-12"), pushed)
	for _, comment := range h.gh.BotComments(12) {
		assertNotIn(t, "moved on GitHub", comment)
	}
	assertNotIn(t, config.LabelBlocked, h.gh.LabelNames(12))
}
