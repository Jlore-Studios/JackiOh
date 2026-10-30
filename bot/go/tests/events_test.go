// Events: comment commands, labels, assignment, reviews, CI results, closed pull requests.
//
// bot/tests/test_events.py. It also holds the small assertions the seven files ported with it
// (events, sweep, rating, rebuild, suspend, hardening, human) share: unittest's assertEqual,
// assertIn and the rest, and a KeyError-checked walk into JSON.
package tests

import (
	"fmt"
	"reflect"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/events"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// ---------------------------------------------------------------------------- shared assertions

// eventsMsg is unittest's `msg` argument: the extra words a failure prints.
func eventsMsg(msg []any) string {
	if len(msg) == 0 {
		return ""
	}
	return " : " + fmt.Sprint(msg...)
}

// eventsSame is Python's ==: JSON values compare as pyjson.Equal does (an int equals a float of
// the same value, objects regardless of key order), Go values (sets, tuples, maps) deeply, and a
// nil slice or map is the empty one (both are Python's [] or {}).
func eventsSame(got, want any) bool {
	if reflect.DeepEqual(got, want) || pyjson.Equal(got, want) {
		return true
	}
	a, b := reflect.ValueOf(got), reflect.ValueOf(want)
	if !a.IsValid() || !b.IsValid() || a.Type() != b.Type() {
		return false
	}
	switch a.Kind() {
	case reflect.Slice, reflect.Array:
		if a.Len() != b.Len() {
			return false
		}
		for i := range a.Len() {
			if !eventsSame(a.Index(i).Interface(), b.Index(i).Interface()) {
				return false
			}
		}
		return true
	case reflect.Map:
		if a.Len() != b.Len() {
			return false
		}
		for _, k := range a.MapKeys() {
			v := b.MapIndex(k)
			if !v.IsValid() || !eventsSame(a.MapIndex(k).Interface(), v.Interface()) {
				return false
			}
		}
		return true
	case reflect.Struct:
		for i := range a.NumField() {
			if !a.Type().Field(i).IsExported() {
				return false
			}
			if !eventsSame(a.Field(i).Interface(), b.Field(i).Interface()) {
				return false
			}
		}
		return true
	}
	return false
}

// eventsEqual is assertEqual.
func eventsEqual(t *testing.T, got, want any, msg ...any) {
	t.Helper()
	if !eventsSame(got, want) {
		t.Fatalf("assertEqual: got %s, want %s%s", eventsShow(got), eventsShow(want), eventsMsg(msg))
	}
}

// eventsNotEqual is assertNotEqual.
func eventsNotEqual(t *testing.T, got, want any, msg ...any) {
	t.Helper()
	if eventsSame(got, want) {
		t.Fatalf("assertNotEqual: both are %s%s", eventsShow(got), eventsMsg(msg))
	}
}

// eventsShow prints a value for a failure: JSON as Python would, anything else as Go does.
func eventsShow(v any) string {
	switch v.(type) {
	case *pyjson.Object, []any:
		return pyjson.Dumps(v)
	}
	return fmt.Sprintf("%#v", v)
}

// eventsHas is Python's `needle in haystack`: a substring of a string, a key of a map or JSON
// object, an element of a slice.
func eventsHas(needle, haystack any) bool {
	switch h := haystack.(type) {
	case string:
		s, ok := needle.(string)
		return ok && strings.Contains(h, s)
	case *pyjson.Object:
		s, ok := needle.(string)
		return ok && h != nil && h.Has(s)
	}
	v := reflect.ValueOf(haystack)
	switch v.Kind() {
	case reflect.Map:
		k := reflect.ValueOf(needle)
		if !k.IsValid() || !k.Type().AssignableTo(v.Type().Key()) {
			return false
		}
		return v.MapIndex(k).IsValid()
	case reflect.Slice, reflect.Array:
		for i := range v.Len() {
			if eventsSame(v.Index(i).Interface(), needle) {
				return true
			}
		}
		return false
	}
	panic(fmt.Sprintf("eventsHas: cannot look in a %T", haystack))
}

// eventsIn is assertIn.
func eventsIn(t *testing.T, needle, haystack any, msg ...any) {
	t.Helper()
	if !eventsHas(needle, haystack) {
		t.Fatalf("assertIn: %s not found in %s%s", eventsShow(needle), eventsShow(haystack), eventsMsg(msg))
	}
}

// eventsNotIn is assertNotIn.
func eventsNotIn(t *testing.T, needle, haystack any, msg ...any) {
	t.Helper()
	if eventsHas(needle, haystack) {
		t.Fatalf("assertNotIn: %s unexpectedly found in %s%s", eventsShow(needle), eventsShow(haystack), eventsMsg(msg))
	}
}

// eventsTrue is assertTrue: Python's truthiness for JSON values, a bool as it is.
func eventsTrue(t *testing.T, v any, msg ...any) {
	t.Helper()
	if !eventsTruthy(v) {
		t.Fatalf("assertTrue: %s is not true%s", eventsShow(v), eventsMsg(msg))
	}
}

// eventsFalse is assertFalse.
func eventsFalse(t *testing.T, v any, msg ...any) {
	t.Helper()
	if eventsTruthy(v) {
		t.Fatalf("assertFalse: %s is not false%s", eventsShow(v), eventsMsg(msg))
	}
}

// eventsTruthy is bool(v): JSON as pyjson.Truthy, a Go slice or map by its length.
func eventsTruthy(v any) bool {
	r := reflect.ValueOf(v)
	switch r.Kind() {
	case reflect.Slice, reflect.Map:
		if _, ok := v.([]any); !ok {
			return r.Len() > 0
		}
	case reflect.Pointer:
		if r.IsNil() {
			return false
		}
	}
	return pyjson.Truthy(v)
}

// eventsNone is assertIsNone: a nil, or a nil pointer.
func eventsNone(t *testing.T, v any, msg ...any) {
	t.Helper()
	if !eventsIsNil(v) {
		t.Fatalf("assertIsNone: %s is not None%s", eventsShow(v), eventsMsg(msg))
	}
}

// eventsNotNone is assertIsNotNone.
func eventsNotNone(t *testing.T, v any, msg ...any) {
	t.Helper()
	if eventsIsNil(v) {
		t.Fatalf("assertIsNotNone: unexpectedly None%s", eventsMsg(msg))
	}
}

func eventsIsNil(v any) bool {
	if v == nil {
		return true
	}
	r := reflect.ValueOf(v)
	switch r.Kind() {
	case reflect.Pointer, reflect.Map, reflect.Slice, reflect.Func, reflect.Interface:
		return r.IsNil()
	}
	return false
}

// eventsDig is `v[k1][k2]…` on JSON objects: a missing key fails the test, as Python's KeyError
// errors it.
func eventsDig(t *testing.T, v any, keys ...string) any {
	t.Helper()
	for _, k := range keys {
		o, ok := v.(*pyjson.Object)
		if !ok || o == nil {
			t.Fatalf("eventsDig: %s is not a dict (looking up %q)", eventsShow(v), k)
		}
		next, found := o.Lookup(k)
		if !found {
			t.Fatalf("KeyError: %q", k)
		}
		v = next
	}
	return v
}

// eventsLast is `xs[-1]`: an empty list fails the test, as Python's IndexError errors it.
func eventsLast[T any](t *testing.T, xs []T) T {
	t.Helper()
	if len(xs) == 0 {
		t.Fatalf("IndexError: list index out of range")
	}
	return xs[len(xs)-1]
}

// eventsSet is a Python set of strings, as FakeGitHub.LabelNames returns one.
func eventsSet(names ...string) map[string]bool {
	out := map[string]bool{}
	for _, n := range names {
		out[n] = true
	}
	return out
}

// eventsUpdate is `ctx.store.update(lambda s: state_item(s, number).update(**fields))`.
func eventsUpdateItem(ctx *hctx.Context, number int, kv ...any) {
	ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, number).Update(pyjson.New(kv...))
		return nil
	}, "state")
}

