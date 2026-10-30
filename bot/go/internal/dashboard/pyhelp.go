package dashboard

// The few Python behaviours this module leans on that `py` and `pyjson` do not hold: kept private
// here (PORTING §2), each naming the Python it stands for.

import (
	"errors"
	"fmt"
	"math"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stats"
)

// strOr is str(v or ""): "" for anything falsy, else str(v).
func strOr(v any) string {
	if !pyjson.Truthy(v) {
		return ""
	}
	return py.Str(v)
}

// parseISO is clock.parse_iso(v) for any value: None (the zero time) for anything falsy, else
// str(v) parsed.
func parseISO(v any) time.Time {
	if !pyjson.Truthy(v) {
		return time.Time{}
	}
	return clock.ParseISO(py.Str(v))
}

// pyInt is int(v): an int, a float truncated, a bool, or the text of an integer; false where
// Python raises (TypeError, ValueError).
func pyInt(v any) (int, bool) {
	switch x := v.(type) {
	case bool:
		if x {
			return 1, true
		}
		return 0, true
	case float64:
		if math.IsNaN(x) || math.IsInf(x, 0) {
			return 0, false
		}
		return int(x), true
	case string:
		n, err := strconv.Atoi(strings.TrimSpace(x))
		return n, err == nil
	}
	return pyjson.AsInt(v)
}

// mustInt is int(v) where the Python lets its ValueError or TypeError go up.
func mustInt(v any) int {
	n, ok := pyInt(v)
	if !ok {
		panic(fmt.Errorf("ValueError: invalid literal for int(): %s", py.Repr(v)))
	}
	return n
}

// pyFloat is float(v): a number, a bool, or the text of one; false where Python raises
// (TypeError, ValueError).
func pyFloat(v any) (float64, bool) {
	switch x := v.(type) {
	case bool:
		if x {
			return 1, true
		}
		return 0, true
	case string:
		f, err := strconv.ParseFloat(strings.TrimSpace(x), 64)
		return f, err == nil
	}
	return pyjson.AsFloat(v)
}

// percent is f"{x:.{prec}%}": x times 100, fixed to prec decimals, then a percent sign.
func percent(x float64, prec int) string { return fmt.Sprintf("%.*f%%", prec, x*100) }

// htmlEscape is html.escape(s, quote=True), which writes `&#x27;` for `'`.
func htmlEscape(s string) string {
	return strings.NewReplacer("&", "&amp;", "<", "&lt;", ">", "&gt;", `"`, "&quot;", "'", "&#x27;").
		Replace(s)
}

// containsStr is `s in values` for a JSON list: an element equal to the string s.
func containsStr(values []any, s string) bool {
	for _, v := range values {
		if x, ok := v.(string); ok && x == s {
			return true
		}
	}
	return false
}

// strIn is `v in (a, b, …)` for strings: v is one of them.
func strIn(v any, options ...string) bool {
	x, ok := v.(string)
	if !ok {
		return false
	}
	for _, o := range options {
		if x == o {
			return true
		}
	}
	return false
}

// isDigits is str.isdigit() for the ASCII digits: not empty, and digits only.
func isDigits(s string) bool {
	if s == "" {
		return false
	}
	for _, r := range s {
		if r < '0' || r > '9' {
			return false
		}
	}
	return true
}

// upTo is xs[:n].
func upTo[T any](xs []T, n int) []T {
	if n < 0 {
		n += len(xs)
		if n < 0 {
			n = 0
		}
	}
	if n > len(xs) {
		n = len(xs)
	}
	return xs[:n]
}

// from is xs[n:] for n >= 0.
func from[T any](xs []T, n int) []T {
	if n > len(xs) {
		n = len(xs)
	}
	return xs[n:]
}

// lookup is d[key], whose KeyError goes up.
func lookup[V any](d map[string]V, key string) V {
	v, ok := d[key]
	if !ok {
		panic(fmt.Errorf("KeyError: %s", py.Repr(key)))
	}
	return v
}

// noneAttr is the AttributeError of a method called on None (`_local(ctx, None)`), which the
// Python lets go up.
func noneAttr(name string) error {
	return errors.New("AttributeError: 'NoneType' object has no attribute '" + name + "'")
}

// windowSpans is providers.WINDOWS, while the providers stub types it `any`: read through it when
// it is the map, else the Python's values.
func windowSpan(window string) time.Duration {
	if spans, ok := any(providers.Windows).(map[string]time.Duration); ok {
		return lookup(spans, window)
	}
	return lookup(map[string]time.Duration{"five_hour": 5 * time.Hour, "seven_day": 7 * 24 * time.Hour},
		window)
}

// statsEveryMinutes is int(stats_mod.STATS_EVERY.total_seconds() // 60), while the stats stub
// types it `any`: read through it when it is a duration, else the Python's 30 minutes.
func statsEveryMinutes() int {
	every := 30 * time.Minute
	if d, ok := any(stats.StatsEvery).(time.Duration); ok {
		every = d
	}
	return int(math.Floor(every.Seconds() / 60))
}
