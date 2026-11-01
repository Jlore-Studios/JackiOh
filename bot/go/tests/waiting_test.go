// What the status issue says the queue waits for and what each subscription can do now
// (`waiting.py`, #148): the planner's own rules on the lanes held now, so an item never "starts
// next" when the planner would not start it and a subscription is never free when it gets no
// work.
//
// bot/tests/test_waiting.py, ported (#467).
package tests

import (
	"sort"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/waiting"
)

// waitingClaudeRefused is claude_refused: every Claude account refused a call: `first` until
// 22:05, the others later. No `first` is "claude-2".
func waitingClaudeRefused(s *pyjson.Object, pool *providers.Pool, first ...string) {
	firstID := "claude-2"
	if len(first) > 0 {
		firstID = first[0]
	}
	var claude []string
	for _, p := range pool.Ordered() {
		if p.CLI == "claude" {
			claude = append(claude, p.ID)
		}
	}
	sort.SliceStable(claude, func(i, j int) bool { return claude[i] == firstID && claude[j] != firstID })
	for i, providerID := range claude {
		s.Obj("providers").Set(providerID, pyjson.New("refused_until",
			clock.ISO(testsupport.Night.Add(time.Duration(5+10*i)*time.Minute))))
	}
}

// waitingQueue is QueueTests' setUp.
type waitingQueue struct {
	gh  *fakes.FakeGitHub
	ctx *hctx.Context
}

func newWaitingQueue() *waitingQueue {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night, Cfg: testsupport.MakeConfig(
		testsupport.Config{Env: crossAll, Machine: []string{"muse"}})})
	return &waitingQueue{gh: gh, ctx: ctx}
}

// explain is QueueTests.explain(live=None).
func (w *waitingQueue) explain() *waiting.Explained {
	s := w.ctx.Store.Load()
	return waiting.Explain(w.ctx, s, pyjson.New(), w.gh.ListIssues("", "open", 300, ""), "")
}

// waitingRow is QueueTests.row: the row of item `number` (StopIteration: the test fails).
func waitingRow(t *testing.T, explained *waiting.Explained, number int) *waiting.Row {
	t.Helper()
	for _, r := range explained.Rows {
		if r.Number == number {
			return r
		}
	}
	t.Fatalf("no row for #%d", number)
	return nil
}

// waitingNow is explained.now[provider_id].
func waitingNow(t *testing.T, explained *waiting.Explained, providerID string) *waiting.Now {
	t.Helper()
	now, ok := explained.Now.Get(providerID).(*waiting.Now)
	if !ok {
		t.Fatalf("explained.now[%q] is %#v", providerID, explained.Now.Get(providerID))
	}
	return now
}

func TestWaiting_AHardItemWaitsForAStrongModelAndSaysWhichFreesFirst(t *testing.T) {
	w := newWaitingQueue()
	w.gh.AddIssue(41, fakes.Issue{Title: "Hard work", Labels: []string{config.LabelBuild, "difficulty:hard"}})
	pool := w.ctx.Cfg.Pool

	flowUpdate(w.ctx, func(s *pyjson.Object) {
		state.Item(s, 41).Update(pyjson.New("planned_at", clock.ISO(testsupport.Night), "planned_tier", "strong"))
		waitingClaudeRefused(s, pool)
	}, "seed")
	explained := w.explain()
	row := waitingRow(t, explained, 41)
	assertEqual(t, []string{row.Step, row.Difficulty, row.Tier}, []string{"build", "hard", "strong"})
	assertTrue(t, row.Starts == nil, "row.starts is not None")
	assertEqual(t, row.Waiting, "⏳ **a strong model (Opus)**: first free `claude-2` at 22:05")
	// Muse may start work, but nothing queued is for it: idle, never "free".
	assertEqual(t, waitingNow(t, explained, "muse").Kind, "idle")
	assertEqual(t, waitingNow(t, explained, "claude-2").Kind, "capped")
	body := dashboard.Render(w.ctx, nil, "", nil, true)
	assertIn(t, "| #41 | Hard work | build | 🔴 hard → strong | ⏳ **a strong model (Opus)**: "+
		"first free `claude-2` at 22:05 | — |", body)
	assertIn(t, "1 waits for a strong model", body)
	assertIn(t, "🔋 **1 of 12 subscriptions can start work now**: `muse`", body)
	assertIn(t, `<b>muse</b><br/>⚪ idle<br/>nothing queued is for it`, body)
}

