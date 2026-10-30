// Package sweep is bot/harness/sweep.py.
//
// The sweep: find every request the event handler never answered, and answer it now.
//
// Events are delivered once, and a handler can fail before it acts (GitHub is down, a write
// races, a bug). The labels are the queue and survive all of that, but a comment, a review, an
// assignment or a failed CI run lives only in its event. So every ten minutes, and on demand, this
// reads recent history back from GitHub itself and answers, through the same code the event would
// have used:
//
//   - a trusted comment, diff comment or review that carries a command or names the bot, and that
//     nobody has claimed (`events.claim`);
//   - a trusted review asking a bot pull request for changes, likewise unclaimed;
//   - an open issue or PR assigned to the bot that nothing ever queued;
//   - a failed CI run on the head of a bot PR that nothing reran or queued a fix for;
//   - a label the bot uses (`config.LABELS`) that the repository does not have: a new one used to
//     wait for a person to run `harness setup`, so the method labels (#307) shipped
//     without existing, and no issue could be triaged; and one it no longer uses
//     (`config.RETIRED_LABELS`), which it deletes;
//   - a tree the bot split (`squishy:tree`, #60) whose sub-issues have all closed: its close-out is
//     queued, which checks the parent against `main` and closes it or opens what is missing;
//   - an issue the bot has planned, and a pull request it opened, that carries no `difficulty:*` label
//     (a planner that gave no rating line leaves none, #735): it gets its issue's label, or medium,
//     which is how it routes anyway; an issue still waiting for its plan is left to the planner;
//   - last, a night run that should be going and is not: GitHub drops scheduled runs, sometimes a
//     whole night of `bot-night`'s hourly ones, so when the gate's own question (`plan.peek`) finds
//     work for a free lane and no `bot-night` run is on its way to take it, the sweep starts one.
//     With `controller` on (#1455) the status loop's sweep plans the work and starts each run with
//     its plan (`controller.start`), and any other sweep leaves the work to the loop while it goes;
//   - just before that, in the status loop, the night runs it started that have ended: it delivers
//     them itself (`controller.deliver`, #1537), as they have no `deliver` job, so a run's item is
//     delivered within a tick of its model job ending, before the loop plans again.
//
// It looks back to the later of `LOOKBACK` and the first sweep ever (so the first one replays
// nothing), leaves anything younger than `SETTLE` to a handler that may still be running, and
// skips an edited comment, whose editor it cannot see: an author's edit arrives as its own event.
// One failure never stops the rest.
package sweep

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"regexp"
	"slices"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/commands"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/controller"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stepup"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/triage"
)

// Lookback is LOOKBACK.
var Lookback = 3 * 24 * time.Hour

// Settle is SETTLE: anything younger than this may still be in its handler's hands.
var Settle = 10 * time.Minute

// Edited is EDITED: an edit shows as `updated_at` later than `created_at` by more than this.
var Edited = 5 * time.Second

// QueueLabels is QUEUE_LABELS.
var QueueLabels = []string{config.LabelBuild, config.LabelRevise, config.LabelWorking, config.LabelBlocked}

// LiveRun is LIVE_RUN: a `bot-night` run in one of these states is going, or about to.
var LiveRun = []string{"queued", "in_progress", "waiting", "pending", "requested"}

var numberRe = regexp.MustCompile(`/(?:issues|pulls)/(\d+)$`)

// part is one of the sweep's parts, with the name its failure is noted under
// (`part.__name__.strip('_')`).
type part struct {
	name string
	run  func(ctx *hctx.Context, since time.Time) []string
}

