package work

// The few Python behaviours work.py leans on that py and herr do not hold: string.Template (the
// prompts and the easy rule), str.format (the module's own `{name}` templates), the
// CompletedProcess git.Run returns, and pathlib's file reads and writes. Each names the Python it
// stands for.

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"regexp"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// ------------------------------------------------------------------ values

// noneIfEmpty is a `str | None` field as JSON: the harness's Go types hold None as "".
func noneIfEmpty(s string) any {
	if s == "" {
		return nil
	}
	return s
}

// intOrNone is an `int | None` as JSON.
func intOrNone(p *int) any {
	if p == nil {
		return nil
	}
	return *p
}

// objOrNone is a `dict | None` as JSON: a nil object is null.
func objOrNone(o *pyjson.Object) any {
	if o == nil {
		return nil
	}
	return o
}

// appendTo is `d[key].append(v)` for a list kept in a JSON object (a Go slice is a value, so
// the grown list goes back under its key, in its place).
func appendTo(o *pyjson.Object, key string, v any) {
	o.Set(key, append(o.List(key), v))
}

// first12 is `s[:12]`.
func first12(s string) string { return py.SliceTo(s, 12) }

// ------------------------------------------------------------------ str.format

var formatField = regexp.MustCompile(`\{\{|\}\}|\{([A-Za-z_][A-Za-z0-9_]*)\}`)

// pyFormat is str.format(**values) for the module's templates: `{name}` is str(values[name]),
// `{{` and `}}` are braces. A missing name raises KeyError.
func pyFormat(s string, values map[string]any) string {
	return formatField.ReplaceAllStringFunc(s, func(m string) string {
		switch m {
		case "{{":
			return "{"
		case "}}":
			return "}"
		}
		name := m[1 : len(m)-1]
		v, ok := values[name]
		if !ok {
			herr.Raise(py.KeyError(name))
		}
		return py.Str(v)
	})
}

// ------------------------------------------------------------------ string.Template

// templatePattern is string.Template.pattern: `$$`, `$name`, `${name}`, or an invalid `$`.
// Its identifier is `(?a:[_a-z][_a-z0-9]*)` under re.IGNORECASE: ASCII letters of either case.
var templatePattern = regexp.MustCompile(
	`\$(?:(\$)|([_a-zA-Z][_a-zA-Z0-9]*)|\{([_a-zA-Z][_a-zA-Z0-9]*)\}|())`)

// pyTemplate is string.Template over its text.
type pyTemplate string

// substitute is Template.substitute(values): a missing name is a KeyError, a `$` that starts no
// placeholder a ValueError, as Python raises them (returned, for the caller to raise or keep).
func (t pyTemplate) substitute(values map[string]string) (string, error) {
	src := string(t)
	var b strings.Builder
	last := 0
	for _, m := range templatePattern.FindAllStringSubmatchIndex(src, -1) {
		b.WriteString(src[last:m[0]])
		last = m[1]
		switch {
		case m[2] >= 0:
			b.WriteString("$")
		case m[4] >= 0 || m[6] >= 0:
			name := ""
			if m[4] >= 0 {
				name = src[m[4]:m[5]]
			} else {
				name = src[m[6]:m[7]]
			}
			v, ok := values[name]
			if !ok {
				return "", py.KeyError(name)
			}
			b.WriteString(v)
		default:
			return "", invalidPlaceholder(src, m[8])
		}
	}
	b.WriteString(src[last:])
	return b.String(), nil
}

// safeSubstitute is Template.safe_substitute(values): a missing name or an invalid `$` stays
// as it is written.
func (t pyTemplate) safeSubstitute(values map[string]string) string {
	src := string(t)
	return templatePattern.ReplaceAllStringFunc(src, func(m string) string {
		sub := templatePattern.FindStringSubmatch(m)
		switch {
		case sub[1] != "":
			return "$"
		case sub[2] != "":
			if v, ok := values[sub[2]]; ok {
				return v
			}
		case sub[3] != "":
			if v, ok := values[sub[3]]; ok {
				return v
			}
		}
		return m
	})
}

// invalidPlaceholder is Template._invalid's ValueError: the line and column (in code points,
// at Python's line boundaries) of the `$` that starts no placeholder; `at` is the byte offset
// just after it, so the text before it ends with that `$` and its last line is never empty.
func invalidPlaceholder(src string, at int) error {
	prefix := []rune(src[:at])
	lineno, start := 1, 0
	for i := 0; i < len(prefix); i++ {
		switch prefix[i] {
		case '\r':
			if i+1 < len(prefix) && prefix[i+1] == '\n' {
				i++
			}
			fallthrough
		case '\n', '\v', '\f', 0x1c, 0x1d, 0x1e, 0x85, 0x2028, 0x2029:
			lineno++
			start = i + 1
		}
	}
	return &py.Error{Type: "ValueError", Msg: fmt.Sprintf("Invalid placeholder in string: line %d, col %d",
		lineno, len(prefix)-start)}
}

// ------------------------------------------------------------------ git

// run is `git.run(*args, check=check)` (its default timeout, no extra environment): its
// returncode and stdout.
func run(g *git.Git, check bool, args ...string) (int, string) {
	proc := g.Run(check, 600, nil, args...)
	return proc.Returncode, proc.Stdout
}

// ------------------------------------------------------------------ pathlib

// exists is Path.exists().
func exists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

// isFile is Path.is_file().
func isFile(path string) bool {
	info, err := os.Stat(path)
	return err == nil && info.Mode().IsRegular()
}

// readText is Path.read_text(encoding="utf-8"), with errors="replace" when `replace` (strict
// reads the bytes as they are). An OSError raises.
func readText(path string, replace bool) string {
	if replace {
		text, err := py.ReadText(path)
		herr.Must(err)
		return text
	}
	data, err := os.ReadFile(path)
	herr.Must(err)
	return py.Universal(string(data))
}

// writeText is Path.write_text(text, encoding="utf-8").
func writeText(path, text string) { herr.Must(os.WriteFile(path, []byte(text), 0o666)) }

// unlinkMissingOK is Path.unlink(missing_ok=True).
func unlinkMissingOK(path string) {
	if err := os.Remove(path); err != nil && !errors.Is(err, fs.ErrNotExist) {
		herr.Raise(err)
	}
}

// unlink is Path.unlink().
func unlink(path string) { herr.Must(os.Remove(path)) }

// mkdirAll is Path.mkdir(parents=True, exist_ok=True).
func mkdirAll(path string) { herr.Must(os.MkdirAll(path, 0o777)) }
