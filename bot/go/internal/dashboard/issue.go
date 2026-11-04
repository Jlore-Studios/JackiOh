package dashboard

import (
	"fmt"
	"slices"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cardsets"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/disk"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/memory"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stats"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/waiting"
)

// View is what both bots' views are drawn from, read from GitHub once per rewrite: the state, the
// open threads, the live lanes, why nothing may start (halted) if so, the queue and the
// subscriptions explained, the last runs and the full status text. `learn` is the learn lane's
// section, and `runners` the runners' drawn from the runs `found`, read once per rewrite by
// `update` (None: `render` reads them). In Go, None is a nil Learn, an empty Runners (a drawn
// section is never empty) and a nil Found.
type View struct {
	State     *pyjson.Object
	Issues    []*pyjson.Object
	Live      *pyjson.Object
	Halted    string
	Explained *waiting.Explained
	Runs      []string
	Report    string
	Learn     []string // default: None
	Runners   string   // default: None
	Found     *Found   // default: None
}

// Look is look.
func Look(ctx *hctx.Context) *View {
	state := ctx.Store.Load()
	issues := ctx.GH.ListIssues("", "open", 300, "")
	_, live := status.LanesNow(ctx, state)
	halted := plan.Stops(ctx, state, false)
	explained := waiting.Explain(ctx, state, live, issues, halted)
	runs := RunsTable(ctx)
	report := status.Report(ctx)
	return &View{State: state, Issues: issues, Live: live, Halted: halted, Explained: explained,
		Runs: runs, Report: report}
}

// Section is section: this bot's section of the shared status issue (#60): Squishy's, drawn by its
// own process and put into the night bot's issue apart from the night bot's own facts.
func Section(ctx *hctx.Context) string {
	view := Look(ctx)
	state, issues, live, halted, explained := view.State, view.Issues, view.Live, view.Halted,
		view.Explained
	title, slash := config.Title, config.Slash
	halt := ""
	if halted != "" {
		halt = "🛑 **Halted.** "
	}
	// Its lanes first, for the night bot's lanes table, which lists every bot's (`marked_lanes`).
	lines := []string{SectionStart, LanesMark_(ctx, state, live, halted), "## 🫧 " + title, "",
		halt + "_@" + ctx.Cfg.BotLogin + ", on its own Claude " +
			"account and GitHub's runners. `" + slash + " status` gives the same facts on demand._",
		"", Headline(ctx, live, explained), ""}
	if rows := DecisionRows(ctx, state, issues, false); len(rows) > 0 {
		// Its items that wait for a person's decision, first (#2986).
		lines = append(append(append(lines, "### Needs a decision", ""), rows...), "")
	}
	if rows := HandedRows(ctx, state, issues, false); len(rows) > 0 {
		// Then those it handed to a person (#3009).
		lines = append(append(append(lines, "### Handed to a person", ""), rows...), "")
	}
	lines = append(lines, "### What it is working on", "")
	// The night bot's two charts, for this bot's runs and lanes: the timeline, then the boxes.
	if running := RunningTable(ctx, state, live, issues); len(running) > 0 {
		lines = append(append(lines, running...), "")
	}
	lines = append(append(lines, Timeline(ctx, state, live, nil)...), "")
	lines = append(append(lines, LanesBoxes(ctx, state, live, explained.Now)...), "")
	lines = append(append(append(lines, "### Its queue", ""), QueueTable(ctx, explained, issues)...), "")
	lines = append(append(append(lines, "### Its account", ""),
		SubscriptionTable(ctx, state, live, explained.Now)...), "")
	if pyjson.Truthy(config.Modes) {
		lines = append(append(append(lines, "### Its trees", ""), TreesTable(ctx, state)...), "")
	}
	// No full status: the line under its heading says where it is, and the issue keeps under
	// GitHub's limit (`MAX_BODY`).
	lines = append(append(lines, "### Last "+title+" runs", ""), view.Runs...)
	head := slices.Index(lines, "## 🫧 "+title) + 2
	folded := Fold("What "+title+" is working on, its queue, account and last runs", lines[head:])
	lines = append(lines[:head:head], folded...)
	return strings.Join(append(lines, SectionEnd), "\n")
}

// SectionStart and SectionEnd are SECTION_START and SECTION_END.
var (
	SectionStart = "<!-- jackioh-bot:section -->"
	SectionEnd   = "<!-- /jackioh-bot:section -->"
)

// ------------------------------------------------------------------ the issue

// Fold is fold: `lines` in a dropdown under the heading before them, so a reader can fold the
// section away. It starts open: GitHub can draw a chart blank that sat in a closed one.
func Fold(summary string, lines []string) []string {
	return slices.Concat([]string{"<details open><summary>" + summary + "</summary>", ""}, lines,
		[]string{"", "</details>", ""})
}

