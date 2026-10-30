// Package stats is bot/harness/stats.py.
//
// The statistics in the pinned status issue (#148): what the night bot has done over the last six
// hours, the last day, the last week and all time, counted afresh every 30 minutes by the
// `bot-status` loop (`dashboard --stats`) and kept in the issue between counts (`kept`).
//
// Where the rest of the issue says what the bot is doing now, this block keeps the record. It opens
// with one table that sets the four windows side by side and charts of the last two weeks, then
// folds each window into a section of its own: what each subscription did in it (plans, builds,
// revisions, reviews, pull requests opened and merged, lines merged, pauses, failures, model time,
// tokens and cost) with bar charts of runs and lines by subscription; then every pull request
// merged, newest first, with who planned, built, revised and approved it. The charts are bars and
// lines, never pies. Everything is read from GitHub and the state file each count, so it holds
// nothing of its own but when it last counted:
//
//   - **Runs and outcomes** come from the bot's own comments. A run's starting comment ("Starting work
//     on this now …, on `claude-1` (claude, `opus`, strong)") names its subscription and model, and
//     every later comment carrying the same run link is joined to it.
//   - **Who did what to a pull request**: its builder from the "Opened #n, built on …" comment on its
//     issue (for one opened before that comment named a builder, the last build started on its issue
//     before it was opened); its planner from "Planned on …" on its issue; its revisers from
//     "Revision pushed … on …" on it; its approvers from the seats a comment says approved it.
//   - **Pull requests, merges, commits and lines** come from the bot's pull requests; the closed
//     issues from their `Closes #n` lines; commits on the default branch from the commit list.
//   - **Model minutes** come from each subscription's `spent` record in the state file. Every entry is
//     dated, so a window sums the entries in it; all time is its last `providers.SPENT_KEEP` runs.
//   - **Model tokens and cost** come from each subscription's `tokens_spent` record beside it
//     (#695): per run its input, output and cached tokens and its USD cost, summed the same way.
//
// Squishy (#60) has a folded section of its own at the end, apart from the night bot's record: the
// same four windows from its own comments, pull requests, runs and state, and its modes (one-shot
// builds, splits, the sub-issues they opened, the trees closed). Squishy's own process draws it
// (`bot_section`), and the night bot's loop runs that process at each count and puts it in
// (`block`'s `extra`).
//
// This used to be an issue of its own, "Night bot statistics" (#190); it is now part of the status
// issue, so one pinned issue says what the bot is doing and what it has done.
package stats

