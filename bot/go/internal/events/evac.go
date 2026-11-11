package events

// Evacuating the merge queue (the owner's ask of 2026-11-07): a pull request whose merge-queue build
// failed a required job of its own leaves the queue at once, not when the build's slowest job ends.
//
// GitHub's merge queue (infra/github.tf `main_merge_queue`) takes a pull request out only once a
// required check reports red, and each of ci.yml's summary checks (`rust`, `web`, `e2e smoke`,
// `db`) reports only once every job it gathers has ended: a job that failed in a build's third
// minute held the queue, and every pull request behind it, until the last of those jobs ended,
// often twenty minutes on, mostly waiting for runners. So at each tick of the runner gate (every
// 30 seconds in the status loop, `--gate-every`, and every sweep) the night bot's process looks at
// the queue's builds still going. Once a required job (`config.RequiredJob`) has failed in the
// current build of a pull request in the queue, and the failure is that pull request's own, it
//
//  1. takes the pull request out of the queue (`gh.Dequeue`, with the bot's token: the Actions
//     token may not), and goes on only once GitHub says it is out: a build cancelled while its pull
//     request is still queued leaves its summary checks skipped, which count as passing, which is
//     why ci.yml's `queue-cancel-*` wait for the summary job;
//  2. records the build in `ci_stopped` (`markStopped`), so `OnCI` ignores its completed event;
//  3. cancels the rest of the build, which only holds runners now (out of the queue, nothing of it
//     can land);
//  4. queues the CI fix of a bot pull request as `OnQueueCI` would once the build ended
//     (`queueFix`), marked `ci_early`, in the same tick, so the housekeeping pass never finds it out
//     of the queue unheld and puts it back (`queue.RequeueCleared`). A person's pull request is told
//     why in a comment. Another bot's (Squishy's) is that bot's: its `OnQueueCI` hears of the
//     cancelled build, whose failed job says why.
//
// Which build is current, and whose failure it is. The queue builds each pull request on top of
// every one ahead of it, on `gh-readonly-queue/<base>/pr-<n>-<sha>`, where <sha> is the commit the
// build stands on: the head of the build ahead of it, or the base branch's own for the first (a
// group that lands fast-forwards the base branch to its head). So the current build of the first
// pull request in the queue is its newest build on the base branch's head, and the current build of
// each one after it is its newest build on the head of the current build ahead of it. A build that
// is none of these was rebuilt (ci.yml's `queue-supersede` cancels it) and is never looked at.
// A job that failed in the k-th pull request's build may have failed for a pull request ahead of
// it, whose changes that build holds too, and then the queue takes that one out instead and builds
// the k-th again without it. So a failure is the k-th's own only when every build ahead of it has
// already passed that job (or skipped it, or passed whole). The look goes down the queue from its
// first pull request and stops at the first build with a required job red: that pull request goes
// out when the failure is its own, and otherwise the look waits, a tick at a time, for the builds
// ahead of it; nothing behind a red build is read, since it holds whatever failed.
//
// A build that ended without passing is left to the queue, which takes its pull request out within
// seconds of a required check reporting red, for `EvacEndedAfter` (a minute). One still in the queue
// past that waits on a required check that never reported: GitHub dropped the jobs it gathers, or
// the check itself, and the queue would hold it, and every pull request behind it, for its whole
// check timeout (`EvacEndedWithin`, 240 minutes). #3505's build of 2026-11-09 ended at 18:15 with
// `rust`, `web` and `e2e smoke` never reported, and nothing landed until the queue gave up at 22:01.
// The look takes such a pull request out (`unstick`): as a red build's, when a required job of its
// own failed, and otherwise straight back in at the back of the queue, since nothing says it failed.

import (
	"fmt"
	"regexp"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cigate"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
)

// EvacRuns is how many of CI's newest runs the look reads for the queue's builds.
var EvacRuns = 100

// EvacEndedAfter is how long a build that ended without passing is left to the queue, which takes
// its pull request out within seconds of a required check reporting red: one still in the queue a
// minute on waits on a check that never reported (the file's comment).
var EvacEndedAfter = time.Minute

