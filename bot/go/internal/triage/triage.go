// Package triage is bot/harness/triage.py.
//
// Triage (`.github/workflows/triage.yml`, #664): what a person's new issue or pull request gets
// without anyone asking, from fixed rules and no model, in one job on GitHub's runner.
//
//   - An issue a trusted person opens gets the answers of the form it came from (`form_labels`: its
//     difficulty, its priority and who does it, `bot:build` or `human`), the type label its title's
//     prefix names when it has none (`type_label`), and an issue type (`fallback_type`). The night bot
//     holds a build that waits for another issue by itself (`queue.waits_for`: a "Blocked by #n" line,
//     the earlier parts of a patch), so triage links nothing.
//   - An issue a person labels `human` (or `human required`, #793), or opens with it, goes to people:
//     both are assigned, the bot is unassigned, and the bot's queue labels come off
//     (`hand_to_people`). The bot hands an issue over itself the same way when it finds it cannot do
//     it (`deliver.Deliverer._needs_person`).
//   - A pull request a trusted person opens gets the type label its title's prefix names.
//
// It adds, never removes, but for that last: a label, an assignee or a type a person set stays. A
// person can call it on any thread (`workflow_dispatch` with its number), which triages it as if it
// had just been opened, its form's answers aside. Any failure skips quietly.
//
// Handing an issue to the bot is `bot:build` (or assigning the bot, or `/harness build`), and that is
// also how a person approves one of the bot's suggestions; handing it to people is `human`.
//
// The patterns are Python's `re` patterns with Python's classes spelled out, since Go's are ASCII:
// `\d` is any script's decimal digit (\p{Nd}), `\s` and `\S` Python's whitespace (`py.ReS`), and `\b`,
// which RE2 has no Unicode form of, the neighbouring character tested against Python's `\w`
// (letters, digits, `_`). `.github/workflows/pr-title.yml` calls PullTitleOK.
package triage

import (
	"errors"
	"fmt"
	"regexp"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/titles"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/trust"
)

// Humans is the people a human task goes to.
var Humans = []string{"MaxGoetzmann", "jgoetzmann"}

// TypeLabels is the type labels (`docs/issues-and-patches.md`): the only ones a pull request gets.
var TypeLabels = config.TypeLabels

// HumanLabel is `human`.
var HumanLabel = config.LabelHuman

// HumanLabels is the labels that hand an issue to people: a person's `human`, and `human
// required` (#793), the bot's own hand-over, which a person may put on too.
var HumanLabels = map[string]bool{config.LabelHuman: true, config.LabelHumanRequired: true}

// Queued is the bot's queue labels `human` takes off: people do the issue now. A run that holds
// it (`bot:working`) or a pull request it opened keep theirs.
var Queued = map[string]bool{config.LabelBuild: true, config.LabelNeedsPlan: true}

// PrefixType is one of PrefixTypes: a title prefix and the type label it names.
type PrefixType struct {
	Pattern *regexp.Regexp
	Label   string
}

// PrefixTypes is the type label each title prefix names (`docs/issues-and-patches.md`, Titles). A
// `Patch: ` title names none: what a change is, the classifier says on its pull request
// (`kind:*`, #1370).
var PrefixTypes = []PrefixType{
	{regexp.MustCompile(`(?i)^Patch v\p{Nd}`), "patch"},
	{regexp.MustCompile(`^v\p{Nd}+\.\p{Nd}+\.0(?:$|` + py.ReNotW + `)`), "major version"},
	{regexp.MustCompile(`(?i)^Night bot:`), "night bot"},
	{regexp.MustCompile(`(?i)^(?:Architecture|CI):`), "architecture"},
}

// partOf is a part of a change split over several issues: `(part n of m)`.
const partOf = `(?: \(part \p{Nd}+ of \p{Nd}+\))?`

// Convention matches a title that follows the convention (#1360): `Patch: <what it does>`, a
// shipped card change's `Patch vX.Y.Z[letter]: …`, which `patches-ship.yml` gives it, and a part's
// `(part n of m)` after either. The older prefixes (`Patch v0.3.X`, `Patch v0.3.Y`, `vX.Y.0`,
// `Night bot`, `CI` and `Architecture`) stopped passing once the title retcon (#1372) had
// retitled every thread.
var Convention = regexp.MustCompile(`^Patch(?: v\p{Nd}+\.\p{Nd}+\.\p{Nd}+[a-z]?)?` + partOf + `: ` + py.ReNotS)

