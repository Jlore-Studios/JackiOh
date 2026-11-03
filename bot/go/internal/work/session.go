package work

// A Claude run's own session, kept and resumed (#2748, section 9; `internal/sessions`).
//
// After each builder or planner call the run notes the session the CLI named; whenever it writes
// `result.json` (its end and every checkpoint) it seals that session's file with the
// subscription's vault key into the output directory, which the job's `work` artifact carries;
// deliver records a pointer to it. The item's next run on the same subscription and model family
// gets the pointer in its plan (`session`): its first builder pass, or its planner, takes the
// session out of that artifact, restores it to where Claude Code files it and resumes it with a
// short continuation prompt instead of the whole first prompt and the handoff. Anything that
// fails on the way (an expired artifact, a key that does not open it, the CLI refusing the id,
// the resumed call failing before it did anything) leaves the call to a fresh
// session with the first prompt and the handoff, as before; `result.json`'s `session` and the
// journal say which happened, and the resumed call's minutes and tokens.

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/prompts"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/sessions"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
)

// Sessions is what a run needs to keep its own session and resume a kept one.
type Sessions struct {
	// Secret is the subscription's own secret, from which the vault key comes.
	Secret string
	// ConfigDir is the CLI's CLAUDE_CONFIG_DIR, under which Claude Code files every session.
	ConfigDir string
	// Fetch is the sealed session a kept record points at, out of its run's `work` artifact
	// (`sessions.Fetch`), or why it could not be had.
	Fetch func(record *pyjson.Object) (string, error)
}

// NewSessions is a run's Sessions, or nil when it keeps and resumes none: `resume_sessions` is
// off, or the subscription's CLI is not Claude, or its login gave no secret or config directory.
//
// Only Claude resumes for now. Codex (`codex exec resume <id>`), Muse and agy would each need
// their session captured here, and their resume flag in their backend's Argv (`runner`).
func NewSessions(cfg *config.Config, provider *providers.Provider, secret string, configDir string,
	fetch func(record *pyjson.Object) (string, error)) *Sessions {
	if !cfg.ResumeSessions || provider == nil || provider.CLI != "claude" || secret == "" ||
		configDir == "" || fetch == nil {
		return nil
	}
	return &Sessions{Secret: secret, ConfigDir: configDir, Fetch: fetch}
}

// sessionState is what the run knows of sessions as it goes.
type sessionState struct {
	// seen is the last builder's or planner's session, the one the run keeps.
	seen *seenSession
	// sealed is what the output's sealed session was made from, so an unchanged one is not
	// sealed again at every checkpoint.
	sealed string
	// decided holds the kinds of call that already decided whether to resume: once a run each.
	decided map[string]bool
	// settled: a call of this run resumed the kept session or tried to, which no later call's
	// decision (a call of the other kind, say) then overwrites.
	settled bool
	// conflicts are the files the merge of `main` left conflicted (`FirstPrompt`).
	conflicts []string
}

type seenSession struct{ id, model, kind, role, cwd string }

// ResumeHead is the start of what a resumed session is told.
var ResumeHead = "## Resuming your own session\n\nThe harness resumed this, your own session from " +
	"an earlier run on `{provider}` ({when}), rather than start a fresh one: everything you read, " +
	"ran and wrote then is above. That run ended: {ended}."

// ResumeBuild is what a resumed builder is told of its worktree and its task.
var ResumeBuild = "\n\nYour worktree is `{cwd}`, checked out afresh at `{head}`: your commits are " +
	"in it, nothing you left uncommitted is, and `" + NotesFile + "` holds the notes you kept.{moved} " +
	"What has changed since is below. Check it against the diff, re-run the checks you need " +
	"before you trust a result from before, bring the `## State` part of your notes up to date, " +
	"and go on from where you stopped. Your final message is the report your first prompt in " +
	"this session asked for, its first line the `<!-- bot: ... -->` comment."

// ResumePlan is what a resumed planner is told.
var ResumePlan = "\n\nYou were planning this and were cut off before the plan was finished. Your " +
	"draft is `" + PlanDraftFile + "` at the top of your worktree (`{cwd}`), as you left it.{moved} " +
	"Check the branch below if it moved, finish the plan, keep the draft up to date as you go, and " +
	"end, as before, with the whole plan as your final message, in the shape your first prompt in " +
	"this session asked for."

// ResumeMoved is what a resumed session is told when its worktree is not where it was.
var ResumeMoved = " The earlier run worked in `{old}`: what your session says is there is now " +
	"under `{cwd}`."

// sessionKind is what a call's session is, for keeping and resuming: a builder's (its first
// pass, a fix, a revision), a planner's that keeps a draft, or neither ("": reviews, self checks,
// splits and surveys run in a fresh session by design).
func sessionKind(reader, drafting bool) string {
	switch {
	case drafting:
		return "plan"
	case !reader:
		return "build"
	}
	return ""
}

