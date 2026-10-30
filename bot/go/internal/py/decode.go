package py

import (
	"os"
	"strings"
	"unicode/utf8"
)

// DecodeReplace is bytes.decode("utf-8", "replace"): each maximal invalid subpart (a stray byte,
// or the valid start of a sequence cut short) is one U+FFFD, as CPython's decoder writes it. Go's
// own conversions write one per byte.
func DecodeReplace(b []byte) string {
	if utf8.Valid(b) {
		return string(b)
	}
	var out strings.Builder
	for i := 0; i < len(b); {
		r, size := utf8.DecodeRune(b[i:])
		if r != utf8.RuneError || size > 1 {
			out.Write(b[i : i+size])
			i += size
			continue
		}
		out.WriteRune(utf8.RuneError)
		i += invalidLen(b[i:])
	}
	return out.String()
}

// invalidLen is the length of the maximal subpart of an ill-formed UTF-8 sequence at b[0]: its
// lead byte and the continuation bytes that were valid for it.
func invalidLen(b []byte) int {
	need, lo, hi := 0, byte(0x80), byte(0xbf)
	switch c := b[0]; {
	case c >= 0xc2 && c <= 0xdf:
		need = 1
	case c == 0xe0:
		need, lo = 2, 0xa0
	case c >= 0xe1 && c <= 0xec, c == 0xee, c == 0xef:
		need = 2
	case c == 0xed:
		need, hi = 2, 0x9f
	case c == 0xf0:
		need, lo = 3, 0x90
	case c >= 0xf1 && c <= 0xf3:
		need = 3
	case c == 0xf4:
		need, hi = 3, 0x8f
	default:
		return 1
	}
	n := 1
	for k := 0; k < need && n < len(b); k++ {
		if k > 0 {
			lo, hi = 0x80, 0xbf
		}
		if b[n] < lo || b[n] > hi {
			break
		}
		n++
	}
	return n
}

// Universal is text as Python's text mode reads it (universal newlines): "\r\n" and a lone "\r"
// become "\n".
func Universal(s string) string {
	if !strings.Contains(s, "\r") {
		return s
	}
	return strings.ReplaceAll(strings.ReplaceAll(s, "\r\n", "\n"), "\r", "\n")
}

// ReadText is Path(path).read_text(encoding="utf-8", errors="replace"): the file decoded with
// DecodeReplace, in text mode. The error is the OSError Python raised.
func ReadText(path string) (string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return "", err
	}
	return Universal(DecodeReplace(data)), nil
}
