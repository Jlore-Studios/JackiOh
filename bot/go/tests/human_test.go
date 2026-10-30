// #793: `human required`. An item only a person can do (a change to `bot/` or `.github/`, a
// secret, a dashboard, an account) gets the label and a comment naming the people, and the bot
// leaves it alone until a person takes the label off.
//
// bot/tests/test_human.py.
package tests

import (
	"fmt"
	"slices"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/human"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/issueplan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/triage"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/trust"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
)

const humanPeople = "@MaxGoetzmann @jgoetzmann"

var humanBotFiles = []string{"bot/harness/events.py", "bot/README.md", "bot/tests/test_flow.py"}

// humanForbidden is FORBIDDEN = config_mod.load(env={}).forbidden_paths.
func humanForbidden() []string { return config.Load("", map[string]string{}).ForbiddenPaths }

// humanBlocked is blocked(question, *, human_required).
func humanBlocked(question string, humanRequired bool) string {
	flag := ""
	if humanRequired {
		flag = ", \"human_required\": true"
	}
	return fmt.Sprintf("<!-- bot: {\"status\": \"blocked\"%s, \"question\": \"%s\"} -->\nStopped.", flag, question)
}

// humanPlanBody is plan_body(files, head=""): a plan as the harness keeps it, its rating line
// taken off.
func humanPlanBody(files []string, head ...string) string {
	_, rest, _ := strings.Cut(ratingRatedPlan("medium", files, ""), "\n")
	return strings.Join(head, "") + rest
}

// ---------------------------------------------------------------------------- PlanNeedsTests

func TestHuman_APlanThatTouchesOnlyForbiddenPathsNeedsAPerson(t *testing.T) {
	why := human.PlanNeeds(humanPlanBody(humanBotFiles), humanForbidden())
	eventsIn(t, "`bot/harness/events.py`, `bot/README.md`", why)
	eventsIn(t, "a person has to make the change", why)
}

func TestHuman_APlanWithAnythingTheBotMayChangeDoesNot(t *testing.T) {
	forbidden := humanForbidden()
	eventsEqual(t, human.PlanNeeds(humanPlanBody(append(slices.Clone(humanBotFiles), "src/game.txt")), forbidden), "")
	eventsEqual(t, human.PlanNeeds("1. Edit it.", forbidden), "") // no table at all
	eventsEqual(t, human.PlanNeeds("", forbidden), "")
}

func TestHuman_ThePlannersOwnLineBeforeTheGoalSaysIt(t *testing.T) {
	forbidden := humanForbidden()
	game := []string{"src/game.txt"}
	head := "**Human required:** add `RENDER_API_KEY` in GitHub's\n  Actions secrets.\n\n"
	eventsEqual(t, human.PlanNeeds(humanPlanBody(game, head), forbidden),
		"add `RENDER_API_KEY` in GitHub's")
	said := "Human required: set `VITE_FLAG` in Vercel.\n\n"
	eventsEqual(t, human.PlanNeeds(humanPlanBody(game, said), forbidden),
		"set `VITE_FLAG` in Vercel.")
	// Past the Goal the words are the plan's own, not the planner's verdict.
	late := humanPlanBody(game) + "\n**Human required:** nothing, really.\n"
	eventsEqual(t, human.PlanNeeds(late, forbidden), "")
	// A planner that says nobody is needed is not taken at its word the other way.
	for _, nobody := range []string{"**Human required:** none.\n\n", "Human required: no\n\n"} {
		eventsEqual(t, human.PlanNeeds(humanPlanBody(game, nobody), forbidden), "")
	}
	// With no Goal heading, only the plan's opening lines count.
	bare := strings.Repeat("1. Edit `src/game.txt`.\n", 6) + "Human required: a dashboard.\n"
	eventsEqual(t, human.PlanNeeds(bare, forbidden), "")
	eventsEqual(t, human.PlanNeeds("Human required: a dashboard.\n"+bare, forbidden),
		"a dashboard.")
}