import (
	"regexp"
	"slices"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Where the block starts and ends in the status issue, so a rewrite between counts keeps it.
var (
	BlockStart = "<!-- jackioh-bot:statistics -->"
	BlockEnd   = "<!-- /jackioh-bot:statistics -->"
)

// StatsEvery is how often the loop counts afresh: often enough that the six-hour window is current.
// (A count reads every bot comment and pull request, a few hundred calls, so not at every rewrite.)
var StatsEvery = 30 * time.Minute

// NamedSpan is one of WINDOWS: a window's name and how far back it reaches, nil for all time.
type NamedSpan struct {
	Name string
	Span *time.Duration
}

func spanOf(d time.Duration) *time.Duration { return &d }

// Windows are the four windows the issue reports, newest first; a nil span is all time.
var Windows = []NamedSpan{
	{"Last 6 hours", spanOf(6 * time.Hour)},
	{"Last 24 hours", spanOf(24 * time.Hour)},
	{"Last 7 days", spanOf(7 * 24 * time.Hour)},
	{"All time", nil},
}

// MergesShown is how many merged pull requests the block lists, newest first; the status issue
// lists fewer when its body would pass GitHub's limit (`dashboard.fit`).
var MergesShown = 40

var (
	TitleShown    = 70
	DaysShown     = 14
	LimitComments = 5000
)

// Unrecorded names a pull request whose builder no comment names.
var Unrecorded = "not recorded"

// Ever is before the bot's first comment: `list_repo_comments` takes a "since".
var Ever = "2000-01-01T00:00:00Z"

var (
	LimitPulls = 500
	LimitRuns  = 1000
)

var RunLink = regexp.MustCompile(`actions/runs/(\d+)`)

// Seat matches "`claude-1` (claude, `opus`, strong)", "`gpt` (codex, gpt-5.6-sol)": a subscription,
// its CLI and the model; the first one a comment names is the run's builder (or planner, or
// reviewer).
var Seat = regexp.MustCompile("`([a-z][a-z0-9-]*)` \\(([a-z]+), `?([A-Za-z0-9][A-Za-z0-9.\\-]*)`?")

// ApprovedBy matches a seat the comment says approved the change: "Its adversarial reviewer,
// `claude-2` (claude, `opus`, strong), approved it", "`claude-2` (…) reviewed it adversarially and
// approved it", "Second review (…): `claude-2` (…) approved it".
var ApprovedBy = regexp.MustCompile("`([a-z][a-z0-9-]*)` \\([a-z]+, `?[A-Za-z0-9][A-Za-z0-9.\\-]*`?" +
	`(?:, [a-z]+)?\),? (?:reviewed it adversarially and )?approved it`)

var Opened = regexp.MustCompile(`\bOpened #(\d+)`)

// SplitInto matches a split's checklist: "Split #40 into 3 sub-issue(s) (…)".
var SplitInto = regexp.MustCompile(`\bSplit #\d+ into (\d+) sub-issue`)

var Closes = regexp.MustCompile(`(?i)\b(?:closes|fixes|resolves) #(\d+)`)

// BranchIssue is `^<prefix>issue-(\d+)$`; Python's `$` also matches before a final newline, hence
// the `\n?`.
var BranchIssue = regexp.MustCompile(`^` + regexp.QuoteMeta(config.BranchPrefix) + `issue-(\d+)\n?$`)

// OutcomeWords is one of OUTCOMES: a kind of comment and the words that mark it.
type OutcomeWords struct {
	Kind  string
	Words []string
}

// Outcomes is what a comment says happened, tried in this order: (kind, words that mark it).
var Outcomes = []OutcomeWords{
	{"start", []string{"Starting work on this now", "Starting a revision now", "Starting a review now",
		"Planning this now", "Starting a one-shot build", "Splitting this now"}},
	{"opened", []string{"Opened #"}},
	{"revised", []string{"Revision pushed"}},
	{"planned", []string{"Planned on"}},
	// Squishy's splits (#60): the checklist of a split, and a tree whose close-out found it done.
	{"split", []string{"sub-issue(s) ("}},
	{"closed tree", []string{"end state holds on"}},
	{"approved", []string{"approved it. Auto-merge", "approved it, so"}},
	{"paused", []string{"Paused:", "was cut short", "reached its usage limit", "stops mid-call"}},
	{"not approved", []string{"did not pass review after", "still had blocking findings"}},
	{"infra", []string{"could not work on this", "died on", "left no result"}},
	{"failed", []string{"This run failed", "failed 3 times", "second review failed"}},
	{"queued", []string{"Queued"}},
}

// ActionWords is one of ACTIONS: the words that start a run, and what the run does.
type ActionWords struct {
	Words string
	Name  string
}

var Actions = []ActionWords{{"Starting work on this now", "build"}, {"Starting a revision now", "revise"},
	{"Starting a review now", "review"}, {"Planning this now", "plan"},
	{"Starting a one-shot build", "oneshot"}, {"Splitting this now", "split"}}

// The outcomes that count as work done, and the ones that are not.
var (
	Done    = []string{"opened", "revised", "planned", "approved", "split", "closed tree"}
	NotDone = []string{"paused", "not approved", "failed", "infra"}
)

// outcomeKinds is DONE + NOT_DONE.
func outcomeKinds() []string { return append(append([]string{}, Done...), NotDone...) }

// Event is stats.Event.
type Event struct {
	At        string
	Number    int
	Kind      string
	Run       string
	Provider  string
	Model     string
	Action    string
	PR        *int
	Approvers []string
	// For a split: how many sub-issues it opened.
	Count int
}

// SpentEntry is one of a subscription's dated model minutes in Facts.Spent: (when, minutes).
type SpentEntry struct {
	At      string
	Minutes float64
}

// TokenEntry is one of a subscription's dated token counts and cost in Facts.Counted: (when,
// input, output, cache, USD).
type TokenEntry struct {
	At                   string
	Input, Output, Cache int
	Cost                 float64
}

// Facts is everything the issue shows, gathered once. Build it with NewFacts.
type Facts struct {
	Events        []*Event
	Pulls         []*pyjson.Object
	CommitsOnMain int
	// `bot-night.yml`'s runs: (when it started, how it ended).
	Runs [][2]string
	// Each subscription's dated model minutes: provider id -> []SpentEntry.
	Spent *pyjson.Object
	// Each subscription's dated token counts and cost: provider id -> []TokenEntry.
	Counted *pyjson.Object
}

// NewFacts is Facts() with its defaults.
func NewFacts() *Facts { return &Facts{Spent: pyjson.New(), Counted: pyjson.New()} }

func spentOf(facts *Facts, pid string) []SpentEntry {
	entries, _ := facts.Spent.Get(pid).([]SpentEntry)
	return entries
}

func countedOf(facts *Facts, pid string) []TokenEntry {
	entries, _ := facts.Counted.Get(pid).([]TokenEntry)
	return entries
}

// Classify is stats.classify.
func Classify(body string) string {
	for _, outcome := range Outcomes {
		for _, word := range outcome.Words {
			if strings.Contains(body, word) {
				return outcome.Kind
			}
		}
	}
	return "other"
}

// Parse is one bot comment as an event: what happened, in which run, on which subscription.
func Parse(comment *pyjson.Object) *Event {
	body := py.OrStr(comment.Get("body"), "")
	issueURL := strings.TrimRight(py.Str(py.Or(comment.Get("issue_url"), "0")), "/")
	last := issueURL[strings.LastIndex(issueURL, "/")+1:]
	number := 0
	if last != "" {
		number = py.MustInt(last)
	}
	kind := Classify(body)
	run := RunLink.FindStringSubmatch(body)
	seat := Seat.FindStringSubmatch(body)
	opened := Opened.FindStringSubmatch(body)
	action := ""
	for _, a := range Actions {
		if strings.Contains(body, a.Words) {
			action = a.Name
			break
		}
	}
	approvers := []string{}
	for _, m := range ApprovedBy.FindAllStringSubmatch(body, -1) {
		if !slices.Contains(approvers, m[1]) {
			approvers = append(approvers, m[1])
		}
	}
	split := SplitInto.FindStringSubmatch(body)
	event := &Event{At: py.OrStr(comment.Get("created_at"), ""), Number: number, Kind: kind,
		Action: action, Approvers: approvers}
	if run != nil {
		event.Run = run[1]
	}
	if seat != nil {
		event.Provider, event.Model = seat[1], seat[3]
	}
	if opened != nil {
		pr := py.MustInt(opened[1])
		event.PR = &pr
	}
	if split != nil {
		event.Count = py.MustInt(split[1])
	}
	return event
}

// Joined is each event with its run's subscription and model when it names none itself.
func Joined(events []*Event) []*Event {
	seats := map[string][2]string{}
	for _, event := range events {
		if event.Kind == "start" && event.Run != "" && event.Provider != "" {
			if _, ok := seats[event.Run]; !ok {
				seats[event.Run] = [2]string{event.Provider, event.Model}
			}
		}
	}
	for _, event := range events {
		if seat, ok := seats[event.Run]; event.Provider == "" && ok {
			event.Provider, event.Model = seat[0], seat[1]
		}
	}
	return events
}

// ------------------------------------------------------------------ gathering

// Collect is stats.collect; a variable so the tests can replace it.
var Collect = collect

func loginOf(thing *pyjson.Object) string {
	return strings.ToLower(py.OrStr(thing.Obj("user").Get("login"), ""))
}

func collect(ctx *hctx.Context) *Facts {
	gh, bot := ctx.GH, strings.ToLower(ctx.Cfg.BotLogin)
	facts := NewFacts()
	events := []*Event{}
	for _, c := range gh.ListRepoComments(Ever, LimitComments) {
		if loginOf(c) == bot {
			events = append(events, Parse(c))
		}
	}
	sort.SliceStable(events, func(i, j int) bool { return events[i].At < events[j].At })
	facts.Events = Joined(events)
	for _, pull := range gh.ListPulls("all", "", LimitPulls) {
		if loginOf(pull) != bot {
			continue
		}
		var detail *pyjson.Object
		if _, failed := herr.Catch[*herr.GitHubError](func() {
			detail = gh.GetPull(py.MustInt(pull.Get("number")))
		}); failed {
			detail = pull
		}
		facts.Pulls = append(facts.Pulls, pull.Copy().Update(detail))
	}
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		facts.CommitsOnMain = len(gh.ListCommits(ctx.Cfg.BotLogin, LimitRuns))
	}); failed {
		facts.CommitsOnMain = 0
	}
	for _, run := range gh.ListRuns(config.NightWorkflow, "", LimitRuns, false) {
		ended := run.Get("conclusion")
		if !pyjson.Truthy(ended) {
			ended = run.Get("status")
		}
		facts.Runs = append(facts.Runs, [2]string{py.OrStr(run.Get("created_at"), ""), py.Str(py.Or(ended, "?"))})
	}
	providers := ctx.Store.Load().Obj("providers")
	for _, providerID := range providers.Keys() {
		record := providers.Obj(providerID)
		spent := []SpentEntry{}
		for _, entry := range py.Each(record.Get("spent")) {
			e := pyjson.Obj(entry)
			spent = append(spent, SpentEntry{py.OrStr(e.Get("at"), ""), py.MustFloat(py.Or(e.Get("minutes"), 0))})
		}
		facts.Spent.Set(providerID, spent)
		counted := []TokenEntry{}
		for _, entry := range py.Each(record.Get("tokens_spent")) {
			e := pyjson.Obj(entry)
			counted = append(counted, TokenEntry{py.OrStr(e.Get("at"), ""), py.MustInt(py.Or(e.Get("input"), 0)),
				py.MustInt(py.Or(e.Get("output"), 0)), py.MustInt(py.Or(e.Get("cache"), 0)), py.MustFloat(py.Or(e.Get("cost"), 0))})
		}
		facts.Counted.Set(providerID, counted)
	}
	return facts
}

