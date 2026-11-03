package work

import (
	"fmt"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
)

// A fullsend reconcile may send a part it finds largely wrong back to the subscription that built
// it (#2748, section 2): the plan names the parts it may (`send_backs`, `plan.SendBackable`), its
// first pass answers `send_back` with each one's findings, and the run then ends `sent_back` with
// nothing of its own kept (`sendBack`); deliver reopens and queues each part (`deliver.SendBack`).
// A part past the cap, `fullsend_send_backs`, is the reconcile's to fix.

// SentBackNote is what a part sent back reads first, before the reconcile's findings. `{parent}`
// and `{onto}` are filled with str.format.
var SentBackNote string = "#{parent}'s reconcile read every part of the tree and sent this one back " +
	"to you, the subscription that built it: it found it largely wrong. Your earlier change is in " +
	"this worktree already (it landed on `{onto}`, or was kept on this part's own branch): rework " +
	"it so that it does what this issue asks and answers every finding below, rather than writing a " +
	"second copy beside it."

// reconciles is: this run is a fullsend tree's reconcile (its plan has the tree's `children`).
func (x *Worker) reconciles() bool {
	return x.Plan.Get("mode") == "fullsend" && x.Plan.Truthy("children") && x.planItem("action") == "build"
}

// sendBackText is what a reconcile is told of the parts it may send back: each with the
// subscription that built it, or that none may go back now, so it fixes every part itself
// (`reconcile.md`'s `$send_back`).
func (x *Worker) sendBackText() string {
	open := pyjson.Objs(x.Plan.Get("send_backs"))
	limit := py.MustInt(py.Or(x.Plan.Get("send_back_cap"), 0))
	if len(open) == 0 {
		why := "this bot sends no part back"
		if limit > 0 {
			why = fmt.Sprintf("each was sent back %d time(s) already, the most a part goes back, or "+
				"was not built by a subscription on record", limit)
		}
		return "No part can be sent back now (" + why + "): fix every part yourself, however wrong it is."
	}
	lines := []string{"These parts may be sent back, each to the subscription that built it:", ""}
	for _, entry := range open {
		lines = append(lines, fmt.Sprintf("- #%s (%s), built by `%s`", py.Str(entry.Get("part")),
			py.OrStr(entry.Get("title"), "untitled"), py.Str(entry.Get("built_by"))))
	}
	lines = append(lines, "", fmt.Sprintf("To send one or more of them back, answer with the `send_back` "+
		"line under \"Your final message\" and stop there: change nothing and merge nothing. The harness "+
		"reopens each part you name, the subscription that built it builds it again onto this branch with "+
		"your findings, and a reconcile runs again once they have landed. A part goes back at most %d "+
		"time(s); any other part, and anything smaller, you fix yourself.", limit))
	return strings.Join(lines, "\n")
}

// sendBack ends a reconcile whose first pass sent parts back (`report.SendBack`): those the plan
// lets go back, each with its builder and findings, go into the result as `send_back`, its status
// `sent_back`, and the worktree goes back to where the run started, so nothing of it ships; a later
// reconcile merges and fixes once the parts have landed again. False when it named none that may
// go back (`send_back_refused` keeps them): the run goes on, and the reconcile fixes them itself.
func (x *Worker) sendBack(report *verdicts.BuildReport) bool {
	allowed := map[string]*pyjson.Object{}
	for _, entry := range pyjson.Objs(x.Plan.Get("send_backs")) {
		allowed[py.Str(entry.Get("part"))] = entry
	}
	sent, refused := []any{}, []any{}
	for _, s := range report.SendBack {
		if entry := allowed[py.Str(s.Part)]; entry != nil {
			sent = append(sent, pyjson.New("part", s.Part, "built_by", py.Str(entry.Get("built_by")),
				"findings", redact.Redact(s.Findings, nil)))
		} else {
			refused = append(refused, s.Part)
		}
	}
	if len(refused) > 0 {
		x.Result.Set("send_back_refused", refused)
	}
	if len(sent) == 0 {
		x.cameOfIt("it sent back parts that may not go back, so it fixes them itself")
		return false
	}
	x.resetTo(x.StartSHA)
	shown := make([]string, len(sent))
	for i, s := range sent {
		shown[i] = "#" + py.Str(pyjson.Obj(s).Get("part"))
	}
	x.cameOfIt("sent " + strings.Join(shown, ", ") + " back to their builders")
	x.Result.Set("status", "sent_back")
	x.Result.Set("reason", "the reconcile sent "+strings.Join(shown, ", ")+" back to the subscriptions "+
		"that built them")
	x.Result.Set("send_back", sent)
	x.Result.Set("report", report.Body)
	x.Result.Set("title", x.Plan.GetOr("title", ""))
	x.lastCheckpoint()
	x.finish()
	return true
}

// refusedFinding is what the reconcile's next pass is told when no part it named may go back.
func (x *Worker) refusedFinding(report *verdicts.BuildReport) *verdicts.Finding {
	named := make([]string, len(report.SendBack))
	for i, s := range report.SendBack {
		named[i] = fmt.Sprintf("#%d", s.Part)
	}
	return newFinding("blocking", strings.Join(named, ", "), "These parts cannot be sent back now "+
		"(only those the prompt lists may go back, each up to its cap): fix them yourself in this "+
		"worktree, with the rest of the reconcile, as the prompt's steps say.", "the harness")
}
