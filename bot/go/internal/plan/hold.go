package plan

// The owner, and the bounded hold (#2748, section 1).
//
// An item's owner is the subscription that planned it, or took it up last: a claim records it on
// the item's record (`owner`, `owner_since`, `owner_why`; a bot pull request's on its issue's,
// `queue.OwnerItem`), and a record from before owners were kept reads its `planned_by_id`
// (`queue.OwnerOf`). A final audit that asks for changes makes the auditor's subscription the
// owner of its fix, and the last approver owns the merge, so a CI fix goes to it (`Own`; #2748,
// section 4). A build, a continuation and a revision go to the owner first (`BuilderSeat`); a
// review run never does, and a fullsend part keeps its own routing. When the owner cannot take
// its item now:
//
//   - its 5-hour window alone keeps it (`providers.FiveHourWait`: over or within `start_headroom`
//     of its 5-hour cap in force, or refused on that window) and it can start again within
//     `hold_minutes` of when the hold began (its reset, or its session's last hour where
//     `last_hour` lifts the cap, whichever comes first): the item is held (`Owning`). No other
//     subscription takes it, it carries `bot:waiting`, and one comment of the bot's on its thread
//     says for whom, until when and why, and how to release it (`/harness handoff …`);
//   - anything else (a weekly cap or refusal, its hours, a suspension, switched off, its lanes all
//     busy, a start further off, a subscription in `hold_exempt`, an item queued with `--force`,
//     which is to start now): it is handed off at once, and whoever takes it owns it from then on
//     (`Claim`).
//
// The planner and the sweep keep the holds (`SyncHolds`): one lifts as soon as its owner can start,
// and the owner takes it at its next run, ahead of new work (`Pairs`); one whose owner cannot start
// by its end, or whose owner's state stops holding it, is released to anyone. A person releases
// one, or any item, at any time (`Handoff`).

import (
	"fmt"
	"slices"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/human"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	hstate "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/triage"
)

// HandoffTo is what `/harness handoff` takes after the verb: nothing (release it to anyone), a
// difficulty to set as the person's label, or `human`.
var HandoffTo = []string{"medium", "hard", "human"}

// Hold is an item kept for its owner.
type Hold struct {
	Owner string
	// Since is when the hold began, and Ends `hold_minutes` after it: if the owner still cannot
	// start by then, the item goes to anyone.
	Since time.Time
	Ends  time.Time
	// Until is when the owner can start again (`providers.FiveHourWait`).
	Until time.Time
	// Why is what keeps the owner now (its `providers.Refusal`).
	Why string
}

// Ownership is a queued item's owner, now.
type Ownership struct {
	// Owner is "" when it has none, and for a review run or a fullsend part, which go by rules of
	// their own.
	Owner string
	// Free: the owner may take it now.
	Free bool
	// Hold: it waits for its owner.
	Hold *Hold
	// Gone is why it leaves its owner now: it is handed off at once ("" while it stays).
	Gone string
}

