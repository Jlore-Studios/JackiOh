package deliver

import (
	"fmt"
	"strconv"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/asks"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/declined"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/easy"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/human"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/issueplan"
	journalmod "github.com/Jlore-Studios/JackiOh/bot/go/internal/journal"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	statemod "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stepup"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/triage"
)

// ------------------------------------------------------------------ an item

// A stop or a halt that came after the model job's last checkpoint still counts.
func (x *Deliverer) lateStop(number int, status string) string {
	if status == "failed" || status == "infra" || status == "stopped" {
		return status
	}
	state := x.Ctx.Store.Load()
	record := state.Obj("items").Obj(strconv.Itoa(number))
	started := clock.ParseISO(py.OrStr(record.Get("started_at"), ""))
	stopped := clock.ParseISO(py.OrStr(record.Get("stopped_at"), ""))
	if record.Truthy("stop_requested") && (started.IsZero() || stopped.IsZero() || !stopped.Before(started)) {
		x.Result.Set("status", "stopped")
		x.Result.Set("reason", "stopped by @"+py.Str(record.Get("stopped_by")))
		return "stopped"
	}
	switch status {
	case "approved", "built", "not_approved", "blocked", "part":
		if state.Truthy("halted") || x.Ctx.RepoHalted() {
			x.Result.Set("status", "interrupted")
			x.Result.Set("interrupt", "halt")
			x.Result.Set("reason", "the bot was halted before this work could be published")
			return "interrupted"
		}
	}
	return status
}

// resultPlan is `self.result.get("plan") if isinstance(self.result.get("plan"), dict) else {}`.
func (x *Deliverer) resultPlan() *pyjson.Object { return x.Result.Obj("plan") }

func (x *Deliverer) hasResultPlan() bool {
	_, ok := x.Result.Get("plan").(*pyjson.Object)
	return ok
}

// Item is Deliverer._item.
func (x *Deliverer) Item() {
	number := py.MustInt(x.Plan.Get("number"))
	x.RatingNote = ""
	action := x.Plan.Get("action")
	if (action == "plan" || action == "build") && x.hasResultPlan() {
		x.RatingNote = x.ApplyRating(number)
	}
	if action == "build" && x.hasResultPlan() {
		// The run planned it first: a later run builds from that plan, never plans again.
		who, tier := x.planner()
		x.remember(number, pyjson.New("planned_at", clock.ISO(x.Ctx.Now()), "planned_by", who,
			"planned_by_id", x.Provider.ID, "planned_tier", tier))
		x.planIntoIssue(number, who)
		if x.RatingNote != "" && x.Result.Get("status") != "planned" {
			x.try(func() { x.GH.CreateComment(number, x.RatingNote) })
		}
	}
	x.deliverItem(number)
	x.keepHandoff(number)
	x.journal(number)
	x.closeAsks(number)
}

func (x *Deliverer) deliverItem(number int) {
	thread := x.GH.GetIssue(number)
	if thread.Get("state") != "open" {
		x.closed(number, thread)
		return
	}
	status := x.lateStop(number, py.Str(x.Result.Get("status")))
	action := x.Plan.Get("action")
	if action == "plan" || (action == "build" && status == "planned") {
		// A build run that rated the item out of its reach stopped after planning it.
		x.planned(number, status)
		return
	}
	if action == "review" {
		x.SecondReview(number, status)
		if status != "infra" && status != "interrupted" && status != "failed" {
			x.settle(number) // a comment left during the review gets its pass
		}
		return
	}
	kind := "build"
	if action == "revise" {
		kind = "revise"
	}
	if status == "infra" {
		if !x.Result.Truthy("no_result") {
			// A login or CLI that broke mid-run (#317 part 1) leaves the work it had, like a
			// pause does: the next run, on another subscription, goes on from it.
			x.keepWork(number, kind)
		}
		x.infra(number, kind, "")
		return
	}
	if status == "interrupted" {
		x.Interrupted(number, kind)
		return
	}
	mode := py.OrStr(x.Plan.Get("mode"), "")
	if status == "split" && (strings.HasPrefix(mode, "split") || mode == "fullsend") {
		x.SplitOut(number)
		x.settle(number)
		return
	}
	if status == "part" {
		if x.Plan.Truthy("onto") {
			x.LandPart(number)
		} else {
			x.Result.Set("reason", "a fullsend part whose tree's branch is not on record")
			x.Fail(number, "build")
		}
		return
	}
	if kind == "revise" {
		x.Revise(number, status)
	} else {
		x.build(number, status)
	}
	x.settle(number)
}

// A planning run's end: its plan is the item's handoff (`_keep_handoff`), and the item
// goes back to the queue to build from it on the cheapest model its difficulty allows.
func (x *Deliverer) planned(number int, status string) {
	if status == "infra" {
		x.infra(number, "build", "")
		return
	}
	if status == "stopped" {
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), "")
		x.GH.CreateComment(number, "Stopped, as asked, before any plan was written.")
		return
	}
	if status == "blocked" && x.Result.Truthy("human_required") {
		// Its plan leaves nothing the bot may build (#793): it goes into the description
		// for the person who builds it, and the item to them.
		who, tier := x.planner()
		x.remember(number, pyjson.New("planned_at", clock.ISO(x.Ctx.Now()), "planned_by", who,
			"planned_by_id", x.Provider.ID, "planned_tier", tier))
		placed := x.planIntoIssue(number, who)
		rated := x.RatingNote
		kept := fmt.Sprintf(" Planned on %s (%s)", who, x.link())
		if placed {
			kept += ", and the plan is in this issue's description, under **Plan**."
		} else {
			kept += "."
		}
		if rated != "" {
			kept = " " + rated + kept
		}
		x.NeedsPerson(number, py.OrStr(x.Result.Get("question"), ""), "build this", kept, "")
		return
	}
	if status != "planned" {
		if status == "interrupted" {
			x.requeueLabel(number, "build")
			handoff := x.Result.Obj("handoff")
			kept := ""
			if strings.TrimSpace(py.OrStr(handoff.Get("draft"), "")) != "" {
				kept = " Its draft so far is kept, and the next planning run goes on from it."
			} else if handoff.Truthy("planning") && strings.TrimSpace(py.OrStr(handoff.Get("trail"), "")) != "" {
				kept = " The end of its session is kept for the next planning run."
			}
			x.GH.CreateComment(number, fmt.Sprintf("Planning was cut short (%s): %s.%s It stays queued.",
				x.link(), py.Str(x.Result.Get("reason")), kept))
			return
		}
		x.Fail(number, "build")
		return
	}
	who, tier := x.planner()
	x.remember(number, pyjson.New("planned_at", clock.ISO(x.Ctx.Now()), "planned_by", who,
		"planned_by_id", x.Provider.ID, "planned_tier", tier))
	x.requeueLabel(number, "build")
	rated := x.RatingNote
	if rated != "" {
		rated += " "
	}
	difficulty := x.Difficulty(number)
	var next string
	if tier != "" && !providers.TierAtLeast(tier, config.PlanFloor[difficulty]) {
		// A medium planner rated it hard: a strong model plans it next (part 6, #695).
		next = fmt.Sprintf("A %s model plans `difficulty:%s` work, so one plans it next; it is queued for that.",
			config.PlanFloor[difficulty], difficulty)
	} else {
		next = fmt.Sprintf("Cleared and queued to build, on the cheapest model `difficulty:%s` allows.", difficulty)
	}
	if x.planIntoIssue(number, who) {
		x.GH.CreateComment(number, fmt.Sprintf("%sPlanned on %s (%s). The plan is in this issue's "+
			"description, under **Plan**: the builder starts from that section, so edit it there to "+
			"change the plan. %s", rated, who, x.link(), next))
		return
	}
	plan := x.resultPlan()
	text := py.SliceTo(redact.Redact(py.OrStr(plan.Get("text"), ""), nil), issueplan.PlanChars)
	x.GH.CreateComment(number, fmt.Sprintf("Planned on %s (%s), but the description could not take "+
		"the plan, so it is here. It is queued to build from this plan, on the cheapest model its "+
		"difficulty allows.\n\n<details><summary>The plan</summary>\n\n%s\n\n</details>", who, x.link(), text))
}

