package deliver

// The Python behaviours this module leans on that py does not hold: len() of any value, the dicts
// a list is taken to hold, a CPython set's order, and pathlib's joining. Each names the Python it
// stands for.

import (
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
)

// ------------------------------------------------------------------ values

// isPriorityTier is `lowered in queue.PRIORITY_TIERS`.
func isPriorityTier(lowered string) bool {
	_, ok := queue.PriorityTiers[lowered]
	return ok
}

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

// pySleep is time.sleep, which `journal.append` takes.
func pySleep(seconds float64) { time.Sleep(time.Duration(seconds * float64(time.Second))) }