// Render is render: the whole issue: the headline, the night bot's facts, its statistics, then
// each other bot's section (`extra`), then the full status folded (without `full`, a line saying
// where to read it instead: it is the same facts as the sections above, `fit`).
func Render(ctx *hctx.Context, extra []string /* = () */, statistics string /* = "" */, view *View /* = None */, full bool /* = True */) string {
	if view == nil {
		view = Look(ctx)
	}
	state, issues, live, halted, explained := view.State, view.Issues, view.Live, view.Halted,
		view.Explained
	updated := Local(ctx, ctx.Now()).Format("2006-01-02 15:04 MST")
	// The first line, so a glance at the issue says how fresh it is; then how far the PC's two
	// local jobs have played.
	lines := []string{"**Last updated at " + updated + "**", "", Progress(ctx, view.Found), "",
		"# 🤖 " + Title, ""}
	if halted != "" {
		lines = append(lines, "> [!WARNING]", "> 🛑 **Halted**: "+halted+". Nothing starts until it is lifted.",
			"")
	}
	lines = append(lines, "> "+Headline(ctx, live, explained), "",
		fmt.Sprintf("<sub>`bot-status` rewrites this issue every "+
			"%d minutes, and its runners every %d; the statistics "+
			"are counted every "+
			"%d. Times are Central; hover one "+
			"to see how long it ran. `%s status` gives the same facts on demand.</sub>",
			EveryMinutes, RunnersMinutes, int(stats.StatsEvery/time.Minute), config.Slash), "")
	// What waits for a person's decision first (#2986), then what is handed to one (#3009): it is
	// what a person can unblock.
	lines = append(lines, DecisionsSection(ctx, state, issues)...)
	lines = append(lines, "## ⚡ Running now", "")
	var running []string
	if table := RunningTable(ctx, state, live, issues); len(table) > 0 {
		running = append(table, "")
	}
	running = append(running, Timeline(ctx, state, live, view.Found)...)
	running = append(running, "")
	running = append(running, LanesBoxes(ctx, state, live, explained.Now)...)
	lines = append(lines, Fold("What each lane is doing, the runs going on a timeline, and every subscription",
		running)...)
	lines = append(lines, "## 🛣️ Lanes", "")
	lines = append(lines, Fold(
		"How many lanes each subscription has, how many its runs hold and how many are open, "+
			"with the totals and the limits", LanesTable(ctx, state, live, extra, halted))...)
	people := WaitingForPeople(issues)
	queued := append([]string{DifficultyLegend(ctx.Cfg.Pool), ""}, QueueTable(ctx, explained, issues)...)
	if people != "" {
		queued = append(queued, "", people)
	}
	lines = append(lines, "## 📋 Queue", "")
	lines = append(lines, Fold("What each queued item waits for, in the order the planner takes them",
		queued)...)
	diskLine, memoryLine := disk.Line(state), memory.Line(state, ctx.Now())
	subscriptions := append(SubscriptionTable(ctx, state, live, explained.Now), "")
	subscriptions = append(subscriptions, UsageCharts(ctx, state)...)
	if line := UsageIssueLine(state); line != "" {
		subscriptions = append(subscriptions, line, "")
	}
	if diskLine != "" {
		subscriptions = append(subscriptions, diskLine, "")
	}
	if memoryLine != "" {
		subscriptions = append(subscriptions, memoryLine)
	}
	lines = append(lines, "## 🔋 Subscriptions", "")
	lines = append(lines, Fold("Each subscription's usage against its caps, and the machine's disk and memory",
		subscriptions)...)
	lines = append(lines, "## 🏃 Last night-bot runs", "")
	lines = append(lines, Fold(fmt.Sprintf("bot-night.yml's last %d runs", RunRows), view.Runs)...)
	learn := view.Learn
	if learn == nil {
		learn = LearnLane(ctx, view.Found)
	}
	lines = append(lines, upTo(learn, 2)...)
	lines = append(lines, Fold("The AI on main, the lane's pull request, the PC and its last runs",
		from(learn, 2))...)
	if statistics != "" {
		lines = append(lines, strings.TrimSpace(statistics), "")
	}
	for _, part := range extra {
		lines = append(lines, "---", "", strings.TrimSpace(part), "")
	}
	// Low in the issue, below everything else the bots draw: it is the longest part, and the
	// least read at a glance.
	runners := view.Runners
	if runners == "" {
		runners = RunnersSection(ctx, state, nil, defaultRoom)
	}
	lines = append(lines, "---", "", runners, "")
	if full {
		lines = append(lines, "<details><summary>The full status</summary>", "", view.Report, "",
			"</details>")
	} else {
		lines = append(lines, "<sub>"+FullLeftOut+"</sub>")
	}
	lines = append(lines, "", Marker)
	return strings.Join(lines, "\n")
}

