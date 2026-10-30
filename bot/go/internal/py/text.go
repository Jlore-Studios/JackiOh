package py

// Python's text rules where Go's differ: what str.isspace(), str.strip(), str.split() and
// str.lower() take, and what `\s`, `\w`, `\d`, `\b` and re.IGNORECASE match in a pattern over
// str. RE2's classes are ASCII and it has no lookbehind or Unicode `\b`, so a pattern that needs
// Python's builds its classes from the constants here, and a match RE2 cannot express is written
// in code with the rune functions.

import (
	"strings"
	"unicode"
)

// SpaceClass, WordClass and DigitClass are the bodies of RE2 character classes that match what
// Python's `\s`, `\w` and `\d` match in a str pattern, for a class that adds to them
// (`[` + py.SpaceClass + `>*_]`). WordClass names CJK Extension I, which Python 3.13 (Unicode
// 15.1) counts as letters and Go's tables (Unicode 15.0) do not know yet.
const (
	SpaceClass = `\t\n\v\f\r \x1c-\x1f\x85\xa0\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}`
	WordClass  = `\p{L}\p{N}_\x{2ebf0}-\x{2ee5d}`
	DigitClass = `\p{Nd}`
)

// ReS, ReNotS, ReW and ReNotW are Python's `\s`, `\S`, `\w` and `\W` in a str pattern, as RE2
// classes to write in their place (DigitClass, `\p{Nd}`, stands for `\d` as it is).
const (
	ReS    = `[` + SpaceClass + `]`
	ReNotS = `[^` + SpaceClass + `]`
	ReW    = `[` + WordClass + `]`
	ReNotW = `[^` + WordClass + `]`
)

// IsSpace is str.isspace() for one character, which `\s`, str.strip() and str.split() go by:
// Go's unicode.IsSpace and the separators U+001C to U+001F.
func IsSpace(r rune) bool { return unicode.IsSpace(r) || (r >= 0x1c && r <= 0x1f) }

// IsWord is what `\w` matches: str.isalnum() or "_" (WordClass).
func IsWord(r rune) bool {
	return r == '_' || unicode.IsLetter(r) || unicode.IsNumber(r) || (r >= 0x2ebf0 && r <= 0x2ee5d)
}

// IsDigit is what `\d` matches, and str.isdecimal() for one character: a decimal digit of any
// script (not str.isdigit(), which also takes the superscripts int() refuses).
func IsDigit(r rune) bool { return unicode.IsDigit(r) }

// IsDecimal is str.isdecimal(): not empty, and every character a decimal digit, so int() reads
// it. The harness's `s.isdigit()` guards an `int(s)`, and is this: the superscript digits
// isdigit() also takes would make that int() raise.
func IsDecimal(s string) bool {
	if s == "" {
		return false
	}
	for _, r := range s {
		if !IsDigit(r) {
			return false
		}
	}
	return true
}

// Strip is str.strip().
func Strip(s string) string { return strings.TrimFunc(s, IsSpace) }

// Lstrip is str.lstrip().
func Lstrip(s string) string { return strings.TrimLeftFunc(s, IsSpace) }

// Rstrip is str.rstrip().
func Rstrip(s string) string { return strings.TrimRightFunc(s, IsSpace) }

// Split is str.split() with no separator: the runs of non-whitespace.
func Split(s string) []string { return strings.FieldsFunc(s, IsSpace) }

// Lower is str.lower(): each character's lowercase, but İ (U+0130) becomes "i̇" and a capital
// sigma at the end of a word becomes ς, as Python's full mapping has it.
func Lower(s string) string {
	rs := []rune(s)
	var b strings.Builder
	for i := range rs {
		writeLower(&b, rs, i)
	}
	return b.String()
}

// writeLower writes the lowercase of rs[i], the rest of rs its context.
func writeLower(b *strings.Builder, rs []rune, i int) {
	switch rs[i] {
	case 0x130:
		b.WriteString("i̇")
	case 0x3a3:
		if finalSigma(rs, i) {
			b.WriteRune(0x3c2)
		} else {
			b.WriteRune(0x3c3)
		}
	default:
		b.WriteRune(unicode.ToLower(rs[i]))
	}
}

