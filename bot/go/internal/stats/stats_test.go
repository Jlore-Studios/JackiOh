package stats

import (
	"os"
	"reflect"
	"regexp"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata/python.json is what bot/harness/stats.py drew for tests/test_stats.py's world(), with a
// subscription's tokens added (written by a script run from bot/ against the Python harness).
func golden(t *testing.T) *pyjson.Object {
	t.Helper()
	data, err := os.ReadFile("testdata/python.json")
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(data)
	if err != nil {
		t.Fatal(err)
	}
	config.NightWorkflow = "bot-night.yml"
	config.BranchPrefix = "bot/"
	BranchIssue = regexp.MustCompile(`^` + regexp.QuoteMeta("bot/") + `issue-(\d+)\n?$`)
	return v.(*pyjson.Object)
}

func eventOf(o *pyjson.Object) *Event {
	e := &Event{At: o.Str("at"), Number: o.Int("number"), Kind: o.Str("kind"), Run: o.Str("run"),
		Provider: o.Str("provider"), Model: o.Str("model"), Action: o.Str("action"), Count: o.Int("count"),
		Approvers: pyjson.Strings(o.Get("approvers"))}
	if e.Approvers == nil {
		e.Approvers = []string{}
	}
	if pr, ok := o.Get("pr").(int); ok {
		e.PR = &pr
	}
	return e
}

func factsOf(o *pyjson.Object) *Facts {
	facts := NewFacts()
	for _, e := range pyjson.Objs(o.Get("events")) {
		facts.Events = append(facts.Events, eventOf(e))
	}
	facts.Pulls = pyjson.Objs(o.Get("pulls"))
	facts.CommitsOnMain = o.Int("commits_on_main")
	for _, r := range o.List("runs") {
		run := pyjson.Strings(r)
		facts.Runs = append(facts.Runs, [2]string{run[0], run[1]})
	}
	spent := o.Obj("spent")
	for _, pid := range spent.Keys() {
		entries := []SpentEntry{}
		for _, e := range spent.List(pid) {
			pair := pyjson.List(e)
			entries = append(entries, SpentEntry{pyjson.Str(pair[0]), py.MustFloat(pair[1])})
		}
		facts.Spent.Set(pid, entries)
	}
	counted := o.Obj("counted")
	for _, pid := range counted.Keys() {
		entries := []TokenEntry{}
		for _, e := range counted.List(pid) {
			x := pyjson.List(e)
			entries = append(entries, TokenEntry{pyjson.Str(x[0]), py.MustInt(x[1]), py.MustInt(x[2]), py.MustInt(x[3]), py.MustFloat(x[4])})
		}
		facts.Counted.Set(pid, entries)
	}
	return facts
}

func same(t *testing.T, what, got, want string) {
	t.Helper()
	if got != want {
		gl, wl := strings.Split(got, "\n"), strings.Split(want, "\n")
		for i := 0; i < len(gl) || i < len(wl); i++ {
			var g, w string
			if i < len(gl) {
				g = gl[i]
			}
			if i < len(wl) {
				w = wl[i]
			}
			if g != w {
				t.Fatalf("%s differs at line %d:\n got %q\nwant %q", what, i+1, g, w)
			}
		}
	}
}

func TestParseReadsEachCommentAsPythonDid(t *testing.T) {
	g := golden(t)
	comments, parsed := pyjson.Objs(g.Get("comments")), pyjson.Objs(g.Get("parsed"))
	if len(comments) != len(parsed) || len(comments) == 0 {
		t.Fatalf("%d comments, %d parsed", len(comments), len(parsed))
	}
	for i, c := range comments {
		if got, want := Parse(c), eventOf(parsed[i]); !reflect.DeepEqual(got, want) {
			t.Errorf("comment %d: got %+v, want %+v", i, got, want)
		}
	}
	for body, kind := range map[string]string{"Revision pushed on `lite` …": "revised",
		"Paused: the run's time budget is spent.": "paused", "This run failed: RuntimeError": "failed",
		"My revision did not pass review after 10 rounds": "not approved", "Queued #4; I will build it": "queued",
		"The run could not work on this on `claude-3`: 401": "infra", "nothing": "other"} {
		if got := Classify(body); got != kind {
			t.Errorf("Classify(%q) = %q, want %q", body, got, kind)
		}
	}
	approvers := Parse(pyjson.New("body", "`claude-opus` (strong) approved it, so it waits",
		"issue_url", "x/issues/3")).Approvers
	if len(approvers) != 0 {
		t.Errorf("a vote tally names no approver: %v", approvers)
	}
}

func TestJoinedGivesAnOutcomeItsRunsSubscription(t *testing.T) {
	events := Joined([]*Event{
		{At: "1", Number: 3, Kind: "start", Run: "5", Provider: "agy", Model: "g"},
		{At: "2", Number: 3, Kind: "paused", Run: "5"},
		{At: "3", Number: 4, Kind: "failed", Run: "6"},
	})
	got := [][2]string{}
	for _, e := range events {
		got = append(got, [2]string{e.Provider, e.Model})
	}
	if want := [][2]string{{"agy", "g"}, {"agy", "g"}, {"", ""}}; !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
}

func TestCreditsNameWhoPlannedBuiltRevisedAndApproved(t *testing.T) {
	g := golden(t)
	facts := factsOf(g.Obj("facts"))
	who := Credits_(facts.Events, facts.Pulls)
	want := g.Obj("credits")
	built := map[int]string{}
	for _, k := range want.Obj("built").Keys() {
		n, _ := strconv.Atoi(k)
		built[n] = want.Obj("built").Str(k)
	}
	if !reflect.DeepEqual(who.Built, built) {
		t.Errorf("built: got %v, want %v", who.Built, built)
	}
	for name, table := range map[string]map[int][]string{"planned": who.Planned, "revised": who.Revised,
		"approved": who.Approved} {
		wanted := map[int][]string{}
		for _, k := range want.Obj(name).Keys() {
			n, _ := strconv.Atoi(k)
			wanted[n] = pyjson.Strings(want.Obj(name).Get(k))
		}
		if !reflect.DeepEqual(table, wanted) {
			t.Errorf("%s: got %v, want %v", name, table, wanted)
		}
	}
	if got := who.Builder(pyjson.New("number", 11)); got != Unrecorded {
		t.Errorf("Builder(#11) = %q", got)
	}
}

func TestSectionDrawsTheWindowAsPythonDid(t *testing.T) {
	g := golden(t)
	facts := factsOf(g.Obj("facts"))
	who := Credits_(facts.Events, facts.Pulls)
	ever := Window_("All time", nil, parseTime(t, "2026-09-29T17:00:00Z"), facts)
	same(t, "section", strings.Join(Section(nil, ever, who, facts), "\n"), g.Str("section_all_time"))
	same(t, "modes", strings.Join(Modes([]*Window{ever}), "\n"), g.Str("modes"))
	same(t, "tokens", tokensText(ever), g.Str("tokens"))
	same(t, "lines", linesText(facts.Pulls), g.Str("lines"))
}

func TestSummarySetsTheWindowsSideBySide(t *testing.T) {
	g := golden(t)
	facts := factsOf(g.Obj("facts"))
	var quiet []*pyjson.Object
	for _, p := range facts.Pulls {
		if !p.Truthy("merged_at") {
			quiet = append(quiet, p)
		}
	}
	facts.Pulls, facts.CommitsOnMain = quiet, 1
	ever := Window_("All time", nil, parseTime(t, "2026-09-29T17:00:00Z"), facts)
	same(t, "summary", strings.Join(Summary([]*Window{ever, ever}), "\n"), g.Str("summary_quiet"))
}

func TestNumbersAreWrittenAsPythonWroteThem(t *testing.T) {
	g := golden(t)
	pcts := []string{Pct(1, 8), Pct(3, 8), Pct(2, 3), Pct(0, 0), Pct(5, 5)}
	if want := pyjson.Strings(g.Get("pct")); !reflect.DeepEqual(pcts, want) {
		t.Errorf("Pct: got %v, want %v", pcts, want)
	}
	for _, s := range pyjson.Objs(g.Get("sums")) {
		var xs []float64
		for _, x := range s.List("xs") {
			xs = append(xs, py.MustFloat(x))
		}
		// Python's sum of nothing is the int 0, which every caller writes as 0.0 would be.
		got := py.Sum(xs)
		if want := s.Float("sum"); got != want || (len(xs) > 0 && pyjson.FloatRepr(got) != s.Str("repr")) {
			t.Errorf("sum of %d: got %s, want %s", len(xs), pyjson.FloatRepr(got), s.Str("repr"))
		}
	}
	drawn := append(append(bars("T", "a", []bar{{"x", 1}, {"y", 0}, {"z", 2.25}, {"w", 0.05}}, false, "nothing yet"),
		bars("T", "a", []bar{{"x", 0}}, true, "nothing yet")...), bars("T", "a", nil, false, "none")...)
	if want := pyjson.Strings(g.Get("bars")); !reflect.DeepEqual(drawn, want) {
		t.Errorf("bars: got %q, want %q", drawn, want)
	}
	lined := append(line("L", "runs", []string{"09-28", "09-29"}, []int{0, 3}), line("L", "r", []string{"a"}, []int{0})...)
	if want := pyjson.Strings(g.Get("line")); !reflect.DeepEqual(lined, want) {
		t.Errorf("line: got %q, want %q", lined, want)
	}
	titles := []string{titleOf(pyjson.New("title", "a|b")), titleOf(pyjson.New("title", strings.Repeat("x", 69)+" yz")),
		titleOf(pyjson.New("title", strings.Repeat("é", 71)))}
	if want := pyjson.Strings(g.Get("titles")); !reflect.DeepEqual(titles, want) {
		t.Errorf("titles: got %q, want %q", titles, want)
	}
}

func TestMedianOfDurationsHalvesToEvenMicroseconds(t *testing.T) {
	for _, c := range []struct {
		in   []time.Duration
		want time.Duration
	}{{[]time.Duration{3}, 3}, {[]time.Duration{1, 2}, 2}, {[]time.Duration{3, 2}, 2},
		{[]time.Duration{5, 1, 9, 3}, 4}} {
		var in []time.Duration
		for _, n := range c.in {
			in = append(in, n*time.Microsecond)
		}
		if got := medianDuration(in); got != c.want*time.Microsecond {
			t.Errorf("median(%v µs) = %v, want %v µs", c.in, got, c.want)
		}
	}
}

func parseTime(t *testing.T, text string) time.Time {
	t.Helper()
	at, err := time.Parse(time.RFC3339, text)
	if err != nil {
		t.Fatal(err)
	}
	return at
}

func TestKeptFindsTheBlock(t *testing.T) {
	body := BlockStart + "\nx\n" + BlockEnd
	if got := Kept("before\n" + body + "\nafter"); got != body {
		t.Fatalf("Kept = %q", got)
	}
	if got := Kept("no block here"); got != "" {
		t.Fatalf("Kept = %q", got)
	}
}
