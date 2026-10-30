package controller

import (
	"archive/zip"
	"bytes"
	"errors"
	"fmt"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"strconv"
	"strings"
	"syscall"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
)

// ---------------------------------------------------------------------- delivering (#1537)

// WorkArtifact is the model job's upload (bot-night.yml's `work` job): the result, the bundle, the
// sealed login and the transcripts.
var WorkArtifact = "work"

// CloneTimeout is how long the clone of the default branch one delivery works in may take, in
// seconds.
var CloneTimeout = 600

// KeepDeliveries is how many of the loop's deliveries the state file remembers.
var KeepDeliveries = 100

// Deliver is controller.deliver; a variable so the tests can replace it.
var Deliver = deliverRuns

// deliverRuns delivers every night run the loop started that has ended and still waits for it
// (`plan.awaiting_delivery`). Each line says what happened; nothing here raises. An empty
// `workspace` is None: the runner's scratch directory.
func deliverRuns(ctx *hctx.Context, workspace string /* = None */) []string {
	var runs []*pyjson.Object
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		runs = ctx.GH.ListRuns(py.Str(config.NightWorkflow), "", 50, false)
	}); failed {
		return []string{"could not list the night runs to deliver: " + exc.Error()}
	}
	scratch := workspace
	if scratch == "" {
		scratch = config.GithubEnv()["RUNNER_TEMP"]
		if scratch == "" {
			scratch = os.TempDir()
		}
	}
	notes := []string{}
	for _, run := range runs {
		var waiting bool
		if exc, failed := herr.Catch[*herr.GitHubError](func() {
			waiting = plan.AwaitingDelivery(ctx, run)
		}); failed {
			notes = append(notes, fmt.Sprintf("could not tell whether run %s waits for delivery: %s",
				py.Str(run.Get("id")), exc.Error()))
			continue
		}
		if waiting {
			notes = append(notes, deliverRun(ctx, run, scratch))
		}
	}
	return notes
}

// deliverRun delivers one run as its own `deliver` job did: its plan, its model job's output and a
// fresh clone of the default branch, with the run's id on the config, so every link and record
// names the night run. A failure is a note, and the run is tried again on the next tick.
func deliverRun(ctx *hctx.Context, run *pyjson.Object, scratch string) string {
	runID := py.Str(run.Get("id"))
	what := "run " + runID
	var tries int
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		tries = countTry(ctx, runID, plan.LoopRef(run))
	}); failed {
		return fmt.Sprintf("could not deliver %s: the state file refused its try (%s)", what, exc.Error())
	}
	var outcome *pyjson.Object
	var found bool
	// One delivery failing never stops the tick: anything it raises is a note.
	if exc := herr.Try(func() {
		planned, err := Fetch(ctx, plan.LoopRef(run))
		if err != nil {
			herr.Raise(err)
		}
		what = fmt.Sprintf("%s (run %s)", What(planned), runID)
		herr.Must(os.MkdirAll(scratch, 0o777))
		tmp, err := os.MkdirTemp(scratch, fmt.Sprintf("deliver-%s-", runID))
		herr.Must(err)
		defer func() { herr.Must(os.RemoveAll(tmp)) }()
		out := filepath.Join(tmp, "out")
		found = downloadWork(ctx, runID, out)
		repo := clone(ctx, filepath.Join(tmp, "repo"))
		asRun := withRunID(ctx, runID)
		deliverer := deliver.NewDeliverer(asRun, planned, out, repo, "", nil, "")
		deliverer.Chain = false // this tick's sweep plans the next runs itself
		outcome = deliverer.Run()
	}); exc != nil {
		left := plan.DeliverTries - tries
		then := "its item is left to housekeeping, as a dead run's"
		if left > 0 {
			then = fmt.Sprintf("it is tried again next tick (%d more)", left)
		}
		return fmt.Sprintf("could not deliver %s, try %d of %d "+
			"(%s: %s): %s", what, tries, plan.DeliverTries,
			pyTypeName(exc), py.SliceTo(redact.Redact(pyStr(exc), nil), 300), then)
	}
	status := orStr(outcome.Get("status"))
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		Record(ctx, runID, pyjson.New("done", true, "status", status))
	}); failed {
		return fmt.Sprintf("delivered %s: %s, but its record was refused (%s)", what, status, exc.Error())
	}
	text := fmt.Sprintf("delivered %s: %s", what, status)
	if !found {
		text += " (its model job left no upload)"
	}
	return text
}

