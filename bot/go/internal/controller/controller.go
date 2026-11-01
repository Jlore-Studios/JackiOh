// Package controller is bot/harness/controller.py.
//
// The status loop's planner (#1455): with `controller` on, `bot-status`'s loop plans the night
// bot's work and starts each run with its plan made, so a run no longer waits in GitHub's queue for
// a runner for its gate and its plan before its model job (6 to 25 minutes each on 2026-10-20, for
// jobs that ran under a minute).
//
// The loop already asked the gate's question every tick (`sweep._night_run`, `plan.peek`); now, when
// there is work, it plans itself, under the planning lock (`plan.take_planning_lock`) that the plan
// job takes too, and for each run:
//
//   - claims the item as a plan job does (`plan.make`), with `pending:<ref>` for the run id, since the
//     run it starts has no id yet; `plan.run_status` reads such a claim by the run whose title names
//     the reference, and gives it `plan.PENDING_GRACE` to show up;
//   - writes the plan to the state branch, `plans/<ref>.json`, where only the bots' tokens write, so the
//     model job reads it and cannot change it (bot-night.yml's `plan_ref`);
//   - starts bot-night with `plan_ref` and the plan's runner, CLI, secret name and shared flag, which
//     bot-night's gate and plan jobs skip on, and whose model job checks every one of them against
//     `providers.json` before any step is handed a secret (`__main__.cmd_plan_fetch`);
//   - finds the run by its title (`bot-night <ref> on <runner>`) and puts its id on the claim.
//
// It plans again until nothing more can start, as each plan job's `fill_lanes` used to start one
// more run, and prunes plan files older than `KEEP_PLANS`. bot-night's own gate and plan still run
// for a run started any other way (its schedule, a command, a person), and plan only while no loop
// is going (`plan.loop_alive`): rule 13 keeps the schedule.
//
// The loop also delivers the runs it started (#1537), so they have no `deliver` job either, which
// waited 7 to 25 minutes for a hosted runner on 2026-10-20 to run for under a minute. Each tick, before
// it plans, `deliver` takes every such run that has ended (`plan.awaiting_delivery`): it reads the
// run's plan from the state branch, downloads the model job's `work` upload through the Actions API
// and unzips it, clones the default branch afresh into the runner's scratch directory (the loop's
// own checkout is shallow and hours old, and the deliverer checks that the bundle descends from the
// commit the work started at), and runs the same `Deliverer` deliver ran, as the run. Each try is
// recorded in the state file first (`plan.DELIVERIES`), so one that fails, or ends the loop's
// process, is tried again on the next tick, at most `plan.DELIVER_TRIES` times. Until then
// housekeeping leaves the item alone; after them, or `plan.DELIVER_WITHIN` after the run ended, it
// is housekeeping's, as a dead run's item is. A delivery that fails never stops the tick: git runs
// through `subprocess` with its timeouts, and every failure is a note.
package controller

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"regexp"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Plans is where the loop keeps the plans of the runs it starts, on the state branch.
var Plans = "plans"

// Ref is a plan's reference: random hex, then the unix time of its claim (`plan.pending_run` reads
// it). (Python's `$` also matches before a final newline, hence the `\n?`.)
var Ref = regexp.MustCompile(`^[0-9a-f]{8}-[0-9]{9,11}\n?$`)

// KeepPlans is how long a plan file stays on the state branch: a run's 350 minutes and a week to
// read it back.
var KeepPlans = 7 * 24 * time.Hour

// How many times, and how far apart in seconds, the loop looks for the run it just started.
var (
	FindTries = 4
	FindWait  = 3.0
)

// Sleep is controller's time.sleep; a variable so the tests can replace it.
var Sleep = func(seconds float64) { time.Sleep(time.Duration(seconds * float64(time.Second))) }

// ValueError is the ValueError `fetch` raises, which its callers catch: Type is type(exc).__name__
// (`ValueError`, or `JSONDecodeError` for a plan that is not JSON) and Msg is str(exc).
type ValueError = py.Error

func valueError(format string, args ...any) *ValueError {
	return &ValueError{Type: "ValueError", Msg: fmt.Sprintf(format, args...)}
}