// UsageIssueKey is where the state file keeps the usage issue (#2749, `usage.DrawIssue`): its
// number and whether it is pinned (`{"issue": 2751, "pinned": true}`), as `dashboard` keeps the
// status issue's.
var UsageIssueKey = "usage_issue"

// UsageIssueText is the status issue's one line about the usage issue, in its subscriptions
// section, with the usage issue's number.
var UsageIssueText = "📈 How each subscription's usage works out over time, its charts and the tuning suggestions: #%d."

// UsageIssueLine is that line, "" while the state file names no usage issue.
func UsageIssueLine(state *pyjson.Object) string {
	number, err := py.Int(state.Obj(UsageIssueKey).Get("issue"))
	if err != nil || number <= 0 {
		return ""
	}
	return fmt.Sprintf(UsageIssueText, number)
}

// FullLeftOut is FULL_LEFT_OUT: what stands at the foot of the issue when the full status is left
// out for room (`fit`).
var FullLeftOut = "The full status is left out to keep this issue under GitHub's limit: " +
	"`" + config.Slash + " status` gives it."

// Fit is fit: the body, while it would pass GitHub's limit on an issue's body: without the full
// status (the same facts as the sections above, and `/harness status` gives it), then listing fewer
// merged pull requests in the statistics (`draw(shown)`), then fewer busy runners (`_room`), and
// leaving the statistics out last of all.
func Fit(ctx *hctx.Context, view *View, extra []string, draw func(int) string) string {
	body := Render(ctx, extra, draw(ShownSteps[0]), view, true)
	if py.Len(body) <= MaxBody {
		return body
	}
	for _, shown := range ShownSteps {
		body = Render(ctx, extra, draw(shown), view, false)
		if py.Len(body) <= MaxBody {
			return body
		}
	}
	body = room(ctx, view, body, func() string {
		return Render(ctx, extra, draw(ShownSteps[len(ShownSteps)-1]), view, false)
	})
	if py.Len(body) <= MaxBody {
		return body
	}
	return Render(ctx, extra, "", view, true)
}

// room is _room: `body` drawn `again` with the runners section cut to the room the rest of it
// leaves, when it would pass GitHub's limit; as it was when it would not, or has no section.
func room(ctx *hctx.Context, view *View, body string, again func() string) string {
	if py.Len(body) <= MaxBody || view.Runners == "" {
		return body
	}
	left := MaxBody - (py.Len(body) - py.Len(view.Runners))
	view.Runners = RunnersSection(ctx, view.State, view.Found, left)
	return again()
}

// Trusted is _trusted: `issue` was opened by the bot or by someone on the trust list, so its body
// is the bot's to rewrite (the status issue's, and the usage issue's, `usage.FindIssue`).
func Trusted(ctx *hctx.Context, issue *pyjson.Object) bool {
	user := issue.Obj("user")
	login := py.OrStr(user.Get("login"), "")
	if strings.ToLower(login) == strings.ToLower(ctx.Cfg.BotLogin) {
		return true
	}
	var userID *int
	if id := user.Get("id"); id != nil {
		if n, err := py.Int(id); err == nil {
			userID = &n
		}
	}
	return ctx.Trust.Level(login, userID, py.OrStr(issue.Get("author_association"), "")) >= 1
}

// Find is find: the open status issue: the one the state file names, or one to adopt.
func Find(ctx *hctx.Context, state *pyjson.Object) *pyjson.Object {
	if number := state.Obj("dashboard").Get("issue"); pyjson.Truthy(number) {
		var named *pyjson.Object
		herr.Catch[*herr.GitHubError](func() {
			issue := ctx.GH.GetIssue(py.MustInt(number))
			if pyjson.Equal(issue.Get("state"), "open") && !issue.Has("pull_request") {
				named = issue
			}
		})
		if named != nil {
			return named
		}
	}
	for _, issue := range ctx.GH.ListIssues("", "open", 300, "") {
		if !issue.Has("pull_request") && pyjson.Equal(issue.Get("title"), Title) &&
			strings.Contains(py.OrStr(issue.Get("body"), ""), Marker) && Trusted(ctx, issue) {
			return issue
		}
	}
	return nil
}

// labelled puts on the status issue any of `Labels` it lacks: `tracking` (#2748) on one opened
// before it, or one a person took off.
func labelled(ctx *hctx.Context, issue *pyjson.Object) {
	names := map[string]bool{}
	for _, label := range pyjson.Objs(issue.GetOr("labels", []any{})) {
		names[py.OrStr(label.Get("name"), "")] = true
	}
	var missing []string
	for _, label := range Labels {
		if !names[label] {
			missing = append(missing, label)
		}
	}
	if len(missing) > 0 {
		ctx.GH.AddLabels(py.MustInt(issue.Get("number")), missing)
	}
}

