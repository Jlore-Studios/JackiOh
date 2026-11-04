package deliver

// The own review run (#2748, the owner's decision of 2026-11-02; `own_review_run`).
//
// A build or revision that changes code hands its lane to its own review run: it pushes the change
// and opens the pull request (a draft, which CI runs on, so CI starts while the review runs), and
// `handOn` reserves the lane for a second run on the same subscription (`own_review` on the pull
// request's record, for `own_review_reserve_minutes`), puts `bot:reviewing` on it, and the run is
// started at once as the next one is (`Chain_`, or the status loop's tick). That run reviews the
// change, fixes what it finds and approves it (`work.Worker.OwnReview`); `OwnReview` delivers it:
// its approval at the item's tier ships the head (G's rule, `reviewrule`), so it marks the pull
// request ready and turns auto-merge on, and its subscription is the merge owner. Its verdict rounds
// running out sends a medium item to its final audit and a hard one to the strike path.

import (
	"fmt"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	planmod "github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	statemod "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
)

// OwnReviewTailChars is how much of each red check's output the reservation keeps for the own
// review run's first round (a green check's none).
var OwnReviewTailChars = 2000

// chained is: this build or revision hands its lane to the change's own review run: the plan said
// so (`plan.Chains`), and the work job built and checked it for that (`review_next`).
func (x *Deliverer) chained() bool {
	return x.Cfg.OwnReviewRun && x.Plan.Truthy("own_review_next") &&
		pyjson.Equal(x.Result.Get("review_next"), "own")
}

// ownReviewDelivery is: this run is a change's own review run.
func (x *Deliverer) ownReviewDelivery() bool {
	return x.Plan.Get("action") == "review" && x.Plan.Truthy("own_review")
}

// handOn reserves pull request `pr`'s own review run on this run's subscription (#2748): `head` is
// the commit this run built, whose votes name it as its builder; auto-merge stays off; the record's
// `own_review` holds the subscription, until when its lane is reserved, the head and the checks
// this run ran on it; and `bot:reviewing` goes on. Returns the line for the comment.
func (x *Deliverer) handOn(pr int, head string) string {
	x.voteReset(pr, head, x.Provider.Family, x.Provider.ID)
	x.holdAutoMerge(pr)
	now := x.Ctx.Now()
	until := now.Add(time.Duration(x.Cfg.OwnReviewReserveMinutes) * time.Minute)
	checks := []any{}
	for _, g := range objList(x.Result.Get("gates")) {
		kept := g.Copy()
		if g.Truthy("ok") {
			kept.Set("tail", "")
		} else {
			kept.Set("tail", py.SliceFrom(py.OrStr(g.Get("tail"), ""), -OwnReviewTailChars))
		}
		checks = append(checks, kept)
	}
	kind := py.OrStr(x.Plan.Get("action"), "build")
	source := "build"
	// Its verdict rounds go on from this run's (#725): a build's on its issue's record, a revision's
	// on its pull request's, as each counted them when it reviewed itself.
	roundsOn := py.MustInt(x.Plan.Get("number"))
	if kind == "revise" {
		source = py.OrStr(x.Plan.Get("source"), "request")
	}
	rounds, roundTier := x.rounds()
	// The model that built the head reviews it and fixes what it finds (#2903): one model, one lane.
	model := x.headModel()
	reservation := pyjson.New("provider", x.Provider.ID, "model", model, "since", clock.ISO(now),
		"until", clock.ISO(until), "head", head, "from_run", x.Cfg.RunID, "kind", kind, "source", source,
		"gates", checks, "rounds_used", rounds, "round_tier", roundTier, "rounds_on", roundsOn)
	x.remember(pr, pyjson.New(queue.OwnReviewKey, reservation, "queued_at", clock.ISO(now)))
	queue.SetStateLabel(x.Ctx, pr, x.Labels(pr), config.LabelOwnReview)
	x.OwnReviewPR = pr
	x.log(fmt.Sprintf("#%d: its own review run on %s is next; its lane is kept until %s", pr,
		x.Provider.ID, clock.ISO(until)))
	reviewer := "the model that built it"
	if seat := x.Cfg.Pool.Seat(x.Provider.ID, model); seat != nil {
		reviewer = seat.Describe() + ", the model that built it,"
	}
	return fmt.Sprintf("Its own review run is next, on `%s` (`%s`): %s reviews it adversarially in a "+
		"fresh session, reads its CI, and fixes what it finds in the same run, up to %d rounds. Once it "+
		"approves, the pull request is marked ready and auto-merge turns on. This lane is kept for it "+
		"until %s.", x.Provider.ID, config.LabelOwnReview, reviewer, x.CyclesCap(), planmod.Clock(x.Ctx, until))
}