// Sweep is sweep.
func Sweep(ctx *hctx.Context) []string {
	st := ctx.Store.Load()
	first := clock.ParseISOValue(st.Obj("last_sweep").Get("since"))
	now := ctx.Now()
	if first.IsZero() {
		ctx.Store.Update(func(s *pyjson.Object) any {
			s.Set("last_sweep", pyjson.New("at", clock.ISO(now), "since", clock.ISO(now),
				"notes", []any{"the first sweep: from now on"}))
			return nil
		}, "sweep")
		return []string{"the first sweep: requests from now on are swept"}
	}
	since := first
	if back := now.Add(-Lookback); back.After(since) {
		since = back
	}
	notes := []string{}
	parts := []part{
		{"labels", Labels}, {"comments", comments}, {"reviews", reviews},
		{"assignments", assignments}, {"failed_ci", failedCI}, {"needs_plan", needsPlan},
		{"green_heads", GreenHeads}, {"trees", Trees}, {"deliveries", Deliveries},
		{"night_run", func(ctx *hctx.Context, since time.Time) []string { return NightRun(ctx, since) }},
		{"issue_types", issueTypes}, {"difficulty_labels", difficultyLabels},
	}
	for _, p := range parts {
		var got []string
		if err := herr.Try(func() { got = p.run(ctx, since) }); err != nil {
			// one part failing never stops the rest
			notes = append(notes, fmt.Sprintf("%s: could not finish (%s: %s)", p.name, herr.Name(err),
				py.SliceTo(err.Error(), 200)))
			continue
		}
		notes = append(notes, got...)
	}
	record := func(s *pyjson.Object) any {
		last := notes
		if len(last) > 20 {
			last = last[len(last)-20:]
		}
		s.Set("last_sweep", pyjson.New("at", clock.ISO(now), "since", clock.ISO(first), "notes", pyjson.ListOf(last)))
		if ctx.Cfg.ControllerLoop {
			s.Set("loop_tick", clock.ISO(now)) // the status loop's heartbeat (`plan.loop_alive`)
		}
		return nil
	}

	ctx.Store.Update(record, "sweep")
	return notes
}

// Labels is _labels: create every label in `config.LABELS` the repository lacks and delete every
// one in `config.RETIRED_LABELS` it still has, once each time those lists change
// (`labels_synced`, their fingerprint, in the state file), so a tick normally reads nothing. A
// label a person deleted comes back with the next change to the list, or with
// `harness setup`.
func Labels(ctx *hctx.Context, since time.Time) []string {
	specs := config.LabelSpecs()
	sorted := append([]config.LabelSpec(nil), specs...)
	sort.SliceStable(sorted, func(i, j int) bool { return sorted[i].Name < sorted[j].Name })
	items := make([]any, len(sorted))
	for i, l := range sorted {
		items[i] = []any{l.Name, []any{l.Color, l.Description}}
	}
	sum := sha256.Sum256([]byte(pyjson.Dumps([]any{items, pyjson.ListOf(config.RetiredLabels)})))
	want := hex.EncodeToString(sum[:])[:16]
	if pyjson.Equal(ctx.Store.Load().Get("labels_synced"), want) {
		return []string{}
	}
	notes, refused := []string{}, false
	for _, l := range specs {
		var created bool
		if exc, ok := herr.Catch[*herr.GitHubError](func() { created = ctx.GH.EnsureLabel(l.Name, l.Color, l.Description) }); ok {
			// one GitHub refuses never stops the rest
			refused = true
			notes = append(notes, fmt.Sprintf("could not create the label `%s`: %s", l.Name, py.SliceTo(exc.Error(), 200)))
		} else if created {
			notes = append(notes, fmt.Sprintf("created the label `%s`", l.Name))
		}
	}
	for _, name := range config.RetiredLabels {
		var deleted bool
		if exc, ok := herr.Catch[*herr.GitHubError](func() { deleted = ctx.GH.DeleteLabel(name) }); ok {
			refused = true
			notes = append(notes, fmt.Sprintf("could not delete the label `%s`: %s", name, py.SliceTo(exc.Error(), 200)))
		} else if deleted {
			notes = append(notes, fmt.Sprintf("deleted the retired label `%s`", name))
		}
	}
	if !refused { // a refused one is tried again next tick
		ctx.Store.Update(func(s *pyjson.Object) any { s.Set("labels_synced", want); return nil }, "labels synced")
	}
	return notes
}

// needsPlan is _needs_plan: every queued item with no plan its difficulty may build from, an
// unrated one included, carries `bot:needs-plan` within one sweep (#317 part 8), so nothing waits
// for a person to rate it: the planning lane rates and plans it, and `_night_run` starts that run.
func needsPlan(ctx *hctx.Context, since time.Time) []string {
	return plan.SyncNeedsPlan(ctx, ctx.Store.Load())
}

