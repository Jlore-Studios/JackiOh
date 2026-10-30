package tests

// test_cross.py: several subscriptions end to end: the review rule (one strong approval, or two
// medium ones, the same model twice included, or for an easy item a weak and a medium one),
// review runs, `difficulty:hard`, handoff and the vault, through plan, work and deliver against a
// real repository and the fake GitHub.
//
// What test_cross.py, test_fullsend.py, test_resolve.py, test_review_fixes.py and test_plan_draft.py
// share: unittest's assertions, and the helpers they import from test_flow.py (Harness),
// test_work.py (builder, reviewer, changes, APPROVE, DONE, GATES) and test_providers.py
// (raw_providers, secrets), each named after the file that uses it first so the files porting
// those modules keep their own names.

import (
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"slices"
	"sort"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/vault"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// ---------------------------------------------------------------------------- assertions

// crossShow is a value as a failure message prints it.
func crossShow(v any) string {
	switch x := v.(type) {
	case *pyjson.Object, []any:
		return pyjson.Dumps(x)
	case string:
		return fmt.Sprintf("%q", x)
	case map[string]bool:
		return fmt.Sprint(crossSorted(x))
	case map[int]bool:
		keys := []int{}
		for k := range x {
			keys = append(keys, k)
		}
		sort.Ints(keys)
		return fmt.Sprint(keys)
	}
	return fmt.Sprintf("%#v", v)
}

// crossEqual is Python's == for what the tests compare: JSON values as pyjson.Equal does, and
// Go's own values (sets, string lists) as reflect.DeepEqual does.
func crossEqual(got, want any) bool {
	return pyjson.Equal(got, want) || reflect.DeepEqual(got, want)
}

// crossEq is assertEqual.
func crossEq(t *testing.T, got, want any, msg ...any) {
	t.Helper()
	if !crossEqual(got, want) {
		t.Fatalf("assertEqual failed%s\n got: %s\nwant: %s", crossMsg(msg), crossShow(got), crossShow(want))
	}
}

// crossNe is assertNotEqual.
func crossNe(t *testing.T, got, other any, msg ...any) {
	t.Helper()
	if crossEqual(got, other) {
		t.Fatalf("assertNotEqual failed%s: both are %s", crossMsg(msg), crossShow(got))
	}
}

// crossIn is assertIn: a substring of a string, a key of a set or JSON object, an item of a list.
func crossIn(t *testing.T, needle, haystack any, msg ...any) {
	t.Helper()
	if !crossContains(needle, haystack) {
		t.Fatalf("assertIn failed%s\n%s\nnot in\n%s", crossMsg(msg), crossShow(needle), crossShow(haystack))
	}
}

// crossNotIn is assertNotIn.
func crossNotIn(t *testing.T, needle, haystack any, msg ...any) {
	t.Helper()
	if crossContains(needle, haystack) {
		t.Fatalf("assertNotIn failed%s\n%s\nis in\n%s", crossMsg(msg), crossShow(needle), crossShow(haystack))
	}
}

// crossTrue is assertTrue.
func crossTrue(t *testing.T, ok bool, msg ...any) {
	t.Helper()
	if !ok {
		t.Fatalf("assertTrue failed%s", crossMsg(msg))
	}
}

// crossFalse is assertFalse.
func crossFalse(t *testing.T, ok bool, msg ...any) {
	t.Helper()
	if ok {
		t.Fatalf("assertFalse failed%s", crossMsg(msg))
	}
}

func crossMsg(msg []any) string {
	if len(msg) == 0 {
		return ""
	}
	return ": " + fmt.Sprint(msg...)
}

func crossContains(needle, haystack any) bool {
	switch h := haystack.(type) {
	case string:
		s, ok := needle.(string)
		return ok && strings.Contains(h, s)
	case map[string]bool:
		s, ok := needle.(string)
		return ok && h[s]
	case map[int]bool:
		n, ok := needle.(int)
		return ok && h[n]
	case *pyjson.Object:
		s, ok := needle.(string)
		return ok && h.Has(s)
	case []string:
		s, ok := needle.(string)
		return ok && slices.Contains(h, s)
	case []int:
		n, ok := needle.(int)
		return ok && slices.Contains(h, n)
	case []any:
		for _, v := range h {
			if crossEqual(v, needle) {
				return true
			}
		}
		return false
	case map[string]string:
		s, ok := needle.(string)
		if !ok {
			return false
		}
		_, found := h[s]
		return found
	case nil:
		panic("argument of type 'NoneType' is not iterable")
	}
	panic(fmt.Sprintf("crossContains: cannot look in %T", haystack))
}

// crossSet is a set of names, as label_names returns.
func crossSet(names ...string) map[string]bool {
	out := map[string]bool{}
	for _, n := range names {
		out[n] = true
	}
	return out
}

// crossSorted is sorted(set).
func crossSorted(set map[string]bool) []string {
	out := []string{}
	for k := range set {
		out = append(out, k)
	}
	sort.Strings(out)
	return out
}

// crossAnd is `a & b` of two sets.
func crossAnd(a, b map[string]bool) map[string]bool {
	out := map[string]bool{}
	for k := range a {
		if b[k] {
			out[k] = true
		}
	}
	return out
}

// crossKey is v[k1][k2]...: a string indexes a JSON object, an int a list; a missing key fails
// the test as Python's KeyError would.
func crossKey(t *testing.T, v any, keys ...any) any {
	t.Helper()
	for _, k := range keys {
		switch key := k.(type) {
		case string:
			o, ok := v.(*pyjson.Object)
			if !ok {
				t.Fatalf("crossKey: %s is not an object (looking up %q)", crossShow(v), key)
			}
			found, ok := o.Lookup(key)
			if !ok {
				t.Fatalf("KeyError: %q in %s", key, crossShow(v))
			}
			v = found
		case int:
			l, ok := v.([]any)
			if !ok {
				t.Fatalf("crossKey: %s is not a list (looking up %d)", crossShow(v), key)
			}
			if key < 0 {
				key += len(l)
			}
			if key < 0 || key >= len(l) {
				t.Fatalf("IndexError: %d in %s", k, crossShow(v))
			}
			v = l[key]
		default:
			t.Fatalf("crossKey: a key of type %T", k)
		}
	}
	return v
}

// crossObj is crossKey's object.
func crossObj(t *testing.T, v any, keys ...any) *pyjson.Object {
	t.Helper()
	found := crossKey(t, v, keys...)
	o, ok := found.(*pyjson.Object)
	if !ok {
		t.Fatalf("crossObj: %s is not an object", crossShow(found))
	}
	return o
}

// crossStr is crossKey's string.
func crossStr(t *testing.T, v any, keys ...any) string {
	t.Helper()
	found := crossKey(t, v, keys...)
	s, ok := found.(string)
	if !ok {
		t.Fatalf("crossStr: %s is not a string", crossShow(found))
	}
	return s
}

// crossInt is int(v[...]).
func crossInt(t *testing.T, v any, keys ...any) int {
	t.Helper()
	found := crossKey(t, v, keys...)
	n, ok := pyjson.AsInt(found)
	if !ok {
		t.Fatalf("crossInt: %s is not a number", crossShow(found))
	}
	return n
}

// crossLast is xs[-1].
func crossLast(t *testing.T, xs []string) string {
	t.Helper()
	if len(xs) == 0 {
		t.Fatalf("IndexError: list index out of range")
	}
	return xs[len(xs)-1]
}

// crossMkdtemp is tempfile.mkdtemp().
func crossMkdtemp(t *testing.T) string {
	t.Helper()
	dir, err := os.MkdirTemp("", "tmp")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { os.RemoveAll(dir) })
	return dir
}

