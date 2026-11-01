package plan

import (
	"fmt"
	"slices"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
)

// Peek is what the gate learns: whether a run would find work, and whether it must first wait
// for the shared subscription to be quiet (`quiet_provider`), with `fallback` when other work
// could go ahead without it.
type Peek struct {
	Work          bool
	Reason        string
	Forced        bool   // default: False
	Provider      string // default: ""
	QuietProvider string // default: ""
	QuietSecret   string // default: ""
	Fallback      bool   // default: False
	Held          int    // default: 0
	// Queued items passed over because of `human` (#96), one line each.
	Skipped []string // default: field(default_factory=list)
	// A lane is free and the queue holds nothing it can start (#797): the sweep keeps the clock
	// a survey waits on from this (`note_idle`).
	Idle bool // default: False
}

// Peek_ is plan.peek: whether a run would find work, why, and with which subscription. Changes
// nothing.
//
// The gate asks this before it spends anything on reading the subscription's usage.
func Peek_(ctx *hctx.Context, force bool /* = False */, item *int /* = None */, mode string /* = "auto" */) *Peek {
	skipped := []string{}
	seen := map[string]bool{}
	look := peek(ctx, force, item, mode, &skipped, seen)
	look.Skipped = skipped
	look.Idle = seen["idle"]
	return look
}

// peek is plan._peek.
func peek(ctx *hctx.Context, force bool, item *int, mode string, skipped *[]string, seen map[string]bool) *Peek {
	if !slices.Contains(Modes, mode) {
		return &Peek{Work: false, Reason: "unknown mode " + py.Repr(mode)}
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
		return &Peek{Work: false, Reason: stop}
	}
	tidy := HousekeepingDue(ctx, state)
	lanes := ReadLanes(ctx, state)
	if !lanes.AnyRoom() {
		if tidy != "" {
			return &Peek{Work: true, Reason: tidy, Forced: force, Held: len(lanes.Held)}
		}
		return &Peek{Work: false, Reason: fmt.Sprintf("every lane is busy (%s)", lanes.Describe()),
			Held: len(lanes.Held)}
	}
	q := queueOf(ctx, state, item, mode, skipped)
	// found: (candidate or None, provider, forced)
	type work struct {
		candidate *queue.Candidate
		provider  *providers.Provider
		forced    bool
	}
	var found *work
	var chosen *Assignment
	if mode != "suggest" {
		if top := Pairs(ctx, state, q, lanes, force, AnyQuiet, *skipped); len(top) > 0 {
			chosen = top[0].Assignment
			found = &work{top[0].Candidate, top[0].Assignment.Provider, force || top[0].Candidate.Forced}
		}
	}
	seen["idle"] = found == nil && item == nil && mode == "auto"
	// A survey once a free lane has had nothing to start for `suggestions.idle_minutes` (#797),
	// or when one is asked for: queued work that waits for another subscription, or for an issue
	// it depends on, no longer keeps an idle one from surveying.
	if found == nil && item == nil && (mode == "suggest" || mode == "auto") &&
		SuggestionsDue(ctx, state, mode == "suggest") != 0 {
		if provider := SurveyProvider(ctx, state, lanes, force, AnyQuiet); provider != nil {
			found = &work{nil, provider, force}
		}
	}
	if found == nil {
		if tidy != "" {
			return &Peek{Work: true, Reason: tidy, Forced: force, Held: len(lanes.Held)}
		}
		if item != nil && mode != "suggest" {
			return &Peek{Work: false, Reason: fmt.Sprintf("#%d is not queued, or no subscription can "+
				"take it now: %s", *item, WhyNone(ctx, state, lanes)), Held: len(lanes.Held)}
		}
		if len(q) > 0 {
			return &Peek{Work: false, Reason: "no subscription can take the queue now: " +
				WhyNone(ctx, state, lanes), Held: len(lanes.Held)}
		}
		return &Peek{Work: false, Reason: "nothing is queued", Held: len(lanes.Held)}
	}
	candidate, provider, forced := found.candidate, found.provider, found.forced
	what := "a suggestion survey is due"
	if candidate != nil {
		what = fmt.Sprintf("#%d (%s) is queued to %s", candidate.Number, candidate.Difficulty, candidate.Kind)
	}
	reason := fmt.Sprintf("%s, for `%s`", what, provider.ID)
	if chosen != nil {
		reason += ": " + chosen.Describe()
	}
	if !(provider.QuietCheck && ctx.Cfg.Quiet.Enabled && !forced) {
		return &Peek{Work: true, Reason: reason, Forced: forced, Provider: provider.ID, Held: len(lanes.Held)}
	}
	// The best subscription for it is the shared one: wait for quiet, unless another run is
	// already waiting, and say whether other work could go ahead without it.
	var other *Pair
	if mode != "suggest" {
		if next := Pairs(ctx, state, q, lanes, force, "", *skipped); len(next) > 0 {
			other = &next[0]
		}
	}
	fallback := other != nil
	if other != nil && other.Assignment.Provider.Family == provider.Family {
		// Another account of the same model can take work now: no reason to wait for this one.
		return &Peek{Work: true, Reason: fmt.Sprintf("#%d (%s) is queued to %s, for `%s`: %s",
			other.Candidate.Number, other.Candidate.Difficulty, other.Candidate.Kind,
			other.Assignment.Provider.ID, other.Assignment.Describe()),
			Forced: force || other.Candidate.Forced, Provider: other.Assignment.Provider.ID,
			Held: len(lanes.Held)}
	}
	if QuietWaiting(ctx) {
		if fallback {
			return &Peek{Work: true, Reason: fmt.Sprintf("#%d (%s) is queued to %s, for `%s` (another "+
				"run waits for `%s`)", other.Candidate.Number, other.Candidate.Difficulty,
				other.Candidate.Kind, other.Assignment.Provider.ID, provider.ID),
				Forced: forced, Provider: other.Assignment.Provider.ID, Held: len(lanes.Held)}
		}
		return &Peek{Work: false, Reason: fmt.Sprintf("another run already waits for `%s` to be quiet",
			provider.ID), Held: len(lanes.Held)}
	}
	return &Peek{Work: true, Reason: reason, Forced: forced, Provider: provider.ID,
		QuietProvider: provider.ID, QuietSecret: provider.Secret, Fallback: fallback,
		Held: len(lanes.Held)}
}

