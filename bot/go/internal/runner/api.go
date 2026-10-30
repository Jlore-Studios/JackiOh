// Package runner is bot/harness/runner.py.
//
// Model calls: one backend per CLI (`claude`, `codex`, `agy`, `muse`), and a scripted
// fake.
//
// Every backend runs its CLI the same way (`_Cli._launch`): the prompt on stdin or in a file, the
// output written straight to a file so a long session never sits in memory, a timeout that kills
// the whole process group, and afterwards every process the call left behind reaped. The child's
// environment has every secret stripped (`config.child_env`) and gets back only its own login
// (`logins.Login.env`). What differs is the command line, how the output is read (the final text,
// success, the subscription's usage, a refusal and when it resets) and how a transcript is boiled
// down to a trail for the agent that picks the work up next (`trail`).
//
// Claude Code takes the system prompt as an appended system prompt and its tool lists as flags.
// The other CLIs get the system text at the top of the prompt, with a note that the instructions
// were written for Claude Code, and their own guard rails: Codex's workspace sandbox with the
// network off; agy's and Muse's own permission switches, inside a job that holds no write token.
package runner

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"time"
)

// RATE_LIMIT_WORDS = re.compile( r"(?i)(usage limit|rate limit|too many requests|limit reached|hit your (?:\w+ )?limit" r"|quota exceeded|exhausted your (?:\w+ )?
var RateLimitWords any // unported

// RESETS_AT = re.compile( r"(?i)resets? at\s+(\d{4}-\d\d-\d\dT\d\d:\d\d(?::\d\d(?:\.\d+)?)?(?:Z|[+-]\d\d:?\d\d))")
var ResetsAt any // unported

// RESETS_IN = re.compile( r"(?i)(?:resets?|try again) in\s*(?:(\d+)\s*d(?:ays?)?\b)?[\s,]*(?:(\d+)\s*h(?:ours?)?\b|(\d+)h)?" r"[\s,]*(?:(\d+)\s*m(?:in(?:ute)?s?)?
var ResetsIn any // unported

// WATCH_SECONDS = 15
var WatchSeconds int = 15

// POLL_SECONDS = 600
var PollSeconds int = 600

// USAGE_TIMEOUT_S = 90
var UsageTimeoutS int = 90

// AUTH_WORDS = re.compile( r"(?i)(invalid api key|authentication[_ ]error|oauth token|not logged in|please run /login" r"|invalid bearer|401 unauthorized|credit b
var AuthWords any // unported

// USAGE_WINDOWS = ("five_hour", "seven_day")
var UsageWindows = []string{"five_hour", "seven_day"}

// CALL_MARKER = "JACKIOH_BOT_CALL"
var CallMarker string = "JACKIOH_BOT_CALL"

// ACTIONS_FILES = ("GITHUB_ENV", "GITHUB_PATH", "GITHUB_OUTPUT", "GITHUB_STATE", "GITHUB_STEP_SUMMARY")
var ActionsFiles = []string{"GITHUB_ENV", "GITHUB_PATH", "GITHUB_OUTPUT", "GITHUB_STATE", "GITHUB_STEP_SUMMARY"}

// QUIET_ENV = {"CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1", "DISABLE_AUTOUPDATER": "1"}
var QuietEnv = map[string]string{"CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1", "DISABLE_AUTOUPDATER": "1"}

// EXIT_TIMEOUT = 124
var ExitTimeout int = 124

// EXIT_NOT_FOUND = 127
var ExitNotFound int = 127

// DEFAULT_PARK = "+PT60M"
var DefaultPark string = "+PT60M"

// ERROR_LINES = 20
var ErrorLines int = 20

// What a failed CLI said on stderr: the message of the JSON error body it ended with, when it
// ended with one (a CLI may print `{"error": {"message": ...}}` over several lines, so its last
// line alone is a bare `}`, #317 part 1), else its last `ERROR_LINES` lines, else `fallback`.

func StderrError(stderr string, fallback string) string {
	panic("unported: runner.stderr_error")
}

