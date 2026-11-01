package dashboard

import (
	"os"
	"slices"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata/python.json holds what bot/harness/dashboard.py answered for each case of its pure
// renderers (the ones that read no clock, config or GitHub), so the Go side writes the same bytes.
func loadPython(t *testing.T) *pyjson.Object {
	t.Helper()
	raw, err := os.ReadFile("testdata/python.json")
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(raw)
	if err != nil {
		t.Fatal(err)
	}
	return pyjson.Obj(v)
}

func cases(t *testing.T, c *pyjson.Object, name string) [][]any {
	t.Helper()
	var out [][]any
	for _, e := range c.List(name) {
		out = append(out, pyjson.List(e))
	}
	if len(out) == 0 {
		t.Fatalf("no %s cases", name)
	}
	return out
}

func strs(v any) []string {
	out := []string{}
	for _, s := range pyjson.List(v) {
		out = append(out, s.(string))
	}
	return out
}

func pairs(v any) [][2]*pyjson.Object {
	var out [][2]*pyjson.Object
	for _, p := range pyjson.List(v) {
		l := pyjson.List(p)
		out = append(out, [2]*pyjson.Object{pyjson.Obj(l[0]), pyjson.Obj(l[1])})
	}
	return out
}

func TestConstantsMatchPython(t *testing.T) {
	var slots []any
	for _, s := range BoxSlots {
		slots = append(slots, []any{s.Place, s.Owners, s.Extras})
	}
	ours := map[string]any{
		"TITLE": Title, "MARKER": Marker, "LABELS": Labels, "EVERY_MINUTES": EveryMinutes,
		"MAX_BODY": MaxBody, "QUEUE_ROWS": QueueRows, "RUN_ROWS": RunRows, "BAR_CELLS": BarCells,
		"BOX_CELLS": BoxCells, "RESULT": Result, "TIMELINE_STYLE": TimelineStyle,
		"BOX_STYLES": BoxStyles, "STYLE": Style, "LEGEND": Legend, "LANE_ORDER": LaneOrder,
		"LANES_MARK": LanesMark, "CLAUDE_SECRET": ClaudeSecret, "USAGE_CHART": UsageChart,
		"STEP": Step, "MODE_KIND": ModeKind, "LEARN_WORKFLOW": LearnWorkflow,
		"LEARN_BRANCH": LearnBranch, "LEARN_SCHEDULE": LearnSchedule, "LEARN_ROWS": LearnRows,
		"LEARN_NO_RUNNER": LearnNoRunner, "RUNNERS_MINUTES": RunnersMinutes,
		"RUNNERS_START": RunnersStart, "RUNNERS_END": RunnersEnd, "EXPLAINED": Explained,
		"HOSTED_RUNNERS": HostedRunners, "RUNNER_RUNS": RunnerRuns, "RUNNER_ROWS": RunnerRows,
		"LOCAL_LABEL": LocalLabel, "GATHER_LABEL": GatherLabel, "WORKFLOWS": Workflows,
		"HOSTED_ROLES": HostedRoles, "LOCAL_ROLES": LocalRoles, "BOXES_NOTE": BoxesNote,
		"PC_FILE": PCFile, "GATHER_WORKFLOW": GatherWorkflow, "BOX_ROW": BoxRow,
		"BOX_STEPS": BoxSteps, "RUNNER_LEGEND": RunnerLegend, "STEP_WORDS": StepWords,
		"RUNNER_WRAP": RunnerWrap, "GHOST_STYLE": GhostStyle, "LEARN_GAMES": LearnGames,
		"PROGRESS_START": ProgressStart, "PROGRESS_END": ProgressEnd, "PROGRESS_CHART": ProgressChart,
		"BOX_SLOTS": slots, "SECTION_START": SectionStart, "SECTION_END": SectionEnd,
		"TRAINING_LANES": TrainingLanes, "SHOWN_STEPS": ShownSteps,
		"TRAINING_STALE": TrainingStale.Seconds(), "LEARN_PR_STALE": LearnPRStale.Seconds(),
		"PC_QUIET": PCQuiet.Seconds(),
	}
	sorted := pyjson.Options{SortKeys: true}
	python := loadPython(t).Obj("constants")
	for _, name := range python.Keys() {
		got, ok := ours[name]
		if !ok {
			t.Errorf("%s: no Go value to compare", name)
			continue
		}
		if g, w := pyjson.DumpsWith(got, sorted), pyjson.DumpsWith(python.Get(name), sorted); g != w {
			t.Errorf("%s = %s, want %s", name, g, w)
		}
	}
}

func TestTextMatchesPython(t *testing.T) {
	c := loadPython(t)
	for _, p := range cases(t, c, "cell") {
		if got := Cell(p[0], p[1].(int)); got != p[2] {
			t.Errorf("_cell(%s, %v) = %q, want %q", py.Repr(p[0]), p[1], got, p[2])
		}
	}
	for _, p := range cases(t, c, "text") {
		if got := Text(p[0], p[1].(int)); got != p[2] {
			t.Errorf("_text(%s, %v) = %q, want %q", py.Repr(p[0]), p[1], got, p[2])
		}
	}
	for _, p := range cases(t, c, "bar") {
		var fraction *float64
		if p[0] != nil {
			f, _ := pyjson.AsFloat(p[0])
			fraction = &f
		}
		if got := Bar(fraction, p[1].(int)); got != p[2] {
			t.Errorf("bar(%s, %v) = %q, want %q", py.Repr(p[0]), p[1], got, p[2])
		}
	}
	for _, p := range cases(t, c, "hover") {
		if got := hover(p[0].(string), p[1].(string), p[2].(string)); got != p[3] {
			t.Errorf("_hover(%q, %q, %q) = %q, want %q", p[0], p[1], p[2], got, p[3])
		}
	}
}

func TestChartsMatchPython(t *testing.T) {
	c := loadPython(t)
	for _, p := range cases(t, c, "boxes") {
		var boxes []Box
		for _, b := range pyjson.List(p[2]) {
			l := pyjson.List(b)
			boxes = append(boxes, Box{strs(l[0]), l[1].(string)})
		}
		if got, want := Boxes(p[0].(string), p[1].(string), boxes), strs(p[3]); !slices.Equal(got, want) {
			t.Errorf("_boxes(%q, …, %d boxes) =\n%q\nwant\n%q", p[0], len(boxes), got, want)
		}
	}
	for _, p := range cases(t, c, "styles") {
		got, want := Styles(strs(p[0]), strs(p[1])), strs(p[2])
		if !slices.Equal(append([]string{}, got...), want) {
			t.Errorf("styles(%q) = %q, want %q", p[0], got, want)
		}
	}
	for _, p := range cases(t, c, "fold") {
		if got, want := Fold(p[0].(string), strs(p[1])), strs(p[2]); !slices.Equal(got, want) {
			t.Errorf("fold(%q) = %q, want %q", p[0], got, want)
		}
	}
}

func TestLanesMatchPython(t *testing.T) {
	c := loadPython(t)
	for i, p := range cases(t, c, "marked") {
		got := pyjson.Dumps(pyjson.ListOf(MarkedLanes(strs(p[0]))))
		if want := pyjson.Dumps(p[1]); got != want {
			t.Errorf("marked_lanes case %d =\n%s\nwant\n%s", i, got, want)
		}
	}
	for _, p := range cases(t, c, "lanes") {
		row := pyjson.Obj(p[0])
		if got := Open(row); got != p[1] {
			t.Errorf("_open(%s) = %d, want %v", pyjson.Dumps(row), got, p[1])
		}
		if got := inUse(row); got != p[2] {
			t.Errorf("_in_use(%s) = %q, want %q", pyjson.Dumps(row), got, p[2])
		}
	}
}

func TestLearnMatchesPython(t *testing.T) {
	c := loadPython(t)
	for _, p := range cases(t, c, "learn_outcome") {
		if got := LearnOutcome(pyjson.Obj(p[0]), pyjson.Objs(p[1])); got != p[2] {
			t.Errorf("learn_outcome(%s, %s) = %q, want %q", pyjson.Dumps(p[0]), pyjson.Dumps(p[1]),
				got, p[2])
		}
	}
	for _, p := range cases(t, c, "next_learn_run") {
		at, err := time.Parse(time.RFC3339, p[0].(string))
		if err != nil {
			t.Fatal(err)
		}
		if got := NextLearnRun(at).Format("2006-01-02T15:04:05Z"); got != p[1] {
			t.Errorf("next_learn_run(%s) = %s, want %s", p[0], got, p[1])
		}
	}
}

func TestRunnersMatchPython(t *testing.T) {
	c := loadPython(t)
	for _, p := range cases(t, c, "pc_load") {
		if got := pcLoad(pyjson.Obj(p[0]), p[1].(bool)); got != p[2] {
			t.Errorf("_pc_load(%s, %v) = %q, want %q", pyjson.Dumps(p[0]), p[1], got, p[2])
		}
	}
	for _, p := range cases(t, c, "played") {
		got := played(pyjson.Obj(p[0]), p[1].(string))
		want := pyjson.Obj(p[2])
		switch {
		case got == nil && want == nil:
		case got == nil || want == nil || got.games != want.Int("games") || got.done != want.Int("done") ||
			got.from != want.Int("from") || got.patch != want.Str("patch"):
			t.Errorf("_played(%s, %q) = %+v, want %s", pyjson.Dumps(p[0]), p[1], got, pyjson.Dumps(p[2]))
		}
	}
	for _, p := range cases(t, c, "where") {
		order, words := Where(pyjson.Obj(p[0]))
		want := pyjson.List(p[1])
		if order != want[0] || words != want[1] {
			t.Errorf("_where(%s) = (%d, %q), want %s", pyjson.Dumps(p[0]), order, words, pyjson.Dumps(p[1]))
		}
	}
	for _, p := range cases(t, c, "step") {
		if got := Step_(pyjson.Obj(p[0]), p[1].(int)); got != p[2] {
			t.Errorf("_step(%s, %v) = %q, want %q", pyjson.Dumps(p[0]), p[1], got, p[2])
		}
	}
	for _, p := range cases(t, c, "job_name") {
		run, job := pyjson.Obj(p[0]), pyjson.Obj(p[1])
		if got := flow(run); got != p[2] {
			t.Errorf("_flow(%s) = %q, want %q", pyjson.Dumps(run), got, p[2])
		}
		if got := jobName(run, job); got != p[3] {
			t.Errorf("_job_name(%s, %s) = %q, want %q", pyjson.Dumps(run), pyjson.Dumps(job), got, p[3])
		}
	}
	for _, p := range cases(t, c, "night_items") {
		if got, want := pyjson.Dumps(nightItems(pyjson.Obj(p[0]))), pyjson.Dumps(p[1]); got != want {
			t.Errorf("_night_items(%s) = %s, want %s", pyjson.Dumps(p[0]), got, want)
		}
	}
	for _, p := range cases(t, c, "run_for") {
		if got := runFor(pyjson.New(), pyjson.New(), pyjson.Obj(p[0])); got != p[1] {
			t.Errorf("_run_for({}, {}, %s) = %q, want %q", pyjson.Dumps(p[0]), got, p[1])
		}
	}
	for _, p := range cases(t, c, "pc_work") {
		if got := pcWork(pyjson.Obj(p[0]), pairs(p[1]), p[2].(bool)); got != p[3] {
			t.Errorf("_pc_work(%s, %s, %v) = %q, want %q", pyjson.Dumps(p[0]), pyjson.Dumps(p[1]),
				p[2], got, p[3])
		}
	}
}
