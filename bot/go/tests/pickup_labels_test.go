// test_pickup_labels.py: labels that steer pickup: the priority tiers (#90), `human` (#96) and the
// difficulty labels.
package tests

import (
	"fmt"
	"path/filepath"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// pickupLabelsAll is ALL: every provider secret set.
var pickupLabelsAll = map[string]string{"HARNESS_SECRETS_SET": strings.Join(providers.SecretNames, " ")}

// HIGH, MEDIUM, LOW.
const (
	pickupLabelsHigh   = "priority:high"
	pickupLabelsMedium = "priority:medium"
	pickupLabelsLow    = "priority:low"
)

// pickupLabelsCtx is ctx_for(gh, at=NIGHT, only=None): every subscription switched on with its
// login, or `only` that one.
func pickupLabelsCtx(gh *fakes.FakeGitHub, only *providers.Provider) *hctx.Context {
	cfg := testsupport.MakeConfig(testsupport.Config{Env: pickupLabelsAll, Machine: testsupport.Machine})
	if only != nil {
		pool := *cfg.Pool
		pool.Priority = []string{only.ID}
		pool.Providers = cfg.Pool.Providers.Copy().Set(only.ID, only)
		replaced := *cfg
		replaced.Pool = &pool
		cfg = &replaced
	}
	return testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night, Cfg: cfg})
}

// pickupLabelsQueued is queued(ctx, number, at, **fields).
func pickupLabelsQueued(ctx *hctx.Context, number int, at string, fields ...any) {
	ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, number).Set("queued_at", at).Update(pyjson.New(fields...))
		return nil
	}, "state")
}

// pickupLabelsPicks is picks(ctx, gh, **plan_args): what successive runs claim, each run's item
// closed before the next one plans.
func pickupLabelsPicks(gh *fakes.FakeGitHub, ctx *hctx.Context) []int {
	taken := []int{}
	for {
		planned := plan.Make(ctx, false, nil, "auto", nil)
		switch py.Str(planned.Get("action")) {
		case "plan", "build", "revise", "review":
		default:
			return taken
		}
		number := needsPlanNumber(planned)
		taken = append(taken, number)
		gh.Threads.Item(number).Set("state", "closed")
	}
}

// pickupLabelsEveryModel is every_model(): each committed subscription.
// The weak one is left out: it takes no work at all since #2748.
func pickupLabelsEveryModel() []*providers.Provider {
	out := []*providers.Provider{}
	for _, p := range testsupport.MakeConfig(testsupport.Config{Env: pickupLabelsAll, Machine: testsupport.AllMachine}).Pool.Ordered() {
		if p.Tier != "weak" {
			out = append(out, p)
		}
	}
	return out
}

// pickupLabelsIssue is gh.add_issue(number, labels=labels).
func pickupLabelsIssue(gh *fakes.FakeGitHub, number int, labels ...string) *pyjson.Object {
	return gh.AddIssue(number, fakes.Issue{Labels: labels})
}

// pickupLabelsSkipped is `planned["skipped"][0]`.
func pickupLabelsSkipped(t *testing.T, planned *pyjson.Object) string {
	t.Helper()
	skipped := planned.List("skipped")
	if len(skipped) == 0 {
		t.Fatal("IndexError: nothing skipped")
	}
	return py.Str(skipped[0])
}

// ---------------------------------------------------------------------------- PriorityTests
//
// #90: `priority:high`, `priority:medium`, none, then `priority:low`.

func TestPickupLabels_TheTierOfASetOfLabels(t *testing.T) {
	tier := func(names ...string) int { return queue.PriorityTier(setOf(names...)) }
	assertEqual(t, tier(pickupLabelsHigh), 0)
	assertEqual(t, tier(pickupLabelsMedium, "bot:build"), 1)
	assertEqual(t, tier(), 2)
	assertEqual(t, tier(pickupLabelsLow), 3)
	// The highest label wins, names match whatever their case, and any other `priority:*` label is
	// no priority at all.
	assertEqual(t, tier(pickupLabelsHigh, pickupLabelsLow), 0)
	assertEqual(t, tier(pickupLabelsMedium, pickupLabelsLow), 1)
	assertEqual(t, tier("Priority:HIGH"), 0)
	assertEqual(t, tier("priority:urgent", "priority: high", "priority:highest"), 2)
	assertEqual(t, tier("priority:urgent", pickupLabelsLow), 3)
}

