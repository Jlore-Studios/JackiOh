package runner

// The Python behaviours this package's parsing and processes depend on where Go's own differ:
// what `\s`, `\w`, `\d` and `\b` match in a Python regular expression over str (Unicode, where
// RE2's are ASCII), the extra letters re.IGNORECASE matches (ı and İ for i), str.strip() and
// str.split(), int() and float() over Unicode digits, bytes.decode("utf-8", "replace"), the
// universal newlines of a file read as text, pathlib's with_suffix, and subprocess's way of
// finding, starting, timing out and reporting a program (Popen, run, OSError,
// TimeoutExpired). Each is private here; py could hold them for every package.

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
	"unicode"
	"unicode/utf8"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// pySpace is the body of a character class matching Python's `\s` over str (str.isspace).
const pySpace = `\t\n\v\f\r \x1c-\x1f\x85\xa0\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}`

// pySpaceNoNL is `[^\S\n]`: Python's whitespace but the line feed.
const pySpaceNoNL = `\t\v\f\r \x1c-\x1f\x85\xa0\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}`

// pyWord is the body of a character class matching Python's `\w` over str: letters, numbers and
// the underscore, with CJK Extension I, which Python 3.13 (Unicode 15.1) counts as letters and
// Go's tables (Unicode 15.0) do not know yet.
const pyWord = `\p{L}\p{N}_\x{2ebf0}-\x{2ee5d}`

// pyDigit is Python's `\d` over str: a decimal digit of any script.
const pyDigit = `\p{Nd}`

// isSpace is str.isspace() for one character, and what `\s` matches.
func isSpace(r rune) bool {
	switch {
	case r >= '\t' && r <= '\r', r >= 0x1c && r <= 0x20, r == 0x85, r == 0xa0, r == 0x1680,
		r >= 0x2000 && r <= 0x200a, r == 0x2028, r == 0x2029, r == 0x202f, r == 0x205f,
		r == 0x3000:
		return true
	}
	return false
}

// isWord is what Python's `\w` matches.
func isWord(r rune) bool {
	return unicode.IsLetter(r) || unicode.IsNumber(r) || r == '_' || (r >= 0x2ebf0 && r <= 0x2ee5d)
}

// isDigit is what Python's `\d` matches.
func isDigit(r rune) bool { return unicode.IsDigit(r) }

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
	return isWord(r)
}

