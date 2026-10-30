package trust

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata.json holds what Python 3.13 answered (bot/harness/trust.py).
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
	parsed := TrustParse(data.Str("text"))
	want := data.Obj("entries")
	if len(parsed.Entries) != want.Len() {
		t.Errorf("entries %d, want %d", len(parsed.Entries), want.Len())
	}
	for _, key := range want.Keys() {
		w := pyjson.List(want.Get(key))
		e, ok := parsed.Entries[key]
		if !ok {
			t.Errorf("no entry %q", key)
			continue
		}
		var id any
		if e.UserID != nil {
			id = *e.UserID
		} else if e.idTooBig {
			id = w[2] // beyond a Go int: Python's holds it
			if _, isFloat := w[2].(float64); !isFloat {
				t.Errorf("%q: id too big here but %v in Python", key, w[2])
			}
		}
		if !pyjson.Equal([]any{e.Login, e.Level, id}, w) {
			t.Errorf("entry %q = %v %v %v, want %v", key, e.Login, e.Level, id, w)
		}
	}
	if !pyjson.Equal(pyjson.ListOf(parsed.Problems), data.Get("problems")) {
		t.Errorf("problems = %q, want %v", parsed.Problems, data.Get("problems"))
	}
	if !pyjson.Equal(pyjson.ListOf(parsed.Logins()), data.Get("logins")) {
		t.Errorf("logins = %q, want %v", parsed.Logins(), data.Get("logins"))
	}
	for _, e := range data.List("calls") {
		c := pyjson.List(e)
		var uid *int
		if n, ok := c[1].(int); ok {
			uid = &n
		}
		assoc, _ := c[2].(string)
		if got := parsed.Level(c[0].(string), uid, assoc); got != c[3] {
			t.Errorf("level(%q, %v, %v) = %d, want %v", c[0], c[1], c[2], got, c[3])
		}
	}
	root := filepath.Join("..", "..", "..", "..")
	committed := TrustLoad(filepath.Join(root, ".harness", "trust.txt"))
	if !pyjson.Equal(pyjson.ListOf(committed.Problems), data.Get("committed_problems")) ||
		!pyjson.Equal(pyjson.ListOf(committed.Logins()), data.Get("committed_logins")) {
		t.Errorf("committed trust = %q %q", committed.Problems, committed.Logins())
	}
	missing := TrustLoad("no/such/trust.txt")
	if len(missing.Entries) != 0 || len(missing.Problems) != 1 ||
		missing.Problems[0] != "no/such/trust.txt does not exist; nobody may command the bot" {
		t.Errorf("missing = %v", missing.Problems)
	}
}