func TestPickupLabels_AMixOfTiersIsPickedHighMediumNoneLow(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	// Oldest first, so without the labels they would go the other way round.
	for _, c := range []struct {
		number int
		label  string
	}{{3, pickupLabelsLow}, {4, ""}, {5, pickupLabelsMedium}, {6, pickupLabelsHigh}} {
		labels := []string{config.LabelBuild}
		if c.label != "" {
			labels = append(labels, c.label)
		}
		pickupLabelsIssue(gh, c.number, labels...)
		pickupLabelsQueued(ctx, c.number, fmt.Sprintf("2026-09-2%dT00:00:00Z", c.number))
	}
	assertEqual(t, pickupLabelsPicks(gh, ctx), []int{6, 5, 4, 3})
}

func TestPickupLabels_TheChosenItemsTierIsInThePlan(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	pickupLabelsIssue(gh, 3, config.LabelBuild, pickupLabelsMedium)
	pickupLabelsIssue(gh, 4, config.LabelBuild)
	ctx := testsupport.MakeCtx(gh)
	planned := plan.Make(ctx, false, nil, "auto", nil)
	needsPlanIs(t, planned, []string{"number", "priority"}, 3, "medium")
	gh.Threads.Item(3).Set("state", "closed")
	assertEqual(t, plan.Make(ctx, false, nil, "auto", nil).Get("priority"), "none")
}

func TestPickupLabels_WithinATierTheOrderIsTheOneBefore(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	pickupLabelsIssue(gh, 3, config.LabelBuild, pickupLabelsHigh)
	pickupLabelsIssue(gh, 4, config.LabelBuild, pickupLabelsHigh)
	pickupLabelsIssue(gh, 5, config.LabelBuild, pickupLabelsHigh)
	gh.AddPull(9, "bot/issue-1", fakes.Pull{Labels: []string{config.LabelPR, config.LabelRevise, pickupLabelsHigh}})
	pickupLabelsIssue(gh, 6, config.LabelBuild)
	for _, c := range []struct {
		number int
		at     string
	}{{3, "2026-09-22"}, {4, "2026-09-21"}, {5, "2026-09-23"}, {9, "2026-09-25"}, {6, "2026-09-20"}} {
		pickupLabelsQueued(ctx, c.number, c.at+"T00:00:00Z")
	}
	// Revisions before builds, then the oldest build first, as before; the unlabelled one, oldest
	// of all, after every `priority:high` one.
	assertEqual(t, pickupLabelsPicks(gh, ctx), []int{9, 4, 3, 5, 6})
}

func TestPickupLabels_HighAndLowTogetherCountAsHigh(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	pickupLabelsIssue(gh, 3, config.LabelBuild, pickupLabelsMedium)
	pickupLabelsIssue(gh, 4, config.LabelBuild, pickupLabelsLow, pickupLabelsHigh)
	pickupLabelsQueued(ctx, 3, "2026-09-20T00:00:00Z")
	pickupLabelsQueued(ctx, 4, "2026-09-28T00:00:00Z")
	assertEqual(t, pickupLabelsPicks(gh, ctx), []int{4, 3})
}

func TestPickupLabels_AForcedItemStillComesFirst(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	pickupLabelsIssue(gh, 3, config.LabelBuild, pickupLabelsHigh)
	pickupLabelsIssue(gh, 4, config.LabelBuild, pickupLabelsLow)
	pickupLabelsQueued(ctx, 4, "2026-09-28T00:00:00Z", "forced", true)
	assertEqual(t, pickupLabelsPicks(gh, ctx), []int{4, 3})
}