// How long a refused CLI is left alone: until the reset its message names (as a time, or as
// a wait plus a minute), or an hour when it names none.

func ParkFor(error_ string) string {
	panic("unported: runner.park_for")
}

// TRAIL_ENTRIES = 40
var TrailEntries int = 40

// TRAIL_CHARS = 6000
var TrailChars int = 6000

// CLI_NOTE = """\ ## You are running in the {cli} CLI These instructions were written for Claude Code. Where they mention subagents, `TodoWrite`, `EnterWorktree`
var CLINote string = "## You are running in the {cli} CLI\n\nThese instructions were written for Claude Code. Where they mention subagents, `TodoWrite`,\n`EnterWorktree` or another tool you do not have, do that work yourself, one step after another.\n`CLAUDE.md` binds you exactly as it binds Claude: read it, and `AGENTS.md`, before you change\nanything."

// RunRequest is runner.RunRequest.

type RunRequest struct {
	Role            string
	Prompt          string
	Cwd             string
	SystemAppend    string
	AllowedTools    []string
	DisallowedTools []string
	MaxTurns        int
	TimeoutS        int
	Model           string
	Effort          string
	Transcript      string                      // default: None
	ReadOnly        bool                        // default: False
	ExtraDirs       []string                    // default: ()
	UsageStop       func(*pyjson.Object) string // default: None
}

// RunResult is runner.RunResult.

type RunResult struct {
	OK        bool
	Text      string
	ExitCode  int            // default: 0
	Turns     *int           // default: None
	DurationS float64        // default: 0.0
	Error     string         // default: None
	Usage     *pyjson.Object // default: None
	ResetAt   string         // default: None
	TimedOut  bool           // default: False
	Extra     *pyjson.Object // default: field(default_factory=dict)
	InfraHint bool           // default: False
	Tokens    *pyjson.Object // default: None
	CostUSD   *float64       // default: None
}

// The CLI could not run at all: missing, or refused by authentication.

func (x *RunResult) Infra() bool {
	panic("unported: runner.RunResult.infra")
}

func (x *RunResult) RateLimited() bool {
	panic("unported: runner.RunResult.rate_limited")
}

// One `rate_limit_event` as `{five_hour: {...}, seven_day: {...}, status}`, or None.

func UsageFromEvent(event *pyjson.Object) *pyjson.Object {
	panic("unported: runner.usage_from_event")
}

// `(result line, last usage reading)` from Claude's stream-json lines.

func ParseStream(lines any) (*pyjson.Object, *pyjson.Object) {
	panic("unported: runner.parse_stream")
}

// _clip is runner._clip.

func Clip(text any, limit int /* = 300 */) string {
	panic("unported: runner._clip")
}

// _read is runner._read.

func Read(path string) string {
	panic("unported: runner._read")
}

// ClaudeCli.cli = "claude"
var ClaudeCliCLI any // unported

// Runs one model call through the `claude` binary.

type ClaudeCli struct {
}

// NewClaudeCli is ClaudeCli(...).

func NewClaudeCli(claudeBin string /* = "claude" */, login any /* = None */, popen func(...any) any /* = subprocess.Popen */) *ClaudeCli {
	panic("unported: runner.ClaudeCli.__init__")
}

func (x *ClaudeCli) Env() map[string]string {
	panic("unported: runner.ClaudeCli.env")
}

func (x *ClaudeCli) Argv(request *RunRequest, systemFile string) []string {
	panic("unported: runner.ClaudeCli.argv")
}

func (x *ClaudeCli) Run(request *RunRequest) *RunResult {
	panic("unported: runner.ClaudeCli.run")
}

func (x *ClaudeCli) LiveUsage(line string) *pyjson.Object {
	panic("unported: runner.ClaudeCli.live_usage")
}

func (x *ClaudeCli) Trail(transcript string) string {
	panic("unported: runner.ClaudeCli.trail")
}

// CodexCli.cli = "codex"
var CodexCliCLI any // unported

// OpenAI's Codex CLI (`codex exec`), signed in with ChatGPT.

type CodexCli struct {
}

