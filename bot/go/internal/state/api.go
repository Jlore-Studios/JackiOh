// Package state is bot/harness/state.py.
//
// Durable state: one JSON file on the unprotected `bot-state` branch.
//
// Written through the Contents API with the blob sha as a compare-and-set, so the event workflow
// and a night run can both update it without a shared lock: a write that lost the race re-reads
// and re-applies its change.
package state

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// MAX_ATTEMPTS = 6
var MaxAttempts int = 6

// default_state is state.default_state.

func DefaultState() *pyjson.Object {
	panic("unported: state.default_state")
}

// `data` with every key `default_state` has, so readers never meet a missing key.

func Normalise(data any) *pyjson.Object {
	panic("unported: state.normalise")
}

// Reads and updates the state file on a branch through a `GitHub` client.

type StateStore struct {
	GH     gh.Client // gh
	Branch any       // branch
	Path   any       // path
	Sleep  any       // sleep
}

// NewStateStore is StateStore(...).

func NewStateStore(gh gh.Client, branch string /* = STATE_BRANCH */, path string /* = STATE_FILE */, sleep func(float64) /* = time.sleep */) *StateStore {
	panic("unported: state.StateStore.__init__")
}

func (x *StateStore) Load() *pyjson.Object {
	panic("unported: state.StateStore.load")
}

// Create the branch with an empty state when it does not exist. True when created.

func (x *StateStore) Ensure() bool {
	panic("unported: state.StateStore.ensure")
}

// Apply `change` (which mutates its argument) and write the result. Returns the state.

func (x *StateStore) Update(change func(*pyjson.Object) any, message string /* = "state" */) *pyjson.Object {
	panic("unported: state.StateStore.update")
}

// The file's JSON. A file that is there but not JSON stops the caller: taken for an
// empty state, the next write put one over the whole of it (2026-10-25).

func (x *StateStore) loads(text string) any {
	panic("unported: state.StateStore._loads")
}

// The record for one issue or PR, created empty when missing.

func Item(state *pyjson.Object, number int) *pyjson.Object {
	panic("unported: state.item")
}
