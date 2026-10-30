package vault

import (
	"encoding/hex"
	"os"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata.json holds what Python 3.13 answered (bot/harness/vault.py), with a vault it sealed.
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
	for _, e := range data.List("looks") {
		c := pyjson.List(e)
		text, ok := c[0].(string)
		if !ok {
			text = strings.Repeat("x", 200001)
		}
		if got := LooksSealed(text); got != c[1] {
			t.Errorf("looks_sealed(%q) = %v, want %v", text, got, c[1])
		}
	}
	for _, e := range data.List("b64") {
		c := pyjson.List(e)
		body, ok := b64decode(c[0].(string))
		var got any
		if ok {
			got = hex.EncodeToString(body)
		}
		if !pyjson.Equal(got, c[1]) {
			t.Errorf("b64decode(%q) = %v, want %v", c[0], got, c[1])
		}
	}
	for _, e := range data.List("opens") {
		c := pyjson.List(e)
		var got any
		if found := Open(c[0].(string), c[1].(string)); found != nil {
			got = found
		}
		if !pyjson.Equal(got, c[2]) {
			t.Errorf("open_(%q, %q) = %s, want %s", c[0], c[1], pyjson.Dumps(got), pyjson.Dumps(c[2]))
		}
	}
	// Sealed here, opened here: and the text has the shape Python's has.
	secret, payload := data.Str("secret"), data.Obj("payload")
	sealed, err := Seal(payload, secret)
	if err != nil {
		t.Fatal(err)
	}
	if !LooksSealed(sealed) || !pyjson.Equal(Open(sealed, secret), payload) || Open(sealed, "other") != nil {
		t.Errorf("seal then open failed: %q", sealed)
	}
}
