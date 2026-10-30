package runner

import (
	"errors"
	"io"
	"io/fs"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"syscall"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/logins"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Runner is a backend as the harness drives it (Python's duck typing of `runner`): every CLI
// backend, and FakeRunner.
type Runner interface {
	Run(request *RunRequest) *RunResult
	Trail(transcript string) string
}

// UsagePoller is a backend that may read its subscription's usage with its CLI's own command
// (`polls_usage`, `read_usage`); `getattr(runner, "polls_usage", False)` is a type assertion to
// it and a call of PollsUsage.
type UsagePoller interface {
	PollsUsage() bool
	ReadUsage(model string) *pyjson.Object
}

// Popen starts a CLI as subprocess.Popen does (the backends' `popen`, a test seam): the command
// for argv, to be run with env. nil is subprocess.Popen itself.
type Popen func(argv []string, env map[string]string) *exec.Cmd

// launchResult is _Launch.
type launchResult struct {
	code     int
	timedOut bool
	raw      string
	stderr   string
	elapsed  float64
	// stopped is why the usage watcher stopped the call, if it did ("" when it did not).
	stopped string
}

// backend is what _Cli's own methods call on `self` that a backend may override.
type backend interface {
	LiveUsage(line string) *pyjson.Object
	PollsUsage() bool
	ReadUsage(model string) *pyjson.Object
}

// cliBase is _Cli: what every CLI backend shares: the environment, the launch, the transcript.
type cliBase struct {
	Binary string
	Login  *logins.Login
	popen  Popen
	// cli is the class's `cli`.
	cli string
	// self is the backend this is part of, for the methods it overrides.
	self backend
}

func newCliBase(binary string, login *logins.Login, popen Popen, cli string) cliBase {
	if popen == nil {
		popen = command
	}
	return cliBase{Binary: binary, Login: login, popen: popen, cli: cli}
}

// ClaudeBin is the name older callers used.
func (x *cliBase) ClaudeBin() string { return x.Binary }

// CLI is the class's `cli`: which CLI this backend runs.
func (x *cliBase) CLI() string { return x.cli }

// Env is the environment the CLI gets: every secret stripped, and its own login.
func (x *cliBase) Env() map[string]string {
	env := copyEnv(config.ChildEnv(nil, nil))
	for _, key := range ActionsFiles {
		delete(env, key)
	}
	if x.Login != nil {
		for k, v := range x.Login.Env {
			env[k] = v
		}
	}
	return env
}

func copyEnv(env map[string]string) map[string]string {
	out := make(map[string]string, len(env)+4)
	for k, v := range env {
		out[k] = v
	}
	return out
}

// paths is the transcript and the raw output beside it.
func (x *cliBase) paths(request *RunRequest) (string, string) {
	transcript := request.Transcript
	if transcript == "" {
		transcript = filepath.Join(request.Cwd, "."+x.cli+"-"+request.Role+".jsonl")
	}
	herr.Must(os.MkdirAll(filepath.Dir(transcript), 0o777))
	return transcript, withSuffix(transcript, ".raw")
}

// process is a started CLI, waited for once by its own goroutine (Popen's wait and poll).
type process struct {
	cmd    *exec.Cmd
	pid    int
	exited chan struct{}
}

func startWaiting(cmd *exec.Cmd) *process {
	p := &process{cmd: cmd, pid: cmd.Process.Pid, exited: make(chan struct{})}
	go func() {
		cmd.Wait()
		close(p.exited)
	}()
	return p
}

// wait is Popen.wait(timeout): whether it ended within timeout (a negative one waits for ever).
func (p *process) wait(timeout time.Duration) bool {
	if timeout < 0 {
		<-p.exited
		return true
	}
	timer := time.NewTimer(timeout)
	defer timer.Stop()
	select {
	case <-p.exited:
		return true
	case <-timer.C:
		return false
	}
}

// running is `proc.poll() is None`.
func (p *process) running() bool {
	select {
	case <-p.exited:
		return false
	default:
		return true
	}
}

// returncode is Popen.returncode once it ended: nil while it runs or when it could not be read.
func (p *process) returncode() *int {
	if p.running() || p.cmd.ProcessState == nil {
		return nil
	}
	code := returnCode(p.cmd.ProcessState)
	return &code
}

// stopBox is the watcher's `stopped` list, shared with the call.
type stopBox struct {
	mu     sync.Mutex
	reason []string
}

func (s *stopBox) add(reason string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.reason = append(s.reason, reason)
}

func (s *stopBox) first() string {
	s.mu.Lock()
	defer s.mu.Unlock()
	if len(s.reason) == 0 {
		return ""
	}
	return s.reason[0]
}

// launch is _Cli._launch: the CLI run with its output in `raw`, its stderr beside it, stdin
// holding `stdinText` (nil for none), killed with its process group at the request's timeout,
// and everything it left behind reaped. The RunResult is a CLI that is not there.
func (x *cliBase) launch(argv []string, request *RunRequest, env map[string]string,
	stdinText *string, raw string) (*launchResult, *RunResult) {
	started := time.Now()
	marker := request.Role + "-" + strconv.Itoa(os.Getpid()) + "-" + strconv.FormatInt(time.Now().UnixNano(), 10)
	env = copyEnv(env)
	env[CallMarker] = marker
	timedOut := false
	var stopped stopBox
	before := ownPIDs()
	var session []int
	stderrPath := withSuffix(raw, ".stderr")
	notFound := func(err error) *RunResult {
		if !errors.Is(err, fs.ErrNotExist) {
			herr.Raise(pyOSError(err, argv[0]))
		}
		r := NewRunResult(false, "")
		r.ExitCode = ExitNotFound
		r.Error = x.Binary + " not found"
		return r
	}
	code := 0
	failed := func() (failed *RunResult) {
		defer func() {
			first := 0
			if len(session) > 0 {
				first = session[0]
			}
			reap(before, marker, first, len(session) > 0)
		}()
		out, err := os.Create(raw)
		if err != nil {
			return notFound(err)
		}
		defer out.Close()
		errFile, err := os.Create(stderrPath)
		if err != nil {
			return notFound(err)
		}
		defer errFile.Close()
		cmd := x.popen(argv, env)
		cmd.Dir = request.Cwd
		cmd.Env = environ(env)
		cmd.Stdout, cmd.Stderr = out, errFile
		cmd.SysProcAttr = &syscall.SysProcAttr{Setsid: true}
		var stdin *os.File
		if stdinText != nil {
			r, w, err := os.Pipe()
			herr.Must(err)
			defer r.Close()
			cmd.Stdin, stdin = r, w
		}
		if err := cmd.Start(); err != nil {
			if stdin != nil {
				stdin.Close()
			}
			return notFound(err)
		}
		if r, ok := cmd.Stdin.(*os.File); ok {
			r.Close()
		}
		proc := startWaiting(cmd)
		session = append(session, proc.pid)
		done := make(chan struct{})
		if request.UsageStop != nil {
			go x.watch(proc, raw, request, done, &stopped)
		}
		func() {
			defer close(done)
			if !communicate(proc, stdin, stdinText, time.Duration(request.TimeoutS)*time.Second) {
				timedOut = true
				kill(proc)
			}
		}()
		if rc := proc.returncode(); rc != nil {
			code = *rc
		} else {
			code = ExitTimeout
		}
		return nil
	}()
	if failed != nil {
		return nil, failed
	}
	return &launchResult{code: code, timedOut: timedOut, raw: raw, stderr: Read(stderrPath),
		elapsed: time.Since(started).Seconds(), stopped: stopped.first()}, nil
}

// communicate is Popen.communicate(text, timeout) for a process whose output goes to files: the
// text written to its stdin and stdin closed, then the process waited for, all within timeout.
// False when the time ran out.
func communicate(proc *process, stdin *os.File, text *string, timeout time.Duration) bool {
	expired := make(chan struct{})
	timer := time.AfterFunc(timeout, func() { close(expired) })
	defer timer.Stop()
	if stdin != nil {
		written := make(chan struct{})
		go func() {
			defer close(written)
			io.WriteString(stdin, *text) // a CLI that left without reading it: Python ignores the broken pipe
			stdin.Close()
		}()
		select {
		case <-written:
		case <-expired:
			return false
		}
	}
	select {
	case <-proc.exited:
		return true
	case <-expired:
		return false
	}
}

// LiveUsage is a usage reading in one line of the running call's output, if the CLI streams any.
func (x *cliBase) LiveUsage(line string) *pyjson.Object { return nil }

// PollsUsage is `polls_usage`: the CLI streams no usage during a call but has a command that
// reads it (`read_usage`): it is read before the run, every `POLL_SECONDS` during a call, and
// after every call.
func (x *cliBase) PollsUsage() bool { return false }

// ReadUsage is the subscription's usage now, for `model`'s pool, read with the CLI's own command;
// nil where it has none or gave no reading.
func (x *cliBase) ReadUsage(model string) *pyjson.Object { return nil }

// withUsage is `result` with a fresh reading, for a CLI that `polls_usage`, after any call its
// login and binary let run. A refusal that named no reset waits for the full window's instead,
// and a failed call whose reading has a window full is that window's refusal, whatever the CLI
// said: agy's and Muse's caps are 100% (#798), so their own limit ends such a call, and Muse's
// words for it are not known.
func (x *cliBase) withUsage(result *RunResult, request *RunRequest) *RunResult {
	if !x.self.PollsUsage() || result.Infra() {
		return result
	}
	usage := x.self.ReadUsage(request.Model)
	if usage.Len() > 0 {
		result.Usage = usage
		if result.ResetAt == defaultPark {
			result.ResetAt = exhaustedReset(usage)
			if result.ResetAt == "" {
				result.ResetAt = defaultPark
			}
		} else if !result.OK && result.ResetAt == "" {
			result.ResetAt = exhaustedReset(usage)
		}
	}
	return result
}

// watch is _Cli._watch: while the call runs, read the usage it streams (or, for a CLI that
// `polls_usage`, its own reading every `POLL_SECONDS`), and stop it once `usage_stop` says so.
// Without this a call (up to `call_timeout_minutes`) ran past its cap unchecked.
func (x *cliBase) watch(proc *process, raw string, request *RunRequest, done <-chan struct{}, stopped *stopBox) {
	defer recoverThread("(_watch)")
	offset, pending := int64(0), ""
	polled := time.Now()
	for {
		timer := time.NewTimer(seconds(WatchSeconds))
		select {
		case <-done:
			timer.Stop()
			return
		case <-timer.C:
		}
		chunk := ""
		if b, err := readFrom(raw, offset); err == nil {
			chunk = py.Universal(py.DecodeReplace(b))
			offset += int64(len(b))
		}
		lines := strings.Split(pending+chunk, "\n")
		pending = lines[len(lines)-1]
		lines = lines[:len(lines)-1]
		var readings []*pyjson.Object
		for _, line := range lines {
			if u := x.self.LiveUsage(line); u.Len() > 0 {
				readings = append(readings, u)
			}
		}
		if x.self.PollsUsage() && time.Since(polled).Seconds() >= PollSeconds {
			polled = time.Now()
			if reading := x.self.ReadUsage(request.Model); reading.Len() > 0 {
				readings = append(readings, reading)
			}
		}
		if len(readings) == 0 || request.UsageStop == nil {
			continue
		}
		reason := request.UsageStop(readings[len(readings)-1])
		if reason != "" && proc.running() {
			stopped.add(reason)
			kill(proc)
			return
		}
	}
}

// readFrom is what a file holds from byte offset on.
func readFrom(path string, offset int64) ([]byte, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	if _, err := f.Seek(offset, io.SeekStart); err != nil {
		return nil, err
	}
	return io.ReadAll(f)
}

// keep is _Cli._keep: the raw output, redacted, becomes the transcript; the raw file goes.
func (x *cliBase) keep(launch *launchResult, transcript string) {
	writeRedacted(launch.raw, transcript)
	if err := os.Remove(launch.raw); err != nil && !errors.Is(err, fs.ErrNotExist) {
		herr.Raise(err)
	}
}

// prompt is the system text, a note on the CLI, then the task: for a CLI with no flag for an
// appended system prompt.
func (x *cliBase) prompt(request *RunRequest) string {
	return py.Rstrip(request.SystemAppend) + "\n\n" + strings.ReplaceAll(CLINote, "{cli}", x.cli) +
		"\n\n---\n\n" + request.Prompt
}

// Trail is the end of a session, as a short list a later agent can read.
func (x *cliBase) Trail(transcript string) string {
	var entries []string
	for _, line := range py.Splitlines(Read(transcript)) {
		if py.Strip(line) != "" {
			entries = append(entries, Clip(line, 300))
		}
	}
	return trailText(entries)
}

// writeText is Path.write_text(text, encoding="utf-8").
func writeText(path, text string) {
	herr.Must(os.WriteFile(path, []byte(text), 0o666))
}