func TestWaiting_ARevisionThatAnswersAStrongAuditWaitsForStrong(t *testing.T) {
	w := newWaitingQueue()
	w.gh.AddPull(9, "bot/issue-3", fakes.Pull{Title: "Fix it", Labels: []string{config.LabelPR, config.LabelRevise}})
	pool := w.ctx.Cfg.Pool

	flowUpdate(w.ctx, func(s *pyjson.Object) {
		state.Item(s, 9).Update(pyjson.New("audit_tier", "strong"))
		waitingClaudeRefused(s, pool)
	}, "seed")
	row := waitingRow(t, w.explain(), 9)
	assertEqual(t, []string{row.Step, row.Difficulty, row.Tier}, []string{"revise", "medium", "strong"})
	assertEqual(t, row.TierWhy, "it answers a strong reviewer's audit")
	assertTrue(t, row.Starts == nil, "row.starts is not None")
	// Without the audit, Muse (medium) takes it at the next run, and says so.
	flowUpdate(w.ctx, func(s *pyjson.Object) {
		state.Item(s, 9).Update(pyjson.New("audit_tier", ""))
	}, "no audit")
	explained := w.explain()
	row = waitingRow(t, explained, 9)
	if row.Starts == nil {
		t.Fatalf("row.starts is None")
	}
	assertEqual(t, row.Starts.Provider.ID, "muse")
	muse := waitingNow(t, explained, "muse")
	assertEqual(t, []string{muse.Kind, muse.Text}, []string{"next", "▶️ takes #9 next"})
	assertIn(t, "▶️ **starts at the next run** on `muse` (`muse-spark-1.3-contributor`, "+
		"medium)", dashboard.Render(w.ctx, nil, "", nil, true))
}

func TestWaiting_APartWaitsForThePartBeforeIt(t *testing.T) {
	w := newWaitingQueue()
	w.gh.AddIssue(1, fakes.Issue{Title: "Patch v0.9.X (part 1 of 2): the base"})
	w.gh.AddIssue(2, fakes.Issue{Title: "Patch v0.9.X (part 2 of 2): the rest", Labels: []string{config.LabelBuild}})
	row := waitingRow(t, w.explain(), 2)
	assertTrue(t, row.Held, "row.held")
	assertEqual(t, row.Waiting, "waits for #1 to close")
	assertIn(t, "| #2 | Patch v0.9.X (part 2 of 2): the rest | — | — | ⏸️ held back: waits "+
		"for #1 to close | — |", dashboard.Render(w.ctx, nil, "", nil, true))
}

// #1370: titles no longer name the patch (`Patch (part 2 of 2): …`), so a part waits for the
// earlier sub-issues of its own parent.
func TestWaiting_PartsWaitForTheirOwnParentsEarlierPartsNeverAnotherPatchs(t *testing.T) {
	w := newWaitingQueue()
	w.gh.AddIssue(10, fakes.Issue{Title: "Patch: the first change"})
	w.gh.AddIssue(20, fakes.Issue{Title: "Patch: the second change"})
	w.gh.AddIssue(11, fakes.Issue{Title: "Patch (part 1 of 2): the base"})
	w.gh.AddIssue(12, fakes.Issue{Title: "Patch (part 2 of 2): the rest", Labels: []string{config.LabelBuild}})
	w.gh.AddIssue(21, fakes.Issue{Title: "Patch (part 1 of 2): another base"})
	w.gh.AddIssue(22, fakes.Issue{Title: "Patch (part 2 of 2): another rest", Labels: []string{config.LabelBuild}})
	for _, family := range []struct {
		parent   int
		children []int
	}{{10, []int{11, 12}}, {20, []int{21, 22}}} {
		for _, child := range family.children {
			id, _ := pyjson.AsInt(w.gh.GetIssue(child).Get("id"))
			w.gh.AddSubIssue(family.parent, id)
		}
	}
	explained := w.explain()
	assertEqual(t, waitingRow(t, explained, 12).Waiting, "waits for #11 to close")
	assertEqual(t, waitingRow(t, explained, 22).Waiting, "waits for #21 to close")
}

