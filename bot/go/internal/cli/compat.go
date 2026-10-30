package cli

// Private helpers the command line needs from Python itself: `int()`, `str.isdigit()`, the
// exceptions its except clauses name (OSError, ValueError), the last line of a traceback,
// `str(timedelta)`, subprocess.run, and the running bot's constants from `config`, read through
// `any` so this package compiles whether `config` declares them as `any` or as their own types.

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io/fs"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"runtime"
	"strconv"
	"strings"
	"syscall"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/identity"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// lower is `str(b).lower()` for a bool.
func lower(b bool) string {
	if b {
		return "true"
	}
	return "false"
}

// pyBool is `str(b)`.
func pyBool(b bool) string {
	if b {
		return "True"
	}
	return "False"
}

// readJSON is `json.loads(Path(path).read_text(encoding="utf-8"))`: an OSError or a
// JSONDecodeError goes up, uncaught, as in the Python.
func readJSON(path string) any {
	data, err := os.ReadFile(path)
	herr.Must(err)
	return loads(string(data))
}

// loads is json.loads, raising JSONDecodeError (a ValueError).
func loads(text string) any {
	v, err := pyjson.LoadsString(text)
	if err != nil {
		panic(&py.Error{Type: "json.decoder.JSONDecodeError", Msg: err.Error()})
	}
	return v
}

// object is a JSON value the Python reads with `.get`: an AttributeError when it is not a dict.
func object(v any) *pyjson.Object {
	if o, ok := v.(*pyjson.Object); ok && o != nil {
		return o
	}
	panic(&py.Error{Type: "AttributeError", Msg: fmt.Sprintf("'%s' object has no attribute 'get'", py.TypeName(v))})
}

// mkdirParent is `Path(path).parent.mkdir(parents=True, exist_ok=True)`; an OSError goes up.
func mkdirParent(path string) { herr.Must(os.MkdirAll(filepath.Dir(path), 0o777)) }

// mkdirAll is `Path(dir).mkdir(parents=True, exist_ok=True)`; an OSError goes up.
func mkdirAll(dir string) { herr.Must(os.MkdirAll(dir, 0o777)) }

// writeText is `Path(path).write_text(text, encoding="utf-8")`; an OSError goes up.
func writeText(path, text string) {
	herr.Must(os.WriteFile(path, []byte(text), 0o666))
}

// ---------------------------------------------------------------------------- exceptions

// isOSError is `isinstance(exc, OSError)` for the errors Go returns where Python raised one.
func isOSError(err error) bool {
	var pathErr *fs.PathError
	var linkErr *os.LinkError
	var sysErr *os.SyscallError
	var execErr *exec.Error
	var errno syscall.Errno
	return errors.As(err, &pathErr) || errors.As(err, &linkErr) || errors.As(err, &sysErr) ||
		errors.As(err, &execErr) || errors.As(err, &errno)
}

// isValueError is `isinstance(exc, ValueError)`: one this package raised, or a Go error another
// package raised for Python's (not one of the harness's own, an OSError, or a Go runtime error,
// which stands for a bug).
func isValueError(err error) bool {
	var pe *py.Error
	if errors.As(err, &pe) {
		return pe.Type == "ValueError" || pe.Type == "json.decoder.JSONDecodeError"
	}
	var p *herr.Panic
	var re runtime.Error
	return !herr.IsHarness(err) && !isOSError(err) && !errors.As(err, &p) && !errors.As(err, &re)
}

// catchIf runs fn and returns the error it raised when `match` says the except clause takes it;
// anything else goes on up.
func catchIf(fn func(), match func(error) bool) (caught error, ok bool) {
	defer func() {
		if r := recover(); r != nil {
			if err, isErr := r.(error); isErr && match(err) {
				caught, ok = err, true
				return
			}
			panic(r)
		}
	}()
	fn()
	return nil, false
}

// lastLine is a traceback's last line: `<module.>Class: message`.
func lastLine(r any) string {
	err, ok := r.(error)
	if !ok {
		err = &herr.Panic{Value: r}
	}
	name := ""
	var pe *py.Error
	var p *herr.Panic
	var re runtime.Error
	var pathErr *fs.PathError
	var timeout *timeoutExpired
	var harness herr.Harness
	switch {
	case errors.As(err, &pe):
		name = pe.Type
	case errors.As(err, &harness):
		name = "harness.errors." + reflect.TypeOf(harness).Elem().Name()
	case errors.As(err, &timeout):
		name = "subprocess.TimeoutExpired"
	case errors.As(err, &pathErr):
		return osErrorLine(pathErr)
	case errors.As(err, &p):
		name = "Exception"
	case errors.As(err, &re):
		name = "RuntimeError"
	default:
		name = strings.TrimPrefix(reflect.TypeOf(err).String(), "*")
	}
	if msg := err.Error(); msg != "" {
		return name + ": " + msg
	}
	return name
}

