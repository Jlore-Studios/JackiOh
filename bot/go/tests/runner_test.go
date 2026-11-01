// The claude CLI backend, driven against a fake `claude` executable.
//
// bot/tests/test_runner.py, ported (#467). The Python's fake `claude` is a Python
// script; here it is this test binary run again as a helper process (runnerFakeClaude), behind a
// two-line sh script, so the suite needs no Python.
package tests

import (
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

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// runnerFakeSwitch is the environment variable that makes this test binary the fake `claude`.
const runnerFakeSwitch = "JACKIOH_TEST_FAKE_CLAUDE"

// The fake `claude` runs before any test: this binary, started by the script runnerFakeBin
// writes, is the fake and nothing else.
func init() {
	if os.Getenv(runnerFakeSwitch) == "1" {
		runnerFakeClaude()
		os.Exit(0)
	}
}

// runnerFakeClaude is FAKE_CLAUDE: it reads the prompt on stdin, dumps its environment, argv
// and prompt to FAKE_ENV_DUMP, and answers in Claude Code's stream-json as FAKE_MODE says.
func runnerFakeClaude() {
	data, _ := io.ReadAll(os.Stdin)
	prompt := string(data)
	mode, ok := os.LookupEnv("FAKE_MODE")
	if !ok {
		mode = "ok"
	}
	keys := []any{}
	names := []string{}
	for _, kv := range os.Environ() {
		name, _, _ := strings.Cut(kv, "=")
		names = append(names, name)
	}
	slices.Sort(names)
	for _, name := range names {
		keys = append(keys, name)
	}
	dump := pyjson.New("keys", keys, "argv", pyjson.ListOf(os.Args[1:]), "prompt", prompt)
	if err := os.WriteFile(os.Getenv("FAKE_ENV_DUMP"), []byte(pyjson.Dumps(dump)), 0o666); err != nil {
		panic(err)
	}
	emit := func(obj *pyjson.Object) { fmt.Println(pyjson.Dumps(obj)) } // os.Stdout is unbuffered
	emit(pyjson.New("type", "system", "subtype", "init"))
	if mode == "sleep" {
		time.Sleep(30 * time.Second)
	}
	if mode == "climb" {
		for _, share := range []float64{0.3, 0.6} {
			emit(pyjson.New("type", "rate_limit_event", "rate_limit_info", pyjson.New("status", "allowed",
				"unifiedWindows", pyjson.New("five_hour", pyjson.New("utilization", share,
					"resetsAt", 1790710800)))))
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
		if err := os.WriteFile(os.Getenv("FAKE_ENV_DUMP")+".pid", []byte(strconv.Itoa(child.Process.Pid)), 0o666); err != nil {
			panic(err)
		}
	}
	status := "allowed"
	five := 0.25
	if mode == "limit" {
		status, five = "rejected", 1.0
	}
	emit(pyjson.New("type", "rate_limit_event", "rate_limit_info", pyjson.New("status", status,
		"resetsAt", 1790712000, "unifiedWindows", pyjson.New(
			"five_hour", pyjson.New("utilization", five, "resetsAt", 1790710800),
			"seven_day", pyjson.New("utilization", 0.5, "resetsAt", 1790712000)))))
	if mode == "limit" {
		emit(pyjson.New("type", "result", "is_error", true, "subtype", "success",
			"result", "You've hit your limit", "num_turns", 1))
		os.Exit(1)
	}
	emit(pyjson.New("type", "result", "is_error", false, "result", "done: "+py.SliceTo(prompt, 20), "num_turns", 7))
}

// runnerFakeBin writes the fake `claude` to `path`: a script that runs this test binary as the
// fake. It starts the binary from the repository root, where the harness's packages find
// `.harness/` as they load; the fake itself never reads its working directory.
func runnerFakeBin(t *testing.T, path string) {
	t.Helper()
	self, err := os.Executable()
	if err != nil {
		t.Fatal(err)
	}
	script := "#!/bin/sh\ncd " + runnerQuote(testsupport.Root) + " && " + runnerFakeSwitch + "=1 exec " +
		runnerQuote(self) + " \"$@\"\n"
	if err := os.WriteFile(path, []byte(script), 0o755); err != nil {
		t.Fatal(err)
	}
}

// runnerQuote is shlex.quote.
func runnerQuote(s string) string { return "'" + strings.ReplaceAll(s, "'", `'"'"'`) + "'" }

type runnerWorld struct {
	t    *testing.T
	tmp  string
	bin  string
	dump string
}

func newRunnerWorld(t *testing.T) *runnerWorld {
	w := &runnerWorld{t: t, tmp: t.TempDir()}
	w.bin = filepath.Join(w.tmp, "claude")
	runnerFakeBin(t, w.bin)
	w.dump = filepath.Join(w.tmp, "env.json")
	return w
}

func (w *runnerWorld) request(timeout ...int) *runner.RunRequest {
	seconds := 60
	if len(timeout) > 0 {
		seconds = timeout[0]
	}
	return &runner.RunRequest{
		Role: "build", Prompt: "build the thing please", Cwd: w.tmp,
		SystemAppend: "system text", AllowedTools: []string{"Bash", "Read"},
		DisallowedTools: []string{"WebFetch"}, MaxTurns: 42, TimeoutS: seconds, Model: "opus",
		Effort: "xhigh", Transcript: filepath.Join(w.tmp, "t", "01-build.jsonl"),
	}
}

// runFake is run_fake(mode, timeout=60), its environment patched as mock.patch.dict does.
func (w *runnerWorld) runFake(mode string, timeout ...int) *runner.RunResult {
	for k, v := range map[string]string{"FAKE_MODE": mode, "FAKE_ENV_DUMP": w.dump,
		"BOT_GITHUB_TOKEN": "ghp_" + strings.Repeat("x", 36), "GITHUB_TOKEN": "actions-token-value",
		"CLAUDE_CODE_OAUTH_TOKEN": "oauth-token-value", "ANTHROPIC_API_KEY": "sk-ant-nope"} {
		w.t.Setenv(k, v)
	}
	return runner.NewClaudeCli(w.bin, nil, nil).Run(w.request(timeout...))
}

// The usage the CLI streams is watched during the call: the call stops once the reading
// crosses the cap, not up to `call_timeout_minutes` later.
func TestRunner_ACallPastItsCapIsStoppedWhileItRuns(t *testing.T) {
	w := newRunnerWorld(t)
	seen := []float64{}
	stop := func(usage *pyjson.Object) string {
		share := usage.Obj("five_hour").Float("utilization")
		seen = append(seen, share)
		if share >= 0.5 {
			return fmt.Sprintf("5-hour usage is %.0f%%, at or over its 50%% cap", share*100)
		}
		return ""
	}
	for k, v := range map[string]string{"FAKE_MODE": "climb", "FAKE_ENV_DUMP": w.dump,
		"CLAUDE_CODE_OAUTH_TOKEN": "oauth-token-value"} {
		t.Setenv(k, v)
	}
	request := w.request()
	request.UsageStop = stop
	old := runner.WatchSeconds
	runner.WatchSeconds = 0.1
	t.Cleanup(func() { runner.WatchSeconds = old })
	result := runner.NewClaudeCli(w.bin, nil, nil).Run(request)
	assertFalse(t, result.OK)
	assertEqual(t, result.Extra.Get("usage_stop"), "5-hour usage is 60%, at or over its 50% cap")
	assertIn(t, "stopped mid-call", result.Error)
	assertTrue(t, result.DurationS < 20, result.DurationS) // the fake would sleep 30 seconds more
	assertTrue(t, len(seen) > 0)
	if len(seen) > 0 {
		assertEqual(t, seen[len(seen)-1], 0.6)
	}
	assertEqual(t, dig(t, result.Usage, "five_hour", "utilization"), 0.6)
	assertFalse(t, result.RateLimited())
}

func TestRunner_Argv(t *testing.T) {
	w := newRunnerWorld(t)
	argv := runner.NewClaudeCli("claude", nil, nil).Argv(w.request(), "/tmp/s.md")
	joined := strings.Join(argv, " ")
	for _, part := range []string{"--print", "--output-format stream-json", "--model opus", "--effort xhigh",
		"--max-turns 42", "--permission-mode acceptEdits", "--allowed-tools Bash,Read",
		"--disallowed-tools WebFetch", "--append-system-prompt-file /tmp/s.md",
		"--strict-mcp-config"} {
		assertIn(t, part, joined)
	}
	assertNotIn(t, "build the thing", joined) // the prompt goes on stdin
}

func TestRunner_SuccessAndEnvironment(t *testing.T) {
	w := newRunnerWorld(t)
	result := w.runFake("ok")
	assertTrue(t, result.OK, result.Error)
	assertTrue(t, result.Turns != nil && *result.Turns == 7, result.Turns)
	assertEqual(t, result.Text, "done: build the thing plea")
	assertAlmostEqual(t, dig(t, result.Usage, "five_hour", "utilization"), 0.25)
	assertFalse(t, result.RateLimited())
	dumped := loadJSON(t, w.dump)
	keys := dumped.List("keys")
	assertIn(t, "CLAUDE_CODE_OAUTH_TOKEN", keys)
	assertIn(t, "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", keys)
	for _, secret := range []string{"BOT_GITHUB_TOKEN", "GITHUB_TOKEN", "ANTHROPIC_API_KEY"} {
		assertNotIn(t, secret, keys)
	}
	assertEqual(t, dumped.Get("prompt"), "build the thing please")
	transcript := readText(t, filepath.Join(w.tmp, "t", "01-build.jsonl"))
	assertIn(t, `"type": "result"`, transcript)
	assertFalse(t, exists(filepath.Join(w.tmp, "t", "01-build.raw")))
}

func TestRunner_RefusalIsARateLimitWithItsReset(t *testing.T) {
	w := newRunnerWorld(t)
	result := w.runFake("limit")
	assertFalse(t, result.OK)
	assertTrue(t, result.RateLimited())
	assertEqual(t, result.ResetAt, "2026-09-29T20:00:00Z")
}

func TestRunner_TimeoutKillsTheProcess(t *testing.T) {
	w := newRunnerWorld(t)
	result := w.runFake("sleep", 2)
	assertTrue(t, result.TimedOut)
	assertFalse(t, result.OK)
}

func TestRunner_WhatTheCallLeavesRunningIsKilledAndNothingElse(t *testing.T) {
	w := newRunnerWorld(t)
	bystander := exec.Command("sleep", "300") // another program of the same user
	if err := bystander.Start(); err != nil {
		t.Fatal(err)
	}
	defer func() {
		bystander.Process.Kill()
		bystander.Wait()
	}()
	result := w.runFake("daemon")
	assertTrue(t, result.OK, result.Error)
	pid, err := strconv.Atoi(strings.TrimSpace(readText(t, w.dump+".pid")))
	if err != nil {
		t.Fatal(err)
	}
	// alive is `Path(f"/proc/{pid}").exists() and "Z" not in its stat's third field`; a stat
	// gone between the two reads is a process gone.
	alive := func() bool {
		stat, err := os.ReadFile(fmt.Sprintf("/proc/%d/stat", pid))
		if err != nil {
			return false
		}
		fields := strings.Fields(string(stat))
		return len(fields) < 3 || !strings.Contains(fields[2], "Z")
	}
	for i := 0; i < 50 && alive(); i++ {
		time.Sleep(50 * time.Millisecond)
	}
	assertFalse(t, alive(), "the daemon the call started is still running")
	// bystander.poll() is None: it has not exited.
	var status syscall.WaitStatus
	reaped, _ := syscall.Wait4(bystander.Process.Pid, &status, syscall.WNOHANG, nil)
	assertEqual(t, reaped, 0, "an unrelated process was killed")
}

func TestRunner_AnAuthFailureIsInfrastructureNotARateLimit(t *testing.T) {
	newRunnerWorld(t) // setUp
	failed := func(text string) *runner.RunResult {
		result := runner.NewRunResult(false, "")
		result.ExitCode, result.Error = 1, text
		return result
	}
	result := failed("Invalid API key · Please run /login")
	assertTrue(t, result.Infra())
	assertFalse(t, result.RateLimited())
	assertFalse(t, failed("max turns reached").Infra())
	// Claude Code with a revoked or mistyped token (CLAUDE_CODE_OAUTH_TOKEN_3, 2026-10-03): it
	// ran a whole revision's checks and a review on it before this counted as a login.
	revoked := "Failed to authenticate. API Error: 401 OAuth access token is invalid."
	assertTrue(t, failed(revoked).Infra())
	assertFalse(t, failed(revoked).RateLimited())
}

func TestRunner_MissingBinary(t *testing.T) {
	w := newRunnerWorld(t)
	result := runner.NewClaudeCli(filepath.Join(w.tmp, "nope"), nil, nil).Run(w.request())
	assertFalse(t, result.OK)
	assertEqual(t, result.ExitCode, 127)
}

func TestRunner_ParseStreamIgnoresNoise(t *testing.T) {
	newRunnerWorld(t) // setUp
	lines := []string{"not json", "", pyjson.Dumps(pyjson.New("type", "result", "result", "x")), "[1,2]"}
	result, usage := runner.ParseStream(lines)
	assertEqual(t, dig(t, result, "result"), "x")
	assertTrue(t, usage == nil, "usage is None")
}

func TestRunner_OverageIsNotARefusal(t *testing.T) {
	newRunnerWorld(t) // setUp
	usage := runner.UsageFromEvent(pyjson.New("rate_limit_info", pyjson.New("status", "rejected",
		"isUsingOverage", true, "unifiedWindows", pyjson.New())))
	assertEqual(t, dig(t, usage, "status"), "allowed_overage")
}
