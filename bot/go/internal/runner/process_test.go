package runner

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"slices"
	"strconv"
	"strings"
	"syscall"
	"testing"
	"time"
	"unsafe"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// The fake CLIs of bot/tests/test_runner.py and test_backends.py, run for real: the launch, its
// watcher, its timeout and its reaping, and Muse's TUI in a pseudo-terminal. The Python's fakes
// are Python scripts; here each is this test binary run again as a helper process (fakeClaude,
// fakeMuse), behind a two-line sh script, so the tests need no Python.

// fakeSwitch names the fake CLI this test binary is when started by fakeBin's script.
const fakeSwitch = "JACKIOH_RUNNER_FAKE_CLI"

// The fakes run before any test: started by fakeBin's script, this binary is the fake and
// nothing else.
func init() {
	switch os.Getenv(fakeSwitch) {
	case "claude":
		fakeClaude()
		os.Exit(0)
	case "muse":
		fakeMuse()
		os.Exit(0)
	}
}

// fakeEnv is os.environ.get(name, default).
func fakeEnv(name, def string) string {
	if v, ok := os.LookupEnv(name); ok {
		return v
	}
	return def
}

// fakeClaude is the fake `claude`: the prompt on stdin, its stream-json on stdout as FAKE_MODE
// says, a line on stderr.
func fakeClaude() {
	data, _ := io.ReadAll(os.Stdin)
	prompt := string(data)
	mode := fakeEnv("FAKE_MODE", "ok")
	emit := func(obj *pyjson.Object) { fmt.Println(pyjson.Dumps(obj)) } // os.Stdout is unbuffered
	emit(pyjson.New("type", "system", "subtype", "init"))
	if mode == "sleep" {
		time.Sleep(30 * time.Second)
	}
	if mode == "climb" {
		for _, share := range []float64{0.3, 0.6} {
			emit(pyjson.New("type", "rate_limit_event", "rate_limit_info", pyjson.New("status", "allowed",
				"unifiedWindows", pyjson.New("five_hour", pyjson.New("utilization", share, "resetsAt", 1790710800)))))
			time.Sleep(400 * time.Millisecond)
		}
		time.Sleep(30 * time.Second)
	}
	if mode == "daemon" {
		child := exec.Command("setsid", "sleep", "300")
		child.SysProcAttr = &syscall.SysProcAttr{Setsid: true} // start_new_session=True
		if err := child.Start(); err != nil {
			panic(err)
		}
		if err := os.WriteFile(os.Getenv("FAKE_PID"), []byte(strconv.Itoa(child.Process.Pid)), 0o666); err != nil {
			panic(err)
		}
	}
	fmt.Fprintln(os.Stderr, "to stderr")
	emit(pyjson.New("type", "result", "is_error", false, "result", "done: "+py.SliceTo(prompt, 20), "num_turns", 7))
}

// fakeMuse is the fake `muse` TUI: in a raw terminal it asks where the cursor is, asks once to
// trust its workspace, draws its prompt and, typed `/usage`, its usage panel.
func fakeMuse() {
	mode := fakeEnv("FAKE_MODE", "ok")
	argv := os.Args[1:]
	workspace := argv[slices.Index(argv, "--workspace")+1]
	draw := func(text string) { os.Stdout.Write([]byte(text)) }
	readByte := func() byte {
		var b [1]byte
		if n, err := os.Stdin.Read(b[:]); n == 0 || err != nil {
			os.Exit(1) // the terminal is gone
		}
		return b[0]
	}
	answer := func() []byte {
		var got []byte
		for !bytes.HasSuffix(got, []byte("\r")) {
			got = append(got, readByte())
		}
		log, err := os.OpenFile(filepath.Join(workspace, "typed.log"), os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o666)
		if err != nil {
			panic(err)
		}
		log.Write(append(bytes.TrimSpace(got), '\n'))
		log.Close()
		return bytes.TrimSpace(got)
	}
	fakeSetRaw(0)
	draw("\x1b[6n")
	var where []byte
	for !bytes.HasSuffix(where, []byte("R")) {
		where = append(where, readByte())
	}
	if _, err := os.Stat(filepath.Join(workspace, ".trusted")); err != nil {
		draw("Do you trust this workspace?\r\n> 1 Trust and continue\r\n  2 Quit\r\n")
		if string(answer()) != "1" {
			os.Exit(3)
		}
		if err := os.WriteFile(filepath.Join(workspace, ".trusted"), nil, 0o666); err != nil {
			panic(err)
		}
	}
	draw("\x1b7 Muse Code 1.4.2\x1b8\r\n❯ \r\n")
	if string(answer()) == "/usage" {
		week := "35"
		if mode == "spent" {
			week = "97"
		}
		draw("│ Session usage │\r\n" +
			"│ Current 5% used · Resets at 6:42 PM │\r\n" +
			"│ Weekly " + week + "% used · Resets Oct 5 at 12:00 AM │\r\n")
	}
	time.Sleep(30 * time.Second)
}