// crossRead is path.read_text().
func crossRead(t *testing.T, path string) string {
	t.Helper()
	text, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	return string(text)
}

// crossWrite is path.write_text(text).
func crossWrite(t *testing.T, path, text string) {
	t.Helper()
	if err := os.WriteFile(path, []byte(text), 0o666); err != nil {
		t.Fatal(err)
	}
}

// crossMkdir is path.mkdir().
func crossMkdir(t *testing.T, path string) {
	t.Helper()
	if err := os.Mkdir(path, 0o777); err != nil {
		t.Fatal(err)
	}
}

// crossMustRead is path.read_text() where a failure raises.
func crossMustRead(path string) string {
	text, err := os.ReadFile(path)
	if err != nil {
		panic(err)
	}
	return string(text)
}

// crossWriteRaise is path.write_text(text), raising on a failure.
func crossWriteRaise(path, text string) {
	if err := os.WriteFile(path, []byte(text), 0o666); err != nil {
		panic(err)
	}
}

// crossExists is path.exists().
func crossExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

// crossIsFile is path.is_file().
func crossIsFile(path string) bool {
	info, err := os.Stat(path)
	return err == nil && info.Mode().IsRegular()
}

// crossLoad is json.loads(path.read_text()), a JSON object.
func crossLoad(t *testing.T, path string) *pyjson.Object {
	t.Helper()
	v, err := pyjson.Loads([]byte(crossRead(t, path)))
	if err != nil {
		t.Fatal(err)
	}
	o, ok := v.(*pyjson.Object)
	if !ok {
		t.Fatalf("%s holds no JSON object", path)
	}
	return o
}

// crossPut is `target.parent.mkdir(parents=True, exist_ok=True); target.write_text(text)`.
func crossPut(target, text string) {
	if err := os.MkdirAll(filepath.Dir(target), 0o777); err != nil {
		panic(err)
	}
	if err := os.WriteFile(target, []byte(text), 0o666); err != nil {
		panic(err)
	}
}

// crossRoles is [c.role for c in runner.calls].
func crossRoles(r *runner.FakeRunner) []string {
	out := []string{}
	for _, c := range r.Calls {
		out = append(out, c.Role)
	}
	return out
}

// crossModels is [c.model for c in runner.calls].
func crossModels(r *runner.FakeRunner) []string {
	out := []string{}
	for _, c := range r.Calls {
		out = append(out, c.Model)
	}
	return out
}

// crossCount is list.count(x).
func crossCount(xs []string, x string) int {
	n := 0
	for _, s := range xs {
		if s == x {
			n++
		}
	}
	return n
}

// crossRunner is FakeRunner(handlers), one handler a role (Python's lone callable).
func crossRunner(handlers map[string]runner.Handler) *runner.FakeRunner {
	queues := map[string][]runner.Handler{}
	for role, h := range handlers {
		queues[role] = []runner.Handler{h}
	}
	return runner.NewFakeRunner(queues)
}

// crossAnswer is a handler that answers `text`: `lambda r: RunResult(True, text)`.
func crossAnswer(text string) runner.Handler {
	return func(*runner.RunRequest) *runner.RunResult { return runner.NewRunResult(true, text) }
}

// crossGitOK is git(...) in a `try: ... except AssertionError:`: false when git failed.
func crossGitOK(cwd string, args ...string) (string, bool) {
	var out string
	if _, failed := herr.Catch[*fakes.PyError](func() { out = fakes.Git(cwd, args...) }); failed {
		return "", false
	}
	return out, true
}

// ---------------------------------------------------------------------------- test_work.py

// crossDone is test_work.DONE.
var crossDone = `<!-- bot: {"status": "done", "title": "Make the rules v2"} -->` + "\n## What changed\nRules v2."

// crossApprove is test_work.APPROVE.
var crossApprove = `<!-- review: {"verdict": "approve", "findings": []} -->` + "\nLooked hard; fine."

// crossChanges is test_work.changes(claim, where).
func crossChanges(claim string, where ...string) string {
	at := "src/game.txt"
	if len(where) > 0 {
		at = where[0]
	}
	finding := pyjson.New("severity", "blocking", "where", at, "claim", claim, "evidence", "read it")
	return "<!-- review: " + pyjson.Dumps(pyjson.New("verdict", "changes", "findings", []any{finding})) + " -->\nNo."
}

// crossBuilder is test_work.builder(edits, text): it writes `edits` (in sorted order) and
// answers `text` (DONE by default).
func crossBuilder(edits map[string]string, text ...string) runner.Handler {
	answer := crossDone
	if len(text) > 0 {
		answer = text[0]
	}
	return func(request *runner.RunRequest) *runner.RunResult {
		names := []string{}
		for name := range edits {
			names = append(names, name)
		}
		sort.Strings(names)
		for _, name := range names {
			crossPut(filepath.Join(request.Cwd, name), edits[name])
		}
		r := runner.NewRunResult(true, answer)
		r.Usage = pyjson.New("five_hour", pyjson.New("utilization", 0.3, "resets_at", nil))
		return r
	}
}

// crossReviewer is test_work.reviewer(text).
func crossReviewer(text string) runner.Handler { return crossAnswer(text) }

// crossWorkGates is test_work.GATES.
func crossWorkGates() []any {
	return []any{
		pyjson.New("name", "rules exist", "run", "test -f src/game.txt", "timeout_minutes", 1),
		pyjson.New("name", "no broken file", "run", "test ! -f broken.txt", "timeout_minutes", 1),
	}
}