// countTry records one more try at delivering `run_id` before it starts, so a try that ends the
// loop's process counts too. Returns which try this is.
func countTry(ctx *hctx.Context, runID string, ref string) int {
	tries := 0

	change := func(state *pyjson.Object) any {
		book, ok := state.Get(plan.Deliveries).(*pyjson.Object)
		if !ok {
			book = pyjson.New()
			state.Set(plan.Deliveries, book)
		}
		entry, ok := book.Get(runID).(*pyjson.Object)
		if !ok {
			entry = pyjson.New()
		}
		tries = pyInt(orDefault(entry.Get("tries"), 0)) + 1
		book.Set(runID, entry.Copy().Set("ref", ref).Set("tries", tries).Set("at", iso(ctx.Now())))
		byAge := sortedBy(book.Keys(), func(key string) string { return orStr(book.Obj(key).Get("at")) })
		for _, old := range byAge[:max(0, len(byAge)-KeepDeliveries)] {
			book.Delete(old)
		}
		return nil
	}

	ctx.Store.Update(change, "delivering run "+runID)
	return tries
}

// Record is controller._record: `fields` set on the run's delivery record, if it has one.
func Record(ctx *hctx.Context, runID string, fields *pyjson.Object) {
	change := func(state *pyjson.Object) any {
		book, ok := state.Get(plan.Deliveries).(*pyjson.Object)
		if !ok {
			return nil
		}
		if entry, ok := book.Get(runID).(*pyjson.Object); ok {
			entry.Update(fields)
		}
		return nil
	}

	ctx.Store.Update(change, "delivered run "+runID)
}

// downloadWork unzips the run's `work` upload into `out`. False when its model job left none (it
// failed before the model started, or was cancelled): the deliverer says so then, as deliver did
// after its download, which was allowed to fail.
func downloadWork(ctx *hctx.Context, runID string, out string) bool {
	herr.Must(os.MkdirAll(out, 0o777))
	var found []*pyjson.Object
	for _, a := range ctx.GH.ListArtifacts(runID) {
		if a.Get("name") == WorkArtifact && !a.Truthy("expired") {
			found = append(found, a)
		}
	}
	if len(found) == 0 {
		return false
	}
	data := ctx.GH.DownloadArtifact(found[0].Get("id"))
	root, err := filepath.EvalSymlinks(out)
	herr.Must(err)
	root, err = filepath.Abs(root)
	herr.Must(err)
	archive, err := zip.NewReader(bytes.NewReader(data), int64(len(data)))
	if err != nil {
		herr.Raise(zipError(err, ""))
	}
	for _, member := range archive.File {
		if !inside(root, member.Name) {
			herr.Raise(valueError("the upload holds a path outside it: %s", py.Repr(member.Name)))
		}
	}
	extractAll(archive, root)
	return true
}

// inside is (root / name).resolve().is_relative_to(root), for a root that holds nothing yet.
func inside(root, name string) bool {
	path := name
	if !filepath.IsAbs(path) {
		path = filepath.Join(root, path)
	}
	path = filepath.Clean(path)
	return path == root || strings.HasPrefix(path, root+string(filepath.Separator))
}

// extractAll is ZipFile.extractall(root): each member's name with its empty, `.` and `..` parts
// dropped, its folders made, its bytes written (no file modes, as Python's zipfile keeps none).
func extractAll(archive *zip.Reader, root string) {
	for _, member := range archive.File {
		var parts []string
		for _, part := range strings.Split(member.Name, "/") {
			if part != "" && part != "." && part != ".." {
				parts = append(parts, part)
			}
		}
		arcname := strings.Join(parts, string(filepath.Separator))
		isDir := strings.HasSuffix(member.Name, "/")
		if arcname == "" && !isDir {
			herr.Raise(valueError("Empty filename."))
		}
		target := filepath.Clean(filepath.Join(root, arcname))
		if upper := filepath.Dir(target); upper != "" {
			if _, err := os.Stat(upper); err != nil {
				herr.Must(os.MkdirAll(upper, 0o777))
			}
		}
		if isDir {
			if info, err := os.Stat(target); err != nil || !info.IsDir() {
				if err := os.Mkdir(target, 0o777); err != nil {
					if info, statErr := os.Stat(target); statErr != nil || !info.IsDir() {
						herr.Raise(err)
					}
				}
			}
			continue
		}
		source, err := member.Open()
		if err != nil {
			herr.Raise(zipError(err, member.Name))
		}
		func() {
			defer source.Close()
			target, err := os.OpenFile(target, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, 0o666)
			herr.Must(err)
			defer target.Close()
			if _, err := io.Copy(target, source); err != nil {
				herr.Raise(zipError(err, member.Name))
			}
		}()
	}
}