// fakeSetRaw is tty.setraw(fd): the terminal in raw mode, set once its queued input is flushed
// (TCSAFLUSH, whose ioctl is TCSETSF: TCSETS + 2 on every Linux).
func fakeSetRaw(fd uintptr) {
	var mode syscall.Termios
	if _, _, errno := syscall.Syscall(syscall.SYS_IOCTL, fd, syscall.TCGETS, uintptr(unsafe.Pointer(&mode))); errno != 0 {
		panic(errno)
	}
	mode.Iflag &^= syscall.BRKINT | syscall.ICRNL | syscall.INPCK | syscall.ISTRIP | syscall.IXON
	mode.Oflag &^= syscall.OPOST
	mode.Cflag &^= syscall.CSIZE | syscall.PARENB
	mode.Cflag |= syscall.CS8
	mode.Lflag &^= syscall.ECHO | syscall.ICANON | syscall.IEXTEN | syscall.ISIG
	mode.Cc[syscall.VMIN] = 1
	mode.Cc[syscall.VTIME] = 0
	if _, _, errno := syscall.Syscall(syscall.SYS_IOCTL, fd, syscall.TCSETS+2, uintptr(unsafe.Pointer(&mode))); errno != 0 {
		panic(errno)
	}
}

// fakeBin writes the fake CLI `which` to a new `name`: a script that runs this test binary as
// it. It starts the binary from the repository root, where the harness's packages find
// `.harness/` as they load; neither fake reads its working directory.
func fakeBin(t *testing.T, name, which string) string {
	t.Helper()
	self, err := os.Executable()
	if err != nil {
		t.Fatal(err)
	}
	quote := func(s string) string { return "'" + strings.ReplaceAll(s, "'", `'"'"'`) + "'" }
	path := filepath.Join(t.TempDir(), name)
	script := "#!/bin/sh\ncd " + quote(config.RepoRoot) + " && " + fakeSwitch + "=" + which + " exec " +
		quote(self) + " \"$@\"\n"
	if err := os.WriteFile(path, []byte(script), 0o755); err != nil {
		t.Fatal(err)
	}
	return path
}

func launchFake(t *testing.T, mode string, timeout int, stop func(*pyjson.Object) string) (*launchResult, *RunResult, string) {
	t.Helper()
	bin := fakeBin(t, "claude", "claude")
	dir := t.TempDir()
	x := NewClaudeCli(bin, nil, nil)
	request := &RunRequest{Role: "build", Cwd: dir, TimeoutS: timeout, UsageStop: stop,
		Transcript: filepath.Join(dir, "t", "01-build.jsonl")}
	_, raw := x.paths(request)
	env := map[string]string{"PATH": os.Getenv("PATH"), "FAKE_MODE": mode,
		"FAKE_PID": filepath.Join(dir, "pid")}
	prompt := "build the thing please"
	launch, failed := x.launch([]string{bin}, request, env, &prompt, raw)
	return launch, failed, dir
}

func TestLaunchRunsTheCLI(t *testing.T) {
	launch, failed, _ := launchFake(t, "ok", 60, nil)
	if failed != nil {
		t.Fatalf("failed: %+v", failed)
	}
	var stream streamParser
	if err := eachLine(launch.raw, stream.add); err != nil {
		t.Fatal(err)
	}
	if launch.code != 0 || launch.timedOut || Text(stream.result) != "done: build the thing plea" {
		t.Errorf("launch %+v, result %v", launch, pyjson.Dumps(stream.result))
	}
	if launch.stderr != "to stderr\n" {
		t.Errorf("stderr %q", launch.stderr)
	}
}

func TestLaunchStopsACallPastItsCap(t *testing.T) {
	old := WatchSeconds
	WatchSeconds = 0.1
	t.Cleanup(func() { WatchSeconds = old })
	var seen []float64
	stop := func(usage *pyjson.Object) string {
		share := usage.Obj("five_hour").Float("utilization")
		seen = append(seen, share)
		if share >= 0.5 {
			return "5-hour usage is " + strconv.Itoa(int(share*100)) + "%, at or over its 50% cap"
		}
		return ""
	}
	launch, failed, _ := launchFake(t, "climb", 60, stop)
	if failed != nil {
		t.Fatalf("failed: %+v", failed)
	}
	if launch.stopped != "5-hour usage is 60%, at or over its 50% cap" || launch.elapsed > 20 {
		t.Errorf("launch %+v", launch)
	}
	if launch.code != -15 || launch.timedOut {
		t.Errorf("code %d (SIGTERM is -15), timed out %v", launch.code, launch.timedOut)
	}
	if len(seen) == 0 || seen[len(seen)-1] != 0.6 {
		t.Errorf("seen %v", seen)
	}
}

