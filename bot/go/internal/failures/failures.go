// Package failures is bot/harness/failures.py.
//
// Why a run's rounds failed, written for the person who reviews it.
//
// A build or revision that uses up every review round it is given (`max_review_cycles`) without an
// approval is labelled `bot:stuck`, and its comment says why, round by round: what the builder's
// session did, which checks were red, what the reviewer said, and which findings kept coming back.
// A run that stopped sooner (an unreadable review, a refusal) says its real round count and reason.
package failures

import (
	"fmt"
	"math"
	"sort"
	"strconv"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// TopFindings are the findings shown in the summary, and CellChars the longest claim or error
// quoted in a table cell.
var TopFindings int = 5

// CellChars = 140
var CellChars int = 140

// Cell is failures._cell: `text` on one line, its pipes escaped, cut to `limit` characters.
func Cell(text any, limit int /* = CELL_CHARS */) string {
	s := ""
	if pyjson.Truthy(text) {
		s = py.Str(text)
	}
	flat := strings.ReplaceAll(strings.Join(strings.FieldsFunc(s, py.IsSpace), " "), "|", "\\|")
	if py.Len(flat) <= limit {
		return flat
	}
	return strings.TrimRightFunc(py.SliceTo(flat, limit-1), py.IsSpace) + "…"
}

// red is _red: the checks this change turned red (not the ones red on main too, nor skipped).
func red(gates []any) []*pyjson.Object {
	var found []*pyjson.Object
	for _, g := range pyjson.Objs(gates) {
		if !g.Truthy("ok") && !g.Truthy("pre_existing") && !g.Truthy("skipped") {
			found = append(found, g)
		}
	}
	return found
}

func blocking(review *pyjson.Object) []*pyjson.Object {
	var found []*pyjson.Object
	for _, f := range pyjson.Objs(review.Get("findings")) {
		if pyjson.Equal(f.Get("severity"), "blocking") {
			found = append(found, f)
		}
	}
	return found
}

func builderCell(builder *pyjson.Object) string {
	if builder.Len() == 0 {
		return "—"
	}
	what := fmt.Sprintf("%s on `%s`, %s min", py.Str(builder.GetOr("role", "build")),
		py.Str(builder.GetOr("model", "?")), py.Str(builder.GetOr("minutes", 0)))
	if builder.Truthy("timed_out") {
		return what + ": **timed out**"
	}
	if !builder.Truthy("ok") {
		return what + ": **failed**: " + Cell(builder.Get("error"), 90)
	}
	return what
}

func checksCell(gates []any) string {
	if len(gates) == 0 {
		return "—"
	}
	found := red(gates)
	if len(found) == 0 {
		return "green"
	}
	parts := make([]string, len(found))
	for i, g := range found {
		parts[i] = fmt.Sprintf("`%s` (exit %s)", py.Str(g.Get("name")), py.Str(g.Get("exit_code")))
	}
	return "red: " + strings.Join(parts, ", ")
}

func reviewCell(review *pyjson.Object, selfChecks []any) string {
	var parts []string
	if len(selfChecks) > 0 {
		flagged := 0
		var failed []string
		for _, s := range pyjson.Objs(selfChecks) {
			if s.Truthy("flagged") {
				flagged++
			}
			if r, ok := s.Get("review").(*pyjson.Object); ok {
				if e := orEmpty(r.Get("error")); e != "" {
					failed = append(failed, e)
				}
			}
		}
		parts = append(parts, fmt.Sprintf("self check ×%d, %d flagged", len(selfChecks), flagged))
		if len(failed) > 0 {
			parts = append(parts, "**self-check call failed**: "+Cell(failed[len(failed)-1], 90))
		}
	}
	switch {
	case review == nil:
		parts = append(parts, "no review")
	case !pyjson.Truthy(review.GetOr("readable", true)):
		text := "**review unreadable**"
		if review.Truthy("error") {
			text += ": " + Cell(review.Get("error"), 90)
		}
		parts = append(parts, text)
	default:
		if found := blocking(review); len(found) > 0 {
			first := found[0]
			parts = append(parts, fmt.Sprintf("blocked, %d finding(s); first `%s`: %s", len(found),
				Cell(first.Get("where"), 60), Cell(first.Get("claim"), 90)))
		} else {
			parts = append(parts, orDefault(review.Get("verdict"), "approved"))
		}
	}
	return strings.Join(parts, "; ")
}

// Stuck is whether it used every round it was given.
func Stuck(cycles []*pyjson.Object, limit int) bool {
	return len(cycles) >= limit && limit > 0
}

// Why is Markdown: why the rounds failed, a summary and then round by round.
func Why(cycles []*pyjson.Object, limit int, reason string /* = "" */) string {
	rounds := len(cycles)
	title := fmt.Sprintf("### Why it stopped after %d of %d round(s)", rounds, limit)
	if Stuck(cycles, limit) {
		title = fmt.Sprintf("### Why it failed %d times", rounds)
	}
	lines := []string{title, ""}
	if reason != "" {
		lines = append(lines, fmt.Sprintf("- **The run's own reason:** %s.", Cell(reason, 300)))
	}
	var reviews []*pyjson.Object
	for _, c := range cycles {
		if c.Get("review") != nil {
			reviews = append(reviews, c.Obj("review"))
		}
	}
	unreadable, blocked := 0, 0
	for _, r := range reviews {
		readable := pyjson.Truthy(r.GetOr("readable", true))
		if !readable {
			unreadable++
		}
		if readable && len(blocking(r)) > 0 {
			blocked++
		}
	}
	if len(reviews) > 0 {
		line := fmt.Sprintf("- **The reviewer blocked %d of %d reviewed round(s)**", blocked, len(reviews))
		if unreadable > 0 {
			line += fmt.Sprintf("; its answer could not be read in %d", unreadable)
		}
		lines = append(lines, line+".")
	}
	redCounts := &counter{}
	redRounds := 0
	for _, c := range cycles {
		found := red(c.List("gates"))
		for _, g := range found {
			name := g.Get("name")
			redCounts.add(pyjson.Dumps(name), py.Str(name))
		}
		if len(found) > 0 {
			redRounds++
		}
	}
	if redRounds > 0 {
		var parts []string
		for _, e := range redCounts.mostCommon() {
			parts = append(parts, fmt.Sprintf("`%s` ×%d", e.shown, e.n))
		}
		lines = append(lines, fmt.Sprintf("- **Checks were red in %d round(s):** ", redRounds)+
			strings.Join(parts, ", ")+".")
	}
	builders := make([]*pyjson.Object, len(cycles))
	for i, c := range cycles {
		builders[i] = c.Obj("builder")
	}
	var broken []*pyjson.Object
	for _, b := range builders {
		if b.Len() > 0 && (!b.Truthy("ok") || b.Truthy("timed_out")) {
			broken = append(broken, b)
		}
	}
	if len(broken) > 0 {
		errors := &counter{}
		for _, b := range broken {
			var e any = b.Get("error")
			if !pyjson.Truthy(e) {
				e = "failed"
				if b.Truthy("timed_out") {
					e = "timed out"
				}
			}
			text := Cell(e, 160)
			errors.add(text, text)
		}
		top := errors.mostCommon()[0]
		line := fmt.Sprintf("- **The builder's session failed in %d round(s)**, most often: %s", len(broken), top.shown)
		if top.n > 1 {
			line += fmt.Sprintf(" (×%d)", top.n)
		}
		lines = append(lines, line+".")
	}
	var minutes []float64
	for _, b := range builders {
		if b.Len() > 0 {
			minutes = append(minutes, pyFloat(orZero(b.Get("minutes"))))
		}
	}
	if total := pySum(minutes); total != 0 {
		lines = append(lines, fmt.Sprintf("- **Builder time:** %.0f min over %d round(s).", total, rounds))
	}
	seen := &counter{}
	for _, r := range reviews {
		for _, f := range blocking(r) {
			where, claim := Cell(f.Get("where"), 80), Cell(f.Get("claim"), CellChars)
			seen.add(where+"\x00"+claim, "`"+where+"`: "+claim)
		}
	}
	var again []counted
	for _, e := range seen.mostCommon() {
		if e.n > 1 {
			again = append(again, e)
		}
	}
	if len(again) > TopFindings {
		again = again[:TopFindings]
	}
	if len(again) > 0 {
		lines = append(lines, "", "**Findings that kept coming back** (the builder never settled them):")
		for _, e := range again {
			lines = append(lines, fmt.Sprintf("- %s (×%d)", e.shown, e.n))
		}
	} else if len(seen.order) > 0 {
		lines = append(lines, "", "No finding came back twice: each round's review found something new.")
	}
	if len(cycles) > 0 {
		lines = append(lines, "", "<details><summary>Round by round</summary>", "",
			"| Round | Builder | Checks | Review |", "|---|---|---|---|")
		for _, c := range cycles {
			var review *pyjson.Object
			if c.Get("review") != nil {
				review = c.Obj("review")
				if review == nil {
					review = pyjson.New() // a review that is not a dict: Python would fail on it
				}
			}
			lines = append(lines, fmt.Sprintf("| %s | %s | %s | %s |", py.Str(c.GetOr("n", "?")),
				builderCell(c.Obj("builder")), checksCell(c.List("gates")),
				reviewCell(review, c.List("self_check"))))
		}
		lines = append(lines, "", "</details>")
	}
	return strings.Join(lines, "\n")
}

// counter is collections.Counter over keys in the order they first came, with most_common.
type counter struct {
	order  []string
	counts map[string]int
	shown  map[string]string
}

type counted struct {
	shown string
	n     int
}

func (c *counter) add(key, shown string) {
	if c.counts == nil {
		c.counts, c.shown = map[string]int{}, map[string]string{}
	}
	if _, ok := c.counts[key]; !ok {
		c.order = append(c.order, key)
		c.shown[key] = shown
	}
	c.counts[key]++
}

// mostCommon is Counter.most_common(): by count, the first counted first among equals.
func (c *counter) mostCommon() []counted {
	out := make([]counted, len(c.order))
	for i, k := range c.order {
		out[i] = counted{c.shown[k], c.counts[k]}
	}
	sort.SliceStable(out, func(i, j int) bool { return out[i].n > out[j].n })
	return out
}

// orEmpty is `str(v or "")`.
func orEmpty(v any) string { return orDefault(v, "") }

// orDefault is `str(v or def)`.
func orDefault(v any, def string) string {
	if !pyjson.Truthy(v) {
		return def
	}
	return py.Str(v)
}

// orZero is `v or 0`.
func orZero(v any) any {
	if !pyjson.Truthy(v) {
		return 0
	}
	return v
}

// pyFloat is float(v) for a JSON value; what float() refuses raises (panics), as in Python.
func pyFloat(v any) float64 {
	switch x := v.(type) {
	case bool:
		if x {
			return 1
		}
		return 0
	case string:
		f, err := strconv.ParseFloat(strings.ReplaceAll(strings.TrimFunc(x, py.IsSpace), "_", ""), 64)
		if err != nil {
			panic(fmt.Errorf("could not convert string to float: %s", py.Repr(x)))
		}
		return f
	}
	if f, ok := pyjson.AsFloat(v); ok {
		return f
	}
	panic(fmt.Errorf("float() argument must be a string or a real number, not '%T'", v))
}

// pySum is sum() over floats as Python 3.12+ adds them: Neumaier's compensated summation, so
// sum([0.1] * 10) is 1.0.
func pySum(xs []float64) float64 {
	if len(xs) == 0 {
		return 0
	}
	total, c := xs[0], 0.0
	for _, x := range xs[1:] {
		t := total + x
		if math.Abs(total) >= math.Abs(x) {
			c += (total - t) + x
		} else {
			c += (x - t) + total
		}
		total = t
	}
	if c != 0 && !math.IsInf(c, 0) && !math.IsNaN(c) {
		total += c
	}
	return total
}
