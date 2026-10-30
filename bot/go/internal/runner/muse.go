package runner

import (
	"bytes"
	"errors"
	"math"
	"os"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"
	"syscall"
	"time"
	"unicode/utf8"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/logins"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
)

// terminalCodes is what a terminal program writes besides text: control sequences, titles,
// cursor save and restore.
var terminalCodes = regexp.MustCompile(`\x1b\[[0-?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)` +
	`|\x1b[()][0-9A-Za-z]|\x1b[=>78MDEc]`)

// MuseTrust is the question Muse's TUI asks once per workspace.
var MuseTrust string = "Do you trust this workspace"

// MusePrompt is Muse's TUI's prompt.
var MusePrompt string = "❯"

// museWindows are the panel `/usage` draws: each window and the label of its row.
var museWindows = [][2]string{{"five_hour", "Current"}, {"seven_day", "Weekly"}}

// museRow is _MUSE_ROW for a label, without its leading `\b` (findAfterBoundary checks that):
//
//	\b{label}\s+(\d+(?:\.\d+)?)%\s+used(?:[^\S\n]*\W[^\S\n]*Resets\s+(.+?))?[^\S\n]*(?:[│|]|$)
func museRow(label string) *regexp.Regexp {
	return regexp.MustCompile(`(?m)` + label + `[` + py.SpaceClass + `]+(` + py.DigitClass + `+(?:\.` + py.DigitClass +
		`+)?)%[` + py.SpaceClass + `]+used(?:[` + pySpaceNoNL + `]*[^` + py.WordClass + `][` + pySpaceNoNL +
		`]*Resets[` + py.SpaceClass + `]+(.+?))?[` + pySpaceNoNL + `]*(?:[│|]|$)`)
}

var museRows = map[string]*regexp.Regexp{"Current": museRow("Current"), "Weekly": museRow("Weekly")}

// museWeekly is `\bWeekly\s+[\d.]+%\s+used`, without its `\b`.
var museWeekly = regexp.MustCompile(`Weekly[` + py.SpaceClass + `]+[` + py.DigitClass + `.]+%[` + py.SpaceClass + `]+used`)

var months = []string{"jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"}

// screenText is a terminal's output as text: control sequences gone, carriage returns as line
// ends.
func screenText(raw []byte) string {
	text := terminalCodes.ReplaceAllLiteralString(py.DecodeReplace(raw), "")
	return strings.ReplaceAll(text, "\r", "\n")
}

// The reset's time in Muse's words, `(?i)(?:\b([a-z]{3})[a-z]*\.?\s+(\d{1,2}),?\s+)?at\s+(\d{1,2}):
// (\d{2})\s*([ap])\.?m\b`, as two patterns tried at each place in turn, with and without the
// month; the `\b`s are checked by hand. `[a-z]` takes ı and İ as Python's IGNORECASE does.
var (
	museAtMonth = regexp.MustCompile(`^(?i)([a-zı\x{130}]{3})[a-zı\x{130}]*\.?[` + py.SpaceClass + `]+(` +
		py.DigitClass + `{1,2}),?[` + py.SpaceClass + `]+at[` + py.SpaceClass + `]+(` + py.DigitClass + `{1,2}):(` + py.DigitClass +
		`{2})[` + py.SpaceClass + `]*([ap])\.?m`)
	museAt = regexp.MustCompile(`^(?i)at[` + py.SpaceClass + `]+(` + py.DigitClass + `{1,2}):(` + py.DigitClass + `{2})[` +
		py.SpaceClass + `]*([ap])\.?m`)
)

// museAtSearch is that search: the month and day ("" without), hour, minute and `a` or `p`.
func museAtSearch(text string) (month, day, hour, minute, half string, ok bool) {
	for p := 0; p <= len(text); {
		if !beforeIsWord(text, p) {
			if m := museAtMonth.FindStringSubmatch(text[p:]); m != nil && !afterIsWord(text, p+len(m[0])) {
				return m[1], m[2], m[3], m[4], m[5], true
			}
		}
		if m := museAt.FindStringSubmatch(text[p:]); m != nil && !afterIsWord(text, p+len(m[0])) {
			return "", "", m[1], m[2], m[3], true
		}
		if p == len(text) {
			break
		}
		_, size := utf8.DecodeRuneInString(text[p:])
		p += size
	}
	return "", "", "", "", "", false
}

