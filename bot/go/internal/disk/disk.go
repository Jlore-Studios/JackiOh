// Package disk is bot/harness/disk.py.
//
// The bot machine's disk (bot/machine/README.md, Disk): what a model job there reads of it, the
// room it makes before it starts, and the one issue that tells people while it is filling up.
//
// Every subscription on the machine shares its one disk, and a full one fails whichever job writes
// next: on 2026-10-04 a revision of #206 died on `OSError: [Errno 28] No space left on device`,
// with 66 MB free when its job began, and it counted as that item's failure. So a model job there
// (`work.Worker`):
//
//   - reads the disk as it starts and as it ends, into `result.json`'s `disk`;
//   - with less than `CLEAN_BELOW` free at its start, runs `bot/machine/clean.sh` first, as the
//     machine does after every job and every ten minutes;
//   - with less than `FLOOR` free after that, does no work: the result is `infra` on the machine's
//     account (`infra_scope: "machine"`), so the item stays queued and nothing counts against it;
//   - and when the disk fills up during the work anyway, it pauses (`interrupted`, `disk`) and keeps
//     what it built, as for any pause.
//
// The deliver job keeps the newest reading in the state file (`machine_disk`, `note`), and `alert`
// opens one issue for people when a reading is `ALERT_PERCENT` full or has less than `FLOOR` free,
// rewrites it as readings come in, and closes it once one is back at `CLEAR_PERCENT` or under. The
// status loop (`bot-status.yml`, every ten minutes) runs `alert` again on the kept reading, so an
// issue that a deliver job could not open or close is settled within ten minutes.
package disk

