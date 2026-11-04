package deliver

// CI run again with no commit (#2948, section 4; the owner's decision of 2026-11-03).
//
// Red CI on a bot pull request's head gets its CI fix at once, with no flake re-run first
// (`events.OnCI`, `events.StopRedCI`): the fix run judges. Its builder runs the failing check
// locally first (`plan.CILocalNote`); when it passes and nothing needed changing, the run ends with
// no commit (`work.Worker.ciPassedLocally`, the result's `ci_local`), and its delivery runs that CI
// run again in full (`RerunRun`: every job, the cancelled and skipped ones too, as a new attempt),
// which costs none of the item's CI fixes (`max_failures`). A head is run again so at most
// `ci_reruns` times (the record's `ci_reruns`, by head; 1): past that a pass is no answer (the plan
// says so, and the run goes on as any CI fix), and a CI fix that ends with nothing to push and no
// re-run left needs a person (`bot:blocked`), since nothing else would run CI on that head again.

import (
	"fmt"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
)

// localPassed is: this run is a CI fix whose failing check passed locally, with nothing changed.
func (x *Deliverer) localPassed() bool {
	return x.Plan.Get("action") == "revise" && x.Result.Obj("ci_local").Truthy("passed")
}

// rerunHead runs CI again in full on pull request `number`'s head `head`, with no commit, when the
// head has re-runs left (`ci_reruns`), and counts it; `giveBack` takes back the CI fix this run was
// queued as (a local pass costs none). The line for the comment, and whether CI runs again.
func (x *Deliverer) rerunHead(number int, head string, giveBack bool) (string, bool) {
	record := x.Record(number)
	reruns := record.Obj("ci_reruns").Copy()
	if reruns == nil {
		reruns = pyjson.New()
	}
	done := py.MustInt(py.Or(reruns.Get(head), 0))
	if done >= x.Cfg.CIReruns {
		return fmt.Sprintf("CI already ran `%s` again %d time(s) with no commit (`ci_reruns`), so it "+
			"does not again.", py.SliceTo(head, 12), done), false
	}
	runID := record.Get("ci_run_id")
	if !pyjson.Truthy(runID) {
		return "No CI run was recorded to run again.", false
	}
	if problem := x.try(func() { x.Ctx.Act().RerunRun(runID) }); problem != "" {
		return "CI could not be run again: " + problem + ".", false
	}
	reruns.Set(head, done+1)
	fields := pyjson.New("ci_reruns", reruns)
	if giveBack {
		fields.Set("ci_fixes", max(0, py.MustInt(py.Or(record.Get("ci_fixes"), 0))-1))
	}
	x.remember(number, fields)
	return fmt.Sprintf("I asked CI to run `%s` again in full (run %s, every job), with no commit.",
		py.SliceTo(head, 12), py.Str(runID)), true
}

// ciStuck blocks pull request `number`, whose CI fix left its red head as it was with no re-run
// of CI left: nothing else would run CI on it again.
func (x *Deliverer) ciStuck(number int, why string) {
	queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelBlocked)
	x.holdAutoMerge(number)
	x.GH.CreateComment(number, fmt.Sprintf("This CI fix (%s) changed nothing, and CI is still red on "+
		"this head. %s It needs a person: a fix pushed, or `%s revise <notes>`.", x.link(), why, config.Slash))
}

// localPass delivers a CI fix whose failing check passed locally (`localPassed`) on pull request
// `number`: CI runs again on the head it started from, with no commit and none of its CI fixes
// spent, and the merge goes on as it stood: auto-merge on again for a head the reviews had
// cleared, else its own review run next (`own_review_run`) or a review run. `before` is its record
// as the run found it.
func (x *Deliverer) localPass(number int, before *pyjson.Object) {
	head := py.OrStr(x.Result.Get("start"), "")
	said, ok := x.rerunHead(number, head, true)
	if !ok {
		x.ciStuck(number, said)
		return
	}
	rounds, roundTier := x.rounds()
	x.remember(number, pyjson.New("feedback_since", before.Get("started_at"), "failures", 0, "source", "",
		"last_findings", []any{}, "question", "", "verdict_rounds", rounds, "round_tier", roundTier))
	waiting := x.Record(number).Truthy("pending_request")
	rule := ""
	switch {
	case waiting:
	case pyjson.Equal(queue.Cleared(before).Get("sha"), head):
		x.markReady(number, false)
		rule = "\n\n" + x.autoMerge(x.GH.GetPull(number), head)
	case x.Cfg.OwnReviewRun && x.Plan.Truthy("own_review_next"):
		rule = "\n\n" + x.handOn(number, head)
	default:
		rule = "\n\n" + x.awaitReview(number, head, true)
	}
	x.GH.CreateComment(number, fmt.Sprintf("CI fix (%s) on %s: it ran the failing check locally, and it "+
		"passed, with nothing to change, so I pushed nothing. %s That costs none of its CI fixes; CI "+
		"red on this head again is a CI fix that must change something.\n\n%s%s", x.link(), x.builtOn(),
		said, py.SliceTo(py.OrStr(x.Result.Get("report"), ""), ReportChars), rule))
}