// Capitalize is str.capitalize(): the first character in titlecase (Python's full mapping,
// which writes some as two or three), the rest lowercased.
func Capitalize(s string) string {
	rs := []rune(s)
	if len(rs) == 0 {
		return ""
	}
	var b strings.Builder
	if title, ok := titleFull[rs[0]]; ok {
		b.WriteString(title)
	} else {
		b.WriteRune(unicode.ToTitle(rs[0]))
	}
	for i := 1; i < len(rs); i++ {
		writeLower(&b, rs, i)
	}
	return b.String()
}

// titleFull is the titlecase of the characters whose full mapping (SpecialCasing.txt) is more
// than one character; every other character's is unicode.ToTitle's.
var titleFull = map[rune]string{0x00df: "Ss", 0x0149: "ʼN",
	0x01f0: "J̌", 0x0390: "Ϊ́", 0x03b0: "Ϋ́",
	0x0587: "Եւ", 0x1e96: "H̱", 0x1e97: "T̈", 0x1e98: "W̊",
	0x1e99: "Y̊", 0x1e9a: "Aʾ", 0x1f50: "Υ̓",
	0x1f52: "Υ̓̀", 0x1f54: "Υ̓́", 0x1f56: "Υ̓͂",
	0x1fb2: "Ὰͅ", 0x1fb4: "Άͅ", 0x1fb6: "Α͂",
	0x1fb7: "ᾼ͂", 0x1fc2: "Ὴͅ", 0x1fc4: "Ήͅ",
	0x1fc6: "Η͂", 0x1fc7: "ῌ͂", 0x1fd2: "Ϊ̀",
	0x1fd3: "Ϊ́", 0x1fd6: "Ι͂", 0x1fd7: "Ϊ͂",
	0x1fe2: "Ϋ̀", 0x1fe3: "Ϋ́", 0x1fe4: "Ρ̓",
	0x1fe6: "Υ͂", 0x1fe7: "Ϋ͂", 0x1ff2: "Ὼͅ",
	0x1ff4: "Ώͅ", 0x1ff6: "Ω͂", 0x1ff7: "ῼ͂",
	0xfb00: "Ff", 0xfb01: "Fi", 0xfb02: "Fl",
	0xfb03: "Ffi", 0xfb04: "Ffl", 0xfb05: "St",
	0xfb06: "St", 0xfb13: "Մն", 0xfb14: "Մե", 0xfb15: "Մի",
	0xfb16: "Վն", 0xfb17: "Մխ"}

// finalSigma is CPython's Final_Sigma test for the capital sigma at rs[i]: a cased letter before
// it and none after it, case-ignorable characters skipped.
func finalSigma(rs []rune, i int) bool {
	j := i - 1
	for j >= 0 && caseIgnorable(rs[j]) {
		j--
	}
	if j < 0 || !cased(rs[j]) {
		return false
	}
	j = i + 1
	for j < len(rs) && caseIgnorable(rs[j]) {
		j++
	}
	return j == len(rs) || !cased(rs[j])
}

// cased is Unicode's Cased property.
func cased(r rune) bool {
	return unicode.In(r, unicode.Lu, unicode.Ll, unicode.Lt, unicode.Other_Lowercase,
		unicode.Other_Uppercase)
}

// midLetters are the characters whose Word_Break is MidLetter, MidNumLet or Single_Quote, which
// Unicode's Case_Ignorable takes in beside its categories.
var midLetters = map[rune]bool{0x27: true, 0x2e: true, 0x3a: true, 0xb7: true, 0x387: true,
	0x55f: true, 0x5f4: true, 0x2018: true, 0x2019: true, 0x2024: true, 0x2027: true, 0xfe13: true,
	0xfe52: true, 0xfe55: true, 0xff07: true, 0xff0e: true, 0xff1a: true}

// caseIgnorable is Unicode's Case_Ignorable property.
func caseIgnorable(r rune) bool {
	return unicode.In(r, unicode.Mn, unicode.Me, unicode.Cf, unicode.Lm, unicode.Sk) || midLetters[r]
}

