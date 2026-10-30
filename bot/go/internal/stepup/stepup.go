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
	"fmt"
	"slices"
	"sort"
	"strconv"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/threads"
)

// STRIKE_LOG: how many strike reasons an item keeps, for the comment that says why it stepped up.
var StrikeLog int = 10

// stateItem is state.item.
func stateItem(s *pyjson.Object, number int) *pyjson.Object { return state.Item(s, number) }

// recordOf is `state["items"].get(str(number), {})`.
func recordOf(s *pyjson.Object, number int) *pyjson.Object {
	if record := s.Obj("items").Obj(strconv.Itoa(number)); record != nil {
		return record
	}
	return pyjson.New()
}

// numberOf is `int(thread["number"])`, a KeyError when it has none.
func numberOf(thread *pyjson.Object) int {
	v, ok := thread.Lookup("number")
	if !ok {
		panic(fmt.Errorf("KeyError: 'number'"))
	}
	return py.MustInt(py.Or(v, 0))
}

// threadsOf is the order CPython iterates `{target, *([pr] if pr else [])}` in, which is the
// order of the GitHub calls made for each: by slot in a set's first table of 8, an int's slot
// its value & 7, a taken one probed onward as setobject.c does (`perturb >>= 5`, then
// `i = (i*5 + 1 + perturb) & 7`; no linear probe fits in 8 slots).
func threadsOf(target int, pr *int) []int {
	members := []int{target}
	if pr != nil && *pr != 0 {
		members = append(members, *pr)
	}
	var table [8]int
	var used [8]bool
	for _, x := range members {
		i, perturb := uint(x)&7, uint(x)
		for used[i] && table[i] != x {
			perturb >>= 5
			i = (i*5 + 1 + perturb) & 7
		}
		table[i], used[i] = x, true
	}
	var order []int
	for i, taken := range used {
		if taken {
			order = append(order, table[i])
		}
	}
	return order
}

// Try is stepup._try: run `action`, a GitHubError ignored.
func Try(action func()) {
	herr.Catch[*herr.GitHubError](action)
}

// `(the thread strikes count on, its open bot pull request or None)` for an issue or a pull
// request: a bot pull request's issue (its record's `issue`, else the one it says it closes),
// or the pull request itself when it closes none.
func Pair(ctx *hctx.Context, number int) (int, *int) {
	thread := ctx.GH.GetIssue(number)
	if !thread.Has("pull_request") {
		pull := queue.OpenPullForBranch(ctx, queue.BranchForIssue(number))
		if pull.Len() > 0 {
			n := numberOf(pull)
			return number, &n
		}
		return number, nil
	}
	record := recordOf(ctx.Store.Load(), number)
	issue := py.MustInt(py.Or(record.Get("issue"), 0))
	if issue == 0 {
		if linked := threads.LinkedIssue(thread); linked != nil {
			issue = *linked
		}
	}
	var openPR *int
	if thread.Get("state") == "open" {
		openPR = &number
	}
	if issue == 0 {
		issue = number
	}
	return issue, openPR
}

// lastN is `xs[-n:]` for n > 0.
func lastN(xs []any, n int) []any {
	if n >= len(xs) {
		return xs
	}
	return xs[len(xs)-n:]
}

// Count one failure of the item's own. At `STEP_UP_AFTER` it steps the item up (`step_up`)
// and returns True: the caller's own ending (a block, a requeue) is then replaced by it.
func Strike(ctx *hctx.Context, number int, why string, link string /* = "" */) bool {
	target, pr := Pair(ctx, number)

	change := func(s *pyjson.Object) any {
		entry := stateItem(s, target)
		entry.Set("strikes", py.MustInt(py.Or(entry.Get("strikes"), 0))+1)
		log := append(slices.Clone(pyjson.List(entry.Get("strike_log"))), py.SliceTo(why, 200))
		entry.Set("strike_log", slices.Clone(lastN(log, StrikeLog)))
		return nil
	}
	s := ctx.Store.Update(change, fmt.Sprintf("strike #%d", target))
	entry := stateItem(s, target)
	count := py.MustInt(py.Or(entry.Get("strikes"), 0))
	if count < config.StepUpAfter {
		return false
	}
	var reasons []string
	for _, r := range lastN(pyjson.List(entry.Get("strike_log")), count) {
		reasons = append(reasons, py.OrStr(r, ""))
	}
	StepUp(ctx, target, pr, reasons, link)
	return true
}

