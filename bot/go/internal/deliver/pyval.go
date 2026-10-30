package deliver

// The Python behaviours this module leans on that the foundation packages do not hold (yet):
// int() and float() of a JSON value, `x or default`, iteration and len() of any value, a file
// read with errors="replace", a CPython set's order, and the constants of packages whose Go type
// is not settled (the stubs' `any`). Each names the Python it stands for.

import (
	"fmt"
	"math"
	"os"
	"path/filepath"
	"reflect"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/human"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/identity"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
)

// ------------------------------------------------------------------ the running bot's words

// The running bot's labels and words (`config.py`, `IDENTITY`). Read through these so this package
// builds whichever Go type the config package gives each (its stub's `any`).
func labelBuild() string      { return py.Str(config.LabelBuild) }
func labelBlocked() string    { return py.Str(config.LabelBlocked) }
func labelCross() string      { return py.Str(config.LabelCross) }
func labelNeedsPlan() string  { return py.Str(config.LabelNeedsPlan) }
func labelPlanned() string    { return py.Str(config.LabelPlanned) }
func labelPR() string         { return py.Str(config.LabelPR) }
func labelPROpen() string     { return py.Str(config.LabelPROpen) }
func labelRevise() string     { return py.Str(config.LabelRevise) }
func labelStuck() string      { return py.Str(config.LabelStuck) }
func labelSuggestion() string { return py.Str(config.LabelSuggestion) }
func labelTree() string       { return py.Str(config.LabelTree) }
func commitPrefix() string    { return py.Str(config.CommitPrefix) }
func slash() string           { return py.Str(config.Slash) }
func stateBranch() string     { return py.Str(config.StateBranch) }
func botTitle() string        { return py.Str(config.Title) }

// others is `config.OTHERS`: the other bots in the repository, in their order.
func others() []*identity.Other {
	switch v := any(config.Others).(type) {
	case []*identity.Other:
		return v
	case []identity.Other:
		out := make([]*identity.Other, len(v))
		for i := range v {
			out[i] = &v[i]
		}
		return out
	case []any:
		var out []*identity.Other
		for _, e := range v {
			switch o := e.(type) {
			case *identity.Other:
				out = append(out, o)
			case identity.Other:
				out = append(out, &o)
			}
		}
		return out
	}
	return nil
}

// labelSpec is `LABELS[name]`: the label's (colour, description).
func labelSpec(name string) (string, string) {
	pair := func(v any) (string, string) {
		switch p := v.(type) {
		case [2]string:
			return p[0], p[1]
		case []string:
			if len(p) == 2 {
				return p[0], p[1]
			}
		case []any:
			if len(p) == 2 {
				return py.Str(p[0]), py.Str(p[1])
			}
		}
		panic(fmt.Errorf("KeyError: %s", py.Repr(name)))
	}
	switch m := any(config.Labels).(type) {
	case *pyjson.Object:
		if v, ok := m.Lookup(name); ok {
			return pair(v)
		}
	case map[string][2]string:
		if v, ok := m[name]; ok {
			return v[0], v[1]
		}
	case map[string][]string:
		if v, ok := m[name]; ok {
			return pair(v)
		}
	case map[string]any:
		if v, ok := m[name]; ok {
			return pair(v)
		}
	}
	panic(fmt.Errorf("KeyError: %s", py.Repr(name)))
}

// hasKey is `key in mapping` for the mappings and sets of other modules whose Go type is not
// settled: `DIFFICULTY_LABELS`, `queue.PRIORITY_TIERS`, `human.LETS_GO`.
func hasKey(mapping any, key string) bool {
	switch m := mapping.(type) {
	case map[string]string:
		_, ok := m[key]
		return ok
	case map[string]int:
		_, ok := m[key]
		return ok
	case map[string]bool:
		_, ok := m[key]
		return ok
	case map[string]struct{}:
		_, ok := m[key]
		return ok
	case map[string]any:
		_, ok := m[key]
		return ok
	case *pyjson.Object:
		return m.Has(key)
	case []string:
		for _, e := range m {
			if e == key {
				return true
			}
		}
	}
	return false
}