// ApplyRating is Deliverer._apply_rating.
//
// Label the item with the difficulty its planner rated (#317 part 8), and say so: only
// when no person set one (`queue.rating_source`), never lower than the bot's own earlier
// rating, and easy only when the plan's **Files to touch** table holds to the easy rule
// (`easy.plan_breach`, borderline included) and the rating's `why` admits no doubt
// (`easy.uncertain_why`, #695). Records `difficulty_by`. Returns the sentence for the
// comment.
func (x *Deliverer) ApplyRating(number int) string {
	plan := x.resultPlan()
	rated := plan.Obj("rating")
	labels := x.Labels(number)
	record := x.Record(number)
	if queue.RatingSource(labels, record) == "person" {
		return ""
	}
	if rated.Len() == 0 || !hasString(config.Difficulties, py.Str(rated.Get("difficulty"))) {
		// The planner left its rating line out (a weaker model may): the item routes as medium
		// anyway, so say so on the thread instead of leaving it unlabelled. An earlier rating
		// of the bot's own stands, and a later planner may raise this one.
		if queue.RatingSource(labels, record) == "bot" {
			return ""
		}
		who, tier := x.planner()
		x.setDifficulty(number, "medium", labels)
		x.remember(number, pyjson.New("difficulty_by", pyjson.New(
			"difficulty", "medium", "provider", x.Provider.ID, "tier", tier,
			"at", clock.ISO(x.Ctx.Now()), "why", "the planner gave no rating line")))
		return fmt.Sprintf("%s gave no rating, so it is `difficulty:medium`, which is how an unrated "+
			"item is routed anyway.", who)
	}
	wanted := py.Str(rated.Get("difficulty"))
	earlier := py.OrStr(record.Obj("difficulty_by").Get("difficulty"), "")
	if hasString(config.Difficulties, earlier) && indexOf(config.Difficulties, wanted) < indexOf(config.Difficulties, earlier) {
		wanted = earlier // a rating stands: nobody lowers it
	}
	breach := ""
	if wanted == "easy" {
		breach = easy.PlanBreach(py.OrStr(plan.Get("text"), ""), x.Cfg.Easy, x.Cfg.ReviewPaths, x.Cfg.ForbiddenPaths)
		if breach != "" {
			wanted = "medium"
		} else if easy.UncertainWhy(py.OrStr(rated.Get("why"), "")) {
			// Torn between two means the higher (#695): the planner admitted doubt.
			breach = fmt.Sprintf("the rating itself was unsure (%s), so borderline easy is medium",
				py.SliceTo(strings.TrimSpace(py.OrStr(rated.Get("why"), "")), 140))
			wanted = "medium"
		}
	}
	who, tier := x.planner()
	x.setDifficulty(number, wanted, labels)
	x.remember(number, pyjson.New("difficulty_by", pyjson.New(
		"difficulty", wanted, "provider", x.Provider.ID, "tier", tier, "at", clock.ISO(x.Ctx.Now()),
		"why", py.SliceTo(py.OrStr(rated.Get("why"), ""), 300))))
	why := ""
	if rated.Truthy("why") {
		why = ": " + py.Str(rated.Get("why"))
	}
	said := fmt.Sprintf("Rated `difficulty:%s` by %s%s.", py.Str(rated.Get("difficulty")), who, why)
	if breach != "" {
		said += fmt.Sprintf(" The easy rule does not hold (%s), so it is `difficulty:medium`.", breach)
	} else if rated.Get("difficulty") != wanted {
		said += fmt.Sprintf(" It was rated `difficulty:%s` before, and a rating is never lowered.", wanted)
	}
	return said
}

func hasString(xs []string, s string) bool { return indexOf(xs, s) >= 0 }

// indexOf is `xs.index(s)` (-1 where Python raises ValueError).
func indexOf(xs []string, s string) int {
	for i, e := range xs {
		if e == s {
			return i
		}
	}
	return -1
}

// Leave exactly `difficulty:<difficulty>` on the thread, of the difficulty labels. (Python walks
// the set of labels in its hash order, which is a process's own; sorted here.)
func (x *Deliverer) setDifficulty(number int, difficulty string, labels map[string]bool) {
	wanted := "difficulty:" + difficulty
	for _, name := range sortedKeys(labels) {
		if config.DifficultyLabels.Has(strings.ToLower(name)) && name != wanted {
			x.try(func() { x.GH.RemoveLabel(number, name) })
		}
	}
	if !labels[wanted] {
		x.try(func() { x.GH.AddLabels(number, []string{wanted}) })
	}
}