// The item's strikes are over: a head met the review rule and is green, or it merged.
func Clear(ctx *hctx.Context, number int) {
	target, _ := Pair(ctx, number)
	ctx.Store.Update(func(s *pyjson.Object) any {
		stateItem(s, target).Set("strikes", 0).Set("strike_log", []any{})
		return nil
	}, fmt.Sprintf("strikes #%d cleared", target))
}

// Every check branch protection requires (`required_checks`) passed on `sha`.
func Green(ctx *hctx.Context, sha string) bool {
	wanted := ctx.Cfg.RequiredChecks
	if len(wanted) == 0 || sha == "" {
		return false
	}
	passed := map[string]bool{}
	for _, run := range ctx.GH.CheckRuns(sha) {
		if c := run.Get("conclusion"); c == "success" || c == "neutral" || c == "skipped" {
			passed[py.Str(run.Get("name"))] = true
		}
	}
	for _, name := range wanted {
		if !passed[name] {
			return false
		}
	}
	return true
}

// A bot pull request's head that met the review rule (its record's `cleared`) and is green in
// CI ends its item's strikes (#316): a person's request never does. Returns whether it cleared.
func ClearIfGreen(ctx *hctx.Context, pr int, head string) bool {
	record := recordOf(ctx.Store.Load(), pr)
	cleared := record.Obj("cleared")
	if head == "" || cleared.Get("sha") != head {
		return false
	}
	target, _ := Pair(ctx, pr)
	if py.MustInt(py.Or(recordOf(ctx.Store.Load(), target).Get("strikes"), 0)) == 0 {
		return false
	}
	if !Green(ctx, head) {
		return false
	}
	Clear(ctx, pr)
	return true
}

func reasonsText(reasons []string) string {
	var kept []string
	for _, r := range reasons {
		if r != "" && !slices.Contains(kept, r) {
			kept = append(kept, r)
		}
	}
	if len(kept) == 0 {
		return "no reason recorded"
	}
	return strings.Join(kept, "; ")
}

