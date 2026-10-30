package state

import (
	"fmt"
	"os"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata/python.json is bot/harness/state.py's file for the same states, byte for byte.
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

func TestTheFileIsPythonsByteForByte(t *testing.T) {
	data := expected(t)
	raw, err := pyjson.LoadsString(data.Str("raw"))
	if err != nil {
		t.Fatal(err)
	}
	if got := dump(Normalise(raw)); got != data.Str("dump") {
		t.Errorf("dump\n%s\nwant\n%s", got, data.Str("dump"))
	}
	if got := dump(DefaultState()); got != data.Str("default") {
		t.Errorf("default\n%s\nwant\n%s", got, data.Str("default"))
	}
	if got := dump(Normalise([]any{1, 2})); got != data.Str("normalise_list") {
		t.Errorf("normalise of a list\n%s", got)
	}
}

// fakeGitHub is the files and branches part of tests/fakes.py's FakeGitHub.
type fakeGitHub struct {
	gh.Client
	files     map[[2]string][2]string
	branches  map[string]bool
	conflicts int
	shas      int
	puts      int
}

func newFake() *fakeGitHub {
	return &fakeGitHub{files: map[[2]string][2]string{}, branches: map[string]bool{}}
}

func (f *fakeGitHub) sha() string { f.shas++; return fmt.Sprintf("sha%d", f.shas) }

func (f *fakeGitHub) GetFile(path, ref string) (string, string) {
	found := f.files[[2]string{ref, path}]
	return found[0], found[1]
}

func (f *fakeGitHub) PutFile(path, text, branch, sha, message string) {
	f.puts++
	if !f.branches[branch] {
		herr.Raise(&herr.GitHubError{Msg: "no branch " + branch, Status: 404})
	}
	if !strings.HasSuffix(message, " [skip ci]") {
		herr.Raise(&herr.GitHubError{Msg: "message " + message, Status: 400})
	}
	current := f.files[[2]string{branch, path}]
	if f.conflicts > 0 {
		// Someone else wrote in between: their change lands, ours is refused.
		f.conflicts--
		other := Normalise(nil)
		if current[0] != "" {
			v, _ := pyjson.LoadsString(current[0])
			other = v.(*pyjson.Object)
		}
		Item(other, 999).Set("note", "written by another run")
		f.files[[2]string{branch, path}] = [2]string{pyjson.Dumps(other), f.sha()}
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

func TestUpdateMakesTheBranchAndReappliesALostRace(t *testing.T) {
	g := newFake()
	slept := 0
	store := NewStateStore(g, "bot-state", "state.json", func(s float64) {
		if s < 0.05 || s > 0.3 {
			t.Errorf("slept %v", s)
		}
		slept++
	})
	if got := store.Load(); !pyjson.Equal(got, DefaultState()) {
		t.Errorf("a missing branch loads the default: %s", pyjson.Dumps(got))
	}
	after := store.Update(func(s *pyjson.Object) any { Item(s, 12).Set("a", 1); return nil }, "state")
	if !g.branches["bot-state"] || after.Obj("items").Obj("12").Int("a") != 1 {
		t.Fatalf("first update: %s", pyjson.Dumps(after))
	}
	if !strings.Contains(g.files[[2]string{"bot-state", "vercel.json"}][0], "deploymentEnabled") {
		t.Error("the state branch starts with its vercel.json")
	}
	g.conflicts = 1
	after = store.Update(func(s *pyjson.Object) any { Item(s, 13).Set("b", 2); return nil }, "state")
	loaded := store.Load()
	if slept != 1 || !loaded.Obj("items").Has("999") || loaded.Obj("items").Obj("13").Int("b") != 2 ||
		!pyjson.Equal(after, loaded) {
		t.Errorf("after a race: slept %d, %s", slept, pyjson.Dumps(loaded))
	}
	text, _ := g.GetFile("state.json", "bot-state")
	if text != dump(loaded) {
		t.Error("the file is the sorted, indented dump")
	}
	puts := g.puts
	store.Update(func(s *pyjson.Object) any { return nil }, "state")
	if g.puts != puts {
		t.Error("an update that changes nothing writes nothing")
	}
	g.conflicts = MaxAttempts
	if _, ok := herr.Catch[*herr.StateConflict](func() {
		store.Update(func(s *pyjson.Object) any { s.Set("halted", true); return nil }, "state")
	}); !ok {
		t.Error("a file that keeps changing gives up")
	}
}

func TestAFileThatIsNotJSONStopsTheCaller(t *testing.T) {
	g := newFake()
	g.branches["bot-state"] = true
	g.files[[2]string{"bot-state", "state.json"}] = [2]string{"{oops é", "sha1"}
	store := NewStateStore(g, "bot-state", "state.json", nil)
	exc, ok := herr.Catch[*herr.StateUnreadable](func() { store.Load() })
	if !ok || exc.Msg != "state.json on bot-state is not JSON (7 characters read); left as it is" {
		t.Errorf("unreadable: %v", exc)
	}
	if _, ok := herr.Catch[*herr.StateUnreadable](func() {
		store.Update(func(s *pyjson.Object) any { return nil }, "state")
	}); !ok || g.puts != 0 {
		t.Error("an update never writes over an unreadable file")
	}
	g.files[[2]string{"bot-state", "state.json"}] = [2]string{"", "sha2"}
	if _, ok := herr.Catch[*herr.StateUnreadable](func() { store.Load() }); !ok {
		t.Error("an empty file that is there is not an empty state")
	}
}