// Who wrote this run's plan, and that model's tier.
func (x *Deliverer) planner() (string, string) {
	plan := x.resultPlan()
	seat := plan.Obj("seat")
	planner := x.Cfg.Pool.Seat(x.Provider.ID, py.OrStr(seat.Get("model"), ""))
	if planner == nil {
		return x.Provider.Describe(), py.OrStr(seat.Get("tier"), "")
	}
	return planner.Describe(), planner.Tier
}

// Put this run's plan into the issue's description, in place of an earlier one, and
// take the item out of the Needs plan stage. False when the description could not take it.
func (x *Deliverer) planIntoIssue(number int, who string) bool {
	plan := x.resultPlan()
	text := strings.TrimSpace(redact.Redact(py.OrStr(plan.Get("text"), ""), nil))
	if text == "" {
		return false
	}
	if _, ok := herr.Catch[*herr.GitHubError](func() {
		thread := x.GH.GetIssue(number)
		x.GH.SetIssueBody(number, issueplan.WithPlan(py.OrStr(thread.Get("body"), ""), text, who, x.link()))
	}); ok {
		return false
	}
	labels := x.Labels(number)
	_, tier := x.planner()
	// A plan built from in the same run stands; one that stopped there must meet its floor.
	planningOnly := x.Plan.Get("action") == "plan" || x.Result.Get("status") == "planned"
	meets := !planningOnly || tier == "" ||
		providers.TierAtLeast(tier, config.PlanFloor[x.Difficulty(number)])
	herr.Catch[*herr.GitHubError](func() {
		if meets {
			if labels[config.LabelNeedsPlan] {
				x.GH.RemoveLabel(number, config.LabelNeedsPlan)
			}
			if !labels[config.LabelPlanned] {
				x.GH.AddLabels(number, []string{config.LabelPlanned})
			}
		} else if !labels[config.LabelNeedsPlan] {
			// A medium plan of a hard item (#317 part 6, #695): a strong one comes next.
			x.GH.AddLabels(number, []string{config.LabelNeedsPlan})
		}
	}) // a GitHubError passes: the next plan job's sync puts the labels right
	return true
}

// Someone closed the issue or pull request while the run worked on it: its work is not
// pushed, no pull request opens, and nothing queues it again.
func (x *Deliverer) closed(number int, thread *pyjson.Object) {
	queue.SetStateLabel(x.Ctx, number, queue.LabelNames(thread), "")
	if thread.Has("pull_request") {
		x.dropWip(number)
	}
	where := "issue"
	if thread.Has("pull_request") {
		where = "pull request"
	}
	x.GH.CreateComment(number, fmt.Sprintf("This %s was closed while a run was working on it "+
		"(%s), so that run's work was not delivered. Reopen it and ask again (`%s build`) to have "+
		"it done.", where, x.link(), config.Slash))
	x.log(fmt.Sprintf("#%d was closed during the run: nothing delivered", number))
}

// Settle the asks this run took (`asks`): 🎉 when it answered them, 😕 when it ended
// without an answer, or back to waiting when the item is queued again.
func (x *Deliverer) closeAsks(number int) {
	if !x.Record(number).Truthy("taken_asks") {
		return
	}
	status := py.Str(x.Result.Get("status"))
	blocked := x.Labels(number)[config.LabelBlocked]
	again := !blocked && (status == "interrupted" || status == "infra" || status == "failed" || status == "planned")
	var settled []string

	change := func(state *pyjson.Object) any {
		entry := statemod.Item(state, number)
		if again {
			asks.GiveBack(entry)
		} else {
			settled = asks.PopTaken(entry)
		}
		return nil
	}

	x.Ctx.Store.Update(change, fmt.Sprintf("asks #%d", number))
	answered := !blocked && (status == "approved" || status == "built" || status == "reviewed" ||
		status == "split" || status == "part")
	content := asks.NoAnswer
	if answered {
		content = asks.Done
	}
	asks.React(x.GH, settled, content)
}

// Keep what a run that did not finish the item leaves for the next agent: its notes
// and the end of its session (`work.py`). A finished item's handoff is dropped.
//
// A planning session cut off before its plan was done leaves its draft (`work.
// PLAN_DRAFT_FILE`) and the end of its session: that handoff is a `draft`, which the next
// planner goes on from and no builder reads. Before this a cut-off plan kept nothing (#37).
func (x *Deliverer) keepHandoff(number int) {
	status := py.Str(x.Result.Get("status"))
	handoff, isDict := x.Result.Get("handoff").(*pyjson.Object)
	finished := status == "approved" || status == "reviewed" || status == "split" || status == "part"
	if !finished && !isDict {
		return
	}
	var kept *pyjson.Object
	if !finished && isDict {
		notes := redact.Redact(py.OrStr(handoff.Get("notes"), ""), nil)
		draft := strings.TrimSpace(redact.Redact(py.OrStr(handoff.Get("draft"), ""), nil))
		kind := "work"
		if status == "planned" {
			kind = "plan"
		} else if draft != "" || handoff.Truthy("planning") {
			kind = "draft"
		}
		var keptNotes string
		if status == "planned" {
			keptNotes = cut(notes, HandoffNotes, true)
		} else {
			keptNotes = journalmod.Compact(notes, HandoffNotes)
		}
		kept = pyjson.New("provider", x.Provider.ID, "family", x.Provider.Family,
			"kind", kind,
			"at", py.OrStr(handoff.Get("at"), ""),
			"reason", py.SliceTo(redact.Redact(py.OrStr(x.Result.Get("reason"), ""), nil), 500),
			"notes", keptNotes,
			"trail", py.SliceFrom(redact.Redact(py.OrStr(handoff.Get("trail"), ""), nil), -HandoffTrail))
		if kind == "draft" && draft != "" {
			kept.Set("draft", cut(draft, issueplan.PlanChars, true))
		}
	}
	change := func(state *pyjson.Object) any {
		entry := statemod.Item(state, number)
		if kept == nil {
			entry.Pop("handoff")
		} else {
			entry.Set("handoff", kept)
		}
		return nil
	}
	x.Ctx.Store.Update(change, fmt.Sprintf("handoff #%d", number))
}