// ---------------------------------------------------------------------------- test_providers.py

// crossRawProviders is test_providers.raw_providers().
func crossRawProviders() *pyjson.Object { return testsupport.RawProviders() }

// crossSecrets is test_providers.secrets(*names).
func crossSecrets(names ...string) map[string]string {
	return map[string]string{"HARNESS_SECRETS_SET": strings.Join(names, " ")}
}

// ---------------------------------------------------------------------------- test_flow.py

// crossHarnessOpts is Harness's keyword arguments: env, at (zero: NIGHT), machine, and the
// config overrides (`**cfg_overrides`, in order; plan_lanes=None is CommittedPlanLanes).
type crossHarnessOpts struct {
	Env                map[string]string
	At                 time.Time
	Machine            []string
	Overrides          *pyjson.Object
	CommittedPlanLanes bool
}

// crossHarness is test_flow.Harness: one repository, one fake GitHub, and the three jobs of a
// night run.
type crossHarness struct {
	t           *testing.T
	root        string
	origin      string
	clone       string
	gh          *fakes.FakeGitHub
	cfg         *config.Config
	ctx         *hctx.Context
	deliverRepo string
	setup       crossHarnessSetup
}

type crossHarnessSetup struct {
	env                map[string]string
	machine            []string
	overrides          *pyjson.Object
	committedPlanLanes bool
	at                 time.Time
}

func newCrossHarness(t *testing.T, opts ...crossHarnessOpts) *crossHarness {
	t.Helper()
	var o crossHarnessOpts
	if len(opts) > 0 {
		o = opts[0]
	}
	h := &crossHarness{t: t}
	h.root = crossMkdtemp(t)
	h.origin, h.clone = fakes.MakeOrigin(h.root)
	h.gh = fakes.NewFakeGitHub()
	env := map[string]string{"GITHUB_SERVER_URL": "file://" + filepath.Join(h.root, "remote"),
		"BOT_GITHUB_TOKEN": strings.Repeat("t", 20)}
	for k, v := range o.Env {
		env[k] = v
	}
	overrides := pyjson.New("gates", crossWorkGates(), "install", pyjson.New("run", "true", "timeout_minutes", 1),
		"max_review_cycles", 2)
	overrides.Update(o.Overrides)
	at := o.At
	if at.IsZero() {
		at = testsupport.Night
	}
	h.setup = crossHarnessSetup{env: env, machine: o.Machine, overrides: overrides,
		committedPlanLanes: o.CommittedPlanLanes, at: at}
	h.cfg = testsupport.MakeConfig(testsupport.Config{Env: env, Machine: o.Machine,
		CommittedPlanLanes: o.CommittedPlanLanes, Overrides: pyjson.DeepCopy(overrides).(*pyjson.Object)})
	h.ctx = testsupport.MakeCtx(h.gh, testsupport.Ctx{Cfg: h.cfg, At: at})
	h.gh.BranchChecks = crossSet(h.cfg.RequiredChecks...)
	h.deliverRepo = filepath.Join(h.root, "deliver")
	fakes.Git(h.root, "clone", "-q", h.origin, h.deliverRepo)
	return h
}

// committedHours: from now on, the subscriptions keep the hours providers.json commits
// (claude-2 and claude-3 all day), with the same GitHub and state.
func (h *crossHarness) committedHours() {
	s := h.setup
	h.cfg = testsupport.MakeConfig(testsupport.Config{Env: s.env, Machine: s.machine, CommittedHours: true,
		CommittedPlanLanes: s.committedPlanLanes, Overrides: pyjson.DeepCopy(s.overrides).(*pyjson.Object)})
	h.ctx = testsupport.MakeCtx(h.gh, testsupport.Ctx{Cfg: h.cfg, At: s.at})
}

// night is Harness.night(runner): plan, work and deliver; the plan and the run's result. `r` is
// any runner the work job takes (a FakeRunner, or test_plan_draft's TrailRunner).
func (h *crossHarness) night(r any) (*pyjson.Object, *pyjson.Object) {
	h.t.Helper()
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	matches, _ := filepath.Glob(filepath.Join(h.root, "out-*"))
	out := filepath.Join(h.root, fmt.Sprintf("out-%d", len(matches)))
	if planned.Str("action") != "none" {
		work.NewWorker(h.cfg, planned, r, h.clone, filepath.Join(h.root, "work"), out, nil, nil, nil, nil, nil, nil).Run()
		deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	}
	result := pyjson.New()
	if crossExists(filepath.Join(out, "result.json")) {
		result = crossLoad(h.t, filepath.Join(out, "result.json"))
	}
	return planned, result
}

// originSHA is Harness.origin_sha: "" for None.
func (h *crossHarness) originSHA(branch string) string {
	sha, _ := crossGitOK(h.origin, "rev-parse", "--verify", "--quiet", "refs/heads/"+branch)
	return sha
}

// crossPullOf is int(h.gh.list_pulls(head=branch)[0]["number"]).
func crossPullOf(t *testing.T, gh *fakes.FakeGitHub, branch string) int {
	t.Helper()
	pulls := gh.ListPulls("open", branch, 200)
	if len(pulls) == 0 {
		t.Fatalf("IndexError: no pull request from %s", branch)
	}
	return crossInt(t, pulls[0], "number")
}

// crossItems is ctx.store.load()["items"].
func crossItems(t *testing.T, ctx *hctx.Context) *pyjson.Object {
	t.Helper()
	return crossObj(t, ctx.Store.Load(), "items")
}

// crossAll is ALL: every subscription's secret set.
var crossAll = map[string]string{"HARNESS_SECRETS_SET": strings.Join(providers.SecretNames, " ")}

// crossTimedBuilder is timed_builder(edits, minutes): the builder, its call ten minutes long.
func crossTimedBuilder(edits map[string]string, minutes ...float64) runner.Handler {
	m := 10.0
	if len(minutes) > 0 {
		m = minutes[0]
	}
	inner := crossBuilder(edits)
	return func(request *runner.RunRequest) *runner.RunResult {
		result := inner(request)
		result.DurationS = m * 60
		return result
	}
}

// crossNextRun is next_run: the next night is another workflow run: its votes count apart from
// the last one's.
func crossNextRun(h *crossHarness, runID string) {
	cfg := *h.cfg
	cfg.RunID = runID
	h.cfg = &cfg
	h.ctx.Cfg = &cfg
}