// AutomatedPull matches the titles tools give the pull requests they open, which the title check
// passes as they are (`pull_title_ok`): `patches ship: v0.3.1` (patches-ship.yml; several versions
// joined by ", "), `patches log: abc1234..def5678` (promote-production.yml's technical log,
// #1370), `Promote main to production: 28 commit(s) up to 6d44be2` (scripts/promote-production.sh,
// into `production`) and a training lane's promotion, `AI gen 8 (improve): …`
// (training/improve.md, learn.yml and unban.md). The bots' own pull requests take a title that
// follows the convention (`pull_title`).
var AutomatedPull = regexp.MustCompile(
	`^(?:patches ship: v\p{Nd}+\.\p{Nd}+\.\p{Nd}+[a-z]?(?:, v\p{Nd}+\.\p{Nd}+\.\p{Nd}+[a-z]?)*$` +
		`|patches log: [0-9a-f]{7,40}\.\.[0-9a-f]{7,40}$` +
		`|Promote main to production: \p{Nd}+ commit\(s\) up to [0-9a-f]{7,40}$` +
		`|AI gen \p{Nd}+ \((?:improve|unban|learn)\): ` + py.ReNotS + `)`)

// Revert matches GitHub's Revert button's title for its pull request: `Revert "<the reverted pull
// request's title>"`.
var Revert = regexp.MustCompile(`^Revert "(.+)"$`)

// DefaultIssueTypes is the organisation's issue types (Settings → Planning → Issue types), used
// when the token cannot read them: an issue gets one; a pull request has none.
var DefaultIssueTypes = map[string]string{
	"Task":    "A specific piece of work",
	"Bug":     "An unexpected problem or behavior",
	"Feature": "A request, idea, or new functionality",
}

// Plan is what `apply` does to a thread.
type Plan struct {
	Labels    []string // default: field(default_factory=list)
	Assignees []string // default: field(default_factory=list)
	// IssueType is the issue's type (Task, Bug, Feature), only when it has none.
	IssueType string // default: ""
	// Unassign is the assignees `human` moves off it: the bot.
	Unassign []string // default: field(default_factory=list)
	// Remove is the bot's queue labels `human` takes off.
	Remove []string // default: field(default_factory=list)
	Notes  []string // default: field(default_factory=list)
}

func (x *Plan) Empty() bool {
	return !(len(x.Labels) > 0 || len(x.Assignees) > 0 || x.IssueType != "" || len(x.Unassign) > 0 ||
		len(x.Remove) > 0)
}

// ------------------------------------------------------------------ titles

// ThreadOf is the issue or pull request an event is about (a copy), and whether it is a pull
// request.
func ThreadOf(payload *pyjson.Object) (*pyjson.Object, bool) {
	if payload.Truthy("pull_request") {
		return pyjson.Obj(payload.Get("pull_request")).Copy(), true
	}
	issue := pyjson.Obj(payload.Get("issue"))
	return issue.Copy(), issue.Has("pull_request")
}

// PullTitleOK is whether a pull request's title may become the squash commit's subject (the title
// check, `.github/workflows/pr-title.yml`, #187): it follows the convention, it is one a tool
// gives its own pull requests (`AUTOMATED_PULL`), or it reverts a pull request whose title passes.
func PullTitleOK(title string) bool {
	text := py.Strip(title)
	if revert := Revert.FindStringSubmatch(text); revert != nil {
		return PullTitleOK(revert[1])
	}
	return Convention.MatchString(text) || AutomatedPull.MatchString(text)
}

// PullTitle is the title of the pull request a bot opens for an issue: its builder's when that
// passes the title check, else the issue's when that does, else the builder's (or the issue's)
// as `Patch: <what it does>`, an old prefix dropped (`titles.new_title`). A workflow's alert keeps
// its `CI: …` title so the workflow finds it again (`titles.LEFT_ALONE`); its fix lands as
// `Patch: …`.
func PullTitle(chosen string, issue string) string {
	for _, title := range []string{chosen, issue} {
		if py.Strip(title) != "" && PullTitleOK(title) {
			return strings.Join(py.Split(title), " ")
		}
	}
	text := py.Strip(chosen)
	if text == "" {
		text = py.Strip(issue)
	}
	if text == "" {
		return ""
	}
	return titles.NewTitle(text, "")
}

