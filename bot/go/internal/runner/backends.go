package runner

import (
	"io/fs"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/logins"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
)

// ClaudeCliCLI is ClaudeCli.cli.
var ClaudeCliCLI = "claude"

// ClaudeCli runs one model call through the `claude` binary.
type ClaudeCli struct{ cliBase }

// NewClaudeCli is ClaudeCli(claude_bin, login, popen); a nil popen is subprocess.Popen.
func NewClaudeCli(claudeBin string /* = "claude" */, login *logins.Login /* = None */, popen Popen /* = subprocess.Popen */) *ClaudeCli {
	x := &ClaudeCli{newCliBase(claudeBin, login, popen, ClaudeCliCLI)}
	x.self = x
	return x
}

// Env is ClaudeCli.env.
func (x *ClaudeCli) Env() map[string]string {
	var env map[string]string
	if x.Login == nil {
		// Run by hand or in a test: the token comes from this process's own environment.
		env = copyEnv(config.ChildEnv(nil, []string{"CLAUDE_CODE_OAUTH_TOKEN"}))
		for _, key := range ActionsFiles {
			delete(env, key)
		}
	} else {
		env = x.cliBase.Env()
	}
	for _, key := range []string{"ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_PROFILE"} {
		delete(env, key)
	}
	for k, v := range QuietEnv {
		env[k] = v
	}
	return env
}

// Argv is ClaudeCli.argv.
func (x *ClaudeCli) Argv(request *RunRequest, systemFile string) []string {
	argv := []string{
		x.Binary,
		"--print",
		"--output-format", "stream-json",
		"--verbose",
		"--model", request.Model,
		"--effort", request.Effort,
		"--max-turns", strconv.Itoa(request.MaxTurns),
		"--permission-mode", "acceptEdits",
		"--allowed-tools", strings.Join(request.AllowedTools, ","),
		"--append-system-prompt-file", systemFile,
		"--strict-mcp-config",
	}
	if len(request.DisallowedTools) > 0 {
		argv = append(argv, "--disallowed-tools", strings.Join(request.DisallowedTools, ","))
	}
	return argv
}

// Run is ClaudeCli.run.
func (x *ClaudeCli) Run(request *RunRequest) *RunResult {
	transcript, raw := x.paths(request)
	systemFile := withSuffix(transcript, ".system.md")
	writeText(systemFile, request.SystemAppend)
	prompt := request.Prompt
	launch, failed := x.launch(x.Argv(request, systemFile), request, x.Env(), &prompt, raw)
	if failed != nil {
		return failed
	}
	var stream streamParser
	herr.Must(eachLine(raw, stream.add))
	result, usage := stream.result, stream.usage
	x.keep(launch, transcript)
	var tokens *pyjson.Object
	if result != nil {
		tokens = TokensFromBlock(result.Get("usage"))
	}
	cost := CostFromResult(result)
	if launch.stopped != "" {
		r := NewRunResult(false, Text(result))
		r.ExitCode, r.Turns, r.DurationS = launch.code, turns(result), launch.elapsed
		r.Error, r.Usage, r.Tokens, r.CostUSD = "stopped mid-call: "+launch.stopped, usage, tokens, cost
		r.Extra = pyjson.New("usage_stop", launch.stopped)
		return r
	}
	if launch.timedOut {
		r := NewRunResult(false, Text(result))
		r.ExitCode, r.Turns, r.DurationS = ExitTimeout, turns(result), launch.elapsed
		r.Error = "timed out after " + strconv.Itoa(request.TimeoutS) + "s"
		r.Usage, r.Tokens, r.CostUSD, r.TimedOut = usage, tokens, cost, true
		return r
	}
	code := launch.code
	isError := true
	if result != nil {
		isError = result.Truthy("is_error")
	}
	text := Text(result)
	errText := ""
	if isError || code != 0 {
		said := text
		if said == "" {
			said = StderrError(launch.stderr, "claude exited "+strconv.Itoa(code))
		}
		errText = redact.Redact(py.SliceFrom(said, -2000), nil)
	}
	resetAt := ""
	rejected := usage != nil && pyjson.Equal(usage.Get("status"), "rejected")
	if (isError || code != 0) && (rejected || RateLimitWords.MatchString(errText)) {
		resetAt = usage.Str("rejected_resets_at")
		if resetAt == "" {
			resetAt = exhaustedReset(usage)
		}
		if resetAt == "" {
			resetAt = defaultPark
		}
	}
	r := NewRunResult(!isError && code == 0, text)
	r.ExitCode, r.Turns, r.DurationS, r.Error = code, turns(result), launch.elapsed, errText
	r.Usage, r.Tokens, r.CostUSD, r.ResetAt = usage, tokens, cost, resetAt
	return r
}