// Raise `target`'s difficulty one step and build it again (`rebuild` its open pull request),
// or block it for a person when a person set its difficulty or it is hard already. Returns what
// it did, in words.
func StepUp(ctx *hctx.Context, target int, pr *int, reasons []string, link string /* = "" */) string {
	thread := ctx.GH.GetIssue(target)
	names := queue.LabelNames(thread)
	record := recordOf(ctx.Store.Load(), target)
	current := queue.DifficultyOf(names, queue.CarriedDifficulty(record))
	ctx.Store.Update(func(s *pyjson.Object) any {
		stateItem(s, target).Set("strikes", 0).Set("strike_log", []any{})
		return nil
	}, fmt.Sprintf("step up #%d", target))
	why := reasonsText(reasons)
	where := ""
	if link != "" {
		where = fmt.Sprintf(" (%s)", link)
	}
	difficulties := config.Difficulties
	hardest := difficulties[len(difficulties)-1]
	slash := config.Slash
	if current == hardest || queue.RatingSource(names, record) == "person" {
		for _, number := range threadsOf(target, pr) {
			Try(func() {
				queue.SetStateLabel(ctx, number, queue.LabelNames(ctx.GH.GetIssue(number)),
					config.LabelBlocked)
			})
		}
		var ask string
		if current == hardest {
			ask = "It is `difficulty:hard` already, so there is no stronger model to hand it to: " +
				fmt.Sprintf("it needs a person. Split it, or answer what blocks it, then `%s build`.", slash)
		} else {
			nxt := difficulties[slices.Index(difficulties, current)+1]
			ask = fmt.Sprintf("A person set `difficulty:%s`, so I do not change it: should it be "+
				"`difficulty:%s`? Relabel it if so, then `%s build` (or "+
				"`%s rebuild` on its pull request) tries again.", current, nxt, slash, slash)
		}
		Try(func() {
			ctx.GH.CreateComment(target, fmt.Sprintf("This failed %d times%s: %s. %s",
				config.StepUpAfter, where, why, ask))
		})
		return fmt.Sprintf("blocked #%d", target)
	}
	nxt := difficulties[slices.Index(difficulties, current)+1]
	wanted := "difficulty:" + nxt
	for _, number := range threadsOf(target, pr) {
		labels := queue.LabelNames(ctx.GH.GetIssue(number))
		// A set of strings, whose order Python leaves to its hash; sorted here.
		var names []string
		for name := range labels {
			names = append(names, name)
		}
		sort.Strings(names)
		for _, name := range names {
			low, isDifficulty := strings.CutPrefix(py.Lower(name), "difficulty:")
			if isDifficulty && slices.Contains(difficulties, low) && name != wanted {
				Try(func() { ctx.GH.RemoveLabel(number, name) })
			}
		}
		if !labels[wanted] {
			Try(func() { ctx.GH.AddLabels(number, []string{wanted}) })
		}
	}
	now := clock.ISO(ctx.Now())
	ctx.Store.Update(func(s *pyjson.Object) any {
		stateItem(s, target).Set("difficulty_by", pyjson.New(
			"difficulty", nxt, "provider", "step-up", "tier", "", "at", now,
			"why", fmt.Sprintf("failed %d times at difficulty:%s", config.StepUpAfter, current)))
		return nil
	}, fmt.Sprintf("raised #%d", target))
	hasPR := pr != nil && *pr != 0
	if hasPR {
		ctx.Store.Update(func(s *pyjson.Object) any {
			stateItem(s, *pr).Set("difficulty", nxt)
			return nil
		}, fmt.Sprintf("raised #%d", *pr))
	}
	said := fmt.Sprintf("This failed %d times at `difficulty:%s`%s: %s. Raised to "+
		"`%s`, so a model of that tier plans and builds it", config.StepUpAfter, current, where, why, wanted)
	if hasPR && target != *pr {
		Rebuild(ctx, target, *pr, fmt.Sprintf("it failed %d times at "+
			"`difficulty:%s` (%s); it was raised to `%s`", config.StepUpAfter, current, why, wanted), said)
		return fmt.Sprintf("raised #%d to %s and rebuilt #%d", target, nxt, *pr)
	}
	if thread.Has("pull_request") {
		// A pull request that closes no issue: revise it again, at the new difficulty.
		Try(func() {
			queue.SetStateLabel(ctx, target, queue.LabelNames(ctx.GH.GetIssue(target)),
				config.LabelRevise)
		})
	} else {
		Try(func() {
			queue.SetStateLabel(ctx, target, queue.LabelNames(ctx.GH.GetIssue(target)),
				config.LabelBuild)
		})
		ctx.Store.Update(func(s *pyjson.Object) any {
			stateItem(s, target).Set("kind", "build").Set("queued_at", now).Set("failures", 0).
				Set("interruptions", 0).Set("died", 0)
			return nil
		}, fmt.Sprintf("requeue #%d", target))
	}
	Try(func() { ctx.GH.CreateComment(target, said+"; it is queued again.") })
	return fmt.Sprintf("raised #%d to %s", target, nxt)
}

