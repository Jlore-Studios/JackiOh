package plan

import (
	"fmt"
	"slices"
	"strconv"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/asks"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/sessions"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/spotcheck"
	hstate "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
)

// Make is plan.make: decide what this run does and with which subscription. A subscription
// outside its hours works only on forced work: the `force` of this dispatch, or an item an
// operator queued with `--force`, which outlives the run it started. `quiet_ok` is the gate's
// verdict: the shared subscription it saw quiet, or "" for none; the workflow always passes one,
// and None (nil: a run by hand) restricts nothing.
func Make(ctx *hctx.Context, force bool /* = False */, item *int /* = None */, mode string /* = "auto" */, quietOK *string /* = None */) *pyjson.Object {
	cfg := ctx.Cfg
	quiet := AnyQuiet
	if quietOK != nil {
		quiet = *quietOK
	}
	if !slices.Contains(Modes, mode) {
		return Nothing("unknown mode " + py.Repr(mode))
	}
	state := ctx.Store.Load()
	asked := RunRequest(ctx, state)
	if asked != nil {
		force = true
		if item == nil {
			item = intPtr(asked.Get("item"))
		}
	}
	if stop := Stops(ctx, state, force); stop != "" {
		return Nothing(stop)
	}
	NoteSecrets(ctx, state)
	notes := Housekeeping(ctx, state)
	notes = append(notes, AnnounceSwitchedOff(ctx, state)...)

	taken := func(planned *pyjson.Object) *pyjson.Object {
		// A `/harness run` is taken up by the run that claims something for it, never by one
		// that found every lane busy: one request, one run.
		if asked != nil {
			ctx.Store.Update(func(s *pyjson.Object) any {
				s.Set("run_requested", nil)
				return nil
			}, "run request taken")
		}
		return planned
	}
	state = ctx.Store.Load()
	notes = append(notes, SyncNeedsPlan(ctx, state)...)
	lanes := ReadLanes(ctx, state)
	// The holds for owners (#2748): put on, lifted or released before anything is given out.
	notes = append(notes, SyncHolds(ctx, state, lanes, force, quiet)...)
	state = ctx.Store.Load()
	if !lanes.AnyRoom() {
		return Nothing(fmt.Sprintf("every lane is busy (%s)", lanes.Describe())).
			Set("housekeeping", pyjson.ListOf(notes))
	}
	skipped := []string{}
	q := queueOf(ctx, state, item, mode, &skipped)
	if mode != "suggest" {
		for _, pair := range Pairs(ctx, state, q, lanes, force, quiet, skipped) {
			candidate, assignment := pair.Candidate, pair.Assignment
			planned := Claim(ctx, candidate, assignment)
			if planned != nil {
				planned.Set("housekeeping", pyjson.ListOf(notes))
				planned.Set("forced", force || candidate.Forced)
				planned.Set("priority", queue.PriorityNames[candidate.Priority])
				planned.Set("skipped", pyjson.ListOf(skipped))
				FillLanes(ctx, lanes, candidate, assignment, planned)
				return taken(planned)
			}
		}
	}
	if item != nil && mode != "suggest" {
		why := WhyNone(ctx, state, lanes)
		if held := Holds(ctx, state, q, lanes, force, quiet)[*item]; held != nil {
			why = fmt.Sprintf("it is %s; %s", HoldLine(ctx, held), why)
		}
		return Nothing(fmt.Sprintf("#%d is not queued (or has failed %d times), or no subscription "+
			"can take it now: %s", *item, cfg.MaxFailures, why)).
			Set("skipped", pyjson.ListOf(skipped))
	}
	if mode == "suggest" || mode == "auto" {
		if planned := SuggestionPlan(ctx, lanes, mode == "suggest", quiet); planned != nil {
			planned.Set("skipped", pyjson.ListOf(skipped))
			return taken(planned)
		}
	}
	reason := "nothing is queued"
	if len(q) > 0 {
		reason = "no subscription can take the queue now: " + WhyNone(ctx, state, lanes)
		if held := HeldNote(ctx, Holds(ctx, state, q, lanes, force, quiet)); held != "" {
			reason += "; waiting for their owners: " + held
		}
	}
	return Nothing(reason).Set("housekeeping", pyjson.ListOf(notes)).Set("skipped", pyjson.ListOf(skipped))
}

