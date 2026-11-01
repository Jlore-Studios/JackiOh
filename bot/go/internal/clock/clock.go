// Package clock is bot/harness/clock.py.
//
// Time: the current instant, ISO formatting, and the nightly run window.
package clock

import (
	"fmt"
	"math"
	"path/filepath"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// TimeOfDay is Python's datetime.time: a wall-clock time with no date or zone.
type TimeOfDay struct {
	Hour, Minute, Second int
}

// Compare is Python's ordering of two `time`s: -1, 0 or 1.
func (t TimeOfDay) Compare(o TimeOfDay) int {
	a, b := t.seconds(), o.seconds()
	switch {
	case a < b:
		return -1
	case a > b:
		return 1
	}
	return 0
}

func (t TimeOfDay) seconds() int { return (t.Hour*60+t.Minute)*60 + t.Second }

// TimeOf is `dt.time()`: the wall clock of `dt` in its own location, to the second. Python's
// `time` also keeps the microseconds; every TimeOfDay the harness compares with is a whole
// minute, so dropping them changes no comparison.
func TimeOf(dt time.Time) TimeOfDay {
	return TimeOfDay{dt.Hour(), dt.Minute(), dt.Second()}
}

// ISO is `dt` as ISO-8601 UTC with a trailing Z, to the second.
func ISO(dt time.Time) string {
	u := dt.UTC()
	// strftime's %Y does not pad a year below 1000 on glibc; Go's "2006" would.
	return fmt.Sprintf("%d-%02d-%02dT%02d:%02d:%02dZ", u.Year(), int(u.Month()), u.Day(),
		u.Hour(), u.Minute(), u.Second())
}

// ParseISO is an aware UTC datetime from ISO text, or None (the zero time) when `text` is empty
// or unreadable. (So 0001-01-01T00:00:00Z itself reads as None.) A text whose instant falls
// outside years 1 to 9999 raises (panics with) Python's OverflowError, which Python does not
// catch here either.
func ParseISO(text string) time.Time {
	if text == "" {
		return time.Time{}
	}
	parsed, ok := fromISOFormat(strings.ReplaceAll(py.Strip(text), "Z", "+00:00"))
	if !ok {
		return time.Time{}
	}
	return parsed
}

// ParseISOValue is parse_iso of a JSON value, as the Python passes whatever a record holds:
// None (the zero time) for a falsy value, else the time its str() reads as.
func ParseISOValue(v any) time.Time {
	if !pyjson.Truthy(v) {
		return time.Time{}
	}
	return ParseISO(py.Str(v))
}

// Now is the current instant, or `override` (HARNESS_NOW) when it parses.
func Now(override string /* = "" */) time.Time {
	fixed := ParseISO(override)
	if !fixed.IsZero() {
		return fixed
	}
	// datetime.now() keeps microseconds and has no monotonic reading.
	return time.Now().UTC().Truncate(time.Microsecond)
}

// Zone is the named IANA zone. America/Chicago falls back to fixed US rules without tzdata.
func Zone(name string) *time.Location {
	validateZoneKey(name)
	// zoneinfo has no "Local", and Go's LoadLocation would read it as the host's zone.
	if name != "Local" {
		if loc, err := time.LoadLocation(name); err == nil {
			return loc
		}
	}
	if name == "America/Chicago" {
		return usCentralLocation()
	}
	return time.UTC
}

// validateZoneKey raises the ValueError zoneinfo raises for a key that is no relative path
// (zoneinfo._tzpath.validate_tzfile_path); zone() does not catch it.
func validateZoneKey(name string) {
	if strings.HasPrefix(name, "/") {
		panic(fmt.Errorf("ZoneInfo keys may not be absolute paths, got: %s", name))
	}
	clean := filepath.Clean(name)
	if len(clean) != len(name) {
		panic(fmt.Errorf("ZoneInfo keys must be normalized relative paths, got: %s", name))
	}
	if resolved := filepath.Clean("_/" + clean); !strings.HasPrefix(resolved, "_/") {
		panic(fmt.Errorf("ZoneInfo keys must refer to subdirectories of TZPATH, got: %s", name))
	}
}

// UsCentral is US Central time with the 2007 DST rules, for hosts without tzdata. A naive
// datetime is a time.Time whose wall clock (in any location) is read as is.
type UsCentral struct {
}

func (x *UsCentral) dstBounds(year int) (time.Time, time.Time) {
	march := time.Date(year, 3, 8, 0, 0, 0, 0, time.UTC)
	start := march.AddDate(0, 0, py.Mod(6-pyWeekday(march), 7))
	november := time.Date(year, 11, 1, 0, 0, 0, 0, time.UTC)
	end := november.AddDate(0, 0, py.Mod(6-pyWeekday(november), 7))
	return start.Add(2 * time.Hour), end.Add(2 * time.Hour)
}

// pyWeekday is date.weekday(): Monday 0 to Sunday 6.
func pyWeekday(t time.Time) int { return (int(t.Weekday()) + 6) % 7 }

// naive is dt.replace(tzinfo=None): its wall clock, as a UTC time.
func naive(dt time.Time) time.Time {
	return time.Date(dt.Year(), dt.Month(), dt.Day(), dt.Hour(), dt.Minute(), dt.Second(),
		dt.Nanosecond(), time.UTC)
}

func (x *UsCentral) Utcoffset(dt time.Time) time.Duration {
	return -6*time.Hour + x.Dst(dt)
}

// Dst is the DST offset at the wall clock `dt`; the zero time stands for None.
func (x *UsCentral) Dst(dt time.Time) time.Duration {
	if dt.IsZero() {
		return 0
	}
	start, end := x.dstBounds(dt.Year())
	at := naive(dt)
	if !at.Before(start) && at.Before(end) {
		return time.Hour
	}
	return 0
}

func (x *UsCentral) Tzname(dt time.Time) string {
	if x.Dst(dt) != 0 {
		return "CDT"
	}
	return "CST"
}

// Fromutc is the Central wall clock of the UTC instant `dt`, in a fixed zone named CST or CDT.
func (x *UsCentral) Fromutc(dt time.Time) time.Time {
	standard := naive(dt.UTC()).Add(-6 * time.Hour)
	start, end := x.dstBounds(standard.Year())
	// DST starts at 02:00 standard time and ends at 02:00 daylight time (01:00 standard).
	if !standard.Before(start) && standard.Before(end.Add(-time.Hour)) {
		return dt.In(time.FixedZone("CDT", -5*3600))
	}
	return dt.In(time.FixedZone("CST", -6*3600))
}

// usCentralLocation is _UsCentral as a Location: one transition long ago into CST, then the
// 2007 rules (`CST6CDT,M3.2.0,M11.1.0`) for every year after, written as TZif data.
func usCentralLocation() *time.Location {
	var b []byte
	be32 := func(v uint32) { b = append(b, byte(v>>24), byte(v>>16), byte(v>>8), byte(v)) }
	be64 := func(v uint64) { be32(uint32(v >> 32)); be32(uint32(v)) }
	header := func(timecnt, typecnt, charcnt uint32) {
		b = append(b, "TZif2"...)
		b = append(b, make([]byte, 15)...)
		for _, n := range []uint32{0, 0, 0, timecnt, typecnt, charcnt} {
			be32(n)
		}
	}
	cst, first := int32(-6*3600), int64(math.MinInt32)*64 // long before any date the harness reads
	zone := func() {
		be32(uint32(cst))
		b = append(b, 0, 0) // not DST, name at 0
		b = append(b, "CST\x00"...)
	}
	header(0, 1, 4)
	zone()
	header(1, 1, 4)
	be64(uint64(first))
	b = append(b, 0)
	zone()
	b = append(b, "\nCST6CDT,M3.2.0,M11.1.0\n"...)
	loc, err := time.LoadLocationFromTZData("America/Chicago", b)
	if err != nil {
		panic(err)
	}
	return loc
}

// ParseHhmm is clock.parse_hhmm. Its error is the ValueError Python raises, with Python's
// message; an hour or minute beyond a C int raises (panics with) the OverflowError time()
// raises, which no caller catches.
func ParseHhmm(text string) (TimeOfDay, error) {
	parts := strings.Split(py.Strip(text), ":")
	if len(parts) < 2 {
		return TimeOfDay{}, &py.Error{Type: "ValueError",
			Msg: fmt.Sprintf("not enough values to unpack (expected 2, got %d)", len(parts))}
	}
	if len(parts) > 2 {
		return TimeOfDay{}, &py.Error{Type: "ValueError", Msg: "too many values to unpack (expected 2)"}
	}
	hours, hoursBig, err := py.IntText(parts[0])
	if err != nil {
		return TimeOfDay{}, err
	}
	minutes, minutesBig, err := py.IntText(parts[1])
	if err != nil {
		return TimeOfDay{}, err
	}
	// time()'s arguments are C ints, all read before any is checked.
	for _, v := range []struct {
		n   int
		big bool
	}{{hours, hoursBig}, {minutes, minutesBig}} {
		switch {
		case v.big:
			panic(&py.Error{Type: "OverflowError", Msg: "Python int too large to convert to C long"})
		case v.n > math.MaxInt32:
			panic(&py.Error{Type: "OverflowError", Msg: "signed integer is greater than maximum"})
		case v.n < math.MinInt32:
			panic(&py.Error{Type: "OverflowError", Msg: "signed integer is less than minimum"})
		}
	}
	if hours < 0 || hours > 23 {
		return TimeOfDay{}, fmt.Errorf("hour must be in 0..23")
	}
	if minutes < 0 || minutes > 59 {
		return TimeOfDay{}, fmt.Errorf("minute must be in 0..59")
	}
	return TimeOfDay{Hour: hours, Minute: minutes}, nil
}

// Window is a daily window in a local zone, such as 21:00 to 07:00 America/Chicago.
type Window struct {
	ZoneName string
	Start    TimeOfDay
	End      TimeOfDay
}

// WindowOf is Window.of. A start or end that is not HH:MM raises (panics with) the ValueError.
func WindowOf(zoneName string, start string, end string) *Window {
	s, err := ParseHhmm(start)
	if err != nil {
		panic(err)
	}
	e, err := ParseHhmm(end)
	if err != nil {
		panic(err)
	}
	return &Window{zoneName, s, e}
}

func (x *Window) IsOpen(at time.Time) bool {
	local := TimeOf(at.In(Zone(x.ZoneName)))
	if x.Start.Compare(x.End) <= 0 {
		return x.Start.Compare(local) <= 0 && local.Compare(x.End) < 0
	}
	return local.Compare(x.Start) >= 0 || local.Compare(x.End) < 0
}

// localInstant is `datetime(day.year, day.month, day.day, at.hour, at.minute, tzinfo=tz)`: the
// wall clock read as zoneinfo reads it (fold=0), so a time a transition skips or repeats takes
// the offset in force before the transition.
func (x *Window) localInstant(day time.Time, at TimeOfDay) time.Time {
	tz := Zone(x.ZoneName)
	return fold0(day.Year(), day.Month(), day.Day(), at.Hour, at.Minute, tz)
}

// fold0 is a wall clock in `loc` as zoneinfo reads a datetime with fold=0: past a transition
// only once the wall clock is past it under both offsets, else with the offset before it.
func fold0(year int, month time.Month, day, hour, minute int, loc *time.Location) time.Time {
	wall := time.Date(year, month, day, hour, minute, 0, 0, time.UTC).Unix()
	guess := time.Date(year, month, day, hour, minute, 0, 0, loc)
	_, offset := guess.Zone()
	start, end := guess.ZoneBounds()
	if !start.IsZero() {
		_, before := start.Add(-time.Second).Zone()
		if wall < start.Unix()+int64(max(before, offset)) {
			return time.Unix(wall-int64(before), 0).In(loc)
		}
	}
	if !end.IsZero() {
		_, after := end.Zone()
		if wall >= end.Unix()+int64(max(offset, after)) {
			return time.Unix(wall-int64(after), 0).In(loc)
		}
	}
	return time.Unix(wall-int64(offset), 0).In(loc)
}

// calendarDay is the date `offset` days after `local`'s, as `local + timedelta(days=offset)`
// gives it (wall-clock arithmetic, whatever the zone does that day).
func calendarDay(local time.Time, offset int) time.Time {
	y, m, d := local.Date()
	return time.Date(y, m, d+offset, 0, 0, 0, 0, time.UTC)
}

// NextOpen is the next instant the window opens, at or after `at` (UTC).
func (x *Window) NextOpen(at time.Time) time.Time {
	local := at.In(Zone(x.ZoneName))
	for offset := 0; offset < 3; offset++ {
		day := calendarDay(local, offset)
		candidate := x.localInstant(day, x.Start).UTC()
		if !candidate.Before(at) {
			return candidate
		}
	}
	panic(fmt.Errorf("a daily window opens within three days")) // AssertionError
}

func (x *Window) Describe() string {
	return fmt.Sprintf("%02d:%02d–%02d:%02d %s", x.Start.Hour, x.Start.Minute, x.End.Hour,
		x.End.Minute, x.ZoneName)
}

// HumanDelta is `3h 05m`, `12m` or `now`.
func HumanDelta(delta time.Duration) string {
	minutes := py.FloorDiv(int(delta), int(time.Minute))
	if minutes <= 0 {
		return "now"
	}
	hours, minutes := py.FloorDiv(minutes, 60), py.Mod(minutes, 60)
	if hours != 0 {
		return fmt.Sprintf("%dh %02dm", hours, minutes)
	}
	return fmt.Sprintf("%dm", minutes)
}