// osErrorLine is the last line of an OSError's traceback: `FileNotFoundError: [Errno 2] No such
// file or directory: 'plan.json'`.
func osErrorLine(e *fs.PathError) string {
	var errno syscall.Errno
	if !errors.As(e.Err, &errno) {
		return "OSError: " + e.Error()
	}
	name := "OSError"
	switch errno {
	case syscall.ENOENT:
		name = "FileNotFoundError"
	case syscall.EACCES, syscall.EPERM:
		name = "PermissionError"
	case syscall.EISDIR:
		name = "IsADirectoryError"
	case syscall.ENOTDIR:
		name = "NotADirectoryError"
	case syscall.EEXIST:
		name = "FileExistsError"
	}
	return fmt.Sprintf("%s: %s", name, osErrorText(e))
}

// osErrorText is str(OSError): `[Errno 2] No such file or directory: 'plan.json'`.
func osErrorText(err error) string {
	var pathErr *fs.PathError
	var errno syscall.Errno
	if errors.As(err, &pathErr) && errors.As(pathErr.Err, &errno) {
		return fmt.Sprintf("[Errno %d] %s: %s", int(errno), capitalize(errno.Error()), py.Repr(pathErr.Path))
	}
	var execErr *exec.Error
	if errors.As(err, &execErr) {
		if errors.As(execErr.Err, &errno) {
			return fmt.Sprintf("[Errno %d] %s: %s", int(errno), capitalize(errno.Error()), py.Repr(execErr.Name))
		}
		return fmt.Sprintf("[Errno %d] %s: %s", int(syscall.ENOENT), "No such file or directory", py.Repr(execErr.Name))
	}
	if errors.As(err, &errno) {
		return fmt.Sprintf("[Errno %d] %s", int(errno), capitalize(errno.Error()))
	}
	return err.Error()
}

func capitalize(s string) string {
	if s == "" {
		return s
	}
	return strings.ToUpper(s[:1]) + s[1:]
}

// excText is str(exc) as an f-string prints it.
func excText(err error) string {
	if isOSError(err) {
		return osErrorText(err)
	}
	return err.Error()
}

// ---------------------------------------------------------------------------- subprocess

// RunOptions are subprocess.run's keyword arguments as this module passes them; every call
// captures the output as text and never checks the exit code.
type RunOptions struct {
	Dir     string            // cwd; "" is this process's
	Env     map[string]string // env; nil is this process's
	Timeout time.Duration     // timeout; 0 waits for ever
}

// Completed is subprocess.CompletedProcess[str].
type Completed struct {
	Args       []string
	Returncode int
	Stdout     string
	Stderr     string
}

// timeoutExpired is subprocess.TimeoutExpired.
type timeoutExpired struct {
	cmd     []string
	timeout time.Duration
}

func (e *timeoutExpired) Error() string {
	secs := e.timeout.Seconds()
	amount := pyjson.FloatRepr(secs)
	if secs == float64(int64(secs)) {
		amount = strconv.FormatInt(int64(secs), 10)
	}
	return fmt.Sprintf("Command '%s' timed out after %s seconds", py.Repr(e.cmd), amount)
}

// subprocessRun is subprocess.run(argv, capture_output=True, text=True, check=False, cwd=…,
// env=…, timeout=…): the finished process, or the OSError (it could not start) or TimeoutExpired
// Python raised. Text mode reads `\r\n` and `\r` as `\n`.
func subprocessRun(argv []string, opts RunOptions) (*Completed, error) {
	ctx := context.Background()
	if opts.Timeout > 0 {
		var cancel context.CancelFunc
		ctx, cancel = context.WithTimeout(ctx, opts.Timeout)
		defer cancel()
	}
	cmd := exec.CommandContext(ctx, argv[0], argv[1:]...)
	cmd.Dir = opts.Dir
	if opts.Env != nil {
		env := make([]string, 0, len(opts.Env))
		for k, v := range opts.Env {
			env = append(env, k+"="+v)
		}
		cmd.Env = env
	}
	var stdout, stderr bytes.Buffer
	cmd.Stdout, cmd.Stderr = &stdout, &stderr
	err := cmd.Run()
	if opts.Timeout > 0 && errors.Is(ctx.Err(), context.DeadlineExceeded) {
		return nil, &timeoutExpired{cmd: argv, timeout: opts.Timeout}
	}
	code := 0
	if err != nil {
		var exit *exec.ExitError
		if !errors.As(err, &exit) {
			return nil, err
		}
		code = exit.ExitCode()
		if ws, ok := exit.Sys().(syscall.WaitStatus); ok && ws.Signaled() {
			code = -int(ws.Signal())
		}
	}
	return &Completed{Args: argv, Returncode: code, Stdout: py.Universal(stdout.String()),
		Stderr: py.Universal(stderr.String())}, nil
}