// LiveUsage is ClaudeCli.live_usage: a `rate_limit_event` line's reading.
func (x *ClaudeCli) LiveUsage(line string) *pyjson.Object {
	if !strings.Contains(line, "rate_limit_event") {
		return nil
	}
	if event := jsonLine(line); event != nil {
		return UsageFromEvent(event)
	}
	return nil
}

// Trail is ClaudeCli.trail.
func (x *ClaudeCli) Trail(transcript string) string {
	var entries []string
	for _, event := range jsonLines(py.Splitlines(Read(transcript))) {
		message := event.Obj("message")
		for _, b := range pyjson.List(message.Get("content")) {
			block, ok := b.(*pyjson.Object)
			if !ok {
				continue
			}
			switch {
			case pyjson.Equal(block.Get("type"), "text") && pyjson.Equal(event.Get("type"), "assistant"):
				entries = append(entries, "said: "+Clip(block.Get("text"), 300))
			case pyjson.Equal(block.Get("type"), "tool_use"):
				entries = append(entries, py.Str(block.Get("name"))+": "+Clip(pyjson.Dumps(block.Get("input")), 200))
			case pyjson.Equal(block.Get("type"), "tool_result") && block.Truthy("is_error"):
				entries = append(entries, "tool error: "+Clip(block.Get("content"), 200))
			}
		}
	}
	return trailText(entries)
}

// CodexCliCLI is CodexCli.cli.
var CodexCliCLI = "codex"

// CodexCli is OpenAI's Codex CLI (`codex exec`), signed in with ChatGPT.
type CodexCli struct{ cliBase }

// NewCodexCli is CodexCli(binary, login, popen); a nil popen is subprocess.Popen.
func NewCodexCli(binary string, login *logins.Login /* = None */, popen Popen /* = subprocess.Popen */) *CodexCli {
	x := &CodexCli{newCliBase(binary, login, popen, CodexCliCLI)}
	x.self = x
	return x
}

// Argv is CodexCli.argv.
func (x *CodexCli) Argv(request *RunRequest, lastMessage string) []string {
	argv := []string{x.Binary, "exec", "--json", "--skip-git-repo-check", "--cd", request.Cwd,
		"--model", request.Model, "--sandbox", "workspace-write",
		"-c", "sandbox_workspace_write.network_access=false",
		"--output-last-message", lastMessage}
	if request.Effort != "" {
		argv = append(argv, "-c", `model_reasoning_effort="`+request.Effort+`"`)
	}
	for _, directory := range request.ExtraDirs {
		argv = append(argv, "--add-dir", directory)
	}
	return append(argv, "-")
}

