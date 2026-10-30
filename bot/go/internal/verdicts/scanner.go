package verdicts

import (
	"math"
	"strconv"
	"strings"
	"unicode/utf8"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// rawDecode is `json.JSONDecoder(strict=False).raw_decode(s)`: the JSON value at the very start
// of `s` (no whitespace skipped) and the byte index just past it, or false where Python raises
// JSONDecodeError. It follows the C scanner (Modules/_json.c) step for step: control characters
// are allowed inside strings, NaN and Infinity are read, a duplicate key keeps its first place and
// its last value, and an integer too large for a Go int is a float (as pyjson.Loads has it). A
// lone surrogate escape, which a Python str holds, becomes U+FFFD.
func rawDecode(s string) (any, int, bool) {
	return (&scanner{s}).value(0)
}

type scanner struct{ s string }

func isJSONSpace(c byte) bool { return c == ' ' || c == '\t' || c == '\n' || c == '\r' }

func isDigit(c byte) bool { return c >= '0' && c <= '9' }

func (d *scanner) skip(i int) int {
	for i < len(d.s) && isJSONSpace(d.s[i]) {
		i++
	}
	return i
}

func (d *scanner) value(i int) (any, int, bool) {
	s := d.s
	if i >= len(s) {
		return nil, 0, false
	}
	switch s[i] {
	case '"':
		return d.str(i + 1)
	case '{':
		return d.object(i + 1)
	case '[':
		return d.array(i + 1)
	case 'n':
		if strings.HasPrefix(s[i:], "null") {
			return nil, i + 4, true
		}
	case 't':
		if strings.HasPrefix(s[i:], "true") {
			return true, i + 4, true
		}
	case 'f':
		if strings.HasPrefix(s[i:], "false") {
			return false, i + 5, true
		}
	case 'N':
		if strings.HasPrefix(s[i:], "NaN") {
			return math.NaN(), i + 3, true
		}
	case 'I':
		if strings.HasPrefix(s[i:], "Infinity") {
			return math.Inf(1), i + 8, true
		}
	case '-':
		if strings.HasPrefix(s[i:], "-Infinity") {
			return math.Inf(-1), i + 9, true
		}
	}
	return d.number(i)
}

// number is _match_number_unicode.
func (d *scanner) number(i int) (any, int, bool) {
	s := d.s
	start := i
	if s[i] == '-' {
		i++
		if i >= len(s) {
			return nil, 0, false
		}
	}
	switch {
	case s[i] >= '1' && s[i] <= '9':
		i++
		for i < len(s) && isDigit(s[i]) {
			i++
		}
	case s[i] == '0':
		i++
	default:
		return nil, 0, false
	}
	isFloat := false
	if i < len(s)-1 && s[i] == '.' && isDigit(s[i+1]) {
		isFloat = true
		i += 2
		for i < len(s) && isDigit(s[i]) {
			i++
		}
	}
	if i < len(s)-1 && (s[i] == 'e' || s[i] == 'E') {
		eStart := i
		i++
		if i < len(s)-1 && (s[i] == '-' || s[i] == '+') {
			i++
		}
		for i < len(s) && isDigit(s[i]) {
			i++
		}
		if isDigit(s[i-1]) {
			isFloat = true
		} else {
			i = eStart
		}
	}
	text := s[start:i]
	if !isFloat {
		if n, err := strconv.ParseInt(text, 10, 64); err == nil {
			return int(n), i, true
		}
	}
	f, _ := strconv.ParseFloat(text, 64) // ±Inf past the largest float, as float() gives
	return f, i, true
}

func hex4(s string) (rune, bool) {
	if len(s) != 4 {
		return 0, false
	}
	var r rune
	for i := 0; i < 4; i++ {
		c := s[i]
		r <<= 4
		switch {
		case c >= '0' && c <= '9':
			r |= rune(c - '0')
		case c >= 'a' && c <= 'f':
			r |= rune(c-'a') + 10
		case c >= 'A' && c <= 'F':
			r |= rune(c-'A') + 10
		default:
			return 0, false
		}
	}
	return r, true
}

// str is scanstring_unicode with strict=False; `i` is just past the opening quote.
func (d *scanner) str(i int) (string, int, bool) {
	s := d.s
	var b strings.Builder
	for {
		j := i
		for j < len(s) && s[j] != '"' && s[j] != '\\' {
			j++
		}
		if j >= len(s) {
			return "", 0, false // unterminated string
		}
		b.WriteString(s[i:j])
		if s[j] == '"' {
			return b.String(), j + 1, true
		}
		j++ // past the backslash
		if j >= len(s) {
			return "", 0, false
		}
		if c := s[j]; c != 'u' {
			switch c {
			case '"', '\\', '/':
				b.WriteByte(c)
			case 'b':
				b.WriteByte('\b')
			case 'f':
				b.WriteByte('\f')
			case 'n':
				b.WriteByte('\n')
			case 'r':
				b.WriteByte('\r')
			case 't':
				b.WriteByte('\t')
			default:
				return "", 0, false // invalid \escape
			}
			i = j + 1
			continue
		}
		j++
		end := j + 4
		if end >= len(s) {
			return "", 0, false // invalid \uXXXX escape
		}
		c, ok := hex4(s[j:end])
		if !ok {
			return "", 0, false
		}
		j = end
		if c >= 0xd800 && c <= 0xdbff && end+6 < len(s) && s[j] == '\\' && s[j+1] == 'u' {
			c2, ok := hex4(s[j+2 : j+6])
			if !ok {
				return "", 0, false
			}
			if c2 >= 0xdc00 && c2 <= 0xdfff {
				c = 0x10000 + (c-0xd800)<<10 + (c2 - 0xdc00)
				j += 6
			}
		}
		if c >= 0xd800 && c <= 0xdfff {
			c = utf8.RuneError
		}
		b.WriteRune(c)
		i = j
	}
}

// object is _parse_object_unicode; `i` is just past the `{`.
func (d *scanner) object(i int) (any, int, bool) {
	s := d.s
	o := pyjson.New()
	i = d.skip(i)
	if i < len(s) && s[i] == '}' {
		return o, i + 1, true
	}
	for {
		if i >= len(s) || s[i] != '"' {
			return nil, 0, false // expecting property name enclosed in double quotes
		}
		key, next, ok := d.str(i + 1)
		if !ok {
			return nil, 0, false
		}
		i = d.skip(next)
		if i >= len(s) || s[i] != ':' {
			return nil, 0, false
		}
		i = d.skip(i + 1)
		val, next, ok := d.value(i)
		if !ok {
			return nil, 0, false
		}
		o.Set(key, val)
		i = d.skip(next)
		if i < len(s) && s[i] == '}' {
			return o, i + 1, true
		}
		if i >= len(s) || s[i] != ',' {
			return nil, 0, false
		}
		i = d.skip(i + 1)
		if i < len(s) && s[i] == '}' {
			return nil, 0, false // illegal trailing comma
		}
	}
}

// array is _parse_array_unicode; `i` is just past the `[`.
func (d *scanner) array(i int) (any, int, bool) {
	s := d.s
	list := []any{}
	i = d.skip(i)
	if i < len(s) && s[i] == ']' {
		return list, i + 1, true
	}
	for {
		val, next, ok := d.value(i)
		if !ok {
			return nil, 0, false
		}
		list = append(list, val)
		i = d.skip(next)
		if i < len(s) && s[i] == ']' {
			return list, i + 1, true
		}
		if i >= len(s) || s[i] != ',' {
			return nil, 0, false
		}
		i = d.skip(i + 1)
		if i < len(s) && s[i] == ']' {
			return nil, 0, false // illegal trailing comma
		}
	}
}