// Owning is who owns `c` and whether it waits for them: free, held, or handed off at once (the
// package's rules above). It changes nothing.
func Owning(ctx *hctx.Context, state *pyjson.Object, c *queue.Candidate, lanes *Lanes, force bool, quietOK string) *Ownership {
	cfg := ctx.Cfg
	if c.Owner == "" || (c.Kind == "review" && !c.OwnReview) || (c.Kind == "build" && c.Part && !c.SentBack) {
		return &Ownership{} // a part its reconcile sent back is its builder's, as owned work is
	}
	if Reserves(c) {
		// Its own review run (#2748, 2026-11-02) takes the lane its build reserved for it, a build
		// the lane its planning run kept for it (#2903), and a turn's conflict revision the lane kept
		// for it once its turn came (#2948).
		lanes = lanes.Without(c.Number)
	}
	own := &Ownership{Owner: c.Owner}
	provider := cfg.Pool.Get(c.Owner)
	if provider == nil {
		own.Gone = fmt.Sprintf("its owner `%s` is not a subscription of providers.json any more", c.Owner)
		return own
	}
	if why := cannotTake(cfg.Pool, provider, c); why != "" {
		own.Gone = why
		return own
	}
	role := RoleFor(c)
	forced := force || c.Forced
	if queue.NeedsPlan(c) && slices.ContainsFunc(LanePlanners(ctx, state, lanes, forced, quietOK),
		func(seat *providers.Seat) bool {
			return seat.Provider.ID == provider.ID && providers.TierAtLeast(seat.Tier, queue.PlanFloor(c))
		}) {
		own.Free = true // it may plan it now, on the planning lane
		return own
	}
	if Usable(ctx, state, provider, lanes, role, forced, quietOK) {
		own.Free = true
		return own
	}
	if busy := busyNow(ctx, state, provider, lanes, role); busy != "" {
		own.Gone = fmt.Sprintf("`%s` %s", provider.ID, busy)
		return own
	}
	now := ctx.Now()
	reason := providers.Availability(provider, state, now, cfg.Timezone, cfg.Secrets, forced, true)
	if reason == "" {
		// Free but for the quiet check (claude-1, shared with its owner), which no reset lifts.
		own.Gone = fmt.Sprintf("`%s` waits for its owner to be quiet", provider.ID)
		return own
	}
	own.Gone = fmt.Sprintf("`%s` %s", provider.ID, reason)
	if c.Forced || !cfg.HoldsFor(provider.ID) {
		return own // queued to start now (`--force`), or an owner no item waits for
	}
	until, why := providers.FiveHourWait(provider, state, now, cfg.Timezone, cfg.Secrets)
	if until.IsZero() {
		return own // what keeps it is no 5-hour window: a weekly cap or refusal, its hours, …
	}
	since := now
	if begun := clock.ParseISO(c.HoldSince); !begun.IsZero() && !begun.After(now) {
		since = begun
	}
	ends := since.Add(time.Duration(cfg.HoldMinutes) * time.Minute)
	if until.After(ends) {
		if since.Equal(now) {
			own.Gone += fmt.Sprintf("; it can start again only in %s, more than the %d minutes an item "+
				"waits for its owner", clock.HumanDelta(until.Sub(now)), cfg.HoldMinutes)
		} else {
			own.Gone += fmt.Sprintf("; the %d minutes this waited for it end at %s, before it can start "+
				"again (in %s)", cfg.HoldMinutes, Clock(ctx, ends), clock.HumanDelta(until.Sub(now)))
		}
		return own
	}
	own.Gone = ""
	own.Hold = &Hold{Owner: provider.ID, Since: since, Ends: ends, Until: until, Why: shortWhy(why)}
	return own
}

// cannotTake is why `provider` could not take `c` even when free, or "": no role for it, a
// `build_last` subscription (claude-2), which builds after every other and so keeps nothing, no
// seat of the tier it needs (its difficulty's, or, for the auditor that owns its fix, the tier of
// the audit it answers), or no reviewer of its own for a revision that needs one in its run.
func cannotTake(pool *providers.Pool, provider *providers.Provider, c *queue.Candidate) string {
	floor := config.MinTier[config.AsDifficulty(c.Difficulty)]
	if c.OwnReview {
		// Its own review run: the lane that built it reviews it and fixes what it finds, so it needs
		// both roles and a seat at the item's tier; `build_last` keeps it too, as it built it.
		for _, role := range []string{"review", "revise"} {
			if !slices.Contains(provider.Roles, role) {
				return fmt.Sprintf("its builder `%s` takes no %s", provider.ID, role)
			}
		}
		if pool.BestSeat(provider, floor) == nil {
			return fmt.Sprintf("its builder `%s` has no %s seat to review it on", provider.ID, floor)
		}
		return ""
	}
	if c.Kind == "revise" && rankOr(c.AuditTier, -1) > rankOr(floor, -1) {
		floor = c.AuditTier
	}
	if c.Kind == "build" && c.Part && queue.PlanMeets(c) && pool.BestSeat(provider, floor) == nil {
		floor = config.PartFloor // a part's builder may be a tier under it (`BuilderSeat`)
	}
	switch {
	case !slices.Contains(provider.Roles, RoleOf[c.Kind]):
		return fmt.Sprintf("its owner `%s` takes no %s", provider.ID, RoleOf[c.Kind])
	case provider.BuildLast:
		return fmt.Sprintf("its owner `%s` builds only after every other subscription (`build_last`)", provider.ID)
	case pool.BestSeat(provider, floor) == nil:
		return fmt.Sprintf("its owner `%s` has no %s seat, which it needs", provider.ID, floor)
	case c.Kind == "revise" && (!c.BotPR || c.Conflict) && RunReviewer(pool, provider, c.Difficulty) == nil:
		return fmt.Sprintf("its owner `%s` has no reviewer of its own, which this revision needs in its run", provider.ID)
	}
	return ""
}

