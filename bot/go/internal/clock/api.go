// Package clock is bot/harness/clock.py.
//
// Time: the current instant, ISO formatting, and the nightly run window.
package clock

import (
	"time"
)

// `dt` as ISO-8601 UTC with a trailing Z, to the second.

func ISO(dt time.Time) string {
	panic("unported: clock.iso")
}

// An aware UTC datetime from ISO text, or None when `text` is empty or unreadable.

func ParseISO(text string) time.Time {
	panic("unported: clock.parse_iso")
}

// The current instant, or `override` (HARNESS_NOW) when it parses.

func Now(override string /* = "" */) time.Time {
	panic("unported: clock.now")
}

// The named IANA zone. America/Chicago falls back to fixed US rules without tzdata.

func Zone(name string) *time.Location {
	panic("unported: clock.zone")
}

// US Central time with the 2007 DST rules, for hosts without tzdata.

type UsCentral struct {
}

func (x *UsCentral) dstBounds(year int) (time.Time, time.Time) {
	panic("unported: clock._UsCentral._dst_bounds")
}

func (x *UsCentral) Utcoffset(dt time.Time) time.Duration {
	panic("unported: clock._UsCentral.utcoffset")
}

func (x *UsCentral) Dst(dt time.Time) time.Duration {
	panic("unported: clock._UsCentral.dst")
}

func (x *UsCentral) Tzname(dt time.Time) string {
	panic("unported: clock._UsCentral.tzname")
}

func (x *UsCentral) Fromutc(dt time.Time) time.Time {
	panic("unported: clock._UsCentral.fromutc")
}

// parse_hhmm is clock.parse_hhmm.

func ParseHhmm(text string) TimeOfDay {
	panic("unported: clock.parse_hhmm")
}

// A daily window in a local zone, such as 21:00 to 07:00 America/Chicago.

type Window struct {
	ZoneName string
	Start    TimeOfDay
	End      TimeOfDay
}

func WindowOf(zoneName string, start string, end string) *Window {
	panic("unported: clock.Window.of")
}

func (x *Window) IsOpen(at time.Time) bool {
	panic("unported: clock.Window.is_open")
}

func (x *Window) localInstant(day time.Time, at TimeOfDay) time.Time {
	panic("unported: clock.Window._local_instant")
}

// The next instant the window opens, at or after `at` (UTC).

func (x *Window) NextOpen(at time.Time) time.Time {
	panic("unported: clock.Window.next_open")
}

// When the window that is open at `at` closes, or None when it is closed.

func (x *Window) ClosesAt(at time.Time) time.Time {
	panic("unported: clock.Window.closes_at")
}

func (x *Window) Describe() string {
	panic("unported: clock.Window.describe")
}

// `3h 05m`, `12m` or `now`.

func HumanDelta(delta time.Duration) string {
	panic("unported: clock.human_delta")
}

// TimeOfDay is Python's datetime.time: a wall-clock time with no date or zone.
type TimeOfDay struct {
	Hour, Minute, Second int
}