func TestPickupLabels_ALabelChangedBetweenRunsCountsAtTheNextPickup(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh)
	for _, number := range []int{3, 4, 5} {
		pickupLabelsIssue(gh, number, config.LabelBuild)
		pickupLabelsQueued(ctx, number, fmt.Sprintf("2026-09-2%dT00:00:00Z", number))
	}
	assertEqual(t, plan.Make(ctx, false, nil, "auto", nil).Get("number"), 3)
	gh.Threads.Item(3).Set("state", "closed")
	gh.AddLabels(5, []string{pickupLabelsHigh})
	assertEqual(t, plan.Make(ctx, false, nil, "auto", nil).Get("number"), 5)
}

func TestPickupLabels_WithNoPriorityLabelsTheOrderIsForcedReviewsRevisionsHarderBuilds(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := pickupLabelsCtx(gh, nil)
	for _, number := range []int{3, 4, 5, 6} {
		pickupLabelsIssue(gh, number, config.LabelBuild)
	}
	pickupLabelsIssue(gh, 7, config.LabelBuild, "difficulty:hard")
	pickupLabelsIssue(gh, 8, config.LabelBuild, "an-unrelated-label", "priority:urgent")
	gh.AddPull(9, "bot/issue-1", fakes.Pull{Labels: []string{config.LabelPR, config.LabelRevise}})
	gh.AddPull(10, "bot/issue-2", fakes.Pull{Labels: []string{config.LabelPR, config.LabelCross}})
	for _, c := range []struct {
		number int
		at     string
	}{{3, "2026-09-25"}, {4, "2026-09-21"}, {5, "2026-09-23"}, {6, "2026-09-23"}, {7, "2026-09-28"},
		{8, "2026-09-20"}, {9, "2026-09-27"}, {10, "2026-09-26"}} {
		pickupLabelsQueued(ctx, c.number, c.at+"T00:00:00Z")
	}
	pickupLabelsQueued(ctx, 6, "2026-09-23T00:00:00Z", "forced", true)
	st := ctx.Store.Load()
	q := queue.Candidates(ctx, st, nil, nil)
	order := []int{}
	for _, pair := range plan.Pairs(ctx, st, q, &plan.Lanes{Limit: 3}, false, plan.AnyQuiet, nil) {
		order = append(order, pair.Candidate.Number)
	}
	// Forced first, then the review and the revision (work already begun), then the builds: the
	// hard one first (only the strongest can take it), the rest oldest first.
	assertEqual(t, order, []int{6, 10, 9, 7, 8, 4, 5, 3})
}

// ---------------------------------------------------------------------------- HumanTests
//
// #96: no model takes `human`, whatever its tier.

func TestPickupLabels_HumanIsNeverSelectedWhateverTheModel(t *testing.T) {
	five := 5
	for _, provider := range pickupLabelsEveryModel() {
		gh := fakes.NewFakeGitHub()
		ctx := pickupLabelsCtx(gh, provider)
		pickupLabelsIssue(gh, 5, config.LabelBuild, "human", pickupLabelsHigh, "difficulty:medium")
		pickupLabelsIssue(gh, 6, config.LabelBuild, "difficulty:medium")
		flowUpdateItem(ctx, 6, pyjson.New("planned_at", "2026-09-20T00:00:00Z"))
		pickupLabelsQueued(ctx, 5, "2026-09-20T00:00:00Z", "forced", true)
		planned := plan.Make(ctx, true, nil, "auto", nil)
		assertEqual(t, planned.Get("number"), 6, provider.ID)
		assertIn(t, "#5 skipped: labelled `human`, so no model takes it", pickupLabelsSkipped(t, planned))
		gh.Threads.Item(6).Set("state", "closed")
		planned = plan.Make(ctx, true, &five, "auto", nil)
		assertEqual(t, planned.Get("action"), "none", provider.ID)
		assertIn(t, "#5 is not queued", py.Str(planned.Get("reason")))
	}
}

