package dashboard

// The few Python behaviours this module leans on that `py` and `pyjson` do not hold: kept private
// here, each naming the Python it stands for.

import (
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// htmlEscape is html.escape(s, quote=True), which writes `&#x27;` for `'`.
func htmlEscape(s string) string {
	return strings.NewReplacer("&", "&amp;", "<", "&lt;", ">", "&gt;", `"`, "&quot;", "'", "&#x27;").
		Replace(s)
}

// containsStr is `s in values` for a JSON list: an element equal to the string s.
func containsStr(values []any, s string) bool {
	for _, v := range values {
		if x, ok := v.(string); ok && x == s {
			return true
		}
	}
	return false
}

// upTo is xs[:n].
func upTo[T any](xs []T, n int) []T {
	if n < 0 {
		n += len(xs)
		if n < 0 {
			n = 0
		}
	}
	if n > len(xs) {
		n = len(xs)
	}
	return xs[:n]
}

// from is xs[n:] for n >= 0.
func from[T any](xs []T, n int) []T {
	if n > len(xs) {
		n = len(xs)
	}
	return xs[n:]
}

// lookup is d[key], whose KeyError goes up.
func lookup[V any](d map[string]V, key string) V {
	v, ok := d[key]
	if !ok {
		panic(py.KeyError(key))
	}
	return v
}

// noneAttr is the AttributeError of a method called on None (`_local(ctx, None)`), which the
// Python lets go up.
func noneAttr(name string) error {
	return &py.Error{Type: "AttributeError", Msg: "'NoneType' object has no attribute '" + name + "'"}
}
