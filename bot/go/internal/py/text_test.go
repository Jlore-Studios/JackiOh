package py

import (
	"os"
	"regexp"
	"slices"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// cases is testdata.json, what Python 3.13 answered for each case.
func cases(t *testing.T) *pyjson.Object {
	t.Helper()
	raw, err := os.ReadFile("testdata.json")
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(raw)
	if err != nil {
		t.Fatal(err)
	}
	return pyjson.Obj(v)
}

func TestTextMatchesPython(t *testing.T) {
	c := cases(t)
	for _, e := range c.List("strip") {
		p := pyjson.List(e)
		s := p[0].(string)
		if Strip(s) != p[1] || Lstrip(s) != p[2] || Rstrip(s) != p[3] {
			t.Errorf("strip(%q) = %q %q %q, want %q %q %q", s, Strip(s), Lstrip(s), Rstrip(s), p[1], p[2], p[3])
		}
		if got, want := Split(s), pyjson.Strings(p[4]); !slices.Equal(got, want) {
			t.Errorf("split(%q) = %q, want %q", s, got, want)
		}
	}
	for _, e := range c.List("lower") {
		p := pyjson.List(e)
		if got := Lower(p[0].(string)); got != p[1] {
			t.Errorf("lower(%q) = %q, want %q", p[0], got, p[1])
		}
	}
	for _, e := range c.List("capitalize") {
		p := pyjson.List(e)
		if got := Capitalize(p[0].(string)); got != p[1] {
			t.Errorf("capitalize(%q) = %q, want %q", p[0], got, p[1])
		}
	}
	space, word, notWord := regexp.MustCompile(`^`+ReS+`$`), regexp.MustCompile(`^`+ReW+`$`),
		regexp.MustCompile(`^`+ReNotW+`$`)
	digit := regexp.MustCompile(`^[` + DigitClass + `]$`)
	for _, e := range c.List("chars") {
		p := pyjson.List(e)
		s := p[0].(string)
		r := []rune(s)[0]
		isSpace, isWord, isDigit := p[1].(bool), p[2].(bool), p[3].(bool)
		if IsSpace(r) != isSpace || space.MatchString(s) != isSpace {
			t.Errorf("%U: isspace() = %v", r, isSpace)
		}
		if IsWord(r) != isWord || word.MatchString(s) != isWord || notWord.MatchString(s) == isWord {
			t.Errorf("%U: \\w = %v", r, isWord)
		}
		if IsDigit(r) != isDigit || digit.MatchString(s) != isDigit || IsDecimal(s) != p[4].(bool) {
			t.Errorf("%U: \\d = %v, isdecimal() = %v", r, isDigit, p[4])
		}
	}
	for _, e := range c.List("atoi") {
		p := pyjson.List(e)
		if got := Atoi(p[0].(string)); got != p[1].(int) {
			t.Errorf("int(%q) = %d, want %v", p[0], got, p[1])
		}
	}
	for _, e := range c.List("nd") {
		p := pyjson.List(e)
		r := rune(p[0].(int))
		if !IsDigit(r) {
			continue // a digit newer than Go's Unicode tables
		}
		if got := DigitValue(r); got != p[1].(int) {
			t.Errorf("unicodedata.decimal(%U) = %d, want %v", r, got, p[1])
		}
	}
	for _, e := range c.List("decimal") {
		p := pyjson.List(e)
		if got := IsDecimal(p[0].(string)); got != p[1].(bool) {
			t.Errorf("%q.isdecimal() = %v, want %v", p[0], got, p[1])
		}
	}
	for _, e := range c.List("fold") {
		p := pyjson.List(e)
		if got := FoldEqual(p[0].(string), p[1].(string)); got != p[2].(bool) {
			t.Errorf("fullmatch(%q, %q, re.I) = %v, want %v", p[1], p[0], got, p[2])
		}
	}
	for _, e := range c.List("boundary") {
		p := pyjson.List(e)
		rs := []rune(p[0].(string))
		for i, want := range pyjson.List(p[1]) {
			if got := Boundary(rs, i); got != want.(bool) {
				t.Errorf("\\b in %q at %d = %v, want %v", p[0], i, got, want)
			}
		}
	}
}

func TestDecodeMatchesPython(t *testing.T) {
	c := cases(t)
	for _, e := range c.List("decode") {
		p := pyjson.List(e)
		var raw []byte
		for _, b := range pyjson.List(p[0]) {
			raw = append(raw, byte(b.(int)))
		}
		if got := DecodeReplace(raw); got != p[1] {
			t.Errorf("%q.decode(\"utf-8\", \"replace\") = %q, want %q", raw, got, p[1])
		}
	}
	for _, e := range c.List("universal") {
		p := pyjson.List(e)
		if got := Universal(p[0].(string)); got != p[1] {
			t.Errorf("text mode of %q = %q, want %q", p[0], got, p[1])
		}
	}
	dir := t.TempDir()
	path := dir + "/f.txt"
	if err := os.WriteFile(path, []byte("a\r\nb\xe2\x82\rc"), 0o644); err != nil {
		t.Fatal(err)
	}
	if got, err := ReadText(path); err != nil || got != "a\nb\ufffd\nc" {
		t.Errorf("ReadText = %q, %v", got, err)
	}
	if _, err := ReadText(dir + "/none"); err == nil {
		t.Error("ReadText of a missing file is an OSError")
	}
}