// crossMedium is MEDIUM: each medium subscription's model and family, for the tests that build
// on one of them.
var crossMedium = map[string][2]string{"muse": {"muse-spark-1.3-contributor", "muse"},
	"agy": {"gemini-3.8-flash-high", "gemini"}}

// ---------------------------------------------------------------------------- ReviewRuleTests

// crossBuildOnMedium is ReviewRuleTests.build_on_medium: by day the Claude accounts are closed:
// Muse, first of the medium models (#317 part 9), or `provider` when it is the only one, plans,
// builds and reviews it in one run, and its own approval is one medium vote.
func crossBuildOnMedium(t *testing.T, h *crossHarness, provider string, edits map[string]string) (int, *pyjson.Object) {
	t.Helper()
	if provider == "" {
		provider = "muse"
	}
	if len(edits) == 0 {
		edits = map[string]string{"src/game.txt": "rules v2\n"}
	}
	h.gh.AddIssue(12, fakes.Issue{Title: "Make the rules v2", Labels: []string{config.LabelBuild}})
	r := crossRunner(map[string]runner.Handler{"build": crossTimedBuilder(edits), "review": crossReviewer(crossApprove)})
	planned, result := h.night(r)
	crossEq(t, []any{planned.Get("provider"), result.Get("status")}, []any{provider, "approved"})
	crossEq(t, crossRoles(r), []string{"plan", "build", "review"})
	models := map[string]bool{}
	for _, c := range r.Calls {
		models[c.Model] = true
	}
	crossEq(t, models, crossSet(crossMedium[provider][0]))
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	return pr, result
}

// Muse's in-run approval never ships a medium item (#725 keeps Strong-only): the head waits for
// its Strong final audit, and no medium review run goes out meanwhile.
func TestCross_AMediumItemWaitsForItsStrongFinal(t *testing.T) {
	h := newCrossHarness(t, crossHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, result := crossBuildOnMedium(t, h, "", nil)
	crossEq(t, h.gh.LabelNames(pr), crossSet(config.LabelPR, config.LabelCross, "difficulty:medium"))
	crossEq(t, h.gh.AutoMerge, map[string]string{})
	votes := crossKey(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "votes")
	crossEq(t, votes, pyjson.New("sha", result.Get("head"), "builder", "muse",
		"approvals", []any{"muse"}, "rejections", []any{},
		"tiers", pyjson.New("muse", "medium"),
		"reviews", []any{pyjson.New("family", "muse", "tier", "medium", "run", "777")}))
	crossIn(t, "`muse` (medium) approved it, so it waits for one strong model's final audit (Opus)",
		crossLast(t, h.gh.BotComments(12)))
	crossEq(t, crossKey(t, h.ctx.Store.Load(), "providers", "muse", "spent", -1, "minutes"), 10.0)
	// No strong model is free by day: no review run goes out at all.
	planned, _ := h.night(crossRunner(map[string]runner.Handler{"review": crossReviewer(crossApprove)}))
	crossEq(t, planned.Get("action"), "none")
	crossEq(t, h.gh.AutoMerge, map[string]string{})
	crossEq(t, h.gh.LabelNames(pr), crossSet(config.LabelPR, config.LabelCross, "difficulty:medium"))
	// With a strong seat free, Opus audits, and that ships it.
	h.committedHours() // claude-3 works all day
	crossNextRun(h, "778")
	planned, review := h.night(crossRunner(map[string]runner.Handler{"review": crossReviewer(crossApprove)}))
	crossEq(t, []any{planned.Get("action"), planned.Get("provider"), crossKey(t, planned, "seats", "review", "tier")},
		[]any{"review", "claude-3", "strong"})
	crossEq(t, []any{review.Get("status"), review.Get("verdict")}, []any{"reviewed", "approve"})
	crossIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
}

// agy builds and its own reviewer approves; with no Strong seat set up, no review run can take
// it to the rule, so the head waits for a person to merge it (#725: no medium review runs go out
// for a medium item).
func TestCross_WithNoStrongSeatSetUpItWaitsForAPerson(t *testing.T) {
	h := newCrossHarness(t, crossHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"agy"}, At: testsupport.Day})
	pr, _ := crossBuildOnMedium(t, h, "agy", nil)
	crossEq(t, h.gh.AutoMerge, map[string]string{})
	crossIn(t, "No subscription that could give that review is set up", crossLast(t, h.gh.BotComments(12)))
	crossNextRun(h, "778")
	planned, _ := h.night(crossRunner(map[string]runner.Handler{"review": crossReviewer(crossApprove)}))
	crossEq(t, planned.Get("action"), "none")
	crossEq(t, h.gh.AutoMerge, map[string]string{})
	crossEq(t, h.gh.LabelNames(pr), crossSet(config.LabelPR, config.LabelCross, "difficulty:medium"))
}

func TestCross_ADeliverJobRunAgainDoesNotCountItsReviewTwice(t *testing.T) {
	h := newCrossHarness(t, crossHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"agy"}, At: testsupport.Day})
	pr, result := crossBuildOnMedium(t, h, "agy", nil)
	d := deliver.NewDeliverer(h.ctx, pyjson.New("action", "review", "provider", "agy"),
		filepath.Join(h.root, "unused"), h.deliverRepo, "", nil, "")
	d.ReviewSeat = h.cfg.Pool.Seat("agy", "")
	votes := d.Vote(pr, crossStr(t, result, "head"), true, false, nil) // run 777 again
	crossEq(t, len(crossKey(t, votes, "reviews").([]any)), 1)
	crossFalse(t, d.RuleMet(votes, "medium"))
}

// The weak one builds an easy item and checks itself, which is no review; the lane's Medium
// final audit then ships it, with no Opus gate (#725). No weak review run goes out.
func TestCross_AnEasyItemShipsOnItsMediumAudit(t *testing.T) {
	h := newCrossHarness(t, crossHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": ""},
		Machine: []string{"lite", "agy"}, At: testsupport.Day})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:easy"},
		Body: "<!-- jackioh-bot:plan -->\n## Plan\n1. Edit src/game.txt.\n<!-- /jackioh-bot:plan -->"})
	planned, result := h.night(crossRunner(map[string]runner.Handler{
		"build":      crossBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"self_check": crossReviewer(crossApprove)}))
	crossEq(t, []any{planned.Get("provider"), result.Get("status")}, []any{"lite", "built"})
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	crossIn(t, "a medium model's final audit", crossLast(t, h.gh.BotComments(12)))
	// The Medium audit approves, and that ships the easy item.
	crossNextRun(h, "778")
	planned, review := h.night(crossRunner(map[string]runner.Handler{"review": crossReviewer(crossApprove)}))
	crossEq(t, []any{planned.Get("provider"), crossKey(t, planned, "seats", "review", "tier")}, []any{"agy", "medium"})
	crossEq(t, review.Get("verdict"), "approve")
	crossIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	votes := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "votes")
	got := []any{}
	for _, r := range pyjson.Objs(votes.Get("reviews")) {
		got = append(got, []any{crossKey(t, r, "family"), crossKey(t, r, "tier")})
	}
	crossEq(t, got, []any{[]any{"gemini", "medium"}})
}