// eventsUpdateState is `ctx.store.update(lambda s: s.update(**fields))`.
func eventsUpdateState(ctx *hctx.Context, kv ...any) {
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.Update(pyjson.New(kv...))
		return nil
	}, "state")
}

// eventsLabels is a thread's `labels` field: [{"name": n} for n in names].
func eventsLabels(names ...string) []any {
	out := []any{}
	for _, n := range names {
		out = append(out, pyjson.New("name", n))
	}
	return out
}

// eventsStr is a pointer to a string, for testsupport.Ctx's TrustText.
func eventsStr(s string) *string { return &s }

// ---------------------------------------------------------------------------- test_events.py

const eventsTrust = "jgoetzmann 3 id:95732896\nhelper 2\nreader 1\n"

var (
	eventsHelper = pyjson.New("login", "helper", "id", 7)
	eventsReader = pyjson.New("login", "reader", "id", 8)
)

// eventsCommentOpts is comment_event's keyword arguments; a zero field is the default.
type eventsCommentOpts struct {
	User        *pyjson.Object // = OPERATOR
	Association string         // = "OWNER"
	PR          bool           // = False
}

func eventsCommentEvent(number int, body string, opts ...eventsCommentOpts) *pyjson.Object {
	var o eventsCommentOpts
	if len(opts) > 0 {
		o = opts[0]
	}
	user := o.User
	if user == nil {
		user = fakes.Operator
	}
	association := o.Association
	if association == "" {
		association = "OWNER"
	}
	issue := pyjson.New("number", number, "title", "t")
	if o.PR {
		issue.Set("pull_request", pyjson.New())
	}
	return pyjson.New("action", "created", "sender", user, "issue", issue,
		"comment", pyjson.New("id", 501, "body", body, "user", user, "author_association", association))
}

// eventsComments is CommentTests' world.
type eventsComments struct {
	t   *testing.T
	gh  *fakes.FakeGitHub
	ctx *hctx.Context
}

func newEventsComments(t *testing.T) *eventsComments {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5, fakes.Issue{Title: "Make the Coin shiny"})
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, TrustText: eventsStr(eventsTrust)})
	return &eventsComments{t: t, gh: gh, ctx: ctx}
}

func (w *eventsComments) send(body string, opts ...eventsCommentOpts) []string {
	return events.Handle(w.ctx, "issue_comment", eventsCommentEvent(5, body, opts...))
}

func (w *eventsComments) reply() string {
	w.t.Helper()
	return eventsLast(w.t, w.gh.BotComments(5))
}

func (w *eventsComments) reactions(cid ...int) []string {
	id := 501
	if len(cid) > 0 {
		id = cid[0]
	}
	out := []string{}
	for _, r := range w.gh.Reacted {
		if r.ID == id {
			out = append(out, r.Content)
		}
	}
	return out
}

func (w *eventsComments) items() any { return eventsDig(w.t, w.ctx.Store.Load(), "items") }

