package redact

import (
	"os"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata.json holds what Python 3.13 answered (bot/harness/redact.py), with no secret in the
// environment.
func TestMatchesPython(t *testing.T) {
	saved := secretValues
	secretValues = func() []string { return nil }
	t.Cleanup(func() { secretValues = saved })
	raw, err := os.ReadFile("testdata.json")
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(raw)
	if err != nil {
		t.Fatal(err)
	}
	data := pyjson.Obj(v)
	for _, e := range data.List("cases") {
		c := pyjson.List(e)
		extra := pyjson.Strings(c[1])
		if got := Redact(c[0].(string), extra); got != c[2] {
			t.Errorf("redact(%q, %v) = %q, want %q", c[0], c[1], got, c[2])
		}
	}
	for _, e := range data.List("fuzz") {
		c := pyjson.List(e)
		if got := Redact(c[0].(string), nil); got != c[1] {
			t.Errorf("redact(%q) = %q, want %q", c[0], got, c[1])
		}
	}
	Remember([]string{"short", "xxxxxxxxxxxxxxx", "long-remembered-value-1", "long-remembered-value-1",
		"long-remembered-value-12"})
	for _, e := range data.List("remembered") {
		c := pyjson.List(e)
		if got := Redact(c[0].(string), nil); got != c[1] {
			t.Errorf("redact(%q) = %q, want %q", c[0], got, c[1])
		}
	}
	in, _ := pyjson.LoadsString(`{"a": ["bearer abcdefghijklmnopqrstuvwxyz0123", 5, null, {"b": "long-remembered-value-1"}], "long-remembered-value-1": true}`)
	if got := RedactJSON(in); !pyjson.Equal(got, data.Get("json")) {
		t.Errorf("redact_json = %s, want %s", pyjson.Dumps(got), pyjson.Dumps(data.Get("json")))
	}
}
