package usage

import (
	"math"
	"sort"
	"strings"
	"time"
)

// ---------------------------------------------------------------------- charts

// The usage issue's charts (#2749, section 2) are mermaid `xychart-beta` bars and lines, never
// pies, drawn as the status issue's statistics draw theirs (`stats.bars`, `stats.line`: the fence,
// the title, the x-axis's labels in quotes, the y-axis's name), with more than one series where a
// chart sets a reading against its cap, as `dashboard.UsageCharts` does, coloured from a palette
// in the order the series come. Mermaid draws bars of several series over one another, not side
// by side: a chart that stacks (section 7) gives each bar the sum of the ones above it.

// Series is one series of a chart: a `line` or a `bar`, one value per x-axis label.
type Series struct {
	Kind   string
	Values []float64
}

// lineOf and barOf are a series of each kind.
func lineOf(values []float64) Series { return Series{Kind: "line", Values: values} }
func barOf(values []float64) Series  { return Series{Kind: "bar", Values: values} }

// The charts' palettes, one colour per series in the order a chart gives them, in colours that
// read in GitHub's light and dark themes.
var (
	// ReadingPalette: the reading (or what was held) in green, the cap in force (or the limit) in
	// grey, as `dashboard.UsageChart` colours them.
	ReadingPalette = "#1f883d, #8c959f"
	// TriplePalette: three series of equal standing (the queue's difficulties).
	TriplePalette = "#0969da, #cf222e, #8c959f"
	// StackPalette: section 7's reasons, bottom to top of a stacked bar (`IdleReasons`).
	StackPalette = "#0969da, #8250df, #bf8700, #cf222e"
)

// chart is one chart: `title`, the x-axis's `labels`, the y-axis named `axis` from 0 to `top`
// (mermaid's own range when `top` is 0), and the series.
func chart(title string, labels []string, axis string, top float64, palette string, series ...Series) []string {
	lines := []string{"```mermaid"}
	if palette != "" {
		lines = append(lines, `%%{init: {"themeVariables": {"xyChart": {"plotColorPalette": "`+palette+`"}}}}%%`)
	}
	quoted := make([]string, len(labels))
	for i, label := range labels {
		quoted[i] = `"` + strings.ReplaceAll(label, `"`, "'") + `"`
	}
	yAxis := `    y-axis "` + axis + `"`
	if top > 0 {
		yAxis += " 0 --> " + N(math.Ceil(top)).String()
	}
	lines = append(lines, "xychart-beta", `    title "`+strings.ReplaceAll(title, `"`, "'")+`"`,
		"    x-axis ["+strings.Join(quoted, ", ")+"]", yAxis)
	for _, s := range series {
		cells := make([]string, len(s.Values))
		for i, v := range s.Values {
			cells[i] = N(round1(v)).String()
		}
		lines = append(lines, "    "+s.Kind+" ["+strings.Join(cells, ", ")+"]")
	}
	return append(lines, "```")
}

// highest is the largest value of any of `lists`, 0 for none.
func highest(lists ...[]float64) float64 {
	top := 0.0
	for _, list := range lists {
		for _, v := range list {
			top = max(top, v)
		}
	}
	return top
}

// any0 is: some value of `lists` is not 0.
func any0(lists ...[]float64) bool { return highest(lists...) > 0 }

// Blocks are section 4's cells, from the lowest eighth to the highest; NoBlock is a cell with
// nothing to show.
var (
	Blocks  = []string{"▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"}
	NoBlock = "·"
)

// block is `v` of `top` as one of `Blocks`.
func block(v, top float64) string {
	if top <= 0 {
		return Blocks[0]
	}
	i := int(math.Round(v / top * float64(len(Blocks)-1)))
	return Blocks[min(max(i, 0), len(Blocks)-1)]
}

// ---------------------------------------------------------------------- points in time

// Bounds are the edges of a chart's points, oldest first: point i is [b[i], b[i+1]), and the
// last one holds the drawing's time.
type Bounds []time.Time

// Hourly is the bounds of points `step` hours wide over the last `span` before `now`, each
// starting on a local hour that is a multiple of `step`.
func Hourly(now time.Time, zone *time.Location, span time.Duration, step int) Bounds {
	step = max(1, step)
	local := now.In(zone)
	y, m, d := local.Date()
	h := local.Hour()
	current := time.Date(y, m, d, h-h%step, 0, 0, 0, zone)
	width := time.Duration(step) * time.Hour
	n := max(1, int(math.Ceil(float64(span)/float64(width))))
	b := make(Bounds, n+1)
	for i := range b {
		b[i] = current.Add(time.Duration(i-n+1) * width)
	}
	return b
}

