package cli

// Private helpers the command line needs from Python itself that py and herr do not hold: the
// exceptions its except clauses name (ValueError), the last line of a traceback, `str(timedelta)`,
// subprocess.run, and pathlib's and json's reads and writes.

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"syscall"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
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
	return !herr.IsHarness(err) && !herr.IsOSError(err) && !errors.As(err, &p) && !errors.As(err, &re)
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
	var exc *py.Error
	var timeout *timeoutExpired
	var panicked *herr.Panic
	var fault runtime.Error
	name := strings.TrimPrefix(fmt.Sprintf("%T", err), "*") // a Go error's own type
	switch {
	case errors.As(err, &exc), herr.IsOSError(err), errors.As(err, &panicked), errors.As(err, &fault):
		name = herr.Name(err)
	case herr.IsHarness(err):
		name = "harness.errors." + herr.Name(err)
	case errors.As(err, &timeout):
		name = "subprocess.TimeoutExpired"
	}
	if msg := herr.Str(err); msg != "" {
		return name + ": " + msg
	}
	return name
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
