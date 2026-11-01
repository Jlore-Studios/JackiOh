package runner

import (
	"bytes"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"syscall"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/logins"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
)

// Backends is BACKENDS: each CLI's backend, made for a binary and a login.
var Backends = map[string]func(binary string, login *logins.Login, popen Popen) Runner{
	"claude": func(b string, l *logins.Login, p Popen) Runner { return NewClaudeCli(b, l, p) },
	"codex":  func(b string, l *logins.Login, p Popen) Runner { return NewCodexCli(b, l, p) },
	"agy":    func(b string, l *logins.Login, p Popen) Runner { return NewAgyCli(b, l, p) },
	"muse":   func(b string, l *logins.Login, p Popen) Runner { return NewMuseCli(b, l, p) },
}

// PingPrompt is PING_PROMPT.
var PingPrompt string = "Reply with the word ok."

// PingSystem is the ping's whole system prompt. With it, and no tools, skills or commands, the
// call is a few hundred tokens of input rather than the CLI's default ~18,000.
var PingSystem string = "Answer in one word."

// PingTimeoutS is PING_TIMEOUT_S.
var PingTimeoutS int = 180

// Ping is `(result line, usage reading)` of the smallest call there is: one turn of `model` (at
// `effort`, when one is named) in an empty directory, with no tools, skills, commands, MCP
// servers or saved session and a one-line system prompt, signed in with `token` (else this
// process's own). `(None, None)` when the CLI could not run. A nil run is subprocess.run.
func Ping(claudeBin string, model string, run RunFunc /* = subprocess.run */, token string /* = "" */, effort string /* = "" */) (*pyjson.Object, *pyjson.Object) {
	if run == nil {
		run = SubprocessRun
	}
	argv := []string{claudeBin, "--print", "--output-format", "stream-json", "--verbose",
		"--model", model, "--max-turns", "1", "--strict-mcp-config", "--tools", "",
		"--disable-slash-commands", "--no-session-persistence", "--system-prompt", PingSystem}
	if effort != "" {
		argv = append(argv, "--effort", effort)
	}
	env := copyEnv(config.ChildEnv(nil, []string{"CLAUDE_CODE_OAUTH_TOKEN"}))
	delete(env, "ANTHROPIC_API_KEY")
	for _, key := range ActionsFiles {
		delete(env, key)
	}
	for k, v := range QuietEnv {
		env[k] = v
	}
	if token != "" {
		env["CLAUDE_CODE_OAUTH_TOKEN"] = token
	}
	empty, err := os.MkdirTemp("", "bot-ping-")
	herr.Must(err)
	proc, err := func() (*Completed, error) {
		defer os.RemoveAll(empty)
		env["CLAUDE_CONFIG_DIR"] = filepath.Join(empty, ".claude")
		input := PingPrompt
		return run(argv, RunArgs{Cwd: empty, Env: env, Input: &input,
			Timeout: time.Duration(PingTimeoutS) * time.Second})
	}()
	if err != nil {
		return nil, nil
	}
	return ParseStream(py.Splitlines(proc.Stdout))
}

// PingUsage is the Claude subscription's usage now, read off `ping`. nil when the CLI gave no
// reading.
func PingUsage(claudeBin string, model string, run RunFunc /* = subprocess.run */, token string /* = "" */, effort string /* = "" */) *pyjson.Object {
	_, usage := Ping(claudeBin, model, run, token, effort)
	return usage
}

// exhaustedReset is _exhausted_reset: the soonest reset of a window that is full, or "".
func exhaustedReset(usage *pyjson.Object) string {
	if usage == nil {
		return ""
	}
	var resets []string
	usage.Items(func(_ string, v any) bool {
		w, ok := v.(*pyjson.Object)
		if ok && w != nil && py.MustFloat(w.GetOr("utilization", 0)) >= 1.0 && w.Truthy("resets_at") {
			resets = append(resets, py.Str(w.Get("resets_at")))
		}
		return true
	})
	if len(resets) == 0 {
		return ""
	}
	soonest := resets[0]
	for _, r := range resets[1:] {
		if r < soonest {
			soonest = r
		}
	}
	return soonest
}

