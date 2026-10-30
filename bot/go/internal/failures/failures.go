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
				if e := py.OrStr(r.Get("error"), ""); e != "" {
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
			parts = append(parts, py.OrStr(review.Get("verdict"), "approved"))
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
			minutes = append(minutes, py.MustFloat(py.Or(b.Get("minutes"), 0)))
		}
	}
	if total := py.Sum(minutes); total != 0 {
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
	} else if len(seen.shown) > 0 {
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

// counter is a py.Counter over keys, each shown as the text it was first counted with.
type counter struct {
	counts *py.Counter
	shown  map[string]string
}

type counted struct {
	shown string
	n     int
}

func (c *counter) add(key, shown string) {
	if c.counts == nil {
		c.counts, c.shown = py.NewCounter(), map[string]string{}
	}
	if _, ok := c.shown[key]; !ok {
		c.shown[key] = shown
	}
	c.counts.Add(key, 1)
}

// mostCommon is Counter.most_common(): by count, the first counted first among equals.
func (c *counter) mostCommon() []counted {
	var out []counted
	if c.counts != nil {
		for _, e := range c.counts.MostCommon(-1) {
			out = append(out, counted{c.shown[e.Key], e.N})
		}
	}
	return out
}
