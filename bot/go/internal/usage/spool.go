package usage

import (
	"reflect"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// SpoolKey is the state file's spool (`_bot-state:state.json`): the rows a delivery hands the
// status loop, each a CSV line under its kind's name (`{"readings": […], "runs": […]}`), until
// the loop has pushed them to the branch and takes them out (`Unspool`). Deliver writes the state
// file anyway, so a run's readings ride on its usage write (`providers.NoteUsage`) and only its
// run row costs a write of its own; the loop reads the spool from the state it reads every tick,
// so a delivery never pushes the branch, wherever it runs (inside the loop, `controller.Deliver`,
// or in a night run's deliver job).
var SpoolKey = "usage_spool"

// SpoolMax is the most lines the spool keeps of each kind; past it the oldest go. The loop drains
// it every 30 minutes, so this is days of runs' readings: only a loop that stopped for that long
// loses any.
var SpoolMax = 2000

// Spool adds `rows` (readings and runs) to the state file's spool, each as its line.
func Spool(state *pyjson.Object, rows ...Row) {
	if len(rows) == 0 {
		return
	}
	spool, ok := state.Get(SpoolKey).(*pyjson.Object)
	if !ok || spool == nil {
		spool = pyjson.New()
		state.Set(SpoolKey, spool)
	}
	for _, row := range rows {
		kind := KindOf(row).Name
		lines := pyjson.List(spool.Get(kind))
		lines = append(lines, line(Encode(row)))
		if len(lines) > SpoolMax {
			lines = lines[len(lines)-SpoolMax:]
		}
		spool.Set(kind, lines)
	}
}

// Spooled is one row the spool holds, with its line as the spool keeps it (what `Unspool` takes).
type Spooled struct {
	Kind *Kind
	Line string
	Row  Row
}

// SpoolOf is every row the state file's spool holds, readings first, each in the order it came.
// A line that is not CSV is skipped.
func SpoolOf(state *pyjson.Object) []Spooled {
	spool, _ := state.Get(SpoolKey).(*pyjson.Object)
	var out []Spooled
	for _, kind := range Kinds {
		for _, raw := range pyjson.List(spool.Get(kind.Name)) {
			text, ok := raw.(string)
			cells := unline(text)
			if !ok || cells == nil {
				continue
			}
			row := newRow(kind)
			decode(kind.Header(), cells, reflect.ValueOf(row).Elem())
			out = append(out, Spooled{Kind: kind, Line: text, Row: row})
		}
	}
	return out
}

// Unspool takes `taken` (lines the loop pushed, by kind name) out of the state file's spool, each
// once: a line spooled again since, or one it never took, stays.
func Unspool(state *pyjson.Object, taken map[string][]string) {
	spool, ok := state.Get(SpoolKey).(*pyjson.Object)
	if !ok || spool == nil {
		return
	}
	for name, lines := range taken {
		left := map[string]int{}
		for _, l := range lines {
			left[l]++
		}
		var kept []any
		for _, raw := range pyjson.List(spool.Get(name)) {
			if text, ok := raw.(string); ok && left[text] > 0 {
				left[text]--
				continue
			}
			kept = append(kept, raw)
		}
		if kept == nil {
			kept = []any{}
		}
		spool.Set(name, kept)
	}
	empty := true
	for _, kind := range Kinds {
		if len(pyjson.List(spool.Get(kind.Name))) > 0 {
			empty = false
		}
	}
	if empty {
		state.Delete(SpoolKey) // nothing waits: the state file holds no spool
	}
}