// Run is CodexCli.run.
func (x *CodexCli) Run(request *RunRequest) *RunResult {
	transcript, raw := x.paths(request)
	last := withSuffix(transcript, ".last.md")
	since := float64(time.Now().UnixNano())/1e9 - 5
	prompt := x.prompt(request)
	launch, failed := x.launch(x.Argv(request, last), request, x.Env(), &prompt, raw)
	if failed != nil {
		return failed
	}
	var failedWith, messages []string
	var tokens *pyjson.Object
	for _, event := range jsonLines(py.Splitlines(Read(raw))) {
		kind := event.Get("type")
		switch {
		case pyjson.Equal(kind, "turn.failed") || pyjson.Equal(kind, "error"):
			e := event
			if inner, ok := event.Get("error").(*pyjson.Object); ok {
				e = inner
			}
			message := ""
			if e.Truthy("message") {
				message = py.Str(e.Get("message"))
			}
			failedWith = append(failedWith, message)
		case pyjson.Equal(kind, "item.completed"):
			item := event.Obj("item")
			if pyjson.Equal(item.Get("type"), "agent_message") {
				said := ""
				if item.Truthy("text") {
					said = py.Str(item.Get("text"))
				}
				messages = append(messages, said)
			}
		case pyjson.Equal(kind, "turn.completed"):
			if found := TokensFromBlock(event.Get("usage")); found != nil {
				tokens = found
			}
		}
	}
	x.keep(launch, transcript)
	text := py.Strip(Read(last))
	if text == "" && len(messages) > 0 {
		text = messages[len(messages)-1]
	}
	home := x.Env()["CODEX_HOME"]
	if home == "" {
		home = filepath.Join(homeDir(), ".codex")
	}
	usage := CodexUsage(home, since)
	if launch.timedOut {
		r := NewRunResult(false, text)
		r.ExitCode, r.DurationS = ExitTimeout, launch.elapsed
		r.Error = "timed out after " + strconv.Itoa(request.TimeoutS) + "s"
		r.Usage, r.TimedOut, r.Tokens = usage, true, tokens
		return r
	}
	ok := launch.code == 0 && len(failedWith) == 0
	errText := ""
	if !ok {
		said := strings.Join(failedWith, "; ")
		if said == "" {
			said = StderrError(launch.stderr, "codex exited "+strconv.Itoa(launch.code))
		}
		errText = redact.Redact(py.SliceFrom(said, -2000), nil)
	}
	resetAt := ""
	if !ok && RateLimitWords.MatchString(errText) {
		resetAt = exhaustedReset(usage)
		if resetAt == "" {
			resetAt = defaultPark
		}
	}
	r := NewRunResult(ok, text)
	r.ExitCode, r.DurationS, r.Error, r.Usage, r.ResetAt = launch.code, launch.elapsed, errText, usage, resetAt
	r.Tokens = tokens
	return r
}

// Trail is CodexCli.trail.
func (x *CodexCli) Trail(transcript string) string {
	var entries []string
	for _, event := range jsonLines(py.Splitlines(Read(transcript))) {
		if !pyjson.Equal(event.Get("type"), "item.completed") {
			continue
		}
		item := event.Obj("item")
		kind := item.Get("type")
		switch {
		case pyjson.Equal(kind, "agent_message"):
			entries = append(entries, "said: "+Clip(item.Get("text"), 300))
		case pyjson.Equal(kind, "command_execution"):
			entries = append(entries, "ran `"+Clip(item.Get("command"), 160)+"` (exit "+
				py.Str(item.Get("exit_code"))+")")
		case pyjson.Equal(kind, "file_change"):
			var paths []string
			for _, c := range pyjson.List(item.Get("changes")) {
				if change, ok := c.(*pyjson.Object); ok {
					paths = append(paths, py.Str(change.Get("path")))
				}
			}
			entries = append(entries, "changed "+strings.Join(paths, ", "))
		}
	}
	return trailText(entries)
}