// MuseReset is _muse_reset: a Muse reset as UTC. It prints the machine's own time: "at 6:42 PM"
// (today, or tomorrow once that has passed), "Oct 5 at 12:00 AM", or "in 3h 5m". "" for None.
//
// `now` is read at its offset at that moment, as Python's datetime.now().astimezone() is: a
// fixed offset, never the zone's rules for another day.
func MuseReset(text string, now time.Time) string {
	name, offset := now.Zone()
	now = now.In(time.FixedZone(name, offset))
	var at time.Time
	found := false
	if month, day, hour, minute, half, ok := museAtSearch(text); ok {
		hour24 := py.Atoi(hour) % 12
		if strings.ToLower(half) == "p" {
			hour24 += 12
		}
		at, found = func() (time.Time, bool) {
			mins := py.Atoi(minute)
			if mins > 59 {
				return time.Time{}, false
			}
			at := time.Date(now.Year(), now.Month(), now.Day(), hour24, mins, 0, 0, now.Location())
			if month != "" {
				index := -1
				for i, m := range months {
					if m == strings.ToLower(month) {
						index = i
					}
				}
				d := py.Atoi(day)
				if index < 0 || d < 1 || d > daysIn(at.Year(), index+1) {
					return time.Time{}, false // an unknown month, or a day the month has not
				}
				at = time.Date(at.Year(), time.Month(index+1), d, hour24, mins, 0, 0, now.Location())
				if at.Before(now.Add(-24 * time.Hour)) {
					year := at.Year() + 1
					if year > 9999 || d > daysIn(year, index+1) {
						return time.Time{}, false
					}
					at = time.Date(year, time.Month(index+1), d, hour24, mins, 0, 0, now.Location())
				}
			} else if at.Before(now) {
				at = at.Add(24 * time.Hour)
			}
			return at, true
		}()
	} else if minutes, ok := resetsInMinutes("resets " + text); ok {
		at, found = now.Add(time.Duration(minutes)*time.Minute), true
	}
	if !found {
		return ""
	}
	return at.UTC().Format("2006-01-02T15:04:05Z")
}

// ParseMuseUsage is Muse's `/usage` panel as a reading: "Current 5% used · Resets at 6:42 PM" is
// the 5-hour window, "Weekly 35% used · Resets Oct 5 at 12:00 AM" the week. The newest drawing
// counts.
func ParseMuseUsage(screen string, now time.Time) *pyjson.Object {
	usage := pyjson.New()
	for _, wl := range museWindows {
		window, label := wl[0], wl[1]
		rows := findAfterBoundary(museRows[label], screen)
		if len(rows) == 0 {
			continue
		}
		last := rows[len(rows)-1]
		percent := screen[last[2]:last[3]]
		var resets any
		if last[4] >= 0 {
			if reset := MuseReset(screen[last[4]:last[5]], now); reset != "" {
				resets = reset
			}
		}
		usage.Set(window, pyjson.New("utilization", py.RoundN(math.Min(1.0, py.MustFloat(percent)/100.0), 4),
			"resets_at", resets))
	}
	if usage.Len() == 0 {
		return nil
	}
	return usage.Copy().Set("status", "allowed")
}

// typeInto is _type: text typed into a terminal a character at a time.
func typeInto(f *os.File, text string) {
	for _, char := range text {
		if _, err := f.Write([]byte(string(char))); err != nil {
			herr.Raise(writeError(err))
		}
		time.Sleep(30 * time.Millisecond)
	}
}

// MuseScreen is what Muse's TUI draws after `/usage`, read in a pseudo-terminal: `muse exec` runs
// no slash commands, and neither its JSON events nor its session files carry the subscription's
// usage. It answers the TUI's cursor query, trusts `workspace` the one time it is asked, types
// `/usage` once the prompt is drawn and quiet, and ends the TUI as soon as the week's row is
// drawn. Nothing else is typed, so no model is called. "" when the panel never came.
func MuseScreen(binary string, env map[string]string, workspace string, timeoutS float64 /* = USAGE_TIMEOUT_S */, quietS float64 /* = 1.0 */) string {
	herr.Must(os.MkdirAll(workspace, 0o777))
	master, slave, err := openPTY()
	herr.Must(err)
	setWinsize(slave, 50, 160) // a terminal that will not take a size is read as it is
	childEnv := copyEnv(env)
	childEnv["TERM"], childEnv["COLUMNS"], childEnv["LINES"] = "xterm-256color", "160", "50"
	cmd := command([]string{binary, "--workspace", workspace}, childEnv)
	cmd.Dir = workspace
	cmd.Env = environ(childEnv)
	cmd.Stdin, cmd.Stdout, cmd.Stderr = slave, slave, slave
	cmd.SysProcAttr = &syscall.SysProcAttr{Setsid: true}
	if err := cmd.Start(); err != nil {
		master.Close()
		slave.Close()
		return ""
	}
	slave.Close()
	proc := startWaiting(cmd)
	var raw []byte
	mark, stage := 0, "start"
	heard := time.Now()
	deadline := heard.Add(seconds(timeoutS))
	quietFor := seconds(quietS)
	buf := make([]byte, 65536)
	func() {
		defer func() {
			for _, sig := range []syscall.Signal{syscall.SIGTERM, syscall.SIGKILL} {
				syscall.Kill(-proc.pid, sig) // already gone: nothing to end
				if proc.wait(3 * time.Second) {
					break
				}
			}
			master.Close()
		}()
		for time.Now().Before(deadline) {
			master.SetReadDeadline(time.Now().Add(200 * time.Millisecond))
			n, err := master.Read(buf)
			if n > 0 || (err != nil && !errors.Is(err, os.ErrDeadlineExceeded)) {
				if n == 0 {
					break // the TUI is gone
				}
				chunk := buf[:n]
				raw = append(raw, chunk...)
				heard = time.Now()
				for range bytes.Count(chunk, []byte("\x1b[6n")) { // where is the cursor? top left
					if _, err := master.Write([]byte("\x1b[1;1R")); err != nil {
						herr.Raise(writeError(err))
					}
				}
				continue
			}
			since := screenText(raw[mark:])
			if stage == "asked" && len(findAfterBoundary(museWeekly, since)) > 0 {
				if time.Since(heard) >= min(quietFor, 500*time.Millisecond) {
					break
				}
				continue
			}
			if time.Since(heard) < quietFor {
				continue
			}
			if stage == "start" && strings.Contains(since, MuseTrust) {
				typeInto(master, "1\r")
				stage, mark = "trusted", len(raw)
			} else if (stage == "start" || stage == "trusted") && strings.Contains(since, MusePrompt) {
				typeInto(master, "/usage")
				time.Sleep(300 * time.Millisecond)
				typeInto(master, "\r")
				stage, mark = "asked", len(raw)
			}
		}
	}()
	if stage == "asked" {
		return screenText(raw[mark:])
	}
	return ""
}