// ------------------------------------------------------------------ who did what

// Credits is who planned, built, revised and approved each of the bot's pull requests, by pull
// request number. Build it with NewCredits.
type Credits struct {
	Built    map[int]string
	Planned  map[int][]string
	Revised  map[int][]string
	Approved map[int][]string
}

// NewCredits is Credits() with its defaults.
func NewCredits() *Credits {
	return &Credits{Built: map[int]string{}, Planned: map[int][]string{}, Revised: map[int][]string{},
		Approved: map[int][]string{}}
}

func (x *Credits) Builder(pull *pyjson.Object) string {
	if built, ok := x.Built[py.MustInt(pull.Get("number"))]; ok {
		return built
	}
	return Unrecorded
}

// IssueOf is the issue a pull request builds: its branch `bot/issue-<n>`, or its first `Closes #n`.
func IssueOf(pull *pyjson.Object) *int {
	if branch := BranchIssue.FindStringSubmatch(py.OrStr(pull.Obj("head").Get("ref"), "")); branch != nil {
		n := py.MustInt(branch[1])
		return &n
	}
	if closes := Closes.FindStringSubmatch(py.OrStr(pull.Get("body"), "")); closes != nil {
		n := py.MustInt(closes[1])
		return &n
	}
	return nil
}

// Credits_ is stats.credits.
func Credits_(events []*Event, pulls []*pyjson.Object) *Credits {
	out := NewCredits()
	for _, event := range events {
		if event.Kind == "opened" && event.PR != nil && event.Provider != "" {
			if _, ok := out.Built[*event.PR]; !ok {
				out.Built[*event.PR] = event.Provider
			}
		}
	}

	add := func(table map[int][]string, number int, provider string) {
		if provider == "" {
			return
		}
		if _, ok := table[number]; !ok {
			table[number] = []string{}
		}
		if !slices.Contains(table[number], provider) {
			table[number] = append(table[number], provider)
		}
	}

	for _, pull := range pulls {
		number, issue := py.MustInt(pull.Get("number")), IssueOf(pull)
		openedAt := py.OrStr(pull.Get("created_at"), "")
		if _, built := out.Built[number]; !built && issue != nil {
			var builds []*Event
			for _, e := range events {
				if e.Number == *issue && e.Kind == "start" && e.Action == "build" && e.Provider != "" &&
					e.At <= openedAt {
					builds = append(builds, e)
				}
			}
			if len(builds) > 0 {
				out.Built[number] = builds[len(builds)-1].Provider
			}
		}
		for _, event := range events {
			if event.Kind == "planned" && issue != nil && event.Number == *issue {
				add(out.Planned, number, event.Provider)
			}
			if event.Kind == "revised" && event.Number == number {
				add(out.Revised, number, event.Provider)
			}
			if len(event.Approvers) > 0 && (event.Number == number ||
				(event.Kind == "opened" && event.PR != nil && *event.PR == number)) {
				for _, approver := range event.Approvers {
					add(out.Approved, number, approver)
				}
			}
		}
	}
	return out
}

