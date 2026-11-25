package plan

// Suggestions that approve themselves (`suggestions.auto_approve`). After a survey, a second model
// call, the judge (`prompts/judge.md`, `work.Worker.judge`), reads each suggestion against the
// latest decisions people made on suggestions (`declined.DecidedLines`, its rolling window) and
// says whether it would approve it. Delivery says so in the issue's footer and, for one it would
// approve, keeps a record (`suggest.auto`, `MarkAutoApprove`). `AutoApprove`, in the sweep, then
// queues each such suggestion that has waited `wait_hours` and that no person took up:
//
//   - one that is closed, or lost `bot:suggestion` (a person voted, or queued it), is decided
//     already, and its record goes;
//   - one with a comment that is not the bot's, or a 👎 on the issue, is left to a person: someone
//     is looking at it, so the record goes and it waits for their vote as any suggestion does;
//   - the rest go ahead, oldest first, at most `per_day` in any 24 hours (`suggest.auto_approved`,
//     the times they did), each at `priority:low` whatever its class opened it with, so it waits
//     behind everything a person asked for. One past the cap waits for the next tick with room.
//
// Approving one is queueing it, as a person's `approve` does (`queue.QueueBuild`), so its build is
// planned, built, reviewed and merged under every gate a person's approval would meet, and `stop`
// takes it back out. It joins the log of decisions as `auto-approved`, no person's verdict, so the
// judge never counts its own approvals as people's.

import (
	"fmt"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/identity"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
)

// AutoApproveDay is the span `per_day` counts over.
var AutoApproveDay = 24 * time.Hour

// MarkAutoApprove keeps suggestion `number`, opened `at`, as one the judge would approve, for
// `AutoApprove`.
func MarkAutoApprove(state *pyjson.Object, number int, at string, why string) {
	suggest := pyjson.Obj(state.SetDefault("suggest", pyjson.New()))
	auto := pyjson.Obj(suggest.SetDefault("auto", pyjson.New()))
	auto.Set(strconv.Itoa(number), pyjson.New("at", at, "why", py.SliceTo(why, 400)))
}

// autoApprovedSince is how many suggestions approved themselves since `since`.
func autoApprovedSince(state *pyjson.Object, since time.Time) int {
	n := 0
	for _, at := range state.Obj("suggest").List("auto_approved") {
		if !clock.ParseISOValue(at).Before(since) {
			n++
		}
	}
	return n
}

// heldBy is who took suggestion `issue` up, so it waits for a person: a 👎 on it, or a comment that
// is not the bot's own. "" when no one did.
func heldBy(ctx *hctx.Context, number int, issue *pyjson.Object) string {
	if issue.Obj("reactions").Int("-1") > 0 {
		return "a 👎 on it"
	}
	bot := strings.ToLower(strings.TrimLeft(ctx.Cfg.BotLogin, "@"))
	for _, c := range ctx.GH.ListComments(number, 100) {
		login := py.OrStr(c.Obj("user").Get("login"), "")
		if strings.ToLower(login) == bot || identity.Carries(c.Str("body"), config.Marker) {
			continue
		}
		return "a comment by @" + login
	}
	return ""
}

// AutoApprove queues the suggestions the judge would approve that waited with no one taking them
// up (the package's rules above). Returns what it did.
func AutoApprove(ctx *hctx.Context) []string {
	auto := ctx.Cfg.AutoApprove
	if !config.Identity.Suggestions || !auto.Enabled {
		return nil
	}
	state := ctx.Store.Load()
	pending := state.Obj("suggest").Obj("auto")
	if pending.Len() == 0 {
		return nil
	}
	now := ctx.Now()
	numbers := []int{}
	for _, key := range pending.Keys() {
		if n, err := strconv.Atoi(key); err == nil {
			numbers = append(numbers, n)
		}
	}
	sort.Ints(numbers) // oldest first
	room := auto.PerDay - autoApprovedSince(state, now.Add(-AutoApproveDay))
	notes := []string{}
	var gone, approved []int
	for _, number := range numbers {
		record := pending.Obj(strconv.Itoa(number))
		opened := clock.ParseISOValue(record.Get("at"))
		if now.Sub(opened) < time.Duration(auto.WaitHours)*time.Hour {
			continue
		}
		var issue *pyjson.Object
		if _, failed := herr.Catch[*herr.GitHubError](func() { issue = ctx.GH.GetIssue(number) }); failed {
			continue // read again at the next tick
		}
		names := queue.LabelNames(issue)
		if issue.Get("state") != "open" || issue.Has("pull_request") || !names[config.LabelSuggestion] {
			gone = append(gone, number) // decided already
			continue
		}
		if who := heldBy(ctx, number, issue); who != "" {
			gone = append(gone, number)
			notes = append(notes, fmt.Sprintf("#%d waits for a person: %s", number, who))
			continue
		}
		if room <= 0 {
			notes = append(notes, fmt.Sprintf("#%d waits: %d approved themselves in the last day already",
				number, auto.PerDay))
			break
		}
		for name := range names {
			if strings.HasPrefix(strings.ToLower(name), "priority:") && name != config.LabelPriorityLow {
				ctx.GH.RemoveLabel(number, name)
			}
		}
		if !names[config.LabelPriorityLow] {
			ctx.GH.AddLabels(number, []string{config.LabelPriorityLow})
		}
		answer := queue.QueueBuild(ctx, number, strings.TrimLeft(ctx.Cfg.BotLogin, "@"), false, false, nil, nil)
		gone = append(gone, number)
		if queue.LabelNames(ctx.GH.GetIssue(number))[config.LabelSuggestion] {
			// Not queued (a `human` label, another bot's): it stays a suggestion for a person.
			notes = append(notes, fmt.Sprintf("#%d could not approve itself: %s", number, py.SliceTo(answer, 200)))
			continue
		}
		room--
		approved = append(approved, number)
		why := py.OrStr(record.Get("why"), "")
		if why != "" {
			why = fmt.Sprintf(" (%s)", strings.TrimRight(why, "."))
		}
		ctx.GH.CreateComment(number, fmt.Sprintf("Approved by itself: no one voted on it, commented or "+
			"reacted 👎 in the %d hours since it opened, and the judge would approve it%s. It goes in at "+
			"`priority:low`. %s `%s stop` takes it back out of the queue.\n\n%s", auto.WaitHours, why,
			answer, config.Slash, config.Marker))
		notes = append(notes, fmt.Sprintf("#%d approved itself", number))
	}
	if len(gone) > 0 {
		ctx.Store.Update(func(s *pyjson.Object) any {
			suggest := pyjson.Obj(s.SetDefault("suggest", pyjson.New()))
			if kept := suggest.Obj("auto"); kept != nil {
				for _, n := range gone {
					kept.Delete(strconv.Itoa(n))
				}
			}
			since := now.Add(-AutoApproveDay)
			times := []any{}
			for _, at := range suggest.List("auto_approved") {
				if !clock.ParseISOValue(at).Before(since) {
					times = append(times, at)
				}
			}
			for range approved {
				times = append(times, clock.ISO(now))
			}
			suggest.Set("auto_approved", times)
			return nil
		}, "suggestions that approve themselves")
	}
	return notes
}
