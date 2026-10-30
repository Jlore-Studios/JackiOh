// Package py holds the Python behaviours the harness depends on where Go's own differ, one tested
// copy each: str() and repr() of a value, round() to even, floor division and modulo,
// str.splitlines(), fnmatch and the thousands separator (here); str.strip(), split(), lower() and
// re's Unicode classes (text.go); bytes.decode("utf-8", "replace") and text mode (decode.go);
// int(), float(), `x or default`, iteration and Python's own exceptions (value.go); Counter,
// sum() of floats, the `%` format and tuple sort keys (collections.go). Each function names the
// Python it stands for, and the tests hold Python 3.13's answers (testdata.json).
package py

import (
	"fmt"
	"math"
	"regexp"
	"strconv"
	"strings"
	"sync"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Str is str(v): "None", "True", "False", a float's repr ("1.0"), a list or dict as Python
// prints it.
func Str(v any) string {
	switch x := v.(type) {
	case nil:
		return "None"
	case string:
		return x
	case bool:
		if x {
			return "True"
		}
		return "False"
	case float64:
		return pyjson.FloatRepr(x)
	case float32:
		return pyjson.FloatRepr(float64(x))
	case int, int64, int32:
		return fmt.Sprint(x)
	}
	return Repr(v)
}

// Repr is repr(v) for JSON values: strings in single quotes, as Python prints them in a list or
// dict.
func Repr(v any) string {
	switch x := v.(type) {
	case string:
		return reprString(x)
	case []any:
		parts := make([]string, len(x))
		for i, e := range x {
			parts[i] = Repr(e)
		}
		return "[" + strings.Join(parts, ", ") + "]"
	case []string:
		parts := make([]string, len(x))
		for i, e := range x {
			parts[i] = reprString(e)
		}
		return "[" + strings.Join(parts, ", ") + "]"
	case *pyjson.Object:
		parts := []string{}
		x.Items(func(k string, e any) bool {
			parts = append(parts, reprString(k)+": "+Repr(e))
			return true
		})
		return "{" + strings.Join(parts, ", ") + "}"
	}
	return Str(v)
}

func reprString(s string) string {
	quote := "'"
	if strings.Contains(s, "'") && !strings.Contains(s, `"`) {
		quote = `"`
	}
	var b strings.Builder
	b.WriteString(quote)
	for _, r := range s {
		switch {
		case r == '\\':
			b.WriteString(`\\`)
		case string(r) == quote:
			b.WriteString(`\` + quote)
		case r == '\n':
			b.WriteString(`\n`)
		case r == '\r':
			b.WriteString(`\r`)
		case r == '\t':
			b.WriteString(`\t`)
		case r < 0x20 || r == 0x7f:
			fmt.Fprintf(&b, `\x%02x`, r)
		default:
			b.WriteRune(r)
		}
	}
	b.WriteString(quote)
	return b.String()
}

// Round is round(x): to the nearest integer, a tie to the even one.
func Round(x float64) int { return int(math.RoundToEven(x)) }

// RoundN is round(x, n): correctly rounded to n decimals, as Python's is.
func RoundN(x float64, n int) float64 {
	f, _ := strconv.ParseFloat(strconv.FormatFloat(x, 'f', n, 64), 64)
	return f
}

// FloorDiv is a // b for ints: rounded toward negative infinity.
func FloorDiv(a, b int) int {
	q := a / b
	if (a%b != 0) && ((a < 0) != (b < 0)) {
		q--
	}
	return q
}

// Mod is a % b for ints: the sign of b.
func Mod(a, b int) int {
	m := a % b
	if m != 0 && ((m < 0) != (b < 0)) {
		m += b
	}
	return m
}

// FloorDivF is a // b for floats.
func FloorDivF(a, b float64) float64 { return math.Floor(a / b) }

// Splitlines is str.splitlines(): it splits at \n, \r\n, \r, \v, \f, \x1c-\x1e, \x85,
// and  , and a trailing break makes no empty last line.
func Splitlines(s string) []string {
	var out []string
	start := 0
	rs := []rune(s)
	for i := 0; i < len(rs); i++ {
		switch rs[i] {
		case '\n', '\v', '\f', 0x1c, 0x1d, 0x1e, 0x85, 0x2028, 0x2029:
			out = append(out, string(rs[start:i]))
			start = i + 1
		case '\r':
			out = append(out, string(rs[start:i]))
			if i+1 < len(rs) && rs[i+1] == '\n' {
				i++
			}
			start = i + 1
		}
	}
	if start < len(rs) {
		out = append(out, string(rs[start:]))
	}
	return out
}

// Thousands is f"{n:,}".
func Thousands(n int) string {
	s := strconv.Itoa(n)
	neg := strings.HasPrefix(s, "-")
	s = strings.TrimPrefix(s, "-")
	var b strings.Builder
	for i, r := range s {
		if i > 0 && (len(s)-i)%3 == 0 {
			b.WriteByte(',')
		}
		b.WriteRune(r)
	}
	if neg {
		return "-" + b.String()
	}
	return b.String()
}

var (
	fnmatchMu    sync.Mutex
	fnmatchCache = map[string]*regexp.Regexp{}
)

// Fnmatch is fnmatch.fnmatch(name, pattern) on POSIX (case-sensitive): `*` matches any run of
// characters, `/` included, `?` any one, `[seq]` and `[!seq]` a set.
func Fnmatch(name, pattern string) bool {
	fnmatchMu.Lock()
	re, ok := fnmatchCache[pattern]
	if !ok {
		re = regexp.MustCompile(fnmatchTranslate(pattern))
		fnmatchCache[pattern] = re
	}
	fnmatchMu.Unlock()
	return re.MatchString(name)
}

// fnmatchTranslate is fnmatch.translate.
func fnmatchTranslate(pat string) string {
	var b strings.Builder
	b.WriteString(`(?s)^`)
	rs := []rune(pat)
	for i := 0; i < len(rs); i++ {
		c := rs[i]
		switch c {
		case '*':
			for i+1 < len(rs) && rs[i+1] == '*' {
				i++
			}
			b.WriteString(`.*`)
		case '?':
			b.WriteString(`.`)
		case '[':
			j := i + 1
			if j < len(rs) && rs[j] == '!' {
				j++
			}
			if j < len(rs) && rs[j] == ']' {
				j++
			}
			for j < len(rs) && rs[j] != ']' {
				j++
			}
			if j >= len(rs) {
				b.WriteString(`\[`)
				continue
			}
			stuff := string(rs[i+1 : j])
			i = j
			stuff = strings.ReplaceAll(stuff, `\`, `\\`)
			if strings.HasPrefix(stuff, "!") {
				stuff = "^" + stuff[1:]
			} else if strings.HasPrefix(stuff, "^") {
				stuff = `\` + stuff
			}
			b.WriteString("[" + stuff + "]")
		default:
			b.WriteString(regexp.QuoteMeta(string(c)))
		}
	}
	b.WriteString(`$`)
	return b.String()
}