// ------------------------------------------------------------------ one window

// Window is everything that happened from `start` to now (a zero Start: all time).
type Window struct {
	Name   string
	Start  time.Time
	Events []*Event
	Opened []*pyjson.Object
	Merged []*pyjson.Object
	Runs   [][2]string
	// Per subscription: its model minutes (float64).
	Minutes *pyjson.Object
	// Per subscription: (input tokens, output tokens, cached tokens) as [3]int, and USD cost
	// (float64) (#695).
	Tokens *pyjson.Object
	Cost   *pyjson.Object
}

func (x *Window) Starts() []*Event {
	var out []*Event
	for _, e := range x.Events {
		if e.Kind == "start" {
			out = append(out, e)
		}
	}
	return out
}

func (x *Window) Outcomes() *py.Counter {
	c := py.NewCounter()
	kinds := outcomeKinds()
	for _, e := range x.Events {
		if slices.Contains(kinds, e.Kind) {
			c.Add(e.Kind, 1)
		}
	}
	return c
}

func minutesOf(w *Window, pid string) float64 { f, _ := w.Minutes.Get(pid).(float64); return f }

func tokensOf(w *Window, pid string) [3]int { t, _ := w.Tokens.Get(pid).([3]int); return t }

func costOf(w *Window, pid string) float64 { f, _ := w.Cost.Get(pid).(float64); return f }