// FillLanes starts one more run when, with this item claimed, a lane (or the planning lane) and
// more work are still free. That run does the same, so the lanes fill one run at a time.
func FillLanes(ctx *hctx.Context, lanes *Lanes, taken *queue.Candidate, assignment *Assignment, planned *pyjson.Object) {
	if ctx.Cfg.ControllerLoop {
		return // the status loop plans again at once, in the same tick (`controller.start`)
	}
	after := lanes.WithRun(taken.Number, assignment.Provider.ID, assignment.LaneAction(), assignment.FillsIn())
	if !after.AnyRoom() {
		return
	}
	state := ctx.Store.Load()
	rest := filter(queue.Candidates(ctx, state, nil, nil), func(c *queue.Candidate) bool { return c.Number != taken.Number })
	if len(Pairs(ctx, state, rest, after, false, AnyQuiet, nil)) == 0 {
		return
	}
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		ctx.Dispatch(nil, false, "auto")
		planned.Set("filled", "started another run for the next free lane")
	}); failed {
		planned.Set("filled", "could not start another run: "+exc.Error())
	}
}

// SyncNeedsPlan keeps `bot:needs-plan` on exactly the queued items in the Needs plan stage, and
// on the ones being planned now; and `bot:planned` on the queued ones a strong model has planned
// (it stays after they leave the queue, and comes off only if the item needs a plan again).
func SyncNeedsPlan(ctx *hctx.Context, state *pyjson.Object) []string {
	needsPlan, plannedLabel := config.LabelNeedsPlan, config.LabelPlanned
	notes := []string{}
	wanted := map[int]bool{}
	planned := map[int]bool{}
	hasPlanned := map[int]bool{}
	var labelled []*pyjson.Object
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		queued := filter(queue.Candidates(ctx, state, nil, nil), func(c *queue.Candidate) bool { return c.Kind == "build" })
		for _, c := range queued {
			if queue.NeedsPlan(c) {
				wanted[c.Number] = true
			}
		}
		labelled = ctx.GH.ListIssues(needsPlan, "open", 300, "")
		// A mode (#60) plans for itself: its item carries no `planned` label.
		for _, c := range queued {
			if c.Planned && !queue.NeedsPlan(c) && c.Mode == "" {
				planned[c.Number] = true
			}
		}
		for _, t := range ctx.GH.ListIssues(plannedLabel, "open", 300, "") {
			hasPlanned[py.MustInt(t.Get("number"))] = true
		}
	}); failed {
		return []string{fmt.Sprintf("could not sync `%s`: %s", needsPlan, exc.Error())}
	}
	for _, number := range sortedInts(planned) {
		if !hasPlanned[number] {
			ctx.GH.AddLabels(number, []string{plannedLabel})
		}
	}
	for _, number := range sortedInts(hasPlanned) {
		if wanted[number] {
			ctx.GH.RemoveLabel(number, plannedLabel)
		}
	}
	have := map[int]bool{}
	for _, thread := range labelled {
		number := py.MustInt(thread.Get("number"))
		have[number] = true
		record := recordOf(state, strconv.Itoa(number))
		planning := config.IsLive(queue.LabelNames(thread)) && eqStr(record.Get("action"), "plan")
		if !wanted[number] && !planning {
			ctx.GH.RemoveLabel(number, needsPlan)
			notes = append(notes, fmt.Sprintf("#%d left the Needs plan stage", number))
		}
	}
	for _, number := range sortedInts(wanted) {
		if !have[number] {
			ctx.GH.AddLabels(number, []string{needsPlan})
			notes = append(notes, fmt.Sprintf("#%d needs a plan", number))
		}
	}
	return notes
}

// NoteSecrets records which provider secrets this run's workflow has, for the status loop: its
// own list is fixed when its long run is created, so a secret added since shows there once a
// plan job has seen it (`providers.newer_secrets`).
func NoteSecrets(ctx *hctx.Context, state *pyjson.Object) {
	record := providers.SecretsRecord(ctx.Cfg.Secrets, state, ctx.Now())
	if record != nil {
		ctx.Store.Update(func(s *pyjson.Object) any {
			s.Set("secrets", record)
			return nil
		}, "secrets seen")
	}
}

// KeepClosed is KEEP_CLOSED, and PruneEvery PRUNE_EVERY: how long an item's record outlives its
// closed thread in the state file, and how often the records are looked over (#160, #317 part
// 12): GitHub's Contents API stops returning a file past 1 MB, and the 95 item records held 303
// KB of the 374 KB file on 2026-10-05.
var KeepClosed = 7 * 24 * time.Hour

// PruneEvery is PRUNE_EVERY.
var PruneEvery = 24 * time.Hour

// ShedWhenClosed is SHED_WHEN_CLOSED: what a closed thread's record sheds at once: the bulky
// parts only an open one uses.
var ShedWhenClosed = []string{"handoff", "last_findings", "self_check_findings", "strike_log", "votes",
	"asks", "taken_asks", "question", "paused", "hold", "handoff_requested", "waiting_comment", "own_review",
	"build_next", "waiting_turn", "turn"}

