package runner

import (
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// The fake CLIs of bot/tests/test_runner.py and test_backends.py, run for real: the launch, its
// watcher, its timeout and its reaping, and Muse's TUI in a pseudo-terminal.

const fakeClaude = `#!%s
import json, os, sys, time
prompt = sys.stdin.read()
mode = os.environ.get("FAKE_MODE", "ok")
def emit(obj):
    print(json.dumps(obj), flush=True)
emit({"type": "system", "subtype": "init"})
if mode == "sleep":
    time.sleep(30)
if mode == "climb":
    for share in (0.3, 0.6):
        emit({"type": "rate_limit_event", "rate_limit_info": {"status": "allowed",
              "unifiedWindows": {"five_hour": {"utilization": share, "resetsAt": 1790710800}}}})
        time.sleep(0.4)
    time.sleep(30)
if mode == "daemon":
    import subprocess
    child = subprocess.Popen(["setsid", "sleep", "300"], start_new_session=True)
    open(os.environ["FAKE_PID"], "w").write(str(child.pid))
print("to stderr", file=sys.stderr)
emit({"type": "result", "is_error": False, "result": "done: " + prompt[:20], "num_turns": 7})
`

func fakeBin(t *testing.T, name, script string) string {
	t.Helper()
	python, err := exec.LookPath("python3")
	if err != nil {
		t.Skip("no python3 for the fake CLI")
	}
	path := filepath.Join(t.TempDir(), name)
	if err := os.WriteFile(path, []byte(strings.Replace(script, "%s", python, 1)), 0o755); err != nil {
		t.Fatal(err)
	}
	return path
}

func launchFake(t *testing.T, mode string, timeout int, stop func(*pyjson.Object) string) (*launchResult, *RunResult, string) {
	t.Helper()
	bin := fakeBin(t, "claude", fakeClaude)
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

const fakeMuse = `#!%s
import json, os, sys, time, tty
mode = os.environ.get("FAKE_MODE", "ok")
argv = sys.argv[1:]
workspace = argv[argv.index("--workspace") + 1]
def draw(text):
    os.write(1, text.encode("utf-8"))
def answer():
    got = b""
    while not got.endswith(b"\r"):
        got += os.read(0, 1)
    with open(os.path.join(workspace, "typed.log"), "ab") as log:
        log.write(got.strip() + b"\n")
    return got.strip()
tty.setraw(0)
draw("\x1b[6n")
where = b""
while not where.endswith(b"R"):
    where += os.read(0, 1)
if not os.path.exists(os.path.join(workspace, ".trusted")):
    draw("Do you trust this workspace?\r\n> 1 Trust and continue\r\n  2 Quit\r\n")
    if answer() != b"1":
        sys.exit(3)
    open(os.path.join(workspace, ".trusted"), "w").close()
draw("\x1b7 Muse Code 1.4.2\x1b8\r\n❯ \r\n")
if answer() == b"/usage":
    week = "97" if mode == "spent" else "35"
    draw("│ Session usage │\r\n"
         "│ Current 5% used · Resets at 6:42 PM │\r\n"
         "│ Weekly " + week + "% used · Resets Oct 5 at 12:00 AM │\r\n")
time.sleep(30)
`

func TestMuseScreenReadsTheUsagePanel(t *testing.T) {
	bin := fakeBin(t, "muse", fakeMuse)
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
