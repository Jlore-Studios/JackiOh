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
	"math"
	"regexp"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
)

// Words is a case-insensitive search for any of a list of phrases (RATE_LIMIT_WORDS and
// AUTH_WORDS), as Python's re.IGNORECASE matches them: its MatchString is
// `bool(PATTERN.search(text))`.
type Words struct{ re *regexp.Regexp }

// MatchString is whether text holds any of the words.
func (w *Words) MatchString(text string) bool { return w.re.MatchString(foldI(text)) }

// RateLimitWords is RATE_LIMIT_WORDS.
var RateLimitWords = &Words{regexp.MustCompile(
	`(?i)(usage limit|rate limit|too many requests|limit reached|hit your (?:[` + py.WordClass + `]+ )?limit` +
		`|quota exceeded|exhausted your (?:[` + py.WordClass + `]+ )?quota|resource_exhausted|terminalquotaerror` +
		// agy, once its plan's allowance is spent: "Individual quota reached. Please upgrade your
		// subscription to increase your limits. Resets in 1h44m44s."
		`|quota reached|upgrade your subscription to increase your limits` +
		// Muse, once its week is spent: "API error 429 […]: Subscription quota exhausted. Your usage
		// window resets at 2026-10-26T00:00:00Z. (rate_limit_error)". Unread, six runs on 2026-10-25
		// ended as infra and handed their items on (reviews/2026-10-27-muse-usage.md, finding 3).
		`|quota exhausted|rate_limit_error)`)}

// ResetsAt is RESETS_AT: "resets at 2026-10-26T00:00:00Z" (Muse): when a refusal ends, as a
// time.
var ResetsAt = regexp.MustCompile(`(?i)resets? at[` + py.SpaceClass + `]+(` + py.DigitClass + `{4}-` + py.DigitClass +
	py.DigitClass + `-` + py.DigitClass + py.DigitClass + `T` + py.DigitClass + py.DigitClass + `:` + py.DigitClass + py.DigitClass + `(?::` +
	py.DigitClass + py.DigitClass + `(?:\.` + py.DigitClass + `+)?)?(?:Z|[+-]` + py.DigitClass + py.DigitClass + `:?` + py.DigitClass +
	py.DigitClass + `))`)

// ResetsIn is RESETS_IN: "Resets in 1h44m44s" (agy), "resets in 3h", "try again in 5 days 2 hours
// 22 minutes" (Codex): how long a refusal lasts. The Python is the regular expression
//
//	(?i)(?:resets?|try again) in\s*(?:(\d+)\s*d(?:ays?)?\b)?[\s,]*(?:(\d+)\s*h(?:ours?)?\b|(\d+)h)?
//	[\s,]*(?:(\d+)\s*m(?:in(?:ute)?s?)?(?![a-z])|(\d+)m(?=\d))?[\s,]*(?:(\d+)\s*s)?
//
// whose lookarounds and Unicode `\b` RE2 lacks, so it is matched by hand, part by part, in the
// order Python's backtracking tries them.
var ResetsIn = resetsInPattern{}

type resetsInPattern struct{}