// Update is update; a variable so the tests can replace it.
var Update = update

// Looked is given each rewrite's view once the rewrite is done (or has failed), by `update`: the
// status loop's usage history records its tick from it (`usage.Recorder.Tick`, #2749), so a tick
// reads nothing from GitHub twice. nil, outside the loop, gives it to nothing.
var Looked func(ctx *hctx.Context, view *View)

// update is update: rewrite the status issue, opening and pinning it first if there is none.
// `extra` are the other bots' sections (`section`), drawn by their own processes.
//
// With `stats`, the issue carries the statistics (`stats.block`): counted afresh when
// `STATS_EVERY` has passed (or `force_stats`, or the issue has none yet, or the kept block no
// longer fits), with `stats_extra` drawing the other bots' records then; otherwise the block
// the issue holds is kept as it is, so a rewrite costs a few calls, not a count's hundreds.
// Without `stats` (bot-commands.yml's rewrite after a sweep) nothing is counted, and the block
// the issue holds is kept too, while it fits. A count reads which set's cards win most from production through `read_cards` once a day
// (`cardsets.section`), and keeps the section the issue holds between reads.
//
// A nil `statsExtra` is the Python's default, `tuple`: no other bot's record.
func update(ctx *hctx.Context, extra []string /* = () */, withStats bool /* = False */, statsExtra func() []string /* = tuple */, forceStats bool /* = False */, readCards cardsets.Fetch /* = None */) string {
	if statsExtra == nil {
		statsExtra = func() []string { return nil }
	}
	view := Look(ctx)
	if Looked != nil {
		defer Looked(ctx, view)
	}
	view.Found = ReadRunners(ctx)
	view.Learn = LearnLane(ctx, view.Found)
	state := view.State
	issue := Find(ctx, state)
	if issue != nil {
		labelled(ctx, issue)
		Explain(ctx, py.MustInt(issue.Get("number")), state)
	}
	view.Runners = RunnersSection(ctx, state, view.Found, defaultRoom)
	bodyNow := ""
	if issue != nil {
		bodyNow = py.OrStr(issue.Get("body"), "")
	}
	kept := stats.Kept(bodyNow) // "" is None: a kept block is never empty
	note := ""

	count := func() (string, string) {
		return counted(ctx, view, extra, statsExtra,
			cardsets.Section(ctx, state, cardsets.Kept(bodyNow), readCards))
	}
	var body string
	switch {
	case withStats && (forceStats || kept == "" || stats.Due(ctx, state)):
		body, note = count()
	case kept != "":
		body = Render(ctx, extra, kept, view, true)
		// Without the full status, then fewer runners listed, as `fit` makes room, rather than a
		// count again.
		if py.Len(body) > MaxBody {
			body = Render(ctx, extra, kept, view, false)
		}
		body = room(ctx, view, body, func() string { return Render(ctx, extra, kept, view, false) })
		if py.Len(body) > MaxBody {
			if withStats {
				body, note = count()
			} else {
				body, note = Render(ctx, extra, "", view, true), ""
			}
		}
	default:
		body = Render(ctx, extra, "", view, true)
	}
	var number int
	var done string
	if issue == nil {
		issue = ctx.GH.CreateIssue(Title, body, Labels)
		number = py.MustInt(issue.Get("number"))
		done = fmt.Sprintf("opened #%d", number)
		if exc, failed := herr.Catch[*herr.GitHubError](func() {
			ctx.GH.PinIssue(py.OrStr(issue.Get("node_id"), ""))
		}); failed {
			done += fmt.Sprintf("; could not pin it (%d): pin it by hand", exc.Status)
		} else {
			done += " and pinned it"
		}
		Explain(ctx, number, state) // the next rewrite links it
	} else {
		number = py.MustInt(issue.Get("number"))
		ctx.GH.UpdateIssue(number, pyjson.New("body", body))
		done = fmt.Sprintf("rewrote #%d", number)
	}
	if !pyjson.Equal(state.Obj("dashboard").Get("issue"), number) {
		ctx.Store.Update(func(s *pyjson.Object) any {
			s.Set("dashboard", pyjson.New("issue", number))
			return nil
		}, "dashboard issue")
	}
	return done + note
}

// counted is _counted: the body with the statistics counted afresh, and the note that says so.
func counted(ctx *hctx.Context, view *View, extra []string, statsExtra func() []string,
	cards string /* = "" */) (string, string) {
	facts := stats.Collect(ctx)
	others := statsExtra()
	body := Fit(ctx, view, extra, func(shown int) string {
		return stats.Block(ctx, facts, shown, others, cards)
	})
	stats.Counted(ctx)
	return body, ", with the statistics counted afresh"
}
