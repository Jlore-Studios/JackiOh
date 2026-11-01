// Package hctx is bot/harness/context.py.
//
// What every command works with: config, a GitHub client, the state store, the trust list.
package hctx

import (
	"path/filepath"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/trust"
)

// Context is context.Context.
type Context struct {
	Cfg     *config.Config
	GH      gh.Client
	Store   *state.StateStore
	Trust   *trust.Trust
	ClockFn func() time.Time
	// ActionsGH is the client for Actions calls (starting a run, re-running CI jobs). In a
	// workflow it holds the job's own token, which has `actions: write`, so the bot's token needs
	// no `repo` scope. nil is None.
	ActionsGH gh.Client // default: None
}

// Act is the client Actions calls go through: ActionsGH when there is one, else GH.
func (x *Context) Act() gh.Client {
	if x.ActionsGH != nil {
		return x.ActionsGH
	}
	return x.GH
}

func (x *Context) Now() time.Time {
	return x.ClockFn()
}

// Window is the Python's `window` property, which reads `cfg.window_start` and
// `cfg.window_end`: Config has neither any more (the hours are each subscription's,
// `providers.Schedule.window`), so it raises AttributeError, and nothing calls it. Ported as it
// is.
func (x *Context) Window() *clock.Window {
	herr.Raise(&py.Error{Type: "AttributeError", Msg: "'Config' object has no attribute 'window_start'"})
	return nil
}

// RepoHalted is true when `.harness/HALT` exists on the default branch. The file exists when it
// has a sha (`gh.GitHub.GetFile`): an empty HALT halts too.
func (x *Context) RepoHalted() bool {
	_, sha := x.GH.GetFile(config.HaltPath, x.Cfg.DefaultBranch)
	return sha != ""
}

// Dispatch starts a night run now through workflow_dispatch. `item` nil is None.
func (x *Context) Dispatch(item *int /* = None */, force bool /* = False */, mode string /* = "auto" */) {
	itemText := ""
	if item != nil && *item != 0 {
		itemText = py.Str(*item)
	}
	forceText := "false"
	if force {
		forceText = "true"
	}
	inputs := map[string]string{"item": itemText, "force": forceText, "mode": mode}
	x.Act().DispatchWorkflow(config.NightWorkflow, x.Cfg.DefaultBranch, inputs)
}

// Build is context.build. `token` "" is None (the only caller, `__main__._ctx`, passes "" only
// when every token is empty, where the two build the same client); `ghc` nil and `clockFn` nil
// are None.
func Build(cfg *config.Config, token string /* = None */, ghc gh.Client /* = None */, clockFn func() time.Time /* = None */) *Context {
	client := ghc
	if client == nil {
		writeToken, fallback := token, ""
		if token == "" {
			writeToken = cfg.WriteToken()
			if cfg.BotToken != "" {
				fallback = cfg.ActionsToken
			}
		}
		client = gh.NewGitHub(cfg.Repo, writeToken, cfg.DryRun, nil, nil, fallback)
	}
	var actions gh.Client
	if ghc == nil && token == "" && cfg.BotToken != "" && cfg.ActionsToken != "" {
		actions = gh.NewGitHub(cfg.Repo, cfg.ActionsToken, cfg.DryRun, nil, nil, "")
	}
	if clockFn == nil {
		clockFn = func() time.Time { return clock.Now(cfg.NowOverride) }
	}
	return &Context{
		Cfg:       cfg,
		GH:        client,
		Store:     state.NewStateStore(client, config.StateBranch, config.StateFile, sleep),
		Trust:     trust.TrustLoad(filepath.Join(cfg.Root, config.TrustPath)),
		ClockFn:   clockFn,
		ActionsGH: actions,
	}
}

func sleep(seconds float64) { time.Sleep(time.Duration(seconds * float64(time.Second))) }
