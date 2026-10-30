// The second adversarial review's findings, each as the scenario that used to lose a request.
//
// bot/tests/test_hardening.py.
package tests

import (
	"fmt"
	"net/http"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/commands"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

const (
	hardeningTrust   = "jgoetzmann 3 id:95732896\nhelper 2\n"
	hardeningBotName = "jgoetzmann-bot"
)

var hardeningHelper = pyjson.New("login", "helper", "id", 7)

// hardeningCommentOpts is comment()'s keyword arguments; a zero field is the default.
type hardeningCommentOpts struct {
	CID    int            // cid = 500 (0 is 500)
	User   *pyjson.Object // user = OPERATOR
	Action string         // action = "created"
	Sender *pyjson.Object // sender = None: the user
}

// hardeningComment is comment(number, body, *, cid=500, user=OPERATOR, action="created",
// sender=None).
func hardeningComment(number int, body string, opts ...hardeningCommentOpts) *pyjson.Object {
	var o hardeningCommentOpts
	if len(opts) > 0 {
		o = opts[0]
	}
	if o.CID == 0 {
		o.CID = 500
	}
	if o.User == nil {
		o.User = fakes.Operator
	}
	if o.Action == "" {
		o.Action = "created"
	}
	sender := o.Sender
	if sender == nil {
		sender = o.User
	}
	return pyjson.New("action", o.Action, "sender", sender, "issue", pyjson.New("number", number),
		"comment", pyjson.New("id", o.CID, "body", body, "user", o.User, "author_association", "OWNER"))
}

// ---------------------------------------------------------------------------- ParserTests

func TestHardening_AControlVerbInPlainWordsIsARequest(t *testing.T) {
	for _, body := range []string{"@jgoetzmann-bot start with option A", "@jgoetzmann-bot go ahead with plan B",
		"@jgoetzmann-bot stop using the old sprite", "@jgoetzmann-bot halt the timer when",
		"@jgoetzmann-bot help the player find the Coin"} {
		assertEqual(t, hardeningVerbs(body, hardeningBotName)[0][0], "request", body)
	}
}

func TestHardening_ABareControlVerbIsStillACommand(t *testing.T) {
	assertEqual(t, hardeningVerbs("@jgoetzmann-bot start", hardeningBotName), [][2]string{{"start", ""}})
	assertEqual(t, hardeningVerbs("@jgoetzmann-bot run #12", hardeningBotName), [][2]string{{"run", "#12"}})
	assertEqual(t, hardeningVerbs("@jgoetzmann-bot stop", hardeningBotName), [][2]string{{"stop", ""}})
	assertEqual(t, hardeningVerbs("/harness halt it is late", hardeningBotName), [][2]string{{"halt", "it is late"}})
}

func TestHardening_CommandsWrittenAsMarkdown(t *testing.T) {
	for _, body := range []string{"- /harness build", "`/harness build`", "**/harness build**", "1. /harness build"} {
		assertEqual(t, hardeningVerbs(body, hardeningBotName), [][2]string{{"build", ""}}, body)
	}
	assertEqual(t, hardeningVerbs("please /harness build this", hardeningBotName), [][2]string{})
	assertTrue(t, commands.NamesTheBot("please /harness build this", hardeningBotName))
}

// hardeningVerbs is ParserTests.verbs: each command's (verb, args).
func hardeningVerbs(body, bot string) [][2]string {
	out := [][2]string{}
	for _, c := range commands.Parse(body, bot) {
		out = append(out, [2]string{c.Verb, c.Args})
	}
	return out
}

// ---------------------------------------------------------------------------- EventTests

func newHardeningEvents() (*fakes.FakeGitHub, *hctx.Context) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5)
	return gh, testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, TrustText: ptr(hardeningTrust)})
}

func TestHardening_AnEditBySomeoneElseRunsNothing(t *testing.T) {
	_, ctx := newHardeningEvents()
	out := events.Handle(ctx, "issue_comment",
		hardeningComment(5, "/harness halt", hardeningCommentOpts{Action: "edited", Sender: hardeningHelper}))
	assertIn(t, "someone other than", out[0])
	assertFalse(t, dig(t, ctx.Store.Load(), "halted"))
}

func TestHardening_AnEditRunsOnlyWhatItAdded(t *testing.T) {
	gh, ctx := newHardeningEvents()
	events.Handle(ctx, "issue_comment", hardeningComment(5, "thanks @jgoetzmann-bot!", hardeningCommentOpts{CID: 7}))
	assertIn(t, "I saw my name", lastOf(t, gh.BotComments(5)))
	events.Handle(ctx, "issue_comment", hardeningComment(5, "/harness status", hardeningCommentOpts{CID: 8}))
	replies := len(gh.BotComments(5))
	events.Handle(ctx, "issue_comment",
		hardeningComment(5, "/harness status\n/harness halt", hardeningCommentOpts{CID: 8, Action: "edited"}))
	assertTrue(t, dig(t, ctx.Store.Load(), "halted"))
	assertNotIn(t, "Night bot status", lastOf(t, gh.BotComments(5)))
	assertEqual(t, len(gh.BotComments(5)), replies+1)
}

