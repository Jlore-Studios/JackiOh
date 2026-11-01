package fakes

import (
	"fmt"
	"iter"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// Dict is a Python dict whose keys are not text (`FakeGitHub.Threads`, keyed by number): its keys
// keep the order they were first set in, as a dict's do, so "the first one" and a listing in
// insertion order come out as the Python's. A dict keyed by text is a *pyjson.Object instead.
type Dict[K comparable, V any] struct {
	keys []K
	vals map[K]V
}

// NewDict is {}.
func NewDict[K comparable, V any]() *Dict[K, V] {
	return &Dict[K, V]{vals: map[K]V{}}
}

// Len is len(d).
func (d *Dict[K, V]) Len() int { return len(d.keys) }

// Keys is list(d): the keys in order. The slice is a copy.
func (d *Dict[K, V]) Keys() []K { return append([]K(nil), d.keys...) }

// Values is list(d.values()), in order.
func (d *Dict[K, V]) Values() []V {
	out := make([]V, 0, len(d.keys))
	for _, k := range d.keys {
		out = append(out, d.vals[k])
	}
	return out
}

// All is d.items(), in order: `for n, thread := range gh.Threads.All()`. It walks a copy of the
// keys, so the loop may set and delete.
func (d *Dict[K, V]) All() iter.Seq2[K, V] {
	return func(yield func(K, V) bool) {
		for _, k := range d.Keys() {
			v, ok := d.vals[k]
			if ok && !yield(k, v) {
				return
			}
		}
	}
}

// Lookup is `k in d` and d[k] together.
func (d *Dict[K, V]) Lookup(k K) (V, bool) {
	v, ok := d.vals[k]
	return v, ok
}

// Has is `k in d`.
func (d *Dict[K, V]) Has(k K) bool {
	_, ok := d.vals[k]
	return ok
}

// Get is d.get(k): the zero value when absent.
func (d *Dict[K, V]) Get(k K) V { return d.vals[k] }

// Item is d[k]: a KeyError when absent.
func (d *Dict[K, V]) Item(k K) V {
	v, ok := d.vals[k]
	if !ok {
		panic(keyError(k))
	}
	return v
}

// Set is d[k] = v: a new key goes last, an existing one keeps its place.
func (d *Dict[K, V]) Set(k K, v V) {
	if d.vals == nil {
		d.vals = map[K]V{}
	}
	if _, ok := d.vals[k]; !ok {
		d.keys = append(d.keys, k)
	}
	d.vals[k] = v
}

// SetDefault is d.setdefault(k, v).
func (d *Dict[K, V]) SetDefault(k K, v V) V {
	if cur, ok := d.vals[k]; ok {
		return cur
	}
	d.Set(k, v)
	return v
}

// Pop is d.pop(k, None), and says whether the key was there.
func (d *Dict[K, V]) Pop(k K) (V, bool) {
	v, ok := d.vals[k]
	if !ok {
		return v, false
	}
	delete(d.vals, k)
	for i, key := range d.keys {
		if key == k {
			d.keys = append(d.keys[:i:i], d.keys[i+1:]...)
			break
		}
	}
	return v, true
}

// Delete is `del d[k]`, without the KeyError.
func (d *Dict[K, V]) Delete(k K) { d.Pop(k) }

// Clear is d.clear().
func (d *Dict[K, V]) Clear() {
	d.keys = nil
	d.vals = map[K]V{}
}

// PyError is one of Python's own exceptions, raised (panicked) where fakes.py's Python would raise
// it: a KeyError for a missing thread, an AttributeError for a method fakes.py does not define,
// StopIteration, ValueError, TypeError, AssertionError from Git, FileExistsError. Type is the
// class's name; Error() is str(e). herr.Try catches it as `except Exception` does.
type PyError struct {
	Type string
	Msg  string
}

func (e *PyError) Error() string { return e.Msg }

// keyError is KeyError(k): its str is repr(k).
func keyError(k any) *PyError {
	var msg string
	switch key := k.(type) {
	case int, string:
		msg = py.Repr(key)
	case [2]string:
		msg = "(" + py.Repr(key[0]) + ", " + py.Repr(key[1]) + ")"
	default:
		msg = fmt.Sprint(key)
	}
	return &PyError{Type: "KeyError", Msg: msg}
}
