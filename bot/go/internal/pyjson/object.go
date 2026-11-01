// Package pyjson holds JSON the way the Python harness held it: objects keep their keys in the
// order they arrived or were set (a Python dict), integers stay integers and floats stay floats,
// and Dumps writes text byte for byte as Python's json.dumps does, so the state branch, the plans
// and every hash over JSON read the same whichever harness wrote them.
//
// A value is one of: nil, bool, int, float64, string, []any or *Object. Loads returns only those.
// Dumps also takes the other Go slices and maps of them, and anything with a PyJSON method.
package pyjson

// Object is a JSON object whose keys keep their order. A nil *Object reads as an empty one, so
// `o.Obj("head").Str("ref")` is Python's `(o.get("head") or {}).get("ref")`.
type Object struct {
	keys []string
	vals map[string]any
}

// New builds an object from key, value pairs: New("number", 1, "title", "x").
func New(kv ...any) *Object {
	o := &Object{vals: make(map[string]any, len(kv)/2)}
	for i := 0; i+1 < len(kv); i += 2 {
		o.Set(kv[i].(string), kv[i+1])
	}
	return o
}

// Len is len(dict).
func (o *Object) Len() int {
	if o == nil {
		return 0
	}
	return len(o.keys)
}

// Keys is list(dict): the keys in order. The slice is a copy.
func (o *Object) Keys() []string {
	if o == nil {
		return nil
	}
	return append([]string(nil), o.keys...)
}

// Lookup is `key in dict` and dict[key] together.
func (o *Object) Lookup(key string) (any, bool) {
	if o == nil {
		return nil, false
	}
	v, ok := o.vals[key]
	return v, ok
}

// Has is `key in dict`.
func (o *Object) Has(key string) bool {
	_, ok := o.Lookup(key)
	return ok
}

// Get is dict.get(key): nil when absent.
func (o *Object) Get(key string) any {
	v, _ := o.Lookup(key)
	return v
}

// GetOr is dict.get(key, def): def only when the key is absent (a present null stays nil).
func (o *Object) GetOr(key string, def any) any {
	if v, ok := o.Lookup(key); ok {
		return v
	}
	return def
}

// Set is dict[key] = v: a new key goes last, an existing one keeps its place. It returns o.
func (o *Object) Set(key string, v any) *Object {
	if o.vals == nil {
		o.vals = map[string]any{}
	}
	if _, ok := o.vals[key]; !ok {
		o.keys = append(o.keys, key)
	}
	o.vals[key] = v
	return o
}

// SetDefault is dict.setdefault(key, v).
func (o *Object) SetDefault(key string, v any) any {
	if cur, ok := o.Lookup(key); ok {
		return cur
	}
	o.Set(key, v)
	return v
}

// Pop is dict.pop(key, None), and says whether the key was there.
func (o *Object) Pop(key string) (any, bool) {
	if o == nil {
		return nil, false
	}
	v, ok := o.vals[key]
	if !ok {
		return nil, false
	}
	delete(o.vals, key)
	for i, k := range o.keys {
		if k == key {
			o.keys = append(o.keys[:i:i], o.keys[i+1:]...)
			break
		}
	}
	return v, true
}

// Delete is `del dict[key]`, without the KeyError.
func (o *Object) Delete(key string) { o.Pop(key) }

// Update is dict.update(other): other's keys in its order, existing ones keeping their place.
func (o *Object) Update(other *Object) *Object {
	for _, k := range other.Keys() {
		o.Set(k, other.vals[k])
	}
	return o
}

// Copy is dict(o): a shallow copy. A nil object copies to an empty one.
func (o *Object) Copy() *Object {
	c := &Object{vals: make(map[string]any, o.Len())}
	if o != nil {
		c.keys = append([]string(nil), o.keys...)
		for k, v := range o.vals {
			c.vals[k] = v
		}
	}
	return c
}

// Items calls fn for each key and value in order until fn returns false.
func (o *Object) Items(fn func(key string, v any) bool) {
	for _, k := range o.Keys() {
		if !fn(k, o.vals[k]) {
			return
		}
	}
}

// Str is the value at key when it is a string, else "".
func (o *Object) Str(key string) string { s, _ := o.Get(key).(string); return s }

// Int is the value at key as an int when it is a number (a float is truncated), else 0.
func (o *Object) Int(key string) int { n, _ := AsInt(o.Get(key)); return n }