// RunRequest is a `/harness run` (or `start --force`) not yet taken up, if one is fresh enough.
func RunRequest(ctx *hctx.Context, state *pyjson.Object) *pyjson.Object {
	asked := state.Obj("run_requested")
	at := clock.ParseISOValue(asked.Get("at"))
	if asked == nil || at.IsZero() || ctx.Now().Sub(at) > RunRequestTtl {
		return nil
	}
	return asked
}

// AnnounceSwitchedOff is, on the day a subscription's `off_from` comes, open one issue asking a
// person what it should do now (the bot cannot change `.harness/`), and remember that it did.
func AnnounceSwitchedOff(ctx *hctx.Context, state *pyjson.Object) []string {
	notes := []string{}
	cfg := ctx.Cfg
	for _, provider := range cfg.Pool.Ordered() {
		if !provider.Enabled || !providers.SwitchedOffByDate(provider, ctx.Now(), cfg.Timezone) {
			continue
		}
		if providers.PeekRecord(state, provider.ID).Truthy("off_announced") {
			continue
		}
		reason := ""
		if provider.OffReason != "" {
			reason = "\n\n" + provider.OffReason
		}
		offFrom := provider.OffFrom.Format("2006-01-02") // str(date)
		issue := ctx.GH.CreateIssue(
			fmt.Sprintf("Night bot: `%s` is switched off from %s; decide what it should do now",
				provider.ID, offFrom),
			fmt.Sprintf("`.harness/providers.json` gives `%s` (%s, `%s`) `off_from: %s`, so from that "+
				"day the night bot starts no new work on it.%s\n\nTo decide, in a pull request a "+
				"person makes (the bot cannot change `.harness/`):\n- keep it off: remove its "+
				"entry, or set `\"enabled\": false`;\n- point it at another model, or give it caps "+
				"under `limits`;\n- or move `off_from` later.",
				provider.ID, provider.CLI, provider.Model, offFrom, reason),
			[]string{"night bot"})
		number := py.MustInt(py.Or(issue.Get("number"), 0))
		providerID := provider.ID
		ctx.Store.Update(func(s *pyjson.Object) any {
			providers.Record(s, providerID).Set("off_announced", number)
			return nil
		}, provider.ID+" switched off")
		notes = append(notes, fmt.Sprintf("`%s` is switched off from %s: opened #%d", provider.ID, offFrom, number))
	}
	return notes
}

// HousekeepingDue is what `housekeeping` would requeue, read without changing anything, or ""
// (None).
func HousekeepingDue(ctx *hctx.Context, state *pyjson.Object) string {
	for _, thread := range WorkingThreads(ctx) {
		runID := recordOf(state, py.Str(thread.Get("number"))).Get("run_id")
		if RunStatus(ctx, runID) == "dead" {
			return fmt.Sprintf("#%s was left working by a run that ended", py.Str(thread.Get("number")))
		}
	}
	for _, thread := range ctx.GH.ListIssues(config.LabelPR, "open", 300, "") {
		names := queue.LabelNames(thread)
		if !thread.Has("pull_request") || names[config.LabelRevise] || names[config.LabelWorking] ||
			names[config.LabelBlocked] || queue.IsHuman(names) {
			continue
		}
		record := recordOf(state, py.Str(thread.Get("number")))
		if record.Truthy("stop_requested") {
			continue
		}
		if eqStr(ctx.GH.GetPull(py.MustInt(thread.Get("number"))).Get("mergeable_state"), "dirty") {
			return fmt.Sprintf("#%s conflicts with main", py.Str(thread.Get("number")))
		}
	}
	return ""
}
