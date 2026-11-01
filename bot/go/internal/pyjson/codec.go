package pyjson

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"math"
	"reflect"
	"sort"
	"strconv"
	"strings"
)

// Marshaler is a Go value that writes itself as a JSON value (a dataclass's asdict).
type Marshaler interface {
	PyJSON() any
}

// Loads is json.loads: objects keep their key order, a number without a fraction or exponent is
// an int and any other a float64. A key that comes twice keeps its first place and its last value,
// as a Python dict built from the pairs does.
func Loads(data []byte) (any, error) {
	dec := json.NewDecoder(bytes.NewReader(data))
	dec.UseNumber()
	v, err := decode(dec)
	if err != nil {
		return nil, err
	}
	if _, err := dec.Token(); err != io.EOF {
		return nil, fmt.Errorf("pyjson: extra data after the value")
	}
	return v, nil
}

// LoadsString is Loads over a string.
func LoadsString(s string) (any, error) { return Loads([]byte(s)) }

func decode(dec *json.Decoder) (any, error) {
	tok, err := dec.Token()
	if err != nil {
		return nil, err
	}
	switch t := tok.(type) {
	case json.Delim:
		switch t {
		case '{':
			o := New()
			for dec.More() {
				kt, err := dec.Token()
				if err != nil {
					return nil, err
				}
				key, ok := kt.(string)
				if !ok {
					return nil, fmt.Errorf("pyjson: object key is not a string")
				}
				v, err := decode(dec)
				if err != nil {
					return nil, err
				}
				o.Set(key, v)
			}
			if _, err := dec.Token(); err != nil {
				return nil, err
			}
			return o, nil
		case '[':
			list := []any{}
			for dec.More() {
				v, err := decode(dec)
				if err != nil {
					return nil, err
				}
				list = append(list, v)
			}
			if _, err := dec.Token(); err != nil {
				return nil, err
			}
			return list, nil
		}
		return nil, fmt.Errorf("pyjson: unexpected %v", t)
	case json.Number:
		s := string(t)
		if !strings.ContainsAny(s, ".eE") {
			if n, err := strconv.ParseInt(s, 10, 64); err == nil {
				return int(n), nil
			}
		}
		f, err := strconv.ParseFloat(s, 64)
		if err != nil {
			return nil, err
		}
		return f, nil
	default:
		return t, nil // string, bool or nil
	}
}

// Options are json.dumps's keyword arguments. The zero value is json.dumps(v) with
// ensure_ascii on (EnsureASCII is inverted as NoASCII so that the zero value matches Python).
type Options struct {
	// Indent is indent=n; 0 with Indented false is indent=None.
	Indent   int
	Indented bool
	SortKeys bool
	// NoASCII is ensure_ascii=False.
	NoASCII bool
	// ItemSep and KeySep override separators=(item, key); empty means Python's default for the
	// indent: (", ", ": ") without one, (",", ": ") with one.
	ItemSep, KeySep string
}

// Dumps is json.dumps(v).
func Dumps(v any) string { return DumpsWith(v, Options{}) }

// DumpsIndent is json.dumps(v, indent=n).
func DumpsIndent(v any, n int) string { return DumpsWith(v, Options{Indent: n, Indented: true}) }

// DumpsWith is json.dumps(v, **opts). It panics on a value JSON cannot hold, as Python raises.
func DumpsWith(v any, opts Options) string {
	e := encoder{opts: opts, item: opts.ItemSep, key: opts.KeySep}
	if e.item == "" {
		e.item = ", "
		if opts.Indented {
			e.item = ","
		}
	}
	if e.key == "" {
		e.key = ": "
	}
	e.value(v, 0)
	return e.b.String()
}

type encoder struct {
	b         strings.Builder
	opts      Options
	item, key string
}

func (e *encoder) newline(level int) {
	if e.opts.Indented {
		e.b.WriteByte('\n')
		e.b.WriteString(strings.Repeat(" ", e.opts.Indent*level))
	}
}

func (e *encoder) value(v any, level int) {
	switch x := v.(type) {
	case nil:
		e.b.WriteString("null")
	case bool:
		if x {
			e.b.WriteString("true")
		} else {
			e.b.WriteString("false")
		}
	case string:
		e.b.WriteString(Quote(x, !e.opts.NoASCII))
	case int:
		e.b.WriteString(strconv.Itoa(x))
	case int64:
		e.b.WriteString(strconv.FormatInt(x, 10))
	case int32:
		e.b.WriteString(strconv.FormatInt(int64(x), 10))
	case float64:
		e.b.WriteString(floatJSON(x))
	case float32:
		e.b.WriteString(floatJSON(float64(x)))
	case *Object:
		if x == nil {
			e.b.WriteString("null")
			return
		}
		keys := x.Keys()
		if e.opts.SortKeys {
			sort.Strings(keys)
		}
		e.object(keys, func(k string) any { return x.vals[k] }, level)
	case map[string]any:
		keys := make([]string, 0, len(x))
		for k := range x {
			keys = append(keys, k)
		}
		sort.Strings(keys) // a Go map has no order: sorted is the one deterministic choice
		e.object(keys, func(k string) any { return x[k] }, level)
	case []any:
		e.list(len(x), func(i int) any { return x[i] }, level)
	case Marshaler:
		e.value(x.PyJSON(), level)
	default:
		rv := reflect.ValueOf(v)
		switch rv.Kind() {
		case reflect.Slice, reflect.Array:
			if rv.Kind() == reflect.Slice && rv.IsNil() {
				e.b.WriteString("[]")
				return
			}
			e.list(rv.Len(), func(i int) any { return rv.Index(i).Interface() }, level)
			return
		case reflect.Map:
			if rv.Type().Key().Kind() == reflect.String {
				keys := make([]string, 0, rv.Len())
				for _, k := range rv.MapKeys() {
					keys = append(keys, k.String())
				}
				sort.Strings(keys)
				e.object(keys, func(k string) any {
					return rv.MapIndex(reflect.ValueOf(k).Convert(rv.Type().Key())).Interface()
				}, level)
				return
			}
		case reflect.Pointer:
			if rv.IsNil() {
				e.b.WriteString("null")
				return
			}
			e.value(rv.Elem().Interface(), level)
			return
		case reflect.Int, reflect.Int8, reflect.Int16, reflect.Uint, reflect.Uint8, reflect.Uint16,
			reflect.Uint32, reflect.Uint64:
			e.b.WriteString(fmt.Sprint(v))
			return
		case reflect.String:
			e.b.WriteString(Quote(rv.String(), !e.opts.NoASCII))
			return
		}
		panic(fmt.Sprintf("pyjson: Object of type %T is not JSON serializable", v))
	}
}

