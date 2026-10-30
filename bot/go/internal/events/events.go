// Package events is bot/harness/events.py.
//
// GitHub events, handled at once with no model call: commands, labels, assignment, CI results.
//
// Runs in `bot-commands.yml`. Commands are acknowledged with a reaction and answered in one reply.
// Model work is only ever queued here; it happens in the night run, or at once when an operator adds
// `--force`.
package events

import (
	"fmt"
	"regexp"
	"slices"
	"sort"
	"strconv"
	"strings"
	"time"
	"unicode"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/asks"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/commands"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/declined"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stepup"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/threads"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/trust"
)

// ActionsBot is ACTIONS_BOT.
var ActionsBot = "github-actions[bot]"

// CIFailed is CI_FAILED: CI conclusions that leave a bot pull request unable to merge by itself.
var CIFailed = []string{"failure", "timed_out", "cancelled", "startup_failure"}

// CIWorkflows is ci_workflows: the workflows whose checks a bot pull request must pass: CI, whose
// jobs include the bot's own selftest (it had a workflow of its own, `bot selftest`, until #1455).
func CIWorkflows(ctx *hctx.Context) []string {
	return []string{ctx.Cfg.CIWorkflow}
}

// FromTheBotToo is FROM_THE_BOT_TOO: events handled even when the bot sent them: CI runs its own
// pushes started, and pull requests auto-merge closed (GitHub credits the merge to whoever turned
// auto-merge on).
var FromTheBotToo = [][2]string{{"workflow_run", "completed"}, {"pull_request_target", "closed"},
	{"pull_request", "closed"}}

func fromTheBotToo(name string, action any) bool {
	a, ok := action.(string)
	if !ok {
		return false
	}
	for _, pair := range FromTheBotToo {
		if pair[0] == name && pair[1] == a {
			return true
		}
	}
	return false
}

// Handle is handle.
func Handle(ctx *hctx.Context, name string, payload *pyjson.Object) []string {
	sender := py.Str(payload.Obj("sender").GetOr("login", ""))
	action := payload.Get("action")
	if (strings.ToLower(sender) == strings.ToLower(ctx.Cfg.BotLogin) || strings.ToLower(sender) == ActionsBot) &&
		!fromTheBotToo(name, action) {
		return []string{"ignored: the bot's own event"}
	}
	if name == "issue_comment" && py.In(action, "created", "edited") {
		return OnComment(ctx, payload, false)
	}
	if name == "pull_request_review_comment" && py.In(action, "created", "edited") {
		return OnComment(ctx, payload, true)
	}
	if name == "pull_request_review" && action == "submitted" {
		return OnReview(ctx, payload)
	}
	if name == "issues" && py.In(action, "labeled", "assigned") {
		return OnIssueChange(ctx, payload, false)
	}
	if name == "pull_request" || name == "pull_request_target" {
		if py.In(action, "labeled", "assigned") {
			return OnIssueChange(ctx, payload, true)
		}
		if action == "closed" {
			return OnPullClosed(ctx, payload)
		}
	}
	if name == "workflow_run" && action == "completed" {
		return OnCI(ctx, payload)
	}
	return []string{fmt.Sprintf("ignored: %s.%s", name, py.Str(action))}
}

// OtherBotVerbs is OTHER_BOT_VERBS: what one bot may ask of another in a comment (#60): Squishy
// stops the sub-issues of a tree it split that are queued for the night bot.
var OtherBotVerbs = []string{"stop"}

// FromOtherBot is from_other_bot: the comment's author is another bot in the repository
// (`config.OTHERS`).
func FromOtherBot(user *pyjson.Object) bool {
	login := strings.ToLower(py.OrStr(user.Get("login"), ""))
	for _, other := range config.Others {
		if strings.ToLower(other.Login) == login {
			return true
		}
	}
	return false
}

// IsOwn is _is_own: true for a comment the bot wrote (a person quoting it keeps the marker
// behind `>`).
func IsOwn(body string) bool {
	mark := config.Marker
	for _, line := range py.Splitlines(body) {
		if strings.Contains(line, mark) && !strings.HasPrefix(strings.TrimLeftFunc(line, unicode.IsSpace), ">") {
			return true
		}
	}
	return false
}

func level(ctx *hctx.Context, user *pyjson.Object, association any) int {
	return trustLevel(ctx.Trust, py.Str(user.GetOr("login", "")), user.Get("id"), association)
}

// Answered is ANSWERED: the reaction the bot leaves once it has answered a comment. The sweep
// treats a command without it as never answered; nobody but the bot's own logins can leave it as
// the bot. The reactions for the later stages of model work are in `asks`.
var Answered = asks.Answered

// Answered_ is answered: true when the bot has already answered this comment (its rocket reaction
// is there).
func Answered_(ctx *hctx.Context, commentID int, reviewComment bool /* = False */) bool {
	logins := map[string]bool{strings.ToLower(ctx.Cfg.BotLogin): true, ActionsBot: true}
	for _, reaction := range ctx.GH.Reactions(commentID, reviewComment) {
		who := strings.ToLower(py.Str(reaction.Obj("user").GetOr("login", "")))
		if reaction.Get("content") == Answered && logins[who] {
			return true
		}
	}
	return false
}

// Nudge is NUDGE.
var Nudge = "I saw my name, but no request I could act on. Start a line with " +
	"`@{bot} <what you want>` or `" + config.Slash + " build`; `" + config.Slash + " help` lists the rest."