// TitlePrefix is controller.title_prefix: how a night run's title starts, which names the plan it
// carries (bot-night.yml's `run-name`, `bot-night <ref> on <runs_on>`).
func TitlePrefix(ref string) string {
	return fmt.Sprintf("bot-night %s ", ref)
}

// NewRef is controller.new_ref.
func NewRef(now time.Time) string {
	token := make([]byte, 4)
	if _, err := rand.Read(token); err != nil {
		panic(err)
	}
	return fmt.Sprintf("%s-%d", hex.EncodeToString(token), now.Unix())
}

// withRunID is dataclasses.replace(ctx, cfg=dataclasses.replace(ctx.cfg, run_id=runID)).
func withRunID(ctx *hctx.Context, runID string) *hctx.Context {
	cfg := *ctx.Cfg
	cfg.RunID = runID
	c := *ctx
	c.Cfg = &cfg
	return &c
}

// Start plans and starts night runs until nothing more can start. Each line says what happened.
func Start(ctx *hctx.Context, sleep func(float64) /* = None */) []string {
	if sleep == nil {
		sleep = Sleep
	}
	pool := ctx.Cfg.Pool
	runID := ctx.Cfg.RunID
	if runID == "" {
		runID = "by hand"
	}
	owner := "loop " + runID
	notes := Resolve(ctx)
	if !plan.TakePlanningLock(ctx, owner) {
		return append(notes, "another planner holds the planning lock: the next tick plans")
	}
	func() {
		defer plan.DropPlanningLock(ctx, owner)
		for turn := range pool.MaxParallel + pool.MachineParallel + pool.PlanLanes + 1 {
			// Each start takes up to half a minute: the lock is taken again, which renews it, so
			// it never ages past `plan.LOCK_TTL` while the loop is still planning.
			if turn != 0 && !plan.TakePlanningLock(ctx, owner) {
				notes = append(notes, "another planner took the planning lock: the next tick plans")
				break
			}
			note, started := startOne(ctx, sleep)
			notes = append(notes, note)
			if !started {
				break
			}
		}
	}()
	return append(notes, Prune(ctx)...)
}

func startOne(ctx *hctx.Context, sleep func(float64)) (string, bool) {
	ref := NewRef(ctx.Now())
	pending := plan.Pending + ref
	claiming := withRunID(ctx, pending)
	quietOK := ""
	planned := plan.Make(claiming, false, nil, "auto", &quietOK)
	if action := planned.Get("action"); action == nil || action == "" || action == "none" {
		return strings.TrimRight("nothing more to start: "+py.Str(planned.GetOr("reason", "")), ": "), false
	}
	what := What(planned)
	runsOn := py.Str(py.Or(planned.Get("runs_on"), "ubuntu-latest"))
	shared, dryRun := "false", "false"
	if planned.Truthy("shared") {
		shared = "true"
	}
	if ctx.Cfg.DryRun {
		dryRun = "true"
	}
	inputs := map[string]string{"plan_ref": ref, "runs_on": runsOn, "cli": py.OrStr(planned.Get("cli"), ""),
		"secret": py.OrStr(planned.Get("secret"), ""), "shared": shared, "dry_run": dryRun}
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		ctx.GH.PutFile(fmt.Sprintf("%s/%s.json", Plans, ref), pyjson.DumpsIndent(planned, 2)+"\n",
			config.StateBranch, "", fmt.Sprintf("plan %s [skip ci]", ref))
		ctx.Act().DispatchWorkflow(config.NightWorkflow, ctx.Cfg.DefaultBranch, inputs)
	}); failed {
		setRun(ctx, planned, pending, "") // dead: the next housekeeping requeues it
		return fmt.Sprintf("could not start %s: %s", what, exc.Error()), false
	}
	runID := Find(ctx, ref, sleep)
	if runID != "" {
		setRun(ctx, planned, pending, runID)
		return fmt.Sprintf("started %s (run %s)", what, runID), true
	}
	return fmt.Sprintf("started %s (its run is not listed yet)", what), true
}

// What is controller._what.
func What(planned *pyjson.Object) string {
	number := planned.Get("number")
	action := planned.Get("action")
	item := py.Str(action)
	if pyjson.Truthy(number) {
		item = fmt.Sprintf("%s #%s", py.Str(action), py.Str(number))
	}
	return fmt.Sprintf("%s on %s", item, py.Str(planned.Get("provider")))
}

