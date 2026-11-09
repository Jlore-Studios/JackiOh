package events

// Stopping red CI early (#2948, section 4; the owner's decision of 2026-11-03).
//
// CI's `workflow_run: completed` (`OnCI`) comes only once the whole run is over, often twenty
// minutes after a required job went red in its third. So the status loop looks at the CI runs still
// going on the bot's pull requests at each of its ticks (its runners refresh, every two minutes,
// reads them already: `dashboard.RefreshRunners`; its sweep, every five, reads them too, which is
// where Squishy's are looked at, `StopRedRuns`). When a required job (`config.RequiredJob`) has
// failed on a bot pull request's current head, on the bot's own branch:
//
//  1. it notes when it first saw that (`ci_red_seen` in the state file), and waits up to
//     `CIEarlyMinutes` from when the job failed for the run's other early jobs to report, or less
//     when no job of the run is still going;
//  2. it records the run (this attempt of it) in the state file's `ci_stopped`, first, then turns
//     the pull request's auto-merge off when it was on (a run cancelled before its summary jobs
//     leaves them skipped, which counts as passing), then cancels the rest of the run;
//  3. it queues the CI fix with the run's failed jobs and their log tails (`fixCI`, marked
//     `ci_early`), on the lane that owns the pull request, unless a run holds it already (a live
//     label) or one is queued: that run reads the head's CI as it ends.
//
// `OnCI` ignores the completed event of a run in `ci_stopped`, whatever its conclusion: the bot's
// own cancellation is never a red run, a re-run or a second fix. A person's branch, `main` and the
// merge queue's builds (`gh-readonly-queue/*`, `OnQueueCI`) are never stopped, nor a run on a head
// the pull request has moved on from.

import (
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
)

// CIEarlyMinutes is how long after a required job failed the status loop lets the rest of its CI
// run's early jobs report before it stops the run.
var CIEarlyMinutes = 3

// CIStoppedKey is the state file's list of the CI runs the bot stopped itself, each
// {"id", "attempt", "sha", "pr", "at"}; CIRedSeenKey its runs with a required job red, by
// `<id>/<attempt>`, each {"at", "pr", "sha"}: when the status loop first saw it.
var (
	CIStoppedKey = "ci_stopped"
	CIRedSeenKey = "ci_red_seen"
)

// CIStoppedKeep is how many stopped runs the list keeps at most, and CIStoppedDays how long it
// keeps one (and a run seen red): a run's completed event comes within hours of its cancel.
var (
	CIStoppedKeep = 100
	CIStoppedDays = 7
)

// CIEarlyRuns is how many runs going the sweep's look reads at most (`StopRedRuns`).
var CIEarlyRuns = 100

// runKey is `<id>/<attempt>`: one attempt of a run (a re-run is the same run's next attempt).
func runKey(run *pyjson.Object) string {
	return py.Str(run.Get("id")) + "/" + strconv.Itoa(attemptOf(run))
}

// attemptOf is a run's attempt (`run_attempt`, 1 when it says none).
func attemptOf(run *pyjson.Object) int {
	return py.MustInt(py.Or(run.Get("run_attempt"), 1))
}

// Stopped is whether `run` (this attempt of it) is one the bot stopped itself (`ci_stopped`).
func Stopped(s *pyjson.Object, run *pyjson.Object) bool {
	key := runKey(run)
	for _, entry := range pyjson.Objs(s.Get(CIStoppedKey)) {
		if runKey(entry) == key {
			return true
		}
	}
	return false
}

// earlyRun is whether `run` is one the early stop may look at: CI, still going, a pull request's,
// on one of this bot's own branches (never `main`, never the merge queue's).
func earlyRun(ctx *hctx.Context, run *pyjson.Object) bool {
	branch := py.OrStr(run.Get("head_branch"), "")
	return pyjson.Equal(run.Get("name"), ctx.Cfg.CIWorkflow) && !pyjson.Equal(run.Get("status"), "completed") &&
		pyjson.Equal(run.Get("event"), "pull_request") && strings.HasPrefix(branch, config.BranchPrefix) &&
		!strings.HasPrefix(branch, QueueBranch) && branch != ctx.Cfg.DefaultBranch
}