// GreenHeads is _green_heads: a bot pull request whose head met the review rule and then went
// green in CI ends its item's strikes (#316). Only a head the review rule cleared is looked at, and
// its check runs are read only when its item has strikes to clear.
func GreenHeads(ctx *hctx.Context, since time.Time) []string {
	notes := []string{}
	items := ctx.Store.Load().Obj("items")

	act := func(pull *pyjson.Object) string {
		if !queue.LabelNames(pull)[config.LabelPR] {
			return ""
		}
		number := py.MustInt(py.Item(pull, "number"))
		record := items.Obj(strconv.Itoa(number))
		if _, ok := record.Get("cleared").(*pyjson.Object); !ok {
			return ""
		}
		if stepup.ClearIfGreen(ctx, number, py.OrStr(pull.Obj("head").Get("sha"), "")) {
			return fmt.Sprintf("#%d: approved and green, so its strikes are cleared", number)
		}
		return ""
	}
	each(ctx.GH.ListPulls("open", "", 200), act, &notes)
	return notes
}

// issueTypes is _issue_types: every open issue gets a type. Triage types the ones people open as
// they open; the ones a bot opens (the CI-duration alerts, the status issue) never reach it, and
// triage can miss one. Those get the same `triage.fallback_type`.
func issueTypes(ctx *hctx.Context, since time.Time) []string {
	notes := []string{}
	var types map[string]string
	fetched := false
	for _, issue := range ctx.GH.ListIssues("", "open", 300, "") {
		if issue.Has("pull_request") || issue.Truthy("type") {
			continue
		}
		if !fetched {
			types, fetched = triage.IssueTypes(ctx.GH), true
		}
		wanted := triage.FallbackType(issue, types)
		if wanted == "" {
			continue
		}
		var done []string
		if err := herr.Try(func() {
			p := &triage.Plan{IssueType: wanted}
			done = triage.Apply(ctx.GH, py.MustInt(py.Item(issue, "number")), p)
		}); err != nil {
			// one issue never stops the rest
			done = []string{fmt.Sprintf("could not type it (%s)", herr.Name(err))}
		}
		notes = append(notes, fmt.Sprintf("#%s: %s", py.Str(py.Item(issue, "number")), strings.Join(done, "; ")))
	}
	return notes
}

// difficultyLabels is _difficulty_labels: a planned issue or a bot pull request with no
// `difficulty:*` label gets one (#735). The planner labels what it rates, but a plan with no
// rating line leaves the item bare, and a pull request copies its issue's label only when the
// issue has one. A pull request takes its issue's label (`bot/issue-<n>`) and an issue takes
// medium, recorded as the bot's own rating so a later planner may still raise it. An issue still
// waiting for its plan is left to the planner, which may rate it easy. A `human` thread, and a
// person's label, are never touched.
func difficultyLabels(ctx *hctx.Context, since time.Time) []string {
	notes := []string{}
	// The issues API lists pull requests too, without their branch: take those from the pulls.
	var threads []*pyjson.Object
	for _, i := range ctx.GH.ListIssues("", "open", 300, "") {
		if !i.Has("pull_request") {
			threads = append(threads, i)
		}
	}
	threads = append(threads, ctx.GH.ListPulls("open", "", 200)...)
	stem := py.SliceTo(queue.BranchForIssue(0), -1)

	act := func(thread *pyjson.Object) string {
		names := queue.LabelNames(thread)
		if queue.LabelledDifficulty(names) != "" || queue.IsHuman(names) {
			return ""
		}
		number := py.MustInt(py.Item(thread, "number"))
		if thread.Has("head") {
			if !names[config.LabelPR] {
				return ""
			}
			ref := py.OrStr(thread.Obj("head").Get("ref"), "")
			wanted := "medium"
			if strings.HasPrefix(ref, stem) && py.IsDecimal(py.SliceFrom(ref, py.Len(stem))) {
				issue := ctx.GH.GetIssue(py.MustInt(py.SliceFrom(ref, py.Len(stem))))
				if got := queue.LabelledDifficulty(queue.LabelNames(issue)); got != "" {
					wanted = got
				}
			}
			ctx.GH.AddLabels(number, []string{"difficulty:" + wanted})
			return fmt.Sprintf("#%d: no difficulty, so `difficulty:%s`", number, wanted)
		}
		if !names[config.LabelPlanned] {
			return ""
		}
		ctx.GH.AddLabels(number, []string{"difficulty:medium"})
		ctx.Store.Update(func(s *pyjson.Object) any {
			state.Item(s, number).Set("difficulty_by", pyjson.New(
				"difficulty", "medium", "provider", "sweep", "tier", "", "at", clock.ISO(ctx.Now()),
				"why", "planned with no rating, so medium"))
			return nil
		}, fmt.Sprintf("sweep #%d", number))
		return fmt.Sprintf("#%d: planned with no difficulty, so `difficulty:medium`", number)
	}
	each(threads, act, &notes)
	return notes
}