// checksSaid is what a build or revision that hands its lane on says of its checks.
func (x *Deliverer) checksSaid() string {
	var red []string
	for _, g := range objList(x.Result.Get("gates")) {
		if !g.Truthy("ok") && !g.Truthy("pre_existing") && !g.Truthy("inconclusive") {
			red = append(red, py.Str(g.Get("name")))
		}
	}
	if len(red) == 0 {
		return "It ran the repository's checks, and they pass."
	}
	return fmt.Sprintf("It ran the repository's checks; %s still red, which its own review run "+
		"fixes as a finding.", ticked(red))
}

// keepOwnWork pushes what an own review run made that did not finish (its fixes so far) onto the
// pull request's branch, auto-merge off, so its next own review run starts from it.
func (x *Deliverer) keepOwnWork() (bool, string) {
	return x.publish(false)
}

// ownReviewAgain puts the pull request's own review back to waiting for its subscription (its run
// ended without a verdict it could deliver): its lane takes it up again as its own work, held for it
// a while or handed off to a final audit as owned work is (`plan.Owning`). One that is no longer
// labelled for it (blocked, stepped up) is dropped.
func (x *Deliverer) ownReviewAgain(number int) {
	labelled := x.Labels(number)[config.LabelOwnReview]
	x.Ctx.Store.Update(func(s *pyjson.Object) any {
		entry := statemod.Item(s, number)
		if labelled {
			queue.RequeueOwnReview(entry, x.Ctx.Now())
		} else {
			entry.Pop(queue.OwnReviewKey)
		}
		return nil
	}, fmt.Sprintf("own review #%d again", number))
}

// dropOwnReview ends the pull request's own review: delivered, stopped or blocked.
func (x *Deliverer) dropOwnReview(number int) {
	x.Ctx.Store.Update(func(s *pyjson.Object) any {
		statemod.Item(s, number).Pop(queue.OwnReviewKey)
		return nil
	}, fmt.Sprintf("own review #%d done", number))
}

// ownRounds records the verdict rounds an own review run used (#725) where its build counted them
// (`rounds_on`: a build's issue, a revision's pull request), and, while it waits for another own
// review run, on its reservation too.
func (x *Deliverer) ownRounds(number int, reserved *pyjson.Object, again bool) {
	rounds, roundTier := x.rounds()
	target := py.MustInt(py.Or(reserved.Get("rounds_on"), number))
	x.remember(target, pyjson.New("verdict_rounds", rounds, "round_tier", roundTier))
	if again {
		x.Ctx.Store.Update(func(s *pyjson.Object) any {
			if found := queue.OwnReviewOf(statemod.Item(s, number)); found != nil {
				found.Set("rounds_used", rounds).Set("round_tier", roundTier)
			}
			return nil
		}, fmt.Sprintf("own review #%d rounds", number))
	}
}

// cutOffAgain counts an own review run cut off before any verdict of its (a pause, a cap, a dying
// job): after `max_failures` of those in a row its review goes to its final audit instead, rather
// than go round its lane for ever. A run that reached a verdict starts the count again. True when it
// was handed off.
func (x *Deliverer) cutOffAgain(number int, reserved *pyjson.Object) bool {
	before := py.MustInt(py.Or(reserved.Get("rounds_used"), 0))
	rounds, _ := x.rounds()
	calls, _ := py.Int(py.Or(x.Result.Get("model_calls"), 0))
	count := 0
	if rounds <= before && calls != 0 {
		count = py.MustInt(py.Or(reserved.Get("interruptions"), 0)) + 1
	}
	x.Ctx.Store.Update(func(s *pyjson.Object) any {
		if found := queue.OwnReviewOf(statemod.Item(s, number)); found != nil {
			found.Set("interruptions", count)
		}
		return nil
	}, fmt.Sprintf("own review #%d cut off", number))
	if count < x.Cfg.MaxFailures {
		return false
	}
	planmod.HandOffOwnReview(x.Ctx, number, fmt.Sprintf("its own review run was cut off %d times in a row "+
		"before a verdict", count), x.Difficulty(number))
	return true
}