func TestEvents_BuildQueuesTheIssue(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness build")
	eventsEqual(t, w.gh.LabelNames(5), eventsSet(config.LabelBuild))
	eventsIn(t, "Queued #5", w.reply())
	eventsIn(t, fakes.Reaction{ID: 501, Content: "eyes"}, w.gh.Reacted)
	eventsEqual(t, w.gh.Dispatches, []fakes.Dispatch{})
	eventsEqual(t, eventsDig(t, w.items(), "5", "requested_by"), "jgoetzmann")
}

func TestEvents_ForceStartsARunNow(t *testing.T) {
	w := newEventsComments(t)
	w.send("@jgoetzmann-bot build --force")
	eventsEqual(t, w.gh.Dispatches[0].Inputs, map[string]string{"item": "5", "force": "true", "mode": "build"})
}

func TestEvents_ForceNeedsTheOperator(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness build --force", eventsCommentOpts{User: eventsHelper, Association: "COLLABORATOR"})
	eventsEqual(t, w.gh.Dispatches, []fakes.Dispatch{})
	eventsEqual(t, w.gh.LabelNames(5), eventsSet())
	eventsIn(t, "operator level", w.reply())
}

func TestEvents_AMaintainerMayBuildWithoutForce(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness build", eventsCommentOpts{User: eventsHelper, Association: "COLLABORATOR"})
	eventsEqual(t, w.gh.LabelNames(5), eventsSet(config.LabelBuild))
}

func TestEvents_AnAskerMayOnlyAsk(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness status\n/harness build", eventsCommentOpts{User: eventsReader, Association: "COLLABORATOR"})
	eventsIn(t, "Night bot status", w.reply())
	eventsIn(t, "maintainer level", w.reply())
	eventsEqual(t, w.gh.LabelNames(5), eventsSet())
}

func TestEvents_StrangersAreIgnoredSilently(t *testing.T) {
	w := newEventsComments(t)
	out := w.send("/harness halt", eventsCommentOpts{User: fakes.Stranger, Association: "NONE"})
	eventsIn(t, "not on the trust list", out[0])
	eventsEqual(t, w.gh.BotComments(5), []string{})
	eventsFalse(t, eventsDig(t, w.ctx.Store.Load(), "halted"))
	// A listed name without an id needs GitHub's word too.
	w.send("/harness build", eventsCommentOpts{User: eventsHelper, Association: "CONTRIBUTOR"})
	eventsEqual(t, w.gh.LabelNames(5), eventsSet())
}

func TestEvents_TheBotsOwnCommentsAreIgnored(t *testing.T) {
	w := newEventsComments(t)
	out := events.Handle(w.ctx, "issue_comment",
		eventsCommentEvent(5, "/harness build\n"+config.Marker, eventsCommentOpts{User: fakes.Bot}))
	eventsEqual(t, out, []string{"ignored: the bot's own event"})
	out = events.Handle(w.ctx, "issue_comment",
		eventsCommentEvent(5, "/harness build\n"+config.Marker, eventsCommentOpts{User: fakes.Operator}))
	eventsEqual(t, out, []string{"ignored: the bot's own comment"})
}

func TestEvents_QuotingTheBotStillLetsAPersonCommand(t *testing.T) {
	w := newEventsComments(t)
	w.send("> earlier reply\n> " + config.Marker + "\n/harness build")
	eventsEqual(t, w.gh.LabelNames(5), eventsSet(config.LabelBuild))
}

func TestEvents_HaltAndStart(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness halt going on holiday")
	s := w.ctx.Store.Load()
	eventsTrue(t, eventsDig(t, s, "halted"))
	eventsEqual(t, eventsDig(t, s, "halt", "reason"), "going on holiday")
	w.send("/harness start")
	eventsFalse(t, eventsDig(t, w.ctx.Store.Load(), "halted"))
}

func TestEvents_HaltNeedsTheOperator(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness halt", eventsCommentOpts{User: eventsHelper, Association: "COLLABORATOR"})
	eventsFalse(t, eventsDig(t, w.ctx.Store.Load(), "halted"))
}

func TestEvents_StatusReportsTheQueueAndWindow(t *testing.T) {
	w := newEventsComments(t)
	w.gh.AddIssue(6, fakes.Issue{Labels: []string{config.LabelBuild}})
	w.send("/harness status")
	text := w.reply()
	eventsIn(t, "Queued to build: #6", text)
	eventsIn(t, "`claude-1` (claude: `opus` strong, `sonnet` medium; 21:00–07:00 "+
		"America/Chicago): outside its "+
		"hours", text)
	eventsIn(t, "`claude-2` (claude: `opus` strong, `sonnet` medium; 21:00–07:00 "+
		"America/Chicago): its secret "+
		"`CLAUDE_CODE_OAUTH_TOKEN_2` is not set", text)
}

func TestEvents_FreeTextOnAnIssueAsksForABuild(t *testing.T) {
	w := newEventsComments(t)
	w.send("@jgoetzmann-bot please also make it sparkle")
	eventsEqual(t, w.gh.LabelNames(5), eventsSet(config.LabelBuild))
	eventsIn(t, "read your notes", w.reply())
}

func TestEvents_ABuildRequestOnAnIssueWithAPRRevisesThePR(t *testing.T) {
	w := newEventsComments(t)
	w.gh.Threads.Item(5).Set("labels", eventsLabels(config.LabelPROpen))
	w.gh.AddPull(9, "bot/issue-5", fakes.Pull{Labels: []string{config.LabelPR}})
	w.gh.Threads.Item(9).Set("auto_merge", pyjson.New("merge_method", "squash"))
	w.gh.AutoMerge["PR_9"] = "squash"
	w.send("@jgoetzmann-bot also make it sparkle")
	eventsEqual(t, w.gh.LabelNames(9), eventsSet(config.LabelPR, config.LabelRevise))
	eventsNotIn(t, "PR_9", w.gh.AutoMerge)
}