// Prune is, once a day: drop the record of every item whose thread closed more than
// `KEEP_CLOSED` ago, and the bulky parts of the others that are closed. Open threads keep
// everything.
func Prune(ctx *hctx.Context, state *pyjson.Object) []string {
	last := clock.ParseISOValue(state.Obj("pruned").Get("at"))
	now := ctx.Now()
	if !last.IsZero() && now.Sub(last) < PruneEvery {
		return []string{}
	}
	openNow := map[int]bool{}
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		for _, t := range ctx.GH.ListIssues("", "open", 1000, "") {
			openNow[py.MustInt(t.Get("number"))] = true
		}
	}); failed {
		return []string{"could not prune the state file: " + exc.Error()}
	}
	var gone, shed []int
	items := state.Obj("items")
	for _, key := range items.Keys() {
		if !py.IsDecimal(key) {
			continue
		}
		number := py.MustInt(key)
		if openNow[number] || number == 0 {
			continue
		}
		var thread *pyjson.Object
		if _, failed := herr.Catch[*herr.GitHubError](func() { thread = ctx.GH.GetIssue(number) }); failed {
			continue
		}
		if !eqStr(thread.Get("state"), "closed") {
			continue
		}
		closed := clock.ParseISOValue(thread.Get("closed_at"))
		if !closed.IsZero() && now.Sub(closed) >= KeepClosed {
			gone = append(gone, number)
		} else if slices.ContainsFunc(ShedWhenClosed, items.Obj(key).Has) {
			shed = append(shed, number)
		}
	}

	ctx.Store.Update(func(s *pyjson.Object) any {
		for _, number := range gone {
			s.Obj("items").Pop(strconv.Itoa(number))
		}
		for _, number := range shed {
			for _, fieldName := range ShedWhenClosed {
				s.Obj("items").Obj(strconv.Itoa(number)).Pop(fieldName)
			}
		}
		s.Set("pruned", pyjson.New("at", clock.ISO(now), "dropped", len(gone), "shed", len(shed)))
		return nil
	}, "prune")
	dropped := sessions.Prune(ctx, openNow) // the Claude sessions kept for closed items (#2748)
	return append([]string{fmt.Sprintf("pruned the state file: dropped %d closed item(s), lightened %d",
		len(gone), len(shed))}, dropped...)
}