// kindName is who a kind of session is.
func kindName(kind string) string {
	if kind == "plan" {
		return "planner"
	}
	return "builder"
}

// resumable is whether a call may go on in a kept session: a builder's first pass (a fix goes on
// from a review this run made, which a kept session never saw) and a planner's call.
func resumable(role, kind string) bool {
	return kind == "plan" || (kind == "build" && (role == "build" || role == "revise"))
}

// resumes is whether this run may go on in a kept session at all: not a change's own review run
// (#2903, the owner's decision of 2026-11-03), whose reviews and fixes are new instances of the
// model that built it, never its build's session. A run that continues one cut off part-way (a
// build, a revision or a plan), and a CI fix after the merge queue took a change out, may.
func (x *Worker) resumes() bool {
	return x.Sessions != nil && !x.OwnReviewRun()
}

// runCall is one model call and its tally on `step`, resumed when it may be: when the resume
// fails before the call did anything, the same call runs again in a fresh session, as its own
// step. Returns the result and the step that has it.
func (x *Worker) runCall(request *runner.RunRequest, step *pyjson.Object, reader, drafting bool) (*runner.RunResult, *pyjson.Object) {
	kind := sessionKind(reader, drafting)
	fresh := *request
	why := py.Str(step.Get("why"))
	if x.resumes() && resumable(request.Role, kind) && !x.session.decided[kind] && !x.session.settled {
		if x.session.decided == nil {
			x.session.decided = map[string]bool{}
		}
		x.session.decided[kind] = true
		if id, prompt := x.resumeFrom(kind, request); id != "" {
			request.Resume, request.Prompt = id, prompt
			step.Set("why", why+"; in its own session `"+py.SliceTo(id, 8)+"`, resumed")
			step.Set("resumed", true)
		}
	}
	before := ""
	if request.Resume != "" {
		before = x.worktreeNow()
	}
	result := x.Runner.(modelRunner).Run(request)
	x.tally(step, result)
	if request.Resume != "" {
		failed := x.resumeFailed(result, before)
		x.noteResumedCall(step, result, failed)
		if failed != "" {
			step.Set("outcome", py.SliceTo("could not resume it: "+failed, 300))
			if result.Usage.Len() > 0 {
				x.LastUsage = result.Usage
			}
			x.signalled() // a cancel while it ran stops the run here
			x.Calls++
			fresh.Transcript = filepath.Join(filepath.Dir(request.Transcript),
				fmt.Sprintf("%02d-%s.jsonl", x.Calls, request.Role))
			if x.BuildTranscript == request.Transcript {
				x.BuildTranscript = fresh.Transcript
			}
			step = x.Step("call", why+"; in a fresh session, as its own could not be resumed",
				pyjson.New("role", request.Role, "model", request.Model, "n", x.Calls))
			request = &fresh
			result = x.Runner.(modelRunner).Run(request)
			x.tally(step, result)
		}
	}
	if x.Sessions != nil && kind != "" && sessions.ValidID(result.SessionID) {
		x.session.seen = &seenSession{id: result.SessionID, model: request.Model, kind: kind,
			role: request.Role, cwd: request.Cwd}
	}
	return result, step
}

// sessionRecord is the result's `session`, made on first use.
func (x *Worker) sessionRecord() *pyjson.Object {
	if found, ok := x.Result.Get("session").(*pyjson.Object); ok && found != nil {
		return found
	}
	record := pyjson.New("resumed", false)
	x.Result.Set("session", record)
	return record
}