func TestHuman_TheCommentNamesThePeopleAndTheWayBack(t *testing.T) {
	text := human.Ask("Add the secret.", "jgoetzmann-bot", "build this", " What I had is on `b`.", "")
	eventsTrue(t, strings.HasPrefix(text, humanPeople+" this needs a person: I cannot build this "+
		"myself. What I had is on `b`."))
	eventsIn(t, "Add the secret.", text)
	eventsIn(t, "assigned you and labelled it `"+config.LabelHumanRequired+"`", text)
	eventsIn(t, "take the label off and `/harness build`", text)
}

// ---------------------------------------------------------------------------- ReportTests

func TestHuman_OnlyABlockedReportWithATrueFlagCounts(t *testing.T) {
	report := verdicts.BuildReport_(humanBlocked("Add the secret.", true))
	eventsEqual(t, []any{report.Status, report.Question, report.HumanRequired},
		[]any{"blocked", "Add the secret.", true})
	eventsFalse(t, verdicts.BuildReport_(humanBlocked("Which?", false)).
		HumanRequired)
	eventsFalse(t, verdicts.BuildReport_(
		`<!-- bot: {"status": "done", "human_required": true} -->`).HumanRequired)
	eventsFalse(t, verdicts.BuildReport_(
		`<!-- bot: {"status": "blocked", "human_required": "yes"} -->`).HumanRequired)
}

// ---------------------------------------------------------------------------- FlowTests

// humanHandedToPeople is FlowTests.handed_to_people: the item is people's, as `human` makes it:
// `human required` on, none of the bot's labels that say it has the item (`bot:blocked`
// included), the bot unassigned and both people assigned, and its last comment names them.
// Returns that comment.
func humanHandedToPeople(t *testing.T, h *ratingHarness, number int) string {
	t.Helper()
	labels := h.gh.LabelNames(number)
	eventsIn(t, config.LabelHumanRequired, labels)
	both := map[string]bool{}
	for n := range labels {
		if human.LetsGo[n] {
			both[n] = true
		}
	}
	eventsEqual(t, both, eventsSet(), labels)
	assigned := map[string]bool{}
	for _, a := range pyjson.List(eventsDig(t, h.gh.Threads.Item(number), "assignees")) {
		assigned[pyjson.Str(eventsDig(t, a, "login"))] = true
	}
	eventsEqual(t, assigned, eventsSet("MaxGoetzmann", "jgoetzmann"))
	said := eventsLast(t, h.gh.BotComments(number))
	eventsTrue(t, strings.HasPrefix(said, humanPeople), said)
	eventsIn(t, "So it is yours now: I took my labels off, unassigned myself", said)
	eventsTrue(t, eventsDig(t, h.ctx.Store.Load(), "items", fmt.Sprint(number), "question"))
	return said
}

func TestHuman_APlanThatTouchesOnlyBotFilesIsHandedToPeopleUnbuilt(t *testing.T) {
	// #760: its plan's every file was under bot/, and two build runs found the same wall.
	h := ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Bring back suggestions."})
	h.gh.AddAssignees(12, []string{"jgoetzmann-bot"}) // #760 was assigned to the bot
	planned, result := h.night(ratingPlanner(ratingRatedPlan("medium", humanBotFiles, "")))
	eventsEqual(t, []any{eventsDig(t, planned, "action"), eventsDig(t, result, "status")}, []any{"plan", "blocked"})
	eventsTrue(t, eventsDig(t, result, "human_required"))
	said := humanHandedToPeople(t, h, 12)
	eventsIn(t, "`bot/harness/events.py`", said)
	eventsIn(t, "the plan is in this issue's description", said)
	eventsIn(t, "Files to touch", issueplan.PlanOf(pyjson.Str(eventsDig(t, h.gh.Threads.Item(12), "body"))))
	eventsIn(t, "bot:planned", h.gh.LabelNames(12)) // what it has stays
	eventsNotEqual(t, plan.Make(h.ctx, false, nil, "auto", nil).Get("number"), 12)
}

