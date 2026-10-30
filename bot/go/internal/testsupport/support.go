// Package testsupport is bot/tests/support.py: shared setup, a Config for tests and a Context
// around a FakeGitHub.
//
// Its keyword arguments follow internal/fakes' conventions (its package doc): one trailing
// options struct whose zero fields are the Python defaults, which may be left out
// (MakeConfig(), MakeConfig(Config{Env: env, Machine: Machine}), MakeCtx(gh, Ctx{At: Day,
// Cfg: cfg})), and a trailing positional argument with a default is variadic (IdleFor(ctx),
// IdleFor(ctx, 45); WithLanes(cfg, 3), WithLanes(cfg, 8, 20)).
package testsupport

import (
	"fmt"
	"maps"
	"os"
	"path/filepath"
	"slices"
	"sync"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/trust"
)

// Root = config_mod.REPO_ROOT
var Root = config.RepoRoot

// 2026-09-30 03:00 UTC is 22:00 CDT on the 29th: inside the night window.
var Night = time.Date(2026, 9, 30, 3, 0, 0, 0, time.UTC)

// 2026-09-29 17:00 UTC is 12:00 CDT: outside it.
var Day = time.Date(2026, 9, 29, 17, 0, 0, 0, time.UTC)

// Only the first Claude account's secret is set, as before there were several subscriptions;
// tests of the others set `HARNESS_SECRETS_SET` themselves.
var TestEnv = map[string]string{"GITHUB_RUN_ID": "777", "GITHUB_REPOSITORY": "jgoetzmann/JackiOh",
	"HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN"}

// RawConfig is `.harness/config.json`'s JSON.
func RawConfig() *pyjson.Object {
	return loadObject(filepath.Join(Root, config.ConfigPath))
}

// RawProviders is `.harness/providers.json`'s JSON.
func RawProviders() *pyjson.Object {
	return loadObject(filepath.Join(Root, providers.ProvidersPath))
}

// A weak subscription for the tests alone (#2129): no committed one is weak any more, so the weak
// tier's rules (weak first for an easy item, `easy_first`, `self_check`, the easy rule) keep one
// to hold them to. It is what the weak seat was: on the machine, any hour, no caps, six lanes,
// last in the usage order, its builds checking themselves.
var Weak = "lite"

// WeakModel = "lite-1"
var WeakModel = "lite-1"

// WeakProvider is the weak subscription's entry in providers.json.
var WeakProvider = pyjson.New(
	"note", "The tests' weak subscription (tests/support.py).", "enabled", true, "cli", "codex",
	"family", "lite", "model", WeakModel, "effort", "", "tier", "weak", "self_check", true,
	"easy_first", true, "login", "machine", "runs_on", "night-vm-"+Weak, "lanes", 6,
	"schedule", pyjson.New("mode", "always"), "limits", pyjson.New("mode", "none"), "quiet_check", false)

// `raw` (providers.json's JSON) with the tests' weak subscription (`WEAK`), last in the usage
// order, its model the weak tier's.
func WithWeak(raw *pyjson.Object) *pyjson.Object {
	pyjson.Obj(item(raw, "providers")).Set(Weak, pyjson.DeepCopy(WeakProvider))
	raw.Set("priority", append(slices.Clone(pyjson.List(item(raw, "priority"))), Weak))
	pyjson.Obj(raw.SetDefault("tiers", pyjson.New())).Set("weak", []any{pyjson.New("model", WeakModel)})
	return raw
}

// The machine subscriptions the tests switch on by default: no secret switches them on or off.
// The weak one (`WEAK`, behind them in `priority`) is named by the tests about it, so the tests
// of the others stay about them.
var Machine = []string{"gpt", "agy", "muse"}

// Every subscription logged in on the machine, the weak one's included.
var AllMachine = allMachine()

func allMachine() []string {
	names := []string{}
	pyjson.Obj(item(WithWeak(RawProviders()), "providers")).Items(func(name string, p any) bool {
		if pyjson.Equal(pyjson.Obj(p).Get("login"), "machine") {
			names = append(names, name)
		}
		return true
	})
	return names
}

// The hours every Claude account has in tests unless a test asks for the committed ones.
var NightWindow = pyjson.New("mode", "window", "start", "21:00", "end", "07:00")