func TestEvents_StopTakesItOutAndTellsARunningJob(t *testing.T) {
	w := newEventsComments(t)
	w.gh.Threads.Item(5).Set("labels", eventsLabels(config.LabelWorking))
	w.send("/harness stop")
	// The run still holds it until it stops, so a new request waits for that run.
	eventsEqual(t, w.gh.LabelNames(5), eventsSet(config.LabelWorking))
	eventsTrue(t, eventsDig(t, w.items(), "5", "stop_requested"))
	eventsIn(t, "next checkpoint", w.reply())
}

func TestEvents_SuggestAndRun(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness suggest")
	eventsTrue(t, eventsDig(t, w.ctx.Store.Load(), "suggest", "requested"))
	w.send("/harness suggest --force")
	eventsEqual(t, eventsLast(t, w.gh.Dispatches).Inputs["mode"], "suggest")
	w.send("/harness run #5")
	eventsEqual(t, eventsLast(t, w.gh.Dispatches).Inputs, map[string]string{"item": "5", "force": "true", "mode": "auto"})
}

func TestEvents_AMisspeltVerbRunsNothingAndAsks(t *testing.T) {
	w := newEventsComments(t)
	w.send("@jgoetzmann-bot stauts")
	eventsIn(t, "`stauts` is not a command; did you mean `status`?", w.reply())
	eventsEqual(t, w.gh.LabelNames(5), eventsSet())
	eventsEqual(t, w.gh.Reacted, []fakes.Reaction{{ID: 501, Content: "eyes"}, {ID: 501, Content: "rocket"}})
	eventsNotIn(t, "5", w.items())
}

func TestEvents_FreeTextAfterTheSlashAsksForABuildToo(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness please make it sparkle")
	eventsEqual(t, w.gh.LabelNames(5), eventsSet(config.LabelBuild))
	eventsIn(t, "read your notes", w.reply())
}

func TestEvents_HelpAndStatus(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness help")
	eventsIn(t, "**Commands.**", w.reply())
	w.send("@jgoetzmann-bot help build")
	eventsIn(t, "**`build [notes]`** (level 2, maintainer; also written `work`)", w.reply())
	w.send("/harness status")
	eventsIn(t, "Night bot status", w.reply())
	eventsIn(t, "lists the commands", w.reply())
	eventsNotIn(t, "| Verb |", w.reply())
}

func TestEvents_WorkForAModelGetsAThumbsUpAndIsKeptOnTheRecord(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness build")
	eventsEqual(t, w.reactions(), []string{"eyes", "+1", "rocket"})
	eventsEqual(t, eventsDig(t, w.items(), "5", "asks"), []any{"c:501"})
}

func TestEvents_ACommandWithNoModelWorkGetsNoThumbsUp(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness status")
	eventsEqual(t, w.reactions(), []string{"eyes", "rocket"})
	w.gh.Threads.Item(5).Set("state", "closed")
	payload := eventsCommentEvent(5, "/harness build")
	comment := eventsCommentEvent(5, "").Obj("comment").Copy()
	comment.Set("id", 502)
	comment.Set("body", "/harness build")
	payload.Set("comment", comment)
	events.Handle(w.ctx, "issue_comment", payload)
	eventsIn(t, "is closed", w.reply())
	eventsEqual(t, w.reactions(502), []string{"eyes", "rocket"})
}

func TestEvents_ANoteDuringARunWaitsForTheNextPass(t *testing.T) {
	w := newEventsComments(t)
	w.gh.Threads.Item(5).Set("labels", eventsLabels(config.LabelWorking))
	w.send("@jgoetzmann-bot also make it sparkle")
	eventsEqual(t, w.reactions(), []string{"eyes", "+1", "rocket"})
	record := eventsDig(t, w.items(), "5")
	eventsEqual(t, []any{eventsDig(t, record, "pending_request"), eventsDig(t, record, "asks")},
		[]any{true, []any{"c:501"}})
}

func TestEvents_StopAnswersWhatWasWaiting(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness build")
	payload := eventsCommentEvent(5, "")
	payload.Set("comment", pyjson.New("id", 502, "body", "/harness stop", "user", fakes.Operator,
		"author_association", "OWNER"))
	events.Handle(w.ctx, "issue_comment", payload)
	eventsEqual(t, w.reactions(), []string{"eyes", "+1", "rocket", "confused"})
	eventsEqual(t, w.reactions(502), []string{"eyes", "rocket"})
	eventsEqual(t, eventsDig(t, w.items(), "5", "asks"), []any{})
}

func TestEvents_ARequestRoutedToThePullRequestKeepsItsComment(t *testing.T) {
	w := newEventsComments(t)
	w.gh.Threads.Item(5).Set("labels", eventsLabels(config.LabelPROpen))
	w.gh.AddPull(9, "bot/issue-5", fakes.Pull{Labels: []string{config.LabelPR}})
	w.send("@jgoetzmann-bot also make it sparkle")
	eventsEqual(t, eventsDig(t, w.items(), "9", "asks"), []any{"c:501"})
}

func TestEvents_ASuggestRequestWaitsForTheSurvey(t *testing.T) {
	w := newEventsComments(t)
	w.send("/harness suggest")
	eventsEqual(t, w.reactions(), []string{"eyes", "+1", "rocket"})
	eventsEqual(t, eventsDig(t, w.ctx.Store.Load(), "suggest", "asks"), []any{"c:501"})
}

