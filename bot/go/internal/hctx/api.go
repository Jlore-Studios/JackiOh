// Package hctx is bot/harness/context.py.
//
// What every command works with: config, a GitHub client, the state store, the trust list.
package hctx

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/trust"
	"time"
)

// Context is context.Context.

type Context struct {
	Cfg       *config.Config
	GH        gh.Client
	Store     *state.StateStore
	Trust     *trust.Trust
	ClockFn   func() time.Time
	ActionsGH gh.Client // default: None
}

func (x *Context) Act() any {
	panic("unported: context.Context.act")
}

func (x *Context) Now() time.Time {
	panic("unported: context.Context.now")
}

func (x *Context) Window() *clock.Window {
	panic("unported: context.Context.window")
}

// True when `.harness/HALT` exists on the default branch.

func (x *Context) RepoHalted() bool {
	panic("unported: context.Context.repo_halted")
}

// Start a night run now through workflow_dispatch.

func (x *Context) Dispatch(item *int /* = None */, force bool /* = False */, mode string /* = "auto" */) {
	panic("unported: context.Context.dispatch")
}

// build is context.build.

func Build(cfg *config.Config, token string /* = None */, gh gh.Client /* = None */, clockFn func() time.Time /* = None */) *Context {
	panic("unported: context.build")
}