// EvacEndedWithin is how long after a build ended the look still reads it: the queue's own check
// timeout (infra/github.tf `check_response_timeout_minutes`), past which the queue has taken the
// pull request out itself.
var EvacEndedWithin = 240 * time.Minute

// evacPassed is the conclusions of a job in a build ahead that clear the same job's failure behind it.
var evacPassed = []string{"success", "skipped", "neutral"}

// groupBuild is a merge-queue build of pull request `number`, standing on commit `base`, with its
// jobs (read only for a build going) and the required ones red among them.
type groupBuild struct {
	run    *pyjson.Object
	number int
	base   string
	jobs   []*pyjson.Object
	red    []string
}

func (b *groupBuild) going() bool { return !pyjson.Equal(b.run.Get("status"), "completed") }

// stuck is whether this build ended without passing between `EvacEndedAfter` and `EvacEndedWithin`
// before `now` (its `updated_at`; a build that says no time is never stuck): its pull request, if
// the queue still holds it, waits on a required check that never reported.
func (b *groupBuild) stuck(now time.Time) bool {
	if b.going() || pyjson.Equal(b.run.Get("conclusion"), "success") {
		return false
	}
	ended := clock.ParseISOValue(b.run.Get("updated_at"))
	return !ended.IsZero() && now.Sub(ended) >= EvacEndedAfter && now.Sub(ended) < EvacEndedWithin
}

// redRequired is the names of the required jobs among `jobs` that ended red.
func redRequired(ctx *hctx.Context, jobs []*pyjson.Object) []string {
	var red []string
	for _, job := range jobs {
		name := py.Str(job.Get("name"))
		if pyjson.Equal(job.Get("status"), "completed") && py.In(job.Get("conclusion"), config.CIRed...) &&
			config.RequiredJob(name, ctx.Cfg.RequiredChecks) {
			red = append(red, name)
		}
	}
	return red
}

// passed is whether this build, ahead of a red one, clears a failure of job `name` there.
func (b *groupBuild) passed(name string) bool {
	if !b.going() {
		return pyjson.Equal(b.run.Get("conclusion"), "success")
	}
	for _, job := range b.jobs {
		if pyjson.Equal(job.Get("name"), name) && pyjson.Equal(job.Get("status"), "completed") &&
			py.In(job.Get("conclusion"), evacPassed...) {
			return true
		}
	}
	return false
}

// EvacQueue is the look at the merge queue's builds (the file's comment): the notes of what it did
// or waits for, nil when nothing in the queue is red or stuck. Only the runner gate's process looks
// (the night bot's): one process is enough, and Squishy's sweep skips it as it skips the gate.
func EvacQueue(ctx *hctx.Context) []string {
	if !strings.EqualFold(ctx.Cfg.BotLogin, cigate.Reviewer) {
		return nil
	}
	pattern := regexp.MustCompile("^" + regexp.QuoteMeta(QueueBranch+ctx.Cfg.DefaultBranch) + `/pr-(\d+)-([0-9a-f]+)$`)
	now := ctx.Now()
	var builds []*groupBuild
	look := false
	for _, run := range ctx.GH.ListRuns(cigate.Workflow, "", EvacRuns, true) {
		found := pattern.FindStringSubmatch(py.OrStr(run.Get("head_branch"), ""))
		if found == nil || !pyjson.Equal(run.Get("event"), "merge_group") {
			continue
		}
		b := &groupBuild{run: run, number: py.MustInt(found[1]), base: found[2]}
		if pyjson.Equal(run.Get("status"), "in_progress") {
			b.jobs = ctx.GH.ListJobs(run.Get("id"), true)
			b.red = redRequired(ctx, b.jobs)
		}
		look = look || len(b.red) > 0 || b.stuck(now)
		builds = append(builds, b)
	}
	if !look {
		return nil
	}
	base := ctx.GH.BranchSHA(ctx.Cfg.DefaultBranch)
	var ahead []*groupBuild
	for _, number := range ctx.GH.QueueOrder(ctx.Cfg.DefaultBranch) {
		b := currentBuild(builds, number, base)
		if b == nil {
			return nil // not built yet on what is ahead of it: nothing from here on can be read
		}
		if len(b.red) > 0 {
			return evacuate(ctx, b, ahead)
		}
		if !b.going() && !pyjson.Equal(b.run.Get("conclusion"), "success") {
			if b.stuck(now) {
				return unstick(ctx, b, ahead)
			}
			return nil // it ended red or cancelled just now: the queue takes it out itself
		}
		ahead = append(ahead, b)
		base = py.Str(b.run.Get("head_sha"))
	}
	return nil
}

