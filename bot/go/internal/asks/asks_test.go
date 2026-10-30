package asks

import (
	"os"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// recorder records React calls; the rest of gh.Client is never called here.
type recorder struct {
	gh.Client
	reacted []any
}

func (r *recorder) React(commentID int, content string, reviewComment bool) {
	r.reacted = append(r.reacted, []any{commentID, content, reviewComment})
}

// testdata.json holds what Python 3.13 answered (bot/harness/asks.py) for a run of operations on
// one record.
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
	holder := data.Obj("start")
	for i, e := range data.List("ops") {
		c := pyjson.List(e)
		var result any
		switch c[0] {
		case "note":
			a := &Ask{Key: c[1].(string)}
			Note(holder, a)
			result = a.Recorded
		case "note_none":
			Note(holder, nil)
		case "add":
			Add(holder, pyjson.Strings(c[1]))
		case "take":
			result = pyjson.ListOf(Take(holder))
		case "give_back":
			GiveBack(holder)
		case "pop_taken":
			result = pyjson.ListOf(PopTaken(holder))
		case "pop_waiting":
			result = pyjson.ListOf(PopWaiting(holder))
		case "react":
			r := &recorder{}
			React(r, pyjson.Strings(c[1]), Done)
			result = pyjson.ListOf(r.reacted)
		}
		want := c[2]
		if c[0] == "take" || c[0] == "pop_taken" || c[0] == "pop_waiting" {
			// Python's list holds whatever the record held (an int stays an int); Go's is []string.
			var strs []any
			for _, x := range pyjson.List(want) {
				strs = append(strs, py.Str(x))
			}
			want = pyjson.ListOf(strs)
		}
		if !pyjson.Equal(result, want) {
			t.Errorf("step %d %v: result %s, want %s", i, c[0], pyjson.Dumps(result), pyjson.Dumps(c[2]))
		}
		if !pyjson.Equal(holder, c[3]) {
			t.Fatalf("step %d %v: holder %s, want %s", i, c[0], pyjson.Dumps(holder), pyjson.Dumps(c[3]))
		}
	}
	if Key(5, false) != "c:5" || Key(7, true) != "rc:7" {
		t.Error("key")
	}
}