// ---------------------------------------------------------------------------- values

// timedelta is str(timedelta): `4:00:00`, `0:41:12.345678`, `1 day, 2:00:00`.
func timedelta(d time.Duration) string {
	us := d.Microseconds()
	const day = int64(86400 * 1e6)
	days := us / day
	if us%day != 0 && us < 0 {
		days--
	}
	rest := us - days*day
	secs, micro := rest/1e6, rest%1e6
	out := fmt.Sprintf("%d:%02d:%02d", secs/3600, secs%3600/60, secs%60)
	if micro != 0 {
		out += fmt.Sprintf(".%06d", micro)
	}
	if days != 0 {
		plural := "s"
		if days == 1 || days == -1 {
			plural = ""
		}
		out = fmt.Sprintf("%d day%s, ", days, plural) + out
	}
	return out
}

// hhmm is `f"{t:%H:%M}"` for a datetime or a datetime.time.
func hhmm(v any) string {
	switch t := v.(type) {
	case time.Time:
		return t.Format("15:04")
	case clock.TimeOfDay:
		return fmt.Sprintf("%02d:%02d", t.Hour, t.Minute)
	case *clock.TimeOfDay:
		return fmt.Sprintf("%02d:%02d", t.Hour, t.Minute)
	}
	panic(fmt.Sprintf("cli: a time of type %T", v))
}

// duration is a timedelta constant of another module.
func duration(v any) time.Duration {
	if d, ok := v.(time.Duration); ok {
		return d
	}
	panic(fmt.Sprintf("cli: a timedelta of type %T", v))
}

func cfgStr(v any) string {
	switch x := v.(type) {
	case nil:
		return ""
	case string:
		return x
	case fmt.Stringer:
		return x.String()
	}
	panic(fmt.Sprintf("cli: constant of type %T, not a string", v))
}

// asPath is `path.as_posix()` (or a str) of another module's constant.
func asPath(v any) string { return cfgStr(v) }

// others is config.OTHERS.
func others() []*identity.Other {
	switch x := any(config.Others).(type) {
	case nil:
		return nil
	case []*identity.Other:
		return x
	case []identity.Other:
		out := make([]*identity.Other, len(x))
		for i := range x {
			out[i] = &x[i]
		}
		return out
	}
	panic(fmt.Sprintf("cli: config.OTHERS of type %T", config.Others))
}

// labelSpec is one of config.LABELS: name: (color, description).
type labelSpec struct{ name, color, description string }

// labels is `LABELS.items()`, in order.
func labels() []labelSpec {
	var out []labelSpec
	var o *pyjson.Object
	switch x := any(config.Labels).(type) {
	case nil:
		return nil
	case *pyjson.Object:
		o = x
	default:
		panic(fmt.Sprintf("cli: config.LABELS of type %T", config.Labels))
	}
	o.Items(func(name string, v any) bool {
		var pair []string
		switch t := v.(type) {
		case []any:
			pair = pyjson.Strings(t)
		case []string:
			pair = t
		case [2]string:
			pair = t[:]
		}
		if len(pair) != 2 {
			panic(fmt.Sprintf("cli: config.LABELS[%q] of type %T", name, v))
		}
		out = append(out, labelSpec{name, pair[0], pair[1]})
		return true
	})
	return out
}

// tierEntries is `cfg.pool.tiers.get(tier, ())`.
func tierEntries(pool *providers.Pool, tier string) []*providers.TierEntry {
	switch x := pool.Tiers.Get(tier).(type) {
	case nil:
		return nil
	case []*providers.TierEntry:
		return x
	case []any:
		var out []*providers.TierEntry
		for _, e := range x {
			out = append(out, e.(*providers.TierEntry))
		}
		return out
	}
	panic(fmt.Sprintf("cli: a pool tier of type %T", pool.Tiers.Get(tier)))
}