// MuseCliCLI is MuseCli.cli.
var MuseCliCLI = "muse"

// MuseCliPollsUsage is MuseCli.polls_usage.
var MuseCliPollsUsage = true

// MuseCli is Meta's Muse Code CLI (`muse exec`). Its call reports no usage, so the TUI's `/usage`
// panel is read around and during each call (`polls_usage`), in a workspace of its own under the
// user's cache, trusted once.
type MuseCli struct {
	cliBase
	// UsageDir is where the TUI is started to read `/usage`; `~/.cache/night-bot/muse-usage`
	// when unset.
	UsageDir string // default: None ("")
}

// NewMuseCli is MuseCli(binary, login, popen); a nil popen is subprocess.Popen.
func NewMuseCli(binary string, login *logins.Login /* = None */, popen Popen /* = subprocess.Popen */) *MuseCli {
	x := &MuseCli{cliBase: newCliBase(binary, login, popen, MuseCliCLI)}
	x.self = x
	return x
}

// PollsUsage is MuseCli.polls_usage.
func (x *MuseCli) PollsUsage() bool { return MuseCliPollsUsage }

// ReadUsage is MuseCli.read_usage.
func (x *MuseCli) ReadUsage(model string) *pyjson.Object {
	env := x.Env()
	workspace := x.UsageDir
	if workspace == "" {
		home := env["HOME"]
		if home == "" {
			home = homeDir()
		}
		workspace = filepath.Join(home, ".cache", "night-bot", "muse-usage")
	}
	return ParseMuseUsage(MuseScreen(x.Binary, env, workspace, float64(UsageTimeoutS), 1.0), time.Now())
}

// Argv is MuseCli.argv.
func (x *MuseCli) Argv(request *RunRequest, promptFile string) []string {
	argv := []string{x.Binary, "exec", "--yolo", "--disable-web-tools",
		"--workspace", request.Cwd, "--model", request.Model,
		"--prompt-file", promptFile, "--max-model-steps", strconv.Itoa(request.MaxTurns)}
	if request.Effort != "" {
		argv = append(argv, "--reasoning-effort", request.Effort)
	}
	return argv
}

// Run is MuseCli.run.
func (x *MuseCli) Run(request *RunRequest) *RunResult {
	transcript, raw := x.paths(request)
	promptFile := withSuffix(transcript, ".prompt.md")
	writeText(promptFile, x.prompt(request))
	launch, failed := x.launch(x.Argv(request, promptFile), request, x.Env(), nil, raw)
	if failed != nil {
		return failed
	}
	text := py.Strip(Read(raw))
	stderr := launch.stderr
	transcriptText := text
	if py.Strip(stderr) != "" {
		transcriptText += "\n\n--- stderr ---\n" + stderr
	}
	writeText(raw, transcriptText)
	x.keep(launch, transcript)
	if launch.stopped != "" {
		r := NewRunResult(false, text)
		r.ExitCode, r.DurationS = launch.code, launch.elapsed
		r.Error = "stopped mid-call: " + launch.stopped
		r.Extra = pyjson.New("usage_stop", launch.stopped)
		return x.withUsage(r, request)
	}
	if launch.timedOut {
		r := NewRunResult(false, text)
		r.ExitCode, r.DurationS = ExitTimeout, launch.elapsed
		r.Error = "timed out after " + strconv.Itoa(request.TimeoutS) + "s"
		r.TimedOut = true
		return x.withUsage(r, request)
	}
	ok := launch.code == 0
	errText := ""
	if !ok {
		errText = redact.Redact(py.SliceFrom(StderrError(stderr, "muse exited "+strconv.Itoa(launch.code)), -2000), nil)
	}
	resetAt := ""
	if !ok && RateLimitWords.MatchString(errText) {
		resetAt = ParkFor(errText)
	}
	// A refusal is never a CLI that could not run, whatever its exit code says.
	r := NewRunResult(ok, text)
	r.ExitCode, r.DurationS, r.Error, r.ResetAt = launch.code, launch.elapsed, errText, resetAt
	r.InfraHint = launch.code == 2 && resetAt == ""
	return x.withUsage(r, request)
}
