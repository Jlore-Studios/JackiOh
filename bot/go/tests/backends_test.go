package tests

// bot/tests/test_backends.py: the codex, agy and muse backends, each driven against a fake
// executable.
//
// The Python fakes are Python scripts. Here each is this test binary run again as a helper
// process: the fake's file is a POSIX sh script that names the fake in BACKENDS_FAKE_CLI and execs
// the binary, whose init (below) plays that fake and exits before any test runs. It first goes back
// to the directory the tests run from, since the packages it imports read the repository from the
// working directory when they load, and a fake is started in a scratch directory.

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"slices"
	"strings"
	"syscall"
	"testing"
	"time"
	"unsafe"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/logins"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	ts "github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// backendsFakeEnv names the fake a helper process plays.
const backendsFakeEnv = "BACKENDS_FAKE_CLI"

func init() {
	name, ok := os.LookupEnv(backendsFakeEnv)
	if !ok {
		return
	}
	fake, known := map[string]func() int{
		"codex":         backendsFakeCodex,
		"agy":           backendsFakeAgy,
		"muse":          backendsFakeMuse,
		"claude-tokens": tokensFakeClaude,
	}[name]
	if !known {
		fmt.Fprintf(os.Stderr, "no fake CLI %q\n", name)
		os.Exit(2)
	}
	os.Exit(fake())
}

// backendsOtherSecrets is OTHER_SECRETS: every provider secret is in this process's environment;
// a child must see only its own login.
var backendsOtherSecrets = map[string]string{
	"CLAUDE_CODE_OAUTH_TOKEN": "claude-token-value", "CODEX_AUTH_JSON": `{"x": 1}`,
	"MUSE_AUTH":        "muse-secret-value",
	"BOT_GITHUB_TOKEN": "ghp_" + strings.Repeat("x", 36), "META_API_KEY": "meta-key-value",
	"HARNESS_PROVIDER_SECRET": "the-one-secret-value",
}

// ---------------------------------------------------------------------------- the fakes

// PRELUDE's mode.
func backendsMode() string {
	if mode, ok := os.LookupEnv("FAKE_MODE"); ok {
		return mode
	}
	return "ok"
}

// PRELUDE's dump: the fake's argv, environment and prompt, for the test to read.
func backendsDump(prompt string) {
	env := pyjson.New()
	for _, kv := range os.Environ() {
		k, v, _ := strings.Cut(kv, "=")
		env.Set(k, v)
	}
	text := pyjson.Dumps(pyjson.New("argv", pyjson.ListOf(os.Args[1:]), "env", env, "prompt", prompt))
	if err := os.WriteFile(os.Getenv("FAKE_DUMP"), []byte(text), 0o666); err != nil {
		panic(err)
	}
}

// PRELUDE's emit: one JSON line on stdout.
func backendsEmit(obj *pyjson.Object) {
	os.Stdout.WriteString(pyjson.Dumps(obj) + "\n")
}

// backendsStdin is sys.stdin.read().
func backendsStdin() string {
	data, err := io.ReadAll(os.Stdin)
	if err != nil {
		panic(err)
	}
	return string(data)
}

// backendsAfter is argv[argv.index(flag) + 1].
func backendsAfter(argv []string, flag string) string {
	return argv[slices.Index(argv, flag)+1]
}

// backendsFakeCodex is FAKE_CODEX.
func backendsFakeCodex() int {
	mode := backendsMode()
	prompt := backendsStdin()
	backendsDump(prompt)
	argv := os.Args[1:]
	last := backendsAfter(argv, "--output-last-message")
	home := os.Getenv("CODEX_HOME")
	if home == "" {
		home = filepath.Join(os.Getenv("HOME"), ".codex")
	}
	sessions := filepath.Join(home, "sessions", "2026", "10", "02")
	if err := os.MkdirAll(sessions, 0o777); err != nil {
		panic(err)
	}
	secondary := 61.0
	if mode == "limit" {
		secondary = 100.0
	}
	line := pyjson.Dumps(pyjson.New("type", "event_msg", "payload", pyjson.New("type", "token_count",
		"rate_limits", pyjson.New(
			"primary", pyjson.New("used_percent", 42.0, "window_minutes", 300, "resets_at", 1790710800),
			"secondary", pyjson.New("used_percent", secondary, "window_minutes", 10080,
				"resets_at", 1791190800))))) + "\n"
	if err := os.WriteFile(filepath.Join(sessions, "rollout-1.jsonl"), []byte(line), 0o666); err != nil {
		panic(err)
	}
	backendsEmit(pyjson.New("type", "thread.started", "thread_id", "t1"))
	backendsEmit(pyjson.New("type", "item.completed", "item", pyjson.New("type", "command_execution",
		"command", "pnpm test", "exit_code", 0)))
	if mode == "limit" {
		backendsEmit(pyjson.New("type", "turn.failed", "error", pyjson.New("message",
			"You’ve hit your usage limit. Visit "+
				"https://chatgpt.com/settings/usage to purchase more credits or try again at 3:45 PM.")))
		return 1
	}
	if mode == "auth" {
		backendsEmit(pyjson.New("type", "turn.failed", "error", pyjson.New("message",
			"Your access token could not be "+
				"refreshed because your refresh token was already used. Please log out and sign in again.")))
		return 1
	}
	backendsEmit(pyjson.New("type", "item.completed", "item", pyjson.New("type", "agent_message",
		"text", "working on it")))
	if err := os.WriteFile(last, []byte("<!-- bot: {\"status\": \"done\", \"title\": \"T\"} -->\nreport"),
		0o666); err != nil {
		panic(err)
	}
	backendsEmit(pyjson.New("type", "turn.completed", "usage", pyjson.New("input_tokens", 10,
		"output_tokens", 5)))
	return 0
}

