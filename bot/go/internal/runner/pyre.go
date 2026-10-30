package runner

// The Python behaviours this package's parsing and processes depend on where Go's own differ and
// py does not hold them: `\b` at a byte offset, the extra letters re.IGNORECASE matches (ı and İ
// for i), a file read line by line in text mode, pathlib's with_suffix, and subprocess's way of
// finding, starting, timing out and reporting a program (Popen, run, OSError, TimeoutExpired).

import (
	"bufio"
	"bytes"
	"errors"
	"fmt"
	"io"
	"io/fs"
	"math"
	"os"
	"os/exec"
	"os/user"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"syscall"
	"time"
	"unicode/utf8"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// pySpaceNoNL is `[^\S\n]`: Python's whitespace but the line feed.
const pySpaceNoNL = `\t\v\f\r \x1c-\x1f\x85\xa0\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}`

// fold is the ASCII letter re.IGNORECASE matches r to: a letter's lower case, and the four
// others Python counts (İ and ı as i, ſ as s, the Kelvin sign as k). Anything else is itself.
func fold(r rune) rune {
	switch r {
	case 0x130, 0x131:
		return 'i'
	case 0x17f:
		return 's'
	case 0x212a:
		return 'k'
	}
	if r >= 'A' && r <= 'Z' {
		return r + 'a' - 'A'
	}
	return r
}

// isAZ is what `(?i)[a-z]` matches in Python.
func isAZ(r rune) bool { f := fold(r); return f >= 'a' && f <= 'z' }

// foldI is text with ı and İ as i: RE2's (?i) matches neither to an i, Python's matches both,
// so a search with no ı or İ of its own reads the same through this.
func foldI(text string) string {
	if !strings.ContainsAny(text, "ıİ") {
		return text
	}
	return strings.NewReplacer("ı", "i", "İ", "i").Replace(text)
}

// beforeIsWord is whether the character before byte offset i of text is a word character.
func beforeIsWord(text string, i int) bool {
	if i <= 0 {
		return false
	}
	r, _ := utf8.DecodeLastRuneInString(text[:i])
	return py.IsWord(r)
}

// afterIsWord is whether the character at byte offset i of text is a word character.
func afterIsWord(text string, i int) bool {
	if i >= len(text) {
		return false
	}
	r, _ := utf8.DecodeRuneInString(text[i:])
	return py.IsWord(r)
}

// findAfterBoundary is re.finditer for a pattern that begins with `\b` before a word
// character (RE2 has no Unicode `\b`): the matches of re, leftmost first and not overlapping,
// that start after a non-word character or at the start of text. Each is a submatch index
// slice into text.
func findAfterBoundary(re *regexp.Regexp, text string) [][]int {
	var found [][]int
	for pos := 0; pos <= len(text); {
		loc := re.FindStringSubmatchIndex(text[pos:])
		if loc == nil {
			break
		}
		for i := range loc {
			if loc[i] >= 0 {
				loc[i] += pos
			}
		}
		if !beforeIsWord(text, loc[0]) {
			found = append(found, loc)
			if loc[1] > loc[0] {
				pos = loc[1]
				continue
			}
		}
		_, size := utf8.DecodeRuneInString(text[loc[0]:])
		pos = loc[0] + max(size, 1)
	}
	return found
}

// eachLine is `for line in open(path, encoding="utf-8", errors="replace")`: each line as
// text mode gives it, ending in "\n" but perhaps the last.
func eachLine(path string, fn func(line string)) error {
	f, err := os.Open(path)
	if err != nil {
		return err
	}
	defer f.Close()
	r := bufio.NewReaderSize(f, 1<<16)
	for {
		chunk, err := r.ReadBytes('\n')
		if len(chunk) > 0 {
			for _, line := range splitCR(chunk) {
				fn(py.Universal(py.DecodeReplace(line)))
			}
		}
		if err == io.EOF {
			return nil
		}
		if err != nil {
			return err
		}
	}
}

// splitCR splits a chunk that ends at "\n" (or the end of the file) after each lone "\r", which
// text mode reads as a line end of its own.
func splitCR(chunk []byte) [][]byte {
	var out [][]byte
	start := 0
	for i := 0; i < len(chunk); i++ {
		if chunk[i] == '\r' && (i+1 >= len(chunk) || chunk[i+1] != '\n') {
			out = append(out, chunk[start:i+1])
			start = i + 1
		}
	}
	if start < len(chunk) {
		out = append(out, chunk[start:])
	}
	return out
}

// withSuffix is pathlib's with_suffix: the last component's suffix (from its last dot, when
// the dot is neither its first nor its last character) replaced by suffix.
func withSuffix(path, suffix string) string {
	dir, name := "", path
	if i := strings.LastIndex(path, "/"); i >= 0 {
		dir, name = path[:i+1], path[i+1:]
	}
	if i := strings.LastIndex(name, "."); i > 0 && i < len(name)-1 {
		name = name[:i]
	}
	return dir + name + suffix
}

// homeDir is Path.home().
func homeDir() string {
	if h, err := os.UserHomeDir(); err == nil {
		return h
	}
	if u, err := user.Current(); err == nil {
		return u.HomeDir
	}
	return ""
}

// seconds is a Python duration in seconds as a Go one.
func seconds(s float64) time.Duration { return time.Duration(s * float64(time.Second)) }

// recoverThread ends a goroutine that stands for a Python thread as an uncaught exception ends
// one: reported on stderr, the process going on.
func recoverThread(name string) {
	if r := recover(); r != nil {
		fmt.Fprintf(os.Stderr, "Exception in thread %s:\n%v\n", name, r)
	}
}

// osError is an OSError as Python prints it: "[Errno 2] No such file or directory: 'claude'".
type osError struct {
	errno    syscall.Errno
	filename string
	named    bool
}

func (e *osError) Error() string {
	text := py.ErrnoText(e.errno)
	if e.named {
		text += ": " + py.Repr(e.filename)
	}
	return text
}

func (e *osError) Unwrap() error { return e.errno }

// pyOSError is a Go error from opening a file or starting a program as the OSError Python
// raised: named for the file, the working directory that was not there, or the program as it
// was given when it would not run.
func pyOSError(err error, program string) error {
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
		return &osError{errno: errno, filename: pe.Path, named: true} // a file, or the working directory
	}
	return &osError{errno: errno, filename: program, named: true}
}