// TypeLabel is the type label a title's prefix names, or "" for a title with no prefix.
func TypeLabel(title string) string {
	text := py.Strip(title)
	for _, pt := range PrefixTypes {
		if pt.Pattern.MatchString(text) {
			return pt.Label
		}
	}
	return ""
}

// priorities are the priority labels, in the order a form's answer is matched against them.
var priorities = []string{config.LabelPriorityHigh, config.LabelPriorityMedium, config.LabelPriorityLow}

// who is who does it, as each form's dropdown spells it: the label in its brackets.
var who = []string{config.LabelBuild, HumanLabel}

// FormLabels is the labels an issue form's dropdowns ask for (`.github/ISSUE_TEMPLATE/`), which
// triage puts on when a trusted person opens the issue, as if they had set them by hand:
// "Difficulty", "Priority", and who does it, `bot:build` or `human`. "Not sure", "No priority"
// and "Decide later" ask for nothing; neither does a body no form wrote.
func FormLabels(body string) []string {
	keys, answers := []string{}, map[string]string{}
	for _, m := range formAnswers(strings.ReplaceAll(body, "\r\n", "\n")) {
		key := strings.ToLower(py.Strip(m[0]))
		if _, ok := answers[key]; !ok {
			keys = append(keys, key)
		}
		answers[key] = strings.ToLower(py.Strip(m[1]))
	}
	labels := []string{}
	difficulty := ""
	for _, k := range keys {
		if strings.HasPrefix(k, "difficulty") {
			difficulty = answers[k]
			break
		}
	}
	// DIFFICULTY_LABELS is {f"difficulty:{name}": name for name in DIFFICULTIES}.
	for _, level := range config.Difficulties {
		if strings.HasPrefix(difficulty, level) {
			labels = append(labels, "difficulty:"+level)
			break
		}
	}
	if priority, ok := answers["priority"]; ok {
		for _, name := range priorities {
			if _, level, _ := strings.Cut(name, ":"); priority == level {
				labels = append(labels, name)
			}
		}
	}
	for _, name := range who {
		if strings.Contains(answers["who does it"], "("+name+")") {
			labels = append(labels, name)
			break
		}
	}
	return labels
}

// formAnswers is `_FORM_ANSWER.finditer(body)`, an issue form's answers in the body GitHub
// writes, `### <field label>`, a blank line, the answer: each (label, answer) the pattern
// `^### (.+?)[ \t]*\n+(.*?)\s*(?=^### |\Z)` (MULTILINE, DOTALL) finds. RE2 has no lookahead, so
// this is its match, step by step: a `### ` at a line's start; the label, lazily, up to the first
// spaces and tabs before a newline (at least one character, newlines included); every newline
// after it; then the answer, lazily, up to the first place from which Python's whitespace alone
// reaches the end or a line that starts `### `, where the next match may begin.
func formAnswers(body string) [][2]string {
	text := []rune(body)
	n := len(text)
	atHeader := func(p int) bool {
		return (p == 0 || text[p-1] == '\n') && p+4 <= n && string(text[p:p+4]) == "### "
	}
	var found [][2]string
	for at := 0; at < n; {
		start := -1
		for s := at; s < n; s++ {
			if atHeader(s) {
				start = s
				break
			}
		}
		if start < 0 {
			break
		}
		i := start + 4
		// The label: the shortest text (one character at least) followed by [ \t]*\n.
		labelEnd, nl := -1, -1
		for j := i + 1; j <= n; j++ {
			k := j
			for k < n && (text[k] == ' ' || text[k] == '\t') {
				k++
			}
			if k < n && text[k] == '\n' {
				labelEnd, nl = j, k
				break
			}
		}
		if labelEnd < 0 {
			at = start + 1
			continue
		}
		k := nl
		for k < n && text[k] == '\n' {
			k++
		}
		// The answer: the shortest text after which whitespace alone reaches the end or a
		// header; the match ends at the farthest such place (`\s*` is greedy).
		answerEnd, matchEnd := n, n
		for e := k; e <= n; e++ {
			w := e
			for w < n && py.IsSpace(text[w]) {
				w++
			}
			end := -1
			for p := w; p >= e; p-- {
				if p == n || atHeader(p) {
					end = p
					break
				}
			}
			if end >= 0 {
				answerEnd, matchEnd = e, end
				break
			}
		}
		found = append(found, [2]string{string(text[i:labelEnd]), string(text[k:answerEnd])})
		at = matchEnd
	}
	return found
}