// backendsFakeAgy is FAKE_AGY.
func backendsFakeAgy() int {
	mode := backendsMode()
	if slices.Equal(os.Args[1:], []string{"-p", "/usage"}) { // its quota table: pool, limit, share left, reset
		week := "40"
		if mode == "spent" || mode == "slow" {
			week = "4"
		}
		for _, row := range [][]string{
			{"Gemini Models", "Weekly Limit Remaining", week + "%", "2026-10-09T17:47:52Z"},
			{"Gemini Models", "Five Hour Limit Remaining", "33%", "2026-10-04T16:18:35Z"},
			{"Claude and GPT models", "Weekly Limit Remaining", "100%", "2026-10-11T15:04:01Z"},
			{"Claude and GPT models", "Five Hour Limit Remaining", "100%", "2026-10-04T20:04:01Z"},
		} {
			os.Stdout.WriteString(strings.Join(row, "\t") + "\n")
		}
		return 0
	}
	prompt := backendsStdin()
	backendsDump(prompt)
	if mode == "slow" {
		time.Sleep(30 * time.Second)
	}
	if mode == "auth" {
		fmt.Fprintln(os.Stderr, "Error: authentication required; run agy to sign in.")
		return 1
	}
	step := func(index int, kind, state string, fields ...any) {
		update := pyjson.New("step_index", index, "step_type", kind, "state", state).Update(pyjson.New(fields...))
		backendsEmit(pyjson.New("event", "step_update", "step_update", update))
	}
	backendsEmit(pyjson.New("event", "init", "init", pyjson.New("session_id", "s1",
		"model", "gemini-3.8-flash-high")))
	step(0, "agent_response", "RUNNING", "text_delta", "Let me ")
	step(0, "agent_response", "DONE", "text_delta", "look.")
	step(1, "tool", "DONE", "tool_info", pyjson.New("name", "run_command",
		"parameters", pyjson.New("CommandLine", "pnpm test")))
	if mode == "quota" {
		backendsEmit(pyjson.New("event", "result", "result", pyjson.New("status", "ERROR", "num_turns", 2,
			"error", "RESOURCE_EXHAUSTED: You have exhausted your quota on this model.")))
		return 1
	}
	if mode == "individual" {
		backendsEmit(pyjson.New("event", "result", "result", pyjson.New("status", "ERROR", "num_turns", 1,
			"error", "Individual quota reached. Please upgrade your subscription to increase "+
				"your limits. Resets in 1h44m44s.")))
		return 1
	}
	answer := `<!-- review: {"verdict": "approve", "findings": []} -->`
	step(2, "agent_response", "DONE", "text_delta", answer)
	backendsEmit(pyjson.New("event", "result", "result", pyjson.New("status", "SUCCESS", "response", answer,
		"num_turns", 3, "error", "", "usage", pyjson.New("input_tokens", 10))))
	return 0
}

// backendsSetRaw is tty.setraw(0): no echo, no line editing, no signals, eight-bit bytes as they
// come, one at a time.
func backendsSetRaw(fd int) {
	var mode syscall.Termios
	if _, _, errno := syscall.Syscall(syscall.SYS_IOCTL, uintptr(fd), syscall.TCGETS,
		uintptr(unsafe.Pointer(&mode))); errno != 0 {
		panic(errno)
	}
	mode.Iflag &^= syscall.BRKINT | syscall.ICRNL | syscall.INPCK | syscall.ISTRIP | syscall.IXON
	mode.Oflag &^= syscall.OPOST
	mode.Cflag &^= syscall.CSIZE | syscall.PARENB
	mode.Cflag |= syscall.CS8
	mode.Lflag &^= syscall.ECHO | syscall.ICANON | syscall.IEXTEN | syscall.ISIG
	mode.Cc[syscall.VMIN] = 1
	mode.Cc[syscall.VTIME] = 0
	if _, _, errno := syscall.Syscall(syscall.SYS_IOCTL, uintptr(fd), syscall.TCSETS,
		uintptr(unsafe.Pointer(&mode))); errno != 0 {
		panic(errno)
	}
}

// backendsReadByte is os.read(0, 1).
func backendsReadByte() byte {
	b := make([]byte, 1)
	for {
		n, err := syscall.Read(0, b)
		if err == syscall.EINTR {
			continue
		}
		if err != nil || n == 0 {
			os.Exit(1) // the terminal is gone
		}
		return b[0]
	}
}