func TestEvents_ALineCommentOnADiffIsKeptByItsOwnKind(t *testing.T) {
	w := newEventsComments(t)
	w.gh.AddPull(9, "feature/mine")
	events.Handle(w.ctx, "pull_request_review_comment", pyjson.New(
		"action", "created", "sender", fakes.Operator, "pull_request", pyjson.New("number", 9),
		"comment", pyjson.New("id", 77, "body", "@jgoetzmann-bot rename this", "user", fakes.Operator,
			"author_association", "OWNER")))
	eventsEqual(t, eventsDig(t, w.items(), "9", "asks"), []any{"rc:77"})
	eventsIn(t, fakes.Reaction{ID: 77, Content: "+1"}, w.gh.Reacted)
}

func TestEvents_ReviseOnAPullRequestFromAForkIsRefused(t *testing.T) {
	w := newEventsComments(t)
	w.gh.AddPull(9, "their-branch", fakes.Pull{Fork: true})
	events.Handle(w.ctx, "issue_comment", eventsCommentEvent(9, "/harness revise x", eventsCommentOpts{PR: true}))
	eventsIn(t, "comes from a fork", eventsLast(t, w.gh.BotComments(9)))
	eventsEqual(t, w.gh.LabelNames(9), eventsSet())
}

func TestEvents_APersonCanAskForChangesOnTheirOwnPullRequest(t *testing.T) {
	w := newEventsComments(t)
	w.gh.AddPull(9, "feature/mine")
	events.Handle(w.ctx, "issue_comment",
		eventsCommentEvent(9, "@jgoetzmann-bot fix the failing test", eventsCommentOpts{PR: true}))
	eventsEqual(t, w.gh.LabelNames(9), eventsSet(config.LabelRevise))
}

// eventsLabelWorld is LabelAndAssignTests' world.
func eventsLabelWorld(t *testing.T) (*fakes.FakeGitHub, *hctx.Context) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5, fakes.Issue{Labels: []string{config.LabelBuild}})
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, TrustText: eventsStr(eventsTrust)})
	return gh, ctx
}

func TestEvents_TheBuildLabelFromATrustedPersonQueues(t *testing.T) {
	gh, ctx := eventsLabelWorld(t)
	payload := pyjson.New("action", "labeled", "sender", fakes.Operator, "label", pyjson.New("name", config.LabelBuild),
		"issue", pyjson.New("number", 5))
	events.Handle(ctx, "issues", payload)
	eventsEqual(t, gh.LabelNames(5), eventsSet(config.LabelBuild))
	eventsIn(t, "Queued #5", eventsLast(t, gh.BotComments(5)))
	eventsIn(t, "queued_at", eventsDig(t, ctx.Store.Load(), "items", "5"))
}

func TestEvents_TheBuildLabelFromAnyoneElseIsRemoved(t *testing.T) {
	gh, ctx := eventsLabelWorld(t)
	payload := pyjson.New("action", "labeled", "sender", fakes.Stranger, "label", pyjson.New("name", config.LabelBuild),
		"issue", pyjson.New("number", 5))
	events.Handle(ctx, "issues", payload)
	eventsEqual(t, gh.LabelNames(5), eventsSet())
}

func TestEvents_AssigningTheBotQueues(t *testing.T) {
	gh, ctx := eventsLabelWorld(t)
	gh.Threads.Item(5).Set("labels", []any{})
	payload := pyjson.New("action", "assigned", "sender", fakes.Operator, "assignee", fakes.Bot, "issue", pyjson.New("number", 5))
	events.Handle(ctx, "issues", payload)
	eventsEqual(t, gh.LabelNames(5), eventsSet(config.LabelBuild))
}

func TestEvents_AssigningSomeoneElseDoesNothing(t *testing.T) {
	gh, ctx := eventsLabelWorld(t)
	gh.Threads.Item(5).Set("labels", []any{})
	payload := pyjson.New("action", "assigned", "sender", fakes.Operator, "assignee", fakes.Stranger, "issue", pyjson.New("number", 5))
	events.Handle(ctx, "issues", payload)
	eventsEqual(t, gh.LabelNames(5), eventsSet())
}

func TestEvents_TheReviseLabelOnAPullRequest(t *testing.T) {
	gh, ctx := eventsLabelWorld(t)
	gh.AddPull(9, "bot/issue-5", fakes.Pull{Labels: []string{config.LabelPR, config.LabelRevise}})
	payload := pyjson.New("action", "labeled", "sender", fakes.Operator, "label", pyjson.New("name", config.LabelRevise),
		"pull_request", pyjson.New("number", 9))
	events.Handle(ctx, "pull_request_target", payload)
	eventsEqual(t, eventsDig(t, ctx.Store.Load(), "items", "9", "source"), "request")
}

// eventsReviewWorld is ReviewAndCiTests' world.
type eventsReviewWorld struct {
	gh   *fakes.FakeGitHub
	pull *pyjson.Object
	ctx  *hctx.Context
}

func newEventsReviewWorld() *eventsReviewWorld {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5, fakes.Issue{Labels: []string{config.LabelPROpen}})
	pull := gh.AddPull(9, "bot/issue-5", fakes.Pull{Body: "Closes #5", Labels: []string{config.LabelPR}, SHA: "s1"})
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, TrustText: eventsStr(eventsTrust)})
	return &eventsReviewWorld{gh: gh, pull: pull, ctx: ctx}
}

