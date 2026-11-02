// Package sessions keeps a Claude run's own Claude Code session for the next run of its item on
// the same subscription (#2748, section 9).
//
// Claude Code files every session of a `claude --print` call under its config directory, as
// `projects/<the working directory, every character but a letter or digit made ->/<id>.jsonl`
// (with whatever it keeps beside it in `<id>/`), and resumes one headless with `--resume <id>`.
// A runner is thrown away after its job, so the model job packs the last builder's (or a cut-off
// planner's) session, redacted, gzipped and sealed under the subscription's own vault key
// (`vault.SealBlob`): it holds every tool output of the run, so it never leaves the job readable.
// `deliver`, which holds the bot's token but no key, stores it on the state branch as
// `sessions/<item>-<subscription>.bin` with a small record in the item's state (`session`), and
// drops it when the item closes or a run of it on another subscription delivers. The next run on
// the same subscription restores it to the same path and resumes it; if anything fails it goes on
// as before, in a fresh session that reads the handoff.
//
// Only Claude resumes for now. Codex (`codex exec resume`), Muse and agy would each need their own
// capture and resume flag: `work.NewSessions` and each backend's `Argv` are where they go.
package sessions

import (
	"archive/tar"
	"bytes"
	"compress/gzip"
	"errors"
	"fmt"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/vault"
)

// Dir is the directory on the state branch the sessions are kept in.
var Dir = "sessions"

// OutFile is the sealed session in the model job's output directory, which the `work` artifact
// carries to `deliver` beside `result.json` and `vault.enc`.
var OutFile = "session.enc"

// MaxBytes caps a sealed session, in characters of its text: one over it is not kept (the run's
// log and journal say so). Every kept session is a new blob in the state branch's history, which
// nothing rewrites, so this bounds what a run adds to it.
var MaxBytes = 4_000_000

// UnpackedMaxBytes caps what a sealed session may unpack to.
var UnpackedMaxBytes int64 = 200_000_000

// Attempts is how many times a write to the state branch is tried when another writer raced it.
var Attempts = 4

// Kinds are the sessions a run keeps and resumes: a builder's (its first pass, a fix, a
// revision) and a planner's that keeps a draft. Reviews, self checks, splits and surveys run in a
// fresh session by design and are never kept.
var Kinds = []string{"build", "plan"}

// Families are the Claude model families: a session resumes only on its own family, whose
// prompt cache it can use.
var Families = []string{"opus", "sonnet", "haiku"}

// Path is where item `number`'s session on subscription `provider` is kept on the state branch.
func Path(number int, provider string) string {
	return fmt.Sprintf("%s/%d-%s.bin", Dir, number, provider)
}

// Family is a model's family (`opus` for `opus` and `claude-opus-4-7`), or its own name lowered.
func Family(model string) string {
	lowered := strings.ToLower(strings.TrimSpace(model))
	for _, family := range Families {
		if strings.Contains(lowered, family) {
			return family
		}
	}
	return lowered
}

var nonAlnum = regexp.MustCompile(`[^A-Za-z0-9]`)

// Project is the directory Claude Code files the sessions of a call in `cwd` under.
func Project(cwd string) string {
	return nonAlnum.ReplaceAllString(cwd, "-")
}

var (
	validID      = regexp.MustCompile(`^[0-9A-Za-z][0-9A-Za-z-]{7,63}$`)
	validProject = regexp.MustCompile(`^[A-Za-z0-9-]{1,255}$`)
	validHead    = regexp.MustCompile(`^[0-9a-f]{40}$`)
	validCwd     = regexp.MustCompile(`^/[A-Za-z0-9._/-]{0,500}$`)
)

// ValidCwd is whether `path` can be the working directory a kept session ran in: an absolute
// path of plain characters, since a resumed session is told it.
func ValidCwd(path string) bool { return validCwd.MatchString(path) }

// ValidID is whether `id` can be a session id: it names a file, so nothing that is not one passes.
func ValidID(id string) bool { return validID.MatchString(id) }

// ValidProject is whether `name` can be a project directory Claude Code made.
func ValidProject(name string) bool { return validProject.MatchString(name) }

// ValidHead is whether `sha` is a full commit sha, or "" for none.
func ValidHead(sha string) bool { return sha == "" || validHead.MatchString(sha) }

// projects is the directory Claude Code files every project's sessions in.
func projects(configDir string) string { return filepath.Join(configDir, "projects") }

// Transcript is where session `id` of `project` lives under `configDir`.
func Transcript(configDir, project, id string) string {
	return filepath.Join(projects(configDir), project, id+".jsonl")
}

