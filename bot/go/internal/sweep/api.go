// Package sweep is bot/harness/sweep.py.
//
// The sweep: find every request the event handler never answered, and answer it now.
//
// Events are delivered once, and a handler can fail before it acts (GitHub is down, a write
// races, a bug). The labels are the queue and survive all of that, but a comment, a review, an
// assignment or a failed CI run lives only in its event. So every ten minutes, and on demand, this
// reads recent history back from GitHub itself and answers, through the same code the event would
// have used:
//
//   - a trusted comment, diff comment or review that carries a command or names the bot, and that
//     nobody has claimed (`events.claim`);
//   - a trusted review asking a bot pull request for changes, likewise unclaimed;
//   - an open issue or PR assigned to the bot that nothing ever queued;
//   - a failed CI run on the head of a bot PR that nothing reran or queued a fix for;
//   - a label the bot uses (`config.LABELS`) that the repository does not have: a new one used to
//     wait for a person to run `python3 -m harness setup`, so the method labels (#307) shipped
//     without existing, and no issue could be triaged; and one it no longer uses
//     (`config.RETIRED_LABELS`), which it deletes;
//   - a tree the bot split (`squishy:tree`, #60) whose sub-issues have all closed: its close-out is
//     queued, which checks the parent against `main` and closes it or opens what is missing;
//   - an issue the bot has planned, and a pull request it opened, that carries no `difficulty:*` label
//     (a planner that gave no rating line leaves none, #735): it gets its issue's label, or medium,
//     which is how it routes anyway; an issue still waiting for its plan is left to the planner;
//   - last, a night run that should be going and is not: GitHub drops scheduled runs, sometimes a
//     whole night of `bot-night`'s hourly ones, so when the gate's own question (`plan.peek`) finds
//     work for a free lane and no `bot-night` run is on its way to take it, the sweep starts one.
//     With `controller` on (#1455) the status loop's sweep plans the work and starts each run with
//     its plan (`controller.start`), and any other sweep leaves the work to the loop while it goes;
//   - just before that, in the status loop, the night runs it started that have ended: it delivers
//     them itself (`controller.deliver`, #1537), as they have no `deliver` job, so a run's item is
//     delivered within a tick of its model job ending, before the loop plans again.
//
// It looks back to the later of `LOOKBACK` and the first sweep ever (so the first one replays
// nothing), leaves anything younger than `SETTLE` to a handler that may still be running, and
// skips an edited comment, whose editor it cannot see: an author's edit arrives as its own event.
// One failure never stops the rest.
package sweep

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"time"
)

// LOOKBACK = timedelta(days=3)
var Lookback any // unported

// SETTLE = timedelta(minutes=10)
var Settle any // unported

// EDITED = timedelta(seconds=5)
var Edited any // unported

// QUEUE_LABELS = {LABEL_BUILD, LABEL_REVISE, LABEL_WORKING, LABEL_BLOCKED}
var QueueLabels any // unported

// LIVE_RUN = ("queued", "in_progress", "waiting", "pending", "requested")
var LiveRun = []string{"queued", "in_progress", "waiting", "pending", "requested"}

// sweep is sweep.sweep.

func Sweep(ctx *hctx.Context) []string {
	panic("unported: sweep.sweep")
}

// Create every label in `config.LABELS` the repository lacks and delete every one in
// `config.RETIRED_LABELS` it still has, once each time those lists change (`labels_synced`,
// their fingerprint, in the state file), so a tick normally reads nothing. A label a person
// deleted comes back with the next change to the list, or with `python3 -m harness setup`.

func Labels(ctx *hctx.Context, since time.Time) []string {
	panic("unported: sweep._labels")
}

// A bot pull request whose head met the review rule and then went green in CI ends its
// item's strikes (#316). Only a head the review rule cleared is looked at, and its check runs
// are read only when its item has strikes to clear.

func GreenHeads(ctx *hctx.Context, since time.Time) []string {
	panic("unported: sweep._green_heads")
}

// A tree the bot split (#60) whose sub-issues have all closed is queued for its close-out:
// the same split session, which checks the parent's end state against `main` and closes it, or
// opens the sub-issues still missing. A fullsend tree's close-out is its reconcile (#505): an
// ordinary build of the parent on its own branch, where its parts landed, which opens one pull
// request into `main`. One stopped (`stop`) or blocked is left alone.

func Trees(ctx *hctx.Context, since time.Time) []string {
	panic("unported: sweep._trees")
}

// The status loop delivers the night runs it started once they end (`controller.deliver`,
// #1537): before `_night_run`, so the lanes they held are free, and their items settled, when it
// plans. Any other sweep leaves them to the loop.

func Deliveries(ctx *hctx.Context, since time.Time) []string {
	panic("unported: sweep._deliveries")
}

// Start a night run when a lane and a subscription are free for work and no run is on its
// way to take it. Runs after the other parts, so a request they just answered counts. A run
// that is going but holds no lane yet (still in its gate or plan, or delivering) is on its way,
// so the sweep waits for it rather than piling more runs behind it.

func NightRun(ctx *hctx.Context, since time.Time) []string {
	panic("unported: sweep._night_run")
}
