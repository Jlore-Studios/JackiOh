package clock

import (
	"fmt"
	"os"
	"testing"
	"time"
	_ "time/tzdata"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata.json holds what Python 3.13 answered for each case (bot/harness/clock.py).
func load(t *testing.T) *pyjson.Object {
	t.Helper()
	raw, err := os.ReadFile("testdata.json")
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(raw)
	if err != nil {
		t.Fatal(err)
	}
	return pyjson.Obj(v)
}

func catch(fn func()) (msg string) {
	defer func() {
		if r := recover(); r != nil {
			msg = fmt.Sprint(r)
		}
	}()
	fn()
	return ""
}

func TestParseISOMatchesPython(t *testing.T) {
	for _, e := range load(t).List("parse_iso") {
		c := pyjson.List(e)
		text := c[0].(string)
		var got any
		if msg := catch(func() {
			d := ParseISO(text)
			if !d.IsZero() {
				got = []any{ISO(d), d.Nanosecond() / 1000}
			}
		}); msg != "" {
			got = "OverflowError"
			if msg != "date value out of range" {
				t.Errorf("parse_iso(%q) raised %q", text, msg)
			}
		}
		if got == nil && pyjson.Equal(c[1], []any{"1-01-01T00:00:00Z", 0}) {
			continue // the zero time stands for None, so year 1's first instant reads as None
		}
		if !pyjson.Equal(got, c[1]) {
			t.Errorf("parse_iso(%q) = %v, want %v", text, got, c[1])
		}
	}
}

func TestHumanDeltaMatchesPython(t *testing.T) {
	for _, e := range load(t).List("human_delta") {
		c := pyjson.List(e)
		us := c[0].(int)
		if got := HumanDelta(time.Duration(us) * time.Microsecond); got != c[1] {
			t.Errorf("human_delta(%dus) = %q, want %q", us, got, c[1])
		}
	}
}

func TestParseHhmmMatchesPython(t *testing.T) {
	for _, e := range load(t).List("parse_hhmm") {
		c := pyjson.List(e)
		text := c[0].(string)
		var got any
		if msg := catch(func() {
			v, err := ParseHhmm(text)
			if err != nil {
				got = err.Error()
			} else {
				got = []any{v.Hour, v.Minute}
			}
		}); msg != "" {
			got = "OverflowError: " + msg
		}
		if !pyjson.Equal(got, c[1]) {
			t.Errorf("parse_hhmm(%q) = %v, want %v", text, got, c[1])
		}
	}
}

func TestWindowsMatchPython(t *testing.T) {
	for _, w := range pyjson.Objs(load(t).Get("windows")) {
		win := WindowOf(w.Str("zone"), w.Str("start"), w.Str("end"))
		if got := win.Describe(); got != w.Str("describe") {
			t.Errorf("describe = %q, want %q", got, w.Str("describe"))
		}
		for _, e := range w.List("rows") {
			c := pyjson.List(e)
			at := time.Unix(int64(c[0].(int)), 0).UTC()
			if got := win.IsOpen(at); got != c[1] {
				t.Errorf("%s %s: is_open = %v, want %v", w.Str("describe"), ISO(at), got, c[1])
			}
			if got := win.NextOpen(at).Unix(); int(got) != c[2] {
				t.Errorf("%s %s: next_open = %s, want %s", w.Str("describe"), ISO(at),
					ISO(time.Unix(got, 0)), ISO(time.Unix(int64(c[2].(int)), 0)))
			}
		}
	}
}

func TestUsCentralMatchesPython(t *testing.T) {
	fallback := &UsCentral{}
	loc := usCentralLocation()
	for _, e := range load(t).List("us_central") {
		c := pyjson.List(e)
		at := ParseISO(c[0].(string))
		local := fallback.Fromutc(at)
		// tzname() and utcoffset() read the wall clock, so the repeated hour reads as CDT.
		got := []any{local.Hour(), fallback.Tzname(local), int(fallback.Utcoffset(local) / time.Second)}
		if !pyjson.Equal(got, c[1:]) {
			t.Errorf("_UsCentral at %s = %v, want %v", c[0], got, c[1:])
		}
		if inLoc := at.In(loc); inLoc.Hour() != c[1] {
			t.Errorf("fallback location at %s = %d, want %v", c[0], inLoc.Hour(), c[1])
		}
	}
}

// The fallback agrees with tzdata (test_basics.py's test_fallback_zone_agrees_with_tzdata).
func TestFallbackZoneAgreesWithTzdata(t *testing.T) {
	fallback, real := usCentralLocation(), Zone("America/Chicago")
	at := time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)
	for i := 0; i < 24*400; i += 7 {
		at = at.Add(7 * time.Hour)
		if at.In(fallback).Hour() != at.In(real).Hour() {
			t.Errorf("%s: fallback %d, tzdata %d", ISO(at), at.In(fallback).Hour(), at.In(real).Hour())
		}
	}
}

func TestZonesMatchPython(t *testing.T) {
	for _, e := range load(t).List("zones") {
		c := pyjson.List(e)
		name := c[0].(string)
		var hour any
		msg := catch(func() {
			hour = fold0(2026, 7, 1, 12, 0, Zone(name)).UTC().Hour()
		})
		var raised any
		if msg != "" {
			raised, hour = msg, nil
		}
		if !pyjson.Equal(raised, c[1]) || !pyjson.Equal(hour, c[2]) {
			t.Errorf("zone(%q) = %v %v, want %v %v", name, raised, hour, c[1], c[2])
		}
	}
}