// afterIsWord is whether the character at byte offset i of text is a word character.
func afterIsWord(text string, i int) bool {
	if i >= len(text) {
		return false
	}
	r, _ := utf8.DecodeRuneInString(text[i:])
	return isWord(r)
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

// digitValue is the value of one decimal digit of any script: Unicode assigns them in runs of
// ten, zero first.
func digitValue(r rune) int {
	if r >= '0' && r <= '9' {
		return int(r - '0')
	}
	start := r
	for unicode.IsDigit(start - 1) {
		start--
	}
	return int(r-start) % 10
}

// atoi is int(s) for a string of decimal digits (`\d+`), of any script.
func atoi(s string) int {
	n := 0
	for _, r := range s {
		n = n*10 + digitValue(r)
	}
	return n
}

// asciiDigits is s with every decimal digit written in ASCII, for strconv.
func asciiDigits(s string) string {
	return strings.Map(func(r rune) rune {
		if r > 0x7f && unicode.IsDigit(r) {
			return '0' + rune(digitValue(r))
		}
		return r
	}, s)
}

// atof is float(s) for `\d+(?:\.\d+)?`, of any script.
func atof(s string) float64 {
	f, _ := strconv.ParseFloat(asciiDigits(s), 64)
	return f
}

// isDecimal is str.isdigit() as int() then reads it: every character a decimal digit. (A
// superscript digit is isdigit() but not int(), so Python raised there; here it is text.)
func isDecimal(s string) bool {
	if s == "" {
		return false
	}
	for _, r := range s {
		if !unicode.IsDigit(r) {
			return false
		}
	}
	return true
}

// strip is str.strip().
func strip(s string) string { return strings.TrimFunc(s, isSpace) }

// lstrip is str.lstrip().
func lstrip(s string) string { return strings.TrimLeftFunc(s, isSpace) }

// rstrip is str.rstrip().
func rstrip(s string) string { return strings.TrimRightFunc(s, isSpace) }

// fields is str.split() with no separator.
func fields(s string) []string { return strings.FieldsFunc(s, isSpace) }

// isNumber is isinstance(v, (int, float)) for a JSON value (a bool is an int in Python).
func isNumber(v any) bool {
	switch v.(type) {
	case int, float64, bool:
		return true
	}
	return false
}

// toFloat is float(v) for a value isNumber accepts.
func toFloat(v any) float64 {
	switch x := v.(type) {
	case int:
		return float64(x)
	case float64:
		return x
	case bool:
		if x {
			return 1
		}
	}
	return 0
}

// toInt is int(v) for a value isNumber accepts: a float truncated.
func toInt(v any) int {
	if f, ok := v.(float64); ok {
		return int(f)
	}
	return int(toFloat(v))
}

// pyFloat is float(v) for any JSON value: a number as it is, a string as Python parses one, and
// anything else Python's TypeError.
func pyFloat(v any) float64 {
	if isNumber(v) {
		return toFloat(v)
	}
	if s, ok := v.(string); ok {
		f, err := strconv.ParseFloat(strings.ReplaceAll(asciiDigits(strip(s)), "_", ""), 64)
		if err != nil {
			panic(fmt.Errorf("could not convert string to float: %s", py.Repr(s)))
		}
		return f
	}
	panic(fmt.Errorf("float() argument must be a string or a real number, not '%s'", pyType(v)))
}

func pyType(v any) string {
	switch v.(type) {
	case nil:
		return "NoneType"
	case []any:
		return "list"
	}
	return "dict"
}

// decodeReplace is bytes.decode("utf-8", errors="replace"): each maximal invalid subpart is
// one U+FFFD, as Python's decoder does (Go's own range over a string gives one per byte).
func decodeReplace(b []byte) string {
	if utf8.Valid(b) {
		return string(b)
	}
	var sb strings.Builder
	for i := 0; i < len(b); {
		r, size := utf8.DecodeRune(b[i:])
		if r != utf8.RuneError || size > 1 {
			sb.Write(b[i : i+size])
			i += size
			continue
		}
		i += invalidLen(b[i:])
		sb.WriteRune(utf8.RuneError)
	}
	return sb.String()
}

// invalidLen is the length of the maximal subpart of an ill-formed UTF-8 sequence at b[0].
func invalidLen(b []byte) int {
	c := b[0]
	need, lo, hi := 0, byte(0x80), byte(0xbf)
	switch {
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
		x := b[n]
		if k > 0 {
			lo, hi = 0x80, 0xbf
		}
		if x < lo || x > hi {
			break
		}
		n++
	}
	return n
}

// universal is text read in Python's text mode: "\r\n" and a lone "\r" become "\n".
func universal(s string) string {
	if !strings.Contains(s, "\r") {
		return s
	}
	return strings.ReplaceAll(strings.ReplaceAll(s, "\r\n", "\n"), "\r", "\n")
}

// readText is path.read_text(encoding="utf-8", errors="replace").
func readText(path string) (string, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return "", err
	}
	return universal(decodeReplace(b)), nil
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
				fn(universal(decodeReplace(line)))
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
	msg := e.errno.Error()
	if msg != "" {
		msg = strings.ToUpper(msg[:1]) + msg[1:]
	}
	text := fmt.Sprintf("[Errno %d] %s", int(e.errno), msg)
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
			Stdout: universal(decodeReplace(stdout.Bytes())),
			Stderr: universal(decodeReplace(stderr.Bytes()))}, nil
	}
	cmd.Process.Kill()
	<-exited
	return nil, &timeoutExpired{cmd: argv, timeout: args.Timeout}
}