// Find is the id of the night run whose title names `ref`, or "" when GitHub does not list it yet.
func Find(ctx *hctx.Context, ref string, sleep func(float64)) string {
	for attempt := range FindTries {
		found, listed := "", false
		herr.Catch[*herr.GitHubError](func() {
			for _, run := range ctx.GH.ListRuns(config.NightWorkflow, "", 20, false) {
				if strings.HasPrefix(py.OrStr(run.Get("display_title"), ""), TitlePrefix(ref)) {
					found, listed = py.OrStr(run.Get("id"), ""), true
					return
				}
			}
		})
		if listed {
			return found
		}
		if attempt < FindTries-1 {
			sleep(FindWait)
		}
	}
	return ""
}

// setRun puts `run_id` on the claim `planned` made, if the claim still carries `pending`.
func setRun(ctx *hctx.Context, planned *pyjson.Object, pending string, runID string) {
	change := func(state *pyjson.Object) any {
		var entry any
		if planned.Get("action") == "suggest" {
			entry = state.Obj("suggest")
		} else {
			entry = state.Obj("items").Get(py.Str(planned.Get("number")))
		}
		if e, ok := entry.(*pyjson.Object); ok && e != nil && e.Get("run_id") == pending {
			e.Set("run_id", runID)
		}
		return nil
	}

	ctx.Store.Update(change, "run of "+What(planned))
}

// Resolve puts its run's id on every claim still pending, once GitHub lists the run.
func Resolve(ctx *hctx.Context) []string {
	state := ctx.Store.Load()
	waiting := pyjson.New() // ref -> the item's key in `items`, or "suggest"
	items := state.Obj("items")
	for _, number := range items.Keys() {
		entry, ok := items.Get(number).(*pyjson.Object)
		if ok && strings.HasPrefix(py.OrStr(entry.Get("run_id"), ""), plan.Pending) {
			waiting.Set(py.SliceFrom(py.Str(entry.Get("run_id")), py.Len(plan.Pending)), number)
		}
	}
	survey := state.Obj("suggest")
	if strings.HasPrefix(py.OrStr(survey.Get("run_id"), ""), plan.Pending) {
		waiting.Set(py.SliceFrom(py.Str(survey.Get("run_id")), py.Len(plan.Pending)), "suggest")
	}
	if waiting.Len() == 0 {
		return []string{}
	}
	var runs []*pyjson.Object
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		runs = ctx.GH.ListRuns(config.NightWorkflow, "", 50, false)
	}); failed {
		return []string{}
	}
	found := pyjson.New() // ref -> run id
	for _, run := range runs {
		title := py.OrStr(run.Get("display_title"), "")
		for _, ref := range waiting.Keys() {
			if strings.HasPrefix(title, TitlePrefix(ref)) && run.Truthy("id") {
				found.Set(ref, py.Str(run.Get("id")))
			}
		}
	}
	if found.Len() == 0 {
		return []string{}
	}

	change := func(state *pyjson.Object) any {
		for _, ref := range found.Keys() {
			key := waiting.Str(ref)
			var entry any
			if key == "suggest" {
				entry = state.Get("suggest")
			} else {
				entry = state.Obj("items").Get(key)
			}
			if e, ok := entry.(*pyjson.Object); ok && e.Get("run_id") == plan.Pending+ref {
				e.Set("run_id", found.Get(ref))
			}
		}
		return nil
	}

	ctx.Store.Update(change, "the loop's runs found")
	var notes []string
	for _, ref := range found.Keys() {
		of := "#" + waiting.Str(ref)
		if waiting.Str(ref) == "suggest" {
			of = "the survey"
		}
		notes = append(notes, fmt.Sprintf("found the run of %s: %s", of, found.Str(ref)))
	}
	return notes
}

