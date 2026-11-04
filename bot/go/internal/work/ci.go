package work

// An own review run reads its pull request's CI (#2748, the owner's decision of 2026-11-02).
//
// Its build pushed the head and opened the pull request (a draft, which CI runs on), so CI runs
// while the review does. The model job holds only its own read-only token (`contents: read`,
// `actions: read`), which reads the CI workflow's runs and jobs for a commit: a required check that
// is red on the head the run reviews is a blocking finding its builder fixes in the same run, with
// the failed jobs' log tails. A check still running, or one the job cannot read, blocks nothing
// here: once the run approves and auto-merge is on, CI's own verdict is the merge's gate, and a red
// check then comes back to the merge owner's lane (`events.fixCI`).

import (
	"fmt"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/prompts"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
)

// CheckFixRounds is how many fix passes a build or revision that hands its lane to its own review
// run spends on the checks it turned red, before it hands the change on as it stands: its own
// review run treats a red check as a finding to fix.
var CheckFixRounds = 2

// CIRed is the conclusions of a CI job that make a required check red (`config.CIRed`).
var CIRed = config.CIRed

// CILogJobs is how many red jobs' log tails a fix pass gets, and CILogChars how much of each.
var (
	CILogJobs  = 4
	CILogChars = 6000
)

// requiredJob is whether a CI job named `name` counts as one of the required checks
// (`config.RequiredJob`): the check's own job (`rust`), one of the jobs it gathers
// (`rust (test engine)`), or a job a check needs that carries no check's name (`config.CINeeded`).
// A run the bot stopped early (#2948, `events.StopRedCI`) ended cancelled, its summary jobs
// skipped: its failed pieces are what is red in it, and they read as red here, never as a run
// still going or a cancellation that says nothing.
func requiredJob(name string, required []string) bool {
	return config.RequiredJob(name, required)
}

// ciRed is the required checks red in CI on `sha` (the CI workflow's newest run for it), as
// blocking findings, and their failed jobs' log tails, fenced as data, for the fix pass. None when
// the run cannot read CI (no client, or GitHub refused), when CI has not run on `sha`, or when every
// required check is green or still running.
func (x *Worker) ciRed(sha string) ([]*verdicts.Finding, string) {
	x.ciRead = ""
	if x.GH == nil || sha == "" {
		x.ciRead = "this run has no client to read CI with"
		return nil, ""
	}
	var findings []*verdicts.Finding
	var logs []string
	if err := herr.Try(func() {
		var latest *pyjson.Object
		for _, run := range x.GH.RunsForSHA(sha, 30) {
			if !pyjson.Equal(run.Get("name"), x.Cfg.CIWorkflow) {
				continue
			}
			if latest == nil || newerRun(run, latest) {
				latest = run
			}
		}
		if latest == nil {
			x.ciRead = "CI has not run on it yet"
			return
		}
		url := py.OrStr(latest.Get("html_url"), "CI run "+py.Str(latest.Get("id")))
		var red []*pyjson.Object
		for _, job := range x.GH.ListJobs(latest.Get("id"), false) {
			if requiredJob(py.Str(job.Get("name")), x.Cfg.RequiredChecks) && py.In(job.Get("conclusion"), CIRed...) {
				red = append(red, job)
			}
		}
		for _, job := range red {
			name := py.Str(job.Get("name"))
			if gathersRed(name, red) {
				continue // the gathering job says only that one it gathers failed, which says why
			}
			var steps []string
			for _, step := range pyjson.Objs(job.GetOr("steps", []any{})) {
				if py.In(step.Get("conclusion"), CIRed...) {
					steps = append(steps, py.Str(step.Get("name")))
				}
			}
			failed := strings.Join(steps, ", ")
			if failed == "" {
				failed = "?"
			}
			conclusion := py.Str(job.Get("conclusion"))
			findings = append(findings, newFinding("blocking", "CI: "+name, fmt.Sprintf(
				"The required check `%s` is red in CI on `%s` (%s; failed steps: %s). Fix what it "+
					"found; no test is skipped or disabled to get green.", name, first12(sha), conclusion,
				failed), url))
			if len(logs) < CILogJobs {
				logs = append(logs, fmt.Sprintf("## Job: %s (%s)\nFailed steps: %s\n\n%s", name, conclusion,
					failed, x.GH.JobLog(job.Get("id"), CILogChars)))
			}
		}
		if len(red) == 0 {
			x.ciRead = "no required check is red in " + url
			if !pyjson.Equal(latest.Get("status"), "completed") {
				x.ciRead += " (still running)"
			}
		} else {
			x.ciRead = fmt.Sprintf("%d required job(s) red in %s", len(findings), url)
		}
		// What the final check reads again before the draft goes ready (#2948, `ciEnd`): a run
		// that was over said its last word, and the jobs red now are judged now.
		x.noteCI(sha, pyjson.Equal(latest.Get("status"), "completed"), findings)
	}); err != nil {
		x.ciRead = "CI could not be read: " + py.SliceTo(redact.Redact(herr.Str(err), nil), 200)
		return nil, ""
	}
	if len(logs) == 0 {
		return findings, ""
	}
	return findings, prompts.Data(redact.Redact(strings.Join(logs, "\n\n"), nil), "CI on "+first12(sha))
}

