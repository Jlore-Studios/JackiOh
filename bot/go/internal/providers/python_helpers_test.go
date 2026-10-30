package providers

// The private stand-ins for Python's behaviour, against answers taken from Python 3.13.

import (
	"testing"
	"time"
	_ "time/tzdata"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

func TestLocalTimeIsFoldZero(t *testing.T) {
	// datetime(y, m, d, h, mi, tzinfo=ZoneInfo(zone)).astimezone(timezone.utc). Go's time.Date
	// reads 02:30 on the night Chicago springs forward as 07:30Z; Python reads it as 08:30Z.
	for _, c := range []struct {
		zone            string
		y, mo, d, h, mi int
		want            string
	}{
		{"America/Chicago", 2027, 3, 14, 2, 30, "2027-03-14T08:30:00Z"},
		{"America/Chicago", 2027, 3, 14, 3, 0, "2027-03-14T08:00:00Z"},
		{"America/Chicago", 2027, 3, 14, 1, 59, "2027-03-14T07:59:00Z"},
		{"America/Chicago", 2026, 11, 1, 1, 30, "2026-11-01T06:30:00Z"},
		{"America/Chicago", 2026, 11, 1, 2, 0, "2026-11-01T08:00:00Z"},
		{"America/Chicago", 2026, 11, 1, 0, 59, "2026-11-01T05:59:00Z"},
		{"Europe/London", 2027, 3, 28, 1, 30, "2027-03-28T01:30:00Z"},
		{"Europe/London", 2026, 10, 25, 1, 30, "2026-10-25T00:30:00Z"},
		{"Australia/Lord_Howe", 2026, 10, 4, 2, 15, "2026-10-03T15:45:00Z"},
		{"Australia/Lord_Howe", 2027, 4, 4, 1, 45, "2027-04-03T14:45:00Z"},
		{"America/Havana", 2027, 3, 14, 0, 30, "2027-03-14T05:30:00Z"},
		{"America/Havana", 2026, 11, 1, 0, 30, "2026-11-01T04:30:00Z"},
		{"Asia/Kolkata", 2026, 10, 30, 12, 0, "2026-10-30T06:30:00Z"},
		{"UTC", 2026, 10, 30, 12, 0, "2026-10-30T12:00:00Z"},
	} {
		zone, err := time.LoadLocation(c.zone)
		if err != nil {
			t.Fatal(err)
		}
		got := localTime(c.y, time.Month(c.mo), c.d, c.h, c.mi, zone).UTC().Format("2006-01-02T15:04:05Z")
		if got != c.want {
			t.Errorf("%s %d-%02d-%02d %02d:%02d: got %s want %s", c.zone, c.y, c.mo, c.d, c.h, c.mi, got, c.want)
		}
	}
}

func TestDateFromISO(t *testing.T) {
	for text, want := range map[string]string{
		"2026-11-01": "2026-11-01", "20261101": "2026-11-01", "2026-W44": "2026-10-26",
		"2026W445": "2026-10-30", "2026-W44-5": "2026-10-30", "2026-W53": "2026-12-28",
		"2026103012": "2026-10-30", "2020-W53-7": "2021-01-03",
		"2026-11-1": "", "2026-1101": "", "2026-13-01": "", "202611-01": "", "0000-01-01": "",
		"2026-02-29": "", "2024-02-29": "2024-02-29", "2027-W53": "", "mid-October": "", "": "",
	} {
		got := ""
		if day, ok := dateFromISO(text); ok {
			got = day.Format("2006-01-02")
		}
		if got != want {
			t.Errorf("date.fromisoformat(%q): got %q want %q", text, got, want)
		}
	}
}

func TestPurePath(t *testing.T) {
	for _, c := range []struct {
		parts []string
		want  string
	}{
		{[]string{"a", "b"}, "a/b"}, {[]string{"/x/", "y"}, "/x/y"},
		{[]string{"", ".harness/providers.json"}, ".harness/providers.json"},
		{[]string{"//a", "b"}, "//a/b"}, {[]string{"///a", "b"}, "/a/b"},
		{[]string{"a/../b", "c"}, "a/../b/c"}, {[]string{"/root", "/abs", "c"}, "/abs/c"},
		{[]string{".", "x"}, "x"}, {[]string{""}, "."}, {[]string{"./a/./b/"}, "a/b"},
	} {
		if got := purePath(c.parts...); got != c.want {
			t.Errorf("PurePosixPath%q: got %q want %q", c.parts, got, c.want)
		}
	}
}

func TestLabelAndDigits(t *testing.T) {
	for s, want := range map[string]bool{"night-vm": true, "night-vm\n": true, "night vm": false,
		"a\n\n": false, "\na": false, "": false} {
		if got := label.MatchString(s); got != want {
			t.Errorf("_LABEL.match(%q): got %v", s, got)
		}
	}
	for s, want := range map[string]int{"١٢": 12, "𝟗": 9, "𝟎𝟏": 1, "٣": 3, "42": 42} {
		if got := py.Atoi(s); got != want {
			t.Errorf("int(%q): got %d want %d", s, got, want)
		}
	}
	for text, want := range map[string]time.Duration{"+PT90M": 90 * time.Minute, "+PT2H": 2 * time.Hour,
		"+PT1H30M": 90 * time.Minute, " +pt10s ": 10 * time.Second, "PT5M": 5 * time.Minute,
		"+PT": 30 * time.Minute, "bogus": 30 * time.Minute, "+PT0M": 0, "+PT١٢M": 12 * time.Minute} {
		if got := Duration(text); got != want {
			t.Errorf("_duration(%q): got %v want %v", text, got, want)
		}
	}
}