// ci is ReviewAndCiTests.ci(attempt=1, sha="s1").
func (w *eventsReviewWorld) ci(attempt int, sha string) []string {
	run := pyjson.New("name", "CI", "conclusion", "failure", "head_branch", "bot/issue-5", "head_sha", sha,
		"id", 99, "run_attempt", attempt, "html_url", "u", "pull_requests", []any{pyjson.New("number", 9)})
	// The bot pushed the branch, so GitHub names it as the sender of the CI run.
	return events.Handle(w.ctx, "workflow_run",
		pyjson.New("action", "completed", "sender", fakes.Bot, "workflow_run", run))
}

func TestEvents_ChangesRequestedOnABotPRQueuesARevision(t *testing.T) {
	w := newEventsReviewWorld()
	payload := pyjson.New("action", "submitted", "sender", fakes.Operator, "pull_request", w.gh.GetPull(9),
		"review", pyjson.New("state", "changes_requested", "body", "Name it better.",
			"user", fakes.Operator, "author_association", "OWNER"))
	events.Handle(w.ctx, "pull_request_review", payload)
	eventsIn(t, config.LabelRevise, w.gh.LabelNames(9))
}

func TestEvents_CIFailureIsRerunOnceThenFixed(t *testing.T) {
	w := newEventsReviewWorld()
	w.ci(1, "s1")
	eventsEqual(t, w.gh.Reruns, []any{99})
	eventsNotIn(t, config.LabelRevise, w.gh.LabelNames(9))
	w.ci(2, "s1")
	eventsIn(t, config.LabelRevise, w.gh.LabelNames(9))
	record := eventsDig(t, w.ctx.Store.Load(), "items", "9")
	eventsEqual(t, []any{eventsDig(t, record, "source"), eventsDig(t, record, "ci_run_id"), eventsDig(t, record, "ci_fixes")},
		[]any{"ci", 99, 1})
}

func TestEvents_ActionsCallsUseTheJobsOwnToken(t *testing.T) {
	w := newEventsReviewWorld()
	actions := fakes.NewFakeGitHub()
	w.ctx.ActionsGH = actions
	w.ci(1, "s1")
	eventsEqual(t, []any{actions.Reruns, w.gh.Reruns}, []any{[]any{99}, []any{}})
	w.ctx.Dispatch(nil, true, "auto")
	eventsEqual(t, len(actions.Dispatches), 1)
	eventsEqual(t, w.gh.Dispatches, []fakes.Dispatch{})
}

func TestEvents_CIFixesAreCapped(t *testing.T) {
	w := newEventsReviewWorld()
	eventsUpdateItem(w.ctx, 9, "ci_fixes", 3, "ci_reruns", pyjson.New("s1", 1))
	w.gh.Threads.Item(9).Set("auto_merge", pyjson.New("merge_method", "squash"))
	w.gh.AutoMerge["PR_9"] = "squash"
	w.ci(2, "s1")
	eventsEqual(t, w.gh.LabelNames(9), eventsSet(config.LabelPR, config.LabelBlocked))
	eventsNotIn(t, "PR_9", w.gh.AutoMerge)
	eventsIn(t, "It needs a person", eventsLast(t, w.gh.BotComments(9)))
}

func TestEvents_APersonAskingAgainDoesNotResetTheCICap(t *testing.T) {
	w := newEventsReviewWorld()
	eventsUpdateItem(w.ctx, 9, "ci_fixes", 2)
	events.Handle(w.ctx, "issue_comment",
		eventsCommentEvent(9, "/harness revise try again", eventsCommentOpts{PR: true}))
	eventsEqual(t, eventsDig(t, w.ctx.Store.Load(), "items", "9", "ci_fixes"), 2)
}

func TestEvents_CILeavesAStoppedOrBlockedPRAlone(t *testing.T) {
	w := newEventsReviewWorld()
	eventsUpdateItem(w.ctx, 9, "stop_requested", true)
	w.ci(3, "s1")
	eventsEqual(t, w.gh.Reruns, []any{})
	eventsEqual(t, w.gh.LabelNames(9), eventsSet(config.LabelPR))
}

func TestEvents_CIOnAStaleCommitOrAHumanPRIsIgnored(t *testing.T) {
	w := newEventsReviewWorld()
	run := pyjson.New("name", "CI", "conclusion", "failure", "head_branch", "bot/issue-5", "head_sha", "old",
		"id", 99, "run_attempt", 3, "pull_requests", []any{pyjson.New("number", 9)})
	events.Handle(w.ctx, "workflow_run", pyjson.New("action", "completed", "sender", fakes.Operator,
		"workflow_run", run))
	eventsEqual(t, w.gh.LabelNames(9), eventsSet(config.LabelPR))
	w.gh.AddPull(10, "feature/human", fakes.Pull{SHA: "h1"})
	run = pyjson.New("name", "CI", "conclusion", "failure", "head_branch", "feature/human", "head_sha", "h1",
		"id", 98, "run_attempt", 3, "pull_requests", []any{pyjson.New("number", 10)})
	events.Handle(w.ctx, "workflow_run", pyjson.New("action", "completed", "sender", fakes.Operator,
		"workflow_run", run))
	eventsEqual(t, w.gh.LabelNames(10), eventsSet())
}

func TestEvents_AMergedBotPRClearsTheIssue(t *testing.T) {
	w := newEventsReviewWorld()
	pull := w.gh.GetPull(9)
	pull.Set("merged", true)
	// Auto-merge is credited to whoever turned it on: the bot.
	events.Handle(w.ctx, "pull_request_target", pyjson.New("action", "closed", "sender", fakes.Bot,
		"pull_request", pull))
	eventsEqual(t, w.gh.LabelNames(5), eventsSet())
	eventsTrue(t, eventsDig(t, w.ctx.Store.Load(), "items", "9", "merged"))
	eventsEqual(t, eventsDig(t, w.gh.GetIssue(5), "state"), "closed")
	eventsEqual(t, eventsDig(t, w.gh.GetIssue(5), "state_reason"), "completed")
}

