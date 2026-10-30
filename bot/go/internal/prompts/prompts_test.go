package prompts

import (
	"crypto/sha256"
	"encoding/hex"
	"os"
	"path/filepath"
	"sort"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata.json holds what Python 3.13 answered (bot/harness/prompts.py and string.Template).
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
	dir := filepath.Join("..", "..", "..", "prompts")
	for _, e := range data.List("files") {
		c := pyjson.List(e)
		name := c[0].(string)
		found, err := Placeholders(name, dir)
		if err != nil {
			t.Fatal(err)
		}
		var keys []string
		values := pyjson.New()
		for k := range found {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		for _, k := range keys {
			values.Set(k, "<"+k+">")
		}
		if !pyjson.Equal(pyjson.ListOf(keys), c[1]) {
			t.Errorf("placeholders(%s) = %v, want %v", name, keys, c[1])
		}
		text, err := Render(name, dir, values)
		if err != nil {
			t.Fatalf("render(%s): %v", name, err)
		}
		sum := sha256.Sum256([]byte(text))
		if got := []any{hex.EncodeToString(sum[:]), len([]rune(text))}; !pyjson.Equal(got, c[2:]) {
			t.Errorf("render(%s) = %v, want %v", name, got, c[2:])
		}
	}
	mappings := map[string]map[string]string{
		"full": {"a": "1", "b": "2", "c": "3", "_x9": "4", "A_b": "5", "Kx": "6"},
		"part": {"a": "1"},
	}
	for _, row := range pyjson.Objs(data.Get("templates")) {
		tpl := &Template{row.Str("t")}
		for name, mapping := range mappings {
			var got []any
			text, err := tpl.Substitute(mapping)
			switch err.(type) {
			case nil:
				got = []any{"ok", text}
			case *KeyError:
				got = []any{"KeyError", err.Error()}
			default:
				got = []any{"ValueError", err.Error()}
			}
			if !pyjson.Equal(got, row.Get(name)) {
				t.Errorf("substitute(%q, %s) = %v, want %v", tpl.Template, name, got, row.Get(name))
			}
			if got := tpl.SafeSubstitute(mapping); got != row.Str("safe_"+name) {
				t.Errorf("safe_substitute(%q, %s) = %q, want %q", tpl.Template, name, got, row.Str("safe_"+name))
			}
		}
	}
	for _, e := range data.List("data") {
		c := pyjson.List(e)
		if got := Data(pyjson.Str(c[0]), "Thing"); got != c[1] {
			t.Errorf("data(%q) = %q, want %q", c[0], got, c[1])
		}
	}
	tmp := t.TempDir()
	if err := os.WriteFile(filepath.Join(tmp, "crlf.md"), []byte("one $a\r\ntwo\rthree\r\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if got, err := Render("crlf", tmp, pyjson.New("a", "x")); err != nil || got != data.Str("crlf") {
		t.Errorf("render(crlf) = %q %v, want %q", got, err, data.Str("crlf"))
	}
	if _, err := Load("missing", tmp); !os.IsNotExist(err) {
		t.Errorf("load(missing) = %v", err)
	}
}