// FindStringSubmatch is RESETS_IN.search(text): nil without a match, else the match and its six
// groups (days, hours, hours, minutes, minutes, seconds), "" where a group took no part.
func (resetsInPattern) FindStringSubmatch(text string) []string {
	rs := []rune(text)
	n := len(rs)
	lit := func(p int, word string) (int, bool) {
		for _, c := range word {
			if p >= n || fold(rs[p]) != c {
				return p, false
			}
			p++
		}
		return p, true
	}
	skip := func(p int, ok func(rune) bool) int {
		for p < n && ok(rs[p]) {
			p++
		}
		return p
	}
	spaceOrComma := func(r rune) bool { return py.IsSpace(r) || r == ',' }
	digits := func(p int) (string, int) {
		e := skip(p, py.IsDigit)
		return string(rs[p:e]), e
	}
	head := func(p int) (int, bool) {
		if e, ok := lit(p, "reset"); ok {
			if e2, ok := lit(e, "s"); ok {
				if e3, ok := lit(e2, " in"); ok {
					return e3, true
				}
			}
			return lit(e, " in")
		}
		if e, ok := lit(p, "try again"); ok {
			return lit(e, " in")
		}
		return p, false
	}
	start, pos := -1, 0
	for p := 0; p <= n; p++ {
		if e, ok := head(p); ok {
			start, pos = p, e
			break
		}
	}
	if start < 0 {
		return nil
	}
	boundary := func(p int) bool { return p >= n || !py.IsWord(rs[p]) }
	// unit is `(\d+)\s*<letter>(?:<rest>)?` and what may follow it: the longest of `options`
	// that is there, tried in order, its end then checked by `follows`.
	unit := func(p int, letter string, options []string, follows func(int) bool) (string, int, bool) {
		num, q := digits(p)
		if num == "" {
			return "", p, false
		}
		q, ok := lit(skip(q, py.IsSpace), letter)
		if !ok {
			return "", p, false
		}
		for _, option := range options {
			if e, ok := lit(q, option); ok && follows(e) {
				return num, e, true
			}
		}
		return "", p, false
	}
	var groups [6]string
	pos = skip(pos, py.IsSpace)
	if num, e, ok := unit(pos, "d", []string{"ays", "ay", ""}, boundary); ok {
		groups[0], pos = num, e
	}
	pos = skip(pos, spaceOrComma)
	if num, e, ok := unit(pos, "h", []string{"ours", "our", ""}, boundary); ok {
		groups[1], pos = num, e
	} else if num, q := digits(pos); num != "" {
		if e, ok := lit(q, "h"); ok {
			groups[2], pos = num, e
		}
	}
	pos = skip(pos, spaceOrComma)
	notLetter := func(p int) bool { return p >= n || !isAZ(rs[p]) }
	if num, e, ok := unit(pos, "m", []string{"inutes", "inute", "ins", "in", ""}, notLetter); ok {
		groups[3], pos = num, e
	} else if num, q := digits(pos); num != "" {
		if e, ok := lit(q, "m"); ok && e < n && py.IsDigit(rs[e]) {
			groups[4], pos = num, e
		}
	}
	pos = skip(pos, spaceOrComma)
	if num, e, ok := unit(pos, "s", []string{""}, func(int) bool { return true }); ok {
		groups[5], pos = num, e
	}
	return append([]string{string(rs[start:pos])}, groups[:]...)
}

// WatchSeconds is how often a running call's usage is looked at (`_Cli._launch`'s watcher).
var WatchSeconds float64 = 15

// PollSeconds is how often a running call's usage is read again with the CLI's own command, for
// a CLI whose call streams none (agy, Muse): each reading starts a process of its own.
var PollSeconds float64 = 600

// UsageTimeoutS is how long one such reading may take before it is given up.
var UsageTimeoutS int = 90

// AuthWords is AUTH_WORDS.
var AuthWords = &Words{regexp.MustCompile(
	`(?i)(invalid api key|authentication[_ ]error|oauth token|not logged in|please run /login` +
		`|invalid bearer|401 unauthorized|credit balance is too low|sign in again` +
		`|refresh token (?:has expired|was already used|was revoked)|could not be refreshed` +
		`|manual authorization is required|no meta credentials|api key from meta_api_key was rejected` +
		"|run `?muse login|authentication required" +
		// Claude Code's answer when its token is revoked or mistyped: "Failed to authenticate. API
		// Error: 401 OAuth access token is invalid." A model never says that on its own.
		`|failed to authenticate|access token is invalid|api error: 401)`)}

// UsageWindows is USAGE_WINDOWS.
var UsageWindows = []string{"five_hour", "seven_day"}

// CallMarker is set in every model call's environment, so the processes it leaves behind can be
// found.
var CallMarker string = "JACKIOH_BOT_CALL"

// ActionsFiles are the Actions file-command variables: a process holding one can set the
// environment, path or outputs of the job's later steps, so the model never sees them.
var ActionsFiles = []string{"GITHUB_ENV", "GITHUB_PATH", "GITHUB_OUTPUT", "GITHUB_STATE", "GITHUB_STEP_SUMMARY"}

// QuietEnv is no telemetry, error reporting or update checks: the session talks to the API and
// nothing else.
var QuietEnv = map[string]string{"CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1", "DISABLE_AUTOUPDATER": "1"}

// ExitTimeout is EXIT_TIMEOUT.
var ExitTimeout int = 124

// ExitNotFound is EXIT_NOT_FOUND.
var ExitNotFound int = 127