// Find is the project directory under `configDir` that holds session `id`, or "" when none does.
func Find(configDir, id string) string {
	if !ValidID(id) {
		return ""
	}
	found, err := filepath.Glob(filepath.Join(projects(configDir), "*", id+".jsonl"))
	if err != nil || len(found) == 0 {
		return ""
	}
	sort.Strings(found)
	return filepath.Base(filepath.Dir(found[0]))
}

// Pack is session `id` of `project` as a gzipped tar: its transcript and what Claude Code keeps
// beside it in `<id>/`, but not its subagents' own transcripts (`<id>/subagents/`, which resuming
// does not read), each file redacted.
func Pack(configDir, project, id string) ([]byte, error) {
	dir := filepath.Join(projects(configDir), project)
	names := []string{id + ".jsonl"}
	side := filepath.Join(dir, id)
	if info, err := os.Stat(side); err == nil && info.IsDir() {
		err := filepath.WalkDir(side, func(path string, entry fs.DirEntry, err error) error {
			if err != nil {
				return err
			}
			rel, _ := filepath.Rel(dir, path)
			if entry.IsDir() && rel == filepath.Join(id, "subagents") {
				return filepath.SkipDir
			}
			if entry.Type().IsRegular() {
				names = append(names, filepath.ToSlash(rel))
			}
			return nil
		})
		if err != nil {
			return nil, err
		}
	}
	var buf bytes.Buffer
	zipped := gzip.NewWriter(&buf)
	archive := tar.NewWriter(zipped)
	for _, name := range names {
		data, err := os.ReadFile(filepath.Join(dir, filepath.FromSlash(name)))
		if err != nil {
			return nil, err
		}
		text := []byte(redact.Redact(string(data), nil))
		if err := archive.WriteHeader(&tar.Header{Name: name, Mode: 0o600, Size: int64(len(text)),
			Typeflag: tar.TypeReg, Format: tar.FormatPAX}); err != nil {
			return nil, err
		}
		if _, err := archive.Write(text); err != nil {
			return nil, err
		}
	}
	if err := archive.Close(); err != nil {
		return nil, err
	}
	if err := zipped.Close(); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}

// Unpack writes what Pack made back under `configDir`, into `project`: only files, only session
// `id`'s, none outside the project's directory.
func Unpack(data []byte, configDir, project, id string) error {
	if !ValidID(id) || !ValidProject(project) {
		return errors.New("not a session this harness keeps")
	}
	dir := filepath.Join(projects(configDir), project)
	zipped, err := gzip.NewReader(bytes.NewReader(data))
	if err != nil {
		return err
	}
	archive := tar.NewReader(zipped)
	var total int64
	for {
		header, err := archive.Next()
		if errors.Is(err, io.EOF) {
			break
		}
		if err != nil {
			return err
		}
		name := header.Name
		if header.Typeflag != tar.TypeReg || !filepath.IsLocal(filepath.FromSlash(name)) ||
			(name != id+".jsonl" && !strings.HasPrefix(name, id+"/")) {
			continue
		}
		total += header.Size
		if total > UnpackedMaxBytes {
			return fmt.Errorf("it unpacks to more than %s bytes", py.Thousands(int(UnpackedMaxBytes)))
		}
		target := filepath.Join(dir, filepath.FromSlash(name))
		if err := os.MkdirAll(filepath.Dir(target), 0o700); err != nil {
			return err
		}
		out, err := os.OpenFile(target, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, 0o600)
		if err != nil {
			return err
		}
		_, err = io.Copy(out, io.LimitReader(archive, header.Size))
		if closeErr := out.Close(); err == nil {
			err = closeErr
		}
		if err != nil {
			return err
		}
	}
	if info, err := os.Stat(Transcript(configDir, project, id)); err != nil || !info.Mode().IsRegular() {
		return fmt.Errorf("it holds no transcript of session %s", id)
	}
	return nil
}

// Seal is session `id` of `project` packed and sealed under `secret`, at most MaxBytes long; a
// larger one is a *vault.TooLarge.
func Seal(configDir, project, id, secret string) (string, error) {
	data, err := Pack(configDir, project, id)
	if err != nil {
		return "", err
	}
	return vault.SealBlob(data, secret, MaxBytes)
}

// Restore opens a sealed session with `secret` and unpacks it into `project` under `configDir`,
// where `claude --resume <id>` run in that project's working directory finds it.
func Restore(sealed, secret, configDir, project, id string) error {
	data, err := vault.OpenBlob(sealed, secret, MaxBytes)
	if err != nil {
		return err
	}
	return Unpack(data, configDir, project, id)
}

