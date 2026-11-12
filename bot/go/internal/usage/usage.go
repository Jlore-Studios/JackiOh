// Package usage is the usage history (#2749): every subscription's usage over time, kept as CSV
// files on the unprotected `_bot-usage` branch (the identity's `usage_branch`), so any spreadsheet
// opens them, for tuning the limits in `.harness/providers.json` from evidence.
//
// Six kinds of row, each a Go type with its header (`Reading`, `Tick`, `PoolTick`, `Session`,
// `Week`, `Run`), in their own files under `usage/`: `readings-YYYY-MM.csv`, `ticks-YYYY-MM.csv`,
// `pools-YYYY-MM.csv`, `sessions-YYYY-MM.csv` and `runs-YYYY-MM.csv` by month (UTC) of the row's
// own time, and `weeks.csv` whole. Times are UTC in RFC3339 (`2026-11-02T03:04:05Z`), numbers
// plain decimals, utilizations percent from 0 to 100 with at most one decimal, flags 1 or 0, and an
// unknown value an empty cell. A column only ever joins at the end of its header, so a file or a
// spool line written before it still reads (`History` rewrites an older file under the new header
// before it appends to it). `bot/README.md`, "The usage history", documents every column.
//
// Where the rows come from:
//
//   - A run's readings (the start ping, each call's streamed reading, Muse's and agy's `/usage`,
//     a refusal) go into its `result.json` (`Note`, numbers only); deliver hands them and the run's
//     own row (`RunOf`) to the status loop through a small spool in the state file (`Spool`).
//   - The status loop (`Recorder`) adds, each tick, the newest reading in the state file when no run
//     handed it on, a tick row per subscription when anything in it changed (and at least hourly)
//     and a pool row; every 30 minutes it writes them and the spool's rows to its clone of the
//     branch, derives the sessions and weeks that closed (`Derive`), and pushes.
//   - The first time it finds the branch without `usage/backfill.json`, it reads `_bot-journal`
//     into runs and cut-off readings since `BackfillSince` (`Backfill`).
//
// The history is a record, never a gate: a failed read or write is logged, and the loop, the run
// and the delivery go on. It holds numbers and ids only: no prompt, tool output, token or secret.
package usage

