// Package resolve is bot/harness/resolve.py.
//
// The conflicts the harness resolves itself when it merges `main`, with no model (`resolvers` in
// `.harness/config.json`).
//
// A conflict in a file a tool rewrites, or in one no branch should change, takes no judgment, yet it
// cost a model run like any other: in the two weeks to 2026-10-28, 169 of the 328 merges the bots
// left conflicted held nothing but `spec/INDEX.md` and `spec/11-rulings.md`, and 27 more nothing but
// those and the golden traces (`crates/engine/tests/golden/games.jsonl`). A resolver names one path
// and how its conflict is settled:
//
//   - `take: "main"`: the version the merge brought in (`git checkout --theirs`), or its deletion. For
//     a file a job on `main` rewrites and no branch should touch (`spec/INDEX.md`, the daily spec fold).
//   - `run: <command>`: a command that rewrites the file from the merged tree (`cargo jackioh golden
//     bless`). It runs only once nothing else is conflicted, because it builds that tree, only when
//     both sides kept the file, and only with time left for it and for a review after it. A command
//     that fails, times out, is cut off, leaves the file as it was or leaves a conflict marker puts
//     the file back exactly as git left it, markers and all, so the builder resolves it as before.
//
// Every other file stays as git left it. The run's checks then judge the merged head as they judge
// a builder's work, and its reviewer reviews it (`work.Worker._resolve`).
package resolve

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// MIN_SECONDS = 120
var MinSeconds int = 120

// What the harness did with one conflicted file: `ok` when it is resolved and staged.

type Resolution struct {
	Path   string
	How    string
	OK     bool
	WhyNot string // default: ""
}

func (x *Resolution) ToDict() *pyjson.Object {
	panic("unported: resolve.Resolution.to_dict")
}

// Settle the conflicts of the merge in progress in `wt` that a resolver names. Returns the
// paths still conflicted, in git's order, and what was done (resolved or not, and why not).
// `commands` False (the dependencies did not install) runs no command resolver; one starts only
// with `MIN_SECONDS` left beyond `reserve_seconds`. `base` names the branch merged in.

func Resolve(wt *git.Git, conflicts []string, resolvers []*config.Resolver, env map[string]string, secondsLeft func() float64, commands bool /* = True */, reserveSeconds float64 /* = 0.0 */, base string /* = "main" */) ([]string, []*Resolution) {
	panic("unported: resolve.resolve")
}

// One line for the journal: what was resolved, what was not and why, what is left.

func Summary(done []*Resolution, left []string) string {
	panic("unported: resolve.summary")
}