// Config is MakeConfig's keyword arguments, and TestPool's (Machine, CommittedHours and the plan
// lanes); a zero field is the default.
type Config struct {
	Root           string            // root = None: Root
	Env            map[string]string // env = None
	Machine        []string          // machine = ()
	CommittedHours bool              // committed_hours = False
	PlanLanes      int               // plan_lanes = 0
	// CommittedPlanLanes is plan_lanes = None: the committed number (PlanLanes is then unread).
	CommittedPlanLanes bool
	Overrides          *pyjson.Object // **overrides: config.json keys, in order
}

func (o Config) planLanes() *int {
	if o.CommittedPlanLanes {
		return nil
	}
	return &o.PlanLanes
}

// The committed subscriptions and the tests' weak one (`WEAK`), with the machine's
// (`"login": "machine"`) switched off
// unless named in `machine`, and on when named. A machine login has no secret to be missing, so it counts as set
// up in every run; tests start, as before the machine, with only the subscriptions whose
// secrets `HARNESS_SECRETS_SET` names.
//
// Every Claude account keeps to the night window, whatever hours the owner commits for it
// (`claude-3` runs all day), so a test of the other subscriptions by day stays about them;
// `committed_hours` keeps the committed ones.
//
// The planning lane (`plan_lanes`) is off, so a test of building finds the builder planning in
// its own run as before; a test of the Needs plan stage turns it on (`plan_lanes=None` keeps the
// committed number).
func TestPool(opts ...Config) *providers.Pool {
	o := one(opts, "TestPool")
	if o.Root != "" || o.Env != nil || o.Overrides != nil {
		panic(fmt.Sprintf("TestPool takes Machine, CommittedHours and the plan lanes, not %+v", o))
	}
	return testPool(o.Machine, o.CommittedHours, o.planLanes())
}

func testPool(machine []string, committedHours bool, planLanes *int) *providers.Pool {
	raw := WithWeak(RawProviders())
	if planLanes != nil {
		raw.Set("plan_lanes", *planLanes)
	}
	pyjson.Obj(item(raw, "providers")).Items(func(name string, p any) bool {
		provider := pyjson.Obj(p)
		if pyjson.Equal(provider.Get("login"), "machine") {
			// Named ones are on whatever the owner commits, so a test of a machine subscription
			// stays about it.
			provider.Set("enabled", slices.Contains(machine, name))
		}
		if pyjson.Equal(provider.Get("cli"), "claude") && !committedHours {
			provider.Set("schedule", NightWindow.Copy())
			provider.Delete("off_hours")
		}
		return true
	})
	return providers.Parse(raw)
}

// `cfg` with other run limits, for a test about lanes filling up. `machineParallel` defaults to
// `maxParallel`.
func WithLanes(cfg *config.Config, maxParallel int, machineParallel ...int) *config.Config {
	machine := optional(machineParallel, "WithLanes", maxParallel)
	pool := *cfg.Pool
	pool.MaxParallel, pool.MachineParallel = maxParallel, machine
	replaced := *cfg
	replaced.Pool = &pool
	return &replaced
}

// `provider` logged in from a secret on GitHub's runners instead of on the machine.
func SecretLogin(provider *providers.Provider, secret string) *providers.Provider {
	replaced := *provider
	replaced.Login, replaced.Secret, replaced.RunsOn = "secret", secret, "ubuntu-latest"
	return &replaced
}

// MakeConfig is the committed config with Config's changes, parsed with TestEnv and TestPool.
func MakeConfig(opts ...Config) *config.Config {
	o := one(opts, "MakeConfig")
	raw := RawConfig()
	raw.Update(o.Overrides)
	root := o.Root
	if root == "" {
		root = Root
	}
	env := maps.Clone(TestEnv)
	maps.Copy(env, o.Env)
	return config.Parse(raw, root, env, testPool(o.Machine, o.CommittedHours, o.planLanes()))
}

// Clock is a clock that reads `At` until a test moves it.
type Clock struct {
	At time.Time
}

// NewClock is Clock(at=NIGHT).
func NewClock(at ...time.Time) *Clock {
	return &Clock{At: optional(at, "NewClock", Night)}
}