// busyNow is why `provider` has no room for a run now, its usage, hours and quiet check aside
// (`Usable`'s lane checks), or "": a subscription whose lanes are all taken is not waited for.
func busyNow(ctx *hctx.Context, state *pyjson.Object, provider *providers.Provider, lanes *Lanes, role string) string {
	cfg := ctx.Cfg
	switch {
	case !lanes.RoomFor(provider.ID):
		return "has no runner free where it runs"
	case MachineFull(cfg.Pool, provider, lanes):
		return fmt.Sprintf("waits for room on the machine (%d at once)", cfg.Pool.MachineParallel)
	case FillInFull(cfg.Pool, provider, lanes):
		return "waits for its lane on its box, which another subscription fills now"
	case RunnerFor(ctx, state, provider, lanes, role) == "":
		return "is busy: its lanes are all taken"
	}
	return ""
}

// shortWhy is a refusal without the reset it names, which the hold gives as its own time.
func shortWhy(why string) string {
	for _, tail := range []string{"; it resets at ", "; its limit resets at "} {
		if head, _, found := strings.Cut(why, tail); found {
			return head
		}
	}
	return why
}

// Clock is a time on the bot's clock: `20:27` today, `Fri 18:47` on another day.
func Clock(ctx *hctx.Context, at time.Time) string {
	zone := clock.Zone(ctx.Cfg.Timezone)
	local := at.In(zone)
	today := ctx.Now().In(zone)
	ly, lm, ld := local.Date()
	ty, tm, td := today.Date()
	if ly == ty && lm == tm && ld == td {
		return local.Format("15:04")
	}
	return local.Format("Mon 15:04")
}

// HoldLine is a held item's line in the status issue, `status` and the queue's waiting: "held for
// `claude-3` until 22:40 (5-hour usage is 97%, at or over its 70% cap)".
func HoldLine(ctx *hctx.Context, h *Hold) string {
	return fmt.Sprintf("held for `%s` until %s (%s)", h.Owner, Clock(ctx, h.Until), h.Why)
}

// RecordedHold is the hold a thread's record keeps (`hold`, as `SyncHolds` last wrote it), or nil
// for none, or one lifted.
func RecordedHold(record *pyjson.Object) *Hold {
	kept := record.Obj("hold")
	if kept == nil || kept.Truthy("lifted") || !kept.Truthy("owner") {
		return nil
	}
	return &Hold{Owner: kept.Str("owner"), Since: clock.ParseISOValue(kept.Get("since")),
		Ends: clock.ParseISOValue(kept.Get("ends")), Until: clock.ParseISOValue(kept.Get("until")),
		Why: kept.Str("why")}
}

// Holds is the queued items held for their owners now, by number: each one `Owning` holds, and
// each still labelled `bot:waiting` whose owner cannot start yet, as its record last said (until
// the next plan or sweep releases it, `SyncHolds`).
func Holds(ctx *hctx.Context, state *pyjson.Object, q []*queue.Candidate, lanes *Lanes, force bool, quietOK string) map[int]*Hold {
	held := map[int]*Hold{}
	for _, c := range q {
		own := Owning(ctx, state, c, lanes, force, quietOK)
		if own.Hold != nil {
			held[c.Number] = own.Hold
		} else if c.Waiting && !own.Free {
			if kept := RecordedHold(recordOf(state, strconv.Itoa(c.Number))); kept != nil {
				held[c.Number] = kept
			}
		}
	}
	return held
}

// HeldNote is "#12 held for `claude-3` until 22:40 (…); …" for the items `held` names, or "".
func HeldNote(ctx *hctx.Context, held map[int]*Hold) string {
	numbers := make([]int, 0, len(held))
	for n := range held {
		numbers = append(numbers, n)
	}
	sort.Ints(numbers)
	parts := make([]string, 0, len(numbers))
	for _, n := range numbers {
		parts = append(parts, fmt.Sprintf("#%d %s", n, HoldLine(ctx, held[n])))
	}
	return strings.Join(parts, "; ")
}