func TestCross_OneStrongReviewIsEnough(t *testing.T) {
	// Only claude-3 is set up: it plans, builds and reviews the item, and its one strong
	// review ships it (#695).
	h := newCrossHarness(t, crossHarnessOpts{Env: map[string]string{"HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN_3"}})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	r := crossRunner(map[string]runner.Handler{"build": crossBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": crossReviewer(crossApprove)})
	planned, result := h.night(r)
	crossEq(t, []any{planned.Get("provider"), result.Get("status")}, []any{"claude-3", "approved"})
	// Sonnet, medium, builds the (unrated, so medium) item; Opus plans and reviews it.
	got := [][2]string{}
	for _, c := range r.Calls {
		got = append(got, [2]string{c.Role, c.Model})
	}
	crossEq(t, got, [][2]string{{"plan", "opus"}, {"build", "sonnet"}, {"review", "opus"}})
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	crossIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	crossEq(t, crossKey(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "votes", "tiers"), pyjson.New("claude", "strong"))
}

func TestCross_AReviewRunPrefersAStrongModel(t *testing.T) {
	h := newCrossHarness(t, crossHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, _ := crossBuildOnMedium(t, h, "", nil)
	h.committedHours() // claude-3 works all day: a strong model is free
	planned, review := h.night(crossRunner(map[string]runner.Handler{"review": crossReviewer(crossApprove)}))
	crossEq(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"review", "claude-3"})
	crossEq(t, crossKey(t, planned, "seats", "review", "tier"), "strong")
	crossEq(t, review.Get("verdict"), "approve")
	crossIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge) // one strong approval is enough
}

// Usage pauses never reach a verdict, so they never spend one (#725): two rejections, then a
// pause, then one more rejection uses up the lane's 3 rounds across the two runs.
func TestCross_APauseDoesNotSpendAVerdictRound(t *testing.T) {
	h := newCrossHarness(t, crossHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day,
		Overrides: pyjson.New("max_review_cycles", 3)})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})

	limited := func(*runner.RunRequest) *runner.RunResult {
		r := runner.NewRunResult(false, "")
		r.ExitCode, r.Error, r.ResetAt = 1, "You've hit your limit", "2026-09-30T05:00:00Z"
		return r
	}

	planned, result := h.night(runner.NewFakeRunner(map[string][]runner.Handler{
		"build":  {crossBuilder(map[string]string{"src/game.txt": "v2\n"})},
		"fix":    {crossBuilder(map[string]string{"src/game.txt": "v3\n"})},
		"review": {crossReviewer(crossChanges("No.")), crossReviewer(crossChanges("No.")), limited}}))
	crossEq(t, []any{planned.Get("provider"), result.Get("status")}, []any{"muse", "interrupted"})
	record := crossObj(t, h.ctx.Store.Load(), "items", "12")
	crossEq(t, []any{crossKey(t, record, "verdict_rounds"), crossKey(t, record, "round_tier")}, []any{2, "medium"})
	// One round left in the lane: the pause parked Muse, so agy (same tier) picks it up
	// and one more rejection ends the lane phase.
	r := crossRunner(map[string]runner.Handler{"build": crossBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"fix":    crossBuilder(map[string]string{"src/game.txt": "v3\n"}),
		"review": crossReviewer(crossChanges("No."))})
	planned, result = h.night(r)
	crossEq(t, []any{planned.Get("provider"), result.Get("status")}, []any{"agy", "built"})
	crossEq(t, crossCount(crossRoles(r), "review"), 1)
	crossIn(t, "Strong final audit", crossStr(t, result, "reason"))
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	crossEq(t, h.gh.LabelNames(pr), crossSet(config.LabelPR, config.LabelCross, "difficulty:medium"))
	record = crossObj(t, h.ctx.Store.Load(), "items", "12")
	crossEq(t, []any{crossKey(t, record, "verdict_rounds"), crossKey(t, record, "round_tier")}, []any{3, "medium"})
}