// each is _each: run `act` on every item; a failure is noted and the next item still runs. An
// empty note is Python's None.
func each(items []*pyjson.Object, act func(*pyjson.Object) string, notes *[]string) {
	for _, entry := range items {
		var note string
		if err := herr.Try(func() { note = act(entry) }); err != nil {
			note = fmt.Sprintf("skipped one (%s: %s)", herr.Name(err), py.SliceTo(err.Error(), 200))
		}
		if note != "" {
			*notes = append(*notes, note)
		}
	}
}

// number is _number: the issue or pull request number at the end of an API URL, or None.
func number(url any) *int {
	match := numberRe.FindStringSubmatch(py.OrStr(url, ""))
	if match == nil {
		return nil
	}
	n := py.MustInt(match[1])
	return &n
}

// settled is _settled.
func settled(ctx *hctx.Context, stamp any, since time.Time) bool {
	at := clock.ParseISOValue(stamp)
	return !at.IsZero() && !at.Before(since) && !at.After(ctx.Now().Add(-Settle))
}

// trusted is _trusted (with `level` 1, as every caller has it).
func trusted(ctx *hctx.Context, entry *pyjson.Object) bool {
	user := entry.Obj("user")
	var id *int
	if v := user.Get("id"); v != nil {
		n := py.MustInt(v)
		id = &n
	}
	assoc := ""
	if v := entry.Get("author_association"); v != nil {
		assoc = py.Str(v)
	}
	return ctx.Trust.Level(py.Str(user.GetOr("login", "")), id, assoc) >= 1
}

// asks is _asks.
func asks(ctx *hctx.Context, body string) bool {
	return len(commands.Parse(body, ctx.Cfg.BotLogin)) > 0 || commands.NamesTheBot(body, ctx.Cfg.BotLogin)
}