// backendsFakeMuse is FAKE_MUSE.
func backendsFakeMuse() int {
	mode := backendsMode()
	argv := os.Args[1:]
	if len(argv) == 0 || argv[0] != "exec" { // the TUI, in a pseudo-terminal: what `/usage` is read from
		workspace := backendsAfter(argv, "--workspace")
		draw := func(text string) { syscall.Write(1, []byte(text)) }
		answer := func() string {
			var got []byte
			for !bytes.HasSuffix(got, []byte("\r")) {
				got = append(got, backendsReadByte())
			}
			typed := bytes.TrimSpace(got)
			log, err := os.OpenFile(filepath.Join(workspace, "typed.log"), os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o666)
			if err != nil {
				panic(err)
			}
			log.Write(append(typed, '\n'))
			log.Close()
			return string(typed)
		}
		backendsSetRaw(0)
		draw("\x1b[6n")
		var where []byte
		for !bytes.HasSuffix(where, []byte("R")) {
			where = append(where, backendsReadByte())
		}
		trusted := filepath.Join(workspace, ".trusted")
		if _, err := os.Stat(trusted); err != nil {
			draw("Do you trust this workspace?\r\n> 1 Trust and continue\r\n  2 Quit\r\n")
			if answer() != "1" {
				return 3
			}
			if err := os.WriteFile(trusted, nil, 0o666); err != nil {
				panic(err)
			}
		}
		draw("\x1b7 Muse Code 1.4.2\x1b8\r\n❯ \r\n")
		if answer() == "/usage" {
			week := "35"
			if mode == "spent" {
				week = "97"
			}
			current := "5"
			if mode == "full" {
				current = "100"
			}
			draw("│ Session usage │\r\n" +
				"│ Subscription · Muse Code High Usage │\r\n" +
				"│ Current " + current + "% used · Resets at 6:42 PM │\r\n" +
				"│ Weekly " + week + "% used · Resets Oct 5 at 12:00 AM │\r\n")
		}
		time.Sleep(30 * time.Second)
		return 0
	}
	text, err := os.ReadFile(backendsAfter(argv, "--prompt-file"))
	if err != nil {
		panic(err)
	}
	backendsDump(string(text))
	fmt.Fprintln(os.Stderr, "step 1: reading")
	if mode == "limit" {
		fmt.Fprintln(os.Stderr, "Error: usage limit reached. Wait for usage to reset at 18:00.")
		return 1
	}
	if mode == "quota" {
		fmt.Fprintln(os.Stderr, "agent loop failed: model failed: API error 429 [request_id=556661e6]: "+
			"Subscription quota exhausted. Your usage window resets at "+
			"2026-10-26T00:00:00Z. (rate_limit_error)")
		return 2
	}
	if mode == "full" || mode == "broken" {
		fmt.Fprintln(os.Stderr, "Error: the request could not be completed.")
		return 1
	}
	fmt.Println(`<!-- bot: {"status": "done", "title": "M"} -->`)
	fmt.Println("report")
	return 0
}

// ---------------------------------------------------------------------------- Base

// backendsShQuote quotes a word for sh.
func backendsShQuote(s string) string {
	return "'" + strings.ReplaceAll(s, "'", `'\''`) + "'"
}

// backendsWorld is Base: a scratch directory, a fake CLI in it, and the committed subscriptions.
type backendsWorld struct {
	t    *testing.T
	tmp  string
	bin  string
	dump string
	pool *providers.Pool
}

// newBackendsWorld is Base.setUp with the fake `fake` written as `name`.
func newBackendsWorld(t *testing.T, fake, name string) *backendsWorld {
	tmp := t.TempDir()
	w := &backendsWorld{t: t, tmp: tmp, bin: filepath.Join(tmp, name), dump: filepath.Join(tmp, "dump.json")}
	exe, err := os.Executable()
	if err != nil {
		t.Fatal(err)
	}
	wd, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	script := fmt.Sprintf("#!/bin/sh\n%s=%s\nexport %s\ncd %s || exit 126\nexec %s \"$@\"\n",
		backendsFakeEnv, backendsShQuote(fake), backendsFakeEnv, backendsShQuote(wd), backendsShQuote(exe))
	if err := os.WriteFile(w.bin, []byte(script), 0o755); err != nil {
		t.Fatal(err)
	}
	w.pool = providers.Load(ts.Root)
	return w
}

// request is Base.request(read_only, max_turns=42).
func (w *backendsWorld) request(readOnly bool) *runner.RunRequest {
	role := "build"
	if readOnly {
		role = "review"
	}
	return &runner.RunRequest{Role: role, Prompt: "THE TASK", Cwd: w.tmp, SystemAppend: "SYSTEM TEXT",
		AllowedTools: []string{}, DisallowedTools: []string{}, MaxTurns: 42, TimeoutS: 60,
		Model: "the-model", Effort: "high", Transcript: filepath.Join(w.tmp, "t", "01.jsonl"),
		ReadOnly: readOnly, ExtraDirs: []string{"/repo/.git"}}
}