// SyncHolds keeps the holds on the queue (`Owning`), as the planner and the sweep find them: a
// held item carries `bot:waiting`, its record's `hold` and one comment of the bot's saying so; a
// hold whose owner can start now is lifted (the owner takes it next, `Pairs`); one that ran out,
// or whose owner can no longer be waited for, is released to anyone. Returns what it did.
func SyncHolds(ctx *hctx.Context, state *pyjson.Object, lanes *Lanes, force bool, quietOK string) []string {
	notes := []string{}
	if !ctx.Cfg.Holds() && !anyHold(state) && len(queue.OwnReviewsWaiting(state)) == 0 {
		return notes
	}
	var q []*queue.Candidate
	if exc, failed := herr.Catch[*herr.GitHubError](func() { q = queue.Candidates(ctx, state, nil, nil) }); failed {
		return []string{"could not keep the holds: " + exc.Error()}
	}
	for _, c := range q {
		if c.Kind == "review" && !c.OwnReview {
			continue
		}
		record := recordOf(state, strconv.Itoa(c.Number))
		kept := record.Obj("hold")
		active := kept != nil && !kept.Truthy("lifted")
		own := Owning(ctx, state, c, lanes, force, quietOK)
		var note string
		if exc := herr.Try(func() {
			switch {
			case own.Hold != nil:
				note = holdOn(ctx, c, own.Hold, kept, active)
			case c.OwnReview && !own.Free:
				// Its own review run cannot start, and nothing holds it for its lane: it goes to its
				// final audit on another subscription instead (#2748, 2026-11-02).
				why := own.Gone
				if why == "" {
					why = fmt.Sprintf("its builder `%s` cannot take it", c.Owner)
				}
				note = HandOffOwnReview(ctx, c.Number, why, c.Difficulty)
			case c.Waiting || active:
				note = holdOff(ctx, c, own)
			}
		}); exc != nil {
			note = fmt.Sprintf("#%d: could not keep its hold (%s)", c.Number, py.SliceTo(herr.Str(exc), 200))
		}
		if note != "" {
			notes = append(notes, note)
		}
	}
	return notes
}

// anyHold is: some item's record holds a hold (one a config that holds no more must still lift).
func anyHold(state *pyjson.Object) bool {
	items := state.Obj("items")
	for _, key := range items.Keys() {
		if items.Obj(key).Truthy("hold") {
			return true
		}
	}
	return false
}

// holdOn puts and keeps `c` on hold for its owner: `bot:waiting`, its record's `hold`, and its one
// comment, written when the hold begins and rewritten when it changes.
func holdOn(ctx *hctx.Context, c *queue.Candidate, h *Hold, kept *pyjson.Object, active bool) string {
	number := c.Number
	queue.SetWaiting(ctx, number, map[string]bool{config.LabelWaiting: c.Waiting}, true)
	entry := pyjson.New("owner", h.Owner, "since", clock.ISO(h.Since), "until", clock.ISO(h.Until),
		"ends", clock.ISO(h.Ends), "why", h.Why)
	same := active && kept.Str("owner") == h.Owner && kept.Str("until") == entry.Str("until") &&
		kept.Str("ends") == entry.Str("ends")
	if !same {
		ctx.Store.Update(func(s *pyjson.Object) any {
			hstate.Item(s, number).Set("hold", entry)
			return nil
		}, fmt.Sprintf("hold #%d for %s", number, h.Owner))
		writeHoldComment(ctx, number, heldText(ctx, c, h), true)
	}
	return fmt.Sprintf("#%d is %s", number, HoldLine(ctx, h))
}

// holdOff ends `c`'s hold: lifted for an owner that can start now, else released to anyone.
func holdOff(ctx *hctx.Context, c *queue.Candidate, own *Ownership) string {
	number := c.Number
	queue.SetWaiting(ctx, number, map[string]bool{config.LabelWaiting: c.Waiting}, false)
	now := ctx.Now()
	if own.Free {
		ctx.Store.Update(func(s *pyjson.Object) any {
			record := hstate.Item(s, number)
			if hold := record.Obj("hold"); hold != nil {
				hold.Set("lifted", clock.ISO(now))
			} else {
				record.Set("hold", pyjson.New("owner", own.Owner, "lifted", clock.ISO(now)))
			}
			return nil
		}, fmt.Sprintf("hold #%d lifted", number))
		writeHoldComment(ctx, number, fmt.Sprintf("▶️ **No longer held**: `%s`, which owns this, can "+
			"start again, so it takes it up at its next run, ahead of new work.", own.Owner), false)
		return fmt.Sprintf("#%d: `%s` can start again, so its hold is lifted", number, own.Owner)
	}
	why := own.Gone
	if why == "" {
		why = "it has no owner any more"
	}
	if c.OwnReview {
		return HandOffOwnReview(ctx, number, why, c.Difficulty)
	}
	ctx.Store.Update(func(s *pyjson.Object) any {
		Release(s, number, "released from its hold: "+why, now)
		return nil
	}, fmt.Sprintf("hold #%d released", number))
	writeHoldComment(ctx, number, fmt.Sprintf("▶️ **Released to anyone**: %s. The next subscription that "+
		"may build it at difficulty:%s takes it, and owns it from then on.", why, c.Difficulty), false)
	return fmt.Sprintf("#%d is released to anyone: %s", number, why)
}