func TestWaiting_AnUnratedBuildNeedsAMediumPlannerFirst(t *testing.T) {
	w := newWaitingQueue()
	w.gh.AddIssue(5, fakes.Issue{Title: "New thing", Labels: []string{config.LabelBuild}})
	flowUpdate(w.ctx, func(s *pyjson.Object) { waitingClaudeRefused(s, w.ctx.Cfg.Pool) }, "seed")
	row := waitingRow(t, w.explain(), 5)
	assertEqual(t, []string{row.Step, row.Difficulty, row.Tier, row.TierWhy},
		[]string{"plan", "unrated", "medium", "rates and plans it first"})
	if row.Starts == nil {
		t.Fatalf("row.starts is None")
	}
	assertEqual(t, row.Starts.Provider.ID, "muse") // Muse plans it in its own run
}

func TestWaiting_ASubscriptionWithNoLaneWhereItRunsIsFullNotFree(t *testing.T) {
	w := newWaitingQueue()
	cfg := testsupport.WithLanes(testsupport.MakeConfig(testsupport.Config{Env: crossAll,
		Machine: []string{"muse", "agy"}}), 11, 1)
	ctx := testsupport.MakeCtx(w.gh, testsupport.Ctx{At: testsupport.Night, Cfg: cfg})
	w.gh.AddIssue(50, fakes.Issue{Labels: []string{config.LabelWorking}})
	w.gh.Runs.Set("300", pyjson.New("status", "in_progress"))
	flowUpdate(ctx, func(s *pyjson.Object) {
		state.Item(s, 50).Update(pyjson.New("provider", "muse", "kind", "build", "run_id", "300"))
	}, "seed")
	s := ctx.Store.Load()
	explained := waiting.Explain(ctx, s, pyjson.New("50", "muse"), w.gh.ListIssues("", "open", 300, ""), "")
	assertEqual(t, waitingNow(t, explained, "muse").Kind, "busy")
	assertEqual(t, waitingNow(t, explained, "agy").Kind, "full")
	assertIn(t, "the night box is full", waitingNow(t, explained, "agy").Text)
}

func TestWaiting_HaltedNothingStarts(t *testing.T) {
	w := newWaitingQueue()
	w.gh.AddIssue(7, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:easy"}})
	flowUpdate(w.ctx, func(s *pyjson.Object) { s.Set("halted", true) }, "halt")
	body := dashboard.Render(w.ctx, nil, "", nil, true)
	assertIn(t, "> 🛑 **Halted**: halted by /harness halt.", body)
	explained := waiting.Explain(w.ctx, w.ctx.Store.Load(), pyjson.New(),
		w.gh.ListIssues("", "open", 300, ""), "halted by /harness halt")
	row := waitingRow(t, explained, 7)
	assertTrue(t, row.Starts == nil, "row.starts is not None")
	assertEqual(t, row.Waiting, "halted by /harness halt")
}

func TestWaiting_TheDifficultyLegendNamesEachTiersModels(t *testing.T) {
	w := newWaitingQueue()
	legend := dashboard.DifficultyLegend(w.ctx.Cfg.Pool)
	assertIn(t, "🔴 **hard**: a strong model only (Opus)", legend)
	assertIn(t, "a medium or strong model (muse, agy, gpt, Sonnet, then Opus)", legend)
	assertIn(t, "any model, the weak ones first (lite)", legend)
}