// backendsPatchEnv is mock.patch.dict(os.environ, values): the values set until the returned
// func puts the environment back as it was.
func backendsPatchEnv(t *testing.T, values map[string]string) func() {
	type was struct {
		value string
		set   bool
	}
	saved := map[string]was{}
	for k, v := range values {
		old, set := os.LookupEnv(k)
		saved[k] = was{old, set}
		os.Setenv(k, v)
	}
	done := false
	restore := func() {
		if done {
			return
		}
		done = true
		for k, w := range saved {
			if w.set {
				os.Setenv(k, w.value)
			} else {
				os.Unsetenv(k)
			}
		}
	}
	t.Cleanup(restore)
	return restore
}

// fakeEnv is {**OTHER_SECRETS, "FAKE_MODE": mode, "FAKE_DUMP": str(self.dump)}.
func (w *backendsWorld) fakeEnv(mode string) map[string]string {
	env := map[string]string{"FAKE_MODE": mode, "FAKE_DUMP": w.dump}
	for k, v := range backendsOtherSecrets {
		env[k] = v
	}
	return env
}

// runFake is Base.run_fake(backend, mode, read_only).
func (w *backendsWorld) runFake(backend runner.Runner, mode string, readOnly bool) *runner.RunResult {
	restore := backendsPatchEnv(w.t, w.fakeEnv(mode))
	defer restore()
	return backend.Run(w.request(readOnly))
}

// seen is Base.seen(): what the fake dumped.
func (w *backendsWorld) seen() *pyjson.Object {
	return loadsObject(w.t, readText(w.t, w.dump))
}

// seenArgv is seen()["argv"].
func (w *backendsWorld) seenArgv() []string {
	return pyjson.Strings(w.seen().Get("argv"))
}

// assertOnlyItsOwnLogin is Base.assert_only_its_own_login(keep).
func (w *backendsWorld) assertOnlyItsOwnLogin(keep ...string) {
	w.t.Helper()
	env := w.seen().Obj("env")
	for key := range backendsOtherSecrets {
		if !slices.Contains(keep, key) {
			assertNotIn(w.t, key, env)
		}
	}
}

// backendsOK is assertTrue(result.ok, result.error).
func backendsOK(t *testing.T, result *runner.RunResult) {
	t.Helper()
	assertTrue(t, result.OK, result.Error)
}

// backendsUtilization is usage[window]["utilization"].
func backendsUtilization(usage *pyjson.Object, window string) any {
	return usage.Obj(window).Get("utilization")
}

// ---------------------------------------------------------------------------- CodexTests

// backendsCodex is CodexTests.backend().
func (w *backendsWorld) backendsCodex() *runner.CodexCli {
	login := logins.Prepare(ts.SecretLogin(w.pool.Get("gpt"), "CODEX_AUTH_JSON"),
		pyjson.Dumps(loginsCodexAuth()), "", filepath.Join(w.tmp, "home"))
	return runner.NewCodexCli(w.bin, login, nil)
}

func TestBackends_SuccessUsageAndArgv(t *testing.T) {
	w := newBackendsWorld(t, "codex", "codex")
	result := w.runFake(w.backendsCodex(), "ok", false)
	backendsOK(t, result)
	assertTrue(t, strings.HasPrefix(result.Text, "<!-- bot:"))
	assertEqual(t, backendsUtilization(result.Usage, "five_hour"), 0.42)
	assertEqual(t, result.Usage.Obj("seven_day").Get("resets_at"), "2026-10-05T09:00:00Z")
	seen := w.seen()
	argvList := pyjson.Strings(seen.Get("argv"))
	argv := strings.Join(argvList, " ")
	for _, part := range []string{"exec --json", "--sandbox workspace-write",
		"sandbox_workspace_write.network_access=false", "--model the-model",
		`model_reasoning_effort="high"`, "--add-dir /repo/.git"} {
		assertIn(t, part, argv)
	}
	assertEqual(t, argvList[len(argvList)-1], "-")
	prompt := seen.Str("prompt")
	assertTrue(t, strings.HasPrefix(prompt, "SYSTEM TEXT"))
	assertIn(t, "You are running in the codex CLI", prompt)
	assertTrue(t, strings.HasSuffix(prompt, "THE TASK"))
	w.assertOnlyItsOwnLogin()
	assertTrue(t, strings.HasSuffix(seen.Obj("env").Str("CODEX_HOME"), "codex"))
	transcript := readText(t, filepath.Join(w.tmp, "t", "01.jsonl"))
	assertIn(t, "thread.started", transcript)
	assertIn(t, "ran `pnpm test`", w.backendsCodex().Trail(filepath.Join(w.tmp, "t", "01.jsonl")))
}

func TestBackends_AUsageLimitParksItUntilTheFullWindowResets(t *testing.T) {
	w := newBackendsWorld(t, "codex", "codex")
	result := w.runFake(w.backendsCodex(), "limit", false)
	assertFalse(t, result.OK)
	assertTrue(t, result.RateLimited())
	assertEqual(t, result.ResetAt, "2026-10-05T09:00:00Z")
}

