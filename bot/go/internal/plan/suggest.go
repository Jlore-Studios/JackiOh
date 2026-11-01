package plan

import (
	"fmt"
	"sort"
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
// is full. One is due when asked for, or once a free lane has had nothing to start for
// `suggestions.idle_minutes` (#797: `suggest.idle_since`, kept by `note_idle`).
func SuggestionsDue(ctx *hctx.Context, state *pyjson.Object, force bool /* = False */) int {
	cfg := ctx.Cfg
	if !cfg.SuggestionsEnabled {
		return 0
	}
	suggest := state.Obj("suggest")
	if suggest.Truthy("provider") && !force {
		return 0 // one is running (a dead one's claim is cleared by `housekeeping`)
	}
	idle := clock.ParseISOValue(suggest.Get("idle_since"))
	due := force || suggest.Truthy("requested") ||
		(!idle.IsZero() && ctx.Now().Sub(idle) >= time.Duration(cfg.SuggestionsIdleMinutes)*time.Minute)
	if !due {
		return 0
	}
	return max(0, cfg.SuggestionsMaxOpen-OpenSuggestionCount(ctx))
}

// NoteIdle keeps the clock a survey waits on (#797): `suggest.idle_since` is when a lane last
// became free with nothing in the queue it could start (`Peek.idle`), and goes when there is
// work for it again. The sweep calls this every tick, writing only when it changes; a survey
// claimed restarts it (`suggestion_plan`). Bookkeeping, so it reports nothing.
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

// SuggestionPlan is plan.suggestion_plan.
func SuggestionPlan(ctx *hctx.Context, lanes *Lanes, force bool, quietOK string /* = "" */) *pyjson.Object {
	cfg := ctx.Cfg
	state := ctx.Store.Load()
	count := SuggestionsDue(ctx, state, force)
	if count <= 0 {
		return nil
	}
	provider := SurveyProvider(ctx, state, lanes, force, quietOK)
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

	var taken []string
	state = ctx.Store.Update(func(s *pyjson.Object) any {
		declined.Sync(s, closed, clock.ISO(ctx.Now()))
		// The idle clock starts again (#797): the next survey waits for another idle spell.
		suggest := s.Obj("suggest")
		suggest.Update(pyjson.New("last_run", clock.ISO(ctx.Now()), "requested", false,
			"provider", provider.ID, "run_id", cfg.RunID, "idle_since", clock.ISO(ctx.Now())))
		taken = asks.Take(suggest)
		s.Set("last_run", pyjson.New("at", clock.ISO(ctx.Now()), "url", cfg.RunURL(),
			"what", "suggestions on "+provider.ID))
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
		"existing", prompts.Data(existing, "Existing issues and pull requests"),
		"declined", prompts.Data(declined.Lines(state), "Suggestions that came to nothing"),
		"forced", force,
		"was_requested", wasRequested,
		"previous_last_run", previous.Get("last_run"),
		"previous_idle_since", previous.Get("idle_since"),
	)
	return planned.Update(providerFields(ctx, provider))
}