// resumeFrom restores the session the item's last run here kept, when this call may go on in
// it, and returns its id and the continuation prompt; ("", "") for a fresh session, with why in
// the result's `session`.
func (x *Worker) resumeFrom(kind string, request *runner.RunRequest) (string, string) {
	record := x.sessionRecord()
	stored := x.Plan.Obj("session")
	fresh := func(why string, tried bool) (string, string) {
		record.Set("resumed", false)
		record.Set("fallback", why)
		if tried {
			record.Set("tried", true)
			x.session.settled = true
		}
		return "", ""
	}
	if stored.Len() == 0 {
		return fresh("none was kept for this item", false)
	}
	from := pyjson.New()
	for _, key := range []string{"id", "provider", "model", "kind", "at", "run", "artifact", "bytes"} {
		if stored.Has(key) {
			from.Set(key, stored.Get(key))
		}
	}
	record.Set("from", from)
	id, project, head := stored.Str("id"), stored.Str("project"), stored.Str("head")
	switch {
	case stored.Str("provider") != x.Provider.ID:
		return fresh(fmt.Sprintf("the one kept is `%s`'s, and this run is on `%s`",
			stored.Str("provider"), x.Provider.ID), false)
	case stored.Str("kind") != kind:
		return fresh(fmt.Sprintf("the one kept is a %s's, and this call is a %s's",
			kindName(stored.Str("kind")), kindName(kind)), false)
	case sessions.Family(stored.Str("model")) != sessions.Family(request.Model):
		return fresh(fmt.Sprintf("the one kept ran on `%s`, and this call runs on `%s`",
			stored.Str("model"), request.Model), false)
	case !sessions.ValidID(id) || !sessions.ValidProject(project) || !sessions.ValidHead(head) ||
		!sessions.ValidCwd(stored.Str("cwd")):
		return fresh("its record is not one this harness writes", false)
	case head != "" && !x.holds(head):
		return fresh(fmt.Sprintf("the branch no longer holds `%s`, where it stopped", first12(head)), false)
	}
	// Out of the run's `work` artifact, which may have expired since (a fallback like any other).
	text := ""
	var problem error
	if exc := herr.Try(func() { text, problem = x.Sessions.Fetch(stored) }); exc != nil {
		problem = exc
	}
	if problem != nil {
		return fresh("it could not be had: "+py.SliceTo(redact.Redact(herr.Str(problem), nil), 200), true)
	}
	if strings.TrimSpace(text) == "" {
		return fresh("it could not be had: its run's artifact holds none", true)
	}
	if stored.Str("cwd") != request.Cwd {
		project = sessions.Project(request.Cwd) // where Claude Code looks for it from here
	}
	if err := sessions.Restore(text, x.Sessions.Secret, x.Sessions.ConfigDir, project, id); err != nil {
		return fresh("it could not be opened: "+py.SliceTo(redact.Redact(err.Error(), nil), 200), true)
	}
	record.Set("resumed", true)
	record.Pop("fallback")
	record.Set("tried", true)
	x.session.settled = true
	return id, x.continuation(stored, kind, request)
}

// holds is whether the worktree's history has `head`, where the kept session stopped: a branch
// built again from `main`, or a revision whose pull request moved past its unfinished work, does
// not, and the session would describe work that is gone.
func (x *Worker) holds(head string) bool {
	if x.Wt == nil {
		return true
	}
	code, _ := run(x.Wt, false, "merge-base", "--is-ancestor", head, "HEAD")
	return code == 0
}

// worktreeNow is the worktree's head and what it holds uncommitted (a conflict revision starts
// with the merge's files), for telling whether a call changed anything.
func (x *Worker) worktreeNow() string {
	if x.Wt == nil {
		return ""
	}
	return x.Wt.Head() + "\n" + strings.Join(x.Wt.Dirty(), "\n")
}

// resumeFailed is why a resumed call failed before it did anything ("" when it did not): the
// CLI refused the id, or the resumed session failed at once. A usage limit, a broken login and a
// call that timed out, worked a while or changed the worktree are the call's own outcome.
func (x *Worker) resumeFailed(result *runner.RunResult, before string) string {
	if result.OK || result.TimedOut || result.RateLimited() || result.Infra() ||
		result.Extra.Truthy("usage_stop") {
		return ""
	}
	if result.Turns != nil && *result.Turns > 1 {
		return ""
	}
	if x.worktreeNow() != before {
		return ""
	}
	return py.SliceTo(redact.Redact(or(strings.TrimSpace(result.Error), "it failed and said nothing"), nil), 200)
}

// noteResumedCall puts the resumed call's minutes, tokens and cost in the result's `session`,
// for the usage history to set against fresh sessions (#2749), and says when it fell back.
func (x *Worker) noteResumedCall(step *pyjson.Object, result *runner.RunResult, failed string) {
	record := x.sessionRecord()
	call := pyjson.New("n", step.Get("n"), "role", step.Get("role"), "model", step.Get("model"),
		"ok", result.OK, "minutes", step.Get("minutes"))
	for _, key := range []string{"tokens", "cost_usd"} {
		if step.Has(key) {
			call.Set(key, step.Get(key))
		}
	}
	record.Set("call", call)
	if failed != "" {
		record.Set("resumed", false)
		record.Set("fallback", "the CLI could not resume it: "+failed)
	}
}