func TestBackends_AMachineLoginUsesTheClisOwnHome(t *testing.T) {
	w := newBackendsWorld(t, "codex", "codex")
	login := logins.Prepare(w.pool.Get("gpt"), "", "", filepath.Join(w.tmp, "unused"))
	machineHome := filepath.Join(w.tmp, "machine-home")
	restore := backendsPatchEnv(t, map[string]string{"HOME": machineHome})
	result := w.runFake(runner.NewCodexCli(w.bin, login, nil), "ok", false)
	restore()
	backendsOK(t, result)
	assertNotIn(t, "CODEX_HOME", w.seen().Obj("env"))
	assertEqual(t, w.seen().Obj("env").Get("HOME"), machineHome)
	assertEqual(t, backendsUtilization(result.Usage, "five_hour"), 0.42) // read from ~/.codex
	w.assertOnlyItsOwnLogin()
}

func TestBackends_AUsedRefreshTokenIsInfrastructure(t *testing.T) {
	w := newBackendsWorld(t, "codex", "codex")
	result := w.runFake(w.backendsCodex(), "auth", false)
	assertTrue(t, result.Infra())
	assertFalse(t, result.RateLimited())
}

// ---------------------------------------------------------------------------- AgyTests

// backendsAgy is AgyTests.backend().
func (w *backendsWorld) backendsAgy() *runner.AgyCli {
	return runner.NewAgyCli(w.bin, logins.Prepare(w.pool.Get("agy"), "", "", filepath.Join(w.tmp, "x")), nil)
}

// gemini is AgyTests.gemini(mode, usage_stop).
func (w *backendsWorld) gemini(mode string, usageStop func(*pyjson.Object) string) *runner.RunResult {
	request := w.request(false)
	request.Model, request.UsageStop = "gemini-3.8-flash-high", usageStop
	restore := backendsPatchEnv(w.t, w.fakeEnv(mode))
	defer restore()
	return w.backendsAgy().Run(request)
}

func TestBackends_TheAnswerSuccessAndArgv(t *testing.T) {
	w := newBackendsWorld(t, "agy", "agy")
	backend := w.backendsAgy()
	result := w.runFake(backend, "ok", true)
	backendsOK(t, result)
	assertEqual(t, result.Text, `<!-- review: {"verdict": "approve", "findings": []} -->`)
	assertTrue(t, result.Turns != nil)
	assertEqual(t, *result.Turns, 3)
	seen := w.seen()
	argv := pyjson.Strings(seen.Get("argv"))
	for _, flag := range []string{"--dangerously-skip-permissions", "--disable-slash-commands"} {
		assertIn(t, flag, argv)
	}
	assertEqual(t, backendsAfter(argv, "--output-format"), "stream-json")
	assertEqual(t, backendsAfter(argv, "--model"), "the-model")
	assertEqual(t, backendsAfter(argv, "--effort"), "high")
	assertEqual(t, backendsAfter(argv, "--add-dir"), "/repo/.git")
	assertTrue(t, strings.HasPrefix(seen.Str("prompt"), "SYSTEM TEXT"))
	assertTrue(t, strings.HasSuffix(seen.Str("prompt"), "THE TASK"))
	w.assertOnlyItsOwnLogin()
	trail := backend.Trail(filepath.Join(w.tmp, "t", "01.jsonl"))
	assertIn(t, "said: Let me look.", trail)
	assertIn(t, "run_command: pnpm test", trail)
}

func TestBackends_AMissingLoginIsInfrastructure(t *testing.T) {
	w := newBackendsWorld(t, "agy", "agy")
	result := w.runFake(w.backendsAgy(), "auth", false)
	assertFalse(t, result.OK)
	assertTrue(t, result.Infra())
	assertIn(t, "authentication required", result.Error)
	assertTrue(t, result.Usage == nil) // no reading is tried with a login that failed
}

// agy's stream carries no usage, so its own `/usage` is read after the call: the Gemini pool's
// row for a Gemini model.
func TestBackends_EachCallEndsWithAReadingOfItsUsage(t *testing.T) {
	w := newBackendsWorld(t, "agy", "agy")
	result := w.gemini("ok", nil)
	backendsOK(t, result)
	assertEqual(t, result.Usage.Get("seven_day"), pyjson.New("utilization", 0.6, "resets_at", "2026-10-09T17:47:52Z"))
	assertEqual(t, backendsUtilization(result.Usage, "five_hour"), 0.67)
	assertEqual(t, strings.HasSuffix(w.seen().Str("prompt"), "THE TASK"), true) // the call's, not /usage's
}