func TestEvents_AMergedBotPRLeavesAnIssueGithubAlreadyClosed(t *testing.T) {
	w := newEventsReviewWorld()
	w.gh.UpdateIssue(5, pyjson.New("state", "closed", "state_reason", "not_planned"))
	pull := w.gh.GetPull(9)
	pull.Set("merged", true)
	events.Handle(w.ctx, "pull_request_target", pyjson.New("action", "closed", "sender", fakes.Operator,
		"pull_request", pull))
	eventsEqual(t, eventsDig(t, w.gh.GetIssue(5), "state_reason"), "not_planned")
}

func TestEvents_ABotPRMergedOutsideTheDefaultBranchLeavesTheIssueOpen(t *testing.T) {
	w := newEventsReviewWorld()
	pull := w.gh.GetPull(9)
	pull.Update(pyjson.New("merged", true, "base", pyjson.New("ref", "release")))
	events.Handle(w.ctx, "pull_request_target", pyjson.New("action", "closed", "sender", fakes.Operator,
		"pull_request", pull))
	eventsEqual(t, eventsDig(t, w.gh.GetIssue(5), "state"), "open")
}

func TestEvents_ABotPRClosedUnmergedSaysHowToRetry(t *testing.T) {
	w := newEventsReviewWorld()
	pull := w.gh.GetPull(9)
	events.Handle(w.ctx, "pull_request_target", pyjson.New("action", "closed", "sender", fakes.Operator,
		"pull_request", pull))
	eventsIn(t, "closed without merging", eventsLast(t, w.gh.BotComments(5)))
}

// eventsQueueWorld is MergeQueueTests' world: `main`'s merge queue (infra/github.tf): a cleared
// head its build took out gets a CI fix at once; one still queued is the queue's; one taken out
// with no failed build goes back once per commit of `main`; a queued pull request is held by
// taking it out.
type eventsQueueWorld struct {
	gh  *fakes.FakeGitHub
	ctx *hctx.Context
}

// eventsQueue is MergeQueueTests.QUEUE.
var eventsQueue = map[string]string{"actor": "github-merge-queue", "type": "Bot", "reason": "CONFLICT"}

func newEventsQueueWorld() *eventsQueueWorld {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(5, fakes.Issue{Labels: []string{config.LabelPROpen}})
	gh.AddPull(9, "bot/issue-5", fakes.Pull{Body: "Closes #5", Labels: []string{config.LabelPR}, SHA: "s1"})
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, TrustText: eventsStr(eventsTrust)})
	eventsUpdateItem(ctx, 9, "cleared", pyjson.New("sha", "s1"))
	return &eventsQueueWorld{gh: gh, ctx: ctx}
}

// queueCI is queue_ci(branch="gh-readonly-queue/main/pr-9-abc", conclusion="cancelled").
func (w *eventsQueueWorld) queueCI(branch, conclusion string) []string {
	// A merge group's run names no pull request; its branch does. It ends cancelled once a
	// required check failed (ci.yml's `queue-cancel-*`).
	run := pyjson.New("name", "CI", "conclusion", conclusion, "head_branch", branch, "head_sha", "mg",
		"id", 77, "html_url", "q", "pull_requests", []any{}, "event", "merge_group")
	return events.Handle(w.ctx, "workflow_run",
		pyjson.New("action", "completed", "sender", fakes.Bot, "workflow_run", run))
}

const eventsQueueBranch = "gh-readonly-queue/main/pr-9-abc"

func (w *eventsQueueWorld) housekeeping() []string {
	return plan.Housekeeping(w.ctx, w.ctx.Store.Load())
}

func eventsCopyMap(m map[string]string) map[string]string {
	out := map[string]string{}
	for k, v := range m {
		out[k] = v
	}
	return out
}

func TestEvents_AClearedHeadItsBuildTookOutGetsACIFixAtOnce(t *testing.T) {
	w := newEventsQueueWorld()
	w.queueCI(eventsQueueBranch, "cancelled")
	eventsEqual(t, []any{w.gh.AutoMerge, w.gh.Reruns}, []any{map[string]string{}, []any{}})
	eventsIn(t, config.LabelRevise, w.gh.LabelNames(9))
	record := eventsDig(t, w.ctx.Store.Load(), "items", "9")
	eventsEqual(t, []any{eventsDig(t, record, "source"), eventsDig(t, record, "ci_run_id"), eventsDig(t, record, "ci_fixes")},
		[]any{"ci", 77, 1})
	eventsIn(t, "The merge queue took this out", eventsLast(t, w.gh.BotComments(9)))
}

func TestEvents_OneStillQueuedAHeldOneOrAnUnclearedHeadIsLeftAlone(t *testing.T) {
	w := newEventsQueueWorld()
	w.gh.MergeQueue = map[string]bool{"PR_9": true} // built again without a pull request ahead that failed
	w.queueCI(eventsQueueBranch, "cancelled")
	w.gh.MergeQueue = map[string]bool{}
	eventsUpdateItem(w.ctx, 9, "stop_requested", true)
	w.queueCI(eventsQueueBranch, "cancelled")
	eventsUpdateItem(w.ctx, 9, "stop_requested", false, "cleared", pyjson.New("sha", "s0"))
	w.queueCI(eventsQueueBranch, "failure")
	w.gh.AddPull(10, "people/x", fakes.Pull{SHA: "p1"})
	w.queueCI("gh-readonly-queue/main/pr-10-abc", "cancelled")
	eventsEqual(t, w.gh.AutoMerge, map[string]string{})
	eventsEqual(t, w.gh.LabelNames(9), eventsSet(config.LabelPR))
	eventsEqual(t, w.gh.BotComments(9), []string{})
	eventsEqual(t, w.gh.Reruns, []any{})
}