// A Strong audit's rejection is answered at its own tier (#725): Opus fixes the medium item in
// its own run, its own reviewer approves, and that ships it.
func TestCross_AStrongReviseAfterRejectionShips(t *testing.T) {
	h := newCrossHarness(t, crossHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	h.committedHours() // claude-3 works all day: the Strong final can audit
	pr, _ := crossBuildOnMedium(t, h, "", nil)
	h.night(crossRunner(map[string]runner.Handler{"review": crossReviewer(crossChanges("It skips the replay check."))}))
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	crossEq(t, crossKey(t, record, "audit_tier"), "strong")
	r := crossRunner(map[string]runner.Handler{"revise": crossBuilder(map[string]string{"src/game.txt": "rules v2 + replay\n"}),
		"review": crossReviewer(crossApprove)})
	planned, result := h.night(r)
	crossEq(t, []any{planned.Get("action"), planned.Get("provider"), result.Get("status")},
		[]any{"revise", "claude-3", "approved"})
	crossEq(t, crossModels(r), []string{"opus", "opus"})
	crossIn(t, "It skips the replay check.", r.Calls[0].Prompt)
	crossIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
}

func TestCross_FindingsSendItBackAndItStopsAfterThree(t *testing.T) {
	h := newCrossHarness(t, crossHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	h.committedHours() // claude-3 works all day: the Strong final can audit
	pr, _ := crossBuildOnMedium(t, h, "", nil)
	h.night(crossRunner(map[string]runner.Handler{"review": crossReviewer(crossChanges("It skips the replay check."))}))
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	crossEq(t, h.gh.LabelNames(pr), crossSet(config.LabelPR, config.LabelRevise, "difficulty:medium"))
	crossEq(t, []any{crossKey(t, record, "source"), crossKey(t, record, "cross_rounds")}, []any{"cross-review", 1})
	// The revision answers the Strong audit, so it stays with a Strong seat (#725).
	// Two failed revisions are strikes two and three (each failed for the item's own sake),
	// so the third steps it up instead of blocking it for a person.
	seen := map[string]string{}

	broken := func(request *runner.RunRequest) *runner.RunResult {
		seen["prompt"] = request.Prompt
		r := runner.NewRunResult(false, "I cannot fix this; the approach is wrong.")
		r.ExitCode, r.Error, r.DurationS = 1, "the model gave up", 120.0
		return r
	}

	for range 2 {
		planned, result := h.night(crossRunner(map[string]runner.Handler{"revise": broken}))
		crossEq(t, []any{planned.Get("action"), planned.Get("provider"), result.Get("status")},
			[]any{"revise", "claude-3", "failed"})
	}
	crossIn(t, "It skips the replay check.", seen["prompt"])
	// Three strikes (#317 parts 4 and 8): instead of blocking it for a
	// person, the bot raises the issue a step and builds it again from main.
	crossEq(t, h.gh.Threads.Item(pr).Get("state"), "closed")
	crossIn(t, "difficulty:hard", h.gh.LabelNames(12))
	crossIn(t, config.LabelBuild, h.gh.LabelNames(12))
	said := crossLast(t, h.gh.BotComments(12))
	crossIn(t, "This failed 3 times at `difficulty:medium`", said)
	crossIn(t, fmt.Sprintf("#%d is closed and #12 is queued to build again from `main`", pr), said)
	crossIn(t, "Closed to build #12 again from `main`", crossLast(t, h.gh.BotComments(pr)))
	record = crossObj(t, h.ctx.Store.Load(), "items", "12")
	crossEq(t, []any{crossKey(t, record, "previous_pr"), crossKey(t, record, "strikes")}, []any{pr, 0})
	crossTrue(t, strings.HasPrefix(crossStr(t, record, "previous_branch"), "bot/old/issue-12-"))
}

func TestCross_AReviewOfAHeadThatMovedDoesNotCount(t *testing.T) {
	h := newCrossHarness(t, crossHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	h.committedHours() // claude-3 works all day: the Strong final can audit
	pr, _ := crossBuildOnMedium(t, h, "", nil)
	moved := func(request *runner.RunRequest) *runner.RunResult {
		// Someone pushes to the branch while the reviewer reads it.
		fakes.PushBranch(h.origin, h.root, "bot/issue-12", map[string]string{"src/game.txt": "someone else\n"}, "bot/issue-12")
		return crossReviewer(crossApprove)(request)
	}
	h.night(crossRunner(map[string]runner.Handler{"review": moved}))
	crossEq(t, h.gh.LabelNames(pr), crossSet(config.LabelPR, config.LabelCross, "difficulty:medium"))
	crossEq(t, h.gh.AutoMerge, map[string]string{})
	crossIn(t, "does not count", crossLast(t, h.gh.BotComments(pr)))
}

func TestCross_AHardItemIsPlannedAndBuiltByOpus(t *testing.T) {
	h := newCrossHarness(t, crossHarnessOpts{Env: crossAll, Machine: testsupport.Machine})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:hard"}})
	r := crossRunner(map[string]runner.Handler{"build": crossBuilder(map[string]string{"src/game.txt": "v2\n"}),
		"review": crossReviewer(crossApprove)})
	planned, result := h.night(r)
	crossEq(t, []any{planned.Get("provider"), planned.Get("difficulty"), result.Get("status")},
		[]any{"claude-3", "hard", "approved"})
	models := map[string]bool{}
	for _, c := range r.Calls {
		models[c.Model] = true
	}
	crossEq(t, models, crossSet("opus"))
	pr := crossPullOf(t, h.gh, "bot/issue-12")
	crossIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	crossEq(t, h.gh.LabelNames(pr), crossSet(config.LabelPR, "difficulty:hard"))
	crossEq(t, crossKey(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr), "difficulty"), "hard")
}

// ---------------------------------------------------------------------------- HandoffTests

func TestCross_TheNextAgentGetsTheNotesAndTheBranch(t *testing.T) {
	h := newCrossHarness(t, crossHarnessOpts{Env: crossAll, Machine: testsupport.Machine})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})

	limited := func(*runner.RunRequest) *runner.RunResult {
		r := runner.NewRunResult(false, "")
		r.ExitCode, r.Error, r.ResetAt = 1, "hit your limit", "2026-09-30T04:00:00Z"
		return r
	}

	first := crossRunner(map[string]runner.Handler{"build": crossBuilder(map[string]string{"src/game.txt": "half\n",
		work.NotesFile: "plan: rules v2\nnext: the Coin\n"}),
		"review": limited})
	planned, result := h.night(first)
	crossEq(t, []any{planned.Get("provider"), result.Get("status")}, []any{"muse", "interrupted"})
	record := crossObj(t, h.ctx.Store.Load(), "items", "12")
	crossIn(t, "next: the Coin", crossStr(t, record, "handoff", "notes"))
	crossEq(t, crossKey(t, record, "handoff", "provider"), "muse")
	crossTrue(t, pyjson.Truthy(crossKey(t, record, "planned_at"))) // planned in that run: not planned again
	// The notes never reach the branch.
	crossNotIn(t, work.NotesFile, fakes.Git(h.origin, "ls-tree", "-r", "--name-only", crossStr(t, result, "head")))
	// Muse is parked until its reset, so the lane's tier order picks agy (#725: the
	// owner keeps it only when it can build it). The next run is agy, told what happened.
	second := crossRunner(map[string]runner.Handler{"build": crossBuilder(map[string]string{"src/game.txt": "whole\n"}),
		"review": crossReviewer(crossApprove)})
	planned, result = h.night(second)
	crossEq(t, []any{planned.Get("provider"), result.Get("status")}, []any{"agy", "approved"})
	crossEq(t, crossRoles(second), []string{"build", "review"})
	prompt := second.Calls[0].Prompt
	crossIn(t, "## Picking up from another agent", prompt)
	crossIn(t, "next: the Coin", prompt)
	crossNotIn(t, "handoff", crossObj(t, h.ctx.Store.Load(), "items", "12"))
}

// ---------------------------------------------------------------------------- VaultDeliveryTests

