package memory

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata.json holds what Python 3.13 answered (bot/harness/memory.py).
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
	day := clock.ParseISO(data.Str("day"))
	check := func(what string, got, want any) {
		t.Helper()
		if !pyjson.Equal(got, want) {
			t.Errorf("%s = %s, want %s", what, pyjson.Dumps(got), pyjson.Dumps(want))
		}
	}
	for _, e := range data.List("readings") {
		c := pyjson.List(e)
		root := t.TempDir()
		files := pyjson.Obj(c[0])
		for _, name := range files.Keys() {
			path := filepath.Join(root, name)
			if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
				t.Fatal(err)
			}
			if err := os.WriteFile(path, []byte(files.Str(name)), 0o644); err != nil {
				t.Fatal(err)
			}
		}
		var got any
		if found := Reading(day, root); found != nil {
			got = found
		}
		check("reading("+pyjson.Dumps(files)+")", got, c[1])
	}
	for _, e := range data.List("adds") {
		c := pyjson.List(e)
		record := pyjson.New()
		for _, step := range pyjson.Objs(c[0]) {
			Add(record, step)
			Add(record, nil)
		}
		check("add", record, c[1])
		var got any
		if s := Summary(record); s != nil {
			got = s
		}
		check("summary", got, c[2])
	}
	for _, e := range data.List("summaries") {
		c := pyjson.List(e)
		var got any
		if s := Summary(pyjson.Obj(c[0])); s != nil {
			got = s
		}
		check("summary("+pyjson.Dumps(c[0])+")", got, c[1])
	}
	for _, e := range data.List("boxes") {
		c := pyjson.List(e)
		check("box_of("+pyjson.Dumps(c[:2])+")", BoxOf(c[0].(string), c[1].(string)), c[2])
	}
	for _, e := range data.List("lines") {
		c := pyjson.List(e)
		check("line("+pyjson.Dumps(c[0])+")", Line(pyjson.Obj(c[0]), day), c[1])
	}
	noted := pyjson.New("machine_memory", []any{})
	var entries []any
	for i := 0; i < Keep; i++ {
		entries = append(entries, pyjson.New("x", i))
	}
	noted.Set("machine_memory", append(entries, "junk"))
	Note(noted, pyjson.New("runner", "old", "a", 1), "new", "b2")
	check("note", noted, data.Get("noted"))
}
