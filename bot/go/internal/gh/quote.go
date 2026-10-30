package gh

// urllib.parse's quoting, which Go's url package does otherwise (url.PathEscape keeps `:@&=+$`
// and more; url.QueryEscape writes `~` as %7E).

import (
	"errors"
	"fmt"
	"math"
	"regexp"
	"strconv"
	"strings"
	"unicode"

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

// str0 is `str(v or "")`: "" for a falsy value, else str(v).
func str0(v any) string {
	if !pyjson.Truthy(v) {
		return ""
	}
	return py.Str(v)
}

// floatSyntax is what float() reads, once stripped: a decimal number with digits that may be
// any script's and `_` between two of them, or inf, infinity or nan in any case.
var floatSyntax = regexp.MustCompile(`^[+-]?(?:(?i:inf|infinity|nan)|(?:\p{Nd}(?:_?\p{Nd})*(?:\.(?:\p{Nd}(?:_?\p{Nd})*)?)?|\.\p{Nd}(?:_?\p{Nd})*)(?:[eE][+-]?\p{Nd}(?:_?\p{Nd})*)?)$`)

// pyFloat is float(text), and false where float() raises ValueError.
func pyFloat(text string) (float64, bool) {
	clean := py.Strip(text)
	if !floatSyntax.MatchString(clean) {
		return 0, false
	}
	if strings.EqualFold(strings.TrimLeft(clean, "+-"), "nan") {
		return math.NaN(), true // Go reads no sign on a NaN
	}
	var b strings.Builder
	for _, r := range clean {
		switch {
		case r == '_':
		case unicode.IsDigit(r) && r > 0x7f:
			b.WriteByte(byte('0' + py.DigitValue(r)))
		default:
			b.WriteRune(r)
		}
	}
	f, err := strconv.ParseFloat(b.String(), 64)
	if err != nil && !errors.Is(err, strconv.ErrRange) {
		return 0, false
	}
	return f, true
}
