package work

// The few Python behaviours work.py leans on that the foundation packages do not hold yet:
// string.Template (the prompts and the easy rule), str.format (the module's own `{name}`
// templates), the CompletedProcess git.Run returns, pathlib's file reads and writes, and the
// `type(exc).__name__` and `str(exc)` a failure's reason is made of. Private to this package;
// each names the Python it stands for.

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"os/exec"
	"reflect"
	"regexp"
	"strconv"
	"strings"
	"syscall"
	"unicode"
	"unicode/utf8"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// ------------------------------------------------------------------ Python's own exceptions

// keyError is Python's KeyError: str() of it is the key's repr.
type keyError struct{ key string }

func (e *keyError) Error() string { return py.Repr(e.key) }

// valueError is Python's ValueError.
type valueError struct{ msg string }

func (e *valueError) Error() string { return e.msg }

// typeError is Python's TypeError.
type typeError struct{ msg string }

func (e *typeError) Error() string { return e.msg }

// runtimeError is Python's RuntimeError, which work.py raises for a run that cannot go on.
type runtimeError struct{ msg string }

func (e *runtimeError) Error() string { return e.msg }

// excName is type(exc).__name__ for what a run's `except Exception` caught.
func excName(err error) string {
	switch e := err.(type) {
	case *Interrupt:
		return "Interrupt"
	case *keyError:
		return "KeyError"
	case *valueError:
		return "ValueError"
	case *typeError:
		return "TypeError"
	case *runtimeError:
		return "RuntimeError"
	case *herr.HarnessError:
		return "HarnessError"
	case *herr.ConfigError:
		return "ConfigError"
	case *herr.GitHubError:
		return "GitHubError"
	case *herr.GitError:
		return "GitError"
	case *herr.StateConflict:
		return "StateConflict"
	case *herr.StateUnreadable:
		return "StateUnreadable"
	case *herr.LoginError:
		return "LoginError"
	case *fs.PathError:
		return osErrorName(e.Err)
	case *os.LinkError:
		return osErrorName(e.Err)
	case *os.SyscallError:
		return osErrorName(e.Err)
	case syscall.Errno:
		return osErrorName(e)
	case *exec.Error:
		if errors.Is(e.Err, exec.ErrNotFound) {
			return "FileNotFoundError"
		}
		return "OSError"
	}
	t := reflect.TypeOf(err)
	for t != nil && t.Kind() == reflect.Pointer {
		t = t.Elem()
	}
	if t == nil || t.Name() == "" {
		return "Exception"
	}
	return t.Name()
}

// osErrorName is the OSError subclass Python raises for an errno.
func osErrorName(err error) string {
	var errno syscall.Errno
	if !errors.As(err, &errno) {
		return "OSError"
	}
	switch errno {
	case syscall.ENOENT:
		return "FileNotFoundError"
	case syscall.EEXIST:
		return "FileExistsError"
	case syscall.EACCES, syscall.EPERM:
		return "PermissionError"
	case syscall.EISDIR:
		return "IsADirectoryError"
	case syscall.ENOTDIR:
		return "NotADirectoryError"
	case syscall.EINTR:
		return "InterruptedError"
	case syscall.ETIMEDOUT:
		return "TimeoutError"
	}
	return "OSError"
}

// excStr is str(exc): an OSError as Python words it (`[Errno 2] No such file or directory:
// 'path'`), anything else its message.
func excStr(err error) string {
	switch e := err.(type) {
	case *fs.PathError:
		if n, s, ok := errnoText(e.Err); ok {
			return fmt.Sprintf("[Errno %d] %s: %s", n, s, py.Repr(e.Path))
		}
	case *os.LinkError:
		if n, s, ok := errnoText(e.Err); ok {
			return fmt.Sprintf("[Errno %d] %s: %s -> %s", n, s, py.Repr(e.Old), py.Repr(e.New))
		}
	case *os.SyscallError:
		if n, s, ok := errnoText(e.Err); ok {
			return fmt.Sprintf("[Errno %d] %s", n, s)
		}
	case syscall.Errno:
		if n, s, ok := errnoText(e); ok {
			return fmt.Sprintf("[Errno %d] %s", n, s)
		}
	}
	return err.Error()
}

// errnoText is an errno and its strerror (C's, capitalised as glibc writes it).
func errnoText(err error) (int, string, bool) {
	var errno syscall.Errno
	if !errors.As(err, &errno) {
		return 0, "", false
	}
	s := errno.Error()
	if r, size := utf8.DecodeRuneInString(s); size > 0 {
		s = string(unicode.ToUpper(r)) + s[size:]
	}
	return int(errno), s, true
}