func isDifficultyLabel(lowered string) bool { return hasKey(any(config.DifficultyLabels), lowered) }
func isPriorityTier(lowered string) bool    { return hasKey(any(queue.PriorityTiers), lowered) }
func letsGo(name string) bool               { return hasKey(any(human.LetsGo), name) }

// lastBackoff is `providers.INFRA_BACKOFFS[-1]`.
func lastBackoff() time.Duration {
	switch v := any(providers.InfraBackoffs).(type) {
	case []time.Duration:
		if len(v) > 0 {
			return v[len(v)-1]
		}
	case []any:
		if len(v) > 0 {
			if d, ok := v[len(v)-1].(time.Duration); ok {
				return d
			}
		}
	}
	panic(fmt.Errorf("IndexError: tuple index out of range"))
}

// ------------------------------------------------------------------ values

// orStr is `str(v or def)`: v as Python prints it when it is truthy, else def.
func orStr(v any, def string) string {
	if pyjson.Truthy(v) {
		return py.Str(v)
	}
	return def
}

// orList is `v or []`.
func orList(v any) any {
	if pyjson.Truthy(v) {
		return v
	}
	return []any{}
}

// firstTruthy is `a or b or …`: the first truthy value, else the last.
func firstTruthy(vs ...any) any {
	for _, v := range vs {
		if pyjson.Truthy(v) {
			return v
		}
	}
	if len(vs) == 0 {
		return nil
	}
	return vs[len(vs)-1]
}

// iterate is `for e in v` over a JSON value: a list's items, an object's keys, a string's
// characters; nothing for anything else.
func iterate(v any) []any {
	switch x := v.(type) {
	case []any:
		return x
	case []string:
		return pyjson.ListOf(x)
	case *pyjson.Object:
		return pyjson.ListOf(x.Keys())
	case string:
		out := []any{}
		for _, r := range x {
			out = append(out, string(r))
		}
		return out
	}
	return nil
}

// objList is `v or []` iterated as the dicts Python's code takes it to hold: each item an object,
// nil (which reads as empty) for an item that is not one, so its length is the list's.
func objList(v any) []*pyjson.Object {
	items := iterate(v)
	out := make([]*pyjson.Object, len(items))
	for i, e := range items {
		out[i] = pyjson.Obj(e)
	}
	return out
}

// pyLen is len(v): a list's items, an object's keys, a string's characters; 0 for anything else.
func pyLen(v any) int {
	switch x := v.(type) {
	case []any:
		return len(x)
	case []string:
		return len(x)
	case *pyjson.Object:
		return x.Len()
	case string:
		return py.Len(x)
	}
	return 0
}

var intLiteral = regexp.MustCompile(`^[+-]?[0-9]+(_[0-9]+)*$`)

// pyInt is int(v) on a JSON value: an int, a float cut toward zero, a bool, or a string of an
// integer (surrounding spaces and single underscores between digits allowed, as Python allows).
// ok is false where Python's int() raises.
func pyInt(v any) (int, bool) {
	switch x := v.(type) {
	case bool:
		if x {
			return 1, true
		}
		return 0, true
	case string:
		s := strings.TrimSpace(x)
		if !intLiteral.MatchString(s) {
			return 0, false
		}
		n, err := strconv.Atoi(strings.ReplaceAll(s, "_", ""))
		return n, err == nil
	case float64:
		if math.IsNaN(x) || math.IsInf(x, 0) {
			return 0, false
		}
	}
	return pyjson.AsInt(v)
}

// mustInt is int(v), raising as Python's int() does when v is no integer.
func mustInt(v any) int {
	n, ok := pyInt(v)
	if !ok {
		if s, isStr := v.(string); isStr {
			panic(fmt.Errorf("ValueError: invalid literal for int() with base 10: %s", py.Repr(s)))
		}
		panic(fmt.Errorf("TypeError: int() argument must be a string, a bytes-like object or a real number, not %s", py.Repr(typeName(v))))
	}
	return n
}