// Daily is the bounds of points `step` local days wide over the last `days`, the last one ending
// at the next local midnight.
func Daily(now time.Time, zone *time.Location, days, step int) Bounds {
	step = max(1, step)
	local := now.In(zone)
	y, m, d := local.Date()
	n := max(1, (days+step-1)/step)
	b := make(Bounds, n+1)
	for i := range b {
		b[i] = time.Date(y, m, d+1-(n-i)*step, 0, 0, 0, 0, zone)
	}
	return b
}

// Points is how many points the bounds hold.
func (b Bounds) Points() int { return len(b) - 1 }

// From and To are the first point's start and the last one's end.
func (b Bounds) From() time.Time { return b[0] }
func (b Bounds) To() time.Time   { return b[len(b)-1] }

// Index is the point holding `at`, -1 outside them.
func (b Bounds) Index(at time.Time) int {
	if at.Before(b[0]) || !at.Before(b.To()) {
		return -1
	}
	return sort.Search(len(b), func(i int) bool { return b[i].After(at) }) - 1
}

// Labels is each point's label: its start in `zone`, as `layout` writes it, with its zone's name
// after a label that repeats (the hour the clocks go back), since mermaid draws points with one
// label as one.
func (b Bounds) Labels(zone *time.Location, layout string) []string {
	out := make([]string, b.Points())
	seen := map[string]bool{}
	for i := range out {
		local := b[i].In(zone)
		out[i] = local.Format(layout)
		if seen[out[i]] {
			out[i] += " " + local.Format("MST")
		}
		seen[out[i]] = true
	}
	return out
}

// AddMinutes adds `weight` for each minute of [lo, hi) to the point it falls in.
func (b Bounds) AddMinutes(into []float64, lo, hi time.Time, weight float64) {
	if lo.Before(b.From()) {
		lo = b.From()
	}
	if hi.After(b.To()) {
		hi = b.To()
	}
	if !hi.After(lo) {
		return
	}
	for i := b.Index(lo); i >= 0 && i < b.Points() && b[i].Before(hi); i++ {
		start, end := lo, hi
		if b[i].After(start) {
			start = b[i]
		}
		if b[i+1].Before(end) {
			end = b[i+1]
		}
		if end.After(start) {
			into[i] += end.Sub(start).Minutes() * weight
		}
	}
}

// eachSpan calls `each` with every row of `rows` (one subscription's ticks, or the pools', oldest
// first) and the span it stands for: until the next row, at most `TickCovers`, cut to [from, to).
func eachSpan[T any](rows []T, at func(T) time.Time, from, to time.Time, each func(row T, lo, hi time.Time)) {
	for i, row := range rows {
		lo := at(row)
		hi := lo.Add(TickCovers)
		if i+1 < len(rows) && at(rows[i+1]).Before(hi) {
			hi = at(rows[i+1])
		}
		if lo.Before(from) {
			lo = from
		}
		if hi.After(to) {
			hi = to
		}
		if hi.After(lo) {
			each(row, lo, hi)
		}
	}
}

// atPoints is, for each point of `b`, the highest `value` of a row of `rows` (oldest first) in it,
// or else that of the row in force at its start (the last one before it, within `TickCovers`);
// `known` says which points had either.
func atPoints[T any](rows []T, at func(T) time.Time, value func(T) (float64, bool), b Bounds) ([]float64, []bool) {
	values, known := make([]float64, b.Points()), make([]bool, b.Points())
	j := 0
	var before T
	have := false
	for i := range values {
		for j < len(rows) && at(rows[j]).Before(b[i]) {
			before, have = rows[j], true
			j++
		}
		if have && b[i].Sub(at(before)) <= TickCovers {
			if v, ok := value(before); ok {
				values[i], known[i] = v, true
			}
		}
		for k := j; k < len(rows) && at(rows[k]).Before(b[i+1]); k++ {
			if v, ok := value(rows[k]); ok && (!known[i] || v > values[i]) {
				values[i], known[i] = v, true
			}
		}
	}
	return values, known
}

// levels is, for each point of `b`, a window's usage in one subscription's readings (oldest
// first): the highest reading in it, or else what the last reading before it left, which is 0
// once that reading's window has reset; `known` is true where a reading is in the point or the
// last one's window still holds.
func levels(readings []*Reading, util func(*Reading) Num, resets func(*Reading) time.Time, b Bounds) ([]float64, []bool) {
	values, known := make([]float64, b.Points()), make([]bool, b.Points())
	var last *Reading
	j := 0
	for i := range values {
		for j < len(readings) && readings[j].ObservedAt.Before(b[i]) {
			if util(readings[j]).OK {
				last = readings[j]
			}
			j++
		}
		if last != nil {
			if at := resets(last); at.IsZero() || at.After(b[i]) {
				values[i], known[i] = util(last).V, true
			}
		}
		for k := j; k < len(readings) && readings[k].ObservedAt.Before(b[i+1]); k++ {
			if u := util(readings[k]); u.OK && (!known[i] || u.V > values[i]) {
				values[i], known[i] = u.V, true
			}
		}
	}
	return values, known
}
