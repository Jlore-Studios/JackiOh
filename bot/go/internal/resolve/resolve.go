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
	"bytes"
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"regexp"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gates"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// MinSeconds: a command resolver needs at least this many seconds of the run left to start,
// beyond what the caller keeps for the calls after it (`reserve_seconds`).
var MinSeconds int = 120

// Resolution is what the harness did with one conflicted file: `ok` when it is resolved and
// staged.
type Resolution struct {
	Path   string
	How    string
	OK     bool
	WhyNot string // default: ""
}

// ToDict is resolve.Resolution.to_dict (dataclasses.asdict).
func (x *Resolution) ToDict() *pyjson.Object {
	return pyjson.New("path", x.Path, "how", x.How, "ok", x.OK, "why_not", x.WhyNot)
}

var markerLine = regexp.MustCompile("(?m)" + git.MarkerLine)

func marked(path string) bool {
	data, err := os.ReadFile(path)
	if errors.Is(err, fs.ErrNotExist) {
		return false
	}
	if err != nil {
		panic(err)
	}
	// Read as text, every line break ("\r\n", "\r") is a "\n" where `^` matches; nothing the
	// decoder replaces can be a break or a marker.
	return markerLine.Match(bytes.ReplaceAll(data, []byte("\r"), []byte("\n")))
}

func existsAt(wt *git.Git, ref string, path string) bool {
	return wt.Run(false, 600, nil, "cat-file", "-e", ref+":"+path).Returncode == 0
}

// takeMain is `base`'s side of `path` (the branch merged in, `main` but for a fullsend part): its
// version, or its deletion when it deleted the file.
func takeMain(wt *git.Git, path string, base string) *Resolution {
	if existsAt(wt, "MERGE_HEAD", path) {
		wt.Run(true, 600, nil, "checkout", "--theirs", "--", path)
		wt.Run(true, 600, nil, "add", "--", path)
		return &Resolution{Path: path, How: fmt.Sprintf("took `%s`'s version", base), OK: true}
	}
	wt.Run(true, 600, nil, "rm", "-q", "--", path)
	return &Resolution{Path: path, How: fmt.Sprintf("took `%s`'s side: it deleted the file", base), OK: true}
}

// exists is Path.exists().
func exists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

// runResolver is _run: rewrite `resolver.path` with its command from the merged tree. Unless that
// succeeds, the file is put back byte for byte as the merge left it, even when the run is cut off
// mid-command (the job's signal handler raises), so it is the builder's again and never a
// half-written file a work-in-progress commit could take up.
func runResolver(wt *git.Git, resolver *config.Resolver, env map[string]string, seconds float64) *Resolution {
	path := resolver.Path
	how := fmt.Sprintf("ran `%s`", resolver.Run)
	if !(existsAt(wt, "HEAD", path) && existsAt(wt, "MERGE_HEAD", path)) {
		return &Resolution{Path: path, How: how, OK: false, WhyNot: "one side deleted the file, which no command decides"}
	}
	target := filepath.Join(wt.Cwd, path)
	var original []byte
	hadOriginal := exists(target)
	if hadOriginal {
		data, err := os.ReadFile(target)
		if err != nil {
			panic(err)
		}
		original = data
	}
	resolved := false
	defer func() {
		if resolved {
			return
		}
		if !hadOriginal {
			if err := os.Remove(target); err != nil && !errors.Is(err, fs.ErrNotExist) {
				panic(err)
			}
			return
		}
		if err := os.MkdirAll(filepath.Dir(target), 0o777); err != nil {
			panic(err)
		}
		if err := os.WriteFile(target, original, 0o666); err != nil {
			panic(err)
		}
	}()
	gate := config.NewGate("resolve "+path, resolver.Run, resolver.TimeoutMinutes)
	timeout := int(seconds)
	result := gates.RunGate(gate, wt.Cwd, env, &timeout)
	var why string
	switch {
	case !result.OK:
		why = fmt.Sprintf("it exited %d: %s", result.ExitCode, py.SliceFrom(strings.TrimSpace(result.Tail), -600))
	case !exists(target):
		why = "it left no file"
	case sameAs(target, original, hadOriginal):
		why = "it left the file as the merge left it"
	case marked(target):
		why = "it left a conflict marker in the file"
	default:
		wt.Run(true, 600, nil, "add", "--", path)
		resolved = true
		return &Resolution{Path: path, How: how, OK: true}
	}
	return &Resolution{Path: path, How: how, OK: false, WhyNot: why}
}