// Append this run to the item's journal on `bot-journal` (#342): what it was, each step
// the work job took and why, how it ended, and the builder's notes. A record, never a gate:
// a failure is logged and the delivery stands.
func (x *Deliverer) journal(number int) {
	key := journalmod.Key(x.Plan)
	if key == 0 {
		key = number
	}
	var seats []string
	for _, s := range []*providers.Seat{x.BuildSeat, x.ReviewSeat} {
		if s != nil {
			seats = append(seats, fmt.Sprintf("`%s` (%s)", s.Model, s.Tier))
		}
	}
	models := strings.Join(uniqStrings(seats), ", ")
	if models == "" {
		models = x.Provider.CLI
	}
	text := journalmod.Section(x.Result, x.Provider.ID, models, py.OrStr(x.Plan.Get("action"), "?"),
		number, clock.ISO(x.Ctx.Now()), x.Cfg.RunURL())
	if x.Cfg.DryRun {
		x.log(fmt.Sprintf("dry run: would add this run to the journal of #%d", key))
		return
	}
	if err := herr.Try(func() {
		journalmod.Append(x.GH, key, text, pySleep)
		x.log(fmt.Sprintf("added this run to the journal of #%d", key))
	}); err != nil { // the delivery already stands
		x.log(fmt.Sprintf("the journal of #%d could not be written: %s", key, redact.Redact(err.Error(), nil)))
	}
}

func (x *Deliverer) journalLink(number int) string {
	key := journalmod.Key(x.Plan)
	if key == 0 {
		key = number
	}
	return fmt.Sprintf("[journal](%s)", journalmod.URL(x.Cfg.ServerURL, x.Cfg.Repo, key))
}

// Labels is Deliverer._labels.
func (x *Deliverer) Labels(number int) map[string]bool {
	return queue.LabelNames(x.GH.GetIssue(number))
}

// Record is Deliverer._record.
func (x *Deliverer) Record(number int) *pyjson.Object {
	return x.Ctx.Store.Load().Obj("items").Obj(strconv.Itoa(number)).Copy()
}

// remember is `_remember(number, **fields)`: `fields` in the keyword arguments' order.
func (x *Deliverer) remember(number int, fields *pyjson.Object) {
	x.Ctx.Store.Update(func(s *pyjson.Object) any {
		statemod.Item(s, number).Update(fields)
		return nil
	}, fmt.Sprintf("deliver #%d", number))
}

func (x *Deliverer) link() string {
	if url := x.Cfg.RunURL(); url != "" {
		return fmt.Sprintf("[run](%s)", url)
	}
	return "the run"
}

// The builder's seat, and every move the run made between its lane's models (each
// Claude account switches between Opus and Sonnet: `work.Worker._switch`).
func (x *Deliverer) builtOn() string {
	var moves []string
	for _, m := range py.Each(x.Result.Get("switches")) {
		move, ok := m.(*pyjson.Object)
		if !ok {
			continue
		}
		when := "before round 1"
		if move.Truthy("n") {
			when = "after round " + py.Str(move.Get("n"))
		}
		moves = append(moves, fmt.Sprintf("%s it switched to `%s`: %s", when, py.Str(move.Get("to")),
			py.Str(move.Get("why"))))
	}
	text := x.BuildSeat.Describe()
	if len(moves) > 0 {
		text += " (" + strings.Join(moves, "; ") + ")"
	}
	return text
}

func (x *Deliverer) requeueLabel(number int, kind string) {
	label := config.LabelBuild
	if kind == "revise" {
		label = config.LabelRevise
	}
	queue.SetStateLabel(x.Ctx, number, x.Labels(number), label)
}

// A run that could not work. Its subscription is left alone for a while
// (`providers.INFRA_BACKOFFS`), so the item goes to another one. When the model job reported
// why (its login, its CLI, an install on untouched main), that is all. When the job left no
// result at all (its runner died or timed out), it may be this item too: it goes to the back
// of the queue, and two in a row block it.
func (x *Deliverer) infra(number int, kind string, requeue string /* = None */) {
	x.Chain = false
	reason := py.OrStr(x.Result.Get("reason"), NoResult)
	now := clock.ISO(x.Ctx.Now())
	label := requeue
	if label == "" {
		label = config.LabelBuild
		if kind == "revise" {
			label = config.LabelRevise
		}
	}
	// A failure that was not the subscription's (an install red on untouched main, a push
	// GitHub refused) backs it off but does not lengthen its streak.
	escalate := !x.Result.Truthy("infra_scope")
	var streak []int
	x.Ctx.Store.Update(func(s *pyjson.Object) any {
		streak = append(streak, providers.NoteInfra(s, x.Provider.ID, reason, x.Ctx.Now(), escalate))
		return nil
	}, "infra "+x.Provider.ID)
	if escalate && len(streak) > 0 {
		x.askAboutInfra(streak[len(streak)-1], reason)
	}
	where := fmt.Sprintf("on `%s`", x.Provider.ID)
	if !x.Result.Truthy("no_result") {
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), label)
		x.GH.CreateComment(number, fmt.Sprintf("The run could not work on this %s (%s): %s\n\nThat is "+
			"not this item's fault; it stays queued, for another subscription meanwhile.", where, x.link(), reason))
		return
	}
	if x.strike(number, "a run died: "+reason) {
		return
	}
	state := x.Ctx.Store.Update(func(s *pyjson.Object) any {
		entry := statemod.Item(s, number)
		entry.Set("died", py.MustInt(statemod.Item(s, number).GetOr("died", 0))+1)
		entry.Set("queued_at", now)
		return nil
	}, fmt.Sprintf("died #%d", number))
	died := py.MustInt(statemod.Item(state, number).GetOr("died", 0))
	if died >= DiedLimit {
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelBlocked)
		x.remember(number, pyjson.New("forced", false))
		x.GH.CreateComment(number, fmt.Sprintf("The run working on this died %d times in a row "+
			"(%s), so I stopped trying: %s. It may be too big or too heavy for one run; `%s build` "+
			"tries again.", died, x.link(), reason, config.Slash))
		return
	}
	queue.SetStateLabel(x.Ctx, number, x.Labels(number), label)
	x.GH.CreateComment(number, fmt.Sprintf("The run working on this died %s (%s): %s. It is back in "+
		"the queue, behind the others.", where, x.link(), reason))
}

// Keep what a run that did not finish made: a build's on its own branch, a revision's
// on `bot/wip/<pr>` (#317 part 3), which the next revision starts from. Before this a
// revision cut off by a usage cap threw its work away: #143 lost 16 Opus runs so.
func (x *Deliverer) keepWork(number int, kind string) (bool, string) {
	if kind == "build" {
		return x.publish(false)
	}
	return x.publishWip(number)
}