func (x *CodexCli) Argv(request *RunRequest, lastMessage string) []string {
	panic("unported: runner.CodexCli.argv")
}

func (x *CodexCli) Run(request *RunRequest) *RunResult {
	panic("unported: runner.CodexCli.run")
}

func (x *CodexCli) Trail(transcript string) string {
	panic("unported: runner.CodexCli.trail")
}

// The subscription's 5-hour and weekly use, from the newest `token_count` event in the
// session logs Codex wrote since `since` (the `exec` stream itself carries none).

func CodexUsage(codexHome string, since float64) *pyjson.Object {
	panic("unported: runner.codex_usage")
}

// agy's `/usage` table as a reading for `model`'s pool. Each row is a pool, a limit, the
// share left and its reset, tab-separated: `Gemini Models  Weekly Limit Remaining  2%
// 2026-10-09T17:47:52Z`. A Gemini model draws on the Gemini pool; the Claude and GPT models agy
// also serves share the other one.

func ParseAgyUsage(text string, model string) *pyjson.Object {
	panic("unported: runner.parse_agy_usage")
}

// agy's usage now, from its own `/usage` command: no model call, so it costs nothing.

func AgyUsage(binary string, model string, env map[string]string, run func(...any) any /* = subprocess.run */) *pyjson.Object {
	panic("unported: runner.agy_usage")
}

// AgyCli.cli = "agy"
var AgyCliCLI any // unported

// AgyCli.polls_usage = True
var AgyCliPollsUsage any // unported

// Google's Antigravity CLI (`agy`), signed in on the machine with a Google account.
//
// Print mode reads the prompt from stdin. In headless mode agy refuses shell commands unless
// told otherwise, so `--dangerously-skip-permissions` lets it run them, as the other CLIs do;
// the job holds no write token. Its model names carry their effort (`gemini-3.8-flash-high`;
// `agy models` lists them), and `--effort` passes the same. Its stream carries no usage, so its
// `/usage` table is read around and during each call (`polls_usage`).

type AgyCli struct {
}

func (x *AgyCli) ReadUsage(model string) *pyjson.Object {
	panic("unported: runner.AgyCli.read_usage")
}

func (x *AgyCli) Argv(request *RunRequest) []string {
	panic("unported: runner.AgyCli.argv")
}

func (x *AgyCli) Run(request *RunRequest) *RunResult {
	panic("unported: runner.AgyCli.run")
}

func (x *AgyCli) Trail(transcript string) string {
	panic("unported: runner.AgyCli.trail")
}

// MUSE_TRUST = "Do you trust this workspace"
var MuseTrust string = "Do you trust this workspace"

// MUSE_PROMPT = "❯"
var MusePrompt string = "❯"

// A Muse reset as UTC. It prints the machine's own time: "at 6:42 PM" (today, or tomorrow
// once that has passed), "Oct 5 at 12:00 AM", or "in 3h 5m".

func MuseReset(text string, now time.Time) string {
	panic("unported: runner._muse_reset")
}

// Muse's `/usage` panel as a reading: "Current 5% used · Resets at 6:42 PM" is the 5-hour
// window, "Weekly 35% used · Resets Oct 5 at 12:00 AM" the week. The newest drawing counts.

func ParseMuseUsage(screen string, now time.Time) *pyjson.Object {
	panic("unported: runner.parse_muse_usage")
}

// What Muse's TUI draws after `/usage`, read in a pseudo-terminal: `muse exec` runs no slash
// commands, and neither its JSON events nor its session files carry the subscription's usage.
// It answers the TUI's cursor query, trusts `workspace` the one time it is asked, types `/usage`
// once the prompt is drawn and quiet, and ends the TUI as soon as the week's row is drawn.
// Nothing else is typed, so no model is called. "" when the panel never came.

func MuseScreen(binary string, env map[string]string, workspace string, timeoutS float64 /* = USAGE_TIMEOUT_S */, quietS float64 /* = 1.0 */) string {
	panic("unported: runner.muse_screen")
}