// defaultPark is DEFAULT_PARK: when a refusal names no reset time, try again after this long.
// It lives in providers (providers.DefaultPark), which the Python imported from here lazily: the one
// import cycle Go would not allow.
var defaultPark = providers.DefaultPark

// ErrorLines is how many of a failed CLI's last stderr lines its error keeps (`stderr_error`).
var ErrorLines int = 20

// jsonMessage is the message in a JSON error body: its `message`, `error`, `detail` or
// `error_description`, looked for inside a nested `error` object too.
func jsonMessage(body any) string {
	if s, ok := body.(string); ok {
		return py.Strip(s)
	}
	obj, ok := body.(*pyjson.Object)
	if !ok {
		return ""
	}
	for _, key := range []string{"message", "error", "detail", "error_description"} {
		value := obj.Get(key)
		found := ""
		switch value.(type) {
		case *pyjson.Object, string:
			found = jsonMessage(value)
		}
		if found != "" {
			return found
		}
	}
	return ""
}

// StderrError is what a failed CLI said on stderr: the message of the JSON error body it ended
// with, when it ended with one (a CLI may print `{"error": {"message": ...}}` over several lines,
// so its last line alone is a bare `}`, #317 part 1), else its last `ERROR_LINES` lines, else
// `fallback`.
func StderrError(stderr string, fallback string) string {
	var lines []string
	for _, line := range py.Splitlines(stderr) {
		if py.Strip(line) != "" {
			lines = append(lines, line)
		}
	}
	if len(lines) == 0 {
		return fallback
	}
	for start := len(lines) - 1; start >= 0; start-- {
		if !strings.HasPrefix(py.Lstrip(lines[start]), "{") {
			continue
		}
		body, err := pyjson.LoadsString(strings.Join(lines[start:], "\n"))
		if err != nil {
			continue
		}
		if message := jsonMessage(body); message != "" {
			return message
		}
		break
	}
	return strings.Join(lines[max(0, len(lines)-ErrorLines):], "\n")
}

// resetsInMinutes is the minutes a "resets in 1h44m44s" names, a started minute counted whole;
// false for none.
func resetsInMinutes(text string) (int, bool) {
	match := ResetsIn.FindStringSubmatch(text)
	if match == nil {
		return 0, false
	}
	named := false
	var g [6]int
	for i, s := range match[1:] {
		if s != "" {
			named = true
			g[i] = py.Atoi(s)
		}
	}
	if !named {
		return 0, false
	}
	days, hours, hours2, minutes, minutes2, secs := g[0], g[1], g[2], g[3], g[4], g[5]
	extra := 0
	if secs != 0 {
		extra = 1
	}
	return max(1, (days*24+hours+hours2)*60+minutes+minutes2+extra), true
}

// ParkFor is how long a refused CLI is left alone: until the reset its message names (as a time,
// or as a wait plus a minute), or an hour when it names none.
func ParkFor(error_ string) string {
	if at := ResetsAt.FindStringSubmatch(error_); at != nil {
		if resets, ok := fromISOFormat(strings.ReplaceAll(at[1], "Z", "+00:00")); ok {
			return resets.UTC().Format("2006-01-02T15:04:05Z")
		}
	}
	total, ok := resetsInMinutes(error_)
	if !ok {
		return defaultPark
	}
	return "+PT" + py.Str(total+1) + "M"
}

