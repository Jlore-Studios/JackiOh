// triage_test.go is bot/tests/test_triage.py: triage (bot/harness/triage.py, #664): who gets
// triaged, and what fixed rules give a new issue or pull request, an issue a person labels
// `human`, and a thread a person calls triage on. No model runs.
package tests

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cli"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/triage"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/trust"
)

// BOT
const triageBot = "jgoetzmann-bot"

// TRUST
var triageTrust = trust.TrustParse("jgoetzmann 3 id:95732896\nMaxGoetzmann 3 id:87041877\nhelper 1\n")

// BUILD, HUMAN, PLAN
const (
	triageBuild = "bot:build"
	triageHuman = "human"
	triagePlan  = "bot:needs-plan"
)

// FORM, its `{who}` filled by triageForm.
const triageFormText = "### What it does\n\nA thing.\n\n### Difficulty (a guess)\n\nhard\n\n" +
	"### Priority\n\nlow\n\n### Who does it\n\n{who}\n"

// triageForm is FORM.format(who=who).
func triageForm(who string) string { return strings.ReplaceAll(triageFormText, "{who}", who) }

// triageFields is event()'s keyword arguments; a zero field is the default.
type triageFields struct {
	Action      string // = "opened"
	Login       string // = "MaxGoetzmann"
	UserID      int    // = 87041877
	Association string // = "OWNER"
	Title       string // = "Fix the thing"
	Body        string // = "Please fix it."
	Labels      []string
	Assignees   []string
	PR          bool
	Number      int     // = 40
	IssueType   *string // = None
	Label       *string // = None
	Sender      *string // = None
}

func triageOr[T comparable](v, def T) T {
	var zero T
	if v == zero {
		return def
	}
	return v
}

// triageEvent is event().
func triageEvent(f triageFields) *pyjson.Object {
	labels := []any{}
	for _, n := range f.Labels {
		labels = append(labels, pyjson.New("name", n))
	}
	assignees := []any{}
	for _, a := range f.Assignees {
		assignees = append(assignees, pyjson.New("login", a))
	}
	var issueType any
	if f.IssueType != nil && *f.IssueType != "" {
		issueType = pyjson.New("name", *f.IssueType)
	}
	thread := pyjson.New("number", triageOr(f.Number, 40), "title", triageOr(f.Title, "Fix the thing"),
		"body", triageOr(f.Body, "Please fix it."),
		"user", pyjson.New("login", triageOr(f.Login, "MaxGoetzmann"), "id", triageOr(f.UserID, 87041877)),
		"author_association", triageOr(f.Association, "OWNER"),
		"labels", labels, "assignees", assignees, "state", "open", "type", issueType)
	key := "issue"
	if f.PR {
		key = "pull_request"
	}
	payload := pyjson.New("action", triageOr(f.Action, "opened"), key, thread)
	if f.Label != nil {
		payload.Set("label", pyjson.New("name", *f.Label))
	}
	if f.Sender != nil {
		payload.Set("sender", pyjson.New("login", *f.Sender))
	}
	return payload
}

// triageThread is thread().
func triageThread(f triageFields) *pyjson.Object {
	thread, _ := triage.ThreadOf(triageEvent(f))
	return thread
}

// ------------------------------------------------------------------ GateTests

// triageGo is GateTests.go.
func triageGo(payload *pyjson.Object, asked bool) [2]string {
	step, why := triage.Gate(payload, triageTrust, triageBot, asked)
	return [2]string{step, why}
}

func TestTriage_AThreadATrustedPersonOpensGoes(t *testing.T) {
	assertEqual(t, triageGo(triageEvent(triageFields{}), false), [2]string{"open", "#40 by @MaxGoetzmann"})
	assertEqual(t, triageGo(triageEvent(triageFields{PR: true}), false)[0], "open")
	// A pinned trust line counts whatever GitHub calls the account.
	assertEqual(t, triageGo(triageEvent(triageFields{Login: "jgoetzmann", UserID: 95732896,
		Association: "NONE"}), false)[0], "open")
}