// The call streams nothing, so the watcher reads `/usage` every POLL_SECONDS and stops the call
// once `usage_stop` says so.
func TestBackends_ACallPastItsCapStopsAtTheNextReading(t *testing.T) {
	w := newBackendsWorld(t, "agy", "agy")
	stop := func(usage *pyjson.Object) string {
		if usage.Obj("seven_day").Float("utilization") >= 0.95 {
			return "week at 96%"
		}
		return ""
	}
	watch, poll := runner.WatchSeconds, runner.PollSeconds
	runner.WatchSeconds, runner.PollSeconds = 0.1, 0.2
	t.Cleanup(func() { runner.WatchSeconds, runner.PollSeconds = watch, poll })
	result := w.gemini("slow", stop)
	runner.WatchSeconds, runner.PollSeconds = watch, poll
	assertFalse(t, result.OK)
	assertEqual(t, result.Extra.Get("usage_stop"), "week at 96%")
	assertTrue(t, result.DurationS < 20)
	assertEqual(t, backendsUtilization(result.Usage, "seven_day"), 0.96)
}

func TestBackends_ASpentQuotaParksIt(t *testing.T) {
	w := newBackendsWorld(t, "agy", "agy")
	result := w.runFake(w.backendsAgy(), "quota", false)
	assertTrue(t, result.RateLimited())
	assertEqual(t, result.ResetAt, "+PT60M")
	assertIn(t, "exhausted your quota", result.Error)
}

// agy's answer once the plan's allowance is spent. Unread, every run went on to the checks and a
// review that failed the same way, and the next run started minutes later.
func TestBackends_ItsPlansIndividualQuotaParksItUntilTheResetItNames(t *testing.T) {
	w := newBackendsWorld(t, "agy", "agy")
	result := w.runFake(w.backendsAgy(), "individual", false)
	assertTrue(t, result.RateLimited())
	assertFalse(t, result.Infra())
	assertEqual(t, result.ResetAt, "+PT106M") // 1h44m44s, rounded up, plus a minute
}

func TestBackends_ANamedResetIsHowLongItStaysParked(t *testing.T) {
	assertEqual(t, runner.ParkFor("Resets in 1h44m44s."), "+PT106M")
	assertEqual(t, runner.ParkFor("Your limit resets in 3h"), "+PT181M")
	assertEqual(t, runner.ParkFor("resets in 12m"), "+PT13M")
	assertEqual(t, runner.ParkFor("Usage limit reached."), "+PT60M")
	assertEqual(t, runner.ParkFor("Your usage window resets at 2026-10-26T00:00:00Z."), "2026-10-26T00:00:00Z")
	assertEqual(t, runner.ParkFor("resets at 2026-10-25T19:00:00-05:00"), "2026-10-26T00:00:00Z")
	at := time.Date(2026, 10, 3, 23, 33, 0, 0, time.UTC)
	st := pyjson.New("providers", pyjson.New())
	providers.NoteUsage(st, "agy", nil, "+PT106M", at, 0, nil, nil)
	assertEqual(t, st.Obj("providers").Obj("agy").Get("refused_until"), "2026-10-04T01:19:00Z")
	providers.NoteUsage(st, "agy", nil, "+PT1H30M", at, 0, nil, nil)
	assertEqual(t, st.Obj("providers").Obj("agy").Get("refused_until"), "2026-10-04T01:03:00Z")
	providers.NoteUsage(st, "muse", nil, "2026-10-05T00:00:00Z", at, 0, nil, nil)
	assertEqual(t, st.Obj("providers").Obj("muse").Get("refused_until"), "2026-10-05T00:00:00Z")
	assertEqual(t, providers.Duration("+PT2H"), 2*time.Hour)
	// Codex names days: gpt's weekly limit holds it until the week resets, not an hour.
	assertEqual(t, runner.ParkFor("You've hit your usage limit. Upgrade to Pro or try again in "+
		"5 days 2 hours 22 minutes."), "+PT7343M")
}

// gpt's refusal named no time, so it was retried every hour although its last reading had its
// week at 89%, resetting days later.
func TestBackends_ARefusalThatNamesNoResetWaitsForTheNearlyFullWindow(t *testing.T) {
	at := time.Date(2026, 10, 3, 23, 47, 0, 0, time.UTC)
	week := func() *pyjson.Object {
		return pyjson.New("utilization", 0.89, "resets_at", "2026-10-09T23:47:06Z")
	}
	gpt := func(seven *pyjson.Object) *pyjson.Object {
		return pyjson.New("providers", pyjson.New("gpt", pyjson.New("usage", pyjson.New("seven_day", seven))))
	}
	st := gpt(week())
	providers.NoteUsage(st, "gpt", nil, providers.DefaultPark, at, 0, nil, nil)
	assertEqual(t, st.Obj("providers").Obj("gpt").Get("refused_until"), "2026-10-09T23:47:06Z")
	// A window with room left, or a reset the refusal named, keeps the shorter park.
	st = gpt(week().Set("utilization", 0.4))
	providers.NoteUsage(st, "gpt", nil, providers.DefaultPark, at, 0, nil, nil)
	assertEqual(t, st.Obj("providers").Obj("gpt").Get("refused_until"), "2026-10-04T00:47:00Z")
	st = gpt(week())
	providers.NoteUsage(st, "gpt", nil, "+PT90M", at, 0, nil, nil)
	assertEqual(t, st.Obj("providers").Obj("gpt").Get("refused_until"), "2026-10-04T01:17:00Z")
}

