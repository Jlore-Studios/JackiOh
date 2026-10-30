package deliver

// The Python behaviours this module leans on that the foundation packages do not hold (yet):
// int() and float() of a JSON value, `x or default`, iteration and len() of any value, a file
// read with errors="replace", a CPython set's order, and the constants of packages whose Go type
// is not settled (the stubs' `any`). Each names the Python it stands for.

import (
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"regexp"
	"sort"
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

// objList is `v or []` iterated as the dicts Python's code takes it to hold: each item an object,
// nil (which reads as empty) for an item that is not one, so its length is the list's.
func objList(v any) []*pyjson.Object {
	items := py.Each(v)
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
var runnerName = regexp.MustCompile(`^[` + py.WordClass + `.-]{1,80}$`)

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