// pyException is a Python exception other than ValueError that a delivery can raise and note.
type pyException struct{ typ, msg string }

func (e *pyException) Error() string { return e.msg }

// zipError is what zipfile raised where archive/zip fails.
func zipError(err error, name string) error {
	switch {
	case errors.Is(err, zip.ErrFormat):
		return &pyException{"BadZipFile", "File is not a zip file"}
	case errors.Is(err, zip.ErrChecksum):
		return &pyException{"BadZipFile", "Bad CRC-32 for file " + py.Repr(name)}
	}
	return err
}

// clone is a fresh clone of the default branch, history and all but blobs fetched as needed, for
// one delivery: the deliverer fetches the bundle into it and checks what it descends from.
func clone(ctx *hctx.Context, into string) string {
	url := fmt.Sprintf("%s/%s.git", ctx.Cfg.ServerURL, ctx.Cfg.Repo)
	git.NewGit(filepath.Dir(into), nil).Run(true, CloneTimeout, nil, "clone", "--quiet", "--filter=blob:none",
		"--no-tags", "--branch", ctx.Cfg.DefaultBranch, url, into)
	return into
}

// pyTypeName is type(exc).__name__ for what a delivery raised: the harness's own errors by their
// class, Python's OSError subclasses for Go's file errors, and RuntimeError for a Go fault (which
// Python would have raised as an AttributeError, KeyError or TypeError).
func pyTypeName(err error) string {
	var value *ValueError
	var other *pyException
	var errno syscall.Errno
	var fault runtime.Error
	var panicked *herr.Panic
	switch {
	case errors.As(err, &value):
		return value.Type
	case errors.As(err, &other):
		return other.typ
	case errors.As(err, &fault), errors.As(err, &panicked):
		return "RuntimeError"
	}
	switch err.(type) {
	case *herr.HarnessError, *herr.ConfigError, *herr.GitHubError, *herr.GitError, *herr.StateConflict,
		*herr.StateUnreadable, *herr.LoginError:
		return reflect.TypeOf(err).Elem().Name()
	}
	switch {
	case errors.Is(err, fs.ErrNotExist):
		return "FileNotFoundError"
	case errors.Is(err, fs.ErrExist):
		return "FileExistsError"
	case errors.Is(err, fs.ErrPermission):
		return "PermissionError"
	case errors.Is(err, syscall.ENOTDIR):
		return "NotADirectoryError"
	case errors.Is(err, syscall.EISDIR):
		return "IsADirectoryError"
	case errors.As(err, &errno):
		return "OSError"
	}
	return "Exception"
}

// pyStr is str(exc): an OSError on a path as Python words it ("[Errno 13] Permission denied:
// '/tmp/x'"), anything else as its error says.
func pyStr(err error) string {
	var path *fs.PathError
	var errno syscall.Errno
	if errors.As(err, &path) && errors.As(path.Err, &errno) {
		text := errno.Error()
		if text != "" {
			text = strings.ToUpper(text[:1]) + text[1:]
		}
		return fmt.Sprintf("[Errno %d] %s: %s", int(errno), text, py.Repr(path.Path))
	}
	return err.Error()
}

// pyInt is int(text) on a reference's digits, or on a record's count.
func pyInt(v any) int {
	if s, ok := v.(string); ok {
		n, err := strconv.Atoi(strings.TrimSpace(s))
		if err != nil {
			panic(&pyException{"ValueError", "invalid literal for int() with base 10: " + py.Repr(s)})
		}
		return n
	}
	if n, ok := pyjson.AsInt(v); ok {
		return n
	}
	if b, ok := v.(bool); ok && b {
		return 1
	} else if ok {
		return 0
	}
	panic(&pyException{"TypeError", fmt.Sprintf(
		"int() argument must be a string, a bytes-like object or a real number, not %T", v)})
}