// ------------------------------------------------------------------ issue types

var (
	bugWords = regexp.MustCompile(
		`(?i)(?:^|` + py.ReNotW + `)(bug|fails?|failing|failed|broken|breaks|crash(?:es)?|wrong|lag(?:s|gy)?|regression` +
			`|errors?|not working|doesn'?t work|stuck|stall(?:s|ing)?|hangs?)(?:$|` + py.ReNotW + `)`)
	featureWords = regexp.MustCompile(
		`(?i)(?:^|` + py.ReNotW + `)(new|adds?|support|page|mode|screen|easter egg|leaderboard|emotes?|feature)(?:$|` + py.ReNotW + `)`)
)

var taskPrefixes = []string{"architecture:", "refactor:", "ci:", "night bot:", "tests:", "docs:"}

// FallbackType is an issue's type from its title and labels (triage, and the sweep for the rest):
// a failure is a Bug; tooling, CI, the bot and trackers are a Task; something new for players or
// the team is a Feature; anything else a Task. "" when the organisation has none of those.
func FallbackType(issue *pyjson.Object, types map[string]string) string {
	title := py.OrStr(issue.Get("title"), "")
	names := map[string]bool{}
	for _, label := range py.Each(issue.Get("labels")) {
		names[strings.ToLower(py.Str(pyjson.Obj(label).Get("name")))] = true
	}
	lower := strings.ToLower(title)
	wanted := "Task"
	switch {
	case bugWords.MatchString(title):
		wanted = "Bug"
	case hasAnyPrefix(lower, taskPrefixes) || names["architecture"] || names["night bot"]:
		wanted = "Task"
	case featureWords.MatchString(title):
		wanted = "Feature"
	}
	for name := range types {
		if strings.ToLower(name) == strings.ToLower(wanted) {
			return name
		}
	}
	return ""
}

func hasAnyPrefix(s string, prefixes []string) bool {
	for _, p := range prefixes {
		if strings.HasPrefix(s, p) {
			return true
		}
	}
	return false
}

// IssueTypes is the organisation's issue types and their descriptions, or the defaults when there
// are none or the token cannot read them.
func IssueTypes(gh gh.Client) map[string]string {
	found := map[string]string{}
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		for _, t := range gh.ListIssueTypes() {
			if t.Truthy("name") {
				found[py.Str(t.Get("name"))] = py.OrStr(t.Get("description"), "")
			}
		}
	}); failed {
		found = map[string]string{}
	}
	if len(found) > 0 {
		return found
	}
	defaults := map[string]string{}
	for k, v := range DefaultIssueTypes {
		defaults[k] = v
	}
	return defaults
}

// ------------------------------------------------------------------ the gate

func isBot(login string, botLogin string) bool {
	return strings.ToLower(login) == strings.ToLower(botLogin) || strings.HasSuffix(login, "[bot]")
}