// typeName is type(v).__name__ for a JSON value.
func typeName(v any) string {
	switch v.(type) {
	case nil:
		return "NoneType"
	case *pyjson.Object:
		return "dict"
	case []any:
		return "list"
	case string:
		return "str"
	}
	return fmt.Sprintf("%T", v)
}

// intOr0 is `int(v or 0)`.
func intOr0(v any) int {
	if !pyjson.Truthy(v) {
		return 0
	}
	return mustInt(v)
}

// tryIntOr0 is `try: int(v or 0) except (TypeError, ValueError): 0`.
func tryIntOr0(v any) int {
	if !pyjson.Truthy(v) {
		return 0
	}
	n, ok := pyInt(v)
	if !ok {
		return 0
	}
	return n
}

// pyFloat is float(v) on a JSON value: a number, a bool, or a string of one (Python's spellings of
// infinity and nan, and underscores between digits, included). ok is false where float() raises.
func pyFloat(v any) (float64, bool) {
	switch x := v.(type) {
	case bool:
		if x {
			return 1, true
		}
		return 0, true
	case string:
		s := strings.ToLower(strings.TrimSpace(x))
		switch strings.TrimLeft(s, "+-") {
		case "inf", "infinity", "nan":
			f, err := strconv.ParseFloat(s, 64)
			return f, err == nil
		}
		if strings.Contains(s, "__") || strings.HasPrefix(strings.TrimLeft(s, "+-"), "_") ||
			strings.HasSuffix(s, "_") || strings.HasPrefix(s, "0x") || strings.HasPrefix(s, "-0x") ||
			strings.HasPrefix(s, "+0x") {
			return 0, false
		}
		f, err := strconv.ParseFloat(strings.ReplaceAll(s, "_", ""), 64)
		if err != nil {
			if ne, ok := err.(*strconv.NumError); ok && ne.Err == strconv.ErrRange {
				return f, true // Python reads an overflowing literal as inf too
			}
			return 0, false
		}
		return f, true
	}
	return pyjson.AsFloat(v)
}

// tryFloatOr0 is `try: float(v or 0) except (TypeError, ValueError): 0.0`.
func tryFloatOr0(v any) float64 {
	if !pyjson.Truthy(v) {
		return 0
	}
	f, ok := pyFloat(v)
	if !ok {
		return 0
	}
	return f
}

// containsEqual is `v in values` with Python's ==.
func containsEqual(values []any, v any) bool {
	for _, e := range values {
		if pyjson.Equal(e, v) {
			return true
		}
	}
	return false
}

// uniq is `list(dict.fromkeys(values))`: the values in order, each once.
func uniq(values []any) []any {
	out := []any{}
	for _, v := range values {
		if !containsEqual(out, v) {
			out = append(out, v)
		}
	}
	return out
}

// uniqStrings is `list(dict.fromkeys(values))` over strings.
func uniqStrings(values []string) []string {
	seen := map[string]bool{}
	out := []string{}
	for _, v := range values {
		if !seen[v] {
			seen[v] = true
			out = append(out, v)
		}
	}
	return out
}

// sortedKeys is `sorted(names)` of a set of strings (Python sorts by code point, as Go's byte
// order of UTF-8 does).
func sortedKeys(set map[string]bool) []string {
	out := make([]string, 0, len(set))
	for k := range set {
		out = append(out, k)
	}
	sort.Strings(out)
	return out
}

// headN is `xs[:n]`.
func headN[T any](xs []T, n int) []T {
	if n < 0 {
		n = 0
	}
	if len(xs) > n {
		return xs[:n]
	}
	return xs
}