// ---------------------------------------------------------------------------- MuseTests

// backendsMuse is MuseTests.backend(secret): secret "" is None.
func (w *backendsWorld) backendsMuse(secret string) *runner.MuseCli {
	if secret == "" {
		secret = pyjson.Dumps(loginsMuseAuth())
	}
	login := logins.Prepare(ts.SecretLogin(w.pool.Get("muse"), "MUSE_AUTH"), secret, "", filepath.Join(w.tmp, "home"))
	backend := runner.NewMuseCli(w.bin, login, nil)
	backend.UsageDir = filepath.Join(w.tmp, "usage")
	return backend
}

func TestBackends_ThePromptGoesInAFileAndTheAnswerComesOnStdout(t *testing.T) {
	w := newBackendsWorld(t, "muse", "muse")
	result := w.runFake(w.backendsMuse(""), "ok", false)
	backendsOK(t, result)
	assertTrue(t, strings.HasPrefix(result.Text, `<!-- bot: {"status": "done"`))
	seen := w.seen()
	argv := pyjson.Strings(seen.Get("argv"))
	assertIn(t, "--yolo", argv)
	assertIn(t, "--disable-web-tools", argv)
	assertEqual(t, backendsAfter(argv, "--max-model-steps"), "42")
	assertIn(t, "THE TASK", seen.Str("prompt"))
	w.assertOnlyItsOwnLogin()
	assertIn(t, "muse", seen.Obj("env").Str("XDG_CONFIG_HOME"))
	transcript := readText(t, filepath.Join(w.tmp, "t", "01.jsonl"))
	assertIn(t, "step 1: reading", transcript)
}

func TestBackends_AUsageLimitIsARateLimit(t *testing.T) {
	w := newBackendsWorld(t, "muse", "muse")
	result := w.runFake(w.backendsMuse(""), "limit", false)
	assertTrue(t, result.RateLimited())
	assertIn(t, "usage limit reached", result.Error)
}

// Muse's own words once its week is spent (2026-10-25). Unread, they ended six runs as infra,
// each of which took an item and handed it on, until the week reset.
func TestBackends_ItsSpentWeekParksItUntilTheResetItNames(t *testing.T) {
	w := newBackendsWorld(t, "muse", "muse")
	result := w.runFake(w.backendsMuse(""), "quota", false)
	assertTrue(t, result.RateLimited())
	assertFalse(t, result.Infra())
	assertEqual(t, result.ResetAt, "2026-10-26T00:00:00Z")
}

func TestBackends_AnAPIKeySecretIsHandedOverAsOne(t *testing.T) {
	w := newBackendsWorld(t, "muse", "muse")
	w.runFake(w.backendsMuse("meta-key-"+strings.Repeat("k", 30)), "ok", false)
	assertEqual(t, w.seen().Obj("env").Get("META_API_KEY"), "meta-key-"+strings.Repeat("k", 30))
}

// `muse exec` runs no slash commands, so the TUI is started in a pseudo-terminal of its own: the
// cursor query answered, the workspace trusted the one time it asks, `/usage` typed, the panel
// read. Nothing else is ever typed, so no model is called.
func TestBackends_EachCallEndsWithAReadingOfItsUsagePanel(t *testing.T) {
	w := newBackendsWorld(t, "muse", "muse")
	backend := w.backendsMuse("")
	result := w.runFake(backend, "ok", false)
	backendsOK(t, result)
	assertEqual(t, backendsUtilization(result.Usage, "seven_day"), 0.35)
	assertEqual(t, backendsUtilization(result.Usage, "five_hour"), 0.05)
	again := backend.ReadUsage("the-model")
	assertEqual(t, backendsUtilization(again, "seven_day"), 0.35)
	typed := strings.Fields(readText(t, filepath.Join(w.tmp, "usage", "typed.log")))
	assertEqual(t, typed, []string{"1", "/usage", "/usage"}) // trusted once, then only /usage
	assertTrue(t, strings.HasSuffix(w.seen().Str("prompt"), "THE TASK"))
}

func TestBackends_ARefusalThatNamesNoResetWaitsForTheFullWindow(t *testing.T) {
	w := newBackendsWorld(t, "muse", "muse")
	result := w.runFake(w.backendsMuse(""), "limit", false)
	assertTrue(t, result.RateLimited())
	assertEqual(t, backendsUtilization(result.Usage, "seven_day"), 0.35)
	assertEqual(t, result.ResetAt, "+PT60M") // nothing is full: the hour stands
	restore := backendsPatchEnv(t, map[string]string{"FAKE_MODE": "spent", "FAKE_DUMP": w.dump})
	usage := w.backendsMuse("").ReadUsage("the-model")
	restore()
	assertEqual(t, backendsUtilization(usage, "seven_day"), 0.97)
}

