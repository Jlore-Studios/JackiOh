// Package redact is bot/harness/redact.py.
//
// Secret redaction. Everything the harness writes to GitHub or to an artifact passes through it.
package redact

import (
	"regexp"
	"slices"
	"sort"
	"strings"
	"sync"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Redaction = "[REDACTED]"
var Redaction string = "[REDACTED]"

// A pattern and what replaces its match. Go's regexp has no lookbehind and its `\b` is ASCII, so
// Python's `\b` before a word is a captured `(^|[^\w])` (Python's Unicode `\w`) that the
// replacement keeps; a match never needs the character the one before it ended on, so the
// matches are Python's. Python's re.IGNORECASE also lets `i` match `ı` and `İ`, `s` match `ſ` and
// `k` match the Kelvin sign: the classes say so.
type pattern struct {
	re   *regexp.Regexp
	repl string
}

const wordStart = `(^|` + py.ReNotW + `)`

var patterns = []pattern{
	{regexp.MustCompile(`(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?-----END [A-Z ]*PRIVATE KEY-----`), Redaction},
	{regexp.MustCompile(`sk-ant-[A-Za-z0-9_\-]{20,}`), Redaction},
	{regexp.MustCompile(`github_pat_[A-Za-z0-9_]{40,}`), Redaction},
	{regexp.MustCompile(`gh[pousr]_[A-Za-z0-9]{30,}`), Redaction},
	{regexp.MustCompile(`AKIA[0-9A-Z]{16}`), Redaction},
	// (?i)\bbearer\s+[A-Za-z0-9._\-]{16,}
	{regexp.MustCompile(wordStart + `[bB][eE][aA][rR][eE][rR][` + py.SpaceClass + `]+[A-Za-z0-9._\-ıİſK]{16,}`), "${1}" + Redaction},
	// (?i)\bbasic\s+[A-Za-z0-9+/=]{24,}
	{regexp.MustCompile(wordStart + `[bB][aA][sSſ][iIıİ][cC][` + py.SpaceClass + `]+[A-Za-z0-9+/=ıİſK]{24,}`), "${1}" + Redaction},
}

var (
	// remembered holds secret values that are not whole environment values: the tokens inside a
	// provider's login file (`logins.py`), which a transcript could show one at a time.
	remembered   []string
	rememberedMu sync.Mutex
)

// MinRemembered = 16
var MinRemembered int = 16

// secretValues is config.secret_values(); a variable so this package's tests can stand in for it.
var secretValues = func() []string { return config.SecretValues(nil) }

// Remember redacts these values too, from now on in this process.
func Remember(values []string) {
	rememberedMu.Lock()
	defer rememberedMu.Unlock()
	for _, value := range values {
		if py.Len(value) >= MinRemembered && !slices.Contains(remembered, value) {
			remembered = append(remembered, value)
		}
	}
}

// Redact is `text` with every known token shape and every live secret value replaced.
func Redact(text string, extra []string /* = None */) string {
	if text == "" {
		return text
	}
	out := text
	for _, p := range patterns {
		out = p.re.ReplaceAllString(out, p.repl)
	}
	values := append([]string(nil), secretValues()...)
	rememberedMu.Lock()
	values = append(values, remembered...)
	rememberedMu.Unlock()
	values = append(values, extra...)
	// sorted(values, key=len, reverse=True): longest first, equals in their order.
	sort.SliceStable(values, func(i, j int) bool { return py.Len(values[i]) > py.Len(values[j]) })
	for _, value := range values {
		if value != "" && strings.Contains(out, value) {
			out = strings.ReplaceAll(out, value, Redaction)
		}
	}
	return out
}

// RedactJSON is every string leaf of nested dicts and lists, redacted.
func RedactJSON(obj any) any {
	switch x := obj.(type) {
	case string:
		return Redact(x, nil)
	case *pyjson.Object:
		if x == nil {
			return obj
		}
		found := pyjson.New()
		for _, k := range x.Keys() {
			found.Set(k, RedactJSON(x.Get(k)))
		}
		return found
	case map[string]any:
		found := make(map[string]any, len(x))
		for k, v := range x {
			found[k] = RedactJSON(v)
		}
		return found
	case map[string]string:
		found := make(map[string]string, len(x))
		for k, v := range x {
			found[k] = Redact(v, nil)
		}
		return found
	case []any:
		found := make([]any, len(x))
		for i, v := range x {
			found[i] = RedactJSON(v)
		}
		return found
	case []string:
		found := make([]any, len(x))
		for i, v := range x {
			found[i] = Redact(v, nil)
		}
		return found
	}
	return obj
}