func nudge(bot string) string { return strings.ReplaceAll(Nudge, "{bot}", bot) }

// ClaimDays is CLAIM_DAYS: how long a claim is kept: longer than the sweep looks back.
var ClaimDays = 5

// Claim is claim: record, compare-and-set, that these lines of one comment or review are being
// answered.
//
// Returns the lines nobody claimed before, so an event handler and the sweep, or a comment and
// its edit, never act on the same line twice. A line is claimed before it runs: a handler that
// dies half-way leaves its 👀 and no reply rather than running a command a second time.
func Claim(ctx *hctx.Context, key string, lines []string) []string {
	now := ctx.Now()
	var fresh []string

	change := func(st *pyjson.Object) any {
		handled := pyjson.Obj(st.SetDefault("handled", pyjson.New()))
		cutoff := now.Add(-time.Duration(ClaimDays) * 24 * time.Hour)
		var old []string
		handled.Items(func(k string, v any) bool {
			at := clock.ParseISOValue(pyjson.Obj(v).Get("at"))
			if at.IsZero() {
				at = now
			}
			if at.Before(cutoff) {
				old = append(old, k)
			}
			return true
		})
		for _, k := range old {
			handled.Delete(k)
		}
		before := pyjson.Obj(handled.Get(key)).GetOr("lines", []any{})
		beforeSet := map[string]bool{}
		for _, line := range pyjson.List(before) {
			beforeSet[py.Str(line)] = true
		}
		fresh = fresh[:0]
		for _, line := range lines {
			if !beforeSet[line] {
				fresh = append(fresh, line)
			}
		}
		if len(fresh) > 0 {
			union := map[string]bool{}
			for k := range beforeSet {
				union[k] = true
			}
			for _, line := range lines {
				union[line] = true
			}
			all := make([]string, 0, len(union))
			for k := range union {
				all = append(all, k)
			}
			sort.Strings(all)
			handled.Set(key, pyjson.New("at", clock.ISO(now), "lines", pyjson.ListOf(all)))
		}
		return nil
	}

	ctx.Store.Update(change, "claim "+key)
	return fresh
}

// Claimed is claimed.
func Claimed(ctx *hctx.Context, key string) bool {
	return ctx.Store.Load().Obj("handled").Has(key)
}

func reply(ctx *hctx.Context, number int, body string) {
	if _, ok := herr.Catch[*herr.GitHubError](func() { ctx.GH.CreateComment(number, body) }); ok {
		ctx.GH.CreateComment(number, body) // once more; a second failure is the job's error
	}
}

// OnComment is on_comment: answer the commands in a comment, a line comment on a diff, or an edit
// of either.
func OnComment(ctx *hctx.Context, payload *pyjson.Object, reviewComment bool /* = False */) []string {
	comment := payload.Obj("comment")
	body := py.OrStr(comment.Get("body"), "")
	if IsOwn(body) {
		return []string{"ignored: the bot's own comment"}
	}
	found := commands.Parse(body, ctx.Cfg.BotLogin)
	nudged := len(found) == 0 && commands.NamesTheBot(body, ctx.Cfg.BotLogin)
	voted, voteLine := "", ""
	if !reviewComment {
		voted, voteLine = commands.Vote(body, ctx.Cfg.BotLogin)
	}
	if FromOtherBot(comment.Obj("user")) {
		// The bots name each other in their replies (#60): only a bare `stop`, which Squishy
		// sends to stop a tree it split, is acted on, and nothing is answered with a hint, so
		// two bots never talk to each other for ever.
		var kept []*commands.Command
		for _, c := range found {
			if slices.Contains(OtherBotVerbs, c.Verb) && c.Args == "" {
				kept = append(kept, c)
			}
		}
		found = kept
		nudged = false
		voted = ""
	}
	var thread *pyjson.Object
	if reviewComment {
		pull := payload.Obj("pull_request")
		thread = pyjson.New("number", pull.Get("number"), "pull_request", pyjson.New())
	} else {
		thread = payload.Obj("issue")
	}
	if voted != "" && OpenSuggestion(ctx, py.MustInt(py.Or(thread.Get("number"), 0))) {
		// A vote on one of my suggestions (#760). `@bot reject` would otherwise read as a request
		// to build it, so the vote is what its line means.
		rest := []*commands.Command{{Verb: voted, Args: "", Force: false, Line: voteLine}}
		for _, c := range found {
			if c.Line != voteLine {
				rest = append(rest, c)
			}
		}
		found = rest
		nudged = false
	}
	if len(found) == 0 && !nudged {
		return []string{"no command"}
	}
	user := comment.Obj("user")
	if payload.Get("action") == "edited" {
		editor := payload.Obj("sender")
		if editor.Get("id") != nil && !pyjson.Equal(editor.Get("id"), user.Get("id")) {
			return []string{"ignored: an edit by someone other than the comment's author"}
		}
	}
	lvl := level(ctx, user, comment.Get("author_association"))
	if lvl <= 0 {
		return []string{fmt.Sprintf("ignored: @%s is not on the trust list", py.Str(user.Get("login")))}
	}
	commentID := py.MustInt(py.Item(comment, "id"))
	prefix := "c"
	if reviewComment {
		prefix = "rc"
	}
	claimKey := fmt.Sprintf("%s:%d", prefix, commentID)
	var lines []string
	for _, c := range found {
		lines = append(lines, c.Line)
	}
	if len(lines) == 0 {
		lines = []string{"(named the bot)"}
	}
	fresh := Claim(ctx, claimKey, lines)
	if len(fresh) == 0 {
		return []string{"ignored: already answered"}
	}
	ctx.GH.React(commentID, asks.Seen, reviewComment)
	ask := &asks.Ask{Key: asks.Key(commentID, reviewComment)}
	var replies []string
	if nudged {
		replies = []string{nudge(ctx.Cfg.BotLogin)}
	} else {
		var chosen []*commands.Command
		for _, c := range found {
			if slices.Contains(fresh, c.Line) {
				chosen = append(chosen, c)
			}
		}
		replies = RunCommands(ctx, chosen, thread, user, lvl, ask)
	}
	if ask.Recorded {
		ctx.GH.React(commentID, asks.Queued, reviewComment)
	}
	login := py.Str(user.GetOr("login", ""))
	reply(ctx, py.MustInt(py.Item(thread, "number")), "@"+login+"\n\n"+strings.Join(replies, "\n\n"))
	ctx.GH.React(commentID, Answered, reviewComment)
	return replies
}