// comments is _comments.
func comments(ctx *hctx.Context, since time.Time) []string {
	notes := []string{}
	lists := []struct {
		reviewComment bool
		found         []*pyjson.Object
	}{
		{false, ctx.GH.ListRepoComments(clock.ISO(since), 500)},
		{true, ctx.GH.ListRepoReviewComments(clock.ISO(since), 500)},
	}
	for _, list := range lists {
		reviewComment := list.reviewComment
		act := func(comment *pyjson.Object) string {
			created := clock.ParseISOValue(comment.Get("created_at"))
			updated := clock.ParseISOValue(comment.Get("updated_at"))
			if updated.IsZero() {
				updated = created
			}
			if !settled(ctx, comment.Get("created_at"), since) {
				return ""
			}
			if created.IsZero() || updated.IsZero() || updated.Sub(created) > Edited {
				return ""
			}
			login := strings.ToLower(py.Str(comment.Obj("user").GetOr("login", "")))
			body := py.OrStr(comment.Get("body"), "")
			if login == strings.ToLower(ctx.Cfg.BotLogin) || login == events.ActionsBot || events.IsOwn(body) {
				return ""
			}
			// A bare `approve` or `reject` is a request too (#929); review comments carry no votes.
			vote := ""
			if !reviewComment {
				vote, _ = commands.Vote(body, ctx.Cfg.BotLogin)
			}
			if (vote == "" && !asks(ctx, body)) || !trusted(ctx, comment) {
				return ""
			}
			prefix := "c"
			if reviewComment {
				prefix = "rc"
			}
			claimKey := prefix + ":" + py.Str(py.Item(comment, "id"))
			if events.Claimed(ctx, claimKey) || events.Answered_(ctx, py.MustInt(py.Item(comment, "id")), reviewComment) {
				return ""
			}
			var payload *pyjson.Object
			var n *int
			if reviewComment {
				n = number(comment.GetOr("pull_request_url", ""))
				if n == nil {
					return ""
				}
				payload = pyjson.New("action", "created", "comment", comment, "pull_request", ctx.GH.GetPull(*n))
			} else {
				n = number(comment.GetOr("issue_url", ""))
				if n == nil {
					return ""
				}
				payload = pyjson.New("action", "created", "comment", comment, "issue", ctx.GH.GetIssue(*n))
			}
			out := events.OnComment(ctx, payload, reviewComment)
			if len(out) == 0 || strings.HasPrefix(out[0], "ignored") || strings.HasPrefix(out[0], "no command") {
				return "" // nothing was answered: the other bot's words, say (#504)
			}
			return fmt.Sprintf("answered a comment on #%d that was never answered", *n)
		}
		each(list.found, act, &notes)
	}
	return notes
}

// reviews is _reviews.
func reviews(ctx *hctx.Context, since time.Time) []string {
	notes := []string{}
	var pulls []*pyjson.Object
	for _, t := range ctx.GH.ListIssues("", "open", 300, "") {
		if t.Has("pull_request") {
			pulls = append(pulls, t)
		}
	}

	act := func(thread *pyjson.Object) string {
		n := py.MustInt(py.Item(thread, "number"))
		isBotPR := queue.LabelNames(thread)[config.LabelPR]
		var answered []string
		for _, review := range ctx.GH.ListReviews(n) {
			if !settled(ctx, review.Get("submitted_at"), since) || !trusted(ctx, review) {
				continue
			}
			changes := isBotPR && strings.ToUpper(py.Str(review.GetOr("state", ""))) == "CHANGES_REQUESTED"
			if !(changes || asks(ctx, py.OrStr(review.Get("body"), ""))) {
				continue
			}
			if events.Claimed(ctx, "r:"+py.Str(review.Get("id"))) {
				continue
			}
			payload := pyjson.New("action", "submitted", "review", review, "pull_request", ctx.GH.GetPull(n))
			out := events.OnReview(ctx, payload)
			if len(out) > 0 && !strings.HasPrefix(out[0], "ignored") && !strings.HasPrefix(out[0], "no command") {
				answered = append(answered, py.Str(review.Get("id")))
			}
		}
		if len(answered) == 0 {
			return ""
		}
		return fmt.Sprintf("answered %d review(s) on #%d never answered", len(answered), n)
	}

	each(pulls, act, &notes)
	return notes
}

// assignments is _assignments.
func assignments(ctx *hctx.Context, since time.Time) []string {
	st := ctx.Store.Load()
	notes := []string{}

	act := func(thread *pyjson.Object) string {
		n := py.MustInt(py.Item(thread, "number"))
		record := st.Obj("items").Obj(strconv.Itoa(n))
		if intersects(queue.LabelNames(thread), append(append([]string(nil), QueueLabels...), config.LabelPROpen)) ||
			record.Truthy("queued_at") || record.Truthy("stop_requested") {
			return ""
		}
		stamp := thread.Get("updated_at")
		if !pyjson.Truthy(stamp) {
			stamp = clock.ISO(since)
		}
		if !settled(ctx, stamp, since.Add(-Lookback)) {
			return ""
		}
		by := "an assignment"
		var answer string
		if thread.Has("pull_request") {
			answer = queue.QueueRevise(ctx, n, by, false, "request", false, nil, nil)
		} else {
			answer = queueBuild(ctx, n, by, nil)
		}
		ctx.GH.CreateComment(n, "I was assigned here but never queued it. "+answer)
		return fmt.Sprintf("queued #%d from an assignment nothing answered", n)
	}

	each(ctx.GH.ListIssues("", "open", 300, ctx.Cfg.BotLogin), act, &notes)
	return notes
}

