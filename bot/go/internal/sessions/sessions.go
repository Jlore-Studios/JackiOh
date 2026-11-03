// Package sessions keeps a Claude run's own Claude Code session for the next run of its item on
// the same subscription (#2748, section 9).
//
// Claude Code files every session of a `claude --print` call under its config directory, as
// `projects/<the working directory, every character but a letter or digit made ->/<id>.jsonl`
// (with whatever it keeps beside it in `<id>/`), and resumes one headless with `--resume <id>`.
// A runner is thrown away after its job, so the model job packs the last builder's (or a cut-off
// planner's) session, redacted, gzipped and sealed under the subscription's own vault key
// (`vault.SealBlob`), into `session.enc` in its output: it holds every tool output of the run, so
// it never leaves the job readable. The job's `work` artifact carries it (for its retention, 14
// days in bot-night.yml), and that artifact is the store: nothing session-sized is ever committed.
// `deliver`, which holds the bot's token but no key, checks its shape and records only a pointer
// in the item's state (`session`: the run and artifact that hold it, and what it is), and clears
// the pointer when the item closes or a run of it on another subscription delivers. The next run
// on the same subscription downloads that artifact (`actions: read`), takes `session.enc` alone out
// of it, restores the session to the same path and resumes it; if anything fails (the artifact
// expired among the reasons) it goes on as before, in a fresh session that reads the handoff.
//
// Only Claude resumes for now. Codex (`codex exec resume`), Muse and agy would each need their own
// capture and resume flag: `work.NewSessions` and each backend's `Argv` are where they go.
package sessions

import (
	"archive/tar"
	"archive/zip"
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
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/vault"
)

// OutFile is the sealed session in the model job's output directory, which the `work` artifact
// carries beside `result.json` and `vault.enc`, and the one entry taken out of that artifact.
var OutFile = "session.enc"

// Artifact is the model job's upload that holds it (bot-night.yml's and squishy-run.yml's `work`,
// `controller.WorkArtifact`).
var Artifact = "work"

// LegacyDir is where an earlier draft of this kept sessions on the state branch, which no run
// reads and the daily prune deletes.
var LegacyDir = "sessions"

// MaxBytes caps a sealed session, in characters of its text: one over it is not kept (the run's
// log and journal say so), and an artifact's `session.enc` over it is not taken out.
var MaxBytes = 4_000_000

// ArtifactMaxBytes caps the artifact the next run downloads to take the session out of (its
// bundle and its transcripts come with it).
var ArtifactMaxBytes = 500_000_000

// UnpackedMaxBytes caps what a sealed session may unpack to.
var UnpackedMaxBytes int64 = 200_000_000

// Kinds are the sessions a run keeps and resumes: a builder's (its first pass, a fix, a
// revision) and a planner's that keeps a draft. Reviews, self checks, splits and surveys run in a
// fresh session by design and are never kept.
var Kinds = []string{"build", "plan"}

// Families are the Claude model families: a session resumes only on its own family, whose
// prompt cache it can use.
var Families = []string{"opus", "sonnet", "haiku"}

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

// ------------------------------------------------------------------ the run's artifact

// WorkArtifact is the newest `work` artifact among a run's that has not expired, or nil.
func WorkArtifact(artifacts []*pyjson.Object) *pyjson.Object {
	var found *pyjson.Object
	for _, a := range artifacts {
		if a.Get("name") != Artifact || a.Truthy("expired") {
			continue
		}
		id, _ := py.Int(py.Or(a.Get("id"), 0))
		best, _ := py.Int(py.Or(found.Get("id"), 0))
		if found == nil || id > best {
			found = a
		}
	}
	return found
}