// ciPassedLocally is a CI fix's first pass that ran the failing check locally, found it passing and
// changed nothing (#2948: no flake re-run before the fix; the fix run judges, `plan.CILocalNote`).
// While its head has re-runs left (the plan's `ci_reruns_left`, from `ci_reruns`), the run ends
// there, with no commit (a merge of `main` the run made is dropped), and no checks or review, which
// would only judge the head CI ran on: deliver runs CI again on it, and that costs none of its CI
// fixes. With none left, a pass is no answer, and the run goes on as any CI fix.
func (x *Worker) ciPassedLocally(built *verdicts.BuildReport, entry *pyjson.Object) bool {
	if x.planItem("action") != "revise" || !pyjson.Equal(x.Plan.Get("source"), "ci") ||
		py.MustInt(py.Or(x.Plan.Get("ci_reruns_left"), 0)) <= 0 || built.Status != "done" ||
		built.CILocal != "passed" || entry.Obj("builder").Truthy("changed") || x.Wt.MergeInProgress() ||
		len(x.Wt.Dirty()) > 0 {
		return false
	}
	x.resetTo(x.StartSHA)
	x.Step("ci", "the failing CI check, run locally by the builder", pyjson.New("outcome",
		"passed, and nothing needed changing: CI runs again on `"+first12(x.StartSHA)+"`, with no commit"))
	x.Result.Set("status", "built")
	x.Result.Set("reason", "the failing CI check passed locally and nothing needed changing, so CI runs "+
		"again on the same head, with no commit")
	x.Result.Set("ci_local", pyjson.New("passed", true, "head", x.StartSHA))
	return true
}

// noteCI keeps what a CI reading on `sha` judged: whether CI was over (`complete`), and the red
// jobs, for the final check's reading of the same head (`ciEnd`).
func (x *Worker) noteCI(sha string, complete bool, red []*verdicts.Finding) {
	if x.final.ciComplete == nil {
		x.final.ciComplete, x.final.ciSeen = map[string]bool{}, map[string]bool{}
	}
	x.final.ciComplete[sha] = x.final.ciComplete[sha] || complete
	for _, f := range red {
		x.final.ciSeen[strings.TrimPrefix(f.Where, "CI: ")] = true
	}
}

// gathersRed is: the job `name` gathers others (`rust` gathers `rust (…)`), and one it gathers is
// among `red`.
func gathersRed(name string, red []*pyjson.Object) bool {
	for _, job := range red {
		if strings.HasPrefix(py.Str(job.Get("name")), name+" (") {
			return true
		}
	}
	return false
}

// newerRun is: `a` is a newer CI run than `b`: created later, or a later attempt of it.
func newerRun(a, b *pyjson.Object) bool {
	at, bt := py.OrStr(a.Get("created_at"), ""), py.OrStr(b.Get("created_at"), "")
	if at != bt {
		return at > bt
	}
	return py.MustInt(py.Or(a.Get("run_attempt"), 1)) > py.MustInt(py.Or(b.Get("run_attempt"), 1))
}

// ciRecord is what a round's record keeps of the CI it read: what it found, and the red checks.
func (x *Worker) ciRecord(red []*verdicts.Finding) *pyjson.Object {
	names := []any{}
	for _, f := range red {
		names = append(names, strings.TrimPrefix(f.Where, "CI: "))
	}
	return pyjson.New("read", x.ciRead, "red", names)
}

// ownReviewNote is what an own review run's reviewer is told of the run it is in: who built the
// change, that this run's builder fixes what it finds, and what CI says on the head.
func (x *Worker) ownReviewNote(cycle int, red []*verdicts.Finding) string {
	by := py.OrStr(x.Plan.Get("built_by"), x.Provider.ID)
	note := fmt.Sprintf("This is the change's own review run (#2748): `%s`, this subscription, built "+
		"it and handed it on to you, its lane's reviewer, in a fresh session. If you find blocking "+
		"problems, this run's builder fixes them and a fresh session of yours reviews the fix, up to "+
		"%d rounds in all; once you approve it, it merges when CI passes, with no other review.",
		by, x.CyclesCap())
	if cycle > 1 {
		note += " The findings above are the last round's, which the builder has answered since."
	}
	switch {
	case len(red) > 0:
		note += "\n\nCI on this head: " + x.ciRead + ". Each red required check is a blocking " +
			"finding of its own, whatever you find."
	case x.ciRead != "":
		note += "\n\nCI on this head: " + x.ciRead + "."
	}
	return note
}