// ownPIDs is every process of this user, or nil where /proc cannot say (then nothing is reaped).
func ownPIDs() map[int]bool {
	if info, err := os.Stat("/proc"); err != nil || !info.IsDir() {
		return nil
	}
	uid := uint32(os.Getuid())
	entries, err := os.ReadDir("/proc")
	herr.Must(err)
	found := map[int]bool{}
	for _, entry := range entries {
		if !py.IsDecimal(entry.Name()) {
			continue
		}
		info, err := os.Stat(filepath.Join("/proc", entry.Name()))
		if err != nil {
			continue
		}
		if st, ok := info.Sys().(*syscall.Stat_t); ok && st.Uid == uid {
			found[py.Atoi(entry.Name())] = true
		}
	}
	return found
}

// statFields are the fields of /proc/<pid>/stat after the command's closing parenthesis.
func statFields(pid int) ([]string, bool) {
	stat, err := os.ReadFile("/proc/" + strconv.Itoa(pid) + "/stat")
	if err != nil {
		return nil, false
	}
	i := bytes.LastIndexByte(stat, ')')
	if i < 0 {
		return nil, false
	}
	return py.Split(string(stat[i+1:])), true
}

// ancestors is this process and every one above it.
func ancestors() map[int]bool {
	found, pid := map[int]bool{}, os.Getpid()
	for pid > 1 {
		found[pid] = true
		f, ok := statFields(pid)
		if !ok || len(f) < 2 {
			break
		}
		next, err := strconv.Atoi(f[1])
		if err != nil {
			break
		}
		pid = next
	}
	return found
}

// belongs is true when `pid` came from the call: it carries the call's marker, or its session.
func belongs(pid int, marker string, session int, hasSession bool) bool {
	environ, err := os.ReadFile("/proc/" + strconv.Itoa(pid) + "/environ")
	if err != nil {
		return false
	}
	if bytes.Contains(environ, []byte(CallMarker+"="+marker)) {
		return true
	}
	f, ok := statFields(pid)
	if !ok || len(f) < 4 {
		return false
	}
	sid, err := strconv.Atoi(f[3])
	if err != nil {
		return false
	}
	return hasSession && sid == session
}

// reap kills what the call left behind (daemons, `nohup`, `setsid`), so nothing the model
// started can touch the worktree after its session ends. Only processes that carry the call's
// marker or share its session are touched, never another program of the same user.
func reap(before map[int]bool, marker string, session int, hasSession bool) {
	if before == nil {
		return
	}
	after := ownPIDs()
	mine := ancestors()
	var pids []int
	for pid := range after {
		if !before[pid] && !mine[pid] {
			pids = append(pids, pid)
		}
	}
	sort.Ints(pids)
	for _, pid := range pids {
		if !belongs(pid, marker, session, hasSession) {
			continue
		}
		syscall.Kill(pid, syscall.SIGKILL) // gone already, or not ours to kill: let it be
	}
}

// kill is _kill: the CLI's process group asked to end, then made to, and the CLI waited for.
func kill(proc *process) {
	if err := syscall.Kill(-proc.pid, syscall.SIGTERM); err == nil && proc.wait(20*time.Second) {
		return
	}
	if err := syscall.Kill(-proc.pid, syscall.SIGKILL); err != nil {
		proc.cmd.Process.Kill()
	}
	proc.wait(-1)
}

// Text is _text: a result event's final text, or "".
func Text(result *pyjson.Object) string {
	s, _ := result.Get("result").(string)
	return s
}

// turns is _turns: a result event's turn count, or nil.
func turns(result *pyjson.Object) *int {
	if n, ok := intValue(result.Get("num_turns")); ok {
		return &n
	}
	return nil
}

// TokenNames is TOKEN_NAMES: a result event's token block under the names one harness-wide
// shape: Claude's result event carries `input_tokens`, `output_tokens`,
// `cache_creation_input_tokens` and `cache_read_input_tokens`; Codex's `turn.completed` and
// agy's result carry subsets.
var TokenNames = [][2]string{{"input_tokens", "input"}, {"output_tokens", "output"},
	{"cache_read_input_tokens", "cache_read"},
	{"cache_creation_input_tokens", "cache_write"}}