// fromISOFormat is datetime.fromisoformat for what ResetsAt captures (a date, `T` or `t`, a time
// to the minute or finer, an offset): false where Python raises ValueError (a digit not ASCII, a
// field out of range, an offset a day or more).
func fromISOFormat(s string) (time.Time, bool) {
	for i := 0; i < len(s); i++ {
		if s[i] > 0x7f {
			return time.Time{}, false
		}
	}
	if len(s) < 16 {
		return time.Time{}, false
	}
	year, month, day := py.Atoi(s[0:4]), py.Atoi(s[5:7]), py.Atoi(s[8:10])
	hour, minute, second := py.Atoi(s[11:13]), py.Atoi(s[14:16]), 0
	rest := s[16:]
	if strings.HasPrefix(rest, ":") && len(rest) >= 3 {
		second, rest = py.Atoi(rest[1:3]), rest[3:]
		if strings.HasPrefix(rest, ".") {
			i := 1
			for i < len(rest) && rest[i] >= '0' && rest[i] <= '9' {
				i++
			}
			rest = rest[i:]
		}
	}
	// What is left is the offset, "+00:00" for a "Z"; a lower-case "z" Python does not read.
	if rest == "" || (rest[0] != '+' && rest[0] != '-') {
		return time.Time{}, false
	}
	sign := 1
	if rest[0] == '-' {
		sign = -1
	}
	hhmm := strings.ReplaceAll(rest[1:], ":", "")
	if len(hhmm) != 4 {
		return time.Time{}, false
	}
	offset := py.Atoi(hhmm[0:2])*60 + py.Atoi(hhmm[2:4])
	if year < 1 || month < 1 || month > 12 || day < 1 || day > daysIn(year, month) ||
		hour > 23 || minute > 59 || second > 59 || offset >= 24*60 {
		return time.Time{}, false
	}
	zone := time.FixedZone("", sign*offset*60)
	return time.Date(year, time.Month(month), day, hour, minute, second, 0, zone), true
}

// daysIn is the number of days in a month of the Gregorian calendar.
func daysIn(year, month int) int {
	return time.Date(year, time.Month(month)+1, 0, 0, 0, 0, 0, time.UTC).Day()
}

// TrailEntries and TrailChars are how much of a transcript the next agent is shown (`trail`).
var TrailEntries int = 40

// TrailChars: see TrailEntries.
var TrailChars int = 6000

// CLINote is CLI_NOTE.
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
	Transcript      string // default: None ("")
	// ReadOnly is a reviewer: may read and run things but not change them (the harness resets it
	// anyway).
	ReadOnly bool // default: False
	// ExtraDirs are directories outside `cwd` the session may need to write (a worktree's git
	// directory).
	ExtraDirs []string // default: ()
	// UsageStop is asked with each usage reading the CLI streams while the call runs (Claude's
	// `rate_limit_event`): a reason stops the call there, "" (None) lets it go on.
	UsageStop func(*pyjson.Object) string // default: None
}

// RunResult is runner.RunResult. Build one with NewRunResult.
type RunResult struct {
	OK        bool
	Text      string
	ExitCode  int            // default: 0
	Turns     *int           // default: None
	DurationS float64        // default: 0.0
	Error     string         // default: None ("")
	Usage     *pyjson.Object // default: None
	ResetAt   string         // default: None ("")
	TimedOut  bool           // default: False
	Extra     *pyjson.Object // default: field(default_factory=dict)
	// InfraHint is the backend knows the CLI could not run at all (its exit code says so).
	InfraHint bool // default: False
	// Tokens are this call's token counts ({input, output, cache_read, cache_write}), when its
	// CLI reports any (#695: Claude's result event, Codex's turn.completed, agy's result).
	Tokens *pyjson.Object // default: None
	// CostUSD is this call's cost in USD, when its CLI reports one (Claude's `total_cost_usd`).
	CostUSD *float64 // default: None
}

// NewRunResult is RunResult(ok, text) with the dataclass's defaults; set the other fields on
// what it returns.
func NewRunResult(ok bool, text string) *RunResult {
	return &RunResult{OK: ok, Text: text, Extra: pyjson.New()}
}

// Infra is whether the CLI could not run at all: missing, or refused by authentication.
func (x *RunResult) Infra() bool {
	if x.OK {
		return false
	}
	return x.InfraHint || x.ExitCode == ExitNotFound || AuthWords.MatchString(x.Error)
}

// RateLimited is RunResult.rate_limited.
func (x *RunResult) RateLimited() bool {
	if x.ResetAt != "" {
		return true
	}
	if x.Usage != nil && pyjson.Equal(x.Usage.Get("status"), "rejected") {
		return !x.OK
	}
	return !x.OK && RateLimitWords.MatchString(x.Error)
}

// epochISO is _epoch_iso: a reset time as ISO UTC (seconds since the epoch, as a number or
// digits) or the text itself; false for None.
func epochISO(value any) (string, bool) {
	switch v := value.(type) {
	case int:
		return unixISO(float64(v)), true
	case float64:
		return unixISO(v), true
	case string:
		s := py.Strip(v)
		if s == "" {
			return "", false
		}
		if py.IsDecimal(s) {
			return epochISO(py.Atoi(s))
		}
		return s, true
	}
	return "", false
}