// extraCases is re._casefix._EXTRA_CASES: the lowercase characters re.IGNORECASE also takes for
// each, beyond those with the same lowercase (`i` matches `ı`, `s` matches `ſ`).
var extraCases = map[rune][]rune{0x0069: {0x0131}, 0x0073: {0x017f}, 0x00b5: {0x03bc},
	0x0131: {0x0069}, 0x017f: {0x0073}, 0x0345: {0x03b9, 0x1fbe}, 0x0390: {0x1fd3},
	0x03b0: {0x1fe3}, 0x03b2: {0x03d0}, 0x03b5: {0x03f5}, 0x03b8: {0x03d1}, 0x03b9: {0x0345, 0x1fbe},
	0x03ba: {0x03f0}, 0x03bc: {0x00b5}, 0x03c0: {0x03d6}, 0x03c1: {0x03f1}, 0x03c2: {0x03c3},
	0x03c3: {0x03c2}, 0x03c6: {0x03d5}, 0x03d0: {0x03b2}, 0x03d1: {0x03b8}, 0x03d5: {0x03c6},
	0x03d6: {0x03c0}, 0x03f0: {0x03ba}, 0x03f1: {0x03c1}, 0x03f5: {0x03b5}, 0x0432: {0x1c80},
	0x0434: {0x1c81}, 0x043e: {0x1c82}, 0x0441: {0x1c83}, 0x0442: {0x1c84, 0x1c85},
	0x044a: {0x1c86}, 0x0463: {0x1c87}, 0x1c80: {0x0432}, 0x1c81: {0x0434}, 0x1c82: {0x043e},
	0x1c83: {0x0441}, 0x1c84: {0x0442, 0x1c85}, 0x1c85: {0x0442, 0x1c84}, 0x1c86: {0x044a},
	0x1c87: {0x0463}, 0x1c88: {0xa64b}, 0x1e61: {0x1e9b}, 0x1e9b: {0x1e61}, 0x1fbe: {0x0345, 0x03b9},
	0x1fd3: {0x0390}, 0x1fe3: {0x03b0}, 0xa64b: {0x1c88}, 0xfb05: {0xfb06}, 0xfb06: {0xfb05}}

// FoldEq is whether text character x matches pattern character c under re.IGNORECASE: c exactly
// when it has no case, else x's lowercase is c's or one of c's extra cases.
func FoldEq(x, c rune) bool {
	lc := unicode.ToLower(c)
	if lc == c && unicode.ToUpper(c) == c {
		return x == c
	}
	lx := unicode.ToLower(x)
	if lx == lc {
		return true
	}
	for _, e := range extraCases[lc] {
		if lx == e {
			return true
		}
	}
	return false
}

// FoldAt matches the literal pat at rs[i:] under re.IGNORECASE, and returns where it ends.
func FoldAt(rs []rune, i int, pat string) (int, bool) {
	for _, c := range pat {
		if i >= len(rs) || !FoldEq(rs[i], c) {
			return 0, false
		}
		i++
	}
	return i, true
}

// FoldEqual is whether all of text matches the literal pat under re.IGNORECASE.
func FoldEqual(text, pat string) bool {
	rs := []rune(text)
	end, ok := FoldAt(rs, 0, pat)
	return ok && end == len(rs)
}

// Atoi is int() of what `\d+` matched: decimal digits of any script, 0 for none.
func Atoi(digits string) int {
	n := 0
	for _, d := range digits {
		n = n*10 + DigitValue(d)
	}
	return n
}

// DigitValue is a decimal digit's value: Unicode puts each script's digits 0 to 9 in a row, so
// it is the digit's place in its run of digits.
func DigitValue(d rune) int {
	if d >= '0' && d <= '9' {
		return int(d - '0')
	}
	start := d
	for unicode.IsDigit(start - 1) {
		start--
	}
	return int(d-start) % 10
}

// Boundary is `\b` at rs[i]: a word character on one side and none on the other.
func Boundary(rs []rune, i int) bool {
	if len(rs) == 0 {
		return false
	}
	before := i > 0 && IsWord(rs[i-1])
	after := i < len(rs) && IsWord(rs[i])
	return before != after
}