func TestEvents_OneTakenOutWithNoFailedBuildGoesBackOncePerCommitOfMain(t *testing.T) {
	w := newEventsQueueWorld()
	w.gh.QueueRemovals["PR_9"] = eventsCopyMap(eventsQueue)
	eventsIn(t, "#9: back in the merge queue (taken out: CONFLICT)", w.housekeeping())
	eventsEqual(t, w.gh.AutoMergeHeads["PR_9"], "s1")
	w.gh.DisableAutoMerge("PR_9") // queued, and out again on the same `main`
	eventsNotIn(t, "#9: back", strings.Join(w.housekeeping(), " "))
	w.gh.Patch.BranchSHA = func(branch string) string { return "main-2" } // `main` moved on
	w.housekeeping()
	eventsIn(t, "PR_9", w.gh.AutoMerge)
}

func TestEvents_APersonWhoTookItOutOrAHeadNeverClearedKeepsItOut(t *testing.T) {
	w := newEventsQueueWorld()
	w.gh.QueueRemovals["PR_9"] = map[string]string{"actor": "jgoetzmann", "type": "User", "reason": "MANUAL"}
	w.housekeeping()
	w.gh.QueueRemovals["PR_9"] = eventsCopyMap(eventsQueue)
	eventsUpdateItem(w.ctx, 9, "cleared", pyjson.New("sha", "s0"))
	w.housekeeping()
	eventsEqual(t, w.gh.AutoMerge, map[string]string{})
}

func TestEvents_ARevisionOrAStopTakesAQueuedPullRequestOut(t *testing.T) {
	w := newEventsQueueWorld()
	for _, body := range []string{"/harness revise name it better", "/harness stop"} {
		w.gh.MergeQueue = map[string]bool{"PR_9": true} // queued: REST `auto_merge` reads null
		events.Handle(w.ctx, "issue_comment", eventsCommentEvent(9, body, eventsCommentOpts{PR: true}))
		eventsEqual(t, w.gh.MergeQueue, map[string]bool{}, body)
	}
}

// StatusTests: `/harness status` says which subscriptions are running, on what, and for how long.

func TestEvents_WhichSubscriptionsAreRunningNow(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	for _, number := range []int{37, 49, 50} {
		gh.AddIssue(number, fakes.Issue{Labels: []string{config.LabelWorking}})
	}
	gh.Runs.Update(pyjson.New("101", pyjson.New("status", "in_progress"), "102", pyjson.New("status", "queued"),
		"103", pyjson.New("status", "in_progress"))) // run 999 has ended (404)
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Night})
	ago := func(minutes int) string {
		return clock.ISO(testsupport.Night.Add(-time.Duration(minutes) * time.Minute))
	}
	ctx.Store.Update(func(s *pyjson.Object) any {
		state.Item(s, 37).Update(pyjson.New("provider", "claude-1", "kind", "build", "run_id", "101",
			"started_at", ago(47)))
		state.Item(s, 49).Update(pyjson.New("provider", "muse", "kind", "revise", "run_id", "102",
			"started_at", ago(123)))
		state.Item(s, 50).Update(pyjson.New("provider", "gpt", "kind", "build", "run_id", "999",
			"started_at", ago(300)))
		s.Obj("suggest").Update(pyjson.New("provider", "agy", "run_id", "103", "last_run", ago(0)))
		return nil
	}, "seed")
	text := status.Report(ctx)
	runs := "https://github.com/jgoetzmann/JackiOh/actions/runs"
	eventsIn(t, strings.Join([]string{
		"- **Running now** (1 of 8 lanes on GitHub's runners, 7 free; 2 of " +
			fmt.Sprint(ctx.Cfg.Pool.MachineParallel) + " on the night box):",
		"  - `claude-1` (claude, `opus`): building #37, for 47m, [run](" + runs + "/101).",
		"  - `muse` (muse, `muse-spark-1.3-contributor`): revising #49, for 2h 03m, " +
			"[run](" + runs + "/102).",
		"  - `agy` (agy, `gemini-3.8-flash-high`): a suggestion survey, just started, [run](" + runs + "/103).",
	}, "\n"), text)
	// A run that ended holds nothing, and its subscription says why it is or is not free.
	eventsNotIn(t, "`gpt` (codex", strings.SplitN(text, "- Subscriptions", 2)[0])
	eventsIn(t, "  - `claude-1` (claude: `opus` strong, `sonnet` medium; 21:00–07:00 "+
		"America/Chicago): "+
		"**working on #37**.", text)
	eventsNotIn(t, "`gpt` (codex, `gpt-5.6-sol`, any time): **working", text)
	eventsIn(t, "- Working on: #37, #49, #50 (no run is going for #50 any more; the next "+
		"plan requeues it).", text)
}

func TestEvents_NothingRunning(t *testing.T) {
	text := status.Report(testsupport.MakeCtx(fakes.NewFakeGitHub(), testsupport.Ctx{At: testsupport.Night}))
	eventsIn(t, "- Running now: nothing (8 of 8 lanes free on GitHub's runners, 6 on the "+
		"night box).", text)
}
