package titles

import (
	"bytes"
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata/python.json holds the Python's answers for the same inputs (harness.titles, run with
// Python 3.13).
func python(t *testing.T, key string) []any {
	t.Helper()
	data, err := os.ReadFile(filepath.Join("testdata", "python.json"))
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(data)
	if err != nil {
		t.Fatal(err)
	}
	cases := pyjson.Obj(v).List(key)
	if len(cases) == 0 {
		t.Fatalf("no %s cases", key)
	}
	return cases
}

func TestWhatItDoesAndNewTitleAsPython(t *testing.T) {
	for _, c := range python(t, "what_it_does") {
		row := pyjson.List(c)
		part, rest := WhatItDoes(row[0].(string))
		if part != row[1] || rest != row[2] {
			t.Errorf("WhatItDoes(%q) = %q, %q; Python %q, %q", row[0], part, rest, row[1], row[2])
		}
	}
	for _, c := range python(t, "new_title") {
		row := pyjson.List(c)
		if got := NewTitle(row[0].(string), row[1].(string)); got != row[2] {
			t.Errorf("NewTitle(%q, %q) = %q; Python %q", row[0], row[1], got, row[2])
		}
	}
}

func TestPullOfAndClosedByAsPython(t *testing.T) {
	for _, c := range python(t, "pull_of") {
		row := pyjson.List(c)
		if got := PullOf(row[0].(string)); got != row[1] {
			t.Errorf("PullOf(%q) = %d; Python %v", row[0], got, row[1])
		}
	}
	for _, c := range python(t, "closed_by") {
		row := pyjson.List(c)
		want := []int{}
		for _, n := range pyjson.List(row[1]) {
			want = append(want, n.(int))
		}
		if got := ClosedBy(row[0].(string)); !reflect.DeepEqual(got, want) {
			t.Errorf("ClosedBy(%q) = %v; Python %v", row[0], got, want)
		}
	}
}

func TestLeftAloneAsPython(t *testing.T) {
	for _, c := range python(t, "left_alone") {
		row := pyjson.List(c)
		thread := pyjson.New("number", 9, "title", row[0], "labels", []any{})
		if got := LeftAlone_(thread); got != row[1] {
			t.Errorf("LeftAlone_(%q) = %v; Python %v", row[0], got, row[1])
		}
	}
	if !LeftAlone_(pyjson.New("number", 148, "title", "Anything")) {
		t.Error("the status issue is left alone")
	}
	merging := pyjson.New("number", 9, "title", "Anything", "labels", []any{pyjson.New("name", "production merge")})
	if !LeftAlone_(merging) {
		t.Error("a production merge thread is left alone")
	}
}

func newThread(number int, title, state string, pull bool, labels ...string) *pyjson.Object {
	names := []any{}
	for _, l := range labels {
		names = append(names, pyjson.New("name", l))
	}
	o := pyjson.New("number", number, "title", title, "state", state, "labels", names)
	if pull {
		o.Set("pull_request", pyjson.New("url", "x"))
	}
	return o
}

// test_titles.py's test_plans_the_new_titles_and_the_open_issues_kind_labels.
func TestPlanRetcon(t *testing.T) {
	threads := []*pyjson.Object{
		newThread(7, "Night bot: a cap", "open", false, "kind:internal"),
		newThread(1, "Patch v0.3.X: Bigot costs (3)", "closed", false),
		newThread(2, "Architecture: patch numbers (part 2 of 8): fragments", "open", false),
		newThread(3, "v0.6.0: daily quests", "open", false),
		newThread(4, "Patch v0.3.Y: a client fix", "open", true),
		newThread(5, "CI: daily super run failed", "open", false),
		newThread(6, "Patch: already new", "open", false),
	}
	rows := PlanRetcon(threads, map[int]string{1: "v0.3.4"})
	var got [][3]any
	for _, r := range rows {
		got = append(got, [3]any{r.Number, r.New, r.Label})
	}
	want := [][3]any{
		{1, "Patch v0.3.4: Bigot costs (3)", ""},
		{2, "Patch: patch numbers (part 2 of 8): fragments", "kind:internal"},
		{3, "Patch: daily quests", "kind:app"},
		{4, "Patch: a client fix", ""},
		{7, "Patch: a cap", ""},
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("PlanRetcon = %v", got)
	}
	var done []*pyjson.Object
	for _, r := range rows {
		if r.Label != "" {
			done = append(done, newThread(r.Number, r.New, "open", false, r.Label))
		} else {
			done = append(done, newThread(r.Number, r.New, "open", false))
		}
	}
	if again := PlanRetcon(done, map[int]string{1: "v0.3.4"}); len(again) != 0 {
		t.Fatalf("a second retcon changes %v", again)
	}
}

func TestShipped(t *testing.T) {
	v := func(names ...string) []*pyjson.Object {
		var out []*pyjson.Object
		for _, n := range names {
			out = append(out, pyjson.New("version", n))
		}
		return out
	}
	if got := Shipped(v("v0.3.17"), v("v0.3.17", "v0.3.17b")); len(got) != 1 || got[0].Str("version") != "v0.3.17b" {
		t.Errorf("Shipped = %v", got)
	}
	before := v("v0.3.16", "v0.3.17")
	for _, after := range [][]*pyjson.Object{before, v("v0.4.0b", "v0.4.0c"), v("v0.4.0b", "v0.4.0c", "v0.4.0d"), before[:1]} {
		if got := Shipped(before, after); len(got) != 0 {
			t.Errorf("Shipped(%v, %v) = %v", before, after, got)
		}
	}
}

func TestWriteCsvAsPythonsWriter(t *testing.T) {
	path := filepath.Join(t.TempDir(), "x.csv")
	WriteCsv([]*Retitle{{1, "issue", "open", `a, "b"`, "line\nbreak", ""}, {2, "pull", "closed", " lead", "x", "kind:app"}}, path)
	got, _ := os.ReadFile(path)
	want := "number,kind,state,old title,new title,label\r\n" +
		"1,issue,open,\"a, \"\"b\"\"\",\"line\nbreak\",\r\n" +
		"2,pull,closed, lead,x,kind:app\r\n"
	if string(got) != want {
		t.Fatalf("csv = %q", got)
	}
}

func TestApplyRetconSleepsBetweenWrites(t *testing.T) {
	var slept []float64
	if n := ApplyRetcon(nil, nil, 2.0, func(s float64) { slept = append(slept, s) }); n != 0 || slept != nil {
		t.Fatal("no rows, no writes")
	}
}

// The argparse parser, against what Python 3.13's printed for the same command lines.
func TestParseArgsAsArgparse(t *testing.T) {
	cases := []struct {
		argv    []string
		code    int
		ok      bool
		lastErr string
		want    *arguments
	}{
		{nil, 2, false, "harness titles: error: the following arguments are required: command\n", nil},
		{[]string{"shipped"}, 2, false, "harness titles shipped: error: the following arguments are required: --before, --after\n", nil},
		{[]string{"shipped", "--before"}, 2, false, "harness titles shipped: error: argument --before: expected one argument\n", nil},
		{[]string{"foo"}, 2, false, "harness titles: error: argument command: invalid choice: 'foo' (choose from 'shipped', 'retcon')\n", nil},
		{[]string{"retcon", "--limit", "x"}, 2, false, "harness titles retcon: error: argument --limit: invalid int value: 'x'\n", nil},
		{[]string{"retcon", "--pause", "0x10"}, 2, false, "harness titles retcon: error: argument --pause: invalid float value: '0x10'\n", nil},
		{[]string{"retcon", "extra"}, 2, false, "harness titles: error: unrecognized arguments: extra\n", nil},
		{[]string{"--x", "shipped", "--before", "a", "--after", "b", "--zz"}, 2, false, "harness titles: error: unrecognized arguments: --x --zz\n", nil},
		{[]string{"shipped", "--before", "a", "--after", "b", "--", "x"}, 2, false, "harness titles: error: unrecognized arguments: -- x\n", nil},
		{[]string{"shipped", "--", "--before", "a"}, 2, false, "harness titles shipped: error: the following arguments are required: --before, --after\n", nil},
		{[]string{"retcon", "--write=1"}, 2, false, "harness titles retcon: error: argument --write: ignored explicit argument '1'\n", nil},
		{[]string{"retcon", "--numbers", "-x"}, 2, false, "harness titles retcon: error: argument --numbers: expected one argument\n", nil},
		{[]string{"shipped", "--zz", "--before"}, 2, false, "harness titles shipped: error: argument --before: expected one argument\n", nil},
		{[]string{"-x"}, 2, false, "harness titles: error: the following arguments are required: command\n", nil},
		{[]string{"shipped", "--before=", "x"}, 2, false, "harness titles shipped: error: the following arguments are required: --after\n", nil},
		{[]string{"retcon", "--wr", "--num", "1,2", "--lim", "-3", "--pa", "1e3"}, 0, true, "",
			&arguments{command: "retcon", write: true, numbers: "1,2", limit: -3, pause: 1000, csv: "title-retcon.csv"}},
		{[]string{"retcon", "--limit", " 7 ", "--pause", "1_0.5", "--numbers", "-x y", "--csv="}, 0, true, "",
			&arguments{command: "retcon", limit: 7, pause: 10.5, numbers: "-x y", csv: ""}},
		{[]string{"retcon", "--limit", "٣", "--numbers=-x"}, 0, true, "",
			&arguments{command: "retcon", limit: 3, pause: 2, numbers: "-x", csv: "title-retcon.csv"}},
		{[]string{"--", "shipped", "--before", "-1", "--after", "b", "--before", "c"}, 0, true, "",
			&arguments{command: "shipped", before: "c", after: "b", pause: 2, csv: "title-retcon.csv"}},
	}
	for _, c := range cases {
		var out, errOut bytes.Buffer
		stdout, stderr = &out, &errOut
		args, code, ok := parseArgs(append([]string{}, c.argv...))
		stdout, stderr = os.Stdout, os.Stderr
		if code != c.code || ok != c.ok {
			t.Errorf("%q: code %d ok %v; want %d %v (%s)", c.argv, code, ok, c.code, c.ok, errOut.String())
			continue
		}
		if c.lastErr != "" && !bytes.HasSuffix(errOut.Bytes(), []byte(c.lastErr)) {
			t.Errorf("%q: stderr %q; want it to end %q", c.argv, errOut.String(), c.lastErr)
		}
		if c.want != nil && !reflect.DeepEqual(args, c.want) {
			t.Errorf("%q: %+v; want %+v", c.argv, args, c.want)
		}
	}
}

func TestHelpAsArgparse(t *testing.T) {
	var out bytes.Buffer
	stdout = &out
	defer func() { stdout = os.Stdout }()
	if _, code, ok := parseArgs([]string{"retcon", "-hx"}); code != 0 || ok {
		t.Fatal("help exits 0")
	}
	// Python 3.13's argparse, with prog "harness titles".
	want := "usage: harness titles retcon [-h] [--write] [--numbers NUMBERS]\n" +
		"                             [--limit LIMIT] [--csv CSV] [--pause PAUSE]\n\noptions:\n" +
		"  -h, --help         show this help message and exit\n" +
		"  --write            write the titles; without it, only the CSV\n" +
		"  --numbers NUMBERS  only these threads, comma-separated\n" +
		"  --limit LIMIT      at most this many threads\n  --csv CSV\n" +
		"  --pause PAUSE      seconds between writes\n"
	if out.String() != want {
		t.Fatalf("help = %q", out.String())
	}
	out.Reset()
	parseArgs([]string{"--he"})
	if out.String() != topHelp {
		t.Fatalf("top help = %q", out.String())
	}
}
