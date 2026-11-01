package identity

import (
	"os"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// asDict is dataclasses.asdict(found).
func asDict(x *Identity) *pyjson.Object {
	others := []any{}
	for _, o := range x.Others {
		others = append(others, pyjson.New("name", o.Name, "login", o.Login, "label_prefix", o.LabelPrefix,
			"home", o.Home, "token_env", o.TokenEnv, "workflow", o.Workflow))
	}
	return pyjson.New("name", x.Name, "title", x.Title, "command", x.Command, "label_prefix", x.LabelPrefix,
		"branch_prefix", x.BranchPrefix, "state_branch", x.StateBranch, "journal_branch", x.JournalBranch,
		"workflow", x.Workflow, "marker", x.Marker, "trust", x.Trust, "modes", pyjson.ListOf(x.Modes),
		"others", others, "suggestions", x.Suggestions)
}

// testdata.json holds what Python 3.13 answered (bot/harness/identity.py).
func TestMatchesPython(t *testing.T) {
	raw, err := os.ReadFile("testdata.json")
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(raw)
	if err != nil {
		t.Fatal(err)
	}
	data := pyjson.Obj(v)
	for _, e := range data.List("parse") {
		c := pyjson.List(e)
		var got, msg any
		if panicked := func() (p any) {
			defer func() { p = recover() }()
			found, err := Parse(c[0], false)
			if err != nil {
				msg = err.Error()
			} else {
				got = asDict(found)
			}
			return nil
		}(); panicked != nil {
			t.Errorf("parse(%s) panicked: %v", pyjson.Dumps(c[0]), panicked)
			continue
		}
		if !pyjson.Equal(got, c[1]) || !pyjson.Equal(msg, c[2]) {
			t.Errorf("parse(%s) = %s %v, want %s %v", pyjson.Dumps(c[0]), pyjson.Dumps(got), msg,
				pyjson.Dumps(c[1]), c[2])
		}
	}
	for _, e := range data.List("load") {
		c := pyjson.List(e)
		found, err := Load(RepoRoot, c[0].(string))
		if err != nil {
			t.Errorf("load(%v): %v", c[0], err)
			continue
		}
		if !pyjson.Equal(asDict(found), c[1]) {
			t.Errorf("load(%v) = %s, want %s", c[0], pyjson.Dumps(asDict(found)), pyjson.Dumps(c[1]))
		}
	}
	if Current == nil || RepoRoot == "" {
		t.Errorf("Current = %v, RepoRoot = %q", Current, RepoRoot)
	}
}