// writeError is a failed write as os.write's OSError, which names no file.
func writeError(err error) error {
	var errno syscall.Errno
	if !errors.As(err, &errno) {
		return err
	}
	return &osError{errno: errno}
}

// command is the program subprocess.Popen runs for argv: argv[0] itself when it names a path,
// else the first executable of that name on the PATH of the child's own environment
// (os.get_exec_path(env)), with Popen's FileNotFoundError or PermissionError when there is none.
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
	cmd.Err = &osError{errno: saved, filename: argv[0], named: true}
	return cmd
}

// environ is a child's environment as exec wants it, in a fixed order.
func environ(env map[string]string) []string {
	keys := make([]string, 0, len(env))
	for k := range env {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	out := make([]string, len(keys))
	for i, k := range keys {
		out[i] = k + "=" + env[k]
	}
	return out
}

// returnCode is Popen.returncode: the exit status, or minus the signal that ended it.
func returnCode(ps *os.ProcessState) int {
	if ws, ok := ps.Sys().(syscall.WaitStatus); ok && ws.Signaled() {
		return -int(ws.Signal())
	}
	return ps.ExitCode()
}

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

// RunArgs are the keyword arguments ping and agy_usage give subprocess.run, which always
// captures the output as text (UTF-8, errors replaced).
type RunArgs struct {
	Cwd string
	Env map[string]string
	// Input is input=...: written to the program's stdin, then closed. nil is
	// stdin=subprocess.DEVNULL.
	Input   *string
	Timeout time.Duration
}

// Completed is the subprocess.CompletedProcess subprocess.run returns.
type Completed struct {
	Args       []string
	ReturnCode int
	Stdout     string
	Stderr     string
}

// RunFunc is subprocess.run as ping and agy_usage call it (a test passes its own). Its error is
// what Python raised: the OSError of a program that would not start, or TimeoutExpired.
type RunFunc func(argv []string, args RunArgs) (*Completed, error)

// SubprocessRun is subprocess.run(argv, cwd=, env=, input= or stdin=DEVNULL,
// capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=): what a nil
// RunFunc stands for. Past the timeout it kills the program (not its children), waits for it
// and returns TimeoutExpired, as Python does; the output must also have ended by then, so a
// child that holds the pipes open runs it out too.
func SubprocessRun(argv []string, args RunArgs) (*Completed, error) {
	cmd := command(argv, args.Env)
	cmd.Dir = args.Cwd
	cmd.Env = environ(args.Env)
	outR, outW, err := os.Pipe()
	if err != nil {
		return nil, err
	}
	errR, errW, err := os.Pipe()
	if err != nil {
		outR.Close()
		outW.Close()
		return nil, err
	}
	defer outR.Close()
	defer errR.Close()
	cmd.Stdout, cmd.Stderr = outW, errW
	var inW *os.File
	if args.Input != nil {
		inR, w, err := os.Pipe()
		if err != nil {
			outW.Close()
			errW.Close()
			return nil, err
		}
		defer inR.Close()
		cmd.Stdin, inW = inR, w
	}
	err = cmd.Start()
	outW.Close()
	errW.Close()
	if err != nil {
		if inW != nil {
			inW.Close()
		}
		return nil, pyOSError(err, argv[0])
	}
	if f, ok := cmd.Stdin.(*os.File); ok {
		f.Close()
	}
	expired := make(chan struct{})
	timer := time.AfterFunc(args.Timeout, func() { close(expired) })
	defer timer.Stop()
	var stdout, stderr bytes.Buffer
	read := make(chan struct{}, 2)
	go func() { io.Copy(&stdout, outR); read <- struct{}{} }()
	go func() { io.Copy(&stderr, errR); read <- struct{}{} }()
	written := make(chan struct{})
	if inW != nil {
		go func() {
			defer close(written)
			inW.WriteString(*args.Input)
			inW.Close()
		}()
	} else {
		close(written)
	}
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
	if before(written) && before(read) && before(read) && before(exited) {
		return &Completed{Args: argv, ReturnCode: returnCode(cmd.ProcessState),
			Stdout: py.Universal(py.DecodeReplace(stdout.Bytes())),
			Stderr: py.Universal(py.DecodeReplace(stderr.Bytes()))}, nil
	}
	cmd.Process.Kill()
	<-exited
	return nil, &timeoutExpired{cmd: argv, timeout: args.Timeout}
}