// ------------------------------------------------------------------ the state branch

// listed is the stored sessions' file names and blob shas: a listing, never a download, since a
// session runs to megabytes.
func listed(client gh.Client) [][2]string {
	return client.ListDir(Dir, config.StateBranch)
}

// shaOf is the blob sha of the stored session at `path`, "" when there is none.
func shaOf(client gh.Client, path string) string {
	for _, entry := range listed(client) {
		if Dir+"/"+entry[0] == path {
			return entry[1]
		}
	}
	return ""
}

// Put stores sealed `text` at `path` on the state branch, again when another writer raced it.
func Put(client gh.Client, path, text, message string) error {
	var problem error
	for range Attempts {
		exc, raised := herr.Catch[*herr.GitHubError](func() {
			client.PutFile(path, text+"\n", config.StateBranch, shaOf(client, path), message)
		})
		if !raised {
			return nil
		}
		problem = exc
		if exc.Status != 409 && exc.Status != 422 {
			break // only a race with another writer is worth trying again
		}
	}
	return problem
}

// Remove deletes the stored session at `path`; one that is gone already is no error.
func Remove(client gh.Client, path, message string) error {
	var problem error
	for range Attempts {
		sha := shaOf(client, path)
		if sha == "" {
			return nil
		}
		exc, raised := herr.Catch[*herr.GitHubError](func() {
			client.DeleteFile(path, config.StateBranch, sha, message)
		})
		if !raised || exc.Status == 404 {
			return nil
		}
		problem = exc
		if exc.Status != 409 && exc.Status != 422 {
			break
		}
	}
	return problem
}

// itemOf is the item a stored session's file name is of (`<item>-<subscription>.bin`), or 0.
func itemOf(name string) int {
	number, _, ok := strings.Cut(name, "-")
	if !ok || !strings.HasSuffix(name, ".bin") || !py.IsDecimal(number) {
		return 0
	}
	n, err := strconv.Atoi(number)
	if err != nil {
		return 0
	}
	return n
}

// DropItem deletes every session kept for item `number` and its record: the item closed, or
// its work moved on. Returns what it did, for a log; nothing here raises.
func DropItem(ctx *hctx.Context, number int, why string) []string {
	notes := []string{}
	if exc := herr.Try(func() {
		for _, entry := range listed(ctx.GH) {
			if itemOf(entry[0]) != number {
				continue
			}
			path := Dir + "/" + entry[0]
			if err := Remove(ctx.GH, path, fmt.Sprintf("session: drop #%d (%s) [skip ci]", number, why)); err != nil {
				notes = append(notes, fmt.Sprintf("could not drop the kept session `%s`: %s", path, err))
				continue
			}
			notes = append(notes, fmt.Sprintf("dropped the kept session `%s`: %s", path, why))
		}
		if ctx.Store.Load().Obj("items").Obj(strconv.Itoa(number)).Has("session") {
			ctx.Store.Update(func(s *pyjson.Object) any {
				state.Item(s, number).Pop("session")
				return nil
			}, fmt.Sprintf("session #%d dropped", number))
		}
	}); exc != nil {
		notes = append(notes, fmt.Sprintf("could not drop the kept sessions of #%d: %s", number,
			py.SliceTo(redact.Redact(herr.Str(exc), nil), 300)))
	}
	return notes
}

// Prune deletes every kept session whose item is closed (once a day, with the state file's own
// prune): `open` holds the open items' numbers, and each other one is asked before its session
// goes. Returns what it did; nothing here raises.
func Prune(ctx *hctx.Context, open map[int]bool) []string {
	notes := []string{}
	var entries [][2]string
	if _, failed := herr.Catch[*herr.GitHubError](func() { entries = listed(ctx.GH) }); failed {
		return notes
	}
	for _, entry := range entries {
		number := itemOf(entry[0])
		if number == 0 || open[number] {
			continue
		}
		var thread *pyjson.Object
		if _, failed := herr.Catch[*herr.GitHubError](func() { thread = ctx.GH.GetIssue(number) }); failed {
			continue
		}
		if thread.Get("state") != "closed" {
			continue
		}
		path := Dir + "/" + entry[0]
		if err := Remove(ctx.GH, path, fmt.Sprintf("session: drop #%d (closed) [skip ci]", number)); err == nil {
			notes = append(notes, fmt.Sprintf("dropped the kept session `%s`: #%d is closed", path, number))
		}
	}
	return notes
}