func TestHardening_TheSameCommentIsNeverRunTwice(t *testing.T) {
	_, ctx := newHardeningEvents()
	events.Handle(ctx, "issue_comment", hardeningComment(5, "/harness halt", hardeningCommentOpts{CID: 9}))
	eventsUpdateState(ctx, "halted", false)
	events.Handle(ctx, "issue_comment", hardeningComment(5, "/harness halt", hardeningCommentOpts{CID: 9}))
	assertFalse(t, dig(t, ctx.Store.Load(), "halted"))
}

func TestHardening_AReviewThatNamesTheBotGetsAHint(t *testing.T) {
	gh, ctx := newHardeningEvents()
	gh.AddPull(9, "feature/x")
	payload := pyjson.New("action", "submitted", "sender", fakes.Operator, "pull_request", gh.GetPull(9),
		"review", pyjson.New("id", 3, "state", "COMMENTED", "user", fakes.Operator,
			"author_association", "OWNER",
			"body", "looks fine, maybe @jgoetzmann-bot could tidy it"))
	events.Handle(ctx, "pull_request_review", payload)
	assertIn(t, "I saw my name", lastOf(t, gh.BotComments(9)))
}

func TestHardening_TheBuildLabelOnAPullRequestAsksForARevision(t *testing.T) {
	gh, ctx := newHardeningEvents()
	gh.AddPull(9, "feature/x", fakes.Pull{Labels: []string{config.LabelBuild}})
	payload := pyjson.New("action", "labeled", "sender", fakes.Operator, "label", pyjson.New("name", config.LabelBuild),
		"pull_request", pyjson.New("number", 9))
	events.Handle(ctx, "pull_request_target", payload)
	assertEqual(t, gh.LabelNames(9), setOf(config.LabelRevise))
}

func TestHardening_AFailedAndACancelledRunCountAsCI(t *testing.T) {
	gh, ctx := newHardeningEvents()
	gh.AddPull(9, "bot/issue-5", fakes.Pull{Labels: []string{config.LabelPR}, SHA: "s1"})
	for _, run := range [][2]string{{"CI", "failure"}, {"CI", "cancelled"}} {
		name, conclusion := run[0], run[1]
		payload := pyjson.New("name", name, "conclusion", conclusion, "head_branch", "bot/issue-5",
			"head_sha", "s1", "id", len(gh.Reruns)+1, "run_attempt", 1,
			"pull_requests", []any{pyjson.New("number", 9)})
		flowUpdateItem(ctx, 9, pyjson.New("ci_reruns", pyjson.New()))
		events.Handle(ctx, "workflow_run", pyjson.New("action", "completed", "sender", fakes.Bot,
			"workflow_run", payload))
	}
	assertEqual(t, len(gh.Reruns), 2)
}

func TestHardening_RunNowIsWrittenDownBeforeItStarts(t *testing.T) {
	gh, ctx := newHardeningEvents()
	// The Python replaces `ctx.dispatch`; hctx.Context.Dispatch is a method, and all it does is
	// ask the Actions client to dispatch, so the refusal comes from there.
	gh.Patch.DispatchWorkflow = func(workflow, ref string, inputs map[string]string) {
		herr.Raise(&herr.GitHubError{Msg: "no", Status: 500})
	}
	events.Handle(ctx, "issue_comment", hardeningComment(5, "/harness run #5", hardeningCommentOpts{CID: 10}))
	assertIn(t, "the next hourly run does it", lastOf(t, gh.BotComments(5)))
	gh.Threads.Item(5).Set("labels", eventsLabels(config.LabelBuild))
	look := plan.Peek_(ctx, false, nil, "auto") // by day, outside claude-1's hours
	assertEqual(t, []any{look.Work, look.Forced}, []any{true, true})
	planned := plan.Make(ctx, false, nil, "auto", nil)
	assertEqual(t, dig(t, planned, "number"), 5)
	assertIsNone(t, ctx.Store.Load().Get("run_requested")) // taken up once
}

// ---------------------------------------------------------------------------- TokenTests