// Push an unfinished revision's bundle to `bot/wip/<pr>`, never to the pull request's
// branch. The same checks as `_publish`, conflict markers aside (a resolution may stop
// half-way): the head the result names, descending from where the work started, the pull
// request's branch not moved meanwhile, and no forbidden path. `(pushed, problem)`.
func (x *Deliverer) publishWip(number int) (bool, string) {
	name := x.Result.Get("bundle")
	branch := py.OrStr(x.Plan.Get("branch"), "")
	if !pyjson.Truthy(name) || branch == "" {
		return false, ""
	}
	bundle := outPath(x.OutDir, py.Str(name))
	if !isFile(bundle) {
		return false, "the result names a bundle the model job did not upload"
	}
	wip := queue.WipBranch(number)
	defaultBranch := x.Cfg.DefaultBranch
	for _, ref := range []string{defaultBranch, branch, wip} {
		x.fetch(ref, ref == defaultBranch)
	}
	local := "deliver/" + wip
	var head string
	if exc, ok := herr.Catch[*herr.GitError](func() { head = x.Repo.FetchBundle(bundle, branch, local) }); ok {
		return false, "the bundle could not be read: " + exc.Error()
	}
	start := py.OrStr(x.Result.Get("start"), "")
	if head != x.Result.Get("head") || start == "" || x.isAncestor(start, head) != 0 {
		return false, "the bundle does not hold the work the result names"
	}
	remote := x.Repo.Rev("origin/" + branch)
	if remote != "" && remote != start {
		return false, fmt.Sprintf("`%s` moved on GitHub while I worked, so the work was not kept", branch)
	}
	mergedMain := x.Repo.MergeBase("origin/"+defaultBranch, local)
	if len(x.Repo.Unsanctioned(local, []string{start, mergedMain}, x.Cfg.ForbiddenPaths)) > 0 {
		return false, "the work touches paths the bot may not change, so it was not kept"
	}
	if x.Cfg.DryRun {
		x.log(fmt.Sprintf("dry run: would push %s to %s", head, wip))
		return true, ""
	}
	url := x.remoteURL()
	old := x.Repo.Rev("origin/" + wip)
	if exc, ok := herr.Catch[*herr.GitError](func() {
		if old != "" && x.isAncestor(old, head) != 0 {
			// The run started over from the pull request's head: the old work goes.
			x.Repo.Push(url, ":refs/heads/"+wip, x.Cfg.WriteToken(), "")
		}
		x.Repo.Push(url, local+":refs/heads/"+wip, x.Cfg.WriteToken(), "")
	}); ok {
		return false, fmt.Sprintf("keeping the work on `%s` was refused: %s", wip, exc.Error())
	}
	x.remember(number, pyjson.New("wip", pyjson.New("sha", head, "start", start, "at", clock.ISO(x.Ctx.Now()),
		"provider", x.Provider.ID)))
	x.log(fmt.Sprintf("kept the unfinished revision of #%d on %s", number, wip))
	return true, ""
}

// A revision delivered, or the pull request closed: its unfinished work is done with.
func (x *Deliverer) dropWip(number int) {
	if !x.Record(number).Truthy("wip") {
		return
	}
	url := x.remoteURL()
	herr.Catch[*herr.GitError](func() {
		if !x.Cfg.DryRun {
			x.Repo.Push(url, ":refs/heads/"+queue.WipBranch(number), x.Cfg.WriteToken(), "")
		}
	}) // a GitError passes: gone already
	x.Ctx.Store.Update(func(s *pyjson.Object) any {
		statemod.Item(s, number).Pop("wip")
		return nil
	}, fmt.Sprintf("wip #%d done", number))
}

// Whether a cut-off run's kept work moved on: a commit of its own, not a merge of
// `main`, past `before` (the head the last cut-off run kept, `bot/wip/<pr>`), or past where
// this run started when it did not build on that. A revision that resumes from its wip
// pushes it again every time, so "pushed" alone is no progress.
func (x *Deliverer) progressed(number int, kind string, before string) bool {
	head := py.OrStr(x.Result.Get("head"), "")
	if head == "" {
		return false
	}
	for _, base := range []string{before, py.OrStr(x.Result.Get("start"), "")} {
		if base == "" || x.isAncestor(base, head) != 0 {
			continue
		}
		// Not main's: a merge of `main` brings its squash commits in, and they are no merges.
		main := "origin/" + x.Cfg.DefaultBranch
		args := []string{"rev-list", "--no-merges", "--count", head, "^" + base}
		if x.Repo.Rev(main) != "" {
			args = append(args, "^"+main)
		}
		found := x.Repo.Run(false, 600, nil, args...)
		out := found.Stdout
		if out == "" {
			out = "0"
		}
		out = strings.TrimSpace(out)
		return found.Returncode == 0 && out != "" && out != "0"
	}
	return false
}

// Interrupted is Deliverer._interrupted.
func (x *Deliverer) Interrupted(number int, kind string) {
	interrupt := py.OrStr(x.Result.Get("interrupt"), "budget")
	reason := x.Result.Get("reason")
	var kept any
	if kind == "revise" {
		kept = x.Record(number).Get("wip")
	}
	before := ""
	if k, ok := kept.(*pyjson.Object); ok {
		before = py.OrStr(k.Get("sha"), "")
	}
	pushed, problem := x.keepWork(number, kind)
	moved := pushed && x.progressed(number, kind, before)
	calls, _ := py.Int(py.Or(x.Result.Get("model_calls"), 0))
	// A cut-off run that made progress (its work moved on) is a step of a long job, not a
	// failure; one that made none counts, whatever cut it off, except a halt, the machine's
	// disk, or a run stopped before any model call (#317 part 3). Before this only the time
	// budget counted, so a usage pause could repeat for ever.
	if (interrupt == "budget" || interrupt == "usage" || interrupt == "died") && calls != 0 {
		count := func(s *pyjson.Object) any {
			entry := statemod.Item(s, number)
			if moved {
				entry.Set("interruptions", 0)
			} else {
				entry.Set("interruptions", py.MustInt(entry.GetOr("interruptions", 0))+1)
			}
			return nil
		}
		state := x.Ctx.Store.Update(count, fmt.Sprintf("interrupted #%d", number))
		countNow := py.MustInt(statemod.Item(state, number).GetOr("interruptions", 0))
		if countNow >= x.Cfg.MaxFailures {
			queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelBlocked)
			x.remember(number, pyjson.New("forced", false))
			x.GH.CreateComment(number, fmt.Sprintf("This was cut off %d runs in a row "+
				"without its work moving on (%s; last: %s), so I stopped. It may be too big for one "+
				"run: split it into smaller issues, or queue it again to keep going.",
				countNow, x.link(), py.Str(reason)))
			return
		}
	}
	x.requeueLabel(number, kind)
	rounds, roundTier := x.rounds()
	x.remember(number, pyjson.New("last_findings", py.Or(x.Result.Get("findings"), []any{}),
		"verdict_rounds", rounds, "round_tier", roundTier))
	where := "on the branch"
	if kind == "revise" {
		where = fmt.Sprintf("on `%s`, where the next revision starts", queue.WipBranch(number))
	}
	keptText := ""
	if pushed {
		keptText = fmt.Sprintf(" The work so far is %s.", where)
	}
	tail := ""
	if problem != "" {
		tail = "\n\n" + problem
	}
	x.GH.CreateComment(number, fmt.Sprintf("Paused: %s.%s It stays queued and the next "+
		"run picks it up from its notes and its %s.", py.Str(reason), keptText, x.journalLink(number))+tail)
}

