package plan

import (
	"fmt"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/asks"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/declined"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/prompts"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
)

// OpenSuggestionCount is plan.open_suggestion_count.
func OpenSuggestionCount(ctx *hctx.Context) int {
	count := 0
	for _, issue := range ctx.GH.ListIssues(config.LabelSuggestion, "open", 300, "") {
		if !issue.Has("pull_request") {
			count++
		}
	}
	return count
}

// SuggestionsDue is how many suggestions a survey may open now: 0 when none is due or the cap
// is full. One is due when asked for, or while the excess compute stands at
// `suggestions.spare_lanes` or more (`spare`, `SpareTotal`, #3659) and has for
// `suggestions.idle_minutes` (#797: `suggest.idle_since`, kept by `note_idle`).
//
// Several run at once (#4026), one an area of `suggestions.areas`: one more starts while an area
// is free, and the room under `suggestions.max_open` that the running surveys have not claimed is
// shared out among the surveys the spare lanes and the free areas let start now, so the first of
// them does not take it all. A forced one (`/harness suggest --force`) starts whatever runs.
func SuggestionsDue(ctx *hctx.Context, state *pyjson.Object, force bool /* = False */, spare int) int {
	cfg := ctx.Cfg
	if !cfg.SuggestionsEnabled {
		return 0
	}
	suggest := state.Obj("suggest")
	surveys := Surveys(state)
	if len(surveys) >= MaxSurveys(cfg) && !force {
		return 0 // every area has its survey (a dead one's claim is cleared by `housekeeping`)
	}
	idle := clock.ParseISOValue(suggest.Get("idle_since"))
	due := force || suggest.Truthy("requested") ||
		(spare >= cfg.SuggestionsSpareLanes && !idle.IsZero() &&
			ctx.Now().Sub(idle) >= time.Duration(cfg.SuggestionsIdleMinutes)*time.Minute)
	if !due {
		return 0
	}
	room := cfg.SuggestionsMaxOpen - OpenSuggestionCount(ctx)
	if !force {
		// A forced one passes over the running surveys' shares: each delivery still opens no more
		// than `suggestions.max_open` allows when it ends.
		room -= reservedSuggestions(cfg, surveys)
	}
	if room <= 0 {
		return 0
	}
	starting := max(1, min(spare, MaxSurveys(cfg)-len(surveys)))
	return (room + starting - 1) / starting
}

// NoteIdle keeps the clock a survey waits on (#797): `suggest.idle_since` is when the excess
// compute last reached `suggestions.spare_lanes`, the queue holding nothing a free lane could start
// (`Peek.idle`, #3659), and goes when it drops under that again. The sweep calls this every tick,
// writing only when it changes; a survey claimed restarts it (`suggestion_plan`). Bookkeeping, so
// it reports nothing.
func NoteIdle(ctx *hctx.Context, idle bool) {
	if !ctx.Cfg.SuggestionsEnabled {
		return
	}
	since := ctx.Store.Load().Obj("suggest").Get("idle_since")
	if idle == pyjson.Truthy(since) {
		return
	}
	var stamp any
	if idle {
		stamp = clock.ISO(ctx.Now())
	}
	ctx.Store.Update(func(s *pyjson.Object) any {
		pyjson.Obj(s.SetDefault("suggest", pyjson.New())).Set("idle_since", stamp)
		return nil
	}, "idle clock")
}

// SuggestionPlan is plan.suggestion_plan: claim a survey, on a spare lane (#4026) and the next
// free area, for its share of the room (`SuggestionsDue`).
func SuggestionPlan(ctx *hctx.Context, lanes *Lanes, force bool, quietOK string /* = "" */) *pyjson.Object {
	cfg := ctx.Cfg
	state := ctx.Store.Load()
	spare := Spare(ctx, state, lanes, force)
	count := SuggestionsDue(ctx, state, force, SpareTotal(spare))
	if count <= 0 {
		return nil
	}
	provider := SurveyProvider(ctx, state, lanes, force, quietOK, spare)
	if provider == nil {
		return nil
	}
	var lines []string
	for _, issue := range ctx.GH.ListIssues("", "all", 150, "") {
		kind := "issue"
		if issue.Has("pull_request") {
			kind = "PR"
		}
		names := make([]string, 0)
		for name := range queue.LabelNames(issue) {
			names = append(names, name)
		}
		sort.Strings(names)
		tags := strings.Join(names, ", ")
		line := fmt.Sprintf("- %s #%s [%s] %s", kind, py.Str(issue.Get("number")), py.Str(issue.Get("state")),
			py.Str(issue.GetOr("title", "")))
		if tags != "" {
			line += " (" + tags + ")"
		}
		lines = append(lines, line)
	}
	previous := state.Obj("suggest")
	wasRequested := previous.Truthy("requested")
	// The suggestions people closed unbuilt join the log of those that came to nothing (#797),
	// however long ago: the list above is only the latest issues.
	closed := filter(ctx.GH.ListIssues(config.LabelSuggestion, "closed", 300, ""),
		func(i *pyjson.Object) bool { return !i.Has("pull_request") })

	surveys := Surveys(state)
	slot := freeSlot(surveys)
	var area *config.SuggestionArea
	if free := freeAreas(cfg, state, surveys); len(free) > 0 {
		area = &free[0]
	}
	var taken []string
	state = ctx.Store.Update(func(s *pyjson.Object) any {
		declined.Sync(s, closed, clock.ISO(ctx.Now()))
		// The idle clock starts again (#797): the next survey waits for another idle spell.
		suggest := s.Obj("suggest")
		suggest.Update(pyjson.New("last_run", clock.ISO(ctx.Now()), "requested", false,
			"idle_since", clock.ISO(ctx.Now())))
		taken = asks.PopWaiting(suggest)
		record := pyjson.New("provider", provider.ID, "run_id", cfg.RunID, "count", count,
			"last_run", clock.ISO(ctx.Now()), "taken_asks", pyjson.ListOf(taken))
		if area != nil {
			record.Set("area", area.ID)
			suggest.Set("last_area", area.ID)
		}
		pyjson.Obj(suggest.SetDefault("running", pyjson.New())).Set(strconv.Itoa(slot), record)
		what := "suggestions on " + provider.ID
		if area != nil {
			what = fmt.Sprintf("suggestions (%s) on %s", area.ID, provider.ID)
		}
		s.Set("last_run", pyjson.New("at", clock.ISO(ctx.Now()), "url", cfg.RunURL(), "what", what))
		return nil
	}, "claim suggestions")
	asks.React(ctx.GH, taken, asks.Working)
	existing := strings.Join(lines, "\n")
	if existing == "" {
		existing = "(none)"
	}
	planned := pyjson.New(
		"action", "suggest",
		"count", count,
		"slot", slot,
		"area", nil,
		"area_focus", nil,
		"existing", prompts.Data(existing, "Existing issues and pull requests"),
		"declined", prompts.Data(declined.Lines(state), "Suggestions that came to nothing"),
		// The judge's rolling window (`suggestions.auto_approve.window`, `prompts/judge.md`).
		"decided", prompts.Data(declined.DecidedLines(state, cfg.AutoApprove.Window), "Recent decisions on suggestions"),
		"forced", force,
		"was_requested", wasRequested,
		"previous_last_run", previous.Get("last_run"),
		"previous_idle_since", previous.Get("idle_since"),
	)
	if area != nil {
		planned.Update(pyjson.New("area", area.ID, "area_focus", area.Focus))
	}
	return planned.Update(providerFields(ctx, provider))
}