func TestHuman_APlannerThatSaysOnlyAPersonCanDoItIsBelieved(t *testing.T) {
	h := ratingDayHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}, Body: "Turn on the deploy hook."})
	text := strings.ReplaceAll(ratingRatedPlan("medium", nil, ""),
		"## 1. Goal", "**Human required:** create `RENDER_DEPLOY_HOOK_URL` in GitHub's "+
			"secrets.\n\n## 1. Goal")
	_, result := h.night(ratingPlanner(text))
	eventsEqual(t, eventsDig(t, result, "status"), "blocked")
	eventsIn(t, "create `RENDER_DEPLOY_HOOK_URL` in GitHub's secrets.",
		humanHandedToPeople(t, h, 12))
}

func TestHuman_ABuilderThatSaysOnlyAPersonCanDoItHandsItOn(t *testing.T) {
	h := newRatingHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	// #796: GitHub had made the label grey and bare when it was first put on.
	h.gh.Labels.Set(config.LabelHumanRequired, pyjson.New("name", config.LabelHumanRequired, "color", "ededed",
		"description", ""))
	_, result := h.night(ratingRunner(map[string]runner.Handler{
		"build": ratingBuilder(map[string]string{}, humanBlocked("Set `VITE_SUPABASE_URL` in Vercel's project settings.",
			true)),
		"review": ratingReviewer(ratingApprove)}))
	eventsEqual(t, []any{eventsDig(t, result, "status"), eventsDig(t, result, "human_required")}, []any{"blocked", true})
	eventsIn(t, "Set `VITE_SUPABASE_URL` in Vercel's project settings.",
		humanHandedToPeople(t, h, 12))
	entry := pyjson.List(config.Labels.Get(config.LabelHumanRequired))
	color, description := entry[0], entry[1]
	eventsEqual(t, eventsDig(t, h.gh.Labels, config.LabelHumanRequired),
		pyjson.New("name", config.LabelHumanRequired, "color", color,
			"description", description))
}

func TestHuman_ABuildWhoseWholeChangeThePathGuardPutBackHandsItOn(t *testing.T) {
	h := newRatingHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	_, result := h.night(ratingRunner(map[string]runner.Handler{
		"build":  ratingBuilder(map[string]string{"bot/harness/new.py": "x = 1\n"}),
		"review": ratingReviewer(ratingApprove)}))
	eventsEqual(t, []any{eventsDig(t, result, "status"), eventsDig(t, result, "human_required")}, []any{"blocked", true})
	eventsIn(t, "is in paths the bot may not change", humanHandedToPeople(t, h, 12))
}

func TestHuman_AQuestionAPersonCanAnswerIsStillOnlyAQuestion(t *testing.T) {
	h := newRatingHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	h.night(ratingRunner(map[string]runner.Handler{
		"build":  ratingBuilder(map[string]string{}, humanBlocked("Option A or B?", false)),
		"review": ratingReviewer(ratingApprove)}))
	labels := h.gh.LabelNames(12)
	eventsIn(t, config.LabelBlocked, labels)
	eventsNotIn(t, config.LabelHumanRequired, labels)
	said := eventsLast(t, h.gh.BotComments(12))
	eventsTrue(t, strings.HasPrefix(said, "I need a decision before I can build this."))
	eventsNotIn(t, humanPeople, said)
}

func TestHuman_ARevisionOnlyAPersonCanMakeHandsThePullRequestOn(t *testing.T) {
	h := newRatingHarness(t)
	start := fakes.PushBranch(h.origin, h.root, "bot/issue-12", map[string]string{"src/game.txt": "v2\n"})
	fakes.Git(h.clone, "fetch", "-q", "origin")
	fakes.Git(h.deliverRepo, "fetch", "-q", "origin")
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelPROpen}})
	h.gh.AddPull(40, "bot/issue-12", fakes.Pull{Body: "Closes #12", Labels: []string{config.LabelPR, config.LabelRevise},
		SHA: start})
	h.gh.AddComment(40, "@jgoetzmann-bot also run it in CI", fakes.Comment{User: fakes.Operator})
	_, result := h.night(ratingRunner(map[string]runner.Handler{
		"revise": ratingBuilder(map[string]string{}, humanBlocked("Add a job to `.github/workflows/ci.yml`.",
			true)),
		"review": ratingReviewer(ratingApprove)}))
	eventsEqual(t, eventsDig(t, result, "status"), "blocked")
	eventsEqual(t, h.gh.LabelNames(40), eventsSet(config.LabelPR, config.LabelHumanRequired)) // `bot:pr` stays
	said := eventsLast(t, h.gh.BotComments(40))
	eventsTrue(t, strings.HasPrefix(said, humanPeople+" this needs a person: I cannot revise this"))
	eventsIn(t, "`/harness revise <notes>`", said)
}