// Some builder session of the run worked: it ended well, or it changed something.
func (x *Deliverer) builderWorked() bool {
	cycles := x.Result.Get("cycles")
	if !pyjson.Truthy(cycles) {
		return true // a result from before rounds were recorded, or a test's
	}
	for _, e := range py.Each(cycles) {
		c, ok := e.(*pyjson.Object)
		if !ok {
			continue
		}
		if c.Obj("builder").Truthy("ok") || c.Obj("builder").Truthy("changed") {
			return true
		}
	}
	return false
}

// A weak builder's change on an easy item that breaks the easy rule (`easy.change_breach`:
// too many files or lines, or an off-limits path) is not pushed (#317 part 8): its work is
// kept for a stronger builder (a build's on its own branch, with no pull request; a
// revision's on `bot/wip/<pr>`), the item becomes medium (its label, or a floor under a
// person's label), and it is queued again. True when it did so.
func (x *Deliverer) easyBreach(number int, kind string) bool {
	if x.BuildSeat == nil || x.BuildSeat.Tier != "weak" {
		return false
	}
	if x.Difficulty(number) != "easy" {
		return false
	}
	counts := x.bundleCounts()
	breach := ""
	if counts.Len() > 0 {
		breach = easy.ChangeBreach(counts, x.Cfg.Easy)
	}
	if breach == "" {
		return false
	}
	var pushed bool
	if kind == "build" {
		pushed, _ = x.publish(false)
	} else {
		pushed, _ = x.publishWip(number)
	}
	issue, pr := stepup.Pair(x.Ctx, number)
	targets := []int{issue}
	if pr != nil && *pr != 0 {
		targets = setOrder(issue, *pr) // `{issue, pr}`, in the order Python's set walks it
	}
	for _, target := range targets {
		labels := x.Labels(target)
		if queue.RatingSource(labels, x.Record(target)) == "person" {
			x.remember(target, pyjson.New("difficulty_floor", "medium"))
		} else {
			x.setDifficulty(target, "medium", labels)
			x.remember(target, pyjson.New("difficulty_by", pyjson.New(
				"difficulty", "medium", "provider", "easy rule", "tier", "",
				"at", clock.ISO(x.Ctx.Now()), "why", breach)))
		}
		if pr != nil && target == *pr {
			x.remember(target, pyjson.New("difficulty", "medium"))
		}
	}
	x.requeueLabel(number, kind)
	kept := ""
	if pushed {
		if kind == "build" {
			kept = fmt.Sprintf(" Its work is kept on `%s`", py.Str(x.Plan.Get("branch")))
		} else {
			kept = fmt.Sprintf(" Its work is kept on `%s`", queue.WipBranch(number))
		}
		kept += ", for it to go on from."
	}
	x.GH.CreateComment(number, fmt.Sprintf("Not pushed (%s): %s built it as `difficulty:easy`, but "+
		"the change breaks the easy rule (%s), so it is `difficulty:medium` now and a medium or "+
		"stronger model takes it.%s", x.link(), x.BuildSeat.Describe(), breach, kept))
	return true
}

// Lines added plus removed per path of the bundle's head against `main`, or, for a
// fullsend part (#505), against its tree's branch: the parts before it are not its change.
func (x *Deliverer) bundleCounts() *pyjson.Object {
	name := x.Result.Get("bundle")
	branch := py.OrStr(x.Plan.Get("branch"), "")
	if !pyjson.Truthy(name) || branch == "" || !isFile(outPath(x.OutDir, py.Str(name))) {
		return pyjson.New()
	}
	base := py.OrStr(x.Plan.Get("onto"), x.Cfg.DefaultBranch)
	var counts *pyjson.Object
	if _, ok := herr.Catch[*herr.GitError](func() {
		x.fetch(base, true)
		local := "deliver/measure/" + branch
		x.Repo.FetchBundle(outPath(x.OutDir, py.Str(name)), branch, local)
		counts = x.Repo.Numstat("origin/"+base, local)
	}); ok {
		return pyjson.New()
	}
	return counts
}

// One failure of the item's own (`stepup.strike`): True when it was its third, and the
// item was stepped up (or blocked at hard) instead of ending the way the caller would.
func (x *Deliverer) strike(number int, why string) bool {
	var struck bool
	if exc, ok := herr.Catch[*herr.GitHubError](func() {
		struck = stepup.Strike(x.Ctx, number, why, x.link())
	}); ok {
		x.log(fmt.Sprintf("could not count a strike on #%d: %s", number, exc.Error()))
		return false
	}
	return struck
}