// headPull is the open bot pull request whose current head `run` ran on, and its number; nil when
// there is none (the head moved on, or it is not the bot's).
func headPull(ctx *hctx.Context, run *pyjson.Object) (int, *pyjson.Object) {
	var numbers []int
	for _, p := range pyjson.Objs(run.Get("pull_requests")) {
		if p.Truthy("number") {
			numbers = append(numbers, py.MustInt(py.Item(p, "number")))
		}
	}
	if len(numbers) == 0 {
		for _, p := range ctx.GH.ListPulls("open", py.Str(run.Get("head_branch")), 200) {
			numbers = append(numbers, py.MustInt(py.Item(p, "number")))
		}
	}
	for _, number := range numbers {
		pull := ctx.GH.GetPull(number)
		if pull.Get("state") == "open" && pyjson.Equal(pull.Obj("head").Get("sha"), run.Get("head_sha")) &&
			queue.LabelNames(pull)[config.LabelPR] {
			return number, pull
		}
	}
	return 0, nil
}

// StopRedRuns is the early stop over every CI run going on this bot's branches, read here (the
// sweep's, each tick of the status loop: Squishy's pull requests are looked at in its sweep). The
// notes of what it did.
func StopRedRuns(ctx *hctx.Context) []string {
	notes := []string{}
	for _, run := range ctx.GH.ListRepoRuns("in_progress", CIEarlyRuns, true) {
		if !earlyRun(ctx, run) {
			continue
		}
		var note string
		if exc, failed := herr.Catch[*herr.GitHubError](func() {
			note = StopRedCI(ctx, run, ctx.GH.ListJobs(run.Get("id"), true))
		}); failed {
			note = fmt.Sprintf("CI run %s could not be read: %s", py.Str(run.Get("id")), exc.Error())
		}
		if note != "" {
			notes = append(notes, note)
		}
	}
	return notes
}

// StopRedCI is the early stop's look at one CI run still going, with its jobs (the package's rules
// above): "" when it does nothing, else what it did or waits for.
func StopRedCI(ctx *hctx.Context, run *pyjson.Object, jobs []*pyjson.Object) string {
	if !earlyRun(ctx, run) {
		return ""
	}
	var red []string
	var failedAt time.Time
	going := false
	for _, job := range jobs {
		if !pyjson.Equal(job.Get("status"), "completed") {
			going = true
			continue
		}
		name := py.Str(job.Get("name"))
		if !config.RequiredJob(name, ctx.Cfg.RequiredChecks) || !py.In(job.Get("conclusion"), config.CIRed...) {
			continue
		}
		red = append(red, name)
		if at := clock.ParseISOValue(job.Get("completed_at")); !at.IsZero() && (failedAt.IsZero() || at.Before(failedAt)) {
			failedAt = at
		}
	}
	s := ctx.Store.Load()
	if len(red) == 0 || Stopped(s, run) {
		return ""
	}
	number, pull := headPull(ctx, run)
	if pull == nil {
		return ""
	}
	names := queue.LabelNames(pull)
	record := s.Obj("items").Obj(strconv.Itoa(number))
	if names[config.LabelBlocked] || queue.IsHuman(names) || record.Truthy("stop_requested") ||
		queue.ArchivedOf(record) != nil {
		return "" // left to a person, as `OnCI` leaves it (an archived hand-over among them, #3009)
	}
	now := ctx.Now()
	sha := py.Str(run.Get("head_sha"))
	key := runKey(run)
	seen := clock.ParseISOValue(s.Obj(CIRedSeenKey).Obj(key).Get("at"))
	if seen.IsZero() {
		seen = now
		ctx.Store.Update(func(s *pyjson.Object) any {
			found := pruned(s, CIRedSeenKey, now)
			found.Set(key, pyjson.New("at", clock.ISO(now), "pr", number, "sha", sha))
			s.Set(CIRedSeenKey, found)
			return nil
		}, fmt.Sprintf("ci red #%d", number))
	}
	since := seen
	if !failedAt.IsZero() && failedAt.Before(since) {
		since = failedAt
	}
	jobsText := ticks(red)
	if going && now.Sub(since) < time.Duration(CIEarlyMinutes)*time.Minute {
		return fmt.Sprintf("#%d: %s red in CI run %s; its other early jobs have until %s to report",
			number, jobsText, py.Str(run.Get("id")), clock.ISO(since.Add(time.Duration(CIEarlyMinutes)*time.Minute)))
	}
	return stopRun(ctx, number, pull, names, record, run, red)
}