// RunCommands is run_commands: run each command and return its reply. `ask` is the comment they
// came from: whatever queues model work records it, so the later stages can react to it (`asks`).
func RunCommands(ctx *hctx.Context, found []*commands.Command, thread *pyjson.Object, user *pyjson.Object, level int, ask *asks.Ask /* = None */) []string {
	var replies []string
	for _, command := range found {
		head := ""
		if len(found) > 1 {
			head = "> " + command.Line + "\n\n"
		}
		if command.Verb == "typo" {
			word := command.Args
			if fields := strings.Fields(command.Args); len(fields) > 0 {
				word = fields[0]
			}
			replies = append(replies, head+fmt.Sprintf("`%s` is not a command; did you mean `%s`? ", word, command.Meant)+
				"I did nothing. Say it again spelt that way, or with more words for a "+
				"request. "+commands.Pointer_(ctx.Cfg.BotLogin))
			continue
		}
		if level < command.Level() {
			need := trust.LevelName(command.Level())
			have := trust.LevelName(level)
			flag := ""
			if command.Force {
				flag = " with `--force`"
			}
			replies = append(replies, head+fmt.Sprintf("`%s`%s needs the %s level; you have ", command.Verb, flag, need)+
				have+". Nothing was done.")
			continue
		}
		var answer string
		if err := herr.Try(func() { answer = Execute(ctx, command, thread, user, ask) }); err != nil {
			// a failure is answered, never dropped in silence
			replies = append(replies, head+fmt.Sprintf("That failed (%s: %s). ", herr.Name(err), py.SliceTo(herr.Str(err), 300))+
				"Nothing is lost by saying it again.")
			continue
		}
		replies = append(replies, head+answer)
	}
	return replies
}