func TestCross_DeliverKeepsASealedLoginAndRefusesAnythingElse(t *testing.T) {
	h := newCrossHarness(t, crossHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	h.gh.AddIssue(12, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned := plan.Make(h.ctx, false, nil, "auto", nil)
	crossEq(t, planned.Get("provider"), "muse")
	out := filepath.Join(h.root, "out-vault")
	crossMkdir(t, out)
	crossWrite(t, filepath.Join(out, "result.json"), pyjson.Dumps(pyjson.New("status", "failed", "reason", "boom")))
	sealed, err := vault.Seal(pyjson.New("provider", "muse", "files", pyjson.New("auth.json", "{}")), "the-secret")
	if err != nil {
		t.Fatal(err)
	}
	crossWrite(t, filepath.Join(out, "vault.enc"), sealed)
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	text, _ := h.gh.GetFile("vault/muse.enc", "bot-state")
	crossEq(t, strings.TrimSpace(text), sealed)
	// The next plan hands it to the model job, which alone can open it.
	planned = plan.Make(h.ctx, false, nil, "auto", nil)
	crossEq(t, strings.TrimSpace(crossStr(t, planned, "vault")), sealed)
	crossWrite(t, filepath.Join(out, "vault.enc"), "not a vault")
	log := crossKey(t, deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run(), "log")
	crossIn(t, "ignored a vault that is not sealed", log)
}

// ---------------------------------------------------------------------------- ConflictCarryTests

// crossResolve is resolve(extra): a builder that resolves the conflict `main` left in
// src/game.txt, and writes `extra`.
func crossResolve(extra map[string]string) runner.Handler {
	return func(request *runner.RunRequest) *runner.RunResult {
		target := filepath.Join(request.Cwd, "src", "game.txt")
		text := crossMustRead(target)
		if !strings.Contains(text, "<<<<<<<") {
			panic(&fakes.PyError{Type: "AssertionError", Msg: text})
		}
		crossPut(target, "rules v2, with main's\n")
		for name, content := range extra {
			crossPut(filepath.Join(request.Cwd, name), content)
		}
		return runner.NewRunResult(true, crossDone)
	}
}

// crossConflicted is ConflictCarryTests.conflicted: `main` moves on and the branch no longer
// merges.
func crossConflicted(h *crossHarness, pr int, main map[string]string) {
	if len(main) == 0 {
		main = map[string]string{"src/game.txt": "rules from main\n"}
	}
	fakes.PushBranch(h.origin, h.root, "main", main)
	h.gh.Threads.Item(pr).Set("mergeable_state", "dirty")
}

// crossClearedPR is ConflictCarryTests.cleared_pr: Muse builds and approves it by day; at night
// a Strong seat reviews it: the rule is met (#695), and then `main` conflicts with it.
func crossClearedPR(t *testing.T, h *crossHarness, edits, main map[string]string) (int, string) {
	t.Helper()
	pr, built := crossBuildOnMedium(t, h, "", edits)
	crossNextRun(h, "778")
	testsupport.ClockOf(h.ctx).At = testsupport.Night
	planned, _ := h.night(crossRunner(map[string]runner.Handler{"review": crossReviewer(crossApprove)}))
	crossEq(t, []any{planned.Get("provider"), crossKey(t, planned, "seats", "review", "tier")},
		[]any{"claude-3", "strong"})
	crossIn(t, fmt.Sprintf("PR_%d", pr), h.gh.AutoMerge)
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	crossEq(t, crossKey(t, record, "cleared", "sha"), built.Get("head"))
	crossEq(t, crossKey(t, record, "cleared", "by"), "`muse` (medium), `claude` (strong)")
	crossConflicted(h, pr, main)
	crossNextRun(h, "779")
	return pr, crossStr(t, built, "head")
}

// `main` conflicts with the cleared change only in `spec/INDEX.md`, which the committed
// resolvers take from `main` (`resolve.py`): the revision spends no builder session, its own
// reviewer approves the merge, and the clearance carries as for a builder's resolution.
func TestCross_AConflictTheHarnessResolvesCarriesWithNoBuilder(t *testing.T) {
	h := newCrossHarness(t, crossHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, clearedSHA := crossClearedPR(t, h,
		map[string]string{"src/game.txt": "rules v2\n", "spec/INDEX.md": "| R12 |\n"},
		map[string]string{"spec/INDEX.md": "| R13 |\n"})
	r := crossRunner(map[string]runner.Handler{"review": crossReviewer(crossApprove)})
	planned, result := h.night(r)
	crossEq(t, []any{planned.Get("action"), planned.Get("source")}, []any{"revise", "conflict"})
	crossEq(t, result.Get("status"), "approved", result.Get("reason"))
	crossEq(t, crossRoles(r), []string{"review"})
	crossIn(t, "the harness resolved `spec/INDEX.md` itself (took `main`'s version)", r.Calls[0].Prompt)
	crossEq(t, crossKey(t, result, "cycles", 0, "builder", "role"), "merge")
	// Delivered: auto-merge back on at the merged head, the clearance carried over it.
	crossEq(t, h.gh.AutoMergeHeads[fmt.Sprintf("PR_%d", pr)], result.Get("head"))
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	carried := crossObj(t, record, "votes", "carried")
	crossEq(t, []any{crossKey(t, carried, "from"), crossKey(t, carried, "conflicts")},
		[]any{clearedSHA, []any{"spec/INDEX.md"}})
	crossIn(t, "This revision only merged `main` and resolved the conflicts in `spec/INDEX.md`, and its own "+
		"reviewer approved that, so the clearance carries over", crossLast(t, h.gh.BotComments(pr)))
	check := filepath.Join(h.root, "check-merge")
	fakes.Git(h.root, "clone", "-q", h.origin, check)
	crossEq(t, fakes.Git(check, "show", "origin/bot/issue-12:spec/INDEX.md"), "| R13 |")
	crossEq(t, fakes.Git(check, "show", "origin/bot/issue-12:src/game.txt"), "rules v2")
}

func TestCross_AConflictOnAClearedChangeShipsOnItsOwnReview(t *testing.T) {
	h := newCrossHarness(t, crossHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, clearedSHA := crossClearedPR(t, h, nil, nil)
	r := crossRunner(map[string]runner.Handler{"revise": crossResolve(nil), "review": crossReviewer(crossApprove)})
	planned, result := h.night(r)
	crossEq(t, []any{planned.Get("action"), planned.Get("source"), planned.Get("provider")},
		[]any{"revise", "conflict", "muse"})
	crossEq(t, []any{result.Get("status"), crossKey(t, planned, "cleared", "sha")}, []any{"approved", clearedSHA})
	comments := h.gh.BotComments(pr)
	crossIn(t, "it merges without another review run", comments[len(comments)-2])
	crossEq(t, crossRoles(r), []string{"revise", "review"})
	crossIn(t, "Resolve the conflicts and change nothing else", r.Calls[0].Prompt)
	crossIn(t, "yours is the only review of the resolution", r.Calls[1].Prompt)
	crossIn(t, "resolved the conflicts in `src/game.txt`", r.Calls[1].Prompt)
	// No review run: auto-merge is back on, pinned to the resolved head.
	crossEq(t, h.gh.LabelNames(pr), crossSet(config.LabelPR, "difficulty:medium"))
	crossEq(t, h.gh.AutoMergeHeads[fmt.Sprintf("PR_%d", pr)], result.Get("head"))
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	families := []any{}
	for _, r := range pyjson.Objs(crossKey(t, record, "votes", "reviews")) {
		families = append(families, crossKey(t, r, "family"))
	}
	crossEq(t, families, []any{"muse"})
	carried := crossObj(t, record, "votes", "carried")
	crossEq(t, []any{crossKey(t, carried, "from"), crossKey(t, carried, "conflicts")},
		[]any{clearedSHA, []any{"src/game.txt"}})
	// The resolved head is cleared in turn, so the next conflict carries again.
	crossEq(t, []any{crossKey(t, record, "cleared", "sha"), crossKey(t, record, "cleared", "carried_from")},
		[]any{result.Get("head"), clearedSHA})
	crossEq(t, []any{crossKey(t, record, "cleared", "by"), crossKey(t, record, "cleared", "resolved_by")},
		[]any{"`muse` (medium), `claude` (strong)", "`muse` (medium)"})
	comment := crossLast(t, h.gh.BotComments(pr))
	crossIn(t, "The reviews had cleared this change at `"+clearedSHA[:12]+"` (`muse` (medium), `claude` "+
		"(strong)). This revision only merged `main` and resolved the conflicts in `src/game.txt`, and its own "+
		"reviewer approved that, so the clearance carries over and no review run is needed. Auto-merge is on",
		comment)
}

func TestCross_AResolutionThatChangesMoreGoesBackToReview(t *testing.T) {
	h := newCrossHarness(t, crossHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, clearedSHA := crossClearedPR(t, h, nil, nil)
	_, result := h.night(crossRunner(map[string]runner.Handler{
		"revise": crossResolve(map[string]string{"src/extra.txt": "more\n"}),
		"review": crossReviewer(crossApprove)}))
	crossEq(t, result.Get("status"), "approved")
	crossEq(t, h.gh.LabelNames(pr), crossSet(config.LabelPR, config.LabelCross, "difficulty:medium"))
	crossEq(t, h.gh.AutoMerge, map[string]string{})
	record := crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr))
	crossNotIn(t, "carried", crossObj(t, record, "votes"))
	crossEq(t, crossKey(t, record, "cleared", "sha"), clearedSHA)
	crossIn(t, "but that does not carry over: it changed more than the conflicts (`src/extra.txt`). `muse` "+
		"(medium) approved it, so it waits for one strong model's final audit (Opus)",
		crossLast(t, h.gh.BotComments(pr)))
}

func TestCross_ABranchThatMovedAfterTheReviewsDoesNotCarry(t *testing.T) {
	h := newCrossHarness(t, crossHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, _ := crossClearedPR(t, h, nil, nil)
	// Someone pushed to the branch after the reviews cleared it; nobody reviewed that.
	fakes.PushBranch(h.origin, h.root, "bot/issue-12", map[string]string{"src/other.txt": "unreviewed\n"}, "bot/issue-12")
	r := crossRunner(map[string]runner.Handler{"revise": crossResolve(nil), "review": crossReviewer(crossApprove)})
	h.night(r)
	crossNotIn(t, "change nothing else", r.Calls[0].Prompt)
	crossNotIn(t, "yours is the only review", r.Calls[1].Prompt)
	crossEq(t, h.gh.LabelNames(pr), crossSet(config.LabelPR, config.LabelCross, "difficulty:medium"))
	crossIn(t, "but that does not carry over: the branch had moved on from the commit they cleared",
		crossLast(t, h.gh.BotComments(pr)))
}

// One medium approval, short of the Strong one: the change was never cleared, so its resolution
// waits for the review run like any revision.
func TestCross_AChangeShortOfTheRuleHasNothingToCarry(t *testing.T) {
	h := newCrossHarness(t, crossHarnessOpts{Env: crossAll, Machine: testsupport.Machine, At: testsupport.Day})
	pr, _ := crossBuildOnMedium(t, h, "", nil)
	crossNotIn(t, "cleared", crossObj(t, h.ctx.Store.Load(), "items", strconv.Itoa(pr)))
	crossConflicted(h, pr, nil)
	crossNextRun(h, "778")
	r := crossRunner(map[string]runner.Handler{"revise": crossResolve(nil), "review": crossReviewer(crossApprove)})
	planned, result := h.night(r)
	crossEq(t, []any{planned.Get("source"), result.Get("status")}, []any{"conflict", "approved"})
	crossNotIn(t, "cleared", planned)
	crossNotIn(t, "yours is the only review", r.Calls[1].Prompt)
	crossEq(t, h.gh.LabelNames(pr), crossSet(config.LabelPR, config.LabelCross, "difficulty:medium"))
	crossEq(t, h.gh.AutoMerge, map[string]string{})
	crossNotIn(t, "cleared this change", crossLast(t, h.gh.BotComments(pr)))
}

// The weak one takes an easy revision first, but never a conflict (#317 part 2): nothing reviews
// in its run, and it committed conflict markers in SPEC §11 again and again (#203, #214, #287).
// agy takes the conflict, cleared or not, and its own reviewer can carry a clearance.
func TestCross_AConflictRevisionGoesToABuilderThatReviewsInItsOwnRun(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day, Cfg: testsupport.MakeConfig(testsupport.Config{
		Env: map[string]string{"HARNESS_SECRETS_SET": ""}, Machine: []string{"lite", "agy"}})})
	gh.AddPull(9, "bot/issue-2", fakes.Pull{Labels: []string{config.LabelPR, config.LabelRevise, "difficulty:easy"}})
	update := func(fields *pyjson.Object) {
		ctx.Store.Update(func(s *pyjson.Object) any { state.Item(s, 9).Update(fields); return nil }, "state")
	}
	update(pyjson.New("kind", "revise", "source", "request"))
	crossEq(t, plan.Peek_(ctx, false, nil, "auto").Provider, "lite")
	update(pyjson.New("kind", "revise", "source", "conflict"))
	crossEq(t, plan.Peek_(ctx, false, nil, "auto").Provider, "agy")
	update(pyjson.New("cleared", pyjson.New("sha", "abc", "by", "`claude` (strong)")))
	planned := plan.Make(ctx, false, nil, "auto", nil)
	crossEq(t, []any{planned.Get("provider"), crossKey(t, planned, "cleared", "sha")}, []any{"agy", "abc"})
	crossIn(t, "it merges without another review run", crossLast(t, gh.BotComments(9)))
}