func TestTriage_AStrangersThreadAndTheBotsOwnStop(t *testing.T) {
	got := triageGo(triageEvent(triageFields{Login: "someone", UserID: 1, Association: "NONE"}), false)
	assertEqual(t, got[0], "")
	assertIn(t, "not trusted", got[1])
	// An unpinned trust line needs GitHub's word too.
	assertEqual(t, triageGo(triageEvent(triageFields{Login: "helper", UserID: 2, Association: "CONTRIBUTOR"}), false)[0],
		"")
	assertEqual(t, triageGo(triageEvent(triageFields{Login: triageBot, Association: "MEMBER"}), false),
		[2]string{"", "opened by " + triageBot + ", which labels its own"})
	assertEqual(t, triageGo(triageEvent(triageFields{Login: "github-actions[bot]", Association: "NONE"}), false)[0], "")
}

// Whoever opened it: only someone with triage access can label.
func TestTriage_APersonsHumanLabelHandsAnyIssueToPeople(t *testing.T) {
	mine := func(sender *string) triageFields {
		return triageFields{Login: triageBot, Association: "MEMBER", Action: "labeled",
			Label: ptr("Human"), Sender: sender}
	}
	assertEqual(t, triageGo(triageEvent(mine(ptr("MaxGoetzmann"))), false),
		[2]string{"human", "#40: @MaxGoetzmann labelled it human"})
	// The bot's own label, an app's, or one nobody is known to have put on, stops.
	for _, sender := range []*string{ptr(triageBot), ptr("github-actions[bot]"), nil} {
		assertEqual(t, triageGo(triageEvent(mine(sender)), false)[0], "", sender)
	}
	// Any other label, or `human` on a pull request, asks nothing of triage.
	assertEqual(t, triageGo(triageEvent(triageFields{Action: "labeled", Label: ptr(triageBuild),
		Sender: ptr("MaxGoetzmann")}), false)[0], "")
	assertEqual(t, triageGo(triageEvent(triageFields{Action: "labeled", Label: ptr(triageHuman),
		Sender: ptr("MaxGoetzmann"), PR: true}), false)[0], "")
}

func TestTriage_ADispatchGoesWhoeverOpenedIt(t *testing.T) {
	assertEqual(t, triageGo(pyjson.New("issue", triageThread(triageFields{Login: triageBot})), true)[0], "open")
	assertEqual(t, triageGo(pyjson.New("issue", pyjson.New()), true)[0], "")
}

// ------------------------------------------------------------------ DecideTests

// triageDecide is DecideTests.decide (step="open", form=True by default).
func triageDecide(step string, form bool, f triageFields) *triage.Plan {
	return triage.Decide(step, triageThread(f), f.PR, triageBot, nil, form)
}

func TestTriage_TheTitlePrefixNamesTheTypeLabel(t *testing.T) {
	for _, c := range [][2]string{{"Patch v0.3.X: a card", "patch"}, {"Patch v0.3.Y: a fix", "patch"},
		{"v0.4.0 (part 1 of 3): Luau", "major version"},
		{"Night bot: the queue", "night bot"},
		{"Architecture: the build", "architecture"},
		{"CI: a slow job", "architecture"}, {"Make it faster", ""}} {
		assertEqual(t, triage.TypeLabel(c[0]), c[1], c[0])
	}
	assertEqual(t, triageDecide("open", true, triageFields{Title: "Patch v0.3.Y: a fix"}).Labels, []string{"patch"})
	// A type label a person (or the form's default) set stays the only one.
	assertEqual(t, triageDecide("open", true, triageFields{Title: "Night bot: x",
		Labels: []string{"architecture"}}).Labels, []string{})
}

func TestTriage_AFormAnswersWithLabelsAndHumanGoesToPeople(t *testing.T) {
	bot := triageDecide("open", true, triageFields{Title: "Patch v0.3.X: x", Labels: []string{"patch"},
		Body: triageForm("The night bot (bot:build)")})
	assertEqual(t, bot.Labels, []string{"difficulty:hard", "priority:low", triageBuild})
	assertEqual(t, bot.Assignees, []string{})
	people := triageDecide("open", true, triageFields{Body: triageForm("People (human)"),
		Assignees: []string{"jgoetzmann"}})
	assertEqual(t, people.Labels, []string{"difficulty:hard", "priority:low", triageHuman})
	assertEqual(t, people.Assignees, []string{"MaxGoetzmann"})
	later := triageDecide("open", true, triageFields{Body: triageForm("Decide later")})
	assertEqual(t, later.Labels, []string{"difficulty:hard", "priority:low"})
	// A dispatch triages it again without its form: a person may have changed those since.
	assertEqual(t, triageDecide("open", false, triageFields{Body: triageForm("People (human)")}).Labels,
		[]string{})
}

