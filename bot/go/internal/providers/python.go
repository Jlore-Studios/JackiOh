package providers

// The Python behaviours this module's logic depends on that py does not hold: isinstance checks
// on JSON values, a datetime in a zone with fold=0, date.fromisoformat and pathlib's joining.

import (
	"slices"
	"sort"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

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
		if !slices.Contains(allowed, k) {
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

// sortedNames is sorted(a set of names).
func sortedNames(set map[string]bool) []string {
	names := []string{}
	for name := range set {
		names = append(names, name)
	}
	sort.Strings(names)
	return names
}

// listOf is list(v or []).
func listOf(v any) []any { return append([]any{}, py.Each(v)...) }

// lastN is xs[-n:].
func lastN(xs []any, n int) []any {
	if len(xs) > n {
		xs = xs[len(xs)-n:]
	}
	return append([]any{}, xs...)
}

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