// unixISO is datetime.fromtimestamp(t, timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"), whose
// microseconds Python rounds half to even before the seconds are shown.
func unixISO(t float64) string {
	sec, frac := math.Modf(t)
	us := math.RoundToEven(frac * 1e6)
	if us >= 1e6 {
		sec++
	} else if us < 0 {
		sec--
	}
	return time.Unix(int64(sec), 0).UTC().Format("2006-01-02T15:04:05Z")
}

// isoOrNone is a reset time as it goes into a reading: the text, or None.
func isoOrNone(value any) any {
	if s, ok := epochISO(value); ok {
		return s
	}
	return nil
}

// UsageFromEvent is one `rate_limit_event` as `{five_hour: {...}, seven_day: {...}, status}`,
// or None.
func UsageFromEvent(event *pyjson.Object) *pyjson.Object {
	info, ok := event.Get("rate_limit_info").(*pyjson.Object)
	if !ok || info == nil {
		return nil
	}
	usage := pyjson.New()
	if windows, ok := info.Get("unifiedWindows").(*pyjson.Object); ok && windows != nil {
		for _, name := range UsageWindows {
			window, ok := windows.Get(name).(*pyjson.Object)
			if utilization, isNumber := py.Number(window.Get("utilization")); ok && window != nil && isNumber {
				usage.Set(name, pyjson.New(
					"utilization", utilization,
					"resets_at", isoOrNone(window.Get("resetsAt"))))
			}
		}
	}
	if status, ok := info.Get("status").(string); ok {
		usage.Set("status", status)
		if status == "rejected" {
			if info.Truthy("isUsingOverage") || info.Truthy("overageInUse") {
				usage.Set("status", "allowed_overage")
			} else {
				usage.Set("rejected_resets_at", isoOrNone(info.Get("resetsAt")))
			}
		}
	}
	if usage.Len() == 0 {
		return nil
	}
	return usage
}

// jsonLine is one line as _json_lines reads it: the JSON object it holds, or nil.
func jsonLine(line string) *pyjson.Object {
	text := py.Strip(line)
	if !strings.HasPrefix(text, "{") {
		return nil
	}
	event, err := pyjson.LoadsString(text)
	if err != nil {
		return nil
	}
	obj, _ := event.(*pyjson.Object)
	return obj
}

// jsonLines is _json_lines: the JSON objects among lines.
func jsonLines(lines []string) []*pyjson.Object {
	var out []*pyjson.Object
	for _, line := range lines {
		if event := jsonLine(line); event != nil {
			out = append(out, event)
		}
	}
	return out
}

// streamParser is parse_stream's loop, fed a line at a time.
type streamParser struct{ result, usage *pyjson.Object }

func (p *streamParser) add(line string) {
	event := jsonLine(line)
	if event == nil {
		return
	}
	switch {
	case pyjson.Equal(event.Get("type"), "result"):
		p.result = event
	case pyjson.Equal(event.Get("type"), "rate_limit_event"):
		if found := UsageFromEvent(event); found != nil {
			p.usage = found
		}
	}
}

// ParseStream is `(result line, last usage reading)` from Claude's stream-json lines.
func ParseStream(lines []string) (*pyjson.Object, *pyjson.Object) {
	var p streamParser
	for _, line := range lines {
		p.add(line)
	}
	return p.result, p.usage
}

// Clip is _clip: text on one line, cut to `limit` characters with an ellipsis.
func Clip(text any, limit int /* = 300 */) string {
	shown := ""
	if pyjson.Truthy(text) {
		shown = py.Str(text)
	}
	flat := strings.Join(py.Split(shown), " ")
	if py.Len(flat) <= limit {
		return flat
	}
	return py.SliceTo(flat, limit-1) + "…"
}

// trailText is _trail_text: the last entries, as a list, redacted.
func trailText(entries []string) string {
	shown := entries[max(0, len(entries)-TrailEntries):]
	lines := make([]string, len(shown))
	for i, e := range shown {
		lines[i] = "- " + e
	}
	return redact.Redact(py.SliceFrom(strings.Join(lines, "\n"), -TrailChars), nil)
}

// Read is _read: a file's text, "" when it cannot be read.
func Read(path string) string {
	text, err := py.ReadText(path)
	if err != nil {
		return ""
	}
	return text
}