// Fail is Deliverer._fail: a run that produced nothing usable: requeue it, or block it after
// too many.
func (x *Deliverer) Fail(number int, kind string) {
	if x.strike(number, "the run failed: "+py.OrStr(x.Result.Get("reason"), "unknown")) {
		return
	}
	state := x.Ctx.Store.Update(func(s *pyjson.Object) any {
		statemod.Item(s, number).Set("failures", py.MustInt(statemod.Item(s, number).GetOr("failures", 0))+1)
		return nil
	}, fmt.Sprintf("failure #%d", number))
	fails := py.MustInt(statemod.Item(state, number).GetOr("failures", 0))
	reason := py.OrStr(x.Result.Get("reason"), "unknown")
	if fails >= x.Cfg.MaxFailures {
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelBlocked)
		x.remember(number, pyjson.New("forced", false))
		x.cameToNothing(number, fmt.Sprintf("it failed %d times in a row: %s", fails, reason))
		x.GH.CreateComment(number, fmt.Sprintf("This failed %d times in a row "+
			"(%s), so I stopped trying. Last error: %s"+
			"\n\nFix the cause, then `%s build` (or `revise`) "+
			"queues it again.", fails, x.link(), reason, config.Slash))
	} else {
		x.requeueLabel(number, kind)
		rounds, roundTier := x.rounds()
		x.remember(number, pyjson.New("verdict_rounds", rounds, "round_tier", roundTier))
		x.GH.CreateComment(number, fmt.Sprintf("This run failed (%s): %s\n\nIt "+
			"is back in the queue (attempt %d of %d).", x.link(), reason, fails, x.Cfg.MaxFailures))
	}
}

// NeedsPerson is Deliverer._needs_person.
//
// Only a person can do it (#793, `human.py`): the bot hands the thread over, as a
// person's `human` would (`triage.hand_to_people`). Its own labels come off
// (`human.LETS_GO`), it unassigns itself and assigns the people, `human required` goes on,
// and a comment names them and says what they must do. The queue leaves it alone from then
// on (`queue.is_human`).
func (x *Deliverer) NeedsPerson(number int, question string, what string /* = "build this" */, kept string /* = "" */, again string /* = "" */) {
	thread := x.GH.GetIssue(number)
	handover := &triage.Plan{}
	triage.HandToPeople(handover, thread, x.Cfg.BotLogin)
	remove := []string{}
	for _, name := range sortedKeys(queue.LabelNames(thread)) {
		if human.LetsGo[name] {
			remove = append(remove, name)
		}
	}
	handover.Remove = remove
	for _, done := range triage.Apply(x.GH, number, handover) {
		x.log(fmt.Sprintf("#%d: %s", number, done))
	}
	// Its colour and description first: a label GitHub makes as it is put on is grey and
	// bare (#796's was), and the sweep only creates missing ones (`sweep._labels`).
	x.try(func() {
		spec := pyjson.List(py.Item(config.Labels, config.LabelHumanRequired))
		color, description := py.Str(spec[0]), py.Str(spec[1])
		x.GH.SyncLabel(config.LabelHumanRequired, color, description)
	})
	if !queue.LabelNames(thread)[config.LabelHumanRequired] {
		x.GH.AddLabels(number, []string{config.LabelHumanRequired})
	}
	x.GH.CreateComment(number, human.Ask(question, x.Cfg.BotLogin, what, kept, again))
	// The next run reads it with the thread, once a person has queued the item again.
	x.remember(number, pyjson.New("question", question))
	x.cameToNothing(number, "the bot could not build it: "+question)
	x.log(fmt.Sprintf("#%d needs a person: handed to people, %s", number, config.LabelHumanRequired))
}

// One of my suggestions, approved, that I could not finish (#797): logged, so no survey
// offers it again (`declined`). A thread that was never a suggestion is left out.
func (x *Deliverer) cameToNothing(number int, why string) {
	if !x.Record(number).Truthy("suggestion") {
		return
	}
	title := py.OrStr(x.GH.GetIssue(number).Get("title"), "")
	x.Ctx.Store.Update(func(s *pyjson.Object) any {
		declined.Note(s, number, title, why, clock.ISO(x.Ctx.Now()))
		return nil
	}, fmt.Sprintf("suggestion #%d came to nothing", number))
}

// After a finished run: clear the one-run flags, and go round again for a comment that
// arrived while the run held the thread (after a stop, one that came after it).
func (x *Deliverer) settle(number int) {
	status := py.Str(x.Result.Get("status"))
	if status == "failed" {
		return
	}
	seen := pyjson.New()

	change := func(state *pyjson.Object) any {
		entry := statemod.Item(state, number)
		seen.Update(entry)
		entry.Update(pyjson.New("forced", false, "interruptions", 0, "died", 0, "pending_request", false))
		return nil
	}

	x.Ctx.Store.Update(change, fmt.Sprintf("settle #%d", number))
	record := seen
	// `/harness stop` clears any earlier pending request, so one still set came after it.
	if !record.Truthy("pending_request") {
		return
	}
	if queue.IsHuman(x.Labels(number)) {
		return // people's now (`human`, or `human required`, #793): a person queues it again
	}
	target := number
	pr := 0
	if status == "approved" {
		pr = py.MustInt(py.Or(record.Get("pr"), 0))
	}
	if x.Plan.Get("action") == "build" && pr != 0 {
		target = pr
		// The asks that came in during the build are answered on the pull request now.
		x.Ctx.Store.Update(func(s *pyjson.Object) any {
			asks.Add(statemod.Item(s, pr), asks.PopWaiting(statemod.Item(s, number)))
			return nil
		}, fmt.Sprintf("asks #%d to #%d", number, pr))
	}
	isPR := x.GH.GetIssue(target).Has("pull_request")
	if isPR {
		queue.HoldMerge(x.Ctx, x.GH.GetPull(target))
	}
	label, kind := config.LabelBuild, "build"
	if isPR {
		label, kind = config.LabelRevise, "revise"
	}
	queue.SetStateLabel(x.Ctx, target, x.Labels(target), label)
	x.remember(target, pyjson.New("kind", kind, "queued_at", clock.ISO(x.Ctx.Now()),
		"stop_requested", false, "source", "request"))
	x.GH.CreateComment(target, "A comment arrived while I was working on this, so it is "+
		"queued again to answer it.")
}

