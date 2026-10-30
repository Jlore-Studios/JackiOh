package split

import (
	"os"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata.json holds what Python 3.13 answered (bot/harness/split.py).
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
	check := func(what string, got, want any) {
		t.Helper()
		if !pyjson.Equal(got, want) {
			t.Errorf("%s = %s, want %s", what, pyjson.Dumps(got), pyjson.Dumps(want))
		}
	}
	check("notes", []any{FullsendNotes, PartNote}, data.Get("notes"))
	for _, c := range pyjson.Objs(data.Get("parse")) {
		text := pyjson.Str(c.Get("text"))
		tree := Parse(text, Difficulties)
		check("parse("+pyjson.Dumps(text)+")", tree.ToDict(), c.Get("tree"))
		check("ok", tree.OK(), c.Get("ok"))
		ordered := Ordered(tree.Issues)
		var keys []any
		var made []Made
		for n, i := range ordered {
			keys = append(keys, i.Key)
			made = append(made, Made{i, 100 + n})
		}
		check("ordered", pyjson.ListOf(keys), c.Get("ordered"))
		var bodies []any
		for n, i := range tree.Issues {
			onto := ""
			if n%2 == 1 {
				onto = "fs/7"
			}
			bodies = append(bodies, Body(i, 7, "Squishy", []int{3, 4}[:n%3], onto))
		}
		check("bodies", pyjson.ListOf(bodies), c.Get("bodies"))
		check("checklist", []any{
			Checklist(7, made, "the night bot", tree.Summary, "run", ""),
			Checklist(7, made, "the night bot", tree.Summary, "run", "fs/7"),
		}, c.Get("checklist"))
	}
	hard := pyjson.Obj(data.List("parse")[10]).Str("text")
	check("parse(hard)", Parse(hard, []string{"easy", "medium"}).ToDict(), data.Get("hard"))
	cyc := []*SubIssue{{Key: "a", BlockedBy: []string{"b"}}, {Key: "b", BlockedBy: []string{"a"}}, {Key: "c"}}
	var order []any
	for _, i := range Ordered(cyc) {
		order = append(order, i.Key)
	}
	check("ordered(cycle)", pyjson.ListOf(order), data.Get("cycle_order"))
}