import (
	"errors"
	"fmt"
	"io/fs"
	"path/filepath"
	"strings"
	"syscall"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Gib is GIB.
var Gib int = 1 << 30

// CleanBelow is CLEAN_BELOW: a job that starts with less free than this cleans first (`clean`).
var CleanBelow int = 8 * Gib

// Floor is FLOOR: a job that has less free than this after cleaning does no work: one job's
// worktrees, install and builds take about 1 to 2 GB (its Rust builds keep no incremental cache
// or debug info, `bot-night.yml`), and every other job going needs room to finish.
var Floor int = 3 * Gib

// AlertPercent is ALERT_PERCENT: the issue opens at this share of the disk used (or under
// `FLOOR` free) and closes at `CLEAR_PERCENT` or under, so a disk that hovers near one line does
// not open and close it.
var AlertPercent int = 80

// ClearPercent is CLEAR_PERCENT; see AlertPercent.
var ClearPercent int = 70

// CleanScript is CLEAN_SCRIPT: the clean-up, in the checkout every job has (setup.sh installs
// the same file on the machine).
var CleanScript string = "bot/machine/clean.sh"

// CleanSeconds is CLEAN_SECONDS.
var CleanSeconds int = 300

// FullText is how a full disk reads in an error from a command (git, a CLI) rather than an
// `OSError`.
var FullText string = "No space left on device"

// Title is TITLE.
var Title string = "Night bot: the machine's disk is filling up"

// Marker is MARKER.
var Marker string = "<!-- jackioh-bot:disk -->"

// Labels is LABELS.
var Labels = []string{"night bot", config.LabelHuman}

// Readings is the order a job's readings are taken in; the last one there is the newest.
var Readings = []string{"start", "after_clean", "end"}

// Reading is how full the filesystem holding `path` is. `percent` is as `df` counts it: used
// over what an ordinary user can have (the blocks kept for root left out). It raises the
// *fs.PathError of a path it cannot read, as Python raised OSError.
func Reading(path string, at time.Time) *pyjson.Object {
	total, free, used := diskUsage(path)
	room := used + free
	percent := 100
	if room != 0 {
		percent = py.Round(100 * float64(used) / float64(room))
	}
	return pyjson.New("total", int(total), "free", int(free), "percent", percent, "at", clock.ISO(at))
}

// diskUsage is shutil.disk_usage(path): its total, free (to an ordinary user) and used bytes.
func diskUsage(path string) (total, free, used uint64) {
	var st syscall.Statfs_t
	if err := syscall.Statfs(path, &st); err != nil {
		herr.Raise(&fs.PathError{Op: "statvfs", Path: path, Err: err})
	}
	frsize := uint64(st.Frsize) // statvfs's f_frsize, which glibc fills from f_bsize without one
	if frsize == 0 {
		frsize = uint64(st.Bsize)
	}
	return st.Blocks * frsize, st.Bavail * frsize, (st.Blocks - st.Bfree) * frsize
}

// IsFull is whether `exc` is a full disk.
func IsFull(exc error) bool {
	if exc == nil {
		return false
	}
	return errors.Is(exc, syscall.ENOSPC) || strings.Contains(exc.Error(), FullText)
}

// TooFull is disk.too_full.
func TooFull(found *pyjson.Object) bool {
	return py.MustInt(found.Get("percent")) >= AlertPercent || py.MustInt(found.Get("free")) < Floor
}

// Recovered is disk.recovered.
func Recovered(found *pyjson.Object) bool {
	return py.MustInt(found.Get("percent")) <= ClearPercent && py.MustInt(found.Get("free")) >= Floor
}

// Size is `17.3 GB`.
func Size(n int) string {
	return fmt.Sprintf("%.1f GB", float64(n)/float64(Gib))
}

// Describe is `83% full, 5.1 GB free of 30.0 GB`.
func Describe(found *pyjson.Object) string {
	return py.Str(found.Get("percent")) + "% full, " + Size(py.MustInt(found.Get("free"))) + " free of " +
		Size(py.MustInt(found.Get("total")))
}

// Clean is disk.clean; a variable so the tests can replace it.
var Clean = clean

// clean runs the machine's clean-up as this job's user, and returns the end of what it said. It
// never fails the job: a script that is missing or stuck is only reported.
func clean(root string, env map[string]string) string {
	script := filepath.Join(root, CleanScript)
	stdout, stderr, err := run([]string{"bash", script}, env, time.Duration(CleanSeconds)*time.Second)
	if decode, ok := err.(*unicodeDecodeError); ok {
		herr.Raise(decode) // output that is not UTF-8: Python's clean() let that through
	}
	if err != nil {
		return "the clean-up did not run: " + err.Error()
	}
	return py.SliceFrom(py.Strip(stdout+stderr), -2000)
}

// valid is _valid: a reading from a model job's result, checked: that job's files are not
// trusted.
func valid(found any) *pyjson.Object {
	obj, ok := found.(*pyjson.Object)
	if !ok || obj == nil {
		return nil
	}
	num := func(key string) (int, bool) { // int(obj[key]); False where it raises
		v, ok := obj.Lookup(key)
		if !ok {
			return 0, false
		}
		n, err := py.Int(v)
		return n, err == nil
	}
	total, ok1 := num("total")
	free, ok2 := num("free")
	percent, ok3 := num("percent")
	if !ok1 || !ok2 || !ok3 {
		return nil
	}
	if total <= 0 || !(0 <= free && free <= total) || !(0 <= percent && percent <= 100) {
		return nil
	}
	at := clock.ParseISOValue(obj.Get("at"))
	shown := ""
	if !at.IsZero() {
		shown = clock.ISO(at)
	}
	return pyjson.New("total", total, "free", free, "percent", percent, "at", shown)
}

// Newest is the newest valid reading a model job's result carries, if any.
func Newest(result *pyjson.Object) *pyjson.Object {
	readings := result.Obj("disk")
	for i := len(Readings) - 1; i >= 0; i-- {
		if found := valid(readings.Get(Readings[i])); found != nil {
			return found
		}
	}
	return nil
}

// Note keeps `found` as the machine's newest reading, unless the state holds a newer one.
func Note(state *pyjson.Object, found *pyjson.Object, runner string, runURL string) {
	entry := state.Obj("machine_disk")
	if entry == nil {
		entry = pyjson.New()
	}
	kept := clock.ParseISOValue(entry.Obj("reading").Get("at"))
	at := clock.ParseISOValue(found.Get("at"))
	if !kept.IsZero() && !at.IsZero() && at.Before(kept) {
		return
	}
	entry.Set("reading", found.Copy()).Set("runner", runner).Set("run", runURL)
	state.Set("machine_disk", entry)
}

// findOpen is the open disk issue: the one the state names, or one with the title and the
// marker.
func findOpen(ctx *hctx.Context, number any) *pyjson.Object {
	if pyjson.Truthy(number) {
		var issue *pyjson.Object
		if _, failed := herr.Catch[*herr.GitHubError](func() {
			issue = ctx.GH.GetIssue(py.MustInt(number))
		}); !failed && pyjson.Equal(issue.Get("state"), "open") {
			return issue
		}
	}
	for _, issue := range ctx.GH.ListIssues(Labels[0], "open", 300, "") {
		body := ""
		if issue.Truthy("body") {
			body = py.Str(issue.Get("body"))
		}
		if pyjson.Equal(issue.Get("title"), Title) && strings.Contains(body, Marker) && !issue.Has("pull_request") {
			return issue
		}
	}
	return nil
}

// closedSince is whether the issue the state names was closed after `found` was read.
func closedSince(ctx *hctx.Context, number any, found *pyjson.Object) bool {
	if !pyjson.Truthy(number) {
		return false
	}
	var closed time.Time
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		closed = clock.ParseISOValue(ctx.GH.GetIssue(py.MustInt(number)).Get("closed_at"))
	}); failed {
		return false
	}
	at := clock.ParseISOValue(found.Get("at"))
	return !closed.IsZero() && !at.IsZero() && !closed.Before(at)
}

