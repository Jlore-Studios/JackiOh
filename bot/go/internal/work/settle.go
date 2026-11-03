package work

// Run 2's final check before it goes ready (#2948, section 2; `own_review_run`).
//
// Once its own review run's reviewer approves the draft, and before deliver marks it ready, three
// things are looked at once each, in this order (`finalCheck`):
//
//  1. Its CI again (`ciEnd`). Round one read the draft's CI on the head its build pushed; when that
//     reading was not CI's last word (still running, or not run yet), it is read again now. A
//     required job red now is a finding this run's builder fixes in the same run, which counts
//     toward `max_failures` as a CI fix does (deliver adds it to the record's `ci_fixes`; none is
//     made once those are used up); a job still running blocks nothing: the draft goes ready, and a
//     red one later comes back to this lane (`events.fixCI`).
//  2. `main` (`settleMain`), test-merged with `git merge-tree`. Clean, it is not merged: the merge
//     queue tests `main` with the pull request anyway, and a merge would only start CI again.
//     Conflicted, `main` is merged into it, once: Mergiraf (`resolve.Mergiraf`) and the resolvers
//     settle what they can, a fix pass of this run's builder resolves what is left, the checks run
//     again, and the reviewer reviews only the merge (`final.note`).
//  3. Every other open pull request whose changed files overlap it (`settle.Pulls`): it is
//     test-merged after each has landed. Deliver, which holds the approval times, keeps it waiting
//     its turn (`bot:waiting-turn`) behind one that is ahead (approved earlier, conflict-free) that it
//     conflicts with; when the job cannot read them, deliver test-merges them itself.
//
// What it found goes in the result's `settle`.

import (
	"fmt"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/resolve"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/settle"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
)

// finalState is what the final check has done so far in this run.
type finalState struct {
	// ciDone and mainDone: each step runs once a run.
	ciDone, mainDone bool
	// ciComplete: CI on that head was over when it was read (or a job had failed already), so its
	// verdict was judged then; ciSeen: the red jobs already judged.
	ciComplete map[string]bool
	ciSeen     map[string]bool
	// note is what the reviewer of the merge of `main` is told: review that merge only.
	note string
	// driver: Mergiraf is this worktree's merge driver already.
	driver bool
	// record is the result's `settle`.
	record *pyjson.Object
}

// finalStep is what the final check asks of the review loop's next round: a fix pass of the builder
// for `findings` (and `failures`), or, with `harness`, the merge of `main` the harness settled
// committed with no builder session; then the checks and a fresh review.
type finalStep struct {
	findings []*verdicts.Finding
	failures string
	harness  bool
	what     string
}

// settleRecord is the result's `settle`, made on first use.
func (x *Worker) settleRecord() *pyjson.Object {
	if x.final.record == nil {
		x.final.record = pyjson.New()
		x.Result.Set("settle", x.final.record)
	}
	return x.final.record
}

// finalCheckOf is the final check of an own review run (`handed`) that the night bot's chain runs
// (`own_review_run`), nil for any other run.
func (x *Worker) finalCheckOf(handed bool, approved string) *finalStep {
	if !handed || !x.Cfg.OwnReviewRun || x.Wt == nil {
		return nil
	}
	return x.finalCheck(approved)
}

// finalCheck is the final check once the reviewer approved `approved` (the package's rules above):
// nil when the draft may go to deliver as it is, else what the next round does.
func (x *Worker) finalCheck(approved string) *finalStep {
	if !x.final.ciDone {
		x.final.ciDone = true
		if red, logs := x.ciEnd(); len(red) > 0 {
			return &finalStep{findings: red, failures: logs, what: "CI on the draft, read again, is red"}
		}
	}
	if !x.final.mainDone {
		x.final.mainDone = true
		if step := x.settleMain(approved); step != nil {
			return step
		}
	}
	x.settlePulls()
	return nil
}

// ciEnd reads the draft's CI again, on the head its build pushed (`StartSHA`), when round one's
// reading was not CI's last word: the required jobs red now that round one had not judged, and their
// log tails, unless this item's CI fixes are used up (`max_failures`).
func (x *Worker) ciEnd() ([]*verdicts.Finding, string) {
	sha := x.StartSHA
	record := x.settleRecord()
	if x.final.ciComplete[sha] {
		return nil, ""
	}
	seen := map[string]bool{}
	for name := range x.final.ciSeen {
		seen[name] = true // what the earlier readings judged; this one notes its own
	}
	red, logs := x.ciRed(sha)
	fresh := []*verdicts.Finding{}
	for _, f := range red {
		if !seen[strings.TrimPrefix(f.Where, "CI: ")] {
			fresh = append(fresh, f)
		}
	}
	reading := x.ciRecord(fresh)
	record.Set("ci", reading)
	if len(fresh) == 0 {
		return nil, ""
	}
	if fixes := py.MustInt(py.Or(x.Plan.Get("ci_fixes"), 0)); fixes >= x.Cfg.MaxFailures {
		reading.Set("fixed", false).Set("why", fmt.Sprintf("its %d CI fixes are used up", fixes))
		return nil, ""
	}
	reading.Set("fixed", true)
	x.Step("final check", "CI on the draft, read again before it goes ready", pyjson.New(
		"outcome", fmt.Sprintf("red: %s; this run fixes it", strings.Join(pyjson.Strings(reading.Get("red")), ", "))))
	return fresh, logs
}

