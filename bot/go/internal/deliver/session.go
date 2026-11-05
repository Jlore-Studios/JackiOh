package deliver

import (
	"fmt"
	"path/filepath"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/sessions"
	statemod "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/vault"
)

// goesOn is whether the item's own next run could take a session of this run up (#2748): a
// revision's pull request stays open for the next revision, as it does for an own review run's
// (2026-11-02), and an unfinished build or plan is built or planned again. A build that opened its
// pull request (but for one that handed its lane to its own review run, whose session goes on on
// the pull request, `OwnReviewPR`), split or landed as a part, a reconcile that sent parts back
// (its next run starts afresh once they land), and a finished plan, hand the work on to another
// item or another kind of call.
func goesOn(action, status string) bool {
	switch action {
	case "revise", "review":
		return true
	case "plan":
		return status != "planned"
	}
	switch status {
	case "approved", "built", "split", "part", "planned", "sent_back":
		return false
	}
	return true
}

// keepSession keeps the Claude session the model job sealed (`work.Worker.keepSession`) for the
// item's next run on this subscription. The session stays where the model job put it, in this
// run's `work` artifact: this job checks its shape (it cannot open it) and records only a pointer
// to it as the item's `session` (the run and artifact that hold it, and what it is), so nothing
// session-sized is ever committed to the state branch. A session another subscription kept goes
// once a run of the item here delivers (the work changed hands); a final audit takes no work over
// and changes nothing. An own review run (#2748, 2026-11-02) is a builder's run on the subscription
// that built the change: its session (its last fix pass's) is kept on the pull request for that
// item's next run there, a CI fix or a revision, as a revision's is; and a build that handed its
// lane to its own review run keeps its own on the pull request (`OwnReviewPR`), which the item's
// runs carry on from then. A closed item keeps none. What it did goes into the result's
// `session`, for the journal.
func (x *Deliverer) keepSession(number int) {
	action := py.OrStr(x.Plan.Get("action"), "")
	if action == "review" && !x.ownReviewDelivery() {
		return
	}
	if action != "plan" && action != "build" && action != "revise" && action != "review" {
		return
	}
	if pr := x.OwnReviewPR; pr != 0 && pr != number {
		// The work goes on on its pull request: the issue's own pointer says where it no longer is.
		if x.Record(number).Obj("session").Len() > 0 {
			x.dropSession(number, fmt.Sprintf("#%d's work goes on on #%d", number, pr))
		}
		number = pr
	}
	if queue.ArchivedOf(x.Record(number)) != nil {
		// Handed to a person (#3009): its archived work keeps no session for a next run.
		if x.Record(number).Obj("session").Len() > 0 {
			x.dropSession(number, fmt.Sprintf("#%d is handed to a person", number))
		}
		return
	}
	session := x.Result.Obj("session")
	text := ""
	if path := filepath.Join(x.OutDir, sessions.OutFile); isFile(path) {
		if read, err := py.ReadText(path); err == nil {
			text = strings.TrimSpace(read)
		}
	}
	record := x.Record(number).Obj("session")
	brought := text != "" || session.Has("not_kept")
	if record.Len() == 0 && !brought {
		return
	}
	said := func(line string) {
		x.log(line)
		if session != nil {
			session.Set("delivered", line)
		}
	}
	if exc := herr.Try(func() {
		if thread := x.GH.GetIssue(number); thread.Get("state") != "open" {
			for _, note := range sessions.DropItem(x.Ctx, number, fmt.Sprintf("#%d is closed", number)) {
				x.log(note)
			}
			said("kept no session: the item is closed")
			return
		}
		// A run here that did no model work (it never started) took nothing over.
		calls, _ := py.Int(py.Or(x.Result.Get("model_calls"), 0))
		if record.Len() > 0 && record.Str("provider") != x.Provider.ID {
			if calls == 0 {
				return
			}
			x.dropSession(number, fmt.Sprintf("#%d's work moved from `%s` to `%s`", number,
				record.Str("provider"), x.Provider.ID))
			record = nil
		}
		status := py.Str(x.Result.Get("status"))
		if !goesOn(action, status) && x.OwnReviewPR == 0 {
			if record.Len() > 0 {
				x.dropSession(number, fmt.Sprintf("this %s ended %s", action, status))
			}
			if brought {
				said(fmt.Sprintf("kept no session: this %s ended %s, so no run of #%d goes on from it",
					action, status, number))
			}
			return
		}
		kept := session.Obj("kept")
		problem := x.sessionProblem(text, kept)
		if problem == "" {
			artifact, size := x.sessionArtifact()
			entry := pyjson.New("id", kept.Str("id"), "provider", x.Provider.ID,
				"model", kept.Str("model"), "kind", kept.Str("kind"), "action", action,
				"cwd", kept.Str("cwd"), "project", kept.Str("project"), "head", kept.Str("head"),
				"bytes", len(text), "at", clock.ISO(x.Ctx.Now()),
				// Where it is: this run's `work` artifact. Without its id the next run looks it up
				// among the run's artifacts by name.
				"run", x.Cfg.RunID, "artifact", artifact, "artifact_name", sessions.Artifact,
				"artifact_bytes", size,
				"ended", py.SliceTo(redact.Redact(status+": "+py.OrStr(x.Result.Get("reason"), ""), nil), 300))
			x.Ctx.Store.Update(func(s *pyjson.Object) any {
				statemod.Item(s, number).Set("session", entry)
				return nil
			}, fmt.Sprintf("session #%d", number))
			said(fmt.Sprintf("kept session `%s` of #%d on %s (%s bytes) in run %s's `%s` artifact, for "+
				"its next run there", kept.Str("id"), number, x.Provider.ID, py.Thousands(len(text)),
				x.Cfg.RunID, sessions.Artifact))
			return
		}
		if brought {
			said("kept no session: " + problem)
		}
		// This run's session is newer than the one kept, which no longer says where the work is.
		if record.Len() > 0 && brought {
			x.dropSession(number, "this run's own session could not be kept")
		}
	}); exc != nil {
		x.log("could not settle the kept session: " + py.SliceTo(redact.Redact(herr.Str(exc), nil), 300))
	}
}