// Float is the value at key as a float64 when it is a number, else 0.
func (o *Object) Float(key string) float64 { f, _ := AsFloat(o.Get(key)); return f }

// Bool is the value at key when it is a bool, else false. For Python's truthiness use Truthy.
func (o *Object) Bool(key string) bool { b, _ := o.Get(key).(bool); return b }

// Obj is the value at key when it is an object, else nil (which reads as empty).
func (o *Object) Obj(key string) *Object { m, _ := o.Get(key).(*Object); return m }

// List is the value at key when it is a list, else nil.
func (o *Object) List(key string) []any { l, _ := o.Get(key).([]any); return l }

// Truthy is bool(dict.get(key)) in Python.
func (o *Object) Truthy(key string) bool { return Truthy(o.Get(key)) }

// Equal is Python's == between two JSON values: objects compare without regard to key order,
// and an int equals a float of the same value.
func Equal(a, b any) bool {
	switch x := a.(type) {
	case *Object:
		y, ok := b.(*Object)
		if !ok || x.Len() != y.Len() {
			return false
		}
		for _, k := range x.Keys() {
			yv, ok := y.Lookup(k)
			if !ok || !Equal(x.vals[k], yv) {
				return false
			}
		}
		return true
	case []any:
		y, ok := b.([]any)
		if !ok || len(x) != len(y) {
			return false
		}
		for i := range x {
			if !Equal(x[i], y[i]) {
				return false
			}
		}
		return true
	case nil:
		return b == nil
	case string:
		y, ok := b.(string)
		return ok && x == y
	case bool:
		y, ok := b.(bool)
		return ok && x == y
	}
	if fa, ok := AsFloat(a); ok {
		if fb, ok := AsFloat(b); ok {
			return fa == fb
		}
	}
	return false
}

// DeepCopy is copy.deepcopy over JSON values.
func DeepCopy(v any) any {
	switch x := v.(type) {
	case *Object:
		if x == nil {
			return x
		}
		c := &Object{keys: append([]string(nil), x.keys...), vals: make(map[string]any, len(x.vals))}
		for k, e := range x.vals {
			c.vals[k] = DeepCopy(e)
		}
		return c
	case []any:
		c := make([]any, len(x))
		for i, e := range x {
			c[i] = DeepCopy(e)
		}
		return c
	}
	return v
}

// Truthy is Python's bool(v) for a JSON value: nil, false, 0, 0.0, "", [] and {} are false.
func Truthy(v any) bool {
	switch x := v.(type) {
	case nil:
		return false
	case bool:
		return x
	case string:
		return x != ""
	case []any:
		return len(x) > 0
	case []string:
		return len(x) > 0
	case *Object:
		return x.Len() > 0
	case map[string]any:
		return len(x) > 0
	}
	if f, ok := AsFloat(v); ok {
		return f != 0
	}
	return true
}

// AsInt is a JSON number as an int: an int as is, a float truncated (Python's int()).
func AsInt(v any) (int, bool) {
	switch x := v.(type) {
	case int:
		return x, true
	case int64:
		return int(x), true
	case int32:
		return int(x), true
	case float64:
		return int(x), true
	case float32:
		return int(x), true
	}
	return 0, false
}

// AsFloat is a JSON number as a float64.
func AsFloat(v any) (float64, bool) {
	switch x := v.(type) {
	case int:
		return float64(x), true
	case int64:
		return float64(x), true
	case int32:
		return float64(x), true
	case float64:
		return x, true
	case float32:
		return float64(x), true
	}
	return 0, false
}

// Str is v when it is a string, else "".
func Str(v any) string { s, _ := v.(string); return s }

// Obj is v when it is an object, else nil.
func Obj(v any) *Object { o, _ := v.(*Object); return o }

// List is v when it is a list, else nil.
func List(v any) []any { l, _ := v.([]any); return l }

// Objs is the objects in a list, skipping anything else.
func Objs(v any) []*Object {
	var out []*Object
	for _, e := range List(v) {
		if o, ok := e.(*Object); ok {
			out = append(out, o)
		}
	}
	return out
}

// Strings is the strings in a list, skipping anything else.
func Strings(v any) []string {
	var out []string
	for _, e := range List(v) {
		if s, ok := e.(string); ok {
			out = append(out, s)
		}
	}
	return out
}

// ListOf turns a typed slice into a JSON list.
func ListOf[T any](xs []T) []any {
	out := make([]any, len(xs))
	for i, x := range xs {
		out[i] = x
	}
	return out
}