// continuation is what a resumed session is told: what it lacks, not what it already read.
func (x *Worker) continuation(stored *pyjson.Object, kind string, request *runner.RunRequest) string {
	when := strings.Replace(py.SliceTo(stored.Str("at"), 16), "T", " ", 1) + " UTC"
	if stored.Str("at") == "" {
		when = "an earlier run"
	}
	ended := py.OrStr(stored.Get("ended"), "")
	if ended == "" {
		ended = py.OrStr(x.Plan.Obj("handoff").Get("reason"), "no reason was recorded")
	}
	text := pyFormat(ResumeHead, map[string]any{"provider": stored.Str("provider"), "when": when,
		"ended": ended})
	moved := ""
	if old := stored.Str("cwd"); old != "" && old != request.Cwd {
		moved = pyFormat(ResumeMoved, map[string]any{"old": old, "cwd": request.Cwd})
	}
	if kind == "plan" {
		return text + pyFormat(ResumePlan, map[string]any{"cwd": request.Cwd, "moved": moved}) +
			"\n\n" + x.branchState()
	}
	head := ""
	if x.Wt != nil {
		head = first12(x.Wt.Head())
	}
	text += pyFormat(ResumeBuild, map[string]any{"cwd": request.Cwd, "head": head, "moved": moved})
	plan := x.Plan
	parts := []string{}
	if plan.Get("action") == "revise" {
		parts = append(parts, "This run revises the pull request because of: **"+
			py.OrStr(plan.Get("source"), "request")+"**. What was asked, as it stands now:\n\n"+
			py.OrStr(plan.Get("feedback"), "(nothing new)"))
	}
	merged := ""
	if conflicts := x.session.conflicts; len(conflicts) > 0 {
		merged = "`main` moved, and merging it into the branch left conflict markers in: " +
			backticked(conflicts) + ". Resolve them, keeping both sides' meaning. Do not commit."
	} else if len(x.Resolved) > 0 {
		merged = "`main` moved, and the harness merged it into the branch and resolved every " +
			"conflict (below). The merge is staged, not committed: the harness commits it after you."
	}
	if merged += x.resolvedNote() + x.CarryBuild; strings.TrimSpace(merged) != "" {
		parts = append(parts, strings.TrimSpace(merged))
	}
	if wip := strings.TrimSpace(x.wipText()); wip != "" {
		parts = append(parts, wip)
	}
	if prior := pyjson.List(plan.Get("previous_findings")); len(prior) > 0 {
		var findings []*verdicts.Finding
		for _, f := range prior {
			findings = append(findings, finding(f))
		}
		parts = append(parts, "The last review's blocking findings:\n\n"+
			x.findingsText(findings, "Blocking findings"))
	}
	if question := strings.TrimSpace(py.OrStr(plan.Get("previous_question"), "")); question != "" {
		parts = append(parts, "A run stopped to ask the question below. The answer, if someone "+
			"gave one, is in the thread as it stands now, below it.\n\n"+
			prompts.Data(question, "The question asked")+"\n\n"+py.OrStr(plan.Get("thread"), ""))
	}
	parts = append(parts, "The branch now:\n\n"+x.branchState())
	return text + "\n\n## What changed since\n\n" + strings.Join(parts, "\n\n")
}

// keepSession seals the run's last builder or planner session into the output directory, where
// the `work` artifact carries it to deliver (`sessions.OutFile`), and says in the result's
// `session` what it kept or why it kept nothing. It never stops the run.
func (x *Worker) keepSession() {
	seen := x.session.seen
	if x.Sessions == nil || seen == nil {
		return
	}
	record := x.sessionRecord()
	out := filepath.Join(x.OutDir, sessions.OutFile)
	notKept := func(why string) {
		record.Pop("kept")
		record.Set("not_kept", why)
		x.session.sealed = ""
		_ = os.Remove(out) // a checkpoint's older one would stand for this session
	}
	if exc := herr.Try(func() {
		project := sessions.Find(x.Sessions.ConfigDir, seen.id)
		if project == "" {
			notKept(fmt.Sprintf("Claude Code left no file for session `%s`", seen.id))
			return
		}
		transcript := sessions.Transcript(x.Sessions.ConfigDir, project, seen.id)
		info, err := os.Stat(transcript)
		herr.Must(err)
		head := ""
		if x.Wt != nil {
			head = x.Wt.Head()
		}
		made := fmt.Sprintf("%s:%d:%d:%s", seen.id, info.Size(), info.ModTime().UnixNano(), head)
		if made == x.session.sealed && isFile(out) {
			return
		}
		sealed, err := sessions.Seal(x.Sessions.ConfigDir, project, seen.id, x.Sessions.Secret)
		if err != nil {
			notKept(py.SliceTo(redact.Redact(err.Error(), nil), 300)) // over `sessions.MaxBytes`, say
			return
		}
		temp := out + ".tmp"
		writeText(temp, sealed+"\n")
		herr.Must(os.Rename(temp, out)) // a kill mid-write leaves the last whole one
		x.session.sealed = made
		record.Pop("not_kept")
		record.Set("kept", pyjson.New("id", seen.id, "model", seen.model, "kind", seen.kind,
			"role", seen.role, "cwd", seen.cwd, "project", project, "head", head,
			"bytes", len(sealed), "at", clock.ISO(x.Now())))
	}); exc != nil {
		notKept("it could not be sealed: " + py.SliceTo(redact.Redact(herr.Str(exc), nil), 300))
	}
}
