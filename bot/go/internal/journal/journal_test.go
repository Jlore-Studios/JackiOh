package journal

import (
	"fmt"
	"os"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata/python.json is bot/harness/journal.py's answers to the same calls.
func expected(t *testing.T) *pyjson.Object {
	t.Helper()
	data, err := os.ReadFile("testdata/python.json")
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(data)
	if err != nil {
		t.Fatal(err)
	}
	return v.(*pyjson.Object)
}

func pairs(o *pyjson.Object, key string) [][]any {
	out := [][]any{}
	for _, p := range o.List(key) {
		out = append(out, p.([]any))
	}
	return out
}

func TestCompactAsPython(t *testing.T) {
	for _, c := range pairs(expected(t), "compacts") {
		notes, limit, want := c[0].(string), c[1].(int), c[2].(string)
		if got := Compact(notes, limit); got != want {
			t.Errorf("Compact(%.40q, %d)\n got %q\nwant %q", notes, limit, got, want)
		}
	}
}

func TestCellsClocksAndTookAsPython(t *testing.T) {
	data := expected(t)
	for _, c := range pairs(data, "cells") {
		if got := Cell(c[0]); got != c[1] {
			t.Errorf("Cell(%v) = %q, want %q", c[0], got, c[1])
		}
	}
	for _, c := range pairs(data, "clocks") {
		if got := Clock(c[0]); got != c[1] {
			t.Errorf("Clock(%v) = %q, want %q", c[0], got, c[1])
		}
	}
	for _, c := range pairs(data, "took") {
		if got := Took(c[0].(*pyjson.Object)); got != c[1] {
			t.Errorf("Took(%s) = %q, want %q", pyjson.Dumps(c[0]), got, c[1])
		}
	}
	for _, c := range pairs(data, "step_rows") {
		if got := StepRows(c[0].([]any)); !pyjson.Equal(pyjson.ListOf(got), c[1]) {
			t.Errorf("StepRows\n got %q\nwant %v", got, c[1])
		}
	}
}

func TestFileShapeAsPython(t *testing.T) {
	data := expected(t)
	if Header(12) != data.Str("header") {
		t.Errorf("Header: %q", Header(12))
	}
	for _, c := range pairs(data, "trims") {
		text, limit, want := c[0].(string), c[1].(int), c[2].(string)
		if got := Trim(text, limit); got != want {
			t.Errorf("Trim(%d)\n got %q\nwant %q", limit, got, want)
		}
	}
	split := data.List("split")
	head, runs := Split(split[0].(string))
	if head != split[1] || !pyjson.Equal(pyjson.ListOf(runs), split[2]) {
		t.Errorf("Split: %q %q", head, runs)
	}
	in := data.List("runs_in")
	if got := RunsIn(in[0].(string)); !pyjson.Equal(pyjson.ListOf(got), in[1]) {
		t.Errorf("RunsIn: %q", got)
	}
	for _, c := range pairs(data, "keys") {
		if got := Key(c[0].(*pyjson.Object)); got != c[1] {
			t.Errorf("Key(%s) = %d, want %v", pyjson.Dumps(c[0]), got, c[1])
		}
	}
	if Branch != data.Str("branch") || URL("https://github.com", "o/r", 12) != data.Str("url") {
		t.Errorf("URL: %s", URL("https://github.com", "o/r", 12))
	}
}

// fakeGitHub is the files and branches part of tests/fakes.py's FakeGitHub.
type fakeGitHub struct {
	gh.Client
	files     map[[2]string][2]string
	branches  map[string]bool
	conflicts int
	shas      int
	fail      bool
}

func newFake() *fakeGitHub {
	return &fakeGitHub{files: map[[2]string][2]string{}, branches: map[string]bool{}}
}

func (f *fakeGitHub) sha() string { f.shas++; return fmt.Sprintf("sha%d", f.shas) }

func (f *fakeGitHub) GetFile(path, ref string) (string, string) {
	if f.fail {
		herr.Raise(&herr.GitHubError{Msg: "boom", Status: 500})
	}
	found := f.files[[2]string{ref, path}]
	return found[0], found[1]
}

func (f *fakeGitHub) PutFile(path, text, branch, sha, message string) {
	if !f.branches[branch] {
		herr.Raise(&herr.GitHubError{Msg: "no branch " + branch, Status: 404})
	}
	current := f.files[[2]string{branch, path}]
	if f.conflicts > 0 {
		f.conflicts--
		f.files[[2]string{branch, path}] = [2]string{current[0] + "\n\nanother run's section\n", f.sha()}
		herr.Raise(&herr.GitHubError{Msg: "conflict", Status: 409})
	}
	if current[1] != sha {
		herr.Raise(&herr.GitHubError{Msg: "sha mismatch", Status: 409})
	}
	f.files[[2]string{branch, path}] = [2]string{text, f.sha()}
}

func (f *fakeGitHub) BranchSHA(branch string) string {
	if f.branches[branch] {
		return "head-of-" + branch
	}
	return ""
}

func (f *fakeGitHub) CreateOrphanBranch(branch, path, text, message string, extra map[string]string) {
	f.branches[branch] = true
	for name, body := range extra {
		f.files[[2]string{branch, name}] = [2]string{body, f.sha()}
	}
	f.files[[2]string{branch, path}] = [2]string{text, f.sha()}
}

func TestAppendCreatesThenAddsAndRetriesARace(t *testing.T) {
	g := newFake()
	slept := 0
	sleep := func(s float64) {
		if s < 0.05 || s > 0.3 {
			t.Errorf("slept %v", s)
		}
		slept++
	}
	Append(g, 12, RunMarker+"\n## one\n", sleep)
	if !g.branches[Branch] || !strings.Contains(g.files[[2]string{Branch, "vercel.json"}][0], `"deploymentEnabled": false`) {
		t.Fatal("the branch starts with its vercel.json")
	}
	g.conflicts = 1
	Append(g, 12, RunMarker+"\n## two\n", sleep)
	text, _ := g.GetFile("12.md", Branch)
	if slept != 1 || !strings.HasPrefix(text, Header(12)) || !strings.Contains(text, "another run's section") ||
		len(RunsIn(text)) != 2 {
		t.Errorf("slept %d, text %q", slept, text)
	}
	g.conflicts = MaxAttempts
	_, ok := herr.Catch[*herr.GitHubError](func() { Append(g, 12, "x", sleep) })
	if !ok {
		t.Error("a write GitHub keeps refusing raises")
	}
}

func TestReadKeepsTheEndFromARun(t *testing.T) {
	g := newFake()
	if Read(g, 3, ReadChars) != "" {
		t.Error("no journal reads empty")
	}
	for n := range 10 {
		Append(g, 3, fmt.Sprintf("%s\n## run %d\n\n%s\n", RunMarker, n, strings.Repeat("é", 3000)), func(float64) {})
	}
	whole, _ := g.GetFile("3.md", Branch)
	if Read(g, 3, len(whole)*2) != whole {
		t.Error("a short journal reads whole")
	}
	read := Read(g, 3, 20000)
	rest := strings.SplitN(read, RunMarker, 2)
	if !strings.HasPrefix(read, Header(3)+"\n_(earlier runs are on the branch, not shown here)_\n\n") ||
		len(rest) != 2 || !strings.HasPrefix(strings.TrimLeft(rest[1], "\n"), "## ") {
		t.Errorf("read %.200q", read)
	}
	g.fail = true
	if Read(g, 3, 20000) != "" {
		t.Error("an unreadable journal reads empty")
	}
}