// currentBuild is pull request `number`'s newest build standing on `base`, or nil.
func currentBuild(builds []*groupBuild, number int, base string) *groupBuild {
	var found *groupBuild
	for _, b := range builds {
		if b.number == number && base != "" && b.base == base &&
			(found == nil || py.MustInt(b.run.Get("id")) > py.MustInt(found.run.Get("id"))) {
			found = b
		}
	}
	return found
}

// evacuate takes the pull request of build `b`, red, out of the queue when its failure is its own
// (`ahead`, the current builds ahead of it, all passed one of its red jobs): out, recorded,
// cancelled, then its fix queued or its person told. The notes.
func evacuate(ctx *hctx.Context, b *groupBuild, ahead []*groupBuild) []string {
	number := b.number
	runID := py.Str(b.run.Get("id"))
	var own []string
	for _, name := range b.red {
		cleared := true
		for _, a := range ahead {
			cleared = cleared && a.passed(name)
		}
		if cleared {
			own = append(own, name)
		}
	}
	if len(own) == 0 {
		return []string{fmt.Sprintf("#%d: %s red in its queue build (run %s); waiting for the builds ahead "+
			"of it to pass the same, since it holds their changes too", number, ticks(b.red), runID)}
	}
	if Stopped(ctx.Store.Load(), b.run) {
		return nil // stopped already, by another process
	}
	pull := ctx.GH.GetPull(number)
	node := py.OrStr(pull.Get("node_id"), "")
	if node == "" || pull.Get("state") != "open" {
		return nil
	}
	herr.Catch[*herr.GitHubError](func() { ctx.GH.Dequeue(node) }) // out of the queue already
	var queued bool
	if exc, failed := herr.Catch[*herr.GitHubError](func() { queued = ctx.GH.InMergeQueue(node) }); failed {
		return []string{fmt.Sprintf("#%d: %s red in its queue build (run %s), and whether it left the merge "+
			"queue could not be read (%s), so the build goes on and its check takes it out", number, ticks(own),
			runID, py.SliceTo(exc.Error(), 200))}
	}
	if queued {
		return []string{fmt.Sprintf("#%d: %s red in its queue build (run %s), and the merge queue kept it, so "+
			"the build goes on and its check takes it out", number, ticks(own), runID)}
	}
	if !markStopped(ctx, b.run, number) {
		return nil
	}
	// A build that ended (`unstick`) has nothing left to stop: the queue kept its pull request because
	// a required check never reported. Read before the cancel, which ends the build.
	going := b.going()
	did := " and stopped the rest of the build"
	told := "so I took this out of the queue at once and stopped the rest of that build"
	if !going {
		did = ", which kept it after the build ended, waiting on a required check that never reported"
		told = "and the queue kept this in it after the build ended, waiting on a required check that never " +
			"reported, so I took this out"
	} else if exc, failed := herr.Catch[*herr.GitHubError](func() { ctx.Act().CancelRun(b.run.Get("id")) }); failed {
		did = " and could not stop the rest of the build (" + py.SliceTo(exc.Error(), 200) + ")"
	}
	note := fmt.Sprintf("#%d: %s red in its queue build (run %s); took it out of the merge queue%s",
		number, ticks(own), runID, did)
	names := queue.LabelNames(pull)
	switch {
	case names[config.LabelPR]:
		failed := fmt.Sprintf("The merge queue's build of `%s` with this on top failed %s, %s",
			ctx.Cfg.DefaultBranch, ticks(own), told)
		fixed := queueFix(ctx, number, pull, b.run, failed, going)
		return []string{note + "; " + strings.TrimPrefix(fixed, fmt.Sprintf("#%d: ", number))}
	case otherBot(names) != "":
		return []string{note + "; " + otherBot(names) + "'s, whose CI event of the build answers it"}
	}
	link := py.OrStr(b.run.Get("html_url"), "")
	ctx.GH.CreateComment(number, fmt.Sprintf("The merge queue's build of `%s` with this on top failed %s "+
		"([run](%s)), %s; the pull requests behind it go on without it. Fix it and queue it again.",
		ctx.Cfg.DefaultBranch, ticks(own), link, told))
	return []string{note + "; told its author"}
}

