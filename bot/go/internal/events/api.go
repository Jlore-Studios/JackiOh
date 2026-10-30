// Package events is bot/harness/events.py.
//
// GitHub events, handled at once with no model call: commands, labels, assignment, CI results.
//
// Runs in `bot-commands.yml`. Commands are acknowledged with a reaction and answered in one reply.
// Model work is only ever queued here; it happens in the night run, or at once when an operator adds
// `--force`.
package events

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/asks"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/commands"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// ACTIONS_BOT = "github-actions[bot]"
var ActionsBot string = "github-actions[bot]"

// CI_FAILED = ("failure", "timed_out", "cancelled", "startup_failure")
var CIFailed = []string{"failure", "timed_out", "cancelled", "startup_failure"}

// The workflows whose checks a bot pull request must pass: CI, whose jobs include the bot's own
// selftest (it had a workflow of its own, `bot selftest`, until #1455).

func CIWorkflows(ctx *hctx.Context) []string {
	panic("unported: events.ci_workflows")
}

// FROM_THE_BOT_TOO = {("workflow_run", "completed"), ("pull_request_target", "closed"), ("pull_request", "closed")}
var FromTheBotToo any // unported

// handle is events.handle.

func Handle(ctx *hctx.Context, name string, payload *pyjson.Object) []string {
	panic("unported: events.handle")
}

// OTHER_BOT_VERBS = ("stop",)
var OtherBotVerbs = []string{"stop"}

// The comment's author is another bot in the repository (`config.OTHERS`).

func FromOtherBot(user *pyjson.Object) bool {
	panic("unported: events.from_other_bot")
}

// True for a comment the bot wrote (a person quoting it keeps the marker behind `>`).

func IsOwn(body string) bool {
	panic("unported: events._is_own")
}

// ANSWERED = asks.ANSWERED
var Answered any // unported

// True when the bot has already answered this comment (its rocket reaction is there).

func Answered_(ctx *hctx.Context, commentID int, reviewComment bool /* = False */) bool {
	panic("unported: events.answered")
}

// NUDGE = ("I saw my name, but no request I could act on. Start a line with " "`@{bot} <what you want>` or `" + SLASH + " build`; `" + SLASH + " help` lists the r
var Nudge any // unported

// CLAIM_DAYS = 5
var ClaimDays int = 5

// Record, compare-and-set, that these lines of one comment or review are being answered.
//
// Returns the lines nobody claimed before, so an event handler and the sweep, or a comment and
// its edit, never act on the same line twice. A line is claimed before it runs: a handler that
// dies half-way leaves its 👀 and no reply rather than running a command a second time.

func Claim(ctx *hctx.Context, key string, lines []string) []string {
	panic("unported: events.claim")
}

// claimed is events.claimed.

func Claimed(ctx *hctx.Context, key string) bool {
	panic("unported: events.claimed")
}

// Answer the commands in a comment, a line comment on a diff, or an edit of either.

func OnComment(ctx *hctx.Context, payload *pyjson.Object, reviewComment bool /* = False */) []string {
	panic("unported: events.on_comment")
}

// Run each command and return its reply. `ask` is the comment they came from: whatever
// queues model work records it, so the later stages can react to it (`asks`).

func RunCommands(ctx *hctx.Context, found []*commands.Command, thread *pyjson.Object, user *pyjson.Object, level int, ask *asks.Ask /* = None */) []string {
	panic("unported: events.run_commands")
}

// execute is events.execute.

func Execute(ctx *hctx.Context, command *commands.Command, thread *pyjson.Object, user *pyjson.Object, ask *asks.Ask /* = None */) string {
	panic("unported: events.execute")
}

// Issue `number` is one of my suggestions, still open: the one thread a bare `approve` or
// `reject` votes on (#760). Anywhere else the words are only words.

func OpenSuggestion(ctx *hctx.Context, number int) bool {
	panic("unported: events.open_suggestion")
}

// A person's vote on one of my suggestions (#760): `approve` queues it to build, which takes
// `bot:suggestion` off (`queue_build`), as `bot:build` does; `reject` closes it as not planned,
// which frees its place under `suggestions.max_open` for the next survey, and goes into the log
// of suggestions that came to nothing (`declined`, #797). A rejected one keeps its label, so it
// still reads as a suggestion, and reopened it can be approved.

func AnswerVote(ctx *hctx.Context, vote string, number int, by string, ask *asks.Ask /* = None */) string {
	panic("unported: events.answer_vote")
}

// `stop` on a parent the bot split (#60): its open sub-issues leave the queue too. Its own
// are stopped here; one queued for another bot gets that bot's own `stop` command, so it stops
// its work the way it always does.

func StopTree(ctx *hctx.Context, parent int, by string) string {
	panic("unported: events.stop_tree")
}

// A review: its body's commands, a hint when it names the bot, or a revision when a trusted
// person asks a bot pull request for changes. Claimed like a comment, so it runs once.

func OnReview(ctx *hctx.Context, payload *pyjson.Object) []string {
	panic("unported: events.on_review")
}

// on_issue_change is events.on_issue_change.

func OnIssueChange(ctx *hctx.Context, payload *pyjson.Object, isPR bool) []string {
	panic("unported: events.on_issue_change")
}

// A closed or merged bot pull request's unfinished revision (`bot/wip/<pr>`, #313) is done
// with: delete the branch and drop the record's `wip`.

func DropWip(ctx *hctx.Context, number int) {
	panic("unported: events._drop_wip")
}

// on_pull_closed is events.on_pull_closed.

func OnPullClosed(ctx *hctx.Context, payload *pyjson.Object) []string {
	panic("unported: events.on_pull_closed")
}

// on_ci is events.on_ci.

func OnCI(ctx *hctx.Context, payload *pyjson.Object) []string {
	panic("unported: events.on_ci")
}

// QUEUE_BRANCH = "gh-readonly-queue/"
var QueueBranch string = "gh-readonly-queue/"

// CI failed on the merge queue's build of a pull request (infra/github.tf): `main` with every
// pull request ahead of it and this one on top. Its own CI ran on an older `main`, so the rest
// of `on_ci` never hears of it. One still in the queue was built again without a pull request
// ahead of it that failed, and is left to the queue. One taken out failed itself, and gets a CI
// fix with this run's logs at once, with no re-run: its own CI already passed, so what failed is
// nearly always what `main` and it do together, which fails the same way again (four of the five
// red runs on main of 2026-10-29). The revision merges `main` first and is reviewed as any, and
// its auto-merge queues it again at the back. A held pull request (a revision, a stop,
// `queue.hold_merge`) and a head the reviews never cleared are left alone.

func OnQueueCI(ctx *hctx.Context, run *pyjson.Object) []string {
	panic("unported: events.on_queue_ci")
}