// queueBuild is `queue_build(ctx, number, by=by, mode=mode)`, where Python's `mode=None` (leave
// the issue's mode labels as they are) is a nil `mode`.
func queueBuild(ctx *hctx.Context, n int, by string, mode *string) string {
	return queue.QueueBuild(ctx, n, by, false, false, nil, mode)
}

// failedCI is _failed_ci.
func failedCI(ctx *hctx.Context, since time.Time) []string {
	st := ctx.Store.Load()
	notes := []string{}
	var pulls []*pyjson.Object
	for _, t := range ctx.GH.ListIssues(config.LabelPR, "open", 300, "") {
		if t.Has("pull_request") {
			pulls = append(pulls, t)
		}
	}

	act := func(thread *pyjson.Object) string {
		n := py.MustInt(py.Item(thread, "number"))
		if intersects(queue.LabelNames(thread), QueueLabels) || st.Obj("items").Obj(strconv.Itoa(n)).Truthy("stop_requested") {
			return ""
		}
		sha := ctx.GH.GetPull(n).Obj("head").Get("sha")
		if !pyjson.Truthy(sha) {
			return ""
		}
		workflows := events.CIWorkflows(ctx)
		var runs []*pyjson.Object
		for _, r := range ctx.GH.RunsForSHA(py.Str(sha), 30) {
			if py.In(r.Get("name"), workflows...) {
				runs = append(runs, r)
			}
		}
		var failed []*pyjson.Object
		for _, r := range runs {
			stamp := r.Get("updated_at")
			if !pyjson.Truthy(stamp) {
				stamp = r.Get("created_at")
			}
			if r.Get("status") == "completed" && py.In(r.Get("conclusion"), events.CIFailed...) &&
				settled(ctx, stamp, since.Add(-Lookback)) {
				failed = append(failed, r)
			}
		}
		if len(failed) == 0 {
			return ""
		}
		// max with a key: the first of the largest (created_at, run_attempt).
		rank := func(r *pyjson.Object) (string, int) {
			attempt := r.Get("run_attempt")
			if !pyjson.Truthy(attempt) {
				attempt = 1
			}
			return py.Str(r.GetOr("created_at", "")), py.MustInt(attempt)
		}
		latest := failed[0]
		bestAt, bestTry := rank(latest)
		for _, r := range failed[1:] {
			at, try := rank(r)
			if at > bestAt || (at == bestAt && try > bestTry) {
				latest, bestAt, bestTry = r, at, try
			}
		}
		out := events.OnCI(ctx, pyjson.New("action", "completed", "workflow_run", latest))
		var acted []string
		for _, line := range out {
			if strings.Contains(line, "re-ran") || strings.Contains(line, "queued") || strings.Contains(line, "blocked") {
				acted = append(acted, line)
			}
		}
		if len(acted) == 0 {
			return ""
		}
		return fmt.Sprintf("CI on #%d: %s", n, strings.Join(acted, "; "))
	}

	each(pulls, act, &notes)
	return notes
}