// within: `at` falls in a window from `start`; an undated thing counts only toward all time.
func within(at any, start time.Time) bool {
	if start.IsZero() {
		return true
	}
	when := clock.ParseISOValue(at)
	return !when.IsZero() && !when.Before(start)
}

// Window_ is stats.window.
func Window_(name string, span *time.Duration, now time.Time, facts *Facts) *Window {
	var start time.Time
	if span != nil {
		start = now.Add(-*span)
	}
	w := &Window{Name: name, Start: start, Minutes: pyjson.New(), Tokens: pyjson.New(), Cost: pyjson.New()}
	for _, e := range facts.Events {
		if within(e.At, start) {
			w.Events = append(w.Events, e)
		}
	}
	for _, p := range facts.Pulls {
		if within(p.Get("created_at"), start) {
			w.Opened = append(w.Opened, p)
		}
	}
	for _, p := range facts.Pulls {
		if pyjson.Truthy(p.Get("merged_at")) && within(p.Get("merged_at"), start) {
			w.Merged = append(w.Merged, p)
		}
	}
	sort.SliceStable(w.Merged, func(i, j int) bool {
		return py.Str(w.Merged[i].Get("merged_at")) > py.Str(w.Merged[j].Get("merged_at"))
	})
	for _, r := range facts.Runs {
		if within(r[0], start) {
			w.Runs = append(w.Runs, r)
		}
	}
	for _, pid := range facts.Spent.Keys() {
		var minutes []float64
		for _, entry := range spentOf(facts, pid) {
			if within(entry.At, start) {
				minutes = append(minutes, entry.Minutes)
			}
		}
		w.Minutes.Set(pid, py.Sum(minutes))
	}
	for _, pid := range facts.Counted.Keys() {
		var tokens [3]int
		for _, entry := range countedOf(facts, pid) {
			if within(entry.At, start) {
				tokens[0] += entry.Input
			}
		}
		for _, entry := range countedOf(facts, pid) {
			if within(entry.At, start) {
				tokens[1] += entry.Output
			}
		}
		for _, entry := range countedOf(facts, pid) {
			if within(entry.At, start) {
				tokens[2] += entry.Cache
			}
		}
		w.Tokens.Set(pid, tokens)
	}
	for _, pid := range facts.Counted.Keys() {
		var cost []float64
		for _, entry := range countedOf(facts, pid) {
			if within(entry.At, start) {
				cost = append(cost, entry.Cost)
			}
		}
		w.Cost.Set(pid, py.Sum(cost))
	}
	return w
}

// head is xs[:n], with Python's slicing of a negative or too large n.
func head[T any](xs []T, n int) []T {
	if n < 0 {
		n += len(xs)
		if n < 0 {
			n = 0
		}
	}
	if n > len(xs) {
		n = len(xs)
	}
	return xs[:n]
}

func itoa(n int) string { return strconv.Itoa(n) }