// Execute is execute.
func Execute(ctx *hctx.Context, command *commands.Command, thread *pyjson.Object, user *pyjson.Object, ask *asks.Ask /* = None */) string {
	number := py.MustInt(py.Or(thread.Get("number"), 0))
	isPR := thread.Has("pull_request")
	by := py.Str(user.GetOr("login", ""))
	verb := command.Verb
	if verb == "status" {
		return status.Report(ctx) + "\n\n" + commands.Pointer_(ctx.Cfg.BotLogin)
	}
	if verb == "help" {
		return commands.HelpText(ctx.Cfg.BotLogin, command.Args)
	}
	if verb == "halt" {
		reason := command.Args
		if reason == "" {
			reason = "no reason given"
		}
		ctx.Store.Update(func(s *pyjson.Object) any {
			s.Set("halted", true)
			s.Set("halt", pyjson.New("by", by, "at", clock.ISO(ctx.Now()), "reason", reason))
			return nil
		}, "halt")
		return fmt.Sprintf("Halted. No new model work starts until `%s start`; a run already going ", config.Slash) +
			"stops at its next checkpoint and keeps its work."
	}
	if verb == "start" {
		// `resume <subscription>` (or `start <subscription>`) lifts that one's suspension; any
		// other words after `start` are a note, and it lifts the halt as it always did.
		if command.Args != "" && (saidResume(command) || firstSubscription(ctx, command.Args) != nil) {
			return resume(ctx, command)
		}
		ctx.Store.Update(func(s *pyjson.Object) any {
			s.Set("halted", false)
			s.Set("halt", pyjson.New())
			return nil
		}, "start")
		extra := ""
		if ctx.RepoHalted() {
			extra = " `.harness/HALT` is still on `main`, though, and it wins until it is deleted."
		}
		if command.Force {
			return "Started. " + runNow(ctx, nil) + extra
		}
		return "Started. The bot takes work again as soon as a subscription is free." + extra
	}
	if verb == "suspend" {
		word, rest, _ := strings.Cut(command.Args, " ")
		if word == "" {
			return fmt.Sprintf("`suspend` needs a subscription, as in `%s suspend claude-3 using it ", config.Slash) +
				"myself`. Nothing was done."
		}
		provider, problem := subscription(ctx, word)
		if provider == nil {
			return problem
		}
		held := pyjson.New("by", by, "at", clock.ISO(ctx.Now()), "reason", strings.TrimSpace(rest))
		ctx.Store.Update(func(s *pyjson.Object) any {
			providers.Record(s, provider.ID).Set("suspended", held)
			return nil
		}, "suspend "+provider.ID)
		return fmt.Sprintf("Suspended `%s`. No new work starts on it until ", provider.ID) +
			fmt.Sprintf("`%s resume %s`; a run already going on it stops at its next ", config.Slash, provider.ID) +
			"checkpoint, keeps its work and goes back to the queue for another subscription."
	}
	if verb == "run" {
		target := strings.TrimLeft(command.Args, "#")
		var item *int
		if py.IsDecimal(target) {
			n := py.MustInt(target)
			item = &n
		}
		return runNow(ctx, item)
	}
	if slices.Contains([]string{"suggest", "oneshot", "split", "fullsend"}, verb) && !commands.Offered(verb) {
		return notMine(verb)
	}
	if verb == "suggest" {
		wanted := func(st *pyjson.Object) any {
			st.Obj("suggest").Set("requested", true)
			asks.Note(st.Obj("suggest"), ask)
			return nil
		}
		ctx.Store.Update(wanted, "suggest")
		if command.Force {
			ctx.Dispatch(nil, true, "suggest")
			return "A suggestion survey is starting now."
		}
		return "I will survey for suggestions the next time a subscription is free with " +
			"nothing in the queue it can start."
	}
	if number == 0 {
		return fmt.Sprintf("`%s` needs an issue or a pull request to act on.", verb)
	}
	if slices.Contains(commands.Votes, verb) {
		return AnswerVote(ctx, verb, number, by, ask)
	}
	if verb == "stop" {
		answer := queue.Stop(ctx, number, by)
		if queue.LabelNames(ctx.GH.GetIssue(number))[config.LabelTree] {
			answer += " " + StopTree(ctx, number, by)
		}
		return answer
	}
	if verb == "rebuild" {
		return stepup.RebuildRequest(ctx, number, by)
	}
	if verb == "review" {
		if !isPR {
			return "`review` asks for a review run of one of my pull requests; on an issue, " +
				fmt.Sprintf("`%s build` queues it.", config.Slash)
		}
		first, rest, _ := strings.Cut(command.Args, " ")
		floor := ""
		if slices.Contains([]string{"strong", "medium"}, strings.ToLower(first)) {
			floor = strings.ToLower(first)
		}
		notes := strings.TrimSpace(command.Args)
		if floor != "" {
			notes = strings.TrimSpace(rest)
		}
		return queue.QueueReview(ctx, number, by, floor, notes, ask)
	}
	notesLine := ""
	if command.Args != "" {
		notesLine = " I will read your notes with the rest of the thread."
	}
	if slices.Contains([]string{"oneshot", "split", "fullsend"}, verb) {
		if isPR {
			return fmt.Sprintf("`%s` works on an issue; on a pull request, ask for a revision.", verb)
		}
		first, rest, _ := strings.Cut(command.Args, " ")
		mode := verb
		if verb == "split" && strings.ToLower(first) == "bot" {
			mode = "split-bot"
			notesLine = ""
			if rest != "" {
				notesLine = " I will read your notes with the rest of the thread."
			}
		}
		if !slices.Contains(config.Modes, mode) {
			return notMine(mode)
		}
		return queueBuild(ctx, number, by, command.Force, false, ask, &mode) + notesLine
	}
	if slices.Contains([]string{"build", "revise", "request"}, verb) {
		if isPR {
			return queue.QueueRevise(ctx, number, by, command.Force, "request", false, nil, ask) + notesLine
		}
		// An explicit `build` is a plain build: a mode the issue was queued in comes off.
		var mode *string
		if verb == "build" && len(config.Modes) > 0 {
			plain := ""
			mode = &plain
		}
		return queueBuild(ctx, number, by, command.Force, false, ask, mode) + notesLine
	}
	return fmt.Sprintf("`%s` is not something I can do here.", verb)
}

// queueBuild is `queue_build(ctx, number, by=by, force=force, label_present=label_present,
// ask=ask, mode=mode)`, where Python's `mode=None` (leave the issue's mode labels as they are) is
// a nil `mode` and "" a plain build (its mode labels come off).
func queueBuild(ctx *hctx.Context, number int, by string, force bool, labelPresent bool, ask *asks.Ask, mode *string) string {
	return queue.QueueBuild(ctx, number, by, force, labelPresent, ask, mode)
}

// OpenSuggestion is open_suggestion: issue `number` is one of my suggestions, still open: the one
// thread a bare `approve` or `reject` votes on (#760). Anywhere else the words are only words.
func OpenSuggestion(ctx *hctx.Context, number int) bool {
	if number == 0 || !config.Identity.Suggestions {
		return false
	}
	var issue *pyjson.Object
	if _, ok := herr.Catch[*herr.GitHubError](func() { issue = ctx.GH.GetIssue(number) }); ok {
		return false
	}
	return issue.Get("state") == "open" && !issue.Has("pull_request") &&
		queue.LabelNames(issue)[config.LabelSuggestion]
}

// AnswerVote is answer_vote: a person's vote on one of my suggestions (#760): `approve` queues it
// to build, which takes `bot:suggestion` off (`queue_build`), as `bot:build` does; `reject` closes
// it as not planned, which frees its place under `suggestions.max_open` for the next survey, and
// goes into the log of suggestions that came to nothing (`declined`, #797). A rejected one keeps
// its label, so it still reads as a suggestion, and reopened it can be approved.
func AnswerVote(ctx *hctx.Context, vote string, number int, by string, ask *asks.Ask /* = None */) string {
	if vote == "approve" {
		return queueBuild(ctx, number, by, false, false, ask, nil)
	}
	issue := ctx.GH.GetIssue(number)
	if issue.Get("state") != "open" {
		return fmt.Sprintf("#%d is closed already, so there was nothing to reject.", number)
	}
	ctx.GH.UpdateIssue(number, pyjson.New("state", "closed", "state_reason", "not_planned"))
	// Logged, so no survey offers it again however long ago it was (#797).
	ctx.Store.Update(func(s *pyjson.Object) any {
		declined.Note(s, number, py.OrStr(issue.Get("title"), ""), "rejected by @"+by, clock.ISO(ctx.Now()))
		return nil
	}, fmt.Sprintf("suggestion #%d rejected", number))
	return fmt.Sprintf("Rejected: I closed #%d without building it. Reopen it and comment `approve` ", number) +
		"to change that."
}