func TestHardening_ARefusedBotTokenFallsBackToTheActionsToken(t *testing.T) {
	seen := []string{}
	opener := func(req *gh.Request, timeout time.Duration) (*gh.Response, error) {
		token := req.Header.Get("Authorization")
		seen = append(seen, token)
		if token == "Bearer bot-token" {
			return nil, &gh.HTTPError{URL: req.URL, Code: 401, Msg: "Bad credentials", Header: http.Header{},
				Body: []byte("{}")}
		}
		return &gh.Response{Header: http.Header{}, Body: []byte(pyjson.Dumps(pyjson.New("ok", true)))}, nil
	}
	client := gh.NewGitHub("o/r", "bot-token", false, opener, nil, "actions-token")
	assertEqual(t, client.Request("GET", "/x", nil, "", false, false), pyjson.New("ok", true))
	assertEqual(t, seen, []string{"Bearer bot-token", "Bearer actions-token"})
	client.Request("GET", "/y", nil, "", false, false)
	assertEqual(t, lastOf(t, seen), "Bearer actions-token")
}

// ---------------------------------------------------------------------------- ConditionalGetTests

// The status loop reads every running job every two minutes: GitHub answers 304 to a conditional
// GET whose ETag still matches, which costs no rate limit.
func TestHardening_ACachedGetSendsItsEtagAndKeepsTheAnswerGithubSaysIsUnchanged(t *testing.T) {
	sent := []any{}
	opener := func(req *gh.Request, timeout time.Duration) (*gh.Response, error) {
		// req.headers.get("If-none-match"): None when it has none.
		var etag any
		if values := req.Header.Values("If-None-Match"); len(values) > 0 {
			etag = values[0]
		}
		sent = append(sent, etag)
		if etag == `"v1"` {
			return nil, &gh.HTTPError{URL: req.URL, Code: 304, Msg: "Not Modified", Header: http.Header{}, Body: nil}
		}
		header := http.Header{}
		header.Set("ETag", `"v1"`)
		return &gh.Response{Header: header,
			Body: []byte(pyjson.Dumps(pyjson.New("jobs", []any{pyjson.New("id", 1)})))}, nil
	}
	client := gh.NewGitHub("o/r", "token-for-etags", false, opener, nil, "")
	assertEqual(t, client.ListJobs(5, true), []*pyjson.Object{pyjson.New("id", 1)})
	assertEqual(t, client.ListJobs(5, true), []*pyjson.Object{pyjson.New("id", 1)}) // the 304, answered as kept
	assertEqual(t, sent, []any{nil, `"v1"`})
	client.ListJobs(5, false) // a plain GET stays unconditional
	assertEqual(t, lastOf(t, sent), nil)
}

// ---------------------------------------------------------------------------- DeliverTests

// hardeningApprovedRun is DeliverTests.approved_run(h, number=12).
func hardeningApprovedRun(h *flowHarness) (*pyjson.Object, string) {
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	out := filepath.Join(h.root, fmt.Sprintf("out-%d-%d", 12, flowOuts(h.root)))
	work.NewWorker(h.cfg, planned, workFake(map[string]runner.Handler{
		"build":  workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)}),
		h.clone, filepath.Join(h.root, "work"), out, nil, nil, nil, nil, nil, nil).Run()
	return planned, out
}

func hardeningDeliver(h *flowHarness, planned *pyjson.Object, out string) {
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
}

func TestHardening_AStopAfterTheLastCheckpointPublishesNoPullRequest(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned, out := hardeningApprovedRun(h)
	flowUpdateItem(h.ctx, 12, pyjson.New("stop_requested", true, "stopped_by", "jgoetzmann", "stopped_at", "2026-09-30T03:30:00Z"))
	hardeningDeliver(h, planned, out)
	assertEqual(t, h.gh.ListPulls("open", "bot/issue-12", 200), []*pyjson.Object{})
	assertEqual(t, h.gh.LabelNames(12), setOf("bot:planned", "difficulty:medium"))
	assertIn(t, "Stopped, as asked", lastOf(t, h.gh.BotComments(12)))
}

func TestHardening_AHaltAfterTheLastCheckpointKeepsTheWorkQueued(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned, out := hardeningApprovedRun(h)
	eventsUpdateState(h.ctx, "halted", true)
	hardeningDeliver(h, planned, out)
	assertEqual(t, h.gh.ListPulls("open", "bot/issue-12", 200), []*pyjson.Object{})
	assertEqual(t, h.gh.LabelNames(12), setOf(config.LabelBuild, "bot:planned", "difficulty:medium"))
	assertIsNotNone(t, h.originSHA("bot/issue-12"))
}

func TestHardening_TheModelJobItselfStopsAfterTheLastReview(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	calls := 0
	probe := func(usage *pyjson.Object) (string, string, bool) {
		calls++
		if usage == nil {
			return "stopped by @jgoetzmann", "stop", true
		}
		return "", "", false
	}
	result := work.NewWorker(h.cfg, planned, workFake(map[string]runner.Handler{
		"build":  workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)}),
		h.clone, filepath.Join(h.root, "work"), filepath.Join(h.root, "out-probe"), probe,
		nil, nil, nil, nil, nil).Run()
	assertEqual(t, dig(t, result, "status"), "stopped")
}

