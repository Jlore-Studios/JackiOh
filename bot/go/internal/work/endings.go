package work

import (
	"errors"
	"fmt"
	"io/fs"
	"path/filepath"
	"sort"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/prompts"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
)

// ------------------------------------------------------------------ endings

// Commit what the worktree holds, so the deliver job can keep it.
func (x *Worker) saveWip(reason string) {
	if x.Wt == nil {
		return
	}
	_ = herr.Try(func() { // a failed save leaves the branch as it was
		if x.Wt.MergeInProgress() && len(x.unresolvedMarkers()) > 0 {
			run(x.Wt, false, "merge", "--abort")
		}
		x.commit(config.CommitPrefix + ": work in progress (" + reason + ")")
	})
}

// Interrupted is work.Worker._interrupted.
func (x *Worker) Interrupted(stop *Interrupt) {
	x.holdSignals()
	x.Step("stop", py.SliceTo(stop.Reason, 300), pyjson.New("kind", stop.Kind))
	x.saveWip(stop.Reason)
	status, ok := KindStatus[stop.Kind]
	if !ok {
		status = "interrupted"
	}
	x.Result.Set("status", status)
	x.Result.Set("reason", stop.Reason)
	x.Result.Set("interrupt", stop.Kind)
	x.Result.Set("reset_at", noneIfEmpty(stop.ResetAt))
	x.finish()
}

// Record the branch's head and bundle what it has beyond where it started.
//
// An approved change is bundled exactly as the reviewer saw it: a commit that appeared
// after the review (a process the model left behind, say) is dropped.
func (x *Worker) finish() {
	if x.Wt == nil {
		return
	}
	if err := herr.Try(func() {
		if x.Result.Get("status") == "approved" && x.ApprovedSHA != "" {
			if x.Wt.Head() != x.ApprovedSHA {
				x.Result.Set("dropped_after_review", x.Wt.Log(x.ApprovedSHA, "HEAD"))
				run(x.Wt, true, "reset", "--quiet", "--hard", x.ApprovedSHA)
			}
		}
		x.recordHead()
	}); err != nil {
		x.Result.Set("bundle_error", py.SliceTo(redact.Redact(herr.Str(err), nil), 500))
	}
}

// The branch's head, what it changed, and a bundle of what it has beyond where it
// started, in the result: at the run's end (`_finish`) and at each checkpoint.
func (x *Worker) recordHead() {
	head := x.Wt.Head()
	x.Result.Set("head", head)
	x.Result.Set("changed_paths", pyjson.ListOf(x.Wt.ChangedPaths(x.BaseRef, "HEAD")))
	x.Result.Set("diffstat", py.SliceFrom(x.Wt.Diffstat(x.BaseRef, "HEAD"), -4000))
	bundle := filepath.Join(x.OutDir, "branch.bundle")
	if head != x.StartSHA {
		// Python's `[s for s in {base_sha, start_sha} if s]`: a set, whose order is the string
		// hash's (random per process); git takes the excluded commits in any order.
		var exclude []string
		for _, s := range []string{x.BaseSHA, x.StartSHA} {
			if s != "" && !contains(exclude, s) {
				exclude = append(exclude, s)
			}
		}
		x.Wt.Bundle(bundle, py.Str(x.planItem("branch")), exclude)
		x.Result.Set("bundle", filepath.Base(bundle))
	} else {
		// Back where it started (a guard put the change back): a checkpoint's bundle is stale.
		x.Result.Pop("bundle")
		unlinkMissingOK(bundle)
	}
}

// ------------------------------------------------------------------ suggestions

func (x *Worker) suggest() {
	count := x.Plan.Get("count")
	if !pyjson.Truthy(count) {
		count = 0
	}
	n := py.MustInt(count)
	if n <= 0 {
		x.Result.Set("status", "nothing")
		x.Result.Set("reason", "the suggestion cap is full")
		return
	}
	fetchRef(x.Repo, x.Cfg.DefaultBranch, true)
	path := filepath.Join(x.WorkDir, "suggest")
	wt := git.WorktreeAdd(x.Repo, path, "bot-suggest", x.BaseRef)
	x.Check()
	x.rejudge(wt.Cwd)
	existing := x.Plan.Get("existing")
	if !pyjson.Truthy(existing) {
		existing = prompts.Data("(none)", "Issues")
	}
	declined := x.Plan.Get("declined")
	if !pyjson.Truthy(declined) {
		declined = prompts.Data("(none)", "Suggestions that came to nothing")
	}
	prompt := x.Render("suggest", pyjson.New("repo", x.Cfg.Repo, "count", n,
		"existing", existing, "declined", declined,
		"priorities", ClassPriorities(x.Cfg.SuggestionPriorities), "area", SurveyArea(x.Plan)))
	result := x.Call("suggest", prompt, wt.Cwd, true,
		"propose improvements while a lane has nothing to start", false)
	found := verdicts.Suggestions(result.Text, n)
	judged := x.judge(found, wt.Cwd)
	suggestions := []any{}
	for i, s := range found {
		entry := pyjson.New("title", s.Title, "body", s.Body, "class", s.Kind)
		if j := judged[i+1]; j != nil {
			entry.Set("judge", pyjson.New("approve", j.Approve, "why", j.Why))
		}
		suggestions = append(suggestions, entry)
	}
	x.Result.Set("status", "suggested")
	x.Result.Set("reason", fmt.Sprintf("%d suggestion(s)", len(found)))
	x.Result.Set("suggestions", suggestions)
}