// settleMain test-merges the approved head with `main`: nil when it merges cleanly (it is not merged)
// or git cannot tell; else `main` is merged into it, Mergiraf and the resolvers settle what they
// can, and the step is the next round: the harness's commit of the merge, or a fix pass for the
// conflicts left.
func (x *Worker) settleMain(approved string) *finalStep {
	record := x.settleRecord()
	fetchRef(x.Repo, x.Cfg.DefaultBranch, false)
	main := x.Repo.Rev(x.BaseRef)
	entry := pyjson.New("sha", main)
	record.Set("main", entry)
	if main == "" {
		entry.Set("state", "unread")
		return nil
	}
	test := settle.TestMerge(x.Wt, "HEAD", main)
	switch {
	case test.Problem != "":
		entry.Set("state", "unread").Set("problem", test.Problem)
		x.Step("final check", "test-merge with `main`", pyjson.New("outcome", test.Problem))
		return nil
	case test.Clean():
		entry.Set("state", "clean")
		x.Step("final check", "test-merge with `main`", pyjson.New("outcome",
			"clean at `"+first12(main)+"`: not merged, since the merge queue tests it with `main`"))
		return nil
	}
	entry.Set("state", "merged").Set("conflicts", pyjson.ListOf(test.Conflicts))
	takes := x.mergeDriver(main)
	conflicts := x.Wt.Merge(main, x.Who)
	x.mergirafSettled(takes, conflicts)
	x.BaseSHA = main
	x.BaseWt = nil // the checks red on `main` are `main`'s new self's now
	x.BaseGateCache = pyjson.New()
	outcome := "merged; git settled every conflict"
	if len(conflicts) > 0 {
		outcome = fmt.Sprintf("conflicts in %d file(s): %s", len(conflicts), strings.Join(upTo(conflicts, 5), ", "))
	}
	x.Step("merge", "the final check: the approved head conflicts with `main`", pyjson.New("outcome", outcome))
	if len(conflicts) > 0 {
		commands := false
		for _, r := range x.Cfg.Resolvers {
			commands = commands || (r.Run != "" && contains(conflicts, r.Path))
		}
		if commands && x.InstalledAt == "" {
			x.install() // a command resolver builds the merged tree
		}
		conflicts = x.Resolve(conflicts, x.InstalledAt != "")
	}
	entry.Set("left", pyjson.ListOf(conflicts))
	x.final.note = x.mergeNote(approved, main, test.Conflicts, conflicts)
	if len(conflicts) == 0 {
		return &finalStep{harness: true, what: "`main` conflicted, and the harness settled the merge"}
	}
	return &finalStep{findings: []*verdicts.Finding{newFinding("blocking", strings.Join(upTo(conflicts, 8), ", "),
		"The final check merged `main` (`"+first12(main)+"`) into the approved head, and it left conflict "+
			"markers in "+backticked(conflicts)+": resolve every one, keeping both `main`'s change and the "+
			"branch's with their meaning, and change nothing else. The merge is staged; the harness "+
			"commits it after you.", "git merge")},
		what: "`main` conflicted; the builder resolves what the harness left"}
}

// mergeNote is what the reviewer of the final check's merge of `main` is told: review the merge
// only.
func (x *Worker) mergeNote(approved, main string, conflicted, left []string) string {
	settled := "the harness settled every conflict (Mergiraf, git, or a resolver of its config)"
	if len(left) > 0 {
		settled = "the harness settled what it could, and this run's builder resolved " + backticked(left)
	}
	return fmt.Sprintf("The final check before this draft goes ready (#2948): you approved `%s`, and its "+
		"test-merge with `main` (`%s`) conflicted in %s, so `main` was merged into it once, and %s. "+
		"Review only that merge: `git log --merges -1` finds it, and `git show --remerge-diff <it>` "+
		"shows how each conflict was resolved. Both `main`'s change and the branch's must survive with "+
		"their meaning, nothing `main` brought may be dropped or reverted, and the merged whole must "+
		"still be correct. What you approved before it stands.", first12(approved), first12(main),
		backticked(conflicted), settled)
}