// FromZip is the sealed session in an artifact's zip: its `session.enc` and nothing else, at most
// MaxBytes long. A zip with an entry outside it (an absolute path, `..`) is refused whole.
func FromZip(data []byte) (string, error) {
	archive, err := zip.NewReader(bytes.NewReader(data), int64(len(data)))
	if err != nil {
		return "", fmt.Errorf("the artifact is not a zip: %s", err)
	}
	var found *zip.File
	for _, member := range archive.File {
		name := member.Name
		if strings.Contains(name, `\`) || !filepath.IsLocal(filepath.FromSlash(name)) {
			return "", fmt.Errorf("the artifact holds a path outside it: %s", py.Repr(name))
		}
		if name != OutFile {
			continue // never taken out: only the session is wanted here
		}
		if found != nil {
			return "", fmt.Errorf("the artifact holds two %s", OutFile)
		}
		found = member
	}
	if found == nil {
		return "", fmt.Errorf("the artifact holds no %s", OutFile)
	}
	limit := int64(MaxBytes) + 1 // the line break it was written with
	if found.UncompressedSize64 > uint64(limit) {
		return "", &vault.TooLarge{Size: int(found.UncompressedSize64), Limit: MaxBytes}
	}
	reader, err := found.Open()
	if err != nil {
		return "", err
	}
	defer reader.Close()
	body, err := io.ReadAll(io.LimitReader(reader, limit+1))
	if err != nil {
		return "", err
	}
	if int64(len(body)) > limit {
		return "", &vault.TooLarge{Size: len(body), Limit: MaxBytes}
	}
	return strings.TrimSpace(string(body)), nil
}

// Fetch is the sealed session a kept record points at, out of its run's `work` artifact
// (`GET /actions/artifacts/<id>/zip`, which needs `actions: read`). Its error says why it could
// not be had: a record with no run, an artifact expired or deleted (404, 410), a download that
// failed, or an artifact that does not hold one.
func Fetch(client gh.Client, record *pyjson.Object) (string, error) {
	run := py.Str(py.Or(record.Get("run"), ""))
	if !py.IsDecimal(run) {
		return "", errors.New("its record names no run whose artifact holds it")
	}
	id := record.Get("artifact")
	size, _ := py.Int(py.Or(record.Get("artifact_bytes"), 0))
	if !pyjson.Truthy(id) {
		// Deliver could not name it when it recorded the session: the run's own, by its name.
		var listed []*pyjson.Object
		if exc, failed := herr.Catch[*herr.GitHubError](func() { listed = client.ListArtifacts(run) }); failed {
			return "", fmt.Errorf("could not list run %s's artifacts: %s", run, exc.Error())
		}
		found := WorkArtifact(listed)
		if found == nil {
			return "", fmt.Errorf("run %s has no `%s` artifact left: it expired, or was deleted", run, Artifact)
		}
		id = found.Get("id")
		size, _ = py.Int(py.Or(found.Get("size_in_bytes"), 0))
	}
	if size > ArtifactMaxBytes {
		return "", fmt.Errorf("run %s's `%s` artifact is %s bytes, over the %s-byte cap", run, Artifact,
			py.Thousands(size), py.Thousands(ArtifactMaxBytes))
	}
	var data []byte
	if exc, failed := herr.Catch[*herr.GitHubError](func() { data = client.DownloadArtifact(id) }); failed {
		if exc.Status == 404 || exc.Status == 410 {
			return "", fmt.Errorf("run %s's `%s` artifact is gone (HTTP %d): it expired, or was deleted",
				run, Artifact, exc.Status)
		}
		return "", fmt.Errorf("could not download run %s's `%s` artifact: %s", run, Artifact,
			py.SliceTo(redact.Redact(exc.Error(), nil), 200))
	}
	if len(data) > ArtifactMaxBytes {
		return "", fmt.Errorf("run %s's `%s` artifact is over the %s-byte cap", run, Artifact,
			py.Thousands(ArtifactMaxBytes))
	}
	return FromZip(data)
}

// ------------------------------------------------------------------ the pointer in the state

// Remove clears item `number`'s kept session in `state` (its pointer: the session itself is its
// run's artifact, which expires by itself). Whether there was one. For a store update.
func Remove(s *pyjson.Object, number int) bool {
	record := s.Obj("items").Obj(strconv.Itoa(number))
	if !record.Has("session") {
		return false
	}
	record.Pop("session")
	return true
}

// DropItem clears the session kept for item `number`: the item closed, or its work moved on.
// Returns what it did, for a log; nothing here raises.
func DropItem(ctx *hctx.Context, number int, why string) []string {
	notes := []string{}
	if exc := herr.Try(func() {
		if !ctx.Store.Load().Obj("items").Obj(strconv.Itoa(number)).Has("session") {
			return
		}
		ctx.Store.Update(func(s *pyjson.Object) any {
			Remove(s, number)
			return nil
		}, fmt.Sprintf("session #%d dropped", number))
		notes = append(notes, fmt.Sprintf("dropped #%d's kept session: %s", number, why))
	}); exc != nil {
		notes = append(notes, fmt.Sprintf("could not drop #%d's kept session: %s", number,
			py.SliceTo(redact.Redact(herr.Str(exc), nil), 300)))
	}
	return notes
}

// Prune, once a day with the state file's own prune: clear the kept session of every item that
// closed (`open` holds the open items' numbers; each other one is asked first), and delete any
// session file an earlier draft left on the state branch (`LegacyDir`), which no run reads.
// Returns what it did; nothing here raises.
func Prune(ctx *hctx.Context, open map[int]bool) []string {
	notes := []string{}
	if exc := herr.Try(func() {
		items := ctx.Store.Load().Obj("items")
		var closed []int
		for _, key := range items.Keys() {
			number, err := strconv.Atoi(key)
			if err != nil || number == 0 || open[number] || !items.Obj(key).Has("session") {
				continue
			}
			var thread *pyjson.Object
			if _, failed := herr.Catch[*herr.GitHubError](func() { thread = ctx.GH.GetIssue(number) }); failed {
				continue
			}
			if thread.Get("state") == "closed" {
				closed = append(closed, number)
			}
		}
		if len(closed) > 0 {
			ctx.Store.Update(func(s *pyjson.Object) any {
				for _, number := range closed {
					Remove(s, number)
				}
				return nil
			}, "sessions of closed items dropped")
			for _, number := range closed {
				notes = append(notes, fmt.Sprintf("dropped #%d's kept session: #%d is closed", number, number))
			}
		}
		var legacy [][2]string
		if _, failed := herr.Catch[*herr.GitHubError](func() {
			legacy = ctx.GH.ListDir(LegacyDir, config.StateBranch)
		}); failed {
			return
		}
		for _, entry := range legacy {
			if !strings.HasSuffix(entry[0], ".bin") {
				continue
			}
			path := LegacyDir + "/" + entry[0]
			if _, failed := herr.Catch[*herr.GitHubError](func() {
				ctx.GH.DeleteFile(path, config.StateBranch, entry[1],
					"session: a kept session lives in its run's artifact [skip ci]")
			}); !failed {
				notes = append(notes, fmt.Sprintf("deleted `%s` from the state branch: a kept session "+
					"lives in its run's artifact", path))
			}
		}
	}); exc != nil {
		notes = append(notes, "could not prune the kept sessions: "+py.SliceTo(redact.Redact(herr.Str(exc), nil), 300))
	}
	return notes
}