// sameAs is `path.read_bytes() == original`, where an original of None (had false) equals no
// bytes read.
func sameAs(path string, original []byte, had bool) bool {
	data, err := os.ReadFile(path)
	if err != nil {
		panic(err)
	}
	return had && bytes.Equal(data, original)
}

// remove is list.remove(x): the first x goes.
func remove(xs []string, x string) []string {
	for i, s := range xs {
		if s == x {
			return append(xs[:i:i], xs[i+1:]...)
		}
	}
	panic(fmt.Sprintf("list.remove(x): %s not in list", x))
}

// Resolve settles the conflicts of the merge in progress in `wt` that a resolver names. Returns
// the paths still conflicted, in git's order, and what was done (resolved or not, and why not).
// `commands` False (the dependencies did not install) runs no command resolver; one starts only
// with `MIN_SECONDS` left beyond `reserve_seconds`. `base` names the branch merged in.
func Resolve(wt *git.Git, conflicts []string, resolvers []*config.Resolver, env map[string]string, secondsLeft func() float64, commands bool /* = True */, reserveSeconds float64 /* = 0.0 */, base string /* = "main" */) ([]string, []*Resolution) {
	byPath := map[string]*config.Resolver{}
	for _, r := range resolvers {
		byPath[r.Path] = r
	}
	left := append([]string{}, conflicts...)
	done := []*Resolution{}
	for _, path := range conflicts {
		if resolver := byPath[path]; resolver != nil && resolver.Take != "" {
			done = append(done, takeMain(wt, path, base))
			left = remove(left, path)
		}
	}
	pending := []*config.Resolver{}
	for _, p := range left {
		if r := byPath[p]; r != nil && r.Run != "" {
			pending = append(pending, r)
		}
	}
	if len(pending) == 0 {
		return left, done
	}
	if len(pending) != len(left) {
		rest := []string{}
		for _, p := range left {
			if byPath[p] == nil {
				rest = append(rest, "`"+p+"`")
			}
		}
		why := fmt.Sprintf("other files are still conflicted (%s), so the merged tree would not build",
			strings.Join(rest, ", "))
		for _, r := range pending {
			done = append(done, &Resolution{Path: r.Path, How: fmt.Sprintf("ran `%s`", r.Run), OK: false, WhyNot: why})
		}
		return left, done
	}
	if !commands {
		for _, r := range pending {
			done = append(done, &Resolution{Path: r.Path, How: fmt.Sprintf("ran `%s`", r.Run), OK: false,
				WhyNot: "the dependencies did not install"})
		}
		return left, done
	}
	for _, resolver := range pending {
		seconds := secondsLeft() - reserveSeconds
		if seconds < float64(MinSeconds) {
			done = append(done, &Resolution{Path: resolver.Path, How: fmt.Sprintf("ran `%s`", resolver.Run), OK: false,
				WhyNot: "too little of the run was left"})
			continue
		}
		resolution := runResolver(wt, resolver, env, seconds)
		done = append(done, resolution)
		if resolution.OK {
			left = remove(left, resolver.Path)
		}
	}
	return left, done
}

// Summary is one line for the journal: what was resolved, what was not and why, what is left.
func Summary(done []*Resolution, left []string) string {
	parts := []string{}
	for _, r := range done {
		part := fmt.Sprintf("`%s`: %s", r.Path, r.How)
		if !r.OK {
			part += ", not resolved: " + r.WhyNot
		}
		parts = append(parts, part)
	}
	if len(left) > 0 {
		parts = append(parts, fmt.Sprintf("%d file(s) left for the builder", len(left)))
	}
	return strings.Join(parts, "; ")
}