import (
	"bytes"
	"encoding/csv"
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"reflect"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// Dir is the directory on the branch the files live in.
var Dir = "usage"

// ---------------------------------------------------------------------- cells

// Num is a number cell: written as a plain decimal, or empty when unknown (`OK` false).
type Num struct {
	V  float64
	OK bool
}

// N is a known number.
func N(v float64) Num { return Num{V: v, OK: true} }

// Flag is 1 for true and 0 for false.
func Flag(v bool) Num {
	if v {
		return N(1)
	}
	return N(0)
}

// Pct is a share (0 to 1, as the readings and providers.json hold it) as a percent with at most
// one decimal.
func Pct(share float64) Num { return N(round1(share * 100)) }

// String is the cell's text.
func (n Num) String() string {
	if !n.OK {
		return ""
	}
	v := n.V
	if v == 0 {
		v = 0 // never "-0"
	}
	return strconv.FormatFloat(v, 'f', -1, 64)
}

// round1 is `v` to one decimal, as Python's round does it (`py.RoundN`).
func round1(v float64) float64 { return py.RoundN(v, 1) }

// TimeFormat is how a time is written: RFC3339 in UTC, to the second.
var TimeFormat = "2006-01-02T15:04:05Z"

// ISO is a time as a cell: RFC3339 UTC, or "" for the zero time.
func ISO(t time.Time) string {
	if t.IsZero() {
		return ""
	}
	return t.UTC().Format(TimeFormat)
}

// ParseTime is a time cell read back: the zero time for an empty or unreadable one.
func ParseTime(text string) time.Time {
	text = py.Strip(text)
	if text == "" {
		return time.Time{}
	}
	if t, err := time.Parse(time.RFC3339, text); err == nil {
		return t.UTC()
	}
	return time.Time{}
}

// parseNum is a number cell read back.
func parseNum(text string) Num {
	text = py.Strip(text)
	if text == "" {
		return Num{}
	}
	v, err := strconv.ParseFloat(text, 64)
	if err != nil {
		return Num{}
	}
	return N(v)
}

// ---------------------------------------------------------------------- rows

// Row is one row of the history: its key, which no file holds twice, and its time, whose month
// names its file.
type Row interface {
	Key() string
	When() time.Time
}

// Kind is one kind of row: its files' name and whether they are monthly.
type Kind struct {
	Name    string
	Monthly bool
	typ     reflect.Type
}

// The kinds, one per row type.
var (
	Readings = &Kind{Name: "readings", Monthly: true, typ: reflect.TypeOf(Reading{})}
	Ticks    = &Kind{Name: "ticks", Monthly: true, typ: reflect.TypeOf(Tick{})}
	Pools    = &Kind{Name: "pools", Monthly: true, typ: reflect.TypeOf(PoolTick{})}
	Sessions = &Kind{Name: "sessions", Monthly: true, typ: reflect.TypeOf(Session{})}
	Weeks    = &Kind{Name: "weeks", Monthly: false, typ: reflect.TypeOf(Week{})}
	Runs     = &Kind{Name: "runs", Monthly: true, typ: reflect.TypeOf(Run{})}
)

// Kinds is every kind, in the order the files are written.
var Kinds = []*Kind{Readings, Ticks, Pools, Sessions, Weeks, Runs}

// KindOf is the kind of `row`.
func KindOf(row Row) *Kind {
	t := reflect.TypeOf(row)
	if t.Kind() == reflect.Pointer {
		t = t.Elem()
	}
	for _, k := range Kinds {
		if k.typ == t {
			return k
		}
	}
	panic(fmt.Sprintf("usage: %T is no kind of row", row))
}

// Header is the kind's columns, in order: its type's `csv` tags.
func (k *Kind) Header() []string {
	header := make([]string, 0, k.typ.NumField())
	for i := range k.typ.NumField() {
		if name := k.typ.Field(i).Tag.Get("csv"); name != "" {
			header = append(header, name)
		}
	}
	return header
}

// Path is the file, relative to the branch's root, that holds the kind's rows of `at`'s month.
func (k *Kind) Path(at time.Time) string {
	if !k.Monthly {
		return filepath.Join(Dir, k.Name+".csv")
	}
	return filepath.Join(Dir, fmt.Sprintf("%s-%s.csv", k.Name, at.UTC().Format("2006-01")))
}

// Encode is `row`'s cells, in its header's order.
func Encode(row Row) []string {
	v := reflect.ValueOf(row)
	if v.Kind() == reflect.Pointer {
		v = v.Elem()
	}
	cells := []string{}
	for i := range v.NumField() {
		if v.Type().Field(i).Tag.Get("csv") == "" {
			continue
		}
		switch f := v.Field(i).Interface().(type) {
		case string:
			cells = append(cells, f)
		case Num:
			cells = append(cells, f.String())
		case time.Time:
			cells = append(cells, ISO(f))
		default:
			panic(fmt.Sprintf("usage: a %T cell", f))
		}
	}
	return cells
}

// decode fills `into` (a struct of a row type) from `record` under `header`: a column the header
// lacks stays empty, one the type lacks is dropped.
func decode(header []string, record []string, into reflect.Value) {
	at := map[string]int{}
	for i, name := range header {
		at[name] = i
	}
	t := into.Type()
	for i := range t.NumField() {
		name := t.Field(i).Tag.Get("csv")
		j, ok := at[name]
		if name == "" || !ok || j >= len(record) {
			continue
		}
		cell := record[j]
		field := into.Field(i)
		switch field.Interface().(type) {
		case string:
			field.SetString(cell)
		case Num:
			field.Set(reflect.ValueOf(parseNum(cell)))
		case time.Time:
			field.Set(reflect.ValueOf(ParseTime(cell)))
		}
	}
}

// line is `cells` as one CSV line, without its newline.
func line(cells []string) string {
	var buf bytes.Buffer
	w := csv.NewWriter(&buf)
	_ = w.Write(cells) // a bytes.Buffer never fails
	w.Flush()
	return strings.TrimRight(buf.String(), "\n")
}

// unline is one CSV line's cells; nil when it is not CSV.
func unline(text string) []string {
	r := csv.NewReader(strings.NewReader(text))
	r.FieldsPerRecord = -1
	cells, err := r.Read()
	if err != nil {
		return nil
	}
	return cells
}

// ---------------------------------------------------------------------- the files

// History is the usage files under one directory (the root of a clone of the usage branch): it
// appends rows, each key once per file, and reads them back. It remembers each file's keys once
// it has read them; `Forget` drops that when the files change under it (the clone moved).
type History struct {
	Root string
	keys map[string]map[string]bool
}

// NewHistory is the history under `root`.
func NewHistory(root string) *History {
	return &History{Root: root, keys: map[string]map[string]bool{}}
}

// Forget drops what the history remembers of its files' keys.
func (h *History) Forget() { h.keys = map[string]map[string]bool{} }

// Append adds each row to its file, unless the file (or an earlier row of this call) already holds
// its key; a missing file starts with its header. Returns how many rows it added.
func (h *History) Append(rows ...Row) (int, error) {
	byPath := map[string][]Row{}
	order := []string{}
	for _, row := range rows {
		path := KindOf(row).Path(row.When())
		if _, seen := byPath[path]; !seen {
			order = append(order, path)
		}
		byPath[path] = append(byPath[path], row)
	}
	added := 0
	for _, path := range order {
		kind := KindOf(byPath[path][0])
		keys, err := h.index(kind, path)
		if err != nil {
			return added, err
		}
		var fresh [][]string
		for _, row := range byPath[path] {
			if keys[row.Key()] {
				continue
			}
			keys[row.Key()] = true
			fresh = append(fresh, Encode(row))
		}
		if len(fresh) == 0 {
			continue
		}
		if err := appendRecords(filepath.Join(h.Root, path), kind.Header(), fresh); err != nil {
			return added, err
		}
		added += len(fresh)
	}
	return added, nil
}

// index is the keys `path` holds, read once, after the file is brought under the kind's header.
func (h *History) index(kind *Kind, path string) (map[string]bool, error) {
	if keys, ok := h.keys[path]; ok {
		return keys, nil
	}
	full := filepath.Join(h.Root, path)
	header, records, err := readFile(full)
	if err != nil {
		return nil, err
	}
	keys := map[string]bool{}
	var rows []Row
	for _, record := range records {
		row := newRow(kind)
		decode(header, record, reflect.ValueOf(row).Elem())
		keys[row.Key()] = true
		rows = append(rows, row)
	}
	if header != nil && !sameHeader(header, kind.Header()) {
		// Written before a column joined its header: rewritten under the new one, so the rows
		// appended now line up.
		var out [][]string
		for _, row := range rows {
			out = append(out, Encode(row))
		}
		if err := writeFile(full, kind.Header(), out); err != nil {
			return nil, err
		}
	}
	h.keys[path] = keys
	return keys, nil
}

// newRow is a new, empty row of `kind`, as a pointer.
func newRow(kind *Kind) Row {
	return reflect.New(kind.typ).Interface().(Row)
}

func sameHeader(a, b []string) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

// readFile is a CSV file's header and records; nil, nil when it is not there.
func readFile(path string) ([]string, [][]string, error) {
	data, err := os.ReadFile(path)
	if errors.Is(err, fs.ErrNotExist) {
		return nil, nil, nil
	}
	if err != nil {
		return nil, nil, err
	}
	r := csv.NewReader(bytes.NewReader(data))
	r.FieldsPerRecord = -1
	all, err := r.ReadAll()
	if err != nil {
		return nil, nil, fmt.Errorf("%s: %w", path, err)
	}
	if len(all) == 0 {
		return nil, nil, nil
	}
	return all[0], all[1:], nil
}

// writeFile writes `path` whole: the header, then the records.
func writeFile(path string, header []string, records [][]string) error {
	var buf bytes.Buffer
	w := csv.NewWriter(&buf)
	_ = w.Write(header)
	_ = w.WriteAll(records)
	if err := os.MkdirAll(filepath.Dir(path), 0o777); err != nil {
		return err
	}
	return os.WriteFile(path, buf.Bytes(), 0o666)
}

// appendRecords adds `records` to the end of `path`, starting it with `header` when it is not
// there.
func appendRecords(path string, header []string, records [][]string) error {
	if _, err := os.Stat(path); errors.Is(err, fs.ErrNotExist) {
		return writeFile(path, header, records)
	}
	f, err := os.OpenFile(path, os.O_WRONLY|os.O_APPEND, 0o666)
	if err != nil {
		return err
	}
	w := csv.NewWriter(f)
	_ = w.WriteAll(records)
	if err := w.Error(); err != nil {
		f.Close()
		return err
	}
	return f.Close()
}

// ---------------------------------------------------------------------- reading it back

var monthly = regexp.MustCompile(`^([a-z]+)-(\d{4})-(\d{2})\.csv$`)

// files is the kind's files under `root` that may hold rows in [from, to) (a zero bound leaves
// that side open), oldest first.
func files(root string, kind *Kind, from, to time.Time) ([]string, error) {
	if !kind.Monthly {
		return []string{filepath.Join(root, kind.Path(time.Time{}))}, nil
	}
	entries, err := os.ReadDir(filepath.Join(root, Dir))
	if errors.Is(err, fs.ErrNotExist) {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	var found []string
	for _, entry := range entries {
		m := monthly.FindStringSubmatch(entry.Name())
		if m == nil || m[1] != kind.Name {
			continue
		}
		year, _ := strconv.Atoi(m[2])
		month, _ := strconv.Atoi(m[3])
		start := time.Date(year, time.Month(month), 1, 0, 0, 0, 0, time.UTC)
		end := start.AddDate(0, 1, 0)
		if (!to.IsZero() && !start.Before(to)) || (!from.IsZero() && !end.After(from)) {
			continue
		}
		found = append(found, filepath.Join(root, Dir, entry.Name()))
	}
	sort.Strings(found)
	return found, nil
}

// load is every row of `kind` under `root` whose time is in [from, to), oldest first (a zero
// bound leaves that side open; rows of one time keep their files' order).
func load[T any](root string, kind *Kind, from, to time.Time) ([]*T, error) {
	paths, err := files(root, kind, from, to)
	if err != nil {
		return nil, err
	}
	var out []*T
	for _, path := range paths {
		header, records, err := readFile(path)
		if err != nil {
			return nil, err
		}
		for _, record := range records {
			row := new(T)
			decode(header, record, reflect.ValueOf(row).Elem())
			when := any(row).(Row).When()
			if (!from.IsZero() && when.Before(from)) || (!to.IsZero() && !when.Before(to)) {
				continue
			}
			out = append(out, row)
		}
	}
	sort.SliceStable(out, func(i, j int) bool {
		return any(out[i]).(Row).When().Before(any(out[j]).(Row).When())
	})
	return out, nil
}

// ReadReadings is every reading under `root` (a clone of the usage branch) observed in
// [from, to), oldest first; a zero bound leaves that side open.
func ReadReadings(root string, from, to time.Time) ([]*Reading, error) {
	return load[Reading](root, Readings, from, to)
}

// ReadTicks is every subscription's tick rows under `root` in [from, to), oldest first.
func ReadTicks(root string, from, to time.Time) ([]*Tick, error) {
	return load[Tick](root, Ticks, from, to)
}

// ReadPools is the pool rows under `root` in [from, to), oldest first.
func ReadPools(root string, from, to time.Time) ([]*PoolTick, error) {
	return load[PoolTick](root, Pools, from, to)
}

// ReadSessions is the closed 5-hour sessions under `root` that reset in [from, to), oldest first.
func ReadSessions(root string, from, to time.Time) ([]*Session, error) {
	return load[Session](root, Sessions, from, to)
}

// ReadWeeks is the closed weeks under `root` that reset in [from, to), oldest first.
func ReadWeeks(root string, from, to time.Time) ([]*Week, error) {
	return load[Week](root, Weeks, from, to)
}

// ReadRuns is the runs under `root` that ended in [from, to), oldest first.
func ReadRuns(root string, from, to time.Time) ([]*Run, error) {
	return load[Run](root, Runs, from, to)
}