func TestTriage_AnIssueGetsATypeAndAPersonsTypeStays(t *testing.T) {
	assertEqual(t, triageDecide("open", true, triageFields{Title: "The board crashes on a trap"}).IssueType, "Bug")
	assertEqual(t, triageDecide("open", true, triageFields{Title: "Night bot: a thing"}).IssueType, "Task")
	assertEqual(t, triageDecide("open", true, triageFields{Title: "A new leaderboard page"}).IssueType, "Feature")
	assertEqual(t, triageDecide("open", true, triageFields{Title: "It crashes", IssueType: ptr("Task")}).IssueType, "")
	assertEqual(t, triageDecide("open", true, triageFields{Title: "It crashes", PR: true}).IssueType, "")
}

func TestTriage_HumanMovesItToPeopleAndOffTheBotsQueue(t *testing.T) {
	plan := triageDecide("human", true, triageFields{
		Labels:    []string{triageHuman, triageBuild, triagePlan, "bot:working", "patch"},
		Assignees: []string{triageBot, "MaxGoetzmann"}})
	assertEqual(t, plan.Assignees, []string{"jgoetzmann"})
	assertEqual(t, plan.Unassign, []string{triageBot})
	assertEqual(t, plan.Remove, []string{triageBuild, triagePlan}) // a run's `bot:working` stays
	gone := triageDecide("human", true, triageFields{Labels: []string{triageBuild}})
	assertTrue(t, gone.Empty())
	assertIn(t, "`human` came off again", gone.Notes[0])
}

func TestTriage_APullRequestGetsOnlyItsTypeLabel(t *testing.T) {
	plan := triageDecide("open", true, triageFields{PR: true, Title: "Night bot: x", Labels: []string{triageHuman},
		Body: triageForm("People (human)")})
	assertEqual(t, plan.Labels, []string{"night bot"})
	assertEqual(t, plan.Assignees, []string{})
	assertEqual(t, plan.IssueType, "")
}

func TestTriage_AClosedThreadChangesNothing(t *testing.T) {
	closed := triageThread(triageFields{Title: "Patch v0.3.Y: x"})
	closed.Set("state", "closed")
	plan := triage.Decide("open", closed, false, triageBot, nil, true)
	assertTrue(t, plan.Empty())
}

// ------------------------------------------------------------------ ApplyTests

func TestTriage_ApplyMakesEachChangeAndOneRefusalLeavesTheRest(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(50, fakes.Issue{Labels: []string{triageBuild}})
	gh.Threads.Item(50).Set("assignees", []any{pyjson.New("login", triageBot)})
	plan := &triage.Plan{Labels: []string{"patch", triageHuman}, Assignees: []string{"MaxGoetzmann"},
		Unassign: []string{triageBot}, IssueType: "Bug", Remove: []string{triageBuild}}
	done := triage.Apply(gh, 50, plan)
	assertEqual(t, gh.LabelNames(50), setOf("patch", triageHuman))
	logins := []string{}
	for _, a := range pyjson.Objs(gh.Threads.Item(50).Get("assignees")) {
		logins = append(logins, a.Str("login"))
	}
	assertEqual(t, logins, []string{"MaxGoetzmann"})
	assertEqual(t, gh.Threads.Item(50).Get("type"), "Bug")
	assertIn(t, "took `bot:build` off", done)
	gh.Patch.UpdateIssue = func(number int, fields *pyjson.Object) *pyjson.Object {
		return pyjson.New("type", nil)
	}
	done = triage.Apply(gh, 50, &triage.Plan{IssueType: "Task", Labels: []string{"night bot"}})
	gh.Patch.UpdateIssue = nil
	assertIn(t, "could not do this: typed it Task (GitHub did not keep it)", done)
	assertIn(t, "night bot", gh.LabelNames(50))
}

