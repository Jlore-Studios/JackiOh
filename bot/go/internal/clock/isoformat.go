package clock

import (
	"fmt"
	"math"
	"strings"
	"time"
	"unicode"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// This file is what parse_iso leans on from Python itself: datetime.fromisoformat as Python 3.13's
// C module reads a string (Lib/_pydatetime.py's algorithm, which it follows but for one case
// noted below), int() of a string, and str.strip().

// overflowError is Python's OverflowError, which parse_iso and parse_hhmm let through.
type overflowError struct{ msg string }

func (e *overflowError) Error() string { return e.msg }

// fromISOFormat is `datetime.fromisoformat(s).astimezone(timezone.utc)` (a naive result read
// as UTC), and false where Python raises ValueError. A result outside years 1 to 9999 raises
// (panics with) Python's OverflowError, as astimezone does; parse_iso does not catch it.
func fromISOFormat(s string) (time.Time, bool) {
	rs := []rune(s)
	if len(rs) < 7 {
		return time.Time{}, false
	}
	sep, ok := isoSeparator(rs)
	if !ok {
		return time.Time{}, false
	}
	date, ok := isoDate(subRunes(rs, 0, sep))
	if !ok {
		return time.Time{}, false
	}
	tstr := subRunes(rs, sep+1, len(rs))
	var clock [4]int
	var offset time.Duration
	// The C module reads a time whenever a separator is there, so a separator with nothing
	// after it is refused (_pydatetime would take the date alone).
	if len(rs) > sep {
		if clock, offset, ok = isoTime(tstr); !ok {
			return time.Time{}, false
		}
	}
	year, month, day := date[0], date[1], date[2]
	if year < 1 || year > 9999 || month < 1 || month > 12 || day < 1 ||
		day > daysIn(year, month) || clock[0] > 23 || clock[1] > 59 || clock[2] > 59 {
		return time.Time{}, false
	}
	if offset <= -24*time.Hour || offset >= 24*time.Hour {
		return time.Time{}, false
	}
	local := time.Date(year, time.Month(month), day, clock[0], clock[1], clock[2], clock[3]*1000,
		time.UTC)
	utc := local.Add(-offset)
	if utc.Year() < 1 || utc.Year() > 9999 {
		panic(&overflowError{"date value out of range"})
	}
	return utc, true
}

func daysIn(year, month int) int {
	return time.Date(year, time.Month(month)+1, 0, 0, 0, 0, 0, time.UTC).Day()
}

func subRunes(rs []rune, a, b int) []rune {
	if a > len(rs) {
		a = len(rs)
	}
	if b > len(rs) {
		b = len(rs)
	}
	if a >= b {
		return nil
	}
	return rs[a:b]
}

func isASCIIDigit(r rune) bool { return r >= '0' && r <= '9' }

func isASCII(rs []rune) bool {
	for _, r := range rs {
		if r > 0x7f {
			return false
		}
	}
	return true
}

// isoSeparator is _find_isoformat_datetime_separator.
func isoSeparator(s []rune) (int, bool) {
	n := len(s)
	if n == 7 {
		return 7, true
	}
	if s[4] == '-' {
		if s[5] == 'W' {
			if n > 8 && s[8] == '-' {
				if n == 9 {
					return 0, false
				}
				if n > 10 && isASCIIDigit(s[10]) {
					return 8, true
				}
				return 10, true
			}
			return 8, true // YYYY-Www (8)
		}
		return 10, true // YYYY-MM-DD (10)
	}
	if s[4] == 'W' {
		// YYYYWww (7) or YYYYWwwd (8)
		idx := 7
		for idx < n && isASCIIDigit(s[idx]) {
			idx++
		}
		if idx < 9 {
			return idx, true
		}
		if idx%2 == 0 {
			return 7, true // the index of the last number is even: YYYYWwwd
		}
		return 8, true
	}
	return 8, true // YYYYMMDD (8)
}

// isoComponent is _read_isoformat_component: exactly n ASCII digits.
func isoComponent(s []rune, n int) (int, bool) {
	if len(s) != n {
		return 0, false
	}
	v := 0
	for _, r := range s {
		if !isASCIIDigit(r) {
			return 0, false
		}
		v = v*10 + int(r-'0')
	}
	return v, true
}

// isoDate is _parse_isoformat_date.
func isoDate(d []rune) ([3]int, bool) {
	var none [3]int
	if n := len(d); n != 7 && n != 8 && n != 10 {
		return none, false
	}
	if !isASCII(d) {
		return none, false
	}
	year, ok := isoComponent(subRunes(d, 0, 4), 4)
	if !ok {
		return none, false
	}
	hasSep := d[4] == '-'
	pos := 4
	if hasSep {
		pos++
	}
	if string(subRunes(d, pos, pos+1)) == "W" {
		// YYYY-?Www-?D?
		pos++
		week, ok := isoComponent(subRunes(d, pos, pos+2), 2)
		if !ok {
			return none, false
		}
		pos += 2
		weekday := 1
		if len(d) > pos {
			if (string(subRunes(d, pos, pos+1)) == "-") != hasSep {
				return none, false // inconsistent use of dash separator
			}
			if hasSep {
				pos++
			}
			if weekday, ok = isoComponent(subRunes(d, pos, pos+1), 1); !ok {
				return none, false
			}
		}
		return isoWeekToGregorian(year, week, weekday)
	}
	month, ok := isoComponent(subRunes(d, pos, pos+2), 2)
	if !ok {
		return none, false
	}
	pos += 2
	if (string(subRunes(d, pos, pos+1)) == "-") != hasSep {
		return none, false // inconsistent use of dash separator
	}
	if hasSep {
		pos++
	}
	day, ok := isoComponent(subRunes(d, pos, pos+2), 2)
	if !ok {
		return none, false
	}
	return [3]int{year, month, day}, true
}

// isoWeekToGregorian is _isoweek_to_gregorian.
func isoWeekToGregorian(year, week, day int) ([3]int, bool) {
	var none [3]int
	if year < 1 || year > 9999 {
		return none, false
	}
	jan1 := time.Date(year, 1, 1, 0, 0, 0, 0, time.UTC)
	if week < 1 || week > 52 {
		leap := daysIn(year, 2) == 29
		first := int(jan1.Weekday()) // _ymd2ord(year, 1, 1) % 7
		if !(week == 53 && (first == 4 || (first == 3 && leap))) {
			return none, false
		}
	}
	if day < 1 || day > 7 {
		return none, false
	}
	// _isoweek1monday
	firstWeekday := pyWeekday(jan1)
	monday := jan1.AddDate(0, 0, -firstWeekday)
	if firstWeekday > 3 {
		monday = monday.AddDate(0, 0, 7)
	}
	found := monday.AddDate(0, 0, (week-1)*7+(day-1))
	return [3]int{found.Year(), int(found.Month()), found.Day()}, true
}

var fractionCorrection = []int{100000, 10000, 1000, 100, 10}

// isoHHMMSSFF is _parse_hh_mm_ss_ff: HH[:?MM[:?SS[{.,}fff[fff]]]].
func isoHHMMSSFF(t []rune) ([4]int, bool) {
	var comps [4]int
	n := len(t)
	pos := 0
	hasSep := false
	for comp := 0; comp < 3; comp++ {
		v, ok := isoComponent(subRunes(t, pos, pos+2), 2)
		if !ok {
			return comps, false
		}
		comps[comp] = v
		pos += 2
		next := string(subRunes(t, pos, pos+1))
		if comp == 0 {
			hasSep = next == ":"
		}
		if next == "" || comp >= 2 {
			break
		}
		if hasSep && next != ":" {
			return comps, false // invalid time separator
		}
		if hasSep {
			pos++
		}
	}
	if pos < n {
		if t[pos] != '.' && t[pos] != ',' {
			return comps, false // invalid microsecond component
		}
		pos++
		rest := subRunes(t, pos, n)
		if len(rest) == 0 {
			return comps, false
		}
		for _, r := range rest {
			if !isASCIIDigit(r) {
				return comps, false // non-digit values in fraction
			}
		}
		toParse := min(len(rest), 6)
		v, _ := isoComponent(rest[:toParse], toParse)
		if toParse < 6 {
			v *= fractionCorrection[toParse-1]
		}
		comps[3] = v
	}
	return comps, true
}

// isoTime is _parse_isoformat_time: the time's fields and its UTC offset (0 when naive).
func isoTime(t []rune) ([4]int, time.Duration, bool) {
	var none [4]int
	n := len(t)
	if n < 2 || !isASCII(t) {
		return none, 0, false
	}
	s := string(t) // ASCII from here: byte and code point indices agree
	tzPos := strings.IndexByte(s, '-') + 1
	if tzPos == 0 {
		tzPos = strings.IndexByte(s, '+') + 1
	}
	if tzPos == 0 {
		tzPos = strings.IndexByte(s, 'Z') + 1
	}
	timestr := t
	if tzPos > 0 {
		timestr = t[:tzPos-1]
	}
	comps, ok := isoHHMMSSFF(timestr)
	if !ok {
		return none, 0, false
	}
	var offset time.Duration
	if tzPos == n && s[n-1] == 'Z' {
		offset = 0
	} else if tzPos > 0 {
		tzstr := t[tzPos:]
		// Valid time zone strings: HH, HHMM, HH:MM, HHMMSS, HHMMSS.f+, HH:MM:SS, HH:MM:SS.f+
		if l := len(tzstr); l == 0 || l == 1 || l == 3 {
			return none, 0, false
		}
		tz, ok := isoHHMMSSFF(tzstr)
		if !ok {
			return none, 0, false
		}
		offset = time.Duration(tz[0])*time.Hour + time.Duration(tz[1])*time.Minute +
			time.Duration(tz[2])*time.Second + time.Duration(tz[3])*time.Microsecond
		if s[tzPos-1] == '-' {
			offset = -offset
		}
	}
	return comps, offset, true
}

// pyInt is int(s) for a string, base 10, with Python's message on a ValueError: Unicode
// whitespace around it and Unicode decimal digits read as Python reads them, a sign, and single
// underscores between digits. `big` says the value is beyond a C long (Python's int holds it;
// whatever takes it as a C integer raises OverflowError).
func pyInt(s string) (n int, big bool, err error) {
	invalid := fmt.Errorf("invalid literal for int() with base 10: %s", py.SliceTo(py.Repr(s), 200))
	var b strings.Builder
	for _, r := range s {
		switch {
		case r < 0x80:
			b.WriteRune(r)
		case py.IsSpace(r):
			b.WriteByte(' ')
		default:
			if d, ok := decimalValue(r); ok {
				b.WriteByte(byte('0' + d))
			} else {
				b.WriteByte('?')
			}
		}
	}
	t := strings.Trim(b.String(), " \t\n\v\f\r")
	neg := false
	if t != "" && (t[0] == '+' || t[0] == '-') {
		neg = t[0] == '-'
		t = t[1:]
	}
	if t == "" || t[0] == '_' || t[len(t)-1] == '_' || strings.Contains(t, "__") {
		return 0, false, invalid
	}
	limit := uint64(math.MaxInt64)
	if neg {
		limit++
	}
	var u uint64
	for i := 0; i < len(t); i++ {
		c := t[i]
		if c == '_' {
			continue
		}
		if c < '0' || c > '9' {
			return 0, false, invalid
		}
		d := uint64(c - '0')
		if big || u > (limit-d)/10 {
			big = true
			continue
		}
		u = u*10 + d
	}
	if big {
		return 0, true, nil
	}
	if neg {
		return int(-u), false, nil
	}
	return int(u), false, nil
}

// decimalValue is the digit a Unicode decimal digit (category Nd) stands for: every Nd range
// runs in whole sets of ten from a zero.
func decimalValue(r rune) (int, bool) {
	if !unicode.IsDigit(r) {
		return 0, false
	}
	for _, rg := range unicode.Nd.R16 {
		if lo, hi := rune(rg.Lo), rune(rg.Hi); r >= lo && r <= hi {
			return int(r-lo) / int(rg.Stride) % 10, true
		}
	}
	for _, rg := range unicode.Nd.R32 {
		if lo, hi := rune(rg.Lo), rune(rg.Hi); r >= lo && r <= hi {
			return int(r-lo) / int(rg.Stride) % 10, true
		}
	}
	return 0, false
}