// Close bot pull request `pr`, keep its branch as `bot/old/issue-<n>-<date>`, and queue
// `issue` to build again from `main`, its plan and last findings kept for the new build, which is
// told about the old pull request (#317 part 4). Returns the line for the comment.
func Rebuild(ctx *hctx.Context, issue int, pr int, why string, said string /* = "" */) string {
	pull := ctx.GH.GetPull(pr)
	head := pull.Obj("head")
	branch, sha := py.OrStr(head.Get("ref"), ""), py.OrStr(head.Get("sha"), "")
	ctx.Store.Update(func(s *pyjson.Object) any {
		stateItem(s, pr).Set("rebuilt", true)
		return nil
	}, fmt.Sprintf("rebuild #%d", pr))
	if pull.Get("state") == "open" {
		if pull.Truthy("auto_merge") && pull.Truthy("node_id") {
			Try(func() { ctx.GH.DisableAutoMerge(py.Str(pull.Get("node_id"))) })
		}
		Try(func() { queue.SetStateLabel(ctx, pr, queue.LabelNames(pull), "") })
		Try(func() { ctx.GH.UpdatePull(pr, pyjson.New("state", "closed")) })
	}
	old := ""
	if branch == queue.BranchForIssue(issue) && sha != "" {
		old = fmt.Sprintf("%sold/issue-%d-%s", config.BranchPrefix, issue,
			ctx.Now().Format("20060102-1504"))
		if _, ok := herr.Catch[*herr.GitHubError](func() {
			ctx.GH.CreateBranch(old, sha)
			ctx.GH.DeleteBranch(branch)
		}); ok {
			old = ""
		}
	}
	Try(func() { ctx.GH.DeleteBranch(queue.WipBranch(pr)) })
	now := clock.ISO(ctx.Now())

	change := func(s *pyjson.Object) any {
		entry := stateItem(s, issue)
		// The pull request's last findings are what killed it: the new build starts from them.
		findings := stateItem(s, pr).Get("last_findings")
		if !pyjson.Truthy(findings) {
			findings = entry.Get("last_findings")
		}
		for _, key := range []string{"pr", "handoff", "wip"} {
			entry.Pop(key)
		}
		entry.Set("kind", "build").Set("queued_at", now).Set("failures", 0).Set("interruptions", 0).
			Set("died", 0).Set("strikes", 0).Set("strike_log", []any{}).Set("stop_requested", false).
			Set("previous_pr", pr).Set("previous_branch", old).Set("previous_why", py.SliceTo(why, 500)).
			Set("last_findings", append([]any{}, pyjson.List(findings)...)).
			Set("verdict_rounds", 0).Set("round_tier", "").Set("audit_tier", "")
		stateItem(s, pr).Pop("wip")
		return nil
	}
	ctx.Store.Update(change, fmt.Sprintf("rebuild #%d", issue))
	labels := queue.LabelNames(ctx.GH.GetIssue(issue))
	if labelPROpen := config.LabelPROpen; labels[labelPROpen] {
		Try(func() { ctx.GH.RemoveLabel(issue, labelPROpen) })
	}
	Try(func() {
		queue.SetStateLabel(ctx, issue, queue.LabelNames(ctx.GH.GetIssue(issue)), config.LabelBuild)
	})
	kept := ""
	if old != "" {
		kept = fmt.Sprintf(" Its branch is kept as `%s`.", old)
	}
	Try(func() {
		ctx.GH.CreateComment(pr, fmt.Sprintf("Closed to build #%d again from `main`: %s.%s", issue, why, kept))
	})
	lead := ""
	if said != "" {
		lead = said + ". "
	}
	line := fmt.Sprintf("%s#%d is closed and #%d is queued to build again from `main`.%s The "+
		"new build is told what went wrong with it.", lead, pr, issue, kept)
	Try(func() { ctx.GH.CreateComment(issue, line) })
	return line
}

// `/harness rebuild` on a bot pull request or its issue: rebuild at once, at the same
// difficulty. Returns the reply.
func RebuildRequest(ctx *hctx.Context, number int, by string) string {
	thread := ctx.GH.GetIssue(number)
	if queue.IsHuman(queue.LabelNames(thread)) {
		return queue.HumanReply(number, queue.LabelNames(thread))
	}
	slash := config.Slash
	var issue, pr int
	if thread.Has("pull_request") {
		if !queue.LabelNames(thread)[config.LabelPR] {
			return fmt.Sprintf("#%d is not a pull request I opened, so I will not close it.", number)
		}
		issue, _ = Pair(ctx, number)
		if issue == number {
			return fmt.Sprintf("#%d closes no issue, so there is nothing to build again from `main`.", number)
		}
		pr = number
	} else {
		issue = number
		found := queue.OpenPullForBranch(ctx, queue.BranchForIssue(number))
		if found.Len() == 0 {
			return fmt.Sprintf("#%d has no open pull request of mine to rebuild; `%s build` "+
				"queues it.", number, slash)
		}
		pr = numberOf(found)
	}
	Rebuild(ctx, issue, pr, fmt.Sprintf("@%s asked for it (`%s rebuild`)", by, slash), "")
	return fmt.Sprintf("Closed #%d and queued #%d to build again from `main`.", pr, issue)
}