func TestHardening_StopThenANewRequestIsAnsweredAfterTheStop(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned, out := hardeningApprovedRun(h)
	events.Handle(h.ctx, "issue_comment", hardeningComment(12, "/harness stop", hardeningCommentOpts{CID: 21}))
	assertEqual(t, h.gh.LabelNames(12), setOf(config.LabelWorking))
	events.Handle(h.ctx, "issue_comment",
		hardeningComment(12, "@jgoetzmann-bot do it with a Token class instead", hardeningCommentOpts{CID: 22}))
	clk := testsupport.ClockOf(h.ctx)
	at := clk.At
	clk.At = time.Date(at.Year(), at.Month(), at.Day(), at.Hour(), 20, at.Second(), at.Nanosecond(), at.Location())
	hardeningDeliver(h, planned, out)
	assertEqual(t, h.gh.ListPulls("open", "bot/issue-12", 200), []*pyjson.Object{})
	assertIn(t, config.LabelBuild, h.gh.LabelNames(12)) // the new request is queued
}

func TestHardening_ARevisionAskedForDuringTheBuildIsKept(t *testing.T) {
	h := newFlowHarness(t)
	start := fakes.PushBranch(h.origin, h.root, "bot/issue-12", map[string]string{"src/game.txt": "v1b\n"})
	fakes.Git(h.clone, "fetch", "-q", "origin")
	fakes.Git(h.deliverRepo, "fetch", "-q", "origin")
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	h.gh.AddPull(40, "bot/issue-12", fakes.Pull{Body: "Closes #12", Labels: []string{config.LabelPR, config.LabelRevise}, SHA: start})
	h.gh.Threads.Item(40).Set("draft", true)
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	assertEqual(t, dig(t, planned, "action"), "revise") // revisions first
	h.gh.Threads.Item(40).Set("labels", eventsLabels(config.LabelPR))
	planned = pyjson.New("action", "build", "number", 12, "branch", "bot/issue-12", "title", "t")
	flowUpdateItem(h.ctx, 12, pyjson.New("started_at", "2026-09-30T03:00:00Z"))
	out := filepath.Join(h.root, "out-keep")
	work.NewWorker(h.cfg, planned, workFake(map[string]runner.Handler{
		"build":  workBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": workReviewer(workApprove)}),
		h.clone, filepath.Join(h.root, "work"), out, nil, nil, nil, nil, nil, nil).Run()
	h.gh.AddLabels(40, []string{config.LabelRevise}) // a person asks for a change while it builds
	hardeningDeliver(h, planned, out)
	assertIn(t, config.LabelRevise, h.gh.LabelNames(40))
	assertNotIn(t, "PR_40", h.gh.AutoMerge)
}

// hardeningWriteResult is out.mkdir(); (out / "result.json").write_text(json.dumps(result)).
func hardeningWriteResult(t *testing.T, out string, result *pyjson.Object) {
	t.Helper()
	if err := os.Mkdir(out, 0o777); err != nil {
		t.Fatal(err)
	}
	rebuildWriteJSON(t, filepath.Join(out, "result.json"), result)
}

func TestHardening_AFailedSurveyGivesBackTheRequest(t *testing.T) {
	h := newFlowHarness(t)
	h.ctx.Store.Update(func(s *pyjson.Object) any {
		s.Obj("suggest").Update(pyjson.New("requested", true))
		return nil
	}, "state")
	planned := plan.Make(h.ctx, false, nil, "suggest", nil)
	assertTrue(t, dig(t, planned, "was_requested"))
	out := filepath.Join(h.root, "out-survey")
	hardeningWriteResult(t, out, pyjson.New("status", "failed", "reason", "boom"))
	hardeningDeliver(h, planned, out)
	assertTrue(t, dig(t, h.ctx.Store.Load(), "suggest", "requested"))
}

func TestHardening_AnUnfinishedSurveyIsDueAgain(t *testing.T) {
	h := newFlowHarness(t)
	eventsUpdateState(h.ctx, "suggest", pyjson.New("last_run", nil, "requested", false))
	testsupport.IdleFor(h.ctx)
	planned := plan.Make(h.ctx, true, nil, "suggest", nil)
	assertEqual(t, dig(t, planned, "action"), "suggest")
	out := filepath.Join(h.root, "out-cut-short")
	hardeningWriteResult(t, out, pyjson.New("status", "interrupted",
		"reason", "the usage limit was reached"))
	hardeningDeliver(h, planned, out)
	assertIsNone(t, dig(t, h.ctx.Store.Load(), "suggest", "last_run"))
	assertTrue(t, plan.SuggestionsDue(h.ctx, h.ctx.Store.Load(), false))
}
