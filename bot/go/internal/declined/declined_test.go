package declined

import (
	"os"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata.json holds what Python 3.13 answered (bot/harness/declined.py) for a run of operations
// on one state.
func TestMatchesPython(t *testing.T) {
	raw, err := os.ReadFile("testdata.json")
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(raw)
	if err != nil {
		t.Fatal(err)
	}
	state := pyjson.New()
	for i, e := range pyjson.List(v) {
		c := pyjson.List(e)
		args := pyjson.List(c[1])
		var result any
		switch c[0] {
		case "note":
			result = Note(state, args[0].(int), args[1].(string), args[2].(string), args[3].(string))
		case "sync":
			result = pyjson.ListOf(Sync(state, pyjson.Objs(args[0]), args[1].(string)))
		case "known":
			result = Known(state, args[0].(string))
		case "lines":
			if i == len(pyjson.List(v))-1 {
				result = Lines(pyjson.New())
			} else {
				result = Lines(state)
			}
		case "final":
			result = state
		}
		if !pyjson.Equal(result, c[2]) {
			t.Fatalf("step %d %v%s: %s, want %s", i, c[0], pyjson.Dumps(args), pyjson.Dumps(result), pyjson.Dumps(c[2]))
		}
		if c[0] != "final" && c[0] != "lines" && len(Entries(state)) != c[3] {
			t.Fatalf("step %d: %d entries, want %v", i, len(Entries(state)), c[3])
		}
	}
}