// Trees is _trees: a tree the bot split (#60) whose sub-issues have all closed is queued for its
// close-out: the same split session, which checks the parent's end state against `main` and
// closes it, or opens the sub-issues still missing. A fullsend tree's close-out is its reconcile
// (#505): an ordinary build of the parent on its own branch, where its parts landed, which opens
// one pull request into `main`. One stopped (`stop`) or blocked is left alone.
func Trees(ctx *hctx.Context, since time.Time) []string {
	var splits []string
	for _, mode := range config.Modes {
		if strings.HasPrefix(mode, "split") || mode == "fullsend" {
			splits = append(splits, mode)
		}
	}
	if len(splits) == 0 {
		return []string{}
	}
	st := ctx.Store.Load()
	notes := []string{}

	act := func(thread *pyjson.Object) string {
		n := py.MustInt(py.Item(thread, "number"))
		names := queue.LabelNames(thread)
		if thread.Has("pull_request") || intersects(names, append(append([]string(nil), QueueLabels...), config.LabelPROpen)) ||
			queue.IsHuman(names) {
			return ""
		}
		record := st.Obj("items").Obj(strconv.Itoa(n))
		if record.Truthy("stop_requested") {
			return ""
		}
		children := ctx.GH.ListSubIssues(n)
		if len(children) == 0 {
			return ""
		}
		for _, child := range children {
			if child.Get("state") == "open" {
				return ""
			}
		}
		tree, _ := record.Get("tree").(*pyjson.Object)
		mode := py.OrStr(tree.Get("mode"), "")
		if !slices.Contains(splits, mode) {
			mode = splits[0]
		}
		answer := queueBuild(ctx, n, ctx.Cfg.BotLogin, &mode)
		ctx.Store.Update(func(s *pyjson.Object) any {
			state.Item(s, n).Set("closeout", true)
			return nil
		}, fmt.Sprintf("close-out #%d", n))
		if mode == "fullsend" {
			ctx.GH.CreateComment(n, "Every part of this has closed, so its reconcile is "+
				"queued: one run merges the parts on "+
				fmt.Sprintf("`%s`, makes every check green and opens ", queue.BranchForIssue(n))+
				"one pull request into `main`. "+answer)
			return fmt.Sprintf("#%d: its parts have all closed; queued its reconcile", n)
		}
		ctx.GH.CreateComment(n, "Every sub-issue of this has closed, so its close-out is "+
			"queued: it checks this issue against `main`, and closes it when its "+
			"end state holds or opens what is still missing. "+answer)
		return fmt.Sprintf("#%d: its sub-issues have all closed; queued its close-out", n)
	}

	each(ctx.GH.ListIssues(config.LabelTree, "open", 300, ""), act, &notes)
	return notes
}

// Deliveries is _deliveries: the status loop delivers the night runs it started once they end
// (`controller.deliver`, #1537): before `_night_run`, so the lanes they held are free, and their
// items settled, when it plans. Any other sweep leaves them to the loop.
func Deliveries(ctx *hctx.Context, since time.Time) []string {
	if !ctx.Cfg.ControllerLoop {
		return []string{}
	}
	return controller.Deliver(ctx, "")
}

// NightRun is _night_run; a variable so the tests can replace it, and the sweep calls it through
// the variable.
var NightRun = nightRun

// nightRun is _night_run: start a night run when a lane and a subscription are free for work and
// no run is on its way to take it. Runs after the other parts, so a request they just answered
// counts. A run that is going but holds no lane yet (still in its gate or plan, or delivering) is
// on its way, so the sweep waits for it rather than piling more runs behind it.
func nightRun(ctx *hctx.Context, since time.Time) []string {
	look := plan.Peek_(ctx, false, nil, "auto")
	// The clock a survey waits on (#797): a free lane with nothing to start, tick after tick.
	plan.NoteIdle(ctx, look.Idle)
	if !look.Work {
		return []string{}
	}
	if ctx.Cfg.Controller {
		// The status loop plans and starts the runs itself (`controller.py`, #1455); any other
		// sweep leaves the work to it while it is going.
		if ctx.Cfg.ControllerLoop {
			return controller.Start(ctx, nil)
		}
		if plan.LoopAlive(ctx) {
			return []string{}
		}
	}
	var live []*pyjson.Object
	for _, r := range ctx.GH.ListRuns(config.NightWorkflow, "", 20, false) {
		if py.In(r.Get("status"), LiveRun...) {
			live = append(live, r)
		}
	}
	// A run waiting for the shared subscription to be quiet may wait two hours; it is not on
	// its way to the other subscriptions' work, which `peek` then offers instead.
	waiting := 0
	if plan.QuietWaiting(ctx) {
		waiting = 1
	}
	if len(live)-waiting > look.Held {
		return []string{}
	}
	ctx.Dispatch(nil, false, "auto")
	return []string{"started a night run, as a lane was free: " + look.Reason}
}
