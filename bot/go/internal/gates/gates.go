// Package gates is bot/harness/gates.py.
//
// The repository's own checks, run in a worktree after each build pass.
//
// The list is `gates` in `.harness/config.json`. It is never widened by the bot: a red gate the
// bot cannot fix is reported, not skipped. A gate that is also red on the untouched base is marked
// pre-existing, so the change is not blamed for it. A gate that runs out of time is inconclusive: it
// said nothing about the change, so it neither blocks it nor runs again on the base (which would only
// time out too); CI on the pull request runs every check before anything merges.
package gates

import (
	"fmt"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
)

// TailChars is TAIL_CHARS.
var TailChars int = 6000

// TimedOut is the exit code `run_gate` gives a gate that ran out of time (as `timeout(1)` does).
var TimedOut int = 124

// GateResult is gates.GateResult.
type GateResult struct {
	Name        string
	Run         string
	OK          bool
	ExitCode    int
	Seconds     float64
	Tail        string
	PreExisting bool   // default: False
	Skipped     string // default: ""
	// Inconclusive: it ran out of time, so it says nothing either way (`mark_inconclusive`).
	Inconclusive bool // default: False
}

// ToDict is gates.GateResult.to_dict (dataclasses.asdict).
func (x *GateResult) ToDict() *pyjson.Object {
	return pyjson.New("name", x.Name, "run", x.Run, "ok", x.OK, "exit_code", x.ExitCode,
		"seconds", x.Seconds, "tail", x.Tail, "pre_existing", x.PreExisting, "skipped", x.Skipped,
		"inconclusive", x.Inconclusive)
}

// RunGate is gates.run_gate; a variable so the tests can replace it. A nil timeoutS is None.
var RunGate = runGate

func runGate(gate *config.Gate, cwd string, env map[string]string, timeoutS *int /* = None */) *GateResult {
	started := time.Now()
	limit := gate.TimeoutMinutes * 60
	if timeoutS != nil && *timeoutS != 0 {
		limit = min(gate.TimeoutMinutes*60, *timeoutS)
	}
	var output string
	var code int
	proc := run([]string{"bash", "-c", gate.Run}, cwd, env, time.Duration(max(30, limit))*time.Second)
	if !proc.timedOut {
		output = py.Universal(py.DecodeReplace(proc.stdout))
		if len(proc.stderr) > 0 {
			output += "\n" + py.Universal(py.DecodeReplace(proc.stderr))
		}
		code = proc.code
	} else {
		// The output read before the timeout, as bytes: subprocess decodes none of it.
		output = py.DecodeReplace(proc.stdout) + "\n" + py.DecodeReplace(proc.stderr) + fmt.Sprintf("\n(timed out after %ds)", limit)
		code = TimedOut
	}
	return &GateResult{
		Name:     gate.Name,
		Run:      gate.Run,
		OK:       code == 0,
		ExitCode: code,
		Seconds:  py.RoundN(time.Since(started).Seconds(), 1),
		Tail:     redact.Redact(py.SliceFrom(output, -TailChars), nil),
	}
}

// RunAll is every gate in order. A gate with no time left is recorded as skipped, never as passed.
// A nil secondsLeft is None.
func RunAll(gates []*config.Gate, cwd string, env map[string]string, secondsLeft func() float64 /* = None */) []*GateResult {
	results := []*GateResult{}
	for _, gate := range gates {
		if secondsLeft != nil {
			left := secondsLeft()
			if left < 60 {
				results = append(results, &GateResult{Name: gate.Name, Run: gate.Run, OK: false, ExitCode: -1,
					Seconds: 0.0, Tail: "", Skipped: "no time left"})
				continue
			}
			seconds := int(left)
			results = append(results, RunGate(gate, cwd, env, &seconds))
			continue
		}
		results = append(results, RunGate(gate, cwd, env, nil))
	}
	return results
}

// MarkInconclusive marks the gates in `gate_names` that ran out of time as inconclusive. Only the
// gates: an install that times out still fails, since nothing after it can run.
func MarkInconclusive(results []*GateResult, gateNames map[string]bool) {
	for _, result := range results {
		if gateNames[result.Name] && result.ExitCode == TimedOut && result.Skipped == "" {
			result.Inconclusive = true
		}
	}
}

// Green is true when every gate passed, was already red on the base, or ran out of time.
func Green(results []*GateResult) bool {
	for _, r := range results {
		if !(r.OK || r.PreExisting || r.Inconclusive) {
			return false
		}
	}
	return true
}

// Table is gates.table.
func Table(results []*GateResult) string {
	if len(results) == 0 {
		return "_No gates ran._"
	}
	rows := []string{"| Gate | Result | Time |", "|---|---|---|"}
	for _, r := range results {
		var verdict string
		switch {
		case r.Skipped != "":
			verdict = "skipped (" + r.Skipped + ")"
		case r.OK:
			verdict = "pass"
		case r.Inconclusive:
			verdict = "timed out: inconclusive, left to CI"
		case r.PreExisting:
			verdict = fmt.Sprintf("fail (exit %d), also red on main", r.ExitCode)
		default:
			verdict = fmt.Sprintf("**fail** (exit %d)", r.ExitCode)
		}
		rows = append(rows, fmt.Sprintf("| %s (`%s`) | %s | %.0fs |", r.Name, r.Run, verdict, r.Seconds))
	}
	return strings.Join(rows, "\n")
}

// FailuresText is the output of every gate this change turned red, for the next build pass.
func FailuresText(results []*GateResult) string {
	parts := []string{}
	for _, r := range results {
		if r.OK || r.PreExisting || r.Inconclusive {
			continue
		}
		tail := r.Tail
		if tail == "" {
			tail = r.Skipped
		}
		parts = append(parts, fmt.Sprintf("### %s: `%s` exited %d\n\n%s", r.Name, r.Run, r.ExitCode, tail))
	}
	return strings.Join(parts, "\n\n")
}