// sessionProblem is why `text` and its `kept` record from the model job cannot be stored, or "".
func (x *Deliverer) sessionProblem(text string, kept *pyjson.Object) string {
	switch {
	case text == "":
		return py.OrStr(x.Result.Obj("session").Get("not_kept"), "the model job sealed none")
	case !py.IsDecimal(x.Cfg.RunID):
		return "this delivery names no run whose artifact holds it"
	case !vault.LooksSealedBlob(text, sessions.MaxBytes):
		return "the model job's session is not sealed, or over the cap"
	case !sessions.ValidID(kept.Str("id")) || !sessions.ValidProject(kept.Str("project")) ||
		!sessions.ValidHead(kept.Str("head")) || !sessions.ValidCwd(kept.Str("cwd")):
		return "the model job's record of its session is not one this harness writes"
	case !py.In(kept.Get("kind"), "build", "plan"):
		return "the model job's session is of no kind a run resumes"
	case x.Cfg.Pool.Seat(x.Provider.ID, kept.Str("model")) == nil:
		return fmt.Sprintf("`%s` runs no model `%s`", x.Provider.ID, kept.Str("model"))
	}
	return ""
}

// sessionArtifact is this run's `work` artifact, which holds the session: its id and size, or
// nil for each when the listing cannot say (the next run then looks it up by name).
func (x *Deliverer) sessionArtifact() (any, any) {
	var found *pyjson.Object
	herr.Catch[*herr.GitHubError](func() { found = sessions.WorkArtifact(x.Ctx.Act().ListArtifacts(x.Cfg.RunID)) })
	if found == nil {
		return nil, nil
	}
	return found.Get("id"), found.Get("size_in_bytes")
}

// dropSession clears the item's kept session: its pointer (the artifact expires by itself).
func (x *Deliverer) dropSession(number int, why string) {
	for _, note := range sessions.DropItem(x.Ctx, number, why) {
		x.log(note)
	}
}