// heldText is the hold's comment: for whom, until when and why, and the commands that release it.
func heldText(ctx *hctx.Context, c *queue.Candidate, h *Hold) string {
	slash := config.Slash
	now := ctx.Now()
	if c.OwnReview {
		return fmt.Sprintf("⏸️ **Its own review is held for `%s`**, the subscription whose run built "+
			"it, until about %s (in %s), as `%s` cannot start before then: %s. Its lane reviews it, "+
			"fixes what it finds and approves it, so no other subscription takes it meanwhile. If `%s` "+
			"still cannot start by %s (`hold_minutes`: %d), it goes to its final audit on another "+
			"subscription instead.\n\n"+
			"To release it now (level 2):\n"+
			"- `%s handoff`: to its final audit, on any subscription that may review it at "+
			"difficulty:%s;\n"+
			"- `%s handoff human`: to people: labelled `%s`, my labels off, and I unassign myself.",
			h.Owner, Clock(ctx, h.Until), clock.HumanDelta(h.Until.Sub(now)), h.Owner, h.Why, h.Owner,
			Clock(ctx, h.Ends), ctx.Cfg.HoldMinutes, slash, c.Difficulty, slash, config.LabelHuman)
	}
	return fmt.Sprintf("⏸️ **Held for `%s`**, the subscription that owns this, until about %s (in %s), "+
		"as `%s` cannot start before then: %s. No other subscription takes it meanwhile, so `%s` goes "+
		"on from where its work stands. If `%s` still cannot start by %s (`hold_minutes`: %d), anyone "+
		"may take it.\n\n"+
		"To release it now (level 2):\n"+
		"- `%s handoff`: to any subscription that may build it at difficulty:%s, which then owns it;\n"+
		"- `%s handoff medium` or `%s handoff hard`: the same, at that difficulty, as your label, which "+
		"I never change;\n"+
		"- `%s handoff human`: to people: labelled `%s`, my labels off, and I unassign myself.",
		h.Owner, Clock(ctx, h.Until), clock.HumanDelta(h.Until.Sub(now)), h.Owner, h.Why, h.Owner,
		h.Owner, Clock(ctx, h.Ends), ctx.Cfg.HoldMinutes, slash, c.Difficulty, slash, slash, slash,
		config.LabelHuman)
}

// writeHoldComment writes the thread's one hold comment (its record's `waiting_comment`): edits
// it, or, with `create` (a hold beginning), makes it when there is none yet or it is gone.
func writeHoldComment(ctx *hctx.Context, number int, body string, create bool) {
	id := py.MustInt(py.Or(recordOf(ctx.Store.Load(), strconv.Itoa(number)).Get("waiting_comment"), 0))
	if id != 0 {
		if _, failed := herr.Catch[*herr.GitHubError](func() { ctx.GH.UpdateComment(id, body) }); !failed {
			return
		}
	}
	if !create {
		return
	}
	made := ctx.GH.CreateComment(number, body)
	if made := py.MustInt(py.Or(made.Get("id"), 0)); made != 0 {
		ctx.Store.Update(func(s *pyjson.Object) any {
			hstate.Item(s, number).Set("waiting_comment", made)
			return nil
		}, fmt.Sprintf("hold comment #%d", number))
	}
}

// Release clears the owner of thread `number`'s item (`queue.OwnerItem`) and any hold on the
// thread, any lane its planner kept for its build (#2903), and any lane kept for its turn (#2948):
// whoever takes it up next owns it (`Claim`). `why` goes on record as `owner_why`.
func Release(s *pyjson.Object, number int, why string, at time.Time) {
	hstate.Item(s, queue.OwnerItem(s, number)).Update(pyjson.New("owner", "",
		"owner_since", clock.ISO(at), "owner_why", py.SliceTo(why, 300)))
	hstate.Item(s, number).Pop("hold")
	hstate.Item(s, number).Pop(queue.BuildNextKey)
	hstate.Item(s, number).Pop(queue.TurnKey)
}

// Reserves is: `c` takes a lane kept for it: its own review run's (#2748), the build its planning run
// kept a lane for (#2903), or the conflict revision of a pull request whose turn came (#2948).
func Reserves(c *queue.Candidate) bool {
	return c.OwnReview || (c.Kind == "build" && c.BuildNext) || (c.Kind == "revise" && c.TurnNext)
}