func TestLaunchTimesOut(t *testing.T) {
	launch, failed, _ := launchFake(t, "sleep", 1, nil)
	if failed != nil {
		t.Fatalf("failed: %+v", failed)
	}
	if !launch.timedOut || launch.elapsed > 20 {
		t.Errorf("launch %+v", launch)
	}
}

func TestLaunchReapsWhatTheCallLeftAndNothingElse(t *testing.T) {
	if _, err := exec.LookPath("setsid"); err != nil {
		t.Skip("no setsid")
	}
	bystander := exec.Command("sleep", "300")
	if err := bystander.Start(); err != nil {
		t.Fatal(err)
	}
	defer func() { bystander.Process.Kill(); bystander.Wait() }()
	launch, failed, dir := launchFake(t, "daemon", 60, nil)
	if failed != nil || launch.code != 0 {
		t.Fatalf("failed: %+v %+v", failed, launch)
	}
	text, err := os.ReadFile(filepath.Join(dir, "pid"))
	if err != nil {
		t.Fatal(err)
	}
	pid, _ := strconv.Atoi(strings.TrimSpace(string(text)))
	alive := func(pid int) bool {
		f, ok := statFields(pid)
		return ok && len(f) > 0 && f[0] != "Z"
	}
	for i := 0; i < 50 && alive(pid); i++ {
		time.Sleep(50 * time.Millisecond)
	}
	if alive(pid) {
		t.Error("the daemon the call started is still running")
	}
	if !alive(bystander.Process.Pid) {
		t.Error("an unrelated process was killed")
	}
}

func TestLaunchWithoutTheBinary(t *testing.T) {
	dir := t.TempDir()
	x := NewClaudeCli(filepath.Join(dir, "nope"), nil, nil)
	request := &RunRequest{Role: "build", Cwd: dir, TimeoutS: 5, Transcript: filepath.Join(dir, "t.jsonl")}
	_, raw := x.paths(request)
	_, failed := x.launch([]string{x.Binary}, request, map[string]string{}, nil, raw)
	if failed == nil || failed.ExitCode != 127 || failed.Error != x.Binary+" not found" || !failed.Infra() {
		t.Errorf("got %+v", failed)
	}
	// A bare name not on the child's PATH is as missing (Popen searches the child's PATH).
	_, failed = x.launch([]string{"claude-nowhere"}, request, map[string]string{"PATH": dir}, nil, raw)
	if failed == nil || failed.ExitCode != 127 {
		t.Errorf("got %+v", failed)
	}
}

func TestMuseScreenReadsTheUsagePanel(t *testing.T) {
	bin := fakeBin(t, "muse", "muse")
	workspace := filepath.Join(t.TempDir(), "usage")
	env := map[string]string{"PATH": os.Getenv("PATH")}
	started := time.Now()
	screen := MuseScreen(bin, env, workspace, 20, 0.3)
	if !strings.Contains(screen, "Weekly 35% used") {
		t.Fatalf("screen %q", screen)
	}
	if time.Since(started) > 15*time.Second {
		t.Errorf("took %v: the TUI is ended once the week is drawn", time.Since(started))
	}
	now := time.Date(2026, 10, 4, 15, 30, 0, 0, time.UTC)
	same(t, "usage", ParseMuseUsage(screen, now), pyjson.New(
		"five_hour", pyjson.New("utilization", 0.05, "resets_at", "2026-10-04T18:42:00Z"),
		"seven_day", pyjson.New("utilization", 0.35, "resets_at", "2026-10-05T00:00:00Z"),
		"status", "allowed"))
	env["FAKE_MODE"] = "spent"
	if again := MuseScreen(bin, env, workspace, 20, 0.3); !strings.Contains(again, "Weekly 97% used") {
		t.Fatalf("again %q", again)
	}
	typed, _ := os.ReadFile(filepath.Join(workspace, "typed.log"))
	if got := strings.Fields(string(typed)); strings.Join(got, " ") != "1 /usage /usage" {
		t.Errorf("typed %v: trusted once, then only /usage", got)
	}
	if MuseScreen(filepath.Join(workspace, "no-such-muse"), env, workspace, 5, 0.3) != "" {
		t.Error("no TUI, no panel")
	}
}