// Call is Clock.__call__: the time it reads.
func (c *Clock) Call() time.Time { return c.At }

// clocks is the Clock behind each context MakeCtx built, for ClockOf.
var clocks sync.Map

// ClockOf is `ctx.clock_fn` of a context MakeCtx built, the Clock a test moves:
// `ctx.clock_fn.at = DAY` is `ClockOf(ctx).At = Day`. hctx.Context holds only the func.
func ClockOf(ctx *hctx.Context) *Clock {
	found, _ := clocks.Load(ctx)
	c, ok := found.(*Clock)
	if !ok {
		panic("testsupport.ClockOf: this context's clock is not a Clock MakeCtx made")
	}
	return c
}

// Ctx is MakeCtx's keyword arguments; a zero field is the default.
type Ctx struct {
	At        time.Time      // at = NIGHT (the zero time is Night)
	Cfg       *config.Config // cfg = None: MakeConfig()
	TrustText *string        // trust_text = None: the committed trust list; TrustText: &text
}

// MakeCtx is a Context around `gh` (a new FakeGitHub when nil).
func MakeCtx(gh *fakes.FakeGitHub, opts ...Ctx) *hctx.Context {
	o := one(opts, "MakeCtx")
	if gh == nil {
		gh = fakes.NewFakeGitHub()
	}
	at := o.At
	if at.IsZero() {
		at = Night
	}
	cfg := o.Cfg
	if cfg == nil {
		cfg = MakeConfig()
	}
	var trusted *trust.Trust
	if o.TrustText != nil {
		trusted = trust.TrustParse(*o.TrustText)
	} else {
		trusted = trust.TrustLoad(filepath.Join(Root, config.TrustPath))
	}
	c := NewClock(at)
	ctx := &hctx.Context{
		Cfg:     cfg,
		GH:      gh,
		Store:   state.NewStateStore(gh, config.StateBranch, config.StateFile, sleep),
		Trust:   trusted,
		ClockFn: c.Call,
	}
	clocks.Store(ctx, c)
	return ctx
}

// A free lane has had nothing to start for `minutes` (#797; default 31): a survey is due once
// that is `suggestions.idle_minutes` (30) or more.
func IdleFor(ctx *hctx.Context, minutes ...int) {
	m := optional(minutes, "IdleFor", 31)
	ctx.Store.Update(func(s *pyjson.Object) any {
		pyjson.Obj(item(s, "suggest")).Set("idle_since", clock.ISO(ctx.Now().Add(-time.Duration(m)*time.Minute)))
		return nil
	}, "state")
}

// ---------------------------------------------------------------------------- helpers

// sleep is time.sleep, StateStore's default.
func sleep(seconds float64) { time.Sleep(time.Duration(seconds * float64(time.Second))) }

// loadObject is json.loads(path.read_text(encoding="utf-8")), a JSON object.
func loadObject(file string) *pyjson.Object {
	text, err := os.ReadFile(file)
	if err != nil {
		panic(err)
	}
	v, err := pyjson.Loads(text)
	if err != nil {
		panic(fmt.Sprintf("testsupport: %s: %v", file, err))
	}
	o, ok := v.(*pyjson.Object)
	if !ok {
		panic(fmt.Sprintf("testsupport: %s is not a JSON object", file))
	}
	return o
}

// item is d[key]: a KeyError when absent.
func item(o *pyjson.Object, key string) any {
	v, ok := o.Lookup(key)
	if !ok {
		panic(&fakes.PyError{Type: "KeyError", Msg: py.Repr(key)})
	}
	return v
}

// one is the options struct a function was given, or the zero one (every default).
func one[T any](given []T, fn string) T {
	var zero T
	switch len(given) {
	case 0:
		return zero
	case 1:
		return given[0]
	}
	panic(fmt.Sprintf("testsupport.%s takes one options struct, got %d", fn, len(given)))
}

// optional is a trailing positional argument with a default: the one given, or `def`.
func optional[T any](given []T, fn string, def T) T {
	switch len(given) {
	case 0:
		return def
	case 1:
		return given[0]
	}
	panic(fmt.Sprintf("testsupport.%s takes at most one optional argument, got %d", fn, len(given)))
}