// CodexUsage is the subscription's 5-hour and weekly use, from the newest `token_count` event
// in the session logs Codex wrote since `since` (the `exec` stream itself carries none).
func CodexUsage(codexHome string, since float64) *pyjson.Object {
	sessions := filepath.Join(codexHome, "sessions")
	var newest *pyjson.Object
	if info, err := os.Stat(sessions); err != nil || !info.IsDir() {
		return nil
	}
	for _, path := range rglob(sessions, "rollout-*.jsonl") {
		info, err := os.Stat(path)
		if err != nil {
			continue
		}
		if float64(info.ModTime().UnixNano())/1e9 < since {
			continue
		}
		for _, event := range jsonLines(py.Splitlines(Read(path))) {
			payload := event.Obj("payload")
			if pyjson.Equal(payload.Get("type"), "token_count") {
				if limits, ok := payload.Get("rate_limits").(*pyjson.Object); ok && limits != nil {
					newest = limits
				}
			}
		}
	}
	if newest.Len() == 0 {
		return nil
	}
	usage := pyjson.New("status", "allowed")
	for _, key := range []string{"primary", "secondary"} {
		window, ok := newest.Get(key).(*pyjson.Object)
		used, isNumber := py.Number(window.Get("used_percent"))
		if !ok || window == nil || !isNumber {
			continue
		}
		minutes, hasMinutes := window.Lookup("window_minutes")
		name := "seven_day"
		if n, isNumber := py.Number(minutes); isNumber && n <= 360 {
			name = "five_hour"
		}
		if key == "primary" && (!hasMinutes || minutes == nil) {
			name = "five_hour"
		}
		usage.Set(name, pyjson.New("utilization", used/100.0,
			"resets_at", isoOrNone(window.Get("resets_at"))))
	}
	if usage.Len() > 1 {
		return usage
	}
	return nil
}

// rglob is sorted(Path(root).rglob(pattern)): every file or directory under root whose name
// matches, symbolic links to directories not followed, in pathlib's order (part by part).
func rglob(root, pattern string) []string {
	var found []string
	filepath.WalkDir(root, func(path string, d fs.DirEntry, err error) error {
		if err != nil || path == root {
			return nil
		}
		if py.Fnmatch(d.Name(), pattern) {
			found = append(found, path)
		}
		return nil
	})
	sort.SliceStable(found, func(i, j int) bool {
		a, b := strings.Split(found[i], "/"), strings.Split(found[j], "/")
		for k := 0; k < len(a) && k < len(b); k++ {
			if a[k] != b[k] {
				return a[k] < b[k]
			}
		}
		return len(a) < len(b)
	})
	return found
}

// isoUTC is _ISO_UTC (Python's `$` also matches before a final line feed).
var isoUTC = regexp.MustCompile(`^` + py.DigitClass + `{4}-` + py.DigitClass + `{2}-` + py.DigitClass + `{2}T` + py.DigitClass +
	`{2}:` + py.DigitClass + `{2}(?::` + py.DigitClass + `{2}(?:\.` + py.DigitClass + `+)?)?Z\n?$`)

var (
	agyCells = regexp.MustCompile(`\t+|[` + py.SpaceClass + `]{2,}`)
	agyShare = regexp.MustCompile(`^(` + py.DigitClass + `+(?:\.` + py.DigitClass + `+)?)%$`)
)

// ParseAgyUsage is agy's `/usage` table as a reading for `model`'s pool. Each row is a pool, a
// limit, the share left and its reset, tab-separated: `Gemini Models  Weekly Limit Remaining  2%
// 2026-10-09T17:47:52Z`. A Gemini model draws on the Gemini pool; the Claude and GPT models agy
// also serves share the other one.
func ParseAgyUsage(text string, model string) *pyjson.Object {
	gemini := strings.HasPrefix(strings.ToLower(model), "gemini")
	usage := pyjson.New()
	for _, line := range py.Splitlines(text) {
		var cells []string
		for _, cell := range agyCells.Split(py.Strip(line), -1) {
			if py.Strip(cell) != "" {
				cells = append(cells, py.Strip(cell))
			}
		}
		if len(cells) < 3 || strings.HasPrefix(strings.ToLower(cells[0]), "gemini") != gemini {
			continue
		}
		limit := strings.ToLower(cells[1])
		share := agyShare.FindStringSubmatch(cells[2])
		window := ""
		if strings.Contains(limit, "week") {
			window = "seven_day"
		} else if strings.Contains(limit, "five") || strings.Contains(limit, "5") {
			window = "five_hour"
		}
		if share == nil || window == "" {
			continue
		}
		fraction := py.MustFloat(share[1]) / 100.0
		used := 1.0 - fraction
		if strings.Contains(limit, "used") {
			used = fraction
		}
		var resets any
		if len(cells) > 3 && isoUTC.MatchString(cells[3]) {
			resets = cells[3]
		}
		usage.Set(window, pyjson.New("utilization", py.RoundN(min(1.0, max(0.0, used)), 4),
			"resets_at", resets))
	}
	if usage.Len() == 0 {
		return nil
	}
	return usage.Copy().Set("status", "allowed")
}