// JudgeAttempts is how many fresh judge calls a run makes for the suggestions still unanswered: the
// first for all of them, then one more for those it gave no word on (#5104).
var JudgeAttempts = 2

// rejudge judges the open suggestions an earlier survey's judge never answered (the plan's
// `rejudge`, `plan.Unjudged`), before this survey's own work can spend the run or the cap it needs,
// and keeps its word in the result as `rejudged`, one entry an issue, for delivery to put on each
// (`deliver.Deliverer.applyRejudged`). An interrupt later in the run leaves the entries in place.
func (x *Worker) rejudge(cwd string) {
	leftovers := pyjson.Objs(x.Plan.Get("rejudge"))
	if len(leftovers) == 0 {
		return
	}
	found := make([]*verdicts.Suggestion, len(leftovers))
	for i, l := range leftovers {
		found[i] = &verdicts.Suggestion{Title: fmt.Sprintf("%s (already open as #%d)", l.Str("title"),
			l.Int("number")), Body: l.Str("body")}
	}
	judged := x.judge(found, cwd)
	out := []any{}
	for i, l := range leftovers {
		entry := pyjson.New("number", l.Int("number"))
		if j := judged[i+1]; j != nil {
			entry.Set("judge", pyjson.New("approve", j.Approve, "why", j.Why))
		}
		out = append(out, entry)
	}
	x.Result.Set("rejudged", out)
}

// judge is a survey's second look at its suggestions (`suggestions.auto_approve`,
// `prompts/judge.md`): a fresh call that argues against each one, reading it against the latest
// decisions people made on suggestions and the open issues and pull requests the survey read
// (`existing`, so it sees work already in flight, #5104), and says, of each, whether it would
// approve it. Its word, by each suggestion's number from 1; none when auto-approval is off or there
// is nothing to judge. A call that answers only some of them is followed by a fresh one for the rest,
// up to `JudgeAttempts` calls, and each way a call can fail leaves a step saying which. The
// suggestions matter more than the judge's word on them, so one still unanswered opens with none,
// and a later survey judges it again (`plan.Unjudged`).
func (x *Worker) judge(found []*verdicts.Suggestion, cwd string) map[int]*verdicts.Judgement {
	judged := map[int]*verdicts.Judgement{}
	if !x.Cfg.AutoApprove.Enabled || len(found) == 0 {
		return judged
	}
	missing := make([]int, len(found)) // the numbers from 1 still unanswered
	for i := range found {
		missing[i] = i + 1
	}
	for attempt := 1; attempt <= JudgeAttempts && len(missing) > 0; attempt++ {
		batch := make([]*verdicts.Suggestion, len(missing))
		for i, n := range missing {
			batch[i] = found[n-1]
		}
		got := x.judgeOnce(batch, cwd, attempt)
		still := []int{}
		for i, n := range missing {
			if j := got[i+1]; j != nil {
				judged[n] = j
			} else {
				still = append(still, n)
			}
		}
		if len(got) > 0 && len(still) > 0 {
			x.Step("judge", fmt.Sprintf("the judge answered %d of %d (try %d)", len(missing)-len(still),
				len(missing), attempt), pyjson.New())
		}
		missing = still
	}
	return judged
}

