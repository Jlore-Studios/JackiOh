package providers

// The Python behaviours this module's logic depends on, kept private here: isinstance checks on
// JSON values, int() and float() of whatever providers.json or the state file holds, the `%`
// format, a datetime in a zone with fold=0, date.fromisoformat and pathlib's joining.

import (
	"fmt"
	"math"
	"sort"
	"strconv"
	"strings"
	"time"
	"unicode"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// pySpace is Python's `\s` (str.isspace) as an RE2 class; RE2's own `\s` is ASCII.
const pySpace = `[\t\n\v\f\r\x1c-\x1f\x{85}\p{Z}]`

// mapping is isinstance(v, Mapping): v as an object. A nil *pyjson.Object is None.
func mapping(v any) (*pyjson.Object, bool) {
	o, ok := v.(*pyjson.Object)
	return o, ok && o != nil
}

// isNone is `v is None`, a nil *pyjson.Object included.
func isNone(v any) bool {
	if v == nil {
		return true
	}
	o, ok := v.(*pyjson.Object)
	return ok && o == nil
}

// noneOrEmpty is `raw in (None, {})`.
func noneOrEmpty(raw any) bool {
	if isNone(raw) {
		return true
	}
	o, ok := raw.(*pyjson.Object)
	return ok && o.Len() == 0
}

// unknownKeys is sorted(set(obj) - set(allowed)).
func unknownKeys(obj *pyjson.Object, allowed []string) []string {
	var unknown []string
	for _, k := range obj.Keys() {
		if !contains(allowed, k) {
			unknown = append(unknown, k)
		}
	}
	sort.Strings(unknown)
	return unknown
}

// repeated is sorted({x for x in xs if xs.count(x) > 1}).
func repeated(xs []string) []string {
	count := map[string]int{}
	for _, x := range xs {
		count[x]++
	}
	var twice []string
	for x, n := range count {
		if n > 1 {
			twice = append(twice, x)
		}
	}
	sort.Strings(twice)
	return twice
}

func contains(xs []string, x string) bool { return index(xs, x) >= 0 }

func index(xs []string, x string) int {
	for i, y := range xs {
		if y == x {
			return i
		}
	}
	return -1
}

// sortedNames is sorted(a set of names).
func sortedNames(set map[string]bool) []string {
	names := []string{}
	for name := range set {
		names = append(names, name)
	}
	sort.Strings(names)
	return names
}

// iterate is Python's iteration over a JSON value: a list's items, a string's characters, an
// object's keys. Anything else raises TypeError, as iterating it does.
func iterate(v any) []any {
	switch x := v.(type) {
	case []any:
		return x
	case string:
		out := []any{}
		for _, r := range x {
			out = append(out, string(r))
		}
		return out
	case *pyjson.Object:
		return pyjson.ListOf(x.Keys())
	}
	panic(fmt.Errorf("TypeError: '%s' object is not iterable", typeName(v)))
}

// listOf is list(v or []).
func listOf(v any) []any {
	if !pyjson.Truthy(v) {
		return []any{}
	}
	return append([]any{}, iterate(v)...)
}

// lastN is xs[-n:].
func lastN(xs []any, n int) []any {
	if len(xs) > n {
		xs = xs[len(xs)-n:]
	}
	return append([]any{}, xs...)
}

// orEmpty is str(v or "").
func orEmpty(v any) string {
	if !pyjson.Truthy(v) {
		return ""
	}
	return py.Str(v)
}

// orZero is `v or 0`.
func orZero(v any) any { return orValue(v, 0) }

// orValue is `v or fallback`.
func orValue(v, fallback any) any {
	if pyjson.Truthy(v) {
		return v
	}
	return fallback
}

// reprStr is repr() of a string.
func reprStr(s string) string { return py.Repr(s) }

// parseISO is clock.parse_iso of any JSON value: None (the zero time) for a falsy one, else the
// time its str() reads as.
func parseISO(v any) time.Time {
	if !pyjson.Truthy(v) {
		return time.Time{}
	}
	return clock.ParseISO(py.Str(v))
}

// number is isinstance(v, (int, float)): a bool is an int in Python, so it counts as 0 or 1.
func number(v any) (float64, bool) {
	if b, ok := v.(bool); ok {
		if b {
			return 1, true
		}
		return 0, true
	}
	return pyjson.AsFloat(v)
}

// typeName is type(v).__name__ for a JSON value.
func typeName(v any) string {
	switch v.(type) {
	case nil:
		return "NoneType"
	case bool:
		return "bool"
	case int:
		return "int"
	case float64:
		return "float"
	case string:
		return "str"
	case []any:
		return "list"
	case *pyjson.Object:
		return "dict"
	}
	return fmt.Sprintf("%T", v)
}

// pyInt is int(v): an int, a bool, a float (truncated), or a string of decimal digits with
// Python's whitespace, sign and underscores. The error stands for ValueError or TypeError.
func pyInt(v any) (int, error) {
	switch x := v.(type) {
	case bool:
		if x {
			return 1, nil
		}
		return 0, nil
	case int:
		return x, nil
	case float64:
		if math.IsNaN(x) || math.IsInf(x, 0) {
			return 0, fmt.Errorf("ValueError: cannot convert float %s to integer", pyjson.FloatRepr(x))
		}
		return int(x), nil
	case string:
		s := strings.TrimSpace(x)
		negative := false
		if s != "" && (s[0] == '+' || s[0] == '-') {
			negative = s[0] == '-'
			s = s[1:]
		}
		digits, ok := pyDigits(s, false)
		if !ok || digits == "" {
			return 0, fmt.Errorf("ValueError: invalid literal for int() with base 10: %s", reprStr(x))
		}
		n, err := strconv.Atoi(digits)
		if err != nil {
			return 0, fmt.Errorf("ValueError: invalid literal for int() with base 10: %s", reprStr(x))
		}
		if negative {
			n = -n
		}
		return n, nil
	}
	return 0, fmt.Errorf("TypeError: int() argument must be a string, a bytes-like object or a "+
		"real number, not '%s'", typeName(v))
}

// mustInt is int(v) where the Python lets its error go up.
func mustInt(v any) int {
	n, err := pyInt(v)
	if err != nil {
		panic(err)
	}
	return n
}

// pyFloat is float(v): a number, a bool, or a string Python's float() reads (whitespace,
// underscores between digits, inf and nan, any decimal digits).
func pyFloat(v any) (float64, error) {
	switch x := v.(type) {
	case bool:
		if x {
			return 1, nil
		}
		return 0, nil
	case int:
		return float64(x), nil
	case float64:
		return x, nil
	case string:
		s, ok := pyDigits(strings.TrimSpace(x), true)
		if ok && !strings.ContainsAny(s, "xXpP_") && s != "" {
			f, err := strconv.ParseFloat(s, 64)
			if err == nil || err.(*strconv.NumError).Err == strconv.ErrRange {
				return f, nil
			}
		}
		return 0, fmt.Errorf("ValueError: could not convert string to float: %s", reprStr(x))
	}
	return 0, fmt.Errorf("TypeError: float() argument must be a string or a real number, not '%s'",
		typeName(v))
}

// mustFloat is float(v) where the Python lets its error go up.
func mustFloat(v any) float64 {
	f, err := pyFloat(v)
	if err != nil {
		panic(err)
	}
	return f
}

// pyDigits rewrites Python's decimal digits in s as ASCII and drops the underscores Python allows
// (one, between two digits); false when an underscore stands anywhere else. Without `other`, any
// character but a digit or an underscore is refused too.
func pyDigits(s string, other bool) (string, bool) {
	rs := []rune(s)
	var b strings.Builder
	for i, r := range rs {
		switch {
		case unicode.Is(unicode.Nd, r):
			b.WriteByte(byte('0' + digitValue(r)))
		case r == '_':
			if i == 0 || i == len(rs)-1 || !unicode.Is(unicode.Nd, rs[i-1]) || !unicode.Is(unicode.Nd, rs[i+1]) {
				return "", false
			}
		case other:
			b.WriteRune(r)
		default:
			return "", false
		}
	}
	return b.String(), true
}

// digitValue is the value of a decimal digit (Unicode category Nd), what Python's int() reads:
// Unicode assigns them in runs of ten, 0 to 9, so a digit's value is its place in its run.
func digitValue(r rune) int {
	if r >= '0' && r <= '9' {
		return int(r - '0')
	}
	k := 0
	for unicode.Is(unicode.Nd, r-rune(k)-1) {
		k++
	}
	return k % 10
}

// digitsInt is int() of what `\d+` matched, 0 for an empty (unmatched) group.
func digitsInt(s string) int {
	n := 0
	for _, r := range s {
		n = n*10 + digitValue(r)
	}
	return n
}

// percent is f"{x:.0%}": x times 100, rounded half to even, and a percent sign.
func percent(x float64) string { return fmt.Sprintf("%.0f%%", x*100) }

// pyMin is min(a, b): a unless b is smaller.
func pyMin(a, b float64) float64 {
	if b < a {
		return b
	}
	return a
}

// pyMax is max(a, b): a unless b is larger.
func pyMax(a, b float64) float64 {
	if b > a {
		return b
	}
	return a
}

func pyMod(a, b int) int { return py.Mod(a, b) }

// weekday is datetime.weekday(): Monday 0 to Sunday 6.
func weekday(t time.Time) int { return (int(t.Weekday()) + 6) % 7 }

// localTime is datetime(year, month, day, hour, minute, tzinfo=zone) as an instant, with
// Python's fold=0: a wall time a transition skips (spring forward) or repeats (fall back) takes
// the offset from before the transition. Go's time.Date picks either in a gap; this does what
// zoneinfo does, for any zone with at most one transition within a day of the wall time.
func localTime(year int, month time.Month, day, hour, minute int, zone *time.Location) time.Time {
	wall := time.Date(year, month, day, hour, minute, 0, 0, time.UTC).Unix()
	offsetAt := func(unix int64) int64 {
		_, offset := time.Unix(unix, 0).In(zone).Zone()
		return int64(offset)
	}
	widest := max(offsetAt(wall-14*3600), offsetAt(wall+14*3600))
	return time.Unix(wall-offsetAt(wall-widest), 0).In(zone)
}

// dateFromISO is date.fromisoformat (CPython 3.13's C parser): YYYY-MM-DD, YYYYMMDD, and the ISO
// week forms YYYY-Www, YYYY-Www-D, YYYYWww and YYYYWwwD, as a date at midnight UTC. Like the C
// parser it reads a 10-character string without dashes as YYYYMMDD and ignores the rest.
func dateFromISO(s string) (time.Time, bool) {
	n := len(s)
	if n != 7 && n != 8 && n != 10 {
		return time.Time{}, false
	}
	pos := 0
	digits := func(count int) (int, bool) {
		v := 0
		for i := 0; i < count; i++ {
			if pos >= n || s[pos] < '0' || s[pos] > '9' {
				return 0, false
			}
			v = v*10 + int(s[pos]-'0')
			pos++
		}
		return v, true
	}
	year, ok := digits(4)
	if !ok {
		return time.Time{}, false
	}
	dashes := pos < n && s[pos] == '-'
	if dashes {
		pos++
	}
	if pos < n && s[pos] == 'W' {
		pos++
		week, ok := digits(2)
		if !ok {
			return time.Time{}, false
		}
		day := 1
		if pos < n {
			if dashes {
				if s[pos] != '-' {
					return time.Time{}, false
				}
				pos++
			}
			if day, ok = digits(1); !ok {
				return time.Time{}, false
			}
		}
		return isoWeekDate(year, week, day)
	}
	month, ok := digits(2)
	if !ok {
		return time.Time{}, false
	}
	if dashes {
		if pos >= n || s[pos] != '-' {
			return time.Time{}, false
		}
		pos++
	}
	day, ok := digits(2)
	if !ok || year < 1 || month < 1 || month > 12 || day < 1 {
		return time.Time{}, false
	}
	date := time.Date(year, time.Month(month), day, 0, 0, 0, 0, time.UTC)
	if date.Day() != day { // past the month's last day
		return time.Time{}, false
	}
	return date, true
}

// isoWeekDate is the C parser's iso_to_ymd.
func isoWeekDate(year, week, day int) (time.Time, bool) {
	if year < 1 || year > 9999 {
		return time.Time{}, false
	}
	if week <= 0 || week >= 53 {
		first := weekday(time.Date(year, 1, 1, 0, 0, 0, 0, time.UTC))
		leap := year%4 == 0 && (year%100 != 0 || year%400 == 0)
		if week != 53 || !(first == 3 || (first == 2 && leap)) {
			return time.Time{}, false
		}
	}
	if day <= 0 || day >= 8 {
		return time.Time{}, false
	}
	jan4 := time.Date(year, 1, 4, 0, 0, 0, 0, time.UTC)
	monday := jan4.AddDate(0, 0, -weekday(jan4))
	return monday.AddDate(0, 0, (week-1)*7+day-1), true
}

// purePath is str(PurePosixPath(*parts)): joined with "/", empty and "." parts dropped, ".."
// kept, an absolute part starting afresh (two leading slashes kept, as POSIX allows).
func purePath(parts ...string) string {
	root := ""
	var segments []string
	for _, part := range parts {
		if strings.HasPrefix(part, "/") {
			segments = nil
			root = "/"
			if strings.HasPrefix(part, "//") && !strings.HasPrefix(part, "///") {
				root = "//"
			}
		}
		for _, s := range strings.Split(part, "/") {
			if s != "" && s != "." {
				segments = append(segments, s)
			}
		}
	}
	if path := root + strings.Join(segments, "/"); path != "" {
		return path
	}
	return "."
}
