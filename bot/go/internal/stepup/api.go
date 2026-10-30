// Package stepup is bot/harness/stepup.py.
//
// An item's own failures, and what three of them lead to (#317 parts 4 and 8).
//
// A strike is a run that failed for the item's own sake: a run that failed, a build or revision its
// reviewer would not approve, a review run that rejected its head, CI still red after its fixes, a
// run that died. Infra, a usage pause, a halt or a stop is no strike. Strikes count on the issue (a
// bot pull request's count on the issue it closes), and nothing a person asks resets them: only a
// head that meets the review rule and is green in CI (`clear_if_green`, from the sweep), a merge,
// or a step up does.
//
// At `STEP_UP_AFTER` (3) strikes the bot raises the item's difficulty one step, easy to medium to
// hard, so a stronger model plans and builds it, and rebuilds its open pull request from `main`
// rather than revising it again (`rebuild`): #141, #203 and #214 were revised for days, a weak
// builder committing conflict markers each time, until a person closed them. A difficulty a person set
// is never replaced: the bot asks instead, and so it does at hard, where there is no step left.
//
// `/harness rebuild` rebuilds at once, at the same difficulty.
package stepup

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
)

// STRIKE_LOG = 10
var StrikeLog int = 10

// _try is stepup._try.

func Try(action any) {
	panic("unported: stepup._try")
}

// `(the thread strikes count on, its open bot pull request or None)` for an issue or a pull
// request: a bot pull request's issue (its record's `issue`, else the one it says it closes),
// or the pull request itself when it closes none.

func Pair(ctx *hctx.Context, number int) (int, *int) {
	panic("unported: stepup.pair")
}

// Count one failure of the item's own. At `STEP_UP_AFTER` it steps the item up (`step_up`)
// and returns True: the caller's own ending (a block, a requeue) is then replaced by it.

func Strike(ctx *hctx.Context, number int, why string, link string /* = "" */) bool {
	panic("unported: stepup.strike")
}

// The item's strikes are over: a head met the review rule and is green, or it merged.

func Clear(ctx *hctx.Context, number int) {
	panic("unported: stepup.clear")
}

// Every check branch protection requires (`required_checks`) passed on `sha`.

func Green(ctx *hctx.Context, sha string) bool {
	panic("unported: stepup.green")
}

// A bot pull request's head that met the review rule (its record's `cleared`) and is green in
// CI ends its item's strikes (#316): a person's request never does. Returns whether it cleared.

func ClearIfGreen(ctx *hctx.Context, pr int, head string) bool {
	panic("unported: stepup.clear_if_green")
}

// Raise `target`'s difficulty one step and build it again (`rebuild` its open pull request),
// or block it for a person when a person set its difficulty or it is hard already. Returns what
// it did, in words.

func StepUp(ctx *hctx.Context, target int, pr *int, reasons []string, link string /* = "" */) string {
	panic("unported: stepup.step_up")
}

// Close bot pull request `pr`, keep its branch as `bot/old/issue-<n>-<date>`, and queue
// `issue` to build again from `main`, its plan and last findings kept for the new build, which is
// told about the old pull request (#317 part 4). Returns the line for the comment.

func Rebuild(ctx *hctx.Context, issue int, pr int, why string, said string /* = "" */) string {
	panic("unported: stepup.rebuild")
}

// `/harness rebuild` on a bot pull request or its issue: rebuild at once, at the same
// difficulty. Returns the reply.

func RebuildRequest(ctx *hctx.Context, number int, by string) string {
	panic("unported: stepup.rebuild_request")
}
