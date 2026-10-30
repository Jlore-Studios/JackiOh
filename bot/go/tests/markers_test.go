// #317 part 2: no path from the model job to a push carries a conflict marker.
//
// A weak builder committed nested conflict markers in SPEC §11 and the rulings index on #203,
// #214 and #287, more than once each: its self checks ran out with them still open, the run
// ended `built`, and deliver pushed it. Review runs then spent themselves finding them.
//
// bot/tests/test_markers.py, ported (PORTING.md §7).
package tests

import (
	"fmt"
	"maps"
	"os"
	"path/filepath"
	"slices"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// markersMarked is MARKED: a file left with conflict markers.
var markersMarked = "<<<<<<< HEAD\nrules v2\n=======\nrules from main\n>>>>>>> origin/main\n"

// markersPlan is PLAN: an issue body carrying a plan.
var markersPlan = "<!-- jackioh-bot:plan -->\n## Plan\n1. Edit src/game.txt.\n" +
	"<!-- /jackioh-bot:plan -->"

// markersWeakHarness is weak_harness(test): the weak subscription alone, and an easy #12 with
// its plan.
func markersWeakHarness(t *testing.T) *flowHarness {
	h := newFlowHarness(t, flowHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"lite"}})
	h.GH.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelBuild, "difficulty:easy"), Body: markersPlan})
	return h
}

// ---------------------------------------------------------------------------- WorkTests

func TestMarkers_MarkersThatSurviveTheSelfChecksAndOneFixEndUnapproved(t *testing.T) {
	h := markersWeakHarness(t)
	r := workFake(map[string]any{"build": workBuilder(map[string]any{"src/game.txt": markersMarked}),
		"self_check": workReviewer(workApprove),
		"fix":        workBuilder(map[string]any{"src/game.txt": markersMarked + "again\n"})})
	planned, result := h.Night(r)
	flowEq(t, planned.Get("provider"), "lite")
	flowEq(t, result.Get("status"), "not_approved")
	flowIn(t, "conflict markers are still in src/game.txt", result.Str("reason"))
	flowTrue(t, pyjson.Obj(flowPath(result, "cycles", 0)).Has("marker_fix"))
	flowEq(t, h.OriginSHA("bot/issue-12"), "")
	flowEq(t, len(h.GH.ListPulls("open", "", 200)), 0)
	flowIn(t, "conflict markers are still in `src/game.txt`", flowLast(t, h.GH.BotComments(12)))
}

func TestMarkers_AFixThatClearsThemLetsItGoToReview(t *testing.T) {
	h := markersWeakHarness(t)
	r := workFake(map[string]any{"build": workBuilder(map[string]any{"src/game.txt": markersMarked}),
		"self_check": workReviewer(workApprove),
		"fix":        workBuilder(map[string]any{"src/game.txt": "rules v2, with main's\n"})})
	_, result := h.Night(r)
	flowEq(t, result.Get("status"), "built")
	flowEq(t, fakes.Git(h.Origin, "show", result.Str("head")+":src/game.txt"), "rules v2, with main's")
}

// ---------------------------------------------------------------------------- DeliverTests

// markersBundle is DeliverTests.bundle(h, files): a bundle of `bot/issue-12` changing `files`,
// and the result that names it.
func markersBundle(t *testing.T, h *flowHarness, files map[string]string) (*pyjson.Object, string) {
	t.Helper()
	planned := flowMake(h.Ctx)
	outs, _ := filepath.Glob(filepath.Join(h.Root, "out-*"))
	out := filepath.Join(h.Root, fmt.Sprintf("out-%d", len(outs)))
	dir := filepath.Join(h.Root, "hand-"+filepath.Base(out))
	fakes.Git(h.Root, "clone", "-q", h.Origin, dir)
	fakes.Git(dir, "checkout", "-q", "-b", "bot/issue-12")
	for _, name := range slices.Sorted(maps.Keys(files)) {
		if err := os.MkdirAll(filepath.Dir(filepath.Join(dir, name)), 0o777); err != nil {
			t.Fatal(err)
		}
		flowWrite(filepath.Join(dir, name), files[name])
	}
	fakes.Git(dir, "add", "-A")
	fakes.Git(dir, "commit", "-q", "-m", "a change")
	flowMkdir(t, out)
	base := fakes.Git(dir, "rev-parse", "origin/main")
	fakes.Git(dir, "bundle", "create", filepath.Join(out, "branch.bundle"), "refs/heads/bot/issue-12",
		"^"+base)
	flowWrite(filepath.Join(out, "result.json"), pyjson.Dumps(pyjson.New(
		"status", "approved", "head", fakes.Git(dir, "rev-parse", "HEAD"), "start", base,
		"bundle", "branch.bundle", "cycles", []any{}, "title", "t")))
	return planned, out
}

func TestMarkers_DeliverRefusesMarkersInAFileTheChangeTouched(t *testing.T) {
	h := newFlowHarness(t)
	h.GH.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelBuild)})
	planned, out := markersBundle(t, h, map[string]string{"src/game.txt": markersMarked})
	deliver.NewDeliverer(h.Ctx, planned, out, h.DeliverRepo, "", nil, "").Run()
	flowEq(t, h.OriginSHA("bot/issue-12"), "")
	flowEq(t, h.GH.LabelNames(12), flowSet(config.LabelBlocked))
	flowIn(t, "conflict markers are still in `src/game.txt`", flowLast(t, h.GH.BotComments(12)))
}

func TestMarkers_AMarkerLineInAFileTheChangeLeftAloneIsNotItsFault(t *testing.T) {
	h := newFlowHarness(t)
	fakes.PushBranch(h.Origin, h.Root, "main", map[string]string{"docs/merging.md": markersMarked})
	h.GH.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelBuild)})
	planned, out := markersBundle(t, h, map[string]string{"src/game.txt": "rules v2\n"})
	deliver.NewDeliverer(h.Ctx, planned, out, h.DeliverRepo, "", nil, "").Run()
	flowTrue(t, h.OriginSHA("bot/issue-12") != "", "origin_sha is not None")
}