// ------------------------------------------------------------------ CommandTests
//
// `python -m harness triage`, as the workflow's one job runs it.

// triageRun is CommandTests.run_triage.
func triageRun(t *testing.T, gh *fakes.FakeGitHub, payload *pyjson.Object) string {
	t.Helper()
	cfg := testsupport.MakeConfig()
	path := filepath.Join(t.TempDir(), "event.json")
	if err := os.WriteFile(path, []byte(pyjson.Dumps(payload)), 0o644); err != nil {
		t.Fatal(err)
	}
	var out bytes.Buffer
	ctx, stdout := cli.Ctx, cli.Stdout
	cli.Ctx = func(cfg *config.Config, write bool) *hctx.Context {
		return testsupport.MakeCtx(gh, testsupport.Ctx{Cfg: cfg})
	}
	cli.Stdout = &out
	defer func() { cli.Ctx, cli.Stdout = ctx, stdout }()
	assertEqual(t, cli.CmdTriage(cfg, &cli.Args{Payload: path}), 0)
	return out.String()
}

func TestTriage_AFormIssueIsLabelledTypedAndHandedOnAsItOpens(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(126, fakes.Issue{Title: "Patch v0.3.Y: the board crashes", Labels: []string{"patch"},
		Body: triageForm("The night bot (bot:build)")})
	gh.Threads.Item(126).Set("author_association", "MEMBER")
	printed := triageRun(t, gh, pyjson.New("action", "opened", "issue", gh.GetIssue(126)))
	assertEqual(t, gh.LabelNames(126), setOf("patch", "difficulty:hard", "priority:low", triageBuild))
	assertEqual(t, gh.Threads.Item(126).Get("type"), "Bug")
	assertEqual(t, gh.Comments.Get(126), []*pyjson.Object{}) // it says nothing on the thread
	assertIn(t, "triage: open: #126 by @", printed)
}

func TestTriage_APersonsHumanLabelOnTheBotsIssueGoesToPeople(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(650, fakes.Issue{Title: "A suggestion", Labels: []string{triageHuman, triageBuild},
		User: pyjson.New("login", triageBot, "id", 1)})
	gh.Threads.Item(650).Set("assignees", []any{pyjson.New("login", triageBot)})
	triageRun(t, gh, pyjson.New("action", "labeled", "label", pyjson.New("name", triageHuman),
		"sender", pyjson.New("login", "MaxGoetzmann"), "issue", gh.GetIssue(650)))
	assertEqual(t, gh.LabelNames(650), setOf(triageHuman))
	logins := []string{}
	for _, a := range pyjson.Objs(gh.Threads.Item(650).Get("assignees")) {
		logins = append(logins, a.Str("login"))
	}
	assertEqual(t, sortedOf(logins), []string{"MaxGoetzmann", "jgoetzmann"})
}

func TestTriage_ADispatchTriagesTheThreadItNamesAndABadOneSkips(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(130, fakes.Issue{Title: "Night bot: a thing", User: pyjson.New("login", triageBot, "id", 1),
		Body: triageForm("People (human)")})
	triageRun(t, gh, pyjson.New("inputs", pyjson.New("number", "#130")))
	assertEqual(t, gh.LabelNames(130), setOf("night bot")) // no form answers on a dispatch
	printed := triageRun(t, gh, pyjson.New("inputs", pyjson.New("number", "9999")))
	assertIn(t, "could not triage", printed)
}

func TestTriage_AStrangersIssueIsLeftAlone(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(131, fakes.Issue{Title: "Patch v0.3.Y: x", User: pyjson.New("login", "someone", "id", 5)})
	gh.Threads.Item(131).Set("author_association", "NONE")
	printed := triageRun(t, gh, pyjson.New("action", "opened", "issue", gh.GetIssue(131)))
	assertIn(t, "triage: skip:", printed)
	assertEqual(t, gh.LabelNames(131), map[string]bool{})
}