// notMine is _not_mine: the reply to a verb this bot does not have, naming the bot that does
// (#60).
func notMine(verb string) string {
	if verb == "suggest" {
		return config.Identity.Title + " makes no suggestions."
	}
	var names []string
	for _, o := range config.Others {
		names = append(names, fmt.Sprintf("%s (@%s)", o.Name, o.Login))
	}
	hint := ""
	if len(names) > 0 {
		hint = " " + strings.Join(names, ", ") + " may: ask there."
	}
	return fmt.Sprintf("I have no `%s` mode.%s", verb, hint)
}

// StopTree is stop_tree: `stop` on a parent the bot split (#60): its open sub-issues leave the
// queue too. Its own are stopped here; one queued for another bot gets that bot's own `stop`
// command, so it stops its work the way it always does.
func StopTree(ctx *hctx.Context, parent int, by string) string {
	var stopped []string
	var children []*pyjson.Object
	if exc, ok := herr.Catch[*herr.GitHubError](func() { children = ctx.GH.ListSubIssues(parent) }); ok {
		return fmt.Sprintf("Its sub-issues could not be read (%s), so they are still queued.", py.SliceTo(exc.Error(), 200))
	}
	for _, child := range children {
		if child.Get("state") != "open" {
			continue
		}
		number := py.MustInt(py.Item(child, "number"))
		var other = -1
		names := map[string]bool{}
		for n := range queue.LabelNames(child) {
			names[strings.ToLower(n)] = true
		}
		for i, candidate := range config.Others {
			for n := range names {
				if strings.HasPrefix(n, candidate.LabelPrefix) {
					other = i
					break
				}
			}
		}
		if other < 0 {
			queue.Stop(ctx, number, by)
		} else {
			ctx.GH.CreateComment(number, fmt.Sprintf("@%s stop\n\n%s stopped the tree this ", config.Others[other].Login, by)+
				fmt.Sprintf("belongs to (#%d).", parent))
		}
		stopped = append(stopped, fmt.Sprintf("#%d", number))
	}
	if len(stopped) == 0 {
		return "It has no open sub-issues."
	}
	return fmt.Sprintf("Its open sub-issues leave the queue too: %s.", strings.Join(stopped, ", "))
}

// subscription is _subscription: the subscription a `suspend` or `resume` names, or None and the
// reply saying why not.
func subscription(ctx *hctx.Context, word string) (*providers.Provider, string) {
	name := strings.ToLower(strings.Trim(word, "`:,.!?"))
	provider := ctx.Cfg.Pool.Get(name)
	if provider != nil {
		return provider, ""
	}
	var known []string
	for _, p := range ctx.Cfg.Pool.Ordered() {
		known = append(known, "`"+p.ID+"`")
	}
	return nil, fmt.Sprintf("`%s` is not a subscription; I know %s. Nothing was done.", name, strings.Join(known, ", "))
}

// firstSubscription is `_subscription(ctx, args.split()[0])[0]`.
func firstSubscription(ctx *hctx.Context, args string) *providers.Provider {
	fields := strings.Fields(args)
	if len(fields) == 0 {
		panic(&py.Error{Type: "IndexError", Msg: "list index out of range"})
	}
	provider, _ := subscription(ctx, fields[0])
	return provider
}

var resumeWord = regexp.MustCompile(`(?:^|[\s/])resume\b`)

// saidResume is _said_resume: the line's verb was `resume`, the alias that always names a
// subscription.
func saidResume(command *commands.Command) bool {
	return resumeWord.MatchString(strings.ToLower(command.Line))
}

// resume is _resume: `start <subscription>` (`resume claude-3`): lift that one's suspension, and
// no halt.
func resume(ctx *hctx.Context, command *commands.Command) string {
	fields := strings.Fields(command.Args)
	if len(fields) == 0 {
		panic(&py.Error{Type: "IndexError", Msg: "list index out of range"})
	}
	provider, problem := subscription(ctx, fields[0])
	if provider == nil {
		return problem + fmt.Sprintf(" `%s start` alone lifts a halt.", config.Slash)
	}
	if providers.Suspension(ctx.Store.Load(), provider.ID) == nil {
		return fmt.Sprintf("`%s` is not suspended, so there was nothing to lift.", provider.ID)
	}
	ctx.Store.Update(func(s *pyjson.Object) any {
		providers.Record(s, provider.ID).Pop("suspended")
		return nil
	}, "resume "+provider.ID)
	answer := fmt.Sprintf("Resumed `%s`: the bot spends it again as soon as it is free.", provider.ID)
	if ctx.Store.Load().Truthy("halted") {
		answer += fmt.Sprintf(" The bot is still halted, though: `%s start` lifts that.", config.Slash)
	}
	if command.Force {
		answer += " " + runNow(ctx, nil)
	}
	return answer
}

