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
		"priorities", ClassPriorities(x.Cfg.SuggestionPriorities)))
	result := x.Call("suggest", prompt, wt.Cwd, true,
		"propose improvements while a lane has nothing to start", false)
	found := verdicts.Suggestions(result.Text, n)
	suggestions := []any{}
	for _, s := range found {
		suggestions = append(suggestions, pyjson.New("title", s.Title, "body", s.Body, "class", s.Kind))
	}
	x.Result.Set("status", "suggested")
	x.Result.Set("reason", fmt.Sprintf("%d suggestion(s)", len(found)))
	x.Result.Set("suggestions", suggestions)
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