// AgyUsage is agy's usage now, from its own `/usage` command: no model call, so it costs nothing.
// A nil run is subprocess.run.
func AgyUsage(binary string, model string, env map[string]string, run RunFunc /* = subprocess.run */) *pyjson.Object {
	if run == nil {
		run = SubprocessRun
	}
	empty, err := os.MkdirTemp("", "bot-agy-usage-")
	herr.Must(err)
	proc, err := func() (*Completed, error) {
		defer os.RemoveAll(empty)
		return run([]string{binary, "-p", "/usage"}, RunArgs{Cwd: empty, Env: env,
			Timeout: time.Duration(UsageTimeoutS) * time.Second})
	}()
	if err != nil {
		return nil
	}
	return ParseAgyUsage(proc.Stdout, model)
}

// AgyCliCLI is AgyCli.cli.
var AgyCliCLI = "agy"

// AgyCliPollsUsage is AgyCli.polls_usage.
var AgyCliPollsUsage = true

// AgyCli is Google's Antigravity CLI (`agy`), signed in on the machine with a Google account.
//
// Print mode reads the prompt from stdin. In headless mode agy refuses shell commands unless
// told otherwise, so `--dangerously-skip-permissions` lets it run them, as the other CLIs do;
// the job holds no write token. Its model names carry their effort (`gemini-3.8-flash-high`;
// `agy models` lists them), and `--effort` passes the same. Its stream carries no usage, so its
// `/usage` table is read around and during each call (`polls_usage`).
type AgyCli struct{ cliBase }

// NewAgyCli is AgyCli(binary, login, popen); a nil popen is subprocess.Popen.
func NewAgyCli(binary string, login *logins.Login /* = None */, popen Popen /* = subprocess.Popen */) *AgyCli {
	x := &AgyCli{newCliBase(binary, login, popen, AgyCliCLI)}
	x.self = x
	return x
}

// PollsUsage is AgyCli.polls_usage.
func (x *AgyCli) PollsUsage() bool { return AgyCliPollsUsage }

// ReadUsage is AgyCli.read_usage.
func (x *AgyCli) ReadUsage(model string) *pyjson.Object {
	return AgyUsage(x.Binary, model, x.Env(), nil)
}

// Argv is AgyCli.argv.
func (x *AgyCli) Argv(request *RunRequest) []string {
	argv := []string{x.Binary, "--output-format", "stream-json", "--dangerously-skip-permissions",
		"--disable-slash-commands", "--model", request.Model}
	if request.Effort != "" {
		argv = append(argv, "--effort", request.Effort)
	}
	for _, directory := range request.ExtraDirs {
		argv = append(argv, "--add-dir", directory)
	}
	return argv
}