// ticked is `", ".join(f"`{p}`" for p in paths)`.
func ticked(paths []string) string {
	parts := make([]string, len(paths))
	for i, p := range paths {
		parts[i] = "`" + p + "`"
	}
	return strings.Join(parts, ", ")
}

// shownPaths is `", ".join(f"`{p}`" for p in paths[:8]) + (", …" if len(paths) > 8 else "")`.
func shownPaths(paths []string) string {
	text := ticked(headN(paths, 8))
	if len(paths) > 8 {
		text += ", …"
	}
	return text
}

// setOrder is the order CPython iterates the set `{a, b}` of two non-negative ints in: by slot
// of its eight-slot table (an int hashes to itself), the second probing on as `set_add_entry`
// does when its slot is taken. Python's `for target in {issue, pr}` meets GitHub in this order.
func setOrder(a, b int) []int {
	if a == b {
		return []int{a}
	}
	const mask = 7
	var table [mask + 1]int
	var used [mask + 1]bool
	for _, n := range []int{a, b} {
		i := uint64(n) & mask
		perturb := uint64(n)
		for used[i] {
			perturb >>= 5
			i = (i*5 + 1 + perturb) & mask
		}
		used[i], table[i] = true, n
	}
	var out []int
	for i := range table {
		if used[i] {
			out = append(out, table[i])
		}
	}
	return out
}

// ------------------------------------------------------------------ files and processes

// outPath is `Path(out_dir) / name`: an absolute name stands for itself.
func outPath(dir, name string) string {
	if filepath.IsAbs(name) {
		return name
	}
	return filepath.Join(dir, name)
}

// isFile is Path.is_file(): a regular file, through any symlink.
func isFile(path string) bool {
	info, err := os.Stat(path)
	return err == nil && info.Mode().IsRegular()
}

// runnerName is `re.fullmatch(r"[\w.-]{1,80}", …)`, `\w` as Python's str patterns read it (any
// letter or digit, and `_`).
var runnerName = regexp.MustCompile(`^[\p{L}\p{N}_.-]{1,80}$`)

// completedProcess is what `Git.run` returns (Python's subprocess.CompletedProcess): its
// return code and its output.
type completedProcess struct {
	returncode int
	stdout     string
}

// completed reads `Git.run`'s answer, whose Go type the git package's stub leaves open (`any`):
// a struct, or a pointer to one, with a return code field (`Returncode`, `ReturnCode`,
// `ExitCode` or `Code`) and a `Stdout` field, or methods of those names.
func completed(v any) completedProcess {
	var out completedProcess
	rv := reflect.ValueOf(v)
	field := func(names ...string) (reflect.Value, bool) {
		for _, name := range names {
			if m := rv.MethodByName(name); m.IsValid() && m.Type().NumIn() == 0 && m.Type().NumOut() == 1 {
				return m.Call(nil)[0], true
			}
		}
		s := rv
		for s.Kind() == reflect.Pointer || s.Kind() == reflect.Interface {
			if s.IsNil() {
				return reflect.Value{}, false
			}
			s = s.Elem()
		}
		if s.Kind() != reflect.Struct {
			return reflect.Value{}, false
		}
		for _, name := range names {
			if f := s.FieldByNameFunc(func(n string) bool { return strings.EqualFold(n, name) }); f.IsValid() {
				return f, true
			}
		}
		return reflect.Value{}, false
	}
	if !rv.IsValid() {
		return out
	}
	if f, ok := field("Returncode", "ReturnCode", "ExitCode", "Code"); ok && f.CanInt() {
		out.returncode = int(f.Int())
	}
	if f, ok := field("Stdout"); ok {
		switch f.Kind() {
		case reflect.String:
			out.stdout = f.String()
		case reflect.Slice:
			if f.Type().Elem().Kind() == reflect.Uint8 {
				out.stdout = string(f.Bytes())
			}
		}
	}
	return out
}

// pySleep is time.sleep, which `journal.append` takes.
func pySleep(seconds float64) { time.Sleep(time.Duration(seconds * float64(time.Second))) }