// Prune deletes the plan files older than `KEEP_PLANS` (their reference holds their time). (The
// Python also took an AttributeError, a client with no `list_dir`, for nothing to prune; every
// gh.Client has it.)
func Prune(ctx *hctx.Context) []string {
	var names [][2]string
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		names = ctx.GH.ListDir(Plans, config.StateBranch)
	}); failed {
		return []string{}
	}
	cutoff := ctx.Now().Add(-KeepPlans)
	gone := 0
	for _, entry := range names {
		name, sha := entry[0], entry[1]
		ref := ""
		if strings.HasSuffix(name, ".json") {
			ref = strings.TrimSuffix(name, ".json")
		}
		if !Ref.MatchString(ref) {
			continue
		}
		made := time.Unix(int64(py.MustInt(ref[strings.LastIndex(ref, "-")+1:])), 0).UTC()
		if !made.Before(cutoff) {
			continue
		}
		herr.Catch[*herr.GitHubError](func() {
			ctx.GH.DeleteFile(fmt.Sprintf("%s/%s", Plans, name), config.StateBranch, sha,
				fmt.Sprintf("plan %s pruned [skip ci]", ref))
			gone++
		})
	}
	if gone > 0 {
		return []string{fmt.Sprintf("pruned %d plan file(s) older than %d days", gone, int(KeepPlans/(24*time.Hour)))}
	}
	return []string{}
}

// Fetch is the plan the loop wrote for `ref`, from the state branch. Its error is a *ValueError when
// the reference is malformed or the file is missing or unreadable; a GitHubError raises.
// (gh.GetFile gives "" for a missing file, so an empty file reads as missing.)
func Fetch(ctx *hctx.Context, ref string) (*pyjson.Object, error) {
	if !Ref.MatchString(ref) {
		return nil, valueError("%s is not a plan reference", py.Repr(ref))
	}
	text, _ := ctx.GH.GetFile(fmt.Sprintf("%s/%s.json", Plans, ref), config.StateBranch)
	if text == "" {
		return nil, valueError("no plan %s on %s", ref, config.StateBranch)
	}
	value, err := pyjson.LoadsString(text)
	if err != nil {
		return nil, &ValueError{Type: "JSONDecodeError", Msg: err.Error()}
	}
	planned, ok := value.(*pyjson.Object)
	if !ok {
		return nil, valueError("plan %s is not an object", ref)
	}
	return planned, nil
}

// Check is what is wrong with a dispatch's `given` runner, CLI, secret name and shared flag against
// the plan and `providers.json`: empty when all agree. The model job runs only on an empty list, so
// a dispatch cannot hand a model job a secret, a CLI or a runner its subscription does not have.
func Check(ctx *hctx.Context, planned *pyjson.Object, given map[string]string) []string {
	problems := []string{}
	providerID := py.OrStr(planned.Get("provider"), "")
	provider := ctx.Cfg.Pool.Get(providerID)
	if provider == nil {
		return []string{fmt.Sprintf("the plan names no subscription providers.json has (%s)", py.Repr(providerID))}
	}
	shared := provider.QuietCheck || provider.ID == "claude-1"
	// Its own runner, or its fill-in runners when the plan put it in another's lane (`fill_in`).
	own := provider.RunsOn
	if own == "" {
		own = "ubuntu-latest"
	}
	runners := []string{own}
	if provider.FillIn != nil {
		runners = append(runners, provider.FillIn.RunsOn)
	}
	runsOn := py.Str(py.Or(planned.Get("runs_on"), "ubuntu-latest"))
	expectedRunsOn := runners[0]
	for _, runner := range runners {
		if runner == runsOn {
			expectedRunsOn = runsOn
		}
	}
	keys := []string{"runs_on", "cli", "secret", "shared"}
	expected := map[string]string{"runs_on": expectedRunsOn, "cli": provider.CLI, "secret": provider.Secret,
		"shared": strings.ToLower(py.Str(shared))}
	plannedShared := "false"
	if planned.Truthy("shared") {
		plannedShared = "true"
	}
	plannedValues := map[string]string{"runs_on": runsOn,
		"cli": py.OrStr(planned.Get("cli"), ""), "secret": py.OrStr(planned.Get("secret"), ""), "shared": plannedShared}
	for _, key := range keys {
		want := expected[key]
		if plannedValues[key] != want {
			problems = append(problems, fmt.Sprintf("the plan's %s is %s, %s's is %s", key,
				py.Repr(plannedValues[key]), providerID, py.Repr(want)))
		}
		if given[key] != want {
			problems = append(problems, fmt.Sprintf("the dispatch's %s is %s, %s's is %s", key,
				py.Repr(given[key]), providerID, py.Repr(want)))
		}
	}
	return problems
}

// iso is clock.iso.
func iso(t time.Time) string { return clock.ISO(t) }