// OwnReview delivers a change's own review run (#2748, 2026-11-02) on bot pull request `number`.
func (x *Deliverer) OwnReview(number int, status string) {
	record := x.Record(number)
	reserved := queue.OwnReviewOf(record)
	who := x.Provider.ID
	switch status {
	case "infra":
		if !x.Result.Truthy("no_result") {
			x.keepOwnWork()
		}
		x.infra(number, "review", config.LabelOwnReview)
		x.ownReviewAgain(number)
		return
	case "interrupted":
		pushed, problem := x.keepOwnWork()
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelOwnReview)
		x.ownReviewAgain(number)
		x.ownRounds(number, reserved, true)
		if x.cutOffAgain(number, reserved) {
			return
		}
		kept := ""
		if pushed {
			kept = " Its fixes so far are on the branch, where its next own review run starts."
		}
		if problem != "" {
			kept += " " + problem + "."
		}
		x.GH.CreateComment(number, fmt.Sprintf("Its own review run was paused (%s): %s.%s It goes on in "+
			"its next own review run on `%s`, its lane's own work (held for it a while when only its "+
			"5-hour window keeps it, else handed to its final audit).", x.link(),
			py.Str(x.Result.Get("reason")), kept, who))
		return
	case "stopped":
		x.dropOwnReview(number)
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), "")
		x.holdAutoMerge(number)
		x.GH.CreateComment(number, "Stopped, as asked. It will not merge by itself.")
		return
	case "failed":
		fails := py.MustInt(record.GetOr("failures", 0)) + 1
		x.remember(number, pyjson.New("failures", fails))
		if fails >= x.Cfg.MaxFailures {
			x.dropOwnReview(number)
			queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelBlocked)
			x.holdAutoMerge(number)
			x.GH.CreateComment(number, fmt.Sprintf("Its own review run failed %d times (%s): %s. It "+
				"needs a person.", fails, x.link(), py.Str(x.Result.Get("reason"))))
			return
		}
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelOwnReview)
		x.ownReviewAgain(number)
		x.GH.CreateComment(number, fmt.Sprintf("Its own review run failed (%s): %s. It goes again on "+
			"`%s` (attempt %d of %d).", x.link(), py.Str(x.Result.Get("reason")), who, fails, x.Cfg.MaxFailures))
		return
	case "blocked":
		x.dropOwnReview(number)
		x.holdAutoMerge(number)
		question := py.OrStr(py.Or(x.Result.Get("question"), x.Result.Get("reason")), "")
		if x.Result.Truthy("human_required") {
			x.NeedsPerson(number, question, "revise this", "", fmt.Sprintf(
				"`%s revise <notes>` (or `@%s <notes>`) queues the rest", config.Slash, x.Cfg.BotLogin))
			return
		}
		x.askDecision(number, "revise", "Its own review run needs a decision before it can fix this.", "", question,
			fmt.Sprintf("answer here with `%s revise <your answer>` (or `@%s <your answer>`), which queues it again",
				config.Slash, x.Cfg.BotLogin))
		return
	}
	approved := status == "approved"
	pushed, problem := x.publish(approved)
	if problem != "" {
		x.dropOwnReview(number)
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelBlocked)
		x.holdAutoMerge(number)
		x.GH.CreateComment(number, fmt.Sprintf("I could not push its own review run's fixes (%s): %s.",
			x.link(), problem))
		return
	}
	head := py.OrStr(x.Result.Get("start"), "")
	if pushed {
		head = py.OrStr(x.Result.Get("head"), "")
	} else if now := x.branchHead(number); now != "" && head != "" && now != head {
		// Someone pushed while it reviewed: that review was of a head the branch no longer has, so it
		// counts for nothing, and the head nobody reviewed goes to its final audit.
		x.dropOwnReview(number)
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), "")
		x.markReady(number, false)
		note := x.approved(number, now, !record.Truthy("pending_request"), false, false, nil)
		x.GH.CreateComment(number, fmt.Sprintf("The branch moved while its own review run (%s) on `%s` "+
			"reviewed it, so that review does not count. %s", x.link(), who, note))
		return
	}
	rounds, _ := x.rounds()
	x.ownRounds(number, reserved, false)
	fields := pyjson.New("failures", 0,
		"self_check_findings", py.Or(x.Result.Get("self_check_findings"), []any{}))
	if pushed && record.Truthy("ci_fixed_head") && pyjson.Equal(record.Get("ci_fixed_head"), x.Result.Get("start")) {
		fields.Set("ci_fixed_head", head) // the head its CI fix ends at (`events.fixCI`'s strikes)
	}
	x.remember(number, fields)
	x.dropOwnReview(number)
	waiting := record.Truthy("pending_request")
	verdicts := pyLen(py.Or(x.Result.Get("cycles"), []any{}))
	fixed := ""
	if pushed {
		fixed = fmt.Sprintf(" It fixed what its earlier rounds found and pushed `%s`.", py.SliceTo(head, 12))
	}
	switch status {
	case "approved":
		x.unstick(number)
		x.countCIFix(number, head, pushed) // red CI its final check fixed in the run (#2948)
		final := x.settleSaid()
		if final != "" {
			final = " " + final
		}
		var ahead []int
		if !waiting && x.Cfg.HoldMinutes > 0 {
			ahead = x.ahead(number, head)
		}
		if len(ahead) > 0 {
			// Its final check found a pull request ahead of it that it conflicts with (#2948): it
			// waits its turn, approved, a draft, with its clearance on record for its resolution.
			x.approved(number, head, false, true, pushed, nil)
			x.holdAutoMerge(number)
			note := x.waitTurn(number, head, ahead)
			x.GH.CreateComment(number, fmt.Sprintf("Its own review run (%s) on `%s`: %s reviewed it "+
				"adversarially and approved it on round %d of %d.%s%s %s", x.link(), who,
				x.ReviewSeat.Describe(), rounds, x.CyclesCap(), fixed, final, note))
			return
		}
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), "")
		x.markReady(number, waiting)
		note := x.approved(number, head, !waiting, true, pushed, nil)
		if waiting {
			note = "A comment arrived during the run, so a revision answers it first, and auto-merge " +
				"waits for it."
		} else if !pushed && reserved.Str("source") == "ci" && record.Truthy("ci_run_id") &&
			!record.Truthy("ci_on_head") {
			// A CI fix of the merge queue's build that changed nothing, which its review agreed
			// with: CI runs its failed jobs again. One of the head's own CI read that CI itself
			// (`ciRed`), and its build ran it again already when its failing check passed locally
			// (#2948, `localPass`).
			x.rerunCI(number)
			note += " Nothing needed changing, so I asked CI to run the failed jobs again."
		}
		x.GH.CreateComment(number, fmt.Sprintf("Its own review run (%s) on `%s`: %s reviewed it "+
			"adversarially and approved it on round %d of %d.%s%s %s", x.link(), who,
			x.ReviewSeat.Describe(), rounds, x.CyclesCap(), fixed, final, note))
	case "built":
		// A medium item's verdict rounds ran out (#725): its final audit judges the kept work, on
		// another subscription first (2026-11-02).
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), "")
		x.markReady(number, false)
		note := x.awaitReview(number, head, !waiting)
		x.GH.CreateComment(number, fmt.Sprintf("Its own review run (%s) on `%s` used its review rounds "+
			"(%d of %d) without an approval, after %d round(s) in this run.%s %s\n\n%s", x.link(), who,
			rounds, x.CyclesCap(), verdicts, fixed, note, x.findingsMd()))
	default: // not_approved: a hard item's rounds ran out, or its review could not be read
		first := x.firstFinding()
		if first == "" {
			first = py.Str(x.Result.Get("reason"))
		}
		x.holdAutoMerge(number)
		if x.strike(number, "its own review run did not approve it: "+first) {
			return
		}
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelBlocked)
		stuck := x.markStuck(number)
		x.remember(number, pyjson.New("last_findings", py.Or(x.Result.Get("findings"), []any{})))
		x.GH.CreateComment(number, fmt.Sprintf("Its own review run (%s) on `%s`: %s, so it will not "+
			"merge by itself.%s\n\n%s\n\n%s\n\n%sComment `@%s <guidance>` here to have me try again with "+
			"your notes.", x.link(), who, x.notApprovedHead(), fixed, x.findingsMd(), x.whyMd(), stuck,
			x.Cfg.BotLogin))
	}
}

// markReady marks a draft pull request ready for review: an approval, or a final audit, can turn
// auto-merge on only on one that is not a draft. A pull request a revision is still asked of stays
// as it is (`waiting`).
func (x *Deliverer) markReady(number int, waiting bool) {
	if waiting {
		return
	}
	pull := x.GH.GetPull(number)
	if pull.Truthy("draft") && pull.Truthy("node_id") {
		x.try(func() { x.GH.MarkReady(py.Str(pull.Get("node_id"))) })
	}
}