// Gate is what this event asks of triage, and why: "open" for a thread just opened (or one a
// person called triage on, `asked`), "human" for an issue a person labelled `human`, or "" to
// skip. A new thread goes only when a trusted person opened it; the bot and apps label their own.
func Gate(payload *pyjson.Object, trust *trust.Trust, botLogin string, asked bool /* = False */) (string, string) {
	thread, isPR := ThreadOf(payload)
	if !thread.Truthy("number") {
		return "", "no issue or pull request in the event"
	}
	number := py.Str(thread.Get("number"))
	if asked {
		return "open", "#" + number + ": a person called triage on it"
	}
	if payload.Get("action") == "labeled" {
		name := py.OrStr(pyjson.Obj(payload.Get("label")).Get("name"), "")
		sender := py.OrStr(pyjson.Obj(payload.Get("sender")).Get("login"), "")
		if isPR || !HumanLabels[strings.ToLower(name)] {
			return "", "label " + name + " asks nothing of triage"
		}
		if sender == "" || isBot(sender, botLogin) {
			who := sender
			if who == "" {
				who = "nobody known"
			}
			return "", "labelled by " + who + ", which labels its own"
		}
		return "human", "#" + number + ": @" + sender + " labelled it " + strings.ToLower(name)
	}
	if payload.Get("action") != "opened" {
		return "", py.Str(payload.Get("action")) + " asks nothing of triage"
	}
	user := pyjson.Obj(thread.Get("user"))
	login := py.OrStr(user.Get("login"), "")
	if isBot(login, botLogin) {
		return "", "opened by " + login + ", which labels its own"
	}
	association := strings.ToUpper(py.OrStr(thread.Get("author_association"), ""))
	if !inSet(trustedAssociations(), association) && trust.Level(login, userID(user.Get("id")), association) < 1 {
		named := association
		if named == "" {
			named = "no association"
		}
		return "", "@" + login + " is not trusted (" + named + ")"
	}
	return "open", "#" + number + " by @" + login
}

// trustedAssociations is trust.TRUSTED_ASSOCIATIONS.
func trustedAssociations() any { return trust.TrustedAssociations }

// inSet is `value in set` for a set as the ports hold one: a map to bool, or a list.
func inSet(set any, value string) bool {
	switch s := set.(type) {
	case map[string]bool:
		return s[value]
	case map[string]struct{}:
		_, ok := s[value]
		return ok
	case []string:
		for _, v := range s {
			if v == value {
				return true
			}
		}
	}
	return false
}

// userID is `user.get("id")` as trust.level takes it: nil for None.
func userID(v any) *int {
	if n, ok := pyjson.AsInt(v); ok {
		return &n
	}
	return nil
}

// ------------------------------------------------------------------ decide and apply

func names(thread *pyjson.Object) []string {
	out := []string{}
	for _, label := range py.Each(thread.Get("labels")) {
		out = append(out, py.Str(pyjson.Obj(label).Get("name")))
	}
	return out
}

// HandToPeople is `human` on an issue: both people take it, and the bot lets go of it.
func HandToPeople(plan *Plan, thread *pyjson.Object, botLogin string) {
	assigned := map[string]bool{}
	for _, a := range py.Each(thread.Get("assignees")) {
		assigned[strings.ToLower(py.OrStr(pyjson.Obj(a).Get("login"), ""))] = true
	}
	var add []string
	for _, h := range Humans {
		if !assigned[strings.ToLower(h)] && !contains(plan.Assignees, h) {
			add = append(add, h)
		}
	}
	plan.Assignees = append(plan.Assignees, add...)
	if assigned[strings.ToLower(botLogin)] {
		plan.Unassign = []string{botLogin}
	}
	plan.Remove = []string{}
	for _, name := range names(thread) {
		if Queued[strings.ToLower(name)] {
			plan.Remove = append(plan.Remove, name)
		}
	}
}

