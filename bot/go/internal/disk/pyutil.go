package disk

// The Python behaviours disk.py depends on where Go's own differ and py does not hold them:
// subprocess.run's way of finding, timing out and reporting a program, with the OSError and
// TimeoutExpired texts the clean-up's note shows (runner keeps the same subprocess helpers).

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"io/fs"
	"math"
	"os"
	"os/exec"
	"sort"
	"strconv"
	"strings"
	"syscall"
	"time"
	"unicode/utf8"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// osError is an OSError as Python prints it: "[Errno 2] No such file or directory: 'bash'".
type osError struct {
	errno    syscall.Errno
	filename string
}

func (e *osError) Error() string { return py.ErrnoText(e.errno) + ": " + py.Repr(e.filename) }

func (e *osError) Unwrap() error { return e.errno }

// timeoutExpired is subprocess.TimeoutExpired.
type timeoutExpired struct {
	cmd     []string
	timeout time.Duration
}

func (e *timeoutExpired) Error() string {
	s := e.timeout.Seconds()
	shown := strconv.FormatFloat(s, 'f', -1, 64)
	if s != math.Trunc(s) {
		shown = py.Str(s)
	}
	return fmt.Sprintf("Command '%s' timed out after %s seconds", py.Repr(e.cmd), shown)
}

// unicodeDecodeError is text=True's strict decoding failing on output that is not UTF-8.
type unicodeDecodeError struct{ msg string }

func (e *unicodeDecodeError) Error() string { return e.msg }

// command is the program subprocess runs for argv: argv[0] itself when it names a path, else the
// first executable of that name on the PATH of the child's own environment.
func command(argv []string, env map[string]string) *exec.Cmd {
	cmd := &exec.Cmd{Path: argv[0], Args: argv}
	if strings.Contains(argv[0], "/") {
		return cmd
	}
	path, ok := env["PATH"]
	if !ok {
		path = "/bin:/usr/bin"
	}
	saved := syscall.Errno(0)
	for _, dir := range strings.Split(path, ":") {
		candidate := argv[0]
		if dir != "" {
			candidate = strings.TrimSuffix(dir, "/") + "/" + argv[0]
		}
		info, err := os.Stat(candidate)
		if err != nil {
			var errno syscall.Errno
			if errors.As(err, &errno) && errno != syscall.ENOENT && errno != syscall.ENOTDIR && saved == 0 {
				saved = errno
			}
			continue
		}
		if info.Mode().IsRegular() && info.Mode()&0o111 != 0 {
			cmd.Path = candidate
			return cmd
		}
		if saved == 0 {
			saved = syscall.EACCES
		}
	}
	if saved == 0 {
		saved = syscall.ENOENT
	}
	cmd.Err = &osError{errno: saved, filename: argv[0]}
	return cmd
}

// run is subprocess.run(argv, env=env, capture_output=True, text=True, timeout=timeout): stdin
// is this process's own, the output is decoded strictly as UTF-8 with universal newlines. Past
// the timeout (the output must have ended by then too) the program is killed, not its children,
// and waited for.
func run(argv []string, env map[string]string, timeout time.Duration) (string, string, error) {
	cmd := command(argv, env)
	keys := make([]string, 0, len(env))
	for k := range env {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		cmd.Env = append(cmd.Env, k+"="+env[k])
	}
	if cmd.Env == nil {
		cmd.Env = []string{}
	}
	cmd.Stdin = os.Stdin
	outR, outW, err := os.Pipe()
	if err != nil {
		return "", "", err
	}
	errR, errW, err := os.Pipe()
	if err != nil {
		outR.Close()
		outW.Close()
		return "", "", err
	}
	defer outR.Close()
	defer errR.Close()
	cmd.Stdout, cmd.Stderr = outW, errW
	err = cmd.Start()
	outW.Close()
	errW.Close()
	if err != nil {
		return "", "", startError(err, argv[0])
	}
	expired := make(chan struct{})
	timer := time.AfterFunc(timeout, func() { close(expired) })
	defer timer.Stop()
	var stdout, stderr bytes.Buffer
	read := make(chan struct{}, 2)
	go func() { io.Copy(&stdout, outR); read <- struct{}{} }()
	go func() { io.Copy(&stderr, errR); read <- struct{}{} }()
	exited := make(chan struct{})
	go func() { cmd.Wait(); close(exited) }()
	before := func(ch <-chan struct{}) bool {
		select {
		case <-ch:
			return true
		case <-expired:
			return false
		}
	}
	if !(before(read) && before(read) && before(exited)) {
		cmd.Process.Kill()
		<-exited
		return "", "", &timeoutExpired{cmd: argv, timeout: timeout}
	}
	out, err := decodeStrict(stdout.Bytes())
	if err != nil {
		return "", "", err
	}
	errText, err := decodeStrict(stderr.Bytes())
	if err != nil {
		return "", "", err
	}
	return out, errText, nil
}

// startError is a Go error from starting a program as the OSError Popen raised.
func startError(err error, program string) error {
	var mine *osError
	if errors.As(err, &mine) {
		return mine
	}
	var errno syscall.Errno
	if !errors.As(err, &errno) {
		return err
	}
	var pe *fs.PathError
	if errors.As(err, &pe) && pe.Op != "fork/exec" {
		return &osError{errno: errno, filename: pe.Path} // the working directory
	}
	return &osError{errno: errno, filename: program}
}

// decodeStrict is bytes.decode("utf-8") with universal newlines, as text=True reads output:
// UnicodeDecodeError, which Python's clean() does not catch, on bytes that are not UTF-8.
func decodeStrict(b []byte) (string, error) {
	for i := 0; i < len(b); {
		r, size := utf8.DecodeRune(b[i:])
		if r == utf8.RuneError && size <= 1 {
			return "", &unicodeDecodeError{fmt.Sprintf(
				"'utf-8' codec can't decode byte 0x%02x in position %d: invalid utf-8", b[i], i)}
		}
		i += size
	}
	return py.Universal(string(b)), nil
}