func TestPickupLabels_AHumanPullRequestIsNotRevisedEither(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddPull(9, "bot/issue-1", fakes.Pull{Labels: []string{config.LabelPR, config.LabelRevise, "Human"}})
	pickupLabelsIssue(gh, 3, config.LabelBuild)
	ctx := pickupLabelsCtx(gh, nil)
	assertEqual(t, pickupLabelsPicks(gh, ctx), []int{3})
	skipped := plan.Peek_(ctx, false, nil, "auto").Skipped
	if len(skipped) == 0 {
		t.Fatal("IndexError: nothing skipped")
	}
	assertIn(t, "#9 skipped: labelled `human`", skipped[0])
}

// `human`: people do it. A request to build it gets a reply and nothing else, and it is never a
// candidate, nor given a stage label, nor revised when it conflicts.
func TestPickupLabels_HumanIsNeverQueuedPlannedLabelledOrBuilt(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night, Cfg: testsupport.MakeConfig(
		testsupport.Config{Env: pickupLabelsAll, Machine: testsupport.Machine, CommittedPlanLanes: true})})
	pickupLabelsIssue(gh, 5, config.LabelBuild, "Human", pickupLabelsHigh)
	pickupLabelsIssue(gh, 6, "human")
	pickupLabelsIssue(gh, 3, config.LabelBuild)
	reply := queue.QueueBuild(ctx, 6, "MaxGoetzmann", false, false, nil, nil)
	assertIn(t, "#6 is labelled `human`: people do it, so I leave it alone", reply)
	assertEqual(t, gh.LabelNames(6), setOf("human"))
	planned := plan.Make(ctx, true, nil, "auto", nil)
	assertEqual(t, planned.Get("number"), 3)
	assertIn(t, "#5 skipped: labelled `human`", pickupLabelsSkipped(t, planned))
	assertEqual(t, gh.LabelNames(5), setOf(config.LabelBuild, "Human", pickupLabelsHigh)) // no bot:needs-plan
	pull := gh.AddPull(9, "bot/issue-1", fakes.Pull{Labels: []string{config.LabelPR, "human"}})
	pull.Set("mergeable_state", "dirty")
	plan.Housekeeping(ctx, ctx.Store.Load())
	assertTrue(t, !gh.LabelNames(9)[config.LabelRevise])
	assertIn(t, "labelled `human`", queue.QueueRevise(ctx, 9, "MaxGoetzmann", false, "request", false, nil, nil))
}

func TestPickupLabels_TheReplySaysWhatTheLabelChanges(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := pickupLabelsCtx(gh, nil)
	pickupLabelsIssue(gh, 5, "human")
	pickupLabelsIssue(gh, 7, "Difficulty:Hard")
	gh.AddIssue(8)
	build := func(number int) string { return queue.QueueBuild(ctx, number, "MaxGoetzmann", false, false, nil, nil) }
	assertIn(t, "#5 is labelled `human`: people do it, so I leave it alone", build(5))
	assertIn(t, "labelled `difficulty:hard`, so only Opus plans, builds and reviews it", build(7))
	assertNotIn(t, "labelled", build(8))
}