// unstick is the look at build `b`, ended without passing while the queue still holds its pull
// request (`stuck`), with `ahead` the current builds ahead of it. With a required job of its own red
// it goes out as a red build's does (`evacuate`). With none, nothing says it failed, so it goes out
// and straight back in at the back of the queue, which builds it again, whoever's it is: once for
// each commit `main` is at, by the item's `requeued` record, which `queue.RequeueCleared` keeps for
// the same. One that sticks again stays out: the night bot's own goes back from the housekeeping
// pass once `main` moves, and anyone else's (a person's, Squishy's) is told why. The notes.
func unstick(ctx *hctx.Context, b *groupBuild, ahead []*groupBuild) []string {
	number := b.number
	runID := py.Str(b.run.Get("id"))
	b.jobs = ctx.GH.ListJobs(b.run.Get("id"), true)
	if b.red = redRequired(ctx, b.jobs); len(b.red) > 0 {
		return evacuate(ctx, b, ahead)
	}
	pull := ctx.GH.GetPull(number)
	node := py.OrStr(pull.Get("node_id"), "")
	if node == "" || pull.Get("state") != "open" {
		return nil
	}
	ended := fmt.Sprintf("its queue build (run %s) ended %s with no required job red, the queue waiting on a "+
		"required check that never reported", runID, py.OrStr(b.run.Get("conclusion"), "unfinished"))
	herr.Catch[*herr.GitHubError](func() { ctx.GH.Dequeue(node) }) // out of the queue already
	var queued bool
	if exc, failed := herr.Catch[*herr.GitHubError](func() { queued = ctx.GH.InMergeQueue(node) }); failed {
		return []string{fmt.Sprintf("#%d: %s, and whether it left the merge queue could not be read (%s)",
			number, ended, py.SliceTo(exc.Error(), 200))}
	}
	if queued {
		return []string{fmt.Sprintf("#%d: %s, and the merge queue kept it", number, ended)}
	}
	note := fmt.Sprintf("#%d: %s; took it out of the merge queue", number, ended)
	head := py.OrStr(pull.Obj("head").Get("sha"), "")
	main := ctx.GH.BranchSHA(ctx.Cfg.DefaultBranch)
	tried := ctx.Store.Load().Obj("items").Obj(strconv.Itoa(number)).Obj("requeued").Copy()
	if head == "" || main == "" || tried.Get(head) == main {
		if queue.LabelNames(pull)[config.LabelPR] {
			return []string{note + ", and it went back once already at this commit of `main`, so it waits for " +
				"`main` to move, when the housekeeping pass puts it back"}
		}
		ctx.GH.CreateComment(number, fmt.Sprintf("The merge queue's build of `%s` with this on top ended "+
			"([run](%s)) without a required check reporting, which would have held the queue and every pull "+
			"request behind it for hours, so I took this out. It went back in once already at this commit of "+
			"`%s` and stuck the same way, so queue it again once you have looked.", ctx.Cfg.DefaultBranch,
			py.OrStr(b.run.Get("html_url"), ""), ctx.Cfg.DefaultBranch))
		return []string{note + ", and it went back once already at this commit of `main`; told its author"}
	}
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		ctx.GH.EnableAutoMerge(node, ctx.Cfg.MergeMethod, head, queue.MergeTitle(pull, ctx.Cfg.MergeMethod))
	}); failed {
		return []string{fmt.Sprintf("%s, and could not put it back (%s)", note, py.SliceTo(exc.Error(), 200))}
	}
	tried.Set(head, main)
	ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, number).Set("requeued", tried)
		return nil
	}, fmt.Sprintf("requeue stuck #%d", number))
	return []string{note + " and put it back at the back of it"}
}

// otherBot is the name of the other bot (`config.Others`) whose pull request carries these
// labels, or "".
func otherBot(names map[string]bool) string {
	for _, other := range config.Others {
		if names[other.LabelPrefix+"pr"] {
			return other.Name
		}
	}
	return ""
}