// MuseCli.cli = "muse"
var MuseCliCLI any // unported

// MuseCli.polls_usage = True
var MuseCliPollsUsage any // unported

// Meta's Muse Code CLI (`muse exec`). Its call reports no usage, so the TUI's `/usage` panel
// is read around and during each call (`polls_usage`), in a workspace of its own under the
// user's cache, trusted once.

type MuseCli struct {
	UsageDir string // default: None
}

func (x *MuseCli) ReadUsage(model string) *pyjson.Object {
	panic("unported: runner.MuseCli.read_usage")
}

func (x *MuseCli) Argv(request *RunRequest, promptFile string) []string {
	panic("unported: runner.MuseCli.argv")
}

func (x *MuseCli) Run(request *RunRequest) *RunResult {
	panic("unported: runner.MuseCli.run")
}

// BACKENDS: dict[str, type[_Cli]] = {"claude": ClaudeCli, "codex": CodexCli, "agy": AgyCli, "muse": MuseCli}
var Backends *pyjson.Object // unported

// PING_PROMPT = "Reply with the word ok."
var PingPrompt string = "Reply with the word ok."

// PING_SYSTEM = "Answer in one word."
var PingSystem string = "Answer in one word."

// PING_TIMEOUT_S = 180
var PingTimeoutS int = 180

// `(result line, usage reading)` of the smallest call there is: one turn of `model` (at
// `effort`, when one is named) in an empty directory, with no tools, skills, commands, MCP
// servers or saved session and a one-line system prompt, signed in with `token` (else this
// process's own). `(None, None)` when the CLI could not run.

func Ping(claudeBin string, model string, run func(...any) any /* = subprocess.run */, token string /* = "" */, effort string /* = "" */) (*pyjson.Object, *pyjson.Object) {
	panic("unported: runner.ping")
}

// The Claude subscription's usage now, read off `ping`. None when the CLI gave no
// reading.

func PingUsage(claudeBin string, model string, run func(...any) any /* = subprocess.run */, token string /* = "" */, effort string /* = "" */) *pyjson.Object {
	panic("unported: runner.ping_usage")
}

// _text is runner._text.

func Text(result *pyjson.Object) string {
	panic("unported: runner._text")
}

// TOKEN_NAMES = (("input_tokens", "input"), ("output_tokens", "output"), ("cache_read_input_tokens", "cache_read"), ("cache_creation_input_tokens", "cache_write")
var TokenNames any // unported

// A result event's token counts as {input, output, cache_read, cache_write}, or None
// when it names none (#695).

func TokensFromBlock(block any) *pyjson.Object {
	panic("unported: runner.tokens_from_block")
}

// A result event's cost in USD (`total_cost_usd`), or None (#695).

func CostFromResult(result *pyjson.Object) *float64 {
	panic("unported: runner.cost_from_result")
}

// FAKE_PLAN = "1. **Goal.** Do what the issue asks.\n2. **Steps.** Change it, test it."
var FakePlan string = `1. **Goal.** Do what the issue asks.
2. **Steps.** Change it, test it.`

// FakeRunner.name = "fake"
var FakeRunnerName any // unported

// FakeRunner.cli = "fake"
var FakeRunnerCLI any // unported

// Replays scripted calls. Each handler receives the request and returns a RunResult.

type FakeRunner struct {
	Handlers *pyjson.Object // {}
	Calls    []*RunRequest  // []
}

// NewFakeRunner is FakeRunner(...).

func NewFakeRunner(handlers *pyjson.Object /* = None */) *FakeRunner {
	panic("unported: runner.FakeRunner.__init__")
}

func (x *FakeRunner) Run(request *RunRequest) *RunResult {
	panic("unported: runner.FakeRunner.run")
}

func (x *FakeRunner) Trail(transcript string) string {
	panic("unported: runner.FakeRunner.trail")
}

// The backend for `provider` (the first Claude account when there is none), signed in with
// `login`.

func GetRunner(cfg *config.Config, provider any /* = None */, login any /* = None */) any {
	panic("unported: runner.get_runner")
}