// runNow is _run_now: ask for a run now. The request is written down first, so a run that GitHub
// replaces or refuses to start is made up by the next hourly one.
func runNow(ctx *hctx.Context, item *int) string {
	ctx.Store.Update(func(s *pyjson.Object) any {
		var it any
		if item != nil {
			it = *item
		}
		s.Set("run_requested", pyjson.New("at", clock.ISO(ctx.Now()), "item", it))
		return nil
	}, "run requested")
	which := ""
	if item != nil && *item != 0 {
		which = fmt.Sprintf(" for #%d", *item)
	}
	if exc, ok := herr.Catch[*herr.GitHubError](func() { ctx.Dispatch(item, true, "auto") }); ok {
		return fmt.Sprintf("Starting a run%s failed (%s); the next hourly run does it ", which, py.SliceTo(exc.Error(), 200)) +
			"instead."
	}
	return fmt.Sprintf("A run%s is starting now, outside a subscription's hours if need be.", which)
}

// OnReview is on_review: a review: its body's commands, a hint when it names the bot, or a
// revision when a trusted person asks a bot pull request for changes. Claimed like a comment, so
// it runs once.
func OnReview(ctx *hctx.Context, payload *pyjson.Object) []string {
	review := payload.Obj("review")
	pull := payload.Obj("pull_request")
	user := review.Obj("user")
	lvl := level(ctx, user, review.Get("author_association"))
	if lvl <= 0 {
		return []string{fmt.Sprintf("ignored: @%s is not on the trust list", py.Str(user.Get("login")))}
	}
	if FromOtherBot(user) {
		return []string{"ignored: another bot's review"}
	}
	body := py.OrStr(review.Get("body"), "")
	found := commands.Parse(body, ctx.Cfg.BotLogin)
	changes := queue.LabelNames(pull)[config.LabelPR] && lvl >= 2 &&
		strings.ToLower(py.Str(review.GetOr("state", ""))) == "changes_requested"
	nudged := len(found) == 0 && !changes && commands.NamesTheBot(body, ctx.Cfg.BotLogin)
	if !(len(found) > 0 || changes || nudged) {
		return []string{"no command in the review"}
	}
	var lines []string
	for _, c := range found {
		lines = append(lines, c.Line)
	}
	if len(lines) == 0 {
		if changes {
			lines = []string{"(changes requested)"}
		} else {
			lines = []string{"(named the bot)"}
		}
	}
	if len(Claim(ctx, "r:"+py.Str(review.Get("id")), lines)) == 0 {
		return []string{"ignored: already answered"}
	}
	number := py.MustInt(py.Item(pull, "number"))
	login := py.Str(user.GetOr("login", ""))
	var replies []string
	if len(found) > 0 {
		replies = RunCommands(ctx, found, pyjson.New("number", number, "pull_request", pyjson.New()), user, lvl, nil)
	} else if changes {
		replies = []string{queue.QueueRevise(ctx, number, login, false, "request", false, nil, nil)}
	} else {
		replies = []string{nudge(ctx.Cfg.BotLogin)}
	}
	reply(ctx, number, "@"+login+"\n\n"+strings.Join(replies, "\n\n"))
	return replies
}

// OnIssueChange is on_issue_change.
func OnIssueChange(ctx *hctx.Context, payload *pyjson.Object, isPR bool) []string {
	var thread *pyjson.Object
	if isPR {
		thread = payload.Obj("pull_request")
	} else {
		thread = payload.Obj("issue")
	}
	sender := payload.Obj("sender")
	// Only people with triage access can label or assign, so GitHub has vouched for the sender.
	lvl := trustLevel(ctx.Trust, py.Str(sender.GetOr("login", "")), sender.Get("id"), "COLLABORATOR")
	number := py.MustInt(py.Or(thread.Get("number"), 0))
	action := payload.Get("action")
	var mode *string
	if action == "labeled" {
		name := py.Str(payload.Obj("label").GetOr("name", ""))
		// Either queue label works on either kind of thread: an issue builds, a PR revises. A
		// mode's label (#60) queues the issue in that mode.
		m, isMode := modeLabel(name)
		wanted := name == config.LabelBuild || name == config.LabelRevise || (isMode && !isPR)
		if !wanted {
			return []string{"ignored: label " + name}
		}
		if isMode {
			mode = &m
		}
	} else if action == "assigned" {
		assignee := py.Str(payload.Obj("assignee").GetOr("login", ""))
		wanted := strings.ToLower(assignee) == strings.ToLower(ctx.Cfg.BotLogin)
		if !wanted {
			return []string{"ignored: assigned to " + assignee}
		}
	}
	if lvl < 2 {
		if action == "labeled" {
			ctx.GH.RemoveLabel(number, py.Str(payload.Obj("label").GetOr("name", "")))
		}
		return []string{fmt.Sprintf("ignored: @%s is below the maintainer level", py.Str(sender.Get("login")))}
	}
	by := py.Str(sender.GetOr("login", ""))
	var answer string
	if isPR {
		answer = queue.QueueRevise(ctx, number, by, false, "request", action == "labeled", nil, nil)
	} else {
		answer = queueBuild(ctx, number, by, false, action == "labeled", nil, mode)
	}
	ctx.GH.CreateComment(number, "@"+by+" "+answer)
	return []string{answer}
}

