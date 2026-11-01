// Package state is bot/harness/state.py.
//
// Durable state: one JSON file on the unprotected `bot-state` branch.
//
// Written through the Contents API with the blob sha as a compare-and-set, so the event workflow
// and a night run can both update it without a shared lock: a write that lost the race re-reads
// and re-applies its change.
package state

import (
	"fmt"
	"math/rand/v2"
	"strconv"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// MaxAttempts is MAX_ATTEMPTS.
var MaxAttempts int = 6

// DefaultState is state.default_state.
func DefaultState() *pyjson.Object {
	return pyjson.New(
		"version", 1,
		"halted", false,
		"halt", pyjson.New(),
		// Before there were several subscriptions: the first Claude account's readings, which
		// `providers.record` moves under `providers["claude-1"]`.
		"usage", nil,
		"rate_limited_until", nil,
		"providers", pyjson.New(),
		"items", pyjson.New(),
		"suggest", pyjson.New("last_run", nil, "requested", false),
		"last_run", nil,
	)
}

// Normalise is `data` with every key `default_state` has, so readers never meet a missing key.
func Normalise(data any) *pyjson.Object {
	state := DefaultState()
	if o, ok := data.(*pyjson.Object); ok {
		o.Items(func(key string, value any) bool {
			state.Set(key, value)
			return true
		})
	}
	if _, ok := state.Get("items").(*pyjson.Object); !ok {
		state.Set("items", pyjson.New())
	}
	if _, ok := state.Get("suggest").(*pyjson.Object); !ok {
		state.Set("suggest", pyjson.New("last_run", nil, "requested", false))
	}
	if _, ok := state.Get("providers").(*pyjson.Object); !ok {
		state.Set("providers", pyjson.New())
	}
	return state
}

// StateStore reads and updates the state file on a branch through a `GitHub` client.
type StateStore struct {
	GH     gh.Client
	Branch string
	Path   string
	Sleep  func(float64)
}

// NewStateStore is StateStore(...). A nil sleep is time.sleep.
func NewStateStore(gh gh.Client, branch string /* = STATE_BRANCH */, path string /* = STATE_FILE */, sleep func(float64) /* = time.sleep */) *StateStore {
	if sleep == nil {
		sleep = pySleep
	}
	return &StateStore{GH: gh, Branch: branch, Path: path, Sleep: sleep}
}

// pySleep is time.sleep(seconds).
func pySleep(seconds float64) { time.Sleep(time.Duration(seconds * float64(time.Second))) }

// missing is `text is None` for what get_file returned: the Go client gives ("", "") for
// (None, None), and a file that is there always has a blob sha.
func missing(text, sha string) bool { return text == "" && sha == "" }

// Load is state.StateStore.load.
func (x *StateStore) Load() *pyjson.Object {
	text, sha := x.GH.GetFile(x.Path, x.Branch)
	if missing(text, sha) {
		return DefaultState()
	}
	return Normalise(x.loads(text))
}

// Ensure creates the branch with an empty state when it does not exist. True when created.
func (x *StateStore) Ensure() bool {
	if x.GH.BranchSHA(x.Branch) != "" {
		return false
	}
	x.GH.CreateOrphanBranch(x.Branch, x.Path, dump(DefaultState()), "state: start [skip ci]",
		config.NoDeploy)
	return true
}

// Update applies `change` (which mutates its argument) and writes the result. Returns the state.
func (x *StateStore) Update(change func(*pyjson.Object) any, message string /* = "state" */) *pyjson.Object {
	for range MaxAttempts {
		text, sha := x.GH.GetFile(x.Path, x.Branch)
		absent := missing(text, sha)
		before := DefaultState()
		if !absent {
			before = Normalise(x.loads(text))
		}
		after := pyjson.DeepCopy(before).(*pyjson.Object)
		change(after)
		if pyjson.Equal(after, before) && !absent {
			return after
		}
		if absent && x.GH.BranchSHA(x.Branch) == "" {
			x.Ensure()
			continue
		}
		written := false
		if exc, ok := herr.Catch[*herr.GitHubError](func() {
			x.GH.PutFile(x.Path, dump(after), x.Branch, sha, message+" [skip ci]")
			written = true
		}); ok {
			if exc.Status == 409 || exc.Status == 422 {
				// Two writers raced; back off a little, differently each time, and re-read.
				x.Sleep(uniform(0.05, 0.3))
				continue
			}
			herr.Raise(exc)
		}
		if written {
			return after
		}
	}
	herr.Raise(&herr.StateConflict{Msg: fmt.Sprintf("%s on %s kept changing; gave up after %d attempts",
		x.Path, x.Branch, MaxAttempts)})
	return nil
}

// uniform is random.uniform(a, b).
func uniform(a, b float64) float64 { return a + (b-a)*rand.Float64() }

// loads is the file's JSON. A file that is there but not JSON stops the caller: taken for an
// empty state, the next write put one over the whole of it (2026-10-25).
func (x *StateStore) loads(text string) any {
	v, err := pyjson.LoadsString(text)
	if err != nil {
		herr.Raise(&herr.StateUnreadable{Msg: fmt.Sprintf("%s on %s is not JSON (%d characters read); left as it is",
			x.Path, x.Branch, py.Len(text))})
	}
	return v
}

func dump(state *pyjson.Object) string {
	return pyjson.DumpsWith(state, pyjson.Options{Indent: 2, Indented: true, SortKeys: true}) + "\n"
}

// ---------------------------------------------------------------------- helpers on a state dict

// Item is the record for one issue or PR, created empty when missing.
func Item(state *pyjson.Object, number int) *pyjson.Object {
	return state.Get("items").(*pyjson.Object).SetDefault(strconv.Itoa(number), pyjson.New()).(*pyjson.Object)
}
