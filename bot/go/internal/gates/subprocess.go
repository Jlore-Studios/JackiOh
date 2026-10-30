package gates

import (
	"bytes"
	"io"
	"os"
	"os/exec"
	"sort"
	"strings"
	"sync"
	"syscall"
	"time"
	"unicode/utf8"
)

// completed is what subprocess.run(..., capture_output=True, timeout=...) gives back: the output
// as bytes, the return code (minus the signal's number when one ended the process), and whether
// the timeout struck first.
type completed struct {
	stdout, stderr []byte
	code           int
	timedOut       bool
}

// lockedBuffer is a pipe's output so far, read while the process runs.
type lockedBuffer struct {
	mu  sync.Mutex
	buf bytes.Buffer
}

func (b *lockedBuffer) Write(p []byte) (int, error) {
	b.mu.Lock()
	defer b.mu.Unlock()
	return b.buf.Write(p)
}

func (b *lockedBuffer) bytes() []byte {
	b.mu.Lock()
	defer b.mu.Unlock()
	return append([]byte(nil), b.buf.Bytes()...)
}

// run is subprocess.run(args, cwd=cwd, env=env, capture_output=True, timeout=timeout) on POSIX:
// it reads both pipes to their end and waits for the process, all within the timeout. When the
// timeout strikes first, the process is killed (SIGKILL, the process alone, not what it started)
// and reaped, and the output read until then is what is left: subprocess does not wait for the
// pipes once it has killed the process. The child inherits stdin, as there. A process that
// cannot start raises the OSError.
func run(args []string, cwd string, env map[string]string, timeout time.Duration) completed {
	cmd := exec.Command(args[0], args[1:]...)
	cmd.Dir = cwd
	cmd.Env = envList(env)
	cmd.Stdin = os.Stdin
	outR, outW, err := os.Pipe()
	if err != nil {
		panic(err)
	}
	errR, errW, err := os.Pipe()
	if err != nil {
		panic(err)
	}
	cmd.Stdout, cmd.Stderr = outW, errW
	deadline := time.NewTimer(timeout)
	defer deadline.Stop()
	if err := cmd.Start(); err != nil {
		for _, f := range []*os.File{outR, outW, errR, errW} {
			f.Close()
		}
		panic(err)
	}
	outW.Close()
	errW.Close()
	var stdout, stderr lockedBuffer
	var reading sync.WaitGroup
	for _, p := range []struct {
		r *os.File
		w io.Writer
	}{{outR, &stdout}, {errR, &stderr}} {
		reading.Add(1)
		go func() {
			defer reading.Done()
			io.Copy(p.w, p.r)
		}()
	}
	piped := make(chan struct{})
	go func() {
		reading.Wait()
		close(piped)
	}()
	var waitErr error
	exited := make(chan struct{})
	go func() {
		waitErr = cmd.Wait()
		close(exited)
	}()
	timedOut := false
	select {
	case <-piped:
		select {
		case <-exited:
		case <-deadline.C:
			timedOut = true
		}
	case <-deadline.C:
		timedOut = true
	}
	if timedOut {
		cmd.Process.Kill()
		<-exited
		outR.Close()
		errR.Close()
		return completed{stdout: stdout.bytes(), stderr: stderr.bytes(), timedOut: true}
	}
	outR.Close()
	errR.Close()
	return completed{stdout: stdout.bytes(), stderr: stderr.bytes(), code: returnCode(cmd, waitErr)}
}

// returnCode is Popen.returncode: the exit status, or minus the signal that ended the process.
func returnCode(cmd *exec.Cmd, waitErr error) int {
	state := cmd.ProcessState
	if state == nil {
		if waitErr != nil {
			panic(waitErr)
		}
		return 0
	}
	if ws, ok := state.Sys().(syscall.WaitStatus); ok && ws.Signaled() {
		return -int(ws.Signal())
	}
	return state.ExitCode()
}

// envList is env=dict(env): exactly these variables, none inherited.
func envList(env map[string]string) []string {
	out := make([]string, 0, len(env))
	for k, v := range env {
		out = append(out, k+"="+v)
	}
	sort.Strings(out)
	return out
}

// translateNewlines is what text mode does to the output: "\r\n" and "\r" become "\n".
func translateNewlines(s string) string {
	return strings.ReplaceAll(strings.ReplaceAll(s, "\r\n", "\n"), "\r", "\n")
}

// decode is bytes.decode("utf-8", "replace"): each maximal ill-formed subsequence becomes one
// U+FFFD, as Python's decoder does (Go's own conversion would leave the bytes in the string).
func decode(b []byte) string {
	var out strings.Builder
	for i := 0; i < len(b); {
		r, size := utf8.DecodeRune(b[i:])
		if r != utf8.RuneError || size > 1 {
			out.WriteRune(r)
			i += size
			continue
		}
		out.WriteRune(utf8.RuneError)
		i += invalidLength(b[i:])
	}
	return out.String()
}

// invalidLength is how many bytes at the start of b (which do not start a well-formed sequence)
// form the longest prefix of one: the bytes one U+FFFD stands for, at least 1.
func invalidLength(b []byte) int {
	lo, hi := byte(0x80), byte(0xBF)
	var need int
	switch c := b[0]; {
	case c >= 0xC2 && c <= 0xDF:
		need = 1
	case c == 0xE0:
		need, lo = 2, 0xA0
	case c >= 0xE1 && c <= 0xEC, c == 0xEE, c == 0xEF:
		need = 2
	case c == 0xED:
		need, hi = 2, 0x9F
	case c == 0xF0:
		need, lo = 3, 0x90
	case c >= 0xF1 && c <= 0xF3:
		need = 3
	case c == 0xF4:
		need, hi = 3, 0x8F
	default:
		return 1
	}
	n := 1
	for k := 0; k < need && n < len(b); k++ {
		if b[n] < lo || b[n] > hi {
			break
		}
		lo, hi = 0x80, 0xBF
		n++
	}
	return n
}