// `tracking` (#2748): a tracker or status issue. Every queue path treats it as `human`: a request
// to build it, to split it as fullsend or to revise one gets a reply saying so and nothing else,
// it is never a candidate however it is labelled, and adopting a tree keeps a tracking sub-issue
// out of its parts. The bot may still comment on it.
func TestPickupLabels_TrackingIsNeverQueuedPlannedOrBuilt(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night, Cfg: testsupport.MakeConfig(
		testsupport.Config{Env: pickupLabelsAll, Machine: testsupport.Machine, CommittedPlanLanes: true})})
	pickupLabelsIssue(gh, 5, config.LabelBuild, "Tracking", pickupLabelsHigh)
	pickupLabelsIssue(gh, 6, config.LabelTracking)
	pickupLabelsIssue(gh, 3, config.LabelBuild)
	reply := queue.QueueBuild(ctx, 6, "MaxGoetzmann", false, false, nil, nil)
	assertIn(t, "#6 is labelled `tracking`: a tracker or status issue, which I may comment on but "+
		"never queue, plan or build", reply)
	fullsend := "fullsend"
	assertIn(t, "labelled `tracking`", queue.QueueBuild(ctx, 6, "MaxGoetzmann", false, false, nil, &fullsend))
	assertEqual(t, gh.LabelNames(6), setOf(config.LabelTracking))
	planned := plan.Make(ctx, true, nil, "auto", nil)
	assertEqual(t, planned.Get("number"), 3)
	assertIn(t, "#5 skipped: labelled `tracking`, so no model takes it", pickupLabelsSkipped(t, planned))
	assertEqual(t, gh.LabelNames(5), setOf(config.LabelBuild, "Tracking", pickupLabelsHigh)) // no bot:needs-plan
	gh.AddPull(9, "bot/issue-1", fakes.Pull{Labels: []string{config.LabelPR, config.LabelTracking}})
	assertIn(t, "labelled `tracking`", queue.QueueRevise(ctx, 9, "MaxGoetzmann", false, "request", false, nil, nil))
	// A tree adopted as fullsend keeps its tracking sub-issue out of its parts.
	gh.AddIssue(20, fakes.Issue{Labels: []string{"bot:fullsend"}})
	pickupLabelsIssue(gh, 21)
	pickupLabelsIssue(gh, 22, config.LabelTracking)
	for _, child := range []int{21, 22} {
		gh.AddSubIssue(20, gh.GetIssue(child).Int("id"))
	}
	adopted := queue.QueueBuild(ctx, 20, "MaxGoetzmann", false, false, nil, nil)
	assertIn(t, "#22 (labelled `tracking`)", adopted)
	assertTrue(t, !gh.LabelNames(22)[config.LabelBuild])
	assertTrue(t, gh.LabelNames(21)[config.LabelBuild])
}

// Each kind of run puts its own live label on the item it holds (#2748), and only one: a planning
// run, a split, a build, a revision, a CI fix (red CI or the merge queue), a resolution of a merge
// conflict (#2948), a review run and a fullsend parent's reconcile. A revision's CI fix is claimed
// with `bot:fixing-ci`, which the item's record keeps as `live`.
func TestPickupLabels_EachKindOfRunHasItsLiveLabel(t *testing.T) {
	for _, c := range []struct {
		kind, action, mode, source string
		reconcile, pr              bool
		want                       string
	}{
		{"build", "plan", "", "", false, false, config.LabelPlanning},
		{"build", "build", "fullsend", "", false, false, config.LabelSplitting},
		{"build", "build", "split", "", false, false, config.LabelSplitting},
		{"build", "build", "fullsend", "", true, false, config.LabelReconciling},
		{"build", "build", "", "", false, false, config.LabelBuilding},
		{"build", "build", "oneshot", "", false, false, config.LabelBuilding},
		{"revise", "revise", "", "request", false, true, config.LabelRevising},
		{"revise", "revise", "", "conflict", false, true, config.LabelResolving},
		{"revise", "revise", "", "ci", false, true, config.LabelFixingCI},
		{"review", "review", "", "", false, true, config.LabelReviewing},
		{"", "", "", "", false, true, config.LabelRevising},
		{"", "", "", "", false, false, config.LabelBuilding},
	} {
		assertEqual(t, config.LiveLabel(config.LiveRun(c.kind, c.action, c.mode, c.source, c.reconcile, c.pr)),
			c.want, c)
	}
	assertEqual(t, len(config.LiveLabels), 8)
	gh := fakes.NewFakeGitHub()
	ctx := pickupLabelsCtx(gh, nil)
	gh.AddPull(9, "bot/issue-1", fakes.Pull{Labels: []string{config.LabelPR, config.LabelRevise}})
	flowUpdateItem(ctx, 9, pyjson.New("kind", "revise", "source", "ci", "queued_at", "2026-09-20T00:00:00Z"))
	planned := plan.Make(ctx, true, nil, "auto", nil)
	assertEqual(t, []any{planned.Get("action"), planned.Get("source")}, []any{"revise", "ci"})
	assertEqual(t, gh.LabelNames(9), setOf(config.LabelPR, config.LabelFixingCI))
	assertEqual(t, ctx.Store.Load().Obj("items").Obj("9").Get("live"), "fixing-ci")
}