// Own makes `providerID` the owner of thread `number`'s item (`queue.OwnerItem`), with `why`, and
// ends any hold on the thread: an auditor whose final audit asked for changes owns the fix
// (`deliver.SecondReview`), and the last approver owns the merge, so a red check or the merge
// queue's failure comes back to it (`events.fixCI`; #2748, section 4). Its revision then goes to
// it first, as its own work (`Pairs`), held for it a while (`Owning`) or handed off as any
// owner's.
func Own(s *pyjson.Object, number int, providerID string, why string, at time.Time) {
	hstate.Item(s, queue.OwnerItem(s, number)).Update(pyjson.New("owner", providerID,
		"owner_since", clock.ISO(at), "owner_why", py.SliceTo(why, 300)))
	hstate.Item(s, number).Pop("hold")
	hstate.Item(s, number).Pop(queue.BuildNextKey)
}

// takeOver records `providerID`, whose run takes thread `number` up, as its item's owner, unless
// it is already: with why, from the assignment's `OwnerWhy` when the item changed hands.
func takeOver(s *pyjson.Object, number int, providerID string, assignment *Assignment, at time.Time) {
	key := queue.OwnerItem(s, number)
	record := recordOf(s, strconv.Itoa(key))
	_, kept := record.Lookup("owner")
	was := queue.OwnerOf(s, number)
	if kept && was == providerID {
		return
	}
	var why string
	switch {
	case was == providerID:
		why = "it planned it" // a record from before owners were kept
	case was != "":
		why = fmt.Sprintf("handed off from `%s`: %s", was, py.OrStr(assignment.OwnerWhy, "it could not take it"))
	case assignment.Action == "plan":
		why = "it plans it"
	default:
		why = "it took it up"
	}
	if was == "" && kept && record.Truthy("owner_why") {
		why += "; " + record.Str("owner_why") // released: by a person, or from its hold
	}
	hstate.Item(s, key).Update(pyjson.New("owner", providerID, "owner_since", clock.ISO(at),
		"owner_why", py.SliceTo(why, 300)))
}

// Handoff is `/harness handoff [medium|hard|human]` on thread `number` (#2748, section 1), from a
// person of level 2: the item leaves its owner and any hold, and the next subscription that takes
// it owns it; with `medium` or `hard`, at that difficulty, the person's label; with `human`, to
// people, as a person's `human` label does (`triage.HandToPeople`). On a bot pull request it is
// its issue's item that changes hands (`queue.OwnerItem`). A run holding the thread keeps it: the
// handoff waits on its record (`handoff_requested`) and deliver applies it when that run delivers
// (`DeliverHandoff`). Returns the reply.
func Handoff(ctx *hctx.Context, number int, by string, args string) string {
	words := py.Split(py.Lower(args))
	to := ""
	if len(words) > 0 {
		to = strings.Trim(words[0], "`.,!")
	}
	if to != "" && !slices.Contains(HandoffTo, to) {
		return fmt.Sprintf("`handoff` takes `medium`, `hard` or `human`, or nothing, not `%s`. Nothing "+
			"was done.", to)
	}
	thread := ctx.GH.GetIssue(number)
	if !eqStr(thread.Get("state"), "open") {
		return fmt.Sprintf("#%d is closed, so there is nothing to hand off.", number)
	}
	names := queue.LabelNames(thread)
	if queue.IsHuman(names) {
		return queue.HumanReply(number, names)
	}
	if other := queue.Owner(thread); other != nil {
		return queue.OwnerReply(number, other)
	}
	if record := recordOf(ctx.Store.Load(), strconv.Itoa(number)); queue.OwnReviewWaits(record) && to != "human" {
		// Its own review run has not started (#2748): released from its lane, it goes to its final
		// audit, on another subscription that may review it first.
		from := queue.OwnReviewOf(record).Str("provider")
		if to != "" {
			setPersonsDifficulty(ctx, number, names, to)
		}
		difficulty := queue.DifficultyOf(queue.LabelNames(ctx.GH.GetIssue(number)), queue.CarriedDifficulty(record))
		HandOffOwnReview(ctx, number, "released by @"+by, difficulty)
		return fmt.Sprintf("Released from `%s`: its own review goes to its final audit instead (`%s`), on "+
			"another subscription that may review it at difficulty:%s first, whose approval ships it.",
			from, config.LabelCross, difficulty)
	}
	if queue.OwnReviewWaits(recordOf(ctx.Store.Load(), strconv.Itoa(number))) {
		ctx.Store.Update(func(s *pyjson.Object) any {
			hstate.Item(s, number).Pop(queue.OwnReviewKey) // to people: no run of the bot's reviews it
			return nil
		}, fmt.Sprintf("own review #%d dropped", number))
		queue.SetStateLabel(ctx, number, names, "")
		names = queue.LabelNames(ctx.GH.GetIssue(number))
	}
	if live := queue.LiveLabelOn(names); live != "" {
		ctx.Store.Update(func(s *pyjson.Object) any {
			hstate.Item(s, number).Set("handoff_requested", pyjson.New("by", by, "to", to,
				"at", clock.ISO(ctx.Now())))
			return nil
		}, fmt.Sprintf("handoff #%d asked", number))
		return fmt.Sprintf("A run holds #%d now (`%s`), so it keeps it until it delivers; then I %s.",
			number, live, handoffWords(to))
	}
	return applyHandoff(ctx, number, by, to)
}

