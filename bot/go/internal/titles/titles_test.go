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
		{[]string{"foo"}, 2, false, "harness titles: error: argument command: invalid choice: 'foo' (choose from 'shipped')\n", nil},
		{[]string{"shipped", "--before", "a", "--after", "b", "extra"}, 2, false, "harness titles: error: unrecognized arguments: extra\n", nil},
		{[]string{"--x", "shipped", "--before", "a", "--after", "b", "--zz"}, 2, false, "harness titles: error: unrecognized arguments: --x --zz\n", nil},
		{[]string{"shipped", "--before", "a", "--after", "b", "--", "x"}, 2, false, "harness titles: error: unrecognized arguments: -- x\n", nil},
		{[]string{"shipped", "--", "--before", "a"}, 2, false, "harness titles shipped: error: the following arguments are required: --before, --after\n", nil},
		{[]string{"shipped", "--zz", "--before"}, 2, false, "harness titles shipped: error: argument --before: expected one argument\n", nil},
		{[]string{"-x"}, 2, false, "harness titles: error: the following arguments are required: command\n", nil},
		{[]string{"shipped", "--before=", "x"}, 2, false, "harness titles shipped: error: the following arguments are required: --after\n", nil},
		{[]string{"shipped", "--before=1", "--after", "-x"}, 2, false, "harness titles shipped: error: argument --after: expected one argument\n", nil},
		{[]string{"shipped", "--bef", "-1", "--aft=b"}, 0, true, "", &arguments{command: "shipped", before: "-1", after: "b"}},
		{[]string{"shipped", "--before", "-x y", "--after", "٣"}, 0, true, "", &arguments{command: "shipped", before: "-x y", after: "٣"}},
		{[]string{"--", "shipped", "--before", "-1", "--after", "b", "--before", "c"}, 0, true, "",
			&arguments{command: "shipped", before: "c", after: "b"}},
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
	if _, code, ok := parseArgs([]string{"shipped", "-hx"}); code != 0 || ok {
		t.Fatal("help exits 0")
	}
	// Python 3.13's argparse, with prog "harness titles".
	want := "usage: harness titles shipped [-h] --before BEFORE --after AFTER\n\noptions:\n" +
		"  -h, --help       show this help message and exit\n  --before BEFORE\n  --after AFTER\n"
	if out.String() != want {
		t.Fatalf("help = %q", out.String())
	}
	out.Reset()
	parseArgs([]string{"--he"})
	if out.String() != topHelp {
		t.Fatalf("top help = %q", out.String())
	}
}