// DropWip is _drop_wip: a closed or merged bot pull request's unfinished revision
// (`bot/wip/<pr>`, #313) is done with: delete the branch and drop the record's `wip`.
func DropWip(ctx *hctx.Context, number int) {
	if !ctx.Store.Load().Obj("items").Obj(strconv.Itoa(number)).Truthy("wip") {
		return
	}
	herr.Catch[*herr.GitHubError](func() { ctx.GH.DeleteBranch(queue.WipBranch(number)) }) // gone already
	ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, number).Pop("wip")
		return nil
	}, fmt.Sprintf("wip #%d closed", number))
}

// OnPullClosed is on_pull_closed.
func OnPullClosed(ctx *hctx.Context, payload *pyjson.Object) []string {
	pull := payload.Obj("pull_request")
	if !queue.LabelNames(pull)[config.LabelPR] {
		return []string{"ignored: not a bot pull request"}
	}
	issue := threads.LinkedIssue(pull)
	number := py.MustInt(py.Item(pull, "number"))
	ctx.Store.Update(func(s *pyjson.Object) any {
		rec := state.Item(s, number)
		rec.Set("closed_at", clock.ISO(ctx.Now()))
		rec.Set("merged", pull.Truthy("merged"))
		return nil
	}, fmt.Sprintf("closed #%d", number))
	DropWip(ctx, number)
	if pull.Truthy("merged") {
		// merged: approved and green, so its strikes are over
		herr.Catch[*herr.GitHubError](func() { stepup.Clear(ctx, number) })
	}
	if issue == nil {
		return []string{"closed; no linked issue"}
	}
	ctx.GH.RemoveLabel(*issue, config.LabelPROpen)
	if ctx.Store.Load().Obj("items").Obj(strconv.Itoa(number)).Truthy("rebuilt") {
		return []string{fmt.Sprintf("#%d closed to build #%d again", number, *issue)} // `stepup.rebuild` said why
	}
	if !pull.Truthy("merged") {
		ctx.GH.CreateComment(*issue, fmt.Sprintf("#%d was closed without merging. Comment ", number)+
			fmt.Sprintf("`%s build` to have me try again.", config.Slash))
		return []string{fmt.Sprintf("#%d closed unmerged", number)}
	}
	// "Closes #N" in the body should do this, but GitHub doesn't always: #43 merged and left #39 open.
	if pull.Obj("base").Get("ref") == ctx.Cfg.DefaultBranch &&
		ctx.GH.GetIssue(*issue).Get("state") == "open" {
		ctx.GH.UpdateIssue(*issue, pyjson.New("state", "closed", "state_reason", "completed"))
		return []string{fmt.Sprintf("#%d merged; closed #%d", number, *issue)}
	}
	return []string{fmt.Sprintf("#%d merged; #%d closes with it", number, *issue)}
}

// OnCI is on_ci.
func OnCI(ctx *hctx.Context, payload *pyjson.Object) []string {
	run := payload.Obj("workflow_run")
	if !py.In(run.Get("name"), CIWorkflows(ctx)...) {
		return []string{"ignored: workflow " + py.Str(run.Get("name"))}
	}
	if !py.In(run.Get("conclusion"), CIFailed...) {
		return []string{"ignored: conclusion " + py.Str(run.Get("conclusion"))}
	}
	branch := py.OrStr(run.Get("head_branch"), "")
	if strings.HasPrefix(branch, QueueBranch) {
		return OnQueueCI(ctx, run)
	}
	var numbers []int
	for _, p := range pyjson.Objs(run.Get("pull_requests")) {
		if p.Truthy("number") {
			numbers = append(numbers, py.MustInt(py.Item(p, "number")))
		}
	}
	if len(numbers) == 0 && branch != "" {
		for _, p := range ctx.GH.ListPulls("open", branch, 200) {
			numbers = append(numbers, py.MustInt(py.Item(p, "number")))
		}
	}
	var out []string
	for _, number := range numbers {
		pull := ctx.GH.GetPull(number)
		if pull.Get("state") != "open" || !pyjson.Equal(pull.Obj("head").Get("sha"), run.Get("head_sha")) {
			out = append(out, fmt.Sprintf("#%d: stale run", number))
			continue
		}
		names := queue.LabelNames(pull)
		if !names[config.LabelPR] {
			out = append(out, fmt.Sprintf("#%d: not a bot pull request", number))
			continue
		}
		record := ctx.Store.Load().Obj("items").Obj(strconv.Itoa(number))
		if intersects(names, config.LabelBlocked, config.LabelRevise, config.LabelWorking) || record.Truthy("stop_requested") {
			out = append(out, fmt.Sprintf("#%d: blocked, stopped or already queued; left alone", number))
			continue
		}
		sha := py.Str(run.Get("head_sha"))
		reruns := record.Obj("ci_reruns").Copy()
		if py.MustInt(reruns.GetOr(sha, 0)) < ctx.Cfg.CIReruns {
			ctx.Act().RerunFailedJobs(py.Item(run, "id"))
			reruns.Set(sha, py.MustInt(reruns.GetOr(sha, 0))+1)
			ctx.Store.Update(func(s *pyjson.Object) any {
				state.Item(s, number).Set("ci_reruns", reruns)
				return nil
			}, fmt.Sprintf("ci rerun #%d", number))
			out = append(out, fmt.Sprintf("#%d: re-ran the failed jobs of run %s", number, py.Str(py.Item(run, "id"))))
			continue
		}
		out = append(out, fixCI(ctx, number, pull, names, record, run, sha, "CI failed again"))
	}
	if len(out) == 0 {
		return []string{"no open pull request for this run"}
	}
	return out
}