// settledPass is the round after the final check's merge of `main` when the harness settled every
// conflict: its merge committed (git committed it already when it merged cleanly), with no builder
// session. The report stays the one the reviewer approved.
func (x *Worker) settledPass(cycle int, report *verdicts.BuildReport) (*verdicts.BuildReport, *pyjson.Object) {
	before := x.Wt.Head()
	if x.Wt.MergeInProgress() {
		x.commitMerge(config.CommitPrefix + ": merge " + x.Cfg.DefaultBranch + " for #" +
			py.Str(x.planItem("number")) + ", the final check before it goes ready")
	}
	changed := x.Wt.Head() != before
	outcome := "git committed it as it merged"
	if changed {
		outcome = "committed `" + first12(x.Wt.Head()) + "`"
	}
	x.Step("commit", "the final check's merge of `main`, as the harness settled it; no builder session",
		pyjson.New("outcome", outcome))
	entry := pyjson.New(
		"n", cycle,
		"builder", pyjson.New("role", "merge", "ok", true, "turns", 0, "minutes", 0.0,
			"status", "done", "error", "", "timed_out", false, "model", "the harness", "tier", "",
			"changed", changed),
	)
	x.checkpoint()
	return report, entry
}

// settlePulls test-merges the head after each open pull request whose changed files overlap it
// (`settle.Pulls`), for deliver to keep it waiting its turn behind one ahead of it.
func (x *Worker) settlePulls() {
	record := x.settleRecord()
	fetchRef(x.Repo, x.Cfg.DefaultBranch, false)
	pulls, why := settle.Pulls(x.Repo, x.GH, py.MustInt(x.planItem("number")), x.Wt.Head(),
		x.Cfg.DefaultBranch, x.Who)
	dicts := []any{}
	var conflicted []string
	for _, p := range pulls {
		dicts = append(dicts, p.ToDict())
		if p.Conflicted() {
			conflicted = append(conflicted, fmt.Sprintf("#%d", p.Number))
		}
	}
	record.Set("head", x.Wt.Head()).Set("pulls", dicts).Set("pulls_read", why)
	outcome := why
	if why == "" {
		outcome = fmt.Sprintf("%d open pull request(s) overlap it", len(pulls))
		if len(conflicted) > 0 {
			outcome += "; it conflicts once " + strings.Join(conflicted, ", ") + " land(s)"
		}
	}
	x.Step("final check", "test-merges after the open pull requests that overlap it", pyjson.New("outcome", outcome))
}

// mergeDriver makes Mergiraf this worktree's merge driver before a merge of `ref` whose test-merge
// conflicts in a file the root `.gitattributes` gives it (#2948, section 3; `resolve.Mergiraf`). It
// keeps a model call's time for the rest of the run. Whatever it did is a step of the run; on any
// failure the merge is git's own. Returns the files git alone would leave conflicted that the driver
// merges, none when it is not in place.
func (x *Worker) mergeDriver(ref string) []string {
	if x.Wt == nil {
		return nil
	}
	takes := resolve.Takes(x.Wt, settle.TestMerge(x.Wt, "HEAD", ref).Conflicts)
	if len(takes) == 0 {
		return nil
	}
	if !x.final.driver {
		seconds := int(x.SecondsLeft()) - x.Cfg.MinMinutesForACall*60
		how, ok := resolve.Mergiraf(x.Repo, x.Wt, x.Env, x.WorkDir, seconds)
		x.final.driver = ok
		x.Result.Set("mergiraf", pyjson.New("used", ok, "how", how, "paths", pyjson.ListOf(takes)))
		x.Step("mergiraf", "a syntax-aware merge for "+backticked(upTo(takes, 5)), pyjson.New("outcome", how))
	}
	if !x.final.driver {
		return nil
	}
	return takes
}

// mergirafSettled records the files Mergiraf merged that git alone would have left conflicted
// (`takes`, less those still conflicted, `left`) as the harness's resolutions (`Resolved`): the
// builder and the reviewer are told so, and a conflict revision with nothing else left spends no
// builder session (`MergeOnly`).
func (x *Worker) mergirafSettled(takes, left []string) {
	for _, p := range takes {
		if contains(left, p) {
			continue
		}
		r := &resolve.Resolution{Path: p, How: "merged by Mergiraf " + resolve.MergirafVersion +
			", syntax-aware, during the merge", OK: true}
		x.Resolved = append(x.Resolved, r)
		appendTo(x.Result, "resolved", r.ToDict())
	}
}

// commitMerge commits the merge in progress as the harness settled it. When the merged tree is the
// branch's own (`main` brought nothing else), the commit still records `main`, or the pull request
// would stay in conflict.
func (x *Worker) commitMerge(message string) {
	if !x.commit(message) && x.Wt.MergeInProgress() {
		x.Wt.Run(true, 600, map[string]string{
			"GIT_AUTHOR_NAME":     x.Who.Name,
			"GIT_AUTHOR_EMAIL":    x.Who.Email,
			"GIT_COMMITTER_NAME":  x.Who.Name,
			"GIT_COMMITTER_EMAIL": x.Who.Email,
		}, "commit", "--quiet", "--no-verify", "--allow-empty", "-m", message)
	}
}
