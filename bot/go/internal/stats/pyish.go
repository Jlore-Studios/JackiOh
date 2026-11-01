package stats

import (
	"sort"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// The Python this module leans on where Go differs and py does not hold it: statistics.median of
// timedeltas, a defaultdict of Counters, and parse_iso of a JSON value. A value the Python could
// not read (py's int() and float() raise) stops the count, as it did there.

// minus is a - b between two parsed times; a None among them raises TypeError, as in Python.
func minus(a, b time.Time) time.Duration {
	if a.IsZero() || b.IsZero() {
		panic(&py.Error{Type: "TypeError", Msg: "unsupported operand type(s) for -: 'datetime.datetime' and 'NoneType'"})
	}
	return a.Sub(b)
}

// medianDuration is statistics.median over timedeltas: the middle one, or the two middle ones'
// sum halved to the microsecond, a tie to even (timedelta / 2).
func medianDuration(took []time.Duration) time.Duration {
	data := append([]time.Duration{}, took...)
	sort.SliceStable(data, func(i, j int) bool { return data[i] < data[j] })
	n := len(data)
	if n%2 == 1 {
		return data[n/2]
	}
	us := int((data[n/2-1] + data[n/2]) / time.Microsecond)
	q, r := py.FloorDiv(us, 2), py.Mod(us, 2)
	if r*2 == 2 && py.Mod(q, 2) == 1 {
		q++
	}
	return time.Duration(q) * time.Microsecond
}

// counterAt is `d[key]` on a defaultdict(Counter): the key's Counter, made (and kept) when missing.
func counterAt(d *pyjson.Object, key string) *py.Counter {
	if c, ok := d.Get(key).(*py.Counter); ok {
		return c
	}
	c := py.NewCounter()
	d.Set(key, c)
	return c
}