// Run is AgyCli.run.
func (x *AgyCli) Run(request *RunRequest) *RunResult {
	transcript, raw := x.paths(request)
	prompt := x.prompt(request)
	launch, failed := x.launch(x.Argv(request), request, x.Env(), &prompt, raw)
	if failed != nil {
		return failed
	}
	result := pyjson.New()
	for _, event := range jsonLines(py.Splitlines(Read(raw))) {
		if pyjson.Equal(event.Get("event"), "result") {
			if found, ok := event.Get("result").(*pyjson.Object); ok && found != nil {
				result = found
			}
		}
	}
	x.keep(launch, transcript)
	text := ""
	if result.Truthy("response") {
		text = py.Strip(py.Str(result.Get("response")))
	}
	var numTurns *int
	if n, ok := intValue(result.Get("num_turns")); ok {
		numTurns = &n
	}
	tokens := TokensFromBlock(result.Get("usage"))
	if launch.stopped != "" {
		r := NewRunResult(false, text)
		r.ExitCode, r.Turns, r.DurationS = launch.code, numTurns, launch.elapsed
		r.Error, r.Tokens = "stopped mid-call: "+launch.stopped, tokens
		r.Extra = pyjson.New("usage_stop", launch.stopped)
		return x.withUsage(r, request)
	}
	if launch.timedOut {
		r := NewRunResult(false, text)
		r.ExitCode, r.Turns, r.DurationS = ExitTimeout, numTurns, launch.elapsed
		r.Error = "timed out after " + strconv.Itoa(request.TimeoutS) + "s"
		r.TimedOut, r.Tokens = true, tokens
		return x.withUsage(r, request)
	}
	ok := launch.code == 0 && pyjson.Equal(result.Get("status"), "SUCCESS")
	errText := ""
	if !ok {
		said := ""
		if result.Truthy("error") {
			said = py.Str(result.Get("error"))
		}
		if said == "" {
			said = StderrError(launch.stderr, "agy exited "+strconv.Itoa(launch.code)+" ("+
				py.Str(result.Get("status"))+")")
		}
		errText = redact.Redact(py.SliceFrom(said, -2000), nil)
	}
	resetAt := ""
	if !ok && RateLimitWords.MatchString(errText) {
		resetAt = ParkFor(errText)
	}
	r := NewRunResult(ok, text)
	r.ExitCode, r.Turns, r.DurationS, r.Error, r.ResetAt = launch.code, numTurns, launch.elapsed, errText, resetAt
	r.Tokens = tokens
	return x.withUsage(r, request)
}

// Trail is AgyCli.trail.
func (x *AgyCli) Trail(transcript string) string {
	var entries []string
	said := map[string]string{}
	for _, event := range jsonLines(py.Splitlines(Read(transcript))) {
		step, ok := event.Get("step_update").(*pyjson.Object)
		if !ok || step == nil {
			continue
		}
		switch {
		case pyjson.Equal(step.Get("step_type"), "agent_response"):
			key := dictKey(step.Get("step_index"))
			delta := ""
			if step.Truthy("text_delta") {
				delta = py.Str(step.Get("text_delta"))
			}
			said[key] += delta
			if pyjson.Equal(step.Get("state"), "DONE") {
				entries = append(entries, "said: "+Clip(said[key], 300))
				delete(said, key)
			}
		case pyjson.Equal(step.Get("step_type"), "tool") && pyjson.Equal(step.Get("state"), "DONE"):
			info := step.Obj("tool_info")
			params := info.Get("parameters")
			if !pyjson.Truthy(params) {
				params = pyjson.New()
			}
			var shown any
			if p, ok := params.(*pyjson.Object); ok {
				shown = p.Get("CommandLine")
			}
			if !pyjson.Truthy(shown) {
				shown = pyjson.Dumps(params)
			}
			entries = append(entries, py.Str(info.Get("name"))+": "+Clip(shown, 200))
		}
	}
	return trailText(entries)
}

// dictKey is a JSON value as a Python dict key: equal numbers (1, 1.0 and True) are one key.
func dictKey(v any) string {
	switch x := v.(type) {
	case nil:
		return "None"
	case string:
		return "s" + x
	case bool, int, float64:
		f, _ := py.Number(x)
		if f == float64(int64(f)) {
			return "n" + strconv.FormatInt(int64(f), 10)
		}
		return "f" + py.Str(f)
	}
	return "x" + pyjson.Dumps(v)
}

// intValue is a value Python's isinstance(v, int) accepts (a bool among them), as an int.
func intValue(v any) (int, bool) {
	switch x := v.(type) {
	case int:
		return x, true
	case bool:
		if x {
			return 1, true
		}
		return 0, true
	}
	return 0, false
}