// judgeOnce is one fresh judge call on `found`, its word by their numbers from 1. A call that is
// refused, fails or times out answers none, with a step saying why.
func (x *Worker) judgeOnce(found []*verdicts.Suggestion, cwd string, attempt int) map[int]*verdicts.Judgement {
	judged := map[int]*verdicts.Judgement{}
	var listed []string
	for i, s := range found {
		kind := s.Kind
		if kind == "" {
			kind = "none"
		}
		listed = append(listed, fmt.Sprintf("### Suggestion %d: %s\n\nClass: `%s`\n\n%s", i+1, s.Title, kind, s.Body))
	}
	decided := x.Plan.Get("decided")
	if !pyjson.Truthy(decided) {
		decided = prompts.Data("(none)", "Recent decisions on suggestions")
	}
	existing := x.Plan.Get("existing")
	if !pyjson.Truthy(existing) {
		existing = prompts.Data("(none)", "Existing issues and pull requests")
	}
	prompt := x.Render("judge", pyjson.New("repo", x.Cfg.Repo, "count", len(found), "existing", existing,
		"suggestions", prompts.Data(strings.Join(listed, "\n\n"), "The suggestions to judge"),
		"decided", decided))
	if err := herr.Try(func() {
		result := x.Call("judge", prompt, cwd, true, "judge the survey's suggestions", false)
		switch {
		case result.OK:
			judged = verdicts.Judgements(result.Text, len(found))
			if len(judged) == 0 {
				x.Step("judge", fmt.Sprintf("the judge's answer held no verdicts (try %d)", attempt), pyjson.New())
			}
		case result.TimedOut:
			x.Step("judge", fmt.Sprintf("the judge's call timed out (try %d)", attempt), pyjson.New())
		default:
			x.Step("judge", fmt.Sprintf("the judge's call failed (try %d): %s", attempt,
				py.SliceTo(py.OrStr(result.Error, "no answer"), 200)), pyjson.New())
		}
	}); err != nil {
		x.Step("judge", fmt.Sprintf("the judge did not answer (try %d): %s", attempt,
			py.SliceTo(err.Error(), 200)), pyjson.New())
		return map[int]*verdicts.Judgement{}
	}
	return judged
}

// SurveyArea is the survey prompt's `$area` (#4026): the one part of the repository the survey
// keeps to while others survey the rest, or the whole of it for a survey with no area (a plan
// from before #4026, or no `suggestions.areas`).
func SurveyArea(planned *pyjson.Object) string {
	id, focus := planned.Str("area"), planned.Str("area_focus")
	if id == "" || focus == "" {
		return "The whole repository."
	}
	return fmt.Sprintf("**`%s`**: %s.\n\nPropose only improvements inside this area: read the rest "+
		"of the repository as far as it needs, but suggest nothing whose work lies mostly outside it. "+
		"Other surveys, each on an area of its own, cover the rest, so a suggestion there would only "+
		"repeat theirs.", id, focus)
}

// ClassPriorities is every suggestion class and the priority it opens with, in pickup order
// (`PRIORITY_TIERS`: a class with no priority comes before one with `low`), for `suggest.md`.
func ClassPriorities(priorities map[string]string) string {
	tier := func(k string) int {
		label := "priority:None" // f"priority:{priorities.get(k)}" of a class with none
		if v, ok := priorities[k]; ok {
			label = "priority:" + v
		}
		return priorityTier(label)
	}
	ordered := append([]string{}, config.SuggestionClasses...)
	sort.SliceStable(ordered, func(i, j int) bool { return tier(ordered[i]) < tier(ordered[j]) })
	lines := make([]string, len(ordered))
	for i, k := range ordered {
		if v, ok := priorities[k]; ok {
			lines[i] = "- `" + k + "`: `priority:" + v + "`"
		} else {
			lines[i] = "- `" + k + "`: no priority (the owner's call)"
		}
	}
	return strings.Join(lines, "\n")
}

// priorityTier is `PRIORITY_TIERS.get(label, NO_PRIORITY)`.
func priorityTier(label string) int {
	if n, ok := queue.PriorityTiers[label]; ok {
		return n
	}
	return queue.NoPriority
}

// finding is work._finding: a Finding, or one from its dict (a missing field defaulted).
func finding(raw any) *verdicts.Finding {
	if f, ok := raw.(*verdicts.Finding); ok {
		return f
	}
	o := pyjson.Obj(raw)
	return &verdicts.Finding{
		Severity: py.Str(o.GetOr("severity", "blocking")),
		Where:    py.Str(o.GetOr("where", "?")),
		Claim:    py.Str(o.GetOr("claim", "")),
		Evidence: py.Str(o.GetOr("evidence", "")),
	}
}

// CheckTemplates is problems with the prompt files: a missing template or a stray `$`.
func CheckTemplates() []string {
	problems := []string{}
	for _, name := range prompts.Names {
		var template *prompts.Template
		if err := herr.Try(func() {
			var perr error
			template, perr = prompts.Load(name, promptsDir())
			herr.Must(perr)
		}); err != nil {
			if errors.Is(err, fs.ErrNotExist) {
				problems = append(problems, "prompts/"+name+".md is missing")
				continue
			}
			herr.Raise(err)
		}
		values := map[string]string{}
		keys, err := prompts.Placeholders(name, promptsDir())
		herr.Must(err)
		for key := range keys {
			values[key] = "x"
		}
		if _, err := pyTemplate(template.Template).substitute(values); err != nil {
			problems = append(problems, "prompts/"+name+".md: "+err.Error())
		}
	}
	return problems
}