// handoffWords is what a handoff to `to` does, after "I".
func handoffWords(to string) string {
	switch to {
	case "human":
		return "hand it to people (`" + config.LabelHuman + "`)"
	case "":
		return "release it to anyone"
	}
	return fmt.Sprintf("release it to anyone at `difficulty:%s`", to)
}

// DeliverHandoff applies a handoff a person asked for while the run that just delivered held
// thread `number` (`Handoff`), and returns what it said ("" when none was asked).
func DeliverHandoff(ctx *hctx.Context, number int) string {
	asked := recordOf(ctx.Store.Load(), strconv.Itoa(number)).Obj("handoff_requested")
	if asked == nil {
		return ""
	}
	if thread := ctx.GH.GetIssue(number); !eqStr(thread.Get("state"), "open") {
		ctx.Store.Update(func(s *pyjson.Object) any {
			hstate.Item(s, number).Pop("handoff_requested")
			return nil
		}, fmt.Sprintf("handoff #%d dropped", number))
		return ""
	}
	by := asked.Str("by")
	said := applyHandoff(ctx, number, by, asked.Str("to"))
	ctx.GH.CreateComment(number, fmt.Sprintf("@%s The run that held this has delivered, so: %s", by, said))
	return said
}

// applyHandoff is the handoff itself, on a thread no run holds; its reply.
func applyHandoff(ctx *hctx.Context, number int, by string, to string) string {
	state := ctx.Store.Load()
	owner := queue.OwnerOf(state, number)
	thread := ctx.GH.GetIssue(number)
	names := queue.LabelNames(thread)
	key := queue.OwnerItem(state, number)
	now := ctx.Now()
	ctx.Store.Update(func(s *pyjson.Object) any {
		Release(s, number, "released by @"+by, now)
		hstate.Item(s, number).Pop("handoff_requested")
		return nil
	}, fmt.Sprintf("handoff #%d", number))
	queue.SetWaiting(ctx, number, names, false)
	from := "No subscription owned it"
	if owner != "" {
		from = fmt.Sprintf("Released from `%s`", owner)
	}
	var said string
	switch to {
	case "human":
		for _, done := range handToPeople(ctx, number) {
			if strings.HasPrefix(done, "could not") {
				said += " I " + done + "."
			}
		}
		said = fmt.Sprintf("%s and handed to people: labelled `%s`, my labels are off and I unassigned "+
			"myself, so I do not touch it while that label is on. Take it off and `%s build` (or "+
			"`revise`) to hand it back.%s", from, config.LabelHuman, config.Slash, said)
	case "medium", "hard":
		setPersonsDifficulty(ctx, number, names, to)
		if key != number {
			setPersonsDifficulty(ctx, key, queue.LabelNames(ctx.GH.GetIssue(key)), to)
		}
		said = fmt.Sprintf("%s; it is `difficulty:%s` now, your label, which I never change: the next "+
			"subscription that may build it at that difficulty takes it, and owns it from then on.", from, to)
	default:
		said = fmt.Sprintf("%s: the next subscription that may build it at difficulty:%s takes it, and "+
			"owns it from then on.", from, queue.DifficultyOf(names, queue.CarriedDifficulty(recordOf(state,
			strconv.Itoa(number)))))
	}
	if to != "human" && !names[config.LabelBuild] && !names[config.LabelRevise] {
		said += fmt.Sprintf(" It is not queued now: `%s build` (or `revise`) queues it.", config.Slash)
	}
	writeHoldComment(ctx, number, fmt.Sprintf("▶️ **Handed off** by @%s: %s", by, said), false)
	return said
}

