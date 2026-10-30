package py

import "unicode/utf8"

// Len is len(s): the number of code points, not bytes.
func Len(s string) int { return utf8.RuneCountInString(s) }

// Slice is s[start:end] over code points, with Python's negative indices and clamping. Use
// SliceFrom and SliceTo for the open-ended forms.
func Slice(s string, start, end int) string {
	rs := []rune(s)
	n := len(rs)
	start, end = clampIndex(start, n), clampIndex(end, n)
	if start >= end {
		return ""
	}
	return string(rs[start:end])
}

// SliceFrom is s[start:].
func SliceFrom(s string, start int) string { return Slice(s, start, Len(s)) }

// SliceTo is s[:end].
func SliceTo(s string, end int) string { return Slice(s, 0, end) }

func clampIndex(i, n int) int {
	if i < 0 {
		i += n
		if i < 0 {
			i = 0
		}
	}
	if i > n {
		i = n
	}
	return i
}

// Ljust is s.ljust(width) (f"{s:<width}"): padded with spaces to width code points.
func Ljust(s string, width int) string {
	for n := Len(s); n < width; n++ {
		s += " "
	}
	return s
}

// Rjust is s.rjust(width) (f"{s:>width}").
func Rjust(s string, width int) string {
	pad := ""
	for n := Len(s); n < width; n++ {
		pad += " "
	}
	return pad + s
}
