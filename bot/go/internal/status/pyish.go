package status

import (
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// orDefault is `v or d`.
func orDefault(v, d any) any {
	if pyjson.Truthy(v) {
		return v
	}
	return d
}

// parseISO is clock.parse_iso on a JSON value: None (the zero time) when it is falsy.
func parseISO(v any) time.Time {
	if !pyjson.Truthy(v) {
		return time.Time{}
	}
	return clock.ParseISO(py.Str(v))
}

// pyFloat is float(v); what float() refuses raises, as it did in Python.
func pyFloat(v any) float64 {
	switch x := v.(type) {
	case int:
		return float64(x)
	case float64:
		return x
	case bool:
		if x {
			return 1
		}
		return 0
	case string:
		text := strings.TrimSpace(x)
		if !strings.ContainsAny(text, "xX") {
			if f, err := strconv.ParseFloat(text, 64); err == nil {
				return f
			}
		}
		panic(fmt.Errorf("ValueError: could not convert string to float: %s", py.Repr(x)))
	}
	panic(fmt.Errorf("TypeError: float() argument must be a string or a real number, not %T", v))
}