// fixCI is _fix_ci: what a red head gets once its one re-run is spent: a strike, a stop once the
// CI fixes are used up, or else a CI fix with `run`'s logs. `failed` opens the comment.
func fixCI(ctx *hctx.Context, number int, pull *pyjson.Object, names map[string]bool,
	record *pyjson.Object, run *pyjson.Object, sha string, failed string) string {
	fixes := py.MustInt(record.GetOr("ci_fixes", 0))
	link := "no CI run found"
	if run.Truthy("html_url") {
		link = fmt.Sprintf("[run](%s)", py.Str(run.Get("html_url")))
	}
	if fixes != 0 && py.OrStr(record.Get("ci_fixed_head"), "") == sha {
		// Every CI fix that left it red is a strike of its own (#316), not only the last one:
		// this is the head the last CI fix pushed (`deliver._revise`). A head some other
		// revision made is no strike, whatever fixes came before.
		var stepped bool
		if _, ok := herr.Catch[*herr.GitHubError](func() {
			stepped = stepup.Strike(ctx, number, fmt.Sprintf("CI still failed after %d fix(es)", fixes), link)
		}); ok {
			stepped = false
		}
		if stepped {
			return fmt.Sprintf("#%d: stepped up after %d CI fixes", number, fixes)
		}
	}
	if fixes >= ctx.Cfg.MaxFailures {
		queue.SetStateLabel(ctx, number, names, config.LabelBlocked)
		if pull.Truthy("auto_merge") {
			herr.Catch[*herr.GitHubError](func() { ctx.GH.DisableAutoMerge(py.Str(py.Item(pull, "node_id"))) })
		}
		ctx.GH.CreateComment(number, fmt.Sprintf("%s (%s) after %d tries at ", failed, link, fixes)+
			"fixing it, so I stopped. It needs a person.")
		return fmt.Sprintf("#%d: blocked after %d CI fixes", number, fixes)
	}
	answer := queue.QueueRevise(ctx, number, ctx.Cfg.BotLogin, false, "ci", false,
		pyjson.New("ci_run_id", run.Get("id"), "ci_sha", sha, "ci_fixes", fixes+1), nil)
	ctx.GH.CreateComment(number, fmt.Sprintf("%s (%s). %s", failed, link, answer))
	return fmt.Sprintf("#%d: queued a CI fix", number)
}

// QueueBranch is QUEUE_BRANCH: the branches `main`'s merge queue builds on:
// `gh-readonly-queue/<base>/pr-<number>-<sha>`.
var QueueBranch = "gh-readonly-queue/"

// OnQueueCI is on_queue_ci: CI failed on the merge queue's build of a pull request
// (infra/github.tf): `main` with every pull request ahead of it and this one on top. Its own CI ran
// on an older `main`, so the rest of `on_ci` never hears of it. One still in the queue was built
// again without a pull request ahead of it that failed, and is left to the queue. One taken out
// failed itself, and gets a CI fix with this run's logs at once, with no re-run: its own CI
// already passed, so what failed is nearly always what `main` and it do together, which fails the
// same way again (four of the five red runs on main of 2026-10-29). The revision merges `main`
// first and is reviewed as any, and its auto-merge queues it again at the back. A held pull
// request (a revision, a stop, `queue.hold_merge`) and a head the reviews never cleared are left
// alone.
func OnQueueCI(ctx *hctx.Context, run *pyjson.Object) []string {
	pattern := regexp.MustCompile("^" + QueueBranch + regexp.QuoteMeta(ctx.Cfg.DefaultBranch) + `/pr-(\d+)-`)
	found := pattern.FindStringSubmatch(py.OrStr(run.Get("head_branch"), ""))
	if found == nil {
		return []string{fmt.Sprintf("ignored: %s is not a build of the default branch's queue", py.Str(run.Get("head_branch")))}
	}
	number := py.MustInt(found[1])
	pull := ctx.GH.GetPull(number)
	if !queue.LabelNames(pull)[config.LabelPR] {
		return []string{fmt.Sprintf("#%d: not a bot pull request", number)}
	}
	if pull.Get("state") != "open" {
		return []string{fmt.Sprintf("#%d: closed", number)}
	}
	var queued bool
	if exc, ok := herr.Catch[*herr.GitHubError](func() { queued = ctx.GH.InMergeQueue(py.Str(py.Item(pull, "node_id"))) }); ok {
		return []string{fmt.Sprintf("#%d: could not read whether it is still in the merge queue (%s)", number, exc.Error())}
	}
	if queued {
		return []string{fmt.Sprintf("#%d: still in the merge queue, built again without what failed ahead of it", number)}
	}
	names := queue.LabelNames(pull)
	record := ctx.Store.Load().Obj("items").Obj(strconv.Itoa(number))
	if intersects(names, config.LabelBlocked, config.LabelRevise, config.LabelWorking, config.LabelReady, config.LabelPersonMerges) ||
		queue.IsHuman(names) || record.Truthy("stop_requested") {
		return []string{fmt.Sprintf("#%d: out of the merge queue, and held, so left alone", number)}
	}
	sha := py.OrStr(pull.Obj("head").Get("sha"), "")
	if !pyjson.Equal(queue.Cleared(record).Get("sha"), sha) {
		return []string{fmt.Sprintf("#%d: out of the merge queue, and its head is not the cleared one", number)}
	}
	return []string{fixCI(ctx, number, pull, names, record, run, sha,
		"The merge queue took this out: CI failed on `main` with it on top")}
}