func (e *encoder) object(keys []string, get func(string) any, level int) {
	if len(keys) == 0 {
		e.b.WriteString("{}")
		return
	}
	e.b.WriteByte('{')
	for i, k := range keys {
		if i > 0 {
			e.b.WriteString(e.item)
		}
		e.newline(level + 1)
		e.b.WriteString(Quote(k, !e.opts.NoASCII))
		e.b.WriteString(e.key)
		e.value(get(k), level+1)
	}
	e.newline(level)
	e.b.WriteByte('}')
}

func (e *encoder) list(n int, get func(int) any, level int) {
	if n == 0 {
		e.b.WriteString("[]")
		return
	}
	e.b.WriteByte('[')
	for i := 0; i < n; i++ {
		if i > 0 {
			e.b.WriteString(e.item)
		}
		e.newline(level + 1)
		e.value(get(i), level+1)
	}
	e.newline(level)
	e.b.WriteByte(']')
}

// Quote is a JSON string as Python writes it: with ascii, every character outside space to tilde
// is a \u escape (lowercase hex, a surrogate pair above the BMP); without it, only the control
// characters, the quote and the backslash are escaped.
func Quote(s string, ascii bool) string {
	var b strings.Builder
	b.WriteByte('"')
	for _, r := range s {
		switch r {
		case '"':
			b.WriteString(`\"`)
		case '\\':
			b.WriteString(`\\`)
		case '\n':
			b.WriteString(`\n`)
		case '\r':
			b.WriteString(`\r`)
		case '\t':
			b.WriteString(`\t`)
		case '\b':
			b.WriteString(`\b`)
		case '\f':
			b.WriteString(`\f`)
		default:
			switch {
			case r < 0x20:
				fmt.Fprintf(&b, `\u%04x`, r)
			case !ascii || r <= 0x7e:
				b.WriteRune(r)
			case r > 0xffff:
				r -= 0x10000
				fmt.Fprintf(&b, `\u%04x\u%04x`, 0xd800|((r>>10)&0x3ff), 0xdc00|(r&0x3ff))
			default:
				fmt.Fprintf(&b, `\u%04x`, r)
			}
		}
	}
	b.WriteByte('"')
	return b.String()
}

func floatJSON(f float64) string {
	switch {
	case math.IsNaN(f):
		return "NaN"
	case math.IsInf(f, 1):
		return "Infinity"
	case math.IsInf(f, -1):
		return "-Infinity"
	}
	return FloatRepr(f)
}

// FloatRepr is Python's repr(float): the shortest digits that read back the same, in plain
// notation from 1e-4 up to 1e16 (with ".0" on a whole number) and in exponent notation outside
// it ("1e+16", "1.5e-05").
func FloatRepr(f float64) string {
	switch {
	case math.IsNaN(f):
		return "nan"
	case math.IsInf(f, 1):
		return "inf"
	case math.IsInf(f, -1):
		return "-inf"
	}
	s := strconv.FormatFloat(f, 'e', -1, 64) // d.ddde±xx
	mant, expPart, _ := strings.Cut(s, "e")
	exp, _ := strconv.Atoi(expPart)
	neg := strings.HasPrefix(mant, "-")
	mant = strings.TrimPrefix(mant, "-")
	digits := strings.Replace(mant, ".", "", 1)
	sign := ""
	if neg {
		sign = "-"
	}
	if exp < -4 || exp >= 16 {
		m := digits[:1]
		if len(digits) > 1 {
			m += "." + digits[1:]
		}
		es := "+"
		if exp < 0 {
			es = "-"
			exp = -exp
		}
		return fmt.Sprintf("%s%se%s%02d", sign, m, es, exp)
	}
	if exp < 0 {
		return sign + "0." + strings.Repeat("0", -exp-1) + digits
	}
	if len(digits) <= exp+1 {
		return sign + digits + strings.Repeat("0", exp+1-len(digits)) + ".0"
	}
	return sign + digits[:exp+1] + "." + digits[exp+1:]
}