// isOSError is `except OSError`.
func isOSError(err error) bool {
	var pe *fs.PathError
	var le *os.LinkError
	var se *os.SyscallError
	var errno syscall.Errno
	var ee *exec.Error
	return errors.As(err, &pe) || errors.As(err, &le) || errors.As(err, &se) ||
		errors.As(err, &errno) || errors.As(err, &ee)
}

// ------------------------------------------------------------------ values

// strOr is `str(v or def)`: v's str() when it is truthy, else def.
func strOr(v any, def string) string {
	if pyjson.Truthy(v) {
		return py.Str(v)
	}
	return def
}

// pyInt is int(v): an int as is, a float truncated, a string parsed; anything else raises.
func pyInt(v any) int {
	switch x := v.(type) {
	case bool:
		if x {
			return 1
		}
		return 0
	case string:
		n, err := strconv.Atoi(strings.ReplaceAll(strings.TrimSpace(x), "_", ""))
		if err != nil {
			herr.Raise(&valueError{msg: "invalid literal for int() with base 10: " + py.Repr(x)})
		}
		return n
	case nil:
		herr.Raise(&typeError{msg: "int() argument must be a string, a bytes-like object or a " +
			"real number, not 'NoneType'"})
	}
	if n, ok := pyjson.AsInt(v); ok {
		return n
	}
	herr.Raise(&typeError{msg: fmt.Sprintf("int() argument must be a string, a bytes-like object "+
		"or a real number, not '%T'", v)})
	return 0
}

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

// rstrip is str.rstrip().
func rstrip(s string) string { return strings.TrimRightFunc(s, unicode.IsSpace) }

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
			herr.Raise(&keyError{key: name})
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
				return "", &keyError{key: name}
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
	return &valueError{msg: fmt.Sprintf("Invalid placeholder in string: line %d, col %d", lineno,
		len(prefix)-start)}
}

// templateText is the text of what prompts.load returned: a string.Template, whose Go form
// holds its source as a string, a `Template` or `Text` field, or a String method.
func templateText(t any) string {
	if s, ok := t.(string); ok {
		return s
	}
	v := reflect.ValueOf(t)
	for v.IsValid() && (v.Kind() == reflect.Pointer || v.Kind() == reflect.Interface) {
		if v.IsNil() {
			break
		}
		v = v.Elem()
	}
	if v.IsValid() && v.Kind() == reflect.Struct {
		for _, name := range []string{"Template", "Text", "Source"} {
			if f := v.FieldByName(name); f.IsValid() && f.Kind() == reflect.String {
				return f.String()
			}
		}
	}
	if s, ok := t.(fmt.Stringer); ok {
		return s.String()
	}
	if v.IsValid() && v.Kind() == reflect.String {
		return v.String()
	}
	panic(fmt.Sprintf("work: prompts.Load returned %T, which holds no template text", t))
}

// ------------------------------------------------------------------ git

// procOf reads what git.Run returned, Python's subprocess.CompletedProcess: its returncode and
// its stdout.
func procOf(proc any) (returncode int, stdout string) {
	v := reflect.ValueOf(proc)
	for v.IsValid() && (v.Kind() == reflect.Pointer || v.Kind() == reflect.Interface) {
		if v.IsNil() {
			return 0, ""
		}
		v = v.Elem()
	}
	if !v.IsValid() || v.Kind() != reflect.Struct {
		return 0, ""
	}
	for _, name := range []string{"Returncode", "ReturnCode", "Code", "ExitCode"} {
		if f := v.FieldByName(name); f.IsValid() && f.CanInt() {
			returncode = int(f.Int())
			break
		}
	}
	if f := v.FieldByName("Stdout"); f.IsValid() {
		switch {
		case f.Kind() == reflect.String:
			stdout = f.String()
		case f.Kind() == reflect.Slice && f.Type().Elem().Kind() == reflect.Uint8:
			stdout = string(f.Bytes())
		}
	}
	return returncode, stdout
}

// run is `git.run(*args, check=check)` (its default timeout, no extra environment).
func run(g *git.Git, check bool, args ...string) (int, string) {
	return procOf(g.Run(check, 600, nil, args...))
}

// cwdOf is `git.cwd`, the worktree's directory.
func cwdOf(g *git.Git) string { return fmt.Sprint(g.Cwd) }

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

// readText is Path.read_text(encoding="utf-8"), with errors="replace" when `replace` (each
// byte that is not UTF-8 read as U+FFFD). An OSError raises.
func readText(path string, replace bool) string {
	data, err := os.ReadFile(path)
	herr.Must(err)
	if !replace || utf8.Valid(data) {
		return string(data)
	}
	var b strings.Builder
	for len(data) > 0 {
		r, size := utf8.DecodeRune(data)
		if r == utf8.RuneError && size <= 1 {
			b.WriteRune(utf8.RuneError)
			size = 1
		} else {
			b.WriteRune(r)
		}
		data = data[size:]
	}
	return b.String()
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