func (x *Deliverer) build(number int, status string) {
	if status == "failed" {
		x.Fail(number, "build")
		return
	}
	if status == "nothing" {
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), "")
		return
	}
	branch := py.Str(x.Plan.Get("branch"))
	if status == "stopped" {
		pushed, _ := x.publish(false)
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), "")
		kept := ""
		if pushed {
			kept = fmt.Sprintf(" What I had is on `%s`.", branch)
		}
		x.GH.CreateComment(number, "Stopped, as asked."+kept)
		return
	}
	if (status == "approved" || status == "built" || status == "not_approved") && !x.builderWorked() {
		// #141 was opened by a run whose builder failed with a 401 in 0.0 minutes, from
		// commits earlier runs had left on the branch (#317 part 4).
		x.Result.Set("reason", "no builder session in the run worked: "+
			py.OrStr(x.Result.Get("reason"), "no reason recorded"))
		x.Fail(number, "build")
		return
	}
	if (status == "approved" || status == "built") && x.easyBreach(number, "build") {
		return
	}
	approved := status == "approved"
	built := status == "built" // no model in the run could review it: a review run will
	pushed, problem := x.publish(approved)
	if strings.HasPrefix(problem, "the push was refused") {
		x.Result.Set("status", "infra")
		x.Result.Set("reason", problem)
		x.Result.Set("no_result", true)
		x.Result.Set("infra_scope", "github")
		x.infra(number, "build", "")
		return
	}
	if problem != "" {
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelBlocked)
		x.GH.CreateComment(number, fmt.Sprintf("I could not publish this run's work (%s): "+
			"%s. It needs a person to look.", x.link(), problem))
		return
	}
	started := x.Record(number).Get("started_at")
	if status == "blocked" {
		question := py.OrStr(py.Or(x.Result.Get("question"), x.Result.Get("reason")), "")
		kept := ""
		if pushed {
			kept = fmt.Sprintf(" What I had is on `%s`.", branch)
		}
		if x.Result.Truthy("human_required") {
			x.NeedsPerson(number, question, "build this", kept, "")
			x.remember(number, pyjson.New("last_findings", []any{}))
			return
		}
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelBlocked)
		x.GH.CreateComment(number, fmt.Sprintf("I need a decision before I can build this.%s\n\n"+
			"%s\n\nAnswer here, then `%s build` (or "+
			"`@%s <your answer>`) queues it again.", kept, question, config.Slash, x.Cfg.BotLogin))
		x.remember(number, pyjson.New("last_findings", []any{}, "question", question))
		return
	}
	if !(x.Repo.Rev("origin/"+branch) != "" || pushed) {
		x.Fail(number, "build")
		return
	}
	body := x.pullBody(number, approved)
	// A title the title check passes (#187): the builder's, else the issue's.
	title := triage.PullTitle(py.OrStr(x.Result.Get("title"), ""), py.OrStr(x.Plan.Get("title"), ""))
	if title == "" {
		title = fmt.Sprintf("Build #%d", number)
	}
	pull := x.openPull()
	if pull == nil {
		pull = x.createPull(title, branch, body, !(approved || built))
	} else {
		x.GH.UpdatePull(py.MustInt(pull.Get("number")), pyjson.New("title", title, "body", body))
		if (approved || built) && pull.Truthy("draft") {
			x.try(func() { x.GH.MarkReady(py.Str(pull.Get("node_id"))) })
		}
	}
	pr := py.MustInt(py.Or(pull.Get("number"), 0))
	if pr != 0 {
		x.GH.AddLabels(pr, append([]string{config.LabelPR}, x.Carried(number)...))
		x.remember(pr, pyjson.New("issue", number, "feedback_since", started, "failures", 0, "kind", "revise",
			"difficulty", x.Difficulty(number),
			"self_check_findings", py.Or(x.Result.Get("self_check_findings"), []any{})))
	}
	issueLabels := x.Labels(number)
	if approved || built {
		queue.SetStateLabel(x.Ctx, number, issueLabels, "")
		if !issueLabels[config.LabelPROpen] {
			x.GH.AddLabels(number, []string{config.LabelPROpen})
		}
		asked := pr != 0 && x.Labels(pr)[config.LabelRevise]
		if pr != 0 && !asked {
			queue.SetStateLabel(x.Ctx, pr, x.Labels(pr), "")
		}
		mergeNote := ""
		if asked {
			mergeNote = "A revision was asked for on it while I built, so it stays queued " +
				"and auto-merge waits for it."
		} else if pr != 0 && x.Record(number).Truthy("pending_request") {
			mergeNote = "A comment arrived during the run, so auto-merge waits for the " +
				"revision that answers it."
		}
		if pr != 0 {
			head := py.OrStr(x.Result.Get("head"), "")
			var rule string
			if approved {
				rule = x.approved(pr, head, mergeNote == "", true, true, nil)
			} else {
				rule = x.awaitReview(pr, head, mergeNote == "")
			}
			if mergeNote == "" {
				mergeNote = rule
			}
		}
		cycles := pyLen(py.Or(x.Result.Get("cycles"), []any{}))
		var said string
		if approved {
			said = fmt.Sprintf("Its adversarial reviewer, %s, approved it on round %d of %d.",
				x.ReviewSeat.Describe(), cycles, x.CyclesCap())
		} else {
			said = x.selfCheckSaid()
		}
		if approved {
			x.unstick(number, pr)
		}
		x.GH.CreateComment(number, fmt.Sprintf("Opened #%d, built on %s. %s %s", pr, x.builtOn(), said, mergeNote))
		rounds, roundTier := x.rounds()
		x.remember(number, pyjson.New("last_findings", []any{}, "question", "", "failures", 0, "pr", pr,
			"verdict_rounds", rounds, "round_tier", roundTier))
	} else {
		first := x.firstFinding()
		if first == "" {
			first = py.Str(x.Result.Get("reason"))
		}
		if x.strike(number, "its reviewer did not approve it: "+first) {
			return
		}
		queue.SetStateLabel(x.Ctx, number, issueLabels, config.LabelBlocked)
		if pr != 0 && !x.Labels(pr)[config.LabelRevise] {
			queue.SetStateLabel(x.Ctx, pr, x.Labels(pr), config.LabelBlocked)
		}
		stuck := x.markStuck(number, pr)
		x.GH.CreateComment(number, fmt.Sprintf("%s, so #%d stays a draft "+
			"and will not merge by itself (%s).\n\n"+
			"%s\n\n%s\n\n%s"+
			"Comment `@%s <guidance>` here or on #%d "+
			"to have me try again with your notes.", x.notApprovedHead(), pr, x.link(),
			x.findingsMd(), x.whyMd(), stuck, x.Cfg.BotLogin, pr))
		rounds, roundTier := x.rounds()
		x.remember(number, pyjson.New("last_findings", py.Or(x.Result.Get("findings"), []any{}), "pr", pr,
			"verdict_rounds", rounds, "round_tier", roundTier))
	}
}