// `bot:working`, from a claim made before the live labels (#2748), still holds its thread: no run
// takes it, and a request waits for the run that holds it.
func TestPickupLabels_AnOldWorkingLabelStillHoldsItsThread(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := pickupLabelsCtx(gh, nil)
	pickupLabelsIssue(gh, 5, config.LabelBuild, config.LabelWorking)
	assertTrue(t, config.IsLive(gh.LabelNames(5)))
	assertEqual(t, len(queue.Candidates(ctx, ctx.Store.Load(), nil, nil)), 0)
	assertIn(t, "I am working on #5 right now", queue.QueueBuild(ctx, 5, "MaxGoetzmann", false, false, nil, nil))
	// The next state label takes it off, as it does a live label.
	queue.SetStateLabel(ctx, 5, gh.LabelNames(5), config.LabelBlocked)
	assertEqual(t, gh.LabelNames(5), setOf(config.LabelBlocked))
}

// `bot:merging` (#2748) is on a bot pull request while its auto-merge is on: it comes off when a
// revision holds the merge, on `/harness stop`, and when the pull request closes or merges.
func TestPickupLabels_MergingComesOffWhenAutoMergeGoesOff(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := pickupLabelsCtx(gh, nil)
	for _, n := range []int{9, 10, 11} {
		gh.AddPull(n, fmt.Sprintf("bot/issue-%d", n-5), fakes.Pull{Labels: []string{config.LabelPR, config.LabelMerging}}).
			Set("auto_merge", pyjson.New("merge_method", "squash"))
	}
	queue.QueueRevise(ctx, 9, "MaxGoetzmann", false, "request", false, nil, nil)
	assertEqual(t, gh.LabelNames(9), setOf(config.LabelPR, config.LabelRevise))
	queue.Stop(ctx, 10, "MaxGoetzmann")
	assertEqual(t, gh.LabelNames(10), setOf(config.LabelPR))
	closed := gh.Threads.Item(11).Set("state", "closed").Set("merged", true)
	events.Handle(ctx, "pull_request", pyjson.New("action", "closed", "pull_request", closed,
		"sender", pyjson.New("login", "MaxGoetzmann")))
	assertEqual(t, gh.LabelNames(11), setOf(config.LabelPR))
}

// ---------------------------------------------------------------------------- SuggestionTests
//
// A suggestion the bot opened is built once a person queues it (#664): `bot:build`, an assignment
// or `/harness build` is their yes, and `bot:suggestion` comes off.

func TestPickupLabels_ASuggestionAPersonLabelledIsACandidate(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := pickupLabelsCtx(gh, nil)
	pickupLabelsIssue(gh, 5, config.LabelBuild, config.LabelSuggestion, pickupLabelsHigh)
	pickupLabelsIssue(gh, 6, config.LabelSuggestion) // nobody has said yes yet
	skipped := []string{}
	numbers := []int{}
	for _, c := range queue.Candidates(ctx, ctx.Store.Load(), &skipped, nil) {
		numbers = append(numbers, c.Number)
	}
	assertEqual(t, numbers, []int{5})
	assertEqual(t, skipped, []string{})
}

func TestPickupLabels_QueueingASuggestionApprovesIt(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := pickupLabelsCtx(gh, nil)
	pickupLabelsIssue(gh, 5, config.LabelSuggestion)
	assertIn(t, "Queued #5", queue.QueueBuild(ctx, 5, "MaxGoetzmann", false, false, nil, nil))
	assertEqual(t, gh.LabelNames(5), setOf(config.LabelBuild))
}

// ---------------------------------------------------------------------------- LabelsTests

