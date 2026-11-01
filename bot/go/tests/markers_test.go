// #317 part 2: no path from the model job to a push carries a conflict marker.
//
// A weak builder committed nested conflict markers in SPEC §11 and the rulings index on #203,
// #214 and #287, more than once each: its self checks ran out with them still open, the run
// ended `built`, and deliver pushed it. Review runs then spent themselves finding them.
//
// bot/tests/test_markers.py, ported (#467).
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
	h.gh.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelBuild, "difficulty:easy"), Body: markersPlan})
	return h
}

// ---------------------------------------------------------------------------- WorkTests

func TestMarkers_MarkersThatSurviveTheSelfChecksAndOneFixEndUnapproved(t *testing.T) {
	h := markersWeakHarness(t)
	r := workFake(map[string]any{"build": workBuilder(map[string]any{"src/game.txt": markersMarked}),
		"self_check": workReviewer(workApprove),
		"fix":        workBuilder(map[string]any{"src/game.txt": markersMarked + "again\n"})})
	planned, result := h.night(r)
	assertEqual(t, planned.Get("provider"), "lite")
	assertEqual(t, result.Get("status"), "not_approved")
	assertIn(t, "conflict markers are still in src/game.txt", result.Str("reason"))
	assertTrue(t, pyjson.Obj(dig(t, result, "cycles", 0)).Has("marker_fix"))
	assertEqual(t, h.originSHA("bot/issue-12"), "")
	assertEqual(t, len(h.gh.ListPulls("open", "", 200)), 0)
	assertIn(t, "conflict markers are still in `src/game.txt`", lastOf(t, h.gh.BotComments(12)))
}

func TestMarkers_AFixThatClearsThemLetsItGoToReview(t *testing.T) {
	h := markersWeakHarness(t)
	r := workFake(map[string]any{"build": workBuilder(map[string]any{"src/game.txt": markersMarked}),
		"self_check": workReviewer(workApprove),
		"fix":        workBuilder(map[string]any{"src/game.txt": "rules v2, with main's\n"})})
	_, result := h.night(r)
	assertEqual(t, result.Get("status"), "built")
	assertEqual(t, fakes.Git(h.origin, "show", result.Str("head")+":src/game.txt"), "rules v2, with main's")
}

// ---------------------------------------------------------------------------- DeliverTests

// markersBundle is DeliverTests.bundle(h, files): a bundle of `bot/issue-12` changing `files`,
// and the result that names it.
func markersBundle(t *testing.T, h *flowHarness, files map[string]string) (*pyjson.Object, string) {
	t.Helper()
	planned := flowMake(h.ctx)
	outs, _ := filepath.Glob(filepath.Join(h.root, "out-*"))
	out := filepath.Join(h.root, fmt.Sprintf("out-%d", len(outs)))
	dir := filepath.Join(h.root, "hand-"+filepath.Base(out))
	fakes.Git(h.root, "clone", "-q", h.origin, dir)
	fakes.Git(dir, "checkout", "-q", "-b", "bot/issue-12")
	for _, name := range slices.Sorted(maps.Keys(files)) {
		if err := os.MkdirAll(filepath.Dir(filepath.Join(dir, name)), 0o777); err != nil {
			t.Fatal(err)
		}
		mustWriteText(filepath.Join(dir, name), files[name])
	}
	fakes.Git(dir, "add", "-A")
	fakes.Git(dir, "commit", "-q", "-m", "a change")
	mkdir(t, out)
	base := fakes.Git(dir, "rev-parse", "origin/main")
	fakes.Git(dir, "bundle", "create", filepath.Join(out, "branch.bundle"), "refs/heads/bot/issue-12",
		"^"+base)
	mustWriteText(filepath.Join(out, "result.json"), pyjson.Dumps(pyjson.New(
		"status", "approved", "head", fakes.Git(dir, "rev-parse", "HEAD"), "start", base,
		"bundle", "branch.bundle", "cycles", []any{}, "title", "t")))
	return planned, out
}

func TestMarkers_DeliverRefusesMarkersInAFileTheChangeTouched(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelBuild)})
	planned, out := markersBundle(t, h, map[string]string{"src/game.txt": markersMarked})
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	assertEqual(t, h.originSHA("bot/issue-12"), "")
	assertEqual(t, h.gh.LabelNames(12), setOf(config.LabelBlocked))
	assertIn(t, "conflict markers are still in `src/game.txt`", lastOf(t, h.gh.BotComments(12)))
}

func TestMarkers_AMarkerLineInAFileTheChangeLeftAloneIsNotItsFault(t *testing.T) {
	h := newFlowHarness(t)
	fakes.PushBranch(h.origin, h.root, "main", map[string]string{"docs/merging.md": markersMarked})
	h.gh.AddIssue(12, fakes.Issue{Labels: flowLabels(config.LabelBuild)})
	planned, out := markersBundle(t, h, map[string]string{"src/game.txt": "rules v2\n"})
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	assertTrue(t, h.originSHA("bot/issue-12") != "", "origin_sha is not None")
}