// Housekeeping requeues items a dead run left working; queues a revision for conflicted bot
// PRs; puts back in the merge queue a cleared one it took out with no failed build
// (`queue.requeue_cleared`).
func Housekeeping(ctx *hctx.Context, state *pyjson.Object) []string {
	labelRevise := config.LabelRevise
	labelBlocked := config.LabelBlocked
	notes := Prune(ctx, state)
	// A pull request waiting its turn whose turn came is taken up first (#2948), so a conflict its
	// turn brought is its lane's, below.
	notes = append(notes, SyncTurns(ctx, 0)...)
	state = ctx.Store.Load()
	for _, thread := range WorkingThreads(ctx) {
		number := py.MustInt(thread.Get("number"))
		record := recordOf(state, strconv.Itoa(number))
		runID := py.OrStr(record.Get("run_id"), "")
		if runID != "" && runID == ctx.Cfg.RunID {
			continue
		}
		open := eqStr(thread.Get("state"), "open")
		if queue.OwnReviewWaits(record) && open {
			continue // its own review run has not started (#2748): it waits in the state file's queue
		}
		if RunStatus(ctx, runID) != "dead" {
			continue
		}
		if !open { // closed meanwhile: nothing to requeue
			queue.SetStateLabel(ctx, number, queue.LabelNames(thread), "")
			if queue.OwnReviewOf(record) != nil {
				ctx.Store.Update(func(s *pyjson.Object) any {
					hstate.Item(s, number).Pop(queue.OwnReviewKey)
					return nil
				}, fmt.Sprintf("own review #%d closed", number))
			}
			if spotcheck.Running(state, number) {
				// An after-merge spot-check's run died (#2748): the check goes back in the queue,
				// a try spent.
				ctx.Store.Update(func(s *pyjson.Object) any {
					spotcheck.Requeue(s, number, "its run ended without finishing", ctx.Cfg.MaxFailures, ctx.Now())
					return nil
				}, fmt.Sprintf("spot-check #%d requeued", number))
				notes = append(notes, fmt.Sprintf("requeued the spot-check of #%d from a dead run", number))
				continue
			}
			notes = append(notes, fmt.Sprintf("#%d was closed; its dead run's label is gone", number))
			continue
		}
		var wanted string
		ownReview := queue.OwnReviewOf(record) != nil
		if ownReview {
			// Its own review run died (#2748): its lane takes it up again, as its own work.
			wanted = config.LabelOwnReview
			ctx.Store.Update(func(s *pyjson.Object) any {
				queue.RequeueOwnReview(hstate.Item(s, number), ctx.Now())
				return nil
			}, fmt.Sprintf("own review #%d requeued", number))
		} else if eqStr(record.Get("kind"), "review") {
			wanted = config.LabelCross
		} else if thread.Has("pull_request") {
			wanted = labelRevise
		} else {
			wanted = config.LabelBuild
		}
		queue.SetStateLabel(ctx, number, queue.LabelNames(thread), wanted)
		ctx.Store.Update(func(s *pyjson.Object) any {
			asks.GiveBack(hstate.Item(s, number))
			return nil
		}, fmt.Sprintf("asks #%d", number))
		ctx.GH.CreateComment(number, "The run that was working on this ended without "+
			"finishing; it is back in the queue.")
		notes = append(notes, fmt.Sprintf("requeued #%d from a dead run", number))
	}
	survey := state.Obj("suggest")
	if survey.Truthy("provider") && RunStatus(ctx, survey.Get("run_id")) == "dead" {
		ctx.Store.Update(func(s *pyjson.Object) any {
			s.Obj("suggest").Update(pyjson.New("provider", nil, "run_id", nil))
			return nil
		}, "survey ended")
	}
	for _, thread := range ctx.GH.ListIssues(config.LabelPR, "open", 300, "") {
		if !thread.Has("pull_request") {
			continue
		}
		names := queue.LabelNames(thread)
		if names[labelRevise] || config.IsLive(names) || names[labelBlocked] || queue.IsHuman(names) {
			continue
		}
		pull := ctx.GH.GetPull(py.MustInt(thread.Get("number")))
		record := recordOf(state, py.Str(pull.Get("number")))
		if eqStr(pull.Get("mergeable_state"), "dirty") && !record.Truthy("stop_requested") {
			number := py.MustInt(pull.Get("number"))
			if queue.WaitingTurnOf(record) != nil {
				// It waited its turn behind another, and `main` conflicts with it now (#2948): its turn.
				notes = append(notes, TakeTurn(ctx, number, "it conflicts with `main` now"))
				continue
			}
			queue.SetStateLabel(ctx, number, names, labelRevise)
			if pull.Truthy("auto_merge") && pull.Truthy("node_id") {
				// The revision may come from another model; its approval decides afresh.
				herr.Catch[*herr.GitHubError](func() { ctx.GH.DisableAutoMerge(py.Str(pull.Get("node_id"))) })
			}
			queue.NotMerging(ctx, number, names)
			// The lane owns its pull request until it merges (#2948): the subscription whose approval
			// shipped this head resolves its conflict, ahead of its new work (`Pairs`), held for it a
			// while or handed off as any owner's work (`Owning`), as a CI fix goes to it
			// (`events.fixCI`).
			sha := py.OrStr(pull.Obj("head").Get("sha"), "")
			owner := MergeOwner(record, sha)
			owned := owner != ""
			ctx.Store.Update(func(s *pyjson.Object) any {
				hstate.Item(s, number).Update(pyjson.New("kind", "revise", "source", "conflict",
					"queued_at", clock.ISO(ctx.Now()), "failures", 0, "stop_requested", false))
				if owned {
					Own(s, number, owner, fmt.Sprintf("its approval shipped `%s`, and the last approver owns "+
						"the merge, so it resolves the conflict", py.SliceTo(sha, 12)), ctx.Now())
				}
				return nil
			}, fmt.Sprintf("conflict #%d", number))
			note := fmt.Sprintf("queued #%d: it conflicts with main", number)
			if owned {
				note += fmt.Sprintf("; `%s`, its merge owner, resolves it", owner)
			}
			notes = append(notes, note)
			continue
		}
		var requeued string
		if exc, failed := herr.Catch[*herr.GitHubError](func() {
			requeued = queue.RequeueCleared(ctx, pull, record)
		}); failed {
			requeued = fmt.Sprintf("#%s: could not check the merge queue (%s)", py.Str(pull.Get("number")), exc.Error())
		}
		if requeued != "" {
			notes = append(notes, requeued)
		}
	}
	labelPROpen := config.LabelPROpen
	for _, thread := range ctx.GH.ListIssues(labelPROpen, "open", 300, "") {
		number := py.MustInt(thread.Get("number"))
		if queue.OpenPullForBranch(ctx, queue.BranchForIssue(number)) == nil {
			ctx.GH.RemoveLabel(number, labelPROpen)
			notes = append(notes, fmt.Sprintf("#%d has no open bot PR any more", number))
		}
	}
	return notes
}
