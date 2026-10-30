package gh

// urllib.parse's quoting, which Go's url package does otherwise (url.PathEscape keeps `:@&=+$`
// and more; url.QueryEscape writes `~` as %7E).

import (
	"fmt"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// quote is urllib.parse.quote(s, safe): every byte of s's UTF-8 but ASCII letters, digits,
// `_.-~` and those in safe written as %XX (upper-case hex). Python's default safe is "/".
func quote(s, safe string) string {
	var b strings.Builder
	for i := 0; i < len(s); i++ {
		c := s[i]
		if c < 0x80 && (c >= 'A' && c <= 'Z' || c >= 'a' && c <= 'z' || c >= '0' && c <= '9' ||
			strings.IndexByte("_.-~", c) >= 0 || strings.IndexByte(safe, c) >= 0) {
			b.WriteByte(c)
			continue
		}
		fmt.Fprintf(&b, "%%%02X", c)
	}
	return b.String()
}

// quotePlus is urllib.parse.quote_plus(s, safe): quote, with a space as "+".
func quotePlus(s, safe string) string {
	if !strings.Contains(s, " ") {
		return quote(s, safe)
	}
	return strings.ReplaceAll(quote(s, safe+" "), " ", "+")
}

// urlencode is urllib.parse.urlencode(query): `key=value` pairs in the query's order, each side
// str() of its value through quote_plus with nothing safe, joined by "&".
func urlencode(query *pyjson.Object) string {
	var parts []string
	query.Items(func(k string, v any) bool {
		parts = append(parts, quotePlus(k, "")+"="+quotePlus(py.Str(v), ""))
		return true
	})
	return strings.Join(parts, "&")
}