// TokensFromBlock is a result event's token counts as {input, output, cache_read, cache_write},
// or None when it names none (#695).
func TokensFromBlock(block any) *pyjson.Object {
	b, ok := block.(*pyjson.Object)
	if !ok || b == nil {
		return nil
	}
	named := false
	for _, kn := range TokenNames {
		if _, isNumber := py.Number(b.Get(kn[0])); isNumber {
			named = true
		}
	}
	if !named {
		return nil
	}
	found := pyjson.New()
	for _, kn := range TokenNames {
		value := b.Get(kn[0])
		n := 0
		if f, isNumber := py.Number(value); isNumber {
			n = int(f)
		}
		found.Set(kn[1], n)
	}
	return found
}

// CostFromResult is a result event's cost in USD (`total_cost_usd`), or None (#695).
func CostFromResult(result *pyjson.Object) *float64 {
	if result == nil {
		return nil
	}
	f, isNumber := py.Number(result.Get("total_cost_usd"))
	if !isNumber {
		return nil
	}
	return &f
}

// writeRedacted is _write_redacted: `source` copied to `target` a line at a time, redacted.
func writeRedacted(source, target string) {
	dst, err := os.Create(target)
	herr.Must(err)
	defer dst.Close()
	herr.Must(eachLine(source, func(line string) {
		_, err := dst.WriteString(redact.Redact(line, nil))
		herr.Must(err)
	}))
}

// FakePlan is what a scripted planning call answers when the script gives it no handler of its
// own.
var FakePlan string = "1. **Goal.** Do what the issue asks.\n2. **Steps.** Change it, test it."

// FakeRunnerName is FakeRunner.name.
var FakeRunnerName = "fake"

// FakeRunnerCLI is FakeRunner.cli.
var FakeRunnerCLI = "fake"

// Handler is one scripted call: it receives the request and returns a RunResult.
type Handler func(request *RunRequest) *RunResult

// FakeRunner replays scripted calls. Each handler receives the request and returns a RunResult.
type FakeRunner struct {
	Handlers map[string][]Handler
	Calls    []*RunRequest
}

// NewFakeRunner is FakeRunner(handlers): each role's handlers in the order they answer, the last
// one answering every call after.
func NewFakeRunner(handlers map[string][]Handler /* = None */) *FakeRunner {
	x := &FakeRunner{Handlers: map[string][]Handler{}}
	for role, queue := range handlers {
		x.Handlers[role] = append([]Handler(nil), queue...)
	}
	return x
}

// Name is FakeRunner.name.
func (x *FakeRunner) Name() string { return FakeRunnerName }

// CLI is FakeRunner.cli.
func (x *FakeRunner) CLI() string { return FakeRunnerCLI }

// Run is FakeRunner.run.
func (x *FakeRunner) Run(request *RunRequest) *RunResult {
	call := *request
	x.Calls = append(x.Calls, &call)
	queue := x.Handlers[request.Role]
	if len(queue) == 0 && request.Role == "plan" {
		return NewRunResult(true, FakePlan) // every build plans first; most scripts skip it
	}
	if len(queue) == 0 {
		r := NewRunResult(false, "")
		r.ExitCode, r.Error = 1, "fake runner has no handler for "+request.Role
		return r
	}
	handler := queue[0]
	if len(queue) > 1 {
		x.Handlers[request.Role] = queue[1:]
	}
	return handler(request)
}

// Trail is FakeRunner.trail.
func (x *FakeRunner) Trail(transcript string) string { return "" }

// GetRunner is the backend for `provider` (the first Claude account when there is none), signed
// in with `login`.
func GetRunner(cfg *config.Config, provider *providers.Provider /* = None */, login *logins.Login /* = None */) Runner {
	if cfg.Backend == "fake" {
		return NewFakeRunner(nil)
	}
	cli := "claude"
	if provider != nil {
		cli = provider.CLI
	}
	newBackend, ok := Backends[cli]
	if !ok {
		panic(py.KeyError(cli))
	}
	return newBackend(cfg.Bin(cli), login, nil)
}