// Decide is what `step` (`gate`'s answer) changes on `thread`, read afresh. `form`: take the
// form's answers too (an issue just opened; not one a person called triage on again). `types`
// nil (or empty) is the defaults.
func Decide(step string, thread *pyjson.Object, isPR bool, botLogin string, types map[string]string /* = None */, form bool /* = True */) *Plan {
	plan := &Plan{}
	if state := thread.Get("state"); state != nil && state != "open" {
		plan.Notes = append(plan.Notes, "closed meanwhile")
		return plan
	}
	present := names(thread)
	lowered := map[string]bool{}
	for _, name := range present {
		lowered[strings.ToLower(name)] = true
	}
	humanLabelled := false
	for name := range HumanLabels {
		if lowered[name] {
			humanLabelled = true
		}
	}
	if step == "human" {
		if humanLabelled {
			HandToPeople(plan, thread, botLogin)
		} else {
			plan.Notes = append(plan.Notes, "`human` came off again, so it stays as it is")
		}
		return plan
	}
	if form && !isPR {
		for _, name := range FormLabels(py.OrStr(thread.Get("body"), "")) {
			if !lowered[strings.ToLower(name)] {
				plan.Labels = append(plan.Labels, name)
			}
		}
	}
	kind := TypeLabel(py.OrStr(thread.Get("title"), ""))
	typed := false
	for _, label := range TypeLabels {
		if lowered[label] {
			typed = true
		}
	}
	if kind != "" && !typed && !contains(plan.Labels, kind) {
		plan.Labels = append(plan.Labels, kind)
	}
	if isPR {
		return plan
	}
	if !thread.Truthy("type") { // a person's type stays
		labelled := append([]any{}, py.Each(thread.Get("labels"))...)
		for _, n := range plan.Labels {
			labelled = append(labelled, pyjson.New("name", n))
		}
		if len(types) == 0 {
			types = DefaultIssueTypes
		}
		plan.IssueType = FallbackType(thread.Copy().Set("labels", labelled), types)
	}
	if humanLabelled || contains(plan.Labels, HumanLabel) {
		HandToPeople(plan, thread, botLogin)
	}
	return plan
}

// Apply makes `plan`'s changes; each one on its own, so one refusal leaves the rest done.
func Apply(gh gh.Client, number int, plan *Plan) []string {
	done := []string{}
	type step struct {
		what string
		do   func()
	}
	var steps []step
	if len(plan.Labels) > 0 {
		steps = append(steps, step{"labelled " + strings.Join(plan.Labels, ", "),
			func() { gh.AddLabels(number, plan.Labels) }})
	}
	if len(plan.Assignees) > 0 {
		steps = append(steps, step{"assigned " + at(plan.Assignees),
			func() { gh.AddAssignees(number, plan.Assignees) }})
	}
	if len(plan.Unassign) > 0 {
		steps = append(steps, step{"unassigned " + at(plan.Unassign),
			func() { gh.RemoveAssignees(number, plan.Unassign) }})
	}
	if plan.IssueType != "" {
		steps = append(steps, step{"typed it " + plan.IssueType, func() { setType(gh, number, plan.IssueType) }})
	}
	for _, name := range plan.Remove {
		steps = append(steps, step{"took `" + name + "` off", func() { gh.RemoveLabel(number, name) }})
	}
	for _, s := range steps {
		failure := ""
		if exc, failed := herr.Catch[*herr.GitHubError](func() {
			if err := catchValueError(s.do); err != nil {
				failure = fmt.Sprintf("could not do this: %s (%s)", s.what, err)
			}
		}); failed {
			failure = fmt.Sprintf("could not do this: %s (%d)", s.what, exc.Status)
		}
		if failure != "" {
			done = append(done, failure)
		} else {
			done = append(done, s.what)
		}
	}
	return done
}

// at is `', '.join('@' + a for a in logins)`.
func at(logins []string) string {
	out := make([]string, len(logins))
	for i, login := range logins {
		out[i] = "@" + login
	}
	return strings.Join(out, ", ")
}

// catchValueError runs fn and returns the ValueError it raised (setType's own, or the client's
// for an answer that is not JSON); anything else goes on up.
func catchValueError(fn func()) (err error) {
	defer func() {
		if r := recover(); r != nil {
			if e, ok := r.(error); ok {
				var mine *py.Error
				var client *gh.ValueError
				if errors.As(e, &mine) && mine.Type == "ValueError" || errors.As(e, &client) {
					err = e
					return
				}
			}
			panic(r)
		}
	}()
	fn()
	return nil
}

// setType sets the issue's type. GitHub drops a type it will not take without an error, so read
// the answer back.
func setType(gh gh.Client, number int, name string) {
	answer := gh.UpdateIssue(number, pyjson.New("type", name))
	got := answer.Get("type")
	if o, ok := got.(*pyjson.Object); ok {
		got = o.Get("name")
	}
	if strings.ToLower(py.OrStr(got, "")) != strings.ToLower(name) {
		herr.Raise(&py.Error{Type: "ValueError", Msg: "GitHub did not keep it"})
	}
}

// ------------------------------------------------------------------ Python's str

func contains(xs []string, s string) bool {
	for _, x := range xs {
		if x == s {
			return true
		}
	}
	return false
}