// body is _body: the issue for people.
func body(entry *pyjson.Object) string {
	found := entry.Obj("reading")
	run := ""
	if entry.Truthy("run") {
		run = " ([run](" + py.Str(entry.Get("run")) + "))"
	}
	runner := "a job"
	if entry.Truthy("runner") {
		runner = "`" + py.Str(entry.Get("runner")) + "`"
	}
	return strings.Join([]string{
		"The bot's machine is **" + Describe(found) + "**, as " + runner + " read it at " +
			py.Str(found.Get("at")) + run + ".",
		"",
		"A full disk fails whichever job on it writes next. Every job there cleans up after " +
			"itself, the machine cleans every ten minutes (`bot/machine/clean.sh`), and a job that " +
			"starts with under " + Size(Floor) + " free does no work, so its item waits, uncounted. What " +
			"fills the disk now is something those leave.",
		"",
		"To fix it, a person (the bot cannot reach the machine):",
		"- sees what fills it: `bot/machine/on-machine.sh bot/machine/disk-report.sh`;",
		"- deletes what the report shows, or makes the disk larger in EC2 (Modify volume); the " +
			"machine grows into it at its next start, or at once with " +
			"`bot/machine/on-machine.sh bot/machine/setup.sh gpt agy muse`;",
		"- and, if the clean-up missed something that comes back, adds it to `clean.sh`.",
		"",
		"The bot rewrites this issue as readings come in and closes it once one is back at " +
			py.Str(ClearPercent) + "% or under with at least " + Size(Floor) + " free.",
		"",
		Marker,
	}, "\n")
}

// Alert opens, rewrites or closes the disk issue for the newest kept reading. Returns what it
// did ("" for None).
func Alert(ctx *hctx.Context) string {
	state := ctx.Store.Load()
	entry := state.Obj("machine_disk")
	found := valid(entry.Get("reading"))
	if found == nil {
		return ""
	}
	entry = entry.Copy().Set("reading", found)
	issue := findOpen(ctx, entry.Get("issue"))
	number, open := 0, issue != nil
	if open {
		number = py.MustInt(issue.Get("number"))
	}
	if TooFull(found) {
		if !open && closedSince(ctx, entry.Get("issue"), found) {
			return "" // a person closed it after this reading: wait for a newer one
		}
		noteText := ""
		if !open {
			number = py.MustInt(ctx.GH.CreateIssue(Title, body(entry), Labels).Get("number"))
			noteText = "opened #" + py.Str(number) + ": the machine's disk is " + Describe(found)
		} else if !pyjson.Equal(entry.Get("shown"), found.Get("at")) {
			ctx.GH.UpdateIssue(number, pyjson.New("body", body(entry)))
			noteText = "rewrote #" + py.Str(number) + ": the machine's disk is " + Describe(found)
		} else {
			return ""
		}
		shown := found.Get("at")
		ctx.Store.Update(func(s *pyjson.Object) any {
			s.SetDefault("machine_disk", pyjson.New()).(*pyjson.Object).Set("issue", number).Set("shown", shown)
			return nil
		}, "machine disk issue")
		return noteText
	}
	if !Recovered(found) || (!open && !entry.Truthy("issue")) {
		return ""
	}
	if open {
		ctx.GH.CreateComment(number, "The machine's disk is back to "+Describe(found)+
			" (read at "+py.Str(found.Get("at"))+"). Closing this.")
		ctx.GH.UpdateIssue(number, pyjson.New("state", "closed", "state_reason", "completed"))
	}
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.SetDefault("machine_disk", pyjson.New()).(*pyjson.Object).Set("issue", nil).Set("shown", nil)
		return nil
	}, "machine disk recovered")
	if open {
		return "closed #" + py.Str(number) + ": the machine's disk is " + Describe(found)
	}
	return ""
}

// Line is the status issue's line about the disk, or "" (None) before any machine job reported
// one.
func Line(state *pyjson.Object) string {
	entry := state.Obj("machine_disk")
	found := valid(entry.Get("reading"))
	if found == nil {
		return ""
	}
	mark := "🟢"
	if TooFull(found) {
		mark = "🔴"
	}
	issue := ""
	if entry.Truthy("issue") {
		issue = "; see #" + py.Str(entry.Get("issue"))
	}
	runner := ""
	if entry.Truthy("runner") {
		runner = " by `" + py.Str(entry.Get("runner")) + "`"
	}
	return mark + " **The machine's disk:** " + Describe(found) + ", read" + runner + " at " +
		py.Str(found.Get("at")) + issue + "."
}
