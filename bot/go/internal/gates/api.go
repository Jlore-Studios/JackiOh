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
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// TAIL_CHARS = 6000
var TailChars int = 6000

// TIMED_OUT = 124
var TimedOut int = 124

// GateResult is gates.GateResult.

type GateResult struct {
	Name         string
	Run          string
	OK           bool
	ExitCode     int
	Seconds      float64
	Tail         string
	PreExisting  bool   // default: False
	Skipped      string // default: ""
	Inconclusive bool   // default: False
}

func (x *GateResult) ToDict() *pyjson.Object {
	panic("unported: gates.GateResult.to_dict")
}

// run_gate is gates.run_gate.

func RunGate(gate *config.Gate, cwd string, env map[string]string, timeoutS *int /* = None */) *GateResult {
	panic("unported: gates.run_gate")
}

// Every gate in order. A gate with no time left is recorded as skipped, never as passed.

func RunAll(gates []*config.Gate, cwd string, env map[string]string, secondsLeft func() float64 /* = None */) []*GateResult {
	panic("unported: gates.run_all")
}

// Mark the gates in `gate_names` that ran out of time as inconclusive. Only the gates: an
// install that times out still fails, since nothing after it can run.

func MarkInconclusive(results []*GateResult, gateNames map[string]bool) {
	panic("unported: gates.mark_inconclusive")
}

// True when every gate passed, was already red on the base, or ran out of time.

func Green(results []*GateResult) bool {
	panic("unported: gates.green")
}

// table is gates.table.

func Table(results []*GateResult) string {
	panic("unported: gates.table")
}

// The output of every gate this change turned red, for the next build pass.

func FailuresText(results []*GateResult) string {
	panic("unported: gates.failures_text")
}