func TestHuman_TheBotLeavesItAloneUntilTheLabelComesOff(t *testing.T) {
	h := newRatingHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelHumanRequired}})
	ratingAsk(h, 12, "@jgoetzmann-bot exception for this issue", 51)
	eventsIn(t, "#12 is labelled `"+config.LabelHumanRequired+"`", eventsLast(t, h.gh.BotComments(12)))
	eventsNotIn(t, config.LabelBuild, h.gh.LabelNames(12))
	// A queue label put on by hand is not picked up while it is on.
	h.gh.AddLabels(12, []string{config.LabelBuild})
	skipped := []string{}
	eventsEqual(t, queue.Candidates(h.ctx, h.ctx.Store.Load(), &skipped, nil), []*queue.Candidate{})
	eventsIn(t, "#12 skipped: labelled `"+config.LabelHumanRequired+"`", skipped[0])
	// A person did their part and took it off: the next request queues it.
	h.gh.RemoveLabel(12, config.LabelHumanRequired)
	ratingAsk(h, 12, "/harness build", 52)
	eventsIn(t, "Queued #12", eventsLast(t, h.gh.BotComments(12)))
	eventsEqual(t, eventsDig(t, plan.Make(h.ctx, false, nil, "auto", nil), "number"), 12)
}

func TestHuman_APersonsHumanRequiredHandsTheIssueToPeopleAsHumanDoes(t *testing.T) {
	payload := pyjson.New("action", "labeled", "label", pyjson.New("name", "Human required"),
		"sender", pyjson.New("login", "MaxGoetzmann"),
		"issue", pyjson.New("number", 12, "labels", []any{}, "user", pyjson.New("login", "MaxGoetzmann")))
	step, why := triage.Gate(payload, trust.TrustParse(""), "jgoetzmann-bot", false)
	eventsEqual(t, [2]string{step, why}, [2]string{"human", "#12: @MaxGoetzmann labelled it human required"})
	thread := pyjson.New("number", 12, "state", "open", "assignees", []any{pyjson.New("login", "jgoetzmann-bot")},
		"labels", eventsLabels(config.LabelHumanRequired, config.LabelBuild))
	decided := triage.Decide("human", thread, false, "jgoetzmann-bot", nil, true)
	eventsEqual(t, []any{decided.Assignees, decided.Unassign, decided.Remove},
		[]any{[]string{"MaxGoetzmann", "jgoetzmann"}, []string{"jgoetzmann-bot"}, []string{config.LabelBuild}})
	// The bot's own hand-over is the bot's to make: triage leaves it alone.
	payload.Set("sender", pyjson.New("login", "jgoetzmann-bot"))
	step, _ = triage.Gate(payload, trust.TrustParse(""), "jgoetzmann-bot", false)
	eventsEqual(t, step, "")
}

func TestHuman_TheLabelIsOneTheSweepMakes(t *testing.T) {
	entry := pyjson.List(config.Labels.Get(config.LabelHumanRequired))
	color, description := pyjson.Str(entry[0]), pyjson.Str(entry[1])
	eventsEqual(t, len([]rune(color)), 6)
	if n := len([]rune(description)); n > config.LabelDescriptionMax {
		t.Fatalf("assertLessEqual: %d > %d", n, config.LabelDescriptionMax)
	}
}
