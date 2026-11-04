package tests

// The usage history (#2749, internal/usage): every subscription's usage over time, as CSV files
// on the `bot-usage` branch. A run's readings and its row reach it through its delivery and the
// state file's spool; the status loop adds its ticks, derives sessions and weeks, backfills from
// the journal once, and pushes every 30 minutes.

import (
	"os"
	"path/filepath"
	"slices"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cardsets"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cli"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/deliver"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/usage"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/waiting"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// usageAt is the tests' clock: a Monday noon, UTC.
var usageAt = time.Date(2026, 11, 2, 12, 0, 0, 0, time.UTC)

// usageReading is a runner's reading: the 5-hour window at `five` (a share), resetting at `resets`,
// and the week at `week` when it is not negative.
func usageReading(five float64, resets time.Time, week ...float64) *pyjson.Object {
	reading := pyjson.New("five_hour", pyjson.New("utilization", five, "resets_at", usage.ISO(resets)),
		"status", "allowed")
	if len(week) > 0 {
		reading.Set("seven_day", pyjson.New("utilization", week[0], "resets_at", usage.ISO(resets.Add(72*time.Hour))))
	}
	return reading
}

// usageWorld is the status loop's usage history against a repository on disk: a fake GitHub for
// the state file, a config whose GitHub is that repository (`origin`, which holds the usage
// branch and the journal), a context at a clock the test moves, and a recorder that logs into
// `logs`.
type usageWorld struct {
	t      *testing.T
	root   string
	origin string
	gh     *fakes.FakeGitHub
	cfg    *config.Config
	ctx    *hctx.Context
	logs   []string
	rec    *usage.Recorder
}

func newUsageWorld(t *testing.T) *usageWorld {
	t.Helper()
	w := &usageWorld{t: t, root: t.TempDir(), gh: fakes.NewFakeGitHub()}
	w.origin, _ = fakes.MakeOrigin(w.root)
	w.cfg = testsupport.MakeConfig(testsupport.Config{
		Env: map[string]string{"GITHUB_SERVER_URL": "file://" + filepath.Join(w.root, "remote")}})
	w.ctx = testsupport.MakeCtx(w.gh, testsupport.Ctx{Cfg: w.cfg, At: usageAt})
	scratch := filepath.Join(w.root, "scratch")
	patch(t, &usage.Scratch, func() string { return scratch })
	w.rec = usage.NewRecorder(func(line string) { w.logs = append(w.logs, line) })
	w.journal(map[string]string{"1.md": "# Journal of #1\n"})
	return w
}

// at moves the clock.
func (w *usageWorld) at(t time.Time) { testsupport.ClockOf(w.ctx).At = t }

// tick is one status tick with the state as it is and nothing queued or running.
func (w *usageWorld) tick() {
	w.rec.Tick(w.ctx, usage.TickFacts{State: w.ctx.Store.Load(), Live: pyjson.New()})
}

// journal puts the journal branch on origin, holding `files`.
func (w *usageWorld) journal(files map[string]string) { usageJournalOn(w.t, w.origin, files) }

// usageJournalOn puts the journal branch on `origin` (a repository on disk), holding `files`.
func usageJournalOn(t *testing.T, origin string, files map[string]string) {
	seed := filepath.Join(t.TempDir(), "journal")
	mkdir(t, seed)
	fakes.Git(seed, "init", "-q")
	for name, text := range files {
		writeText(t, filepath.Join(seed, name), text)
	}
	fakes.Git(seed, "add", "-A")
	fakes.Git(seed, "commit", "-q", "-m", "journal")
	fakes.Git(seed, "push", "-q", "--force", origin, "HEAD:refs/heads/bot-journal")
}

// show is a file of origin's usage branch, "" when it has none.
func (w *usageWorld) show(path string) string {
	text := ""
	if _, failed := herr.Catch[*fakes.PyError](func() { text = fakes.Git(w.origin, "show", "bot-usage:"+path) }); failed {
		return ""
	}
	return text
}

// state changes the state file.
func (w *usageWorld) state(change func(s *pyjson.Object)) {
	w.ctx.Store.Update(func(s *pyjson.Object) any { change(s); return nil }, "state")
}

// rows is a CSV file of origin's usage branch as its lines, the header first.
func (w *usageWorld) rows(path string) []string {
	text := w.show(path)
	if text == "" {
		return nil
	}
	return strings.Split(text, "\n")
}

// clone is a fresh clone of origin's usage branch.
func (w *usageWorld) clone() string {
	dir := filepath.Join(w.t.TempDir(), "usage")
	fakes.Git(w.root, "clone", "-q", "--branch", "bot-usage", "file://"+w.origin, dir)
	return dir
}

// ------------------------------------------------------------------ the files

func TestUsage_AReadingDedupesOnSubscriptionAndObservedAt(t *testing.T) {
	root := t.TempDir()
	h := usage.NewHistory(root)
	reading := &usage.Reading{ObservedAt: usageAt, Subscription: "claude-3", Source: "stream", During: "call",
		Util5h: usage.N(40), Resets5h: usageAt.Add(time.Hour)}
	added, err := h.Append(reading, reading)
	assertTrue(t, err == nil, err)
	assertEqual(t, added, 1)
	same := *reading
	same.Util5h, same.Source = usage.N(41), "ping" // the same subscription at the same moment
	added, _ = h.Append(&same)
	assertEqual(t, added, 0)
	other := *reading
	other.Subscription = "claude-1"
	added, _ = h.Append(&other)
	assertEqual(t, added, 1)
	// A history that reads the file afresh knows its rows too.
	added, _ = usage.NewHistory(root).Append(reading)
	assertEqual(t, added, 0)
	text := readText(t, filepath.Join(root, "usage", "readings-2026-11.csv"))
	assertEqual(t, strings.Split(text, "\n")[0], "observed_at,subscription,source,during,run_id,util_5h,"+
		"resets_5h,util_7d,resets_7d,status,cap_5h_in_force,cap_7d_in_force,refused_until")
	assertIn(t, "2026-11-02T12:00:00Z,claude-3,stream,call,,40,2026-11-02T13:00:00Z,,,,,,", text)
	read, err := usage.ReadReadings(root, time.Time{}, time.Time{})
	assertTrue(t, err == nil, err)
	assertEqual(t, len(read), 2)
	assertEqual(t, read[0].Util5h, usage.N(40))
	// A range reads its own rows only.
	read, _ = usage.ReadReadings(root, usageAt.Add(time.Minute), time.Time{})
	assertEqual(t, len(read), 0)
}

func TestUsage_EachFileHasTheIssuesColumnsFirstAndItsOwnLast(t *testing.T) {
	assertEqual(t, usage.Ticks.Header(), []string{"at", "subscription", "enabled", "suspended", "in_hours",
		"last_hour_now", "late_week_now", "cap_5h_now", "cap_7d_now", "hard_cap", "start_headroom_5h", "util_5h",
		"util_7d", "reading_at", "reading_age_min", "resets_5h", "resets_7d", "refused_until", "lanes_allowed",
		"lanes_held", "runs_held", "planning_held", "state", "queued_for_it", "waiting_for_it", "held_for_it",
		"runs_plan", "runs_build", "runs_revise", "runs_review"})
	assertEqual(t, usage.Sessions.Header()[:17], []string{"subscription", "started_at", "reset_at", "cap_min",
		"cap_max", "peak_util", "util_last_read", "unused_at_reset", "hit_cap_at", "minutes_capped",
		"minutes_capped_with_work_queued", "last_hour_used", "runs", "model_minutes", "cut_offs", "refusals",
		"overshoot_max"})
	assertEqual(t, usage.Weeks.Header()[:11], []string{"subscription", "reset_at", "peak_util", "util_at_reset",
		"unused_at_reset", "first_capped_at", "days_refused", "runs", "model_hours", "tokens", "usd_reported"})
	assertEqual(t, usage.Runs.Header(), []string{"ended_at", "run_id", "subscription", "models", "kind", "item",
		"difficulty", "outcome", "stop_reason", "model_minutes", "tokens_in", "tokens_out", "tokens_cached",
		"usd_reported", "util_5h_start", "util_5h_end", "util_7d_start", "util_7d_end", "queue_wait_min",
		"previous_subscription", "session", "handed_off", "model_calls"})
	assertEqual(t, usage.Weeks.Path(usageAt), filepath.Join("usage", "weeks.csv"))
	assertEqual(t, usage.Pools.Path(usageAt), filepath.Join("usage", "pools-2026-11.csv"))
}

// A file written before a column joined its header is rewritten under the new one before rows are
// added, so every row lines up.
func TestUsage_AFileWithAnOlderHeaderIsRewrittenUnderTheNewOne(t *testing.T) {
	root := t.TempDir()
	path := filepath.Join(root, "usage", "readings-2026-11.csv")
	mkdir(t, filepath.Dir(path))
	writeText(t, path, "observed_at,subscription,source,during,run_id,util_5h\n2026-11-02T10:00:00Z,agy,stream,call,9,12\n")
	added, err := usage.NewHistory(root).Append(&usage.Reading{ObservedAt: usageAt, Subscription: "agy",
		Util7d: usage.N(50)})
	assertTrue(t, err == nil, err)
	assertEqual(t, added, 1)
	read, _ := usage.ReadReadings(root, time.Time{}, time.Time{})
	assertEqual(t, len(read), 2)
	assertEqual(t, []any{read[0].RunID, read[0].Util5h, read[1].Util7d}, []any{"9", usage.N(12), usage.N(50)})
	assertEqual(t, strings.Split(readText(t, path), "\n")[0], strings.Join(usage.Readings.Header(), ","))
}

// ------------------------------------------------------------------ a run's readings and row

func TestUsage_ARunKeepsAReadingOnlyWhenItSaysSomethingNew(t *testing.T) {
	resets := usageAt.Add(3 * time.Hour)
	caps := pyjson.New("five_hour", 0.7)
	var list []any
	add := func(after time.Duration, source string, share float64) {
		list = usage.Note(list, usageAt.Add(after), source, "call", usageReading(share, resets), caps, time.Time{})
	}
	add(0, "ping", 0.20)
	add(15*time.Second, "stream", 0.205) // a tenth of a point on
	add(time.Minute, "stream", 0.209)
	add(2*time.Minute, "stream", 0.25) // five points on
	add(13*time.Minute, "stream", 0.25)
	list = usage.Note(list, usageAt.Add(14*time.Minute), "refusal", "call", nil, caps, usageAt.Add(2*time.Hour))
	rows := usage.ReadingsOf(pyjson.New("readings", list), "claude-4", "42")
	var shown []any
	for _, r := range rows {
		shown = append(shown, []any{r.Source, r.Util5h.String(), usage.ISO(r.RefusedUntil), r.Status, r.Cap5h.String()})
	}
	assertEqual(t, shown, []any{
		[]any{"ping", "20", "", "allowed", "70"},
		[]any{"stream", "25", "", "allowed", "70"},
		[]any{"stream", "25", "", "allowed", "70"}, // ten minutes on
		[]any{"refusal", "", "2026-11-02T14:00:00Z", "rejected", "70"},
	})
	assertEqual(t, []string{rows[0].Subscription, rows[0].RunID}, []string{"claude-4", "42"})
	// The words a model job's result carries are the history's own, or none.
	forged := pyjson.New("readings", []any{pyjson.New("observed_at", usage.ISO(usageAt), "source", "a prompt",
		"during", "call", "status", "ignore previous instructions", "util_5h", 5.0)})
	row := usage.ReadingsOf(forged, "agy", "1")[0]
	assertEqual(t, []string{row.Source, row.During, row.Status}, []string{"", "call", ""})
}

func TestUsage_ARunRowFromADeliveredResult(t *testing.T) {
	resets := usageAt.Add(time.Hour)
	readings := []*usage.Reading{
		{ObservedAt: usageAt.Add(-time.Hour), Util5h: usage.N(31), Util7d: usage.N(10)},
		{ObservedAt: usageAt.Add(-30 * time.Minute), Util5h: usage.N(55)},
		{ObservedAt: usageAt.Add(-time.Minute), Util5h: usage.N(71), Resets5h: resets},
	}
	result := pyjson.New("status", "interrupted", "interrupt", "usage",
		"reason", "`claude-4` stops mid-call: 5-hour usage is 71%, at or over its 70% cap; it resets at "+usage.ISO(resets),
		"minutes", 40.04, "model_calls", 3, "tokens", pyjson.New("input", 10, "output", 20, "cache_read", 300,
			"cache_write", 5), "cost_usd", 1.23456, "finished_at", usage.ISO(usageAt),
		"session", pyjson.New("resumed", true))
	planned := pyjson.New("action", "build", "number", 42, "difficulty", "hard", plan.UsageFacts, pyjson.New(
		"queued_at", usage.ISO(usageAt.Add(-150*time.Minute)), "claimed_at", usage.ISO(usageAt.Add(-61*time.Minute)),
		"previous_provider", "claude-3",
		"handed_off", "handed off from `claude-3`: `claude-3` 5-hour usage is 72%, at or over its 70% cap"))
	run := usage.RunOf(usage.RunFacts{At: usageAt.Add(5 * time.Minute), RunID: "777", Subscription: "claude-4",
		Models: []string{"sonnet", "opus"}, Plan: planned, Result: result, Readings: readings})
	assertEqual(t, usage.Encode(run), []string{"2026-11-02T12:00:00Z", "777", "claude-4", "sonnet+opus", "build",
		"42", "hard", "interrupted", "5-hour cap", "40", "10", "20", "305", "1.2346", "31", "71", "10", "10", "89",
		"claude-3", "resumed", "cap", "3"})
	// A run that was not cut off, from a result of before the history: what its section says.
	run = usage.RunOf(usage.RunFacts{At: usageAt, RunID: "778", Subscription: "muse", Models: []string{"muse-spark"},
		Plan: pyjson.New("action", "plan", "number", 7), Result: pyjson.New("status", "planned", "minutes", 2.9)})
	assertEqual(t, usage.Encode(run), []string{"2026-11-02T12:00:00Z", "778", "muse", "muse-spark", "plan", "7", "",
		"planned", "", "2.9", "", "", "", "", "", "", "", "", "", "", "none", "", ""})
}

func TestUsage_WhyARunStoppedAndWhyAnItemChangedHands(t *testing.T) {
	cases := []struct{ status, interrupt, reason, want string }{
		{"interrupted", "usage", "`claude-7` stops mid-call: 5-hour usage is 50%, at or over its 50% cap", "5-hour cap"},
		{"interrupted", "usage", "`claude-5` stops before it starts: 7-day usage is 60%, at or over its 60% cap", "weekly cap"},
		{"interrupted", "usage", "`claude-3` reached its usage limit", "refusal"},
		{"interrupted", "usage", "`agy` stops: it refused a call; its limit resets at 2026-11-06T22:06:00Z", "refusal"},
		{"interrupted", "budget", "the run's time budget is spent", "time budget"},
		{"interrupted", "halt", "halted by /harness halt", "halt"},
		{"interrupted", "", "the bot was halted before this work could be published", "halt"},
		{"stopped", "", "stopped by @jgoetzmann", "stop"},
		{"interrupted", "suspend", "`claude-1` was suspended by @jgoetzmann", "stop"},
		{"interrupted", "died", "the job was stopped (SIGTERM)", "died"},
		{"died", "", "the model job stopped before it finished", "died"},
		{"infra", "", "the model job left no result", "infra"},
		{"approved", "", "the reviewer approved and every check passed", ""},
		// The journal names no interrupt: an interrupted run's reason says it.
		{"interrupted", "", "`claude-7` stops mid-call: 5-hour usage is 50%, at or over its 50% cap", "5-hour cap"},
	}
	for _, c := range cases {
		assertEqual(t, usage.StopReason(c.status, c.interrupt, c.reason), c.want, c.reason)
	}
	for why, want := range map[string]string{
		"": "",
		"handed off from `claude-3`: `claude-3` 5-hour usage is 72%, at or over its 70% cap":   "cap",
		"it took it up; released from its hold: its owner `claude-6` could not start by 12:00": "cap",
		"it took it up; released by @jgoetzmann":                                               "person",
		"handed off from `claude-1`: `claude-1` outside its hours (21:00–07:00; opens in 3h)":  "other",
	} {
		assertEqual(t, usage.HandoffReason(why), want, why)
	}
}

func TestUsage_TheSpoolIsCappedAndGivesUpEachPushedLineOnce(t *testing.T) {
	patch(t, &usage.SpoolMax, 3)
	state := pyjson.New()
	for i := range 5 {
		usage.Spool(state, &usage.Reading{ObservedAt: usageAt.Add(time.Duration(i) * time.Minute), Subscription: "gpt",
			Util7d: usage.N(float64(90 + i))})
	}
	usage.Spool(state, &usage.Run{EndedAt: usageAt, RunID: "9", Subscription: "gpt"})
	spooled := usage.SpoolOf(state)
	assertEqual(t, len(spooled), 4)
	assertEqual(t, spooled[0].Row.(*usage.Reading).Util7d, usage.N(92)) // the oldest went
	assertEqual(t, spooled[3].Row.(*usage.Run).RunID, "9")
	usage.Spool(state, spooled[1].Row) // the same line again, after the loop took it
	usage.Unspool(state, map[string][]string{"readings": {spooled[0].Line, spooled[1].Line}})
	left := usage.SpoolOf(state)
	assertEqual(t, len(left), 3)
	assertEqual(t, left[0].Row.(*usage.Reading).Util7d, usage.N(94))
	assertEqual(t, left[1].Row.(*usage.Reading).Util7d, usage.N(93)) // spooled again: it stays
	var all []string
	for _, s := range left {
		all = append(all, s.Line)
	}
	usage.Unspool(state, map[string][]string{"readings": all[:2], "runs": all[2:]})
	assertFalse(t, state.Has(usage.SpoolKey))
}

// A run's readings go into its result as it takes them (its start ping, what each call streams,
// a call's end), deliver hands them and the run's row to the loop in the state file's spool, and
// the loop's flush writes them to the branch and takes them out of the spool.
func TestUsage_ARunsReadingsReachTheReadingsFileThroughDeliverAndTheLoopsFlush(t *testing.T) {
	h := newFlowHarness(t)
	scratch := filepath.Join(h.root, "scratch")
	patch(t, &usage.Scratch, func() string { return scratch })
	resets := testsupport.Night.Add(2 * time.Hour)
	h.gh.AddIssue(5, fakes.Issue{Title: "Rules v2", Body: "Make the rules v2.", Labels: []string{config.LabelBuild}})
	build := func(request *runner.RunRequest) *runner.RunResult {
		assertTrue(t, request.UsageSeen != nil, "the call's watch hands on what it sees")
		request.UsageSeen(usageReading(0.20, resets))
		request.UsageSeen(usageReading(0.35, resets))
		writeText(t, filepath.Join(request.Cwd, "src", "game.txt"), "rules v2\n")
		result := runner.NewRunResult(true, workDone)
		result.Usage = usageReading(0.42, resets)
		return result
	}
	usageJournalOn(t, h.origin, map[string]string{"1.md": "# Journal of #1\n"})
	// The night run, with a clock a second on at each look, so each reading has a moment of its own.
	planned := flowMake(h.ctx)
	out := filepath.Join(h.root, "out-usage")
	moment := testsupport.Night
	worker := work.NewWorker(h.cfg, planned, workFake(map[string]runner.Handler{"build": build,
		"review": workReviewer(workApprove)}), h.clone, filepath.Join(h.root, "work"), out, nil, nil,
		func() time.Time { moment = moment.Add(time.Second); return moment }, nil, nil, nil)
	worker.GH = h.gh
	worker.Run()
	deliver.NewDeliverer(h.ctx, planned, out, h.deliverRepo, "", nil, "").Run()
	result := loadJSON(t, filepath.Join(out, "result.json"))
	assertEqual(t, result.Get("status"), "approved")
	provider := planned.Str("provider")
	assertEqual(t, len(pyjson.List(result.Get("readings"))), 3)
	facts := planned.Obj(plan.UsageFacts) // what the claim noted for the run's row: a first claim
	assertEqual(t, []any{facts.Get("handed_off"), facts.Get("previous_provider"), facts.Get("claimed_at")},
		[]any{"", nil, usage.ISO(testsupport.Night)})
	spooled := usage.SpoolOf(h.ctx.Store.Load())
	assertEqual(t, len(spooled), 4) // three readings, then the run's row
	run := spooled[3].Row.(*usage.Run)
	assertEqual(t, []any{run.RunID, run.Subscription, run.Kind, run.Item, run.Outcome, run.StopReason, run.Util5hStart,
		run.Util5hEnd}, []any{"777", provider, "build", "5", "approved", "", usage.N(20), usage.N(42)})
	// The loop's tick: the state file's reading of the subscription is the run's last, re-dated
	// by its delivery, so it adds nothing; the flush pushes the spool's rows and empties it.
	rec := usage.NewRecorder(nil)
	rec.Tick(h.ctx, usage.TickFacts{State: h.ctx.Store.Load(), Live: pyjson.New()})
	assertEqual(t, rec.Problem, "")
	text := fakes.Git(h.origin, "show", "bot-usage:usage/readings-2026-09.csv")
	lines := strings.Split(text, "\n")
	assertEqual(t, len(lines), 4, text)
	assertIn(t, ","+provider+",stream,call,777,20,"+usage.ISO(resets)+",,,allowed,", lines[1])
	assertIn(t, ","+provider+",stream,call,777,35,", lines[2])
	assertIn(t, ","+provider+",stream,call,777,42,", lines[3])
	assertIn(t, "777,"+provider+",", fakes.Git(h.origin, "show", "bot-usage:usage/runs-2026-09.csv"))
	assertFalse(t, h.ctx.Store.Load().Has(usage.SpoolKey))
}

// ------------------------------------------------------------------ the loop

// The loop's first flush starts the branch, an orphan with an empty first commit, and pushes it.
func TestUsage_TheBranchIsCreatedAsAnOrphan(t *testing.T) {
	w := newUsageWorld(t)
	assertEqual(t, w.show("usage/ticks-2026-11.csv"), "")
	w.tick()
	assertEqual(t, w.rec.Problem, "")
	assertIn(t, "`bot-usage` is not there yet: its first push makes it", strings.Join(w.logs, "\n"))
	assertEqual(t, w.show("vercel.json"), "")
	first := fakes.Git(w.origin, "rev-list", "--max-parents=0", "bot-usage")
	assertEqual(t, fakes.Git(w.origin, "log", "-1", "--format=%s", first), "usage: start [skip ci]")
	_, failed := herr.Catch[*fakes.PyError](func() { fakes.Git(w.origin, "merge-base", "main", "bot-usage") })
	assertTrue(t, failed, "an orphan: it shares no history with main")
	assertTrue(t, len(w.rows("usage/ticks-2026-11.csv")) > 1)
	assertTrue(t, usage.HasMarker(w.clone()), "the backfill ran once, by itself")
}

// A tick writes a subscription's row when anything in it changed, and at least hourly, not
// otherwise; the state file's newest reading is recorded once, as the loop's.
func TestUsage_ATickRowIsWrittenWhenSomethingChangedAndAtLeastHourly(t *testing.T) {
	w := newUsageWorld(t)
	base := usageAt.Add(-8 * time.Hour) // 22:00 in Chicago: the Claude accounts' hours, all along
	read := func(share float64, at time.Time) {
		w.state(func(s *pyjson.Object) {
			reading := usageReading(share, base.Add(4*time.Hour))
			reading.Set("observed_at", usage.ISO(at))
			pyjson.Obj(pyjson.Obj(s.SetDefault("providers", pyjson.New())).SetDefault("claude-3", pyjson.New())).Set("usage", reading)
		})
	}
	read(0.2, base.Add(-10*time.Minute))
	w.at(base)
	w.tick() // the first: it flushes
	for _, minutes := range []int{5, 10, 35, 65} {
		w.at(base.Add(time.Duration(minutes) * time.Minute))
		if minutes == 10 {
			read(0.3, base.Add(8*time.Minute))
		}
		w.tick()
	}
	at := func(sub string) []string {
		var found []string
		for _, line := range w.rows("usage/ticks-2026-11.csv")[1:] {
			if cells := strings.Split(line, ","); len(cells) > 1 && cells[1] == sub {
				found = append(found, cells[0][11:16])
			}
		}
		return found
	}
	assertEqual(t, at("claude-3"), []string{"04:00", "04:10"}) // its reading changed at 04:10
	assertEqual(t, at("claude-1"), []string{"04:00", "05:05"}) // nothing changed: hourly
	readings := w.rows("usage/readings-2026-11.csv")
	assertEqual(t, len(readings), 3)
	assertIn(t, "2026-11-02T03:50:00Z,claude-3,,tick,,20,", readings[1])
	assertIn(t, "2026-11-02T04:08:00Z,claude-3,,tick,,30,", readings[2])
	pools := w.rows("usage/pools-2026-11.csv")
	assertEqual(t, len(pools), 3) // 04:00 and 05:05: an hour with nothing changed
}

// A reading the history holds already (a run's, which its delivery re-dated in the state file)
// is not recorded again from the state file.
func TestUsage_TheStateFilesReadingOfADeliveredRunIsNotRecordedTwice(t *testing.T) {
	w := newUsageWorld(t)
	resets := usageAt.Add(time.Hour)
	w.state(func(s *pyjson.Object) {
		usage.Spool(s, usage.FromUsage(usageReading(0.5, resets), usageAt.Add(-20*time.Minute), "claude-1", "stream",
			"call", "555", pyjson.New("five_hour", 1.0)))
		reading := usageReading(0.5, resets)
		reading.Set("observed_at", usage.ISO(usageAt.Add(-5*time.Minute)))
		pyjson.Obj(pyjson.Obj(s.SetDefault("providers", pyjson.New())).SetDefault("claude-1", pyjson.New())).Set("usage", reading)
	})
	w.tick()
	readings := w.rows("usage/readings-2026-11.csv")
	assertEqual(t, len(readings), 2)
	assertIn(t, "2026-11-02T11:40:00Z,claude-1,stream,call,555,50,", readings[1])
}

func TestUsage_EachTickStateFromWhatTheStatusIssueSays(t *testing.T) {
	ctx := testsupport.MakeCtx(nil, testsupport.Ctx{At: usageAt})
	for kind, want := range map[string]string{"busy": "working", "next": "ready", "idle": "idle",
		"capped": "capped", "reviews": "capped", "closed": "outside-hours", "broken": "backing-off",
		"off": "off", "full": "no-runner"} {
		explained := &waiting.Explained{Now: pyjson.New("claude-3", &waiting.Now{Kind: kind})}
		ticks, _ := usage.TickRows(ctx, usage.TickFacts{State: pyjson.New(), Live: pyjson.New(), Explained: explained})
		assertEqual(t, ticks[0].Subscription, "claude-3")
		assertEqual(t, ticks[0].State, want, kind)
	}
}

func TestUsage_ATickCountsTheLanesTheQueueAndTheHoldsForEachSubscription(t *testing.T) {
	ctx := testsupport.MakeCtx(nil, testsupport.Ctx{At: usageAt})
	state := pyjson.New("items", pyjson.New("7", pyjson.New("action", "build"), "8", pyjson.New("action", "plan")),
		"providers", pyjson.New("claude-5", pyjson.New("refused_until", usage.ISO(usageAt.Add(time.Hour)),
			"usage", usageReading(0.38, usageAt.Add(2*time.Hour), 0.55).Set("observed_at",
				usage.ISO(usageAt.Add(-90*time.Minute))))))
	rows := []*waiting.Row{
		{Number: 1, Step: "build", Difficulty: "medium", Able: []string{"claude-3", "claude-1"},
			QueuedAt: usage.ISO(usageAt.Add(-30 * time.Minute))},
		{Number: 2, Step: "plan", Difficulty: "unrated", Able: []string{"claude-3"}, Starts: &plan.Assignment{}},
		{Number: 3, Step: "build", Difficulty: "hard", Held: true, Able: []string{"claude-3"}},
		{Number: 4, Step: "revise", Difficulty: "hard", Able: []string{"claude-1"}, HeldFor: "claude-1",
			QueuedAt: usage.ISO(usageAt.Add(-10 * time.Minute))},
	}
	ticks, pool := usage.TickRows(ctx, usage.TickFacts{State: state,
		Live: pyjson.New("7", "claude-3", "8", "claude-3"), Explained: &waiting.Explained{Rows: rows}})
	by := map[string]*usage.Tick{}
	for _, tick := range ticks {
		by[tick.Subscription] = tick
	}
	three, one, five := by["claude-3"], by["claude-1"], by["claude-5"]
	assertEqual(t, []string{three.QueuedForIt.String(), three.WaitingForIt.String(), three.HeldForIt.String()},
		[]string{"2", "1", "0"})
	assertEqual(t, []string{one.QueuedForIt.String(), one.WaitingForIt.String(), one.HeldForIt.String()},
		[]string{"2", "2", "1"})
	assertEqual(t, []string{three.RunsHeld.String(), three.RunsBuild.String(), three.RunsPlan.String(),
		three.PlanningHeld.String()}, []string{"2", "1", "1", "1"})
	assertEqual(t, []string{five.Cap5hNow.String(), five.Cap7dNow.String(), five.HardCap.String(),
		five.StartHeadroom5h.String(), five.Util5h.String(), five.Util7d.String(), five.ReadingAgeMin.String(),
		usage.ISO(five.RefusedUntil)}, []string{"40", "60", "90", "5", "38", "55", "90", "2026-11-02T13:00:00Z"})
	assertEqual(t, usage.Encode(pool)[1:], []string{"2", "0", "1", "0", "0", "1", "1", "0", "0", "0", "1", "0", "0",
		"0", "0", "30", "8", "6", "0", "3", "1"})
}

// The loop's flush pushes at most every 30 minutes; a push that fails is logged, the loop goes
// on, and the next flush pushes what it kept.
func TestUsage_AFailedPushIsLoggedAndTheLoopGoesOn(t *testing.T) {
	w := newUsageWorld(t)
	w.tick()
	assertEqual(t, w.rec.Problem, "")
	gone := w.origin + ".gone"
	assertTrue(t, os.Rename(w.origin, gone) == nil)
	w.at(usageAt.Add(31 * time.Minute))
	w.state(func(s *pyjson.Object) {
		usage.Spool(s, &usage.Run{EndedAt: usageAt.Add(20 * time.Minute), RunID: "901", Subscription: "agy"})
	})
	w.tick() // no panic: a warning
	assertIn(t, "could not push `bot-usage` (try 1)", w.rec.Problem)
	assertIn(t, "::warning::", strings.Join(w.logs, "\n"))
	assertEqual(t, len(usage.SpoolOf(w.ctx.Store.Load())), 1) // kept for the next push
	assertTrue(t, os.Rename(gone, w.origin) == nil)
	w.at(usageAt.Add(45 * time.Minute))
	w.tick() // not due: nothing pushed
	assertNotIn(t, "901", w.show("usage/runs-2026-11.csv"))
	w.at(usageAt.Add(62 * time.Minute))
	w.tick()
	assertIn(t, "901,agy", w.show("usage/runs-2026-11.csv"))
	assertEqual(t, len(usage.SpoolOf(w.ctx.Store.Load())), 0)
}

// A push the branch's moving refused is tried again on its new tip, with the rows written again on
// top, so neither side's rows are lost.
func TestUsage_APushTheBranchMovedUnderIsWrittenAgainOnItsNewTip(t *testing.T) {
	w := newUsageWorld(t)
	w.tick()
	other := w.clone()
	writeText(t, filepath.Join(other, "usage", "note.txt"), "someone else's commit\n")
	fakes.Git(other, "add", "-A")
	fakes.Git(other, "commit", "-q", "-m", "elsewhere")
	fakes.Git(other, "push", "-q", "origin", "bot-usage")
	w.at(usageAt.Add(30 * time.Minute))
	w.state(func(s *pyjson.Object) {
		usage.Spool(s, &usage.Run{EndedAt: usageAt.Add(20 * time.Minute), RunID: "902", Subscription: "muse"})
	})
	w.tick()
	assertEqual(t, w.rec.Problem, "")
	assertIn(t, "902,muse", w.show("usage/runs-2026-11.csv"))
	assertEqual(t, w.show("usage/note.txt"), "someone else's commit")
	assertEqual(t, strings.Count(w.show("usage/ticks-2026-11.csv"), "2026-11-02T12:00:00Z,claude-3,"), 1)
}

// The status issue's rewrite hands its view to the history, and the loop (`harness dashboard
// --every`) records a tick from it, and flushes once more as it ends.
func TestUsage_TheStatusLoopRecordsEachRewritesView(t *testing.T) {
	w := newUsageWorld(t)
	var seen *dashboard.View
	restore := patch(t, &dashboard.Looked, func(_ *hctx.Context, view *dashboard.View) { seen = view })
	dashboardUpdate(w.ctx)
	assertTrue(t, seen != nil && seen.State != nil && seen.Explained != nil, "the rewrite's view")
	restore()
	update := func(ctx *hctx.Context, _ []string, _ bool, _ func() []string, _ bool, _ cardsets.Fetch) string {
		assertTrue(t, dashboard.Looked != nil, "the loop records its ticks")
		dashboard.Looked(ctx, &dashboard.View{State: ctx.Store.Load(), Live: pyjson.New()})
		return "rewrote #148"
	}
	patch[dashboardUpdateFunc](t, &dashboard.Update, update)
	patch(t, &cli.Ctx, func(cfg *config.Config, write bool) *hctx.Context { return w.ctx })
	now := 0.0
	patch(t, &cli.Monotonic, func() float64 { return now })
	patch(t, &cli.Sleep, func(s float64) { now += s })
	out, _ := capture(t, &cli.Stdout)
	assertEqual(t, cli.CmdDashboard(w.cfg, &cli.Args{Every: 300, ForSeconds: 300}), 0)
	assertTrue(t, dashboard.Looked == nil, "put back when the loop ends")
	assertIn(t, "usage: pushed", out.String())
	assertTrue(t, len(w.rows("usage/ticks-2026-11.csv")) > 1)
}

// ------------------------------------------------------------------ sessions and weeks

// claude-4's session from 05:00 to its reset at 10:00: cut off at its 70% cap at 08:00, past it in
// its last hour (lifted to 100%), with ticks capped from 08:05 to 09:05, work queued for half of
// that, and two runs ending in it.
func usageSession() ([]*usage.Reading, []*usage.Tick, []*usage.Run, time.Time) {
	reset := time.Date(2026, 11, 1, 10, 0, 0, 0, time.UTC)
	at := func(h, m int) time.Time { return time.Date(2026, 11, 1, h, m, 0, 0, time.UTC) }
	read := func(when time.Time, util, cap float64) *usage.Reading {
		return &usage.Reading{ObservedAt: when, Subscription: "claude-4", Source: "stream", During: "call",
			Util5h: usage.N(util), Resets5h: reset, Cap5h: usage.N(cap)}
	}
	readings := []*usage.Reading{read(at(5, 30), 10, 70), read(at(7, 0), 50, 70), read(at(8, 0), 71, 70),
		read(at(8, 30), 72, 70), read(at(9, 10), 80, 100), read(at(9, 40), 85, 100),
		// another subscription, and claude-4's next session
		{ObservedAt: at(9, 0), Subscription: "claude-3", Util5h: usage.N(30), Resets5h: reset.Add(time.Hour)},
		read(at(10, 30), 5, 70)}
	readings[len(readings)-1].Resets5h = reset.Add(5 * time.Hour)
	tick := func(when time.Time, state string, queued float64) *usage.Tick {
		return &usage.Tick{At: when, Subscription: "claude-4", State: state, QueuedForIt: usage.N(queued),
			Cap5hNow: usage.N(70)}
	}
	ticks := []*usage.Tick{tick(at(8, 5), "capped", 2), tick(at(8, 35), "capped", 0), tick(at(9, 5), "working", 1)}
	runs := []*usage.Run{{EndedAt: at(8, 10), RunID: "1", Subscription: "claude-4", StopReason: "5-hour cap",
		ModelMinutes: usage.N(40)}, {EndedAt: at(9, 50), RunID: "2", Subscription: "claude-4", ModelMinutes: usage.N(30)},
		{EndedAt: at(10, 30), RunID: "3", Subscription: "claude-4", ModelMinutes: usage.N(9)}}
	return readings, ticks, runs, reset
}

func TestUsage_AClosedSessionIsDerivedFromItsReadings(t *testing.T) {
	readings, ticks, runs, reset := usageSession()
	assertEqual(t, len(usage.DeriveSessions(readings, ticks, runs, nil, reset.Add(2*time.Hour))), 0, "not settled yet")
	var mine []*usage.Session
	for _, s := range usage.DeriveSessions(readings, ticks, runs, nil, reset.Add(7*time.Hour)) {
		if s.Subscription == "claude-4" {
			mine = append(mine, s)
		}
	}
	assertEqual(t, len(mine), 1) // its next session has not settled
	assertEqual(t, usage.Encode(mine[0]), []string{"claude-4", "2026-11-01T05:00:00Z", "2026-11-01T10:00:00Z",
		"70", "100", "85", "85", "15", "2026-11-01T08:00:00Z", "60", "30", "13", "2", "70", "1", "0", "2", "6"})
}

func TestUsage_AWeekIsDerived(t *testing.T) {
	reset := time.Date(2026, 10, 30, 22, 0, 8, 0, time.UTC)
	at := func(month time.Month, day, h int) time.Time { return time.Date(2026, month, day, h, 0, 0, 0, time.UTC) }
	read := func(when time.Time, util float64, resets time.Time) *usage.Reading {
		return &usage.Reading{ObservedAt: when, Subscription: "agy", Source: "usage-command", During: "after",
			Util7d: usage.N(util), Resets7d: resets, Cap7d: usage.N(100)}
	}
	readings := []*usage.Reading{read(at(10, 24, 9), 20, reset), read(at(10, 28, 9), 90, reset.Add(time.Minute)),
		read(at(10, 29, 12), 100, reset), {ObservedAt: at(10, 29, 12).Add(time.Second), Subscription: "agy",
			Source: "refusal", During: "call", Status: "rejected", RefusedUntil: reset}}
	runs := []*usage.Run{{EndedAt: at(10, 25, 3), RunID: "1", Subscription: "agy", ModelMinutes: usage.N(90),
		TokensIn: usage.N(100), TokensOut: usage.N(50)}, {EndedAt: at(10, 29, 11), RunID: "2", Subscription: "agy",
		ModelMinutes: usage.N(30), TokensIn: usage.N(200), TokensOut: usage.N(0)}}
	weeks := usage.DeriveWeeks(readings, nil, runs, nil, reset.Add(7*time.Hour))
	assertEqual(t, len(weeks), 1)
	assertEqual(t, usage.Encode(weeks[0]), []string{"agy", "2026-10-30T22:00:08Z", "100", "100", "0",
		"2026-10-29T12:00:00Z", "1.4", "2", "2", "350", "", "3"})
}

func TestUsage_DerivationIsIdempotent(t *testing.T) {
	readings, ticks, runs, reset := usageSession()
	h := usage.NewHistory(t.TempDir())
	var rows []usage.Row
	for _, r := range readings {
		rows = append(rows, r)
	}
	for _, r := range ticks {
		rows = append(rows, r)
	}
	for _, r := range runs {
		rows = append(rows, r)
	}
	_, err := h.Append(rows...)
	assertTrue(t, err == nil, err)
	now := reset.Add(7 * time.Hour)
	sessions, _, err := h.Derive(now, usage.DeriveLookback)
	assertTrue(t, err == nil, err)
	assertEqual(t, sessions, 2) // claude-4's and claude-3's
	sessions, weeks, _ := h.Derive(now, usage.DeriveLookback)
	assertEqual(t, []int{sessions, weeks}, []int{0, 0})
	// A later reading that names the same reset a minute off (as Muse's `/usage` does) is the same
	// session, never a second one.
	late := *readings[0]
	late.ObservedAt, late.Resets5h = late.ObservedAt.Add(time.Minute), reset.Add(time.Minute)
	h.Append(&late)
	sessions, _, _ = usage.NewHistory(h.Root).Derive(now, usage.DeriveLookback)
	assertEqual(t, sessions, 0)
	// A later flush looks back a couple of days only, and finds nothing new either.
	sessions, weeks, _ = usage.NewHistory(h.Root).Derive(now.Add(72*time.Hour), usage.DeriveRecent)
	assertEqual(t, []int{sessions, weeks}, []int{0, 0})
	read, _ := usage.ReadSessions(h.Root, time.Time{}, time.Time{})
	assertEqual(t, len(read), 2)
}

// ------------------------------------------------------------------ the backfill

// One journal file, in each format `journal.Section` has written: a run before the backfill's
// first day, an old section with no tokens or steps, a new one with tokens, a session and steps
// (its notes, which repeat a line the backfill reads, are not its facts), a `/usage` cut-off
// before a run started, and the same run delivered twice.
var usageJournal = "# Journal of #900\n\nEvery night-bot run on #900 adds a section here.\n\n" +
	"<!-- jackioh-bot:run -->\n## 2026-10-05 23:10 UTC · plan #900 · `claude-1` · planned\n\n" +
	"[run](https://github.com/Jlore-Studios/JackiOh/actions/runs/100) on `claude-1` (`opus` (strong)). It ended " +
	"**planned**: planned on opus\nModel calls: 1, 2.0 model minutes.\n\n" +
	"<!-- jackioh-bot:run -->\n## 2026-10-06 01:32 UTC · build #900 · `claude-4` · interrupted\n\n" +
	"[run](https://github.com/Jlore-Studios/JackiOh/actions/runs/101) on `claude-4` (`opus` (strong)). It ended " +
	"**interrupted**: `claude-4` stops mid-call: 5-hour usage is 50%, at or over its 50% cap outside its hours; it " +
	"resets at 2026-10-06T05:20:00Z\nModel calls: 4, 48.8 model minutes.\nIts work ended at `eb46d5242439` on " +
	"`bot/issue-900`.\n\n<details><summary>The builder's notes when it stopped</summary>\n\n````markdown\n" +
	"Model calls: 9, 99.0 model minutes.\n````\n\n</details>\n\n" +
	"<!-- jackioh-bot:run -->\n## 2026-10-16 05:37 UTC · build #900 · `claude-3` · interrupted\n\n" +
	"[run](https://github.com/Jlore-Studios/JackiOh/actions/runs/102) on `claude-3` (`sonnet` (medium), `opus` " +
	"(strong)). It ended **interrupted**: `claude-3` stops: it refused a call; its limit resets at " +
	"2026-10-16T08:00:00Z\nModel calls: 1, 11.7 model minutes.\nTokens: 16 in / 4585 out / 385955 cached, $0.26.\n" +
	"Session: resumed `abcd1234`, kept by the run on `claude-3` at 01:00; that call took 11.7 min.\n\n" +
	"| # | When (UTC) | Step | Why | Took | What came of it |\n|---|---|---|---|---|---|\n" +
	"| 1 | 05:25 | start | from `origin/main` |  | at `701e34feb3c2` |\n" +
	"| 2 | 05:25 | build (`sonnet`) | build the issue | 11.7 min, 4601 tok, $0.26 | refused: the usage limit |\n" +
	"| 3 | 05:36 | stop | `claude-3` stops |  |  |\n\n" +
	"<!-- jackioh-bot:run -->\n## 2026-10-30 16:40 UTC · plan #900 · `muse` · interrupted\n\n" +
	"[run](https://github.com/Jlore-Studios/JackiOh/actions/runs/103) on `muse` (`muse-spark-1.3-contributor` " +
	"(medium)). It ended **interrupted**: `muse` stops before it starts: 7-day usage is 100%, at or over its 100% " +
	"cap; it resets at 2026-11-02T00:00:00Z\nModel calls: 0, 0.0 model minutes.\n\n" +
	"<!-- jackioh-bot:run -->\n## 2026-10-30 16:41 UTC · plan #900 · `muse` · interrupted\n\n" +
	"[run](https://github.com/Jlore-Studios/JackiOh/actions/runs/103) on `muse` (`muse-spark-1.3-contributor` " +
	"(medium)). It ended **interrupted**: `muse` stops before it starts: 7-day usage is 100%, at or over its 100% " +
	"cap; it resets at 2026-11-02T00:00:00Z\n"

func TestUsage_TheBackfillReadsTheJournalsOldAndNewSections(t *testing.T) {
	runs, readings := usage.ParseJournal(usageJournal, testsupport.MakeConfig().Pool, usage.BackfillSince)
	var shown [][]string
	for _, r := range runs {
		shown = append(shown, usage.Encode(r))
	}
	assertEqual(t, shown, [][]string{
		{"2026-10-06T01:32:00Z", "101", "claude-4", "opus", "build", "900", "", "interrupted", "5-hour cap", "48.8",
			"", "", "", "", "", "50", "", "", "", "claude-1", "none", "", "4"},
		{"2026-10-16T05:37:00Z", "102", "claude-3", "sonnet+opus", "build", "900", "", "interrupted", "refusal",
			"11.7", "16", "4585", "385955", "0.26", "", "", "", "", "", "claude-4", "resumed", "", "1"},
		{"2026-10-30T16:40:00Z", "103", "muse", "muse-spark-1.3-contributor", "plan", "900", "", "interrupted",
			"weekly cap", "0", "", "", "", "", "", "", "", "100", "", "claude-3", "none", "", "0"},
		{"2026-10-30T16:41:00Z", "103", "muse", "muse-spark-1.3-contributor", "plan", "900", "", "interrupted",
			"weekly cap", "", "", "", "", "", "", "", "", "100", "", "muse", "none", "", ""},
	})
	shown = nil
	for _, r := range readings {
		shown = append(shown, usage.Encode(r))
	}
	assertEqual(t, shown, [][]string{
		{"2026-10-06T01:32:00Z", "claude-4", "stream", "call", "101", "50", "2026-10-06T05:20:00Z", "", "", "", "50", "", ""},
		{"2026-10-16T05:36:00Z", "claude-3", "refusal", "call", "102", "", "", "", "", "rejected", "", "",
			"2026-10-16T08:00:00Z"},
		{"2026-10-30T16:40:00Z", "muse", "usage-command", "before", "103", "", "", "100", "2026-11-02T00:00:00Z", "",
			"", "100", ""},
		{"2026-10-30T16:41:00Z", "muse", "usage-command", "before", "103", "", "", "100", "2026-11-02T00:00:00Z", "",
			"", "100", ""},
	})
}

// The loop backfills by itself the first time it finds the branch without the marker, and
// `harness usage backfill` reads the journal again: it adds nothing the branch already holds.
func TestUsage_TheBackfillRunsOnceByItselfAndItsRerunAddsNothing(t *testing.T) {
	w := newUsageWorld(t)
	w.journal(map[string]string{"900.md": usageJournal, "901.md": "# Journal of #901\n"})
	w.tick()
	assertEqual(t, w.rec.Problem, "")
	assertIn(t, "backfilled from `bot-journal` since 2026-10-06: 4 run(s) and 4 reading(s) in 2 file(s)",
		strings.Join(w.logs, "\n"))
	runs := w.rows("usage/runs-2026-10.csv")
	assertEqual(t, len(runs), 4, runs) // the header, and runs 101, 102, 103: 103's second delivery adds nothing
	assertEqual(t, len(w.rows("usage/readings-2026-10.csv")), 5)
	marker := loadsObject(t, w.show(usage.MarkerPath))
	assertEqual(t, []any{marker.Get("read"), marker.Get("since"), marker.Get("runs"), marker.Get("files")},
		[]any{"bot-journal", "2026-10-06", 4, 2})
	// A later flush does not backfill again.
	w.at(usageAt.Add(31 * time.Minute))
	w.tick()
	assertEqual(t, strings.Count(strings.Join(w.logs, "\n"), "backfilled from"), 1)
	// A person's rerun, as the command line runs it.
	patch(t, &cli.Ctx, func(cfg *config.Config, write bool) *hctx.Context { return w.ctx })
	out, _ := capture(t, &cli.Stdout)
	args, _, ok := cli.Parser().ParseArgs([]string{"usage", "backfill", "--since", "2026-10-01"})
	assertTrue(t, ok)
	assertEqual(t, cli.CmdUsage(w.cfg, args), 0, out.String())
	assertIn(t, "since 2026-10-01: 5 run(s)", out.String())
	assertIn(t, "2026-10-05T23:10:00Z,100,claude-1,", w.show("usage/runs-2026-10.csv")) // the day it reached back to
	assertEqual(t, len(w.rows("usage/runs-2026-10.csv")), 5)
	assertEqual(t, len(w.rows("usage/readings-2026-10.csv")), 5)
	assertEqual(t, loadsObject(t, w.show(usage.MarkerPath)).Get("since"), "2026-10-01")
}

// The night bot records; its identity names the branch. (Squishy's process checks that Squishy
// records nothing: squishy_cases_test.go.)
func TestUsage_TheNightBotRecordsOnItsOwnBranch(t *testing.T) {
	assertEqual(t, config.Identity.UsageBranch, "bot-usage")
	assertTrue(t, testsupport.MakeConfig().RecordsUsage())
	off := testsupport.MakeConfig(testsupport.Config{Overrides: pyjson.New("usage_history", false)})
	assertFalse(t, off.RecordsUsage())
	w := newUsageWorld(t)
	w.cfg = off
	w.ctx = testsupport.MakeCtx(w.gh, testsupport.Ctx{Cfg: off, At: usageAt})
	w.tick()
	assertEqual(t, w.rec.Root(), "")
	assertEqual(t, w.show("usage/ticks-2026-11.csv"), "")
	assertFalse(t, slices.ContainsFunc(w.logs, func(l string) bool { return strings.Contains(l, "usage:") }))
}

// A claim that takes an item from its owner notes why, for the run row's `handed_off`; one that
// takes up an item nobody owned notes nothing.
func TestUsage_AClaimNotesWhyItsItemChangedHands(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(5, fakes.Issue{Title: "Rules v2", Body: "Make the rules v2.", Labels: []string{config.LabelBuild}})
	flowUpdate(h.ctx, func(s *pyjson.Object) {
		pyjson.Obj(pyjson.Obj(s.Get("items")).SetDefault("5", pyjson.New())).Update(pyjson.New("provider", "claude-4",
			"owner", "claude-4", "owner_since", usage.ISO(testsupport.Night.Add(-time.Hour)), "owner_why", "it plans it"))
	})
	planned := flowMake(h.ctx)
	assertEqual(t, planned.Get("provider"), "claude-1")
	facts := planned.Obj(plan.UsageFacts)
	assertEqual(t, facts.Get("previous_provider"), "claude-4")
	assertIn(t, "handed off from `claude-4`", facts.Str("handed_off"))
	run := usage.RunOf(usage.RunFacts{At: testsupport.Night, Subscription: "claude-1", Plan: planned,
		Result: pyjson.New("status", "approved")})
	assertEqual(t, []string{run.PreviousSubscription, run.HandedOff}, []string{"claude-4", "other"})
}