// setPersonsDifficulty leaves exactly `difficulty:<difficulty>` of the difficulty labels on thread
// `number`, as a person's label (`queue.RatingSource`): the bot's own rating and any floor or
// carried difficulty on its record give way to it, so the bot never changes it.
func setPersonsDifficulty(ctx *hctx.Context, number int, names map[string]bool, difficulty string) {
	wanted := "difficulty:" + difficulty
	labels := make([]string, 0, len(names))
	for name := range names {
		labels = append(labels, name)
	}
	sort.Strings(labels)
	for _, name := range labels {
		if _, ok := config.LabelDifficulty(py.Lower(name)); ok && name != wanted {
			ctx.GH.RemoveLabel(number, name)
		}
	}
	if !names[wanted] {
		ctx.GH.AddLabels(number, []string{wanted})
	}
	ctx.Store.Update(func(s *pyjson.Object) any {
		record := hstate.Item(s, number)
		record.Pop("difficulty_by")
		record.Pop("difficulty_floor")
		if record.Has("difficulty") {
			record.Set("difficulty", difficulty)
		}
		return nil
	}, fmt.Sprintf("difficulty #%d by a person", number))
}

// handToPeople is a person's `human` on thread `number`, as triage applies it (`triage.HandToPeople`:
// both people assigned, the bot unassigned), with `human` put on and every label that says the bot
// has it taken off (`human.LetsGo`, `bot:waiting` among them). What it did, step by step.
func handToPeople(ctx *hctx.Context, number int) []string {
	thread := ctx.GH.GetIssue(number)
	names := queue.LabelNames(thread)
	handover := &triage.Plan{}
	triage.HandToPeople(handover, thread, ctx.Cfg.BotLogin)
	labels := make([]string, 0, len(names))
	for name := range names {
		labels = append(labels, name)
	}
	sort.Strings(labels)
	handover.Remove = []string{}
	for _, name := range labels {
		if human.LetsGo[name] {
			handover.Remove = append(handover.Remove, name)
		}
	}
	if !queue.IsHuman(names) {
		handover.Labels = []string{config.LabelHuman}
	}
	return triage.Apply(ctx.GH, number, handover)
}

// RoleFor is the role a subscription must allow to take `c` (`RoleOf`), and whose start rule holds
// for it: an own review run reviews and fixes what it finds, so it starts as a revision does, only
// `start_headroom` under each cap.
func RoleFor(c *queue.Candidate) string {
	if c.OwnReview {
		return "revise"
	}
	return RoleOf[c.Kind]
}

// HandOffOwnReview hands thread `number`'s own review run (#2748, 2026-11-02) off to its final audit:
// its builder's subscription cannot take it (`why`), and no hold keeps it for that one. The
// reservation goes, the pull request is marked ready (CI ran on its draft) and queued for a review
// run (`bot:cross-review`), whose reviewer goes on another subscription of the item's tier first
// (`ReviewSeat`) and whose approval ships it, and the hold's comment, or a new one, says so. The
// item's owner is released, as a hold that runs out releases it. Returns what it did.
func HandOffOwnReview(ctx *hctx.Context, number int, why string, difficulty string) string {
	thread := ctx.GH.GetIssue(number)
	names := queue.LabelNames(thread)
	now := ctx.Now()
	ctx.Store.Update(func(s *pyjson.Object) any {
		record := hstate.Item(s, number)
		record.Pop(queue.OwnReviewKey)
		record.Set("queued_at", clock.ISO(now))
		Release(s, number, "its own review was handed off to its final audit: "+why, now)
		return nil
	}, fmt.Sprintf("own review #%d handed off", number))
	queue.SetStateLabel(ctx, number, names, config.LabelCross)
	herr.Catch[*herr.GitHubError](func() {
		if pull := ctx.GH.GetPull(number); pull.Truthy("draft") && pull.Truthy("node_id") {
			ctx.GH.MarkReady(py.Str(pull.Get("node_id")))
		}
	})
	body := fmt.Sprintf("▶️ **Its own review is handed off**: %s. It goes to its final audit instead "+
		"(`%s`): a review run on another subscription that may review difficulty:%s first, whose "+
		"approval ships it.", why, config.LabelCross, difficulty)
	if py.MustInt(py.Or(recordOf(ctx.Store.Load(), strconv.Itoa(number)).Get("waiting_comment"), 0)) != 0 {
		writeHoldComment(ctx, number, body, false)
	} else {
		ctx.GH.CreateComment(number, body)
	}
	return fmt.Sprintf("#%d: its own review goes to its final audit: %s", number, why)
}