// stopRun stops CI run `run` on pull request `number`'s head, where the required jobs `red` failed:
// recorded first, auto-merge off, cancelled, then its CI fix queued unless a run holds the pull
// request or one is queued for it.
func stopRun(ctx *hctx.Context, number int, pull *pyjson.Object, names map[string]bool,
	record *pyjson.Object, run *pyjson.Object, red []string) string {
	sha := py.Str(run.Get("head_sha"))
	if !markStopped(ctx, run, number) {
		return ""
	}
	if pull.Truthy("auto_merge") {
		queue.HoldMerge(ctx, pull)
	}
	did := "stopped the rest of it"
	if exc, failed := herr.Catch[*herr.GitHubError](func() { ctx.Act().CancelRun(run.Get("id")) }); failed {
		did = "could not cancel it (" + exc.Error() + ")"
	}
	runID := py.Str(run.Get("id"))
	if config.IsLive(names) || names[config.LabelRevise] {
		return fmt.Sprintf("#%d: %s red in CI run %s; %s; the run that holds it, or is queued for it, "+
			"reads its CI", number, ticks(red), runID, did)
	}
	failed := fmt.Sprintf("CI's required job(s) %s failed on `%s`, so I stopped the rest of its run "+
		"early (#2948)", ticks(red), py.SliceTo(sha, 12))
	return fmt.Sprintf("#%d: %s red in CI run %s; %s; %s", number, ticks(red), runID, did,
		strings.TrimPrefix(fixCI(ctx, number, pull, names, record, run, sha, failed, true, true),
			fmt.Sprintf("#%d: ", number)))
}

// markStopped records CI run `run` (this attempt of it), of pull request `number`, in the state
// file's `ci_stopped` before the bot cancels it, so `OnCI` ignores its completed event: false when
// another process stopped it meanwhile (the state file is compare-and-set).
func markStopped(ctx *hctx.Context, run *pyjson.Object, number int) bool {
	now := ctx.Now()
	already := false
	ctx.Store.Update(func(s *pyjson.Object) any {
		if already = Stopped(s, run); already {
			return nil
		}
		kept := []any{}
		for _, entry := range pyjson.Objs(s.Get(CIStoppedKey)) {
			if at := clock.ParseISOValue(entry.Get("at")); !at.IsZero() && now.Sub(at) < keepFor() {
				kept = append(kept, entry)
			}
		}
		kept = append(kept, pyjson.New("id", run.Get("id"), "attempt", attemptOf(run),
			"sha", py.Str(run.Get("head_sha")), "pr", number, "at", clock.ISO(now)))
		if len(kept) > CIStoppedKeep {
			kept = kept[len(kept)-CIStoppedKeep:]
		}
		s.Set(CIStoppedKey, kept)
		seen := pruned(s, CIRedSeenKey, now)
		seen.Pop(runKey(run))
		s.Set(CIRedSeenKey, seen)
		return nil
	}, fmt.Sprintf("ci stopped #%d", number))
	return !already
}

// pruned is the state file's object at `key` without the entries older than `CIStoppedDays`: a copy,
// for the caller to set.
func pruned(s *pyjson.Object, key string, now time.Time) *pyjson.Object {
	out := pyjson.New()
	for _, name := range s.Obj(key).Keys() {
		entry := s.Obj(key).Obj(name)
		if at := clock.ParseISOValue(entry.Get("at")); !at.IsZero() && now.Sub(at) < keepFor() {
			out.Set(name, entry)
		}
	}
	return out
}

// keepFor is `CIStoppedDays` as a duration.
func keepFor() time.Duration { return time.Duration(CIStoppedDays) * 24 * time.Hour }

// ticks is the job names, each in backticks, joined by commas.
func ticks(names []string) string {
	out := make([]string, len(names))
	for i, name := range names {
		out[i] = "`" + name + "`"
	}
	return strings.Join(out, ", ")
}