func TestPickupLabels_SetupCreatesTheNewLabels(t *testing.T) {
	_, humanText := repoMetaLabel(t, "human")
	assertEqual(t, humanText, "People do this: the night bot never queues, plans, builds or labels it")
	assertTrue(t, !config.Labels.Has("shitter"))
	assertTrue(t, !config.Labels.Has("difficult"))
	colors := map[string]string{}
	for _, name := range []string{pickupLabelsHigh, pickupLabelsMedium, pickupLabelsLow} {
		colors[name], _ = repoMetaLabel(t, name)
	}
	assertEqual(t, colors, map[string]string{pickupLabelsHigh: "d73a4a", pickupLabelsMedium: "fbca04",
		pickupLabelsLow: "0e8a16"})
	gh := fakes.NewFakeGitHub()
	created := map[string]bool{}
	for _, name := range config.Labels.Keys() {
		color, text := repoMetaLabel(t, name)
		if gh.EnsureLabel(name, color, text) {
			created[name] = true
		}
	}
	for _, name := range []string{"human", "difficulty:medium", "difficulty:hard",
		pickupLabelsHigh, pickupLabelsMedium, pickupLabelsLow} {
		assertTrue(t, created[name], name)
	}
	assertFalse(t, created["difficulty:easy"]) // retired (#2748)
	again := false
	for _, name := range config.Labels.Keys() {
		color, text := repoMetaLabel(t, name)
		if gh.EnsureLabel(name, color, text) { // any(): it stops at the first
			again = true
			break
		}
	}
	assertFalse(t, again)
}

func TestPickupLabels_ThePullRequestKeepsTheIssuesDifficultyAndTier(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: pickupLabelsAll, Machine: testsupport.Machine, At: testsupport.Day})
	pickupLabelsIssue(h.gh, 12, config.LabelBuild, "difficulty:medium", "Priority:High")
	planned, result := h.night(workFake(map[string]runner.Handler{
		"build": workBuilder(map[string]string{"src/game.txt": "v2\n"}), "review": workReviewer(workApprove)}))
	assertEqual(t, []any{planned.Get("provider"), planned.Get("priority"), planned.Get("difficulty"), result.Get("status")},
		[]any{"muse", "high", "medium", "approved"})
	pr, _ := pyjson.AsInt(wastePull(t, h.gh, "bot/issue-12").Get("number"))
	// Muse planned, built and reviewed it in the run, and its own approval ships it (2026-11-02,
	// #2748): it auto-merges, with the issue's labels.
	assertEqual(t, h.gh.LabelNames(pr), setOf(config.LabelPR, config.LabelMerging, "difficulty:medium", "Priority:High"))
	assertEqual(t, h.ctx.Store.Load().Obj("items").Obj(fmt.Sprint(pr)).Get("difficulty"), "medium")
	// A label still saying `difficulty:easy` is not carried: it is retired, and the sweep labels
	// the pull request from its issue (medium).
	d := deliver.NewDeliverer(h.ctx, pyjson.New("action", "build", "provider", "muse"), filepath.Join(h.root, "x"),
		h.deliverRepo, "", nil, "")
	pickupLabelsIssue(h.gh, 13, "difficulty:easy", "priority:low")
	assertEqual(t, d.Carried(13), []string{"priority:low"})
}

func TestPickupLabels_ADraftPullRequestKeepsThemToo(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Env: pickupLabelsAll, Machine: testsupport.Machine, At: testsupport.Day})
	pickupLabelsIssue(h.gh, 12, config.LabelBuild, "difficulty:hard", pickupLabelsLow)
	planned, result := h.night(workFake(map[string]runner.Handler{
		"build":  workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"fix":    workBuilder(map[string]string{"src/game.txt": "v3\n"}),
		"review": workReviewer(workChanges("Still wrong."))}), flowPlanArgs{Force: true})
	assertEqual(t, []any{planned.Get("provider"), result.Get("status")}, []any{"claude-3", "not_approved"})
	pull := wastePull(t, h.gh, "bot/issue-12")
	assertTrue(t, pull.Truthy("draft"))
	pr, _ := pyjson.AsInt(pull.Get("number"))
	assertEqual(t, h.gh.LabelNames(pr),
		setOf(config.LabelPR, "bot:blocked", "bot:stuck", "difficulty:hard", pickupLabelsLow))
}
