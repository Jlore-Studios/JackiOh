package deliver

import (
	"fmt"
	"path/filepath"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/sessions"
	statemod "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/vault"
)

// goesOn is whether the item's own next run could take a session of this run up (#2748): a
// revision's pull request stays open for the next revision, and an unfinished build or plan is
// built or planned again. A build that opened its pull request, split or landed as a part, and
// a finished plan, hand the work on to another item or another kind of call.
func goesOn(action, status string) bool {
	switch action {
	case "revise":
		return true
	case "plan":
		return status != "planned"
	}
	switch status {
	case "approved", "built", "split", "part", "planned":
		return false
	}
	return true
}

// keepSession keeps the Claude session the model job sealed (`work.Worker.keepSession`) for the
// item's next run on this subscription, as `sessions/<item>-<subscription>.bin` on the state
// branch and the item's `session` record. This job cannot open it and only checks its shape. A
// session another subscription kept goes once a run of the item here delivers (the work changed
// hands); a review run takes no work over and changes nothing. A closed item keeps none. What
// it did goes into the result's `session`, for the journal.
func (x *Deliverer) keepSession(number int) {
	action := py.OrStr(x.Plan.Get("action"), "")
	if action != "plan" && action != "build" && action != "revise" {
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
			x.dropSession(number, record, fmt.Sprintf("#%d's work moved from `%s` to `%s`", number,
				record.Str("provider"), x.Provider.ID))
			record = nil
		}
		status := py.Str(x.Result.Get("status"))
		if !goesOn(action, status) {
			if record.Len() > 0 {
				x.dropSession(number, record, fmt.Sprintf("this %s ended %s", action, status))
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
			path := sessions.Path(number, x.Provider.ID)
			if err := sessions.Put(x.GH, path, text, fmt.Sprintf("session: #%d on %s [skip ci]", number,
				x.Provider.ID)); err != nil {
				problem = "could not store it: " + err.Error()
			} else {
				entry := pyjson.New("id", kept.Str("id"), "provider", x.Provider.ID,
					"model", kept.Str("model"), "kind", kept.Str("kind"), "action", action,
					"cwd", kept.Str("cwd"), "project", kept.Str("project"), "head", kept.Str("head"),
					"bytes", len(text), "at", clock.ISO(x.Ctx.Now()), "run", x.Cfg.RunID,
					"ended", py.SliceTo(redact.Redact(status+": "+py.OrStr(x.Result.Get("reason"), ""), nil), 300))
				x.Ctx.Store.Update(func(s *pyjson.Object) any {
					statemod.Item(s, number).Set("session", entry)
					return nil
				}, fmt.Sprintf("session #%d", number))
				said(fmt.Sprintf("kept session `%s` of #%d on %s (%s bytes) for its next run there",
					kept.Str("id"), number, x.Provider.ID, py.Thousands(len(text))))
				return
			}
		}
		if brought {
			said("kept no session: " + problem)
		}
		// This run's session is newer than the one kept, which no longer says where the work is.
		if record.Len() > 0 && brought {
			x.dropSession(number, record, "this run's own session could not be kept")
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

// dropSession deletes the session `record` names and the record.
func (x *Deliverer) dropSession(number int, record *pyjson.Object, why string) {
	path := sessions.Path(number, record.Str("provider"))
	if err := sessions.Remove(x.GH, path, fmt.Sprintf("session: drop #%d (%s) [skip ci]", number, why)); err != nil {
		x.log(fmt.Sprintf("could not drop the kept session `%s`: %s", path, err))
	} else {
		x.log(fmt.Sprintf("dropped the kept session `%s`: %s", path, why))
	}
	x.Ctx.Store.Update(func(s *pyjson.Object) any {
		statemod.Item(s, number).Pop("session")
		return nil
	}, fmt.Sprintf("session #%d dropped", number))
}