// Muse's caps are 100% (#798), so its own limit ends a call that runs a window out, in words the
// bot does not know: a failed call whose reading has a window full parks it until that window
// resets, rather than counting against the item. One with room left does not.
func TestBackends_AFailedCallThatRanAWindowOutIsItsRefusal(t *testing.T) {
	w := newBackendsWorld(t, "muse", "muse")
	result := w.runFake(w.backendsMuse(""), "full", false)
	assertFalse(t, result.OK)
	assertEqual(t, backendsUtilization(result.Usage, "five_hour"), 1.0)
	assertTrue(t, result.RateLimited())
	assertEqual(t, result.ResetAt, result.Usage.Obj("five_hour").Get("resets_at"))
	broken := w.runFake(w.backendsMuse(""), "broken", false)
	assertFalse(t, broken.OK)
	assertEqual(t, backendsUtilization(broken.Usage, "five_hour"), 0.05)
	assertFalse(t, broken.RateLimited())
	assertIsNone(t, broken.ResetAt)
}

func TestBackends_NoPanelIsNoReading(t *testing.T) {
	w := newBackendsWorld(t, "muse", "muse")
	missing := runner.NewMuseCli(filepath.Join(w.tmp, "no-such-muse"), nil, nil)
	missing.UsageDir = filepath.Join(w.tmp, "usage")
	assertTrue(t, missing.ReadUsage("the-model") == nil)
}

// ---------------------------------------------------------------------------- UsageTextTests
//
// agy's `/usage` table and Muse's `/usage` panel as the bot reads them, from what each one printed
// on the machine on 2026-10-04.

// backendsAgyTable is UsageTextTests.AGY.
const backendsAgyTable = "Gemini Models\tWeekly Limit Remaining\t2%\t2026-10-09T17:47:52Z\n" +
	"Gemini Models\tFive Hour Limit Remaining\t33%\t2026-10-04T16:18:35Z\n" +
	"Claude and GPT models\tWeekly Limit Remaining\t100%\t2026-10-11T15:04:01Z\n" +
	"Claude and GPT models\tFive Hour Limit Remaining\t100%\t2026-10-04T20:04:01Z\n"

// backendsMusePanel is UsageTextTests.MUSE: the panel as the TUI drew it, its control sequences
// gone: one line, boxes and all.
const backendsMusePanel = "│ Turns 3 │ │ Subagents none │ │ │ │ Subscription " +
	"· Muse Code High Usage │ │ Current 5% used · Resets at 6:42 PM " +
	"│ │ Weekly 35% used · Resets Oct 5 at 12:00 AM │└─┘"

func TestBackends_AgyReadsThePoolItsModelDrawsOn(t *testing.T) {
	gemini := runner.ParseAgyUsage(backendsAgyTable, "gemini-3.8-flash-high")
	assertEqual(t, gemini.Get("seven_day"), pyjson.New("utilization", 0.98, "resets_at", "2026-10-09T17:47:52Z"))
	assertEqual(t, backendsUtilization(gemini, "five_hour"), 0.67)
	claude := runner.ParseAgyUsage(backendsAgyTable, "claude-opus-4-6-thinking")
	assertEqual(t, []any{backendsUtilization(claude, "seven_day"), backendsUtilization(claude, "five_hour")},
		[]any{0.0, 0.0})
	assertTrue(t, runner.ParseAgyUsage("Fetching usage...\nerror: offline\n", "gemini-3.8") == nil)
}

func TestBackends_MuseReadsThe5HourWindowAndTheWeekWithTheirResets(t *testing.T) {
	now := time.Date(2026, 10, 4, 15, 30, 0, 0, time.UTC)
	usage := runner.ParseMuseUsage(backendsMusePanel, now)
	assertEqual(t, usage.Get("five_hour"), pyjson.New("utilization", 0.05, "resets_at", "2026-10-04T18:42:00Z"))
	assertEqual(t, usage.Get("seven_day"), pyjson.New("utilization", 0.35, "resets_at", "2026-10-05T00:00:00Z"))
	assertTrue(t, runner.ParseMuseUsage("│ Session usage │ Turns 3", now) == nil)
}

func TestBackends_MusePrintsTheMachinesOwnTime(t *testing.T) {
	central := time.FixedZone("UTC-05:00", -5*60*60)
	now := time.Date(2026, 10, 4, 10, 30, 0, 0, central)
	assertEqual(t, runner.MuseReset("at 6:42 PM", now), "2026-10-04T23:42:00Z")
	assertEqual(t, runner.MuseReset("at 9:15 AM", now), "2026-10-05T14:15:00Z") // tomorrow's
	assertEqual(t, runner.MuseReset("Oct 5 at 12:00 AM", now), "2026-10-05T05:00:00Z")
	assertEqual(t, runner.MuseReset("Jan 2 at 12:30 PM", now), "2027-01-02T17:30:00Z")
	assertEqual(t, runner.MuseReset("in 3h 5m", now), "2026-10-04T18:35:00Z")
	assertIsNone(t, runner.MuseReset("soon", now))
	assertIsNone(t, runner.MuseReset("Feb 30 at 1:00 AM", now))
}
